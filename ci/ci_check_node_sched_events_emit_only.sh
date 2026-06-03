#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-J S1 (CN-NODE-04): the feed/forge scheduling diagnostic events
# are EMIT-ONLY. The GREEN pure planner (run_loop_planner / plan_loop_step)
# emits NO NodeSchedEvent and reads NO event — the CN-NODE-04 vocabulary flows
# one-directionally relay-loop -> log, never log -> planner. The planner stays a
# pure scheduler, never a diagnostic consumer; the scheduling decision cannot
# depend on what was (or was not) logged.
#
# Mechanical guards (#[cfg(test)] + line comments stripped first):
#   (a) NEGATIVE: run_loop_planner.rs names NONE of the diagnostic
#       event/sink/vocabulary tokens.
#   (b) POSITIVE: the closed NodeSchedEvent vocabulary exists in its sibling
#       module, and the relay loop (node_lifecycle.rs) is the sole producer.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLANNER="$REPO_ROOT/crates/ade_node/src/run_loop_planner.rs"
SCHED_EVENT="$REPO_ROOT/crates/ade_node/src/live_log/sched_event.rs"
LIFECYCLE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$PLANNER" "$SCHED_EVENT" "$LIFECYCLE"; do
    [[ -f "$f" ]] || print_fail "missing expected source $f"
done
if (( FAILED != 0 )); then
    echo "FAIL: ci_check_node_sched_events_emit_only"
    exit 1
fi

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# --- Guard (a): NEGATIVE — the planner names no CN-NODE-04 diagnostic token.
PLANNER_BODY="$(strip_for_grep "$PLANNER")"
DIAG_RE='NodeSchedEvent|NodeSchedSink|NodeSchedLogWriter|sched_event|sched_writer|FeedReason|ForgeOutcome'
if echo "$PLANNER_BODY" | grep -qE "$DIAG_RE"; then
    print_fail "run_loop_planner.rs references a CN-NODE-04 diagnostic token (emit-only violation): the planner must never emit or read a NodeSchedEvent — events flow one-directionally relay-loop -> log: $(echo "$PLANNER_BODY" | grep -nE "$DIAG_RE" | head -n1)"
fi

# --- Guard (b): POSITIVE — the vocabulary is defined + the relay loop emits it.
if ! grep -qE 'enum NodeSchedEvent' "$SCHED_EVENT"; then
    print_fail "the closed NodeSchedEvent vocabulary is not defined in $SCHED_EVENT"
fi
LIFECYCLE_BODY="$(strip_for_grep "$LIFECYCLE")"
if ! echo "$LIFECYCLE_BODY" | grep -qE 'NodeSchedEvent::'; then
    print_fail "node_lifecycle.rs does not emit any NodeSchedEvent — the relay loop is the sole producer"
fi

if (( FAILED == 0 )); then
    echo "OK: CN-NODE-04 feed/forge events are emit-only — run_loop_planner names no NodeSchedEvent token; the relay loop is the sole producer (one-directional relay-loop -> log)."
fi
exit $FAILED
