# Slice S-27 Entry Obligation Discharge

> **Status:** Discharged. S-27 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 3
> cluster plan §"Slice-Entry Proof Obligations").
>
> **Version scope:** cardano-node 10.6.2; cardano-ledger source tree
> at the tag aligned with that release.

This document discharges the three slice-entry proof obligations
gating S-27 (collateral + input resolution, Cluster P-A). Each
obligation is answered with citations to the Haskell ledger source
and formal spec; no discharge is by footnote or assumption.

---

## O-27.1 — Collateral Percent Rule

**Obligation:** Exact collateral percent per era. Rounding. Citation.

### Answer

**Parameter value (`collateralPercentage`):** type `Word16`, **percent**
(not basis points). Mainnet value **150** unchanged from Alonzo
genesis through Conway. No governance action has altered it.

**Predicate (all three eras, integer-exact):**

```
100 * bal  >=  collPerc * txfee
```

- `bal` = `sum(collateralInputs.coin) − collateralReturn.coin` (Babbage+)
- `collPerc` = protocol parameter (150 on mainnet)
- `txfee` = tx body fee field
- Cross-multiplied — **no division, no rounding in the predicate**
- Comparator is `>=` (inclusive)
- `Val.scale` widens to signed `Integer`; overflow-safe

**Rounding (error payload only):** when the predicate fails, the
error carries the "required collateral" value computed as
`ceiling((collPerc * txfee) / 100)` via `rationalToCoinViaCeiling`.
This value is reporting-only and does not affect the validity
decision.

**Error constructor:** `InsufficientCollateral` carrying
`(DeltaCoin balance, Coin required_ceil)`.

- Alonzo: `AlonzoUtxoPredFailure.InsufficientCollateral` (CBOR tag 13)
- Babbage: inherits via `AlonzoInBabbageUtxoPredFailure`
- Conway: `ConwayUtxoPredFailure.InsufficientCollateral` (CBOR tag 12)

### Rust implications

- Use `i128` (or `num-bigint::BigInt` if already in scope) for the
  cross-multiplication — `u64 * u64` can overflow on adversarial
  fees and is unsafe.
- Do not round during the predicate check. Compute the error
  payload's "required" amount via ceiling only when reporting.
- Error type in `ade_ledger::error` must carry `(balance, required)`
  to match oracle's error encoding at the wire level.

### Citations

| Source | Reference |
|--------|-----------|
| Predicate | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` `validateInsufficientCollateral` L341–357 |
| Constructor | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` `AlonzoUtxoPredFailure.InsufficientCollateral` L162 |
| Default 150 | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/PParams.hs` `appCollateralPercentage` L507 |
| Reuse in Babbage | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxo.hs` `validateTotalCollateral` L248–263, L256 |
| Reuse in Conway | `eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs` `babbageUtxoValidation` L257; `InsufficientCollateral` L129 |
| Spec | CIP-28 (Alonzo protocol parameters); CIP-55 (Babbage) |

---

## O-27.2 — `collateralReturn` and `totalCollateral`

**Obligation:** Optional vs. mandatory? Relationship? Phase-2 failure
state delta? Citation.

### Answer

Both fields introduced in Babbage by CIP-0040. Both are
**optional** in the CDDL: tx body keys `? 16 : transaction_output`
(`collateralReturn`) and `? 17 : coin` (`totalCollateral`).
Haskell representation is `StrictMaybe` for each.

### `collateralReturn` (CDDL key 16)

**When required:** when any collateral input carries non-ADA assets.
`validateCollateralContainsNonADA` raises `CollateralContainsNonADA`
otherwise. Non-ADA assets in collateral MUST be returned (they cannot
be "consumed as fee" because fees are ADA only).

**When optional:** when all collateral is ADA AND the submitter is
willing to forfeit any excess as fee on phase-2 failure.

**When zero excess:** still allowed to be present; the balance
equation with `totalCollateral` (if declared) governs consistency.

### `totalCollateral` (CDDL key 17)

**Role:** purely declarative. Enables wallets/hardware to verify
collateral balance without needing the live UTxO values. Inspired by
CIP-0040's hardware-wallet use case.

**When present** → triggers `validateCollateralEqBalance`, which
enforces exact equality (see balance equation below). Violation:
`IncorrectTotalCollateralField (DeltaCoin balance, Coin declared)`.

**When absent** → no additional check beyond the percent rule of
O-27.1.

### Balance equation

```
bal = sum(collateralInputs.coin) − coalesce(collateralReturn.coin, 0)
```

When `totalCollateral = SJust tc` is present:
```
bal == tc
```
must hold exactly. Mismatch → `IncorrectTotalCollateralField`.

The percent check from O-27.1 uses this `bal` value directly.

### Phase-2 failure state delta

On `isValid = False` (phase-2 script failure),
`updateUTxOStateByTxValidity` applies:

1. **Remove** all `collateralInputs` from UTxO.
2. **Add** `collateralReturn` as a new output at
   `TxIn(txId, |outputs|)` via `mkCollateralTxIn`, if present.
3. **Credit** the fee pot by `collAdaBalance = sum(colIn.coin) −
   collateralReturn.coin` (≡ `totalCollateral` when declared).
4. **Do not apply** any other part of the tx (no regular outputs,
   no certs, no withdrawals, no gov actions).

Non-ADA assets survive only via `collateralReturn`; the rule
`validateCollateralContainsNonADA` makes it impossible to lose them
in a well-formed tx.

### Rust implications

- `LedgerState::apply_block` must distinguish phase-1 (no state
  change) from phase-2 (collateral-only state change). The current
  `ScriptVerdict::NotYetEvaluated` path does neither; S-32 will
  integrate this properly.
- S-27 scope is limited to validating the percent + balance +
  non-ADA rules at the UTXO layer (phase-1 structural checks);
  the actual collateral-consumption state transition is S-32 work.

### Citations

| Source | Reference |
|--------|-----------|
| CDDL | `eras/babbage/impl/cddl-files/babbage.cddl` tx body keys 16, 17 |
| `collAdaBalance` | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Collateral.hs` |
| Balance rule | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxo.hs` `validateCollateralEqBalance` L317–322 |
| Non-ADA check | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` `validateCollateralContainsNonADA` |
| Phase-2 delta | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxo.hs` `updateUTxOStateByTxValidity` |
| Output construction | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Collateral.hs` `collOuts`, `mkCollateralTxIn` |
| Design | CIP-0040 Collateral Output |

---

## O-27.3 — Missing-Input Error Classification

**Obligation:** At PV≤6 vs. PV>6, does missing-input produce the
same error classification (phase-1 failure)? Citation.

### Answer

**Uniformly phase-1** across Alonzo (PV 5), Babbage (PV 6/7), and
Conway (PV 8/9). Same error constructor `BadInputsUTxO` in all three.
Classification does NOT change at the PV≤6/PV>6 boundary.

### Per-era trace

| Era | Rule | Input set checked | Missing-input raises |
|-----|------|-------------------|----------------------|
| Alonzo | `AlonzoUTXO` | `txins ∪ collateral` | `BadInputsUTxO (NonEmptySet TxIn)` — phase-1 |
| Babbage | `BabbageUTXO` | `txins ∪ collateral ∪ refInputs` (widened) | `BadInputsUTxO` via `AlonzoInBabbageUtxoPredFailure` |
| Conway | `ConwayUTXO` | same widened set | `ConwayUtxoPredFailure.BadInputsUTxO` |

All three invoke the Shelley-base predicate
`validateBadInputsUTxO utxo inputs = failureOnNonEmptySet (inputs ➖ dom utxo) BadInputsUTxO`.

**Reference inputs (Babbage+):** missing reference inputs raise the
exact same `BadInputsUTxO` constructor — there is no separate
constructor and no separate rule for reference inputs. They are
merged into the single `allInputs` check inside the UTXO rule.

**Conway governance:** `GovAction`s reference `GovActionId`, not
`TxIn`. No new UTxO input kinds introduced; no Conway-specific input
resolution rule exists.

**Phase-1 vs. phase-2:** UTXO runs before UTXOS (script eval). A
missing input blocks the tx before any script is even executed.
`UtxosFailure` (phase-2 wrapper, CBOR tag 7 in Alonzo) is only
raised on `ValidationTagMismatch` or `CollectErrors` from
`UTXOS.hs` — script evaluation failures, not input-resolution
failures.

### CBOR tag note

`BadInputsUTxO` CBOR tag:
- Alonzo: 0
- Babbage: 0 (inherited via `AlonzoInBabbageUtxoPredFailure`)
- Conway: **1** — renumbered because Conway moved `UtxosFailure`
  to tag 0

This matters for wire-level error matching; a Rust implementation
targeting Conway must encode the tag as 1 for that era.

### Rust implications

- `ade_ledger::error::LedgerError` needs a `BadInputsUTxO` variant
  carrying `Vec<TxIn>` (or `BTreeSet<TxIn>`) for the missing set.
- The check applies to the union of (spend inputs + collateral
  inputs Alonzo+ + reference inputs Babbage+). A single check site
  covers all three.
- No protocol-version gating on the error classification — the
  PV-handling in S-27 can treat this rule as PV-invariant.

### Citations

| Source | Reference |
|--------|-----------|
| Alonzo rule | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` L130, L550 |
| Babbage rule | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxo.hs` L397 |
| Conway rule | `eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs` L94, L247–249 |
| Base predicate | `eras/shelley/impl/src/Cardano/Ledger/Shelley/Rules/Utxo.hs` `validateBadInputsUTxO` L462–474 |
| Phase-2 boundary | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxos.hs` L212, L246 (`ValidationTagMismatch`, `CollectErrors`) |

---

## Summary of decisions locked for S-27

1. **Collateral percent check**: implement integer cross-multiply
   `100 * bal >= collPerc * txfee` using `i128`. No division, no
   rounding in the predicate.

2. **Collateral percent param value**: hardcode mainnet default
   `150` until protocol params are plumbed; pparam parser in
   S-30 will make it dynamic.

3. **Balance value**: `bal = sum(colIn.coin) − collateralReturn.coin`.
   Must account for the optional return.

4. **Optional fields**: `collateralReturn` and `totalCollateral` are
   both optional. Check is conditional on presence.

5. **Missing-input error**: `BadInputsUTxO` carrying the missing
   `BTreeSet<TxIn>`. Uniform across Alonzo/Babbage/Conway. CBOR
   tag is 0 in Alonzo/Babbage, 1 in Conway.

6. **Check union**: the missing-input check applies to
   `spendInputs ∪ collateralInputs ∪ referenceInputs`. S-27 handles
   spend + collateral; S-28 widens to reference inputs.

7. **Phase classification**: every check in S-27 is phase-1. Phase-2
   state delta (collateral consumption) is deferred to S-32.

8. **Error variant alignment**: new variants in `LedgerError`:
   - `BadInputsUTxO(BTreeSet<TxIn>)`
   - `InsufficientCollateral { balance: i128, required: u64 }`
   - `CollateralContainsNonADA`
   - `IncorrectTotalCollateralField { balance: i128, declared: u64 }`
   - `NoCollateralInputs`
   These mirror the Haskell error constructors for later wire-level
   agreement.

---

## Authority Reminder

This discharge is a planning artifact. If any finding conflicts with
the normative specifications or CI enforcement, the normative
specifications are authoritative. The Haskell source paths cited
above are the authoritative implementation at the cardano-node
10.6.2 tag; their behavior supersedes any paraphrase here.
