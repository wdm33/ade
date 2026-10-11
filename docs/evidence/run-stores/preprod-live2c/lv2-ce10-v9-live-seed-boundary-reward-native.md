# LV-2 live — the seed boundary now pays, and the header that was rejected is accepted

preprod, 2026-10-11. Slice: `SLICE-LV-2-seed-boundary-reward-authority.md` (CE-LV2-3, CE-LV2-10).
Earlier state of the same defect:
`lv1-ce9-v7-live-NOT-FIXED-seed-boundary-reward-update-absent.md`.

**Result.** A fresh bootstrap from the same snapshot, with LV-2 and LV-3, pays the first boundary,
seals a stake distribution that equals the reference's for **447 of 448 pools**, accepts the header
the previous store rejected, and follows on. The one pool that differs is off by **11,802 lovelace** —
exactly the key/script credential residual already recorded and owned by slice LV-4. So the bar as
written (448 of 448) is **not** met; it is one known credential short.

---

## 1. THE RUN

| | |
|---|---|
| snapshot | `preprod-snapshot-6009`, certified anchor slot 129,813,427 (slot-in-epoch 127,027 of epoch 304) |
| binary | LV-2 working tree on `67ba3e3c`, sha256 `9ceda23afc36ead4…d50ce2cc` |
| store | `~/.cardano-live1/ade-preprod-v9`, fresh, store semantics 9 |
| mode | follower only — no operator keys, so nothing can be forged |
| peer | local docker `cardano-node-preprod` (11.0.1), `127.0.0.1:3001` |
| start | 2026-10-11T14:16:51Z |

A first attempt four minutes earlier bootstrapped, sealed the seed, and then exited at once: it was
launched with `--genesis-file` but no keys, and the node refuses a partial operator key set. That
store was deleted and the run restarted without the flag. Its log is kept
(`lv2-ce10-v9-attempt1-partial-keyset.log`).

## 2. WHAT THE SEED WAS SEALED AS

```
ade_node native-firstrun: seed-boundary reward authority = NATIVE (snapshot nesRu SNothing; ssFee 23699540134)
```

The reference state at the seed has `nesRu = SNothing` and `feeSS = 23,699,540,134`. The same snapshot
under the previous binary was sealed as an empty completed update.

## 3. CE-LV2-3 — the sealed seed, crossed by the production transition, against the reference update

`seed_boundary_reward_census::native_seed_boundary_reward_vs_cardano_rs` on a copy of the fresh
store: the sealed bootstrap accumulator is decoded and crossed into epoch 305 by
`cross_epoch_boundary` exactly as sealed. Nothing is emulated.

```
SEED authority=NATIVE fee_ss=23699540134
[epoch_boundary] pool_pot=28118802294396
cardano credentials 11,056    exact 11,051    differ 2    only-cardano 3    only-Ade 3
reserves draw   7,422,111,617,574     = reference deltaR, exact
fee pot rotated 4,772,459,863         = 28,471,999,997 - 23,699,540,134
```

The five credentials that do not match, unchanged from the pre-code measurement:

| credential | amount | what |
|---|---|---|
| `94857be3…4dfd5b`, `85ecddcb…99163` | 6,296,662 + 11,847 | not registered at the seed point; this test crosses the SEED state, before the rest of the epoch is replayed. Both go to the treasury here, which is the whole of the +6,308,509 treasury difference |
| `900fbcf7…7fedb` | 11,802 | a script credential's reward credited to the key credential with the same 28 bytes — LV-4 |
| `7a507e54…98756`, `4245236a…a6454` | +500,000,000, +1,000,000,000 | credits at the same boundary that are not in the reward update (a pool deposit refund; one unidentified) |

Whether the first row is really only an artifact is answered by §4: the live run replays the rest of
the epoch, and the pools those credentials delegate to come out exact.

## 4. CE-LV2-10 — live

**(a) The first boundary pays.**

```
[epoch_boundary] protocol_major=11 total_stake=32006186920793256 active_stake=1623266825127463 pool_pot=28118802294396 go_pools=439 cert_pools=559
epoch-accumulator: CROSSED boundary 304 -> 305 at slot 130118424 (mark from s_prev 130118358)
```

The previous store logged `pool_pot=0` on the same line.

**(b) The sealed epoch-306 leadership distribution against the reference's end-of-304 snapshot.**
Reference: `stake-snapshot-epoch305-all.json` (`stakeMark`, all pools), captured from a second
cardano-node stopped inside epoch 305.

```
reference   448 pools                  total 1,626,474,372,102,448
Ade         448 pools with stake       total 1,626,474,372,090,646     (-11,802)
            + 57 zero-stake pools, which the reference query does not list
exact       447 of 448
differ      1    8ed5ab11…eea88    Ade 63,136,879,302,990   reference 63,136,879,314,792   (-11,802)
```

The previous store was 408,132,859,573 short across 75 pools. The one remaining pool is the pool the
script credential `900fbcf7…7fedb` delegates to, and the difference is that credential's reward to
the lovelace.

The issuer of the rejected header, pool `0ed90e52…b0e9`: Ade 1,731,950,202,087 = reference
1,731,950,202,087.

**(c) The header that was rejected.** The previous store halted on the header after durable tip
130,733,645 with `LeaderValueAboveThreshold`. This run:

```
follow: tip slot=130733645
follow: tip slot=130745349
follow: tip slot=130757157
```

No `LeaderValueAboveThreshold` and no `failing closed` line anywhere in the log.

**(d) It follows on.** At the time of writing the tip is past 130,802,866, inside epoch 306, with the
node still running.

## 5. WHAT THIS DOES AND DOES NOT SHOW

- The root cause was the one named: the seed's reward update was absent, Ade wrote it down as empty,
  and the first boundary paid nothing. With a typed owner for that boundary the distribution is right
  to one credential.
- LV-3 (owner stake) is exercised by the same boundary: the pool it stopped paying is among the 447.
- **Not shown:** the 448th pool (LV-4); anything about `Pulsing` seeds; any other venue; forging — this
  run carries no keys. B12's live bar is a separate leg.
- **Seen and not new:** the accumulator's first pass stops at block 130,350,133
  (`UnresolvedCollateralInput`, observe-only) until the checkpoint has moved past it. The previous
  store logged the same line; it is the known positioning sequence, not a result of this change.
- The sealed denominator is 11,802 lower than the reference's — 7 parts in 10^12. It moved no header
  here. It is still a difference in a consensus input, which is why LV-4 is not optional.

## 6. ARTIFACTS (off-repo)

```
~/.cardano-live1/lv2-ce10-v9-bootstrap.log                 the run
~/.cardano-live1/lv2-ce10-v9-attempt1-partial-keyset.log   the aborted first launch
~/.cardano-live1/lv2-evidence/lv2-ce3.log                  §3
~/.cardano-live1/lv2-evidence/lv2-306.log, cmp306.py       §4(b)
~/.cardano-live1/lv2-evidence/lv2_mut.log                  the nine mutations
~/.cardano-live1/oracle-end304-node/stake-snapshot-epoch305-all.json
~/.cardano-live1/oracle-rupd304-node/rupd-304to305-rs-by-credential.txt
```
