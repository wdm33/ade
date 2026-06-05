#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AB S1 — outbound mux segmentation (CN-SESS-05).
#
# The GREEN session reducer (session/core.rs) must SEGMENT outbound mini-protocol
# payloads larger than MAX_PAYLOAD into ordered <= MAX_PAYLOAD mux frames (the
# outbound inverse of CN-SESS-04 inbound reassembly), reusing the SINGLE existing
# frame encoder authority (encode_inner_frame -> mux::frame::encode_frame), and
# fail closed above a FIXED, non-configurable MAX_OUTBOUND_PAYLOAD_BYTES. Fences:
#   (a) no second/alternate mux frame encoder — exactly one encode_frame( call +
#       one MuxFrame { construction, both inside encode_inner_frame;
#   (b) encode_inner_frame keeps its per-frame  > MAX_PAYLOAD  guard;
#   (c) handle_outbound owns segmentation (chunks(MAX_PAYLOAD) + the
#       MAX_OUTBOUND_PAYLOAD_BYTES fail-closed bound);
#   (d) MAX_OUTBOUND_PAYLOAD_BYTES is a fixed const literal, not runtime-configurable.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE_RS="$REPO_ROOT/crates/ade_network/src/session/core.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$CORE_RS" ]]; then
    echo "FAIL: missing $CORE_RS"
    exit 1
fi

# Production view: strip line comments + stop at the first #[cfg(test)] module.
PROD="$(awk '
    /^[[:space:]]*#\[cfg\(test\)\]/ { exit }
    { line = $0; sub(/\/\/.*$/, "", line); print line }
' "$CORE_RS")"

# Guard (a) — exactly ONE frame encoder authority. A parallel/duplicate encoder
# in the session layer (e.g. handle_outbound building frames directly instead of
# delegating to encode_inner_frame) pushes either count above 1.
N_ENCODE=$(echo "$PROD" | grep -cE 'encode_frame[[:space:]]*\(')
N_MUXFRAME=$(echo "$PROD" | grep -cE 'MuxFrame[[:space:]]*\{')
if [[ "$N_ENCODE" -ne 1 ]]; then
    print_fail "expected exactly 1 encode_frame( call in session/core.rs (single encoder authority), found $N_ENCODE"
fi
if [[ "$N_MUXFRAME" -ne 1 ]]; then
    print_fail "expected exactly 1 'MuxFrame {' construction in session/core.rs (single encoder authority), found $N_MUXFRAME"
fi

# Guard (b) — encode_inner_frame keeps the per-frame > MAX_PAYLOAD guard.
if ! echo "$PROD" | grep -qE 'payload\.len\(\)[[:space:]]*>[[:space:]]*MAX_PAYLOAD'; then
    print_fail "encode_inner_frame's per-frame guard (payload.len() > MAX_PAYLOAD) is missing"
fi

# Guard (c) — handle_outbound owns segmentation: chunks at MAX_PAYLOAD and fails
# closed above the OUTBOUND ceiling.
if ! echo "$PROD" | grep -qE 'chunks[[:space:]]*\([[:space:]]*MAX_PAYLOAD'; then
    print_fail "outbound segmentation (chunks(MAX_PAYLOAD)) is missing from session/core.rs"
fi
if ! echo "$PROD" | grep -qE 'payload\.len\(\)[[:space:]]*>[[:space:]]*MAX_OUTBOUND_PAYLOAD_BYTES'; then
    print_fail "handle_outbound does not fail closed above MAX_OUTBOUND_PAYLOAD_BYTES"
fi

# Guard (d) — MAX_OUTBOUND_PAYLOAD_BYTES is a FIXED const literal, never runtime-
# configurable (no CLI/env/config read in the reducer).
if ! echo "$PROD" | grep -qE 'const MAX_OUTBOUND_PAYLOAD_BYTES:[[:space:]]*usize[[:space:]]*=[[:space:]]*[0-9].*;'; then
    print_fail "MAX_OUTBOUND_PAYLOAD_BYTES is not a fixed 'const ...: usize = <literal>;'"
fi
if echo "$PROD" | grep -qE 'std::env|env::var|env!\(|option_env!\('; then
    print_fail "session/core.rs reads the environment — the outbound bound must be a fixed compile-time const:"
    echo "$PROD" | grep -nE 'std::env|env::var|env!\(|option_env!\('
fi

if (( FAILED == 0 )); then
    echo "OK: outbound mux segmentation (CN-SESS-05) — single encoder authority, encode_inner_frame per-frame guard intact, handle_outbound segments at MAX_PAYLOAD + fails closed above the fixed MAX_OUTBOUND_PAYLOAD_BYTES"
fi
exit $FAILED
