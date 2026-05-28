#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-K S7 — node binary uses single bootstrap authority (CN-NODE-01).
# Updated PHASE4-N-T: produce mode is a second legitimate startup path
# that ALSO bootstraps via the sole authority. The invariant is "each
# startup path bootstraps once via bootstrap_initial_state; no path
# bypasses it or double-bootstraps" — NOT "exactly one call site in the
# whole crate". So: every production .rs file calls bootstrap_initial_state
# at most once (no double-bootstrap within a path) AND the crate calls it
# at least once (the authority is actually used, not bypassed). Per-mode
# no-synthetic-bypass is enforced by
# ci_check_produce_mode_uses_bootstrap_initial_state.sh (produce) and the
# ReceiveState guard below (run).

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
        # No single startup path may bootstrap more than once.
        if (( c > 1 )); then
            print_fail "$(basename "$f") calls bootstrap_initial_state $c times in one file — a startup path must bootstrap exactly once"
        fi
    fi
done

# The authority must actually be used (no zero-call bypass). Multiple
# modes (run, produce) each calling it once is correct.
if (( CALLS < 1 )); then
    print_fail "ade_node never calls bootstrap_initial_state — initial state must come from the single bootstrap authority, not a bypass"
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
