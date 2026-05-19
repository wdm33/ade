#!/usr/bin/env bash
set -euo pipefail

# Grep BLUE crate src/ for semantic cfg attributes (T-BUILD-01).
# Feature flags in authoritative code could alter semantics per build profile.

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

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

scan_for_feature_cfg() {
    local target="$1"
    local label="$2"

    local matches
    matches=$(grep -rn '#\[cfg(feature' "$target" --include='*.rs' 2>/dev/null || true)
    if [ -n "$matches" ]; then
        echo "FAIL: Feature cfg found in BLUE $label:"
        echo "$matches"
        FAILED=1
    fi

    matches=$(grep -rn 'cfg!(feature' "$target" --include='*.rs' 2>/dev/null || true)
    if [ -n "$matches" ]; then
        echo "FAIL: Feature cfg! found in BLUE $label:"
        echo "$matches"
        FAILED=1
    fi
}

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi
    scan_for_feature_cfg "$SRC_DIR" "crate $crate"
done

for path in "${ADE_NETWORK_BLUE_PATHS[@]}"; do
    FULL_PATH="$REPO_ROOT/$path"
    if [ ! -e "$FULL_PATH" ]; then
        continue
    fi
    scan_for_feature_cfg "$FULL_PATH" "ade_network path $path"
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No semantic cfg attributes in BLUE crates"
    exit 0
else
    exit 1
fi
