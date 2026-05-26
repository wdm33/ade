#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-L S1 — handshake closure (CN-SESS-02).
#
# Single pub `n2n_transition` and single pub `n2c_transition`,
# both in `crates/ade_network/src/handshake/transition.rs`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_network/src/handshake/transition.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

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

EXTRA_N2N=()
EXTRA_N2C=()
for f in $(find "$REPO_ROOT/crates" -type f -name '*.rs'); do
    if [[ "$f" == "$TARGET" ]]; then continue; fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'pub fn n2n_transition\b'; then
        EXTRA_N2N+=("$f")
    fi
    if echo "$body" | grep -qE 'pub fn n2c_transition\b'; then
        EXTRA_N2C+=("$f")
    fi
done

if (( ${#EXTRA_N2N[@]} > 0 )); then
    print_fail "second pub fn n2n_transition in:"
    for f in "${EXTRA_N2N[@]}"; do echo "  $f"; done
fi
if (( ${#EXTRA_N2C[@]} > 0 )); then
    print_fail "second pub fn n2c_transition in:"
    for f in "${EXTRA_N2C[@]}"; do echo "  $f"; done
fi

PROD=$(strip_for_grep "$TARGET")
if ! echo "$PROD" | grep -qE 'pub fn n2n_transition'; then
    print_fail "n2n_transition missing"
fi
if ! echo "$PROD" | grep -qE 'pub fn n2c_transition'; then
    print_fail "n2c_transition missing"
fi

if (( FAILED == 0 )); then
    echo "OK: handshake closure (CN-SESS-02) holds"
fi
exit $FAILED
