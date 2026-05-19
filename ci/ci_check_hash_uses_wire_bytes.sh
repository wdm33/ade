#!/usr/bin/env bash
set -euo pipefail

# Verify that all hash-computation paths use .wire_bytes(), never
# .canonical_bytes() or manual re-encoding.
#
# Pattern scan on BLUE crate source (not tests).
# Invariants: DC-CBOR-02, T-ENC-01

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core" "ade_ledger" "ade_plutus")
ADE_NETWORK_BLUE_PATHS=(
    "crates/ade_network/src/mux/frame.rs"
    "crates/ade_network/src/codec"
    "crates/ade_network/src/handshake"
    "crates/ade_network/src/chain_sync"
    "crates/ade_network/src/block_fetch"
    "crates/ade_network/src/tx_submission"
    "crates/ade_network/src/keep_alive"
    "crates/ade_network/src/peer_sharing"
    "crates/ade_network/src/n2c"
)

FAILED=0

scan_for_hash_misuse() {
    local target="$1"
    local label="$2"

    local matches
    matches=$(grep -rn 'canonical_bytes.*hash\|hash.*canonical_bytes' "$target" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: Hash computation using canonical_bytes in $label:"
        echo "$matches"
        FAILED=1
    fi

    matches=$(grep -rn 'ade_encode.*hash\|hash.*ade_encode' "$target" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' || true)

    if [ -n "$matches" ]; then
        echo "FAIL: Hash computation using re-encoding in $label:"
        echo "$matches"
        FAILED=1
    fi
}

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi
    scan_for_hash_misuse "$SRC_DIR" "$crate"
done

for path in "${ADE_NETWORK_BLUE_PATHS[@]}"; do
    FULL_PATH="$REPO_ROOT/$path"
    if [ ! -e "$FULL_PATH" ]; then
        continue
    fi
    scan_for_hash_misuse "$FULL_PATH" "ade_network path $path"
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No hash paths use canonical_bytes or re-encoding in BLUE crates"
    exit 0
else
    exit 1
fi
