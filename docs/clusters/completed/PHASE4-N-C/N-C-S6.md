# Invariant Slice — PHASE4-N-C S6

## Slice Header
**Slice Name:** RED scheduler + GREEN tick-assembler + RED broadcast handoff; slot-deadline operational SLA
**Cluster:** PHASE4-N-C
**Status:** Merged
**CEs addressed:** CE-N-C-6 (deterministic slot loop; non-leader silence; self-accept halts clean; slot-deadline measurement)
**Registry flips on merge:** `OP-OPS-05` → `enforced`
**Dependencies:** S1, S2, S3, S4, S5 merged. S5 ships `AcceptedBlock` with private constructor; this slice consumes that token at the broadcast boundary and is the entire reason for the type-level gate.

---

## Intent

Make the producer's RED side a deterministic, replayable state machine
that drives RED-signing → GREEN tick-assembly → BLUE forge → BLUE
self_accept → RED broadcast queue. Three load-bearing properties:

- The scheduler core is a **pure value transition** `scheduler_step(state, input) -> (state, Vec<effect>)` (mirrors `process_stream_input` from N-B). Wall-clock and I/O live in the outer driver, not the core.
- The GREEN tick-assembler is observably deterministic: identical RED outputs produce byte-identical `ProducerTick` values across two replays.
- `broadcast` consumes `AcceptedBlock` only (S5's type-level gate). Non-leader slots produce no effects; self-accept failures halt the scheduler deterministically.

Slot-deadline (`OP-OPS-05`) is an **operational SLA, not a hash-critical invariant**. The slice measures the full pipeline's wall-clock latency on a reference fixture and asserts it's under the slot deadline; missing the deadline costs a slot but does not violate any constitutional rule.

---

## The change (atomic; compile green as one unit)

### 1. New module `crates/ade_runtime/src/producer/scheduler.rs` (RED)

```rust
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::mempool::admit::MempoolState;
use ade_ledger::producer::{AcceptedBlock, SelfAcceptError, self_accept};
use ade_ledger::producer::forge::{forge_block, ForgeError, ForgeEffects};
use ade_ledger::producer::state::ProducerTick;
use ade_ledger::state::LedgerState;

use crate::producer::tick_assembler::{assemble_tick, TickInputs, TickAssemblyError};

/// The closed scheduler input. RED only — never crosses into BLUE.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerInput {
    /// A slot has elapsed; the producer should attempt forging at this slot.
    SlotTick {
        slot: u64,
        inputs: TickInputs,
    },
    /// The chain selector advanced; refresh the ledger / chain_dep / mempool
    /// baseline the next SlotTick starts from.
    ChainAdvanced {
        ledger: LedgerState,
        chain_dep: PraosChainDepState,
        mempool: MempoolState,
    },
}

/// The closed scheduler effect. RED dispatches.
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerEffect {
    /// Forge succeeded and self-accepted; queue for broadcast.
    EnqueueBroadcast(AcceptedBlock),
    /// Slot was non-leader; the producer is silent. (Observable for tests.)
    SilentNonLeader { slot: u64 },
    /// Forge or self-accept failed; the scheduler halts at this slot.
    /// `reason` is one of the closed BLUE error sums.
    HaltOnInvariant { slot: u64, reason: SchedulerHaltReason },
    /// Tick assembly produced a structurally inconsistent tick — should not
    /// happen if the GREEN assembler is honest, but defensive.
    HaltOnAssembly { slot: u64, reason: TickAssemblyError },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerHaltReason {
    Forge(ForgeError),
    SelfAccept(SelfAcceptError),
}

/// The closed scheduler state.
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulerState {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub mempool: MempoolState,
    pub era_schedule: EraSchedule,
    pub last_seen_slot: Option<u64>,
    pub prev_opcert_counter: Option<u64>,
    /// True once HaltOn* has been observed; subsequent SlotTick inputs are
    /// ignored (SchedulerEffect::HaltOnInvariant is re-emitted with the
    /// original reason captured at first halt).
    pub halted: Option<SchedulerHaltReason>,
}

/// Pure RED state transition. No I/O, no clock — the outer driver feeds the
/// slot number from a wall-clock source; this function consumes that value.
/// Determinism: identical (state, input) -> identical (state', effects).
pub fn scheduler_step<L: LedgerView>(
    state: SchedulerState,
    input: SchedulerInput,
    ledger_view: &L,
) -> (SchedulerState, Vec<SchedulerEffect>) {
    /* impl: dispatch on input variant; for SlotTick:
       1. If state.halted.is_some(): return (state, vec![HaltOnInvariant{...}])
       2. assemble_tick(state, &inputs, ledger_view) -> ProducerTick OR HaltOnAssembly
       3. forge_block(&tick) -> Ok((forged, effects)) OR HaltOnInvariant(Forge)
       4. self_accept(&forged.bytes, &state.ledger, &state.chain_dep,
                      &state.era_schedule, ledger_view) -> Ok(accepted) OR HaltOnInvariant(SelfAccept)
       5. Update state.prev_opcert_counter from forge effects
       6. Return EnqueueBroadcast(accepted)
       For ChainAdvanced: refresh state.{ledger, chain_dep, mempool}, last_seen_slot.
    */
}
```

### 2. New module `crates/ade_runtime/src/producer/tick_assembler.rs` (GREEN)

```rust
use ade_crypto::vrf::{vrf_proof_to_hash, VrfOutput, VrfProof};
use ade_crypto::kes::{Ed25519VerificationKey, KesPeriod, KesSignature};
use ade_types::shelley::block::OperationalCert;
use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_ledger::mempool::admit::MempoolState;
use ade_ledger::producer::state::ProducerTick;
use ade_ledger::state::LedgerState;
use ade_ledger::pparams::ProtocolParameters;

/// Closed RED-supplied inputs the assembler stitches into a canonical
/// ProducerTick. Carries signed artifacts only; no private keys.
#[derive(Debug, Clone, PartialEq)]
pub struct TickInputs {
    pub vrf_proof: VrfProof,
    pub kes_period: KesPeriod,
    pub kes_signature: KesSignature,
    pub opcert: OperationalCert,
    pub cold_vk: Ed25519VerificationKey,
    pub leader_answer: LeaderScheduleAnswer,
    pub pparams: ProtocolParameters,
    pub mempool_tx_bytes: Vec<Vec<u8>>,
    pub prev_opcert_counter: Option<u64>,
    /* + whatever additional ProducerTick header fields S3's final struct
       requires (block_no, prev_hash, issuer_vkey_hash, protocol_version) */
}

#[derive(Debug, Clone, PartialEq)]
pub enum TickAssemblyError {
    /// vrf_proof_to_hash failed structurally (S1's verify path already pins
    /// proof-byte length, but defensive at this boundary).
    VrfProofMalformed { detail: &'static str },
    /// inputs.mempool_tx_bytes.len() != mempool.accepted().len()
    MempoolWidthMismatch { tx_bytes: usize, accepted_ids: usize },
}

/// Pure GREEN function. No clock, no rand, no I/O. Two identical (slot, state, inputs)
/// inputs MUST produce byte-identical ProducerTick values — that's the
/// "observably deterministic" property the test pins.
pub fn assemble_tick(
    slot: u64,
    base_state: &LedgerState,
    mempool: &MempoolState,
    inputs: &TickInputs,
) -> Result<ProducerTick, TickAssemblyError> {
    /* impl: vrf_proof_to_hash(vrf_proof), build ProducerTick value */
}
```

### 3. New module `crates/ade_runtime/src/producer/broadcast.rs` (RED)

```rust
use ade_ledger::producer::AcceptedBlock;

/// The closed broadcast effect surface. Outbound only; the actual N2N
/// delivery wiring lives in `ade_network` and is consumed by N-A's
/// block-fetch / chain-sync server path (out of scope here; this slice
/// ships only the queue).
#[derive(Debug, Clone, PartialEq)]
pub enum BroadcastError {
    /// Queue is at capacity. Closed for back-pressure.
    QueueFull,
    /// The shutdown signal was received and the queue refuses new work.
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BroadcastQueue {
    queue: std::collections::VecDeque<AcceptedBlock>,
    capacity: usize,
}

impl BroadcastQueue {
    pub fn new(capacity: usize) -> Self {
        Self { queue: std::collections::VecDeque::new(), capacity }
    }

    /// Enqueue a self-accepted block. Type-level gate: caller must hand over
    /// an `AcceptedBlock` value (no public constructor outside S5's
    /// `self_accept`).
    pub fn enqueue(&mut self, block: AcceptedBlock) -> Result<(), BroadcastError> {
        if self.queue.len() >= self.capacity {
            return Err(BroadcastError::QueueFull);
        }
        self.queue.push_back(block);
        Ok(())
    }

    /// Dequeue for the network-handoff layer.
    pub fn dequeue(&mut self) -> Option<AcceptedBlock> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize { self.queue.len() }
    pub fn is_empty(&self) -> bool { self.queue.is_empty() }
}
```

`VecDeque` is closed-iteration order; the queue is FIFO. No `HashMap`,
no async, no I/O.

### 4. Register modules in `crates/ade_runtime/src/producer/mod.rs`

```rust
pub mod broadcast;
pub mod keys;       // S1
pub mod scheduler;
pub mod signing;    // S1
pub mod tick_assembler;
```

### 5. Unit + integration tests

**In `crates/ade_runtime/src/producer/scheduler.rs` `#[cfg(test)] mod tests`:**

- `producer_scheduler_silent_on_non_leader_slots` — feed a SlotTick whose
  `inputs.leader_answer` + `vrf_proof` will not pass `is_leader_for_vrf_output`.
  Assert effect set is exactly `[SilentNonLeader { slot }]` and
  `state.halted.is_none()`.
- `producer_scheduler_self_accept_failure_halts_clean` — feed a SlotTick
  where forge succeeds but the produced bytes fail self_accept (e.g.,
  corrupted via a test-only intercept point — simplest: inject an opcert
  with a flipped sigma byte that nevertheless passes opcert_validate's
  shape check but fails the cold-signature step — actually opcert_validate
  catches that, so use a different corruption path: a ProducerTick whose
  KES signature is malformed but length-correct so self_accept's KES verify
  rejects). Assert effects start with one `HaltOnInvariant { slot, reason:
  SelfAccept(_) }` and `state.halted` is set.
- `producer_scheduler_halted_state_ignores_future_ticks` — once halted,
  subsequent SlotTick inputs produce a single `HaltOnInvariant` effect
  with the ORIGINAL reason; the state's `halted` field is the source of
  truth.
- `producer_scheduler_chain_advanced_refreshes_baseline` — feed a
  ChainAdvanced input; assert state.ledger / chain_dep / mempool are
  replaced and effects vec is empty.

**In `crates/ade_runtime/src/producer/tick_assembler.rs` `#[cfg(test)] mod tests`:**

- `tick_assembler_deterministic_over_captured_red_outputs` — for a fixed
  `(slot, base_state, mempool, inputs)` quadruple, `assemble_tick` called
  twice produces byte-identical `ProducerTick` outputs (via `==`).
- `tick_assembler_rejects_mempool_width_mismatch` —
  `inputs.mempool_tx_bytes.len() = 3`, `mempool.accepted().len() = 2` →
  `Err(MempoolWidthMismatch { tx_bytes: 3, accepted_ids: 2 })`.
- `tick_assembler_rejects_malformed_vrf_proof` — proof bytes of wrong
  derived shape → `Err(VrfProofMalformed { .. })`.

**In `crates/ade_runtime/src/producer/broadcast.rs` `#[cfg(test)] mod tests`:**

- `broadcast_queue_enqueues_only_accepted_block` — the queue's `enqueue`
  signature is `fn(&mut self, AcceptedBlock) -> Result<(), BroadcastError>`,
  so the test merely calls it with a real `AcceptedBlock` (built via
  `self_accept` in a `#[cfg(test)]` helper) and asserts `Ok(())`.
- `broadcast_queue_rejects_when_full` — fill to capacity, next enqueue
  returns `Err(BroadcastError::QueueFull)`.
- `broadcast_queue_fifo` — enqueue A, B, C; dequeue returns A, B, C in
  order.

**In `crates/ade_runtime/tests/producer_pipeline_slot_deadline.rs` (integration, RED):**

- `producer_full_path_under_slot_deadline_on_reference_fixture` — over a
  small reference corpus (the same fixtures S3's testkit ships), measure
  wall-clock latency of one full RED→GREEN→BLUE→BLUE→RED step
  (`scheduler_step` with a SlotTick input). Assert the median latency
  over N=10 runs is under 1000ms (the mainnet slot deadline). This is
  an **operational SLA**, not a hash-critical invariant — the test is
  the only place wall-clock measurement is permitted in this cluster.

### 6. New CI gate `ci/ci_check_scheduler_closure.sh`

Mechanical guards:

1. **`scheduler_step` is pure of I/O.** Grep
   `crates/ade_runtime/src/producer/scheduler.rs` and
   `tick_assembler.rs` for `std::fs` / `std::env` / `tokio::time` /
   `getrandom` / `rand::` / `println!` / `dbg!` / `async fn` / `.await`
   — any match outside `#[cfg(test)]` is a failure. (`std::time` is
   permitted only in the `tests/producer_pipeline_slot_deadline.rs`
   integration test — that file is whitelisted.)
2. **`broadcast::enqueue` consumes `AcceptedBlock` by value.** Grep
   `crates/ade_runtime/src/producer/broadcast.rs` for
   `fn enqueue.*&AcceptedBlock\|fn enqueue.*Vec<u8>\|fn enqueue.*&\[u8\]` —
   any match is a failure (must take `AcceptedBlock` by value or by
   move; reference-typed argument or raw bytes are forbidden).
3. **`SchedulerInput`, `SchedulerEffect`, `SchedulerHaltReason`,
   `BroadcastError`, `TickAssemblyError` are closed sums.** No
   `#[non_exhaustive]` immediately preceding any `pub enum`.
4. **No `pub fn` in `broadcast.rs` returns raw forged bytes from the
   queue except via `dequeue() -> Option<AcceptedBlock>` (which
   preserves the type).** Grep for `pub fn .*-> Vec<u8>\|pub fn .*->
   &\[u8\]` in `broadcast.rs` — any match other than methods
   *on* `AcceptedBlock` is a failure.
5. **No `cardano_crypto::vrf::VrfDraft03::prove` / `kes::sign_kes` /
   `kes::update_kes` in `scheduler.rs`, `tick_assembler.rs`, or
   `broadcast.rs`.** Signing is S1's `signing.rs` only.

### 7. Registry updates (same commit)

Flip `OP-OPS-05` to `enforced` with populated arrays (the schema fix
from S5 permits this on operational tier):

- `OP-OPS-05` — `tests = ["producer_full_path_under_slot_deadline_on_reference_fixture"]`,
  `ci_script = "ci/ci_check_scheduler_closure.sh"`,
  `code_locus = "crates/ade_runtime/src/producer/scheduler.rs (scheduler_step + the full pipeline timing); crates/ade_runtime/tests/producer_pipeline_slot_deadline.rs (wall-clock measurement)"`,
  `status = "enforced"`.

Also flip `OP-OPS-04` to `enforced` (the schema is now permissive on
operational tier when status=enforced; the mechanical guard
`ci/ci_check_private_key_custody.sh` has been in place since S1):

- `OP-OPS-04` — `tests = ["cardano_cli_skey_envelope_round_trips_through_keys_loader",
  "keys_loader_rejects_wrong_envelope_type",
  "keys_loader_rejects_malformed_cbor_hex",
  "key_load_error_io_carries_no_path_bytes"]`,
  `ci_script = "ci/ci_check_private_key_custody.sh"`,
  `code_locus = "crates/ade_runtime/src/producer/keys.rs (load_*_signing_key_skey, KeyLoadError); crates/ade_runtime/src/producer/signing.rs (RED-confined custody)"`,
  `status = "enforced"`. **Update the `open_obligation` field**: remove
  the schema half (resolved by S5); retain the Sum6KES serialization
  half (the cardano-crypto 1.0.8 gap remains an honest limitation).

### 8. ProducerTick header-fields completeness check

S3 may not yet carry every header field needed for a full Shelley/Conway
header on `ProducerTick` (block_no, prev_hash, issuer_vkey_hash,
protocol_version, etc.). S6 consumes `assemble_tick` which produces a
`ProducerTick`; if header fields are missing, this slice ADDS the missing
fields to `ProducerTick` (`crates/ade_ledger/src/producer/state.rs`) and
to `TickInputs` (above). Pin every addition with a one-line "introduced
in S6 for header-field completeness" comment so a reader knows the
provenance.

If S3 already covered them, no change to `state.rs`.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_runtime producer::scheduler` green (4
  tests).
- **AC-3** — `cargo test -p ade_runtime producer::tick_assembler` green
  (3 tests).
- **AC-4** — `cargo test -p ade_runtime producer::broadcast` green (3
  tests).
- **AC-5** — `cargo test -p ade_runtime --test producer_pipeline_slot_deadline`
  green (1 integration test; uses `std::time` and is the only file in
  the cluster that may).
- **AC-6** — `cargo test --workspace` green except pre-existing
  `boundary_fingerprint_matches_pins`.
- **AC-7** — `bash ci/ci_check_scheduler_closure.sh` returns `PASS`
  (5 guards).
- **AC-8** — all prior CI gates pass unregressed:
  `ci_check_self_accept_gate.sh`, `ci_check_no_producer_body_encoder.sh`,
  `ci_check_forge_purity.sh`, `ci_check_no_private_keys_in_corpus.sh`,
  `ci_check_private_key_custody.sh`, `ci_check_opcert_closed.sh`.
- **AC-9** — `bash ci/ci_check_constitution_coverage.sh` returns
  `PASS` (OP-OPS-04 + OP-OPS-05 status fields parse as `"enforced"`
  with populated `tests` + `ci_script`).
- **AC-10** — `grep -rE 'std::time' crates/ade_runtime/src/producer/`
  returns no matches (wall-clock is integration-test-only;
  `tests/producer_pipeline_slot_deadline.rs` is allowed).
- **AC-11** — `grep -rE 'fn enqueue.*&AcceptedBlock' crates/`
  returns no matches (broadcast takes the token by value).

---

## Hard Prohibitions

Cluster-level prohibitions inherited. Slice-specific:

- No `std::time` / `tokio::time` / `getrandom` / `rand::` /
  `println!` / `dbg!` / `async fn` / `.await` / `std::fs` / `std::env`
  in `scheduler.rs`, `tick_assembler.rs`, or `broadcast.rs`. Allowed
  ONLY in `crates/ade_runtime/tests/producer_pipeline_slot_deadline.rs`
  (whitelisted by the CI gate).
- No `HashMap` / `HashSet` iteration anywhere in the new modules. The
  broadcast queue is a `VecDeque` (closed iteration order).
- No `pub` constructor of `AcceptedBlock` here. The token comes from
  S5's `self_accept` only.
- No `#[non_exhaustive]` on any new sum type.
- No `String`-bearing variant on any new sum type.
- No reference-typed `enqueue` argument; `AcceptedBlock` is consumed
  by value/move.
- No `serde` Serialize/Deserialize on any of the new types (the
  scheduler core is pure values; persistence is out of scope).
- No call to `cardano_crypto::vrf::VrfDraft03::prove` /
  `kes::sign_kes` / `kes::update_kes` in the new modules. Those are
  S1's `signing.rs` exclusive.
- No `ade_network` dependency at the producer-RED boundary — broadcast
  is a value queue; network delivery is upstream-wired.
- No widening of `ProducerTick` to carry private-key material under
  any pretense.

---

## Explicit Non-Goals

- RED signing primitives — S1.
- BLUE `opcert_validate` — S2.
- BLUE forge core — S3.
- BLUE body-hash unification — S4.
- BLUE self-acceptance gate — S5.
- Cross-impl adapter + live-evidence binary — S7 (CE-N-C-7/8).
- The N2N delivery itself. S6 ships the value queue; wiring to
  `ade_network`'s block-fetch / chain-sync server path is N-A
  follow-on scope.
- Outer-loop wall-clock driver. The slice ships the pure
  `scheduler_step` function and the integration timing test; a real
  process loop (`tokio::time::interval` etc.) lives in `ade_node`
  binary, not here.
- Hot-reload of keys without restart — non-goal per OQ-11.
- A full chain-selector-to-producer wiring. The `ChainAdvanced` input
  is the abstract handoff point; how `chain_dep` etc. flow from
  the chain selector to here is the integration question the binary
  layer answers.

---

## Failure Modes

`SchedulerHaltReason::{Forge,SelfAccept}` and `TickAssemblyError::*`
are deterministic and structured. Once halted, the scheduler emits
`HaltOnInvariant` for every subsequent SlotTick — RED restart is the
only recovery path.

`BroadcastError::{QueueFull, Shutdown}` are recoverable at the queue
boundary (back-pressure pattern). They do NOT halt the scheduler;
they fail the enqueue and the scheduler re-tries on the next
`ChainAdvanced` or `SlotTick`.

`OP-OPS-05` is **operational**: missing the slot deadline is not a
halt condition; it's a measurement that the operator should monitor.
The test asserts the median is under 1000ms; outliers (e.g., cold
cache) don't fail the test.

---

## Grounding (verified at HEAD `aa7a7dd`)

- S1's `ade_runtime::producer::{signing, keys}` exist; this slice adds
  three sibling modules to the same crate.
- S5's `ade_ledger::producer::{AcceptedBlock, SelfAcceptError,
  self_accept}` are re-exported from
  `crates/ade_ledger/src/producer/mod.rs`.
- S3's `ade_ledger::producer::{forge::forge_block, state::ProducerTick}`
  are reachable; S6 consumes both at the scheduler core.
- S3's `ade_codec::shelley::tx_components::split_conway_tx_components`
  is used by forge — not directly by S6.
- N-B's existing RED orchestrator at
  `crates/ade_runtime/src/consensus/chain_selector.rs` is the template
  for the scheduler-core shape (`process_stream_input(...)`); S6's
  `scheduler_step` mirrors that pattern.
- `ade_core::consensus::leader_schedule::is_leader_for_vrf_output` is
  the leader-check function forge calls — the scheduler doesn't call
  it directly; forge already does.
- The `LedgerView` trait at
  `crates/ade_core/src/consensus/ledger_view.rs:32` is what
  `scheduler_step` takes generically; same as N-B's pattern. The
  binary layer instantiates a concrete `LedgerView` from chain DB.

---

## Notes on the timing test

The wall-clock measurement in
`producer_pipeline_slot_deadline.rs` uses `std::time::Instant`
strictly to record the duration of `scheduler_step` calls. It is the
*only* file in this slice (and across `ade_runtime/src/producer/`)
permitted to import `std::time`. The CI gate explicitly whitelists
that path.

The deadline (1000ms = mainnet slot) is intentionally loose. The
operational SLA is "the producer must complete its work within a
slot"; tighter bounds (e.g., the 500ms safety margin for the gossip
mesh, or the 100ms tail-latency target for SPO peers) live in
operational documentation, not in the constitutional invariant
registry.
