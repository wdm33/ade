// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Genesis UTxO loader — parses ExtLedgerState binary dumps from
//! cardano-node to extract the initial UTxO set.

use std::collections::BTreeMap;
use std::path::Path;

use ade_codec::cbor::{
    read_array_header, read_bytes, read_map_header, read_uint, skip_item, ContainerEncoding,
};
use ade_ledger::utxo::{TxOut, UTxOState};
use ade_types::address::Address;
use ade_types::tx::{Coin, TxIn};
use ade_types::Hash32;

use super::HarnessError;

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

/// Load genesis UTxO from a raw ExtLedgerState binary dump.
///
/// Parses the CBOR structure to extract the UTxO map, converting
/// Haskell's decomposed TxId (4 x u64) back to a 32-byte Hash32.
///
/// The CBOR layout is:
/// ```text
/// array(2) [LedgerState, HeaderState]
///   LedgerState = array(1) [Current]
///     Current = array(2) [Bound, ByronLedgerState]
///       Bound = array(3) [epoch, slot, time]
///       ByronLedgerState = array(3) [tipBlockNo, ChainState, Transition]
///         ChainState = array(5) [version, slot_info, UTxO_map, update_state, delegation]
/// ```
pub fn load_genesis_utxo(dump_path: &Path) -> Result<UTxOState, HarnessError> {
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

    // LedgerState: array(1) [Current]
    let ls_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "LedgerState",
    )?;
    if ls_len != 1 {
        return Err(HarnessError::DecodingError(format!(
            "expected LedgerState array(1), got array({ls_len})"
        )));
    }

    // Current: array(2) [Bound, ByronLedgerState]
    let cur_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "Current",
    )?;
    if cur_len != 2 {
        return Err(HarnessError::DecodingError(format!(
            "expected Current array(2), got array({cur_len})"
        )));
    }

    // Skip Bound: array(3) [epoch, slot, time]
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // ByronLedgerState: array(3) [tipBlockNo, ChainState, Transition]
    let bls_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "ByronLedgerState",
    )?;
    if bls_len != 3 {
        return Err(HarnessError::DecodingError(format!(
            "expected ByronLedgerState array(3), got array({bls_len})"
        )));
    }

    // Skip tipBlockNo (WithOrigin encoding)
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // ChainState: array(5) [version, slot_info, UTxO_map, update_state, delegation]
    let cs_len = container_count(
        read_array_header(&data, &mut offset).map_err(cbor_err)?,
        "ChainState",
    )?;
    if cs_len != 5 {
        return Err(HarnessError::DecodingError(format!(
            "expected ChainState array(5), got array({cs_len})"
        )));
    }

    // Skip version
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // Skip slot_info
    skip_item(&data, &mut offset).map_err(cbor_err)?;

    // UTxO map: map(N)
    let map_len = container_count(
        read_map_header(&data, &mut offset).map_err(cbor_err)?,
        "UTxO map",
    )?;

    let mut utxos = BTreeMap::new();

    for _ in 0..map_len {
        // Key: array(2) [array(4) [u64, u64, u64, u64], uint_index]
        let key_len = container_count(
            read_array_header(&data, &mut offset).map_err(cbor_err)?,
            "UTxO key",
        )?;
        if key_len != 2 {
            return Err(HarnessError::DecodingError(format!(
                "expected UTxO key array(2), got array({key_len})"
            )));
        }

        // TxId: array(4) of u64
        let txid_len = container_count(
            read_array_header(&data, &mut offset).map_err(cbor_err)?,
            "TxId array",
        )?;
        if txid_len != 4 {
            return Err(HarnessError::DecodingError(format!(
                "expected TxId array(4), got array({txid_len})"
            )));
        }

        let mut hash_bytes = [0u8; 32];
        for chunk in 0..4 {
            let (val, _) = read_uint(&data, &mut offset).map_err(cbor_err)?;
            hash_bytes[chunk * 8..(chunk + 1) * 8].copy_from_slice(&val.to_be_bytes());
        }

        // Output index
        let (index_val, _) = read_uint(&data, &mut offset).map_err(cbor_err)?;
        let index = u16::try_from(index_val).map_err(|_| {
            HarnessError::DecodingError(format!("output index {index_val} exceeds u16::MAX"))
        })?;

        let tx_in = TxIn {
            tx_hash: Hash32(hash_bytes),
            index,
        };

        // Value: array(2) [bytes(address), uint(coin)]
        let val_len = container_count(
            read_array_header(&data, &mut offset).map_err(cbor_err)?,
            "UTxO value",
        )?;
        if val_len != 2 {
            return Err(HarnessError::DecodingError(format!(
                "expected UTxO value array(2), got array({val_len})"
            )));
        }

        let (addr_bytes, _) = read_bytes(&data, &mut offset).map_err(cbor_err)?;
        let (coin_val, _) = read_uint(&data, &mut offset).map_err(cbor_err)?;

        let tx_out = TxOut::Byron {
            address: Address::Byron(addr_bytes),
            coin: Coin(coin_val),
        };

        utxos.insert(tx_in, tx_out);
    }

    Ok(UTxOState { utxos })
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
            .join("byron")
            .join("slot_0.bin")
    }

    #[test]
    fn loads_14505_genesis_utxos() {
        let path = dump_path();
        if !path.exists() {
            eprintln!("skipping: genesis dump not found at {}", path.display());
            return;
        }
        let state = load_genesis_utxo(&path).unwrap();
        assert_eq!(state.len(), 14_505, "expected 14,505 genesis UTxO entries");
    }

    #[test]
    fn all_entries_are_byron_addresses() {
        let path = dump_path();
        if !path.exists() {
            return;
        }
        let state = load_genesis_utxo(&path).unwrap();
        for tx_out in state.utxos.values() {
            match tx_out {
                TxOut::Byron { address, coin } => {
                    assert!(matches!(address, Address::Byron(_)));
                    assert!(coin.0 > 0, "genesis UTxO should have positive coin");
                }
                _ => panic!("expected Byron TxOut in genesis UTxO"),
            }
        }
    }
}
