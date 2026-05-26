#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-L S2 — session core closure (CN-SESS-03 + DC-SESS-01 + DC-SESS-05).
#
# Mechanical guards:
#   1. session/core.rs has exactly one pub fn (`step`).
#   2. session/{core,event,state,demux,handshake_driver,mod}.rs
#      contain no tokio / SystemTime / Instant / rand / HashMap / float.
#   3. SessionState type carries both Handshaking + Connected variants
#      (type-state DC-SESS-01).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SESSION_DIR="$REPO_ROOT/crates/ade_network/src/session"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

CORE="$SESSION_DIR/core.rs"
if [[ ! -f "$CORE" ]]; then print_fail "missing $CORE"; exit $FAILED; fi

# 1. Single pub fn in core.rs.
CORE_BODY=$(strip_for_grep "$CORE")
N=$(echo "$CORE_BODY" | grep -cE 'pub fn [a-zA-Z0-9_]+')
if (( N != 1 )); then
    print_fail "session/core.rs must have exactly one pub fn (found $N)"
fi
if ! echo "$CORE_BODY" | grep -qE 'pub fn step\b'; then
    print_fail "session::core::step missing"
fi

# 2. Purity of every session/*.rs file (no RED constructs).
for f in "$SESSION_DIR"/*.rs; do
    rel="${f#$REPO_ROOT/}"
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'tokio::|SystemTime|\bInstant\b'; then
        print_fail "$rel contains tokio / SystemTime / Instant constructs"
    fi
    if echo "$body" | grep -qE '\brand::|\brand_core'; then
        print_fail "$rel contains rand"
    fi
    if echo "$body" | grep -qE '\bHashMap\b|\bHashSet\b'; then
        print_fail "$rel contains HashMap / HashSet"
    fi
    if echo "$body" | grep -qE '\bf32\b|\bf64\b'; then
        print_fail "$rel contains floating-point types"
    fi
done

# 3. Type-state present.
STATE="$SESSION_DIR/state.rs"
if [[ -f "$STATE" ]]; then
    body=$(strip_for_grep "$STATE")
    if ! echo "$body" | grep -qE 'Handshaking\('; then
        print_fail "SessionState must carry a Handshaking(...) variant"
    fi
    if ! echo "$body" | grep -qE 'Connected\('; then
        print_fail "SessionState must carry a Connected(...) variant"
    fi
fi

if (( FAILED == 0 )); then
    echo "OK: session core closure (CN-SESS-03 + DC-SESS-01 + DC-SESS-05) holds"
fi
exit $FAILED
