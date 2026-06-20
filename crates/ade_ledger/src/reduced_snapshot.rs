// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3d (DC-EVIEW-06) — snapshot formation + the k-immutability
//! stability gate.
//!
//! Forms the MARK stake snapshot from the S3c per-pool aggregate and exposes the
//! k-IMMUTABILITY STABILITY GATE that Ade currently lacks. mark/set/go rotation already
//! exists (`crate::epoch::rotate_snapshots`); this slice adds (a) the conversion of the
//! S3c [`StakeByPool`] into a [`StakeSnapshot`], and (b) the STABILITY GATE: a boundary
//! snapshot / view may be FINALIZED (used) ONLY once its defining boundary block is
//! `> k` (the security parameter, 2160) deep — i.e. settled beyond rollback. cardano
//! forces the lazy MARK snapshot only after one stability window for exactly this
//! reason; a snapshot used before its boundary is immutable could be invalidated by a
//! rollback.
//!
//! Phase semantics (cardano): leader election for epoch L reads the SET snapshot — the
//! MARK captured at the previous epoch boundary — giving the 2-epoch lag (the stake at
//! the (L-2 -> L-1) boundary drives leadership in epoch L). GO drives rewards (3-epoch
//! lag). The existing `rotate_snapshots` (mark <- new_mark, set <- old mark, go <- old
//! set) encodes the lag; [`LEADERSHIP_SNAPSHOT_PHASE`] names the phase leadership reads.
//!
//! OBSERVE-ONLY: nothing here is wired to live leader election or the boundary authority
//! (DC-EVIEW-08 activation). NO live-path change.

use std::collections::BTreeMap;

use ade_core::consensus::SecurityParam;

use crate::epoch::StakeSnapshot;
use crate::reduced_aggregate::StakeByPool;

/// Which mark/set/go phase drives a leader-election epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotPhase {
    /// Freshly computed at the epoch boundary (from the just-closed epoch's stake).
    Mark,
    /// The MARK from one boundary earlier; drives LEADER ELECTION (2-epoch lag).
    Set,
    /// The SET from one boundary earlier; drives REWARDS (3-epoch lag).
    Go,
}

/// Leader election reads the SET snapshot.
pub const LEADERSHIP_SNAPSHOT_PHASE: SnapshotPhase = SnapshotPhase::Set;

/// Form the MARK stake snapshot from the S3c per-pool aggregate. The pool-level
/// `pool_stakes` is what leader election consumes (a pool's relative stake); the
/// per-credential `delegations` detail was aggregated away in S3c and is not needed for
/// the leadership view.
pub fn form_mark_snapshot(stake: &StakeByPool) -> StakeSnapshot {
    StakeSnapshot {
        delegations: BTreeMap::new(),
        pool_stakes: stake.pool_stakes.clone(),
    }
}

/// The k-IMMUTABILITY STABILITY GATE: a boundary at `boundary_block_no` is STABLE
/// (its snapshot/view finalizable) at chain tip `tip_block_no` iff the boundary is more
/// than `k` blocks deep — i.e. `tip - boundary > k`. A boundary that is not yet `> k`
/// deep could still be rolled back (DC-NODE-29 / `SecurityParam`), so a snapshot derived
/// from it MUST NOT be finalized/used yet. Saturating subtraction: a boundary ahead of
/// the tip (degenerate) is never stable.
pub fn is_boundary_stable(boundary_block_no: u64, tip_block_no: u64, k: SecurityParam) -> bool {
    tip_block_no.saturating_sub(boundary_block_no) > k.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::tx::{Coin, PoolId};
    use ade_types::Hash28;

    fn pool(fill: u8) -> PoolId {
        PoolId(Hash28([fill; 28]))
    }

    // The conversion: S3c per-pool aggregate -> the mark snapshot's pool_stakes.
    #[test]
    fn forms_mark_snapshot_from_aggregate() {
        let stake = StakeByPool {
            pool_stakes: [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(),
            total_active_stake: Coin(300),
        };
        let snap = form_mark_snapshot(&stake);
        assert_eq!(snap.pool_stakes, stake.pool_stakes);
        assert!(snap.delegations.is_empty());
    }

    // The stability gate boundary: with k=2160, a boundary exactly k deep is NOT yet
    // stable; one more block (k+1 deep) is stable.
    #[test]
    fn stability_gate_requires_more_than_k_deep() {
        let k = SecurityParam(2160);
        // boundary at block 1000.
        let b = 1000u64;
        assert!(!is_boundary_stable(b, b, k), "depth 0 is not stable");
        assert!(!is_boundary_stable(b, b + 2160, k), "exactly k deep is NOT yet stable");
        assert!(is_boundary_stable(b, b + 2161, k), "k+1 deep IS stable");
        assert!(is_boundary_stable(b, b + 10_000, k), "deep past k is stable");
    }

    // A boundary ahead of the tip (degenerate) is never stable (saturating).
    #[test]
    fn boundary_ahead_of_tip_is_not_stable() {
        let k = SecurityParam(2160);
        assert!(!is_boundary_stable(5000, 4000, k));
    }

    // Leader election reads SET (the 2-epoch lag).
    #[test]
    fn leadership_reads_the_set_snapshot() {
        assert_eq!(LEADERSHIP_SNAPSHOT_PHASE, SnapshotPhase::Set);
    }

    // The existing rotation still composes: rotate the formed mark; leadership then reads
    // the SET (the prior mark).
    #[test]
    fn formed_mark_rotates_into_set() {
        use crate::epoch::SnapshotState;
        let prior = SnapshotState::new();
        let stake = StakeByPool {
            pool_stakes: [(pool(7), Coin(42))].into_iter().collect(),
            total_active_stake: Coin(42),
        };
        let mark = form_mark_snapshot(&stake);
        let rotated = crate::epoch::rotate_snapshots(&prior, mark.clone());
        // after rotation, the new SET is the PRIOR mark (empty here); the new MARK is ours.
        assert_eq!(rotated.mark.0.pool_stakes, mark.pool_stakes);
    }
}
