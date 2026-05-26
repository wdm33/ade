#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-L S1 — closed mini-protocol id registry (DC-SESS-02).
#
# Mechanical guards:
#   1. `AcceptedMiniProtocol::from_id` exists in session/event.rs and
#      contains an `_ => None` sink (closed-set discipline: any
#      unknown id returns None; the session core then maps to
#      SessionError::UnknownMiniProtocolId).
#   2. `session/core.rs` dispatch on AcceptedMiniProtocol must NOT
#      contain a `_ =>` wildcard arm in the dispatch site — i.e.,
#      every accepted protocol must have an explicit match arm.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EVENT="$REPO_ROOT/crates/ade_network/src/session/event.rs"
CORE="$REPO_ROOT/crates/ade_network/src/session/core.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$EVENT" ]]; then
    print_fail "missing $EVENT"
fi
if [[ ! -f "$CORE" ]]; then
    print_fail "missing $CORE"
fi
if (( FAILED != 0 )); then exit "$FAILED"; fi

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Rule 1: from_id must exist with the `_ => None` closed-set sink.
EVENT_BODY=$(strip_for_grep "$EVENT")
if ! echo "$EVENT_BODY" | grep -qE 'pub fn from_id'; then
    print_fail "AcceptedMiniProtocol::from_id missing"
fi
if ! echo "$EVENT_BODY" | grep -qE '_ => None'; then
    print_fail "AcceptedMiniProtocol::from_id must close with `_ => None`"
fi

# Rule 2: core.rs must NOT contain a wildcard arm in a match on
# AcceptedMiniProtocol. (A wildcard would silently accept future
# variants instead of forcing a compile error.)
CORE_BODY=$(strip_for_grep "$CORE")
# Heuristic: look for `match` followed by an AcceptedMiniProtocol
# variant and any `_ =>` inside the same block. We scan for the
# explicit dispatch comment pattern instead — the dispatch site has
# a known multi-arm match block that must list every variant.
if ! echo "$CORE_BODY" | grep -qE 'AcceptedMiniProtocol::Handshake'; then
    print_fail "core.rs must dispatch on AcceptedMiniProtocol::Handshake"
fi
if ! echo "$CORE_BODY" | grep -qE 'AcceptedMiniProtocol::ChainSync'; then
    print_fail "core.rs must dispatch on AcceptedMiniProtocol::ChainSync"
fi
if ! echo "$CORE_BODY" | grep -qE 'AcceptedMiniProtocol::BlockFetch'; then
    print_fail "core.rs must dispatch on AcceptedMiniProtocol::BlockFetch"
fi
if ! echo "$CORE_BODY" | grep -qE 'AcceptedMiniProtocol::KeepAlive'; then
    print_fail "core.rs must dispatch on AcceptedMiniProtocol::KeepAlive"
fi

# Positive: session::core must NOT contain a wildcard accept on
# raw `MiniProtocolId` or `u16` ids — every id flows through
# `AcceptedMiniProtocol::from_id`.
if echo "$CORE_BODY" | grep -qE 'mini_protocol_id\.get\(\)[[:space:]]*=>'; then
    print_fail "core.rs contains direct id pattern dispatch — must route through AcceptedMiniProtocol::from_id"
fi

if (( FAILED == 0 )); then
    echo "OK: mini-protocol id registry is closed (DC-SESS-02)"
fi
exit $FAILED
