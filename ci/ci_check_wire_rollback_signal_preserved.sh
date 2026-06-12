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

# NOTE: greps use here-strings (`<<<`), NOT `echo "$VAR" | grep -q`, which under
# `set -o pipefail` flaky-false-fails when grep -q matches early and SIGPIPEs the
# echo (this gate intermittently red on check #2 before the conversion).

# 1. The rollback-as-TipUpdate-only downgrade is gone (point no longer discarded).
if grep -qE 'RollBackward \{ point: _' <<< "$WPP"; then
    fail "the rollback point is still discarded (RollBackward { point: _ ... } -> TipUpdate only)"
fi

# 2. The closed event variant exists + is emitted, carrying point: Point.
grep -qE 'RollBackward \{' <<< "$WPP" || fail "RollBackward variant/arm missing"
grep -qE 'point: Point' <<< "$WPP" || fail "the RollBackward event must carry point: Point"
grep -qE 'AdmissionPeerEvent::RollBackward \{' <<< "$WPP" \
    || fail "the wire pump does not emit AdmissionPeerEvent::RollBackward"

# 3. Origin fails closed.
grep -qE 'UnsupportedRollbackPoint' <<< "$WPP" \
    || fail "rollback-to-Origin is not fail-closed (UnsupportedRollbackPoint missing)"

# 4. Preservation only -- no consumption in the wire-pump chain-sync handler.
REGION=$(awk '/async fn handle_chain_sync/{f=1} f{print} f&&/^}/{exit}' <<< "$WPP")
for needle in apply_chain_event 'StreamInput::RollBack' select_best_chain chain_selector; do
    if grep -qF "$needle" <<< "$REGION"; then
        fail "wire-pump chain-sync handler must not reference ${needle} (preservation only)"
    fi
done

# 5. AI-S4b-ii flipped the AI-S4a latent skip: node_sync now CONSUMES RollBackward
#    as an ordered NodeSyncItem::RollBack. node_sync still does not call the live
#    apply itself (apply_chain_event / StreamInput live in node_lifecycle).
NSP=$(strip_for_grep "$NS")
# PHASE4-N-AO S1 (DC-NODE-34): NodeSyncItem is a struct variant carrying peer
# (`NodeSyncItem::RollBack { peer, point }`); the consumed-not-dropped invariant
# is unchanged.
grep -qE 'NodeSyncItem::RollBack \{' <<< "$NSP" \
    || fail "node_sync must queue RollBackward as NodeSyncItem::RollBack { peer, point }"
for needle in apply_chain_event 'StreamInput::RollBack'; do
    if grep -qF "$needle" <<< "$NSP"; then
        fail "node_sync must not call ${needle} (the live apply lives in node_lifecycle)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: wire rollback signal preserved + consumed as NodeSyncItem (AI-S4a + AI-S4b-ii)"
fi
exit $FAILED
