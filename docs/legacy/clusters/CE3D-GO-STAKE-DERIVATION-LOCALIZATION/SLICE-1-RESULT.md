# Slice 1 — result (the −343B is a per-credential reward-account stake-value difference, NOT base/folding/delegation)

The credential-level differential localizes the entire −343,260,172,883 lovelace CE-3d go-stake residual to a
single closed cause. GREEN/evidence only — NO BLUE change.

## The credential-level differential (fast, doubled byte-identical)

`gsd_go_phase_credential_differential` diffs Ade's `go(1342)` (the seed accumulator's imported `mark`, rotated
forward — see provenance below) against cardano's decoded `go(1342)`, per credential (`Hash28 → (PoolId, Coin)`),
classifying every non-zero delta into the closed cause set. Two independent processes produced a byte-identical
report (`report_hash = 1e07cc50ee1bf14b3c5520fc3ba68694e969fdad7a366b5824f9f18c7492d385`):

```
chain_point            = slot:115948834 | epoch:1342
input_seed_accumulator = c19429e8244ac56f5034c5e33b22bb1f2fdf923b6cee57bfacb0ccf93a4be7ed
reference_state        = ac2329cca7e4df4701c32bd8b85a0acf6ae6021f4b30ab1ef8539160758b9564
ade_go_creds = 60337 | card_go_creds = 59700
ade_go_pools = 658   | card_go_pools = 626
ade_go_total = 1,674,023,071,155,299 | card_go_total = 1,674,366,331,328,182 | go_residual = -343,260,172,883
only_ade         : count = 637    sum = 0                 <- phantom 0-stake credentials (32 phantom pools)
only_ref         : count = 0      sum = 0
target_mismatch  : count = 0      sum = 0
value_delta      : count = 55820  sum = -343,260,172,883  <- the ENTIRE residual
matched          : 3880
classified_sum   = -343,260,172,883   (== go_residual, exactly)
fold_ok_ade = true | fold_ok_card = true
```

## Classification (every lovelace assigned to a deterministic cause)

- **`value_delta` = −343,260,172,883 (100% of the residual), 55,820 credentials.** Each is present on BOTH sides,
  delegated to the SAME pool, with a DIFFERENT per-credential stake amount. This is a per-credential stake-VALUE
  difference — the user's **reward-account contribution** dimension.
- **`only_ade` sum = 0** (637 credentials Ade's go carries that cardano's go does not, all carrying ZERO stake —
  the phantom 0-stake credentials behind the 658-vs-626 pool-count gap; they add nothing to the residual).
- **`only_ref` = 0, `target_mismatch` = 0** — the residual is NOT delegation presence and NOT delegation target.
- **`fold_ok` = true on both sides** — `pool_stakes == fold(delegations)`, so it is NOT a double/absent folding
  defect. The pool-level aggregate was hiding a purely per-credential value difference.

## Base-UTxO contribution asserted zero (I-GSD-5)

`gsd_base_zero_at_post1340`: the durable reduced checkpoint's per-credential base equals a fresh `reduce_txout`
of cardano's POST-1340 reference UTxO byte-for-byte — total 3,853,775,699,903,323 == 3,853,775,699,903,323,
**0 mismatches across all 254,385 credentials** (the B3c.0 proof, re-asserted here). So the base-UTxO pipeline
contributes zero error; a same-credential/same-pool stake-value difference is therefore the reward-account
(non-base) component of the stake.

## Snapshot-phase provenance (I-GSD-6)

`go(1342)` is the seed's imported `mark` (M0) rotated forward twice — `rotate_snapshots` is a pure clone
(`go := set; set := mark; mark := fresh`), and the seed accumulator's `mark` (658 pools /
1,674,023,071,155,299) is exactly Ade's `go(1342)` total. The residual is therefore carried on the go phase,
which traces to the **bootstrap seed's imported mark snapshot**, not a freshly live-derived snapshot.

`gsd_provenance_and_live_derivation` (SLOW, advance to POST-1342, both boundaries crossed cleanly) proves
`go(1342).delegations` == the seed's imported `mark` byte-for-byte (`go_equals_seed_mark = true`), and emits the
FRESH live-derived `mark(1342)` (← POST-1341) / `set(1342)` (← POST-1340) per-credential differential vs cardano
(`report_hash = 8b254305d5028ce23603ee2550d2f057c1ea4a042b324559ea0ed8b838a96b29`):

```
go_equals_seed_mark = true
mark(1342)  residual = -355,446,908,982   (pure value_delta; only_ref = target_mismatch = 0)
set(1342)   residual = -363,268,230,670   (pure value_delta; only_ref = target_mismatch = 0)
go(1342)    residual = -343,260,172,883   (pure value_delta; == the pinned residual)
```

**Outcome (b): a live reward-derivation discrepancy — NOT merely a seed-import artifact.** The freshly
live-derived `mark(1342)` and `set(1342)` ALSO diverge from cardano, as pure per-credential `value_delta` (same
credential, same delegation target), of the same ~−350B magnitude. `set(1342)` is derived from the POST-1340
base, which is exonerated byte-for-byte (I-GSD-5), so its −363,268,230,670 value_delta is unambiguously the
**reward-account contribution**. The discrepancy is therefore in Ade's LIVE snapshot derivation (the reward
balances folded into `build_boundary_mark_snapshot`), not confined to the bootstrap seed's imported go.

## Cause named (for the separate BLUE corrective slice)

The −343,260,172,883 residual is a **per-credential reward-account stake-value difference on delegated
credentials, present in the LIVE-derived go-stake snapshot** (base exonerated, delegation structure and folding
correct). It is NOT the base UTxO, NOT delegation presence/target, NOT pool folding, NOT confined to the seed
import. The narrow BLUE corrective slice (separate, gated on this cause) targets the **reward-account
contribution folded into `build_boundary_mark_snapshot`** — i.e. the per-credential reward balances the snapshot
derivation combines with base UTxO. Diagnosis stops here; no BLUE change was made.

Note (scope boundary): the −500,037,651,836 reward-map residual is the CONWAY-PROPOSAL-DEPOSIT gov-refund gap on
UNDELEGATED accounts (closed in CPDE; this seed predates CPDE S1). It is disjoint from this go-stake residual
(delegated credentials) at the credential level, but both are reward-account derivation discrepancies and a BLUE
reward slice should check whether they share a root cause.
