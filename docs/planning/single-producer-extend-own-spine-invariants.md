# Invariant sketch — Single-producer successor forge extends the adopted durable spine (DC-NODE-18)

> IDD Part I artifact (invariants phase). The **implementable slice** for rung-1 Finding B, after OQ-1.
> Supersedes the DC-NODE-17-as-fix hypothesis (see `sustained-single-producer-forge-invariants.md`
> §"OQ-1 RESULT" + the DC-NODE-17 registry notes). **DC-NODE-17 is retained as a safety/observation
> invariant only** ("if the peer advertises a tip, the RED signal must reflect it; never local inference").

**Why this slice (OQ-1, live-proven 2026-06-07).** A recovered/following `--mode node` Ade forges
exactly one block per recover, then stalls. OQ-1 (run c2t4) showed the relay genuinely *adopts* Ade's
forged block (`AddedToCurrentChain blockNo=12`) but does **not re-announce it** back over Ade's follow
link (`followed_peer_tip` stayed at block 11; no `RollForward(12)`; then the link EOF'd). So the
AE.A/DC-NODE-15 gate (`forge only when durable_servable_tip == followed_peer_tip`) waits forever for an
echo that never comes. A real sole producer does **not** learn its own tip from a relay echo — it
extends the chain it is building.

**Concept (one line).** Once a single-producer Ade has caught up to a real peer tip (DC-NODE-15) and the
relay has adopted Ade's first successor, Ade may extend its **own durable adopted spine** (forge N+2 on
durable N+1, …) without requiring the relay to re-announce each Ade-authored block — fenced strictly to a
declared single-producer venue.

## The invariant — DC-NODE-18

> After a recovered/following node has satisfied the initial caught-up gate against a real peer tip and
> has produced a successor that a non-producing Haskell relay adopts, subsequent forge attempts **in a
> declared single-producer venue** may use Ade's own durable servable tip as the forge base, **without**
> requiring the relay to re-announce that same Ade-authored block back over the follow link.
>
> Valid **only** while the venue is explicitly single-producer: the relay is non-producing, Ade is the
> sole block producer, and no competing candidate chain is admitted. Multi-producer behaviour remains
> out of scope and belongs to fork-choice / chain-selection authority (DC-CONS-03).

**Inductive justification (why it's safe).** 1. Ade recovers a real peer tip. 2. Catches up (DC-NODE-15).
3. Forges N+1 on it. 4. The relay adopts N+1. 5. Ade's durable servable tip is now a **peer-adopted
parent**. 6. Ade may forge N+2 on the durable spine (it extends an already-adopted parent; the relay, a
pure follower, adopts it). 7. Repeat while the venue stays single-producer and no competing chain appears.
This is not fork-choice — it is a single-producer liveness rule over an **already-adopted lineage**.

## 1. What must always be true
- **DC-NODE-18** (above).
- **DC-NODE-15 still gates the INITIAL catch-up** — the first successor must be on a real peer-adoptable tip.
- The forge mode is an **explicit state machine** (no hidden boolean / implicit fallback):
  `InitialCatchupRequired → CaughtUpToPeerTip → FirstOwnBlockAdoptedByRelay → SingleProducerExtendOwnSpine`.
- **DC-NODE-17 still holds** as safety/observation: if the peer *does* advertise a tip, the RED signal
  must reflect it — never local inference.

## 2. What must never be possible (the hard fence — mechanically enforced; else fail closed)
- No use in a multi-producer venue.
- No use when the relay is producing.
- No use when a competing peer block beyond Ade's durable tip is observed.
- No bypass of chain selection (DC-CONS-03) once competing candidates exist.
- No fabricated peer catch-up.
- No RED signal selecting / replacing / reordering / preferring chains.
- The slice **fails closed** if the venue is not explicitly declared single-producer.

## 3. Deterministic surface (unchanged)
The durable post-state after a chain of K own-forged blocks (ledger, chain_dep, WAL, ChainDb tip) is
unchanged by this concept. The forge-mode state machine is RED scheduling — not part of the deterministic
surface.

## 4. Replay-equivalence
Same recovered checkpoint + same WAL (the own-forged chain) → byte-identical post-state **and** served
chain (T-REC-05 / DC-WAL-02 / DC-WAL-04). A K>1 own-forged chain must warm-start-replay byte-identically
(extends N-U S2's single-block proof).

## 5. State transitions in scope (RED scheduling; the BLUE reducer is unchanged)
- `(CaughtUpToPeerTip, forge N+1 on followed==durable peer tip) → FirstOwnBlockAdoptedByRelay` — the
  existing DC-NODE-15 path.
- `(SingleProducerExtendOwnSpine, ForgeTick) → forge N+2 on durable_servable_tip` — the new path; **no**
  followed==durable requirement.
- Each durable admit remains the BLUE `pump_block` chokepoint transition (unchanged, DC-NODE-16 idempotent).

## 6. TCB color hypothesis
- **BLUE unchanged:** `pump_block` durable admission (authoritative + idempotent); ledger / chain_dep / WAL;
  successor construction from the evolved admitted spine (DC-NODE-10 / DC-CONS-24); chain selection
  (DC-CONS-03, the sole follow/fork authority).
- **RED/GREEN refinement:** the forge scheduler's explicit mode state machine
  (`ade_node::node_lifecycle` ForgeTick + `node_sync` classifier). **No BLUE type/authority change.**

## 7. Tier classification
**Derived, not true.** Cardano-compatibility/liveness for the C2-LOCAL single-producer relay spine. The
true-tier laws underneath are unchanged — deterministic chain selection, replay equivalence, no RED input
in BLUE fork-choice, no nondeterministic authoritative behaviour. The c2-guide §7b frames rung 1 as
single-producer local robustness *before* rung 2 multi-producer fork-choice and rung 3 preprod.

## 8. Acceptance evidence (the slice must produce)
- Forge N → relay adopts; Ade forges N+1 → relay adopts;
- sustained **past k** — several Ade blocks settle into the relay's ImmutableDB (rung-1 criterion 1);
- warm-start replay **byte-identical** (T-REC-05 over the multi-block own-forged chain).

## 9. Open questions
- **OQ-KA (secondary, non-blocking):** does follow-link keep-alive cause a *delayed* self-echo? If yes,
  DC-NODE-17 also becomes enforceable + a RED keep-alive liveness slice is justified — but **DC-NODE-18
  does not depend on it** (relying on the echo would be a weaker design: sole-producer liveness must not
  hinge on an echo that is not semantically necessary).
- **Venue declaration:** how is "declared single-producer" represented + checked (a CLI flag / explicit
  mode)? Must be explicit and fail-closed — the fence in §2 keys off it.
- **Recover-anchor k=0 edge (rung-1 sub-finding):** at k=0 the follow re-delivers the recover anchor and
  hits a snapshot-slot conflict (the anchor is a slot-keyed snapshot, not an AE.F-idempotent StoredBlock).
  Separate recover→follow robustness edge; rung1-auto guards k≥2 operationally. A real fix would make the
  anchor AE.F-idempotent or skip its re-delivery.

## Proposed registry entry (appended to `docs/ade-invariant-registry.toml`)
`DC-NODE-18`, `tier = derived`, `status = declared` (enforced only when the slice lands with the §2 fence
mechanically gated + the §8 acceptance evidence). Not a chain-selection rule.
