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
| **S1** | The live warm-start feed `ledger_view` is projected from the recovered `SeedEpochConsensusInputs` via the forge's `PoolDistrView::from_seed_epoch_consensus_inputs` authority (not an empty placeholder); fail-closed when the recovered record is absent on a feed-wired node; regression pins (a) the projected view exposes real ASC + total_active_stake + pool_active_stake + pool_vrf_keyhash and (b) forge and feed views are byte-identical projections of the same recovered record | CE-G-P-1 | DC-CINPUT-04 → enforced | planned |
| **S2** | Live C1 rerun: the feed validates the previously-failing block past Step 5 + Step 7 (no `VrfCert(VerificationFailed)`); serve stays alive; `correlate` decides adoption | CE-G-P-2 | operator-gated | planned |

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
- **OQ-P1:** the FIRST-RUN arm (`node_lifecycle.rs:398`) builds the SAME empty placeholder view and also feeds
  `run_node_sync`. Its consensus-input source is the Mithril first-run import, NOT the recovered
  `SeedEpochConsensusInputs`. Whether a live feed on first-run needs the equivalent projection (from the
  Mithril-imported surface) is a follow-on; G-P scopes the WARM-START feed path the C1 rehearsal exercises.
- **OQ-P2:** multi-epoch — the recovered view is the single seed epoch (`DC-EPOCH-03`); a feed block past the
  seed-epoch boundary is off-epoch (the forge already fails closed there). The feed validator's cross-epoch
  stake-view evolution is the N-U / multi-epoch concern, not G-P.
