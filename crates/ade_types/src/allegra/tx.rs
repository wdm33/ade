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
use crate::shelley::tx::ShelleyTxOut;

/// Allegra transaction body — extends Shelley with validity_interval_start.
///
/// CBOR map with keys 0–8. Key 8 is validity_interval_start (optional).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllegraTxBody {
    /// Set of transaction inputs (map key 0).
    pub inputs: BTreeSet<TxIn>,
    /// Transaction outputs (map key 1).
    pub outputs: Vec<ShelleyTxOut>,
    /// Transaction fee in lovelace (map key 2).
    pub fee: Coin,
    /// Time-to-live slot number (map key 3). Optional in Allegra+.
    pub ttl: Option<SlotNo>,
    /// Certificates — opaque CBOR (map key 4).
    pub certs: Option<Vec<u8>>,
    /// Withdrawals — opaque CBOR (map key 5).
    pub withdrawals: Option<Vec<u8>>,
    /// Protocol parameter update — opaque CBOR (map key 6).
    pub update: Option<Vec<u8>>,
    /// Auxiliary data hash (map key 7).
    pub metadata_hash: Option<Hash32>,
    /// Validity interval start — earliest slot this tx is valid (map key 8).
    pub validity_interval_start: Option<SlotNo>,
}
