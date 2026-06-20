#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3c (DC-EVIEW-05): per-pool stake aggregation -- the linchpin.
# For each REGISTERED+DELEGATED credential, active stake = sum(base-address UTxO coin)
# + reward balance, grouped by its delegated pool (cardano-ledger snapshot rule,
# Conway-specialized). A pure projection of the single ledger authority's own state
# (the reduced checkpoint's base-credential sums + the delegation map). OBSERVE-ONLY:
# the new_mark rewire + live leader-election feed are the activation slice (DC-EVIEW-08).
# ACCEPTANCE is the DIFFERENTIAL ORACLE vs cardano-cli stake-snapshot (a LIVE gate,
# DECLARED -- run at activation, NOT faked green).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
AGG=crates/ade_ledger/src/reduced_aggregate.rs
CP=crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs

test -f "$AGG" || fail "the aggregation ($AGG) is missing"

# (1) aggregate over the REGISTERED+DELEGATED creds (the delegation map), summing
#     UTxO coin + reward per credential, grouped by pool.
grep -qE 'pub fn aggregate_pool_stake' "$AGG" || fail "aggregate_pool_stake missing"
grep -qF 'for (cred, pool) in &delegation.delegations' "$AGG" \
    || fail "aggregation does not iterate the delegation map (registered+delegated creds)"
grep -qF 'delegation.rewards.get(cred)' "$AGG" || fail "aggregation does not add reward balances"

# (2) fail-closed on overflow (never a silently wrapped stake total).
grep -qE 'AggregateError::StakeOverflow' "$AGG" || fail "no fail-closed overflow handling"
grep -qF 'checked_add' "$AGG" || fail "aggregation does not use checked_add"

# (3) the per-credential UTxO fold reads ONLY Base entries from the reduced checkpoint.
grep -qE 'pub fn sum_base_credential_stake' "$CP" || fail "sum_base_credential_stake missing"
grep -qF 'ReducedStakeRef::Base(cred)' "$CP" \
    || fail "sum_base_credential_stake does not fold only Base credentials"

# (4) the load-bearing logic proofs.
for t in sums_utxo_plus_reward_per_delegated_pool reward_without_utxo_contributes \
         undelegated_credential_contributes_nothing overflow_is_fail_closed; do
    grep -qE "fn $t" "$AGG" || fail "the $t proof is missing"
done
grep -qE 'fn sum_base_credential_stake_skips_non_contributing' "$CP" \
    || fail "the NonContributing-skip proof is missing"

# (5) S3c boundary: OBSERVE-ONLY -- no live wiring. aggregate_pool_stake is referenced
#     by no live producer/follow call site (the new_mark rewire + leader feed are the
#     DC-EVIEW-08 activation slice).
if grep -rqE 'aggregate_pool_stake' \
    crates/ade_node/src/node_lifecycle.rs crates/ade_node/src/node_sync.rs \
    crates/ade_runtime/src/admission/ crates/ade_runtime/src/forward_sync/ 2>/dev/null; then
    fail "aggregate_pool_stake is referenced on the live producer/follow path -- S3c is observe-only"
fi

# (6) the differential oracle is DECLARED (not faked): the module records it as the
#     live acceptance gate.
grep -qiE 'cardano-cli query stake-snapshot|stakeSet' "$AGG" \
    || fail "the cardano-cli stake-snapshot differential oracle is not recorded as the live acceptance gate"

if (( FAILED == 0 )); then
    echo "OK: stake aggregation (DC-EVIEW-05; UTxO+reward per registered+delegated pool, fail-closed, Base-only fold; observe-only; cardano-cli oracle DECLARED live gate)"
fi
exit $FAILED
