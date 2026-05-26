#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-A1.1 — seed importer full-preprod-support gate.
#
# Asserts that the seed importer carries the structural pieces
# needed to import a full cardano-cli `query utxo --whole-utxo`
# preprod dump:
#
#   1. `encode_script_ref` is the canonical authority for the
#      Babbage `script_ref` field — exactly one definition,
#      living in the GREEN seed importer.
#   2. The closed-vocabulary `script_variant_tag` discriminator
#      handles the four cardano-node-supported variants
#      (SimpleScript / PlutusScriptV1 / PlutusScriptV2 /
#      PlutusScriptV3).
#   3. `decode_cli_address` accepts BOTH the bech32-Shelley path
#      AND the Base58-Byron path (no silent partial-seed on
#      Byron-era entries).
#   4. The old `UnsupportedTxOutFeature { feature: "referenceScript" }`
#      fail-fast literal is GONE from the importer's success path
#      (preserving the closed `UnsupportedTxOutFeature` variant
#      itself for future genuine unsupported features is OK; we
#      only reject the literal reference-script fail-fast in the
#      production codepath).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

TARGET="$REPO_ROOT/crates/ade_runtime/src/seed_import/importer.rs"
JSON_TARGET="$REPO_ROOT/crates/ade_runtime/src/seed_import/json.rs"

if [[ ! -f "$TARGET" ]]; then
    print_fail "seed_import importer.rs missing: $TARGET"
    exit $FAILED
fi
if [[ ! -f "$JSON_TARGET" ]]; then
    print_fail "seed_import json.rs missing: $JSON_TARGET"
    exit $FAILED
fi

# (1) encode_script_ref: exactly one definition, in importer.rs.
n_enc=$(grep -cE '^fn encode_script_ref\b|^pub(\(crate\))? fn encode_script_ref\b' "$TARGET")
if (( n_enc != 1 )); then
    print_fail "expected exactly 1 `fn encode_script_ref` in importer.rs, found $n_enc"
fi

# Also confirm no second copy elsewhere in the workspace.
others=$(grep -rlE '^fn encode_script_ref\b|^pub(\(crate\))? fn encode_script_ref\b' "$REPO_ROOT/crates" 2>/dev/null | grep -v "$TARGET" || true)
if [[ -n "$others" ]]; then
    print_fail "second encode_script_ref definition in:"
    echo "$others" | sed 's/^/  /'
fi

# (2) Closed-vocabulary script-variant-tag check: SimpleScript,
# PlutusScriptV1, PlutusScriptV2, PlutusScriptV3 must all appear
# as match arms in the same file.
for v in 'SimpleScript' 'PlutusScriptV1' 'PlutusScriptV2' 'PlutusScriptV3'; do
    if ! grep -qE "\"$v\"" "$TARGET"; then
        print_fail "missing script-variant match arm: \"$v\""
    fi
done

# (3) decode_cli_address dispatches to both bech32 and Byron Base58.
if ! grep -qE 'fn decode_cli_address\b' "$TARGET"; then
    print_fail "decode_cli_address authority missing from importer.rs"
fi
if ! grep -qE 'fn decode_bech32_address\b' "$TARGET"; then
    print_fail "decode_bech32_address sub-authority missing from importer.rs"
fi
if ! grep -qE 'fn decode_byron_base58_address\b' "$TARGET"; then
    print_fail "decode_byron_base58_address sub-authority missing from importer.rs"
fi

# (4) The old reference-script fail-fast literal is gone from the
# production codepath. The variant `UnsupportedTxOutFeature` may
# still exist as a closed-sum member; we only reject the literal
# `feature: "referenceScript"` use.
if grep -qE 'UnsupportedTxOutFeature \{[^}]*feature: *"referenceScript"' "$TARGET"; then
    # Allow the literal inside a comment line; strip comments
    # before checking.
    if awk '{ sub(/\/\/.*$/, ""); print }' "$TARGET" \
       | grep -qE 'UnsupportedTxOutFeature \{[^}]*feature: *"referenceScript"'; then
        print_fail "found legacy reference-script fail-fast in importer.rs (must be removed under A1.1)"
    fi
fi

# (5) Sanity: the new BadReferenceScript closed-sum variant
# exists in the JsonSeedError enum.
if ! grep -qE 'BadReferenceScript' "$TARGET"; then
    print_fail "BadReferenceScript variant missing from JsonSeedError"
fi

if (( FAILED == 0 )); then
    echo "OK: full-preprod-support seed importer gates hold (A1.1 + A1.2)"
fi
exit $FAILED
