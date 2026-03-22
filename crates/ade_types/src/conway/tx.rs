// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use crate::babbage::tx::BabbageTxOut;
use crate::tx::{Coin, TxIn};
use crate::{Hash28, Hash32, SlotNo};

/// Conway transaction body — extends Babbage with governance.
///
/// New keys (Conway adds 19, 20, 21, 22):
/// - 19: voting procedures
/// - 20: proposal procedures
/// - 21: treasury value
/// - 22: donation
///
/// Conway also removes key 6 (update) — governance replaces the old
/// update mechanism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayTxBody {
    pub inputs: BTreeSet<TxIn>,
    pub outputs: Vec<BabbageTxOut>,
    pub fee: Coin,
    pub ttl: Option<SlotNo>,
    pub certs: Option<Vec<u8>>,
    pub withdrawals: Option<Vec<u8>>,
    pub metadata_hash: Option<Hash32>,
    pub validity_interval_start: Option<SlotNo>,
    pub mint: Option<Vec<u8>>,
    // Alonzo fields
    pub script_data_hash: Option<Hash32>,
    pub collateral_inputs: Option<BTreeSet<TxIn>>,
    pub required_signers: Option<BTreeSet<Hash28>>,
    pub network_id: Option<u8>,
    // Babbage fields
    pub collateral_return: Option<BabbageTxOut>,
    pub total_collateral: Option<Coin>,
    pub reference_inputs: Option<BTreeSet<TxIn>>,
    // Conway additions
    pub voting_procedures: Option<Vec<u8>>,
    pub proposal_procedures: Option<Vec<u8>>,
    pub treasury_value: Option<Coin>,
    pub donation: Option<Coin>,
}
