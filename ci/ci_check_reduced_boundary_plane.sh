#!/usr/bin/env bash
# ci_check_reduced_boundary_plane.sh -- REDUCED-VALIDATION-BOUNDARY-PLANE (P1+P2).
#
# The reduced follower (track_utxo=false) may never carry a half-real epoch ledger. At a Conway boundary it
# either has the inputs for a full authoritative transition, or it produces a clearly TYPED reduced projection
# -- never a degraded full transition, never a fabricated mark/set/go. This gate pins the type-level guarantees:
#
#   (A) EpochStakeSnapshots is the capability sum type; ReducedUnavailable exposes no authority (the only door is
#       as_authoritative()).
#   (B) No widening: no From/Into that turns a reduced projection into full authority.
#   (C) The two fail-closed doors (require_full, require_full_ledger) return FullBoundaryStateRequired.
#   (D) The production boundary transitions fail closed via as_authoritative().ok_or(FullBoundaryStateRequired).
#   (E) The reduced snapshot encoding + fingerprint are DISTINCT from authoritative (array(0) / reduced-unavailable
#       component header) so a reduced projection is never persisted or fingerprinted as authority.
#   (F) B1: the reduced dispatch is on the ACTUAL live follower path. A single `dispatch_epoch_boundary` routes by
#       plane before any full boundary execution; BOTH live crossers use it (no direct full-boundary call), the
#       reduced boundary sets cert/gov = ReducedUnavailable (unavailable by type), and a behavioral test drives the
#       real entry point (apply_block_with_verdicts) across a boundary to prove the reduced crossing carries NO
#       authority -- not a grep against a zero-caller helper.
#   (G) Deviation 2: ReducedEpochProgress carries cert_projection/governance_projection (Unavailable), never a full
#       CertState/ConwayGovState.
set -euo pipefail

EPOCH="crates/ade_ledger/src/epoch.rs"
RB="crates/ade_ledger/src/reduced_boundary.rs"
RULES="crates/ade_ledger/src/rules.rs"
FP="crates/ade_ledger/src/fingerprint.rs"
ES="crates/ade_ledger/src/snapshot/epoch_state.rs"
fail() { echo "FAIL (ci_check_reduced_boundary_plane): $1" >&2; exit 1; }
for f in "$EPOCH" "$RB" "$RULES" "$FP" "$ES"; do [ -f "$f" ] || fail "module $f missing"; done

# (A) The capability sum type + its named absence, with as_authoritative as the ONLY authority door.
grep -q "pub enum EpochStakeSnapshots" "$EPOCH" || fail "EpochStakeSnapshots enum missing"
grep -q "Authoritative(SnapshotState)" "$EPOCH" || fail "Authoritative variant missing"
grep -q "ReducedUnavailable" "$EPOCH" || fail "ReducedUnavailable variant missing"
grep -q "pub fn as_authoritative(" "$EPOCH" || fail "as_authoritative door missing"

# (B) No widening: a reduced projection can never be converted into full authority.
if grep -rn "impl.*From<ReducedBoundaryProjection>\|impl.*From<ReducedEpochProgress>\|impl.*From<ReducedBlockWindow>" \
     crates/*/src --include=*.rs | grep -v "//"; then
  fail "a From impl widens a reduced projection into authority -- forbidden (N-RVB-4)"
fi

# (C) The two typed fail-closed doors exist and terminal with FullBoundaryStateRequired.
grep -q "pub fn require_full(" "$RB" || fail "LedgerBoundaryVerdict::require_full missing"
grep -q "pub fn require_full_ledger(" "$RB" || fail "LedgerValidityCapability::require_full_ledger missing"
grep -q "FullBoundaryStateRequired" "$RB" || fail "reduced_boundary doors must fail closed with FullBoundaryStateRequired"
grep -q "StructuralValidity" "$RB" || fail "LedgerValidityCapability::StructuralValidity missing (I-RVB-3)"

# (D) The production authoritative boundary transitions require Authoritative and fail closed (never a fabricated
#     read). Both apply_epoch_boundary (epoch.rs) and apply_epoch_boundary_with_registrations (rules.rs).
for f in "$EPOCH" "$RULES"; do
  grep -q "snapshots.as_authoritative().ok_or(" "$f" \
    || fail "$f: authoritative boundary must gate snapshots via as_authoritative().ok_or(FullBoundaryStateRequired)"
done

# (E) Reduced encoding + fingerprint are DISTINCT from authoritative (gate 2/3): the snapshot encoder handles both
#     the legacy array(3) authoritative arm AND the reduced arm, and the fingerprint has a distinct reduced header.
grep -q "reduced-unavailable" "$FP" || fail "fingerprint must give ReducedUnavailable a distinct component header"
grep -q "ReducedUnavailable" "$ES" || fail "snapshot encoder must handle the ReducedUnavailable arm distinctly"

# (F) Gate 1 / N-RVB-1 + B1 REPAIR: a reduced (track_utxo=false) Conway boundary produces NO mark/set/go bytes and
#     NO advanced certificate/pool lifecycle, AND the reduced dispatch is on the ACTUAL live follower path -- not a
#     helper with zero production callers. A single dispatcher (`dispatch_epoch_boundary`) routes by validation
#     plane BEFORE any full boundary execution; BOTH live crossers (apply_shelley_era_block_with_verdicts,
#     apply_shelley_era_block_classified) route through it, so no production track_utxo=false Conway path can reach
#     the full boundary (which would fabricate a stub mark).
grep -q "pub fn apply_reduced_epoch_boundary" "$RULES" \
  || fail "apply_reduced_epoch_boundary (the reduced boundary transition) missing"
grep -q "snapshots = crate::epoch::EpochStakeSnapshots::ReducedUnavailable" "$RULES" \
  || fail "the reduced boundary must set snapshots = ReducedUnavailable (no fabricated mark, gate 1)"
grep -q "reduced.cert_state = crate::state::CertStateProjection::ReducedUnavailable" "$RULES" \
  || fail "the reduced boundary must make cert UNAVAILABLE BY TYPE (ReducedUnavailable), never a cleared full CertState"
grep -q "reduced.gov_state = crate::state::GovStateProjection::ReducedUnavailable" "$RULES" \
  || fail "the reduced boundary must make gov UNAVAILABLE BY TYPE (ReducedUnavailable), never a cleared full gov"
# The single dispatcher exists and routes track_utxo=false Conway to the reduced transition.
grep -q "fn dispatch_epoch_boundary" "$RULES" \
  || fail "the shared dispatch_epoch_boundary (the single plane-dispatch point) is missing (B1)"
grep -q "apply_reduced_epoch_boundary(state, new_epoch)" "$RULES" \
  || fail "dispatch_epoch_boundary must route track_utxo=false Conway boundaries to the reduced transition"
# BOTH live crossers route boundaries through the dispatcher (>=2 crosser call sites), and NONE of them reaches
# the full boundary directly (the old `apply_epoch_boundary_full(&current_state, ...)` direct call is gone).
dispatch_call_sites=$(grep -c "dispatch_epoch_boundary(&current_state, new_epoch)" "$RULES" || true)
[ "${dispatch_call_sites:-0}" -ge 2 ] \
  || fail "both live crossers must route the boundary through dispatch_epoch_boundary (found $dispatch_call_sites/2 crosser call sites) (B1)"
if grep -q "apply_epoch_boundary_full(&current_state" "$RULES"; then
  fail "a live crosser still calls apply_epoch_boundary_full directly -- track_utxo=false Conway would reach the FULL boundary (B1 regression)"
fi

# (F-live) The reduced dispatch is proven on the REAL follower ENTRY POINT (apply_block_with_verdicts), not merely
#          asserted by grep: the behavioral test drives a track_utxo=false Conway boundary crossing through the
#          actual path and asserts the post-state carries NO authority (snapshots/cert/gov = ReducedUnavailable).
RVBP_LIVE_TEST="crates/ade_ledger/tests/rvbp_live_path_reduced_dispatch.rs"
[ -f "$RVBP_LIVE_TEST" ] || fail "the live-follower-path reduced-dispatch behavioral test is missing (B1)"
cargo test -p ade_ledger --test rvbp_live_path_reduced_dispatch --quiet \
  || fail "the live follower path (apply_block_with_verdicts) must cross a Conway boundary in the REDUCED plane (B1)"

# (G) Deviation 2: the reduced boundary carries no full CertState/gov -- the typed reduced projection uses distinct
#     ReducedCertProjection/ReducedGovernanceProjection (Unavailable) on ReducedEpochProgress, never a full
#     CertState/ConwayGovState that could be mistaken for post-POOLREAP / enacted-governance state.
grep -q "pub enum ReducedCertProjection" "$RB" || fail "ReducedCertProjection type missing (deviation 2)"
grep -q "pub enum ReducedGovernanceProjection" "$RB" || fail "ReducedGovernanceProjection type missing (deviation 2)"
grep -q "cert_projection: ReducedCertProjection" "$RB" \
  || fail "ReducedEpochProgress must carry cert_projection: ReducedCertProjection, not a full CertState (deviation 2)"
grep -q "governance_projection: ReducedGovernanceProjection" "$RB" \
  || fail "ReducedEpochProgress must carry governance_projection: ReducedGovernanceProjection, not a full gov (deviation 2)"

echo "ok (ci_check_reduced_boundary_plane): reduced plane is typed non-authority; no widening; no mark or cert/gov lifecycle on the reduced boundary; the reduced dispatch is on the real follower path (both crossers); boundaries fail closed"
