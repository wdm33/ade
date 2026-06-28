#!/usr/bin/env bash
set -uo pipefail

# DC-EPOCH-21 (LIVE-LEDGER-EPOCH-TRANSITION S3): the epoch-boundary POOLREAP is ONE canonical transition,
# in the cardano order, whose halves cannot silently fail to compose. Before S3 the reap was SPLIT across
# apply_epoch_boundary_with_registrations (inline retirement: deposits, `<= e`, NO clear) and a trailing
# delegation::apply_pool_reap call in cross_epoch_boundary -- and the inline retirement emptied `retiring`
# FIRST, so the trailing reap's `== e` match found nothing and the delegation-clear was DEAD CODE. S3
# consolidates the single canonical order (adopt -> reap `== e` -> discriminant-correct refund + treasury
# -> clear delegators -> remove) inside the shared boundary fn. Mechanical enforcement (IDD principle 10)
# of the structure the behavioural unit tests (poolreap_ce3a::*) exercise but do not pin structurally:
#   (A) STRICT `== e`: the retirement predicate is `retire_epoch.0 == new_epoch.0`, never `<= new_epoch.0`
#       (a `<= e` wrongly reaps stale retirements).
#   (B) DISCRIMINANT-CORRECT REFUND: the deposit refund target decodes the real key/script discriminant via
#       reward_account_credential, not a bare KeyHash projection of the reward-account bytes.
#   (C) LIVE CLEAR: the boundary fn itself clears the reaped pools' delegators (`!retired.contains(pool_id)`)
#       -- the clear runs where the reap happens, so it can never be dead.
#   (D) SINGLE POOLREAP: cross_epoch_boundary does NOT call apply_pool_reap -- there is no trailing reap to
#       compose (and thus no dead-clear / double-apply seam).
#   (E) DC-EPOCH-21 is in the registry.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RULES="$REPO_ROOT/crates/ade_ledger/src/rules.rs"
ACC="$REPO_ROOT/crates/ade_ledger/src/epoch_accumulator.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$RULES" "$ACC" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done
[[ $FAILED -eq 0 ]] || exit 1

# (A) STRICT `== e`. The boundary POOLREAP reaps ONLY exact-epoch retirements.
grep -q 'retire_epoch.0 == new_epoch.0' "$RULES" \
    || print_fail "(A) the POOLREAP retirement predicate 'retire_epoch.0 == new_epoch.0' is missing"
if grep -q 'retire_epoch.0 <= new_epoch.0' "$RULES"; then
    print_fail "(A) a '<= new_epoch.0' retirement predicate is present -- POOLREAP must reap EXACTLY '== e'"
fi

# (B) DISCRIMINANT-CORRECT REFUND. The deposit refund routes by the real key/script discriminant.
grep -q 'reward_account_credential' "$RULES" \
    || print_fail "(B) the POOLREAP refund does not decode the reward-account discriminant (reward_account_credential)"

# (C) LIVE CLEAR. The boundary fn clears the reaped pools' delegators in place.
grep -q '!retired.contains(pool_id)' "$RULES" \
    || print_fail "(C) the boundary fn does not clear the reaped pools' delegators (!retired.contains(pool_id))"

# (D) SINGLE POOLREAP. No trailing apply_pool_reap CALL in the accumulator boundary path (a call has a
#     paren; a doc-comment reference like [`apply_pool_reap`] does not, so it cannot false-FAIL).
if grep -Eq 'apply_pool_reap[[:space:]]*\(' "$ACC"; then
    print_fail "(D) cross_epoch_boundary still calls apply_pool_reap -- the single canonical POOLREAP lives inside the boundary fn; a trailing reap re-introduces the dead-clear/double-apply seam"
fi

# (E) DC-EPOCH-21 in the registry.
grep -q 'DC-EPOCH-21' "$REG" \
    || print_fail "(E) DC-EPOCH-21 is not declared in the invariant registry"

if [[ $FAILED -ne 0 ]]; then
    echo "DC-EPOCH-21 single-canonical-POOLREAP check FAILED" >&2
    exit 1
fi
echo "OK: the epoch-boundary POOLREAP is one canonical transition (== e; discriminant-correct refund; live clear; no trailing reap)"
