#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-1 (DC-EPOCH-40) -- the leader-check sigma denominator is a SNAPSHOT fact, and the
# leadership pool-set membership filter cannot move it.
#
# ORACLE (cardano-ledger, quoted at two points in its history; see
# docs/evidence/run-stores/preprod-live2c/leadervalue-oracle-extraction-sigma-denominator.md):
#   Conway  let total = sumAllStakeCompact stake      (VMap.foldl (<>) mempty . unStake)
#   master  ssTotalActiveStake = sumAllActiveStake ssActiveStake  in mkSnapShot, then
#           calculatePoolDistr' ... = PoolDistr { ..., pdTotalActiveStake = activeStake }
# In BOTH the denominator is folded over the STAKE (credential) map, and the membership guards
# (includeHash, spssNumDelegators > 0) filter unPoolDistr ONLY -- they run AFTER the total is fixed.
#
# Asserts:
#  (a) FrozenLeadershipPoolDistr carries total_active_stake;
#  (b) to_pool_distr_view READS it and contains NO summing loop and NO sum-based fallback -- a
#      "sum if it looks unset" fallback would reintroduce the defect on exactly the objects that
#      need the fix;
#  (c) StakeSnapshot::total_active_stake is the SOLE definition and folds `delegations` (the
#      credential side), NOT pool_stakes and NOT any filtered pool set;
#  (d) the boundary freeze reads that total from the mark BEFORE the membership filter, and the
#      bootstrap import does not derive it from mark_pool_distr (already the FILTERED PoolDistr);
#  (e) the field is inside the canonical encoding, so it is committed to;
#  (f) the schema and store-semantics versions moved together;
#  (g) the CE tests exist and are non-vacuous.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FROZEN="crates/ade_ledger/src/frozen_leadership.rs"
EPOCH="crates/ade_ledger/src/epoch.rs"
ACC="crates/ade_ledger/src/epoch_accumulator.rs"
FIRSTRUN="crates/ade_node/src/native_firstrun.rs"
SEMANTICS="crates/ade_ledger/src/store_semantics.rs"
TESTS="crates/ade_ledger/tests/lv1_sigma_denominator_authority.rs"

for f in "$FROZEN" "$EPOCH" "$ACC" "$FIRSTRUN" "$SEMANTICS" "$TESTS"; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL (sigma denominator): $f not found"
        exit 1
    fi
done

FAILED=0
fail() { echo "FAIL (sigma denominator): $1"; FAILED=1; }

# Production body only: truncate at the TRAILING #[cfg(test)] MODULE (an inline test shim must not
# blind this gate -- the ci_check_forge_followed_tip_admission lesson), and strip line comments so
# commentary naming a token cannot satisfy or trip a grep.
prod_body() {
    awk '
        /^#\[cfg\(test\)\]$/ { pend = $0; holding = 1; next }
        holding && /^#\[allow\(/ { pend = pend "\n" $0; next }
        holding && /^mod / { exit }
        holding { print pend; holding = 0 }
        { print }
    ' "$1" | sed -E 's://.*::'
}

isolate() {
    awk -v pat="$1" '
        $0 ~ pat { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    ' <<<"$2"
}

FROZEN_PROD="$(prod_body "$FROZEN")"
EPOCH_PROD="$(prod_body "$EPOCH")"
ACC_PROD="$(prod_body "$ACC")"
if [[ -z "$FROZEN_PROD" || -z "$EPOCH_PROD" || -z "$ACC_PROD" ]]; then
    echo "FAIL (sigma denominator): could not isolate production bodies"
    exit 1
fi

# --- (a) the field exists -------------------------------------------------------
grep -qE '^\s*pub total_active_stake: u64,' <<<"$FROZEN_PROD" \
    || fail "FrozenLeadershipPoolDistr has no total_active_stake -- the denominator must be CARRIED, not derived from the pool map"

# --- (b) to_pool_distr_view reads it, and does not sum --------------------------
VIEW_FN="$(awk '/pub fn to_pool_distr_view/{c=1} c{print} c&&/^    }$/{exit}' <<<"$FROZEN_PROD")"
if [[ -z "$VIEW_FN" ]]; then
    fail "could not isolate to_pool_distr_view"
else
    grep -qE 'self\.total_active_stake' <<<"$VIEW_FN" \
        || fail "to_pool_distr_view does not read self.total_active_stake"
    # The exact defect: accumulating entry stakes into the denominator.
    if grep -qE 'total_active_stake[^;]*(saturating_add|checked_add|\+=)' <<<"$VIEW_FN"; then
        fail "to_pool_distr_view ACCUMULATES into total_active_stake -- cardano fixes pdTotalActiveStake before its membership guards run, so summing the surviving entries is the defect LV-1 closes"
    fi
    grep -qE '\.sum\(\)|fold\(' <<<"$VIEW_FN" \
        && fail "to_pool_distr_view sums/folds -- the denominator is READ, never derived here"
    for sloppy in 'unwrap_or' 'unwrap_or_default' 'if .*== 0'; do
        grep -qE "$sloppy" <<<"$VIEW_FN" \
            && fail "to_pool_distr_view has a fallback ($sloppy) -- a 'sum if it looks unset' path reintroduces the bug on exactly the objects that need the fix"
    done
fi

# --- (c) ONE definition, folded over the CREDENTIAL side ------------------------
# Method-level isolation: `isolate` stops at a column-0 `}`, which inside an impl block would run
# past this fn into its siblings (StakeSnapshot::new mentions pool_stakes, a false positive).
SUM_FN="$(awk '/pub fn total_active_stake\(&self\)/{c=1} c{print} c&&/^    }$/{exit}' <<<"$EPOCH_PROD")"
if [[ -z "$SUM_FN" ]]; then
    fail "StakeSnapshot::total_active_stake not found in $EPOCH -- the single definition is gone"
else
    grep -qE 'self\.delegations' <<<"$SUM_FN" \
        || fail "StakeSnapshot::total_active_stake does not fold delegations -- cardano's sumAllStake folds the CREDENTIAL map"
    grep -qE 'pool_stakes' <<<"$SUM_FN" \
        && fail "StakeSnapshot::total_active_stake reads pool_stakes -- that is the POOL side; a membership change would move it"
fi

# --- (d) captured at the freeze, before the filter; bootstrap does not derive ----
if ! grep -qE 'mark_snapshot\.total_active_stake\(\)' <<<"$ACC_PROD"; then
    fail "the boundary freeze does not take the mark's total via StakeSnapshot::total_active_stake"
fi
TOTAL_LINE="$(grep -nE 'let total_active_stake = mark_snapshot\.total_active_stake\(\)' <<<"$ACC_PROD" | head -1 | cut -d: -f1)"
FREEZE_LINE="$(grep -nE 'from_boundary_snapshot\(' <<<"$ACC_PROD" | head -1 | cut -d: -f1)"
if [[ -n "$TOTAL_LINE" && -n "$FREEZE_LINE" ]] && (( TOTAL_LINE >= FREEZE_LINE )); then
    fail "the total is read at line $TOTAL_LINE, at/after the freeze at $FREEZE_LINE -- it must be captured BEFORE the membership filter runs"
fi
FIRSTRUN_PROD="$(prod_body "$FIRSTRUN")"
if grep -qE 'mark_pool_distr[^)]*\.(values|iter)\(\)[^;]*(sum|fold)' <<<"$FIRSTRUN_PROD"; then
    fail "the bootstrap import derives its total from mark_pool_distr -- that is already the FILTERED PoolDistr"
fi
grep -qE 'snapshots\.mark\.0\.total_active_stake\(\)' <<<"$FIRSTRUN_PROD" \
    || fail "the bootstrap import does not take the IMPORTED mark snapshot's credential-side total"

# --- (e) the field is inside the canonical commitment ---------------------------
ENC_FN="$(isolate 'pub fn encode_frozen_leadership' "$FROZEN_PROD")"
grep -qE 'write_uint_canonical\(&mut buf, d\.total_active_stake\)' <<<"$ENC_FN" \
    || fail "encode_frozen_leadership does not write total_active_stake -- the denominator would not be committed to, and a wrong one could be sealed undetected"

# --- (f) both versions moved -----------------------------------------------------
SCHEMA="$(grep -oE 'FROZEN_LEADERSHIP_SCHEMA_VERSION: u32 = [0-9]+' "$FROZEN" | grep -oE '[0-9]+$')"
STORE="$(grep -oE 'STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$SEMANTICS" | grep -oE '[0-9]+$')"
OUTER="$(grep -oE 'const OUTER_FIELDS: u64 = [0-9]+' "$FROZEN" | grep -oE '[0-9]+$')"
(( SCHEMA >= 7 )) || fail "FROZEN_LEADERSHIP_SCHEMA_VERSION is $SCHEMA, expected >= 7 (the object gained a field)"
(( STORE  >= 7 )) || fail "STORE_SEMANTICS_VERSION is $STORE, expected >= 7 -- a v6 store's leadership objects carry no total, and reconstructing one by summing entries is the defect"
(( OUTER  == 7 )) || fail "OUTER_FIELDS is $OUTER, expected 7 (version, epoch, slot, hash, commitment, total, pools)"

# --- (g) the CE tests exist and cannot pass vacuously ---------------------------
TEST_COUNT="$(grep -cE '^#\[test\]' "$TESTS" || true)"
(( TEST_COUNT >= 4 )) || fail "$TESTS declares $TEST_COUNT #[test] fns, expected >= 4 (CE-LV1-1/2/3/6+7)"
for ce in \
    'adding_or_removing_a_pool_does_not_move_the_denominator' \
    'the_retired_pool_no_longer_deflates_every_other_pools_sigma' \
    'the_credential_side_sum_includes_stake_the_membership_filter_would_drop' \
    'the_total_round_trips_and_is_committed_to'
do
    grep -qF "fn $ce" "$TESTS" || fail "the named CE test is missing: $ce"
done
# The real-case test must keep the retired pool, or it is fixing the symptom.
REAL_FN="$(awk '/fn the_retired_pool_no_longer_deflates_every_other_pools_sigma/{c=1} c{print} c&&/^}$/{exit}' "$TESTS")"
grep -qE 'contains_key' <<<"$REAL_FN" \
    || fail "the real-case test does not assert the retired pool REMAINS in the set -- membership is correct per the oracle and must not be 'fixed'"
# BLUE crate: no floating point in the ratio comparisons.
grep -qE '\bf32\b|\bf64\b' "$TESTS" \
    && fail "$TESTS uses floating point -- ade_ledger is BLUE (IDD Part V); compare ratios by integer cross-multiplication"

if (( FAILED == 0 )); then
    echo "OK (sigma denominator): FrozenLeadershipPoolDistr carries total_active_stake and to_pool_distr_view READS it with no summing loop and no fallback; StakeSnapshot::total_active_stake is the sole definition and folds the credential side; the boundary freeze captures it from the mark BEFORE the membership filter and the bootstrap import takes the imported mark's own total; the field is inside the canonical commitment; schema $SCHEMA / store semantics $STORE / OUTER_FIELDS $OUTER; $TEST_COUNT CE tests present (DC-EPOCH-40)"
fi
exit $FAILED
