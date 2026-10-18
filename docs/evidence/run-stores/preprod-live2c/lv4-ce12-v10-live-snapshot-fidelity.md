# LV-4 live — every pool equal at the seed boundary; the retired pool paid at the next one

preprod, 2026-10-17. Slice: `SLICE-LV-4-snapshot-fidelity.md` (CE-LV4-11, CE-LV4-12). Earlier state:
`lv2-ce10-v9-live-seed-boundary-reward-native.md`.

**Result.** A fresh bootstrap from the same snapshot under LV-4 seals stake distributions that equal the
reference **for every pool** at both boundaries measured: 448 of 448 for epoch 306, and 447 of 447 for
epoch 307. Reserves after 305→306 are **exact**. The treasury is **1,000,000 lovelace** short at 305→306.
That gap was already in the v9 run, it is not a snapshot-fidelity matter, and its cause is not yet
confirmed (§4).

---

## 1. THE RUN

| | |
|---|---|
| snapshot | `preprod-snapshot-6009`, anchor slot 129,813,427 (epoch 304) |
| binary | built from the tree committed as `19387e86` (only the test kit, lock, registry and docs changed after the build), sha256 `831554492ddf3b51…` |
| store | `~/.cardano-live1/ade-preprod-v10`, fresh, store semantics 10 |
| mode | follower only — no operator keys, no `--genesis-file`; nothing can be forged |
| peer | local docker `cardano-node-preprod` (11.0.1), `127.0.0.1:3001` |

The seed was sealed as before:
`seed-boundary reward authority = NATIVE (snapshot nesRu SNothing; ssFee 23699540134)`.

## 2. CE-LV4-11 — the sealed seed crossed 304→305 against the reference reward update

`seed_boundary_reward_census::native_seed_boundary_reward_vs_cardano_rs` on a copy of the fresh store:

```
RESULT cardano_creds=11056 ade_creds=11056 exact=11052 differ=2 only_cardano=2 only_ade=2
RESULT reserves_draw=7422111617574 treasury_gain=7029706882108 prev_fees_after=4772459863
```

`900fbcf7…7fedb`, the script staker whose reward v9 credited to the key credential with the same bytes,
now matches. The rest are the residuals named in the v9 record:

- two credentials not registered at the seed point (`94857be3…`, `85ecddcb…`, together the +6,308,509
  treasury difference of this seed-only crossing; the live run replays them);
- two credits outside the reward update (`4245236a…` +1,000,000,000 and `7a507e54…` +500,000,000);
- two 1,000,000,000 credits to credentials in no go pool.

## 3. CE-LV4-12 — live

**(a) 304→305.**

```
[epoch_boundary] protocol_major=11 total_stake=32006186920793256 active_stake=1623266825127463 pool_pot=28118802294396
epoch-accumulator: CROSSED boundary 304 -> 305 at slot 130118424
```

Sealed epoch-306 leadership stake against the reference `stakeMark` at the end of 304
(`stake-snapshot-epoch305-all.json`):

```
reference 448 pools  total 1,626,474,372,102,448
Ade       448 pools  total 1,626,474,372,102,448   (+ 57 zero-stake pools the query omits)
exact 448 of 448      denominator delta 0
```

v9 was 11,802 short on pool `8ed5ab11…`. The header issuer `0ed90e52…b0e9` is 1,731,950,202,087 in both.

**(b) 305→306.** The node stopped at the 306→307 header boundary with
`LeadershipEpochNotSealed { requested: 307 }`, as v9 did, and a warm restart carried on:

```
bnd2c-transition: cursor 130350114 -> 130550399 epoch_fees 5858944830 -> 10290010819
[epoch_boundary] total_stake=32013609032410830 active_stake=1627117252973386 pool_pot=27525851410056
epoch-accumulator: CROSSED boundary 305 -> 306 at slot 130550441
```

| after 305→306 | Ade v10 | reference | v9 |
|---|---|---|---|
| reserves | 12,979,123,112,128,607 | 12,979,123,112,128,607 | +42,082,928,795 |
| treasury | 1,959,103,173,719,172 | 1,959,103,174,719,172 | −7,225,473 |

Reserves are exact: the retired pool `8ed5ab11…eea88` is now paid for its epoch-304 blocks from the
parameters frozen in the go snapshot. The treasury is 1,000,000 short (§4).

**(c) Sealed epoch-307 stake** against the reference `stakeMark` at the end of 305
(`stake-snapshot-epoch306-all.json`):

```
reference 448 pools  total 1,563,914,995,371,294
Ade       447 pools  total 1,563,914,995,371,294
exact 447 of 447      denominator delta 0
only reference: 8ed5ab11…eea88 with stake 0 (the retired pool; Ade's leadership set drops it)
```

v9 differed on `5cb14d2c…`, by −16,453,955: a former member of the retired pool who had not been paid.

**(d) It follows on.** After the restart the node walked epoch 306 and crossed 306→307 with
`active_stake=1628085207615496`. That equals the total of the reference's go snapshot for that boundary
(the `stakeRatio` denominator in `ledger-state-306.json`). It then crossed 307→308 at slot 131,414,402
and followed on past slot 131,568,535. There is no `failing closed`, `LeaderValueAboveThreshold` or
`RewardPoolParamsMissing` line anywhere in either log. From the restart on, progress lines go to the
store's `node.log`, not the launcher's log.

## 4. THE 1,000,000 TREASURY GAP — not explained here

- It is not LV-4's. v9's −7,225,473 was −6,225,473 from the retired pool's unpaid rewards plus this
  same −1,000,000. With the reserves exact, the reward calculation is not the source.
- **Lead, not a finding:** Ade's accumulator does not handle Conway treasury donations at all. They
  appear only in validation and in the seed import's skip comments. A 1-ADA donation during epoch 305
  would produce exactly this. Not confirmed: the captured reference JSON omits the donation pot, and no
  block of epoch 305 has been checked for a donation field.
- **Owner decision, 2026-10-18.** The gap is reclassified to its own derived compatibility slice,
  LV-5. DC-EPOCH-43/44 are `enforced`: their statements claim no treasury or whole-state equality.
  This record makes no claim of full epoch-state equality with cardano, and none is made until LV-5
  closes.

## 5. ARTIFACTS (off-repo)

```
~/.cardano-live1/lv4-ce12-v10-bootstrap.log     the run
~/.cardano-live1/lv4-ce12-v10-restart1.log      after the warm restart
<scratch>/lv4-ce11.log                          §2
<scratch>/lv4-306.log, lv4-307.log              §3 (a), (c)  — cmp306.py / cmp307.py
<scratch>/lv4_mut.log                           the ten mutations
```
