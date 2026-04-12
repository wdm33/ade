# Slice S-31 Entry Obligation Discharge

> **Status:** Discharged. S-31 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 3
> cluster plan §"Slice-Entry Proof Obligations").
>
> **Version scope:** cardano-node 10.6.2 / plutus 1.57.0.0 /
> aiken v1.1.21 / commit `42babe5d`.

This document discharges the four slice-entry proof obligations
gating S-31 (ScriptContext derivation, Cluster P-C).

---

## O-31.1 — V1 / V2 / V3 ScriptContext Structural Differences

**Obligation:** Document exact structural differences between
ScriptContext V1, V2, and V3.

### V1 ScriptContext (Alonzo)

```haskell
ScriptContext { scriptContextTxInfo :: TxInfo
              , scriptContextPurpose :: ScriptPurpose }

TxInfo                              -- 10 fields, Constr 0 arity 10
  { txInfoInputs      :: [TxInInfo]
  , txInfoOutputs     :: [TxOut]
  , txInfoFee         :: Value
  , txInfoMint        :: Value
  , txInfoDCert       :: [DCert]
  , txInfoWdrl        :: [(StakingCredential, Integer)]   -- assoc list
  , txInfoValidRange  :: POSIXTimeRange
  , txInfoSignatories :: [PubKeyHash]
  , txInfoData        :: [(DatumHash, Datum)]             -- assoc list
  , txInfoId          :: TxId }

TxInInfo { txInInfoOutRef :: TxOutRef, txInInfoResolved :: TxOut }
TxOut    { txOutAddress :: Address, txOutValue :: Value
         , txOutDatumHash :: Maybe DatumHash }            -- 3 fields

ScriptPurpose = Minting CurrencySymbol
              | Spending TxOutRef
              | Rewarding StakingCredential
              | Certifying DCert                          -- 4 variants, ctors 0..3
```

### V2 ScriptContext (Babbage)

Envelope structure unchanged (`ScriptContext { TxInfo, ScriptPurpose }`).
`ScriptPurpose` re-exported from V1.

**TxInfo — 12 fields** (adds 2):
- **Added** `txInfoReferenceInputs :: [TxInInfo]` (CIP-31 reference inputs)
- **Added** `txInfoRedeemers :: Map ScriptPurpose Redeemer`
- **Retyped** `txInfoWdrl :: Map StakingCredential Integer` (was assoc list)
- **Retyped** `txInfoData :: Map DatumHash Datum` (was assoc list)

**TxOut — 4 fields** (arity grows from 3):
```haskell
data OutputDatum = NoOutputDatum
                 | OutputDatumHash DatumHash
                 | OutputDatum Datum                      -- 3-way ctor 0/1/2
data TxOut = TxOut
  { txOutAddress         :: Address
  , txOutValue           :: Value
  , txOutDatum           :: OutputDatum                   -- replaces Maybe DatumHash
  , txOutReferenceScript :: Maybe ScriptHash              -- CIP-33
  }
```

### V3 ScriptContext (Conway) — envelope REFACTORED

```haskell
ScriptContext                        -- 3 fields, Constr 0 arity 3
  { scriptContextTxInfo     :: TxInfo
  , scriptContextRedeemer   :: Redeemer          -- now in envelope
  , scriptContextScriptInfo :: ScriptInfo }      -- replaces ScriptPurpose in envelope
```

`ScriptInfo` is a richer version of `ScriptPurpose` — carries extra
context the script needs. `ScriptPurpose` still exists as a separate
(narrower) type used only as the key of `txInfoRedeemers`.

**TxInfo — 16 fields** (adds 4, renames/retypes 4):
```haskell
txInfoInputs                  :: [TxInInfo]      -- unchanged
txInfoReferenceInputs         :: [TxInInfo]      -- unchanged
txInfoOutputs                 :: [V2.TxOut]      -- reuses V2 TxOut
txInfoFee                     :: Lovelace        -- retyped (was Value)
txInfoMint                    :: MintValue       -- retyped (was Value)
  -- MintValue = UnsafeMintValue (Map CurrencySymbol (Map TokenName Integer))
txInfoTxCerts                 :: [TxCert]        -- renamed from txInfoDCert + new cert ADT
txInfoWdrl                    :: Map Credential Lovelace   -- key & value retyped
txInfoValidRange              :: POSIXTimeRange  -- unchanged
txInfoSignatories             :: [PubKeyHash]    -- unchanged
txInfoRedeemers               :: Map ScriptPurpose Redeemer  -- unchanged
txInfoData                    :: Map DatumHash Datum         -- unchanged
txInfoId                      :: TxId            -- unchanged
txInfoVotes                   :: Map Voter (Map GovernanceActionId Vote)   -- NEW
txInfoProposalProcedures      :: [ProposalProcedure]                        -- NEW
txInfoCurrentTreasuryAmount   :: Maybe Lovelace                             -- NEW
txInfoTreasuryDonation        :: Maybe Lovelace                             -- NEW
```

Nothing is semantically removed; four Conway-governance additions,
three retypings (fee, mint, wdrl), one rename (DCert → TxCerts).

### Wire encoding consequences

All three versions derive `ToData`/`FromData` via `makeIsDataIndexed`
— Data encoding is `Constr idx [fields...]` with positional fields
in record declaration order.

A V3 script receiving a V1/V2 envelope will phase-2-fail at
`unsafeDataAsConstr` because the envelope arity is wrong
(3 vs 2 fields). Ade's ScriptContext builder must dispatch on the
executing script's language version.

### Citations

| Source | Reference |
|--------|-----------|
| V1 Contexts | `plutus-ledger-api/src/PlutusLedgerApi/V1/Contexts.hs` at 1.57.0.0 |
| V1 Tx (TxOut) | `plutus-ledger-api/src/PlutusLedgerApi/V1/Tx.hs` L118-122 |
| V2 Contexts | `plutus-ledger-api/src/PlutusLedgerApi/V2/Contexts.hs` L81-108, L103 |
| V2 Tx (TxOut + OutputDatum) | `plutus-ledger-api/src/PlutusLedgerApi/V2/Tx.hs` L67, L85-89 |
| V3 Contexts | `plutus-ledger-api/src/PlutusLedgerApi/V3/Contexts.hs` L485-538 |
| CIPs | CIP-0031 (ref inputs), CIP-0032 (inline datums), CIP-0033 (ref scripts), CIP-1694 (Conway gov) |

---

## O-31.2 — Reference Input Representation (V2/V3)

**Obligation:** Are reference inputs in a separate field or merged
with regular inputs?

### Answer

**Separate field in both V2 and V3.** Not merged with
`txInfoInputs`.

```haskell
-- V2 TxInfo (L82-84):
txInfoInputs          :: [TxInInfo]
txInfoReferenceInputs :: [TxInInfo]   -- "Added in V2" comment
```

V3 retains the same split verbatim (V3 `Contexts.hs` L486-487).

Wire-level: Both lists appear as distinct sibling list elements at
positions 0 and 1 of the `TxInfo` Constr payload. They are NEVER
concatenated.

### Script execution model

Script invocations are driven by `ScriptPurpose` / `ScriptInfo`.
Neither V2's `ScriptPurpose` nor V3's `ScriptInfo` has a variant
constructed from a reference input:

- V2: `Minting | Spending | Rewarding | Certifying` — `Spending`
  takes a `TxOutRef` that must appear in `txInfoInputs`, not
  `txInfoReferenceInputs`.
- V3: adds `VotingScript | ProposingScript`, still no
  reference-input purpose.

Reference inputs thus provide **data access only** — no script
is triggered by their presence. `findOwnInput` in both V2 and V3
searches `txInfoInputs` only (V2 `Contexts.hs` L148, L166 has an
explicit comment: *"this only searches the true transaction
inputs and not the referenced transaction inputs."*).

### TxInInfo resolution

Both `txInfoInputs` and `txInfoReferenceInputs` carry
`TxInInfo { txInInfoOutRef, txInInfoResolved :: TxOut }`. The
resolved `TxOut` is the full V2 `TxOut` (address, value, datum
option, reference script). An inline-datum or reference-script
UTxO used as a reference input is fully visible — datum inline,
reference script hash present — without being consumed.

### Citations

| Source | Reference |
|--------|-----------|
| V2 TxInfo | `plutus-ledger-api/src/PlutusLedgerApi/V2/Contexts.hs` L81-108 |
| V2 findOwnInput note | same file L148, L166 |
| V3 TxInfo | `plutus-ledger-api/src/PlutusLedgerApi/V3/Contexts.hs` L485-506 |
| TxInInfo | V2 L66-69, V3 L469-472 |
| Spec | CIP-0031 |

---

## O-31.3 — Conway Governance ScriptInfo Variants

**Obligation:** Enumerate the per-variant contents of V3
`ScriptInfo`. How do governance scripts see tx context?

### V3 `ScriptInfo` (L451-466)

```haskell
data ScriptInfo
  = MintingScript    CurrencySymbol
  | SpendingScript   TxOutRef (Maybe Datum)
  | RewardingScript  Credential
  | CertifyingScript Integer TxCert          -- Integer = cert index
  | VotingScript     Voter
  | ProposingScript  Integer ProposalProcedure  -- Integer = proposal index
```

`ScriptInfo` differs from V3's `ScriptPurpose` only by the
`Maybe Datum` on `SpendingScript`. `ScriptPurpose` (L433) is the
narrower type used as `txInfoRedeemers` key; `ScriptInfo` is what
a script actually receives in its `ScriptContext`.

### Voter (L242-246)

```haskell
data Voter = CommitteeVoter HotCommitteeCredential  -- newtype over Credential
         | DRepVoter      DRepCredential            -- newtype over Credential
         | StakePoolVoter PubKeyHash                -- PKH only — SPO cold key
```

SPO voters are key-credential only. Script-credential voters can
only be DRep or committee member.

### ProposalProcedure (L416-420)

```haskell
data ProposalProcedure = ProposalProcedure
  { ppDeposit          :: Lovelace
  , ppReturnAddr       :: Credential       -- script cred → triggers ProposingScript
  , ppGovernanceAction :: GovernanceAction }
```

`GovernanceAction`: `ParameterChange | HardForkInitiation |
TreasuryWithdrawals | NoConfidence | UpdateCommittee |
NewConstitution | InfoAction` (L388-410).

### TxCert (L189-216) — Conway's extended cert set

11 variants replace V1/V2's `DCert`:
`TxCertRegStaking | TxCertUnRegStaking | TxCertDelegStaking (Delegatee) |
TxCertRegDeleg | TxCertRegDRep | TxCertUpdateDRep | TxCertUnRegDRep |
TxCertPoolRegister | TxCertPoolRetire | TxCertAuthHotCommittee |
TxCertResignColdCommittee`.

### Execution model — critical semantics for Ade

**`VotingScript` invocation: once per script-credentialed voter, NOT once per vote.**

The voter's full `Map GovernanceActionId Vote` is visible in that
single invocation — the script authorises the entire bundle of
that voter's votes atomically. Redeemer map keying:
`txInfoRedeemers[Voting voter]` — single entry per voter.

**`ProposingScript` invocation: once per proposal whose
`ppReturnAddr` is a script credential.** Keyed by proposal index.

**`CertifyingScript` invocation: once per cert whose credential is
a script.** Keyed by cert index.

### Governance visibility

Every script (regardless of variant) receives the full `TxInfo`
including `txInfoVotes` and `txInfoProposalProcedures`. A spending
or minting script can inspect the entire ballot/proposal landscape,
not just the row that triggered it.

### Ade implication

Ade's redeemer-map constructor must emit one redeemer entry per
script-credentialed voter (covering their entire vote bundle).
Do NOT generate per-(voter, action_id) redeemers. Same for
proposals: per-proposal, not per-action.

### Citations

| Source | Reference |
|--------|-----------|
| ScriptInfo | `plutus-ledger-api/src/PlutusLedgerApi/V3/Contexts.hs` L451-466 |
| ScriptPurpose | same file L433 |
| Voter | L242-246 |
| ProposalProcedure | L416-420 |
| GovernanceAction | L388-410 |
| TxCert | L189-216 |
| TxInfo | L485-506 |
| ScriptContext | L532-538 |
| Spec | CIP-1694 |

---

## O-31.4 — Datum Resolution in ScriptContext

**Obligation:** When an input refers to a datum by hash, does
ScriptContext include the body or only the hash? Per V1/V2/V3.

### Summary

Input datums are referenced by hash on the TxOut; the body is
supplied separately via `txInfoData` (V1/V2/V3) or inline via
`OutputDatum` (V2/V3). V3 additionally delivers the spending
datum directly in `ScriptInfo.SpendingScript`.

### V1 — hash-only outputs + witness-set datum list

- `TxOut` carries **only** `Maybe DatumHash` (no inline).
- `txInfoData :: [(DatumHash, Datum)]` — flat witness-datums
  assoc list.
- Access path: `txInInfoResolved.txOutDatumHash` → `findDatum`
  in `txInfoData`.

### V2 — Babbage `OutputDatum` sum + witness map kept

- `txOutDatum :: OutputDatum` where
  `OutputDatum = NoOutputDatum | OutputDatumHash DatumHash | OutputDatum Datum`.
- `txInfoData :: Map DatumHash Datum` (retyped from assoc list).
- Access path: check `txOutDatum` variant:
  - `OutputDatum d` → use `d` directly.
  - `OutputDatumHash h` → `findDatum h` in `Map`.
  - `NoOutputDatum` → script-locked output without datum is
    invalid to spend under V2.

### V3 — same TxOut + `ScriptInfo` carries own-input datum

- Reuses V2 `TxOut` / `OutputDatum` (no V3-local redefinition).
- `txInfoData :: Map DatumHash Datum` unchanged.
- **New**: `ScriptInfo.SpendingScript TxOutRef (Maybe Datum)` —
  ledger pre-resolves the own-input datum and hands it to the
  script directly as `Maybe Datum`.

### CIP-0069 (V3 mandatory datum)

V3 script-locked outputs **require** a datum at the ledger level
(phase-1 check). A V3 spend of a V1-hash-only output with no
datum would phase-1-fail. Consequence: the `Maybe` in
`SpendingScript` is always `Just` for V3-script inputs. This is
a ledger-rule constraint, not a type-level change in
`plutus-ledger-api`.

### Ade ScriptContext builder rules

1. V1/V2 spend of hash-only UTxO: populate `txInfoData` from the
   witness set (`TxWitnessSet.plutusData`). Do NOT resolve into
   the TxOut.
2. V2/V3 inline-datum output: place the body in
   `txOutDatum = OutputDatum d`. Do NOT also add to `txInfoData`
   (V2 ledger semantics — inline datums are NOT duplicated in
   the witness map).
3. V3 spend: additionally set
   `ScriptInfo = SpendingScript ref (Just d)` where `d` is the
   resolved datum (from inline or from witness map).
4. Reference inputs: carry full resolved TxOut (including
   inline datum if present) but do NOT participate in
   SpendingScript construction.

### Citations

| Source | Reference |
|--------|-----------|
| V1 TxOut | `plutus-ledger-api/src/PlutusLedgerApi/V1/Tx.hs` L118-122 |
| V1 txInfoData | `plutus-ledger-api/src/PlutusLedgerApi/V1/Contexts.hs` L107-124, findDatum L180-181 |
| V2 OutputDatum | `plutus-ledger-api/src/PlutusLedgerApi/V2/Tx.hs` L67 |
| V2 TxOut | same file L85-89 |
| V2 txInfoData (Map) | `plutus-ledger-api/src/PlutusLedgerApi/V2/Contexts.hs` L103 |
| V3 SpendingScript | `plutus-ledger-api/src/PlutusLedgerApi/V3/Contexts.hs` L451-453 |
| V3 txInInfoResolved | V3 `Contexts.hs` L471 |
| V3 txInfoData | V3 `Contexts.hs` L500 |
| Specs | CIP-0031, CIP-0032, CIP-0069 |

---

## Summary of decisions locked for S-31

1. **Three per-version builders**. Ade's `ade_plutus` crate gains
   `script_context::build_v1(tx, state) -> PlutusData`,
   `build_v2(tx, state) -> PlutusData`,
   `build_v3(tx, state) -> PlutusData`. Callers dispatch on
   executing-script language version (already known from
   witness-set keys 3/6/7).

2. **Per-version Data shape** fixed by `makeIsDataIndexed`
   derivations: V1 TxInfo = Constr 0 arity 10; V2 = arity 12;
   V3 = arity 16. V3 envelope = Constr 0 arity 3 (not 2).
   Positional field order matches record-declaration order above.

3. **Reference inputs always separate**. Never merged with spend
   inputs, regardless of version.

4. **Datum resolution strategy per version**:
   - V1/V2 hash-only: build `txInfoData` map from witness datums.
   - V2/V3 inline: place in TxOut's `OutputDatum`, do NOT
     duplicate in `txInfoData`.
   - V3 spend: set `ScriptInfo.SpendingScript _ (Just d)` by
     pre-resolving from inline or witness map.

5. **Governance dispatch semantics (V3)**:
   - One VotingScript invocation per script-credentialed voter
     (not per vote).
   - One ProposingScript invocation per script-credentialed
     proposal (not per contained governance action).
   - Redeemer map keyed by `ScriptPurpose`, not `ScriptInfo`.

6. **TxInfo retypings for V3**: Fee → `Lovelace`, Mint →
   `MintValue` (newtype over nested map), Wdrl key → `Credential`
   (not `StakingCredential`). CertDCert → `TxCert` (11-variant
   Conway ADT).

7. **Aiken Data types already available** transitively via
   `pallas-primitives` inside the ade_plutus quarantine. Ade's
   builder emits Ade-canonical intermediates and converts at the
   boundary.

---

## Authority Reminder

This discharge is a planning artifact. If any finding conflicts
with the plutus 1.57.0.0 source tree, the source is authoritative
for wire-format decisions. CIP text is authoritative for
intention; source is authoritative for implementation.
