#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW DC-EVIEW-08 / S3f-1: the activation consumption point. The epoch
# boundary CAN consume the S3c per-pool aggregate as the new MARK (form_mark_snapshot)
# when one is provided; the live path (apply_epoch_boundary_full) passes None, so the
# live boundary behaviour is UNCHANGED (fail-safe). The FULL live activation -- the
# window driver (S3f-2), the ledger_view rebind seam (S3f-3), and feeding the bound view
# to live leader election (S3f-4) -- is NOT YET BUILT and is gated on two LIVE cardano-node
# proofs (the boundary-aligned stake oracle + the leadership-schedule proof). This gate
# enforces the fail-safe consumption point AND that no live flip has happened yet.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
R=crates/ade_ledger/src/rules.rs

# (1) the boundary takes the precomputed-mark aggregate and uses it when provided.
grep -qE 'precomputed_mark: Option<&crate::reduced_aggregate::StakeByPool>' "$R" \
    || fail "apply_epoch_boundary_with_registrations does not take the precomputed-mark aggregate"
grep -qF 'Some(agg) => crate::reduced_snapshot::form_mark_snapshot(agg)' "$R" \
    || fail "the boundary does not build the new MARK from the aggregate when provided"

# (2) FAIL-SAFE: the live path (apply_epoch_boundary_full) passes None -> the stub is used,
#     UNCHANGED. (A live behaviour change would require this to pass Some.)
grep -qF 'apply_epoch_boundary_with_registrations(state, new_epoch, None, None)' "$R" \
    || fail "apply_epoch_boundary_full does not pass None for the precomputed mark -- the live boundary must be UNCHANGED until S3f-4"

# (3) the fail-safe proof: both the Some(agg) and the None (stub) paths are pinned.
grep -qE 'fn epoch_boundary_consumes_precomputed_aggregate_mark' "$R" \
    || fail "the S3f-1 consume/stub fail-safe proof is missing"

# (4) the live flip has NOT happened: the EpochConsensusView is NOT yet fed to live leader
#     election (no derived-view PoolDistrView on the live forge path). S3f-4 is gated.
if grep -rqE 'EpochConsensusView|reduced_epoch_view' crates/ade_node/src/node_sync.rs 2>/dev/null; then
    fail "the EpochConsensusView is wired into the live forge path (node_sync) -- S3f-4 is gated on the live proofs, not yet permitted"
fi

if (( FAILED == 0 )); then
    echo "OK: activation consumption point (DC-EVIEW-08 S3f-1; boundary consumes the aggregate when provided, live path passes None=UNCHANGED fail-safe; live flip S3f-4 NOT done, gated on live proofs)"
fi
exit $FAILED
