# Invariant Slice — PHASE4-N-H S2

## Slice Header

**Slice Name:** `receive_apply` reducer — pure, total BLUE bridge composing `block_validity` + header cross-check
**Cluster:** PHASE4-N-H
**Status:** In Progress
**CEs addressed:** CE-N-H-2
**Registry flips on merge:** `CN-CONS-08`, `DC-CONS-19` → `enforced`
**Dependencies:** N-H-S1

---

## Intent

Ship the single BLUE transition the receive bridge revolves around.
`receive_apply` consumes one `ReceiveEvent` at a time and:

* On `RollForward { slot, hash, header_bytes, .. }`: inserts the
  header bytes into `PendingHeaderCache` and returns
  `ReceiveEffect::Cached`. **No mutation** of `ledger`, `chain_dep`,
  or `chain_write` — Invariant I-6 enforced at the function shape.
* On `BlockDelivered { block_bytes }`: decodes the body, looks up
  the cached header at `(slot, block_hash)`, runs
  `admit_via_block_validity` (which composes `block_validity`),
  persists the resulting `AdmittedBlock` through the `ChainDbWrite`
  trait, then atomically commits the new `(ledger, chain_dep)` and
  evicts the consumed pending header. Header cross-check failure or
  validity failure leaves state unchanged.
* On `RollBackward { target_point, .. }`: returns
  `Err(ReceiveError::RollbackOutOfScope { target_point })` —
  Path A scope edge. State unchanged.

Reducer takes `state: &mut ReceiveState`; on error, state is
unchanged (staged-then-committed shape — all fallible operations
finish before any mutation).

---

## The change

### 1. New `ade_ledger::receive::reducer`

```rust
pub struct ReceiveState {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub pending_headers: PendingHeaderCache,
}

pub fn receive_apply<W: ChainDbWrite>(
    state: &mut ReceiveState,
    event: ReceiveEvent,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<ReceiveEffect, ReceiveError>;

pub fn receive_apply_sequence<W: ChainDbWrite>(
    state: &mut ReceiveState,
    events: impl IntoIterator<Item = ReceiveEvent>,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Vec<ReceiveEffect>, ReceiveError>;
```

### 2. Add `remove(slot, &hash)` to `PendingHeaderCache`

The reducer evicts the consumed header on successful admission. Per
the cluster doc, eviction below a slot is the orchestrator's job;
removing a single key on admission keeps the cache lean without
hiding state.

### 3. CI gate `ci/ci_check_receive_reducer_closure.sh`

* No `HashMap`/`HashSet`/`wall-clock`/`tokio`/`rand` in reducer
  production code.
* Positive grep for the `block_validity` call site (via
  `admit_via_block_validity`).
* Positive grep that `RollBackward` arm returns
  `RollbackOutOfScope`.
* Forbid any `pub fn` that mutates `state.ledger` or
  `state.chain_dep` from a `RollForward` branch (approximated by
  forbidding `state.ledger =` / `state.chain_dep =` in the
  RollForward arm via static layout convention).

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_ledger/src/receive/reducer.rs`:

- `receive_apply_roll_forward_caches_header_without_state_mutation`
  — admit a RollForward; assert pending_headers grew by 1 AND
  ledger fingerprint AND chain_dep AND chain_write call-count are
  unchanged.
- `receive_apply_block_delivered_with_matching_header_admits` —
  corpus path: insert header via RollForward, then BlockDelivered;
  assert `Admitted{slot,hash}`; ledger fingerprint changes; chain_dep
  evolves; pending_headers shrinks; chain_write recorded the bytes.
- `receive_apply_block_delivered_with_no_cached_header_rejects` —
  BlockDelivered without prior RollForward → `HeaderBodyMismatch`;
  state unchanged.
- `receive_apply_block_delivered_with_mismatched_cached_header_rejects`
  — cache (slotA, hashA, bytesA); deliver (slotA, hashB) →
  `HeaderBodyMismatch`; state unchanged.
- `receive_apply_block_delivered_validity_invalid_rejects` —
  corrupted body bytes (flipped) → `Err(Validity(BodyHashMismatch))`;
  state unchanged.
- `receive_apply_rollback_returns_out_of_scope` — state unchanged;
  `Err(RollbackOutOfScope { target_point })`.
- `receive_apply_replay_byte_identical_over_corpus` — drive corpus
  event sequence twice; assert identical final-state fingerprints.
- `receive_apply_sequence_admits_corpus_block` — single-block
  RollForward+BlockDelivered drive via the sequence helper.

CI: `ci/ci_check_receive_reducer_closure.sh` (new).

---

## §14 Hard Prohibitions

- No mutation of `state.ledger` / `state.chain_dep` /
  `chain_write` from the `RollForward` branch.
- No mutation of any sub-state on the `RollBackward` branch
  (Path A: `RollbackOutOfScope` is structural; state stays
  consistent).
- No partial mutation on validity failure — fallible ops finish
  before any commit; failure leaves state unchanged.
- No `unwrap` / `expect` / `panic!` in reducer production code.
- No `HashMap` / `HashSet` / `wall-clock` / `tokio` / `rand`.

---

## §15 Explicit Non-Goals

- GREEN adapter (S3); RED orchestrator (S4); mechanical adapter
  (S5); live evidence (S6); rollback (Path A scope edge).

---

## Replay obligations

`receive_apply_replay_byte_identical_over_corpus` proves the
reducer-level replay-equivalence. S3 covers the end-to-end transcript
replay over signal+event interleavings.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
