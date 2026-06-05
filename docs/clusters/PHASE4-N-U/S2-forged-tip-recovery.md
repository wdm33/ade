# Invariant Slice — PHASE4-N-U S2: forged-tip crash recovery + replay-equivalence

## §2 Slice Header
- **Slice Name:** forged-tip crash recovery + replay-equivalence
- **Cluster:** PHASE4-N-U — primary invariant DC-NODE-12 (S2 enforces T-REC-05 + the DC-WAL-04 no-orphan clause)
- **Status:** in progress
- **Cluster Exit Criteria Addressed:** **CE-5** (T-REC-05 + DC-WAL-04 no-orphan — production `warm_start_recovery` recovers a forged-block durable tip byte-identically; WAL-tail reconciliation drops an un-WAL'd forged orphan), **CE-6** (T-REC-05 replay — two clean forge-runs → byte-identical durable outputs).

## §3 Dependencies
S1 (`admit_forged_block_durably` — produces the durable forged WAL `AdmitBlock` entries + ChainDb blocks that S2 recovers). No dependency on S3.

## §4 Intent (invariant impact)
Production `warm_start_recovery` currently **requires a snapshot exactly at the tip** (`WarmStartForwardReplayUnsupported` otherwise) and **lacks the WAL-tail reconciliation** that the test-only `recover_node_state` has. A forged tip has **no snapshot-at-tip** (S1 captures none — rides DC-STORE-07; recovery is via WAL replay), so it currently fails recovery. S2 wires the **existing** `bootstrap_initial_state` warm-start **forward-replay** (already proven by `recover_node_state`'s `recovery_crash_at_phase_sync_byte_identical`) + the **WAL-tail reconciliation** into the production path, so a forged-block durable tip recovers byte-identically (T-REC-05) and a torn forge-admit crash (StoreBlockBytes done, AppendWal not) leaves no un-WAL'd forged orphan (DC-WAL-04 no-orphan).

## §5 Scope / What is built
In `ade_node::node_lifecycle::warm_start_recovery`:
1. **Reconstruct the recovery `era_schedule` + `ledger_view` from the recovered sidecar** (replacing the placeholders): read the sidecar via `ChainDb::get_seed_epoch_consensus_inputs(anchor_fp)` → `decode_seed_epoch_consensus_inputs` → `SeedEpochConsensusInputs`; `ledger_view = PoolDistrView::from_seed_epoch_consensus_inputs(&sidecar)`; `era_schedule = make_node_schedule(SlotNo(epoch_start), EpochNo(sidecar.epoch_no))` where `epoch_start = epoch_no * 432_000` (the from-genesis Conway era — for the genesis seed epoch this is `(0, 0)`, matching the live WarmStart arm's `make_node_schedule(0, 0)`).
2. **WAL-tail reconciliation** (port from `recover_node_state`): compute `wal_tail_slot` = the slot of the last `AdmitBlock` entry (or `SlotNo(0)` if none), then `chaindb.rollback_to_slot(wal_tail_slot)` BEFORE `bootstrap_initial_state` — drops any block durable in the ChainDb but absent from the WAL (the torn-write orphan).
3. **Remove the snapshot-at-tip guard** (`WarmStartForwardReplayUnsupported`), so `bootstrap_initial_state`'s warm-start branch forward-replays from the nearest snapshot ≤ tip.
4. **Fingerprint guard:** after `bootstrap_initial_state` returns, assert the recovered ledger fingerprint equals the WAL tail `post_fp` (when ≥1 `AdmitBlock`) — a deterministic fail-fast (`WarmStartWalTailFingerprintMismatch`), never silent divergence.

## §6 Execution Boundary (TCB color)
- **RED (changed):** `ade_node::node_lifecycle::warm_start_recovery`.
- **RED/GREEN (reused, unchanged):** `ade_runtime::bootstrap::bootstrap_initial_state` (the warm-start forward-replay authority), `ChainDb::rollback_to_slot` / `get_seed_epoch_consensus_inputs`.
- **BLUE (reused, NOT edited):** `ade_ledger::wal::replay_from_anchor`, `block_validity` (re-apply during forward-replay), `seed_consensus_inputs::decode_*`, `consensus_view::from_seed_epoch_consensus_inputs`.
- **No new BLUE authority or canonical type.**

## §7 Invariants Preserved
DC-SYNC-01 (durable-before-tip — unchanged; recovery is read-only over the WAL/ChainDb), T-REC-01/02 (`replay_from_anchor` fingerprint chain — unchanged; still run first as the integrity gate), CN-NODE-02 (recovery is the dispatcher's once-before-the-loop, not the loop), DC-NODE-12 (the admit path is unchanged), the BLUE forward-replay authority (`bootstrap_initial_state` — reused, not re-implemented).

## §8 Invariants Strengthened
- **T-REC-05** declared → **enforced**: same anchor + WAL (incl. forged AdmitBlock entries) → byte-identical recovered tip + ledger fp; two clean forge-runs → byte-identical durable outputs.
- **DC-WAL-04** partial → **enforced**: the no-orphan-recovery clause (WAL-tail reconciliation drops the un-WAL'd forged orphan) joins the S1 chaining clause.

## §11 Replay / Crash / Epoch Validation
- `forge_kill_then_warm_start_recovers_same_tip` — forge+admit blocks 0..1 via `admit_forged_block_durably`, drop the handles, reopen, `warm_start_recovery` (forward-replay) → recovered tip + ledger fp byte-identical to the pre-kill durable tip. (era_schedule matched: the test admits with the same `make_node_schedule(0,0)`-equivalent schedule recovery reconstructs.)
- `torn_forge_admit_crash_drops_orphan` — a block durable in the ChainDb but absent from the WAL (the StoreBlockBytes-before-AppendWal window) is dropped by the WAL-tail reconciliation; recovery yields the WAL-tail tip.
- `forge_two_clean_runs_byte_identical` — two clean forge+admit runs over identical inputs → byte-identical durable outputs (tip, WAL image).

## §12 Mechanical Acceptance Criteria
- `cargo test -p ade_node` green incl. the three new recovery tests above.
- The S1 gates (`ci_check_forged_durable_admit_via_pump.sh`, `ci_check_node_run_loop_containment.sh`, `ci_check_node_sync_via_pump.sh`) stay green (S2 does not touch the admit/loop bodies).
- Registry: T-REC-05 declared → enforced; DC-WAL-04 partial → enforced.
- Relevant crate tests green; full `cargo test --workspace` is the cluster-close gate (timeouts reported honestly).

## §14 Hard Prohibitions
No new BLUE authority/type; reuse `bootstrap_initial_state` (no re-implemented forward-replay); recovery stays read-only over the WAL/ChainDb except the deterministic `rollback_to_slot` reconciliation; the forward-replay MUST reproduce the exact tip (replay-equivalence) or fail fast (fingerprint guard), never silently diverge; no RO-LIVE flip.

## §15 Explicit Non-Goals
Non-genesis seed-epoch `era_schedule` generality (the `epoch_start = epoch_no * 432_000` reconstruction is the from-genesis single-Conway-era case; a multi-era / mid-epoch-anchor reconstruction is a separate concern, like the live WarmStart arm's existing approximation). S3 serve-projection (DC-NODE-13). Any RO-LIVE flip / bounty claim.
