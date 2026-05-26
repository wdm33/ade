#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-FRAG — session-reducer per-mini-protocol payload
# reassembly gate (CN-SESS-04, DC-SESS-06).
#
# Asserts:
#   1. Exactly one definition of `ProtoBuffers` in the workspace,
#      living in `ade_network::session::state`.
#   2. `ConnectedState` carries a `proto_buffers: ProtoBuffers`
#      field (no second per-protocol accumulator anywhere).
#   3. `drain_protocol_items` is the SOLE per-protocol drain
#      authority, defined in `ade_network::session::core`.
#   4. No `HashMap` / `HashSet` in `session/state.rs` or
#      `session/core.rs` — the per-protocol buffers must use a
#      closed-sum-indexed structure.
#   5. `SessionError::ProtocolPayloadMalformed` exists as a
#      closed-sum variant.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NETWORK="$REPO_ROOT/crates/ade_network/src"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# (1) ProtoBuffers struct defined exactly once.
n_proto=$(grep -rE '^pub struct ProtoBuffers\b|^struct ProtoBuffers\b' "$REPO_ROOT/crates" \
    --include='*.rs' 2>/dev/null | wc -l)
if (( n_proto != 1 )); then
    print_fail "expected exactly 1 ProtoBuffers struct, found $n_proto"
fi
# And the one definition lives in session/state.rs.
if ! grep -qE '^pub struct ProtoBuffers\b' "$NETWORK/session/state.rs"; then
    print_fail "ProtoBuffers definition missing from ade_network/src/session/state.rs"
fi

# (2) ConnectedState carries proto_buffers field.
if ! grep -qE '\bproto_buffers: ProtoBuffers\b' "$NETWORK/session/state.rs"; then
    print_fail "ConnectedState missing proto_buffers field"
fi

# (3) drain_protocol_items single authority.
n_drain=$(grep -rE '^fn drain_protocol_items\b|^pub(\(crate\))? fn drain_protocol_items\b' \
    "$REPO_ROOT/crates" --include='*.rs' 2>/dev/null | wc -l)
if (( n_drain != 1 )); then
    print_fail "expected exactly 1 drain_protocol_items, found $n_drain"
fi
if ! grep -qE '^fn drain_protocol_items\b|^pub(\(crate\))? fn drain_protocol_items\b' \
    "$NETWORK/session/core.rs"; then
    print_fail "drain_protocol_items missing from ade_network/src/session/core.rs"
fi

# (4) No HashMap/HashSet in session/state.rs or session/core.rs
# (strip comments before checking — doc references are OK).
strip_comments() {
    awk '{ sub(/\/\/.*$/, ""); print }' "$1"
}
for f in "$NETWORK/session/state.rs" "$NETWORK/session/core.rs"; do
    if strip_comments "$f" | grep -qE '\b(HashMap|HashSet)\b'; then
        print_fail "HashMap/HashSet used in $(basename "$f") (must be closed-sum-indexed)"
    fi
done

# (5) SessionError::ProtocolPayloadMalformed variant exists.
if ! grep -qE '\bProtocolPayloadMalformed\b' "$NETWORK/session/event.rs"; then
    print_fail "SessionError::ProtocolPayloadMalformed variant missing"
fi

if (( FAILED == 0 )); then
    echo "OK: session per-mini-protocol reassembly gates hold (CN-SESS-04 + DC-SESS-06)"
fi
exit $FAILED
