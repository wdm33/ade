# B3c.0 — adjudication result (the −343B go-stake residual is REAL; base UTxO is exonerated)

A sealed, GREEN-only adjudication pass re-ran the ORIGINAL CE-3d go-stake decomposition path to settle whether
the −343,260,172,883 lovelace residual is real or a contaminated-measurement artifact — BEFORE committing any
conclusion. It is REAL. Combined with the byte-exact base-UTxO proof, the open problem is renamed from a
"B3c base-UTxO undercount" to a **go-stake DERIVATION discrepancy**.

## Method (sealed, per the adjudication rules)

- **Same original path**, not a substitute: `b3c0_adjudication_go_stake` (in the original ce3d harness) reuses
  the real `co_advance` + `ade_post_state` / `ref_post_state` decomposition, advancing the accumulator to
  POST-1342 and diffing Ade's self-derived go against cardano's go.
- **Fresh, copy-verified, isolated stores**: two independent copies (A, B) of the re-bootstrap seed accumulator +
  reduced checkpoint, `sha256`-identical before the runs.
- **One process, uninterrupted**: each run advances to POST-1342 in a single process, never interrupted, never a
  concurrent redb open. (`ReducedUtxoCheckpoint::open` is read-write — analysis must run from an isolated stable
  copy with exclusive ownership; an OPERATIONAL/RELEASE-EVIDENCE control, not a BLUE-bug claim.)
- **Pinned + doubled**: the canonical (path-free) report records the chain point, input-store blake2b hashes, the
  reference-state hash, and a report hash; two independent runs must produce byte-identical reports.

## Result (run A, uninterrupted, both boundaries crossed cleanly)

```
chain_point                = slot:115948834 | epoch:1342
input_accumulator_blake2b  = c19429e8244ac56f5034c5e33b22bb1f2fdf923b6cee57bfacb0ccf93a4be7ed
input_checkpoint_blake2b   = 5d5ea64ae16452bc5503b8ece02226100f2ddf05c2d7b7af7b5b68e2f2ff8a89
reference_state_blake2b    = ac2329cca7e4df4701c32bd8b85a0acf6ae6021f4b30ab1ef8539160758b9564
ade_go_total               = 1,674,023,071,155,299   (658 pools)
card_go_total              = 1,674,366,331,328,182   (626 pools)
go_stake_residual          = -343,260,172,883        <-- REPRODUCES, to the lovelace
reward_residual            = -500,037,651,836         (the CPDE gov-refund gap; this seed predates CPDE S1)
treasury_residual          = +232,057,252
reserves_residual          = +962,654,839
report_hash                = 6b04d8c0de217b408ca8bd44e003de6922bc37224080fe6272490032939252d9
```

Run B (an independent, `sha256`-verified copy, run sequentially and uninterrupted) produces a **byte-identical**
report: identical `go_stake_residual=-343,260,172,883` and identical
`report_hash=6b04d8c0de217b408ca8bd44e003de6922bc37224080fe6272490032939252d9`. The doubled requirement is met —
the residual is deterministic and real.

## Adjudication

- **The −343,260,172,883 go-stake residual is REAL** — it reproduces exactly, cleanly, deterministically.
- **The base UTxO is EXONERATED** — `b3c0_clean_checkpoint_vs_reduction` proves the durable reduced checkpoint
  equals a fresh `reduce_txout` of the reference UTxO byte-for-byte (all 254,385 credentials, total
  3,853,775,699,903,323, diff 0). So the residual is NOT a base-UTxO undercount; the earlier "base-UTxO"
  attribution — and my own initial reproduction of it — were checkpoint-handling artifacts (killed mid-advance +
  redb copy/open races).

## Outcome (per the adjudication branch): OUTCOME 2

The residual reproduces ⇒ **rename the open problem** from "B3c base-UTxO undercount" (closed / non-existent) to a
narrower **go-stake / reward DERIVATION discrepancy**. The next evidence slice localizes it across the three
candidates, none of which is the base UTxO:

1. **delegation folding** — how `aggregate_pool_stake` groups per-credential base UTxO into per-pool go (the
   delegation map: cred → pool);
2. **reward-account contribution** — the reward component folded into go for DELEGATED credentials (note the
   −500B reward residual is the CPDE gov-refund gap on UNDELEGATED accounts, so it is separate — but a
   delegated-cred reward component must be checked);
3. **go-snapshot construction** — the mark→set→go rotation the accumulator performs across boundaries.

**B3c.1 stays CLOSED** — there is no base-UTxO defect to fix. No BLUE change was made in B3c.0.

## Note on the CE-3d differential

Because the −343B is real (not a measurement artifact), the CE-3d byte-exact differential remains blocked on the
go-derivation discrepancy above, not on the base UTxO. The full CE-3d differential should be re-run under these
clean-harness rules (fresh isolated copies, one uninterrupted process) once the go-derivation slice lands.
