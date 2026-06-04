# PHASE4-N-F-G-P — Feed-side leader-threshold stake-distribution fidelity (DC-CINPUT-04)

> **Grounded in proven evidence (capture-first, hypothesis REFUTED).** With G-O's tag-24 unwrap in, the C1
> follower's block reaches Ade's feed and DECODES; the feed then fails closed at
> `Receive(Validity(Header(VrfCert(VerificationFailed))))`. An instrumented header validator (FEED-VRF-DIAG +
> DIAG-2, both reverted) PROVED the eta0/VRF-mirror hypothesis WRONG: eta0 = `953a4c34…` (correct, recovered),
> the VRF proof VERIFIES (`verify_ok=true`), the output MATCHES (`recomputed_eq_output=true`) — so this is
> NEITHER eta0 NOR the VRF crypto. The failure is **Step 7 (leader threshold)**:
> `epoch=0 issuer_pool=b462622d… pool_active_stake=None total_active_stake=Some(0) asc=Some((0,1))` — the
> receive-path `ledger_view` has an EMPTY pool distribution. ROOT CAUSE: `node_lifecycle.rs:462` (the live
> warm-start feed arm) builds the feed `ledger_view` as `PoolDistrView::new(epoch, 0, ASC{0,1}, BTreeMap::new())`
> — an empty placeholder (justified pre-G-O by "feed source empty, UNCONSUMED — feed-end halts on iteration 1";
> now consumed, because the live feed DELIVERS blocks). The FORGE path uses the populated
> `PoolDistrView::from_seed_epoch_consensus_inputs(recovered)` (node_sync.rs:553) — so the forge self-accepts
> while the feed rejects: a forge/feed asymmetry in the RECOVERED consensus surface.
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`.

## §1 Primary invariant (DC-CINPUT-04)
The receive/feed-path **header-validation consensus view** — the `LedgerView` passed to `block_validity` →
`validate_and_apply_header` for Step 5 (VRF-keyhash binding) and Step 7 (leader threshold) — MUST be the
**recovered consensus surface**: ASC + total_active_stake + pool_distribution + per-pool VRF keyhash, projected
from the recovered `SeedEpochConsensusInputs` via the SAME single authority the forge uses
(`PoolDistrView::from_seed_epoch_consensus_inputs`). It is NEVER an empty / zero / default placeholder. Forge
and feed validation share ONE recovered consensus surface. Fail-closed: a missing recovered
`SeedEpochConsensusInputs` on a feed-wired node → a structured error / halt — never an empty view, never
"accept if missing stake," never a leader-threshold bypass.

This is the RECEIVE-side mirror of `DC-CINPUT-02b` (forge leadership view from recovered) and the same CLASS as
`DC-CINPUT-03` / `T-REC-04` (G-N: forge eta0 from recovered) — but a DIFFERENT recovered input (the **stake
distribution + ASC**, not the nonce) on a DIFFERENT path (the **feed**, not the forge).

**Scope note (narrow, load-bearing):** this is the Praos **header-validation consensus view** ONLY — the
leader-check `LedgerView`. The authoritative **ledger state** (UTxO / certs / ledger verdicts) is unchanged and
remains the authority for body/ledger validity. DC-CINPUT-04 does NOT "populate ledger state from consensus
inputs"; it populates the header-validation consensus view.

## §2 The defect (proven from captured evidence, not hypothesis)
`block_validity(ledger, chain_dep, era_schedule, ledger_view, block_cbor)` (transition.rs) passes its
`ledger_view: &dyn LedgerView` straight to `validate_and_apply_header`, which consults it in Step 5
(`pool_vrf_keyhash`) and Step 7 (`pool_active_stake` / `total_active_stake` / `active_slots_coeff`). On the live
feed path the run-loop is `node_lifecycle` (On-arm) → `run_node_sync` (node_sync.rs:343, `ledger_view` param) →
`pump_block` → `block_validity`. The On-arm builds that `ledger_view` as an EMPTY placeholder
(`node_lifecycle.rs:462`: `PoolDistrView::new(epoch, 0, ActiveSlotsCoeff{0,1}, BTreeMap::new())`), annotated
"PROVABLY UNCONSUMED on this binary path (empty source — feed-end halts on iteration 1)." Post-G-O that
annotation is FALSE: the live feed delivers Ade's own slot-107405 block, so the empty view IS consumed → Step 7
`pool_active_stake(epoch, issuer_pool).ok_or(VrfCert(VerificationFailed))?` (header_validate.rs:215) fires.
Meanwhile `forge_one_from_recovered` (node_sync.rs:553) builds `PoolDistrView::from_seed_epoch_consensus_inputs
(recovered_inputs)` — populated — so the forge self-accepts. Forge/feed asymmetry, proven by DIAG.

The persisted record already carries everything: `SeedEpochConsensusInputs` has `epoch_nonce`,
`active_slots_coeff`, `total_active_stake`, and `pool_distribution` (whose `PoolEntry` carries the VRF
keyhash). So this is purely a WIRING gap — the recovered record is not projected into the feed's view.

## §3 The fix — one recovered consensus surface for forge AND feed
On the live warm-start feed arm (`node_lifecycle` On-arm), build the feed `ledger_view` via
`PoolDistrView::from_seed_epoch_consensus_inputs(<recovered seed_epoch_consensus_inputs>)` — the EXACT
projection `forge_one_from_recovered` uses (node_sync.rs:553) — instead of the empty
`PoolDistrView::new(..., BTreeMap::new())`. Fail-closed (a structured `NodeLifecycleError`) when the recovered
`SeedEpochConsensusInputs` is absent on a feed-wired node — no empty fallback. No change to `decode_block`,
`validate_and_apply_header` (Steps 5/6/7 unchanged), the VRF, eta0, or ledger state.

## §6 TCB color
RED node wiring (`node_lifecycle`) selecting the BLUE recovered consensus-input projection
(`PoolDistrView::from_seed_epoch_consensus_inputs`, already the forge's authority) as the feed's
header-validation `LedgerView`. No new BLUE type; the leader-threshold check (Step 7) + VRF (Step 6) +
VRF-keyhash binding (Step 5) are unchanged. The recovered consensus surface is GREEN-projected once and shared
by forge and feed.

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | The live warm-start feed `ledger_view` is projected from the recovered `SeedEpochConsensusInputs` via the forge's `PoolDistrView::from_seed_epoch_consensus_inputs` authority (not an empty placeholder); fail-closed when the recovered record is absent on a feed-wired node; regression pins (a) the projected view exposes real ASC + total_active_stake + pool_active_stake + pool_vrf_keyhash and (b) forge and feed views are byte-identical projections of the same recovered record | CE-G-P-1 | DC-CINPUT-04 → enforced | **closed** (`609dc3cc`; live-confirmed) |
| **S2** | Live C1 rerun: the feed validates the previously-failing block past Step 5 + Step 7 (no `VrfCert(VerificationFailed)`); serve stays alive; `correlate` decides adoption | CE-G-P-2 | operator-gated | **partial** — feed past Step 5+7 + block ingested live; a SEPARATE forge-successor blocker (`RecoveredTipMissingBlockNo`) halts before serve/`correlate` → PHASE4-N-F-G-Q |

## §8 Cluster Exit Criteria
- **CE-G-P-1 (mechanical):**
  1. The WarmStart-recovered feed header-validation view exposes ASC, total_active_stake, pool_active_stake (for the recorded pool), and the producer VRF keyhash — all from the persisted `SeedEpochConsensusInputs` (not zero/empty).
  2. A header that previously failed Step 7 (the recorded pool at positive recovered stake) validates through Step 5 + Step 7 against the recovered view in a hermetic test.
  3. A missing recovered `SeedEpochConsensusInputs` on a feed-wired node fails closed (structured error) — never an empty view, never accept-if-missing.
  4. The forge path and feed validation path use byte-identical projections of the same recovered record (one consensus surface).
  5. (covered by S2) the C1 feed gets past `Header(VrfCert(VerificationFailed))`.
- **CE-G-P-2 (operator-gated):** a C1 rerun shows the feed validates the echoed block past Step 5 + Step 7, the
  serve remains alive, and the follower's adoption (or not) is decided only by the follower log through
  `correlate`. `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE flip.

## §9 Replay obligations
The projection `PoolDistrView::from_seed_epoch_consensus_inputs` is a pure, deterministic function of the
recovered record (same record ⇒ same view). No new authoritative transition; the leader-threshold check it
feeds (Step 7) is the existing BLUE authority, unchanged.

## §10 Invariants
- **Adds:** `DC-CINPUT-04` (feed/receive header-validation view from the recovered consensus surface),
  declared → enforced at S1.
- **Preserves / cross-ref:** `DC-CINPUT-02b` (forge leadership view from recovered — the mirror, unchanged),
  `DC-CINPUT-03` + `T-REC-04` (G-N forge eta0 from recovered), `CN-WIRE-12` (G-O feed decode), the BLUE
  Step 5 / Step 6 / Step 7 (`validate_and_apply_header`, unchanged), `DC-EPOCH-03` (single seed epoch),
  `RO-LIVE-01` (no flip).

## §11 Forbidden during this cluster (hard boundaries)
- **no hardcoded stake**; **no C1-only stake override**; **no leader-threshold bypass**; **no "accept if
  missing stake"** — a missing/inconsistent recovered record fails closed.
- **no weakening Step 5 or Step 7** — they still require the real recovered pool/total/ASC/vrf-keyhash.
- **no VRF / eta0 change** (the VRF + eta0 are proven correct; the defect is the stake-view wiring).
- **no broad "populate ledger state from consensus inputs"** — narrow to the header-validation consensus view;
  the ledger state stays authoritative for ledger verdicts.
- **no RO-LIVE flip; no acceptance claim** without the follower log through `correlate`.

## §12 Open questions
- **OQ-P1:** the `ForgeIntent::Off` (relay-only) arm (`node_lifecycle.rs:398`) builds the same empty
  placeholder view but hardcodes an ALWAYS-empty in-memory source (no `--peer` live feed, line ≈409), so its
  placeholder is genuinely unconsumed — no fix needed there. The real open question is the FIRST-RUN (Mithril)
  path: when it reaches the forge-on arm with a live feed, does the Mithril bootstrap populate
  `state.seed_epoch_consensus_inputs`? If not, S1's fail-closed (`FeedMissingRecoveredConsensusInputs`) fires
  — the Mithril-first-run feed view (projecting from the Mithril-imported surface) is the follow-on. G-P scopes
  the WARM-START forge-on feed path the C1 rehearsal exercises.
- **OQ-P2:** multi-epoch — the recovered view is the single seed epoch (`DC-EPOCH-03`); a feed block past the
  seed-epoch boundary is off-epoch (the forge already fails closed there). The feed validator's cross-epoch
  stake-view evolution is the N-U / multi-epoch concern, not G-P.

## §13 Close record — S1 (2026-06-04)
**G-P CLOSED with a narrow claim.** `DC-CINPUT-04` enforced: the feed/receive header-validation `LedgerView` is
projected from the recovered `SeedEpochConsensusInputs` via the single
`PoolDistrView::from_seed_epoch_consensus_inputs` authority the forge uses (one recovered consensus surface) —
not an empty placeholder; fail-closed (`FeedMissingRecoveredConsensusInputs`) when `--peer` is set but the
record is absent. Mechanical (CE-G-P-1): `feed_header_validates_against_recovered_surface_not_empty_view` (the
recovered-surface view validates the forged genesis-successor header through Step 5 + Step 7; the empty
placeholder fails closed `VrfCert(VerificationFailed)`) + `ci/ci_check_feed_leader_threshold_view.sh`. No VRF /
eta0 / Step-5/6/7 / ledger-state change. The `ForgeIntent::Off` relay-only arm hardcodes an always-empty
in-memory source (no `--peer`), so its placeholder is genuinely unconsumed — left as-is.

**LIVE-CONFIRMED:** the C1 `--mode node` rerun (2026-06-04 13:20Z, the fix in the binary) shows
`VrfCert(VerificationFailed)` GONE (count = 0). The feed DECODES (G-O) + VALIDATES Step 5 + Step 7 (G-P) +
INGESTS Ade's own block 0 (slot 107405, served by the follower at 13:20:32 `BlockFetch.Server.SendBlock`),
advancing Ade's tip genesis → slot 107405.

**NOT claimed:** serve stays alive; C1 rehearsal complete; RO-LIVE flip; bounty success. CE-G-P-2's serve-alive
/ `correlate`-adoption half is NOT reached — a separate forge-successor blocker halts before the follower's next
`:3002` retry. (A real cardano-node DID adopt an Ade-forged genesis-successor block at slot 107405 in a PRIOR
run — strong standing evidence, NOT a project acceptance claim until `correlate` binds the follower log.)

**NEW separate blocker → PHASE4-N-F-G-Q (Forge-successor tip/block_no fidelity):** with the block ingested, the
forge tries to build the successor and fails `relay run-loop sync step failed (RecoveredTipMissingBlockNo)` =
`forge_header_position(selected_tip=Some(slot 107405), last_block_no=None)` (node_sync.rs:501-503). The
feed/spine evolves its `chain_dep` (block_no 0) but the forge reads the un-evolved recovered baseline
(`state.chain_dep.last_block_no = None`) — a forge/feed state desync after feed-ingest. Capture-first:
instrument the feed apply/admit result + `forge_header_position` (selected_tip slot/hash, recovered/chain tip
block_no, `last_block_no` source, which `chain_dep` the forge reads), rerun C1, THEN fix; no guessed block_no,
no `unwrap_or(0/1)`, no synthetic successor numbering.
