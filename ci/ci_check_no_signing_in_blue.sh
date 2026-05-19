#!/usr/bin/env bash
set -euo pipefail

# Grep BLUE crate src/ for signing/private-key patterns (CE-05/T-KEY-01).
# Signing operations must be confined to the RED shell.
# Verification is permitted in BLUE; signing is not.

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

SIGNING_PATTERNS=(
    "SigningKey"
    "SecretKey"
    "PrivateKey"
    "private_key"
    "sign_message"
    "sign_block"
)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

scan_for_signing() {
    local target="$1"
    local label="$2"

    for pattern in "${SIGNING_PATTERNS[@]}"; do
        local matches
        matches=$(grep -rn "$pattern" "$target" --include='*.rs' 2>/dev/null | \
            grep -v '^\s*//' || true)

        if [ -n "$matches" ]; then
            echo "FAIL: Signing pattern '$pattern' found in BLUE $label:"
            echo "$matches"
            FAILED=1
        fi
    done
}

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi
    scan_for_signing "$SRC_DIR" "crate $crate"
done

for path in "${ADE_NETWORK_BLUE_PATHS[@]}"; do
    FULL_PATH="$REPO_ROOT/$path"
    if [ ! -e "$FULL_PATH" ]; then
        continue
    fi
    scan_for_signing "$FULL_PATH" "ade_network path $path"
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No signing patterns in BLUE crates"
    exit 0
else
    exit 1
fi
