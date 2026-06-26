// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Ledger view trait surface — the typed boundary by which BLUE
//! consensus consumes ledger-owned stake snapshots without taking
//! ownership of them.
//!
//! This module declares only the trait. Implementations live elsewhere:
//! a test-only GREEN stub in `ade_testkit::consensus::ledger_view_stub`,
//! and a future production-grade impl in `ade_ledger`.
//!
//! The trait is intentionally small. Each method returns `Option<...>`
//! to encode "snapshot has no opinion" — callers that need a typed
//! consensus error (e.g. `LeaderScheduleError::UnknownPool`) map the
//! `None` themselves so the failure taxonomy stays in BLUE.

use ade_types::{EpochNo, Hash28, Hash32};

use crate::consensus::vrf_cert::ActiveSlotsCoeff;

/// Stake snapshot frozen at epoch E-2, surfaced for the active
/// epoch E. Consumed by-reference; never owned by BLUE consensus.
///
/// The trait is the canonical surface BLUE consensus uses to consult
/// ledger-owned state for leader scheduling and header validation.
/// Implementations must preserve determinism: same `(epoch, pool)`
/// queries must yield byte-identical answers across runs.
pub trait LedgerView {
    /// Total active stake (lovelace) across all registered pools
    /// for the current operating epoch.
    fn total_active_stake(&self, epoch: EpochNo) -> Option<u64>;

    /// Active stake for one pool. Returns `None` if the pool is
    /// unknown to this snapshot.
    fn pool_active_stake(&self, epoch: EpochNo, pool: &Hash28) -> Option<u64>;

    /// Pool's registered VRF key *hash* (`blake2b-256` of the VRF
    /// verification key) for the operating epoch. The ledger holds the
    /// keyhash, not the vkey; the vkey itself arrives in the block header,
    /// and header validation binds the two by checking
    /// `blake2b_256(header.vrf_vkey) == pool_vrf_keyhash`. Returns `None`
    /// if the pool is unknown to this snapshot.
    fn pool_vrf_keyhash(&self, epoch: EpochNo, pool: &Hash28) -> Option<Hash32>;

    /// Active-slots-coefficient for the operating epoch — pulled
    /// from the era's protocol parameters; ledger surfaces it so
    /// BLUE has one canonical source for `f`.
    fn active_slots_coeff(&self, epoch: EpochNo) -> Option<ActiveSlotsCoeff>;
}
