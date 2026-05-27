#!/usr/bin/env bash
#
# ci_check_no_produce_mode_direct_transport_writes.sh — PHASE4-N-S-B B4 gate.
#
# Enforces CN-OUTBOUND-RELAY-01's "no Vec<u8> byte tunnel"
# rule: produce_mode MUST NOT write directly to
# MuxTransportHandle::outbound. The only outbound API is
# `peer_outbound.get(&peer_id)?.send(OutboundCommand { ... })`,
# which is processed by MuxPump's session-aware encoder.
#
# Heuristic: search for any line in ade_node/src/produce_mode.rs
# that writes to a MuxTransportHandle::outbound field. Allow
# OutboundCommand-typed sends (these go through the relay).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

TARGET="crates/ade_node/src/produce_mode.rs"

if [[ ! -f "$TARGET" ]]; then
  echo "[ci_check_no_produce_mode_direct_transport_writes] FAIL — target file missing: $TARGET"
  exit 1
fi

# Reject any direct `transport.outbound.send`, `MuxTransportHandle`,
# or `transport_back_out` reference in produce_mode.rs. These
# would be a direct bypass of the session-aware encoder.
VIOLATIONS=$(
  grep -nE "transport\.outbound\.send|MuxTransportHandle\b|transport_back_out" "$TARGET" 2>/dev/null || true
)

if [[ -n "$VIOLATIONS" ]]; then
  echo "[ci_check_no_produce_mode_direct_transport_writes] FAIL — direct transport.outbound write(s) detected:"
  echo "$VIOLATIONS" | sed 's/^/  /'
  echo ""
  echo "  produce_mode MUST NOT bypass MuxPump's session-aware encoder."
  echo "  The only outbound API is:"
  echo "    peer_outbound.read().await"
  echo "      .get(&PeerId)?"
  echo "      .try_send(OutboundCommand { ... })"
  exit 1
fi

echo "[ci_check_no_produce_mode_direct_transport_writes] PASS"
