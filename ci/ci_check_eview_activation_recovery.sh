#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-4c (DC-EPOCH-06): activation is durable-before-visible and
# replay-identical. The activation WAL record is written BEFORE the active view is published
# (a non-durable write halts: terminal EpochViewActivationFailed). On recovery: no record ->
# Seed (old epoch active); a record + a re-derived view that reproduces its identity ->
# republish the SAME view (recovered == WAL); a record + a mismatched/absent view -> terminal
# EpochViewPostPromotionMismatch (NEVER fallback). Repeated records fold via the DC-EPOCH-04
# idempotence/conflict rule.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
A=crates/ade_node/src/epoch_activation.rs

# (1) durable-before-visible: a non-durable WAL write is terminal, never a publish.
grep -qE 'pub fn activate_durable_before_visible' "$A" || fail "activate_durable_before_visible missing"
grep -qF 'return Err(EpochViewActivationError::EpochViewActivationFailed)' "$A" \
    || fail "a non-durable WAL write does not halt (terminal) before publication"

# (2) recovery reconstruction: no record -> Seed; record+match -> Promoted; else terminal mismatch.
grep -qE 'pub fn recover_active_view' "$A" || fail "recover_active_view missing"
grep -qF '(None, _) => Ok(ActiveEpochView::Seed)' "$A" \
    || fail "crash before durable WAL must keep the seed epoch active"
grep -qF 'EpochViewActivationError::EpochViewPostPromotionMismatch' "$A" \
    || fail "a recovery mismatch is not terminal (PostPromotionMismatch) -- fallback hazard"

# (3) the identity match is COMPLETE (every binding + both hashes + verify_canonical_hash).
grep -qE 'fn activation_record_matches' "$A" || fail "activation_record_matches missing"
grep -qF 'candidate.verify_canonical_hash()' "$A" || fail "the recovery match does not verify the candidate hash"
grep -qF 'candidate.canonical_hash() == *view_canonical_hash' "$A" || fail "the recovery match does not pin the full-view hash"
grep -qF 'candidate.stake_view_canonical_hash() == *stake_view_canonical_hash' "$A" || fail "the recovery match does not pin the stake-view hash"

# (4) the replay application folds DC-EPOCH-04 idempotence/conflict on recovery.
grep -qE 'pub fn resolve_activation_record' "$A" || fail "resolve_activation_record missing"
grep -qF 'ActivationReplayOutcome::Conflict' "$A" || fail "the WAL fold does not surface a conflict as terminal"

# (5) the record builder (the live emit, S3f-4d, writes this before publishing).
grep -qE 'pub fn activation_record_for' "$A" || fail "activation_record_for (the WAL record builder) missing"

# (6) the load-bearing crash-recovery proofs.
for t in crash_before_durable_wal_keeps_seed crash_after_wal_republishes_same_view \
         recovered_view_mismatch_is_terminal durable_before_visible_halts_on_wal_failure \
         resolve_activation_idempotent_conflict_supersede; do
    grep -qE "fn $t" "$A" || fail "the $t proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: activation recovery (DC-EPOCH-06; durable-before-visible, replay-identical recovery, complete identity match, terminal-not-fallback)"
fi
exit $FAILED
