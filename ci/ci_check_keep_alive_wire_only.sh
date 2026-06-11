#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AM — wire-pump keep-alive client is WIRE-ONLY (DC-PUMP-03).
#
# The admission wire pump runs the N2N keep-alive CLIENT (mini-protocol 8)
# by REUSING the BLUE ade_network::keep_alive grammar, and the keep-alive
# code paths NEVER synthesize a semantic AdmissionPeerEvent (Block /
# TipUpdate / RollBackward) -- keep-alive is transport liveness only, so the
# DC-PUMP-01 emit-set stays unwidened.
#
# Mechanical guards:
#   1. handle_keep_alive is defined exactly once in wire_pump.rs and drives
#      the BLUE keep_alive_transition over a decode_keep_alive_message (it
#      consumes the peer's MsgResponseKeepAlive -- no longer the silent
#      multi-protocol drop).
#   2. The handle_keep_alive body constructs NO AdmissionPeerEvent
#      (wire-only -- it returns Result<(), KeepAliveError>, no event channel).
#   3. The keep-alive cadence tick block enqueues an OutboundFrame on
#      AcceptedMiniProtocol::KeepAlive via encode_keep_alive_message (the pump
#      SENDS MsgKeepAlive on a cadence), and synthesizes NO semantic
#      AdmissionPeerEvent (Block / TipUpdate / RollBackward / tip_update).
#   4. The pump REUSES the BLUE grammar -- it does NOT redefine the keep-alive
#      state machine or message type in the RED pump file, and it imports the
#      BLUE ade_network::keep_alive module.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PUMP="$REPO_ROOT/crates/ade_runtime/src/admission/wire_pump.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$PUMP" ]]; then
    print_fail "missing $PUMP"
    exit "$FAILED"
fi

strip_comments() {
    awk '{ line=$0; sub(/\/\/.*$/, "", line); print line }'
}

# Guard 1: handle_keep_alive defined exactly once + drives the BLUE grammar.
n_def=$(grep -cE '^fn handle_keep_alive\b' "$PUMP" 2>/dev/null || echo 0)
if [[ "$n_def" -ne 1 ]]; then
    print_fail "expected exactly 1 fn handle_keep_alive in wire_pump.rs, found $n_def"
fi
ka_body=$(awk '/^fn handle_keep_alive\(/{f=1;n=0} f{print;n++; if(n>=24) exit}' "$PUMP")
if ! echo "$ka_body" | grep -q 'keep_alive_transition('; then
    print_fail "handle_keep_alive must drive the BLUE keep_alive_transition (DC-PUMP-03 reuse)"
fi
if ! echo "$ka_body" | grep -q 'decode_keep_alive_message('; then
    print_fail "handle_keep_alive must decode via the BLUE decode_keep_alive_message"
fi

# Guard 2: handle_keep_alive emits no AdmissionPeerEvent (strip comments).
if echo "$ka_body" | strip_comments | grep -qE 'AdmissionPeerEvent'; then
    print_fail "handle_keep_alive must not construct an AdmissionPeerEvent (wire-only, DC-PUMP-03):"
    echo "$ka_body" | strip_comments | grep -nE 'AdmissionPeerEvent'
fi

# Guard 3: the cadence tick block SENDS MsgKeepAlive + emits no semantic event.
tick_block=$(awk '/keep_alive_timer\.tick\(\) =>/{f=1;n=0} f{print;n++; if(n>=44) exit}' "$PUMP")
if ! echo "$tick_block" | grep -q 'AcceptedMiniProtocol::KeepAlive'; then
    print_fail "keep-alive cadence must enqueue an OutboundFrame on AcceptedMiniProtocol::KeepAlive"
fi
if ! echo "$tick_block" | grep -q 'encode_keep_alive_message('; then
    print_fail "keep-alive cadence must encode via the BLUE encode_keep_alive_message"
fi
if echo "$tick_block" | strip_comments | grep -qE 'AdmissionPeerEvent::(Block|TipUpdate|RollBackward)|tip_update\('; then
    print_fail "keep-alive cadence must not synthesize a semantic AdmissionPeerEvent (DC-PUMP-01 preserved):"
    echo "$tick_block" | strip_comments | grep -nE 'AdmissionPeerEvent::(Block|TipUpdate|RollBackward)|tip_update\('
fi

# Guard 4: REUSE not redefine -- no local keep-alive grammar in the RED pump.
pump_code=$(strip_comments < "$PUMP")
if echo "$pump_code" | grep -qE 'fn keep_alive_transition\b|enum KeepAliveMessage\b|enum KeepAliveState\b'; then
    print_fail "wire_pump.rs must REUSE ade_network::keep_alive, not redefine the grammar:"
    echo "$pump_code" | grep -nE 'fn keep_alive_transition\b|enum KeepAliveMessage\b|enum KeepAliveState\b'
fi
if ! grep -q 'use ade_network::keep_alive::' "$PUMP"; then
    print_fail "wire_pump.rs must import the BLUE ade_network::keep_alive grammar (reuse)"
fi

if (( FAILED == 0 )); then
    echo "OK: wire-pump keep-alive client is wire-only + reuses the BLUE grammar (DC-PUMP-03)"
fi
exit "$FAILED"
