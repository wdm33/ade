#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-4 (DC-EPOCH-43, DC-EPOCH-44) -- a stake snapshot keeps what cardano's keeps.
#
# REFERENCE (cardano-node 11.0.1, preprod, esSnapshots of a captured ledger state): activeStake is keyed by
# the TAGGED credential (keyHash-/scriptHash-), and stakePoolsSnapShot carries per pool its tagged
# accountId, cost, margin, pledge and selfDelegatedOwners, frozen at SNAP. The reward calculation reads
# pool parameters from the go snapshot. Ade keyed snapshots by bare hash, carried no pool parameters,
# read current registrations (a retired pool's last blocks went unpaid) and credited rewards key-first
# (a script staker's reward landed on a key account with the same bytes).
#
# Asserts:
#  (a) StakeSnapshot is keyed by StakeCredential and carries pool_params;
#  (b) the reward loop takes params from go.0.pool_params, never cert.pool.pools, and a missing entry
#      for a pool that earned is the RewardPoolParamsMissing terminal;
#  (c) the reward path has no bare-hash projection and no key-then-script resolution;
#  (d) the boundary mark keeps whole credentials and freezes pool params from the registrations at
#      SNAP -- before future-pool adoption and POOLREAP;
#  (e) the DRep voting stake reads whole credentials;
#  (f) the seed import decodes the tagged per-pool entry and whole credentials, and the cert-state
#      import reads a registration's reward account as a credential, not [network, hash];
#  (g) the version gates moved;
#  (h) the CE tests exist and ran.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EPOCH="crates/ade_ledger/src/epoch.rs"
RULES="crates/ade_ledger/src/rules.rs"
ACC="crates/ade_ledger/src/epoch_accumulator.rs"
GOV="crates/ade_ledger/src/governance.rs"
DECODE="crates/ade_ledger/src/ledgerdb_state.rs"
FRAMING="crates/ade_ledger/src/snapshot/framing.rs"
SEMANTICS="crates/ade_ledger/src/store_semantics.rs"
TESTS="crates/ade_ledger/tests/lv4_snapshot_fidelity.rs"

for f in "$EPOCH" "$RULES" "$ACC" "$GOV" "$DECODE" "$FRAMING" "$SEMANTICS" "$TESTS"; do
    [[ -f "$f" ]] || { echo "FAIL (snapshot fidelity): $f not found"; exit 1; }
done

FAILED=0
fail() { echo "FAIL (snapshot fidelity): $1"; FAILED=1; }

# Production body: drop EVERY #[cfg(test)] module (ledgerdb_state.rs has test modules mid-file) and
# strip line comments so commentary cannot satisfy or trip a grep.
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

EPOCH_PROD="$(prod_body "$EPOCH")"
RULES_PROD="$(prod_body "$RULES")"
ACC_PROD="$(prod_body "$ACC")"
GOV_PROD="$(prod_body "$GOV")"
DECODE_PROD="$(prod_body "$DECODE")"

# --- (a) the type -----------------------------------------------------------------------------
SNAP_TY="$(isolate '^pub struct StakeSnapshot \{' "$EPOCH_PROD")"
grep -qE 'pub delegations: BTreeMap<StakeCredential, \(PoolId, Coin\)>' <<<"$SNAP_TY" \
    || fail "(a) StakeSnapshot.delegations is not keyed by StakeCredential"
grep -qE 'pub pool_params: BTreeMap<PoolId, SnapshotPoolParams>' <<<"$SNAP_TY" \
    || fail "(a) StakeSnapshot carries no pool_params"
PP_TY="$(isolate '^pub struct SnapshotPoolParams \{' "$EPOCH_PROD")"
grep -qE 'pub reward_account: StakeCredential' <<<"$PP_TY" \
    || fail "(a) SnapshotPoolParams.reward_account is not a tagged StakeCredential"

# --- (b)+(c) the reward loop ------------------------------------------------------------------
REWARD_FN="$(isolate '^pub fn apply_epoch_boundary_with_registrations\(' "$RULES_PROD")"
if [[ -z "$REWARD_FN" ]]; then
    fail "(b) could not isolate apply_epoch_boundary_with_registrations"
else
    LOOP="$(awk '/for \(pool_id, pool_stake\) in &go\.0\.pool_stakes/{c=1} c{print} /deltaT2: rewards to unregistered/{exit}' <<<"$REWARD_FN")"
    [[ -n "$LOOP" ]] || fail "(b) could not isolate the reward loop"
    grep -qE 'cert\.pool\.pools\.get\(' <<<"$LOOP" \
        && fail "(b) the reward loop reads cert.pool.pools -- current registrations, not the frozen go-snapshot parameters"
    grep -qE 'go\.0\.pool_params\.get\(pool_id\)' <<<"$LOOP" \
        || fail "(b) the reward loop does not take params from go.0.pool_params"
    grep -qE 'RewardPoolParamsMissing' <<<"$LOOP" \
        || fail "(b) a pool that earned with no frozen params is not the RewardPoolParamsMissing terminal"
    grep -qE '\.hash\(\)' <<<"$REWARD_FN" \
        && fail "(c) the reward path projects a credential to its bare hash"
    grep -qE 'as_script|StakeCredential::ScriptHash\(' <<<"$REWARD_FN" \
        && fail "(c) the reward path re-introduces a key-then-script resolution"
    grep -qE 'BTreeMap<ade_types::Hash28' <<<"$REWARD_FN" \
        && fail "(c) the reward path keys a credential map by bare Hash28"
    grep -qE 'reward_account\[1\.\.29\]' <<<"$REWARD_FN" \
        && fail "(c) the operator credential is sliced out of the raw account bytes (tag dropped)"
fi

# --- (d) the boundary mark --------------------------------------------------------------------
MARK_FN="$(isolate '^pub\(crate\) fn build_boundary_mark_snapshot\(' "$ACC_PROD")"
grep -qE 'delegations\.insert\(cred\.clone\(\)' <<<"$MARK_FN" \
    || fail "(d) the mark does not insert the whole credential"
grep -qE 'snapshot_pool_params\(pools' <<<"$MARK_FN" \
    || fail "(d) the mark does not freeze pool params"
MARK_CALL_LINE="$(grep -nE 'build_boundary_mark_snapshot\($' <<<"$REWARD_FN" | head -1 | cut -d: -f1)"
REAP_LINE="$(grep -nE 'let mut pool_state = cert\.pool\.clone\(\);' <<<"$REWARD_FN" | head -1 | cut -d: -f1)"
if [[ -z "$MARK_CALL_LINE" || -z "$REAP_LINE" ]]; then
    fail "(d) could not locate the mark build and the POOLREAP state in the boundary"
else
    (( MARK_CALL_LINE < REAP_LINE )) \
        || fail "(d) the mark is built after the POOLREAP/adoption state is formed -- it must freeze the registrations at SNAP"
    awk -v a="$MARK_CALL_LINE" 'NR>=a && NR<=a+5' <<<"$REWARD_FN" | grep -qE '&cert\.pool\.pools' \
        || fail "(d) the mark is not handed the registrations at SNAP (&cert.pool.pools)"
fi

# --- (e) DRep voting stake --------------------------------------------------------------------
DREP_FN="$(isolate '^pub fn derive_drep_voting_stake\(' "$GOV_PROD")"
grep -qE 'mark\.delegations\.get\(cred\)' <<<"$DREP_FN" \
    || fail "(e) the DRep voting stake does not look up the whole credential"

# --- (f) the imports --------------------------------------------------------------------------
SNAP_DEC="$(isolate '^fn read_stake_snapshot_full\(' "$DECODE_PROD")"
grep -qE 'expect_array\(d, o, 10, "snap\.pool"\)' <<<"$SNAP_DEC" \
    || fail "(f) the seed import does not decode the array(10) per-pool entry"
grep -qE '\.hash\(\)' <<<"$SNAP_DEC" \
    && fail "(f) the seed import projects a credential to its bare hash"
grep -qE 'has stake but no frozen parameters' <<<"$SNAP_DEC" \
    || fail "(f) the seed import accepts a staked pool without frozen parameters"
POOL_DEC="$(isolate '^fn read_pool_params\(' "$DECODE_PROD")"
grep -qE 'let account = read_credential\(d, o\)\?' <<<"$POOL_DEC" \
    || fail "(f) the cert-state import does not read the reward account as a credential"
grep -qE 'rewardAcct\.net' <<<"$POOL_DEC" \
    && fail "(f) the cert-state import reads the reward-account tag as a network number"

# --- (g) versions -----------------------------------------------------------------------------
STORE="$(grep -oE 'STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$SEMANTICS" | grep -oE '[0-9]+$')"
(( STORE >= 10 )) || fail "(g) STORE_SEMANTICS_VERSION is $STORE, expected >= 10"
ACCV="$(grep -oE 'EPOCH_ACCUMULATOR_SCHEMA_VERSION: u32 = [0-9]+' "$ACC" | grep -oE '[0-9]+$')"
(( ACCV >= 5 )) || fail "(g) EPOCH_ACCUMULATOR_SCHEMA_VERSION is $ACCV, expected >= 5"
FRV="$(grep -oE '^pub const SCHEMA_VERSION: u32 = [0-9]+' "$FRAMING" | grep -oE '[0-9]+$')"
(( FRV >= 3 )) || fail "(g) snapshot framing SCHEMA_VERSION is $FRV, expected >= 3"

# --- (h) the CE tests exist and ran -----------------------------------------------------------
for ce in \
    'a_key_and_a_script_credential_with_the_same_bytes_are_two_stakers' \
    'a_script_credentials_reward_is_never_credited_to_the_key_credential_with_the_same_bytes' \
    'a_retired_pool_is_paid_for_its_last_blocks_from_its_frozen_parameters' \
    'a_re_registered_pool_is_paid_on_the_values_frozen_in_the_go_snapshot' \
    'an_operator_with_a_script_reward_account_is_paid_on_the_script_credential' \
    'an_owner_is_matched_as_a_key_credential_only' \
    'a_pool_that_earned_with_no_frozen_parameters_is_a_terminal' \
    'the_mark_freezes_parameters_before_adoption_and_before_poolreap'
do
    grep -qF "fn $ce" "$TESTS" || fail "(h) the named CE test is missing: $ce"
done
OUT="$(cargo test -p ade_ledger --test lv4_snapshot_fidelity 2>&1 || true)"
if grep -qE '^test result: ok\. [0-9]+ passed; 0 failed' <<<"$OUT"; then
    RAN="$(grep -oE '^test result: ok\. [0-9]+ passed' <<<"$OUT" | grep -oE '[0-9]+')"
    (( RAN >= 8 )) || fail "(h) only $RAN LV-4 integration tests ran, expected >= 8"
else
    fail "(h) the LV-4 integration tests did not pass"
fi
UNIT_NAMES=(
    'read_stake_snapshot_full_decodes_combined_stake_and_delegation'
    'read_stake_snapshot_full_refuses_an_entry_that_disagrees_with_its_snapshot'
    'a_registration_reward_account_tag_is_the_credential_kind_not_the_network'
    'a_script_voter_does_not_take_the_stake_of_a_key_credential_with_the_same_bytes'
    'codec_rejects_pre_lv4_store_rebootstrap_required'
)
for t in "${UNIT_NAMES[@]}"; do
    UOUT="$(cargo test -p ade_ledger --lib "$t" 2>&1 || true)"
    grep -qE "^test .*$t \.\.\. ok$" <<<"$UOUT" || fail "(h) unit test $t did not run and pass"
done
grep -qE '\bf32\b|\bf64\b' "$TESTS" && fail "(h) $TESTS uses floating point -- ade_ledger is BLUE"

if (( FAILED == 0 )); then
    echo "OK (snapshot fidelity): snapshots keyed by whole credential with frozen pool params; reward loop reads go.0.pool_params only with a typed terminal for a missing entry; no bare-hash projection or key-then-script resolution; mark frozen at SNAP before adoption/POOLREAP; DRep stake by whole credential; seed and cert-state imports keep the tag; store semantics $STORE, accumulator schema $ACCV, framing $FRV; CE tests ran (DC-EPOCH-43, DC-EPOCH-44)"
fi
exit $FAILED
