# Ledger snapshot + replay-forward rollback — Invariants sketch

**Concept.** Close PHASE4-N-H's deferred DC-CONS-20 rollback-side
half. When the receive bridge receives `RollBackward(target_point)`,
materialize the rolled-back `(LedgerState, PraosChainDepState)` and
commit it atomically with `ChainDb::rollback_to_slot`. Strategy:
**snapshot-and-replay-forward**. Periodically encode
`(LedgerState, PraosChainDepState)` into bytes, persist via
`SnapshotStore`. On rollback: find nearest snapshot ≤ target,
decode, replay forward via `apply_block_with_verdicts` over the
ChainDb's block range up to target.

The key authority claim: **rollback materialization is a pure cache
over canonical history, not a second ledger evolution path.**

Cluster-id candidate: **PHASE4-N-I**.

**Status.** Invariants sketch only. Cluster plan + slice docs
follow. Not yet implementation-bound.

## Scope decisions (locked before this sketch)

1. **Cadence is BLUE-structural for PHASE4-N-I.** Default: every
   `N` blocks (proposed N=100). Operator-tunable cadence is
   explicitly **out of scope** unless later represented as
   anchored, replay-derivable runtime data. If two executions admit
   the same blocks but take snapshots at different slots because of
   operator config, telemetry, wall-clock pressure, or restart
   timing, the snapshot set ceases to be replay-equivalent
   evidence. Operator tunability is a follow-on Tier-5 surface if
   telemetry demands it.
2. **Combined snapshot.** One `SnapshotStore` entry per slot covers
   BOTH `LedgerState` AND `PraosChainDepState`. Single I/O round
   trip per snapshot; replay-equivalence checks simpler.
3. **No genesis-replay fallback.** If no snapshot ≤ target exists,
   return `RollbackTooDeep { target, oldest_snapshot }`. Receive
   state unchanged; orchestrator halts the peer. A genesis-replay
   fallback would create a second materialization regime with
   different runtime behavior, validation surface, and failure
   modes — weakening the single-authority claim. Cardano-node's
   actual recovery shape IS snapshot + forward replay; we follow
   the same shape.
4. **Conway-only encoder scope — limited closure.** This cluster
   ships rollback materialization **only for histories whose
   snapshot and replay-forward range are Conway-supported under the
   current ledger implementation**. Pre-Conway eras return a
   structural `EraNotSupported { era }` from both encode and
   materialize. This does NOT close all-era Cardano recovery; the
   public challenge eventually requires the full audited protocol
   surface. Pre-Conway snapshot support is a future cluster.
5. **Cluster ends by closing DC-CONS-20.** The final slice updates
   N-H S2's receive reducer: `RollBackward` branch replaces
   `Err(RollbackOutOfScope)` with actual rollback via the new
   materialize driver. **DC-CONS-20 may flip to `enforced` only
   when the receive reducer's RollBackward branch no longer
   returns `RollbackOutOfScope`, and the branch atomically commits
   ChainDb rollback, ledger replacement, PraosChainDepState
   replacement, pending-header reset, and ChainSelectorState
   rollback.** This is the explicit closure criterion — recorded
   here and to be carried verbatim into the cluster doc's final-
   slice CE.

## 1. What must always be true

- **I-1 — Encode/decode round-trip.** `decode(encode(state)) ==
  state` for any reachable `(LedgerState, PraosChainDepState)`,
  measured via `ade_ledger::fingerprint::fingerprint(decoded) ==
  fingerprint(original)`. The fingerprint function already walks
  every field deterministically and is the natural state-
  equivalence witness.
- **I-2 — Encode determinism.** `encode(state) == encode(state)`
  byte-identical across runs. Encoder uses `BTreeMap` iteration
  only; no `HashMap`, no floats, no wall-clock.
- **I-3 — Replay-forward correctness.** Given `state_at_slot_S`
  and the ordered block sequence `blocks(S+1..=T)` from ChainDb,
  replay-forward yields a state byte-equal (by fingerprint) to the
  state that would result from applying those blocks via
  `apply_block_with_verdicts` in normal forward operation.
- **I-4 — Snapshot-then-replay equivalence.** For any reachable
  target T and any snapshot slot S ≤ T,
  `(decode(snapshot@S) → replay-forward to T)` yields a state
  byte-equal to a snapshot taken directly at T. **Snapshotting is
  a pure cache; never an authoritative side path.**
- **I-5 — Single materialize authority.** The function that
  materializes `(LedgerState, PraosChainDepState)` at a target
  point uses ONLY: (a) one `SnapshotStore` lookup, (b)
  `ChainDb::iter_from_slot` for the replay-forward block sequence,
  (c) `apply_block_with_verdicts` (+ `apply_epoch_boundary` when
  crossing boundaries). No bypass; no parallel ledger-evolution
  path. *(Receive-side analog of CN-CONS-08.)*
- **I-6 — Rollback atomicity.** Once materialized state is
  computed, the receive reducer commits
  `ChainDb::rollback_to_slot(target_slot)` AND state replacement
  (ledger, chain_dep, pending_headers reset) AND `apply_rollback`
  on `ChainSelectorState` as **one structural transition**.
  Partial rollback unrepresentable. *(Closes DC-CONS-20 rollback-
  side fully — see closure criterion in scope decision #5.)*
- **I-7 — Fail-closed on missing snapshot.** If no snapshot ≤
  target exists, return `RollbackTooDeep { target,
  oldest_snapshot }`. Receive state unchanged; orchestrator halts
  the peer pipeline.
- **I-8 — Snapshot cadence determinism.** The decision to take a
  snapshot at slot S is a pure function of `(slot, block_no,
  cadence_params, last_snapshot)`. Same canonical inputs → same
  set of snapshot slots. Two runs admit the same blocks → same
  snapshot bytes at the same slot keys.
- **I-9 — Snapshot bytes carry version + fingerprint.** Encoded
  bytes start with a closed version tag (`u32`) and embed the
  source state's `LedgerFingerprint.combined`. Decode validates
  both before producing a state. Future schema changes bump the
  version; unknown version = structured decode error.

## 2. What must never be possible

- **¬P-1.** Decoding bytes that weren't produced by `encode` and
  obtaining a `LedgerState` with semantically arbitrary contents.
  (Version tag + fingerprint cross-check rejects garbage.)
- **¬P-2.** Replay-forward silently advancing past a block that
  `block_validity` would reject. (Fail-closed: any rejection halts
  with `ReplayFailedAt { slot, error }`.)
- **¬P-3.** Two `encode(state_T)` invocations producing different
  bytes across runs that reached the same `state_T`.
- **¬P-4.** Materializing rolled-back state via a path other than
  the canonical snapshot + replay-forward driver. (CI gate +
  trait surface centralization.)
- **¬P-5.** Rollback that updates ChainDb but not LedgerState (or
  vice versa). (Same atomicity shape as N-H S2's admit branch.)
- **¬P-6.** Snapshot cadence depending on wall-clock, RNG, or
  operator timing input.
- **¬P-7.** Two encode calls for the same `(LedgerState,
  PraosChainDepState)` at the same slot producing different bytes.
- **¬P-8.** Snapshot stored without its source state's
  fingerprint embedded.
- **¬P-9.** Replay-forward omitting an epoch boundary transition
  for blocks that cross one. (See blocking design question §7 #1.)

## 3. What must remain identical across executions

- **Snapshot bytes** for any reachable state. `encode(state_T)`
  must be byte-identical across runs.
- **Rolled-back state** for any reachable `target_point`. Two
  rollbacks from the same chain history to the same target produce
  byte-identical materialized states (by fingerprint).
- **Snapshot slot set** for any reachable chain history. Same
  input block sequence → same set of snapshot slots persisted.

## 4. What must be replay-equivalent

For a synthetic fixture corpus of
`(initial_state, ordered_block_sequence, rollback_target) →
expected_rolled_back_state`:

- Two runs with identical inputs MUST produce byte-identical
  materialized rolled-back state (by fingerprint).
- The materialized rolled-back state MUST equal the state obtained
  by applying `initial_state + blocks_up_to_target` directly
  forward (no snapshot path) — i.e. snapshot+replay-forward is a
  pure cache, not an authoritative side path.
- Snapshot bytes round-trip: `fingerprint(decode(encode(state)))
  == fingerprint(state)`.

## 5. State transitions in scope

```text
// BLUE — snapshot encode/decode
fn encode_snapshot(
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
) -> Result<Vec<u8>, SnapshotEncodeError>

fn decode_snapshot(
    bytes: &[u8],
) -> Result<(LedgerState, PraosChainDepState), SnapshotDecodeError>

// BLUE — replay-forward driver (pure, takes read-only traits)
fn materialize_rolled_back_state(
    target: TargetPoint,                   // slot + hash
    snapshot_store: &dyn SnapshotReader,   // BLUE read-only trait
    block_source: &dyn BlockSource,        // BLUE read-only trait
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<(LedgerState, PraosChainDepState), MaterializeError>

// BLUE — rollback commit (called from receive reducer S2 update)
fn commit_rollback<W: ChainDbWrite>(
    state: &mut ReceiveState,
    target: TargetPoint,
    new_ledger: LedgerState,
    new_chain_dep: PraosChainDepState,
    chain_write: &mut W,
) -> Result<(), CommitRollbackError>

// GREEN — cadence policy (pure)
fn should_snapshot_after_block(
    slot: SlotNo,
    block_no: BlockNo,
    cadence: SnapshotCadence,           // BLUE-structural params
    last_snapshot: Option<SlotNo>,
) -> bool
```

All four BLUE functions are pure, total, deterministic. Errors are
structured closed sums.

## 6. TCB color hypothesis

- **BLUE (new):**
  - `ade_ledger::snapshot::encode` / `decode` — canonical
    `(LedgerState, PraosChainDepState)` codec. Walks every field
    deterministically (mirrors `fingerprint.rs`'s structure but
    emits bytes via `ade_codec::cbor::*` instead of feeding a
    hash).
  - `ade_ledger::rollback::materialize` — pure replay-forward
    driver composing snapshot decode + block iteration + apply.
  - `ade_ledger::rollback::commit` — atomic state replacement.
  - `SnapshotReader` / `BlockSource` narrow read-only traits.
  - Closed sums: `SnapshotEncodeError`, `SnapshotDecodeError`,
    `MaterializeError` (variants `RollbackTooDeep`,
    `ReplayFailedAt`, `SnapshotDecode`, `EraNotSupported`),
    `CommitRollbackError`.
- **GREEN (new):**
  - `should_snapshot_after_block` — pure cadence decision.
  - `SnapshotReader` + `BlockSource` adapter impls over
    `ade_runtime::chaindb::{SnapshotStore, ChainDb}`.
  - Replay test scaffolding.
- **RED (modified):**
  - `ade_ledger::receive::reducer` (extended) — `RollBackward`
    branch now calls `materialize_rolled_back_state` +
    `commit_rollback` instead of returning `RollbackOutOfScope`.
  - New RED helper: snapshot-write orchestration after each
    successful admission, gated by the GREEN cadence policy.

## 7. Open questions

1. **BLOCKING DESIGN QUESTION (pre-S1).** Identify whether
   `apply_block_with_verdicts` already performs all required
   epoch-boundary transitions for replay-forward. If yes,
   materialization is a simple fold. If no, the materializer must
   explicitly invoke the unique epoch-boundary authority
   (`apply_epoch_boundary` / `rotate_snapshots` etc.). **No
   duplicate or partial epoch transition path may be introduced.**
   The dangerous failure is subtle: an omitted epoch boundary
   would make snapshot materialization agree locally for intra-
   epoch rollback but diverge across epoch boundaries — a
   replay-equivalence violation that's hard to detect without
   cross-epoch corpora. Must be resolved before S1 implementation
   begins.
2. **Snapshot cadence default value.** Proposed N=100 blocks.
   Worst-case replay-forward = N-1 blocks. Pin at cluster-plan.
3. **Snapshot encoder version policy.** `u32` version tag at
   start. Initial version = 1. Decoder rejects unknown versions
   with `SnapshotDecodeError::UnknownVersion`.
4. **Snapshot eviction policy.** Keep every snapshot newer than
   `immutable_tip - k`; evict older. Operator force-evict via
   tooling. Lean: ship basic eviction in this cluster as a Tier-5-
   style operator-tunable, but the BLUE eviction *decision*
   function stays deterministic given inputs.
5. **Replay-forward determinism vs LedgerView dependency.**
   `apply_block_with_verdicts` takes `&dyn LedgerView`. Verify
   existing `LedgerView` impls are deterministic over the same
   `(snapshot_slot, target_slot)` inputs (no peer queries / no
   non-deterministic data sources). Blocker if any impl violates.
6. **PraosChainDepState encoding scope.** Verify no hidden
   non-deterministic fields (`op_cert_counters` is already
   BTreeMap-backed per the survey). Mechanical.
7. **RollBackward sequencing.** Two consecutive rollbacks: the
   second treats the post-rollback state as canonical. Trivial; no
   special handling needed. Confirm in S6 integration test.

## 8. Acceptance evidence shape

Mechanical CEs prove the four BLUE pieces + replay-equivalence
over a synthetic fixture corpus (encode-then-decode round trip;
snapshot-then-replay-forward equivalence with direct-apply;
materialize-rolled-back-state correctness; commit atomicity).

**DC-CONS-20 closes fully when the receive reducer's RollBackward
branch goes from `RollbackOutOfScope` to actual rollback AND the
branch atomically commits ChainDb rollback + ledger replacement +
PraosChainDepState replacement + pending-header reset +
ChainSelectorState rollback.** This is the cluster's final slice
acceptance criterion, carried verbatim into the cluster doc.

No new RO-LIVE entry — the live evidence concerns peer interaction
already covered by RO-LIVE-01 / RO-LIVE-02.

---

## Proposed registry entries (4 new + 1 in-place close, to be confirmed)

```toml
[[rules]]
id = "DC-CONS-21"
tier = "derived"
statement = """
Snapshot encode/decode round-trip equivalence: for any reachable
(LedgerState, PraosChainDepState), decode(encode(state)) yields a
state whose ade_ledger::fingerprint::fingerprint matches the
original's. Encoder is canonical (BTreeMap iteration, no HashMap,
no floats, no wall-clock); encoded bytes start with a closed
version tag and embed the source state's fingerprint for decode-
side cross-check.
"""
source = "docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-1, I-2, I-9)"
cross_ref = ["T-ENC-01", "T-DET-01", "DC-CONS-22"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "DC-CONS-22"
tier = "derived"
statement = """
Replay-forward correctness: given state_at_slot_S and the ordered
block sequence blocks(S+1..=T) from ChainDb, the replay-forward
driver yields a state whose fingerprint matches the state that
would result from applying those blocks via
apply_block_with_verdicts in normal forward operation. Snapshot+
replay-forward is a pure cache for direct-apply; never an
authoritative side path. Replay-forward MUST honor the unique
epoch-boundary authority for any range crossing one (¬P-9).
"""
source = "docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-3, I-4)"
cross_ref = ["T-DET-01", "DC-CONS-21", "CN-STORE-07"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "CN-STORE-07"
tier = "release"
statement = """
Single materialize authority for rolled-back state: the function
that materializes (LedgerState, PraosChainDepState) at a target
point uses ONLY one SnapshotStore lookup + ChainDb::iter_from_slot
+ apply_block_with_verdicts (+ apply_epoch_boundary when crossing).
No bypass; no parallel rolled-back-state computation path. Mirror
of CN-CONS-08 (admission gate) for the rollback path. Single-
public-function discipline; type-level + CI grep enforcement.
"""
source = "docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-5)"
cross_ref = ["CN-CONS-08", "DC-CONS-20", "DC-CONS-22"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
attack_rationale = "A parallel materializer can produce a rolled-back state inconsistent with what block_validity would compute, allowing peer-controlled state divergence."
evidence_notes = "Enforcement is type-level: materialize_rolled_back_state is the sole pub fn returning the rolled-back state tuple. The function takes narrow read-only traits (SnapshotReader, BlockSource), not concrete chain stores — test-side and production-side go through the same single composition."

[[rules]]
id = "DC-STORE-07"
tier = "derived"
statement = """
Snapshot cadence determinism: the decision to take a snapshot at
slot S is a pure function of (slot, block_no, cadence_params,
last_snapshot). Same canonical input chain history produces the
same set of snapshot slot keys. Cadence is BLUE-structural; not
operator-tunable in this cluster (operator-tunable cadence is
out of scope until represented as anchored, replay-derivable
runtime data).
"""
source = "docs/planning/ledger-snapshot-rollback-invariants.md §1 (I-8)"
cross_ref = ["T-DET-01", "DC-PROTO-09"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
```

And the **DC-CONS-20 closure** (NOT a new rule; updated in-place
on the cluster's final slice — flip `status` from `declared` →
`enforced`, populate code/tests/ci, remove `open_obligation`):

```toml
# update in-place on cluster close:
[[rules]]
id = "DC-CONS-20"
# ... statement / source / cross_ref unchanged ...
code_locus = "<final-slice receive-reducer + rollback driver + commit helper>"
tests = ["<rollback-driver tests>", "<receive-reducer rollback-branch tests>", "<commit-atomicity tests>"]
ci_script = "<rollback-closure CI + receive-reducer-closure>"
status = "enforced"
# REMOVE: open_obligation = "rollback_side_blocked_until_ledger_snapshot_cluster"
```

## Existing rules this cluster will eventually strengthen

(`strengthened_in` appends recorded at `/cluster-doc` time.)

- `T-DET-01` — new authoritative-deterministic surface (snapshot
  bytes + materialized rolled-back state).
- `T-ENC-01` — canonical snapshot encoding (new hash-critical byte
  path).
- `CN-CONS-08` — receive admission authority's mirror property
  (rollback is the inverse transition; same single-authority
  discipline).
- `DC-PROTO-09` — receive transcript determinism extended to
  include the rollback transition.

## Related

- [[project-phase4-n-h-handoff]] — the deferring cluster;
  DC-CONS-20's `open_obligation` names this cluster.
- [[project-phase4-n-g-handoff]] — send-side mirror.
- [[feedback-fail-closed-validation]] — ¬P-2 + I-7 enforcement
  shape.
- [[feedback-diverge-on-internal-surfaces]] — snapshot encoding is
  permitted internal divergence (Tier 5).
