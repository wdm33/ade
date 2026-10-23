# CENSUS LV-15 — what the block path decides today, against Cardano's Conway LEDGER

**Kind:** findings before a slice. No code. Purpose: identify the single sequential transition that should
replace today's mixture of authoritative checks, diagnostic counters and pre-block-state validation.

Reference: cardano-ledger cb57dc73. Ade: `main` at `64964b4b`.

## 1. What Cardano runs

Per block (BBODY): body size and body hash against the header, total execution units and total
reference-script size against their block limits, then LEDGERS — the transactions **in order**, each LEDGER
seeing the UTxO, cert state and gov state left by the previous one (`Conway/Rules/Ledger.hs:380`):

```
if isValid == True:
    validateTreasuryValue; validateRefScriptSize; validateWithdrawalsDelegated (PV >= 10)
    (PV gate) testIncompleteAndMissingWithdrawals; drain withdrawn accounts; DRep expiry updates
    CERTS   (deleg / pool / gov-cert state checks, deposits, refunds)
    GOV     (proposals, votes)
UTXOW (both shapes)  -- witnesses, scripts closure (LV-13), vkeys (LV-14), metadata, integrity hash
  UTXO  (both shapes) -- validity interval, sizes, inputs exist, fee, consumed == produced, collateral, networks
    UTXOS:  isValid == True  -> every Plutus script passes; normal effects (spend inputs, produce outputs,
                                  deposits, fees, donation)
            isValid == False -> some Plutus script fails (else ValidationTagMismatch); collateral only
                                  (consume collateral, produce collateral return, collateral to fees)
```

Two transition shapes, not one checklist. An `isValid = False` transaction is valid in its block when its
scripts *fail*, its phase-1 checks pass, and only its collateral moves.

## 2. What Ade runs per block on the live path

The live preprod node is a reduced follower (`track_utxo = false`). Per block:

1. **Admission** (`block_validity` → header authority → `apply_block_with_verdicts`): body decode and
   structure, the Conway witness closure (required tx-derived signers, every supplied signature — LV-14,
   native scripts at the reduced level — LV-13), the reduced epoch crossing. Rejects the block on failure.
2. **After the block is durable**, two advances, which **halt the node** on failure instead of rejecting a
   block:
   - the reduced UTxO checkpoint (`reduced_block_delta`): per output only `(coin, stake reference)`; address
     payment credential, multi-assets, datums and scripts are dropped. Spent inputs are removed whether or
     not present. `isValid` handled since BND-2 (collateral consumed, collateral return produced).
   - the epoch accumulator (`apply_selected_block`): fees, donations, withdrawals, certificates, proposals,
     collateral of invalid transactions (valued from the checkpoint). It holds the authoritative cert,
     account, pool, DRep, gov and treasury state.

The full-UTxO path (`track_utxo = true`: `track_utxo` + `run_phase_one_composers`) is not used live. It
validates every transaction against the **pre-block** UTxO, silently skips a transaction whose inputs do not
resolve, and **counts** phase-1 rejections and Plutus failures without rejecting the block. It never compares
a Plutus outcome with the block's `isValid` flag.

An on-disk full UTxO store exists (`ade_runtime::chaindb::utxo_anchor`, MEM-OPT-UTXO-DISK) with a
pre-resolve design for BLUE validation, but the live preprod node does not run on it.

## 3. The table

"Live" = the reduced follower's admission. "Full" = `track_utxo = true`. **Auth** = rejects the block;
**diag** = counted only; **halt** = a post-admission advance halts the node; **—** = not checked.

| Check (Cardano rule) | Live today | Full today | State it needs | Needs evolving intra-block state? | isValid shape |
|---|---|---|---|---|---|
| Body decode, structure, inputs non-empty (`InputSetEmptyUTxO`) | Auth | Auth | tx | no | both |
| Body hash vs header; header (VRF, KES, opcert, leader value) | Auth | Auth | header, epoch view | no | — |
| Block body size, block ex-units total, block ref-script size (BBODY) | — | — | tx sizes, pparams, **UTxO** (ref scripts) | no / ref scripts yes | — |
| Validity interval (`OutsideValidityIntervalUTxO`) | **—** | **—** | tx, slot | no | both |
| Max tx size (`MaxTxSizeUTxO`) | — | — | tx size, pparams | no | both |
| Tx ex-units cap (`ExUnitsTooBigUTxO`) | — | diag | tx, pparams | no | both |
| Network ids: body, outputs, withdrawals (`WrongNetwork*`) | — | diag (withdrawals —) | tx, pparams | no | both |
| Output too small / too big / boot attrs (`BabbageOutputTooSmall`, `OutputTooBig`, `OutputBootAddrAttrsTooBig`) | — (only coin = 0) | — | tx, pparams | no | both |
| Inputs, collateral, reference inputs exist (`BadInputsUTxO`) | — (checkpoint removes silently) | skipped (silently) | UTxO (keys) | **yes** | both |
| Ref inputs disjoint (`BabbageNonDisjointRefInputs`) | — | diag | tx | no | both |
| Fee ≥ min fee incl. ref-script size tiers (`FeeTooSmallUTxO`) | — | — | tx size, pparams, **UTxO** (ref script sizes) | yes | both |
| consumed == produced (`ValueNotConservedUTxO`) | — | diag, **coin only** | UTxO **values incl. multi-asset**, deposits/refunds (cert state) | yes | isValid = True |
| Collateral: present, % , no non-ADA without return, count, total field (`InsufficientCollateral`, `CollateralContainsNonADA`, `TooManyCollateralInputs`, `IncorrectTotalCollateralField`, `NoCollateralInputs`, `ScriptsNotPaidUTxO`) | — | diag (partial) | UTxO values (multi-asset for non-ADA), pparams | yes | both (phase-1) |
| Required vkey signers (`validateNeededWitnesses`) | Auth, tx-derived part | Auth (pre-block UTxO) | UTxO addresses for inputs/collateral | yes | both |
| Every supplied signature (`validateVerifiedWits`) | Auth (LV-14) | Auth | tx | no | both |
| Native scripts: failing provided script | Auth (LV-13) | Auth | tx | no | both |
| Native scripts: exact closure, reference scripts (`babbageMissingScripts`, `validateFailedBabbageScripts`) | — (Undetermined) | Auth, pre-block UTxO only | UTxO **addresses + script_refs** | **yes** | both |
| Metadata hash, script-integrity hash, redeemers, datums (`validateMetadata`, `checkScriptIntegrityHash`, `hasExactSetOfRedeemers`, `missingRequiredDatums`) | — | — | tx, pparams (cost models), UTxO (datums, scripts) | yes | both |
| Plutus outcome agrees with `isValid` (UTXOS, `ValidationTagMismatch`) | — | diag (pass/fail counted, **never compared with isValid**) | full UTxO outputs, pparams, cost models | yes | the split itself |
| Treasury value field (`ConwayTreasuryValueMismatch`) | — | — | treasury (accumulator) | no (per block) | isValid = True |
| Withdrawals exist, complete, delegated to a DRep (`ConwayWithdrawalsMissingAccounts`, `ConwayIncompleteWithdrawals`, `ConwayWdrlNotDelegatedToDRep`) | — (accumulator zeroes named accounts, no amount check) | — | accounts (accumulator) | **yes** (two txs in one block) | isValid = True |
| CERTS: registration states, deposits, refunds, delegatee exists, DRep/committee (`*DELEG`, `*POOL`, `ConwayDRep*`, `ConwayCommittee*`) | halt (accumulator) | Auth for some reasons, after composers | cert state (accumulator) | **yes** | isValid = True |
| GOV: proposals, votes, deposits, prev-action, guardrails (`ConwayGovPredFailure`) | halt / tripwire (accumulator) | partial | gov state, pparams, committee | **yes** | isValid = True |
| Effects: spend/produce, deposits, fees, donation, collateral | checkpoint + accumulator (after admission) | `track_utxo` (pre-block for validation, sequential for effects) | UTxO + accumulator | **yes** | both shapes |

## 4. What the table says

1. **Several UTxO-free rules are not enforced anywhere on the Conway path**: the validity interval, max tx
   size, output size limits, withdrawal network, collateral input count, block body size and block ex-units.
   They need only the transaction, the slot and the protocol parameters. These are the cheapest false-accepts
   left, and none needs any new state.
2. **The cert, account, gov and treasury state the remaining rules need already exists live**, in the epoch
   accumulator. But it is consulted only after admission, and a violation halts the node instead of
   rejecting the block. In the right order, validation reads that state at admission and the same transition
   produces the next state.
3. **Exact UTxO rules need a full UTxO, which the live node does not keep.** The reduced checkpoint is enough
   for input existence and coin-level conservation and collateral. Multi-asset conservation, non-ADA
   collateral, spending-script and reference-script needs (the rest of LV-13), datums, the integrity hash
   and Plutus need addresses, values, datums and script references. The full-UTxO store exists but is not on
   the live path.
4. **Validation and effects use different states today.** The full path validates every transaction
   against the pre-block UTxO but applies effects sequentially. So a transaction spending an output created
   earlier in the same block cannot be validated exactly, and two transactions in one block that both spend
   the same input or drain the same account are not caught.
5. **`isValid` is an effect selector in the accumulator and checkpoint (BND-2), but never a verdict.** No
   path checks that an `isValid = True` transaction's scripts pass or that an `isValid = False` one's fail.

## 5. The transition this points to

One BLUE transition per transaction, folded over the block in order, both shapes explicit:

```
ledger_tx(state, tx) -> Result<state', TxRejection>
  state = { utxo view, cert/account/pool/DRep state, gov state, treasury, pparams, slot }
  isValid == True : treasury, ref-script size, withdrawals, CERTS, GOV, UTXOW, UTXO, Plutus passes, normal effects
  isValid == False: UTXOW, UTXO (phase-1), Plutus fails, collateral effects only
ledger_block(state, block) = BBODY limits, then fold ledger_tx; any rejection rejects the block, no partial state
```

It replaces the composers' pre-block validation, the counters, and the post-admission halts with one
admission-time verdict whose output is the next state. The checkpoint and accumulator become the storage
of that state, not separate authorities.

## 6. The decision this census cannot make

Exactness for the UTxO-dependent rules on the live path needs the live node to hold full outputs (address,
value with multi-assets, datum, script reference). There are three ways forward:

- **(a) Run live on the full on-disk UTxO** (the existing `utxo_anchor` with its pre-resolve design), seeded
  from the Mithril snapshot's UTxO, and keep the reduced checkpoint as a derived projection or retire it.
  Exact, and the larger move: storage, recovery, the store-semantics version, and bootstrap all change.
- **(b) Widen the reduced record** with what the rules read (payment credential, multi-asset value, script
  reference, datum hash). It approaches (a) in content without its machinery.
- **(c) Keep the reduced live node partial** for UTxO-dependent rules, and make it exact only in full mode.

Whichever you choose, the UTxO-free rules (§4.1) and moving the accumulator's checks to admission (§4.2) do
not depend on this decision.
