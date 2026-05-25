# Invariant Slice — PHASE4-N-I S3

## Slice Header

**Slice Name:** `commit_rollback` helper + ChainDbWrite trait extension + atomicity tests
**Cluster:** PHASE4-N-I
**Status:** In Progress
**CEs addressed:** CE-N-I-3 (atomicity half of DC-CONS-20; final flip in S6)
**Registry effects on merge:** none directly; foundation for DC-CONS-20 closure.
**Dependencies:** N-I-S1, N-I-S2

---

## Intent

The atomic state-replacement helper the S6 reducer wires into.
Takes the materialized `(new_ledger, new_chain_dep)` from S2 and:

1. Calls `chain_write.rollback_to_slot(target.slot)` first
   (irreversible step). If fails → return `Err(ChainDb(_))`;
   receive state unchanged.
2. Replaces `state.ledger`, `state.chain_dep` (infallible
   assignments).
3. Resets `state.pending_headers` (post-rollback the cached
   headers are stale — the new chain may not include them).

The trait extension: `ChainDbWrite` gains
`rollback_to_slot(slot)`. N-H's existing N-G in-memory impl gains
the new method (trivial — delegates to `ChainDb::rollback_to_slot`).
Mock impls in N-H tests gain a default `Ok(())` or no-op.

---

## The change

### 1. Extend `ChainDbWrite` trait

```rust
pub trait ChainDbWrite {
    fn write_admitted(&mut self, block: AdmittedBlock) -> Result<(), ChainWriteError>;
    /// Roll the chain store back to `slot`, discarding all blocks
    /// at slots strictly greater than `slot`.
    fn rollback_to_slot(&mut self, slot: SlotNo) -> Result<(), ChainWriteError>;
}
```

### 2. Update existing impls

- `ade_runtime::receive::in_memory_chain_write::ChainDbWriter` —
  delegate to `self.db.rollback_to_slot(slot)`.
- Mock impls in `ade_ledger::receive::{chain_write, reducer}::tests`
  — implement `rollback_to_slot` as `Ok(())` no-op (tests don't
  exercise it).

### 3. New `ade_ledger::rollback::commit::commit_rollback`

```rust
pub fn commit_rollback<W: ChainDbWrite>(
    state: &mut ReceiveState,
    target: TargetPoint,
    new_ledger: LedgerState,
    new_chain_dep: PraosChainDepState,
    chain_write: &mut W,
) -> Result<(), CommitRollbackError>;
```

Sequence: chain_write rollback first (irreversible); on success,
replace state.ledger + state.chain_dep + state.pending_headers =
PendingHeaderCache::new().

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_ledger/src/rollback/commit.rs`:

- `commit_rollback_advances_chaindb_and_ledger_atomically` —
  after Ok return, state.ledger fingerprint != pre-commit fingerprint,
  state.chain_dep equals new_chain_dep, pending_headers is empty,
  chain_write recorded the rollback call.
- `commit_rollback_chain_write_failure_leaves_state_unchanged` —
  failing chain_write impl → Err(ChainDb(_)); state fields
  (ledger, chain_dep, pending_headers) unchanged.
- `commit_rollback_resets_pending_headers` — pre-commit cache
  has N entries; post-commit cache has 0.

---

## §14 Hard Prohibitions

- No state mutation if chain_write rollback fails.
- No partial pending_header reset — full reset only.
- No HashMap / wall-clock / tokio / rand in commit module.

---

## §15 Explicit Non-Goals

- GREEN cadence/cache (S4); orchestrator (S5); reducer wire-up
  (S6).

---

## Replay obligations

Atomicity tests pin the staged-then-committed shape. S6's
integration test proves end-to-end snapshot-is-cache.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
