#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-E S1 — live-feed bounded-memory constants (DC-LIVEMEM-01).
#
# The two defensive live-feed memory bounds are CLOSED CONSTANTS: they exist as
# compile-time literals and are NOT wired to CLI / env / config — no runtime
# option may disable them or set them unbounded. (Defensive implementation
# bounds, NOT Cardano semantic parameters; tightenable by a future hardening
# slice, never disableable at runtime.)
#
# Guards:
#   (1) MAX_REASSEMBLY_TAIL_BYTES is the closed 16 MiB literal const (GREEN
#       session::core reassembly-tail cap).
#   (2) MAX_WIRE_PUMP_LOOKAHEAD is the closed 256 literal const (RED node_sync
#       WirePump lookahead-depth cap).
#   (3) NO escape hatch — no line referencing either bound also reads CLI / env
#       / config (no runtime override path). Line comments are stripped first so
#       the doc-comments naming "CLI / env / config" do not self-trip.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$REPO_ROOT/crates/ade_network/src/session/core.rs"
SYNC="$REPO_ROOT/crates/ade_node/src/node_sync.rs"

FAIL=0
print_fail() { echo "FAIL (live-feed memory bounds): $1"; FAIL=1; }

[[ -f "$CORE" ]] || print_fail "missing $CORE"
[[ -f "$SYNC" ]] || print_fail "missing $SYNC"

# --- guard (1): reassembly-tail cap is a closed 16 MiB literal const ---------
if [[ -f "$CORE" ]] && ! grep -qE 'const MAX_REASSEMBLY_TAIL_BYTES: usize = 16 \* 1024 \* 1024;' "$CORE"; then
    print_fail "MAX_REASSEMBLY_TAIL_BYTES is not the closed 16 MiB literal const in session/core.rs"
fi

# --- guard (2): lookahead-depth cap is a closed 256 literal const ------------
if [[ -f "$SYNC" ]] && ! grep -qE 'const MAX_WIRE_PUMP_LOOKAHEAD: usize = 256;' "$SYNC"; then
    print_fail "MAX_WIRE_PUMP_LOOKAHEAD is not the closed 256 literal const in node_sync.rs"
fi

# --- guard (3): no CLI / env / config escape hatch on either bound -----------
ESCAPE_RE='env::|std::env|::var\(|from_env|getenv|[Cc]li|clap|from_config|config\.'
for f in "$CORE" "$SYNC"; do
    [[ -f "$f" ]] || continue
    BAD="$(sed -E 's://.*$::' "$f" | grep -E 'MAX_REASSEMBLY_TAIL_BYTES|MAX_WIRE_PUMP_LOOKAHEAD' | grep -E "$ESCAPE_RE" || true)"
    if [[ -n "$BAD" ]]; then
        print_fail "a live-feed memory bound is wired to CLI/env/config (escape hatch) in $(basename "$f"): $BAD"
    fi
done

if (( FAIL == 0 )); then
    echo "OK (live-feed memory bounds): MAX_REASSEMBLY_TAIL_BYTES (16 MiB) + MAX_WIRE_PUMP_LOOKAHEAD (256) are closed literal constants, not wired to CLI/env/config (DC-LIVEMEM-01)."
fi
exit $FAIL
