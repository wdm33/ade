#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-4a (DC-EPOCH-04 / DC-EPOCH-06 substrate): the WAL activation
# record. A distinct WalEntry::EpochConsensusViewActivated (TAG 4) records the ENTIRE
# activation identity -- the durable proof that THIS exact EpochConsensusView became
# authoritative for target_epoch at THIS exact selected-chain transition. Activation
# idempotence is EXPLICIT (same target epoch + byte-identical -> idempotent; differing ->
# conflict) and does NOT weaken the seed's DuplicateProvenance.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
E=crates/ade_ledger/src/wal/event.rs
R=crates/ade_ledger/src/wal/replay.rs

# (1) the distinct variant + its append-only tag 4.
grep -qE 'EpochConsensusViewActivated \{' "$E" || fail "the EpochConsensusViewActivated WAL variant is missing"
grep -qE 'TAG_EPOCH_CONSENSUS_VIEW_ACTIVATED: u64 = 4' "$E" || fail "TAG 4 (append-only) is missing"

# (2) it records the ENTIRE activation identity (not just hash + point).
for f in target_epoch network_magic era transition_point source_checkpoint_commitment \
         snapshot_phase nonce_commitment stake_view_canonical_hash view_canonical_hash; do
    grep -qE "$f" "$E" || fail "the activation record is missing the $f field"
done

# (3) explicit activation idempotence-vs-conflict (DC-EPOCH-04: at most one bound view per
#     target epoch) -- byte-identical => idempotent, differing => conflict.
grep -qE 'pub fn activation_replay_outcome' "$E" || fail "activation_replay_outcome (the idempotence/conflict rule) is missing"
grep -qE 'ActivationReplayOutcome::(Idempotent|Conflict)' "$E" || fail "the Idempotent/Conflict outcomes are missing"

# (4) the seed's DuplicateProvenance is NOT weakened (the activation rule is SEPARATE).
grep -qE 'return Err\(WalError::DuplicateProvenance\)' "$R" \
    || fail "the seed DuplicateProvenance guard was weakened/removed -- activation idempotence must be separate"

# (5) the load-bearing proofs (round-trip byte-identical + tag-4 + idempotent/conflict).
for t in wal_epoch_view_activated_round_trips_byte_identical wal_epoch_view_activated_uses_tag_four \
         activation_replay_idempotent_vs_conflict; do
    grep -qE "fn $t" "$E" || fail "the $t proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: WAL activation record (DC-EPOCH-04 substrate; EpochConsensusViewActivated TAG 4, full activation identity, explicit idempotence-vs-conflict, DuplicateProvenance intact)"
fi
exit $FAILED
