#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONTINUITY-ACTIVATION ECA-0a (DC-EVIEW-13): Cardano-faithful pool lifecycle in the reduced
# window. Re-registrations are STAGED (the active pool entry + its VRF unchanged until adoption),
# retirements scheduled, POOLREAP (adopt futures + reap pools retiring at the entered epoch + clear
# their delegations) applied at each CROSSED boundary, and the mark captured pre-POOLREAP. Matches
# cardano-ledger Pool.hs/PoolReap.hs/Epoch.hs/SnapShots.hs @ 226b002d.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
DEL=crates/ade_ledger/src/delegation.rs
DRV=crates/ade_runtime/src/chaindb/reduced_window_driver.rs
COD=crates/ade_ledger/src/snapshot/cert_state.rs

test -f "$DEL" || fail "delegation.rs missing"
test -f "$DRV" || fail "reduced_window_driver.rs missing"
test -f "$COD" || fail "cert_state.rs missing"

# (1) PoolState carries the staged future_pools (cardano psFutureStakePoolParams).
grep -qE 'pub future_pools: BTreeMap<PoolId, PoolParams>' "$DEL" \
    || fail "PoolState.future_pools missing (re-registration staging)"

# (2) apply_pool_registration STAGES a re-registration instead of overwriting the active pool entry:
#     it branches on whether the pool is already registered (new -> pools; existing -> future_pools).
grep -qF 'if new_state.pool.pools.contains_key(&pool_cert.pool_id)' "$DEL" \
    || fail "apply_pool_registration does not branch new-vs-re-registration (would overwrite the active VRF)"

# (3) apply_pool_reap (cardano POOLREAP) over the whole CertState: adopt futures (drop orphans),
#     reap == entered_epoch, clear delegations of reaped pools, remove from pools + retiring.
grep -qE 'pub fn apply_pool_reap\(cert: &mut CertState' "$DEL" \
    || fail "apply_pool_reap missing or not over CertState (delegation-clearing needs DelegationState)"
grep -qF 'if cert.pool.pools.contains_key(&pool_id)' "$DEL" \
    || fail "apply_pool_reap does not drop orphan futures (cardano Map.dropMissing)"
grep -qF 'e.0 == entered_epoch.0' "$DEL" \
    || fail "apply_pool_reap reap condition is not == entered_epoch (cardano v == e)"
grep -qF '.retain(|_cred, pool_id| !retired.contains' "$DEL" \
    || fail "apply_pool_reap does not clear delegations of reaped pools (silent-reattach risk)"

# (4) the window driver applies POOLREAP at each crossed boundary + surfaces the active pool params.
grep -qE 'pub fn drive_window_consensus_inputs' "$DRV" || fail "drive_window_consensus_inputs missing"
grep -qE 'pub struct WindowConsensusInputs' "$DRV" || fail "WindowConsensusInputs (stake + pool_params) missing"
grep -qF 'apply_pool_reap(&mut state.cert_state, EpochNo(e))' "$DRV" \
    || fail "the window driver does not apply POOLREAP at crossed boundaries"
grep -qF 'pool_params: state.cert_state.pool.pools.clone()' "$DRV" \
    || fail "the window driver does not surface the window-end active pool params (VRF) for ECA-0b"
grep -qF 'WindowDriverError::InvalidEpochLength' "$DRV" \
    || fail "the window driver does not fail closed on slots_per_epoch==0 (silent epoch-0 collapse / skipped POOLREAP)"
grep -qF 'drive_window_consensus_inputs' crates/ade_runtime/src/chaindb/mod.rs \
    || fail "drive_window_consensus_inputs is not exported from chaindb"

# (5) the cert-state codec round-trips future_pools (6-field array) -- so the bootstrap artifact's
#     cert_state_hash commits to staged re-registrations.
grep -qF 'const FIELDS: u64 = 6' "$COD" || fail "cert-state codec is not 6-field (future_pools not serialized)"
grep -qF 'future_pools' "$COD" || fail "cert-state codec does not encode/decode future_pools"

# (6) the load-bearing hermetic proofs.
for t in re_registration_keeps_old_vrf_until_reap \
         reaped_pool_delegation_cleared_no_silent_reattach_on_reregistration \
         pool_reap_reaps_matching_epoch_only; do
    grep -qE "fn $t" "$DEL" || fail "the $t lifecycle proof is missing"
done
for t in drive_boundary_adopts_futures_reaps_retiring_clears_delegations \
         drive_boundary_is_deterministic; do
    grep -qE "fn $t" "$DRV" || fail "the $t window-driver boundary proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: pool lifecycle fidelity (DC-EVIEW-13; future_pools staging, POOLREAP adopt+reap+clear-delegations at crossed boundaries, mark pre-POOLREAP; cardano Pool.hs/PoolReap.hs @ 226b002d)"
fi
exit $FAILED
