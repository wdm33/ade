// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use crate::tx::{Coin, PoolId};
use crate::{EpochNo, Hash28, Hash32};

/// Shelley-era delegation and governance certificates.
///
/// Each variant corresponds to a distinct CBOR-tagged certificate
/// within a Shelley (or later) transaction body's certificate array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Certificate {
    /// Register a staking credential, locking a key deposit.
    StakeRegistration(StakeCredential),
    /// Deregister a staking credential, reclaiming the key deposit.
    StakeDeregistration(StakeCredential),
    /// Delegate stake from a credential to a pool.
    StakeDelegation {
        credential: StakeCredential,
        pool_id: PoolId,
    },
    /// Register a new stake pool (or update an existing registration).
    PoolRegistration(PoolRegistrationCert),
    /// Announce that a pool will retire at the end of a given epoch.
    PoolRetirement {
        pool_id: PoolId,
        epoch: EpochNo,
    },
    /// Genesis key delegation (governance-only, pre-Conway).
    GenesisKeyDelegation {
        genesis_hash: Hash28,
        delegate_hash: Hash28,
        vrf_hash: Hash32,
    },
    /// Move instantaneous rewards from reserves or treasury.
    MIRTransfer(MIRCert),
}

/// Staking credential — a 28-byte hash identifying a stake key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StakeCredential(pub Hash28);

/// Full pool registration certificate parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRegistrationCert {
    /// Pool operator identifier.
    pub pool_id: PoolId,
    /// VRF verification key hash.
    pub vrf_hash: Hash32,
    /// Pledge amount in lovelace.
    pub pledge: Coin,
    /// Fixed operational cost per epoch in lovelace.
    pub cost: Coin,
    /// Pool margin as a rational (numerator, denominator).
    pub margin: (u64, u64),
    /// Reward account address (raw bytes).
    pub reward_account: Vec<u8>,
}

/// Move Instantaneous Rewards certificate — transfers from a pot to stake credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MIRCert {
    /// Source pot for the transfer.
    pub pot: MIRPot,
    /// Per-credential reward amounts.
    pub rewards: BTreeMap<StakeCredential, Coin>,
}

/// Source pot for MIR transfers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MIRPot {
    /// Transfer from the reserves.
    Reserves,
    /// Transfer from the treasury.
    Treasury,
}
