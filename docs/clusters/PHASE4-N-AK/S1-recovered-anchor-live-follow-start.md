# Slice AK-S1 — Recovered Anchor Live-Follow Start (persisted recovery provenance)

## 1. Title
Persist the recovered bootstrap anchor point as replayable recovery provenance, and resolve the
live-follow start tip from it. The ONLY slice of PHASE4-N-AK (a narrow post-N-AH regression-remediation).

## 2. Slice Header
- **Cluster:** PHASE4-N-AK. **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-AK-1, CE-AK-2, CE-AK-3 (and CE-AK-4 no-collateral).
- **Primary registry rule:** DC-NODE-31 (`declared` → targeted `enforced` at AK close).

## 4. Intent (invariant impact)
Strengthen **DC-NODE-31** `declared → enforced`: the recovered store **persists** the bootstrap anchor
point `(slot, hash)` as replayable recovery provenance (bound to the recovered anchor fingerprint), and
`BootstrapState` resolves the live-follow start tip from it for a **bare-anchor** recovery — so the
wire-pump FindIntersect starts at the anchor, **not Origin**, and AI-S4a's `RollBackward(Origin)`
fail-close is not spuriously triggered. This replaces N-AH's re-sync-from-genesis with the correct,
replay-equivalent FindIntersect-at-the-anchor; restart correctness is **store-derived, never
CLI-dependent**.

## 6. Execution Boundary (TCB color)
- **BLUE** — (a) `resolve_live_follow_start` + the `BootstrapState.tip` resolution
  (`crates/ade_runtime/src/bootstrap.rs`); (b) the **persisted anchor-point provenance record** (its
  canonical content `(slot, hash)` + the `anchor_fp` binding) and its **load + fail-closed verify** at
  warm-start. The authoritative, replay-equivalent recovery decision (what point the node recovered
  to).
- **RED I/O of a BLUE record** — the *write* of the anchor-point record at seed/recover, and its read.
- **Canonical input** — `BootstrapAnchor.seed_point` (`ade_ledger/src/bootstrap_anchor/anchor.rs:76`).
- **GREEN (caller wiring)** — `warm_start_recovery` / the recover path (`node_lifecycle.rs`,
  `mithril_bootstrap.rs`) thread the loaded anchor point into `BootstrapInputs.recovered_anchor`.
- **RED (UNCHANGED)** — `spawn_live_wire_pump_source` / wire pump (`node_lifecycle.rs`, `wire_pump.rs`).
- **UNCHANGED** — `ChainDb::tip()`; AI-S4a (`wire_pump.rs:447`); the materialization null-hash
  `TargetPoint` (`bootstrap.rs:216`); N-AJ evidence emission.

## 7. Invariants Preserved
- **T-REC-05** — recovery replay-equivalence (recovered ledger fp == WAL-tail post_fp).
- **DC-NODE-23..29** — single-best-peer rollback-follow, incl. AI-S4a `RollBackward(Origin)` fail-close.
- **DC-NODE-28** — forge gate.
- **DC-MITHRIL-02** — the anchor `seed_point` binding (the canonical-input source the record carries).
- **CN-CONS-03** — untouched.
- The `ChainDb::tip()` storage contract (Some only for a servable post-anchor block).

## 8. Invariants Strengthened
- **DC-NODE-31** `declared → enforced` — AK-S1 populates its `tests` with the named tests below.
- **T-REC-05** strengthened — `strengthened_in += PHASE4-N-AK`; the recovered tip surface is now
  replay-equivalent (same store ⇒ same persisted anchor point ⇒ same tip ⇒ same FindIntersect start).

## 9. Design Summary
- **Persist (additive recovery provenance).** Add a new persisted **anchor-point record** carrying the
  recovered `(slot, hash)`, **bound to `anchor_fp`** — persisted via the existing `SnapshotStore`
  sidecar surface (analogous to the seed-epoch consensus sidecar; a SEPARATE additive record, **not** a
  field added to `SeedEpochConsensusInputs`, so the existing `sidecar_hash`/provenance binding is
  untouched). **Write** it at seed/recover (`mithril_bootstrap::bootstrap_from_mithril_snapshot` /
  `seed_to_snapshot`) where `BootstrapAnchor.seed_point` is known.
- **Load + fail-closed verify (warm-start).** `warm_start_recovery` (and the recover path) **load** the
  anchor-point record and verify it is bound to the recovered `anchor_fp`. A non-Origin recovered store
  (snapshot non-empty) with a **missing / malformed / fingerprint-mismatched** record ⇒ a typed
  `BootstrapError` / `NodeLifecycleError` **fail-closed BEFORE live follow starts** (no silent Origin
  fallback). A true Origin / cold-start (empty snapshot set) needs no record.
- **Resolve.** Add `resolve_live_follow_start(servable_chaindb_tip: Option<ChainTip>, recovered_anchor:
  Option<ChainTip>) -> Option<ChainTip>` — pure: (1) servable if `Some`; (2) else `recovered_anchor`
  if `Some` and **non-Origin (non-zero hash)**; (3) else `None`. A zero/null-hash anchor is truly
  Origin (rule 3). `bootstrap_initial_state` (`bootstrap.rs:259-262`) sets `tip = resolve_live_follow_
  start(chaindb.tip(), inputs.recovered_anchor)`. Extend `BootstrapInputs` with `recovered_anchor:
  Option<ChainTip>` (`None` for cold-start / true-Origin; `Some` = the loaded persisted anchor).
- The materialization `TargetPoint` (`bootstrap.rs:198-218`) is **unchanged** (OQ-AK-2).
- **OQ-AK-1 (corrected):** the recovered store does NOT carry the seed-point today — the sidecar
  (`SeedEpochConsensusInputs`) + `RecoveredBootstrapProvenance` carry only `anchor_fp`; the FirstRun
  arm gets the point from the CLI. AK **persists** `BootstrapAnchor.seed_point` as an additive,
  provenance-bound record and **loads** it at warm-start. **CLI seed-point is first-run input only, NOT
  restart authority** (warm-start is store-derived).
- The wire-pump consumer (`spawn_live_wire_pump_source`) is **unchanged**.

## 11. Replay / Crash / Epoch Validation
Same on-disk recovered store + same WAL ⇒ byte-identical persisted anchor point ⇒ byte-identical
`BootstrapState.tip` ⇒ byte-identical FindIntersect `start_point` (extends T-REC-05 to the recovered
*tip* surface; `same_store_same_anchor_point_same_findintersect_start`). The fail-closed verify
guarantees a non-Origin recovered store always carries a provenance-bound anchor point or refuses to
start. Existing recovery tests stay green:
`warm_start_recovers_seed_epoch_consensus_inputs_byte_identical` and
`warm_start_dispatch_succeeds_end_to_end` (`crates/ade_node/src/node_lifecycle.rs`).

## 12. Mechanical Acceptance Criteria
- **CE-AK-1** (`ade_runtime`, hermetic):
  - `bootstrap_recover_persists_anchor_point_sidecar` — seed/recover writes the anchor-point record
    (bound to `anchor_fp`).
  - `warm_start_loads_persisted_anchor_point` — warm-start loads it ⇒ live-follow start tip == anchor.
  - `warm_start_non_origin_anchor_missing_anchor_point_fails_closed`.
  - `warm_start_anchor_point_fingerprint_mismatch_fails_closed`.
  - `same_store_same_anchor_point_same_findintersect_start`.
  - `bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip` (bare anchor ⇒ tip == anchor).
  - `bootstrap_true_origin_recovery_surfaces_none_tip` (cold-start ⇒ tip == None).
  - `bootstrap_servable_chaindb_tip_wins_over_anchor` (post-anchor ⇒ servable ChainDb tip wins).
  - `resolve_live_follow_start_treats_zero_hash_anchor_as_origin` (pure-fn unit).
  - `cargo test -p ade_runtime` green.
- **CE-AK-2** (`ade_node`, hermetic): `recovered_bare_anchor_findintersect_starts_at_anchor_not_origin`
  — a bare-anchor warm-start ⇒ start_point == the anchor `Block` point (NOT `Origin`). `cargo test -p
  ade_node` green.
- **CE-AK-3** (live, operator-run at AK close): the FIXED binary **re-recovers** (writing the anchor-
  point record) then `--mode node --single-producer-venue` on the frozen venue ⇒ FindIntersect from
  the persisted anchor (not Origin) ⇒ `forge_base_block_no == frozen relay tip block_no` ⇒ **0**
  `UnsupportedRollbackPoint`.
- **CE-AK-4** (no collateral): `cargo test --workspace` green; `warm_start_*` + T-REC-05 tests green;
  the three `ci/ci_check_convergence_evidence_*.sh` gates green; the `ChainDb::tip()` contract unchanged.

## 14. Hard Prohibitions (inherit cluster Forbidden verbatim)
- Do NOT weaken AI-S4a — `RollBackward(Origin)` stays fail-closed.
- Do NOT modify peer/relay behavior.
- Do NOT special-case the venue harness.
- Do NOT make ChainDb invent/synthesize a servable block.
- Do NOT use WAL `admit_count` (or any guess) as the anchor point.
- Do NOT use CLI re-supply as the durable restart fix — warm-start must be store-derived (CLI
  seed-point is first-run input only).
- Do NOT touch N-AJ evidence emission.
- Do NOT alter ledger materialization (`bootstrap.rs:216`) unless a test proves dependence (OQ-AK-2).
- Do NOT redesign admission orchestration (OQ-AK-3). Do NOT flip CN-CONS-03.
