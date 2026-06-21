#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONTINUITY-ACTIVATION ECA-0b (DC-EVIEW-12 / DC-EPOCH-12): the leadership-complete,
# self-contained EpochConsensusView. The candidate carries the effective per-pool VRF keyhash + a
# FULL consensus-profile commitment (genesis + protocol-params + ASC), folded into the canonical
# hash; the PoolDistrView projection derives EXCLUSIVELY from the view + a commitment-verified
# profile (no live CertState read, no unbound protocol-param read). The aggregate matches cardano's
# numDelegators>0 inclusion (a delegated pool is included even at 0 stake).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
RV=crates/ade_ledger/src/reduced_epoch_view.rs
AGG=crates/ade_ledger/src/reduced_aggregate.rs
CAND=crates/ade_node/src/epoch_candidate.rs

test -f "$RV" || fail "reduced_epoch_view.rs missing"
test -f "$CAND" || fail "epoch_candidate.rs missing"

# (1) the view carries the VRF mapping + the protocol-params commitment (both folded into the hash).
grep -qE 'pub pool_vrf_keyhashes: BTreeMap<PoolId, Hash32>' "$RV" || fail "EpochConsensusView.pool_vrf_keyhashes missing"
grep -qE 'pub protocol_params_commitment: Hash32' "$RV" || fail "EpochConsensusView.protocol_params_commitment missing"

# (2) leadership-completeness + the projection + the FULL-profile commitment helper (not ASC-only).
grep -qE 'pub fn is_leadership_complete' "$RV" || fail "is_leadership_complete missing"
grep -qE 'pub fn to_pool_distr_view' "$RV" || fail "to_pool_distr_view projection missing (DC-EPOCH-12)"
grep -qE 'pub fn consensus_profile_commitment' "$RV" || fail "consensus_profile_commitment helper missing"
grep -qF 'genesis_hash.0' "$RV" || fail "the commitment does not cover the genesis hash (ASC-only commitment forbidden)"
grep -qF 'protocol_params_hash.0' "$RV" || fail "the commitment does not cover the protocol-params hash"

# (3) matches is INERT unless leadership-complete AND the commitment matches.
grep -qF 'self.is_leadership_complete()' "$RV" || fail "matches does not require leadership-completeness"
grep -qF 'self.protocol_params_commitment == b.protocol_params_commitment' "$RV" || fail "matches does not require the protocol-params commitment"

# (4) the projection fails closed on a wrong/unbound profile.
grep -qF 'ProjectionError::ParamsCommitmentMismatch' "$RV" || fail "the projection does not fail closed on a wrong profile"

# (5) derive_candidate builds the candidate by the delegated ∩ registered intersection + computes
#     the commitment from the bound profile (no I/O in derivation).
grep -qF 'drive_window_consensus_inputs' "$CAND" || fail "derive_candidate does not use the boundary-aware driver"
grep -qF 'inputs.pool_params.get(pool)' "$CAND" || fail "derive_candidate does not intersect delegated ∩ registered"
grep -qF 'consensus_profile_commitment(' "$CAND" || fail "derive_candidate does not compute the profile commitment"

# (6) the aggregate matches cardano numDelegators>0 -- NO zero-stake skip.
if grep -qF 'if cred_total.0 == 0' "$AGG"; then
    fail "aggregate_pool_stake still skips zero-stake delegated pools (cardano numDelegators>0 violated)"
fi

# (7) the load-bearing hermetic proofs.
grep -qF 'fn leadership_complete_required_for_matches' "$RV" || fail "leadership-complete/matches proof missing"
grep -qF 'fn to_pool_distr_view_builds_from_bound_profile_and_rejects_wrong_params' "$RV" || fail "projection proof missing"
grep -qF 'fn derive_candidate_canonical_hash_is_replay_equivalent' "$CAND" || fail "replay-equivalence proof missing"
grep -qF 'fn projection_rejects_wrong_profile_through_the_real_derive_path' "$CAND" || fail "wrong-profile-through-real-path proof missing"
grep -qF 'fn delegated_zero_stake_pool_is_included_with_zero' "$AGG" || fail "numDelegators>0 inclusion proof missing"

if (( FAILED == 0 )); then
    echo "OK: leadership-complete EpochConsensusView (DC-EVIEW-12 + DC-EPOCH-12; per-pool VRF + full-profile commitment in the canonical hash; projection exclusive + fail-closed; cardano numDelegators>0 aggregate)"
fi
exit $FAILED
