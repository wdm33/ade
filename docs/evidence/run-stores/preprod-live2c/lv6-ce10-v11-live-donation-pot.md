# LV-6 CE-LV6-10 — live v11 run: the treasury equals cardano's after 305→306

**Result:** a fresh store-semantics-v11 bootstrap of snapshot 6009 followed the local docker
`cardano-node-preprod` across 305→306. Afterwards the treasury and reserves equal the reference
exactly, and the donation pot is 0. The 1,000,000 lovelace gap from LV-4 is closed: it was the two
epoch-305 treasury donations of 500,000 each (LV-5).

## 1. Run

| item | value |
|---|---|
| binary | release build of `fec37fb9` (the LV-6 commit), sha256 `ae871bc8b6cf6923…` |
| mode | relay only: no operator keys, no genesis file |
| seed | `preprod-snapshot-6009`, certified anchor slot 129,813,427, seed artifact commit `7033018c…` |
| store | fresh, `~/.cardano-live1/ade-preprod-v11` (store semantics 11, accumulator schema 6) |
| peer | local docker `cardano-node-preprod` at `127.0.0.1:3001` |
| bootstrap | 2026-10-19T01:54:19Z |
| warm restart | 2026-10-19T02:46:45Z, same binary, same store |

The pots were read from copies of the store's `epoch-accumulator.redb` with
`leadership_distr_census::dump_current_pots` (it opens the store read-write, so only copies are read).

## 2. First leg: the pot holds the first donation

The node crossed 304→305 at slot 130,118,424, then stopped at the 306→307 header boundary with
`LeadershipEpochNotSealed { requested: 307 }`. That is the same known stop as in LV-4, before the
accumulator reached 305→306. A copy of the stopped store:

```
POTS applied_through_slot=130350114 epoch=305 treasury=1952221204641186 reserves=12986390967589170 donation_pot=500000
```

The accumulator is applied through slot 130,350,114. That includes the first donation (tx
`24428858…5cd5`, slot 130,271,804) but not the second (tx `853764ba…58a5`, slot 130,404,902). The pot is
500,000: the first donation only, credited by the live fold.

## 3. After 305→306: equal to the reference

The warm restart replayed forward and crossed the boundary:

```
bnd2c-transition: non_authoritative=true cursor 130350114 -> 130550399 epoch_fees 5858944830 -> 10290010819 (delta 4431065989) acc_fp f93d6f57 -> 98bd3694 cp_cursor=130550441
  [epoch_boundary] protocol_major=11 total_stake=32013609032410830 active_stake=1627117252973386 pool_pot=27525851410056 go_pools=444 cert_pools=558
epoch-accumulator: CROSSED boundary 305 -> 306 at slot 130550441 (mark from s_prev 130550399)
```

The store was copied as soon as the crossing line appeared. That copy:

```
POTS applied_through_slot=130550451 epoch=306 treasury=1959103174719172 reserves=12979123112128607 donation_pot=0
```

| after 305→306 | Ade v11 | reference | Ade v10 (LV-4) |
|---|---|---|---|
| treasury | 1,959,103,174,719,172 | 1,959,103,174,719,172 | 1,959,103,173,719,172 (−1,000,000) |
| reserves | 12,979,123,112,128,607 | 12,979,123,112,128,607 | 12,979,123,112,128,607 |
| donation pot | 0 | 0 after the boundary | — (no pot) |

The epoch fees, `total_stake`, `active_stake` and `pool_pot` at the boundary are the same as in the
v10 run. The only number that changed is the treasury, by exactly the two donations.

## 4. Follows on

The node walked epoch 306 and crossed the next boundary:

```
  [epoch_boundary] protocol_major=11 total_stake=32020876887871393 active_stake=1628085207615496 pool_pot=28406553377992 go_pools=446 cert_pools=557
epoch-accumulator: CROSSED boundary 306 -> 307 at slot 130982402 (mark from s_prev 130982398)
```

`active_stake` is 1,628,085,207,615,496, the reference go-snapshot total that LV-4 matched for this
boundary. The node was still running after the crossing (2026-10-19T03:30:43Z). Neither log has a
`failing closed` line after the restart, and neither has a `LeaderValueAboveThreshold`,
`RewardPoolParamsMissing` or `panicked` line at all.

## 5. Bounds

- The copy at §3 was taken from a running node. It opened, sealed and decoded cleanly, and it agrees
  with the reference to the lovelace.
- The stop at the 306→307 header boundary (`LeadershipEpochNotSealed`) is the known catch-up stop
  recorded in LV-4. It is outside LV-6 (slice §6).
