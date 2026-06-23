#!/usr/bin/env bash
# ci_check_native_nonutxo_decode.sh -- MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1a.
#
# The native non-UTxO snapshot decoder is a DERIVED, BLUE, deterministic, fail-closed compatibility
# decoder. It takes only (state_cbor, point, manifest_epoch) -- NO cardano-cli, NO JSON consensus-input
# bundle, NO operator seed -- and EMITS a complete `NativeSnapshotNonUtxoState` (era, epoch, point, the
# FULL CertState, all five Praos nonces, the PoolDistr with VRF bindings, the Conway current protocol
# params, reserves/treasury, and nesBprev block production) + a commitment over ALL emitted fields. It
# does NOT assemble LedgerState/PraosChainDepState and does NOT persist (that is S1b). No silent default
# for any snapshot-present field; every fail mode is terminal.
set -euo pipefail

MOD="crates/ade_ledger/src/ledgerdb_state.rs"
HERM="crates/ade_ledger/tests/ledgerdb_nonutxo_hermetic.rs"
REAL="crates/ade_runtime/tests/ledgerdb_nonutxo_mithril.rs"
PPARAMS="crates/ade_ledger/src/pparams.rs"
SHELLEY="crates/ade_ledger/src/shelley.rs"
MARY="crates/ade_ledger/src/mary.rs"
fail() { echo "FAIL (ci_check_native_nonutxo_decode): $1" >&2; exit 1; }
for f in "$MOD" "$HERM" "$REAL" "$PPARAMS" "$SHELLEY" "$MARY"; do [ -f "$f" ] || fail "file $f missing"; done

# (A) The entry point takes ONLY the manifest-authoritative inputs (state_cbor, point,
# manifest_epoch, manifest_network_magic) -- no CLI / JSON bundle / seed.
grep -Eq "pub fn decode_native_nonutxo_state\(" "$MOD" || fail "entry point missing"
grep -Eq "state_cbor: &\[u8\]" "$MOD" || fail "entry point must take state_cbor: &[u8]"
grep -Eq "point: SeedPoint" "$MOD" || fail "entry point must take point: SeedPoint"
grep -Eq "manifest_epoch: u64" "$MOD" || fail "entry point must take manifest_epoch: u64"
grep -Eq "manifest_network_magic: u32" "$MOD" || fail "entry point must take manifest_network_magic: u32"

# (B) EMITS every required non-UTxO field (each a field of NativeSnapshotNonUtxoState).
for field in "pub era: CardanoEra" "pub network_id: u8" "pub epoch: EpochNo" "pub point: SeedPoint" \
             "pub cert_state: CertState" "pub praos_nonces: PraosNonces" \
             "pub pool_distr: BTreeMap<PoolId, \(u64, Hash32\)>" \
             "pub protocol_params: ProtocolParameters" "pub reserves: Coin" \
             "pub treasury: Coin" "pub block_production: BTreeMap<PoolId, u64>"; do
  grep -Eq "$field" "$MOD" || fail "emitted field missing: $field"
done
# era is EXPLICIT Conway (never inferred); telescope index gate is reused.
grep -Eq "era: CardanoEra::Conway" "$MOD" || fail "era must be EXPLICIT Conway"

# (C) Fail-closed: every terminal variant present (no default / partial emission).
for v in MalformedCbor UnsupportedEra EpochMismatch ZeroVrf PoolDistrVrfMismatch \
         AccountStateMissing BlockProductionMissing BlockProductionUnknownPool \
         ProtocolParamsMissing RoundTripMismatch; do
  grep -Eq "$v" "$MOD" || fail "fail-closed variant $v missing"
done

# (D) Protocol params are decoded FROM the Conway curPParams (esPp / GovState), not defaulted: the
# Conway-pinned 31-field PParams arity gate + the govState curPParams index must be present.
grep -Eq "CONWAY_PPARAMS_FIELDS" "$MOD" || fail "Conway PParams arity gate missing"
grep -Eq "CONWAY_GOV_STATE_CURPPARAMS_INDEX" "$MOD" || fail "curPParams index gate missing"
grep -Eq "read_conway_pparams\b" "$MOD" || fail "native Conway PParams decoder missing"

# (E) Reserves/treasury are decoded from esAccountState (treasury FIRST, cardano-ledger order); block
# production from nesBprev.
grep -Eq "fn read_account_state\(" "$MOD" || fail "esAccountState decoder missing"
grep -Eq "fn read_block_production\(" "$MOD" || fail "nesBprev decoder missing"

# (F) Coherence: PoolDistr<->CertState VRF cross-check + block-producer subset + CertState round-trip.
grep -Eq "PoolDistrVrfMismatch" "$MOD" || fail "PoolDistr<->CertState VRF cross-check missing"
grep -Eq "BlockProductionUnknownPool" "$MOD" || fail "block-producer subset check missing"
grep -Eq "decode_cert_state" "$MOD" || fail "CertState canonical round-trip self-check missing"

# (G) The commitment binds ALL emitted fields (single canonical encoder over every field).
grep -Eq "fn commit_native_nonutxo_state\(" "$MOD" || fail "commitment encoder missing"
grep -Eq "ade-native-nonutxo-state-commitment" "$MOD" || fail "commitment domain tag missing"

# (H) Scope fence: the decoder EMITS a structure; it does NOT assemble LedgerState/PraosChainDepState
# nor persist (S1b). The S1a additions must not construct those -- checked on CODE lines only (a
# doc comment declaring the fence is not a construction). Strip comment/blank lines first.
if awk '/S1a — native non-UTxO snapshot decoder/{f=1} f' "$MOD" \
   | grep -vE '^\s*//' \
   | grep -Eq "PraosChainDepState \{|encode_ledger_state\(|encode_snapshot\(|WalEntry|\.persist\(|ChainDb"; then
  fail "S1a must NOT assemble LedgerState/PraosChainDepState or persist (that is S1b)"
fi

# (I) BLUE determinism: no wall-clock / rand / env / HashMap USAGE in the decoder (the core-contract
# header comment naming prohibitions is not a usage).
if grep -Eq "std::env::|SystemTime::|Instant::now|rand::|HashMap::|HashMap<|HashSet::|HashSet<" "$MOD"; then
  fail "the BLUE decoder must be deterministic (no env / clock / rand / HashMap)"
fi

# (K) network_id is DERIVED from the manifest network magic (the SOLE network authority), not a
# placeholder: the `network_id = 0` literal placeholder is gone, the derive helper exists, and the
# derived id is set on the state + the protocol params + bound in the commitment. The
# reward-account nibble is DIAGNOSTIC ONLY — there is NO terminal network identity check.
if grep -Eq "network_id: 0\b" "$MOD"; then
  fail "network_id must be DERIVED from the manifest magic, not the literal placeholder (network_id: 0)"
fi
grep -Eq "fn network_id_from_magic\(" "$MOD" || fail "network-id derive helper missing"
grep -Eq "network_id: u8" "$MOD" || fail "network_id field must be on the emitted state"
if grep -Eq "NetworkIdentityMismatch" "$MOD"; then
  fail "network identity must NOT be a terminal check — the reward-account nibble is operator-controlled, diagnostic only"
fi
grep -Eq "enum RewardNibbleObservation" "$MOD" || fail "diagnostic RewardNibbleObservation missing"
grep -Eq "reward_nibble_observation" "$MOD" || fail "reward_nibble_observation diagnostic field missing on the emitted state"

# (L) coinsPerUTxOByte is preserved faithfully as a PER-BYTE rule and NEVER remapped onto the
# absolute floor: the native decoder builds MinUtxoRule::PerByte from coinsPerUTxOByte.
grep -Eq "MinUtxoRule::PerByte\(Coin\(nn_read_u64\(d, o, \"pp.coinsPerUTxOByte\"\)\?\)\)" "$MOD" \
  || fail "Conway coinsPerUTxOByte must decode to MinUtxoRule::PerByte (no absolute-floor remap)"
if grep -Eq "LegacyAbsoluteMin.*coinsPerUTxOByte|coinsPerUTxOByte.*LegacyAbsoluteMin" "$MOD"; then
  fail "Conway coinsPerUTxOByte must NOT populate LegacyAbsoluteMin"
fi

# (M) The shared ProtocolParameters carries the era-faithful MinUtxoRule; the old single-Coin
# `min_utxo_value` field is gone from the struct. (The distinct ProtocolParameterUpdate keeps
# an Option<Coin> proposal field, which is NOT the struct field.)
grep -Eq "pub enum MinUtxoRule" "$PPARAMS" || fail "MinUtxoRule enum missing"
grep -Eq "LegacyAbsoluteMin\(Coin\)" "$PPARAMS" || fail "MinUtxoRule::LegacyAbsoluteMin missing"
grep -Eq "PerByte\(Coin\)" "$PPARAMS" || fail "MinUtxoRule::PerByte missing"
grep -Eq "pub min_utxo_rule: MinUtxoRule" "$PPARAMS" || fail "ProtocolParameters.min_utxo_rule field missing"
# The ProtocolParameters struct body (between `pub struct ProtocolParameters {` and its close)
# must NOT carry a `min_utxo_value` field.
if awk '/pub struct ProtocolParameters \{/{f=1} f{print} f&&/^\}/{exit}' "$PPARAMS" \
   | grep -Eq "min_utxo_value"; then
  fail "ProtocolParameters must NOT keep a min_utxo_value field (replaced by min_utxo_rule)"
fi

# (N) The Conway min-UTxO VALIDATION is TERMINAL on a per-byte rule (no permissive absolute
# fallback): both the Shelley and Mary validators match MinUtxoRule and return the structured
# terminal UnsupportedConwayMinUtxoRule for PerByte.
grep -Eq "UnsupportedConwayMinUtxoRule" crates/ade_ledger/src/error.rs \
  || fail "UnsupportedConwayMinUtxoRule terminal error variant missing"
for v in "$SHELLEY" "$MARY"; do
  grep -Eq "MinUtxoRule::PerByte" "$v" || fail "$v: min-UTxO check must match MinUtxoRule::PerByte"
  grep -Eq "UnsupportedConwayMinUtxoRule" "$v" || fail "$v: per-byte rule must be terminal (UnsupportedConwayMinUtxoRule)"
  grep -Eq "MinUtxoRule::LegacyAbsoluteMin" "$v" || fail "$v: legacy absolute-min path must be explicit"
done

# (O) The new tests exist (network-id derive, reward-nibble diagnostic-not-terminal, per-byte decode,
# per-byte validation terminal, legacy regression).
grep -Eq "fn network_id_derived_from_manifest_magic\(" "$HERM" || fail "test network_id_derived_from_manifest_magic missing"
grep -Eq "fn reward_nibble_disagreement_is_diagnostic_not_terminal\(" "$HERM" || fail "test reward_nibble_disagreement_is_diagnostic_not_terminal missing"
grep -Eq "fn conway_pparams_decode_yields_per_byte_min_utxo_rule\(" "$HERM" || fail "test conway_pparams_decode_yields_per_byte_min_utxo_rule missing"
grep -Eq "fn mary_min_utxo_per_byte_rule_is_terminal_not_permissive\(" "$MARY" || fail "test mary_min_utxo_per_byte_rule_is_terminal_not_permissive missing"
grep -Eq "fn mary_min_utxo_legacy_absolute_min_unchanged\(" "$MARY" || fail "test mary_min_utxo_legacy_absolute_min_unchanged missing"

# (P) The hermetic tests pass (determinism + commitment-binds-every-field + fail-closed
# negatives + network-id derive/mismatch + per-byte preservation).
cargo test -p ade_ledger --test ledgerdb_nonutxo_hermetic --quiet >/dev/null 2>&1 \
  || fail "hermetic decoder tests failed (run: cargo test -p ade_ledger --test ledgerdb_nonutxo_hermetic)"
# The Conway min-UTxO validation tests pass (per-byte terminal + legacy regression).
cargo test -p ade_ledger --lib mary_min_utxo --quiet >/dev/null 2>&1 \
  || fail "Conway min-UTxO validation tests failed (run: cargo test -p ade_ledger --lib mary_min_utxo)"

echo "OK: native non-UTxO snapshot decoder emits all fields + fail-closed + commitment-bound + coherent + deterministic; network_id derived+bound (reward nibble diagnostic only, no terminal network check); coinsPerUTxOByte preserved as PerByte (no absolute-floor remap); Conway min-UTxO validation terminal (MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1a)"
