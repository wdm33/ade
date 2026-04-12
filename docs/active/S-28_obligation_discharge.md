# Slice S-28 Entry Obligation Discharge

> **Status:** Discharged. S-28 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 3
> cluster plan §"Slice-Entry Proof Obligations").
>
> **Version scope:** cardano-node 10.6.2; cardano-ledger source tree
> at the tag aligned with that release.

This document discharges the four slice-entry proof obligations
gating S-28 (reference inputs + datum-hash + required signers +
network ID, Cluster P-A). Each obligation is answered with
citations to the Haskell ledger source; no discharge is by
footnote or assumption.

---

## O-28.1 — Reference Input Also in Spend Inputs

**Obligation:** Can a `TxIn` appear in both `reference_inputs`
(Babbage+) and `inputs` (spend) of the same transaction?

### Answer

**Protocol-version gated.** Disallowed from Conway (PV ≥ 9)
onward; silently allowed in Babbage (PV 7–8). Babbage itself does
not enforce the disjointness requirement; Conway added it as an
era-boundary tightening.

The Haskell predicate:

```haskell
disjointRefInputs pp inputs refInputs =
  when
    ( pvMajor (pp ^. ppProtocolVersionL) > eraProtVerHigh @BabbageEra
        && pvMajor (pp ^. ppProtocolVersionL) < natVersion @11 )
    (failureOnNonEmpty (inputs ∩ refInputs) BabbageNonDisjointRefInputs)
```

- `eraProtVerHigh @BabbageEra` = 8, so the gate fires when
  `PV > 8` (i.e. Conway PV 9 and 10).
- Upper bound `< 11` reserves room for a future era to re-evaluate.
- Error constructor: `BabbageNonDisjointRefInputs (NonEmpty TxIn)`
  carrying the non-empty intersection.

### Rust implications

- The check function must take the current protocol version as
  input; the return must be `Ok(())` for PV ≤ 8 regardless of
  overlap.
- For PV 9 (and PV 10 when it ships), any `inputs ∩ reference_inputs`
  non-empty must raise `BabbageNonDisjointRefInputs`.
- Ade's `LedgerError` needs this variant with a `BTreeSet<TxIn>`
  payload to match the Haskell `NonEmpty TxIn`.

### Citations

| Source | Reference |
|--------|-----------|
| Predicate | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxo.hs` `disjointRefInputs` ~L135–149 |
| Constructor | same file `BabbageUtxoPredFailure::BabbageNonDisjointRefInputs` ~L95–113 |
| Invocation | same file `babbageUtxoValidation` ~L361 |
| Conway embedding | `eras/conway/impl/src/Cardano/Ledger/Conway/Rules/Utxo.hs` `babbageToConwayUtxoPredFailure` |
| Spec | CIP-0031 (reference inputs) |

---

## O-28.2 — Datum Hash Binding

**Obligation:** When an output has a `datum_hash` (not inline
datum), and a script consuming it needs the datum body, the
witness set supplies a datum. Does the ledger check bit-exact
equality of the witness datum's hash against the output's
`datum_hash`, or canonical-form equality?

### Answer

**Bit-exact hashing over the witness datum's raw wire bytes.**

The witness datum is stored via `MemoBytes`, which preserves the
original serialized bytes captured during deserialization.
`hashData` computes `Blake2b-256` over those preserved bytes, NOT
over a re-encoding of the decoded `PlutusData`:

```haskell
hashData :: Data era -> DataHash
hashData = hashAnnotated

instance HashAnnotated (Data era) EraIndependentData where
  hashAnnotated = getMemoSafeHash  -- hashes stored MemoBytes
```

If a wallet re-encodes a datum (e.g., through a CBOR round-trip
that reshuffles map keys or picks different integer widths), the
hash changes and the ledger rejects the tx with
`MissingRequiredDatums`.

The UTXOW check:

```haskell
txHashes             = Map.keysSet (tx ^. witsTxL . datsTxWitsL . unTxDatsL)
unmatchedDatumHashes = Set.difference inputHashes txHashes
```

`inputHashes` comes from `getInputDataHashesTxBody` which reads
each output's `DatumHash dataHash` directly.

### Inline datums (Babbage+) skip this check entirely

`getInputDataHashesTxBody` only emits a hash for the
`DatumHash dataHash` case; `Datum _` (inline) and `NoDatum` fall
through and contribute nothing to `inputHashes`. The inline
datum's integrity comes from its presence in the UTxO body
itself, not from a separate hash check.

### Rust implications

- The witness-datum parser MUST preserve raw wire bytes (already
  the pattern used for tx bodies via `PreservedCbor<T>`). Do NOT
  re-encode before hashing.
- Check: `Blake2b-256(witness_datum_wire_bytes) == output.datum_hash`.
- For each required datum hash, if no witness-provided datum
  hashes to it, raise `MissingRequiredDatums` carrying the
  unmatched set.
- Inline-datum outputs bypass this check (no witness datum
  needed).

### Citations

| Source | Reference |
|--------|-----------|
| `hashData` | `libs/cardano-ledger-core/src/Cardano/Ledger/Plutus/Data.hs` L115, L193 |
| Memo bytes preservation | `libs/cardano-ledger-core/src/Cardano/Ledger/MemoBytes/Internal.hs` L107, L195 |
| Witness parse | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/TxWits.hs` L339, L346 (`\dat -> (hashData dat, dat)`) |
| UTXOW check | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxow.hs` `missingRequiredDatums` L226–250 |
| Input hash collection | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/UTxO.hs` `getInputDataHashesTxBody` L165–194 |
| Babbage invocation | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxow.hs` L368 |

---

## O-28.3 — Required Signers Enforcement

**Obligation:** Must every `Hash28` in `required_signers` appear
in `vkey_witnesses`, or is a redeemer-provided / script-verified
signer acceptable?

### Answer

**Strict subset check on vkey witness key-hashes.**
`required_signers ⊆ { Blake2b-224(vk) | vk ∈ vkey_witnesses }`.
Plutus scripts, redeemers, or native scripts that *reference* a
signer hash are NOT acceptable substitutes — every required
signer hash must be matched by an actual vkey witness with a
valid Ed25519 signature.

The Haskell:

```haskell
getAlonzoWitsVKeyNeeded certState utxo txBody =
  getShelleyWitsVKeyNeeded certState utxo txBody
    `Set.union` Set.map asWitness (txBody ^. reqSignerHashesTxBodyG)
```

The subset check runs inside `validateNeededWitnesses`:

```haskell
validateNeededWitnesses witsKeyHashes certState utxo txBody =
  let needed = getWitsVKeyNeeded certState utxo txBody
      missingWitnesses = Set.difference needed witsKeyHashes
   in failureOnNonEmptySet missingWitnesses MissingVKeyWitnessesUTXOW
```

### Unconditional — applies without Plutus scripts

The union into `witsVKeyNeeded` runs for every Alonzo+ tx.
A tx with a non-empty `required_signers` but no Plutus script
still requires a matching vkey witness. The formal spec is
explicit.

### Error constructor

**`MissingVKeyWitnessesUTXOW`** — the general Shelley
witness-shortfall error. There is NO dedicated
`MissingRequiredSigners` constructor (it was removed from
Alonzo). Required-signers failures are reported as ordinary
missing-vkey-witness failures.

### Babbage / Conway

No rule change. Same union, same subset check, same error
constructor.

### Rust implications

- Ade's existing `WitnessInfo.available_key_hashes: BTreeSet<Hash28>`
  is exactly the needed input (already computed as
  `Blake2b-224(vkey_bytes)`).
- Check: `required_signers.is_subset(&available_key_hashes)`;
  missing set → error.
- Reuse the existing `MissingWitness` / `WitnessError` domain, OR
  add a dedicated variant for clarity. Since Haskell folds this
  into `MissingVKeyWitnessesUTXOW`, Ade can do the same — the
  existing `WitnessError` with algorithm `Ed25519` and the
  missing key_hash is semantically equivalent, just reporting one
  hash at a time rather than a set.

### Citations

| Source | Reference |
|--------|-----------|
| `getAlonzoWitsVKeyNeeded` | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/UTxO.hs` L321–337 |
| EraUTxO instance | same file L81, L97 |
| Subset check | `eras/shelley/impl/src/Cardano/Ledger/Shelley/Rules/Utxow.hs` `validateNeededWitnesses` L421–432 |
| Error | `MissingVKeyWitnessesUTXOW` (Shelley `ShelleyUtxowPredFailure` tag 2) |
| Conway reuse | `eras/conway/impl/src/Cardano/Ledger/Conway/UTxO.hs` `getConwayWitsVKeyNeeded` |
| Spec | `eras/alonzo/formal-spec/utxo.tex` L764, L818–820 |

---

## O-28.4 — Network ID Check

**Obligation:** Which field carries the network ID check? What's
the exact check semantics?

### Answer

Two independent checks, both in the UTXO rule:

### Check 1 — Tx-body `network_id` field (Alonzo+, optional)

`network_id` (tx body key 15, `StrictMaybe Network`) is a
redundant sanity field. Purpose: defense-in-depth against an
off-net wallet mis-signing a tx that happens to contain only
cross-compatible artifacts. NOT surfaced in Plutus ScriptContext
(no `txInfoNetworkId` field exists in the Alonzo TxInfo).

Rule (`validateWrongNetworkInTxBody`):
```
(txnetworkid = netId) ∨ (txnetworkid = SNothing)
```

If absent (`SNothing`): pass.
If present (`SJust bid`): require `bid == netId`.

Error: `WrongNetworkInTxBody(Mismatch RelEQ Network)` —
Alonzo `AlonzoUtxoPredFailure` tag 17.

### Check 2 — Output addresses (Shelley+, always)

Every output's address network byte must match the current
network. Inputs are not re-checked (already validated when they
became outputs; by induction in-network).

Rule (`validateWrongNetwork`, Shelley-era predicate, inherited
through all later eras):
```
∀(_ → (a, _)) ∈ txouts txb, netId a = NetworkId
```

Error: `WrongNetwork Network (Set Addr)` —
Shelley `ShelleyUtxoPredFailure` tag 8. Re-exported in Alonzo.

Withdrawals: separately checked by
`validateWrongNetworkWithdrawal`, error tag 9.

### Babbage widens output check to include `collateralReturn`

Babbage replaces `outputs` with `allOutputs = outputs ++
[collateralReturn]` for the address-level check. The tx-body
`network_id` rule is unchanged.

### Network encoding in addresses

The network ID occupies the low nibble of the address's first
byte. Mainnet = 1, testnets = 0. See Shelley address spec (CIP-19
predecessor). For Ade's purposes, the network byte is always
`address_bytes[0] & 0x0f`, and we compare against the current
network (1 for mainnet).

### Rust implications

- Two check functions needed:
  - `check_tx_network_id(declared: Option<u8>, current: u8)` —
    pass if `declared` absent or matches.
  - `check_output_networks(outputs: &[impl HasAddress], current: u8)`
    — every output's address byte must match.
- For Babbage+, callers pass the union
  `outputs ∪ {collateral_return?}`.
- Withdrawal network check: per-reward-account, deferred to S-32
  or a later slice when withdrawals are parsed.

### Citations

| Source | Reference |
|--------|-----------|
| `network_id` field | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/TxBody.hs` L141, L172, L450, L484 |
| Tx-body check | `eras/alonzo/impl/src/Cardano/Ledger/Alonzo/Rules/Utxo.hs` `validateWrongNetworkInTxBody` L449–462 |
| Tx-body error | same file `WrongNetworkInTxBody` L174–175, tag 17 |
| Output check | `eras/shelley/impl/src/Cardano/Ledger/Shelley/Rules/Utxo.hs` `validateWrongNetwork` L479–490, `WrongNetwork` error |
| Withdrawal check | same file `validateWrongNetworkWithdrawal`, error tag 9 |
| Babbage widening | `eras/babbage/impl/src/Cardano/Ledger/Babbage/Rules/Utxo.hs` L409–419 (`allOutputs` includes `collateralReturn`) |

---

## Summary of decisions locked for S-28

1. **Reference-input overlap**: PV-gated check. Only fails when
   `PV >= 9 && PV < 11`. Signature:
   `check_reference_input_disjoint(inputs, refs, pv)`.

2. **Datum hash binding**: bit-exact hashing of the witness
   datum's preserved wire bytes. `Blake2b-256(raw) == expected`.
   Inline datums skip this check.

3. **Required signers**: pure subset check
   `required ⊆ available_key_hashes`. Unconditional — runs
   regardless of Plutus presence. Reuse existing `WitnessError`
   domain per missing hash (mirrors Haskell's one-error-per-hash
   reporting pattern).

4. **Network ID — two checks**:
   - `check_tx_network_id(declared, current)` — pass if absent
     or matches.
   - `check_output_networks(addresses, current)` — every
     address's network nibble must match.
   - Babbage: callers pass `outputs ∪ {collateral_return?}`.

5. **New `LedgerError` variants**:
   - `NonDisjointRefInputs(BTreeSet<TxIn>)` — carries the
     intersection (Conway PV 9+).
   - `MissingRequiredDatums(BTreeSet<Hash32>)` — carries the
     unmatched set of datum hashes.
   - `WrongNetworkInTxBody { declared, current }` — tx-body
     field mismatch.
   - `WrongNetworkInOutput { address: Vec<u8>, network: u8 }` —
     output address network mismatch (one per offending output).

---

## Authority Reminder

This discharge is a planning artifact. If any finding conflicts
with the normative specifications or CI enforcement, the normative
specifications are authoritative. The Haskell source paths cited
above are the authoritative implementation at the cardano-node
10.6.2 tag; their behavior supersedes any paraphrase here.
