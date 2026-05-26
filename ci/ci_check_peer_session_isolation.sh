#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-K S4 — peer-session structural isolation (DC-NODE-01).
#
# The peer session task MUST:
#   - own its own `mpsc::Receiver` (no shared mutable state across
#     peer tasks);
#   - never import directly from `ade_ledger::receive::*` or
#     `ade_network::codec::*` (those flow through orchestrator
#     events).
#   - never spawn additional tasks (each peer task is one tokio
#     task, no fanout).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_runtime/src/orchestrator/peer_session.rs"

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

body=$(strip_for_grep "$TARGET")

# No direct receive-side or codec import (flow through orchestrator
# events).
if echo "$body" | grep -qE 'use ade_ledger::receive|use ade_network::codec'; then
    print_fail "peer_session.rs must not import from ade_ledger::receive or ade_network::codec (route via OrchestratorEvent)"
fi

# No tokio::spawn (one task per peer, no fanout).
if echo "$body" | grep -qE 'tokio::spawn'; then
    print_fail "peer_session.rs must not spawn additional tasks (one tokio task per peer)"
fi

# Positive: must hold mpsc::Receiver and emit OrchestratorEvent.
if ! echo "$body" | grep -qE 'mpsc::Receiver<PeerInboundFrame>'; then
    print_fail "peer_session.rs must own its own mpsc::Receiver<PeerInboundFrame>"
fi
if ! echo "$body" | grep -qE 'OrchestratorEvent'; then
    print_fail "peer_session.rs must emit OrchestratorEvent values"
fi

if (( FAILED == 0 )); then
    echo "OK: peer session structural isolation invariants hold"
fi
exit $FAILED
