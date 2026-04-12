# Slice S-30 Entry Obligation Discharge

> **Status:** Discharged. S-30 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 3
> cluster plan §"Slice-Entry Proof Obligations").
>
> **Version scope:** cardano-node 10.6.2; plutus 1.57; aiken v1.1.21
> (commit `42babe5d5fcdd403ed58ed924fdc2aed331ede4d`).

This document discharges the three slice-entry proof obligations
gating S-30 (cost models + budget accounting + conformance gate,
Cluster P-B).

---

## O-30.1 — Cost Model CBOR Format

**Obligation:** Cost-model CBOR format in pparams: V1 flat integer
array, V2 keyed map, V3 versioned map? Decode from a 10.6.2
snapshot; compare to cardano-cli `query protocol-parameters`.

### Answer

**Outer shape is a CBOR map keyed by Plutus language index:**

```cddl
cost_models = { * uint => [ * int ] }
```

- `uint` language index: `0 = PlutusV1`, `1 = PlutusV2`, `2 = PlutusV3`
- value: a CBOR definite-length array of integers (signed, int64 range)

**Inner array is positional — parameter names are implicit.** Order
matches `Plutus.V{1,2,3}.ParamName` in the plutus package. Entry
counts:

| Language | Params | Notes |
|----------|--------|-------|
| V1 | 166 | Shelley params set, stable |
| V2 | 175 | Adds `serialiseData`, `verifyEd25519Signature`, ECDSA / Schnorr secp256k1 |
| V3 | 233 | Adds BLS12-381, keccak_256, blake2b_224, bitwise ops, ratify ops |

### Historical quirk — Alonzo throwaway format

Alonzo era used a **keyed map from parameter-name bytes to int**:

```cddl
cost_model = { * bytes => integer }   -- ALONZO ONLY, abandoned
```

This format was dropped in Babbage. Ade targets Babbage+ in the
current release scope (CE-79 release tier 3 is "full block
validation at cardano-node 10.6.2," which is Conway). The Alonzo
name-keyed form does not need to be implemented for current scope,
but must be documented in code comments as "pre-Babbage" for any
future backfill.

### Conway lenience (PV ≥ 9)

`Cardano.Ledger.Plutus.CostModels.decodeCostModels` branches on the
`DecoderVersion`:

- pre-PV9: **failing decoder** — unknown language → error; wrong
  array length → error.
- PV9+ (Conway): **lenient decoder** — unknown language indices go
  into a sidecar `_costModelsUnknown :: Map Word8 [Int64]`,
  array-length deviations are tolerated (treat extra entries as
  forward-compat cost-model additions).

Ade's pparams parser must carry both the typed cost models (V1/V2/V3)
and the sidecar map for PV9+ forward compatibility.

### Aiken does not accept `Vec<i64>` directly

`aiken_uplc::machine::cost_model::CostModel` is a typed struct with
named fields (`bls12_381_g1_add`, `add_integer`, etc.), one per
builtin, grouped under `MachineCosts` and `BuiltinCosts`. Aiken
provides hardcoded `CostModel::v1() / v2() / v3()` factories but
**no `TryFrom<&[i64]>` positional adapter**.

Ade must supply its own adapter:

```
Vec<i64> ──(positional by V{1,2,3}::ParamName order)──▶ aiken::CostModel
```

The name-order tables live in plutus's `Plutus.V1.ParamName`,
`Plutus.V2.ParamName`, `Plutus.V3.ParamName`. Ade either hard-codes
those orderings (with a CI regression test against a fresh cardano-cli
dump) or generates them at build-time from an upstream manifest.

### Verification source

- `cardano-cli query protocol-parameters` JSON (`costModels` field,
  object with string keys `"PlutusV1" | "PlutusV2" | "PlutusV3"`,
  each value an array of ints). JSON array order matches CBOR array
  order.
- A 10.6.2 snapshot pulled from the AWS Cardano node via the
  existing snapshot-loader infrastructure gives a live cross-check.

### Citations

| Source | Reference |
|--------|-----------|
| Outer CDDL | `eras/conway/impl/cddl-files/conway.cddl` (cost_models) |
| Alonzo legacy CDDL | `eras/alonzo/impl/cddl-files/alonzo.cddl` (`cost_model = { * bytes => int }`) |
| Haskell encoder | `Cardano.Ledger.Plutus.CostModels` (`encCBOR . flattenCostModels`) |
| PV9 lenience | `decodeCostModels` branch `ifDecoderVersionAtLeast (natVersion @9)` |
| V3 params | `Plutus.V3.ParamName` (233 entries, plutus 1.57) |
| Aiken cost model | `crates/uplc/src/machine/cost_model.rs` at v1.1.21 |

---

## O-30.2 — Aiken Budget Parity Against IOG Conformance

**Obligation:** Does aiken's cost accounting produce byte-identical
budget consumption to the Haskell reference on every IOG
conformance test?

### Answer

**Not proven by aiken's own CI.** Aiken's green CI at v1.1.21 is
narrower than "full conformance budget parity":

- **V1 is not registered.** Only `plutus_conformance_tests_v2` and
  `plutus_conformance_tests_v3` run in aiken's `crates/uplc/tests/conformance.rs`.
- **V2 budget checks are skipped.** The runner matches
  `Language::PlutusV1 | Language::PlutusV2 => {}` — budget file
  ignored, only the output value is asserted.
- **V3 budget checks silently skip missing files.** The runner uses
  `if let Ok(budget) = file_to_budget(&expected_budget_file) {
  assert_eq!(budget, cost, …); }` — a malformed or absent budget
  file is a no-op, not a failure.
- **Vendored snapshot lag.** Aiken v1.1.21 was cut 2025-12-12; its
  vendored conformance corpus snapshot is ~4 months behind Ade's
  corpus (IOG commit `643ddd13…`, 2026-04-10).
- **PV11 builtins disabled.** `ExpModInteger`, `CaseList`,
  `CaseData` are commented out in aiken v1.1.21's
  `builtins.rs` / `runtime.rs` / `cost_model.rs`. Any conformance
  test that exercises those builtins fails at decode time — this
  is correct for cardano-node 10.6.2 (PV10, PV11 not activated),
  but would show up as a "failure" in a naive runner.

### Ade's budget parity gate (CE-85)

CE-85 cannot simply cite aiken's CI. Ade must run its own
conformance harness that:

1. Walks `corpus/plutus_conformance/test-cases/uplc/` (our 1,998
   test triples).
2. For each `.uplc` source:
   - Parses to a `Program<NamedDeBruijn>` via aiken's parser.
   - Evaluates via aiken's CEK machine with the PV-appropriate
     cost model.
   - **Unconditionally asserts**
     `actual_budget == parsed(.uplc.budget.expected)`.
   - **Unconditionally asserts**
     `actual_output == parsed(.uplc.expected)`.
3. Skips tests that reference PV11-only builtins
   (`ExpModInteger`, `CaseList`, `CaseData`) with a skip count, not
   a failure — these are expected to fail until PV11 activates.
4. Reports pass / fail / skip counts per language.

**CE-85 closure criteria**:
- 100% pass on V2 budget + output.
- 100% pass on V3 budget + output (excluding PV11-only cases).
- Any failure is investigated, reported upstream to aiken if it's
  an aiken defect, or to IOG if it's a conformance-suite defect.

### Known risks

- Aiken's integer budget arithmetic uses `i64` (`ExBudget { mem: i64, cpu: i64 }`),
  matching Haskell's `CostingInteger = Int64`. No rational
  intermediate values. Matches byte-for-byte provided the cost
  model coefficients match.
- If aiken's vendored V3 conformance snapshot has fixed tests
  in a newer revision that Ade's snapshot doesn't match, the
  harness surfaces it. Response: pin an IOG snapshot that both
  aiken and Ade can verify against, or upgrade our corpus.

### Budget arithmetic shape

From `aiken/crates/uplc/src/machine/cost_model.rs` at v1.1.21:

```rust
pub struct ExBudget { pub mem: i64, pub cpu: i64 }
```

Summed per-step via `ExBudget { mem: a.mem + b.mem, cpu: a.cpu + b.cpu }`.
Integer addition only — matches Haskell's `CostingInteger` Int64
semantics and avoids any floating-point introduction.

### Citations

| Source | Reference |
|--------|-----------|
| Aiken conformance runner | `aiken/crates/uplc/tests/conformance.rs` at v1.1.21 |
| V1/V2 budget skip | runner `match language { Language::PlutusV1 \| Language::PlutusV2 => {} }` |
| V3 conditional budget | runner `if let Ok(budget) = file_to_budget(…)` |
| Budget type | `crates/uplc/src/machine/cost_model.rs` `ExBudget` |
| PV11 disabled builtins | `crates/uplc/src/builtins.rs` (commented-out `ExpModInteger`, `CaseList`, `CaseData`) |
| CHANGELOG budget fixes | v1.1.2x CHANGELOG: `integerToByteString` / `byteStringToInteger`, `constr-3.uplc` |
| IOG snapshot in aiken | `crates/uplc/test_data/conformance/{v2,v3}` |
| IOG snapshot in Ade | `corpus/plutus_conformance/MANIFEST.toml` (commit `643ddd13…`, 2026-04-10) |

---

## O-30.3 — Tx-Level Budget Cap Semantics

**Obligation:** Is the tx-level budget cap enforced per-script or
aggregated across all scripts in a tx?

### Answer

**Aggregated sum across all redeemers — enforced in phase-1 UTXO,
before any script executes.**

### The exact check

`cardano-ledger/eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs:475-481`:

```haskell
validateExUnitsTooBigUTxO pp tx =
  failureUnless (pointWiseExUnits (<=) totalExUnits maxTxExUnits) …
  where maxTxExUnits = pp ^. ppMaxTxExUnitsL
        totalExUnits = totExUnits tx
```

And `totExUnits` at `Alonzo/Tx.hs:379-383`:

```haskell
totExUnits tx = foldMap snd $ tx ^. witsTxL . rdmrsTxWitsL . unRedeemersL
```

A monoidal fold (pointwise `mem + mem`, `cpu + cpu`) over every
redeemer's declared `ex_units`. Compared pointwise against
`ppMaxTxExUnits`. Any exceedance in either dimension fails the tx.

### Enforcement is phase-1

`validateExUnitsTooBigUTxO` is called from the Alonzo `UTXO` rule
(`eras/alonzo/impl/.../Rules/Utxo.hs:583`), which runs **before**
`UTXOS` (phase-2, script evaluation). A tx that declares too much
budget is rejected outright — no collateral is taken, no scripts
run.

Error constructor: `ExUnitsTooBigUTxO (Mismatch RelLTEQ ExUnits)`
(`Utxo.hs:171`, CBOR tag 15 at line 708).

### Per-script semantics

Each redeemer's `ex_units` value plays two roles:

1. **Tx-level contribution** (phase-1, ledger): summed into
   `totalExUnits` and checked against `ppMaxTxExUnits`.
2. **Per-script evaluator cap** (phase-2, Plutus CEK): passed as
   `exBudget` to `PVn.evaluateScriptRestricting pv vm ec exBudget rs`
   (`cardano-ledger-core/src/Cardano/Ledger/Plutus/Language.hs:488,
   508, 528, 548`). The CEK machine aborts the script if its
   running consumption exceeds its declared redeemer budget.

So the redeemer declares its own budget, and that declaration is
both counted against the tx cap AND enforced as a per-script
ceiling. There is **no protocol-level per-script cap beyond the
declared redeemer budget.**

### Block-level cap

`eras/alonzo/impl/.../Rules/Bbody.hs:244-249`:

```haskell
{- ∑(tx ∈ txs)(totExunits tx) ≤ maxBlockExUnits pp -}
txTotal = foldMap totExUnits txs
```

Block cap = sum of each tx's declared redeemer ex_units (NOT sum of
`maxTxExUnits` caps). Error: `TooManyExUnits`.

### Era invariance

Babbage and Conway re-export the Alonzo function unchanged:
- `eras/babbage/impl/.../Rules/Utxo.hs:45,431` imports
  `Alonzo.validateExUnitsTooBigUTxO`.
- `eras/conway/impl/.../Rules/Utxo.hs:409` maps
  `Alonzo.ExUnitsTooBigUTxO` straight through.

Identical semantics Alonzo → Babbage → Conway → Dijkstra.

### Ade implications

Ade's phase-1 validator must:

1. Parse redeemers from the witness set (currently `decode_witness_info`
   only detects Plutus presence — S-30 must extend this to parse
   redeemer `ex_units` fields).
2. Sum pointwise across all redeemers (spend + mint + cert + reward
   + vote + propose).
3. Compare pointwise against `ppMaxTxExUnits`. Failure raises a new
   `ExUnitsTooBigUTxO` variant in `LedgerError`.
4. Pass each redeemer's declared `ex_units` to the evaluator as the
   per-script ceiling when phase-2 runs.

### Citations

| Source | Reference |
|--------|-----------|
| Tx-level fold | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Tx.hs` `totExUnits` L379–383 |
| Tx-level check | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` `validateExUnitsTooBigUTxO` L475–481 |
| Call site | same file L583 (within UTXO rule, before UTXOS) |
| Error constructor | `ExUnitsTooBigUTxO` L171, CBOR tag 15 L708 |
| Per-script cap | `Cardano.Ledger.Plutus.Language` `evaluateScriptRestricting` L488/508/528/548 |
| Block-level cap | `eras/alonzo/impl/.../Rules/Bbody.hs` L244–249 `TooManyExUnits` |
| Babbage reuse | `eras/babbage/impl/.../Rules/Utxo.hs` L45, L431 |
| Conway reuse | `eras/conway/impl/.../Rules/Utxo.hs` L409 |

---

## Summary of decisions locked for S-30

1. **Cost model parser** — Ade writes a positional adapter from the
   CBOR `cost_models` map (per-language `Vec<i64>`) to aiken's
   typed `CostModel` struct. Parameter-name orderings hard-coded
   from `Plutus.V{1,2,3}::ParamName`, with a CI cross-check
   against a fresh cardano-cli dump.

2. **PV9+ lenience** — Cost-model parser tolerates unknown language
   indices (store in sidecar `BTreeMap<u8, Vec<i64>>`) and tolerates
   array-length deviations past V3's 233-entry baseline
   (forward-compat). Pre-PV9 snapshots use strict parsing.

3. **Conformance test harness** — Ade runs its own harness at
   `corpus/plutus_conformance/test-cases/uplc/` with unconditional
   budget + output assertions per test. PV11-only builtins
   (`ExpModInteger`, `CaseList`, `CaseData`) are explicitly skipped
   with a skip count, not a failure. CE-85 closure requires 100%
   non-skip pass.

4. **Aiken's CI is not the CE-85 gate.** Aiken's runner is too
   loose (V1 not run, V2 budget skipped, V3 budget silently skipped
   on parse error). Ade supplies its own strict harness.

5. **Tx-level budget cap — aggregated sum** of all redeemers'
   declared `ex_units` pointwise against `ppMaxTxExUnits`. Enforced
   in phase-1 UTXO rule, before any script runs. Error:
   `ExUnitsTooBigUTxO` (new `LedgerError` variant). Invariant
   across Alonzo/Babbage/Conway/Dijkstra.

6. **Redeemer `ex_units` is dual-use**: counted against tx cap
   (phase-1) and enforced as per-script CEK ceiling (phase-2).

7. **Witness-set redeemer parsing** required for S-30 — extends
   `ade_ledger::witness` similar to the Plutus-script extraction
   from S-29's closing work.

---

## Authority Reminder

This discharge is a planning artifact. If any finding conflicts
with the normative specifications or CI enforcement, the normative
specifications are authoritative. The Haskell cardano-ledger source
paths and IOG's plutus-conformance vectors are the authoritative
references.
