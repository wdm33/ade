# Slice S-32 Entry Obligation Discharge

> **Status:** Discharged. S-32 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 3
> cluster plan §"Slice-Entry Proof Obligations").
>
> **Version scope:** cardano-node 10.6.2 / plutus 1.57.0.0 /
> aiken v1.1.21.

This document discharges the three slice-entry proof obligations
gating S-32 (verdict integration, Cluster P-D).

---

## O-32.1 — Phase-1 vs Phase-2 Failure Classification

**Obligation:** On what exact failure classes does collateral
get consumed (phase-2) vs. tx-rejected-outright (phase-1)?

### Answer

**The phase is structural, not by variant.** A tx flows through
`UTXOW → UTXO → UTXOS`. Any predicate failure raised BEFORE
script execution is phase-1 (tx dropped, block invalid if
included). Script-related failures during phase-2 evaluation
yield the collateral-consuming phase-2 path.

### Driver mechanics

`Cardano.Ledger.Alonzo.Rules.Utxos.utxosTransition`
(`eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs`)
branches on the transaction's `IsValid :: Bool` flag:

- `IsValid True` → `scriptsYesTransition`: runs
  `collectPlutusScriptsWithContext` + `evalPlutusScripts`.
  If scripts return `Fails`, raises
  `ValidationTagMismatch (IsValid True) FailedUnexpectedly`.
  Context-building errors raise `CollectErrors`.
  **Both are phase-2** — `utxosFailure` branch applies collateral-
  only delta.
- `IsValid False` → `scriptsNoTransition`: expects scripts to
  fail. If all scripts actually pass, raises
  `ValidationTagMismatch (IsValid False) PassedUnexpectedly`
  — **phase-1** (producer lied about `IsValid False`; tx dropped).

### Phase-1 enumeration (tx rejected, NO state delta)

Every `AlonzoUtxoPredFailure` except `UtxosFailure` wrapping a
phase-2 variant:

- `BadInputsUTxO`, `OutsideValidityIntervalUTxO`, `MaxTxSizeUTxO`
- `InputSetEmptyUTxO`, `FeeTooSmallUTxO`, `ValueNotConservedUTxO`
- `WrongNetwork`, `WrongNetworkWithdrawal`, `WrongNetworkInTxBody`
- `OutputTooSmallUTxO`, `OutputBootAddrAttrsTooBig`, `OutputTooBigUTxO`
- `TriesToForgeADA`, `OutsideForecast`
- `InsufficientCollateral`, `ScriptsNotPaidUTxO`, `ExUnitsTooBigUTxO`
- `CollateralContainsNonADA`, `TooManyCollateralInputs`, `NoCollateralInputs`
- `ValidationTagMismatch PassedUnexpectedly`

Every `AlonzoUtxowPredFailure` is phase-1:

- All Shelley witness failures wrapped by `ShelleyInAlonzoUtxowPredFailure`:
  `InvalidWitnessesUTXOW`, `MissingVKeyWitnessesUTXOW`,
  `MissingScriptWitnessesUTXOW`, `ScriptWitnessNotValidatingUTXOW`
  (native scripts only; Plutus native-script failure has no
  effect since native scripts are phase-1), metadata variants,
  `ExtraneousScriptWitnessesUTXOW`, `MIRInsufficientGenesisSigsUTXOW`.
- Alonzo additions: `MissingRedeemers`, `MissingRequiredDatums`,
  `NonOutputSupplimentaryDatums`, `PPViewHashesDontMatch`,
  `MissingRequiredSigners`, `UnspendableUTxONoDatumHash`,
  `ExtraRedeemers`.

`UpdateFailure` (PPUP, protocol parameter update) is phase-1.

**Babbage additions** (all phase-1):
- `IncorrectTotalCollateralField`
- `BabbageOutputTooSmallUTxO`
- `BabbageNonDisjointRefInputs` (promoted to enforcement at
  Conway, per O-28.1)
- `MalformedScriptWitnesses`, `MalformedReferenceScripts`

**Conway additions** (all phase-1):
- Governance-witness failures under `ConwayUtxowPredFailure`

### Phase-2 enumeration (tx stays in block, collateral consumed)

**Exactly two variants.** From `AlonzoUtxosPredFailure`
(unchanged through Babbage / Conway):

- `ValidationTagMismatch (IsValid True) FailedUnexpectedly [FailureDescription]`
- `CollectErrors [CollectError]`
  (sub-variants: `NoRedeemer`, `NoWitness`, `NoCostModel`, `BadTranslation`)

### State delta on phase-2 (Babbage+)

From `updateUTxOState` in the `utxosFailure` branch:

1. **Remove** all `collateralInputs` from UTxO.
2. **Add** `collateralReturn` output (if present) at
   `TxIn(txId, |outputs|)` — Babbage+ only.
3. **Credit** fees by `totalCollateral` (if declared) else
   `collAdaBalance = sum(colIn.coin) − collateralReturn.coin`.
4. **Discard** everything else: regular outputs, cert effects
   (`DELEG`/`POOL`/`GOV`), mint effects, withdrawals, treasury
   donation, PPUP, deposits.

### Ade routing implications

| Classification | Ade action |
|----------------|------------|
| Phase-1 failure | Return `LedgerError::*` from `apply_block`; state unchanged |
| Phase-2 failure | Return `Ok(new_state)` where `new_state` reflects collateral-only delta; surface script failure via `ScriptVerdict::Failed` in the block report |

### Citations

| Source | Reference |
|--------|-----------|
| `utxosTransition` | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs` |
| `scriptsYesTransition` / `scriptsNoTransition` | same file |
| `utxosFailure` / state delta | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` L582-601 |
| Babbage additions | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/{Utxo,Utxow}.hs` |
| Conway additions | `eras/conway/impl/src/Cardano/Ledger/Conway/Rules/{Utxo,Utxow,Utxos}.hs` |
| Spec | Alonzo formal spec §4.3 (two-phase validation, `scriptsValidate`) |

---

## O-32.2 — Multi-Script Tx-Level Budget Cap

**Obligation:** Is the tx-level budget cap per-script or aggregated?

### Answer

**Aggregated sum across all redeemers — already discharged in
O-30.3.** No new research needed. Summary:

- `validateExUnitsTooBigUTxO` computes
  `totExUnits = foldMap snd (tx.redeemers)` and checks
  `totExUnits <= ppMaxTxExUnits` pointwise.
- Enforced in **phase-1 UTXO rule**, before any script runs.
- Error: `ExUnitsTooBigUTxO` (phase-1 per O-32.1 above).
- Each redeemer's `ex_units` is ALSO the per-script CEK ceiling
  during phase-2 evaluation (dual role).

Ade's existing implementation: `ade_ledger::late_era_validation::check_tx_ex_units_within_cap`
(shipped in commit `8cb48b8`). S-32 does not need to add a new
check — just confirms this check runs as part of the phase-1
composer sequence per era.

### No new citations required

All citations in O-30.3 discharge
(`docs/active/S-30_obligation_discharge.md` §O-30.3).

---

## O-32.3 — Conway Governance Script Failure

**Obligation:** When a Voting / Proposing / Certifying script
fails, does the whole tx fail or only the vote/proposal/cert?

### Answer

**All-or-nothing.** A failing governance script consumes
collateral and invalidates the ENTIRE transaction. No
partial-success mechanism exists in Conway.

### Mechanical evidence

1. **Unified script evaluation.** All six ScriptPurpose variants
   (Spending, Minting, Certifying, Rewarding, Voting, Proposing)
   go through the same `ConwayPlutusPurpose` ADT (Conway
   `Scripts.hs` L202-208), collected into a single
   `collectPlutusScriptsWithContext` and evaluated by one
   `evalPlutusScripts` call. No per-purpose success tracking.

2. **Single `isValid` flag.** `conway.cddl` keeps `is_valid`
   as a single tx-wide Bool. CIP-1694 does not introduce a
   per-action validity flag. Redeemer tags 3 (Cert), 4 (Voting),
   5 (Proposing) are just new indices into the same redeemer
   array sharing the same flag.

3. **UTXOS rule dispatch.** Conway `Utxos.hs` L252-257
   (`utxosTransition`) branches on `tx.isValid`:
   - `IsValid True` → `expectScriptsToPass`: pattern-matches on
     the aggregate `evalPlutusScripts` result. Any `Fails _ fs`
     → `ValidationTagMismatch FailedUnexpectedly` — phase-2
     collateral-only.
   - `IsValid False` → `babbageEvalScriptsTxInvalid`.
   No per-script branching; the aggregate failure aggregates.

4. **LEDGER rule gates governance.** `Conway/Rules/Ledger.hs`
   L380-445 (`conwayLedgerTransitionTRC`):
   ```haskell
   if tx.isValid == IsValid True
     then do ...CERTS..., ...GOV... ; pure (utxoState', certStateAfterCERTS)
     else pure (utxoState, certState)
   ```
   When `isValid = False`, the entire governance branch —
   `trans @"CERTS"` and `trans @"GOV"` — is skipped.
   `certState` passes through unchanged; `proposalsGovStateL` is
   not updated.

### State delta table (governance script phase-2 failure)

| Effect | Applied? |
|--------|---------|
| Collateral consumed | ✅ |
| Fee added to `utxosFees` | ✅ |
| Votes applied | ❌ |
| Proposals registered | ❌ |
| Certs applied | ❌ |
| Regular outputs | ❌ |
| Withdrawals | ❌ |
| Treasury donation | ❌ (only in `updateUTxOState` True-branch) |
| Deposits | ❌ |

### Asymmetry

None. Voting / Proposing / Certifying failures are treated
identically to Spending / Minting / Rewarding — all six flow
through one `evalPlutusScripts`, one `Fails` aggregator, one
`ValidationTagMismatch`, one tx-wide `isValid`.

### Citations

| Source | Reference |
|--------|-----------|
| `ConwayPlutusPurpose` | `eras/conway/impl/src/Cardano/Ledger/Conway/Scripts.hs` L202-208 |
| `utxosTransition` | `eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxos.hs` L252-279 |
| `expectScriptsToPass` | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxos.hs` L178-197 |
| `conwayLedgerTransitionTRC` | `eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Ledger.hs` L380-445 |
| State delta | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` L582-601 |
| `is_valid` field | `conway.cddl` (same-named field as Alonzo) |
| Spec | CIP-1694 |

---

## Summary of decisions locked for S-32

1. **`ScriptVerdict` enum extended**:
   ```rust
   enum ScriptVerdict {
       NativeScriptPassed,
       NativeScriptFailed(NativeScriptFailure),
       PlutusPassed { ex_units_consumed: (i64, i64) },
       PlutusFailed { reason: PlutusFailure, ex_units_attempted: (i64, i64) },
       NotYetEvaluated,  // deprecated; phased out during S-32
   }
   ```

2. **Two-phase decision function**:
   ```rust
   pub fn classify_failure_phase(err: &LedgerError) -> ValidationPhase
   enum ValidationPhase { Phase1, Phase2 }
   ```
   Maps every LedgerError variant to its phase per the O-32.1
   enumeration. Phase-1 is the default; only two
   Plutus-specific error categories map to Phase-2.

3. **Collateral-consumption state delta**:
   ```rust
   pub fn apply_phase_2_failure(
       state: &LedgerState,
       tx: &Alonzo+Tx,
   ) -> LedgerState
   ```
   Pure function implementing the Babbage+ collateral-only
   state delta: remove collateral inputs, add collateralReturn
   output, credit fees.

4. **Per-era integration in `apply_block`**:
   - Run `validate_alonzo/babbage/conway_state_backed` first
     (phase-1 checks). Any failure → return LedgerError
     immediately, state unchanged.
   - If phase-1 passes AND tx has Plutus scripts: call
     `eval_tx_phase_two` (from ade_plutus).
   - If script evaluation agrees with `isValid = True`:
     verdict `Passed`, normal state transition.
   - If script evaluation disagrees: verdict `Failed`,
     `apply_phase_2_failure` state delta, tx stays in block.
   - No Plutus scripts: verdict `NativeScriptPassed` (or
     per-native-script), normal state transition.

5. **Real-mainnet-tx integration test**: requires tx-CBOR →
   pallas-compatible-UTxO-CBOR conversion, which means full
   Alonzo+ tx-output serialization in `ade_codec`. Builds as
   part of S-32 integration.

6. **ADE routing for the 2 phase-2 error classes**:
   - `PlutusFailure::ValidationTagMismatch` — scripts disagreed
     with `isValid`.
   - `PlutusFailure::CollectErrors` — couldn't build
     ScriptContext / missing witness / missing cost model /
     translation failure.

7. **Ade `apply_block` must expose per-tx verdict + state delta
   as a value, not just Ok/Err**:
   ```rust
   pub struct BlockApplyResult {
       pub new_state: LedgerState,
       pub tx_verdicts: Vec<TxVerdict>,
   }
   pub struct TxVerdict {
       pub tx_id: Hash32,
       pub outcome: TxOutcome, // Pass / Phase2Fail / reporting
       pub ex_units_consumed: Option<(i64, i64)>,
   }
   ```

---

## Authority Reminder

This discharge is a planning artifact. If any finding conflicts
with the cardano-ledger Haskell source or the Alonzo/Conway
formal specs, those sources are authoritative. The Haskell
implementation at cardano-node 10.6.2's tag is the reference
for phase classification.
