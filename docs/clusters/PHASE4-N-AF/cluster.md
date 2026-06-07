# PHASE4-N-AF — Single-producer sustained forge (DC-NODE-18 extend-own-durable-spine)

> **Single-slice cluster.** The rung-1 Finding B fix, scoped after **OQ-1** (2026-06-07) live-disproved
> the DC-NODE-17-as-fix hypothesis. Authority source:
> `docs/planning/single-producer-extend-own-spine-invariants.md`.
> **RUNG-1 ONLY · SINGLE-PRODUCER ONLY · DC-CONS-03 UNTOUCHED · BLUE core unchanged.**

## §1 Primary property (DC-NODE-18 — new rule, declared → enforced on close)
After a recovered/following node has satisfied the initial caught-up gate against a real peer tip
(DC-NODE-15) **and** has produced a successor that a non-producing Haskell relay adopts (per an explicit
RED venue-adoption certificate), subsequent forge attempts **in a declared single-producer venue** may
use Ade's own durable servable tip as the forge base, **without** requiring the relay to re-announce that
block back over the follow link. A successor is adoptable **by induction** (it extends an already-adopted
parent; the relay is a pure follower of Ade's chain). This is a gate-**applicability** refinement, **not**
a fork-choice weakening.

## §2 The finding (OQ-1, live-proven 2026-06-07)
A recovered/following `--mode node` Ade forges exactly **one** block per recover, then stalls. OQ-1 (run
c2t4, instrumented + reverted) showed the relay genuinely **adopts** Ade's forged block
(`AddedToCurrentChain blockNo=12`) but does **not re-announce it** back over Ade's follow link
(`followed_peer_tip` stayed at block 11; no `RollForward(12)`; then the link EOF'd). So the
AE.A/DC-NODE-15 gate (`forge only when durable_servable_tip == followed_peer_tip`) waits forever for an
echo that never comes. The pump emits `TipUpdate` on every `RollForward` (verified for block 11) — there
is simply no `RollForward(12)`. **DC-NODE-17 is therefore not the stall fix** (retained as a
safety/observation invariant only). A real sole producer does not learn its own tip from a relay echo; it
extends the chain it is building.

## §3 The design
An explicit `ForgeMode` enum (RED loop state; GREEN transition function) makes the ForgeTick gate
mode-aware:
`InitialCatchupRequired → CaughtUpToPeerTip{peer_tip} → FirstOwnBlockServed{own_tip, parent_peer_tip} →
SingleProducerExtendOwnDurableSpine{adopted_root, current_tip}`. Promotion into the last state fires
**only** on an explicit RED venue-adoption certificate (operator/harness-supplied evidence that the relay
adopted `own_tip`) — never inferred from self-admit. **The promotion certificate is admissibility evidence
only: it MUST NOT be persisted as authoritative chain state and MUST NOT alter replay-visible durable
state; it may advance only the RED/GREEN forge-mode state.** In the extend state, each ForgeTick forges on
`current_tip` (the durable spine) without the followed==durable requirement, behind a fail-closed
single-producer fence (§8).

## §4 Scope of claim
- **Proves:** sustained single-producer settlement **past k** (rung-1 criterion 1) — Ade forges a chain,
  the relay adopts each, several Ade blocks settle into the relay's ImmutableDB; warm-start replay
  byte-identical.
- **Does NOT claim:** multi-producer fork-choice (rung 2); the slot-2000 epoch transition (rung-1
  criterion 2); preprod acceptance (rung 3); the recover-anchor k=0 fix; follow-link keep-alive (OQ-KA).

## §5 TCB color map (FC/IS partition)
- **BLUE (unchanged — zero diff):** `ade_runtime::forward_sync::pump` (`pump_block`, DC-NODE-16);
  `ade_ledger` ledger/chain_dep/WAL; `forge_one_from_recovered` (DC-NODE-10/DC-CONS-24); chain selection
  (DC-CONS-03).
- **GREEN:** the `ForgeMode` mode/fence classifier in `ade_node::node_sync` (pure/total/deterministic).
- **RED:** the `ade_node::node_lifecycle` ForgeTick arm; the venue-certificate input.

## §6 Slices
| Slice | Scope | CE | Registry | TCB |
|---|---|---|---|---|
| **S1** | `ForgeMode` enum (no booleans) + promotion-requires-RED-certificate (admissibility-only, not persisted/replay-visible) + fail-closed single-producer fence (`ForgeRefused::SingleProducerFenceViolation`) + mode-aware ForgeTick gate; hermetic tests + `ci_check_single_producer_extend_own_spine.sh`; live CE-AF-6 transcript. | CE-AF-1..6 | DC-NODE-18 declared→enforced | RED/GREEN |

## §7 Cluster Exit Criteria (mechanical except the operator-gated CE-AF-6)
- **CE-AF-1:** explicit `ForgeMode` enum; transitions total + deterministic; no booleans.
- **CE-AF-2:** promotion to `SingleProducerExtendOwnDurableSpine` requires an explicit RED venue-adoption
  certificate; never inferred from self-admit (self-admit w/o cert stays `FirstOwnBlockServed`).
- **CE-AF-3:** in the extend state, ForgeTick forges on `durable_servable_tip` without `followed==durable`;
  successor parent byte-equals the durable tip (DC-CONS-24).
- **CE-AF-4:** fail-closed fence → `ForgeRefused::SingleProducerFenceViolation{reason, durable_tip,
  followed_peer_tip, observed_peer_tip, venue_role}` for each refuse condition (§8).
- **CE-AF-5:** warm-start replay of a K≥2 own-forged chain byte-identical (durable state + served chain).
- **CE-AF-6 (operator-gated):** committed `rung1-auto.sh` (k≥2) live transcript — forge N→adopt; N+1→adopt
  without relay echo; sustained past k.
- Tree green: `cargo test -p ade_node`; the new CI gate green.

## §8 Forbidden during this cluster (hard boundaries — the fence)
- No booleans for the forge mode (explicit enum only).
- No global config knob that weakens semantics — single-producer is a **venue-scoped** RED certificate.
- No silent inference of relay adoption (promotion requires explicit RED evidence).
- No RED signal selecting / replacing / reordering / preferring chains (**DC-CONS-03 untouched**).
- No use in a multi-producer venue / when the relay is producing / when a competing peer block beyond the
  adopted root is observed / when the peer tip disagrees with the expected single-producer spine / on the
  recovered-anchor k=0 snapshot-conflict edge — each **fails closed**.
- **Zero BLUE change** (no new BLUE type/authority); no persisted/replay-visible certificate state.
- No epoch-transition / keep-alive / preprod / k=0-recovery work.

## §9 Replay obligations
Warm-start replay of a K≥2 own-forged chain is byte-identical (durable state + served chain) — T-REC-05
extended to the chain (CE-AF-5). The `ForgeMode` is RED scheduling state, **not** persisted/WAL'd → it
cannot perturb the deterministic surface. The venue-adoption certificate is admissibility evidence only,
**not** replay-visible. No new durability law, no WAL/schema change (DC-NODE-16 / DC-WAL-04 unchanged).

## §10 Invariants
- **Adds:** DC-NODE-18 (declared → enforced on close).
- **Preserves:** DC-NODE-15, DC-NODE-16, DC-CONS-03, DC-NODE-10, DC-CONS-24, DC-NODE-12, DC-WAL-04,
  T-REC-05, DC-WAL-02, **DC-NODE-17 (retained safety/observation-only — not the fix)**, T-DET-01.
- **Strengthens:** none (DC-NODE-18 is the new rule; the preserved rules are not weakened).

## §11 Close record
*(Filled at `/cluster-close`: commits, CE-AF-1..6 pass evidence, the committed CE-AF-6 transcript path,
DC-NODE-18 declared→enforced + its tests/ci_script appended, reviews.)*
