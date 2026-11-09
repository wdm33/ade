# Invariant Slice — PHASE4-N-F-G-P S1: feed header-validation view from the recovered consensus surface

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header
- **Slice:** PHASE4-N-F-G-P S1 — the live warm-start feed/receive run-loop projects its header-validation
  `LedgerView` from the recovered `SeedEpochConsensusInputs` via the SAME single authority the forge uses
  (`PoolDistrView::from_seed_epoch_consensus_inputs`), so Step 5 (VRF-keyhash binding) + Step 7 (leader
  threshold) see the real recovered stake surface — never an empty placeholder.
- **Cluster:** PHASE4-N-F-G-P — Feed-side leader-threshold stake-distribution fidelity.
- **Status:** planned.
- **CE addressed:** CE-G-P-1 (the wiring + projection + fail-closed + pins). [S2 = live C1, operator-gated.]

## §3 Dependencies
- Captured evidence (G-P, FEED-VRF-DIAG, reverted): eta0 `953a4c34…` correct, VRF `verify_ok=true` +
  `recomputed_eq_output=true`; Step 7 `pool_active_stake=None total_active_stake=Some(0) asc=Some((0,1))` from
  the empty feed view.
- `PoolDistrView::from_seed_epoch_consensus_inputs` (ade_ledger consensus_view) — the existing single
  projection authority the forge uses (`forge_one_from_recovered`, node_sync.rs:553; DC-CINPUT-02b).
- `SeedEpochConsensusInputs` (already persisted with ASC + total_active_stake + pool_distribution + per-pool
  VRF keyhash; G-N added epoch_nonce) — recovered into `BootstrapState.seed_epoch_consensus_inputs`.
- The defect locus: `node_lifecycle.rs` On-arm `ledger_view = PoolDistrView::new(epoch, 0, ASC{0,1},
  BTreeMap::new())` (≈:462) threaded to `run_node_sync` (node_sync.rs:343) → `pump_block` → `block_validity` →
  `validate_and_apply_header`.

## §4 Intent (invariant impact)
Close the proven feed-side Step-7 rejection so the feed validates blocks against the same recovered consensus
surface the forge produces under. Enforces `DC-CINPUT-04`. Uses the EXISTING forge projection authority — no
new view type, no Step 5/6/7 change, no VRF/eta0 change, no ledger-state change.

## §5 Scope / What is built
1. **Feed view projection** — on the live warm-start feed arm, build the header-validation `ledger_view` via
   `PoolDistrView::from_seed_epoch_consensus_inputs(<recovered seed_epoch_consensus_inputs>)` instead of the
   empty `PoolDistrView::new(..., BTreeMap::new())`. Fail-closed (structured `NodeLifecycleError`, e.g.
   `MissingRecoveredConsensusInputs`) when the recovered record is absent on a feed-wired node — no empty
   fallback, no accept-if-missing.
2. **Pin tests:** (a) a recovered `BootstrapState` with a populated `SeedEpochConsensusInputs` projects a feed
   view exposing real ASC + total_active_stake + pool_active_stake (recorded pool) + pool_vrf_keyhash; a header
   for the recorded pool at positive recovered stake validates through Step 5 + Step 7 against it; (b) the
   forge view (`from_seed_epoch_consensus_inputs`) and the feed view are byte-identical projections of the same
   recovered record; (c) a recovered state lacking `SeedEpochConsensusInputs` on a feed-wired node fails closed.
3. **Registry + CI:** `DC-CINPUT-04` → enforced; a CI gate asserts the feed arm projects from the recovered
   record (not `PoolDistrView::new(.., BTreeMap::new())`), shares the forge's projection authority, and
   fail-closes on a missing record.

**Out of scope:** the live C1 confirmation (S2); the FIRST-RUN/Mithril feed view (OQ-P1); cross-epoch stake
evolution (OQ-P2); any VRF / eta0 / Step-5/6/7 / ledger-state change.

## §6 Execution Boundary (TCB color)
RED node wiring (`node_lifecycle`) selecting the BLUE/GREEN recovered projection
(`PoolDistrView::from_seed_epoch_consensus_inputs`) as the feed's `LedgerView`. `validate_and_apply_header`
(Steps 5/6/7), the VRF, eta0, and ledger state are unchanged. One recovered consensus surface; forge and feed
share it.

## §11 Replay / Crash / Epoch Validation
`from_seed_epoch_consensus_inputs` is a pure deterministic projection (same recovered record ⇒ same view).
Covered by pin (b) (forge view ≡ feed view) + the existing recovered-projection tests. No new authoritative
transition; the leader threshold (Step 7) it feeds is unchanged.

## §12 Mechanical Acceptance Criteria
- [ ] The WarmStart-recovered feed view exposes ASC, total_active_stake, pool_active_stake (recorded pool), and
      the producer VRF keyhash from the persisted `SeedEpochConsensusInputs` (not zero/empty).
- [ ] A header for the recorded pool at positive recovered stake validates through Step 5 + Step 7 against the
      recovered feed view (hermetic).
- [ ] A missing recovered `SeedEpochConsensusInputs` on a feed-wired node fails closed (structured error).
- [ ] The forge and feed views are byte-identical projections of the same recovered record (one surface).
- [ ] `DC-CINPUT-04` enforced; CI gate present; the feed arm no longer constructs an empty placeholder view.
- [ ] No regression: ade_node node_lifecycle / node_sync + ade_ledger consensus_view + ade_runtime suites pass.

## §14 Hard Prohibitions
- no hardcoded stake; no C1-only stake override; no leader-threshold bypass; no accept-if-missing-stake;
- no weakening Step 5 or Step 7; no VRF / eta0 change;
- no broad "populate ledger state from consensus inputs" — narrow to the header-validation consensus view;
- fail-closed on a missing/inconsistent recovered record; no RO-LIVE flip; no acceptance claim without
  the follower log through `correlate`.

## §15 Explicit Non-Goals
The live C1 confirmation (S2, operator-gated); the FIRST-RUN/Mithril feed view (OQ-P1); cross-epoch stake-view
evolution (OQ-P2); durable block-1+ progression; any VRF / eta0 / Step-5/6/7 / ledger-state change.
