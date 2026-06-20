#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3d (DC-EVIEW-06): snapshot formation + the k-immutability
# stability gate. Forms the MARK stake snapshot from the S3c aggregate, and adds the
# stability gate Ade lacked: a boundary snapshot/view is finalizable ONLY once its
# boundary block is > k (SecurityParam) deep (settled beyond rollback). Leader election
# reads the SET snapshot (the 2-epoch lag). Observe-only; no live wiring.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
S=crates/ade_ledger/src/reduced_snapshot.rs

test -f "$S" || fail "the snapshot/stability module ($S) is missing"

# (1) the conversion from the S3c aggregate to the mark snapshot.
grep -qE 'pub fn form_mark_snapshot' "$S" || fail "form_mark_snapshot missing"

# (2) the k-immutability stability gate: STRICTLY more than k deep (tip - boundary > k).
grep -qE 'pub fn is_boundary_stable' "$S" || fail "is_boundary_stable missing"
grep -qF 'tip_block_no.saturating_sub(boundary_block_no) > k.0' "$S" \
    || fail "the stability gate is not (tip - boundary) > k (strict, saturating)"

# (3) leader election reads the SET snapshot (the 2-epoch lag).
grep -qE 'pub const LEADERSHIP_SNAPSHOT_PHASE: SnapshotPhase = SnapshotPhase::Set' "$S" \
    || fail "LEADERSHIP_SNAPSHOT_PHASE is not Set (the 2-epoch lag)"

# (4) the boundary proof: k deep NOT stable, k+1 deep stable.
grep -qE 'fn stability_gate_requires_more_than_k_deep' "$S" \
    || fail "the stability boundary proof (k vs k+1) is missing"

# (5) observe-only: no live wiring of the stability gate / snapshot formation.
if grep -rqE 'is_boundary_stable|form_mark_snapshot' \
    crates/ade_node/src/node_lifecycle.rs crates/ade_node/src/node_sync.rs \
    crates/ade_runtime/src/admission/ crates/ade_runtime/src/forward_sync/ 2>/dev/null; then
    fail "the stability gate / snapshot formation is referenced on the live path -- S3d is observe-only"
fi

if (( FAILED == 0 )); then
    echo "OK: snapshot + stability gate (DC-EVIEW-06; mark formation, k-immutability gate (tip-boundary>k), leadership reads SET; observe-only)"
fi
exit $FAILED
