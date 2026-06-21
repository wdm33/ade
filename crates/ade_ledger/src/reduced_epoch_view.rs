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
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_crypto::blake2b::blake2b_256;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32};

use crate::consensus_view::{PoolDistrView, PoolEntry};
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
    /// Per-pool registered VRF keyhash, effective for this snapshot (ECA-0b). Same key set as
    /// `stake_by_pool` (leadership-complete) — every included pool has BOTH its active stake and
    /// its era-correct VRF key, so the rebind builds a complete `PoolDistrView` from this view alone.
    pub pool_vrf_keyhashes: BTreeMap<PoolId, Hash32>,
    pub total_active_stake: Coin,
    /// Commitment to the leadership-relevant protocol/genesis parameters (ECA-0b; the ASC). The
    /// `PoolDistrView` projection resolves the ASC ONLY through this commitment, so the promoted view
    /// is unusable under a different parameter set (no unbound protocol-param read).
    pub protocol_params_commitment: Hash32,
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
    pub protocol_params_commitment: Hash32,
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
    pool_vrf_keyhashes: &BTreeMap<PoolId, Hash32>,
    total_active_stake: Coin,
    protocol_params_commitment: &Hash32,
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
    // ECA-0b: the protocol-params commitment (a binding) + the per-pool VRF keyhashes (payload).
    buf.extend_from_slice(&protocol_params_commitment.0);
    buf.extend_from_slice(&(pool_vrf_keyhashes.len() as u64).to_be_bytes());
    for (pool, vrf) in pool_vrf_keyhashes {
        buf.extend_from_slice(&pool.0 .0); // PoolId(Hash28) -> 28 bytes
        buf.extend_from_slice(&vrf.0); // Hash32 -> 32 bytes
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
        pool_vrf_keyhashes: BTreeMap<PoolId, Hash32>,
        total_active_stake: Coin,
        protocol_params_commitment: Hash32,
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
            &pool_vrf_keyhashes,
            total_active_stake,
            &protocol_params_commitment,
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
            pool_vrf_keyhashes,
            total_active_stake,
            protocol_params_commitment,
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
            &self.pool_vrf_keyhashes,
            self.total_active_stake,
            &self.protocol_params_commitment,
        )
    }

    /// A commitment to JUST the stake distribution (per-pool stakes + total), distinct
    /// from the full-view identity hash. Recorded in the WAL activation record (S3f-4a) so
    /// the durable activation identity pins the stake content independently of the bindings.
    pub fn stake_view_canonical_hash(&self) -> Hash32 {
        let mut buf = Vec::with_capacity(16 + self.stake_by_pool.len() * 36);
        buf.extend_from_slice(&self.total_active_stake.0.to_be_bytes());
        buf.extend_from_slice(&(self.stake_by_pool.len() as u64).to_be_bytes());
        for (pool, coin) in &self.stake_by_pool {
            buf.extend_from_slice(&pool.0 .0); // PoolId(Hash28) -> 28 bytes
            buf.extend_from_slice(&coin.0.to_be_bytes());
        }
        blake2b_256(&buf)
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
            && self.is_leadership_complete()
            && self.network_magic == b.network_magic
            && self.era == b.era
            && self.epoch == b.epoch
            && self.source_point == b.source_point
            && self.checkpoint_commitment == b.checkpoint_commitment
            && self.nonce == b.nonce
            && self.snapshot_phase == b.snapshot_phase
            && self.protocol_params_commitment == b.protocol_params_commitment
    }

    /// Leadership-complete (DC-EVIEW-12): the key sets of `stake_by_pool` and `pool_vrf_keyhashes`
    /// are identical — every staked pool has an era-correct VRF keyhash and vice versa. An
    /// incomplete view is INERT (cannot be activated via `matches` nor projected).
    pub fn is_leadership_complete(&self) -> bool {
        // Both are BTreeMap (sorted-key iteration), so equal key SEQUENCES (Iterator::eq, which also
        // checks equal length) ⇒ equal key SETS.
        self.stake_by_pool.keys().eq(self.pool_vrf_keyhashes.keys())
    }

    /// Project the promoted view into the leadership `PoolDistrView` (DC-EPOCH-12). Derived
    /// EXCLUSIVELY from this view + the bound-commitment-checked ASC: no live CertState read, no
    /// unbound protocol-parameter read. Fail-closed unless (1) the supplied consensus profile
    /// (genesis + protocol-params + ASC) matches the bound `protocol_params_commitment`, and (2) the
    /// view is leadership-complete.
    pub fn to_pool_distr_view(
        &self,
        genesis_hash: &Hash32,
        protocol_params_hash: &Hash32,
        asc: ActiveSlotsCoeff,
    ) -> Result<PoolDistrView, ProjectionError> {
        if consensus_profile_commitment(genesis_hash, protocol_params_hash, asc)
            != self.protocol_params_commitment
        {
            return Err(ProjectionError::ParamsCommitmentMismatch);
        }
        if !self.is_leadership_complete() {
            return Err(ProjectionError::NotLeadershipComplete);
        }
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool, coin) in &self.stake_by_pool {
            // is_leadership_complete guarantees the VRF keyhash is present for every staked pool.
            let vrf_keyhash = self
                .pool_vrf_keyhashes
                .get(pool)
                .ok_or(ProjectionError::NotLeadershipComplete)?
                .clone();
            pools.insert(pool.0.clone(), PoolEntry { active_stake: coin.0, vrf_keyhash });
        }
        Ok(PoolDistrView::new(self.epoch, self.total_active_stake.0, asc, pools))
    }
}

/// The canonical commitment to the leadership-relevant consensus profile (ECA-0b): the genesis
/// profile + the protocol parameters + the ASC the projection consumes. Reuses the canonical
/// `genesis_hash` (the immutable genesis params incl. ASC, k, epoch length) + `protocol_params_hash`
/// (the updatable protocol params), and binds the ASC explicitly so the projection can VERIFY the
/// param it uses. A promoted view is unusable under any other parameter set.
///
/// Encoding (fixed-width, unambiguous, FROZEN): a domain-separation tag, then the 32-byte genesis
/// hash, the 32-byte protocol-params hash, and the ASC as two big-endian `u32` (`numer` then `denom`,
/// 4 + 4 bytes — NOT u64). Every field is constant-length, so no separator/length-prefix is needed and
/// two distinct profiles cannot collide. The domain tag prevents this digest from being confused with
/// any other blake2b commitment in the system. The tag, the field widths, and the order are part of
/// the frozen format: changing any of them is a commitment-format change.
pub fn consensus_profile_commitment(
    genesis_hash: &Hash32,
    protocol_params_hash: &Hash32,
    asc: ActiveSlotsCoeff,
) -> Hash32 {
    const DOMAIN: &[u8] = b"ADE-ECA0b-consensus-profile-v1";
    let mut buf = Vec::with_capacity(DOMAIN.len() + 72);
    buf.extend_from_slice(DOMAIN);
    buf.extend_from_slice(&genesis_hash.0);
    buf.extend_from_slice(&protocol_params_hash.0);
    buf.extend_from_slice(&asc.numer.to_be_bytes());
    buf.extend_from_slice(&asc.denom.to_be_bytes());
    blake2b_256(&buf)
}

/// Why a promoted view could not be projected into a leadership `PoolDistrView` (DC-EPOCH-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    /// The supplied consensus profile (genesis / protocol-params / ASC) does not match the view's
    /// bound `protocol_params_commitment` — an unbound or wrong parameter set.
    ParamsCommitmentMismatch,
    /// The view is not leadership-complete (a staked pool is missing its VRF keyhash).
    NotLeadershipComplete,
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
    fn test_asc() -> ActiveSlotsCoeff {
        ActiveSlotsCoeff { numer: 1, denom: 20 }
    }
    fn test_gen() -> Hash32 {
        Hash32([0x91; 32])
    }
    fn test_pp() -> Hash32 {
        Hash32([0x92; 32])
    }
    fn sample_vrf() -> BTreeMap<PoolId, Hash32> {
        [(pool(1), Hash32([0x71; 32])), (pool(2), Hash32([0x72; 32]))].into_iter().collect()
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
            sample_vrf(),
            Coin(300),
            consensus_profile_commitment(&test_gen(), &test_pp(), test_asc()),
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
            protocol_params_commitment: v.protocol_params_commitment.clone(),
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

    // The canonical hash is binding-sensitive: changing ANY binding, the stake distribution, the
    // VRF mapping, or the protocol-params commitment changes the identity.
    #[test]
    fn canonical_hash_is_binding_sensitive() {
        let base = sample_view().canonical_hash();
        let pp = consensus_profile_commitment(&test_gen(), &test_pp(), test_asc());
        let diff_network = EpochConsensusView::bind(
            3, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Set,
            [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(), sample_vrf(), Coin(300), pp.clone(),
        ).canonical_hash();
        assert_ne!(base, diff_network, "network changes the hash");
        let diff_stake = EpochConsensusView::bind(
            2, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Set,
            [(pool(1), Coin(101)), (pool(2), Coin(200))].into_iter().collect(), sample_vrf(), Coin(301), pp.clone(),
        ).canonical_hash();
        assert_ne!(base, diff_stake, "a stake change changes the hash");
        let diff_vrf = EpochConsensusView::bind(
            2, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Set,
            [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(),
            [(pool(1), Hash32([0x99; 32])), (pool(2), Hash32([0x72; 32]))].into_iter().collect(), Coin(300), pp.clone(),
        ).canonical_hash();
        assert_ne!(base, diff_vrf, "a VRF keyhash change changes the hash");
        let diff_params = EpochConsensusView::bind(
            2, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Set,
            [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(), sample_vrf(), Coin(300), Hash32([0xab; 32]),
        ).canonical_hash();
        assert_ne!(base, diff_params, "the protocol-params commitment changes the hash");
    }

    // DC-EVIEW-12: a view is leadership-complete (key sets equal) and matches requires it + the
    // protocol-params commitment; an incomplete or wrong-params view is INERT.
    #[test]
    fn leadership_complete_required_for_matches() {
        let v = sample_view();
        assert!(v.is_leadership_complete());
        assert!(v.matches(&bindings_of(&v)));
        let mut wrong = bindings_of(&v);
        wrong.protocol_params_commitment = Hash32([0x00; 32]);
        assert!(!v.matches(&wrong), "a wrong protocol-params commitment makes the view inert");
        let incomplete = EpochConsensusView::bind(
            2, CardanoEra::Conway, EpochNo(1334), point(115_000_000, 0xaa),
            Hash32([0xc1; 32]), Hash32([0xe7; 32]), SnapshotPhase::Set,
            [(pool(1), Coin(100)), (pool(2), Coin(200))].into_iter().collect(),
            [(pool(1), Hash32([0x71; 32]))].into_iter().collect(), // pool(2) has stake but no VRF
            Coin(300),
            consensus_profile_commitment(&test_gen(), &test_pp(), test_asc()),
        );
        assert!(!incomplete.is_leadership_complete());
        assert!(!incomplete.matches(&bindings_of(&incomplete)), "an incomplete view is inert");
    }

    // DC-EPOCH-12: the projection builds a complete PoolDistrView from the view + the bound profile,
    // and fails closed under an unbound/wrong consensus profile (no unbound protocol-param read).
    #[test]
    fn to_pool_distr_view_builds_from_bound_profile_and_rejects_wrong_params() {
        use ade_core::consensus::ledger_view::LedgerView;
        let v = sample_view();
        let pdv = v
            .to_pool_distr_view(&test_gen(), &test_pp(), test_asc())
            .expect("projects from the bound profile");
        assert_eq!(pdv.total_active_stake(EpochNo(1334)), Some(300));
        assert_eq!(pdv.pool_active_stake(EpochNo(1334), &Hash28([1; 28])), Some(100));
        assert_eq!(pdv.pool_vrf_keyhash(EpochNo(1334), &Hash28([1; 28])), Some(Hash32([0x71; 32])));
        let wrong_asc = ActiveSlotsCoeff { numer: 1, denom: 21 };
        assert_eq!(
            v.to_pool_distr_view(&test_gen(), &test_pp(), wrong_asc),
            Err(ProjectionError::ParamsCommitmentMismatch)
        );
        assert_eq!(
            v.to_pool_distr_view(&Hash32([0x00; 32]), &test_pp(), test_asc()),
            Err(ProjectionError::ParamsCommitmentMismatch)
        );
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
