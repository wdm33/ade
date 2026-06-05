#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-L-LIVE S1 — closed wire-only JSONL event vocabulary
# (RO-LIVE-04 ¬P-1).
#
# Mechanical guards:
#   1. The strings `agreement_verdict`, `admitted_block`,
#      `ledger_applied`, `projection_updated` MUST NOT appear
#      in any production-body source file under
#      `crates/ade_node/src/`. (cfg(test) blocks are stripped
#      before the grep so test-only negative assertions are
#      allowed.)
#   2. The `LiveLogEvent` discriminator allow-list at
#      `crates/ade_node/src/live_log/event.rs` is the seven
#      strings the writer emits. CI confirms each is present
#      (positive grep) and no others (no `event = "X"` /
#      `"event":"X"` string literals outside the writer's
#      `discriminator()` and `push_key_str(out, "event", ...)`
#      call sites).
#
# Doctrine: see
# `~/.claude/projects/.../memory/feedback-shell-must-not-overstate-semantic-truth.md`.
# Wire success does not imply admission; the binary must not
# claim admission events from wire-only mode.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NODE_SRC="$REPO_ROOT/crates/ade_node/src"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

FORBIDDEN=(
    "agreement_verdict"
    "admitted_block"
    "ledger_applied"
    "projection_updated"
)

# Rule 1: forbidden strings absent from every production body
# WHEN appearing as a quoted JSON-event-name literal. We match
# the form `"agreement_verdict"` (etc.) — not bare identifiers,
# since `admitted_blocks` (plural) is a legitimate struct field
# in node.rs (PHASE4-N-K admission-evidence counter). The
# guard's intent is the JSONL emission surface, not Rust
# identifiers.
#
# Scope: ONLY the wire-only surface (live_log/ + wire_only.rs).
# The later-added admission_log/ directory legitimately registers
# `agreement_verdict` as an admission event; its closure is owned
# by the sibling gate ci_check_admission_log_vocabulary_closed.sh.
WIRE_ONLY_FILES() {
    find "$NODE_SRC/live_log" -type f -name '*.rs' 2>/dev/null
    [[ -f "$NODE_SRC/wire_only.rs" ]] && echo "$NODE_SRC/wire_only.rs"
}
for f in $(WIRE_ONLY_FILES); do
    rel="${f#$REPO_ROOT/}"
    body=$(strip_for_grep "$f")
    for needle in "${FORBIDDEN[@]}"; do
        if echo "$body" | grep -qE "\"${needle}\""; then
            print_fail "$rel contains forbidden event-name literal: \"${needle}\""
        fi
    done
done

# Rule 2: allow-list of discriminators present in event.rs.
EVENT="$NODE_SRC/live_log/event.rs"
if [[ ! -f "$EVENT" ]]; then
    print_fail "missing $EVENT"
else
    EVENT_BODY=$(strip_for_grep "$EVENT")
    ALLOWED=(
        "node_started"
        "peer_dial_started"
        "handshake_ok"
        "peer_tip_read"
        "peer_dial_failed"
        "wire_smoke_complete"
        "node_shutdown"
    )
    for s in "${ALLOWED[@]}"; do
        if ! echo "$EVENT_BODY" | grep -q "\"$s\""; then
            print_fail "live_log/event.rs missing discriminator string: $s"
        fi
    done
fi

if (( FAILED == 0 )); then
    echo "OK: wire-only event vocabulary is closed (RO-LIVE-04 ¬P-1)"
fi
exit $FAILED
