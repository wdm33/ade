#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-4b (DC-EPOCH-05 / DC-EPOCH-07): the activation predicate, the
# atomically-published active view, and the terminal activation states. The gate is the
# PREDICATE (not a flag): all bindings verify + WAL durable + selected point correct +
# transition eligible => promote. The active view is a ONE-WAY Seed->Promoted transition
# (DC-EPOCH-05: N+1 leadership reads only the promoted view; no seed-after-promotion). A
# conflicting/failed/mismatched activation is a TERMINAL state (DC-EPOCH-07), never fallback.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
A=crates/ade_node/src/epoch_activation.rs

test -f "$A" || fail "the activation module ($A) is missing"

# (1) the predicate is the gate (NOT a flag) and checks ALL four preconditions.
grep -qE 'pub fn activation_predicate' "$A" || fail "activation_predicate missing"
for r in TransitionIneligible BindingsUnverified WrongSelectedPoint WalNotDurable; do
    grep -qE "ActivationReject::$r" "$A" || fail "the predicate is missing the $r precondition"
done
grep -qF 'candidate.matches(n1_bindings)' "$A" || fail "the predicate does not verify the bindings (matches)"
grep -qF 'candidate.source_point != *selected_point' "$A" || fail "the predicate does not check the selected-chain point"

# (2) the terminal fail-closed states (DC-EPOCH-07) -- never fallback.
for t in EpochViewActivationFailed EpochViewActivationConflict EpochViewPostPromotionMismatch; do
    grep -qE "$t" "$A" || fail "the terminal $t state is missing"
done

# (3) the active view is a ONE-WAY Seed->Promoted transition (DC-EPOCH-05) with a terminal
#     conflict on a differing re-promotion -- NOT a "choose old or new by config" flag.
grep -qE 'pub enum ActiveEpochView' "$A" || fail "ActiveEpochView missing"
grep -qE 'Seed' "$A" || fail "ActiveEpochView::Seed missing"
grep -qE 'Promoted\(EpochConsensusView\)' "$A" || fail "ActiveEpochView::Promoted missing"
grep -qF 'EpochViewActivationError::EpochViewActivationConflict' "$A" \
    || fail "a differing re-promotion is not a terminal conflict (silent-swap hazard)"

# (4) NO runtime flag / config-select alternate consensus mode.
if grep -qiE 'env::var|feature_flag|cfg!\(feature|ACTIVATION_ENABLED|if .*flag' "$A"; then
    fail "the activation must be ONE atomic path -- no runtime flag / config-select mode"
fi

# (5) the load-bearing proofs.
for t in predicate_promotes_only_when_every_precondition_holds predicate_rejects_each_failed_precondition \
         active_view_one_way_promote_and_idempotence active_view_conflicting_promotion_is_terminal \
         seed_exposes_no_n1_view_until_promotion; do
    grep -qE "fn $t" "$A" || fail "the $t proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: activation predicate + active view (DC-EPOCH-05/07; predicate-gated not flagged, one-way Seed->Promoted, terminal fail-closed never fallback)"
fi
exit $FAILED
