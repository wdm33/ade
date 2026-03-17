// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;
use crate::tx::{Coin, TxIn};
use crate::Hash32;
use crate::SlotNo;

/// Shelley transaction body — decoded from CBOR map with keys 0–7.
///
/// Opaque substructures (certs, withdrawals, update) are preserved as raw CBOR
/// for later slices to decode when their invariant surface is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyTxBody {
    /// Set of transaction inputs (map key 0).
    pub inputs: BTreeSet<TxIn>,
    /// Transaction outputs (map key 1).
    pub outputs: Vec<ShelleyTxOut>,
    /// Transaction fee in lovelace (map key 2).
    pub fee: Coin,
    /// Time-to-live slot number (map key 3).
    pub ttl: SlotNo,
    /// Certificates — opaque CBOR, decoded in S-11 (map key 4).
    pub certs: Option<Vec<u8>>,
    /// Withdrawals — opaque CBOR, decoded in S-11 (map key 5).
    pub withdrawals: Option<Vec<u8>>,
    /// Protocol parameter update proposal — opaque CBOR, decoded in S-16 (map key 6).
    pub update: Option<Vec<u8>>,
    /// Auxiliary data hash (map key 7).
    pub metadata_hash: Option<Hash32>,
}

/// Shelley transaction output — address + coin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyTxOut {
    /// Raw address bytes (preserved for wire-byte fidelity).
    pub address: Vec<u8>,
    /// Output value in lovelace.
    pub coin: Coin,
}
