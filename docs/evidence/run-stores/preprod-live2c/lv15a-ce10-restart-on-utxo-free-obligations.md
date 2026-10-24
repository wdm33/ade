# LV-15a CE-LV15a-10 — live: the same store restarts on the LV-15a binary and follows to the peer tip

**Result:** the store LV-14 was following restarted on the LV-15a binary. That binary checks the validity
interval, every network id, bootstrap attribute size and the declared body size at admission. It recovered
at its durable tip with nothing reset and caught up to the local docker `cardano-node-preprod`, with no
failure line.

## 1. The LV-14 run

| item | value |
|---|---|
| binary | release build of `df057072` (LV-14), sha256 `3220174bc7ccc960…` |
| store | `~/.cardano-live1/ade-preprod-lv11`, semantics 11 |
| run | 2026-10-23T19:36:02Z to 2026-10-24T17:16:18Z (about 21 h 40 min), pid 2126042, own session (`setsid`) |

It ran without stopping on its own. The LV-13 run stopped cleanly with no recorded cause, and that did not
happen again in this run. It was stopped for the restart:
1. SIGINT was sent first, and the process was still running when checked.
2. SIGTERM was then sent, and it exited by 17:16:18Z.

The last node-log lines are a `HaltCleanly` loop step. Its log has no `failing closed`, `panicked` or
`HALTING` line. Durable tip at stop: 137,174,181 (block 5,330,424, `e467a222…`).

## 2. Census of that store before the restart

`persisted_utxo_free_obligation_census` was run over a copy of the stopped store:
- **Coverage:** 330,060 blocks (slots 129,813,444..=137,174,181) and 997,827 transactions.
- **What the checks had to judge:** 497,212 transactions with a validity bound (tightest margin one slot on
  each side), 85,868 with withdrawals, 400,654 with a body network id, and 412 bootstrap outputs (largest
  attributes 28 bytes).
- **Result:** 0 failures. Every persisted block passes the new checks.

## 3. The LV-15a restart

| item | value |
|---|---|
| binary | release build of `bbde0036` (LV-15a), sha256 `2a835a2022f8bc3c…` |
| mode | relay only, warm start (`--data-dir`, `--peer` only), own session (`setsid`) |
| store | the same `~/.cardano-live1/ade-preprod-lv11`, semantics 11, not bumped |
| start | 2026-10-24T17:17:15Z, pid 1531060 |

```
recovery-trace: path=recovery_admit action=forward_fold reason=forward_fold_no_reset anchor_before=137174181/5330424/e467a222 durable_tip=137174181/5330424/e467a222 rollback_target=none anchor_after=137174181/5330424/e467a222
follow: tip slot=137174197 behind=4976 slots from peer
follow: tip slot=137179173 -- AT PEER TIP (caught up, following live)
```

A second `recovery_admit` trace follows in the same run, at durable tip 137,179,173, again
`forward_fold_no_reset`. It is recorded as observed. At 17:22:20Z the node was at slot 137,179,326, block
5,330,670. The peer's `cardano-cli query tip` gave the same slot and block. The log has no `failing closed`,
`panicked`, `HALTING`, `UtxoFreeObligation` or `admission_admit_rejected` line.

## 4. Bounds

The census covers the store up to 137,174,181. The blocks after it were checked live by the LV-15a binary.
This was a short catch-up, not a soak.
