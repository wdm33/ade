# Invariant Slice — Forge-base authority rewire (DC-NODE-20 S1)

## 2. Slice Header
**Slice Name:** Forge-base authority rewire — self-admit enters extend mode (DC-NODE-20, PHASE4-N-AH S1)
**Cluster:** PHASE4-N-AH — local selected durable chain forge-base authority; **rung-1, single-producer only**
**Status:** Proposed
**Authority source:** `docs/clusters/PHASE4-N-AH/cluster.md` (§4, CE-AH-1/2/5, §10); registry `DC-NODE-20` (declared — **introduced/enforced** by this cluster, flipped at close)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AH-1:** post-self-admit, the forge builds on `ChainDb::tip` with `durable_servable_tip != followed_peer_tip` and **no cert file present**; the direct `CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine` transition. New gate `ci/ci_check_local_durable_forge_base.sh` green.
- [ ] **CE-AH-2:** the local-tip forge base **fails closed** when any of the 6 fence conditions fails (incl. the broadened observed-feed competing-candidate predicate); no fallback to followed/cert.
- [ ] **CE-AH-5:** core acceptance — catch up once → self-admit first own block via `pump_block` → forge N+1 on `ChainDb::tip` → forge **N+2** sustained on the local spine, **no cert in the forge path** (forged ≥ 2 own blocks).

Exit criteria not listed (CE-AH-3=S2 cert evidence-only; CE-AH-4=S3 replay; CE-AH-6=S4 live; CE-AH-7=close) are out of scope.

**Slice Dependencies:** DC-NODE-18 (the `ForgeMode` machinery + the extend-state fence) + DC-NODE-19 (the continuation loop) — both reused; their **cert clauses are superseded** here (cores preserved).

## 3. Implementation Instruction (AI — INLINE)
**Read §§9–10 + `forge_mode_after_admit` (node_sync ~895) + `single_producer_forge_decision` (node_sync ~922–989) + the `proceed_to_forge` gate (node_lifecycle ~1277–1366) first.** The surgical change: `forge_mode_after_admit`'s `CaughtUpToPeerTip` arm enters the extend state **directly** on self-admit (no `FirstOwnBlockServed`, no cert). The extend state already forges on the local tip under a fence — **do not** invent a new forge-base path and **do not** touch `dc_node_15_refusal` / the `durable==followed` admission (initial-catch-up only, unchanged). Remove the cert-promotion + the `continuation_cert_missing` gate. **S1 removes the cert as local-spine ENTRY / CONTINUATION authority only** — the global "cert is evidence-only" prohibition + audit gate is **S2 (DC-NODE-21)**, not this slice. §12 is the completion proof. Commit carries the repo's model trailer.

## 4. Intent
Move the **entry authority into the single-producer extend state** from the operator adoption certificate to **Ade's own self-admitted durable spine**. The extend state (`SingleProducerExtendOwnDurableSpine`) already forges on the local durable tip (`current_tip` = `ChainDb::tip`) under a fence; the run-4 stall was the **`FirstOwnBlockServed` cert-wait** (`AwaitAdoptionCertificate` → `NoTipAvailable`, forever) blocking entry. DC-NODE-20: `CaughtUpToPeerTip + self-admit via pump_block (+ fence) → extend` directly. **DC-NODE-15 is unchanged** (initial catch-up still requires `durable == followed`); DC-NODE-20 changes only the **post-self-admit** transition. The cert is removed from the forge-loop entry **and** the continuation path; relay adoption becomes evidence (formalized in S2).

## 5. Scope (the corrected, sealed shape)
1. `forge_mode_after_admit` (node_sync): `CaughtUpToPeerTip + admitted → SingleProducerExtendOwnDurableSpine{adopted_root: own, current_tip: own}` **directly** (replace the `forge_mode_on_first_own_block_served` call).
2. Remove `FirstOwnBlockServed` from production forge authority — the variant + the cert-promotion branch in `single_producer_forge_decision` (the `Promote`/`AwaitAdoptionCertificate` on a `FirstOwnBlockServed` mode).
3. Remove the cert read from the forge-loop path (node_lifecycle ~1297–1302).
4. Remove the **DC-NODE-19 continuation cert requirement** — the `continuation_cert_missing` gate (node_lifecycle ~1279–1285) + its `AdoptionCertificateMissingOrMalformed` refusal. Continue-past-EOF in the extend state is **preserved** (it no longer depends on the cert).
5. **Preserve** the existing extend-state local-tip forge + fence (`single_producer_forge_decision`'s `SingleProducerExtendOwnDurableSpine` arm — `ExtendOwnSpine`).
6. **Broaden** the competing-candidate predicate (fence condition 2) to: a **peer-origin candidate not already part of Ade's local admitted spine / own-served lineage** ⇒ fail closed (no fork resolution — rung 2). *(Today: `CompetingPeerBlockBeyondAdoptedRoot` + `PeerTipDisagreesWithSpine` keyed on `observed_peer_tip`; broaden to the spine-membership predicate.)*
7. **Unchanged:** `dc_node_15_refusal` / `durable == followed` (initial catch-up); `ci_check_forge_followed_tip_admission.sh` (already initial-catch-up-only — **no phase-split**); `run_loop_planner`; all BLUE.

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED):** `ChainDb::tip`, `pump_block`, `forge_one_from_recovered`, `block_validity`/`prior_fp`, the extend-state forge.
- **GREEN:** `ade_node::node_sync` — `forge_mode_after_admit` (the direct transition), `single_producer_forge_decision` (remove the `FirstOwnBlockServed` branch; broaden condition 2), the `ForgeMode` enum (remove `FirstOwnBlockServed`).
- **RED:** `ade_node::node_lifecycle` — `run_relay_loop` `proceed_to_forge` gate (remove the cert read + `continuation_cert_missing`).

## 7. Invariants Preserved (registry IDs)
`DC-NODE-05` + `DC-NODE-12` (`pump_block` sole durable admit authority — the forge advances no tip; condition 4 of the fence requires admission through `pump_block`) · `DC-NODE-15` (initial catch-up gate — **unchanged**, `durable == followed` still required pre-self-admit) · `DC-NODE-18` **core** (own-spine forge on `current_tip`, no relay echo — preserved; only its cert-promotion clause is superseded) · `DC-NODE-19` **core** (continue-past-EOF in the extend state — preserved; only its cert-fence clause is superseded) · `DC-CONS-03` (fork-choice untouched — a competing candidate fails closed, never resolved) · `T-REC-03`/`T-REC-05` (the local-tip forge base derives from the local durable spine alone).

## 8. Invariants Strengthened or Introduced
**Introduces `DC-NODE-20`** (the forge-base authority — the extend state is entered by self-admit + fence, not the cert). Exactly **one** invariant family (forge-base authority). `DC-NODE-21` (the global cert-evidence-only prohibition + audit gate) is **S2**, not this slice. The cert-clause **supersessions** of DC-NODE-18 (cert-promotion) and DC-NODE-19 (cert-fence) are recorded at `/cluster-close` (CE-AH-7) as *"DC-NODE-18/19 cert clauses superseded by DC-NODE-20/DC-NODE-21; cores preserved"* — not flipped here. `DC-NODE-20` flips declared→enforced at close.

## 9. Design Summary
The single behavioral change: **self-admit, not the cert, enters the extend state.**
- `forge_mode_after_admit`: `(CaughtUpToPeerTip, admitted, own_tip) → SingleProducerExtendOwnDurableSpine{adopted_root: own_tip, current_tip: own_tip}`.
- `single_producer_forge_decision`: the `FirstOwnBlockServed` arm (cert-promotion / `AwaitAdoptionCertificate`) is removed (the mode is never constructed); the `SingleProducerExtendOwnDurableSpine` arm (the fence + `ExtendOwnSpine` forge on the spine head) is unchanged except condition 2 is broadened to the spine-membership predicate.
- `proceed_to_forge`: the `continuation_cert_missing` pre-check and the `FirstOwnBlockServed` cert read are removed; the extend state continues past a feed EOF on its own spine + fence (DC-NODE-19 core), no cert.
- The forge base is `ChainDb::tip` (the local durable head), exactly as the extend state already reads it. No new forge-base path; no `durable == followed` change.

## 10. Changes Introduced
- **Types:** remove `ForgeMode::FirstOwnBlockServed` (the best option — every production use is the retired cert machinery); `forge_mode_on_first_own_block_served` removed.
- **State transitions:** `forge_mode_after_admit` `CaughtUpToPeerTip → extend` direct.
- **Gate:** `proceed_to_forge` loses the cert read + `continuation_cert_missing`.
- **Fence:** condition 2 broadened (spine-membership).
- **CI:** new `ci/ci_check_local_durable_forge_base.sh`; update `ci/ci_check_single_producer_extend_own_spine.sh` (extend entered via self-admit, not cert) + `ci/ci_check_single_producer_loop_continuation.sh` (drop the `AdoptionCertificateMissingOrMalformed` continuation assertion; keep the continue-past-EOF assertions). `ci_check_cert_evidence_only.sh` is **S2**.

## 11. Replay, Crash, and Epoch Validation
- **Replay/crash:** S1 adds no replay corpus (that is S3); the existing replay gates (`ci_check_node_run_loop_containment.sh`, the T-REC tests) must stay green — the local-tip forge base derives from the durable spine alone, so replay is preserved.
- **Epoch:** within-epoch (initial catch-up + sustained local forging); off-epoch forge-validity bounds unchanged.

## 12. Mechanical Acceptance Criteria
- [ ] `caughtup_self_admit_enters_extend_directly_no_cert` (`ade_node::node_sync::tests`) — `forge_mode_after_admit(CaughtUpToPeerTip, admitted=true, own_tip)` → `SingleProducerExtendOwnDurableSpine{current_tip = own_tip}`; **no `FirstOwnBlockServed`**. [CE-AH-1]
- [ ] `post_self_admit_forges_on_local_tip_durable_ne_followed_no_cert` (loop test, reuse `s2_extend_lead`) — post-self-admit, the forge builds on `ChainDb::tip` with `durable != followed` and **no cert file present**. [CE-AH-1]
- [ ] `extend_entry_fence_fails_closed_each_condition` + `competing_peer_origin_block_not_in_spine_fails_closed` (`node_sync::tests`) — the fence refuses on each of the 6 conditions; the broadened predicate trips on a peer-origin non-spine block. [CE-AH-2]
- [ ] `continuation_past_eof_no_longer_requires_cert` (loop test) — in the extend state, a feed EOF continues the loop with **no cert file present** (DC-NODE-19 core preserved; cert-fence superseded). [CE-AH-2]
- [ ] `local_spine_sustains_two_successors_no_cert` (loop test) — catch up → self-admit → forge N+1 → forge N+2 on the local spine, **no cert in the forge path**. [CE-AH-5]
- [ ] `cargo test -p ade_node` green (the new tests + all existing, incl. the updated DC-NODE-18/19 tests).
- [ ] `ci/ci_check_local_durable_forge_base.sh` green; `ci_check_single_producer_extend_own_spine.sh` + `ci_check_single_producer_loop_continuation.sh` updated + green; `ci_check_node_run_loop_containment.sh` + `ci_check_forge_followed_tip_admission.sh` stay green (unchanged).
- [ ] `DC-NODE-20` still `declared` (flips at close); `DC-NODE-21` untouched (S2).

## 13. Failure Modes
- A premature extend entry (before a real `pump_block` admit) → impossible: `forge_mode_after_admit` only transitions on `admitted == true` (a real own-block admit through `pump_block`, DC-NODE-12).
- A competing producer / fork → the broadened fence fails closed (no resolution; rung 2). A regression here would be a DC-NODE-20 correctness defect — the negative tests catch it.

## 14. Hard Prohibitions
**Inherited (cluster §8):** no new BLUE authority; no cert read in the forge path; no forge path requires `FirstOwnBlockServed` + cert to enter extend; no fork-choice in the fence (DC-CONS-03 untouched); no silent fallback to `followed_peer_tip`/cert; no weakening of DC-NODE-15; `pump_block` stays sole durable admit authority; no new `cli.rs` flag.
**Slice-specific:**
- **Do not** do the global DC-NODE-21 cert-evidence-only prohibition + audit gate — that is **S2**. S1 removes the cert only from the **local-spine entry / continuation authority** path.
- **Do not** phase-split `ci_check_forge_followed_tip_admission.sh` — it is already initial-catch-up-only.
- **Do not** weaken DC-NODE-18's own-spine forge core or DC-NODE-19's continue-past-EOF core — only the cert clauses are superseded.
- **Do not** add a numeric cap or a new forge-base path — reuse the extend state's existing local-tip forge.

## 15. Explicit Non-Goals
The global cert-evidence-only prohibition + `ci_check_cert_evidence_only.sh` (S2) · replay-equivalence corpus (S3) · the live re-homed CE-AF-6b (S4) · flipping DC-NODE-20/21 or the registry strengthenings (close) · the phase-split (not needed) · any BLUE change.

## 16. Completion Checklist
- [ ] `forge_mode_after_admit` enters extend directly on self-admit (no `FirstOwnBlockServed`).
- [ ] The cert read + `continuation_cert_missing` removed from the forge loop; continue-past-EOF preserved without the cert.
- [ ] The extend-state local-tip forge + fence preserved; condition 2 broadened.
- [ ] The 5 named tests green; `cargo test -p ade_node` green.
- [ ] `ci_check_local_durable_forge_base.sh` (new) + the two updated gates green; the unchanged gates stay green.
- [ ] `DC-NODE-20` still `declared`; DC-NODE-21 untouched.
