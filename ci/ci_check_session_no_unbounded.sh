#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-L S5+S6+S7+S8 — backpressure discipline (DC-SESS-04).
#
# No `unbounded_channel` / `unbounded_send` / `mpsc::unbounded` in
# any wire-session, mux-transport, mux-pump, n2n-dialer, or
# keep-alive-session file.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

TARGETS=(
    "$REPO_ROOT/crates/ade_network/src/session"
    "$REPO_ROOT/crates/ade_network/src/mux/transport.rs"
    "$REPO_ROOT/crates/ade_runtime/src/network/mux_pump.rs"
    "$REPO_ROOT/crates/ade_runtime/src/network/n2n_dialer.rs"
    "$REPO_ROOT/crates/ade_runtime/src/orchestrator/keep_alive_session.rs"
)

scan_file() {
    local f="$1"
    local rel="${f#$REPO_ROOT/}"
    local body
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE 'unbounded_channel|unbounded_send|::unbounded\b'; then
        print_fail "$rel uses unbounded channel constructor"
    fi
}

for t in "${TARGETS[@]}"; do
    if [[ -d "$t" ]]; then
        for f in "$t"/*.rs; do
            [[ -f "$f" ]] || continue
            scan_file "$f"
        done
    elif [[ -f "$t" ]]; then
        scan_file "$t"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: session / mux-pump / dialer / keep-alive use only bounded channels (DC-SESS-04)"
fi
exit $FAILED
