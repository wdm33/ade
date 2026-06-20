#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-2 (DC-EVIEW-10): the window driver. Advances the reduced UTxO
# checkpoint + the cert/delegation state forward over a window of blocks, then aggregates
# per-pool stake -- orchestrating the PROVEN pieces (reduced_block_delta == reduce(track_utxo);
# apply_block_delta; advance_cert_state == process_block_certificates; sum_base_credential_stake;
# aggregate_pool_stake) in order. Starts from the manifest-bound bootstrap cert state
# (DC-EVIEW-09), so PRE-bootstrap delegators are counted (NOT an empty map). Fail-closed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
D=crates/ade_runtime/src/chaindb/reduced_window_driver.rs

test -f "$D" || fail "the window driver ($D) is missing"

# (1) the driver exists and is exported.
grep -qE 'pub fn drive_window_aggregate' "$D" || fail "drive_window_aggregate missing"
grep -qE 'drive_window_aggregate' crates/ade_runtime/src/chaindb/mod.rs \
    || fail "drive_window_aggregate is not exported from chaindb"

# (2) it sequences the proven pieces in order: per-block delta -> apply; advance cert; then sum -> aggregate.
grep -qF 'reduced_block_delta(block, era)' "$D" || fail "driver does not compute the per-block reduced delta"
grep -qF 'apply_block_delta(&delta.spent, &delta.produced)' "$D" || fail "driver does not apply the delta to the checkpoint"
grep -qF 'advance_cert_state(block, era, &state)' "$D" || fail "driver does not advance the cert state"
grep -qF 'sum_base_credential_stake()' "$D" || fail "driver does not sum the per-credential UTxO stake"
grep -qF 'aggregate_pool_stake(&cred_utxo_stake' "$D" || fail "driver does not aggregate per-pool stake"

# (3) it STARTS from the bootstrap cert state (NOT an empty map) -- the pre-bootstrap
#     delegators are counted. The state is cloned from bootstrap_state, not LedgerState::new().
grep -qF 'let mut state = bootstrap_state.clone()' "$D" \
    || fail "driver does not start from the bootstrap cert state (the pre-bootstrap delegators would be lost)"

# (4) it mirrors the ledger's per-block cert application (cert_state + gov_state both threaded).
grep -qF 'state.cert_state = cert_state' "$D" || fail "driver does not thread cert_state forward"
grep -qF 'state.gov_state = gov_state' "$D" || fail "driver does not thread gov_state forward (ledger-faithful)"

# (5) fail-closed: every step maps to a WindowDriverError variant (no partial/wrong aggregate).
for v in 'Checkpoint(ReducedCheckpointError)' 'Ledger(LedgerError)' 'Aggregate(AggregateError)'; do
    grep -qF "$v" "$D" || fail "WindowDriverError is missing the $v fail-closed variant"
done

# (6) the load-bearing proofs (the bootstrap->aggregate wiring + the real-block sequencing).
for t in empty_window_aggregates_bootstrap_state real_conway_block_drive_equals_composed_pieces; do
    grep -qE "fn $t" "$D" || fail "the $t proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: window driver (DC-EVIEW-10; sequences the proven pieces over a window, starts from the bootstrap cert state, fail-closed; full-epoch proof = the boundary-aligned live oracle)"
fi
exit $FAILED
