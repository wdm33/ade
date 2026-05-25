#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-G S6 — n2n_server key-boundary preservation.
#
# Mechanical guard: the producer-side server orchestrator MUST NOT
# import from `crate::producer::signing` (or any item therein).
# Private-key custody stays RED-confined to the producer signing
# pipeline; the server pump only handles AcceptedBlock-derived
# bytes.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$REPO_ROOT/crates/ade_runtime/src/network"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -d "$TARGET_DIR" ]]; then
    print_fail "target directory missing: $TARGET_DIR"
    exit "$FAILED"
fi

# Strip line comments + tests before grepping.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

for f in "$TARGET_DIR"/*.rs; do
    [[ -f "$f" ]] || continue
    body=$(strip_for_grep "$f")
    # Forbid imports / paths into producer::signing.
    if echo "$body" | grep -qE 'producer::signing|crate::producer::signing'; then
        print_fail "$(basename "$f"): producer::signing import forbidden in n2n_server modules"
    fi
    if echo "$body" | grep -qE '\bVrfSigningKey\b|\bKesSecret\b|\bKesSigningKey\b|\bSigningError\b'; then
        print_fail "$(basename "$f"): private-key types forbidden in n2n_server modules"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: n2n_server modules do not depend on producer::signing"
fi
exit $FAILED
