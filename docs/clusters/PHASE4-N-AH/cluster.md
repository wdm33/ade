# Cluster PHASE4-N-AH — Local selected durable chain forge-base authority

**Primary invariant:** `DC-NODE-20` (paired with `DC-NODE-21`).
**Status:** Active — S1 next. **Authority bundle:** committed `b261589d`.
**Rung:** rung-1, single-producer only.

## 1. Primary Invariant
**`DC-NODE-20`** (registry) — in a declared rung-1 single-producer venue, after Ade self-admits a
valid forged block through `pump_block` onto its local durable ChainDB spine, the next forge base is
the **local selected durable tip** (`ChainDb::tip`), not `followed_peer_tip` and not the operator
adoption cert; fenced to 6 fail-closed conditions. **Paired (not folded):** **`DC-NODE-21`** — the
adoption cert is rung-1 evidence-only, never forge authority. `DC-CONS-03` (fork-choice) is the
**rung-2 successor**, untouched here.

## 2. Normative Anchors
- Registry rules `DC-NODE-20`, `DC-NODE-21` (the canonical statements).
- Invariants sketch `docs/planning/local-selected-durable-chain-forge-base-invariants.md`.
- Cluster plan `docs/planning/phase4-n-ah-cluster-slice-plan.md`.
- `docs/active/live-pass-path-fidelity-guide.md` (the verbatim `--mode node` path; the cert/flag
  fidelity that `DC-NODE-21` must not violate).
- `docs/active/c2-preprod-tip-guide.md` (the rung ladder context).
- Run-4 live finding (operator scratch, **not** committed): `~/.cardano-rung1-host/run4-dcnode20-motivation/`.

## 3. Entry Conditions (prior clusters guarantee)
- `DC-NODE-05` — `pump_block` sole durable admit authority (`ci_check_forged_durable_admit_via_pump.sh`).
- `DC-NODE-12` — own-forged durable admit chokepoint.
- `DC-NODE-15` — initial catch-up gate (`ci_check_forge_followed_tip_admission.sh`, the
  `durable == followed` admission).
- `DC-NODE-18` — extend-own-spine forge core + the `ForgeMode` machinery
  (`InitialCatchupRequired → CaughtUpToPeerTip → FirstOwnBlockServed → SingleProducerExtendOwnDurableSpine`;
  `forge_mode_on_caughtup` / `forge_mode_after_admit`) (`ci_check_single_producer_extend_own_spine.sh`).
- `DC-NODE-19` — continue-past-EOF in the extend state (hermetic)
  (`ci_check_single_producer_loop_continuation.sh`).
- `DC-CONS-03` (fork-choice), `T-REC-03`/`T-REC-05` (replay).
- Code: `selected_tip = ChainDb::tip` is **already computed** at `node_lifecycle` ~1221; the
  `proceed_to_forge` gate is ~1277–1366; `NoTipAvailable` fires at ~1489 when `proceed_to_forge` is
  false. The rung1-auto C2-LOCAL harness (σ=0.5 2-pool + dir/race fixes from run-4).

## 4. What Changes (design)
**The defect (run-4):** the `proceed_to_forge` gate requires `durable_servable_tip == followed_peer_tip`
(`forge_followed_tip_admission`). The instant Ade self-admits its own block, the non-producing relay
does not re-announce → `durable != followed` → `NoTipAvailable`; the only escape (cert-promotion
`FirstOwnBlockServed → extend`) raced and lost to the follow-link EOF. The cert had become forge
authority.

**The correction:**
- `proceed_to_forge`, **post-self-admit**, derives the forge base from the **local durable tip**
  (`ChainDb::tip`) — not `durable == followed`, not `read_adoption_cert`.
- `forge_mode_after_admit` no longer routes through `FirstOwnBlockServed` as a cert-wait:
  `CaughtUpToPeerTip + self-admit (+ fence) → SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}`,
  directly, no cert read. **Hard requirement: no forge-loop path may require `FirstOwnBlockServed` +
  cert to enter `SingleProducerExtendOwnDurableSpine`.** S1 leans to **removing** the
  `FirstOwnBlockServed` variant; an acceptable transitional is to keep the variant temporarily ONLY
  if no production forge transition parks there waiting for the cert. *(Exact enum shape is the one
  OPEN slice-doc detail for S1.)*
- `DC-NODE-15`'s `durable == followed` gate stays for **initial** catch-up
  (`InitialCatchupRequired → CaughtUpToPeerTip`); **only the post-self-admit re-check is superseded**.
  S1 must **split `ci_check_forge_followed_tip_admission.sh` by phase** — NOT remove the
  `durable == followed` check: **initial catch-up** requires `durable_servable_tip == followed_peer_tip`;
  **post-self-admit local-tip mode** does NOT require it (forge base = `ChainDb::tip`, only under the
  `DC-NODE-20` fence). The gate must assert exactly this phase split.
- **The 6-condition fence** (fail closed; no fallback to followed/cert):
  1. `VenueRole::SingleProducer`;
  2. **no competing candidate observed** — i.e. no peer-origin block received after local-tip
     authority is active whose hash is **not already part of Ade's admitted local spine** and whose
     **origin is not Ade's own served/admitted block**. In a single-producer venue with non-producing
     relays the relay should send no new blocks once Ade is sole producer; if one arrives, **fail
     closed** — no classification cleverness, no fork resolution (that is rung 2);
  3. relay non-producing;
  4. admitted via `pump_block`;
  5. ChainDB spine contiguous + servable;
  6. no fork-choice required — **mechanically derived from (2)**.
- `DC-NODE-21`: `read_adoption_cert` **removed** from the forge-base/`proceed_to_forge` path; cert
  **writing** preserved only as transcript/evidence (harness/`docs/evidence/…`).

## 5. Exit Criteria (CE — each CI-verifiable)
- **CE-AH-1 (`DC-NODE-20` forge-base authority) [S1]:** new hermetic test in `ade_node::node_sync`
  — post-self-admit, the forge builds on `ChainDb::tip` with `durable_servable_tip != followed_peer_tip`
  and **no cert file present**; the direct `CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine`
  transition. New gate `ci/ci_check_local_durable_forge_base.sh` green.
- **CE-AH-2 (`DC-NODE-20` fence) [S1]:** hermetic negative tests — the local-tip forge base **fails
  closed** when any of the 6 conditions fails (incl. the observed-feed competing-candidate predicate
  tripping on any peer-origin non-spine block); no silent fallback to followed/cert. Asserted by
  `ci_check_local_durable_forge_base.sh`; `ci_check_forge_followed_tip_admission.sh` **phase-split** +
  green; `ci_check_node_run_loop_containment.sh` stays green.
- **CE-AH-3 (`DC-NODE-21` cert evidence-only) [S2]:** `read_adoption_cert` removed from
  forge-base/`proceed_to_forge`; new gate `ci/ci_check_cert_evidence_only.sh` asserts the cert is
  never read into the forge path and never appears in multi-producer/preprod/production forge paths;
  `ci_check_node_path_fidelity.sh` stays green.
- **CE-AH-4 (replay-equivalence) [S3]:** new hermetic tests over the local-tip-derived
  post-self-admit chain — two clean runs byte-identical + kill/warm-start byte-identical (all four
  surfaces) + feed-end appends nothing to WAL (T-REC-03/05; reuses the N-AG `s2_extend_lead` harness).
  `cargo test -p ade_node` green.
- **CE-AH-5 (core acceptance — hermetic end-to-end) [S1]:** a test driving catch up once →
  self-admit first own block via `pump_block` → forge **N+1** on `ChainDb::tip` → forge **N+2**
  sustained on the local spine, **no cert in the forge path** (forged ≥ 2 own blocks).
- **CE-AH-6 (operator-gated live = re-homed CE-AF-6b) [S4]:** committed transcript
  `docs/evidence/phase4-n-ah-local-tip-forge.{md,jsonl}` — sustained > k Ade-forged blocks settle
  into the relay's ImmutableDB across ≥ 1 follow-link EOF, forge base derives from local `ChainDb::tip`,
  warm-start byte-identical; rung1-auto C2-LOCAL; verbatim `--mode node`.
- **CE-AH-7 (close) [/cluster-close]:** `DC-NODE-20` + `DC-NODE-21` flipped declared→enforced
  (tests + ci_scripts appended); strengthen `DC-NODE-05`/`DC-NODE-12`/`DC-NODE-15`/`DC-NODE-18`/`DC-NODE-19`
  (`DC-CONS-03` untouched); PHASE4-N-AG superseded/partial-close bookkeeping (hermetic core complete;
  live CE re-homed; DC-NODE-19 declared/partial); 4 grounding docs refreshed (incl. the CODEMAP+SEAMS
  deferred from N-AF baseline `f87d0056`).

## 6. Expected Slices
- **S1** `DC-NODE-20` forge-base authority rewire — **one sealed slice** (local `ChainDb::tip` base +
  direct `CaughtUpToPeerTip → extend` (no cert-wait) + fail-closed 6-condition fence + the phase-split
  followed-tip-admission gate + hermetic N/N+1/N+2) — CE-AH-1, CE-AH-2, CE-AH-5. GREEN+RED; BLUE
  unchanged.
- **S2** `DC-NODE-21` cert evidence-only — cert **read** off the forge path; cert **write** preserved
  as evidence; the prohibition gate — CE-AH-3. RED + CI.
- **S3** replay-equivalence — T-REC-03/05 over the local-tip-derived post-self-admit chain — CE-AH-4.
  Tests over existing BLUE/RED (no production change).
- **S4** operator-gated live acceptance — re-homed CE-AF-6b on the DC-NODE-20 path — CE-AH-6. RED
  harness + evidence.
- **close** — CE-AH-7 via `/cluster-close`.

## 7. TCB Color Map
- **BLUE [unchanged — no new authority]:** `ade_core`/`ade_runtime` `ChainDb::tip`, `pump_block`,
  `forge_one_from_recovered`, `block_validity`/`prior_fp`.
- **GREEN:** `ade_node::node_sync` — `forge_mode_after_admit` (the cert-free transition) + the
  venue/fence machinery (forge-base selection, the observed-feed competing-candidate predicate).
- **RED:** `ade_node::node_lifecycle` — `run_relay_loop` `proceed_to_forge` gate rewire; the demoted
  `read_adoption_cert` (evidence-only); the operator harness.
- **Affected gates:** `ci_check_forge_followed_tip_admission.sh` (**phase-split** — initial-catch-up
  requires `durable==followed`, post-self-admit local-tip does not), `ci_check_node_run_loop_containment.sh`
  + `ci_check_node_path_fidelity.sh` (stay green); new `ci_check_local_durable_forge_base.sh` (S1) +
  `ci_check_cert_evidence_only.sh` (S2).
- `run_loop_planner` — **NOT touched** (OQ-AH-1: it decides *when* to forge, not the base).

## 8. Forbidden During This Cluster (slice-level hard prohibitions inherit)
- **No new BLUE authority** (reuse `ChainDb::tip` + `pump_block`).
- **No cert read in the forge-base/`proceed_to_forge` path** (`DC-NODE-21`).
- **No forge-loop path may require `FirstOwnBlockServed` + cert to enter `SingleProducerExtendOwnDurableSpine`.**
- **No fork-choice / chain-selection in the fence** (`DC-CONS-03` untouched) — a competing candidate
  → fail closed, **never** resolved (no classification cleverness).
- **No silent fallback** to `followed_peer_tip` or the cert when the fence fails.
- **No weakening of `DC-NODE-15`'s initial catch-up gate** — it stays for
  `InitialCatchupRequired → CaughtUpToPeerTip` (the gate is phase-split, not removed).
- `pump_block` stays the sole durable admit authority (`DC-NODE-05`); the forge advances no tip directly.
- **No new `cli.rs` flag.** No multi-producer / preprod / keep-alive (OQ-KA) work.

## 9. Replay Obligations
S3 introduces the post-self-admit **local-tip-derived forged-successor** replay corpus (hermetic;
reuses the WAL/ChainDB — **no new canonical type**). Key strengthening: the forge base now derives
from the local durable spine **alone** — the RED cert/timing is removed from the authority path, so
T-REC-03/05 extend to cover cert-free local-tip-derived successors.

## 10. Open Questions
- OQ-AH-1 → **resolved:** no `run_loop_planner` touch; rewire is `node_lifecycle` + `node_sync` only.
- OQ-AH-2 → **resolved:** competing-block predicate = any peer-origin candidate block observed after
  local-tip authority is active that is **not already part of Ade's admitted local spine / own-served
  lineage**. In rung 1 this **fails closed**; no fork resolution is attempted. Condition 6 is derived
  from this signal.
- OQ-AH-3 → **resolved:** S2 removes the cert read; cert write preserved as evidence.
- **OPEN at slice-doc (S1):** the exact `ForgeMode` enum shape after dropping the cert-wait —
  **best:** remove `FirstOwnBlockServed` if all uses become evidence-only/obsolete; **acceptable
  transitional:** keep the variant temporarily ONLY if no production forge transition parks there
  waiting for the cert. Hard requirement (above) holds either way.

## 11. Cluster Close Record
*OPEN — filled at `/cluster-close` (CE-AH-7).*

## 12. Follow-ons & Notes (surfaced during S1)
- **AH-FOLLOW-1 (rung-1 hardening; NOT an S1 blocker):** broaden the DC-NODE-20
  competing-block fence from the observed-tip `block_no`/hash checks
  (`CompetingPeerBlockBeyondAdoptedRoot` / `PeerTipDisagreesWithSpine`) to a RED-computed
  "peer-origin candidate not in Ade's admitted spine / own-served lineage" flag (ChainDb
  spine-membership of the observed tip) threaded into the GREEN fence. Classify as
  **rung-1 hardening before multi-producer / rung 2**, not a blocker for the S1 cert
  authority correction. (The existing fence already fails closed on competing tips
  ahead-of / diverging-at the spine head + adopted root.)
- **`ci_check_forge_followed_tip_admission.sh` — pre-existing stale grep:** its loop
  grep for `forge_followed_tip_admission(` is stale (the call lives in `dc_node_15_refusal`,
  not the loop body); **identical on HEAD and current**. Unchanged by S1; DC-NODE-15
  covered code not touched. **This gate did NOT pass during S1 verification** (it fails
  pre-existing). Repair owed separately or during AH close if required by project gate
  discipline — not claimed as passing.
- **Cert parser retained (S1 only):** `read_adoption_cert` / `parse_hex32` are
  `#[allow(dead_code)]` — not used by forge authority; retained for S2 (DC-NODE-21)
  evidence-only transcript work, which removes or fences the parser's final role and adds
  `ci_check_cert_evidence_only.sh`. *(Resolved in S2 `050237e9`: parser fully deleted.)*
- **Path-fidelity reconciliation (S2 premise correction):** CN-REHEARSAL-FIDELITY-01
  preserved: pinned flag set reconciled from **28→29** to include the pre-existing
  legitimate `--single-producer-venue` flag; `--adoption-cert-path` removed from cli.rs
  and never added to the allow-list. (The S2 slice doc's assumed "28→27" was a premise
  error — `--adoption-cert-path` was never in the pinned set; the real divergence was the
  N-AF-introduced `--single-producer-venue`, legitimately missing from PINNED since N-AF.
  The allow-list stays closed: no from-genesis/devnet/backdoor flag added.)
