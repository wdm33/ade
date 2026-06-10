# Slice AK-S1 — Recovered Anchor Live-Follow Start

## 1. Title
Recovered-anchor live-follow start authority — the BLUE recovery-decision fix + its mechanical proof.
The ONLY slice of PHASE4-N-AK (a narrow post-N-AH regression-remediation).

## 2. Slice Header
- **Cluster:** PHASE4-N-AK. **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-AK-1, CE-AK-2, CE-AK-3 (and CE-AK-4 no-collateral).
- **Primary registry rule:** DC-NODE-31 (`declared` → targeted `enforced` at AK close).

## 4. Intent (invariant impact)
Strengthen **DC-NODE-31** `declared → enforced`: the live-follow start tip exposed by `BootstrapState`
resolves to the recovered anchor tip (slot + real hash) for a **bare-anchor** recovery, so the
wire-pump FindIntersect starts at the anchor — **not Origin** — and AI-S4a's `RollBackward(Origin)`
fail-close is not spuriously triggered. This restores the N-AH live recover→follow path that the
current binary regressed (exposed, not caused, by AI-S4a's stricter Origin refusal). Servable-tip and
true-Origin recoveries are byte-unchanged.

## 6. Execution Boundary (TCB color)
- **BLUE** — `resolve_live_follow_start` + the `BootstrapState.tip` resolution in
  `crates/ade_runtime/src/bootstrap.rs`: the authoritative deterministic recovery decision (what point
  the node recovered to). Replay-equivalent.
- **Canonical input** — `BootstrapInputs.recovered_anchor: Option<ChainTip>` (new field) = the
  recovered `BootstrapAnchor.seed_point`.
- **GREEN (caller wiring)** — `warm_start_recovery` (`crates/ade_node/src/node_lifecycle.rs`) threads
  the recovered anchor seed_point into `BootstrapInputs.recovered_anchor`. Deterministic; it produces
  the BLUE recovery state.
- **RED (UNCHANGED)** — `spawn_live_wire_pump_source` / the wire pump
  (`node_lifecycle.rs`, `crates/ade_runtime/src/admission/wire_pump.rs`): consumes `state.tip`; not
  modified.
- **UNCHANGED** — `ChainDb::tip()`; AI-S4a (`wire_pump.rs:447`); the materialization null-hash
  `TargetPoint` (`bootstrap.rs:216`); N-AJ evidence emission.

## 7. Invariants Preserved
- **T-REC-05** — recovery replay-equivalence (recovered ledger fp == WAL-tail post_fp).
- **DC-NODE-23..29** — single-best-peer rollback-follow, incl. AI-S4a `RollBackward(Origin)`
  fail-close (`wire_pump.rs:447`).
- **DC-NODE-28** — forge gate.
- **DC-MITHRIL-02** — the anchor `seed_point` binding (the canonical-input source).
- **CN-CONS-03** — untouched (stays `declared`).
- The `ChainDb::tip()` storage contract (Some only for a servable post-anchor block).

## 8. Invariants Strengthened
- **DC-NODE-31** `declared → enforced` — AK-S1 populates its `tests` with the four named tests below.
- **T-REC-05** strengthened — `strengthened_in += PHASE4-N-AK`; gains the recovered-tip replay test in
  its `tests` (the recovered tip surface is now replay-equivalent, not just the ledger fingerprint).

## 9. Design Summary
- `resolve_live_follow_start(servable_chaindb_tip: Option<ChainTip>, recovered_anchor: Option<ChainTip>)
  -> Option<ChainTip>` — pure:
  1. `servable_chaindb_tip` if `Some`;
  2. else `recovered_anchor` if `Some` **and non-Origin (non-zero hash)**;
  3. else `None`.
  A zero/null-hash anchor is treated as truly Origin (rule 3) — never surfaced as a non-Origin tip.
- `bootstrap_initial_state` warm-start return (`bootstrap.rs:259-262`) sets
  `tip = resolve_live_follow_start(chaindb.tip(), inputs.recovered_anchor)`. The cold-start path
  (`bootstrap.rs:184-194`, no snapshot + no anchor) stays `tip: None`. The materialization `TargetPoint`
  (`bootstrap.rs:198-218`) is **unchanged** (OQ-AK-2 — out of scope; it serves the ledger skeleton,
  not the FindIntersect tip).
- **OQ-AK-1 resolved:** `RecoveredBootstrapProvenance` (`ade_ledger/src/wal/replay.rs:40`) carries only
  `anchor_fp` / `sidecar_hash` / `epoch_no` — **not** the seed_point. So extend `BootstrapInputs` with
  `recovered_anchor: Option<ChainTip>` (`None` for cold-start / true-Origin); the caller
  (`warm_start_recovery`) sources it from the recovered `BootstrapAnchor.seed_point`
  (`SeedPoint { slot, block_hash }`, `ade_ledger/src/bootstrap_anchor/anchor.rs:76` — already recorded
  at recover, already touched for the DC-MITHRIL-02 verification). No new data; no ChainDb change.
- The wire-pump consumer (`spawn_live_wire_pump_source`) is **unchanged** — it already maps
  `state.tip` → the FindIntersect `start_point` (`Block(tip)` | `Origin`).

## 11. Replay / Crash / Epoch Validation
Same on-disk recovered store ⇒ byte-identical `recovered_anchor` ⇒ byte-identical `BootstrapState.tip`
⇒ byte-identical FindIntersect `start_point` — extends T-REC-05 from the recovered *ledger* fingerprint
to the recovered *tip* surface. The existing recovery tests stay green:
`warm_start_recovers_seed_epoch_consensus_inputs_byte_identical` (the servable-tip path; `state.tip`
unchanged) and `warm_start_dispatch_succeeds_end_to_end` (`crates/ade_node/src/node_lifecycle.rs`). New
coverage: `bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip` (the bare-anchor path).

## 12. Mechanical Acceptance Criteria
- **CE-AK-1** (`ade_runtime`, hermetic):
  - `bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip` — snapshot @ a non-Origin
    anchor, no servable post-anchor block (`chaindb.tip()==None`), `recovered_anchor=Some(anchor)` ⇒
    `BootstrapState.tip == anchor` (slot + real hash).
  - `bootstrap_true_origin_recovery_surfaces_none_tip` — cold-start (empty snapshot set, no anchor) ⇒
    `BootstrapState.tip == None`.
  - `bootstrap_servable_chaindb_tip_wins_over_anchor` — ChainDb has a servable post-anchor block ⇒ the
    servable ChainDb tip wins over the anchor.
  - `resolve_live_follow_start_treats_zero_hash_anchor_as_origin` — pure-fn unit: `None` servable +
    zero-hash anchor ⇒ `None`.
  - `cargo test -p ade_runtime` green.
- **CE-AK-2** (`ade_node`, hermetic): `recovered_bare_anchor_findintersect_starts_at_anchor_not_origin`
  — a bare-anchor warm-start ⇒ `spawn_live_wire_pump_source` start_point == the anchor `Block` point
  (NOT `Origin`) ⇒ the AI-S4a Origin fail-close is not reached. `cargo test -p ade_node` green.
- **CE-AK-3** (live, operator-run at AK close — not a CI gate): the FIXED binary on the SAME frozen
  venue/store/relay ⇒ `--mode node --single-producer-venue` ⇒ FindIntersect from the recovered anchor
  (not Origin) ⇒ catch-up reaches the frozen relay tip ⇒ `forge_base_block_no == frozen relay tip
  block_no` ⇒ **0** `UnsupportedRollbackPoint`.
- **CE-AK-4** (no collateral): `cargo test --workspace` green;
  `warm_start_recovers_seed_epoch_consensus_inputs_byte_identical` +
  `warm_start_dispatch_succeeds_end_to_end` + the T-REC-05 tests stay green; the three
  `ci/ci_check_convergence_evidence_{vocabulary_closed,emit_only,schema}.sh` gates green; the
  `ChainDb::tip()` contract is unchanged.

## 14. Hard Prohibitions (inherit cluster Forbidden verbatim)
- Do NOT weaken AI-S4a — `RollBackward(Origin)` stays fail-closed.
- Do NOT modify peer/relay behavior.
- Do NOT special-case the venue harness.
- Do NOT make ChainDb invent/synthesize a servable block.
- Do NOT use WAL `admit_count` as the anchor-hash proxy.
- Do NOT touch N-AJ evidence emission.
- Do NOT alter ledger materialization (`bootstrap.rs:216` null-hash target) unless a test proves
  materialization depends on the placeholder (OQ-AK-2 — out of scope by default).
- Do NOT redesign admission orchestration — `--mode admission` in scope only if it consumes the same
  `resolve_live_follow_start` helper (OQ-AK-3).
- Do NOT flip CN-CONS-03.
