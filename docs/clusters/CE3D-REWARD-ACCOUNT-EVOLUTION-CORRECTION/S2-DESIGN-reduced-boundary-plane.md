# S2 — the reduced-validation boundary plane (capability-typed split)

The base-required full boundary (S1, implemented) exposed a pre-existing architectural gap: the node's
block-validity plane (`track_utxo=false`) was crossing epoch boundaries through the FULL ledger transition and
producing a reward-only stub mark. The trace proves that stub reaches recovery persistence + WAL fingerprints —
a fake snapshot leaking into the replay plane. Per the ruling, the reduced plane must become a **typed
reduced-projection transition, not a degraded full transition**. This design translates that ruling into a
concrete type architecture, does the required read-set audit, and lists the acceptance gates.

## The two non-interchangeable boundary results

```rust
/// The authoritative epoch boundary. The ONLY producer of RUPD-applied rewards, pots, mark/set/go, pool/leader
/// inputs, full governance/pparam enactment, and CE-3d-comparable state. REQUIRES point-bound BoundaryBaseStake.
/// (= the S1 apply_epoch_boundary_with_registrations output — already implemented, base-required, post-RUPD mark.)
struct FullEpochBoundaryResult { ledger: LedgerState /*with snapshots*/, accounting: EpochBoundaryAccounting }

/// A reduced-plane boundary projection. Carries ONLY facts provably derivable from reduced inputs. It has NO
/// rewards, NO pots, NO RUPD state, NO mark/set/go, NO governance/pparam enactment, and makes NO claim of a full
/// ledger verdict. It is a DISTINCT type — it cannot be widened into a LedgerState, serialized as an accumulator
/// snapshot, or fingerprinted as full authority.
struct ReducedBoundaryProjection { /* see read-set audit below */ }
```

These are separate types (NOT `Option<Snapshot>` inside one `LedgerState`) so a reduced result is
**unrepresentable as full authority** in the type system.

## The verdict capability split

```rust
enum LedgerValidity {
    /// Full Conway ledger validity across ALL epoch effects (rewards/pots/snapshots/gov). Only from the full
    /// path (track_utxo=true or within-epoch full apply).
    FullLedgerValidity(..),
    /// Header/body-structure/cert-continuation validity ONLY. The reduced plane returns this after a reduced
    /// boundary; it can NEVER be promoted to FullLedgerValidity merely because the boundary crossed without error.
    StructuralValidity(..),
}
```
A caller that requires a full verdict after a reduced boundary receives a structured
`FullBoundaryStateRequired` terminal — never a silent success.

## Read-set audit — what the reduced boundary MAY advance (the load-bearing decision)

A fact enters `ReducedBoundaryProjection` ONLY if ALL hold: (a) its next value is derivable from reduced inputs
alone; (b) later reduced validation needs it; (c) it cannot be mistaken for full authority; (d) it does not
certify validity beyond the reduced plane's declared coverage. Auditing every effect of the full boundary
(`apply_epoch_boundary_with_registrations`, rules.rs):

| full-boundary effect | reads | reduced-derivable? | verdict |
|---|---|---|---|
| **epoch number + slot progression** | slot, era schedule | YES (arithmetic on the header slot) | **ADVANCE** |
| **nesBcur reset** (block_production→∅, epoch_fees→0) | — | YES (definitional reset) | **ADVANCE** |
| Governance ratify/enact (gov_state, pparam change) | mark (DRep stake), go (pool stake), proposals | NO — needs full stake authority | **UNAVAILABLE** (gov_state + pparams NOT advanced; a gov-activated pparam change is absent, never silently retained) |
| RUPD (reward accounts) | go, block_production, reserves | NO — needs go + full | **UNAVAILABLE** (no reward change) |
| Pots (reserves/treasury) | RUPD outputs | NO — derived from RUPD | **UNAVAILABLE** |
| mark/set/go rotation | BoundaryBaseStake | NO — no base | **UNAVAILABLE** (the whole point) |
| POOLREAP (adopt/reap/refund/clear) | pools, retiring, rewards | NO — refund touches reward accounts; full effect not reduced-derivable | **UNAVAILABLE** (pools/delegations NOT reaped in the reduced plane) |

**Conclusion:** the reduced boundary advances ONLY epoch/slot + nesBcur reset. Every authoritative effect
(rewards, pots, snapshots, gov/pparam enactment, POOLREAP) is **unavailable** in the reduced plane, because each
depends on stake/reward/snapshot authority the reduced plane lacks. This is conservative by construction — the
reduced projection is nearly a no-op, which is correct: reduced validation checks header structure (leader via
the SEPARATE `ledger_view`), body-hash, and cert structure — none of which needs those effects. A structural
check that WOULD depend on a gov-activated pparam is answered `StructuralValidity`, never `FullLedgerValidity`.

## Where the split lands in the code

- **Full path (done):** `apply_epoch_boundary_with_registrations` (base-required, post-RUPD mark) = the
  `FullEpochBoundaryResult` producer.
- **Reduced path (new):** a `reduced_boundary_projection(reduced_facts, new_epoch)` that advances only the audited
  facts and returns `ReducedBoundaryProjection`.
- **The dispatch:** `apply_block_with_accounting` (rules.rs:120) currently calls `apply_epoch_boundary_full`
  unconditionally. It splits by capability: `track_utxo=true` → full boundary → `FullLedgerValidity`;
  `track_utxo=false` → reduced projection → `StructuralValidity`. `block_validity` (the reduced-plane entry)
  returns `StructuralValidity`; the full-ledger/accumulator paths return `FullLedgerValidity`.
- **Persistence/fingerprint:** the recovery-checkpoint encoder and WAL fingerprint must distinguish a reduced
  projection (no-snapshot) from full authority state — a reduced projection can never be encoded in the
  authoritative accumulator-snapshot format.

## The 7 acceptance gates (from the ruling)

1. A `track_utxo=false` Conway boundary produces **no mark/set/go bytes at all**.
2. No reduced result can be serialized into the authoritative accumulator-snapshot format.
3. WAL/recovery fingerprints **distinguish** `ReducedBoundaryProjection` from full authority state.
4. No reduced result can feed `ActiveEpochAuthority`, forging, leadership, CE-3d, or a full ledger verdict.
5. Reduced validation still crosses boundaries and supports fork-choice / header continuation.
6. A caller requiring a full verdict after a reduced boundary gets a structured `FullBoundaryStateRequired`.
7. Full accumulator/direct-full-ledger paths still match **byte-for-byte** with the corrected post-RUPD mark.

## Phasing

- **P0 (done, S1):** the full boundary post-RUPD mark, base-required, terminal, + accumulator rewire. ade_ledger
  green.
- **P1:** introduce `ReducedBoundaryProjection` + `reduced_boundary_projection` (epoch/slot + nesBcur only) + the
  `StructuralValidity`/`FullLedgerValidity` capability + `FullBoundaryStateRequired`.
- **P2:** split the dispatch in `apply_block_with_accounting` / `block_validity` by `track_utxo`; the reduced
  plane returns `StructuralValidity` and never touches rewards/pots/snapshots.
- **P3:** persistence/fingerprint distinction (gates 2/3); update the 10 fork-choice tests to prove reduced
  boundary-crossing without invented stake (gate 5); the no-fallback + full-path byte-identical gates (1/6/7).

## Note

This is a distinct architectural concern from the reward-order correction (it is the reduced-plane's typed
non-authority, which the base-required full path merely exposed). It may warrant its own cluster
(`REDUCED-VALIDATION-BOUNDARY-PLANE`); kept here as S2 for continuity since it gates the same commit.
