# Slice 1 — per-credential mark/set/go phase differential (GREEN / evidence; NO BLUE change)

Localize the REAL −343,260,172,883 lovelace CE-3d go-stake residual at the **credential level**, across the
**mark/set/go snapshot phases**, to a closed set of deterministic causes — none of which is the base UTxO
(exonerated in B3c.0). Diagnosis only; the corrective change is a separate BLUE slice gated on the cause named
here. Governed by `INVARIANTS.md`.

## What the totals already tell us (the frame this slice proves at the credential level)

The seed accumulator (epoch 1340, slot 115776011) carries imported snapshots:

| phase | pools | delegations | stake total |
|---|---|---|---|
| mark | 658 | 60337 | 1,674,023,071,155,299 |
| set  | 658 | 60333 | 1,673,585,390,664,020 |
| go   | 626 | 59687 | 1,670,947,881,740,492 |

Advancing to POST-1342 crosses two boundaries, so the snapshots rotate twice (`go := set; set := mark;
mark := fresh`):

- **`mark(1342)`** = a FRESH mark live-derived from POST-1341 (base+reward+delegation).
- **`set(1342)`** = a FRESH mark live-derived from POST-1340.
- **`go(1342)`** = the seed's ORIGINAL imported `mark` (M0), rotated forward twice — **NOT** live-derived.

Ade's `go(1342)` total is therefore the seed `mark` total `1,674,023,071,155,299` (658 pools) — exactly the
adjudicated `ade_go_total`. Cardano's `go(1342)` is `1,674,366,331,328,182` (626 pools). The difference is the
−343,260,172,883 residual. So the residual is carried on the **go phase, which is the bootstrap seed's imported
mark** — this slice proves that at the credential level and localizes WHY the imported mark differs from
cardano's `mark(1340)`, and whether the fresh live-derived phases (`mark(1342)`/`set(1342)`) share the
discrepancy or are clean.

## The two sides (like-for-like, per credential)

For each phase P ∈ {mark, set, go}:

- **Ade** — `epoch_state.snapshots.P.0.delegations`: `Hash28 → (PoolId, Coin)` (Coin = base+reward folded by
  delegation; read as-is, self-derived by the co-advance the live node runs).
- **cardano** — `decode_native_nonutxo_state(POST-1342).snapshots.P.0.delegations`: the same shape, decoded from
  cardano's `ssStake`.

The differential is keyed by `Hash28` (the discriminant-stripped go-snapshot key — both sides use it). The
per-credential go delta ≡ the per-pool go delta in aggregate (pool_stakes is the fold of delegations by pool), so
the credential-level classification sums to the same −343,260,172,883.

## The closed cause set (never a free-form string)

Every non-zero per-credential **go-phase** delta is exactly one of:

| cause | user dimension | meaning |
|---|---|---|
| `OnlyAde` | delegation presence | credential in Ade's go, absent from cardano's go — delta = +ade_coin |
| `OnlyRef` | delegation presence | credential in cardano's go, absent from Ade's go — delta = −ref_coin |
| `DelegationTargetMismatch` | delegation target | credential in both, different `PoolId` (stake re-attributed between pools; per-credential value delta may be zero but the fold differs) |
| `ValueDelta` | reward-account contribution | credential in both, same `PoolId`, different `Coin`; since base is exonerated, the delta is the reward-account (or snapshot-time reward) component — delta = ade_coin − ref_coin |
| `SnapshotPhaseProvenance` | snapshot phase & boundary point | the residual is carried by the go phase because `go(1342)` ≡ the seed's imported `mark` (M0), not a live-derived snapshot; recorded as the structural provenance of the whole go-phase residual |

Pool folding (double/absent) is checked as an invariant, not a per-credential bucket: for each pool,
`pool_stakes[pool] == Σ delegations coin where target == pool` on BOTH sides (asserted; a violation would be a
`FoldingError`, reported explicitly with the offending pool).

## Method

1. **Base-zero gate (fast).** A fresh isolated copy of the seed reduced checkpoint at POST-1340, opened once:
   assert `sum_base_credential_stake()` equals a fresh `reduce_txout` of cardano's POST-1340 reference UTxO,
   per full `StakeCredential`, 0 mismatches (the B3c.0 proof). Establishes base contributes zero, so the go
   residual is non-base.
2. **Advance (slow, one uninterrupted process).** Fresh isolated copies of the seed accumulator + checkpoint.
   Capture the seed `mark` (M0) per-credential BEFORE advancing. `co_advance` to POST-1342 (the same public
   primitives the live co-advancer calls).
3. **Phase-provenance assertion.** Assert `go(1342).delegations` ≡ the captured seed `mark` (M0) byte-for-byte —
   proving go(1342) is the imported snapshot, not live-derived.
4. **Per-phase per-credential differential.** For each phase, canonical `Hash28`-sorted diff:
   `matched / value-mismatch(same pool) / target-mismatch / only-Ade / only-cardano`, lovelace-exact.
5. **Classify + sum-conserve the go phase.** Assign every non-zero go delta to the closed cause; assert
   `Σ classified go deltas == −343,260,172,883` exactly.
6. **Fold invariant.** Assert `pool_stakes == fold(delegations)` on both sides for every phase.
7. **Localize.** Emit, per phase, the count and summed delta of each bucket, plus the largest per-credential
   deltas — so whether the −343B is go-only (seed import) or spans set/mark (live derivation) is a mechanically
   emitted fact.
8. **Pin + double.** Canonical path-free report (chain point, input-store blake2b, checkpoint fingerprint,
   reference-state hash, per-phase bucket sums, report hash). Run twice from independently prepared copies;
   require byte-identical reports + report hash.

## Acceptance (CE)

- The per-phase per-credential differential + go-phase classification is emitted by an `#[ignore]` local-artifact
  test, `Hash28`-sorted, lovelace-exact.
- Every non-zero go-phase component is assigned to a closed cause; the assigned sum equals −343,260,172,883
  exactly.
- Base-UTxO contribution asserted zero (POST-1340 checkpoint == reduction, 0 mismatches).
- `pool_stakes == fold(delegations)` holds on both sides for every phase (no double/absent folding).
- Replay-identical: two isolated single-process runs ⇒ byte-identical report + report hash.
- NO BLUE/authoritative change; NO rounding; NO compensating adjustment; NO aggregate-only match.

## Tiering

- **True**: deterministic replay; one authoritative snapshot result (`snapshots.*.delegations`, unchanged).
- **Derived**: cardano per-credential snapshot parity (the differential target).
- **Release**: the per-credential differential + the classified-residual regression fixture + report hash.
- **Operational**: local ChainDB corpus + the re-bootstrapped seed copies (isolated, single-process).

## Hard prohibitions (binding)

No rounding tolerance; no compensating offset; no external stake dump becoming live authority; no aggregate-only
correction; no broad governance expansion; no `MissingDRepActivityParam` work; diagnosis and fix NEVER in the
same change.

## What this slice does NOT do

No correction (that is the separate BLUE slice, gated on the cause named here); no `MissingDRepActivityParam`
work; no S6; no snapshot/reward-model change. Diagnosis only. When the cause is named, STOP and open the
corrective slice; then `MissingDRepActivityParam`, then S6.
