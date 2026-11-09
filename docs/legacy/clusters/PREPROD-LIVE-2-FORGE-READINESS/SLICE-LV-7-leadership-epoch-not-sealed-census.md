# SLICE LV-7 — why epoch N+2's leadership is requested before it is sealed: census first, no guard changes

**Status:** FINDINGS COMPLETE 2026-10-20. No production code changed, and `LeadershipEpochNotSealed`
is untouched. **Cause:** at a catch-up boundary, the accumulator walk asks the reduced checkpoint for a
collateral value before the checkpoint has reached the block that spends it. The walk stops,
observe-only, and the boundary that seals leadership for N+2 never runs. The retry that would clear it
is pre-empted by the next sync step, which admits about an epoch of blocks. The fix is not designed
here; §6 lists what the owner decides.

**Entry state:** `73fda1d1`. Opened by the owner after LV-6, which left this as a separate open defect.
Evidence: the LV-6 live v11 run, preserved at `~/.cardano-live1/lv6-ce10-preserved/` (off-repo),
including the store as it stopped.

---

## 1. THE QUESTION

**Why is epoch 310 (and, on a fresh bootstrap, 307) requested before its leadership authority is
sealed?**

The candidates were:
- request too early;
- seal too late or missing;
- state lost or rebuilt on restart;
- an epoch-label mismatch;
- a multi-epoch advance that skips a sealing transition.

The guard was assumed correct until shown otherwise.

## 2. THE TWO STOPS ARE ONE DEFECT

Both the v10 run (LV-4) and the v11 run (LV-6) stopped twice, at the same blocks:

| run | stop | follow tip | refused | within-epoch apply failed at |
|---|---|---|---|---|
| v10, v11 fresh bootstrap | 1 | 130,975,160 (epoch 306) | `{ requested: 307 }` | 130,350,133 |
| v10, v11 warm restart | 2 | 132,271,745 (epoch 309) | `{ requested: 310 }` | 131,626,888 |

The forward fold at each boundary tick, from the logs:

| boundary | fold reached | durable tip at the tick | checkpoint cursor at walk time | crossed |
|---|---|---|---|---|
| 304→305 | 130,118,358 | 130,118,358 | 129,813,427 (seed) | yes |
| **305→306** | **130,350,114** | 130,550,399 | 130,118,424 | **no** |
| 306→307 | 130,982,398 | 130,982,398 | 130,550,399 | yes |
| 307→308 | 131,414,390 | 131,414,390 | 130,982,402 | yes |
| **308→309** | **131,626,842** | 131,846,389 | 131,414,402 | **no** |

Each short fold is followed by the same line:

```
epoch-accumulator: WITHIN-EPOCH apply failed at 130350133 (observe-only, NOT a boundary -- no rewind, no mark, no cross): CollateralBalance(UnresolvedCollateralInput { tx_index: 0, txin: TxIn { tx_hash: Hash32(0326ab20d9cf533634f9d6838ae327971ac1606ab69f4378c1fc8009091e225a), index: 1 } }) cursor=Some(130350114) tx_count=1 tx0[invalid=true certs=false wdl=2 votes=false proposals=false collateral=None]
epoch-accumulator: WITHIN-EPOCH apply failed at 131626888 (observe-only, NOT a boundary -- no rewind, no mark, no cross): CollateralBalance(UnresolvedCollateralInput { tx_index: 0, txin: TxIn { tx_hash: Hash32(90a5961dcfa196deb0cf98a4dc391b53858c84df1ecd10f736b9d600b18e4bae), index: 1 } }) cursor=Some(131626842) tx_count=8 tx0[invalid=true certs=false wdl=0 votes=false proposals=false collateral=Some(5000000)]
```

After that line there is exactly one more run-loop census (`blocks_admitted=19692`, then `19738`), and
then the terminal.

## 3. THE CENSUS AT THE FAILING HEADER

### 3.1 The store as it stopped at `{310}`

Read from copies, with `leadership_distr_census::dump_sealed_leadership_epochs`:

```
ACC applied_through_slot=131626842 epoch=308
LEADERSHIP epoch=304..309 SEALED (target == epoch for each)
LEADERSHIP epoch=310 LeadershipEpochNotSealed { requested: 310 }
CHECKPOINT last_advanced_slot=Some(131846412) seed_slot=Some(129813427)
TXIN 0326ab20…225a#1 live=None retained=Some(2060733)
TXIN 90a5961d…4bae#1 live=None retained=Some(17308819)
```

The accumulator copy taken at the `{307}` stop: applied through 130,350,114 in epoch 305; sealed
304–306; 307 not sealed.

| field | `{307}` stop | `{310}` stop |
|---|---|---|
| failing header | first header of 307, parent = durable tip in 306 | first header of 310, parent = durable tip at 132,271,745 in 309 |
| requested leadership epoch | 307 | 310 |
| accumulator epoch / cursor | 305 / 130,350,114 | 308 / 131,626,842 |
| sealed leadership epochs | 304, 305, 306 | 304 … 309 |
| last sealed by | crossing 304→305 (seals 306) | crossing 307→308 (seals 309) |
| would be sealed by | crossing 305→306 (never ran) | crossing 308→309 (never ran) |
| last epoch transition | CROSSED 304→305 at 130,118,424 | CROSSED 307→308 at 131,414,402 |
| reduced checkpoint at the walk | 130,118,424 | 131,414,402 |
| reduced checkpoint at the stop | not read (that checkpoint was not preserved) | 131,846,412, past B |
| selected-chain tip | 130,975,160 | 132,271,745 |

Leadership for N+2 is sealed when the accumulator crosses into N+1. The labels are exact: every
sealed object's `target_leadership_epoch` equals its key.

### 3.2 Why the caller requested the epoch

`epoch_wire::prepare_authority_for_candidate_slot` asks for epoch C on the first header whose slot is
in C and whose parent is the durable tip in C−1. It asks for the frozen leadership object only, with
no fallback. Both requests came at exactly that point, the first header of the new epoch. **The
request is on time.**

### 3.3 Why the seal was not produced

The two collateral inputs, located on cardano's own ImmutableDB with Ade's decoders
(`treasury_donation_census::find_txids`):

```
CREATED txid=0326ab20…225a slot=130178856 block_no=5016581 tx_index=0 phase2_valid=true
NAMED collateral 0326ab20…225a#1 slot=130350133 block_no=5024326 tx_index=0 phase2_valid=false
CREATED txid=90a5961d…4bae slot=131625032 block_no=5081795 tx_index=0 phase2_valid=true
NAMED collateral 90a5961d…4bae#1 slot=131626888 block_no=5081885 tx_index=0 phase2_valid=false
```

| stop | checkpoint cursor at walk time | input created | spent at B |
|---|---|---|---|
| `{307}` | 130,118,424 | 130,178,856 | 130,350,133 |
| `{310}` | 131,414,402 | 131,625,032 | 131,626,888 |

Each input was created after the checkpoint's cursor and spent at B. The `node_lifecycle` co-advance
pass runs the accumulator walk first and `advance_reduced_checkpoint_forward_to(…, tip)` after. So
while the walk applies B, the resolver (the reduced checkpoint, BND-2d) is still at the previous
pass's tip:

- its live table does not yet hold the input, because the input's creating block has not been applied;
- its retention does not hold it either, because B has not been applied.

This is the third case named in the BND-2d resolver's own comment: "cursor before `B` and `x` created
strictly between the cursor and `B` — neither answers, and the refusal self-clears once the authority
walks past `B`."

The refusal does clear itself in the store. At the stop the checkpoint is past B and the retention
answers both inputs. But nothing retries in time:

1. The walk's `ApplyFailedAt` arm is observe-only. It logs once and stops the walk, with the cursor
   unchanged.
2. The checkpoint is then moved forward to the tip.
3. The next sync step, during catch-up, admits about one epoch of blocks before the next co-advance
   pass.
4. It reaches the first header of the next epoch, and the promotion asks for leadership that the
   skipped crossing would have sealed. That is the terminal.

### 3.4 Why a warm restart gets past it

On restart, `recovery_admit` folds forward first, with the checkpoint already past B, so B resolves
through the retention. At stop 1 the restart crossed 305→306 (the LV-6 live result), 306→307 and
307→308, then met the same case at 131,626,888.

### 3.5 Why the other boundaries pass

Their walks reached the boundary, so every collateral input they met resolved. No chain-wide scan was
made of the other windows; the evidence is only that those walks completed.

## 4. VERDICT ON THE CANDIDATES

| candidate | verdict |
|---|---|
| request too early | **no** (§3.2) |
| seal too late or missing | **yes, the crossing that seals N+2 never runs** (§3.3) |
| state lost or rebuilt on restart | **no**: restart repairs it (§3.4); the stopped store's state is exact |
| epoch-label mismatch | **no**: every sealed object's target equals its key |
| multi-epoch advance skipping a seal | **contributing, not the cause**: no sealing transition is skipped by the walk itself; the one-epoch sync step only removes the retry window |

**The defect:** inside one co-advance pass, the accumulator walk resolves collateral from a checkpoint
positioned behind blocks the walk is applying. The resulting refusal is treated as transient and
observe-only. During catch-up, the leadership read is reached before any retry.

**Secondary:** the guard fails closed correctly, but about an epoch away from the cause, with a reason
that names the read and not the walk. The `node_lifecycle` reconcile arm already records "at the S4
authority flip this Err arm MUST halt (swallow only stalls)". The `ApplyFailedAt` arm swallows a
stall of an accumulator that is now the sole leadership authority.

## 5. TIERS (owner's framing)

- **True:** leadership authority is never consumed before it is deterministically sealed. Held: the
  guard refused both times, and nothing fell back.
- **Derived:** cardano's geometry makes epoch C's leadership required at the first header of C. Held
  (§3.2).
- **Release:** fresh bootstrap and warm restart must cross these boundaries without intervention.
  **Fails**, reproducibly, at 130,350,133 and 131,626,888.
- **Operational:** the restart is a workaround, not acceptable steady state.

## 6. FOR THE OWNER — what the fix slice must decide

The candidate invariant (unregistered until the fix slice): **the resolver the accumulator walk uses
answers every collateral input of every block the walk applies.** BND-2d's third case must then be
unreachable in production, rather than transient. Equivalently: when the walk applies block B, the
reduced checkpoint has applied B.

The decisions are:

1. **Where the guarantee lives.**
   - (a) Order the pass so the checkpoint reaches the walk's target before the walk resolves
     collateral there. The checkpoint already rewinds onto a boundary point before sealing (DC-EPOCH-32,
     seen in the restart log), so this interacts with that rewind and with the retention it keeps.
   - (b) Keep the order, and re-run the walk inside the same pass once the checkpoint has passed B.
     That makes "self-clears" a guarantee rather than a hope.
2. **Whether `ApplyFailedAt` stays observe-only** now that the accumulator is the sole leadership
   authority. It could halt at the cause, or be classified so that an unresolved input after the
   checkpoint has passed B is a terminal.
3. **The evidence bar.**
   - A unit differential built from the two real blocks, each with its creating block (fixtures from
     the ImmutableDB): created after the checkpoint cursor, spent by a phase-2-invalid transaction in
     the same pass.
   - A live fresh bootstrap that crosses 305→306 and 308→309 with no restart.

No guard is weakened by any of these: `LeadershipEpochNotSealed` and the exact-epoch read stay as
they are.

## 7. TOOLS USED

All are `#[ignore]`d evidence tests, read-only and run on copies. No new framework.

- `treasury_donation_census::find_txids`: cardano ImmutableDB chunks, Ade decoders.
- `leadership_distr_census::dump_sealed_leadership_epochs`: probes the existing exact-epoch read for
  each epoch and the checkpoint's existing `get` / `retained_collateral_value`.
- The two run logs and the store as stopped, preserved off-repo.
