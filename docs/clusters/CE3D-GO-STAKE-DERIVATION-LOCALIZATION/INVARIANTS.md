# CE3D-GO-STAKE-DERIVATION-LOCALIZATION — invariants sketch (GREEN / evidence)

This cluster localizes the REAL −343,260,172,883 lovelace CE-3d go-stake residual to a closed set of
deterministic causes at the **credential level**, across the **mark/set/go snapshot phases**. The base-UTxO
pipeline was already proven byte-exact and EXONERATED (see `B3C-STAKE-RESIDUAL/B3c.0-adjudication-result.md`);
this cluster does NOT re-open that. It changes NO authoritative behaviour — it observes Ade's existing
`EpochAccumulator` snapshots and the cardano reference decode. The corrective change is a SEPARATE, later BLUE
slice, gated on the cause this cluster names (diagnosis and fix NEVER in the same change).

## The differential is credential-level, not aggregate

The per-credential go-stake is retained on BOTH sides in `snapshots.{mark,set,go}.0.delegations`, a
`BTreeMap<Hash28, (PoolId, Coin)>` (credential hash → delegation target + stake = base+reward). Ade builds it in
`build_boundary_mark_snapshot` (`ade_ledger/src/epoch_accumulator.rs`); the oracle decodes cardano's `ssStake`
map into the same shape in `read_stake_snapshot_full` (`ade_ledger/src/ledgerdb_state.rs`). The existing CE-3d
harness compares only the per-pool `pool_stakes` aggregate — a pool total can hide whether the error is an
omitted reward balance, a delegation-target/presence difference, a stale snapshot rotation, or double/absent
folding. This cluster iterates `delegations` (per credential) instead.

## What must ALWAYS be true

- **I-GSD-1 (one authoritative snapshot, observed not recomputed).** The differential reads Ade's existing
  `epoch_state.snapshots.{mark,set,go}.0.delegations` as-is (self-derived by the co-advance the live node runs);
  it introduces no second snapshot computation. The cardano side is decoded once via
  `decode_native_nonutxo_state`. (TCB: True — one authoritative result, deterministic replay.)
- **I-GSD-2 (canonical credential-sorted differential).** For each phase, the differential is keyed by `Hash28`
  in a `BTreeMap`/`BTreeSet` (canonical order), lovelace-exact per credential. Identical inputs (same seed stores
  advanced to the same point + same reference state) yield a byte-identical differential and report hash.
- **I-GSD-3 (complete, closed classification).** Every non-zero per-credential go-phase delta is assigned to
  EXACTLY ONE cause from the CLOSED set — `OnlyAde | OnlyRef | DelegationTargetMismatch | ValueDelta |
  SnapshotPhaseProvenance` — never a free-form string, never unassigned. Each cause maps to one of the user's
  named input dimensions (delegation presence / delegation target / reward-account contribution / snapshot phase
  & boundary point / pool folding).
- **I-GSD-4 (sum conservation — the credential-level acceptance bar).** The sum of the classified per-credential
  go-phase deltas equals the total residual −343,260,172,883 EXACTLY (Σ per-credential go delta ≡ Σ per-pool go
  delta, because `pool_stakes` is exactly the aggregation of `delegations` by pool). The acceptance is NOT "the
  total is close"; it is "every credential-level component assigned to a deterministic cause, summing exactly."
- **I-GSD-5 (base-UTxO contribution asserted zero).** The base-UTxO component contributes zero error: at the
  POST-1340 anchor the reduced checkpoint's per-credential base equals a fresh `reduce_txout` of cardano's
  reference UTxO byte-for-byte (0 mismatches — the B3c.0 proof, re-asserted in this harness). No classified cause
  is a base-UTxO defect.
- **I-GSD-6 (snapshot-phase provenance is explicit).** The harness records, per phase, whether Ade's snapshot is
  freshly live-derived by the co-advance (`mark(1342)` ← POST-1341, `set(1342)` ← POST-1340) or the bootstrap
  seed's imported snapshot rotated forward (`go(1342)` ≡ seed `mark`). Whether the −343B is go-only (a seed
  import) or spans set/mark (the live derivation) is a mechanically emitted fact, not an assumption.

## What must NEVER be possible

- **N-GSD-1 (no rounding / no tolerance).** No epsilon, no "within N lovelace," no percentage tolerance. A
  residual not fully assigned is a FAILURE, not a pass.
- **N-GSD-2 (no BLUE semantic change).** No change to snapshot construction, folding, rotation, the reduced-UTxO
  extraction, reward derivation, or any fingerprinted/authoritative output. GREEN/RED evidence only.
- **N-GSD-3 (no compensating adjustment / no aggregate-only match).** The differential must not add or subtract
  any value to force a match; the classification stands at the CREDENTIAL level, never merely at the aggregate.
- **N-GSD-4 (no external stake dump as live authority).** The cardano reference is decoded ONLY as the
  differential target (evidence), never fed back as a live stake source.
- **N-GSD-5 (no scope bleed).** No `MissingDRepActivityParam` work (a separate continuity slice); no S6 per-action
  work; no broader governance. This cluster localizes the go-stake residual and stops.

## What must remain identical across executions (replay)

- **R-GSD-1.** Two independently prepared isolated single-process copies of the seed stores, advanced to POST-1342
  over the same corpus + same reference state ⇒ byte-identical per-phase differential, classification, and a
  canonical report hash. Bound to the anchor identities (input-store blake2b, checkpoint fingerprint, reference
  state hash, chain point). `ReducedUtxoCheckpoint::open` / `EpochAccumulatorStore::open` are read-write, so each
  copy is opened once, exclusively, in one uninterrupted process (the B3c.0 evidence-harness control).

## Tiering

- **True**: deterministic replay; one authoritative snapshot result (the accumulator's `delegations`, unchanged).
- **Derived**: cardano snapshot per-credential stake parity (the differential target).
- **Release**: the per-credential differential corpus + the classified-residual regression fixture + report hash.
- **Operational**: local ChainDB extraction + the re-bootstrapped seed copies (isolated, single-process).

## Hard prohibitions (binding, carried from the user)

No rounding tolerance; no compensating offset; no external stake dump becoming live authority; no aggregate-only
correction; no broad governance expansion; no `MissingDRepActivityParam` work folded in; diagnosis and fix NEVER
in the same change. When the cause is named, STOP and open a separate narrow BLUE corrective slice; then
`MissingDRepActivityParam`, then S6.
