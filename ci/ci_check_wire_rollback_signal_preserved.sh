#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S4a — wire rollback signal preservation. Production code only
# (test modules stripped).
#
# Guards:
#   1. The dangerous downgrade is GONE: a rollback is no longer represented as a
#      TipUpdate only -- the point-discard pattern `RollBackward { point: _` is absent.
#   2. The closed `AdmissionPeerEvent::RollBackward { peer, point, tip }` variant
#      exists and the wire pump EMITS it (carrying point: Point).
#   3. Rollback-to-Origin fails closed (UnsupportedRollbackPoint).
#   4. Preservation only: the wire-pump chain-sync handler references no
#      apply_chain_event / StreamInput::RollBack / select_best_chain / chain_selector.
#   5. The live sync path (node_sync) has a latent skip-arm for RollBackward and
#      does NOT consume it yet (no apply_chain_event / StreamInput::RollBack) -- AI-S4b.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WP="$REPO_ROOT/crates/ade_runtime/src/admission/wire_pump.rs"
NS="$REPO_ROOT/crates/ade_node/src/node_sync.rs"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

[[ -f "$WP" ]] || fail "missing $WP"
[[ -f "$NS" ]] || fail "missing $NS"
WPP=$(strip_for_grep "$WP")

# 1. The rollback-as-TipUpdate-only downgrade is gone (point no longer discarded).
if echo "$WPP" | grep -qE 'RollBackward \{ point: _'; then
    fail "the rollback point is still discarded (RollBackward { point: _ ... } -> TipUpdate only)"
fi

# 2. The closed event variant exists + is emitted, carrying point: Point.
echo "$WPP" | grep -qE 'RollBackward \{' || fail "RollBackward variant/arm missing"
echo "$WPP" | grep -qE 'point: Point' || fail "the RollBackward event must carry point: Point"
echo "$WPP" | grep -qE 'AdmissionPeerEvent::RollBackward \{' \
    || fail "the wire pump does not emit AdmissionPeerEvent::RollBackward"

# 3. Origin fails closed.
echo "$WPP" | grep -qE 'UnsupportedRollbackPoint' \
    || fail "rollback-to-Origin is not fail-closed (UnsupportedRollbackPoint missing)"

# 4. Preservation only -- no consumption in the wire-pump chain-sync handler.
REGION=$(echo "$WPP" | awk '/async fn handle_chain_sync/{f=1} f{print} f&&/^}/{exit}')
for needle in apply_chain_event 'StreamInput::RollBack' select_best_chain chain_selector; do
    if echo "$REGION" | grep -qF "$needle"; then
        fail "wire-pump chain-sync handler must not reference ${needle} (preservation only)"
    fi
done

# 5. The live sync path skips RollBackward (latent), does not consume it yet.
NSP=$(strip_for_grep "$NS")
echo "$NSP" | grep -qE 'AdmissionPeerEvent::RollBackward \{ \.\. \}' \
    || fail "node_sync has no latent skip-arm for RollBackward"
for needle in apply_chain_event 'StreamInput::RollBack'; do
    if echo "$NSP" | grep -qF "$needle"; then
        fail "node_sync must not consume RollBackward yet (found ${needle}) -- that is AI-S4b"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: wire rollback signal preserved (AI-S4a, latent until AI-S4b)"
fi
exit $FAILED
