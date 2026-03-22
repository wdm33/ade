// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use ade_codec::allegra::script::decode_native_scripts;
use ade_codec::cbor::{self, ContainerEncoding};
use ade_types::allegra::script::NativeScript;
use ade_types::Hash28;

use crate::error::LedgerError;

/// Skip an optional CBOR tag (e.g. tag 258 for sets in Conway).
fn skip_optional_tag(data: &[u8], offset: &mut usize) -> Result<(), LedgerError> {
    if *offset < data.len() {
        let major = (data[*offset] >> 5) & 0x7;
        if major == 6 {
            // Tag — read and discard
            let _ = cbor::read_tag(data, offset)?;
        }
    }
    Ok(())
}

/// Structured classification of what a witness set contains.
///
/// Deterministic: same CBOR input always produces the same classification.
/// This is the authoritative witness surface for ScriptVerdict determination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessInfo {
    /// Available verification key hashes (from VKey witnesses).
    pub available_key_hashes: BTreeSet<Hash28>,
    /// Parsed native scripts (key 1 in witness set map).
    pub native_scripts: Vec<NativeScript>,
    /// Whether Plutus V1 scripts are present (key 3).
    pub has_plutus_v1: bool,
    /// Whether Plutus V2 scripts are present (key 6).
    pub has_plutus_v2: bool,
    /// Whether Plutus V3 scripts are present (key 7).
    pub has_plutus_v3: bool,
}

impl WitnessInfo {
    /// True if any Plutus script is present in the witness set.
    pub fn has_plutus(&self) -> bool {
        self.has_plutus_v1 || self.has_plutus_v2 || self.has_plutus_v3
    }
}

/// Decode all witness sets from a block's witness_sets CBOR array.
///
/// Returns one WitnessInfo per transaction, in the same order as tx_bodies.
pub fn decode_witness_infos(
    witness_sets_cbor: &[u8],
) -> Result<Vec<WitnessInfo>, LedgerError> {
    let mut offset = 0;
    let enc = cbor::read_array_header(witness_sets_cbor, &mut offset)?;

    let mut results = Vec::new();
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                results.push(decode_single_witness_info(witness_sets_cbor, &mut offset)?);
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(witness_sets_cbor, offset)? {
                results.push(decode_single_witness_info(witness_sets_cbor, &mut offset)?);
            }
        }
    }

    Ok(results)
}

/// Decode a single witness set map into WitnessInfo.
///
/// Witness set CBOR map keys:
/// - 0: vkeywitnesses (array of [vkey, sig])
/// - 1: native_scripts (array of NativeScript)
/// - 2: bootstrap_witnesses (opaque, skipped)
/// - 3: plutus_v1_scripts (presence detected)
/// - 4: plutus_data / datums (opaque, skipped)
/// - 5: redeemers (opaque, skipped)
/// - 6: plutus_v2_scripts (presence detected, Babbage+)
/// - 7: plutus_v3_scripts (presence detected, Conway+)
fn decode_single_witness_info(
    data: &[u8],
    offset: &mut usize,
) -> Result<WitnessInfo, LedgerError> {
    let enc = cbor::read_map_header(data, offset)?;

    let mut info = WitnessInfo {
        available_key_hashes: BTreeSet::new(),
        native_scripts: Vec::new(),
        has_plutus_v1: false,
        has_plutus_v2: false,
        has_plutus_v3: false,
    };

    let process_key = |data: &[u8],
                       offset: &mut usize,
                       info: &mut WitnessInfo|
     -> Result<(), LedgerError> {
        let (key, _) = cbor::read_uint(data, offset)?;
        match key {
            0 => {
                // VKey witnesses: extract key hashes (may be tag(258)-wrapped)
                skip_optional_tag(data, offset)?;
                info.available_key_hashes = decode_vkey_hashes(data, offset)?;
            }
            1 => {
                // Native scripts: parse for evaluation (may be tag(258)-wrapped)
                skip_optional_tag(data, offset)?;
                info.native_scripts = decode_native_scripts(data, offset)?;
            }
            3 => {
                // Plutus V1 scripts: detect presence, skip content
                info.has_plutus_v1 = true;
                let _ = cbor::skip_item(data, offset)?;
            }
            6 => {
                // Plutus V2 scripts (Babbage+): detect presence, skip
                info.has_plutus_v2 = true;
                let _ = cbor::skip_item(data, offset)?;
            }
            7 => {
                // Plutus V3 scripts (Conway+): detect presence, skip
                info.has_plutus_v3 = true;
                let _ = cbor::skip_item(data, offset)?;
            }
            _ => {
                // Keys 2 (bootstrap), 4 (datums), 5 (redeemers), others: skip
                let _ = cbor::skip_item(data, offset)?;
            }
        }
        Ok(())
    };

    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_key(data, offset, &mut info)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)? {
                process_key(data, offset, &mut info)?;
            }
            *offset += 1;
        }
    }

    Ok(info)
}

/// Extract key hashes from VKey witnesses.
///
/// Each VKey witness is `array(2) [vkey_bytes(32), sig_bytes(64)]`.
/// The key hash is `Blake2b-224(vkey)`.
fn decode_vkey_hashes(
    data: &[u8],
    offset: &mut usize,
) -> Result<BTreeSet<Hash28>, LedgerError> {
    let enc = cbor::read_array_header(data, offset)?;
    let mut hashes = BTreeSet::new();

    let mut process_one =
        |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
            // array(2) [vkey, sig]
            let inner = cbor::read_array_header(data, offset)?;
            match inner {
                ContainerEncoding::Definite(2, _) => {}
                _ => {
                    return Err(LedgerError::Decoding(crate::error::DecodingError {
                        offset: *offset,
                        reason: crate::error::DecodingFailureReason::InvalidStructure,
                    }));
                }
            }
            let (vkey_bytes, _) = cbor::read_bytes(data, offset)?;
            // Skip signature
            let _ = cbor::skip_item(data, offset)?;

            // Key hash = Blake2b-224(vkey)
            let key_hash = ade_crypto::blake2b_224(&vkey_bytes);
            hashes.insert(key_hash);
            Ok(())
        };

    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_one(data, offset)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)? {
                process_one(data, offset)?;
            }
            *offset += 1;
        }
    }

    Ok(hashes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_witness_set() {
        // map(0)
        let data = vec![0xa0];
        let mut offset = 0;
        let info = decode_single_witness_info(&data, &mut offset).unwrap();
        assert!(info.available_key_hashes.is_empty());
        assert!(info.native_scripts.is_empty());
        assert!(!info.has_plutus());
    }

    #[test]
    fn witness_info_plutus_detection() {
        let info = WitnessInfo {
            available_key_hashes: BTreeSet::new(),
            native_scripts: Vec::new(),
            has_plutus_v1: true,
            has_plutus_v2: false,
            has_plutus_v3: false,
        };
        assert!(info.has_plutus());
    }

    #[test]
    fn witness_info_no_plutus() {
        let info = WitnessInfo {
            available_key_hashes: BTreeSet::new(),
            native_scripts: Vec::new(),
            has_plutus_v1: false,
            has_plutus_v2: false,
            has_plutus_v3: false,
        };
        assert!(!info.has_plutus());
    }
}
