#!/usr/bin/env bash
set -euo pipefail

# Verify that all CBOR decode flows go through named chokepoints.
# Pattern scan: no raw CBOR decoding outside named chokepoint functions.
#
# Named chokepoints (grows with each era slice):
#   - decode_block_envelope
#   - decode_byron_ebb_block, decode_byron_regular_block
#   - decode_shelley_block, decode_allegra_block, decode_mary_block
#   - decode_alonzo_block, decode_babbage_block, decode_conway_block
#   - decode_address
#
# Invariants: DC-INGRESS-01, T-INGRESS-01

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core")

FAILED=0

# Check 1: PreservedCbor::new must only appear in ade_codec (pub(crate) enforced by compiler,
# but verify no leakage through re-export or unsafe workarounds)
for crate in "${BLUE_CRATES[@]}"; do
    if [ "$crate" = "ade_codec" ]; then
        continue
    fi

    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    matches=$(grep -rn 'PreservedCbor::new\|PreservedCbor\s*{' "$SRC_DIR" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: PreservedCbor construction outside ade_codec in $crate:"
        echo "$matches"
        FAILED=1
    fi
done

# Check 2: Verify that decode_block_envelope exists as a named chokepoint
ENVELOPE_FILE="$REPO_ROOT/crates/ade_codec/src/cbor/envelope.rs"
if [ ! -f "$ENVELOPE_FILE" ]; then
    echo "FAIL: Missing envelope chokepoint file: $ENVELOPE_FILE"
    FAILED=1
else
    if ! grep -q 'pub fn decode_block_envelope' "$ENVELOPE_FILE"; then
        echo "FAIL: decode_block_envelope function not found in envelope.rs"
        FAILED=1
    fi
fi

# Check 3: No direct minicbor::decode or raw CBOR parsing outside ade_codec
for crate in "${BLUE_CRATES[@]}"; do
    if [ "$crate" = "ade_codec" ]; then
        continue
    fi

    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    matches=$(grep -rn 'minicbor::decode\|minicbor::Decode\|from_cbor\|cbor_decode' "$SRC_DIR" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: Raw CBOR decoding outside ade_codec in $crate:"
        echo "$matches"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: All decode flows through named chokepoints"
    exit 0
else
    exit 1
fi
