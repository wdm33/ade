# SLICE P1 — the preprod ledger state carries a CDDL `pool_params` the decoder cannot read

> **SEALED — DIAGNOSED, NOT FIXED.** Opened 2026-08-03 on the first attempt to bootstrap Ade from
> the preprod Mithril snapshot. Blocks preprod entry, and therefore LIVE-2 on preprod. It is
> consensus-adjacent (the bootstrap entry authority) and must not be patched against a forge-window
> clock.

## Symptom

```
ade_node --mode node: NATIVE Mithril FirstRun bootstrap failed
  (NonUtxoDecode("MalformedCbor(\"poolparams.vrf: byte len 28 != 32 at offset 1057362\")"));
  failing closed.   exit 41
```

**It failed closed correctly** — terminal before the WAL commit-point, no bootable partial state, no
fallback to a cardano-cli / JSON seed. The defect is that a valid preprod snapshot cannot be read at
all, not that a bad one was accepted.

## Root cause — two `pool_params` encodings, one of which is unhandled

The preprod ledger state contains **both** of these, and Ade's decoder handles only the first:

| | shape | first field | reward account | count (preprod) | count (preview) |
|---|---|---|---|---|---|
| **A** LedgerDB-internal | `array(10)` | `bytes(32)` vrf — no operator, the map key supplies it | `array(2)[net, hash28]` | **559** | **713** |
| **B** canonical CDDL `pool_params` | `array(9)` | `bytes(28)` **operator** | `bytes(29)` address | **1** | **0** |

Byte evidence, walked from the snapshot (not inferred):

```
A (ADE1, offset 404229)
  array(10)[ bytes(32) 08a5dbda…,  uint 1000000000,  uint 170000000,
             tag(30)[0,1],  array(2)[uint 0, bytes(28)],  tag(258)[…], … ]
             ^vrf            ^pledge 1000 tADA   ^cost 170 tADA  ^margin 0%

B (offset 1057361 — where the decode dies)
  array(9)[ bytes(28) f9647122…,  bytes(32) 247ac887…,  uint 75000000, uint 170000000,
            tag(30)[1,25],  bytes(29) e040b5e8…,  tag(258)[…], … ]
            ^OPERATOR          ^vrf                                ^29-byte reward addr
```

Shape B is exactly the Shelley→Conway CDDL:
`(operator, vrf_keyhash, pledge, cost, margin, reward_account, pool_owners, relays, pool_metadata)`
— i.e. the form carried by a **pool registration certificate**, not by the pool-params map.

So the decoder is not meeting a malformed pool. It walks into a state field holding a certificate
and reads that certificate's `pool_params` as the internal form. `read_pool_params`
(`ledgerdb_state.rs:188`) reads element 0 as a 32-byte vrf; element 0 there is the 28-byte operator.

**Preview has ZERO shape-B entries; preprod has exactly ONE.** That single entry, in a 32 MB state,
is the whole difference — which is why a year of preview bootstraps never surfaced it and preprod
fails on the first attempt.

## Why hermetic tests could never have caught this

`interop-finds-bugs` / `wire-byte-authority`, again. Round-trip tests generate the shape the encoder
writes; they cannot invent a *foreign* wire form that appears once per network, only when some pool
has a registration certificate pending in state. Only bootstrapping a second real venue exposed it.

## Instrumentation landed with this diagnosis

`read_fixed_bytes` now reports the **byte offset** on a width mismatch. Without it the error named a
field but not a position, which cannot distinguish "this field is the wrong width" from "the cursor
is misaligned upstream" — un-locatable in a 32 MB blob. The offset is what turned this from a guess
into a byte-level diagnosis, and it made two of my own hypotheses falsifiable:

- *"pool_params is operator-first and Ade reads it vrf-first"* — **wrong**; 559 preprod entries are
  vrf-first and decode fine.
- *"the two venues' snapshots use different node-release encodings"* — **wrong**; both use shape A
  for the pool map. The difference is one extra certificate, not a format revision.

## RESOLVED — the field is `psFutureStakePoolParams`, confirmed by the decoder's own code

Structural walk of the snapshot located the entry at
`root[1]/[0]/[6]/[1]/[1]/[1]/[3]/[1]/[0]/[1]/[2]` — i.e. Conway era → NewEpochState[3]=EpochState →
esLState → CertState → **PState[2]**. Both venues agree on the shape and differ in exactly one child:

| PState child | preprod | preview |
|---|---|---|
| `[0]` (skipped, 32-byte-key map) | `map(indef)` | `map(indef)` |
| `[1]` psStakePoolParams | `map(indef)` — 559 shape-A | `map(indef)` — 713 shape-A |
| **`[2]` psFutureStakePoolParams** | **`map(1)`** — one shape-B | **`map(0)`** — empty |
| `[3]` psRetiring | `map(2)` | `map(1)` |

And `ledgerdb_state.rs:942` applies the **same reader to both maps**:

```rust
expect_array(d, o, 4, "PState")?;
skip_item(d, o)?;                          // [0]
let pools        = read_pool_map(d, o)?;   // [1] psStakePoolParams       -> shape A, works
let future_pools = read_pool_map(d, o)?;   // [2] psFutureStakePoolParams -> shape B, FAILS
let retiring     = read_retiring(d, o)?;   // [3] psRetiring
```

**The two maps do not share a value encoding.** `psStakePoolParams` holds the LedgerDB-internal form
(`array(10)`, vrf-first, no operator — the map key supplies it). `psFutureStakePoolParams` holds the
params **as the registration certificate delivered them**: canonical CDDL `array(9)`, operator-first,
29-byte reward account. Preview's future map is perpetually empty, so the second `read_pool_map` call
never had to decode a single entry — the bug sat dormant behind an empty collection.

`psFutureStakePoolParams` is the pending pool re-registration set: params that take effect at the
next epoch boundary. Exactly one preprod pool currently has one; no preview pool does.

## Fix shape (specified, NOT yet implemented)

`read_pool_map` must stop being used for both. The future-pools map needs its own reader for the CDDL
form, with a **verifiable** safety property rather than a width sniff:

> the `operator` inside the value MUST equal the map key — otherwise reject.

That is checkable, self-validating, and cannot silently accept a certificate for the wrong pool.
Branching on `element[0]` width (28 vs 32) alone would make the symptom disappear while accepting
whatever happened to be there; **that is not the fix.**

Still to confirm before implementing: whether `future_pools` is consumed anywhere that would change
authoritative output (it feeds `future_pool_count` in the probe at line 925), and whether any other
Conway state field carries a CDDL sub-structure with the same latent hazard.

## Impact

- **Preprod entry is blocked.** The Mithril bundle fetched 2026-08-03 (cert `857f3800…`, certified
  point 129966825, genesis hash verified byte-identical to the venue's own `shelley-genesis.json`)
  is cert-valid but **not yet usable as an entry authority**.
- **LIVE-2 on preprod is blocked** behind it, despite every operator prerequisite being green:
  opcert `✓ within the correct KES period interval` (start 970 / current 1003 / end 1032, expires
  2026-09-15, counter 0), stake `stakeGo == stakeSet == stakeMark == 1,009,506,139,807` (σ ≈ 6.2e-4,
  ~13 leader slots/epoch), and 3 remaining epoch-304 leader slots at launch time.
- Preview is unaffected — it has no shape-B entry.

## Not claimed

No fix, no invariant registered, no CE. This seals the diagnosis and the byte evidence so it cannot
be re-discovered from scratch, and records that a forge window was deliberately skipped rather than
met with a rushed decoder change on the entry-authority path.
