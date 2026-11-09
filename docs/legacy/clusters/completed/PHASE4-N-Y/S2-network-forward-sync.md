# Invariant Slice — S2: Network forward-sync durable lifecycle

> **Scope decision (locked):** **OI-Y.2 → GREEN reducer + RED pump.** The admission
> reducer (`ade_ledger::receive::receive_apply_sequence`, generic over `ChainDbWrite`)
> is already pure BLUE/GREEN; the network fetch (`ade_core_interop::follow` + `mux_pump`)
> is RED. S2 adds a GREEN forward-sync **lifecycle reducer** composing admit + durability
> cadence, driven by the RED pump — mirroring the existing `session`/`mux_pump` split.
> No single-RED-blob driver.

## §2 Slice Header

- **Slice Name:** Durable network forward-sync — anchor→tip through chokepoints, with preserved-bytes + WAL committed before every tip advance, replay-equivalent.
- **Cluster:** PHASE4-N-Y.
- **Status:** Merged.
- **Cluster Exit Criteria Addressed** (verbatim):
  - [ ] **CE-Y-5.** Forward-sync admits blocks only through `decode_block`→`validate_and_apply_header`→`block_validity`→fork-choice; gate `ci_check_forward_sync_chokepoint_only.sh` (negative: no block reaches the store without passing the validators).
  - [ ] **CE-Y-6.** Each admitted block is preserved-byte-stored **and** WAL-committed before tip advance; test `forward_sync_wal_and_bytes_precede_tip_advance` passes.
  - [ ] **CE-Y-7.** Forward-sync replay-equivalence: test `forward_sync_replay_two_runs_byte_identical` over corpus `corpus/sync/preprod_snapshot_to_tip_*`.
  - [ ] **CE-Y-15** *(partial):* `DC-SYNC-01` resolved; `DC-CONS-20`/`DC-STORE-02`/`DC-STORE-05`/`T-DET-01` `strengthened_in += "PHASE4-N-Y"`.
- **Slice Dependencies:** S1 (consumes the verified `BootstrapAnchor` as the sync origin).

## §3 Implementation Instruction (AI)

Compose existing authorities; do not rewrite admission or validation. The GREEN lifecycle reducer must hold no socket/clock/redb state — those live in the RED pump. The durability-before-tip ordering is the slice's whole point; do not advance the tip before the block's preserved bytes + WAL entry are durable. §12 is the completion contract.

## §4 Intent

Make it impossible for the chain tip to advance to a block whose preserved wire bytes and WAL entry are not yet durable, and impossible for a block to reach the store without passing the canonical decode→header→ledger→fork-choice chokepoints — and prove the admitted sequence is replay-equivalent.

## §5 Scope

- **GREEN (new):** `ade_runtime` forward-sync lifecycle reducer — composes `receive_apply_sequence` (BLUE admit) + the existing `rollback::cadence` checkpoint cadence; emits a closed effect set `{StoreBlockBytes, AppendWal, CommitCheckpoint, AdvanceTip}` with the ordering invariant `AdvanceTip` is unreachable until `StoreBlockBytes`+`AppendWal` for that block are acknowledged durable.
- **RED:** the pump driving ChainSync/BlockFetch from the anchor (reuse/extend `ade_core_interop::follow` + `mux_pump`); redb writes via `persistent` ChainDb + `FileWalStore`.
- **BLUE (reused, no new authority):** `ade_codec` decode, `ade_core::consensus::{header_validate, fork_choice, nonce}`, `ade_ledger::{receive::admit_via_block_validity, block_validity, wal encode}`.
- **Persistence:** preserved block bytes (`ChainDb::put_block`) + WAL append per admitted block + checkpoint cadence. No anchor change.
- **Out of scope:** crash recovery wiring (S3 — S2 only *writes* durably; restart-recovery is S3), genesis source (S4), evidence (S5), epoch-boundary nonce roll (separate cluster).

## §6 Execution Boundary (TCB color)

- **BLUE:** decode + header/ledger validation + fork-choice + WAL encode (reused).
- **GREEN:** the forward-sync lifecycle reducer (closed effect enum; pure; no socket/clock/redb — purity-gated, BLUE-style banner + deny attrs).
- **RED:** the fetch pump (`follow`/`mux_pump`), redb ChainDb writes, `FileWalStore::append`.

Color resolved (OI-Y.2). The reducer is GREEN only because it holds no I/O state; CI asserts no `tokio`/`redb`/`SystemTime` in the reducer module.

## §7 Invariants Preserved

[[DC-CONS-20]] (admit-side semantics unchanged — admission still flows through `admit_via_block_validity`), [[CN-WAL-01]] (WAL append-only), [[DC-WAL-01]]..[[DC-WAL-03]], [[DC-STORE-03]] (atomic snapshots), [[T-DET-01]], [[CN-ANCHOR-01]] (anchor from S1 is the sync origin), the BLUE forbidden-pattern + `ci_check_hash_uses_wire_bytes.sh` / `ci_check_dependency_boundary.sh` gates, and `ci_check_admitted_block_closure.sh`.

## §8 Invariants Strengthened or Introduced

**One family — durable forward-sync admission:**
- **Introduces `DC-SYNC-01`** — during forward-sync, a block's preserved wire bytes and WAL entry MUST be durable before the tip advances to it; admission is chokepoint-only. *(OI-Y.3 confirmed: the durability-before-tip **ordering** is not expressed by any existing DC-CONS/DC-STORE rule — `DC-STORE-02` is append-only provenance, `DC-CONS-20` is admit semantics — so one new rule is warranted.)*
- Side-effect strengthenings: [[DC-CONS-20]] (admit now exercised over a real synced sequence), [[DC-STORE-02]], [[DC-STORE-05]] (snapshot + forward-replay path now driven end-to-end), [[T-DET-01]] (replay-equivalence over the synced corpus).

## §9 Design Summary

The GREEN reducer is `fn forward_sync_step(state, event) -> (state, Vec<SyncEffect>)`. `SyncEffect` is closed; the type system makes `AdvanceTip` follow `StoreBlockBytes`+`AppendWal` (the reducer emits `AdvanceTip` only after the durability effects for that block are in the same step's effect list, applied in order by the RED pump, which acks durability before issuing the tip write). Admission is `admit_via_block_validity` unchanged; fork-choice picks the tip.

## §10 Changes Introduced

- **Types:** closed `SyncEffect` enum; forward-sync reducer state (GREEN value).
- **State transitions:** the forward-sync step reducer (composition of existing transitions).
- **Persistence:** per-block preserved-byte store + WAL append + checkpoint cadence (existing stores; new ordering).
- **Removal/refactors:** none required.

## §11 Replay / Crash / Epoch Validation

- **Replay:** `forward_sync_replay_two_runs_byte_identical` over `corpus/sync/preprod_snapshot_to_tip_*` — same anchor + same ordered block sequence → byte-identical post-state fingerprint + WAL.
- **Crash/restart:** not proven here (S3); S2 proves only the *durable-write ordering* (`forward_sync_wal_and_bytes_precede_tip_advance`).
- **Epoch boundary:** not applicable (single-epoch sync window; cross-epoch nonce roll is a separate cluster).

## §12 Mechanical Acceptance Criteria

- [ ] `forward_sync_wal_and_bytes_precede_tip_advance` — the reducer never emits `AdvanceTip` for a block before its `StoreBlockBytes`+`AppendWal` effects; a constructed out-of-order attempt fails the type/test.
- [ ] `forward_sync_replay_two_runs_byte_identical` — corpus replay, two runs byte-identical post-state + WAL.
- [ ] `forward_sync_admission_through_chokepoints` — a block that fails `block_validity` is never stored/WAL'd/tip-advanced.
- [ ] `ci/ci_check_forward_sync_chokepoint_only.sh` — negative grep: no store/WAL/tip-advance path bypassing `admit_via_block_validity`; the reducer module has no `tokio`/`redb`/`SystemTime`.
- [ ] `cargo test --workspace` clean; carry-forward gates pass (`ci_check_admitted_block_closure.sh`, `ci_check_wal_append_only.sh`, `ci_check_hash_uses_wire_bytes.sh`, `ci_check_receive_replay_purity.sh`).

## §13 Failure Modes

Block decode/validation failure → block dropped, no store/WAL/tip (deterministic, fail-closed). Durability write failure (redb/WAL) → halt before tip advance (fail-fast; no partial tip). Fork-choice reject → block stored as non-tip per existing semantics. All replay-safe (tip only advances on durable + valid).

## §14 Hard Prohibitions

**Inherited (cluster §7).** **Slice-specific:** no tip advance before durable preserved-bytes+WAL; no admission path bypassing `admit_via_block_validity`; no `tokio`/`redb`/`SystemTime`/clock in the GREEN reducer; no re-decoding for hash (preserved bytes only); no cross-epoch nonce roll; no crash-recovery claim (S3).

## §15 Explicit Non-Goals

No crash recovery (S3), no genesis source (S4), no evidence harness (S5), no new protocol surface, no performance tuning, no epoch-boundary handling.

## §16 Completion Checklist

- [ ] New state replay-derivable; data canonically encoded; failures deterministic; no TODO in GREEN reducer; CI enforces DC-SYNC-01; replay-equivalence passes.

## §17 Review Notes

Risk: the GREEN reducer accreting I/O state → CI purity grep. Risk: tip advancing on a non-durable block → the ordering test + type shape. Follow-up: S3 consumes the durable writes for recovery.

## §18 Authority Reminder

Planning aid only; registry + CI authoritative.
