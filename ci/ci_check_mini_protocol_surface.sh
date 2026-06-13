#!/usr/bin/env bash
# ci_check_mini_protocol_surface.sh -- DC-PROTO-03 (N2N) + DC-PROTO-04 (N2C).
#
# The full mini-protocol surface must be PRESENT: every named N2N / N2C mini-protocol has
# a closed codec module carrying a `roundtrip_every_variant` proof (its wire grammar exists
# and round-trips byte-identically). Structural surface-completeness anchor -- a named
# protocol cannot silently disappear. Behavioral half = the per-codec roundtrip/reject
# tests (also under CN-WIRE-07's closed-codec gate). Together = complete enforcement of the
# "full surface" claim.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CODEC="$REPO_ROOT/crates/ade_network/src/codec"
fail() { echo "FAIL (ci_check_mini_protocol_surface): $1" >&2; exit 1; }
[ -d "$CODEC" ] || fail "codec dir missing: $CODEC"

# DC-PROTO-03: Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing.
N2N=(handshake chain_sync block_fetch tx_submission keep_alive peer_sharing)
# DC-PROTO-04: Handshake (N2C), LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor.
N2C=(n2c_handshake local_chain_sync local_tx_submission local_state_query local_tx_monitor)

check_surface() {  # $1=label; rest=protocol codec stems
  local label="$1"; shift
  local p f
  for p in "$@"; do
    f="$CODEC/$p.rs"
    [ -f "$f" ] || { echo "  $label: missing codec $p.rs"; return 1; }
    grep -Eq 'fn roundtrip_every_variant' "$f" \
      || { echo "  $label: $p.rs lacks the roundtrip_every_variant closed-grammar proof"; return 1; }
  done
  return 0
}

if [ "${1:-}" = "--self-test" ]; then
  if check_surface "self-test" "${N2N[@]}" __synthetic_absent_protocol__ >/dev/null 2>&1; then
    echo "FAIL: surface check passed with a bogus missing protocol" >&2; exit 1
  fi
  echo "PASS: surface check detects a missing mini-protocol"; exit 0
fi

check_surface "N2N (DC-PROTO-03)" "${N2N[@]}" || fail "N2N mini-protocol surface incomplete (see above)"
check_surface "N2C (DC-PROTO-04)" "${N2C[@]}" || fail "N2C mini-protocol surface incomplete (see above)"
echo "OK: full N2N (${#N2N[@]}) + N2C (${#N2C[@]}) mini-protocol surface present, each with a closed-grammar roundtrip_every_variant proof (DC-PROTO-03/04)"
