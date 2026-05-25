# Invariant Slice — PHASE4-N-C S3

## Slice Header
**Slice Name:** BLUE `forge_block` + `ProducerTick` canonical type + leader-check gate + tx-admissibility prefix + purity
**Cluster:** PHASE4-N-C
**Status:** Proposed (v2 — host crate corrected to `ade_ledger::producer`)
**CEs addressed:** CE-N-C-3 (forge purity + replay byte-equality + leader-check + tx-admissibility)
**Registry flips on merge:** `DC-CONS-13`, `DC-CONS-14`, `DC-CONS-15`, `DC-LEDGER-12` → `enforced`
**Dependencies:** S1 merged (RED signing primitives — provides `VrfProof`, `KesSignature` artifact types). S2 merged (BLUE `opcert_validate` in `ade_core::consensus` — consumed here).

### Host-crate decision (correction to cluster doc v1)

The cluster doc v1 placed `forge` and `producer_state` under `ade_core::consensus::*`. That is **structurally impossible**: `ade_ledger` already declares `ade_core = { path = "../ade_core" }` in `Cargo.toml`, so `ade_core` cannot import `ade_ledger::state::LedgerState`, `ade_ledger::mempool::admit::{admit, MempoolState}`, or any other ledger value type without a dependency cycle.

**This slice relocates the BLUE forge authority to `crates/ade_ledger/src/producer/`** (mirroring the existing producer/consumer split: `ade_ledger::block_validity` and `ade_ledger::mempool::admit` are the validator-side BLUE; `ade_ledger::producer` is the producer-side BLUE; `ade_runtime::producer` is the RED shell). The cluster doc's TCB color map is patched in the same commit as this slice to reflect the relocation. The classification (BLUE) is unchanged.

S4 and S5 also relocate to `ade_ledger::producer::*` for the same reason — those slice docs will pin the new paths.

---

## Intent

Make `ade_ledger::producer::forge::forge_block` the single producer authority
for block assembly. `forge_block` is a pure function from a canonical
`ProducerTick` to forged block bytes. Three forbidden states are mechanically
unreachable at the forge boundary:

- non-leader tick → `ForgeError::NotLeader`
- tx-set that is not a prefix of `mempool::admit`'s canonical accumulating
  order → `ForgeError::TxSetNotAdmissiblePrefix`
- opcert that fails `opcert_validate` → `ForgeError::OpCertRejected(OpCertError)`

Replaying any captured `ProducerTick` stream produces byte-identical forged
block bytes across two runs. The forge function imports no I/O, no wall-clock,
no rand, no `HashMap` iteration, no env, no locale, and no `std::fs` —
mechanically enforced by a CI grep gate.

The producer's leader decision uses the **same** `is_leader_for_vrf_output`
function the N-B validator uses (re-exported from
`ade_core::consensus::leader_schedule::is_leader_for_vrf_output`); the slice
forbids a producer-side re-implementation.

---

## The change (atomic; compile green as one unit)

### 1. New canonical type `ProducerTick` in `crates/ade_ledger/src/producer/state.rs` (BLUE)

`ProducerTick` is the only sanctioned input to `forge_block`. It carries every
input forge needs as an explicit value — no ambient state, no implicit reads.

```rust
use ade_crypto::vrf::{VrfOutput, VrfProof};
use ade_crypto::kes::{Ed25519VerificationKey, KesPeriod, KesSignature};
use ade_types::shelley::block::OperationalCert;
use ade_types::primitives::SlotNo;
use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;

use crate::state::LedgerState;
use crate::mempool::admit::MempoolState;
use crate::pparams::ProtocolParameters;  // existing — see grounding

/// A canonical producer tick — every input `forge_block` needs as an explicit
/// value. This is the BLUE input surface; replay corpora carry `ProducerTick`
/// values directly. Private-key fields are absent by construction (S1's
/// invariant — captured signed artifacts only).
#[derive(Debug, Clone, PartialEq)]
pub struct ProducerTick {
    pub slot: SlotNo,
    pub base_state: LedgerState,
    pub mempool: MempoolState,
    /// The ordered, byte-preserved tx CBOR slices that produced this mempool
    /// snapshot. The order matches `mempool.accepted()` index-by-index.
    /// Carried explicitly because `MempoolState` only retains `Hash32` ids —
    /// forge must have the wire bytes to assemble the block body.
    pub mempool_tx_bytes: Vec<Vec<u8>>,
    pub pparams: ProtocolParameters,
    pub leader_answer: LeaderScheduleAnswer,
    pub vrf_proof: VrfProof,
    pub vrf_output: VrfOutput,
    pub kes_period: KesPeriod,
    pub kes_signature: KesSignature,
    pub opcert: OperationalCert,
    /// The operator's cold verification key — supplied by RED, validated by
    /// BLUE via `opcert_validate`.
    pub cold_vk: Ed25519VerificationKey,
    /// Durable per-(cold-key, node) opcert counter; `None` only for the first
    /// opcert this node has ever produced.
    pub prev_opcert_counter: Option<u64>,
}
```

If `ProtocolParameters` is not cleanly Clone+Debug+PartialEq, the implementer
takes the smallest pinning needed for replay-equivalence rather than
duplicating the type. Surface the choice as a deviation if there's friction.

### 2. New module `crates/ade_ledger/src/producer/forge.rs` (BLUE)

```rust
use ade_core::consensus::leader_schedule::is_leader_for_vrf_output;
use ade_core::consensus::opcert_validate::{opcert_validate, OpCertError};
use ade_crypto::blake2b_256;
use ade_types::Hash32;
use ade_types::shelley::block::{ShelleyBlock, ShelleyHeader};

use crate::mempool::admit::{admit, AdmitOutcome, MempoolState, TxRejectClass};
use crate::producer::state::ProducerTick;

/// The closed forge-time error sum. Every variant is fail-fast at the
/// RED -> BLUE boundary: a tick that produces any `Err` here MUST NOT be
/// retried with the same artifacts.
#[derive(Debug, Clone, PartialEq)]
pub enum ForgeError {
    /// is_leader_for_vrf_output(leader_answer, vrf_output) == false.
    NotLeader { slot: u64 },
    /// opcert_validate rejected the tick's opcert.
    OpCertRejected(OpCertError),
    /// One of mempool_tx_bytes failed to re-admit against the base state in
    /// the tick's order. Forge returns the index that failed; this is a
    /// pure prefix-respecting check.
    TxSetNotAdmissiblePrefix {
        failed_at: usize,
        rejected_class: TxRejectClass,
    },
    /// mempool_tx_bytes length != mempool.accepted().len() — tick is
    /// structurally inconsistent.
    MempoolWidthMismatch { tx_bytes: usize, accepted_ids: usize },
    /// kes_signature length not SUM6_KES_SIG_LEN (defense-in-depth; S1's
    /// closed type already pins this).
    BadKesSignatureLength { found: usize },
    /// One of mempool_tx_bytes failed CBOR component-split (malformed at
    /// forge time despite passing admit — should be impossible if the
    /// admit-prefix check above is honest; defensive catch).
    TxComponentSplit { failed_at: usize, detail: &'static str },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForgedBlock {
    pub bytes: Vec<u8>,
    pub block: ShelleyBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForgeEffects {
    /// The forged block is ready for self-acceptance (S5) and broadcast (S6).
    /// Carries the opcert sequence_number BLUE accepted, so RED can persist
    /// `prev_opcert_counter = Some(...)` for the next tick.
    ReadyForSelfAccept { next_prev_opcert_counter: u64 },
}

/// Forge a block from a canonical ProducerTick. Pure, BLUE, total.
///
/// Pipeline (in order — every step is total and deterministic):
/// 1. Width check: `tick.mempool_tx_bytes.len() == tick.mempool.accepted().len()`.
/// 2. Opcert validate: `opcert_validate(&tick.opcert, &tick.cold_vk,
///    tick.opcert.kes_period, tick.prev_opcert_counter)`.
/// 3. Leader check: `is_leader_for_vrf_output(&tick.leader_answer, &tick.vrf_output)`.
/// 4. Admit-prefix check: iterate `tick.mempool_tx_bytes` in order,
///    running `admit(&running_mempool, &bytes)` starting from
///    `MempoolState::new(tick.base_state.clone())`. Every step must
///    `AdmitOutcome::Admitted`. After all steps,
///    `running_mempool.accepted()` must byte-equal `tick.mempool.accepted()`.
/// 5. Per-tx component split: `split_conway_tx_components(tx_bytes)` for each
///    tick.mempool_tx_bytes[i] -> (body, witness_set, is_valid, aux_or_nil).
/// 6. Build the four body byte buckets:
///       tx_bodies   = CBOR(definite_array(n)) || body_0 || body_1 || ...
///       witness_sets= CBOR(definite_array(n)) || ws_0 || ws_1 || ...
///       metadata    = CBOR(definite_map(k)) || (idx_i, aux_i)*
///                       for tx i with non-nil aux_data; empty map (a0) if k=0.
///       invalid_txs = CBOR(definite_array(m)) || idx_0 || idx_1 || ...
///                       for tx i where is_valid==false; absent (None) if m=0.
/// 7. Body hash (validator-shared recipe, matching
///    ade_ledger::block_validity::header_input::block_body_hash):
///       body_hash = blake2b_256(
///           blake2b_256(tx_bodies)
///        || blake2b_256(witness_sets)
///        || blake2b_256(metadata)
///        || blake2b_256(invalid_txs OR empty bytes if None)
///       )
/// 8. Build the ShelleyHeader{ body: {..., body_hash, vrf, operational_cert,
///    kes_period, protocol_version, ...}, kes_signature }. Most header fields
///    come straight from the tick (slot, vrf_proof, vrf_output, opcert,
///    kes_period, kes_signature); a few (block_no, prev_hash, issuer_vkey,
///    block_size, protocol_version) come from the tick via tick.leader_answer
///    or new tick fields if not already represented — pin in implementation.
/// 9. Build the ShelleyBlock value and encode via ShelleyBlock::ade_encode
///    (existing in ade_codec::shelley::block:80).
/// 10. Return ForgedBlock { bytes, block } + ForgeEffects::ReadyForSelfAccept{
///     next_prev_opcert_counter = tick.opcert.sequence_number }.
pub fn forge_block(tick: &ProducerTick) -> Result<(ForgedBlock, Vec<ForgeEffects>), ForgeError> {
    /* impl */
}
```

Implementation notes:

- Step 8's "header fields not on the tick today" (block_no, prev_hash,
  issuer_vkey, block_size, protocol_version) need to be sourced from the
  tick. Add them to `ProducerTick` as needed — the slice's invariants are
  about *purity and gating*, not about which specific header fields are
  inputs. Whatever shape `ProducerTick` ends up with, every header field
  must be derivable purely from `ProducerTick` values; nothing comes from
  ambient state.
- `block_size` is computed *after* the body bytes are assembled but
  *before* the header is encoded — that means a small fixed-point: encode
  the header twice (once with placeholder block_size to get the encoded
  body length, then with the real block_size). The implementer chooses
  either the fixed-point approach or directly computing
  `block_size = body_bytes.len()` if the validator's `block_size`
  field means body bytes only (verify by reading
  `ade_codec::shelley::block::decode_header_body`).

### 3. New module `crates/ade_codec/src/shelley/tx_components.rs` (BLUE)

The tx-component splitter S3 needs and S4 reuses.

```rust
use crate::CodecError;
use crate::cbor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxComponents<'a> {
    pub body_bytes: &'a [u8],
    pub witness_set_bytes: &'a [u8],
    pub is_valid: bool,
    pub aux_data_bytes: Option<&'a [u8]>,  // None if nil
}

/// Split a full Conway tx CBOR `[body, witness_set, is_valid_bool,
/// aux_data_or_nil]` into preserved-byte component slices. Pure, total,
/// deterministic over the input.
pub fn split_conway_tx_components(tx_cbor: &[u8]) -> Result<TxComponents<'_>, CodecError> {
    /* impl: cbor::read_array_header, then four cbor::skip_item slices */
}
```

### 4. New module `crates/ade_testkit/src/producer/replay.rs` + `crates/ade_testkit/src/producer/fixtures.rs`

Fixtures are compiled into the binary as Rust source (no `*.skey` / no
serde-dep churn). Three initial fixtures:

```rust
// fixtures.rs
pub fn fixture_empty_mempool_leader() -> ProducerReplayFixture;
pub fn fixture_two_tx_mempool_leader() -> ProducerReplayFixture;
pub fn fixture_non_leader() -> ProducerReplayFixture;

// replay.rs
pub struct ProducerReplayFixture {
    pub label: &'static str,
    pub ticks: Vec<ade_ledger::producer::state::ProducerTick>,
    pub expected_forged: Vec<Vec<u8>>,
    /// Expected `Err` shape for the i-th tick if `expected_forged[i].is_empty()`.
    /// Carries the variant tag only, not detailed payload, so the replay test
    /// can match by discriminant.
    pub expected_err_tag: Vec<Option<ExpectedErr>>,
}

pub enum ExpectedErr {
    NotLeader,
    TxSetNotAdmissiblePrefix,
    OpCertRejected,
    MempoolWidthMismatch,
}

pub fn producer_replay_fixtures() -> Vec<ProducerReplayFixture>;
```

Fixture generation (`#[cfg(test)] #[ignore]` helper at
`crates/ade_testkit/tests/regen_producer_fixtures.rs`):

- For each fixture, build a `ProducerTick` deterministically:
  - seed cold key, KES, VRF from fixed 32-byte seeds
  - construct opcert via `ed25519_dalek` signing (same pattern S2 used)
  - call `ade_runtime::producer::signing::vrf_prove` + `kes_sign` to get
    artifacts
  - assemble the tick value
- For positive fixtures, run forge once via this regen helper, capture the
  bytes, and write them into `fixtures.rs` as a Rust byte-literal constant.
- For negative fixtures, set `expected_forged[i] = Vec::new()` and
  `expected_err_tag[i] = Some(ExpectedErr::...)`.

For `fixture_two_tx_mempool_leader`: the "two trivially valid txs" question
is real. Two acceptable paths:

(a) Find existing trivially-valid tx fixtures in `ade_ledger::tx_validity`
or `ade_testkit::tx_validity` tests and reuse one or two of them.

(b) If no such fixture is reachable without extensive setup, the
`fixture_two_tx_mempool_leader` is degraded to a NEGATIVE fixture: use
two txs that fail admit at index 1 and check the
`TxSetNotAdmissiblePrefix { failed_at: 1, .. }` path. The positive
byte-equality case is then covered by `fixture_empty_mempool_leader`
alone. **This is a documented deviation if taken**.

### 5. Unit + replay tests (named in cluster doc CE-N-C-3)

In `crates/ade_ledger/src/producer/forge.rs` `#[cfg(test)] mod tests` (BLUE):

- `forge_block_rejects_non_leader_tick`
- `forge_block_rejects_tx_not_in_mempool_accepted_prefix`
- `forge_block_rejects_tx_permuted_from_accumulating_order`
- `forge_block_empty_mempool_produces_empty_body`
- `forge_block_uses_validator_leader_check_function` — a compile-time
  reference: `const _: fn(&LeaderScheduleAnswer, &VrfOutput) -> bool =
  is_leader_for_vrf_output;` plus a CI grep that the actual call site
  appears in `forge.rs`.

In `crates/ade_testkit/src/producer/replay.rs` `#[cfg(test)] mod tests`:

- `forge_block_pure_no_io` — for every positive fixture, forge twice on
  the same input; outputs byte-equal.
- `forge_block_replay_byte_identical` — for every fixture with
  `!expected_forged[i].is_empty()`, `forge_block(&ticks[i]).0.bytes ==
  expected_forged[i]`. For empty entries, match against
  `expected_err_tag[i]`.

In `crates/ade_codec/src/shelley/tx_components.rs` `#[cfg(test)] mod tests`:

- `split_conway_tx_components_round_trips` — for a known Conway tx CBOR,
  the four returned slices concatenated with the 4-element array header
  recover the input.
- `split_conway_tx_components_rejects_short_array` — 3-element input →
  `Err(CodecError::InvalidCborStructure)`.
- `split_conway_tx_components_rejects_trailing_garbage` — trailing byte →
  `Err`.

In `crates/ade_ledger/src/mempool/admit.rs` `#[cfg(test)] mod tests`:

- `admit_prefix_property_documented` — sanity-pin admit's prefix
  property: admitting `[tx_a, tx_b]` produces the same accumulating
  state as `admit(admit(empty, tx_a).0, tx_b)`. Already provable by
  construction; pinning as documentation-as-test.

### 6. New CI gate `ci/ci_check_forge_purity.sh` (closure proof)

Mechanical guards (all paths under `crates/ade_ledger/src/producer/` and
`crates/ade_codec/src/shelley/tx_components.rs`):

1. **No I/O / clock / rand.** Grep for `std::time` / `std::time::Instant` /
   `tokio::time` / `rand::` / `getrandom` / `std::fs` / `std::env` /
   `std::net` / `HashMap::iter` / `HashMap::keys` / `HashMap::values` /
   `HashSet::iter` / `f32` / `f64` / `println!` / `eprintln!` / `dbg!` /
   `async fn` / `.await` in the target files. Any match is a failure.
2. **Producer-side `is_leader` is the validator's `is_leader`.** Grep
   `crates/ade_ledger/src/producer/forge.rs` for the substring
   `is_leader_for_vrf_output(` — must appear. Additionally grep ALL of
   `crates/ade_ledger/src/` and `crates/ade_core/src/` for any new
   `fn .*is_leader.*` definition other than the canonical one at
   `crates/ade_core/src/consensus/leader_schedule.rs:127` — any other
   definition is a failure.
3. **`forge_block` returns the closed sum.** Grep `forge.rs` for
   `pub fn forge_block` and check the return signature contains
   `Result<(ForgedBlock, Vec<ForgeEffects>), ForgeError>` as a substring.
4. **`ForgeError`, `ForgeEffects`, `ProducerTick`, `ForgedBlock` are
   closed sums.** Grep for `#[non_exhaustive]` preceding each
   `pub enum` / `pub struct` definition — any match is a failure.
5. **No private-key types reachable from `ProducerTick`.** Grep
   `crates/ade_ledger/src/producer/state.rs` for `VrfSigningKey`,
   `KesSecret`, `ColdSigningKey`, `KesSigningKey` — any match is a
   failure.
6. **No call to `cardano_crypto::vrf::VrfDraft03::prove` /
   `cardano_crypto::kes::KesAlgorithm::sign_kes` /
   `cardano_crypto::kes::KesAlgorithm::update_kes` in
   `crates/ade_ledger/src/producer/` or `crates/ade_core/src/`.**
7. **No `String`-bearing variant on `ForgeError` or `ForgeEffects`.**
   Grep their definitions for `: String` / `: alloc::string::String` —
   forbidden.

### 7. New CI gate `ci/ci_check_no_private_keys_in_corpus.sh`

Mechanical guards:

1. **No `*.skey` / `*.sk` / `*.signingkey` files under
   `crates/ade_testkit/fixtures/producer/`** (`find` glob).
2. **No `VrfSigningKey` / `KesSecret` / `KesSigningKey` /
   `ColdSigningKey` literal under
   `crates/ade_testkit/src/producer/fixtures.rs`** (grep).
3. **`ProducerTick` has no `Serialize` impl that mentions any
   private-key field name.** Grep
   `crates/ade_ledger/src/producer/state.rs` for `impl Serialize for
   ProducerTick`; if present, additionally grep for known private-key
   field names; finding any is a failure. (Currently no serde impl —
   the gate is the "stays this way" lock.)

### 8. Registry updates (same commit)

Flip to `enforced` with populated arrays:

- `DC-CONS-13` — `tests = ["forge_block_pure_no_io", "forge_block_replay_byte_identical"]`,
  `ci_script = "ci/ci_check_forge_purity.sh"`,
  `code_locus = "crates/ade_ledger/src/producer/forge.rs (forge_block); crates/ade_ledger/src/producer/state.rs (ProducerTick)"`.
- `DC-CONS-14` — `tests = ["forge_block_replay_byte_identical"]`,
  `ci_script = "ci/ci_check_forge_purity.sh, ci/ci_check_no_private_keys_in_corpus.sh"`,
  `code_locus = "crates/ade_ledger/src/producer/forge.rs; crates/ade_testkit/src/producer/replay.rs (producer_replay_fixtures)"`.
- `DC-CONS-15` — `tests = ["forge_block_rejects_non_leader_tick",
  "forge_block_uses_validator_leader_check_function"]`,
  `ci_script = "ci/ci_check_forge_purity.sh"`,
  `code_locus = "crates/ade_ledger/src/producer/forge.rs (leader-check gate); crates/ade_core/src/consensus/leader_schedule.rs (is_leader_for_vrf_output — shared with validator)"`.
- `DC-LEDGER-12` — `tests = ["forge_block_rejects_tx_not_in_mempool_accepted_prefix",
  "forge_block_rejects_tx_permuted_from_accumulating_order",
  "forge_block_empty_mempool_produces_empty_body",
  "admit_prefix_property_documented"]`,
  `ci_script = "ci/ci_check_forge_purity.sh"`,
  `code_locus = "crates/ade_ledger/src/producer/forge.rs (tx-admissibility prefix gate); crates/ade_ledger/src/mempool/admit.rs (admit — reused for prefix check)"`.

### 9. Cluster doc patch (same commit)

Amend `docs/clusters/PHASE4-N-C/cluster.md`:

- TCB Color Map: rename `ade_core::consensus::forge` →
  `ade_ledger::producer::forge`; `ade_core::consensus::producer_state` →
  `ade_ledger::producer::state`; `ade_core::consensus::self_accept` →
  `ade_ledger::producer::self_accept` (anticipated for S5).
- Grounding: append a one-line note recording the host-crate correction
  (link to this slice doc).

S4 and S5 slice docs will inherit the corrected host crate naturally
(they're written after S3 closes).

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_ledger producer::forge` green
  (5 unit tests).
- **AC-3** — `cargo test -p ade_ledger producer::state` green (any unit
  tests on `state.rs`; vacuous if none — but `-p ade_ledger` overall must
  still be green).
- **AC-4** — `cargo test -p ade_testkit producer::replay` green
  (2 replay tests over 3 fixtures).
- **AC-5** — `cargo test -p ade_codec shelley::tx_components` green
  (3 unit tests).
- **AC-6** — `cargo test -p ade_ledger mempool::admit::tests::admit_prefix_property_documented`
  green.
- **AC-7** — `cargo test --workspace` green. The pre-existing
  `boundary_fingerprint_matches_pins` failure is the only allowed
  pre-existing fail. Every `roundtrip_*` and `*_determinism` test must
  remain green.
- **AC-8** — `bash ci/ci_check_forge_purity.sh` returns `PASS` (all 7
  guards).
- **AC-9** — `bash ci/ci_check_no_private_keys_in_corpus.sh` returns
  `PASS` (all 3 guards).
- **AC-10** — `bash ci/ci_check_constitution_coverage.sh` returns `PASS`
  (registry edits round-trip).
- **AC-11** — `bash ci/ci_check_private_key_custody.sh` returns `PASS`
  (S1's gate unregressed).
- **AC-12** — `bash ci/ci_check_opcert_closed.sh` returns `PASS`
  (S2's gate unregressed).
- **AC-13** — `grep -r 'fn .*is_leader' crates/ade_core/src/ crates/ade_ledger/src/` returns at most one definition site
  (the canonical one in `leader_schedule.rs`).

---

## Hard Prohibitions

Cluster-level prohibitions inherited (cluster.md §Forbidden during this
cluster). Slice-specific additions:

- No `pub fn forge_block` outside
  `crates/ade_ledger/src/producer/forge.rs`.
- No `pub struct ProducerTick` outside
  `crates/ade_ledger/src/producer/state.rs`.
- No `#[non_exhaustive]` on `ProducerTick`, `ForgeError`, `ForgeEffects`,
  `ForgedBlock`, or `TxComponents`.
- No producer-side reimplementation of `is_leader` / `check_leader_claim`.
  Must call `ade_core::consensus::leader_schedule::is_leader_for_vrf_output`.
- No `std::time` / `rand` / `std::fs` / `std::env` / `std::net` /
  `HashMap` iteration / floating-point in `forge.rs`, `state.rs`, or
  `tx_components.rs`.
- No `String`-bearing variant on any closed sum introduced by this slice.
- No private-key types as fields of `ProducerTick`.
- No filesystem reads at forge time; corpus is `include_bytes!` /
  Rust source literals.
- No `*.skey` / `KesSecret` byte sequences anywhere under
  `crates/ade_testkit/fixtures/producer/` (the directory may not exist
  if fixtures are compiled-in via `fixtures.rs`; if it exists, the gate
  scans it).
- No call to `cardano_crypto::vrf::VrfDraft03::prove` /
  `kes::KesAlgorithm::sign_kes` / `update_kes` from `crates/ade_ledger/`
  or `crates/ade_core/`.
- No re-encoding of header CBOR bytes after `ShelleyHeader` is encoded
  into the block — body_hash is computed BEFORE header construction
  (preserves T-ENC-01).

---

## Explicit Non-Goals

- RED signing primitives — S1 (landed).
- BLUE `opcert_validate` — S2 (landed; consumed here).
- Body-hash *parity gate* (CI guard against parallel body encoders) —
  S4 (CE-N-C-4). S3 computes body_hash by the validator-shared recipe
  documented in §2 step 7; S4 adds the CI gate that locks "exactly one
  body encoder" and the explicit parity test.
- Self-acceptance gate — S5 (CE-N-C-5).
- Scheduler / tick-assembler / broadcast — S6 (CE-N-C-6).
- Cross-impl adapter + live-evidence binary — S7 (CE-N-C-7/8).
- Per-node persistent opcert-counter store — RED, S6.
- TPraos-era block assembly — non-goal per OQ-4. S3 targets Conway forge.
- Flipping `T-DET-01.strengthened_in` / `T-ENC-01.strengthened_in` —
  cluster-close.
- Resolving the OP-OPS-04 schema gap recorded in
  `OP-OPS-04.open_obligation` — cluster-close.

---

## Failure Modes

Every `ForgeError` variant is deterministic, structured, fail-fast at
the forge boundary. Replay equivalence requires byte-identical error
verdicts across runs — covered by `PartialEq` / `Debug` discipline on
every variant. No `String` / no path / no key bytes in any variant.

---

## Grounding (verified at HEAD `4cf4b65`)

- `ade_core::consensus::leader_schedule::is_leader_for_vrf_output`
  exists at `crates/ade_core/src/consensus/leader_schedule.rs:127`.
- `ade_core::consensus::opcert_validate::opcert_validate` exists at
  `crates/ade_core/src/consensus/opcert_validate.rs` (landed in S2).
- `ade_ledger::mempool::admit::admit` exists at
  `crates/ade_ledger/src/mempool/admit.rs:78`. `MempoolState` exposes
  `accepted()` (`Vec<Hash32>` ordered) and `accumulating()`
  (`&LedgerState`). No tx_bytes storage — `ProducerTick.mempool_tx_bytes`
  carries them explicitly.
- `ade_ledger::state::LedgerState` exists at
  `crates/ade_ledger/src/state.rs`. Used as the `base_state` field on
  `ProducerTick`.
- `ade_types::shelley::block::ShelleyBlock` at
  `crates/ade_types/src/shelley/block.rs:15` with the 4-bucket body
  shape; `ConwayBlock = ShelleyBlock` at
  `crates/ade_types/src/conway/mod.rs:16`.
- `ShelleyBlock::ade_encode` exists at
  `crates/ade_codec/src/shelley/block.rs:80`.
- Validator body-hash recipe at
  `crates/ade_ledger/src/block_validity/header_input.rs:151`
  (`block_body_hash`):
  ```
  body_hash = blake2b_256(
      blake2b_256(tx_bodies) || blake2b_256(witness_sets)
   || blake2b_256(metadata) || blake2b_256(invalid_txs_or_empty)
  )
  ```
  Each per-bucket hash is over the PRESERVED CBOR slice including its
  array/map header. `invalid_txs_or_empty` is `block.invalid_txs.as_deref().unwrap_or(&[])`.
- `crates/ade_ledger/Cargo.toml` declares `ade_core = { path = "../ade_core" }`
  (line 10). This is the structural fact that forces the host-crate
  correction.
- `ade_runtime::producer::signing::{vrf_prove, kes_sign}` exist (S1) —
  the regen helper uses them at fixture-generation time only; replay
  tests never invoke them.
- `ProtocolParameters` lives at... the implementer locates the correct
  path; existing era-specific protocol-params types are reachable from
  `ade_ledger`. If a Clone+Debug+PartialEq value is not available
  cleanly, the implementer surfaces the friction as a deviation rather
  than forking the type.

---

## Notes on body_hash and S4's complementarity

S3's forge produces `body_hash` via the recipe pinned in §2 step 7,
which mirrors the validator's `block_body_hash` byte-for-byte. The
mechanical proof that this is correct is "every existing
`roundtrip_*` / `*_determinism` test remains green" — those tests
round-trip block CBOR through the codec and recompute body_hash, so a
divergence would surface as a regression.

S4's slice will:
- explicitly add a parity test
  (`forged_body_hash_matches_validator_recomputation`) that asserts
  the validator's `block_body_hash(&forged.block) ==
  forged.block.header.body.body_hash`;
- add a CI gate (`ci_check_no_producer_body_encoder.sh`) that forbids
  any parallel body encoder definition;
- if helpful, lift any shared encoder code into a single named module.

S3 is where the property first holds; S4 is where it's mechanically
locked in.
