# LV-12 CE-LV12-8 — live: the same store restarts on the LV-12 binary and keeps following

**Result:** the store LV-11 resumed was stopped cleanly and restarted on the LV-12 binary, the one with
iterative CBOR skipping and flat native scripts. It recovered at its durable tip with nothing reset and
kept following the local docker `cardano-node-preprod`, with no failure line.

## 1. The LV-11 run, to its clean stop

| item | value |
|---|---|
| binary | release build of `a4610e16` (LV-11), sha256 `3eaceca25e84efb4…` |
| store | `~/.cardano-live1/ade-preprod-lv11` (copy of the stopped LV-8 store, semantics 11) |
| run | 2026-10-22T22:49:44Z to 2026-10-23T00:11:33Z, one process (pid 956970) |

After passing block 5,184,855 (LV-11 CE-LV11-7), the same process crossed 313→314 and 314→315
(`epoch-accumulator: CROSSED boundary 314 -> 315 at slot 134438444`). SIGINT was sent at
2026-10-22T23:40:34Z. The node finished the pass it was in, the post-boundary accumulator walk
(`walk_ms=2171037`), and halted at its safe point:

```
live2b-tick-probe: … loop_step=HaltCleanly
ade_node --mode node: relay run loop exited (… forge OFF — no operator keys supplied; followed the live peer until shutdown / feed-end). NO block produced.
```

Its log after the resume has no `failing closed`, `panicked`, `HALTING`, `LeadershipEpochNotSealed` or
`AccumulatorWalkHalted` line. The durable tip at stop was 134,438,444 (block 5,208,747).

## 2. The LV-12 restart

| item | value |
|---|---|
| binary | release build of `a82d11b8` (LV-12), sha256 `8d05d0ca11caaba9…` |
| mode | relay only, warm start (`--data-dir`, `--peer` only) |
| store | the same `~/.cardano-live1/ade-preprod-lv11`, semantics 11, not bumped (CE-LV12-7) |
| start | 2026-10-23T00:11:45Z, pid 1068056 |

```
recovery-trace: path=recovery_admit action=forward_fold reason=forward_fold_no_reset anchor_before=134438444/5208747/7017a02a durable_tip=134438444/5208747/7017a02a rollback_target=none anchor_after=134438444/5208747/7017a02a
follow: tip slot=134438457 behind=2592856 slots from peer
…
follow: tip slot=134531007 behind=2500400 slots from peer
```

That was five minutes in, at 00:16:56Z, with no failure line.

## 3. Bounds

This shows that a store written under the recursive implementation recovers and follows under the
flat one, which is what CE-LV12-7's argument predicts. The deep block 5,183,974 lies before this
store's tip, so this run did not re-decode it live. Its proof is in-tree, on a 256 KiB thread
(CE-LV12-1), and over the whole persisted interval (CE-LV12-7).
