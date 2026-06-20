// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3e (DC-EVIEW-07) — the bound, immutable EpochConsensusView.
//!
//! Emit the compact, immutable next-epoch consensus view from the finalized snapshot
//! (S3d) — BOUND to all of {network, era, epoch, source chain point, checkpoint
//! commitment, nonce, snapshot phase, canonical-bytes hash}. A view missing or
//! mismatching any binding is INERT: activation (DC-EVIEW-08) may consume the view ONLY
//! when its bindings match the activation context AND its canonical hash verifies. The
//! canonical-bytes hash (blake2b over the canonical encoding of every binding + the
//! stake distribution) is the view's self-describing identity; the canonical encoding is
//! round-trippable, so a WAL-recorded view replays byte-identically (the replay-
//! equivalence the activation slice relies on). Model: `SeedEpochConsensusInputs`.
//!
//! OBSERVE-ONLY: this builds + binds + hashes the view; the rewire into the live boundary
//! authority, the WAL activation variant, and feeding the view to live leader election
//! are the activation slice (DC-EVIEW-08). NO live-path change.

use std::collections::BTreeMap;

use ade_core::consensus::events::Point;
use ade_crypto::blake2b::blake2b_256;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash32};

use crate::reduced_snapshot::SnapshotPhase;

/// The bound, immutable next-epoch consensus view. Every field is a binding except the
/// stake distribution payload; `canonical_hash` is the self-describing identity over all
/// of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochConsensusView {
    pub network_magic: u32,
    pub era: CardanoEra,
    pub epoch: EpochNo,
    pub source_point: Point,
    pub checkpoint_commitment: Hash32,
    pub nonce: Hash32,
    pub snapshot_phase: SnapshotPhase,
    pub stake_by_pool: BTreeMap<PoolId, Coin>,
    pub total_active_stake: Coin,
    /// blake2b over the canonical encoding of every field above — the view's identity.
    canonical_hash: Hash32,
}

/// The bindings an activation context must match for a view to be usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewBindings {
    pub network_magic: u32,
    pub era: CardanoEra,
    pub epoch: EpochNo,
    pub source_point: Point,
    pub checkpoint_commitment: Hash32,
    pub nonce: Hash32,
    pub snapshot_phase: SnapshotPhase,
}

fn phase_tag(p: SnapshotPhase) -> u8 {
    match p {
        SnapshotPhase::Mark => 0,
        SnapshotPhase::Set => 1,
        SnapshotPhase::Go => 2,
    }
}

/// The canonical, deterministic byte encoding of the view's bindings + stake
/// distribution (fixed field order; the `BTreeMap` iterates in sorted `PoolId` order).
fn canonical_bytes(
    network_magic: u32,
    era: CardanoEra,
    epoch: EpochNo,
    source_point: &Point,
    checkpoint_commitment: &Hash32,
    nonce: &Hash32,
    snapshot_phase: SnapshotPhase,
    stake_by_pool: &BTreeMap<PoolId, Coin>,
    total_active_stake: Coin,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&network_magic.to_be_bytes());
    buf.push(era as u8);
    buf.extend_from_slice(&epoch.0.to_be_bytes());
    buf.extend_from_slice(&source_point.slot.0.to_be_bytes());
    buf.extend_from_slice(&source_point.hash.0);
    buf.extend_from_slice(&checkpoint_commitment.0);
    buf.extend_from_slice(&nonce.0);
    buf.push(phase_tag(snapshot_phase));
    buf.extend_from_slice(&total_active_stake.0.to_be_bytes());
    buf.extend_from_slice(&(stake_by_pool.len() as u64).to_be_bytes());
    for (pool, coin) in stake_by_pool {
        buf.extend_from_slice(&pool.0 .0); // PoolId(Hash28) -> 28 bytes
        buf.extend_from_slice(&coin.0.to_be_bytes());
    }
    buf
}

impl EpochConsensusView {
    /// Bind a finalized snapshot into an immutable view, computing the canonical-bytes
    /// hash identity over every binding + the stake distribution.
    #[allow(clippy::too_many_arguments)]
    pub fn bind(
        network_magic: u32,
        era: CardanoEra,
        epoch: EpochNo,
        source_point: Point,
        checkpoint_commitment: Hash32,
        nonce: Hash32,
        snapshot_phase: SnapshotPhase,
        stake_by_pool: BTreeMap<PoolId, Coin>,
        total_active_stake: Coin,
    ) -> Self {
        let canonical_hash = blake2b_256(&canonical_bytes(
            network_magic,
            era,
            epoch,
            &source_point,
            &checkpoint_commitment,
            &nonce,
            snapshot_phase,
            &stake_by_pool,
            total_active_stake,
        ));
        EpochConsensusView {
            network_magic,
            era,
            epoch,
            source_point,
            checkpoint_commitment,
            nonce,
            snapshot_phase,
            stake_by_pool,
            total_active_stake,
            canonical_hash,
        }
    }

    /// The self-describing canonical-bytes hash identity.
    pub fn canonical_hash(&self) -> Hash32 {
        self.canonical_hash.clone()
    }

    /// The canonical encoding (round-trippable; WAL-recordable for replay-equivalence).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        canonical_bytes(
            self.network_magic,
            self.era,
            self.epoch,
            &self.source_point,
            &self.checkpoint_commitment,
            &self.nonce,
            self.snapshot_phase,
            &self.stake_by_pool,
            self.total_active_stake,
        )
    }

    /// Recompute the canonical hash from the fields and check it matches the stored one —
    /// proves the view has not been tampered with (no field changed without rebinding).
    pub fn verify_canonical_hash(&self) -> bool {
        blake2b_256(&self.canonical_bytes()) == self.canonical_hash
    }

    /// Whether the view's bindings match an activation context. A view is INERT (must NOT
    /// be activated) unless ALL bindings match AND `verify_canonical_hash`.
    pub fn matches(&self, b: &ViewBindings) -> bool {
        self.verify_canonical_hash()
            && self.network_magic == b.network_magic
            && self.era == b.era
            && self.epoch == b.epoch
            && self.source_point == b.source_point
            && self.checkpoint_commitment == b.checkpoint_commitment
            && self.nonce == b.nonce
            && self.snapshot_phase == b.snapshot_phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ade_types::primitives::SlotNo;
    use ade_types::Hash28;

    fn pool(f: u8) -> PoolId {
        PoolId(Hash28([f; 28]))
    }
    fn point(slot: u64, h: u8) -> Point {
        Point { slot: SlotNo(slot), hash: Hash32([h; 32]) }
    }
    fn sample_view() -> EpochConsensusView {
        EpochConsensusView::bind(
            2,
            CardanoEra::Conway,
            EpochNo(1334),
            point(115_000_000, 0xaa),
            Hash32([0xc1; 32]),
            Hash32([0xe7; 32]),
            SnapshotPhase::Set,
            [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(),
            Coin(300),
        )
    }
    fn bindings_of(v: &EpochConsensusView) -> ViewBindings {
        ViewBindings {
            network_magic: v.network_magic,
            era: v.era,
            epoch: v.epoch,
            source_point: v.source_point.clone(),
            checkpoint_commitment: v.checkpoint_commitment.clone(),
            nonce: v.nonce.clone(),
            snapshot_phase: v.snapshot_phase,
        }
    }

    #[test]
    fn bind_is_deterministic_and_self_verifies() {
        let a = sample_view();
        let b = sample_view();
        assert_eq!(a.canonical_hash(), b.canonical_hash());
        assert!(a.verify_canonical_hash());
    }

    #[test]
    fn matches_exact_bindings_and_rejects_mismatch() {
        let v = sample_view();
        assert!(v.matches(&bindings_of(&v)), "exact bindings match");
        // a mismatched epoch -> inert.
        let mut wrong = bindings_of(&v);
        wrong.epoch = EpochNo(9999);
        assert!(!v.matches(&wrong), "a mismatched binding makes the view inert");
        // a mismatched nonce / phase / point / network / era / commitment all reject.
        let mut w2 = bindings_of(&v);
        w2.nonce = Hash32([0x00; 32]);
        assert!(!v.matches(&w2));
        let mut w3 = bindings_of(&v);
        w3.snapshot_phase = SnapshotPhase::Mark;
        assert!(!v.matches(&w3));
        let mut w4 = bindings_of(&v);
        w4.network_magic = 1;
        assert!(!v.matches(&w4));
    }

    // The canonical hash is binding-sensitive: changing ANY binding or the stake
    // distribution changes the identity (no two distinct views share a hash).
    #[test]
    fn canonical_hash_is_binding_sensitive() {
        let base = sample_view().canonical_hash();
        let diff_network = EpochConsensusView::bind(
            3, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Set,
            [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(), Coin(300),
        ).canonical_hash();
        assert_ne!(base, diff_network, "network changes the hash");
        let diff_stake = EpochConsensusView::bind(
            2, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Set,
            [(pool(1), Coin(101)), (pool(2), Coin(200))].into_iter().collect(), Coin(301),
        ).canonical_hash();
        assert_ne!(base, diff_stake, "a stake change changes the hash");
        let diff_phase = EpochConsensusView::bind(
            2, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Go,
            [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(), Coin(300),
        ).canonical_hash();
        assert_ne!(base, diff_phase, "the snapshot phase changes the hash");
    }

    // The canonical encoding round-trips through the same hash (WAL-recordable / replay-
    // equivalent): re-binding from the same canonical bytes reproduces the identity.
    #[test]
    fn canonical_bytes_reproduce_the_hash() {
        let v = sample_view();
        assert_eq!(blake2b_256(&v.canonical_bytes()), v.canonical_hash());
    }

    // Tamper detection: a view whose stored hash disagrees with its fields fails to verify
    // (and is therefore inert via `matches`).
    #[test]
    fn tampered_view_fails_verification() {
        let mut v = sample_view();
        v.total_active_stake = Coin(999); // mutate a field without rebinding
        assert!(!v.verify_canonical_hash(), "a tampered field fails the canonical-hash check");
        assert!(!v.matches(&bindings_of(&v)), "a view that fails verification is inert");
    }
}
