# LV-13 CE-LV13-12 — live: the same store restarts on the LV-13 binary and follows to the peer tip

**Result:** the store LV-12 was following was stopped cleanly, censused in full, and restarted on the LV-13
binary, which rejects on native-script witness verdicts. It recovered at its durable tip with nothing reset
and caught up to the local docker `cardano-node-preprod`, with no failure line.

## 1. The LV-12 run, to its clean stop

| item | value |
|---|---|
| binary | release build of `a82d11b8` (LV-12), sha256 `8d05d0ca11caaba9…` |
| store | `~/.cardano-live1/ade-preprod-lv11`, semantics 11 |
| run | 2026-10-23T00:11:45Z to 19:02:16Z, one process (pid 1068056) |

It had been following at the peer tip (`follow: tip slot=137096367 -- AT PEER TIP`). SIGINT at 18:40:09Z; it
finished its accumulator pass and halted at its safe point (`loop_step=HaltCleanly`). The run's log has no
`failing closed`, `panicked` or `HALTING` line. Durable tip at stop: 137,096,367 (block 5,326,835).

## 2. The census of that store

A byte copy of its `chain.db` (326,471 blocks, slots 129,813,444..=137,096,367) went through
`persisted_script_witness_census` before the LV-13 reject path was used live: 991,602 transactions,
46,866 witness native scripts all passing, 866,432 fully resolved transactions all exactly valid, zero
findings (SLICE-LV-13 §8). So the restarted node holds no block the new rule refuses.

## 3. The LV-13 restart

| item | value |
|---|---|
| binary | release build of `e6910d5a` (LV-13), sha256 `13596e732f406a00…` |
| mode | relay only, warm start (`--data-dir`, `--peer` only) |
| store | the same `~/.cardano-live1/ade-preprod-lv11`, semantics 11, not bumped |
| start | 2026-10-23T19:04:34Z, pid 1655886 |

```
recovery-trace: path=recovery_admit action=forward_fold reason=forward_fold_no_reset anchor_before=137096367/5326835/5946504b durable_tip=137096367/5326835/5946504b rollback_target=none anchor_after=137096367/5326835/5946504b
follow: tip slot=137096397 behind=2716 slots from peer
follow: tip slot=137099113 -- AT PEER TIP (caught up, following live)
```

At the peer tip by 19:07:47Z, with no `failing closed`, `panicked`, `HALTING`, `NativeScriptFailed` or
`ScriptWitness` line.

## 4. Bounds

The live node is a reduced follower (`track_utxo = false`), so live it runs the reduced level: every
witness-set native script of every new block is evaluated and a failure would reject the block. The exact
level is exercised by the census and the tests, not live. A short run after catch-up shows the new rule
accepts the blocks cardano-node serves; it is not a soak.
