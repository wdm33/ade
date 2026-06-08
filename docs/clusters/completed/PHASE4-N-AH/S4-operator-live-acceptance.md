# Invariant Slice — Operator-gated live acceptance of the DC-NODE-20 path (PHASE4-N-AH S4)

## 2. Slice Header
**Slice Name:** Operator-gated live acceptance — cert-free local-tip forging sustained past follow-link EOF (re-homed CE-AF-6b)
**Cluster:** PHASE4-N-AH — local selected durable chain forge-base authority; **rung-1, single-producer only**
**Status:** Proposed — **OPERATOR-GATED** (no live run until this doc is committed AND the operator green-lights)
**Authority source:** `docs/clusters/PHASE4-N-AH/cluster.md` (§4, CE-AH-6); registry `DC-NODE-20` (live half) + `DC-NODE-19` core (sustained-past-EOF, re-homed)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AH-6 (operator-gated live = re-homed CE-AF-6b):** committed transcript — sustained > k Ade-forged blocks settle into a real non-producing Haskell relay's immutable surface across ≥1 follow-link EOF, the forge base deriving from local `ChainDb::tip` with **no cert anywhere in Ade's forge path**, and warm-start recovers byte-identically; rung1-auto C2-LOCAL.

Exit criteria not listed (CE-AH-1/2/5=S1; CE-AH-3=S2; CE-AH-4=S3; CE-AH-7=close) are out of scope.

**Slice Dependencies:** S1 (`b0fb8817`, DC-NODE-20 local-tip forge base), S2 (`050237e9`, DC-NODE-21 cert removed), S3 (`dad29b43`, replay-equivalence). All production is already merged; S4 adds **no repo production or test code** — it is the operator-witnessed live acceptance of the path S1–S3 built.

## 3. Implementation Instruction (operator-gated)
**No live run until this doc is committed and the operator green-lights.** Then: (1) apply the harness prerequisite (below) outside the repo; (2) run the live pass on rung1-auto C2-LOCAL; (3) capture the JSONL transcript + write the narrative; (4) commit the transcript + narrative + the validation gate. The validation gate `ci/ci_check_phase4_n_ah_live_evidence.sh` parses the committed transcript and mechanically asserts the §12 bar + negative evidence. **Zero repo production/test change** (the production is S1–S3). `DC-NODE-20`/`DC-NODE-21` stay `declared` until CE-AH-7 close. Commit carries the repo's model trailer.

## 4. Intent
Convert DC-NODE-20's enforcement from **hermetic-only** to **hermetic + operator-witnessed**: prove that on a real wire, against a real non-producing Haskell relay, Ade forges on its own local durable spine (`ChainDb::tip`) and **sustains** that across at least one follow-link EOF — the exact run-4 stall the pivot was built to eliminate — with the adoption certificate absent from Ade's forge path entirely. The live transcript is **GREEN evidence** (a committed observation that the BLUE/GREEN/RED stack behaved as specified); it is not new authority, and wire success is reported as wire success, adoption as adoption, settlement as settlement — never conflated.

## 5. Scope
- **The operator live run** on rung1-auto C2-LOCAL (cert-free; see §9) + the committed evidence artifacts:
  - `docs/evidence/phase4-n-ah-ce-ah-6-live-transcript.jsonl` — the closed-vocabulary event log.
  - `docs/evidence/phase4-n-ah-ce-ah-6-narrative.md` — the narrative, **including the verbatim rung1-auto command line** (showing no `--adoption-cert-path`) and the relay's adoption/ImmutableDB excerpts.
  - `ci/ci_check_phase4_n_ah_live_evidence.sh` — the transcript-validation gate (the §12 bar + negatives).
- **The harness prerequisite** (§ Harness, below) — an operator-scratch edit recorded in the narrative, **not a repo code change**.
- **Out of scope:** any repo production/test code (S1–S3); flipping DC-NODE-20/21 (CE-AH-7 close); multi-producer / preprod / rung-2 / real fork-choice.

## 6. Execution Boundary (TCB color)
- **BLUE (UNCHANGED, exercised live):** `ChainDb`/`pump_block` durable admit, `block_validity`, `warm_start_recovery` forward-replay.
- **GREEN (exercised live):** DC-NODE-20 forge-base selection (`forge_mode_after_admit` direct extend, `single_producer_forge_decision`) + the committed transcript (GREEN evidence) + the validation gate.
- **RED (exercised live):** `run_relay_loop`, the n2n dialer/mux, the rung1-auto harness (operator scratch).
- No new authority of any color; the only repo additions are committed **data** (the transcript/narrative) + a **GREEN validation gate**.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-20` (forge base = local durable tip — the run must derive every forge base from `ChainDb::tip`, never a peer tip or cert) · `DC-NODE-21` (cert evidence-only — the run uses no cert in Ade's forge path) · `DC-NODE-05`/`DC-NODE-12` (`pump_block` sole durable admit) · `DC-NODE-15` (initial catch-up gate) · `DC-NODE-18` core / `DC-NODE-19` core (own-spine forge / continue-past-EOF — the sustained-past-k live half re-homes here) · `T-REC-03`/`T-REC-05` (warm-start byte-identical, now live) · `DC-CONS-03` (untouched; rung-2 successor) · the closed live-evidence vocabulary discipline (wire ≠ admission ≠ settlement; JSONL events are a closed allow-list).

## 8. Invariants Strengthened or Introduced
**Strengthens `DC-NODE-20`** — adds the **operator-witnessed live transcript** as its evidence (the live half of the local-tip forge-base invariant), and **re-homes the DC-NODE-19 sustained-past-EOF live evidence** onto the DC-NODE-20 path (superseding the cert-promotion entry that CE-AF-6b originally exercised). Exactly **one** family (the DC-NODE-20 live-acceptance evidence). The transcript is GREEN evidence, not authority; DC-NODE-20 flips declared→enforced at CE-AH-7 close, where this transcript is cited in its `evidence_notes`. Until the operator pass runs, CE-AH-6 is `blocked_until_operator_pass_executed`.

## 9. Design Summary — the live path is now cert-free inside Ade
- **rung1-auto must NOT pass `--adoption-cert-path`** (S2 deleted the flag; passing it now is a `cli UnknownFlag` startup error).
- **Adoption evidence MAY still be captured** by the harness / log bundle (the relay's `AddedToCurrentChain` + ImmutableDB excerpts) — that is operator/harness evidence, outside Ade.
- **Ade MUST NOT read an adoption cert** — there is no cert parser in the node (S2; `ci_check_cert_evidence_only.sh`).
- **The forge base MUST derive from local `ChainDb::tip`** — `forge_mode_after_admit` enters `SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip}` directly on self-admit (S1; `ci_check_local_durable_forge_base.sh`).

## 10. Changes Introduced
- The committed transcript + narrative (data) + the validation gate (GREEN). **No production/test code.**

## 11. Replay, Crash, and Epoch Validation
- **Crash/warm-start (live):** the run includes a kill + warm-start; the recovered durable tip + served chain must match pre-kill byte-identically (T-REC-05, now over the live local-tip chain) — recorded as `warm_start_recovered` in the transcript with matching fingerprints.
- The hermetic replay proof is S3 (`local_spine_*` tests); S4 is its live confirmation.
- **Epoch:** not applicable.

## 12. Mechanical Acceptance Criteria
**The committed transcript must exhibit the bounded bar below, validated by `ci/ci_check_phase4_n_ah_live_evidence.sh`. Gated on the operator pass (`blocked_until_operator_pass_executed`).**

**Acceptance bar (positive — all eight, in order):**
- [ ] 1. Ade catches up once (`caught_up_to_peer_tip`).
- [ ] 2. Ade self-admits its first own block via `pump_block` (`block_admitted`, own issuer).
- [ ] 3. Ade directly enters local-tip extend mode (`forge_mode_extend`, `SingleProducerExtendOwnDurableSpine`; no `FirstOwnBlockServed`).
- [ ] 4. Ade forges successors from `ChainDb::tip` with no cert path (`forge_succeeded` ×, `forge_base = chaindb_tip`).
- [ ] 5. A real non-producing Haskell relay adopts the blocks (relay `AddedToCurrentChain`; relay `forging = 0`).
- [ ] 6. Ade continues across ≥1 follow-link EOF (`follow_link_eof` ≥ 1, each followed by continued `forge_succeeded`).
- [ ] 7. > k Ade-forged blocks settle into the relay's immutable surface (`settled_immutable` count > k).
- [ ] 8. Warm-start recovers to the same durable tip / served chain (`warm_start_recovered`, matching fingerprints).

**Negative evidence (must all hold):**
- [ ] No `--adoption-cert-path` flag (absent from the verbatim rung1-auto command line in the narrative; the binary rejects it — `ci_check_node_path_fidelity.sh`).
- [ ] No adoption cert read by Ade (`ci_check_cert_evidence_only.sh` green — no cert parser in the node).
- [ ] No cert file in forge authority (`ci_check_cert_evidence_only.sh` + `ci_check_local_durable_forge_base.sh` green).

## 13. Failure Modes
If the live run stalls after the first own block (the run-4 `no_tip_available` failure), the bar is **not met** and S4 does not ship — that would mean DC-NODE-20 did not eliminate the cert-authority dependency on the wire. S1–S3 closed that hermetically; S4 is the live confirmation, and it fails closed (the bar is exact, not "best effort").

## 14. Hard Prohibitions
**Inherited (cluster §8):** no cert in Ade's forge path; no new authority of any color; no fork-choice; no weakening of DC-NODE-15 / DC-NODE-20.
**Slice-specific:**
- **No repo production or test code** — S4 is evidence + a validation gate only.
- **The rung1-auto harness MUST NOT pass `--adoption-cert-path`** — the live path is cert-free inside Ade.
- **No live run** until this doc is committed AND the operator green-lights.
- **Do not** overstate the transcript — wire success ≠ adoption ≠ immutable settlement; each is a distinct event.
- **Do not** run `cargo fmt -p ade_node` (cluster.md §12 lesson); **do not** touch the pre-existing-stale `ci_check_forge_followed_tip_admission.sh`.

## 15. Explicit Non-Goals
The hermetic core (S1–S3) · flipping DC-NODE-20/21 + the grounding-doc refresh (CE-AH-7 close) · multi-producer / preprod / rung-2 / real fork-choice (DC-CONS-03) · the competing-block fence broadening (AH-FOLLOW-1).

## 16. Completion Checklist
- [ ] This doc committed; operator green-light obtained.
- [ ] Harness prerequisite applied (no `--adoption-cert-path`), recorded in the narrative.
- [ ] Live pass run; transcript + narrative + `ci_check_phase4_n_ah_live_evidence.sh` committed.
- [ ] The §12 bar (8 positive) + negative evidence (3) all hold; the validation gate is green.
- [ ] `DC-NODE-20` + `DC-NODE-21` still `declared` (flip at CE-AH-7 close).

## Harness prerequisite (operator scratch — NOT a repo change)
Before the live run, remove `--adoption-cert-path` from `~/.cardano-rung1-host/rung1-auto.sh` (S2 deleted the flag). This is an operator-scratch edit, **outside the repo**, recorded verbatim in `docs/evidence/phase4-n-ah-ce-ah-6-narrative.md` (the command-line excerpt) — never committed as repo code (competition secrecy + the harness lives in `~/.cardano-rung1-host/`).
