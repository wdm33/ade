#!/usr/bin/env bash
set -euo pipefail

# Verify that all hash-computation paths use .wire_bytes(), never
# .canonical_bytes() or manual re-encoding.
#
# Pattern scan on BLUE crate source (not tests).
# Invariants: DC-CBOR-02, T-ENC-01

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core")

FAILED=0

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    # Check for hash computation using canonical_bytes (forbidden)
    # Pattern: any call to canonical_bytes followed by hash-related function
    matches=$(grep -rn 'canonical_bytes.*hash\|hash.*canonical_bytes' "$SRC_DIR" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: Hash computation using canonical_bytes in $crate:"
        echo "$matches"
        FAILED=1
    fi

    # Check for re-encoding used in hash paths (forbidden)
    # Pattern: ade_encode followed by hash computation
    matches=$(grep -rn 'ade_encode.*hash\|hash.*ade_encode' "$SRC_DIR" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: Hash computation using re-encoding in $crate:"
        echo "$matches"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No hash paths use canonical_bytes or re-encoding in BLUE crates"
    exit 0
else
    exit 1
fi
