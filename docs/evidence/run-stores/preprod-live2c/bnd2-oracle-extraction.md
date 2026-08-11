# BND-2 ORACLE EXTRACTION — the phase-2-invalid transition, from cardano-ledger

> **No production code.** This answers the eight reference questions mechanically and specifies the
> minimal input the accumulator needs. Implementation is a separate slice. `InvalidTxCarriesAuthorityEffect`,
> `InvalidTxCollateralNeedsUtxo` and the DC-NODE-15 / B12 gate all remain unchanged.

## Method — two independent sources, neither of them Ade

**(1) The block.** Slot 130,350,133 was extracted from the durable store and decoded with a
**neutral CBOR reader written for this task** — deliberately not Ade's parser, so a finding cannot be
an artefact of the code under investigation. Block hash `46905dcf3a51fb6ca7d53a9606bdd0f12fa5da4be9372585e392303a64ad2f2f`,
15,252 bytes, Conway (era tag 7), shape `[header, tx_bodies(1), witness_sets(1), aux{}, invalid_transactions(1)]`.

**(2) The rule.** `IntersectMBO/cardano-ledger` @ master, quoted verbatim below.

### What the block actually contains — tx0, decoded independently
```
invalid_transactions = [0]                      <- tx 0 IS phase-2 invalid
field  0 inputs            = 2   b9fede11…#1, b9fede11…#3
field  1 outputs           = 4
field  2 fee               = 904,638
field  5 withdrawals       = 2   f01e0a86…17 -> 0     (header 0xf0 = SCRIPT stake cred)
                                 f08a199a…cf -> 0     (header 0xf0 = SCRIPT stake cred)
field 11 script_data_hash  = b34dde5a…
field 13 collateral_inputs = 1   0326ab20d9cf533634f9d6838ae327971ac1606ab69f4378c1fc8009091e225a#1
field 14 required_signers  = 1
field 16 collateral_return = ABSENT
field 17 total_collateral  = ABSENT
field  4 certs             = ABSENT      field 19 votes / 20 proposals = ABSENT
```
Every field Ade's own guard reported is reproduced by the independent decoder, and two new facts
appear that change the shape of the problem: **both withdrawals are for amount 0**, and **there is no
collateral return**.

### A query that was run and DISCARDED
`cardano-cli query utxo` for the collateral input and both regular inputs returned `{}` for all three.
That is **inconclusive, not supporting evidence**: the inputs are ~380k slots old, and a regular input
this tx did *not* consume would very likely have been spent by some later transaction. Current-UTxO
absence cannot distinguish "consumed here" from "spent afterwards", so it is recorded and set aside
rather than read as agreement.

## THE RULE — `Cardano.Ledger.Babbage.Rules.Utxo`, verbatim

```haskell
Phase2Invalid ->
  let !(utxoKeep, utxoDel) = extractKeys (unUTxO utxo) (txBody ^. collateralInputsTxBodyL)
      UTxO collouts = collOuts txBody
      DeltaCoin collateralFees = collAdaBalance txBody utxoDel
   in pure $!
        utxoState
          { utxosUtxo = UTxO (Map.union utxoKeep collouts)
          , utxosFees = utxosFees utxoState <> Coin collateralFees
          , utxosInstantStake =
              deleteInstantStake (UTxO utxoDel) (addInstantStake (UTxO collouts) (utxoState ^. instantStakeL))
          }
```

`Cardano.Ledger.Babbage.Collateral`, verbatim:
```haskell
collAdaBalance txBody utxoCollateral = toDeltaCoin $
  case txBody ^. collateralReturnTxBodyL of
    SNothing -> colbal
    SJust txOut -> colbal <-> (txOut ^. coinTxOutL @era)
  where colbal = sumAllCoin utxoCollateral

collOuts txBody =
  case txBody ^. collateralReturnTxBodyL of
    SNothing    -> UTxO Map.empty
    SJust txOut -> UTxO (Map.singleton (mkCollateralTxIn txBody) txOut)
```

## THE EIGHT QUESTIONS, ANSWERED

**1. For `isValid = False`, which normal body effects are discarded?**
**All of them.** The entire state update is the four lines above: remove the collateral inputs, add
`collOuts`, add `collateralFees` to the fee pot, adjust instant stake. Regular inputs are NOT removed,
regular outputs are NOT created, and no certificate, withdrawal, mint, vote or proposal processing is
reachable on this path.

**2. Are the two withdrawals ignored, or do they affect intermediate accounting?**
**Ignored — there is no withdrawal processing on the Phase2Invalid path at all.** Independently, both
withdrawals here are for **0 lovelace** (the withdraw-zero idiom that forces a staking script to run),
so they could not move ADA even under valid semantics. Ade already collects withdrawal credentials
ONLY in the valid branch, so its structure matches the rule; it is the *guard in front of that branch*
that halts.

**3. Which collateral inputs are consumed?** Exactly `collateralInputsTxBodyL` — here the single
`0326ab20…#1`. The regular inputs are untouched.

**4. If `total_collateral` is absent, how is the consumed amount determined?**
By `collAdaBalance` = `sumAllCoin(utxoDel)` − collateral return. **Field 17 is not an input to the
computation at any point.** It is a *declared assertion*: when present the Babbage UTXO rule enforces
`failureUnless (bal == toDeltaCoin tc) (IncorrectTotalCollateralField bal tc)`; when absent
(`SNothing`) the check simply passes and the field constrains nothing.

> This is the precise shape of Ade's gap. Using field 17 as the fee contribution is **byte-equivalent
> to Cardano wherever the field is declared**, because the ledger validates equality. Ade is not wrong
> where it works — it is incomplete exactly where the field is optional, which is this transaction.

**5. Does that require reading the actual collateral UTxOs?** **Yes, unavoidably.** `utxoDel` is the
*resolved* UTxO entries for the collateral inputs (`extractKeys` over the UTxO map). The consumed
amount is a property of the entries, not of the transaction.

**6. Is `collateral_return` relevant in this exact case?** **No — field 16 is ABSENT.** Therefore
`collOuts = ∅` (no output created) and `collAdaBalance = sumAllCoin(utxoDel)`: the *entire* value of
`0326ab20…#1` is consumed. This is the simplest of the possible cases.

**7. What exact UTxO delta and fee/pot delta result?**

| | effect for THIS transaction |
|---|---|
| UTxO removed | `0326ab20…#1` only |
| UTxO added | nothing (`collOuts` empty) |
| regular inputs `b9fede11…#1/#3` | **untouched** |
| the 4 outputs | **never created** |
| fee pot | **+ value(`0326ab20…#1`)**, in full |
| declared fee 904,638 | **NOT collected** — `utxosFees <> Coin collateralFees`, never `txBody.fee` |
| instant stake | delete the removed collateral entry's stake; add nothing |

**8. Which part is reduced-UTxO authority and which is EpochAccumulator?**

The accumulator needs exactly **one scalar per phase-2-invalid transaction**: the fee-pot delta
`collAdaBalance`. Everything else it already does correctly by structure (it applies no body effects
for an invalid tx). Decomposing that scalar:

```
collAdaBalance = Σ value(collateral inputs)          <- UTxO AUTHORITY (needs resolution)
               − collateral_return.coin (if present) <- IN THE BLOCK (no resolution needed)
```

So the minimal deterministic fact crossing the boundary is **the summed ADA of the transaction's
collateral inputs** — one `Coin` per invalid tx. The collateral return, when present, is already in the
canonical block and requires nothing.

The target shape is therefore the one hypothesised, and the extraction supports it:

```
canonical block + accumulator state + reduced-UTxO-derived collateral resolution
    -> Cardano-equivalent invalid-tx transition
```

and NOT `EpochAccumulator gains another UTxO implementation`. The accumulator never needs the UTxO
map; it needs a resolver it can ask for `Σ value(collateral inputs)`.

## A SECOND SURFACE THIS RAISES — flagged, not investigated

`utxosInstantStake` is adjusted on this path: the consumed collateral entry's stake is deleted. Ade's
**reduced UTxO checkpoint** is the stake authority, and it walks the same blocks. Whether it applies a
phase-2-invalid transaction correctly — regular inputs/outputs skipped, collateral input removed —
was NOT examined here and does not follow from anything above. If it treats an invalid tx as ordinary
traffic, its stake view diverges from Cardano's independently of the accumulator. That is a distinct
question for the BND-2 slice and should be answered before, not after, the accumulator work.

## WHAT REMAINS OPEN FOR THE IMPLEMENTATION SLICE
1. Can the reduced UTxO checkpoint resolve an arbitrary collateral `TxIn`? Its retained shape is not
   the full UTxO, and if it cannot, the resolver contract has to be designed rather than assumed.
2. The behaviour of the reduced checkpoint on phase-2-invalid txs (above).
3. Whether a `Coin` handed across that boundary is replay-stable and fingerprint-safe.

Until (1) and (2) are settled, both guards stay exactly where they are. The rejection is not a false
positive — the transaction genuinely carries two (zero-valued) withdrawals and genuinely declares no
total collateral — and the halt is what stopped a wrong fee from entering the accumulator.

## SOURCES
- [cardano-ledger Babbage Rules/Utxo.hs](https://raw.githubusercontent.com/IntersectMBO/cardano-ledger/master/eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxo.hs)
- [cardano-ledger Babbage Collateral.hs](https://raw.githubusercontent.com/IntersectMBO/cardano-ledger/master/eras/babbage/impl/src/Cardano/Ledger/Babbage/Collateral.hs)
- [cardano-ledger Conway Rules/Utxos.hs](https://raw.githubusercontent.com/IntersectMBO/cardano-ledger/master/eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxos.hs)
