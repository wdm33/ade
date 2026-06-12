#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S3 — live fork-choice apply driver (DC-NODE-25 + DC-NODE-26;
# CE-AI-1 production half). Production code only (test modules stripped).
#
# Guards:
#   1. apply_chain_event reuses the existing authorities (materialize_rolled_back_state,
#      commit_rollback, pump_block) and appends WalEntry::RollBack — never a second
#      rollback/admit/materialize implementation.
#   2. The driver APPLIES a decision, never makes one: no select_best_chain /
#      fork_choice / chain_selector, and no direct ChainDb rollback_to_slot
#      (commit_rollback owns the ChainDb rollback via ChainDbWriter).
#   3. WAL-after-commit ordering: WalEntry::RollBack is appended ONLY after
#      commit_rollback — structurally guaranteeing a failed commit appends no WAL
#      (append-only durability never lies about state).
#   4. The GREEN reconciliation helper (DC-NODE-26) exists.
#   5. The AI-S3 hermetic tests are present.
#   6. The reused-authority gates exist (run separately; must stay green).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NL="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
NS="$REPO_ROOT/crates/ade_node/src/node_sync.rs"
TEST="$REPO_ROOT/crates/ade_node/tests/apply_driver_ai_s3.rs"

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
[[ -f "$TEST" ]] || fail "missing $TEST"

# NOTE: greps use here-strings (`<<<`), NOT `echo "$VAR" | grep -q` / `strip | grep -q`,
# which under `set -o pipefail` false-fail when grep -q matches early and SIGPIPEs the
# producer of a large stripped file (bit this gate after AI-S4b-ii grew node_sync).
PROD=$(strip_for_grep "$NL")
grep -qE 'pub fn apply_chain_event' <<< "$PROD" || fail "apply_chain_event missing"

# The apply_chain_event region -- bounded at the NEXT function definition after
# apply_chain_event (PHASE4-N-AO inserted the S3/S4 fork-switch helpers, which
# legitimately call select_best_chain, between apply_chain_event and
# run_participant_sync; the region must be apply_chain_event's body ONLY).
REGION=$(awk '
    /pub fn apply_chain_event/{f=1; print; next}
    f && /^[[:space:]]*(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]/{f=0}
    f{print}
' <<< "$PROD")

# 1. Reuse, not reimplement.
for needle in materialize_rolled_back_state commit_rollback pump_block 'WalEntry::RollBack'; do
    grep -qF "$needle" <<< "$REGION" || fail "apply_chain_event must use ${needle} (reuse, not reimplement)"
done

# 2. Applies, never selects; no second rollback path.
for needle in select_best_chain fork_choice chain_selector; do
    if grep -qE "\b${needle}\b" <<< "$REGION"; then
        fail "apply_chain_event must not reference ${needle} (the orchestrator owns selection — DC-CONS-03)"
    fi
done
if grep -qE 'rollback_to_slot' <<< "$REGION"; then
    fail "apply_chain_event must not call rollback_to_slot directly (commit_rollback owns the ChainDb rollback)"
fi

# 3. WAL-after-commit ordering (commit-fail => no WAL).
COMMIT_LN=$(grep -nE 'commit_rollback\(' <<< "$REGION" | head -1 | cut -d: -f1)
WALAPP_LN=$(grep -nE 'wal\.append\(WalEntry::RollBack' <<< "$REGION" | head -1 | cut -d: -f1)
if [[ -z "${COMMIT_LN:-}" || -z "${WALAPP_LN:-}" ]]; then
    fail "could not locate commit_rollback / wal.append(WalEntry::RollBack) for the ordering check"
elif (( WALAPP_LN <= COMMIT_LN )); then
    fail "WalEntry::RollBack must be appended AFTER commit_rollback (append@${WALAPP_LN} <= commit@${COMMIT_LN})"
fi

# 4. GREEN reconciliation helper (DC-NODE-26).
NSP=$(strip_for_grep "$NS")
grep -qE 'pub fn durable_tip_matches' <<< "$NSP" \
    || fail "durable_tip_matches (DC-NODE-26 reconciliation) missing"

# 5. Hermetic tests present.
for t in \
    apply_rolledback_rolls_back_and_appends_wal_record_after_commit \
    apply_rolledback_replays_byte_identical_recovers_forkpoint \
    apply_rollback_no_snapshot_fails_closed_appends_no_wal \
    apply_reconciliation_mismatch_fails_fast \
    apply_rejected_makes_no_durable_change \
    apply_chain_selected_invalid_body_fails_via_pump_no_advance ; do
    grep -qF "$t" "$TEST" || fail "missing AI-S3 test: $t"
done

# 6. Reused-authority gates exist (must stay green; run separately in CI).
for g in \
    ci_check_rollback_materialize_closure.sh \
    ci_check_receive_reducer_closure.sh \
    ci_check_wal_rollback_replay_equiv.sh ; do
    [[ -f "$REPO_ROOT/ci/$g" ]] || fail "reused-authority gate missing: $g"
done

if (( FAILED == 0 )); then
    echo "OK: live fork-choice apply driver (DC-NODE-25 + DC-NODE-26; CE-AI-1 production half)"
fi
exit $FAILED
