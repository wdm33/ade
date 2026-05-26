#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-L S1 — mux frame closure (CN-SESS-01).
#
# Single pub `encode_frame`/`decode_frame` pair, in
# `crates/ade_network/src/mux/frame.rs`. No alternative encoders
# anywhere in `crates/`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_network/src/mux/frame.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -f "$TARGET" ]]; then
    print_fail "missing $TARGET"
    exit "$FAILED"
fi

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Search every .rs file in crates/ for a pub fn named encode_frame
# or decode_frame. The only acceptable site is mux/frame.rs.
EXTRA_ENCODE=()
EXTRA_DECODE=()
for f in $(find "$REPO_ROOT/crates" -type f -name '*.rs'); do
    if [[ "$f" == "$TARGET" ]]; then
        continue
    fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'pub fn encode_frame\b'; then
        EXTRA_ENCODE+=("$f")
    fi
    if echo "$body" | grep -qE 'pub fn decode_frame\b'; then
        EXTRA_DECODE+=("$f")
    fi
done

if (( ${#EXTRA_ENCODE[@]} > 0 )); then
    print_fail "second pub fn encode_frame in:"
    for f in "${EXTRA_ENCODE[@]}"; do echo "  $f"; done
fi
if (( ${#EXTRA_DECODE[@]} > 0 )); then
    print_fail "second pub fn decode_frame in:"
    for f in "${EXTRA_DECODE[@]}"; do echo "  $f"; done
fi

PROD=$(strip_for_grep "$TARGET")
if ! echo "$PROD" | grep -qE 'pub fn encode_frame'; then
    print_fail "encode_frame missing from mux/frame.rs"
fi
if ! echo "$PROD" | grep -qE 'pub fn decode_frame'; then
    print_fail "decode_frame missing from mux/frame.rs"
fi

if (( FAILED == 0 )); then
    echo "OK: mux frame closure (CN-SESS-01) holds"
fi
exit $FAILED
