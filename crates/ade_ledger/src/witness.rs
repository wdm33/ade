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

/// Plutus script version, matching witness-set map keys 3/6/7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlutusVersion {
    V1,
    V2,
    V3,
}

/// A single Plutus script extracted from a witness set.
///
/// `flat_bytes` is the inner Flat-encoded UPLC program (the content of
/// the CBOR bytestring stored under the witness-set map). Callers
/// invoke `ade_plutus::PlutusScript::from_flat(&flat_bytes)` on these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlutusScriptEntry {
    pub version: PlutusVersion,
    pub flat_bytes: Vec<u8>,
}

/// Sum `ex_units` across all redeemers in a witness set.
///
/// Handles both wire forms:
///
/// - **Alonzo / Babbage array form**: `[* redeemer]` where each
///   `redeemer = [tag, index, data, ex_units]` and
///   `ex_units = [mem, cpu]`.
///
/// - **Conway map form**: `{(tag, index) => (data, ex_units)}`
///   keyed by a 2-tuple. Introduced in Conway for compactness.
///
/// Detection is by CBOR major type: major 4 (array) = Alonzo form,
/// major 5 (map) = Conway form. Major 6 (tag) can wrap the map
/// (tag 258 set encoding) — stripped transparently.
///
/// Silent-skips the redeemers item and returns `(0, 0)` if the wire
/// form is malformed in a way the two parsers don't recognize.
/// Callers treat `(0, 0)` as "no redeemers" for the budget-cap
/// check; a real tx with scripts would not under-declare to 0 and
/// expect to pass validation.
fn sum_redeemer_ex_units(
    data: &[u8],
    offset: &mut usize,
) -> Result<TotalExUnits, LedgerError> {
    skip_optional_tag(data, offset)?;
    if *offset >= data.len() {
        return Err(LedgerError::Decoding(crate::error::DecodingError {
            offset: *offset,
            reason: crate::error::DecodingFailureReason::InvalidStructure,
        }));
    }
    let major = data[*offset] >> 5;
    match major {
        4 => sum_redeemers_array(data, offset),
        5 => sum_redeemers_map(data, offset),
        _ => {
            // Unknown form — skip, declare zero. Real txs never hit this.
            let _ = cbor::skip_item(data, offset)?;
            Ok(TotalExUnits::default())
        }
    }
}

/// Alonzo/Babbage: `[* [tag, index, data, [mem, cpu]]]`.
fn sum_redeemers_array(
    data: &[u8],
    offset: &mut usize,
) -> Result<TotalExUnits, LedgerError> {
    let enc = cbor::read_array_header(data, offset)?;
    let mut total = TotalExUnits::default();

    let mut process_one =
        |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
            let inner = cbor::read_array_header(data, offset)?;
            let expected_len = 4;
            match inner {
                ContainerEncoding::Definite(n, _) if n == expected_len => {}
                _ => {
                    return Err(LedgerError::Decoding(crate::error::DecodingError {
                        offset: *offset,
                        reason: crate::error::DecodingFailureReason::InvalidStructure,
                    }));
                }
            }
            // Skip tag (uint), index (uint), data (any).
            let _ = cbor::skip_item(data, offset)?;
            let _ = cbor::skip_item(data, offset)?;
            let _ = cbor::skip_item(data, offset)?;
            // ex_units: [mem, cpu]
            let (mem, cpu) = read_ex_units(data, offset)?;
            total.mem = total.mem.saturating_add(mem);
            total.cpu = total.cpu.saturating_add(cpu);
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

    Ok(total)
}

/// Conway: `{[tag, index] => [data, [mem, cpu]]}`.
fn sum_redeemers_map(
    data: &[u8],
    offset: &mut usize,
) -> Result<TotalExUnits, LedgerError> {
    let enc = cbor::read_map_header(data, offset)?;
    let mut total = TotalExUnits::default();

    let mut process_one =
        |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
            // Key: [tag, index] — skip entirely.
            let _ = cbor::skip_item(data, offset)?;
            // Value: [data, [mem, cpu]]
            let val_hdr = cbor::read_array_header(data, offset)?;
            match val_hdr {
                ContainerEncoding::Definite(2, _) => {}
                _ => {
                    return Err(LedgerError::Decoding(crate::error::DecodingError {
                        offset: *offset,
                        reason: crate::error::DecodingFailureReason::InvalidStructure,
                    }));
                }
            }
            // Skip data
            let _ = cbor::skip_item(data, offset)?;
            // Read ex_units
            let (mem, cpu) = read_ex_units(data, offset)?;
            total.mem = total.mem.saturating_add(mem);
            total.cpu = total.cpu.saturating_add(cpu);
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

    Ok(total)
}

/// Read `[mem, cpu]` as `(i64, i64)`. Values are CBOR unsigned ints
/// on wire; we store as `i64` to match Haskell's `ExUnits` (Natural
/// → Int64 at the budget-check layer).
fn read_ex_units(data: &[u8], offset: &mut usize) -> Result<(i64, i64), LedgerError> {
    let hdr = cbor::read_array_header(data, offset)?;
    match hdr {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(LedgerError::Decoding(crate::error::DecodingError {
                offset: *offset,
                reason: crate::error::DecodingFailureReason::InvalidStructure,
            }));
        }
    }
    let (mem, _) = cbor::read_uint(data, offset)?;
    let (cpu, _) = cbor::read_uint(data, offset)?;
    Ok((clamp_u64_to_i64(mem), clamp_u64_to_i64(cpu)))
}

fn clamp_u64_to_i64(v: u64) -> i64 {
    if v > i64::MAX as u64 {
        i64::MAX
    } else {
        v as i64
    }
}

/// Extract the Plutus script bytes from a single witness set's CBOR.
///
/// Complements `decode_single_witness_info`, which only detects
/// presence. This function returns the raw Flat-encoded UPLC for each
/// script found under witness-set map keys 3 (V1), 6 (V2), 7 (V3).
///
/// Used by the S-29 Flat decoder probe
/// (docs/active/S-29_flat_decoder_probe.md) and by future slices that
/// actually evaluate scripts.
pub fn decode_plutus_scripts_in_witness_set(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<PlutusScriptEntry>, LedgerError> {
    let enc = cbor::read_map_header(data, offset)?;
    let mut out = Vec::new();

    let mut process_key =
        |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
            let (key, _) = cbor::read_uint(data, offset)?;
            match key {
                3 => out.extend(decode_plutus_script_array(data, offset, PlutusVersion::V1)?),
                6 => out.extend(decode_plutus_script_array(data, offset, PlutusVersion::V2)?),
                7 => out.extend(decode_plutus_script_array(data, offset, PlutusVersion::V3)?),
                _ => {
                    let _ = cbor::skip_item(data, offset)?;
                }
            }
            Ok(())
        };

    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_key(data, offset)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)? {
                process_key(data, offset)?;
            }
            *offset += 1;
        }
    }

    Ok(out)
}

fn decode_plutus_script_array(
    data: &[u8],
    offset: &mut usize,
    version: PlutusVersion,
) -> Result<Vec<PlutusScriptEntry>, LedgerError> {
    // Conway may wrap the array with tag(258) for set encoding.
    skip_optional_tag(data, offset)?;
    let enc = cbor::read_array_header(data, offset)?;
    let mut out = Vec::new();

    let mut read_one = |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
        let (bytes, _width) = cbor::read_bytes(data, offset)?;
        out.push(PlutusScriptEntry {
            version,
            flat_bytes: bytes,
        });
        Ok(())
    };

    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                read_one(data, offset)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)? {
                read_one(data, offset)?;
            }
            *offset += 1;
        }
    }

    Ok(out)
}

/// Extract Plutus script entries from an entire witness_sets CBOR array
/// (the block-level witness container). Returns a flat list across all
/// transactions in the block; tx boundaries are not preserved.
///
/// For tx-boundary-preserving extraction, iterate
/// `decode_plutus_scripts_in_witness_set` manually.
pub fn decode_all_plutus_scripts_in_block(
    witness_sets_cbor: &[u8],
) -> Result<Vec<PlutusScriptEntry>, LedgerError> {
    let mut offset = 0;
    let enc = cbor::read_array_header(witness_sets_cbor, &mut offset)?;
    let mut out = Vec::new();
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                out.extend(decode_plutus_scripts_in_witness_set(
                    witness_sets_cbor,
                    &mut offset,
                )?);
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(witness_sets_cbor, offset)? {
                out.extend(decode_plutus_scripts_in_witness_set(
                    witness_sets_cbor,
                    &mut offset,
                )?);
            }
        }
    }
    Ok(out)
}

/// Aggregate execution units declared across all redeemers in a
/// witness set.
///
/// Shelley-through-Mary witness sets don't carry redeemers at all;
/// this is `(0, 0)` for those eras. Alonzo/Babbage use the array
/// form `[* redeemer]` where each `redeemer = [tag, index, data,
/// ex_units]`. Conway introduced the map form `{(tag, index) =>
/// (data, ex_units)}`; both are decoded. Redeemer payloads other
/// than `ex_units` are not retained here — Phase 3B slices parse
/// them where they're needed.
///
/// The declared values are authoritative for the tx-level budget cap
/// check (`validateExUnitsTooBigUTxO` in cardano-ledger): the ledger
/// sums pointwise across all redeemers and compares against
/// `ppMaxTxExUnits`. See O-30.3 discharge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TotalExUnits {
    pub mem: i64,
    pub cpu: i64,
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
    /// Sum of declared `ex_units` across all redeemers (key 5).
    /// `(0, 0)` if redeemers are absent or unparseable. See
    /// [`TotalExUnits`] for format details.
    pub total_ex_units: TotalExUnits,
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
        total_ex_units: TotalExUnits::default(),
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
            5 => {
                // Redeemers: sum ex_units across all entries. See
                // `sum_redeemer_ex_units` for format handling
                // (Alonzo/Babbage array vs. Conway map).
                info.total_ex_units = sum_redeemer_ex_units(data, offset)?;
            }
            _ => {
                // Keys 2 (bootstrap), 4 (datums), others: skip.
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
            total_ex_units: TotalExUnits::default(),
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
            total_ex_units: TotalExUnits::default(),
        };
        assert!(!info.has_plutus());
    }
}
