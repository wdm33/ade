#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-3 (DC-EVIEW-11, strengthening DC-EPOCH-03): the deterministic,
# fail-closed epoch-rebind seam. The current seed-epoch view stays authoritative until, at
# the immediate epoch transition, a fully-bound MATCHING N+1 EpochConsensusView atomically
# promotes; anything else (missing/stale/conflicting/wrongly-bound view, non-immediate
# epoch, unlocatable slot) fails closed. The live seam passes None today (S3f-4 supplies
# the bound view), so OffEpoch fails closed EXACTLY as the pre-seam wall -- no
# leader-election change. decide_epoch_rebind is PURE (deterministic / replay-equivalent).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
R=crates/ade_node/src/epoch_rebind.rs
S=crates/ade_node/src/node_sync.rs

test -f "$R" || fail "the epoch-rebind reducer ($R) is missing"

# (1) the pure reducer + the closed decision/reject sums.
grep -qE 'pub fn decide_epoch_rebind' "$R" || fail "decide_epoch_rebind missing"
grep -qE 'pub enum EpochRebindDecision' "$R" || fail "EpochRebindDecision missing"
for d in KeepCurrent 'Promote\(EpochConsensusView\)' 'FailClosed\(EpochRebindReject\)'; do
    grep -qE "$d" "$R" || fail "EpochRebindDecision is missing the $d arm"
done
for r in Unlocatable NotImmediateNext NoBoundView ViewMismatch; do
    grep -qE "$r" "$R" || fail "EpochRebindReject is missing the $r fail-closed variant"
done

# (2) the eligibility is STRICT: immediate-next epoch only + the view must match the bound
#     context (all bindings + verify_canonical_hash via EpochConsensusView::matches).
grep -qE 'seed_epoch\.0\.wrapping_add\(1\)|seed_epoch\.0 \+ 1' "$R" \
    || fail "the reducer does not restrict promotion to the IMMEDIATE next epoch"
grep -qF 'view.matches(bindings)' "$R" || fail "the reducer does not require the view to match the bound context"

# (3) the reducer is PURE -- no I/O / clock / rand / float in the seam.
if grep -qE 'std::fs|SystemTime|Instant::now|rand::|thread_rng|: f64|: f32' "$R"; then
    fail "the epoch-rebind reducer must be PURE (no I/O / clock / rand / float)"
fi

# (4) FAIL-SAFE live seam: the wall passes None (no bound view yet) -> OffEpoch fails
#     closed as before; the Promote arm is an empty no-op (no live rebind until S3f-4).
grep -qE 'decide_epoch_rebind\(admission, None\)' "$S" \
    || fail "the live seam does not pass None for the bound view -- OffEpoch must stay fail-closed until S3f-4"
grep -qE 'EpochRebindDecision::Promote\(_view\) => \{\}' "$S" \
    || fail "the live seam's Promote arm is not an empty no-op -- S3f-4 wiring must NOT be present yet"

# (5) the load-bearing proofs (the required S3f-3 proof set).
for t in simulated_transition_promotes_bound_n1_view same_epoch_keeps_current \
         off_epoch_without_bound_view_fails_closed not_immediate_next_fails_closed \
         unlocatable_fails_closed rejects_each_wrong_binding rejects_tampered_view_wrong_hash \
         replay_equivalent_deterministic crash_restart_redrives_same_decision_both_sides; do
    grep -qE "fn $t" "$R" || fail "the $t proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: epoch-rebind seam (DC-EVIEW-11; deterministic fail-closed reducer, immediate-next-only + matching-view-only promotion, live seam passes None = byte-identical, no S3f-4 wiring)"
fi
exit $FAILED
