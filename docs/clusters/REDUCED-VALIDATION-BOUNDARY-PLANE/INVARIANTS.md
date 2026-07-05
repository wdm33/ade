# REDUCED-VALIDATION-BOUNDARY-PLANE — invariants

The reduced follower (`track_utxo=false`) is NOT allowed to carry a half-real epoch ledger. At a Conway epoch
boundary it either has the inputs for a full boundary transition, or it produces a **clearly typed reduced
projection** — never a degraded full transition, never a fabricated snapshot. The full `EpochAccumulator` path
remains the SOLE source of future-epoch authority (rewards, governance, pots, mark/set/go); the reduced
checkpoint owns UTxO-derived stake. This cluster makes the reduced plane's non-authority unrepresentable-as-
authority in the type system. (Design: `CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION/S2-DESIGN-reduced-boundary-plane.md`.)

## What must ALWAYS be true

- **I-RVB-1 (two non-interchangeable results).** An epoch boundary yields EITHER a `FullEpochBoundaryResult`
  (the authoritative transition — RUPD-applied rewards, pots, mark/set/go, governance/pparam enactment,
  CE-3d-comparable; REQUIRES point-bound `BoundaryBaseStake`) OR a `ReducedBoundaryProjection`. They are DISTINCT
  types; a reduced result cannot be widened into a `LedgerState`, serialized as an accumulator snapshot, or
  fingerprinted as full authority.
- **I-RVB-2 (the reduced contract — advance ONLY the audited facts).** At a `track_utxo=false` Conway boundary
  the reduced projection advances ONLY: (a) epoch/slot progression; (b) a `ReducedEpochProgress { epoch, slot,
  reduced_block_window }` recording that the structural block-production window rolled over. `ReducedEpochProgress`
  MUST NOT be convertible into the full nesBcur / reward-calculation input without an explicit
  `FullBoundaryStateRequired` failure.
- **I-RVB-3 (typed verdict capability).** A block/header/cert-continuation verdict from the reduced plane is
  `StructuralValidity`. It can NEVER be promoted to `FullLedgerValidity` merely because a boundary crossed
  without error. A caller requiring a full verdict after a reduced boundary receives a structured
  `FullBoundaryStateRequired`, never a silent success.
- **I-RVB-4 (the full path is the sole authority).** Only `FullEpochBoundaryResult` may feed `ActiveEpochAuthority`,
  leadership, forging, CE-3d state comparison, persistence as an epoch-authority snapshot, or a full-ledger
  verdict. The reduced projection feeds none of these.

## What must NEVER be possible

- **N-RVB-1 (no fabricated snapshot).** A `track_utxo=false` Conway boundary produces NO mark/set/go bytes at all
  — no reward-only/empty-base mark, no stale snapshot, nothing that could be persisted, fingerprinted, or
  rehydrated as authority.
- **N-RVB-2 (nothing stale/inferred/partial).** Beyond epoch/slot + the reduced block-window rollover, every
  boundary effect is UNAVAILABLE — RUPD, reward-account mutation, pots, mark/set/go, POOLREAP, governance
  ratify/enactment, pparam activation, and any full certificate/pool-lifecycle claim. Unavailable means ABSENT,
  never a stale or empty stand-in.
- **N-RVB-3 (POOLREAP stays whole or absent).** POOLREAP is unavailable in the reduced plane — splitting its
  pool/delegation cleanup from its reward refund would create a hybrid state matching neither cardano nor the
  authoritative accumulator (the constitution forbids partial epoch finalization / path-dependent state).
- **N-RVB-4 (no widening).** No `Option<Snapshot>` inside a shared `LedgerState`; no boolean mode flag; no
  empty-base sentinel. The reduced and full results are separate types that cannot be silently interchanged.

## What must remain identical across executions (replay / recovery)

- **R-RVB-1.** A reduced follower crosses Conway boundaries, restarts, rolls back, and continues structural/fork
  validation deterministically, WITHOUT emitting or persisting a fabricated snapshot. WAL/recovery fingerprints
  distinguish a `ReducedBoundaryProjection` from full authority state, and reproduce byte-for-byte on replay.
- **R-RVB-2.** The full authoritative path (accumulator and direct-full-ledger with the same `BoundaryBaseStake`)
  remains byte-identical with the corrected post-RUPD mark — proven in the authoritative mark-correction slice.

## The 7 acceptance gates

1. A `track_utxo=false` Conway boundary produces **no mark/set/go bytes at all**.
2. No reduced result can be serialized into the authoritative accumulator-snapshot format.
3. WAL/recovery fingerprints **distinguish** `ReducedBoundaryProjection` from full authority state.
4. No reduced result can feed `ActiveEpochAuthority`, forging, leadership, CE-3d, or a full-ledger verdict.
5. Reduced validation still crosses boundaries and supports fork-choice / header continuation.
6. A caller requiring a full verdict after a reduced boundary gets a structured `FullBoundaryStateRequired`.
7. Full accumulator/direct-full-ledger paths still match **byte-for-byte** with the corrected post-RUPD mark.

## Read-set audit (binding — every reduced advancement was proven derivable-from-reduced-inputs)

epoch/slot progression → **advance**; reduced block-window rollover (`ReducedEpochProgress`) → **advance**;
governance ratify/enact + pparam activation → **unavailable**; RUPD/reward accounts → **unavailable**; pots →
**unavailable**; mark/set/go → **unavailable**; POOLREAP → **unavailable**. A structural check that would depend
on a gov-activated pparam is answered `StructuralValidity`, never `FullLedgerValidity`.

## Sequence + slices

P1 typed reduced boundary projection (types + remove reward-only mark + persistence/fingerprint distinction —
independently safe) → P2 capability-gated consumers (`FullBoundaryStateRequired` everywhere) → [then the
authoritative post-RUPD mark correction, in CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION] → P3 recovery/fork-switch/
boundary-crossing proof → CE-3d rerun. Each slice independently safe: P1 must not leave the old reward-only mark
persisted anywhere.
