#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-H S4 — receive orchestrator key-boundary.
#
# Mechanical guard: no module under `ade_runtime::receive::*` may
# import from `producer::signing`, `producer::broadcast`, or
# `producer::scheduler`. The receive side has no signing keys and no
# path to the producer pipeline.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$REPO_ROOT/crates/ade_runtime/src/receive"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -d "$TARGET_DIR" ]]; then
    print_fail "target directory missing: $TARGET_DIR"
    exit "$FAILED"
fi

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
    if echo "$body" | grep -qE 'producer::signing|producer::broadcast|producer::scheduler|crate::producer::'; then
        print_fail "$(basename "$f"): producer::* import forbidden in receive modules"
    fi
    if echo "$body" | grep -qE '\bVrfSigningKey\b|\bKesSecret\b|\bKesSigningKey\b|\bSigningError\b|\bBroadcastQueue\b'; then
        print_fail "$(basename "$f"): producer-side private-key/broadcast types forbidden"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: receive modules do not depend on producer::signing/broadcast/scheduler"
fi
exit $FAILED
