# SLICE LV-15a — admission obligations that need neither the UTxO nor protocol parameters

**Cluster:** PREPROD-LIVE-2-FORGE-READINESS. **Follows:** CENSUS-LV-15 §7 (decision (a), slice 1).
**Rules:** DC-TXV-09 (per transaction), DC-VAL-07 (per block). **Kind:** rejection only.

## 1. Why this slice is narrower than CENSUS-LV-15 §7 slice 1

The census listed the UTxO-free rules as one slice: validity interval, tx/output/block size, network ids,
collateral count, tx and block ex-units. The owner's boundary for merging them early is that each check
"needs no unavailable context and can only eliminate blocks Cardano rejects". Findings while scoping:

- **Protocol parameters are unavailable context at admission today.** Admission reads
  `LedgerState.protocol_params`. On the live reduced follower that copy is set at bootstrap and never
  changes: `apply_reduced_epoch_boundary` carries it unchanged, and enactment is the accumulator's alone
  (`EpochAccumulator.protocol_params`). Preprod has already enacted a parameter change Ade follows
  (e641ec80..#0: maxTxExUnits mem 16.5M → 17.5M, maxBlockExUnits mem 72M → 77.5M, minPoolCost). A limit check
  against the stale copy would reject blocks Cardano accepts after any raised limit.
- **Two needed limits are not held anywhere.** `maxValSize` (Conway `curPParams` index 19) and
  `maxCollateralInputs` (index 21) are skipped by the bootstrap decoder (`ledgerdb_state::read_conway_pparams`)
  and absent from `ProtocolParameters`.
- **The persisted parameters lose the min-UTxO rule's kind.** `snapshot::gov_state::decode_pparams` always
  rebuilds `MinUtxoRule::LegacyAbsoluteMin`; the accumulator persists through it, so after a restart
  Conway's per-byte `coinsPerUTxOByte` would read as an absolute floor. No rule reads it today; the output
  size rule would.
- **Reference-input disjointness is gated on the protocol version** (`disjointRefInputs`: 8 < PV < 11), a
  parameter too.

So every parameter-dependent rule (max tx size, output too small / too big, collateral count, tx and block
ex-units, header and body size limits, header protocol version, ref-input disjointness) moves to LV-15b,
whose prerequisite is an authoritative, epoch-bound parameter view at admission sourced from the
accumulator, with the missing fields and the min-UTxO kind persisted. That is a durable-state change and
gets its own findings.

This slice takes what is left: rules whose only inputs are the transaction or block bytes, the slot, and
the network id. The network id is not a protocol parameter in Cardano (it is `Globals.networkId`); Ade
binds it from the manifest network magic at bootstrap and governance cannot change it.

## 2. Cardano's rules (cardano-ledger cb57dc73)

Conway UTXO = `babbageUtxoValidation` (Babbage `Rules/Utxo.hs`), then UTXOS. The checks taken here, all
`runTest`/`runTestOnSignal`, so all apply in both `isValid` shapes:

| Cardano check | Definition | Inputs |
|---|---|---|
| `validateOutsideValidityIntervalUTxO` | `inInterval slot vi`: `before <= slot` when set, `slot < hereafter` when set | body, slot |
| `validateWrongNetwork` | every output in `allOutputs` (outputs and collateral return): `getNetwork addr == netId`. Shelley address: header nibble. **Bootstrap address: `NetworkTestnet` iff its attributes carry key 2 (network magic), else `Mainnet`.** | body, netId |
| `validateWrongNetworkWithdrawal` | every withdrawal reward account: `aaNetworkId == netId` | body, netId |
| `validateWrongNetworkInTxBody` | body field 15, when present, equals netId | body, netId |
| `validateOutputBootAddrAttrsTooBig` | every bootstrap output: `bootstrapAddressAttrsSize <= 64`, where size = length of the derivation-path payload (key 1, the inner bytes) + total length of the values of unknown attribute keys | body |
| BBODY `WrongBlockBodySizeBBODY` | `bBodySize == header body size`; `bBodySize` = length of `encCBORGroup` = the four preserved segments concatenated (`bodyBytes <> witsBytes <> metadataBytes <> invalidBytes`) | block |

`Mainnet` is network id 1, `Testnet` 0. `validateOutsideForecast` is not taken: it extends the epoch info
linearly (`unsafeLinearExtendEpochInfo`) and so cannot fail. Cardano decodes every bootstrap address fully
(`decodeBootstrapAddress` → `decodeFull'`, CRC included); a bootstrap output whose attributes cannot be
read is therefore a Cardano rejection too.

## 3. What Ade does today

- Validity interval: not checked at admission. (LV-13 reads the interval for timelocks only.)
- Network ids: not checked at admission. The full path's `late_era_validation::check_address_network`
  passes every bootstrap address (`addr_type_high == 0x8 → Ok`), which understates Cardano.
- Bootstrap attributes size: not checked anywhere.
- Declared body size: not checked. Admission binds the body *hash* to the header (block_validity step 4),
  not the size.

## 4. Design

One BLUE module, `crates/ade_ledger/src/tx_validity/utxo_free.rs`:

- `check_utxo_free_tx(body: &ConwayTxBody, slot: SlotNo, network_id: u8) -> Result<(), UtxoFreeRejection>`
  — the five per-transaction checks: `check_validity_interval(body, slot)` then
  `check_tx_networks(body, network_id)` (output network, withdrawal network, body network, bootstrap
  attributes). Order does not change acceptance.
- `bootstrap_address_network` / `bootstrap_attributes_size`, over one lenient read of the Byron address:
  `[tag24(bytes [root, attrs, type]), crc]`. Lenient means it checks no CRC and no canonical lengths, so
  it is weaker than Cardano's decoder, never stricter. An address it cannot read at all is
  `MalformedBootstrapAddress` (Cardano's decoder rejects it as well).
- `check_declared_body_size(block: &ShelleyBlock) -> Result<(), UtxoFreeRejection>` — the BBODY size rule.

Wiring (Conway blocks only, Ade's live era):

- the block rule runs in `apply_shelley_era_block_with_verdicts` / `_classified`, before the per-tx loop;
- the per-tx rule runs first inside the per-tx loop of `verify_conway_witness_closure`, which already
  decodes each body once — so the rejection reaches both the reduced and full modes. `tx_phase_one`
  calls the slot-free part (`check_tx_networks`); it carries no slot, so the standalone path does not
  check the validity interval;
- `check_address_network` is corrected for bootstrap addresses by delegating to the same reader, so the
  full path stops understating, with one definition of an address's network.

New `LedgerError::UtxoFreeObligation(UtxoFreeRejection)` mapped to Phase1 in `phase.rs`.

No state is written. An accepted block's outputs are unchanged.

## 5. Proof obligation: rejection only

Each check is a pure function of (body or block bytes, slot, network id). Slot comes from the validated
header; the network id is fixed per network. None reads state Ade does not hold, so none can reject on
missing context. Each equals Cardano's predicate except where §4 says it is weaker. So every rejection is
a block Cardano rejects. Census before enforcement confirms it on the real chain.

## 6. Mechanical acceptance

- **CE-LV15a-1** Census before enforcement over every stored block of a copy of the live store: 0 failures
  for every check, with counts of bootstrap outputs, withdrawals and transactions with a validity bound.
- **CE-LV15a-2** A tx before `invalid_before`, at `invalid_hereafter`, and after it rejects; at the bounds'
  inside edge it passes. In both modes.
- **CE-LV15a-3** A Shelley output, a collateral return, a withdrawal, and the body field each on the wrong
  network reject.
- **CE-LV15a-4** Bootstrap network: a real testnet bootstrap address passes on preprod; the same address with
  its key-2 attribute removed (re-CRC'd) rejects as wrong network; an unreadable one rejects as malformed.
- **CE-LV15a-5** Bootstrap attributes: payload + unknown = 64 passes, 65 rejects.
- **CE-LV15a-6** A block whose header body size is one off rejects; the real value passes.
- **CE-LV15a-7** Mutations, each killed by the intended test and named by the gate: (a) drop the interval
  check, (b) `<=` for `<` on the upper bound, (c) skip the collateral return in the output network check,
  (d) treat every bootstrap address as passing again, (e) skip the declared body size check.
- **CE-LV15a-8** Gate `ci/ci_check_utxo_free_admission_obligations.sh`.
- **CE-LV15a-9** Full sweep: failures identical to the baseline.
- **CE-LV15a-10** Live: the binary restarts on the live store and reaches the peer tip with no failure line.

## 7. Store semantics

`rules.rs` and `phase.rs` are on the surface. Neutral entry at the current version, as LV-13 and LV-14:
the checks write no state, so a store from the previous binary is replay-equivalent unless it holds a
block the new checks refuse, which the census rules out.

## 8. Results

(filled after implementation)
