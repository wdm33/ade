#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3b-2 (DC-EVIEW-04b): the windowed advance. reduced_block_delta
# is a FAITHFUL MIRROR of the ledger's own track_utxo (same extract_inputs_outputs_from_tx,
# same tx_hash, same produced keys) reduced to (Coin, ReducedStakeRef) -- so the reduced
# UTxO the boundary window maintains is the reduced PROJECTION of the ledger transition's
# own UTxO (single authority, not a parallel reimplementation), proven on a REAL Conway
# block. The cert/delegation/reward advance reuses the ledger's own
# process_block_certificates. The durable checkpoint advances by per-block delta
# (apply_block_delta), INCOMPLETE until finalize. No live producer-path change.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
ADV=crates/ade_ledger/src/reduced_advance.rs
CP=crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs

test -f "$ADV" || fail "the windowed advance ($ADV) is missing"

# (1) reduced_block_delta MIRRORS the ledger's own apply (reuses its extraction).
grep -qE 'pub fn reduced_block_delta' "$ADV" || fail "reduced_block_delta missing"
grep -qF 'extract_inputs_outputs_from_tx' "$ADV" \
    || fail "reduced_block_delta does not reuse the ledger's extract_inputs_outputs_from_tx"
grep -qF 'ade_crypto::blake2b::blake2b_256(wire_bytes)' "$ADV" \
    || fail "reduced_block_delta does not compute the tx_hash like track_utxo"

# (2) THE rigor proof: equality vs the ledger's track_utxo on a REAL Conway block.
grep -qE 'fn reduced_delta_equals_reduce_of_track_utxo_on_real_conway_block' "$ADV" \
    || fail "the real-block reduced==reduce(track_utxo) equality proof is missing"
grep -qF 'track_utxo(' "$ADV" || fail "the proof does not compare against the ledger's track_utxo"
grep -qF 'raw_era_block_conway.cbor' "$ADV" \
    || fail "the proof does not use the REAL conway block fixture (synthetic misses wire bugs)"
# intra-block chained-spend: a produced-then-spent output must be CANCELLED (not a
# phantom UTxO), matching the ledger's threaded track_utxo. (Security-review regression.)
grep -qE 'fn intra_block_chained_spend_cancels_phantom_matches_track_utxo' "$ADV" \
    || fail "the intra-block chained-spend regression (phantom cancellation) is missing"
grep -qF 'produced.remove(&input)' "$ADV" \
    || fail "reduced_block_delta does not cancel intra-block produced-then-spent outputs"

# (3) the cert advance reuses the ledger's OWN process_block_certificates (single authority).
grep -qE 'pub fn advance_cert_state' "$ADV" || fail "advance_cert_state missing"
grep -qF 'crate::rules::process_block_certificates(block, era, state)' "$ADV" \
    || fail "advance_cert_state does not reuse the ledger's process_block_certificates"

# (4) the durable checkpoint advance: apply_block_delta + finalize, incomplete-until-finalize.
grep -qE 'pub fn apply_block_delta' "$CP" || fail "apply_block_delta missing"
grep -qE 'pub fn finalize' "$CP" || fail "finalize missing"
grep -qE 'fn apply_block_delta_then_finalize' "$CP" || fail "the apply+finalize proof is missing"
grep -qE 'fn advance_over_real_conway_block_matches_build_from' "$CP" \
    || fail "the real-block advance==build_from end-to-end proof is missing"

# (5) S3b-2 boundary: NO EpochConsensusView emission / leader-schedule use here
#     (those TYPES belong to S3d/S3e); NO live wiring.
if grep -qE 'EpochConsensusView|query_leader_schedule|stake_by_pool' "$ADV"; then
    fail "S3b-2 reaches into emission / leader-schedule -- out of scope (S3d/S3e)"
fi
if grep -rqE 'reduced_block_delta|apply_block_delta|advance_cert_state' \
    crates/ade_node/src/node_lifecycle.rs crates/ade_node/src/node_sync.rs \
    crates/ade_runtime/src/admission/ crates/ade_runtime/src/forward_sync/ 2>/dev/null; then
    fail "the windowed advance is referenced on the live producer/follow path -- S3b-2 has no live wiring"
fi

if (( FAILED == 0 )); then
    echo "OK: windowed advance (DC-EVIEW-04b; reduced_block_delta MIRRORS track_utxo proven on a REAL conway block, cert advance reuses process_block_certificates, checkpoint apply+finalize; single authority, no live wiring)"
fi
exit $FAILED
