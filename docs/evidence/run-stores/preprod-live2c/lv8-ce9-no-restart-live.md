# LV-8 CE-LV8-9 — live: a fresh bootstrap reaches epoch 310 with no restart

**Result:** a fresh bootstrap of snapshot 6009 on the LV-8 binary followed the local docker
`cardano-node-preprod` from epoch 304 into epoch 310 in one process, with no restart. It crossed both
regions where every earlier fresh bootstrap stopped (305→306 / 306→307 and 308→309) and followed
headers of epoch 310. Neither log has a `LeadershipEpochNotSealed`, `AccumulatorWalkHalted`, `HALTING`,
`failing closed` or `panicked` line.

## 1. Run

| item | value |
|---|---|
| binary | release build of `6b5c34b9` (LV-8 with the boundary guardrail), sha256 `e89a489a1360842d…` |
| mode | relay only: no operator keys, no genesis file |
| seed | `preprod-snapshot-6009`, certified anchor slot 129,813,427 |
| store | fresh, `~/.cardano-live1/ade-preprod-lv8b` (store semantics 11) |
| peer | local docker `cardano-node-preprod` at `127.0.0.1:3001` |
| start | 2026-10-20T23:28:09Z |
| restarts | none: one process (pid 1122950) for the whole run, still running at 2026-10-21T02:37:45Z |

## 2. Excluded: the earlier six-minute run

An earlier attempt started at 2026-10-20T23:17:13Z on a release build of `4a37624c` (sha256
`f4c0b4e37631d8a0…`), with store `~/.cardano-live1/ade-preprod-lv8` and log
`lv8-ce9-aborted-pre-guard-bootstrap.log`. It is **excluded from this proof** because it used the
pre-guard binary: `4a37624c` lacks the same-epoch condition that `6b5c34b9` added to the retry
decision. It was stopped after about six minutes, before reaching any boundary. Its store is
abandoned. No number in this record comes from it.

## 3. The two failing regions

The retry fired four times. Each was the typed checkpoint-behind case, and each passed on its single
retry:

```
epoch-accumulator: checkpoint behind the walk at 130350133 (checkpoint cursor Some(130118424)): CollateralBalance(UnresolvedCollateralInput { tx_index: 0, txin: TxIn { tx_hash: Hash32(0326ab20…
epoch-accumulator: checkpoint behind the walk at 130350647 (checkpoint cursor Some(130350133)): CollateralBalance(UnresolvedCollateralInput { tx_index: 1, txin: TxIn { tx_hash: Hash32(e947a330…
epoch-accumulator: checkpoint behind the walk at 131626888 (checkpoint cursor Some(131414402)): CollateralBalance(UnresolvedCollateralInput { tx_index: 0, txin: TxIn { tx_hash: Hash32(90a5961d…
epoch-accumulator: checkpoint behind the walk at 131627824 (checkpoint cursor Some(131626888)): CollateralBalance(UnresolvedCollateralInput { tx_index: 0, txin: TxIn { tx_hash: Hash32(d894b…
```

- 130,350,133 and 131,626,888 are the two blocks LV-7 measured (pairs A and B).
- 130,350,647 and 131,627,824 are different blocks, each a few hundred slots later, with a different
  collateral input that was also created after the checkpoint's cursor. LV-7 did not see them,
  because its runs had already stopped at the first block. Each is a distinct failing block and got
  its own single retry, as the rule allows.
- Each retry's cursor equals the block the previous retry advanced to, or a boundary point. The
  checkpoint only moved forward.

Every boundary was crossed in the same process:

```
epoch-accumulator: CROSSED boundary 304 -> 305 at slot 130118424 (mark from s_prev 130118358)
epoch-accumulator: CROSSED boundary 305 -> 306 at slot 130550441 (mark from s_prev 130550399)
epoch-accumulator: CROSSED boundary 306 -> 307 at slot 130982402 (mark from s_prev 130982398)
epoch-accumulator: CROSSED boundary 307 -> 308 at slot 131414402 (mark from s_prev 131414390)
epoch-accumulator: CROSSED boundary 308 -> 309 at slot 131846412 (mark from s_prev 131846389)
epoch-accumulator: CROSSED boundary 309 -> 310 at slot 132278419 (mark from s_prev 132278396)
```

The crossing at 308→309 seals leadership for 310. That crossing never ran in any earlier fresh
bootstrap.

## 4. First use of epoch 310

The earlier fresh bootstraps (v10 and v11) stopped at the follow tip 132,271,745 with
`LeadershipEpochNotSealed { requested: 310 }`. That happened right after the 309→310 nonce line,
before any epoch-310 header was followed. This run passed the same point:

```
follow: tip slot=132271745 behind=4593462 slots from peer
epoch-boundary yield: 309 -> 310 (eta0 Hash32(7c00177d…))
  [epoch_boundary] protocol_major=11 total_stake=32043094991318145 active_stake=1564451887616823 pool_pot=28430240131887 go_pools=447 cert_pools=559
epoch-accumulator: CROSSED boundary 309 -> 310 at slot 132278419 (mark from s_prev 132278396)
follow: tip slot=132278434 behind=4586773 slots from peer
follow: tip slot=132289789 behind=4577225 slots from peer
…
follow: tip slot=132354325 behind=4512689 slots from peer
```

Epoch 310 begins at slot 132,278,400. The node followed headers from 132,278,434 onward. Each
followed header needs the sealed leadership authority for its epoch, so this is the first use of 310,
and it succeeded.

## 5. Failure lines

Counted in `lv8-ce9-bootstrap.log` and the store's `node.log` at 2026-10-21T02:37:45Z:

| line | count |
|---|---|
| `LeadershipEpochNotSealed` | 0 |
| `AccumulatorWalkHalted` | 0 |
| `HALTING` | 0 |
| `failing closed` | 0 |
| `panicked` | 0 |

## 6. CI sweep

CI sweep observed 47 failures versus the recorded 46-count baseline. The additional failure was
independently reproduced nondeterministically on pre-LV-8 input and traced to the containment gate's
grep -q + pipefail pipeline; it is not an LV-8 semantic regression. The gate is
`ci/ci_check_node_run_loop_containment.sh`. It failed 4 of 6 runs on the LV-8 tree and 7 of 8 runs on
the pre-LV-8 file, and the 51,276-byte function body it reads is the same size in both.

## 7. Bounds

- This run proves the repair on the path that failed: catch-up from snapshot 6009 across 304→310. It
  does not show the node at the network tip. It was still about 4.5 million slots behind the peer
  when this record was taken.
- The 309→310 pass took about 29 minutes (`accumulator_loop_ms=1759655`, of which the walk took
  1,716,993). It admitted 19,823 blocks in one sync step. Pass time is outside LV-8.
