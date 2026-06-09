#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S4b-ii — live rollback-follow routing + forge gate. Production
# code only (test modules stripped). Greps use here-strings (`<<<`), NOT
# `echo "$VAR" | grep -q` (pipefail+SIGPIPE false-fails on a large stripped file).
#
# Guards:
#   1. run_participant_sync reuses the authorities (classify_receive,
#      resolve_disposition, apply_chain_event, pump_block) + constructs
#      ChainEvent::RolledBack (it APPLIES the peer's rollback, builds no chain).
#   2. It APPLIES, never SELECTS: no process_stream_input / select_best_chain /
#      chain_selector in the live routing (single-best-peer follow; DC-CONS-03
#      stays the orchestrator's).
#   3. DC-NODE-28 ordering: pending_reselection is cleared ONLY AFTER
#      apply_chain_event returns (no forge slips through the window).
#   4. The SyncOnce venue branch: Participant -> run_participant_sync; SP/Unknown
#      keep run_node_sync.
#   5. The ForgeTick gate uses pending_reselection_forge_refusal.
#   6. node_sync surfaces the richer NodeSyncItem and QUEUES RollBackward (not drop).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NL="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
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

[[ -f "$NL" ]] || fail "missing $NL"
[[ -f "$NS" ]] || fail "missing $NS"
NLP=$(strip_for_grep "$NL")
NSP=$(strip_for_grep "$NS")

# 1. The Participant routing exists + reuses the authorities.
grep -qE 'pub async fn run_participant_sync' <<< "$NLP" || fail "run_participant_sync missing"
REGION=$(awk '/pub async fn run_participant_sync/{f=1} f{print}' <<< "$NLP")
for needle in classify_receive resolve_disposition apply_chain_event pump_block 'ChainEvent::RolledBack'; do
    grep -qF "$needle" <<< "$REGION" || fail "run_participant_sync must use ${needle}"
done

# 2. Applies, never selects.
for needle in process_stream_input select_best_chain chain_selector; do
    if grep -qF "$needle" <<< "$REGION"; then
        fail "run_participant_sync must not call ${needle} (single-best-peer follow; DC-CONS-03)"
    fi
done

# 3. DC-NODE-28 ordering: pending cleared only after apply returns.
APPLY_LN=$(grep -nF 'apply_chain_event(' <<< "$REGION" | head -1 | cut -d: -f1)
CLEAR_LN=$(grep -nE '\*pending_reselection = false' <<< "$REGION" | head -1 | cut -d: -f1)
if [[ -z "${APPLY_LN:-}" || -z "${CLEAR_LN:-}" ]]; then
    fail "could not locate apply_chain_event / pending-clear for the ordering check"
elif (( CLEAR_LN <= APPLY_LN )); then
    fail "pending_reselection must be cleared AFTER apply_chain_event (clear@${CLEAR_LN} <= apply@${APPLY_LN})"
fi

# 4. SyncOnce venue branch.
grep -qE 'VenueRole::Participant' <<< "$NLP" || fail "loop has no VenueRole::Participant branch"
grep -qE 'run_participant_sync\(' <<< "$NLP" || fail "loop does not call run_participant_sync"
grep -qE 'run_node_sync\(' <<< "$NLP" || fail "loop does not keep run_node_sync for SP/Unknown"

# 5. ForgeTick gate uses the pending helper.
grep -qE 'pending_reselection_forge_refusal' <<< "$NLP" \
    || fail "ForgeTick gate does not use pending_reselection_forge_refusal (DC-NODE-28)"

# 6. node_sync: richer item + RollBackward queued (not dropped).
grep -qE 'enum NodeSyncItem' <<< "$NSP" || fail "NodeSyncItem missing"
grep -qE 'NodeSyncItem::RollBack\(point\)' <<< "$NSP" \
    || fail "pump/wait must QUEUE RollBackward as NodeSyncItem::RollBack(point)"

if (( FAILED == 0 )); then
    echo "OK: live rollback-follow routing + forge gate (AI-S4b-ii)"
fi
exit $FAILED
