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

# `ReceiveState::new` is the recovered/bootstrapped-state entry into the relay
# spine. It is legitimate ONLY in the lifecycle-owner files: node.rs's run loop
# and node_lifecycle.rs's --mode node arms (the FirstRun/WarmStart bootstrap +,
# PHASE4-N-F-F, the mutually-exclusive ForgeIntent::Off/On branches of the single
# run_node_lifecycle_inner dispatcher — only one runs per process). Any
# occurrence in any OTHER ade_node production file is a synthetic/rogue bypass of
# the recovered state and fails closed. (A "<=1 per crate" count is impossible
# once two owner files exist, and cannot tell mutually-exclusive arms apart;
# double-bootstrap-within-a-path is covered by the per-file bootstrap_initial_state
# check above + ci_check_node_run_loop_containment.sh.)
RECEIVE_OWNERS="node.rs node_lifecycle.rs"
for f in $(find "$NODE_DIR" -type f -name '*.rs'); do
    body=$(strip_for_grep "$f")
    c=$(echo "$body" | grep -cE 'ReceiveState::new\(' || true)
    if (( c > 0 )); then
        base=$(basename "$f")
        if ! echo " $RECEIVE_OWNERS " | grep -q " $base "; then
            print_fail "$base constructs ReceiveState::new (rogue recovered-state bypass) — only the lifecycle owners ($RECEIVE_OWNERS) may construct it"
        fi
    fi
done

if (( FAILED == 0 )); then
    echo "OK: ade_node binary uses the single bootstrap authority"
fi
exit $FAILED
