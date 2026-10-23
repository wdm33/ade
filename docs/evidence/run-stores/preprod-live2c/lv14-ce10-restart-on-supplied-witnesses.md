# LV-14 CE-LV14-10 — live: the same store restarts on the LV-14 binary and follows to the peer tip

**Result:** the store LV-13 was following restarted on the LV-14 binary, which verifies every supplied
witness signature. It recovered at its durable tip with nothing reset and caught up to the local docker
`cardano-node-preprod`, with no failure line.

## 1. The LV-13 run

| item | value |
|---|---|
| binary | release build of `e6910d5a` (LV-13), sha256 `13596e732f406a00…` |
| store | `~/.cardano-live1/ade-preprod-lv11`, semantics 11 |
| run | 2026-10-23T19:04:34Z to about 19:24Z, pid 1655886 |

It followed at the peer tip (last `follow: tip slot=137100226 -- AT PEER TIP`) and then stopped on its own
before any stop was sent: `loop_step=HaltCleanly`, then `relay run loop exited (... followed the live peer
until shutdown / feed-end)`. Its log has no `failing closed`, `panicked`, `HALTING`, `NativeScriptFailed` or
`ScriptWitness` line. The node does not log which of a signal or the end of the peer feed ended it, and this
record does not guess; it was started from a tool shell, so this run (LV-14) is started under `setsid` in
its own session. Durable tip at stop: 137,100,226 (block 5,327,015).

## 2. The LV-14 restart

| item | value |
|---|---|
| binary | release build of `df057072` (LV-14), sha256 `3220174bc7ccc960…` |
| mode | relay only, warm start (`--data-dir`, `--peer` only), own session (`setsid`) |
| store | the same `~/.cardano-live1/ade-preprod-lv11`, semantics 11, not bumped |
| start | 2026-10-23T19:36:02Z, pid 2126042 |

```
recovery-trace: path=recovery_admit action=forward_fold reason=forward_fold_no_reset anchor_before=137100226/5327015/101ce2fe durable_tip=137100226/5327015/101ce2fe rollback_target=none anchor_after=137100226/5327015/101ce2fe
follow: tip slot=137100270 behind=840 slots from peer
follow: tip slot=137101171 -- AT PEER TIP (caught up, following live)
```

At the peer tip by 19:40:09Z, with no `failing closed`, `panicked`, `HALTING`, `NativeScriptFailed`,
`ScriptWitness` or `WitnessClosure` line.

## 3. Bounds

The census (SLICE-LV-14 §8) covers the store up to 137,096,367; the blocks after it were verified live by the
LV-13 and LV-14 binaries. A short catch-up, not a soak.
