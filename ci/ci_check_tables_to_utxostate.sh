#!/usr/bin/env bash
# ci_check_tables_to_utxostate.sh -- MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1c.
#
# The Stage-2 `tables` (MemPack-decoded TxOuts) materialize into Ade's authoritative `UTxOState`: each
# `DecodedTxOut` -> ledger `TxOut` with the hash-critical inline-datum / reference-script bytes
# PRESERVED VERBATIM (CBOR tag-24), u64 output quantities carried through with NO i64 conversion, in
# canonical TxIn order, fail-closed on any unsupported form. The materialized UTxO commitment
# (fingerprint_utxo_v2) is bound to the SAME manifest point + the Stage-1 non-UTxO commitment + the
# Stage-2 tables commitment. Registry: DC-MITHRIL-06.
set -euo pipefail

MOD="crates/ade_ledger/src/mithril_utxo_materialize.rs"
REAL="crates/ade_runtime/tests/mithril_tables_to_utxostate.rs"
fail() { echo "FAIL (ci_check_tables_to_utxostate): $1" >&2; exit 1; }
for f in "$MOD" "$REAL"; do [ -f "$f" ] || fail "file $f missing"; done

# The production body with the cfg(test) module stripped (negative greps must not match test fixtures).
# Comment lines are also stripped so the negative-control greps only see executable code, not the
# doc-comments that legitimately describe the prohibition ("never truncated / saturated / i64-cast").
PROD="$(awk '/^#\[cfg\(test\)\]/{exit} {print}' "$MOD" | grep -vE '^\s*(//|\*)')"

# (A) The pure converter exists with the closed error enum.
grep -Eq "pub fn decoded_txout_to_ledger\(o: DecodedTxOut\) -> R<TxOut>" "$MOD" \
  || fail "converter decoded_txout_to_ledger(DecodedTxOut) -> Result<TxOut, _> missing"
grep -Eq "pub enum TxOutMaterializeError" "$MOD" || fail "closed TxOutMaterializeError enum missing"

# (B) FAITHFUL Word64: each Stage-2 u64 quantity wraps into OutputAssetQuantity -- NO i64 cast/truncate.
grep -Eq "OutputAssetQuantity\(\*qty\)" "$MOD" || fail "u64 quantity must wrap into OutputAssetQuantity (faithful Word64)"
if printf '%s' "$PROD" | grep -Eq "as i64|i64::try_from|saturating|truncate"; then
  fail "the converter must NOT i64-cast / saturate / truncate output quantities (faithful Word64)"
fi

# (C) The datum/script bytes are embedded VERBATIM via the single tag-24 authority (wrap_tag24), NEVER
# a re-encode. The converter must route the inline-datum + script bytes through wrap_tag24.
grep -Eq "use ade_codec::wrap_tag24" "$MOD" || fail "must reuse the ade_codec wrap_tag24 tag-24 authority"
grep -Eq "wrap_tag24\(bytes\)" "$MOD" || fail "inline-datum bytes must be embedded verbatim via wrap_tag24"
grep -Eq "wrap_tag24\(&inner\)" "$MOD" || fail "reference-script bytes must be embedded verbatim via wrap_tag24"
# No hand-rolled re-encode of the preserved bytes (a second CBOR parse of the inline datum / script).
if printf '%s' "$PROD" | grep -Eq "decode_plutus_data|reencode|re_encode"; then
  fail "the preserved inline-datum / script bytes must NEVER be re-decoded/re-encoded"
fi

# (D) Canonical Conway TxOut raw map -- keys ascending 0 (address), 1 (value), 2 (datum), 3 (script),
# built via the shared canonical cbor primitives (not a forked encoder).
grep -Eq "pub fn encode_conway_txout_raw\(o: &DecodedTxOut\) -> Vec<u8>" "$MOD" || fail "canonical raw encoder missing"
grep -Eq "write_map_header|write_uint_canonical|write_bytes_canonical|write_array_header" "$MOD" \
  || fail "the raw map must use the shared ade_codec cbor primitives"

# (E) AlonzoPlus for datum/script; ShelleyMary/Byron for pure-payment.
grep -Eq "TxOut::AlonzoPlus" "$MOD" || fail "datum/script outputs must materialize to TxOut::AlonzoPlus"
grep -Eq "TxOut::ShelleyMary" "$MOD" || fail "pure-payment outputs must materialize to TxOut::ShelleyMary"
grep -Eq "TxOut::Byron" "$MOD" || fail "Byron-header outputs must materialize to TxOut::Byron"

# (F) Materialization: era-bound to Conway, canonical ASCENDING TxIn order (terminal otherwise), the
# 34-byte TxIn key parse, fail-closed, accumulating BTreeMap<TxIn, TxOut> -> UTxOState::from_map.
grep -Eq "pub fn materialize_tables_to_utxo\(" "$MOD" || fail "materialization entry missing"
grep -Eq "CONWAY_ERA_INDEX" "$MOD" || fail "Conway era gate missing"
grep -Eq "NonAscendingTxIn" "$MOD" || fail "non-ascending TxIn terminal missing"
grep -Eq "BadTxInKey" "$MOD" || fail "34-byte TxIn key parse terminal missing"
grep -Eq "UTxOState::from_map" "$MOD" || fail "must accumulate into UTxOState::from_map"

# (G) The commitment binding: fingerprint_utxo_v2 over the materialized UTxO, bound to the manifest
# point + Stage-1 + Stage-2 commitments, TERMINAL on mismatch.
grep -Eq "fingerprint_utxo_v2" "$MOD" || fail "the UTxO authority must bind via fingerprint_utxo_v2"
grep -Eq "pub fn bind_utxo_to_manifest\(" "$MOD" || fail "bind_utxo_to_manifest missing"
grep -Eq "pub fn verify_utxo_binding\(" "$MOD" || fail "verify_utxo_binding (terminal on mismatch) missing"
grep -Eq "manifest_point_hash|stage1_nonutxo_commitment|stage2_tables_commitment" "$MOD" \
  || fail "the binding must cover the manifest point + Stage-1 + Stage-2 commitments"

# (H) hermetic tests pass (the converter, materialization, byte-preservation, round-trip, binding,
# persist->recover, u64 > i64::MAX).
cargo test -p ade_ledger --lib mithril_utxo_materialize --quiet >/dev/null 2>&1 \
  || fail "hermetic S1c tests failed (cargo test -p ade_ledger --lib mithril_utxo_materialize)"

echo "OK: tables -> authoritative UTxOState materialization is faithful-u64 + byte-preserving (tag-24) + canonical-ordered + fail-closed + manifest-bound (MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1c)"
