#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S6 (DC-NODE-29) — live rollback-target canonical binding (H-1
# remediation). The live RollBack arm (run_participant_sync) MUST resolve the wire
# hash against the durable ChainDb and use the STORED slot as the target authority;
# a peer slot != the stored slot for that hash fails closed (typed) BEFORE
# apply_chain_event -- i.e. before commit_rollback / WalEntry::RollBack / any
# durable mutation. The peer slot must never build the rollback target.
#
# Production code only (test modules stripped); greps use here-strings.

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

# The typed mismatch error exists (closed-enum fail-closed surface).
grep -qE 'RollbackPointSlotMismatch' <<< "$NSP" \
    || fail "NodeSyncError::RollbackPointSlotMismatch missing (the typed fail-closed)"

# Region: run_participant_sync (the last prod fn; apply_chain_event is defined before it).
REGION=$(awk '/pub async fn run_participant_sync/{f=1} f{print}' <<< "$NLP")
[[ -n "$REGION" ]] || fail "run_participant_sync not found"

# 1. The arm resolves the hash against the durable ChainDb + binds the STORED slot.
grep -qF 'get_block_by_hash' <<< "$REGION" \
    || fail "the RollBack arm must resolve the hash via get_block_by_hash"
grep -qE 'slot: stored\.slot' <<< "$REGION" \
    || fail "to_point must be built from stored.slot (the durable authority), not the wire slot"

# 2. The slot-mismatch fail-closed appears BEFORE apply_chain_event (pre-mutation).
MISMATCH_LN=$(grep -nE 'RollbackPointSlotMismatch' <<< "$REGION" | head -1 | cut -d: -f1)
APPLY_LN=$(grep -nF 'apply_chain_event(' <<< "$REGION" | head -1 | cut -d: -f1)
if [[ -z "${MISMATCH_LN:-}" || -z "${APPLY_LN:-}" ]]; then
    fail "could not locate the mismatch check / apply_chain_event in run_participant_sync"
elif (( MISMATCH_LN >= APPLY_LN )); then
    fail "the slot-mismatch fail-closed must be BEFORE apply_chain_event (mismatch@${MISMATCH_LN} >= apply@${APPLY_LN})"
fi

# 3. The wire slot is COMPARED to the stored slot (not silently trusted).
grep -qE 'slot != stored\.slot|stored\.slot != ' <<< "$REGION" \
    || fail "the arm must require peer.slot == stored.slot (compare the wire slot to the stored slot)"

if (( FAILED == 0 )); then
    echo "OK: rollback target canonical binding (AI-S6, DC-NODE-29)"
fi
exit $FAILED
