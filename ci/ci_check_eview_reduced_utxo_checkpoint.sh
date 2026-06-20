#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3b-1 (DC-EVIEW-04): the durable reduced-UTxO checkpoint --
# the "minimal native state" (Option B). A disk-backed redb store of TxIn ->
# (Coin, ReducedStakeRef), built from the bootstrap UTxO, crash-safe (a completeness
# marker so a partial build is never mistaken for complete) and replay-equivalent
# (a hash-chain fingerprint over canonical records). The single ledger authority's
# own reduced-UTxO projection -- a GREEN durable cache, reconstructible by replay,
# NEVER authority, NEVER on the live track_utxo=false path.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
RED=crates/ade_ledger/src/reduced_utxo.rs
CP=crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs

test -f "$RED" || fail "the reduced-UTxO record/reduction ($RED) is missing"
test -f "$CP" || fail "the durable checkpoint store ($CP) is missing"

# (1) the Conway-specialized reduced reference (option b): Base | NonContributing.
grep -qE 'pub enum ReducedStakeRef' "$RED" || fail "ReducedStakeRef missing"
grep -qE 'Base\(StakeCredential\)' "$RED" || fail "ReducedStakeRef has no Base variant"
grep -qE 'NonContributing' "$RED" || fail "ReducedStakeRef has no NonContributing variant"

# (2) the reduction reuses Slice-2's classifier at Conway (no parallel decode).
grep -qE 'classify_output_stake_ref\(out\.address_bytes\(\), CardanoEra::Conway\)' "$RED" \
    || fail "reduce_txout does not reuse classify_output_stake_ref at Conway"

# (3) the durable store: completeness marker (crash-safety) + the build that writes it LAST.
grep -qE 'fn is_complete' "$CP" || fail "the completeness check (is_complete) is missing"
grep -qE 'COMPLETE_KEY' "$CP" || fail "the completeness marker key is missing"
grep -qE 'fn build_from' "$CP" || fail "build_from is missing"
grep -qE 'fn fingerprint' "$CP" || fail "the replay-equivalence fingerprint is missing"
# build clears any prior partial build first (rebuild-safe).
grep -qE 'delete_table\(REDUCED_TABLE\)' "$CP" || fail "build_from does not clear a prior partial build"

# (4) the load-bearing proofs.
grep -qE 'fn crash_mid_build_is_incomplete_then_rebuilds' "$CP" \
    || fail "the crash-mid-build recovery test is missing"
grep -qE 'fn replay_equivalent_two_builds_byte_identical' "$CP" \
    || fail "the replay-equivalence proof is missing"
grep -qE 'fn durable_across_reopen' "$CP" || fail "the durable-across-reopen proof is missing"
grep -qE 'fn fresh_store_is_incomplete' "$CP" || fail "the fresh-store-incomplete proof is missing"

# (5) S3b-1 boundary: NO advance / aggregation / live wiring / track_utxo. The
#     checkpoint is reachable only from S3b; the live producer path never references it.
if grep -qiE 'aggregate|new_mark|EpochConsensusView|leader|pool_distr' "$RED" "$CP"; then
    fail "S3b-1 reaches into aggregation / leader -- out of scope (S3c+)"
fi
if grep -rqE 'ReducedUtxoCheckpoint|reduce_txout' \
    crates/ade_node/src/node_lifecycle.rs crates/ade_node/src/node_sync.rs \
    crates/ade_runtime/src/admission/ crates/ade_runtime/src/forward_sync/ 2>/dev/null; then
    fail "the reduced-UTxO checkpoint is referenced on the live producer/follow path -- S3b-1 has no live wiring"
fi
if grep -qE '\.track_utxo *= *true|track_utxo: *true' "$RED" "$CP"; then
    fail "S3b-1 enables track_utxo=true -- the checkpoint is built off the per-block path"
fi

if (( FAILED == 0 )); then
    echo "OK: reduced-UTxO checkpoint (DC-EVIEW-04; durable disk-backed, crash-safe completeness, replay-equivalent, Conway-specialized Base|NonContributing, GREEN cache; no live wiring)"
fi
exit $FAILED
