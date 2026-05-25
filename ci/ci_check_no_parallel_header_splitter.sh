#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-G S1 — single canonical header/body byte splitter.
#
# Closure proof for DC-CONS-16 strengthening + DC-CONS-18 entry: the
# only function in the workspace that returns a Cardano block-envelope
# header byte sub-slice is the validator's existing recipe at
# `crates/ade_ledger/src/block_validity/header_input.rs`. Any parallel
# splitter outside this site is forbidden — the validator and the
# producer-side server pump MUST share one authority.
#
# What we accept:
#   - `pub fn accepted_block_header_bytes` in
#     `crates/ade_ledger/src/block_validity/header_input.rs`
#     (S1's lifted public accessor; reuses `header_cbor_slice`)
#   - `fn header_cbor_slice` (file-private; the canonical walker)
#   - `pub fn header_cbor_slice` is NOT exposed; the public API is
#     `accepted_block_header_bytes` only.
#
# What we forbid:
#   - Any new `pub fn .*header_bytes(...)` outside the canonical site
#   - Any new `pub fn .*split_header(...)` anywhere
#   - Any new `pub fn .*split_block_envelope(...)` anywhere
#   - Re-implementations of the inner header sub-slice walker.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

CANONICAL_SITE="crates/ade_ledger/src/block_validity/header_input.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

# Find all `pub fn` whose name ends in `header_bytes` across the
# workspace. Only the canonical site may host one.
mapfile -t HITS < <(
    grep -rEn 'pub fn [a-zA-Z0-9_]*header_bytes\b' "$REPO_ROOT/crates" \
        --include='*.rs' \
        | grep -v "/$CANONICAL_SITE:" \
        || true
)

if (( ${#HITS[@]} > 0 )); then
    print_fail "found pub fn *_header_bytes outside $CANONICAL_SITE:"
    for h in "${HITS[@]}"; do
        echo "  $h"
    done
fi

# Forbid `pub fn .*split_header(` anywhere.
mapfile -t SPLIT_HITS < <(
    grep -rEn 'pub fn [a-zA-Z0-9_]*split_header\b' "$REPO_ROOT/crates" \
        --include='*.rs' || true
)
if (( ${#SPLIT_HITS[@]} > 0 )); then
    print_fail "found pub fn *_split_header (forbidden by N-G S1):"
    for h in "${SPLIT_HITS[@]}"; do
        echo "  $h"
    done
fi

# Forbid `pub fn .*split_block_envelope(` anywhere.
mapfile -t ENV_HITS < <(
    grep -rEn 'pub fn [a-zA-Z0-9_]*split_block_envelope\b' "$REPO_ROOT/crates" \
        --include='*.rs' || true
)
if (( ${#ENV_HITS[@]} > 0 )); then
    print_fail "found pub fn *_split_block_envelope (forbidden by N-G S1):"
    for h in "${ENV_HITS[@]}"; do
        echo "  $h"
    done
fi

# Positive check: the canonical accessor must exist.
if ! grep -q 'pub fn accepted_block_header_bytes' "$REPO_ROOT/$CANONICAL_SITE"; then
    print_fail "canonical accessor accepted_block_header_bytes missing from $CANONICAL_SITE"
fi

if (( FAILED == 0 )); then
    echo "OK: header/body splitter is single-authority at $CANONICAL_SITE"
fi
exit $FAILED
