// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Shelley UTxO oracle loader — parses ExtLedgerState binary dumps from
//! cardano-node to extract the Shelley-era UTxO set.
//!
//! The Shelley ExtLedgerState uses compact 28-byte TxId hashes (Blake2b-224
//! of the full 32-byte TxId), so we cannot reconstruct `ade_types::TxIn`.
//! Instead we return Shelley-specific oracle types.

use std::collections::BTreeMap;
use std::path::Path;

use ade_codec::cbor::{
    is_break, read_array_header, read_bytes, read_map_header, read_uint, skip_item,
    ContainerEncoding,
};

use super::HarnessError;

// ---------------------------------------------------------------------------
// Shelley oracle types
// ---------------------------------------------------------------------------

/// Compact TxIn used in Shelley ExtLedgerState dumps.
///
/// Shelley stores TxId as a 28-byte Blake2b-224 hash (not the full 32-byte
/// hash), so these cannot be directly compared with on-chain TxIds.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShelleyCompactTxIn {
    /// Blake2b-224 of the full 32-byte TxId.
    pub tx_id_hash28: [u8; 28],
    /// Output index within the transaction.
    pub output_index: u16,
}

/// A single UTxO entry from the Shelley oracle dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyOracleUtxoEntry {
    /// Raw CBOR bytes of the compact address structure.
    pub address_bytes: Vec<u8>,
    /// Lovelace value.
    pub coin: u64,
    /// Slot in which this UTxO was deposited (0 for genesis outputs).
    pub slot_deposited: u64,
}

/// The full Shelley oracle UTxO set extracted from an ExtLedgerState dump.
#[derive(Debug, Clone)]
pub struct ShelleyOracleUtxo {
    /// All UTxO entries keyed by compact TxIn.
    pub entries: BTreeMap<ShelleyCompactTxIn, ShelleyOracleUtxoEntry>,
}

impl ShelleyOracleUtxo {
    /// Number of UTxO entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the UTxO set is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sum of all lovelace across all entries.
    pub fn total_lovelace(&self) -> u128 {
        self.entries
            .values()
            .map(|e| u128::from(e.coin))
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

/// Extract the definite element count from a ContainerEncoding,
/// or return a decoding error for indefinite containers.
fn container_count(enc: ContainerEncoding, label: &str) -> Result<u64, HarnessError> {
    match enc {
        ContainerEncoding::Definite(n, _) => Ok(n),
        ContainerEncoding::Indefinite => Err(HarnessError::DecodingError(format!(
            "expected definite-length {label}, got indefinite"
        ))),
    }
}

/// Load the Shelley-era UTxO set from a raw ExtLedgerState binary dump.
///
/// The CBOR layout navigated is:
/// ```text
/// array(2) [LedgerState, HeaderState]
///   LedgerState = Telescope array(2) [Past(Byron), Current(Shelley)]
///     Past(Byron) = array(2) [Bound_start, Bound_end]
///     Current(Shelley) = array(2) [Bound, ShelleyLedgerState]
///       ShelleyLedgerState = array(2) [version=2, array(3)]
///         array(3) [tip, NewEpochState, transition]
///           NewEpochState = array(7) [epochNo, prevBlocks, currBlocks, EpochState, ...]
///             EpochState = array(4) [acct, LedgerStatePair, snapshots, nonMyopic]
///               LedgerStatePair = array(2) [UTxOState, DelegationState]
///                 UTxOState = array(2) [dpstate, utxo_inner]
///                   utxo_inner = array(4) [utxo_container, deposits, fees, ppups]
///                     utxo_container = array(2) [utxo_map(*), ...]
///                       utxo_map = indef map { array(2)[uint,bytes(28)] => array(4)[...] }
/// ```
pub fn load_shelley_oracle_utxo(dump_path: &Path) -> Result<ShelleyOracleUtxo, HarnessError> {
    let data = std::fs::read(dump_path).map_err(|e| {
        HarnessError::IoError(format!("failed to read {}: {e}", dump_path.display()))
    })?;

    let mut offset = 0;

    // Top level: array(2) [LedgerState, HeaderState]
    let top_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "top-level array",
    )?;
    if top_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected top-level array(2), got array({top_len})"
        )));
    }

    // Telescope: array(2) [Past(Byron), Current(Shelley)]
    let tele_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "telescope",
    )?;
    if tele_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected telescope array(2), got array({tele_len})"
        )));
    }

    // Skip Past(Byron) entirely
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // Current(Shelley): array(2) [Bound, ShelleyLedgerState]
    let cur_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "Current(Shelley)",
    )?;
    if cur_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected Current array(2), got array({cur_len})"
        )));
    }

    // Skip Bound
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // ShelleyLedgerState: array(2) [version, payload]
    let sls_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "ShelleyLedgerState",
    )?;
    if sls_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected ShelleyLedgerState array(2), got array({sls_len})"
        )));
    }

    // version
    let (version, _) = read_uint(&data, &mut offset).map_err(cbor_err)?;
    if version != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected ShelleyLedgerState version 2, got {version}"
        )));
    }

    // payload: array(3) [tip, NewEpochState, transition]
    let pl_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "payload",
    )?;
    if pl_len != 3 {
        return Err(HarnessError::DecodingError(format!(
            "expected payload array(3), got array({pl_len})"
        )));
    }

    // Skip tip
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // NewEpochState: array(7)
    let nes_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "NewEpochState",
    )?;
    if nes_len != 7 {
        return Err(HarnessError::DecodingError(format!(
            "expected NewEpochState array(7), got array({nes_len})"
        )));
    }

    // [0] epochNo
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // [1] prevBlocks (indef map)
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // [2] currBlocks (indef map)
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // [3] EpochState: array(4) [acct, LedgerStatePair, snapshots, nonMyopic]
    let es_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "EpochState",
    )?;
    if es_len != 4 {
        return Err(HarnessError::DecodingError(format!(
            "expected EpochState array(4), got array({es_len})"
        )));
    }

    // [0] acct (treasury/reserves)
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // [1] LedgerStatePair: array(2) [UTxOState, DelegationState]
    let lsp_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "LedgerStatePair",
    )?;
    if lsp_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected LedgerStatePair array(2), got array({lsp_len})"
        )));
    }

    // [0] UTxOState: array(2) [dpstate, utxo_inner]
    let ust_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "UTxOState",
    )?;
    if ust_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected UTxOState array(2), got array({ust_len})"
        )));
    }

    // UTxOState[0] = dpstate: array(4) — skip delegation/pool/stake data
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // UTxOState[1] = array(4) [utxo_container, deposits, fees, ppups]
    let ui_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "utxo_inner",
    )?;
    if ui_len != 4 {
        return Err(HarnessError::DecodingError(format!(
            "expected utxo_inner array(4), got array({ui_len})"
        )));
    }

    // utxo_inner[0] = utxo_container: array(2) [utxo_map, ...]
    let uc_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "utxo_container",
    )?;
    if uc_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected utxo_container array(2), got array({uc_len})"
        )));
    }

    // The UTxO map — indefinite-length map
    let map_enc = read_map_header(&data, &mut offset).map_err(cbor_err)?;

    let mut entries = BTreeMap::new();

    match map_enc {
        ContainerEncoding::Indefinite => {
            // Read entries until break byte (0xff)
            while !is_break(&data, offset).map_err(cbor_err)? {
                let (key, value) = read_utxo_entry(&data, &mut offset)?;
                entries.insert(key, value);
            }
            // Break byte is verified but not consumed — offset is not used further.
        }
        ContainerEncoding::Definite(count, _) => {
            for _ in 0..count {
                let (key, value) = read_utxo_entry(&data, &mut offset)?;
                entries.insert(key, value);
            }
        }
    }

    Ok(ShelleyOracleUtxo { entries })
}

/// Read a single UTxO map entry (key + value).
///
/// Key format: `array(2) [uint(output_index), bytes(28)]`
/// Value format: `array(4) [address_cbor, uint(slot_deposited), uint(coin), bytes(28)|null]`
fn read_utxo_entry(
    data: &[u8],
    offset: &mut usize,
) -> Result<(ShelleyCompactTxIn, ShelleyOracleUtxoEntry), HarnessError> {
    // --- Key ---
    let key_len = container_count(
        read_array_header(data, offset).map_err(cbor_err)?,
        "UTxO key",
    )?;
    if key_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected UTxO key array(2), got array({key_len}) at offset {}",
            *offset
        )));
    }

    // output_index
    let (index_val, _) = read_uint(data, offset).map_err(cbor_err)?;
    let output_index = u16::try_from(index_val).map_err(|_| {
        HarnessError::DecodingError(format!("output index {index_val} exceeds u16::MAX"))
    })?;

    // tx_id_hash28: bytes(28)
    let (hash_bytes, _) = read_bytes(data, offset).map_err(cbor_err)?;
    if hash_bytes.len() != 28 {
        return Err(HarnessError::DecodingError(format!(
            "expected 28-byte TxId hash, got {} bytes",
            hash_bytes.len()
        )));
    }
    let mut tx_id_hash28 = [0u8; 28];
    tx_id_hash28.copy_from_slice(&hash_bytes);

    let key = ShelleyCompactTxIn {
        tx_id_hash28,
        output_index,
    };

    // --- Value ---
    let val_len = container_count(
        read_array_header(data, offset).map_err(cbor_err)?,
        "UTxO value",
    )?;
    if val_len != 4 {
        return Err(HarnessError::DecodingError(format!(
            "expected UTxO value array(4), got array({val_len}) at offset {}",
            *offset
        )));
    }

    // [0] address — capture as raw CBOR bytes via skip_item
    let (addr_start, addr_end) = skip_item(data, offset).map_err(cbor_err)?;
    let address_bytes = data[addr_start..addr_end].to_vec();

    // [1] slot_deposited
    let (slot_deposited, _) = read_uint(data, offset).map_err(cbor_err)?;

    // [2] coin (lovelace)
    let (coin, _) = read_uint(data, offset).map_err(cbor_err)?;

    // [3] optional 28-byte hash (or CBOR null)
    // We skip this field — it's a stake credential hash that we don't need yet.
    skip_item(data, offset).map_err(cbor_err)?;

    let value = ShelleyOracleUtxoEntry {
        address_bytes,
        coin,
        slot_deposited,
    };

    Ok((key, value))
}

fn cbor_err(e: ade_codec::CodecError) -> HarnessError {
    HarnessError::DecodingError(format!("{e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dump_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("corpus")
            .join("ext_ledger_state_dumps")
            .join("shelley")
            .join("slot_10800019.bin")
    }

    #[test]
    fn loads_84609_shelley_utxos() {
        let path = dump_path();
        if !path.exists() {
            eprintln!("skipping: Shelley dump not found at {}", path.display());
            return;
        }
        let utxo = load_shelley_oracle_utxo(&path).unwrap();
        assert_eq!(utxo.len(), 84_609, "expected 84,609 Shelley UTxO entries");
    }

    #[test]
    fn all_entries_have_nonzero_coin() {
        let path = dump_path();
        if !path.exists() {
            return;
        }
        let utxo = load_shelley_oracle_utxo(&path).unwrap();
        for (key, entry) in &utxo.entries {
            assert!(
                entry.coin > 0,
                "UTxO entry {:?} has zero coin",
                key
            );
        }
    }
}
