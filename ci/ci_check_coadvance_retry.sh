#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-8 (DC-EPOCH-46) -- a co-advance pass never leaves leadership stale only because the reduced
# checkpoint was behind the accumulator walk.
#
# LV-7 measured the defect: at preprod 130,350,133 and 131,626,888 the walk asked the reduced checkpoint
# for a collateral value created after the checkpoint's cursor, the observe-only ApplyFailedAt arm stopped
# the walk, and the crossing that seals leadership for N+2 never ran.
#
# Asserts, over the PRODUCTION body of crates/ade_node/src/node_lifecycle.rs:
#  (a) ONE retry decision, classify_walk_apply_failure: it matches only the typed
#      UnresolvedCollateralInput, with the checkpoint cursor STRICTLY before the failing block and the
#      failing block in the accumulator's own epoch (so a retry never carries the checkpoint across a
#      boundary the accumulator has not crossed);
#  (b) it has exactly one production caller, the walk loop's ApplyFailedAt arm;
#  (c) that arm is bounded (one retry per failing block, recorded once), advances the checkpoint to the
#      failing block before retrying, halts with the typed AccumulatorWalkHalted otherwise, and has no
#      observe-only exit (no `break`, no "observe-only" log);
#  (d) the walk's Err arm halts with AccumulatorWalkHalted, with no observe-only exit;
#  (e) the CE tests exist and ran.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

NL="crates/ade_node/src/node_lifecycle.rs"
[[ -f "$NL" ]] || { echo "FAIL (co-advance retry): $NL not found"; exit 1; }

FAILED=0
fail() { echo "FAIL (co-advance retry): $1"; FAILED=1; }

# Production body: drop EVERY #[cfg(test)] module and strip line comments so commentary cannot satisfy
# or trip a grep.
prod_body() {
    awk '
        /^#\[cfg\(test\)\]$/ { pend = $0; holding = 1; next }
        holding && /^#\[allow\(/ { pend = pend "\n" $0; next }
        holding && /^mod .*\{$/ { skipping = 1; holding = 0; next }
        holding { print pend; holding = 0 }
        skipping { if ($0 ~ /^}$/) skipping = 0; next }
        { print }
    ' "$1" | sed -E 's://.*::'
}
isolate() {
    awk -v pat="$1" '$0 ~ pat { c=1 } c { print } c && /^}/ { exit }' <<<"$2"
}
# Lines from the first match of $1 up to (not including) the first later match of $2.
between() {
    awk -v a="$1" -v b="$2" 'c && $0 ~ b { exit } $0 ~ a { c=1 } c { print }' <<<"$3"
}

NL_PROD="$(prod_body "$NL")"

# --- (a) one retry decision, exact ----------------------------------------------------------------
DEFS="$(grep -cE 'fn classify_walk_apply_failure\(' <<<"$NL_PROD" || true)"
[[ "$DEFS" == "1" ]] || fail "(a) classify_walk_apply_failure is defined ${DEFS:-0} times, expected 1"
CLS="$(isolate '^pub\(crate\) fn classify_walk_apply_failure\(' "$NL_PROD")"
[[ -n "$CLS" ]] || fail "(a) could not isolate classify_walk_apply_failure"
grep -qE 'CollateralBalanceError::UnresolvedCollateralInput \{ \.\. \}' <<<"$CLS" \
    || fail "(a) the retry decision does not match the typed UnresolvedCollateralInput"
grep -qE '\) if cursor < failed_at && same_epoch => WalkApplyFailure::CheckpointBehind,' <<<"$CLS" \
    || fail "(a) CheckpointBehind is not gated on the checkpoint cursor STRICTLY before the failing block AND the same epoch"
grep -qE '\(Some\(b\), Some\(a\)\) if b == a' <<<"$CLS" \
    || fail "(a) the same-epoch condition does not require both epochs known and equal"
N_BEHIND="$(grep -cE 'WalkApplyFailure::CheckpointBehind' <<<"$CLS" || true)"
[[ "$N_BEHIND" == "1" ]] || fail "(a) CheckpointBehind is produced ${N_BEHIND:-0} times, expected 1"
grep -qE '_ => WalkApplyFailure::Terminal,' <<<"$CLS" || fail "(a) every other case is not Terminal"

# --- (b) exactly one production caller -------------------------------------------------------------
CALLS="$(grep -cE '&& classify_walk_apply_failure\($' <<<"$NL_PROD" || true)"
[[ "$CALLS" == "1" ]] || fail "(b) classify_walk_apply_failure has ${CALLS:-0} production callers, expected 1"

# --- (c) the ApplyFailedAt arm ----------------------------------------------------------------------
COADV="$(isolate '^fn advance_ledger_state_to_durable_tip_memo\(' "$NL_PROD")"
[[ -n "$COADV" ]] || fail "(c) could not isolate advance_ledger_state_to_durable_tip_memo"
ARM="$(between 'Ok\(AccumulatorChaindbOutcome::ApplyFailedAt \{' 'Ok\(AccumulatorChaindbOutcome::BoundaryRequiredAt \{' "$COADV")"
[[ -n "$ARM" ]] || fail "(c) could not isolate the ApplyFailedAt arm"
CALL="$(awk '/&& classify_walk_apply_failure\($/ { c=1 } c { print } c && /\) == WalkApplyFailure::CheckpointBehind$/ { exit }' <<<"$ARM")"
tr -d ' \n' <<<"$CALL" | grep -qF '&&classify_walk_apply_failure(&error,cp_cursor,s_bb,failed_block_epoch,accumulator_epoch,)==WalkApplyFailure::CheckpointBehind' \
    || fail "(c) the arm does not classify with the checkpoint's cursor, the failing block and both epochs"
grep -qE 'let failed_block_epoch = era_schedule\.locate\(s_bb\)\.ok\(\)\.map\(\|l\| l\.epoch\);' <<<"$ARM" \
    || fail "(c) the failing block's epoch is not placed by the era schedule"
grep -qE '\.map\(\|\(_, acc\)\| acc\.epoch_state\.epoch\);' <<<"$ARM" \
    || fail "(c) the accumulator's epoch is not read from the store"
grep -qE 'let retried = retried_at == Some\(s_bb\);' <<<"$ARM" || fail "(c) the retry is not bounded per failing block"
grep -qE '^\s*if !retried$' <<<"$ARM" || fail "(c) a spent retry is not excluded from the retry branch"
grep -qE 'advance_reduced_checkpoint_forward_to\(reduced_checkpoint, chaindb, s_bb\)\?;' <<<"$ARM" \
    || fail "(c) the retry does not advance the checkpoint to the failing block first"
N_SET="$(grep -cE 'retried_at = Some\(s_bb\);' <<<"$COADV" || true)"
[[ "$N_SET" == "1" ]] || fail "(c) the retry marker is set ${N_SET:-0} times, expected 1"
N_CONT="$(grep -cE '^\s*continue;$' <<<"$ARM" || true)"
[[ "$N_CONT" == "1" ]] || fail "(c) the arm retries from ${N_CONT:-0} places, expected 1"
grep -qE 'return Err\(NodeLifecycleError::AccumulatorWalkHalted \{' <<<"$ARM" \
    || fail "(c) the arm does not halt with AccumulatorWalkHalted"
grep -qE '^\s*break;$' <<<"$ARM" && fail "(c) the ApplyFailedAt arm still has an observe-only break"
grep -qiE 'observe-only' <<<"$ARM" && fail "(c) the ApplyFailedAt arm still logs observe-only"
grep -qE 'stalled_boundary' <<<"$ARM" && fail "(c) the ApplyFailedAt arm still writes the stall memo"

# --- (d) the walk's Err arm -------------------------------------------------------------------------
WALK_ERR="$(awk '/let cause = format!\("walk fault: \{e:\?\}"\);/ { c=1 } c { print } c && /^\s*\}\);$/ { exit }' <<<"$COADV")"
[[ -n "$WALK_ERR" ]] || fail "(d) could not find the walk Err arm's halt cause"
grep -qE 'return Err\(NodeLifecycleError::AccumulatorWalkHalted \{' <<<"$WALK_ERR" \
    || fail "(d) the walk Err arm does not halt with AccumulatorWalkHalted"
grep -qE 'within-epoch reconcile fault \(observe-only\)' <<<"$COADV" \
    && fail "(d) the walk Err arm still logs observe-only"

# --- (e) the CE tests exist and ran -----------------------------------------------------------------
TESTS=(
    'lv8_pair_a_retries_once_and_seals_n_plus_2'
    'lv8_pair_b_retries_once_and_seals_n_plus_2'
    'lv8_a_retry_that_fails_again_halts_at_the_block'
    'lv8_not_checkpoint_behind_halts_without_a_retry'
    'lv8_classification_is_exact'
    'lv8_a_walk_fault_halts'
    'lv8_a_dependency_created_across_a_boundary_crosses_first_then_retries'
)
OUT="$(cargo test -p ade_node --lib -- lv8_ 2>&1 || true)"
for t in "${TESTS[@]}"; do
    grep -qE "^test .*::$t \.\.\. ok$" <<<"$OUT" || fail "(e) unit test $t did not run and pass"
done
grep -qE '^test result: ok\. 7 passed;' <<<"$OUT" || fail "(e) expected exactly 7 passing LV-8 tests"

if (( FAILED == 0 )); then
    echo "OK (co-advance retry): one exact retry decision (same epoch as the accumulator, checkpoint strictly behind) with one caller; the ApplyFailedAt arm advances the checkpoint to the failing block and retries once per block, otherwise halts typed; the walk Err arm halts typed; no observe-only exit; CE tests ran (DC-EPOCH-46)"
fi
exit $FAILED
