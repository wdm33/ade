#!/usr/bin/env bash
set -euo pipefail

# SLICE LV-15b-0 (DC-EPOCH-47) -- ratification for the boundary E->E+1 is decided from the authority frozen at
# the start of E, and only applied at E->E+1.
#
# REFERENCE (cardano-ledger cb57dc73): Conway/Rules/Epoch.hs consumes the DRep pulser seeded at the start of
# the epoch (extractDRepPulsingState, then setFreshDRepPulsingState); Governance/Proposals.hs
# proposalsApplyEnactment removes the expired with their descendants, then enacts and prunes.
#
# Asserts:
#  (a) the boundary consumes and freezes through the one shared function: rules.rs production calls
#      apply_ratification_record once, ratify_frozen once, and the composed planner never;
#  (b) the apply half reads no votes, stake, committee, thresholds or expiry: apply_ratification_record and
#      apply_decision call no ratification predicate and no distribution derivation;
#  (c) the evidence is read by no rule: RatificationRecord::evidence and ::from_parts are called only by the
#      record codec (snapshot/gov_state.rs) and frozen.rs; fixture_decided_nothing has no production caller;
#  (d) the bootstrap imports the snapshot's finished ratification (govState[6]), refuses dangling vote
#      delegations, and binds the record into the seed commitment; the Mithril assembly carries it;
#  (e) the version gates moved: store semantics >= 12, accumulator schema >= 7, snapshot framing >= 5;
#  (f) the CE tests exist and ran.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

RULES="crates/ade_ledger/src/rules.rs"
GOV="crates/ade_ledger/src/governance.rs"
FROZEN="crates/ade_ledger/src/governance/frozen.rs"
CODEC="crates/ade_ledger/src/snapshot/gov_state.rs"
DECODE="crates/ade_ledger/src/ledgerdb_state.rs"
ASSEMBLY="crates/ade_runtime/src/mithril_native_assembly.rs"
ACC="crates/ade_ledger/src/epoch_accumulator.rs"
FRAMING="crates/ade_ledger/src/snapshot/framing.rs"
SEMANTICS="crates/ade_ledger/src/store_semantics.rs"

for f in "$RULES" "$GOV" "$FROZEN" "$CODEC" "$DECODE" "$ASSEMBLY" "$ACC" "$FRAMING" "$SEMANTICS"; do
    [[ -f "$f" ]] || { echo "FAIL (frozen ratification): $f not found"; exit 1; }
done

FAILED=0
fail() { echo "FAIL (frozen ratification): $1"; FAILED=1; }

# Production body: drop every #[cfg(test)] module AND every #[cfg(test)] item, and strip line comments.
prod_body() {
    awk '
        /^#\[cfg\(test\)\]$/ { pend = $0; holding = 1; next }
        holding && /^#\[allow\(/ { pend = pend "\n" $0; next }
        holding && /^mod .*\{$/ { skipping = 1; holding = 0; next }
        holding && /^(pub(\(crate\))? )?fn / { skipping = 1; holding = 0; next }
        holding { print pend; holding = 0 }
        skipping { if ($0 ~ /^}$/) skipping = 0; next }
        { print }
    ' "$1" | sed -E 's://.*::'
}
isolate() {
    awk -v pat="$1" '$0 ~ pat { c=1 } c { print } c && /^}/ { exit }' <<<"$2"
}

RULES_PROD="$(prod_body "$RULES")"
GOV_PROD="$(prod_body "$GOV")"
FROZEN_PROD="$(prod_body "$FROZEN")"
DECODE_PROD="$(prod_body "$DECODE")"
ASSEMBLY_PROD="$(prod_body "$ASSEMBLY")"

# --- (a) one boundary authority -----------------------------------------------------------------
BFN="$(isolate '^pub fn apply_epoch_boundary_with_registrations\(' "$RULES_PROD")"
[[ -n "$BFN" ]] || fail "(a) could not isolate apply_epoch_boundary_with_registrations"
n() { grep -cE "$1" <<<"$2" || true; }
(( $(n 'apply_ratification_record\(' "$RULES_PROD") == 1 )) || fail "(a) rules.rs must apply the record exactly once"
(( $(n 'apply_ratification_record\(' "$BFN") == 1 )) || fail "(a) the record is not applied inside the boundary fn"
(( $(n 'ratify_frozen\(' "$RULES_PROD") == 1 )) || fail "(a) rules.rs must freeze exactly once"
(( $(n 'ratify_frozen\(' "$BFN") == 1 )) || fail "(a) the record is not frozen inside the boundary fn"
(( $(n 'plan_conway_governance_epoch\(' "$RULES_PROD") == 0 )) || fail "(a) the boundary runs the composed planner"
(( $(n 'RatificationRecordAbsent' "$BFN") >= 1 )) || fail "(a) a state without a record is not refused"
L_APPLY="$(grep -nE 'apply_ratification_record\(' <<<"$BFN" | head -1 | cut -d: -f1 || true)"
L_FREEZE="$(grep -nE 'ratify_frozen\(' <<<"$BFN" | head -1 | cut -d: -f1 || true)"
if [[ -z "$L_APPLY" || -z "$L_FREEZE" ]] || (( L_FREEZE <= L_APPLY )); then
    fail "(a) the freeze does not follow the application"
fi
for f in $(grep -rlE 'ratify_frozen\(|apply_ratification_record\(' crates/*/src); do
    [[ "$f" == "$RULES" || "$f" == "$FROZEN" ]] && continue
    if prod_body "$f" | grep -qE 'ratify_frozen\(|apply_ratification_record\('; then
        fail "(a) $f freezes or applies a record outside the boundary fn"
    fi
done

# --- (b) the apply half reads no ratification input ----------------------------------------------
APPLY_REC="$(isolate '^pub fn apply_ratification_record\(' "$FROZEN_PROD")"
APPLY_DEC="$(isolate '^pub\(crate\) fn apply_decision\(' "$GOV_PROD")"
[[ -n "$APPLY_REC" ]] || fail "(b) could not isolate apply_ratification_record"
[[ -n "$APPLY_DEC" ]] || fail "(b) could not isolate apply_decision"
FORBIDDEN='check_ratification|active_drep_stake|decide_ratification|derive_frozen_|derive_drep_voting_stake|committee_quorum|drep_expiry|vote_delegations|_votes|voting_thresholds'
for body in "$APPLY_REC" "$APPLY_DEC"; do
    if grep -qE "$FORBIDDEN" <<<"$body"; then
        fail "(b) the apply half names a ratification input: $(grep -oE "$FORBIDDEN" <<<"$body" | sort -u | tr '\n' ' ')"
    fi
done

# --- (c) evidence is read by no rule ---------------------------------------------------------------
for f in $(grep -rlE '\.evidence\(\)|RatificationRecord::from_parts\(|fixture_decided_nothing\(' crates/*/src); do
    body="$(prod_body "$f")"
    if grep -qE '\.evidence\(\)|RatificationRecord::from_parts\(' <<<"$body"; then
        [[ "$f" == "$CODEC" || "$f" == "$FROZEN" ]] || fail "(c) $f reads or rebuilds record evidence"
    fi
    if [[ "$f" != "$FROZEN" ]] && grep -qE 'fixture_decided_nothing\(' <<<"$body"; then
        fail "(c) $f calls the fixture constructor in production"
    fi
done
grep -qE '^    pub (frozen_proposals|drep_distribution|pool_distribution):' <<<"$(isolate '^pub struct RatificationRecord \{' "$FROZEN_PROD")" \
    && fail "(c) a RatificationRecord evidence field is public"

# --- (d) bootstrap import ------------------------------------------------------------------------
grep -qE 'let ratification = nn_read_drep_pulser\(d, o, network_id\)\?' <<<"$DECODE_PROD" \
    || fail "(d) the seed decoder does not read govState[6]"
grep -qE 'import_snapshot_ratification\(' <<<"$DECODE_PROD" || fail "(d) the seed decoder does not import the record"
grep -qE 'NativeNonUtxoError::DanglingVoteDelegation \{' <<<"$DECODE_PROD" \
    || fail "(d) the seed decoder does not refuse a dangling vote delegation"
grep -qE 'write_ratification_record\(&mut rec, &s\.ratification\)' <<<"$DECODE_PROD" \
    || fail "(d) the seed commitment does not bind the record"
CTAG="$(grep -oE 'commitment-v[0-9]+' <<<"$DECODE_PROD" | head -1 | grep -oE '[0-9]+$' || true)"
(( ${CTAG:-0} >= 14 )) || fail "(d) the seed commitment tag is v${CTAG:-?}, expected >= v14"
grep -qE 'RatificationState::Recorded\(s1a\.ratification\.clone\(\)\)' <<<"$ASSEMBLY_PROD" \
    || fail "(d) the Mithril assembly does not carry the imported record"
grep -qE 'ratification: ade_ledger::state::RatificationState::Unversioned' <<<"$ASSEMBLY_PROD" \
    && fail "(d) the Mithril assembly seeds an Unversioned record"

# --- (e) versions ---------------------------------------------------------------------------------
STORE="$(grep -oE '^pub const STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$SEMANTICS" | grep -oE '[0-9]+$' || true)"
(( ${STORE:-0} >= 12 )) || fail "(e) STORE_SEMANTICS_VERSION is ${STORE:-?}, expected >= 12 -- a v11 store holds no record"
ACCV="$(grep -oE '^pub const EPOCH_ACCUMULATOR_SCHEMA_VERSION: u32 = [0-9]+' "$ACC" | grep -oE '[0-9]+$' || true)"
(( ${ACCV:-0} >= 7 )) || fail "(e) EPOCH_ACCUMULATOR_SCHEMA_VERSION is ${ACCV:-?}, expected >= 7"
FRV="$(grep -oE '^pub const SCHEMA_VERSION: u32 = [0-9]+' "$FRAMING" | grep -oE '[0-9]+$' || true)"
(( ${FRV:-0} >= 5 )) || fail "(e) snapshot framing SCHEMA_VERSION is ${FRV:-?}, expected >= 5"

# --- (f) the CE tests exist and ran ----------------------------------------------------------------
TESTS=(
    ce_lv15b0_1_late_votes_do_not_enact_at_the_consuming_boundary
    ce_lv15b0_1_a_vote_cast_during_epoch_e_enacts_at_e_plus_2
    ce_lv15b0_2_late_proposal_is_carried
    ce_lv15b0_3a_account_unregistration_removes_vote_delegation_and_reregistration_starts_empty
    ce_lv15b0_3a_drep_reregistration_does_not_regain_old_delegators
    ce_lv15b0_3b_vote_delegation_to_unregistered_drep_is_rejected
    ce_lv15b0_3b_drep_registration_then_delegation_is_ordered
    ce_lv15b0_3b_dangling_delegation_is_a_terminal_not_excluded
    ce_lv15b0_4_pool_distribution_is_mark_plus_deposits
    ce_lv15b0_3_4_boundary_evidence_is_post_boundary_accounts_and_the_new_mark
    ce_lv15b0_5_7_changes_after_the_freeze_do_not_change_the_outcome
    ce_lv15b0_6_application_prunes_late_siblings_and_routes_by_registration_now
    ce_lv15b0_6_unenactable_halts_at_the_consuming_boundary
    ce_lv15b0_8_import_supported_enactment_is_decided_and_cross_checked
    ce_lv15b0_8_import_unsupported_enactments_are_unenactable
    lv15b0_expired_proposal_removes_its_descendants
    lv15b0_expired_sibling_is_removed_as_expired_not_pruned
    lv15b0_boundary_refuses_a_state_without_a_record
    lv15b0_boundary_refuses_a_record_from_another_epoch
    lv15b0_v4_record_round_trips_byte_exact
    lv15b0_v3_bytes_decode_unversioned
    lv15b0_evidence_only_change_is_visible
    lv15b0_record_decoder_refuses_non_ascending_keys
    codec_rejects_pre_lv15b0_store_rebootstrap_required
)
HERMETIC=(
    lv15b0_drep_pulser_is_imported_as_the_snapshot_epoch_record
    lv15b0_pulser_parameters_mismatch_is_refused
    lv15b0_pulser_root_mismatch_is_refused
    lv15b0_pulser_delayed_without_unenactable_action_is_refused
    lv15b0_malformed_pulser_is_refused
    lv15b0_dangling_vote_delegation_in_a_snapshot_is_refused
    protocol_major_below_10_is_refused
)
OUT_V="$(cargo test -p ade_ledger --lib -- lv15b0 codec_rejects_pre_lv15b0 2>&1 || true)"
OUT_HV="$(cargo test -p ade_ledger --test ledgerdb_nonutxo_hermetic 2>&1 || true)"
for t in "${TESTS[@]}"; do
    grep -qE "^test [a-z0-9_:]*::${t} \.\.\. ok$" <<<"$OUT_V" || fail "(f) $t did not run and pass"
done
for t in "${HERMETIC[@]}"; do
    grep -qE "^test ${t} \.\.\. ok$" <<<"$OUT_HV" || fail "(f) $t did not run and pass"
done
grep -qE 'test result: ok\.' <<<"$OUT_V" || fail "(f) the ade_ledger lv15b0 tests did not pass"
grep -qE 'test result: ok\.' <<<"$OUT_HV" || fail "(f) the hermetic bootstrap tests did not pass"

if (( FAILED == 0 )); then
    echo "PASS: frozen ratification authority (LV-15b-0, DC-EPOCH-47)"
fi
exit "$FAILED"
