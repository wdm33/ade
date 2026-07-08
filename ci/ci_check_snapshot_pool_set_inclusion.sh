#!/usr/bin/env bash
set -uo pipefail

# DC-EPOCH-24 (CE-3d go pool-set inclusion): the per-epoch stake snapshot (mark/set/go) includes a
# registered+delegated credential IFF its combined (base UTxO + reward) stake is NON-ZERO -- cardano's
# ssActiveStake NonZero VMap (Stake.hs resolveActiveInstantStakeCredentials). A zero-stake credential is
# OMITTED from both delegations and pool_stakes, at the point of construction (NOT a post-filter), in BOTH
# the authoritative boundary mark and the reduced-checkpoint projection. Mechanical enforcement (IDD
# principle 10) of the properties the unit tests exercise but do not pin structurally.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ACC="$REPO_ROOT/crates/ade_ledger/src/epoch_accumulator.rs"
AGG="$REPO_ROOT/crates/ade_ledger/src/reduced_aggregate.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$ACC" "$AGG" "$REG"; do
  [[ -f "$f" ]] || fail "missing expected file $f"
done
[[ $FAILED -eq 0 ]] || exit 1

# (1) AUTHORITATIVE PATH: build_boundary_mark_snapshot skips a zero combined-stake credential BEFORE
#     inserting into delegations/pool_stakes. The guard lives inside the builder (construction, not a
#     post-filter over the finished map).
awk '/fn build_boundary_mark_snapshot/{i=1} i&&/if stake == 0/{f=1} i&&/^}/{i=0} END{exit !f}' "$ACC" \
  || fail "(1) build_boundary_mark_snapshot does not skip a zero-combined-stake credential (if stake == 0)"

# (2) REDUCED PROJECTION: aggregate_pool_stake skips a zero combined-stake credential BEFORE or_insert.
awk '/fn aggregate_pool_stake/{i=1} i&&/if cred_total\.0 == 0/{f=1} i&&/^}/{i=0} END{exit !f}' "$AGG" \
  || fail "(2) aggregate_pool_stake does not skip a zero-combined-stake credential (if cred_total.0 == 0)"

# (3) NO WRONG-RULE COMMENT REGRESSION: the reversed 'included even at 0 stake (numDelegators>0)' rule
#     must not reappear as an assertion in the aggregation tests.
if grep -Eq 'fn delegated_zero_stake_pool_is_included_with_zero' "$AGG"; then
  fail "(3) the wrong-rule test delegated_zero_stake_pool_is_included_with_zero is back -- ssActiveStake OMITS 0-stake pools"
fi
grep -q 'fn delegated_zero_stake_pool_is_omitted' "$AGG" \
  || fail "(3) the corrected test delegated_zero_stake_pool_is_omitted is missing"

# (4) FROZEN DECISION TABLE: the cardano-derived ssActiveStake membership table is pinned.
grep -q 'fn ssactivestake_membership_decision_table' "$AGG" \
  || fail "(4) the frozen ssActiveStake membership decision-table test is missing"

# (5) AUTHORITATIVE-PATH TEST: the boundary-mark omission is pinned.
grep -q 'fn build_boundary_mark_snapshot_omits_zero_stake_credential' "$ACC" \
  || fail "(5) build_boundary_mark_snapshot_omits_zero_stake_credential test is missing"

# (6) DC-EPOCH-24 is in the registry.
grep -q 'DC-EPOCH-24' "$REG" || fail "(6) DC-EPOCH-24 is not in the invariant registry"

# (7) PERSISTED-SIDE (schema-reject compatibility slice): the accumulator schema version is bumped to 4,
#     so a pre-C v3 store -- whose mark/set/go were built under the prior numDelegators>0 rule (phantom
#     0-stake pools) -- fails closed (UnknownVersion) on decode. A warm-start never RELOADS a stale
#     snapshot-inclusion semantics; persisted authority has one replay meaning; re-bootstrap is the only
#     migration. This is the persisted-side enforcement of DC-EPOCH-24.
grep -q 'pub const EPOCH_ACCUMULATOR_SCHEMA_VERSION: u32 = 4' "$ACC" \
  || fail "(7) EPOCH_ACCUMULATOR_SCHEMA_VERSION is not 4 -- a pre-C v3 store must fail closed"
grep -q 'fn codec_rejects_pre_c_v3_store_rebootstrap_required' "$ACC" \
  || fail "(7) the pre-C v3 fail-closed (re-bootstrap required) test is missing"

[[ $FAILED -eq 0 ]] || exit 1
echo "DC-EPOCH-24 OK: zero-stake credentials omitted at construction in both the boundary mark and the reduced projection; wrong-rule test reversed; ssActiveStake membership decision table frozen; accumulator schema v4 rejects a pre-C store (re-bootstrap required)"
