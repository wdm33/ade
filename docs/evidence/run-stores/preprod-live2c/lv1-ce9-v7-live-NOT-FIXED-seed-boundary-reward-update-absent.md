# CE-LV1-9 — LIVE: NOT FIXED. The halt is a missing reward update, not a denominator.

preprod, 2026-10-10. Slice: `SLICE-LV-1-sigma-denominator-authority.md`. Code: `b502bd5c`
(DC-EPOCH-40, `partial`).

**Result in one line.** A fresh v7 bootstrap reached the header that halted v6 and rejected it with
byte-identical operands. LV-1 did not change the outcome. A reference node, rebuilt for the disputed
epoch, shows why: Ade's end-of-304 stake snapshot is short by exactly the rewards cardano credited at
the 304→305 boundary — the first boundary after the seed.

This file WITHDRAWS three recorded claims (§6). It changes no code.

---

## 1. PROVENANCE

| | |
|---|---|
| binary | `b502bd5c`, `target/release/ade` sha256 `b3f4bf59ed94ca48cb78…`, clean tree |
| store | `~/.cardano-live1/ade-preprod-v7` — fresh `--bootstrap-mithril` from `preprod-snapshot-6009` |
| seed | slot 129,813,427 — epoch 304, slot-in-epoch 127,027 |
| peer | docker `cardano-node-preprod` 127.0.0.1:3001 (cardano-node 11.0.1), at tip in epoch 318 |
| log | `~/.cardano-live1/lv1-ce9-v7-bootstrap.log` |

The slice doc §6 said a v7 bootstrap "anchors near the current tip (epoch ~310)" and so could never
validate the failing header. That was wrong: snapshot 6009 was still on disk and anchors BEFORE it.
CE-LV1-9 was therefore a real test of the fix, not a regression check.

## 2. THE HALT

```
epoch-boundary yield: 304 -> 305 (eta0 74f10bea…)
  [epoch_boundary] protocol_major=11 total_stake=32006186920793256
                   active_stake=1623266825127463 pool_pot=0 go_pools=439 cert_pools=559
epoch-accumulator: CROSSED boundary 304 -> 305 at slot 130118424 (mark from s_prev 130118358)
epoch-boundary yield: 305 -> 306 (eta0 59d6c17a…)
follow: tip slot=130733645 behind=5254463 slots from peer
relay run-loop sync step failed (… LeaderValueAboveThreshold {
    value:     [0, 3, 148, 36, 101, 52, 221, 219],
    threshold: [0, 3, 147, 218, 125, 24, 43, 26] } …); failing closed (no skip-past, no fallback).
```

`value` and `threshold` are byte-identical to the three v6 occurrences
(`b12-ce10-BLOCKED-leader-value-above-threshold.txt`). Fourth occurrence, third binary, a different
store. **`pool_pot=0` on the 304→305 line is the defect, visible in the log.**

## 3. WHAT LV-1 CHANGED HERE: NOTHING

Read from a COPY of the v7 store (`leadership_distr_census.rs`, plus a temporary print of the new
field, reverted):

```
epoch 306  STORED total_active_stake = 1,626,066,239,242,875
           sum of the 505 pool entries = 1,626,066,239,242,875      (equal)
```

The credential-side fold and the pool-entry sum are the same number at this boundary. LV-1 changed
the kind of quantity carried, not its value. CE-LV1-2 passed in-tree because the test SUPPLIED a
total that excluded the retired pool; the live derivation never produces that total, and (§4) should
not.

## 4. THE REFERENCE — reopening a closed oracle window

The live peer cannot answer for epoch 306. A second cardano-node 11.0.1 can:

1. copy the Mithril snapshot `db` (18 GB);
2. run a second container on the copy, `--network host`, topology `localRoots` = the local preprod
   peer; poll `query tip`; `docker stop` at the wanted slot;
3. restart it `--network none` (a placeholder trustable `localRoot` is required or it refuses to
   start) and query offline.

Two such nodes were used: `ade-oracle-end304` (stopped in epoch 305, later advanced into 306) and
`ade-oracle-rupd304` (stopped inside epoch 304). Roughly ten minutes each.

### 4.1 The retired pool belongs in the snapshot

| where | `pool-state 8ed5ab11…eea88` |
|---|---|
| slot 129,816,426 (mid-304, the seed) | registered, `retiring: 305` |
| slot 130,133,233 (epoch 305) | `{}` |

It is reaped at the 304→305 boundary AFTER the SNAP, so it is legitimately in the end-of-304 mark.
Cardano's mark carries it at 63,136,879,314,792. **The denominator theory is refuted.**

### 4.2 End-of-304 mark (= epoch-306 leadership), cardano vs Ade

| | cardano `stakeMark` in epoch 305 | Ade sealed epoch 306 | Ade short by |
|---|---|---|---|
| total active stake | 1,626,474,372,102,448 | 1,626,066,239,242,875 | 408,132,859,573 (0.0251%) |
| issuer `0ed90e52…8b0e9` | 1,731,950,202,087 | 1,730,595,594,678 | 1,354,607,409 (0.0782%) |
| retired `8ed5ab11…eea88` | 63,136,879,314,792 | 63,075,223,742,053 | 61,655,572,739 (0.0977%) |
| issuer σ | 0.10648494% | 0.10642836% | 0.0532% |

The header needed σ +0.0315%. The measured gap is +0.0532%. **This accounts for the halt.**

Membership is identical: 448 pools with stake on both sides (Ade's other 57 entries are zero-stake).
373 are byte-equal; 75 are lower in Ade; **none is higher**. The 75 shortfalls sum to exactly
408,132,859,573.

### 4.3 The shortfall IS the 304→305 reward update — to the lovelace

`query ledger-state --output-text` (raw CBOR) at slot 130,062,360, slot-in-epoch 375,960, `nesRu`:

```
Complete   deltaT 7,029,700,573,599   deltaR 7,422,111,617,574   deltaF 23,699,540,134
           rs: 11,056 credentials, sum 416,110,584,109
```

Aggregating `rs` by each credential's delegation at that slot:

```
to credentials delegated to a pool     408,139,156,235   across 75 pools
to registered, undelegated credentials   7,971,416,027   (4 credentials)
to a credential unregistered at dump            11,847   (1 credential)
```

| | |
|---|---|
| pools whose Ade shortfall equals cardano's `rs` for that pool EXACTLY | **72 of 75** |
| issuer: shortfall / `rs` | 1,354,607,409 / 1,354,607,409 |
| the other 3 | each off by ONE credential's whole reward |

The three residuals: credential `8ca0bed6…0dd7b0` (reward 3,046,152,871) is delegated to pool
`1e3105f2…` at the dump and accounts for −3,046,152,871 there and +3,046,152,871 at `e9b3a2c7…`;
credential `94857be3…4dfd5b` (reward 6,296,662) accounts for the retired pool's 6,296,662. Both are
consistent with a delegation change in the 56,040 slots between the dump and the boundary. That
reading is INFERRED — the certificates were not looked up. With them, 408,139,156,235 − 6,296,662 =
408,132,859,573, the total gap exactly.

(`--output-json` prints `possibleRewardUpdate: null` in every state, including Complete. Use the CBOR.)

## 5. ROOT CAUSE

At the seed, cardano's `nesRu` is **`SNothing`** — measured: the reference ledger state at slot
129,816,426 has no pending reward update, with `blocksBefore` = 79 pools / 19,463 blocks and
`feeSS` = 23,699,540,134. Cardano starts the reward calculation only after
`randomnessStabilisationWindow` = 4k/f = 172,800 slots into the epoch, and forces completion at
345,600 (observed: still `Pulsing` at slot-in-epoch 337,873, `Complete` at 375,960). The seed is at
127,027. `SNothing` here means **"not computed yet"**.

Ade reads it as **"computed, and empty"**:

- `ledgerdb_state.rs` `read_reward_update_deltas`: `sm == 0` returns an empty delta map and zero
  pots. (A mid-pulse `Pulsing` fails closed; `SNothing` does not.)
- `native_firstrun.rs` step 9b persists that as a bound `BootstrapRewardUpdate` — "An EMPTY delta is
  still persisted".
- `epoch_accumulator.rs` `cross_epoch_boundary_transition`: a pending record targeting this boundary
  makes `is_seed_boundary` true, which forces the native reward to zero (empty block production, zero
  fees → `pool_pot=0`) on the stated ground that the bootstrap record "supplies cardano's EXACT
  reward". It supplies nothing.

So at 304→305 Ade pays no rewards, moves no pots, and builds the mark without them. Not credited:
416,110,584,109 to reward accounts; 7,029,700,573,599 not added to treasury; 7,422,111,617,574 not
drawn from reserves; the fee pot not reduced by 23,699,540,134.

The inputs the native calculation needs are in the snapshot: `nesBprev` is already imported
(`prev_block_production`); the go snapshot is already held; `ssFee` is decoded past and discarded
(`skip_item(d, o)?; // ssFee`), and `seed_from_bootstrap_ledger` sets `prev_epoch_fees = Coin(0)`.

**Which bootstraps are affected.** Any seed in the first 172,800 slots of an epoch (40% of a preprod
or mainnet epoch): silent under-count. Seeds between 172,800 and completion: refused (fail-closed,
correct). Seeds after completion: the path the one-shot bootstrap RUPD was designed and proven on.
(Where earlier live proofs' seeds fell in their epochs was not re-checked here.)

### 5.1 It does not heal

The rewards are never credited later, and the pots stay wrong. Cardano's end-of-305 mark (leadership
307), from the same reference node advanced to slot 130,563,009:

| | cardano | Ade sealed 307 (v6 store, 2026-08-29 census) | Ade short by |
|---|---|---|---|
| total active stake | 1,563,914,995,371,294 | 1,563,586,879,499,918 | 328,115,871,376 (0.0210%) |
| issuer | 1,714,597,466,534 | 1,713,246,643,021 | 1,350,823,513 (0.0788%) |
| issuer σ | 0.10963495% | 0.10957157% | 0.0578% |

The issuer's shortfall is still within 0.3% of the reward it was never paid. Every leadership epoch
after such a bootstrap carries the error, so a header whose VRF value lands in the gap is rejected in
any of them — epoch 306 is simply where the first one fell.

## 6. WITHDRAWN

1. **"The retired pool inflated the epoch-306 denominator by 3.879%, deflating every σ by 2.87%"**
   (DC-EPOCH-40 evidence; `leadervalue-census-sigma-denominator.txt`; slice doc §1.1). The pool is in
   cardano's snapshot too. Ade's denominator was 0.0251% LOW, not 3.879% high.
2. **"Ade's epoch-307 σ matches cardano's 0.10958% to five significant figures — 307 is the
   known-correct derivation"** (same sources). The cardano figure quoted was its `stakeGo` at the end
   of **307**, which is leadership for epoch 309. Against the right snapshot (§5.1) Ade's 307 is
   0.0578% low. 307 was never a control.
3. **"A fresh v7 bootstrap cannot reach the failing header; the proof is CE-LV1-2"** (slice doc §6;
   DC-EPOCH-40 `evidence_notes`). It can and did; CE-LV1-2 proves a property of a supplied operand.

What stands: DC-EPOCH-40's STATEMENT. The denominator is a credential-side snapshot fact that
membership must not move — cardano's source says so and nothing here contradicts it. LV-1 made that
structural. It was not the cause of this halt, and its evidence must not say it was.

## 7. NOT CLAIMED

- No fix exists. `DC-EPOCH-40` stays `partial`; CE-LV1-9 is NOT met; CE-B12-10 and CE-L2c-7/8/9 stay
  blocked.
- Ade's per-credential reward accounts were not diffed against cardano's; the per-POOL identity
  (72/75 exact, 3 explained by one credential each) is the evidence.
- That a native 304→305 computation from `nesBprev` + `ssFee` + go reproduces cardano's `rs` is the
  expected design and is unproven until a slice does it against the capture in §8.
- The v7 accumulator did not cross 305→306 (observe-only `UnresolvedCollateralInput` at 130,350,133,
  as in v6 leg 1); the 307 comparison uses the v6 store's sealed object.

## 8. ARTIFACTS (off-repo; reference captures a fix can be tested against)

```
~/.cardano-live1/ade-preprod-v7/                          the reproducer — parked before the header
~/.cardano-live1/oracle-end304-node/
    stake-snapshot-epoch305-all.json                      end-of-304 mark, all pools
    stake-snapshot-epoch306-all.json                      end-of-305 mark, all pools
    ade-v7-sealed-epoch306-pools.txt                      Ade's sealed 306, all pools
~/.cardano-live1/oracle-rupd304-node/
    ipc/ledger-state-seed304.json                         slot 129,816,426 — nesRu SNothing
    ipc/ledger-state-late304.{txt,json}                   slot 130,062,360 — nesRu Complete (txt = CBOR)
    rupd-304to305-rs-by-credential.json                   sha256 4b5679a2ed181618…
    ru.py  cmp.py                                         the decode and the per-pool comparison
```

Containers `ade-oracle-end304` and `ade-oracle-rupd304` are stopped, not removed.
