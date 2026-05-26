#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-K S7 — node binary uses single bootstrap authority (CN-NODE-01).
#
# The `ade_node` crate calls `bootstrap_initial_state` exactly once
# (the orchestrator-startup branch) and never constructs the
# `(LedgerState, PraosChainDepState, Option<ChainTip>)` triple
# directly via a second path.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NODE_DIR="$REPO_ROOT/crates/ade_node/src"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

CALLS=0
for f in $(find "$NODE_DIR" -type f -name '*.rs'); do
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'bootstrap_initial_state\b'; then
        c=$(echo "$body" | grep -cE 'bootstrap_initial_state\(')
        CALLS=$((CALLS + c))
    fi
done

if (( CALLS != 1 )); then
    print_fail "ade_node must call bootstrap_initial_state exactly once across its source (found $CALLS production-body invocations)"
fi

# Forbid `ReceiveState::new` outside the run-loop construction.
# (The single legit caller is node.rs's run loop after bootstrap.)
RECEIVE_NEW=0
for f in $(find "$NODE_DIR" -type f -name '*.rs'); do
    body=$(strip_for_grep "$f")
    c=$(echo "$body" | grep -cE 'ReceiveState::new\(' || true)
    RECEIVE_NEW=$((RECEIVE_NEW + c))
done
if (( RECEIVE_NEW > 1 )); then
    print_fail "ade_node must construct ReceiveState at most once (found $RECEIVE_NEW)"
fi

if (( FAILED == 0 )); then
    echo "OK: ade_node binary uses the single bootstrap authority"
fi
exit $FAILED
