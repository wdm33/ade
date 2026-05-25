#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-C S4 — single canonical body-hash authority gate.
#
# Mechanical guards (closure proof for CE-N-C-4 / DC-CONS-16):
#
#   1. EXACTLY two `pub fn block_body_hash{,_from_buckets}` definitions
#      in BLUE crates, both in
#      crates/ade_ledger/src/block_body_hash.rs.
#   2. No private function in BLUE crates that re-implements the recipe
#      shape: forbidden names (`compute_body_hash`,
#      `block_body_hash_inner`, `recompute_body_hash`) AND no
#      `let mut concat = [0u8; 128];` outside the canonical authority.
#   3. `producer/forge.rs` does not import or call `blake2b_256`
#      directly (the canonical authority owns the primitive).
#   4. `producer/forge.rs` calls `block_body_hash_from_buckets(`.
#   5. `block_validity/header_input.rs` calls
#      `block_body_hash::block_body_hash(`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

BODY_HASH_AUTHORITY="$REPO_ROOT/crates/ade_ledger/src/block_body_hash.rs"
FORGE_RS="$REPO_ROOT/crates/ade_ledger/src/producer/forge.rs"
HEADER_INPUT_RS="$REPO_ROOT/crates/ade_ledger/src/block_validity/header_input.rs"

BLUE_CRATE_SRC=(
    "$REPO_ROOT/crates/ade_ledger/src"
    "$REPO_ROOT/crates/ade_core/src"
    "$REPO_ROOT/crates/ade_codec/src"
    "$REPO_ROOT/crates/ade_types/src"
    "$REPO_ROOT/crates/ade_crypto/src"
)

FAIL=0

print_fail() {
    echo "FAIL: $1"
    FAIL=1
}

# Required-file presence.
for f in "$BODY_HASH_AUTHORITY" "$FORGE_RS" "$HEADER_INPUT_RS"; do
    [ -f "$f" ] || { print_fail "expected file missing: $f"; }
done
[ "$FAIL" -eq 0 ] || exit 1

# ---------------------------------------------------------------------------
# Guard 1 — Single canonical body-hash function (exactly two pub fn
# definitions, both in block_body_hash.rs).
# ---------------------------------------------------------------------------
GUARD1_MATCHES=$(grep -rE 'pub fn block_body_hash\b|pub fn block_body_hash_from_buckets\b' \
    "${BLUE_CRATE_SRC[@]}" 2>/dev/null || true)
GUARD1_COUNT=$(echo -n "$GUARD1_MATCHES" | grep -c . || true)
if [ "$GUARD1_COUNT" -ne 2 ]; then
    print_fail "Guard 1 (expected exactly 2 'pub fn block_body_hash*' matches across BLUE, got $GUARD1_COUNT):"
    echo "$GUARD1_MATCHES"
fi
# Both matches must live in the canonical authority file.
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    if [ "$file" != "$BODY_HASH_AUTHORITY" ]; then
        print_fail "Guard 1 (body-hash pub fn declared outside canonical authority):"
        echo "  $hit"
    fi
done <<< "$GUARD1_MATCHES"

# ---------------------------------------------------------------------------
# Guard 2 — No private body-hash re-implementation by name OR by shape.
# ---------------------------------------------------------------------------
GUARD2_NAMES_PATTERN='\bfn (compute_body_hash|block_body_hash_inner|recompute_body_hash)\b'
GUARD2_HITS=$(grep -rEn "$GUARD2_NAMES_PATTERN" "${BLUE_CRATE_SRC[@]}" 2>/dev/null || true)
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    print_fail "Guard 2 (forbidden private body-hash impl by name):"
    echo "  $hit"
done <<< "$GUARD2_HITS"

# Shape check: any `let mut concat = [0u8; 128];` outside the authority.
GUARD2_SHAPE=$(grep -rEn 'let mut concat\s*=\s*\[0u8;\s*128\]\s*;' "${BLUE_CRATE_SRC[@]}" 2>/dev/null || true)
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    if [ "$file" != "$BODY_HASH_AUTHORITY" ]; then
        print_fail "Guard 2 (forbidden 128-byte concat outside canonical authority):"
        echo "  $hit"
    fi
done <<< "$GUARD2_SHAPE"

# ---------------------------------------------------------------------------
# Guard 3 — forge.rs no longer imports or directly calls blake2b_256.
# Strip line comments and skip `#[cfg(test)]` tail so doc-comments and
# test helpers are not false positives.
# ---------------------------------------------------------------------------
FORGE_PROD=$(awk '
    /^#\[cfg\(test\)\]/ { exit }
    {
        line = $0
        sub(/\/\/.*$/, "", line)
        print NR ":" line
    }' "$FORGE_RS")

if echo "$FORGE_PROD" | grep -nE '^[0-9]+:\s*use\s+ade_crypto::blake2b_256\b' > /dev/null; then
    print_fail "Guard 3 (forge.rs still imports ade_crypto::blake2b_256 — should route through block_body_hash):"
    echo "$FORGE_PROD" | grep -nE '^[0-9]+:\s*use\s+ade_crypto::blake2b_256\b'
fi
if echo "$FORGE_PROD" | grep -nE '\bblake2b_256\s*\(' > /dev/null; then
    print_fail "Guard 3 (forge.rs still calls blake2b_256 directly):"
    echo "$FORGE_PROD" | grep -nE '\bblake2b_256\s*\('
fi

# ---------------------------------------------------------------------------
# Guard 4 — forge.rs calls the canonical function.
# ---------------------------------------------------------------------------
if ! grep -nE 'block_body_hash_from_buckets\s*\(' "$FORGE_RS" > /dev/null; then
    print_fail "Guard 4 (forge.rs does not call block_body_hash_from_buckets — canonical authority is unused)"
fi

# ---------------------------------------------------------------------------
# Guard 5 — header_input.rs calls the canonical block-level wrapper.
# ---------------------------------------------------------------------------
if ! grep -nE 'block_body_hash::block_body_hash\s*\(' "$HEADER_INPUT_RS" > /dev/null; then
    print_fail "Guard 5 (header_input.rs does not call block_body_hash::block_body_hash — canonical authority is unused)"
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: body-hash single-authority gates green (5/5)"
    exit 0
else
    exit 1
fi
