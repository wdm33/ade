#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-C S3 — replay corpus must carry no private-key material.
#
# Mechanical guards (closure proof for DC-CONS-14 / DC-CRYPTO-03):
#
#   1. No `*.skey` / `*.sk` / `*.signingkey` files under
#      crates/ade_testkit/fixtures/producer/.
#   2. No `VrfSigningKey` / `KesSecret` / `KesSigningKey` /
#      `ColdSigningKey` literal under
#      crates/ade_testkit/src/producer/fixtures.rs.
#   3. `ProducerTick` has no `Serialize` impl that mentions any
#      private-key field name. (Currently no serde impl on
#      ProducerTick — the gate is the stays-this-way lock.)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FIXTURES_DIR="$REPO_ROOT/crates/ade_testkit/fixtures/producer"
FIXTURES_RS="$REPO_ROOT/crates/ade_testkit/src/producer/fixtures.rs"
STATE_RS="$REPO_ROOT/crates/ade_ledger/src/producer/state.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

emit_production_lines() {
    local f="$1"
    awk '
        /^#\[cfg\(test\)\]/ { exit }
        {
            line=$0
            sub(/\/\/.*$/, "", line)
            print NR ":" line
        }
    ' "$f"
}

# ---------------------------------------------------------------------------
# Guard 1 — no .skey / .sk / .signingkey files under producer fixtures dir.
# (The directory may not exist if all fixtures are compiled in as Rust
# source; absence is acceptable, presence with offending files is not.)
# ---------------------------------------------------------------------------
if [ -d "$FIXTURES_DIR" ]; then
    bad_files=$(find "$FIXTURES_DIR" \( -name '*.skey' -o -name '*.sk' -o -name '*.signingkey' \) -print 2>/dev/null || true)
    if [ -n "$bad_files" ]; then
        print_fail "Guard 1 (private-key file under producer fixtures dir):"
        echo "$bad_files"
    fi
fi

# ---------------------------------------------------------------------------
# Guard 2 — no private-key type literal under fixtures.rs.
# Comments are stripped to avoid tripping on documentation that names
# the forbidden types in its rationale.
# ---------------------------------------------------------------------------
if [ -f "$FIXTURES_RS" ]; then
    GUARD2_PATTERNS=(
        'VrfSigningKey'
        'KesSecret'
        'KesSigningKey'
        'ColdSigningKey'
    )
    for pattern in "${GUARD2_PATTERNS[@]}"; do
        matches=$(emit_production_lines "$FIXTURES_RS" | grep -E "$pattern" || true)
        if [ -n "$matches" ]; then
            print_fail "Guard 2 (private-key literal in fixtures.rs — pattern $pattern):"
            echo "$matches"
        fi
    done
else
    print_fail "Guard 2 setup: $FIXTURES_RS not found"
fi

# ---------------------------------------------------------------------------
# Guard 3 — ProducerTick has no Serialize impl that names a private-key
# field. We grep for `impl Serialize for ProducerTick`; if present,
# we additionally fail on any private-key field name nearby.
# ---------------------------------------------------------------------------
if [ -f "$STATE_RS" ]; then
    if grep -E -q '^\s*impl\s+([a-zA-Z_:]+::)?Serialize\s+for\s+ProducerTick\b' "$STATE_RS"; then
        # Found a Serialize impl. Surface any private-key field name.
        bad=$(grep -E '(vrf_signing_key|kes_secret|cold_signing_key|kes_signing_key)' "$STATE_RS" || true)
        if [ -n "$bad" ]; then
            print_fail "Guard 3 (ProducerTick Serialize impl mentions private-key field):"
            echo "$bad"
        else
            print_fail "Guard 3 (ProducerTick has a Serialize impl — must not exist; replay corpora are in-source byte literals)"
        fi
    fi
else
    print_fail "Guard 3 setup: $STATE_RS not found"
fi

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: no-private-keys-in-corpus gates green (3/3)"
    exit 0
else
    exit 1
fi
