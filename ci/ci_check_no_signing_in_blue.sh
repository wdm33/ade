#!/usr/bin/env bash
set -euo pipefail

# Grep BLUE crate src/ for signing/private-key patterns (CE-05/T-KEY-01).
# Signing operations must be confined to the RED shell.
# Verification is permitted in BLUE; signing is not.

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core")

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

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    for pattern in "${SIGNING_PATTERNS[@]}"; do
        matches=$(grep -rn "$pattern" "$SRC_DIR" --include='*.rs' 2>/dev/null | \
            grep -v '^\s*//' || true)

        if [ -n "$matches" ]; then
            echo "FAIL: Signing pattern '$pattern' found in BLUE crate $crate:"
            echo "$matches"
            FAILED=1
        fi
    done
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No signing patterns in BLUE crates"
    exit 0
else
    exit 1
fi
