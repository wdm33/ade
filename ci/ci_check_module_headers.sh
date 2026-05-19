#!/usr/bin/env bash
set -euo pipefail

# Verify contract header present as first line of every .rs source file
# in BLUE scope. Six BLUE crates scan their full src/ tree; ade_network
# is BLUE per-submodule (mux::frame plus 8 placeholder modules) — its
# RED siblings (mux::transport, session) and GREEN siblings (lib,
# mux::mod) are excluded by listing the BLUE paths explicitly.

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
EXPECTED_FIRST_LINE="// Core Contract:"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        echo "FAIL: Source directory not found: $SRC_DIR"
        FAILED=1
        continue
    fi

    while IFS= read -r -d '' file; do
        first_line=$(head -n 1 "$file")
        if [ "$first_line" != "$EXPECTED_FIRST_LINE" ]; then
            echo "FAIL: Missing contract header in $file"
            echo "  Expected: $EXPECTED_FIRST_LINE"
            echo "  Got:      $first_line"
            FAILED=1
        fi
    done < <(find "$SRC_DIR" -name '*.rs' -print0)
done

for path in "${ADE_NETWORK_BLUE_PATHS[@]}"; do
    FULL_PATH="$REPO_ROOT/$path"
    if [ ! -e "$FULL_PATH" ]; then
        echo "FAIL: BLUE path not found: $FULL_PATH"
        FAILED=1
        continue
    fi

    while IFS= read -r -d '' file; do
        first_line=$(head -n 1 "$file")
        if [ "$first_line" != "$EXPECTED_FIRST_LINE" ]; then
            echo "FAIL: Missing contract header in $file"
            echo "  Expected: $EXPECTED_FIRST_LINE"
            echo "  Got:      $first_line"
            FAILED=1
        fi
    done < <(find "$FULL_PATH" -name '*.rs' -print0)
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: All BLUE crate source files have contract headers"
    exit 0
else
    exit 1
fi
