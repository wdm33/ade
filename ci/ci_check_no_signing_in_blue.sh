#!/usr/bin/env bash
set -euo pipefail

# Grep BLUE crate src/ for signing/private-key patterns (CE-05/T-KEY-01).
# Signing operations (key custody) must be confined to the RED shell.
# Verification is permitted in BLUE; signing is not.
#
# Scope: PRODUCTION code only — line comments (// /// //!) + each file's
# `#[cfg(test)]` module are stripped (test fixtures legitimately build a signed
# input to feed the BLUE *verifier*; e.g. opcert_validate.rs / forge.rs tests).
# The BLUE Sum6KES *algorithm* (crates/ade_crypto/src/kes_sum/, PHASE4-N-P) is
# ALLOW-LISTED: it is deterministic crypto defining the KES SigningKey TYPES +
# the pure sign/update algorithm — NOT key custody (custody is RED ade_runtime,
# OP-OPS-04). kes_sum is covered by ci_check_private_key_custody.sh +
# ci_check_kes_sum_compatibility.sh.

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

# Emit production lines of a .rs file: strip line comments (// /// //!) and stop
# at the first `#[cfg(test)]` module opener, so the scan sees only production
# code (a SigningKey in a doc comment or a signed-fixture test is not a custody
# leak).
emit_production_lines() {
    awk '
        /^[[:space:]]*#\[cfg\(test\)\]/ { exit }
        { line=$0; sub(/\/\/.*$/, "", line); print NR ":" line }
    ' "$1"
}

scan_for_signing() {
    local target="$1"
    local label="$2"

    while IFS= read -r -d '' rs; do
        # Allow-list the BLUE Sum6KES algorithm (deterministic crypto, not custody).
        case "$rs" in
            */ade_crypto/src/kes_sum/*) continue ;;
        esac
        local prod
        prod=$(emit_production_lines "$rs")
        for pattern in "${SIGNING_PATTERNS[@]}"; do
            local matches
            matches=$(echo "$prod" | grep -E "$pattern" || true)
            if [ -n "$matches" ]; then
                echo "FAIL: Signing pattern '$pattern' found in BLUE $label ($rs):"
                echo "$matches"
                FAILED=1
            fi
        done
    done < <(find "$target" -name '*.rs' -print0 2>/dev/null)
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
