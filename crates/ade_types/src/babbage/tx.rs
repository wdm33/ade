// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::tx::{Coin, TxIn};
use crate::{Hash28, Hash32, SlotNo};

/// Babbage transaction output.
///
/// Babbage outputs can be array format `[address, value, datum_option, script_ref]`
/// or map format `{0: address, 1: value, 2: datum_option, 3: script_ref}`.
/// Datum can be inline datum or datum hash. Script ref is optional reference script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BabbageTxOut {
    pub address: Vec<u8>,
    pub coin: Coin,
    pub multi_asset: Option<Vec<u8>>,
    /// Opaque datum option — either [0, datum_hash] or [1, inline_datum].
    pub datum_option: Option<Vec<u8>>,
    /// Opaque reference script.
    pub script_ref: Option<Vec<u8>>,
}

/// Babbage transaction body — extends Alonzo with reference inputs, collateral return.
///
/// New keys (Babbage adds 16, 17, 18):
/// - 16: collateral return output
/// - 17: total collateral (Coin)
/// - 18: reference inputs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BabbageTxBody {
    pub inputs: BTreeSet<TxIn>,
    pub outputs: Vec<BabbageTxOut>,
    pub fee: Coin,
    pub ttl: Option<SlotNo>,
    pub certs: Option<Vec<u8>>,
    pub withdrawals: Option<Vec<u8>>,
    pub update: Option<Vec<u8>>,
    pub metadata_hash: Option<Hash32>,
    pub validity_interval_start: Option<SlotNo>,
    pub mint: Option<Vec<u8>>,
    // Alonzo fields
    pub script_data_hash: Option<Hash32>,
    pub collateral_inputs: Option<BTreeSet<TxIn>>,
    pub required_signers: Option<BTreeSet<Hash28>>,
    pub network_id: Option<u8>,
    // Babbage additions
    pub collateral_return: Option<BabbageTxOut>,
    pub total_collateral: Option<Coin>,
    pub reference_inputs: Option<BTreeSet<TxIn>>,
}
