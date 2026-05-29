# Invariant Slice — S6: Torn-write recovery reconciliation (HIGH remediation)

> Remediates the cluster-close security BLOCK #1. S2/S3 left a torn-write window:
> `tip = put_block` (no separate pointer) and the durable writes apply
> `StoreBlockBytes` before `AppendWal`, so a crash between them leaves an orphan
> block durable in the chaindb but absent from the WAL — and recovery
> (`bootstrap_initial_state` warm-start from `chaindb.tip()`) silently incorporates
> it, yielding a state that is NOT byte-identical to a clean run. The WAL is the
> admission authority; recovery must target the WAL tail.

## §2 Slice Header

- **Slice Name:** Recovery reconciles chaindb to the WAL tail — orphan blocks beyond the last WAL entry are never incorporated.
- **Cluster:** PHASE4-N-Y.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** strengthens CE-Y-6, CE-Y-8 (closes the torn-write gap behind DC-SYNC-01 / DC-STORE-01). No new CE.
- **Slice Dependencies:** S2, S3 (merged).

## §4 Intent

Make it impossible for recovery to incorporate a block that is not in the WAL: the recovered authoritative state MUST equal the WAL-tail post-fp that `replay_from_anchor` already computes, regardless of any orphan block left durable in the chaindb by a torn `put_block`/`wal.append` crash. The WAL is the single admission authority; the chaindb is reconciled to it, no operator repair.

## §5 Scope

- **RED/GREEN (wiring):** `ade_runtime::recovery::restart::recover_node_state` — after the BLUE `replay_from_anchor` integrity gate returns the WAL-tail post-fp, reconcile the chaindb to the WAL tail (drop/ignore blocks beyond the WAL-tail slot) before/within the warm-start materialize, then assert the recovered fingerprint equals the WAL-tail post-fp.
- **BLUE (reused):** `replay_from_anchor` (already returns the WAL-tail `Hash32`), `fingerprint`, `bootstrap_initial_state`, the existing `ChainDb` rollback/truncation primitive (`rollback_to_slot` or equivalent).
- **Out of scope:** the durable-write *ordering* in S2's pump (recovery-side reconciliation is the chosen fix per the security review; reordering the write path is not required and not done here), any new WAL entry shape.

## §6 Execution Boundary (TCB color)

- **BLUE:** `replay_from_anchor`, `fingerprint` (reused).
- **GREEN/RED:** `recover_node_state` reconciliation + the chaindb truncation call (RED shell, redb).

No new BLUE authority. Reconciliation is deterministic (drop blocks with slot > WAL-tail slot).

## §7 Invariants Preserved

[[DC-STORE-01]], [[DC-STORE-05]], [[T-DET-01]], [[CN-WAL-01]]/[[DC-WAL-01]]..[[DC-WAL-03]], [[CN-ANCHOR-01]], and the existing recovery tests (`recovery_crash_at_phase_*_byte_identical`, `replay_from_anchor_two_runs_byte_identical`, `bootstrap_warm_start_equals_direct_materialize`) — all must still pass.

## §8 Invariants Strengthened or Introduced

**One family — recovery determinism.** Strengthens [[DC-STORE-01]] + [[T-DET-01]]: recovery is now byte-identical to a clean run **even across a torn `put_block`/`wal.append` crash**. No new registry rule; this closes the gap that blocked DC-SYNC-01's strong enforcement.

## §9 Design Summary

`replay_from_anchor` already returns the WAL-tail post-fp. `recover_node_state` derives the WAL-tail point from the last WAL entry (`slot`, `block_hash`). If `chaindb.tip()` is beyond the WAL tail (a stored block whose slot exceeds the WAL-tail slot, or a tip hash that is not the WAL-tail hash), the chaindb is reconciled to the WAL-tail slot (truncate orphans) before warm-start. After materialize, recovery asserts `recovered_fp == wal_tail_fp`; a residual mismatch is a fail-fast `NodeRecoveryError` (never a silent divergence).

## §10 Changes Introduced

- **Types:** possibly a new closed `NodeRecoveryError` variant (e.g. `WalTailFingerprintMismatch`) for the post-reconciliation guard.
- **State transitions:** recovery reconciliation step (deterministic truncation to WAL-tail slot).
- **Persistence:** read + truncate (existing `rollback_to_slot`); no new shape.

## §11 Replay / Crash / Epoch Validation

- **Crash/restart:** new `recovery_torn_put_block_before_wal_append_drops_orphan` — store a block's bytes (advancing the chaindb tip) but DO NOT append its WAL entry, crash, restart; assert recovered state == the WAL-tail state (orphan dropped) and byte-identical to the clean run that never stored the orphan.
- The four existing `recovery_crash_at_phase_*_byte_identical` + carry-forward tests still pass.

## §12 Mechanical Acceptance Criteria

- [ ] `recovery_torn_put_block_before_wal_append_drops_orphan` — orphan block (bytes durable, no WAL entry) is NOT in the recovered state; recovered_fp == WAL-tail post-fp == clean-run fp.
- [ ] `recover_node_state` asserts `recovered_fp == wal_tail_fp` (the value `replay_from_anchor` returns) — a mismatch is fail-fast, not silent.
- [ ] All four `recovery_crash_at_phase_*_byte_identical` + `replay_from_anchor_two_runs_byte_identical` + `bootstrap_warm_start_equals_direct_materialize` still pass.
- [ ] `cargo test -p ade_runtime recovery` + `-p ade_ledger replay_from_anchor` clean; `ci_check_persistent_writer_no_parallel_cadence.sh`, `ci_check_snapshot_cadence_purity.sh`, `ci_check_wal_append_only.sh` pass.

## §13 Failure Modes

Orphan beyond WAL tail → reconciled away (dropped), deterministic, no operator step. Post-reconciliation fingerprint mismatch (should be unreachable after reconciliation) → fail-fast `NodeRecoveryError`. Corrupt-WAL / missing-bytes → existing fail-fast (`BlockBytesMissing`/`CorruptCrc`) unchanged.

## §14 Hard Prohibitions

**Inherited (cluster §7).** **Slice-specific:** recovery MUST NOT incorporate a block absent from the WAL; MUST NOT require operator repair; reconciliation MUST be deterministic; no second recovery engine; no nondeterminism in the replay path; the WAL — not the chaindb tip — is the recovery authority.

## §15 Explicit Non-Goals

No change to S2's pump write-ordering; no new WAL entry shape; no genesis/evidence work; no performance tuning.

## §16 Completion Checklist

- [ ] Torn-write orphan dropped; recovered == WAL-tail == clean byte-identical; guard is fail-fast; existing recovery tests green.

## §18 Authority Reminder

Planning aid only; registry + CI authoritative.
