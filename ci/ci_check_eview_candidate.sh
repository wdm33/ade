#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-4d-2 (DC-EPOCH-09): the activation candidate derivation. A
# VALIDATED source window (DC-EPOCH-08) drives the reduced checkpoint + cert state
# (DC-EVIEW-10) -> per-pool stake, bound (DC-EVIEW-07) into an EpochConsensusView with the
# window's TARGET-epoch context (NOT source_epoch). Candidate binding happens BEFORE WAL
# activation; the candidate's identity is exactly what the WAL record commits to + recovery
# reproduces. Fail closed -- no partial candidate reaches the predicate.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
C=crates/ade_node/src/epoch_candidate.rs

test -f "$C" || fail "the candidate module ($C) is missing"

# (1) it composes the proven pieces (the driver + bind).
grep -qE 'pub fn derive_candidate' "$C" || fail "derive_candidate missing"
grep -qE 'drive_window_consensus_inputs\(' "$C" \
    || fail "derive_candidate does not drive the window (DC-EVIEW-10)"
grep -qF 'EpochConsensusView::bind(' "$C" || fail "derive_candidate does not bind the view (DC-EVIEW-07)"

# (2) bound to the WINDOW's TARGET epoch (the Mark->Set lag), NOT source_epoch.
grep -qF 'window.target_epoch' "$C" || fail "the candidate is not bound to the window's target_epoch"
if grep -qE 'bind\([^)]*window\.source_epoch' "$C"; then
    fail "the candidate is bound to source_epoch -- it must be the target_epoch (off-by-one hazard)"
fi

# (3) the source checkpoint commitment is the (finalized) window-end checkpoint.
grep -qF 'checkpoint' "$C" || fail "no checkpoint commitment"
grep -qE '\.finalize\(\)' "$C" || fail "the checkpoint commitment is not the finalized window-end commitment"

# (4) fail closed -- no partial candidate.
grep -qE 'enum CandidateDeriveError' "$C" || fail "CandidateDeriveError missing"
for e in Drive Checkpoint; do
    grep -qE "CandidateDeriveError::$e" "$C" || fail "CandidateDeriveError is missing the $e fail-closed variant"
done

# (5) the load-bearing proof (target-epoch binding + WAL/recovery round-trip).
grep -qE 'fn derive_candidate_binds_target_epoch_and_round_trips_through_recovery' "$C" \
    || fail "the target-epoch-binding + round-trip proof is missing"

if (( FAILED == 0 )); then
    echo "OK: candidate derivation (DC-EPOCH-09; window -> drive -> bind to the TARGET-epoch context, finalized checkpoint commitment, round-trips through the WAL record, fail-closed)"
fi
exit $FAILED
