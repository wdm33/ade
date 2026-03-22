// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::tx::{Coin, TxIn};
use crate::{Hash28, Hash32, SlotNo};

/// Alonzo transaction output.
///
/// Wire format: `[address, value]` or `[address, value, datum_hash]`
/// where value is either `uint` or `[uint, multiasset_map]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlonzoTxOut {
    pub address: Vec<u8>,
    pub coin: Coin,
    pub multi_asset: Option<Vec<u8>>,
    pub datum_hash: Option<Hash32>,
}

/// Alonzo transaction body — extends Mary with Plutus infrastructure.
///
/// New keys (Alonzo adds 11, 13, 14, 15):
/// - 11: script_data_hash — Blake2b-256 of redeemers/datums/cost-models
/// - 13: collateral inputs — consumed if Plutus script fails
/// - 14: required signers — key hashes that must sign
/// - 15: network ID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlonzoTxBody {
    pub inputs: BTreeSet<TxIn>,
    pub outputs: Vec<AlonzoTxOut>,
    pub fee: Coin,
    pub ttl: Option<SlotNo>,
    pub certs: Option<Vec<u8>>,
    pub withdrawals: Option<Vec<u8>>,
    pub update: Option<Vec<u8>>,
    pub metadata_hash: Option<Hash32>,
    pub validity_interval_start: Option<SlotNo>,
    pub mint: Option<Vec<u8>>,
    // Alonzo additions
    pub script_data_hash: Option<Hash32>,
    pub collateral_inputs: Option<BTreeSet<TxIn>>,
    pub required_signers: Option<BTreeSet<Hash28>>,
    pub network_id: Option<u8>,
}

/// Alonzo full transaction (body + witnesses + validity + auxiliary). Opaque witness/auxiliary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlonzoTx {
    pub raw: Vec<u8>,
}
