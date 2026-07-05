// Core Contract:
// - Deterministic, no I/O.
// - The REDUCED-VALIDATION-BOUNDARY-PLANE: a track_utxo=false follower crossing a Conway epoch boundary produces
//   a typed reduced projection, NOT a degraded full transition and NOT a fabricated snapshot. See
//   docs/clusters/REDUCED-VALIDATION-BOUNDARY-PLANE/INVARIANTS.md.

//! Typed reduced boundary projection (P1). The authoritative epoch boundary
//! (`rules::apply_epoch_boundary_with_registrations`) is the SOLE producer of RUPD-applied rewards, pots,
//! mark/set/go, governance/pparam enactment, and CE-3d-comparable state — it REQUIRES point-bound
//! `BoundaryBaseStake`. When a reduced follower (`track_utxo=false`) reaches a Conway boundary it lacks those
//! inputs, so instead of building a reward-only stub mark (which the trace proved leaks into recovery
//! persistence + WAL fingerprints) it produces a [`ReducedBoundaryProjection`]: epoch/slot progression + a
//! distinct [`ReducedEpochProgress`] block-window rollover ONLY. Everything authoritative is UNAVAILABLE.
//!
//! The two results are DISTINCT, non-interchangeable types (I-RVB-1): a `ReducedBoundaryProjection` is not a
//! `LedgerState`, cannot be widened into one, cannot be serialized as an accumulator snapshot, and cannot be
//! fingerprinted as full authority.

use ade_types::{EpochNo, SlotNo};

/// The structural block-production window rolled over at a reduced boundary — the ONLY nesBcur-adjacent fact the
/// reduced plane may record. It is DELIBERATELY not the full `block_production`/`epoch_fees` reward-calculation
/// input: it carries no per-pool counts and no fees, so it CANNOT be fed to RUPD or epoch authority. Converting a
/// reduced follower's window into a full reward input requires a `FullBoundaryStateRequired` failure, never a
/// silent widening (N-RVB-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReducedBlockWindow {
    /// The reduced plane observed that the previous epoch's block-production window closed at this boundary.
    /// A structural fact (the window rolled), NOT the reward-bearing per-pool nesBprev.
    pub rolled_at_epoch: EpochNo,
}

/// The reduced facts advanced across a `track_utxo=false` Conway boundary: epoch/slot progression + the reduced
/// block-window rollover. Read-set audited (INVARIANTS §read-set): NOTHING else — no rewards, no pots, no
/// mark/set/go, no POOLREAP, no governance/pparam enactment — is derivable from reduced inputs, so nothing else
/// is present (absent, never stale/empty/inferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReducedEpochProgress {
    pub epoch: EpochNo,
    pub slot: SlotNo,
    pub reduced_block_window: ReducedBlockWindow,
}

/// A reduced-plane boundary result. A DISTINCT type from the authoritative `FullEpochBoundaryResult` /
/// `LedgerState` — it carries ONLY [`ReducedEpochProgress`], so it is structurally incapable of holding a
/// snapshot, reward account, pot, or governance effect. It can never feed `ActiveEpochAuthority`, leadership,
/// forging, CE-3d, or a full-ledger verdict (I-RVB-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReducedBoundaryProjection {
    pub progress: ReducedEpochProgress,
}

impl ReducedBoundaryProjection {
    /// Cross a `track_utxo=false` epoch boundary in the reduced plane. Advances ONLY epoch (to `new_epoch`) and
    /// records the block-window rollover; the header slot carries forward as the boundary point. Pure. No reward,
    /// pot, snapshot, POOLREAP, or governance effect is computed — the reduced plane lacks the inputs to do so
    /// correctly, and a partial/stale stand-in is forbidden (N-RVB-2/3).
    pub fn cross(from_epoch: EpochNo, new_epoch: EpochNo, slot: SlotNo) -> Self {
        ReducedBoundaryProjection {
            progress: ReducedEpochProgress {
                epoch: new_epoch,
                slot,
                reduced_block_window: ReducedBlockWindow { rolled_at_epoch: from_epoch },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_cross_advances_only_epoch_slot_window() {
        let p = ReducedBoundaryProjection::cross(EpochNo(1340), EpochNo(1341), SlotNo(115_862_416));
        assert_eq!(p.progress.epoch, EpochNo(1341), "epoch advances to the new epoch");
        assert_eq!(p.progress.slot, SlotNo(115_862_416), "the boundary slot carries forward");
        assert_eq!(
            p.progress.reduced_block_window.rolled_at_epoch,
            EpochNo(1340),
            "the reduced block window records the rolled-from epoch, nothing reward-bearing",
        );
    }

    /// The reduced projection is structurally incapable of carrying authoritative state — there is no field for a
    /// snapshot, reward, pot, or governance effect. This test documents that the type itself is the guarantee
    /// (I-RVB-1/N-RVB-4): the only public surface is epoch/slot + the reduced window.
    #[test]
    fn reduced_projection_has_no_authority_surface() {
        let p = ReducedBoundaryProjection::cross(EpochNo(1), EpochNo(2), SlotNo(10));
        // The whole type is Copy + carries only ReducedEpochProgress — nothing to convert into a reward input.
        let _progress: ReducedEpochProgress = p.progress;
    }
}
