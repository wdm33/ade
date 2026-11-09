# Slice / Design — PRODUCER-PARTICIPANT-FOLLOW

> **Status: SCOPED (design only). NO implementation.** Grounded in live observation +
> a characterization diagnostic + a code trace (below). This is **not "fix rollback."**
> It **removes a wrong authority split**: the block-producing node's *follow* side runs a
> private, weaker single-producer/anchor-only authority instead of the already-proven
> participant/AO chain-selection authority.

## Cluster

PRODUCER-PARTICIPANT-FOLLOW (new). Related: **PHASE4-N-AO** (CN-CONS-03 — the proven
multi-candidate fork-choice, live on cardano-testnet, on the *participant* path);
LIVE-FOLLOW-THROUGHPUT (DC-MEM-11 kept; DC-MEM-12 rejected — not the blocker).

## Intent (the invariant)

The block-producing `--mode node` MUST follow the public multi-producer chain through the
**already-proven participant/AO chain-selection authority** (`run_participant_sync` +
fork-choice + store-based intersect/rewind), **not** a private single-producer/anchor-only
follow path. Forging/signing stays **producer-keyed (RED)** and fires **only at a leader
slot on the AO-selected canonical tip.**

> **Following a public multi-producer chain is PARTICIPANT behavior. Forging is PRODUCER
> behavior. They are separate authorities.** The current forge path conflates them by
> running a private single-producer follow — that conflation is the architecture smell
> this slice removes. The fix makes **follow authority singular** (the proven one) while
> keeping **forge authority narrow and keyed.**

## Evidence (grounding — why this slice, what is/ isn't proven)

**Confirmed (live + code):**
- The BA02 forge follow uses `run_relay_loop → run_node_sync` (`node_lifecycle.rs:1743`) +
  the wire pump — **not** `run_participant_sync`. [code]
- **Startup intersect fails on an orphaned recovered tip**: the wire pump offers a SINGLE
  anchor point → `admission_wire_pump exit=UnsupportedRollbackPoint`. [live: re-run on the
  orphaned `store-live`]
- **A fresh-frontier live run reached and held the tip** (~8 min, gap ~0, `post_fp` ~10 µs —
  rate/post_fp are NOT the blocker), **then died on a rollback** (`UnexpectedRollback`). [live]
- `run_node_sync` (the forge's follow) is **anchor-only**: only a RollBackward to the exact
  recovered anchor is a no-op; **every other rollback fail-closes** (`node_sync.rs:504-510`),
  with no store-rewind and no fork-choice. [code]
- The AO participant fork-choice (CN-CONS-03) **exists and is proven** but is **never invoked**
  on the forge path; `run_participant_sync` (`node_lifecycle.rs:3422`) is a separate function. [code]

**Not confirmed (and deliberately not chased):**
- The exact *logged* `run_node_sync:510` rollback point/hash for the fresh-frontier death.
  It is **code-traced** (death msg `node_lifecycle.rs:2757` wraps `run_node_sync`, whose only
  `UnexpectedRollback` is `:510`), not REORGDIAG-captured — the orphaned store died at the
  wire-pump intersect first. Reproducing it would need a clean fresh-frontier store via
  `--mode admission`, which is a **path-fidelity violation** (the C2 guide mandates: recover
  an existing non-Origin tip, continue through the real `--mode node` path; do not invent
  alternate paths as proof). Declined. The architectural boundary is already clear without it.

## The two legs (both MUST be covered)

1. **Startup / intersection** from a recovered store after a shallow reorg — the wire-pump's
   single-anchor FindIntersect must resolve an orphaned recovered tip to the LCA via
   **store-backed points through the participant follow authority**, not fail closed with
   `UnsupportedRollbackPoint`.
2. **Live RollBackward / competing branch** while holding the frontier — a rollback to a
   known volatile/recent point must **rewind and continue**; a competing branch after rollback
   must be **selected/rejected by the AO fork-choice**, not by single-producer linear
   `run_node_sync:510` fail-close.

## Scope

- Route the producer `--mode node` follow through `run_participant_sync` (store-based intersect +
  fork-choice + rewind) instead of `run_node_sync` (single-producer anchor-only).
- **Preserve producer-only signing**: forge/KES/VRF signing stays RED/keyed, leader-gated on the
  **AO-selected** canonical tip. Signing does NOT move into participant code.
- **Modules (design targets):** `node_lifecycle.rs` (forge loop selection → participant sync;
  forge-activation wiring preserved); the wire-pump intersect (store-backed points via the
  participant authority). **Reuse** the existing AO machinery — do not duplicate it.

## Invariants preserved

- Deterministic chain selection (CN-CONS-03): canonical tip from canonical observables only —
  never arrival timing / local path quirks.
- Recovered-anchor start-tip authority (DC-NODE-31); the recovered anchor floor.
- Producer signing authority (T-KEY-01): RED/keyed, leader-gated.
- Replay equivalence (same anchor/inputs/WAL/checkpoints → byte-identical).

## Invariant introduced (candidate — registry ID to be assigned)

- **CN-FOLLOW-01 (candidate, `true` tier):** the block-producing node's follow authority IS the
  participant/AO chain-selection (`run_participant_sync` + fork-choice + store-based
  intersect/rewind). The producer path MUST NOT use a private single-producer/anchor-only follow
  (`run_node_sync` anchor-only rollback + single-anchor intersect). Follow = participant; forge =
  producer; separate authorities.

## Mechanical Acceptance Criteria

- [ ] producer `--mode node` follows via participant/AO chain-selection authority
- [ ] producer signing remains RED/keyed; fires only at a leader slot on the AO-selected canonical tip
- [ ] recovered-anchor lineage preserved
- [ ] rollback below the supported anchor/floor → **fail closed** (structured error)
- [ ] rollback to a known volatile/recent point → **rewind and continue**
- [ ] competing branch after rollback → **AO-selected/rejected**, not single-producer linear
- [ ] startup intersect offers/uses **store-backed points** through the participant follow authority
- [ ] hermetic test: **orphaned-tip startup intersect** resolves via store-backed points (no `UnsupportedRollbackPoint`)
- [ ] hermetic test: **live RollBackward to a known LCA → competing-branch continuation** (AO selects)
- [ ] live observation repeats: **reaches frontier, survives rollback/reorg, stays caught up**

## Hard Prohibitions

- Do NOT duplicate fork-choice — route through the existing `run_participant_sync`/AO.
- Do NOT weaken anchor floors.
- Do NOT add an Origin fallback.
- Do NOT move signing into participant code.
- Do NOT resurrect the quarantined FindIntersect point-list (DC-NODE-42) as a *new* intersect —
  use the participant authority's existing store-backed intersect.
- Do NOT claim BA02 until the Haskell peer logs `AddedToCurrentChain` for Ade's exact forged
  hash (adoption/correlation).

## Tier framing

| Item | Tier | Meaning |
|---|---|---|
| deterministic chain selection from canonical observables | **true** | never depend on arrival timing or local path quirks |
| match Cardano/Haskell fork-choice behavior | **derived** | compatibility obligation (CN-CONS-03 / DC-CONSENSUS) |
| preview accepted block | **bounty** | only after adoption/correlation |
| topology / log capture | **operational** | venue mechanics |

## Design-pass findings (2026-06-18 — read-only, answered)

**MAJOR FINDING — the authority-separated path ALREADY EXISTS and is flag-gated.** The relay
loop routes by venue role (`node_lifecycle.rs:1467-1471`): `venue_role == VenueRole::Participant`
→ `run_participant_sync` (AO fork-choice + store-based rewind + fence resolution `:1735-1741`);
else → `run_node_sync` (single-producer anchor-only). The **`--participant-venue` CLI flag**
(`cli.rs:139-144` → `declare_participant_venue` → `VenueRole::Participant`, `node_lifecycle.rs:1295`)
selects it, and a Participant venue **requires a forge activation** (`:1472-1474`) — i.e. **a
participant venue IS a keyed forge following via the AO.** AI-S4b-ii wired this live; CN-CONS-03
was proven on the 2-producer testnet through this path. **The BA02 forge ran WITHOUT
`--participant-venue` → `VenueRole::Unknown` → `run_node_sync` → single-producer → died on reorgs.**
So the "wrong authority split" is a **configuration gap, not a missing capability.**

- **Q1 (forge activation) — CORRECTED 2026-06-19 (see Verification): ACTIVATION preserved, but the
  producing DECISION is SingleProducer-only.** `ForgeActivation` + the forge/sign call live in the
  relay-loop WRAPPER (the `ForgeTick` arm, `node_lifecycle.rs:1748-1842`) and drive BOTH sync paths,
  so `--participant-venue` yields a keyed forge following via the AO (signing stays RED/keyed).
  **BUT** the decision that actually produces (`ExtendOwnSpine`) is gated on
  `venue_role == SingleProducer` (`:1836`); the Participant venue takes the exact-caught-up
  DC-NODE-15 gate and produced 0 blocks in ~5 h. So "no wiring" was wrong — see Verification + the
  re-scope below.
- **Q3 (fence + pending-fork-switch) — COMPOSES, no new gate.** DC-NODE-28 `pending_reselection`
  is set with `pending_fork_switch` in `dispatch_competing_fork_choice` (`:3392-3395`); the
  ForgeTick gate refuses while set (`:1765-1768`); the participant arm clears it on
  `fork_switch_fence_resolved` + caught_up (`:1735-1741`, `fork_switch.rs:395-401`). Forge fires
  only on the AO-selected tip.
- **Q2 (startup intersect) — the ONE genuine gap.** The wire-pump FindIntersect
  (`admission/wire_pump.rs:187`, `wire_pump_start_point` = recovered tip or Origin) offers a
  SINGLE anchor point, SHARED across both sync paths; `--participant-venue` does NOT change it.
  So leg 1 (orphaned recovered tip → `UnsupportedRollbackPoint` at startup) is NOT fixed by the
  flag. The participant fork-choice's store-backed LCA walk (DC-NODE-38) is FOLLOW-time, not the
  STARTUP intersect. **Open:** store-backed startup intersect (recent durable points) WITHOUT
  resurrecting the quarantined point-list (DC-NODE-42) — OR accept leg 1 is largely *mitigated*
  once leg 2 stops the forge dying on reorgs (a clean store is never left orphaned; only a
  crash / downtime-reorg orphans the recovered tip — a narrower case).

## Verification (2026-06-19) — overturns "config-only"

The `--participant-venue` forge ran **~5 h live** vs `cardano-node-preview`: caught up, **760
blocks admitted**, AO routing confirmed, survived 3 divergence verdicts — but **produced 0
blocks**. `forge_result` outcomes: **18052 `no_tip_available` + 988 `not_leader`** (0 forged;
`is_leader=true` count 0). At the ADE1 leader slot 115152430 the forge did not fire (the network
itself left 115152430 empty).

**Root (code-traced, `node_lifecycle.rs`):** the productive decision
`SingleProducerForgeDecision::ExtendOwnSpine` (forge on the local durable head) is reached **only
when `venue_role == VenueRole::SingleProducer`** (`:1836-1880`, forge-base read `:1900-1904`). The
Participant venue falls to the `else` branch (`:1881-1893`) = the **pure DC-NODE-15 gate**
(`dc_node_15_refusal`: `durable_servable_tip == followed_peer_tip`, hash AND block_no, exact). At
the live frontier Ade's durable tip is ~1 behind the racing live tip, so the gate **refused ~95%**
(the 18052 `no_tip_available` = `!forged`, `:2043-2052`); it passed only ~5% (the 988 that reached
the leader check → not-leader, correct for σ≈0.0003). The participant forge can fire only in the
rare instant it is byte-exactly at the live tip — and so missed the one leader slot.

**No flag turns this on (verified):** the full CLI set has `--participant-venue` /
`--single-producer-venue` (mutually exclusive, `cli.rs:483`), the keys, `--listen`/`--peer` — **no
forge/produce/duplex/venue-combine option**. `VenueRole` = `Participant` / `SingleProducer` /
`Unknown` only — **no combined role**. A `SingleProducer` forge (ExtendOwnSpine) + a `Participant`
follow (AO) is **not reachable by configuration**.

So **Q1 was wrong**: the forge ACTIVATION (keys + the ForgeTick wrapper) is preserved across venues,
but the forge **DECISION** that produces (ExtendOwnSpine) is `SingleProducer`-only. This is a real
code slice, not a config change.

## Re-scope — the forge-decision code slice (CN-FOLLOW-01)

**Target:** route the **keyed-producer** Participant venue to the `ExtendOwnSpine` decision — forge
on Ade's own AO-followed durable head — instead of the exact-caught-up DC-NODE-15 gate that refuses
~95% at the frontier. Concretely: the `venue_role == SingleProducer` branch (`:1836`) and the
forge-base evidence (`:1900`) must also admit `VenueRole::Participant`, so the participant forge
builds on `ChainDb::tip` (the AO-selected durable head) at its leader slot.

- The AO follow already keeps that durable head correct (fork-choice + store rewind, proven
  CN-CONS-03). ExtendOwnSpine on the AO-followed head = produce the next block on the network's
  chain — the correct participant-producer behavior.
- The DC-NODE-28 fence (`pending_reselection`/`pending_fork_switch`) already composes (Q3) — forge
  still fires only on the AO-selected tip, so ExtendOwnSpine does NOT bypass fork-choice.
- The exact-caught-up DC-NODE-15 gate stays the rule for the **non-producer** follow; this slice
  only re-routes the keyed-producer Participant venue.

**Slice-entry proof obligations (answer before code):**
- Does ExtendOwnSpine on a participant (multi-producer) head need a guard the single-producer path
  didn't — beyond the DC-NODE-20 rule that the base is a head a peer can FindIntersect?
- `no_tip_available` also covers a missing `durable_servable_tip` (served projection) — confirm the
  served projection is populated on the participant path so ExtendOwnSpine always has a base.
- Legs 1 (startup intersect) + 2 (live rollback) from the original scope still apply.
- **Doc cleanup:** `cli.rs:142-143` + `node_lifecycle.rs:1290-1294` "inert until the live wiring
  lands" is stale — AI-S4b-ii landed.

## Status

**RE-SCOPED to a code slice (2026-06-19).** The ~5 h live verification proved the follow half
(`--participant-venue` → AO, caught up, stable) AND falsified the "config-only" hypothesis: the
forge half never produced (`no_tip_available` ~95%) because `ExtendOwnSpine` is `SingleProducer`-
only and the Participant venue takes the exact-caught-up DC-NODE-15 gate. **No flag enables a
participant-follow + producing-forge** (full flag set + venue roles checked). **Slice doc WRITTEN**
(`CN-FOLLOW-01-participant-forge-on-ao-selected-head.md`, 2026-06-19) — full 18-section doc with the
three entry obligations ANSWERED via parallel code-trace: (1) DC-NODE-20 is rung-1-single-producer-
only and `single_producer_forge_decision` fails closed on `venue != SingleProducer` AND on any
competing `observed_peer_tip`, so it is NOT reusable — the additional guard ALREADY EXISTS as
**DC-NODE-28**, which the new Participant forge-decision reuses; (2) the base IS available (the
served projection is a thin direct-read on the shared ChainDb) — the ~95% refusal is the per-tick
exact-equality DC-NODE-15 gate, because the Participant `else` branch never does the
`forge_mode_on_caughtup` transition the SingleProducer venue does; (3) the startup-intersect +
live-rollback legs are covered and UNAFFECTED (forge is strictly downstream of `SyncOnce`; the
`pending_*` fences gate the ForgeTick). Tier split kept un-flattened (true/derived/release/bounty/
operational), 10-item MAC. **Next gate: review the slice doc, then implement under the Hard
Prohibitions above.** NO code yet; NO BA02 until a Haskell `AddedToCurrentChain` correlation.
