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

/// Mary transaction output — address + value (coin or coin + multi-asset).
///
/// Wire format: `[address, value]` where value is either `uint` or `[uint, multiasset_map]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaryTxOut {
    /// Raw address bytes (preserved for wire-byte fidelity).
    pub address: Vec<u8>,
    /// Coin amount in lovelace.
    pub coin: Coin,
    /// Multi-asset bundle — opaque CBOR, decoded in S-13.
    /// None if the output is pure lovelace.
    pub multi_asset: Option<Vec<u8>>,
}

/// Mary transaction body — extends Allegra with mint field.
///
/// CBOR map with keys 0–9. Key 9 is mint (optional, decoded in S-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaryTxBody {
    /// Set of transaction inputs (map key 0).
    pub inputs: BTreeSet<TxIn>,
    /// Transaction outputs (map key 1).
    pub outputs: Vec<MaryTxOut>,
    /// Transaction fee in lovelace (map key 2).
    pub fee: Coin,
    /// Time-to-live slot number (map key 3). Optional in Mary.
    pub ttl: Option<SlotNo>,
    /// Certificates — opaque CBOR (map key 4).
    pub certs: Option<Vec<u8>>,
    /// Withdrawals — opaque CBOR (map key 5).
    pub withdrawals: Option<Vec<u8>>,
    /// Protocol parameter update — opaque CBOR (map key 6).
    pub update: Option<Vec<u8>>,
    /// Auxiliary data hash (map key 7).
    pub metadata_hash: Option<Hash32>,
    /// Validity interval start (map key 8).
    pub validity_interval_start: Option<SlotNo>,
    /// Minting field — opaque CBOR, decoded in S-13 (map key 9).
    pub mint: Option<Vec<u8>>,
}
