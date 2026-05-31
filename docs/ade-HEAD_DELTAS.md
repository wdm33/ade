# Ade — HEAD Deltas (Changes Since Baseline)

> **Status:** Living architectural document. Regenerated; not hand-edited.
>
> Regenerate with `/head-deltas <baseline>` after every cluster close. Baseline is recorded in `.idd-config.json` `head_deltas_baseline`.

> Baseline: `7de1462` (PHASE4-N-F-D close — live relay run-loop, 2026-05-31 19:04)
> HEAD: `cd2484f` (PHASE4-N-F-E S3b — single-epoch / KES fail-closed containment, 2026-05-31 22:30)
> Cluster: **PHASE4-N-F-E — forge-tick (hermetic, single-epoch, self-accept-only)**, slice span closed; close-pass commit to follow.
> 13 commits, 13 files changed, +2317 / -66 lines.

This window narrates the **PHASE4-N-F-E cluster** — wiring the already-enforced
`forge_one_from_recovered` (DC-CINPUT-02b) into the slot path of the N-F-D relay
run-loop. The cluster is **hermetic, single-epoch, and self-accept-only**: a forged
block is a local artifact, advances no durable tip, and is never served, admitted,
broadcast, or gossiped. The sync spine (`run_node_sync → pump_block`) remains the sole
durable tip-advance authority. **No operator-key ingestion. No live peer. No BA-02 /
RO-LIVE claim. No BLUE crate change.** The span also carries a test-integrity repair
(`ffa76fc`) that un-redded the N-F-D `node_sync` suite, which had been broken on `main`
since the N-F-D close.

## 0. Headline

| Count | Baseline (`7de1462`) | HEAD (`cd2484f`) | Δ |
|---|---|---|---|
| CI gates (`ci/ci_check_*.sh`) | 110 | 110 | 0 (two gates **extended in place**, none added) |
| Registry rules | 309 | **310** | +1 (DC-NODE-05 declared) |
| Test attributes (`#[test]`/`#[tokio::test]`, workspace) | 2176 | **2188** | +12 (S1–S3b + `ffa76fc` repairs; ~+25 case-level) |
| BLUE canonical types | 456 | 456 | 0 (no BLUE change) |

> Registry note: at HEAD `cd2484f`, `DC-NODE-05` sits at `status = "declared"`. The
> not-yet-made close-pass commit flips it to `enforced` and records the 6 cross-slice
> strengthenings (mirroring how the N-F-D close-pass flipped its rules after the slice
> span). This doc narrates the slice span; the close-pass commit follows, as in prior
> cluster closes.

## 1. Commit Log (newest first)

| Hash | Type | Summary |
|------|------|---------|
| `cd2484f` | test | PHASE4-N-F-E S3b — single-epoch / KES fail-closed containment |
| `0b832e8` | docs | specify PHASE4-N-F-E S3b fail-closed tests |
| `2484dd1` | test | PHASE4-N-F-E S3a — forge-tick replay-equivalence |
| `0ab9043` | docs | specify PHASE4-N-F-E S3a forge replay |
| `98b488a` | feat | PHASE4-N-F-E S2 — RED forge-tick wiring (self-accept-only) |
| `2ae53d7` | docs | specify PHASE4-N-F-E S2 forge-tick wiring |
| `2980861` | docs | add N-F-D T-REC-03 green-on-close check to N-F-E plan |
| `214b0d3` | feat | PHASE4-N-F-E S1 — GREEN planner forge step |
| `ffa76fc` | fix | restore N-F-D relay replay test integrity |
| `58de369` | docs | add PHASE4-N-F-E S1 slice doc — GREEN planner forge step |
| `c875655` | docs | define PHASE4-N-F-E forge-tick cluster |
| `77901d9` | docs | plan PHASE4-N-F-E forge-tick cluster |
| `de497c4` | docs | declare PHASE4-N-F-E forge-tick invariants |

(Plus the pending close-pass commit: grounding-doc refresh + DC-NODE-05 `declared→enforced`
flip with 6 strengthenings + `.idd-config.json` baseline bump + cluster-doc archive + this
HEAD_DELTAS.)

## 2. New Modules

None. No new module, no new crate, no new WAL/checkpoint/canonical type. All changes
land in the existing RED `ade_node` crate (one GREEN submodule, one RED submodule, plus a
test-only repair).

## 3. Modules Modified

| Module | Color | Key changes |
|--------|-------|-------------|
| `ade_node::run_loop_planner` | **GREEN** | **S1.** `LoopStep` gains a `ForgeTick` variant; new closed `ForgeSlotStatus` (`Due` \| `NotDue`) and the pure monotonic guard `forge_slot_status(current_slot, last_forged_slot) -> ForgeSlotStatus`. `plan_loop_step` stays **content-blind** over the closed step vocabulary — its forge input is the opaque `Due\|NotDue`; leadership eligibility is NOT decided here. The guard is the only place `SlotNo` is consumed. |
| `ade_node::node_lifecycle` | **RED** | **S2.** New `ForgeActivation` (fenced producer-shell forge material) + a `ForgeTick` branch in `run_relay_loop`. The branch derives the current `SlotNo` only through the clock seam (RED observes `SystemClock`; GREEN converts via `millis_to_slot` over `SystemStart` + `EraSchedule` slot length — only `SlotNo` crosses the seam), reuses `kes_period_for_slot`, and makes **exactly one** fenced `forge_one_from_recovered` call. Self-accept-only: advances no durable tip, serves/admits/broadcasts/gossips nothing. |
| `ade_node::node_sync` | **RED** | **`ffa76fc` (test-only):** repaired the N-F-D `node_sync` test suite — restored missing imports, a moved `anchor_fp`, and corrected a size-ordered feed to slot-ordered; fixed the T-REC-03 `code_locus`. Un-redded `cargo test -p ade_node`, broken on `main` since the N-F-D close. **S2/S3a/S3b (`#[cfg(test)]` only):** forge-tick wiring tests, replay-equivalence (S3a), and single-epoch / KES fail-closed containment (S3b). `run_node_sync` itself remains **UNMODIFIED**. |

## 4. Feature Flags

No feature-flag deltas. No `Cargo.toml` changed in the span.

## 5. CI Checks (110 → 110, two extended in place)

| Check | Status | Backs |
|-------|--------|-------|
| `ci_check_loop_planner_closed.sh` | **Extended** (S1) | CN-NODE-02 (planner half). Removes `SlotNo` from the whole-module ban (the pure `forge_slot_status` guard legitimately consumes a `SlotNo`) and adds a **scoped** check: `plan_loop_step` itself must name no `SlotNo`, so step selection stays content-blind over the closed `ForgeSlotStatus`. |
| `ci_check_node_run_loop_containment.sh` | **Extended** (S2) | CN-NODE-02 / DC-SYNC-02 / CE-E-4. The loop may make **exactly one** fenced `forge_one_from_recovered` call (zero ⇒ unwired; >1 ⇒ second forge path). All other forge/evidence tokens (`run_real_forge`, `correlate(`, `Ba02Manifest`) stay forbidden. New neg2b guard: no `served_chain_admit` / `push_atomic` / `OutboundCommand` / `broadcast` / `block_fetch` of a forged block — self-accept-only. |

No CI gate was added or removed; the 110 total is unchanged from the N-F-D baseline.
`ci_check_node_sync_via_pump.sh` stays green (`run_node_sync` unmodified).

## 6. Canonical Type Registry Delta

n/a — no separate canonical-type registry is configured (`canonical_type_registry: null`),
and no BLUE crate changed. The 456 BLUE canonical-type total is unchanged. The new
`ForgeTick` / `ForgeSlotStatus` / `ForgeActivation` types live in the RED/GREEN `ade_node`
crate and are not canonical-counted.

## 7. Normative / Invariant Rule Delta (309 → 310)

### New rule (declared at sketch this cluster)

| ID | Tier | Status @ `cd2484f` | Summary |
|----|------|--------------------|---------|
| `DC-NODE-05` | derived | **declared** | Forge-slot discipline on the `--mode node` relay run-loop: a forge is attempted at most once per `SlotNo` and never for a slot `<=` the last forged slot (no past/duplicate forge); the current slot is derived ONLY through the clock seam (only `SlotNo` crosses; no `SystemTime`/`Instant`/float past the RED boundary); the forge tick advances no durable tip and admits/serves/gossips nothing (subordinate to the sync spine); for a fixed recovered state + ordered feed + injected clock schedule + shutdown schedule the forge-attempt sequence and forged bytes are byte-identical across runs; leadership eligibility stays in BLUE inside `forge_one_from_recovered`; single-epoch this cluster (unsupported slot fails closed / skips with a structured local outcome). |

> At HEAD `cd2484f`, `DC-NODE-05.status = "declared"`. The pending close-pass commit
> flips it to `enforced` (tests + `ci_script` populated) and records the **6 cross-slice
> strengthenings** against the rules it composes on (`CN-NODE-02`, `DC-SYNC-02`,
> `T-REC-03`, `DC-NODE-03`, `CN-PROD-02`, `DC-CINPUT-02b` per `cross_ref`). This
> mechanical narration counts only what is committed at `cd2484f`.

This section is informational and reflects the registry state at HEAD. No rule was
removed (expected: 0).

## 8. Honest residual (cluster scope)

**Hermetic, single-epoch, self-accept-only — no operator keys, no live peer, no BA-02.**

- **No operator-key/config ingestion.** Forge material is fenced producer-shell
  (`ForgeActivation`); real `--mode node` KES/VRF/cold/opcert/pool-id/pparams ingress is a
  separate RED key-ingress cluster.
- **Self-accept-only.** A forged block is a local artifact: it advances **no durable tip**
  and is never served, admitted, broadcast, or gossiped (mechanically fenced by the
  extended containment gate). `run_node_sync → pump_block` remains the sole durable
  tip-advance authority.
- **No live claim.** No live peer, no BA-02, no RO-LIVE-01/06 acceptance. RO-LIVE-01 stays
  partial/operator-gated. The live, unbounded peer is the RO-LIVE-01 follow-on.
- **Single-epoch.** An unsupported slot fails closed / skips with a structured local
  outcome — cluster-scope containment proven by S3b, not permanent behavior.
- **No BLUE change.** Mirrors N-F-D: 456 BLUE canonical types unchanged; leadership
  eligibility stays in BLUE `forge_one_from_recovered`; the GREEN planner is content-blind;
  `run_node_sync` is UNMODIFIED.
