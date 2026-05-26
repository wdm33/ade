#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-L-LIVE S2 — wire-only path does not call
# bootstrap_initial_state (RO-LIVE-04 ¬P-2).
#
# Mechanical guards:
#   1. `crates/ade_node/src/wire_only.rs` MUST NOT call
#      `bootstrap_initial_state` anywhere in its production
#      body.
#   2. `crates/ade_node/src/main.rs`'s `Mode::WireOnly` arm
#      MUST NOT call `bootstrap_initial_state`. We enforce
#      this by requiring `bootstrap_initial_state` to NOT
#      appear in main.rs at all (the binary's only bootstrap
#      caller would be the admission cluster's future code).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WIRE="$REPO_ROOT/crates/ade_node/src/wire_only.rs"
MAIN="$REPO_ROOT/crates/ade_node/src/main.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

for f in "$WIRE" "$MAIN"; do
    if [[ ! -f "$f" ]]; then
        print_fail "missing $f"
        continue
    fi
    rel="${f#$REPO_ROOT/}"
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE '\bbootstrap_initial_state\b'; then
        print_fail "$rel calls bootstrap_initial_state (wire-only path MUST NOT bootstrap)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: wire-only path does not call bootstrap_initial_state (RO-LIVE-04 ¬P-2)"
fi
exit $FAILED
