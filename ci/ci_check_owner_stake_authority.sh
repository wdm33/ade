#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-3 (DC-EPOCH-42) -- in the reward calculation a pool's owner stake is the stake of the
# owners DELEGATED TO THAT POOL, and it has exactly ONE definition.
#
# ORACLE (cardano-ledger master, Cardano.Ledger.State.SnapShots / Shelley.Rewards):
#   selfDelegatedOwners =
#     Set.filter (\o -> KeyHashObj o `Set.member` spsDelegators) spsOwners
#   maxP = if pledge <= selfDelegatedOwnersStake then maxPool' ... else mempty
#   poolOwnerRelativeStake = selfDelegatedOwnersStake %? totalStake
# One set drives the pledge gate and the leader term. Ade had two inline sums, each over every
# listed owner wherever delegated; two definitions of one quantity is how the sites could drift.
#
# Asserts:
#  (a) StakeSnapshot::self_delegated_owner_stake exists, reads the credential side, and FILTERS by
#      the pool the stake is delegated to;
#  (b) the reward path calls it exactly once and binds the result;
#  (c) the pledge gate compares the pledge against that binding, and the leader term's owner
#      stake IS that binding;
#  (d) the reward path does not iterate the owner list at all -- no second, inline sum;
#  (e) the store-semantics version moved with the rule;
#  (f) the CE tests exist and are non-vacuous.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EPOCH="crates/ade_ledger/src/epoch.rs"
RULES="crates/ade_ledger/src/rules.rs"
SEMANTICS="crates/ade_ledger/src/store_semantics.rs"
TESTS="crates/ade_ledger/tests/lv3_self_delegated_owner_stake.rs"

for f in "$EPOCH" "$RULES" "$SEMANTICS" "$TESTS"; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL (owner stake): $f not found"
        exit 1
    fi
done

FAILED=0
fail() { echo "FAIL (owner stake): $1"; FAILED=1; }

# Production body only: truncate at the TRAILING #[cfg(test)] MODULE (an inline test shim must not
# blind this gate), and strip line comments so commentary naming a token cannot satisfy or trip a grep.
prod_body() {
    awk '
        /^#\[cfg\(test\)\]$/ { pend = $0; holding = 1; next }
        holding && /^#\[allow\(/ { pend = pend "\n" $0; next }
        holding && /^mod / { exit }
        holding { print pend; holding = 0 }
        { print }
    ' "$1" | sed -E 's://.*::'
}

EPOCH_PROD="$(prod_body "$EPOCH")"
RULES_PROD="$(prod_body "$RULES")"
if [[ -z "$EPOCH_PROD" || -z "$RULES_PROD" ]]; then
    echo "FAIL (owner stake): could not isolate production bodies"
    exit 1
fi

# --- (a) the single definition, filtered by delegation target -------------------
DEF_FN="$(awk '/pub fn self_delegated_owner_stake(<[^>]*>)?\(/{c=1} c{print} c&&/^    }$/{exit}' <<<"$EPOCH_PROD")"
if [[ -z "$DEF_FN" ]]; then
    fail "StakeSnapshot::self_delegated_owner_stake not found in $EPOCH -- the single definition is gone"
else
    grep -qE 'self\.delegations' <<<"$DEF_FN" \
        || fail "self_delegated_owner_stake does not read self.delegations (the credential side)"
    grep -qE '==\s*pool_id' <<<"$DEF_FN" \
        || fail "self_delegated_owner_stake does not compare the delegation target with pool_id -- without that filter it is the full-go owner sum, which pays a pool whose owner delegates elsewhere"
    grep -qE 'pool_stakes' <<<"$DEF_FN" \
        && fail "self_delegated_owner_stake reads pool_stakes -- owner stake is a per-credential fact"
fi
DEF_COUNT="$(grep -cE 'fn self_delegated_owner_stake(<[^>]*>)?\(' <<<"$EPOCH_PROD" || true)"
(( DEF_COUNT == 1 )) || fail "$DEF_COUNT definitions of self_delegated_owner_stake in $EPOCH, expected exactly 1"

# --- (b) called once in the reward path, result bound ---------------------------
REWARD_FN="$(awk '/^pub fn apply_epoch_boundary_with_registrations\(/{c=1} c{print} c&&/^}$/{exit}' <<<"$RULES_PROD")"
if [[ -z "$REWARD_FN" ]]; then
    fail "could not isolate apply_epoch_boundary_with_registrations"
else
    CALLS="$(grep -cE '\.self_delegated_owner_stake\(' <<<"$REWARD_FN" || true)"
    (( CALLS == 1 )) || fail "the reward path calls self_delegated_owner_stake $CALLS times, expected exactly 1 (one value feeds both sites)"
    # LV-4 strengthened: the owners are the self-delegated set FROZEN in the go snapshot's pool parameters
    # (cardano's spssSelfDelegatedOwners), never the current registration's owner list.
    grep -qE 'go\.0\.self_delegated_owner_stake\(pool_id, &params\.self_delegated_owners\)' <<<"$REWARD_FN" \
        || fail "the reward path does not take the owner stake from the GO snapshot for the owners frozen in it"

    # --- (c) both sites read the one binding -----------------------------------
    grep -qE 'params\.pledge\.0 > self_delegated_owner_stake' <<<"$REWARD_FN" \
        || fail "the pledge gate does not compare the pledge against self_delegated_owner_stake"
    LEADER="$(awk '/let owner_stake: u64 = if use_owners \{/{c=1} c{print} c&&/\};/{exit}' <<<"$REWARD_FN")"
    if [[ -z "$LEADER" ]]; then
        fail "could not isolate the leader term's owner_stake binding"
    else
        FIRST_ARM="$(sed -n '2p' <<<"$LEADER" | tr -d '[:space:]')"
        [[ "$FIRST_ARM" == "self_delegated_owner_stake" ]] \
            || fail "the leader term's owner stake is '$FIRST_ARM', expected the self_delegated_owner_stake binding"
    fi

    # --- (d) no second sum over the owner list ----------------------------------
    # The only permitted uses of the owner list here are: is_empty (gating), contains (member
    # exclusion, already confined to this pool's delegators) and the one call above.
    if grep -qE 'owners\s*$' <<<"$REWARD_FN" || grep -qE 'owners\s*\.(iter|into_iter)\(' <<<"$REWARD_FN" \
        || grep -qE 'for .* in .*owners' <<<"$REWARD_FN"; then
        fail "the reward path iterates the owner list -- a second, inline owner-stake sum is the defect LV-3 closes"
    fi
    OTHER="$(grep -oE 'owners\.[a-z_]+' <<<"$REWARD_FN" | sort -u | grep -vE '^owners\.(is_empty|contains)$' || true)"
    [[ -z "$OTHER" ]] || fail "unexpected use of the owner list in the reward path: $(tr '\n' ' ' <<<"$OTHER")"
fi

# --- (e) the store-semantics version moved --------------------------------------
STORE="$(grep -oE 'STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$SEMANTICS" | grep -oE '[0-9]+$')"
(( STORE >= 8 )) || fail "STORE_SEMANTICS_VERSION is $STORE, expected >= 8 -- a v7 store crossed boundaries under the full-go owner sum and can hold different reward balances"

# --- (f) the CE tests exist and cannot pass vacuously ---------------------------
TEST_COUNT="$(grep -cE '^#\[test\]' "$TESTS" || true)"
(( TEST_COUNT >= 5 )) || fail "$TESTS declares $TEST_COUNT #[test] fns, expected >= 5"
for ce in \
    'a_pool_whose_owner_delegates_elsewhere_earns_nothing' \
    'the_same_owner_delegated_to_the_pool_satisfies_the_pledge' \
    'the_leader_term_counts_only_owners_delegated_to_the_pool' \
    'only_the_self_delegated_subset_of_owners_is_summed' \
    'a_pledge_met_only_by_counting_owners_elsewhere_is_not_met'
do
    grep -qF "fn $ce" "$TESTS" || fail "the named CE test is missing: $ce"
done
# The invariant test must assert that NOTHING is distributed, not merely less.
INV_FN="$(awk '/fn a_pool_whose_owner_delegates_elsewhere_earns_nothing/{c=1} c{print} c&&/^}$/{exit}' "$TESTS")"
grep -qE 'acct\.sum_rewards, 0' <<<"$INV_FN" \
    || fail "the invariant test does not assert sum_rewards == 0"
grep -qE '\bf32\b|\bf64\b' "$TESTS" \
    && fail "$TESTS uses floating point -- ade_ledger is BLUE (IDD Part V)"

# The tests must have RUN and passed -- a compile-only pass proves nothing.
OUT="$(cargo test -p ade_ledger --test lv3_self_delegated_owner_stake 2>&1 || true)"
if ! grep -qE '^test result: ok\. [0-9]+ passed; 0 failed' <<<"$OUT"; then
    fail "the LV-3 tests did not pass"
else
    RAN="$(grep -oE '^test result: ok\. [0-9]+ passed' <<<"$OUT" | grep -oE '[0-9]+')"
    (( RAN >= 5 )) || fail "only $RAN LV-3 tests ran, expected >= 5"
fi

if (( FAILED == 0 )); then
    echo "OK (owner stake): StakeSnapshot::self_delegated_owner_stake is the sole definition, reads the credential side and filters by delegation target; the reward path calls it once from the go snapshot and that one binding feeds the pledge gate and the leader term, with no iteration of the owner list; store semantics $STORE; $TEST_COUNT CE tests present and passing (DC-EPOCH-42)"
fi
exit $FAILED
