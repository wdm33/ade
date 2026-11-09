# Cluster/Slice Plan — Ade · PHASE4-N-F (Operator-Production Wiring, BA-02)

> **Status:** overall cluster/slice plan (IDD `/cluster-plan` output). No registry promotion,
> no code, no cluster close. Source invariants: `docs/planning/phase4-n-f-invariants.md`.
> **Split decision recorded below** — an S1 provenance drill found a constitutional seam.

## Why this is two clusters (the S1 provenance drill)

The naïve single cluster would have been **unsafe**. The drill answered: *where do seed-epoch
`eta0 / PoolDistr / ASC / pool_vrf_keyhashes` live after verified bootstrap, and does recovery
restore them byte-identically before produce consumes them?*

- **eta0** — recovered surface **exists**: `PraosChainDepState.epoch_nonce`; `RecoveredNode { ledger, chain_dep, tip }` (`recovery/restart.rs:49-52`); `bootstrap_initial_state → (LedgerState, PraosChainDepState, Option<ChainTip>)`. *(But `produce_mode` today re-seeds it from the operator bundle on cold-start rather than consuming recovered `chain_dep` — rewire needed.)*
- **PoolDistr + ASC + pool_vrf_keyhashes** — **no recovered surface today.** `BootstrapAnchor` does not carry them (`bootstrap_anchor/anchor.rs:62-69`); `RecoveredNode` does not carry them; `recover_node_state` and `bootstrap_initial_state` take `ledger_view: &dyn LedgerView` as an **external input** (`restart.rs:117`, `bootstrap.rs:49`). They exist **only** in `LiveConsensusInputsCanonical { active_slots_coeff, epoch_nonce, pool_distribution, pool_vrf_keyhashes }` (`consensus_inputs/canonical.rs:54-64`), read from `--consensus-inputs-path` **each run**.

→ **Outcome B** (addable as a small anchor-bound recovered surface; *not* C — seed-epoch constants need no rotation). Because PoolDistr/ASC are a **new authoritative persisted state surface**, they must be replay-tested independently *before* produce consumes them. Hence the split. Without this split, produce would still need PoolDistr/ASC at forge time and would be tempted to re-read `--consensus-inputs-path` during produce — the old operator-graft model in disguise.

## Cluster Index (dependency order)
1. **PHASE4-N-F-A — Seed-Epoch Consensus Input Provenance** — primary invariant: *the seed-epoch consensus inputs (eta0, PoolDistr, ASC, pool_vrf_keyhashes) established during verified bootstrap are persisted as an anchor-bound canonical surface and recovered byte-identically — `bootstrap → persist → recover` yields the same inputs — before any producer consumes them.*
2. **PHASE4-N-F-B — Operator-Production Wiring (BA-02)** — primary invariant: *produce forges block #(tip+1) on Ade's recovered selected tip, using consensus inputs that were established during verified bootstrap and persisted/recovered as part of Ade state, **not supplied at forge time**; BA-02 acceptance is proven only by the Haskell peer.*

## Non-negotiable dependency (the core safety line)

**N-F-B depends on N-F-A. N-F-B may not start until N-F-A has mechanically proven:**

> `verified bootstrap anchor + persisted seed-epoch consensus inputs + recovered ChainDb/WAL/checkpoint = the same eta0 / PoolDistr / ASC / pool_vrf_keyhashes after restart`

Until that replay-byte-identity holds, PHASE4-N-F may **not** claim "Ade-derived selected tip" — it would be "operator-supplied consensus bundle at forge time," which is rejected.

---

## PHASE4-N-F-A — Seed-Epoch Consensus Input Provenance (predecessor)

- **Primary invariant:** as above. Single-epoch (the seed epoch); does **not** compute or rotate stake (cross-epoch rotation is a separate predecessor and an explicit non-goal).
- **Depends on:** N-Y / N-Z (bootstrap anchor + recovery, shipped). No other dependency.
- **TCB partition:** **BLUE** [the closed persisted consensus-input type + version-gated codec + anchor binding + the projection to `PoolDistrView` / `ExpectedVrfInput`]; **GREEN** [the carry/restore reducer]; **RED** [persist write, recovery read].
- **Cluster Exit Criteria:**
  - **CE-A-1** — a closed canonical surface carries seed-epoch (eta0, PoolDistr, ASC, pool_vrf_keyhashes), **bound to the `BootstrapAnchor`** (anchor `ANCHOR_SCHEMA_VERSION` bump or a checkpoint sidecar); version-gated decode (rejects unknown version); round-trips byte-canonically.
  - **CE-A-2** — bootstrap **populates it during verified bootstrap from the documented seed extraction path, binds it to the `BootstrapAnchor`, and persists it before recovery/produce can consume it. The forge-time `--consensus-inputs-path` is fenced out** (CI containment, N-Z `ci_check_mithril_seed_point_independence.sh` style) — no forge-time path may populate this surface.
  - **CE-A-3** — recovery restores the surface **byte-identically**: replay test `bootstrap + persist + recover = same eta0/PoolDistr/ASC/pool_vrf_keyhashes` (extends T-REC-01/02). *This is the core safety line above.*
  - **CE-A-4** — a projection API yields `PoolDistrView` + `ExpectedVrfInput` from the **recovered** surface, replacing `pool_distr_view_from_consensus_inputs(operator bundle)` on the bounty-primary path.
- **Slices:**
  - **A1** — define the closed canonical persisted consensus-input surface + anchor binding + version-gated codec — addresses CE-A-1 — TCB: BLUE.
  - **A2** — bootstrap populates it (bootstrap-time only) from the documented seed extraction; fence the forge-time `--consensus-inputs-path` out — addresses CE-A-2 — TCB: RED/GREEN + CI gate.
  - **A3** — recovery restores it byte-identically + replay-equivalence test — addresses CE-A-3 — TCB: RED restore + BLUE replay.
  - **A4** — projection API: recovered surface → `PoolDistrView` + `ExpectedVrfInput` — addresses CE-A-4 — TCB: BLUE (`consensus_view`).
- **Replay obligations:** **new persisted authoritative surface** — recovery must restore it byte-identically (the core proof, CE-A-3). New BLUE canonical type (version-gated, anchor-bound). New corpus: a `bootstrap → persist → recover` replay fixture + a pinning fixture (recovered eta0/PoolDistr/ASC = seed-epoch peer-equivalent).

## PHASE4-N-F-B — Operator-Production Wiring (BA-02)

- **Primary invariant:** as above (tightened — does not imply forward-sync *derived* PoolDistr).
- **Depends on:** **PHASE4-N-F-A** (recovered consensus-input surface + projection API; the CE-A-3 byte-identity proof) + shipped N-Y / N-Z. **Cannot start until N-F-A's core safety line holds.**
- **TCB partition:** **BLUE** [leader_check, vrf_cert leader-input, forge, self_accept, unsigned_header_pre_image, the canonical slot input, the single-epoch guard, the deterministic opcert/genesis parse-verdict]; **GREEN** [`select_forge_base`, `produce_run_guard`, the evidence-correlation reducer, the coordinator]; **RED** [operator-file I/O, wall-clock observation, KES/VRF custody+signing, ChainDb/recovery read, peer-log read].
- **Cluster Exit Criteria:**
  - **CE-B-1** — forge base (parent/block_no/ledger/chain_dep) is the recovered ChainDb selected tip via the single `bootstrap_initial_state` warm path, and eta0/PoolDistr/ASC come from the **N-F-A recovered surface** — never re-seeded from the operator bundle; test builds block #(tip+1) on the tip hash.
  - **CE-B-2** — `--opcert` / `--genesis-file` load via `parse_opcert_envelope` / `parse_shelley_genesis`; `parse_simple_*` removed from the produce path (CI gate); parse-verdict color classified (BLUE/GREEN), file I/O RED.
  - **CE-B-3** — protocol_version / prev_opcert_counter / pparams derive from recovered state + real opcert/genesis, not defaults; opcert counter never reused/skipped.
  - **CE-B-4** — explicit canonical slot; BLUE reads no clock; `SlotDrift` structured fail-closed; cross-epoch boundary fail-closed (no stale-eta0 forge).
  - **CE-B-5** — the forge-time operator bundle + any seed-point graft are fenced **diagnostic-only** (CI containment): no BA-02 evidence, no sync claim, no durability-as-synced; produce makes **one** BA-02 attempt per run; restart-after-signing fail-closes absent N-U; no auto-retry/resume.
  - **CE-B-6** — BA-02 evidence manifest is **peer-acceptance-only** (forged-block-hash ↔ peer accept log); schema + correlation CI-gated.
  - *(NOT a CE — open obligation:* live cardano-node acceptance of the Ade-forged block — operator-pass-gated, RO-LIVE-01 / CN-CONS-06, held to NF-7. No carry-forward.)
- **Slices:**
  - **B1** — Ade-derived selected-tip handoff: produce opens the persistent ChainDb + recovery warm-start via `bootstrap_initial_state` → `ForgeBase`, **consuming the N-F-A recovered consensus inputs** (not the operator bundle) — addresses CE-B-1 — TCB: GREEN reducer + RED ChainDb/recovery read + BLUE bootstrap.
  - **B2** — real operator-file ingress (G1/G2) — addresses CE-B-2 — TCB: RED I/O + BLUE/GREEN parse-verdict.
  - **B3** — live header inputs (G4) — addresses CE-B-3 — TCB: GREEN derive + BLUE consume.
  - **B4** — explicit canonical slot + single-epoch guard (G7 / NF-6) — addresses CE-B-4 — TCB: RED align + BLUE slot/guard.
  - **B5** — diagnostic-graft fence + single-shot/non-restartable fence (RD-OQ2 / RD-OQ3) — addresses CE-B-5 — TCB: GREEN guards + RED restart-detect + CI gates.
  - **B6** — BA-02 evidence correlation (mechanical); live capture stays operator-gated — addresses CE-B-6 — TCB: GREEN reducer + RED peer-log read.
- **Replay obligations:** `(recovered ChainDb + N-F-A recovered consensus inputs + canonical slot) → byte-identical ForgeBase + forged block + filtered ProducerLogEvent` (extends DC-PROD-02). New canonical types: `ForgeBase`, `CanonicalSlot`. New corpus: a recovered-tip → forge replay fixture.

---

## Candidate registry entries (proposed only — NOT promoted here)

From the sketch, re-homed across the split. New: **N-F-A** — a consensus-input-provenance family (e.g. `CN-PROVENANCE-01` surface + binding, `DC-PROVENANCE-01` recovery byte-identity, `DC-PROVENANCE-02` projection API). **N-F-B** — `CN-PROD-05` (tip+recovered-inputs base), `DC-PROD-04` (live header inputs), `DC-PROD-05` (canonical slot/SlotDrift), `CN-PROD-06` (diagnostic fence), `DC-PROD-06` (single-epoch), `DC-PROD-07` (single-shot). Strengthenings: `CN-NODE-01`, `CN-ANCHOR-01`/`DC-ANCHOR-01` (anchor surface), `CN-OPCERT-01`, `CN-GENESIS-01`, `CN-KES-HEADER-01`, `RO-LIVE-01`, `CN-CONS-06`, `CN-OPERATOR-EVIDENCE-01`, `T-CONS-02`, `T-REC-01/02`. Exact per-family IDs assigned at slice-doc/implement time, not now.

## Next steps (not taken here)
- `/cluster-doc PHASE4-N-F-A` then `/cluster-doc PHASE4-N-F-B` to expand each. **Implement N-F-A first**; N-F-B is blocked on N-F-A's CE-A-3 recovery-byte-identity proof.
- Bounty BA-02 *live* half stays gated on the operator-pass peer-acceptance evidence (RO-LIVE-01 / CN-CONS-06) + preprod stake (~10-day) for C2.
