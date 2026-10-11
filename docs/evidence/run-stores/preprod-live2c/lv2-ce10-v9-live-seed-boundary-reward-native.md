# LV-2 live — the seed boundary now pays, and the header that was rejected is accepted

preprod, 2026-10-11. Slice: `SLICE-LV-2-seed-boundary-reward-authority.md` (CE-LV2-3, CE-LV2-10).
Earlier state of the same defect:
`lv1-ce9-v7-live-NOT-FIXED-seed-boundary-reward-update-absent.md`.

**Result.** A fresh bootstrap from the same snapshot, with LV-2 and LV-3, pays the first boundary,
seals a stake distribution that equals the reference's for **447 of 448 pools**, accepts the header
the previous store rejected, and follows on. The one pool that differs is off by **11,802 lovelace** —
exactly the key/script credential residual already recorded and owned by slice LV-4. So the bar as
written (448 of 448) is **not** met; it is one known credential short.

Following the run one boundary further found **a third reward defect**, unrelated to the seed: a pool
that retires is not paid for the blocks of its final epoch (§6).

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

## 6. THE SECOND BOUNDARY (305→306) — one more defect, found and identified

The node stopped at the 306→307 boundary with `LeadershipEpochNotSealed { requested: 307 }`: the
accumulator had not yet crossed 305→306 when the tip got there (its first pass stops at the
phase-2-invalid block until the checkpoint has moved past it). A warm restart on the same store
carried on, and the accumulator crossed:

```
bnd2c-transition: cursor 130350114 -> 130550399 epoch_fees 5858944830 -> 10290010819
[epoch_boundary] total_stake=32013609032410830 active_stake=1627117252973386 pool_pot=27525851410056
epoch-accumulator: CROSSED boundary 305 -> 306 at slot 130550441
```

What is exact at this boundary:

| quantity | Ade | independent value |
|---|---|---|
| reserves after 304→305 (implied by `total_stake`) | 12,986,390,967,589,170 | seed reserves − reference `deltaR` |
| active stake of the go snapshot | 1,627,117,252,973,386 | reference `total.stakeGo` |
| fee input of this boundary | 22,236,845,053 | public fee total of epoch 304 |
| fees of epoch 305, rotated out | 10,290,010,819 | public fee total of epoch 305 = reference `feeSS` in 306 |
| pool pot | 27,525,851,410,056 | recomputed from the three rows above and 19,064 blocks |

So LV-2's fee handling (CE-LV2-4) is confirmed live on the boundary after the seed.

What is not:

```
sealed epoch-307 stake vs the reference's end-of-305 snapshot
  exact   446 of 447 pools
  differ  1    5cb14d2c…1bddc   -16,453,955     (one credential, 32a4d66f…1134)
reserves after 305→306    Ade 12,979,165,195,057,402   reference 12,979,123,112,128,607   (+42,082,928,795)
treasury after 305→306    Ade  1,959,103,167,493,699   reference  1,959,103,174,719,172   (-7,225,473)
```

**Cause, confirmed against the reference.** Pool `8ed5ab11…eea88` retired at the 304→305 boundary.
It made blocks in epoch 304, its last. Those blocks are rewarded at 305→306, one boundary after the
pool has been removed from the registrations. cardano computes rewards from the pool parameters
frozen in the go snapshot, so it still pays that pool's operator and members. Ade looks the pool up
in the CURRENT registrations, does not find it, and pays nothing.

- In the reference's go snapshot for this boundary the pool has stake 66,476,670,913,199 (saturated),
  pledge met, 669 of 16,609 blocks at the last capture inside 304. Its maximum reward works out at
  42,347,662,127; the reserves gap is 42,082,928,795.
- The credential behind the one differing pool was a member of the retired pool in that snapshot and
  had re-delegated to `5cb14d2c…` by the end of 305. Reference balance 43,318,478, Ade 26,864,523:
  the difference is 16,453,955, the same number as the pool difference.
- The former members who have not re-delegated are in no pool's stake, which is why 446 pools still
  match.

This is a defect in the shared reward function, older than this slice, and not a seed-boundary
matter: it happens whenever a pool retires after producing blocks in its final epoch. It was on the
slice's own list as entry obligation E2 ("cardano rewards from the pool parameters frozen in the go
snapshot; Ade reads the current registrations"), which was checked at the first boundary only, where
no pool had yet been removed.

Ade's stake snapshot does not carry pool parameters at all, so the repair is a change to what a
snapshot holds — the same kind of change the key/script residual needs (the snapshot also drops the
credential's key/script tag). Neither is attempted here.

## 7. ARTIFACTS (off-repo)

```
~/.cardano-live1/lv2-ce10-v9-bootstrap.log                 the run
~/.cardano-live1/lv2-ce10-v9-attempt1-partial-keyset.log   the aborted first launch
~/.cardano-live1/lv2-evidence/lv2-ce3.log                  §3
~/.cardano-live1/lv2-evidence/lv2-306.log, cmp306.py       §4(b)
~/.cardano-live1/lv2-evidence/lv2_mut.log                  the nine mutations
~/.cardano-live1/lv2-ce10-v9-restart1.log                  the warm restart, §6
~/.cardano-live1/lv2-evidence/lv2-307.log, cmp307.py       §6 pool comparison
~/.cardano-live1/lv2-evidence/lv2-mark5cb.log              §6 per-credential view of the differing pool
~/.cardano-live1/oracle-end304-node/ipc/ledger-state-306.json           reference state at slot 130,563,009
~/.cardano-live1/oracle-end304-node/stake-snapshot-epoch306-all.json    end-of-305 mark, all pools
~/.cardano-live1/oracle-end304-node/stake-snapshot-epoch305-all.json
~/.cardano-live1/oracle-rupd304-node/rupd-304to305-rs-by-credential.txt
```
