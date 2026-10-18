# LV-5 — the 1 ADA is two epoch-305 treasury donations that Ade never moves into the treasury

preprod, 2026-10-18. Slice: `SLICE-LV-5-treasury-divergence-census.md`. Earlier state:
`lv4-ce12-v10-live-snapshot-fidelity.md` §4.

**Result.** Ade and cardano agree on the treasury and reserves at the seed and after 304→305. They
first differ at the **305→306 boundary**, and only in the treasury, by 1,000,000. Epoch 305 contains
exactly **two treasury donations of 500,000 each**, both in phase-2-valid transactions. Cardano
collects them in its donation pot and adds that pot to the treasury at the boundary. Ade keeps no
donation pot and adds nothing.

---

## 1. F1 — the first divergent transition

Reference: raw `state` files of the reference node's LedgerDB snapshots, read with `cbor2`
(`NewEpochState → EpochState → [treasury, reserves]`, `UTxOState` field 6 = `utxosDonation`). Ade:
`leadership_distr_census::dump_current_pots` on copies of the fresh v10 accumulator.

| point | Ade treasury | reference treasury | Ade reserves | reference reserves | reference donation pot |
|---|---|---|---|---|---|
| epoch 304 (Ade at the seed 129,813,427; reference at 129,975,591) | 1,945,191,497,759,078 | 1,945,191,497,759,078 | 12,993,813,079,206,744 | 12,993,813,079,206,744 | 0 |
| after 304→305 (Ade at 130,118,424; reference at 130,133,280) | 1,952,221,204,641,186 | 1,952,221,204,641,186 | 12,986,390,967,589,170 | 12,986,390,967,589,170 | 0 |
| late 305 (reference at 130,515,796) | — | 1,952,221,204,641,186 | — | 12,986,390,967,589,170 | **1,000,000** |
| after 305→306 (Ade at 130,552,801) | 1,959,103,173,719,172 | 1,959,103,174,719,172 | 12,979,123,112,128,607 | 12,979,123,112,128,607 | — |

Pots only move at a boundary. The reference rows taken a few blocks into an epoch therefore stand for
the state just after its boundary.

**P = the 305→306 boundary.** The treasury rise across it is 6,881,969,077,986 for Ade and
6,881,970,077,986 for cardano, a difference of exactly 1,000,000. The seed boundary 304→305 is exact
in both pots, live.

## 2. F2 — donations in epoch 305, from both sides

**Reference:** the donation pot is 0 at slot 130,133,280 and 1,000,000 at 130,515,796.

**Chain:** `treasury_donation_census` ran Ade's own envelope, Conway block and transaction-body
decoders over every block of epoch 305 (slots 130,118,400 to 130,550,399). It read the reference
node's ImmutableDB chunks 06023–06042 and its VolatileDB files. These are offline copies of the
canonical chain; Ade's own `chain.db` was held open by the running follower.

```
DONATION slot=130271804 block_no=5020772 tx_index=0 txid=24428858f6016a2ddde62811574fe80e27b0f87843f38f3408064a83d61a5cd5 amount=500000 phase2_valid=true
DONATION slot=130404902 block_no=5026831 tx_index=0 txid=853764ba663d1089396c56e7bb581d42f475072dcf8d044c8dfd85ac685e58a5 amount=500000 phase2_valid=true
SUMMARY blocks=18145 txs=25304 slots=130118424..130515796 donations=2 total=1000000     (ImmutableDB)
SUMMARY blocks=1547  txs=3164  slots=130515830..130550399 donations=0 total=0           (VolatileDB, rest of 305)
```

**The two sides agree.** Both donations fall between the two reference points (pot 0, then
1,000,000), and none falls after the second.

## 3. Classification (decision tree)

**YES, a donation is present** (F3):
- **Where:** two transactions, listed above, both valid, 500,000 each, total 1,000,000.
- **What they explain:** the treasury difference is exactly 1,000,000, the reserves difference is 0,
  and the divergence first appears at P = 305→306, the boundary where cardano moves the donation pot
  into the treasury.
- **Nothing is left over.**

**Lead confirmed.** Ade's epoch accumulator holds no donation pot. Transaction body key 22 is decoded
and counted in the balance check, but never added to any pot. At the boundary, Ade's state has
nothing to move.

## 4. F5 — rule check

The cause is outside DC-EPOCH-40, -41, -43 and -44:
- **40** is the leadership stake total; **43** is credential identity; **44** is frozen pool
  parameters. Donations are none of these.
- **41** governs the seed boundary 304→305, and F1 shows that crossing exact live in both pots.

The condition of the LV-4 decision, "outside the invariant", is verified.

Bearing on DC-EPOCH-41 (LV-4 §7): F1 gives live, whole-pot equality at the seed boundary itself.
That is stronger than CE-LV2-3's seed-state emulation, whose four named residuals came from crossing
the seed state in isolation.

## 5. Artifacts

```
crates/ade_runtime/tests/treasury_donation_census.rs           F2 chain side (#[ignore], evidence-only)
crates/ade_runtime/tests/leadership_distr_census.rs            dump_current_pots, F1 Ade side
<scratch>/lv5/pots.py                                           F1 reference side (cbor2)
<scratch>/lv5/cmp_rewards.py, wdrl.py                           section 6 (cbor2)
crates/ade_runtime/tests/leadership_distr_census.rs            dump_reward_balances, section 6 Ade side
~/.cardano-live1/oracle-end304-node/db/ledger/{130133280,130515796}/state, db/immutable, db/volatile
~/.cardano-live1/oracle-rupd304-node/db/ledger/129975591/state
```

## 6. DC-EPOCH-41 — the seed-boundary crossing, per credential, on the real transition

Owner condition (2026-10-18): promote DC-EPOCH-41 only if the live transition proves the leftovers of
the isolated CE-LV2-3 crossing, credential by credential.

- **Ade:** `leadership_distr_census::dump_reward_balances` on a copy of the fresh v10 accumulator,
  taken right after the live crossing (applied through slot 130,118,424, the first block of 305).
- **Reference:** the accounts map (`DState` accounts, field 0 = balance) of the LedgerDB snapshot at
  slot 130,133,280.

```
cardano 33257 ade 33257 equal 33256 different 1
DIFF key d0f4075a0ab29c0f71c8310f0b3151fbfee6fdc2de910e2f3471455a ade 694110 cardano 0
```

The one difference is a valid withdrawal of exactly 694,110 at slot 130,121,698, transaction index 9,
found on chain with `cbor2`. That slot falls between the two points: Ade's copy had not applied it yet,
and the reference had.

| isolated CE-LV2-3 leftover | live result |
|---|---|
| `4245236a…` (+1,000,000,000, outside the reward update) | equal, 1,377,424,518 |
| `7a507e54…` (+500,000,000, outside the reward update) | equal, 3,475,776,685 |
| `03b56d5e…`, `6103000c…` (1,000,000,000 each, credentials in no go pool) | equal, 1,000,000,000 each |
| `85ecddcb…`, `94857be3…` (not registered at the seed point) | absent from BOTH account maps |

Treasury and reserves after 304→305 are exact (§1). **The owner's condition is met. CE-LV2-3 is
superseded by this proof; its isolated result stays on record. DC-EPOCH-41 is `enforced`.**
