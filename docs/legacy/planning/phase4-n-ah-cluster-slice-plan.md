# Cluster/Slice Plan — Ade · PHASE4-N-AH (local selected durable chain forge-base authority)

## Cluster Index (Dependency Order)
1. PHASE4-N-AF — DC-NODE-18 (extend-own-spine via cert-promotion) — **enforced** (prior).
2. PHASE4-N-AG — DC-NODE-19 (continue-past-EOF in the extend state). **Hermetic core complete; live
   sustained proof superseded/re-homed to PHASE4-N-AH; DC-NODE-19 remains declared/partial until
   exercised on the local-tip authority path.** (Not failed or wasted — it produced the hermetic
   loop machinery; the live CE moved because the architecture pivoted.)
3. **PHASE4-N-AH — DC-NODE-20 (local selected durable chain forge-base authority) + DC-NODE-21
   (adoption cert evidence-only)** — the correcting dependency that retires the cert as forge
   authority. ← THIS PLAN.

## Cluster PHASE4-N-AH — local selected durable chain forge-base authority
- **Primary invariant:** DC-NODE-20 — in a declared rung-1 single-producer venue, after Ade
  self-admits a valid forged block through `pump_block` onto its local durable ChainDB spine, the
  next forge base is the local selected durable tip (`ChainDb::tip`), not `followed_peer_tip` and
  not the operator adoption cert; fenced to 6 conditions, fail-closed. **Paired (not folded):**
  DC-NODE-21 — the cert is rung-1 evidence-only, never forge authority (its own check + future-removal
  boundary). DC-CONS-03 (fork-choice) = rung-2 successor, untouched.
- **Entry conditions (prior clusters guarantee):** DC-NODE-05 (`pump_block` sole durable admit
  authority), DC-NODE-12 (own-forged admit chokepoint), DC-NODE-15 (initial catch-up gate),
  DC-NODE-18 (extend-own-spine forge core), DC-NODE-19 (continue-past-EOF, hermetic), DC-CONS-03,
  T-REC-03/05; `ChainDb::tip` + the `proceed_to_forge` gate (`node_lifecycle` ~1221–1492); the
  rung1-auto C2-LOCAL harness (σ=0.5 + dir/race fixes from run-4).
- **TCB partition:**
  - **BLUE [unchanged — no new authority]:** `ade_core`/`ade_runtime` `ChainDb::tip`, `pump_block`,
    `forge_one_from_recovered`, `block_validity`/`prior_fp`.
  - **GREEN:** `ade_node::node_sync` ForgeMode transition + the venue/fence machinery (forge-base
    selection, the observed-feed competing-block predicate). (`run_loop_planner` only if the venue
    table is touched — OQ-AH-1.)
  - **RED:** `ade_node::node_lifecycle` `run_relay_loop` loop + the demoted `read_adoption_cert`
    (evidence-only); the operator harness.

- **Cluster Exit Criteria (each CI-verifiable):**
  - **CE-AH-1 (DC-NODE-20 forge-base authority):** hermetic test — post-self-admit, Ade forges the
    successor on `ChainDb::tip` with `durable_servable_tip != followed_peer_tip` and **no cert file
    present**; the direct `CaughtUpToPeerTip → SingleProducerExtendOwnDurableSpine{current_tip=ChainDb::tip}`
    transition (FirstOwnBlockServed folded out). Gate: `ci_check_local_durable_forge_base.sh`.
  - **CE-AH-2 (DC-NODE-20 fence):** hermetic negative tests — the local-tip forge base **fails
    closed** when ANY of the 6 conditions fails (incl. the observed-feed competing-block predicate
    tripping on a competing block in the canonical receive stream); no silent fallback to
    followed/cert. Asserted by the same gate.
  - **CE-AH-3 (DC-NODE-21 cert evidence-only):** `read_adoption_cert` removed from the
    forge-base/`proceed_to_forge` path (the node no longer **reads** the cert to decide forging);
    cert **writing** is preserved only as transcript/evidence capture (operator harness +
    `docs/evidence/…` bundles). `ci_check_cert_evidence_only.sh` asserts: the cert is never read into
    forge-base/`proceed_to_forge` logic, and never appears in multi-producer/preprod/production forge
    paths.
  - **CE-AH-4 (replay-equivalence):** two clean runs byte-identical + kill/warm-start byte-identical
    over the local-tip-derived post-self-admit chain (T-REC-03/05); feed-end appends nothing to WAL
    (reuses the DC-NODE-19 S3 pattern).
  - **CE-AH-5 (core acceptance — hermetic end-to-end):** catch up once → self-admit first own block
    via `pump_block` → forge N+1 on `ChainDb::tip` → forge **N+2** sustained on the local spine,
    **no cert in the forge path** (forged ≥ 2 own blocks).
  - **CE-AH-6 (operator-gated live = re-homed CE-AF-6b):** committed transcript — sustained > k
    Ade-forged blocks settle into the relay's ImmutableDB across ≥ 1 follow-link EOF, forge base
    derives from local `ChainDb::tip`, warm-start byte-identical; rung1-auto C2-LOCAL.
  - **CE-AH-7 (close):** DC-NODE-20 + DC-NODE-21 flipped declared→enforced; strengthen
    DC-NODE-05/12/15/18/19 (DC-CONS-03 untouched); PHASE4-N-AG superseded/partial-close bookkeeping;
    4 grounding docs refreshed (incl. the CODEMAP+SEAMS deferred from N-AF baseline `f87d0056`).

- **Slices:**
  - **S1 forge-base authority rewire** — invariant: DC-NODE-20 — addresses: CE-AH-1, CE-AH-2,
    CE-AH-5 — TCB: GREEN (forge-base selection/fence) + RED (loop wiring); BLUE unchanged.
    **One sealed invariant slice — the smallest safe mergeable unit:** local `ChainDb::tip` forge-base
    + direct self-admit → extend transition (FirstOwnBlockServed folded out) + the fail-closed
    six-condition fence + no cert/followed fallback + the hermetic N, N+1, N+2 proof. The rewire and
    its fence land together; an unfenced "forge from local tip" slice would be unsafe and
    non-mergeable.
  - **S2 cert evidence-only** — invariant: DC-NODE-21 — addresses: CE-AH-3 — TCB: RED (demoted cert
    read) + the CI gate. Remove cert **reads** from forge authority; preserve cert **writing** only
    as transcript/evidence capture. Rule: the cert MAY be produced by the operator harness; MAY be
    included in evidence bundles; MUST NOT be read by forge-base/`proceed_to_forge` logic; MUST NOT
    appear in multi-producer/preprod/production forge paths.
  - **S3 replay-equivalence** — invariant: T-REC-03/05 over the local-tip-derived post-self-admit
    successors — addresses: CE-AH-4 — TCB: tests over existing BLUE/RED (no production change; reuse
    the DC-NODE-19 S3 harness).
  - **S4 operator-gated live acceptance** — invariant: re-homed CE-AF-6b on the DC-NODE-20 path —
    addresses: CE-AH-6 — TCB: RED (operator harness + evidence); no BLUE/GREEN change.
  - **close (CE-AH-7)** via `/cluster-close`.

- **Replay obligations:** S3 introduces the post-self-admit **local-tip-derived forged-successor**
  replay corpus (hermetic; reuses the WAL/ChainDB — **no new canonical type**). Key strengthening:
  the forge base now derives from the local durable spine **alone** (the RED cert/timing is removed
  from the authority path), so T-REC-03/05 extend to cover cert-free local-tip-derived successors.

- **Open questions (resolve at /cluster-doc):**
  - OQ-AH-1: does S1 touch `run_loop_planner`'s venue-policy table, or is the rewire entirely in
    `node_lifecycle`/`node_sync`? (Planner stays a candidate, not assumed.)
  - OQ-AH-2: the exact observed-feed competing-block predicate source (the canonical receive-stream
    signal; GREEN) — pin the mechanical signal.
  - OQ-AH-3 → **resolved:** S2 removes the cert **read** from forge authority and preserves cert
    **writing** as evidence (the harness may still emit adoption evidence for `docs/evidence/…`); the
    node never reads the cert to decide forging.
```
