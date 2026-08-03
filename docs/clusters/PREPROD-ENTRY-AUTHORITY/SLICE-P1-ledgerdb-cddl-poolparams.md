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

## What is NOT yet known (the fix cannot be chosen until it is)

- **Which state field** holds the shape-B entry. It must be identified, not guessed — the fix
  differs entirely between "a pending pool registration in a cert queue", "a governance proposal",
  and "a stashed/future pool params map".
- Whether that field is currently **skipped, mis-skipped, or read** by the decoder.
- Whether other Conway state fields carry CDDL sub-structures with the same hazard.

Branching on `element[0]` width (28 vs 32) would make the symptom disappear without answering any of
these, and would silently accept a certificate as a registered pool. **That is not the fix.**

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
