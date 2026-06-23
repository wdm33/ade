#!/usr/bin/env bash
# ci_check_value_quantity_domain.sh -- LEDGER-VALUE-QUANTITY-CORRECTNESS S1 (DC-LEDGER-VALUE-01).
#
# Ade's authoritative UTxO OUTPUT asset quantity preserves the full non-negative Cardano Word64
# domain (0 ..= 2^64-1) via the `OutputAssetQuantity(u64)` newtype. Output arithmetic is CHECKED
# (overflow/underflow -> a structured `LedgerError`, never a wrap and never a negative). Mint/burn is
# the DISTINCT signed `MintBurnQuantity(i64)` and can never enter an output bundle. Representable
# values (<= i64::MAX) stay byte-identical (a non-negative CBOR int and a u64 <= i64::MAX encode to
# the same major-0 uint). Cross-ref DC-MITHRIL-02 (the snapshot decoder already yields faithful u64).
set -euo pipefail

TYPES="crates/ade_types/src/mary/value.rs"
LVAL="crates/ade_ledger/src/value.rs"
SNAP="crates/ade_ledger/src/snapshot/utxo_state.rs"
FPR="crates/ade_ledger/src/fingerprint.rs"
ERR="crates/ade_ledger/src/error.rs"

fail() { echo "FAIL (ci_check_value_quantity_domain): $1" >&2; exit 1; }
for f in "$TYPES" "$LVAL" "$SNAP" "$FPR" "$ERR"; do
  [ -f "$f" ] || fail "module $f missing"
done

# (A) The newtypes are defined in the shared value-types layer, as newtypes (not aliases).
grep -Eq "pub struct OutputAssetQuantity\(pub u64\)" "$TYPES" \
  || fail "OutputAssetQuantity(u64) newtype must be defined in $TYPES"
grep -Eq "pub struct MintBurnQuantity\(pub i64\)" "$TYPES" \
  || fail "MintBurnQuantity(i64) newtype must be defined in $TYPES"

# (B) BOTH MultiAsset defs hold OutputAssetQuantity as the quantity; NO `AssetName, i64` survives.
grep -Eq "BTreeMap<AssetName, OutputAssetQuantity>" "$TYPES" \
  || fail "$TYPES MultiAsset must hold OutputAssetQuantity"
grep -Eq "BTreeMap<AssetName, OutputAssetQuantity>" "$LVAL" \
  || fail "$LVAL MultiAsset must hold OutputAssetQuantity"
if grep -Eq "BTreeMap<AssetName, i64>" "$TYPES" "$LVAL"; then
  fail "an output MultiAsset still holds a signed (i64) quantity -- migrate to OutputAssetQuantity"
fi

# (C) ade_ledger REUSES the ade_types newtype (it must NOT define its own).
grep -Eq "use ade_types::mary::value::OutputAssetQuantity" "$LVAL" \
  || fail "$LVAL must import OutputAssetQuantity from ade_types (no duplicate definition)"
if grep -Eq "struct OutputAssetQuantity" "$LVAL"; then
  fail "$LVAL must NOT define its own OutputAssetQuantity"
fi

# (D) Output arithmetic is CHECKED: the underflow variant exists and multi_asset_sub uses checked_sub
# routed to it; multi_asset_add uses checked_add. No unchecked `-=` on the quantity remains.
grep -Eq "AssetUnderflow\(AssetUnderflowError\)" "$ERR" \
  || fail "the structured AssetUnderflow LedgerError variant is missing"
grep -Eq "checked_sub.*AssetUnderflow|AssetUnderflow" "$LVAL" \
  || fail "$LVAL multi_asset_sub must route an output underflow to AssetUnderflow"
grep -Eq "current\.checked_sub" "$LVAL" \
  || fail "$LVAL multi_asset_sub must use checked_sub (no unchecked subtraction)"
grep -Eq "current\.checked_add" "$LVAL" \
  || fail "$LVAL multi_asset_add must use checked_add"
if grep -Eq '\*current[[:space:]]*-=' "$LVAL"; then
  fail "$LVAL still has an UNCHECKED `*current -= qty` on the output quantity path"
fi

# (E) NO truncating cast on the quantity value path. The migrated arithmetic in value.rs production
# code must contain no `as i64`/`as u64` quantity cast (length casts live elsewhere; the only allowed
# `i64::MAX as u64` forms are test-fixture boundary CONSTRUCTORS, never on the live path). Enforce by
# requiring the arithmetic helpers carry no cast: extract the non-test span and check it.
PROD_VALUE="$(sed '/#\[cfg(test)\]/,$d' "$LVAL")"
if grep -Eq "as i64|as u64" <<<"$PROD_VALUE"; then
  fail "$LVAL production code must not cast the quantity path (no `as i64`/`as u64`)"
fi
# The snapshot OUTPUT quantity is read with the dedicated non-negative reader, not a signed i64 path.
grep -Eq "read_output_quantity" "$SNAP" \
  || fail "$SNAP must read the output quantity via the non-negative read_output_quantity"
if grep -Eq "fn read_int_i64|fn write_int_i64" "$SNAP"; then
  fail "$SNAP must not keep the signed i64 helpers on the OUTPUT quantity path"
fi

# (F) MintBurnQuantity is DORMANT: it is never used as a MultiAsset map value type (it cannot enter
# an output bundle). A `BTreeMap<..., MintBurnQuantity>` anywhere would violate the boundary.
if grep -rEq "BTreeMap<[^>]*MintBurnQuantity>|AssetName, MintBurnQuantity" crates/; then
  fail "MintBurnQuantity must NOT be used as a MultiAsset/output map value type (it is signed/dormant)"
fi

# (G) BYTE-IDENTITY: the output quantity is written as a canonical CBOR unsigned int on every
# authoritative encode path (snapshot + fingerprint), so a representable value's bytes are unchanged.
grep -Eq "write_uint_canonical\(buf, qty\.0\)" "$SNAP" \
  || fail "$SNAP write_multi_asset must encode the quantity as a CBOR unsigned int (qty.0)"
grep -Eq "write_uint_canonical\(buf, qty\.0\)" "$FPR" \
  || fail "$FPR write_multi_asset must encode the quantity as a CBOR unsigned int (qty.0)"

# (H) the hermetic value + snapshot tests pass (the proof: Word64 round-trip, underflow/overflow ->
# structured error, byte-identity golden, Stage-2 MemPack u64 -> snapshot -> recovery).
cargo test -p ade_types -p ade_ledger --lib >/dev/null 2>&1 \
  || fail "hermetic ade_types + ade_ledger lib tests failed"

echo "PASS (ci_check_value_quantity_domain): output asset quantity is the checked, byte-identical Word64 domain."
