#!/usr/bin/env bash
set -euo pipefail

# Grep BLUE crate src/ dirs for forbidden nondeterministic patterns (T-CORE-02).
# Excludes comment lines and deny attribute lines.
#
# ade_plutus is BLUE by declaration (pure-functional UPLC wrapper) and
# is included in the scan. Its source must also respect these
# constraints. Aiken's transitive use of `indexmap` stays inside the
# aiken crate tree (not in our sources) and therefore is not flagged.

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core" "ade_ledger" "ade_plutus")

FORBIDDEN_PATTERNS=(
    "HashMap"
    "HashSet"
    "IndexMap"
    "IndexSet"
    "indexmap::"
    "SystemTime"
    "Instant"
    "std::fs"
    "std::net"
    "tokio"
    "async fn"
    '\bf32\b'
    '\bf64\b'
    "anyhow"
    "rand::thread_rng"
    "thread::spawn"
)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
        # Search .rs files, exclude comment-only lines and deny attribute lines
        # After grep -rn, lines look like "path:lineno:content" so we match // after the line number
        matches=$(grep -rn "$pattern" "$SRC_DIR" --include='*.rs' 2>/dev/null | \
            grep -v ':[0-9]*:\s*//' | \
            grep -v '#!\[deny' | \
            grep -v '#\[deny' || true)

        if [ -n "$matches" ]; then
            echo "FAIL: Forbidden pattern '$pattern' found in $crate:"
            echo "$matches"
            FAILED=1
        fi
    done
done

# Check for unsafe code in BLUE crates with documented allowlist.
# Constitutional exception: VRF FFI in ade_crypto/src/vrf.rs (Slice 2A-3).
UNSAFE_ALLOWLIST=(
    "crates/ade_crypto/src/vrf.rs"
)

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    # Find unsafe usage excluding deny attributes and comments
    unsafe_matches=$(grep -rn 'unsafe' "$SRC_DIR" --include='*.rs' 2>/dev/null | \
        grep -v ':[0-9]*:\s*//' | \
        grep -v '#!\[deny(unsafe_code)' | \
        grep -v '#\[allow(unsafe_code)' || true)

    if [ -n "$unsafe_matches" ]; then
        # Check each match against the allowlist
        while IFS= read -r line; do
            allowed=0
            for entry in "${UNSAFE_ALLOWLIST[@]}"; do
                if echo "$line" | grep -q "$entry"; then
                    allowed=1
                    break
                fi
            done
            if [ "$allowed" -eq 0 ]; then
                echo "FAIL: Unsafe code found outside allowlist in $crate:"
                echo "$line"
                FAILED=1
            fi
        done <<< "$unsafe_matches"
    fi
done

# PHASE4-B4-S3/S4 (CE-B4-4, DC-LEDGER-08): the cert-state accumulation path must
# stay fail-closed. The removed swallow carried the rationale "non-fatal during
# replay"; that justification was false (the path runs only at track_utxo, i.e.
# with full state present). Fail if it — or a bare Err(_) swallow arm in the
# cert-state accumulation function — reappears.
SWALLOW_MATCHES=$(grep -rn "non-fatal during replay" "$REPO_ROOT/crates" --include='*.rs' 2>/dev/null || true)
if [ -n "$SWALLOW_MATCHES" ]; then
    echo "FAIL: cert-state fail-open rationale 'non-fatal during replay' reintroduced (DC-LEDGER-08):"
    echo "$SWALLOW_MATCHES"
    FAILED=1
fi
ACC_BODY=$(awk '/fn accumulate_tx_certs/,/^}/' "$REPO_ROOT/crates/ade_ledger/src/rules.rs" 2>/dev/null || true)
if echo "$ACC_BODY" | grep -qE 'Err\(_\)\s*=>'; then
    echo "FAIL: accumulate_tx_certs contains an Err(_) swallow arm (DC-LEDGER-08 fail-closed)"
    FAILED=1
fi

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No forbidden patterns in BLUE crates"
    exit 0
else
    exit 1
fi
