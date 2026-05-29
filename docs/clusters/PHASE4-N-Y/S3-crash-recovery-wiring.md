# Invariant Slice — S3: End-to-end crash recovery wiring

## §2 Slice Header

- **Slice Name:** Node-binary crash recovery — restart reconstructs byte-identical state from {anchor + preserved bytes + WAL + latest checkpoint + forward replay}, no operator repair.
- **Cluster:** PHASE4-N-Y.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed** (verbatim):
  - [ ] **CE-Y-8.** Crash at any phase (import/sync/admit/checkpoint) recovers to byte-identical state, no operator step; tests `recovery_crash_at_phase_{import,sync,admit,checkpoint}_byte_identical` pass.
  - [ ] **CE-Y-9.** `replay_from_anchor_two_runs_byte_identical` + `bootstrap_warm_start_equals_direct_materialize` still pass (carry-forward).
  - [ ] **CE-Y-15** *(partial):* `DC-STORE-01`/`DC-STORE-03`/`DC-WAL-01`..`DC-WAL-03`/`T-DET-01` `strengthened_in += "PHASE4-N-Y"`.
- **Slice Dependencies:** S2 (consumes the durable preserved-bytes + WAL + checkpoint writes S2 produces).

## §3 Implementation Instruction (AI)

Wire the **existing** `ade_runtime::recovery` (snapshot + forward-replay, S-36) + `bootstrap_initial_state` warm-start branch into the node binary's startup path so an unclean restart self-recovers. Do not write a second recovery engine. The recovered fingerprint must equal a clean run's. §12 is the contract.

## §4 Intent

Make it impossible for an unclean restart to produce a state that differs from a clean run, or to require operator repair — recovery is {latest valid checkpoint + forward replay over preserved bytes + WAL} via the single existing authority.

## §5 Scope

- **GREEN/RED (wiring):** node-binary startup routes through `bootstrap_initial_state` (warm-start: `None` genesis + non-empty store → recover at largest stored slot) + `ade_runtime::recovery::recover` (`Recoverable` trait) + `wal::replay_from_anchor`.
- **BLUE (reused):** `replay_from_anchor`, `snapshot` decode, `fingerprint`, `block_validity` (replay re-validates).
- **Persistence:** read-path only (recovery); no new on-disk shape. Uses `rollback::{persistent_cache, persistent_writer, snapshot_writer}` + `FileWalStore`.
- **Out of scope:** the durable *write* path (S2), genesis source (S4), evidence (S5).

## §6 Execution Boundary (TCB color)

- **BLUE:** `wal::replay_from_anchor`, `snapshot::decode_*`, `fingerprint`, `block_validity` (reused).
- **GREEN:** `ade_runtime::recovery` (`Recoverable` composition — deterministic; reused), `bootstrap_initial_state` (reused).
- **RED:** node-binary restart driver, redb reads, crash-injection test scaffolding.

Color resolved (all reused; no new BLUE authority).

## §7 Invariants Preserved

[[T-DET-01]], [[DC-STORE-01]] (the law being strengthened), [[DC-STORE-05]] (snapshot + forward replay, not genesis replay), [[CN-WAL-01]]/[[DC-WAL-01]]..[[DC-WAL-03]], [[CN-ANCHOR-01]] (recovery binds to the same anchor lineage), [[DC-STORE-02]]/[[DC-STORE-03]], `ci_check_persistent_writer_no_parallel_cadence.sh`, `ci_check_snapshot_cadence_purity.sh`.

## §8 Invariants Strengthened or Introduced

**One family — power-loss recovery determinism.** No new rule; this slice **strengthens** [[DC-STORE-01]] ("recovery from power-loss produces replay-equivalent state") and [[T-DET-01]] by adding the node-binary crash-at-each-phase byte-identity tests, and [[DC-STORE-03]]/[[DC-WAL-01]]..[[DC-WAL-03]] by exercising atomic-checkpoint + WAL recovery end-to-end. *(No `introduces` — the laws exist; this slice makes their enforcement mechanical at the binary level.)*

## §9 Design Summary

Restart path: open persistent ChainDb + SnapshotStore + WAL → `bootstrap_initial_state` warm-start materializes from the latest valid checkpoint → `recovery::recover` replays forward over preserved bytes (re-validating via `block_validity`) to the stored tip → compare recovered `fingerprint` to the pre-crash clean-run fingerprint. Atomicity of checkpoints (`DC-STORE-03`, fully-written-or-absent) means a crash mid-checkpoint recovers to the prior valid checkpoint.

## §10 Changes Introduced

- **Types:** none new (reuse `RecoveryOutcome`, `RecoveryError`).
- **State transitions:** none new (recovery composition).
- **Persistence:** none new; read-path wiring.
- **Removal/refactors:** none.

## §11 Replay / Crash / Epoch Validation

- **Crash/restart:** `recovery_crash_at_phase_{import,sync,admit,checkpoint}_byte_identical` — inject a crash at each phase (truncate WAL / kill before checkpoint commit / partial block write), restart, assert recovered fingerprint == clean-run fingerprint and `blocks_replayed` accounts for the gap.
- **Replay:** `replay_from_anchor_two_runs_byte_identical` (carry-forward), `bootstrap_warm_start_equals_direct_materialize` (carry-forward).
- **Epoch:** n/a.

## §12 Mechanical Acceptance Criteria

- [ ] `recovery_crash_at_phase_import_byte_identical`
- [ ] `recovery_crash_at_phase_sync_byte_identical`
- [ ] `recovery_crash_at_phase_admit_byte_identical`
- [ ] `recovery_crash_at_phase_checkpoint_byte_identical` (crash mid-checkpoint → recover to prior valid checkpoint; `DC-STORE-03`).
- [ ] `replay_from_anchor_two_runs_byte_identical` + `bootstrap_warm_start_equals_direct_materialize` still pass.
- [ ] `cargo test --workspace` clean; `ci_check_persistent_writer_no_parallel_cadence.sh`, `ci_check_snapshot_cadence_purity.sh`, `ci_check_wal_append_only.sh` pass.

## §13 Failure Modes

Unrecoverable corruption (WAL CRC fail / missing block bytes for a WAL-referenced hash) → fail-fast deterministic `RecoveryError` (`BlockBytesMissing`/`CorruptCrc`), no silent partial recovery. A checkpoint that is partially written is treated as absent (`DC-STORE-03`), recovery falls back to the prior valid one. All deterministic; replay-safe.

## §14 Hard Prohibitions

**Inherited (cluster §7).** **Slice-specific:** no second recovery engine; no genesis-replay-from-scratch as the recovery path (must be snapshot + forward replay, `DC-STORE-05`); no operator-repair step; no nondeterminism in the replay path; no swallowing a corrupt-WAL/missing-bytes error (must fail-fast).

## §15 Explicit Non-Goals

No durable write path (S2), no genesis source (S4), no evidence harness (S5), no new storage shape, no cross-epoch handling.

## §16 Completion Checklist

- [ ] Recovery is replay-derivable from anchor+bytes+WAL+checkpoint; failures deterministic; no operator step; CI exercises crash-at-each-phase; recovered == clean byte-identical.

## §17 Review Notes

Risk: a crash window where the tip advanced but WAL/bytes weren't durable → covered by S2's ordering invariant (DC-SYNC-01); S3 assumes S2's durability ordering holds. Risk: nondeterministic replay → reuses the proven `replay_from_anchor` determinism.

## §18 Authority Reminder

Planning aid only; registry + CI authoritative.
