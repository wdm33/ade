# SLICE: MITHRIL-VERIFIED-ANCHOR-INTEGRATION / S1a — native non-UTxO snapshot decoder

## Claim (narrow)
A verified Conway V2 snapshot yields a point- and network-bound native non-UTxO authority state with
protocol parameters preserved without cross-era semantic remapping. Concretely: a verified V2 LedgerDB
`state` file + the manifest-authoritative chain point + the manifest network magic deterministically
yield the COMPLETE authoritative NON-UTxO ledger state — `NativeSnapshotNonUtxoState` — natively, with
every snapshot-present field decoded and commitment-bound, the internal `network_id` derived from the
manifest magic and bound onto every authority-bearing field, and the Conway `coinsPerUTxOByte` preserved
faithfully as a per-byte rule (NOT remapped onto the Shelley absolute-floor `minUTxOValue`), and **no
CLI / JSON consensus-input bundle / operator seed participating**. This is a DERIVED compatibility
decoder (BLUE, deterministic): it EMITS a structure + a commitment; it does NOT assemble
`LedgerState`/`PraosChainDepState` and does NOT persist (that is S1b, the authority transition). It does
NOT follow / ChainSync (that is S2, the release gate).

## Input → Output
- Input: `state_cbor: &[u8]` (the V2 LedgerDB `state` file = the HardFork telescope ending in Conway's
  NewEpochState, plus the headerState), `point: SeedPoint` (the manifest-certified slot+hash),
  `manifest_epoch: u64` (the manifest-certified epoch), `manifest_network_magic: u32` (the
  manifest-certified network magic; the internal `network_id` is derived from it — mainnet
  `764824073` → 1, any other (testnet) magic → 0).
- Output: `NativeSnapshotNonUtxoState` (BLUE struct) carrying every field below + `commitment: Hash32`
  (blake2b over the canonical encoding of ALL emitted fields).

## Must emit — each decoded AND bound; NO silent default
| Field | Type | Source in the `state` file |
|---|---|---|
| `era` | CardanoEra | the telescope era index (Conway = 6, EXPLICIT, never inferred) |
| `network_id` | u8 | DERIVED from `manifest_network_magic` (mainnet → 1, testnet → 0); also set on `protocol_params.network_id` |
| `epoch` | EpochNo | NewEpochState `nesEL` — cross-checked `== manifest_epoch` |
| `point` | SeedPoint | the manifest-certified point (bound through, not decoded) |
| `cert_state` | CertState | `nesEs.esLState.lsCertState` — the FULL struct, not a commitment hash |
| `praos_nonces` | all 5: evolving, candidate, epoch(eta0), previous_epoch, lab | the headerState / PraosState |
| `pool_distr` | BTreeMap<PoolId, (stake: u64, vrf: Hash32)> | `nesPd` (PoolDistr) — the VRF bindings |
| `protocol_params` | ProtocolParameters | the Conway current params (`esPp` / GovState curPParams — locate empirically); `min_utxo_rule = MinUtxoRule::PerByte(coinsPerUTxOByte)`, NOT remapped; `network_id` = the derived id |
| `reserves` | Coin | `nesEs.esAccountState.reserves` |
| `treasury` | Coin | `nesEs.esAccountState.treasury` |
| `block_production` | BTreeMap<PoolId, u64> | `nesBprev` (blocks made in the previous epoch) |

## Network identity (DERIVED; DC-LEDGER-PARAMS bind)
- `network_id` is DERIVED from the manifest network magic (mainnet `764824073` → 1; any other (testnet)
  magic → 0) and bound onto the emitted state, `protocol_params.network_id`, and the commitment.
  *Rationale (derived):* Cardano compatibility requires the imported state bound to the intended network;
  this strengthens the coherence invariant that ONE manifest binds every authority-bearing field.
- The pool reward-account network nibble is OPERATOR-controlled ledger data, NOT a network discriminator.
  EMPIRICAL FINDING (real preprod snapshot, epoch 296): the V2 `state` file carries **no
  network-authoritative identity field** — each pool reward-account stake-address header nibble is
  operator-supplied and demonstrably MIXED (both net-0 and net-1 reward accounts coexist on real preprod).
  It is therefore recorded as a DIAGNOSTIC `RewardNibbleObservation` (`Uniform`/`Mixed`/`None`) in the
  canonical report — committed evidence that NEVER accepts or rejects the snapshot. A unanimous-only
  terminal would be a heuristic masquerading as an authority check; the manifest magic is the SOLE network
  authority, and a stronger decoded cross-check awaits a genuinely network-authoritative snapshot field.

## Protocol params: faithful preservation, NO cross-era semantic remap (DC-LEDGER-PARAMS-01)
- The shared `ProtocolParameters.min_utxo_value: Coin` is replaced by `min_utxo_rule: MinUtxoRule`, an
  era-aware sum type: `LegacyAbsoluteMin(Coin)` (Shelley/Mary absolute per-output floor) and
  `PerByte(Coin)` (Conway `coinsPerUTxOByte`, a per-byte coefficient — NOT an absolute floor).
- S1a decodes Conway `coinsPerUTxOByte` (4310 on the real snapshot) into `MinUtxoRule::PerByte(Coin(4310))`.
  It MUST NOT populate `LegacyAbsoluteMin` from it.
- The min-UTxO VALIDATION matches on `MinUtxoRule`: `LegacyAbsoluteMin(c)` → the existing absolute check;
  `PerByte(_)` → a structured TERMINAL `LedgerError::UnsupportedConwayMinUtxoRule` (fail closed rather
  than accept outputs under a false minimum). The old permissive absolute check is NOT left active for
  the per-byte rule.
- *Classification:* **true** — imported protocol parameters may not be semantically remapped; **derived**
  — Conway `coinsPerUTxOByte` is a per-byte rule, not an absolute floor; **RELEASE BLOCKER** — Ade's
  Conway min-UTxO validation must compute the era-correct per-byte minimum before the native bootstrap
  path can claim full ledger-validation compatibility.
- *Byte-identity / replay:* the canonical pparams encoders (`encode_pparams`, `fingerprint_pparams`)
  serialize the rule's coin payload only — byte-identical to the prior single-`Coin` field for legacy
  (`LegacyAbsoluteMin`) states, so the pinned non-Conway pparams fingerprints and all differential replay
  suites are unchanged. The rule KIND is bound separately in the S1a commitment (which bumps to v2).

## No-silent-default rule (HARD)
Do NOT fill any field above with a default. A default is legitimate ONLY for a field PROVEN absent from
the snapshot and defined by Ade's cold-start semantics (e.g. esSnapshots mark/set/go, op-cert counters)
— those belong to S1b, NOT here. For EVERY field present in the snapshot, decode and bind it. A missing
expected field is a TERMINAL error, never a default.

## Fail closed (terminal, structured errors — never a partial or defaulted emission)
- unsupported telescope layout / non-Conway era → terminal;
- `epoch != manifest_epoch` → terminal (point/epoch coherence);
- any expected field missing / malformed → terminal (no default fill);
- inconsistent pool/VRF data (a PoolDistr pool whose VRF disagrees with the CertState pool's VRF, or a
  zero VRF) → terminal;
- protocol params / accountState / blocksPrev absent where expected → terminal.

## Empirical grounding (slice-entry obligation; required)
The exact CBOR offsets for `esPp` (protocol params), `esAccountState` (reserves/treasury), and `nesBprev`
(block_production) in the Conway NewEpochState are slice-entry decode work — locate them empirically
against the real snapshot `.mithril-scratch/restore-ancillary/db/ledger/126400064/state` (cardano-node
11.0.1, cardano-ledger-conway 1.22.1.0). The existing `probe_ledgerdb_state`
(`crates/ade_ledger/src/ledgerdb_state.rs`) already navigates to CertState + PoolDistr + the 5 nonces —
extend that navigation. VALIDATE each newly-decoded field against the real snapshot with a sanity check:
- protocol_params: plausible Conway/preprod values (min_fee_a/min_fee_b, key_deposit, pool_deposit,
  max_tx_size, protocol_major) — sane, not garbage;
- reserves + treasury: plausible; reserves + treasury + circulating-from-UTxO ≈ max_lovelace_supply (45e15);
- block_production: pool ids ⊆ CertState pools; counts plausible;
- pool_distr VRF bindings agree with CertState pool VRFs.

## Acceptance
- same snapshot + manifest → BYTE-IDENTICAL `NativeSnapshotNonUtxoState` (deterministic; commitment equality);
- all emitted fields are commitment-bound (perturb any field, incl. `network_id` and the min-UTxO rule
  KIND → commitment changes — tested);
- `network_id` is DERIVED from the manifest magic (mainnet → 1, testnet → 0) and bound on the state + the
  protocol params (tested);
- Conway `coinsPerUTxOByte` decodes to `MinUtxoRule::PerByte` (NOT an absolute min); the Conway min-UTxO
  validation is TERMINAL (`UnsupportedConwayMinUtxoRule`, no permissive absolute fallback); the legacy
  `LegacyAbsoluteMin` path is unchanged (regression tested);
- protocol params / pots / block-production are decoded FROM the state file (sanity-checked, not defaulted,
  not external);
- no CLI, JSON consensus-input bundle, or operator seed participates (the function takes only
  `state_cbor` + `point` + `manifest_epoch` + `manifest_network_magic`);
- fail-closed negatives: malformed / missing-field / non-Conway / epoch-mismatch / inconsistent-VRF are
  terminal; network identity is NOT a terminal check (the reward nibble is diagnostic evidence only).

## CI gate
`ci/ci_check_native_nonutxo_decode.sh`: the decoder emits all listed fields (incl. `network_id`);
fail-closed on a missing field (no silent default); commitment binds all fields; era/epoch/point coherence
enforced; `network_id` is derived (no `network_id = 0` literal placeholder), the reward nibble is a
DIAGNOSTIC `RewardNibbleObservation` (no terminal network check); `min_utxo_rule`/`MinUtxoRule` present
(no `min_utxo_value` field remains on `ProtocolParameters`);
Conway decodes to `PerByte` (grep, no `LegacyAbsoluteMin` remap); the Conway min-UTxO validation path is
terminal (`UnsupportedConwayMinUtxoRule` in both `shelley.rs` and `mary.rs`, no permissive fallback); the
hermetic + real-snapshot tests pass. Registry: `DC-LEDGER-PARAMS-01`.

## Scope fence
DECODER ONLY. `NativeSnapshotNonUtxoState` is emitted + committed; it is NOT assembled into
`LedgerState`/`PraosChainDepState` and NOT persisted (S1b). No CLI / JSON bundle / operator seed. No
Mithril routing / admission / ChainSync wiring (S2). Do not touch the value model (DC-LEDGER-VALUE-01 is
committed and frozen for this slice).
