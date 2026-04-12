// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Cost model parsing (Phase 3 Cluster P-B, slice S-30 item 1).
//!
//! Parses the `cost_models` pparams field from its CBOR wire form
//! into per-language `Vec<i64>` coefficient arrays. Aiken's
//! `Program::eval_as(version, costs, budget)` accepts `&[i64]`
//! directly, so no struct-adapter is required — the positional
//! integer array IS aiken's expected input.
//!
//! Wire format (Babbage+):
//! ```cddl
//! cost_models = { * uint => [ * int ] }
//! ```
//! Language indices: `0 = V1`, `1 = V2`, `2 = V3`.
//!
//! Conway (PV ≥ 9) is LENIENT: unknown language indices go into a
//! sidecar map; array-length deviations past the known baseline are
//! tolerated (forward-compat with post-V3 cost-model extensions).
//! Pre-PV9 behavior is strict.
//!
//! Discharge: docs/active/S-30_obligation_discharge.md §O-30.1.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    self, read_array_header, read_map_header, read_uint, ContainerEncoding,
    MAJOR_NEGATIVE, MAJOR_UNSIGNED,
};
use ade_codec::CodecError;

use crate::evaluator::PlutusLanguage;

/// Parsed cost models keyed by Plutus language.
///
/// Each entry is the positional `Vec<i64>` accepted by aiken's
/// `eval_as`. Parameter names are implicit — order matches plutus's
/// `Plutus.V{1,2,3}::ParamName` lists.
///
/// `unknown_languages` holds entries for any language index the
/// parser did not recognize. Present only under PV9+ lenient
/// decoding; callers typically ignore this field but surface it in
/// error messages when a tx references an unsupported language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostModels {
    pub v1: Option<Vec<i64>>,
    pub v2: Option<Vec<i64>>,
    pub v3: Option<Vec<i64>>,
    pub unknown_languages: BTreeMap<u8, Vec<i64>>,
}

impl CostModels {
    pub fn new() -> Self {
        CostModels {
            v1: None,
            v2: None,
            v3: None,
            unknown_languages: BTreeMap::new(),
        }
    }

    /// Look up the cost-model coefficients for a Plutus language.
    pub fn get(&self, language: PlutusLanguage) -> Option<&[i64]> {
        match language {
            PlutusLanguage::V1 => self.v1.as_deref(),
            PlutusLanguage::V2 => self.v2.as_deref(),
            PlutusLanguage::V3 => self.v3.as_deref(),
        }
    }

    /// True if at least one known language has a cost model.
    pub fn is_empty(&self) -> bool {
        self.v1.is_none()
            && self.v2.is_none()
            && self.v3.is_none()
            && self.unknown_languages.is_empty()
    }
}

impl Default for CostModels {
    fn default() -> Self {
        Self::new()
    }
}

/// Strictness mode for the decoder.
///
/// - `Strict`: pre-Conway (PV < 9). Unknown language indices are a
///   decode error.
/// - `Lenient`: Conway+ (PV ≥ 9). Unknown languages go into
///   `unknown_languages`; no error raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderMode {
    Strict,
    Lenient,
}

/// Decode `cost_models` from CBOR wire bytes.
///
/// Advances `offset` past the cost_models map. Callers who have
/// already advanced to the value position pass `&mut offset`; callers
/// who want to decode a standalone cost-models bytestring pass a
/// fresh `0usize`.
pub fn decode_cost_models(
    data: &[u8],
    offset: &mut usize,
    mode: DecoderMode,
) -> Result<CostModels, CodecError> {
    let enc = read_map_header(data, offset)?;
    let mut out = CostModels::new();

    let mut process_entry = |data: &[u8], offset: &mut usize| -> Result<(), CodecError> {
        let (lang_index, _) = read_uint(data, offset)?;
        let costs = read_int_array(data, offset)?;
        match lang_index {
            0 => out.v1 = Some(costs),
            1 => out.v2 = Some(costs),
            2 => out.v3 = Some(costs),
            other => match mode {
                DecoderMode::Lenient => {
                    let idx = clamp_to_u8(other);
                    out.unknown_languages.insert(idx, costs);
                }
                DecoderMode::Strict => {
                    return Err(CodecError::InvalidCborStructure {
                        offset: *offset,
                        detail: "unknown Plutus language index (strict mode)",
                    });
                }
            },
        }
        Ok(())
    };

    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_entry(data, offset)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, *offset)? {
                process_entry(data, offset)?;
            }
            *offset += 1;
        }
    }

    Ok(out)
}

/// Read a CBOR array of signed integers. CBOR `int` is major type 0
/// (unsigned, value `n`) or major type 1 (negative, value `-1-n`).
///
/// Cost-model coefficients are typed `Int64` in the Haskell ledger;
/// the parser preserves that as `i64`. Values outside `i64` range
/// are rejected with an error rather than silently clamped.
fn read_int_array(data: &[u8], offset: &mut usize) -> Result<Vec<i64>, CodecError> {
    let enc = read_array_header(data, offset)?;
    let mut out = Vec::new();

    let mut process_one = |data: &[u8], offset: &mut usize| -> Result<(), CodecError> {
        let v = read_i64(data, offset)?;
        out.push(v);
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

    Ok(out)
}

/// Read a CBOR signed integer (major 0 or major 1) as `i64`.
/// Overflow (value outside `i64` range) is a decode error.
fn read_i64(data: &[u8], offset: &mut usize) -> Result<i64, CodecError> {
    let start = *offset;
    let initial = *data.get(start).ok_or(CodecError::UnexpectedEof {
        offset: start,
        needed: 1,
    })?;
    let major = initial >> 5;
    match major {
        MAJOR_UNSIGNED => {
            let (u, _) = read_uint(data, offset)?;
            if u > i64::MAX as u64 {
                return Err(CodecError::InvalidCborStructure {
                    offset: start,
                    detail: "unsigned int exceeds i64::MAX",
                });
            }
            Ok(u as i64)
        }
        MAJOR_NEGATIVE => {
            // Major type 1 encodes -1-n where n is the argument.
            // Overwrite the major-type bits with unsigned so we can
            // reuse read_uint for the argument.
            let mut o2 = *offset;
            // Read the argument as if it were major 0.
            // We can't just mask in-place because read_uint expects a
            // real unsigned. Reimplement argument extraction here.
            let initial_b = data[o2];
            o2 += 1;
            let ai = initial_b & 0x1f;
            let n: u64 = match ai {
                0..=23 => u64::from(ai),
                24 => {
                    let b = *data.get(o2).ok_or(CodecError::UnexpectedEof {
                        offset: o2,
                        needed: 1,
                    })?;
                    o2 += 1;
                    u64::from(b)
                }
                25 => {
                    if o2 + 2 > data.len() {
                        return Err(CodecError::UnexpectedEof {
                            offset: o2,
                            needed: 2,
                        });
                    }
                    let v = u16::from_be_bytes([data[o2], data[o2 + 1]]);
                    o2 += 2;
                    u64::from(v)
                }
                26 => {
                    if o2 + 4 > data.len() {
                        return Err(CodecError::UnexpectedEof {
                            offset: o2,
                            needed: 4,
                        });
                    }
                    let v = u32::from_be_bytes([
                        data[o2],
                        data[o2 + 1],
                        data[o2 + 2],
                        data[o2 + 3],
                    ]);
                    o2 += 4;
                    u64::from(v)
                }
                27 => {
                    if o2 + 8 > data.len() {
                        return Err(CodecError::UnexpectedEof {
                            offset: o2,
                            needed: 8,
                        });
                    }
                    let v = u64::from_be_bytes([
                        data[o2], data[o2 + 1], data[o2 + 2], data[o2 + 3],
                        data[o2 + 4], data[o2 + 5], data[o2 + 6], data[o2 + 7],
                    ]);
                    o2 += 8;
                    v
                }
                _ => {
                    return Err(CodecError::InvalidCborStructure {
                        offset: start,
                        detail: "reserved additional info in negative int",
                    });
                }
            };
            // value = -1 - n
            if n > i64::MAX as u64 {
                // -1 - n would overflow below i64::MIN
                return Err(CodecError::InvalidCborStructure {
                    offset: start,
                    detail: "negative int below i64::MIN",
                });
            }
            let value = -1i64 - (n as i64);
            *offset = o2;
            Ok(value)
        }
        _ => Err(CodecError::InvalidCborStructure {
            offset: start,
            detail: "expected CBOR int (major 0 or 1)",
        }),
    }
}

fn clamp_to_u8(v: u64) -> u8 {
    if v > u8::MAX as u64 {
        u8::MAX
    } else {
        v as u8
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_codec::cbor::{
        write_array_header, write_map_header, write_uint_canonical,
        ContainerEncoding, IntWidth, MAJOR_NEGATIVE,
    };

    fn write_canonical_map(buf: &mut Vec<u8>, n: u64) {
        write_map_header(buf, ContainerEncoding::Definite(n, IntWidth::Inline));
    }

    fn write_canonical_array(buf: &mut Vec<u8>, n: u64) {
        write_array_header(buf, ContainerEncoding::Definite(n, IntWidth::Inline));
    }

    fn write_neg(buf: &mut Vec<u8>, value: i64) {
        // Encode -1 - n where n = -1 - value for a negative CBOR int.
        assert!(value < 0);
        let n: u64 = (-1 - value) as u64;
        ade_codec::cbor::write_argument(buf, MAJOR_NEGATIVE, n, IntWidth::Inline);
    }

    #[test]
    fn decode_empty_cost_models() {
        let mut buf = Vec::new();
        write_canonical_map(&mut buf, 0);
        let mut off = 0;
        let cm = decode_cost_models(&buf, &mut off, DecoderMode::Strict).unwrap();
        assert!(cm.is_empty());
    }

    #[test]
    fn decode_v1_only() {
        let mut buf = Vec::new();
        write_canonical_map(&mut buf, 1);
        write_uint_canonical(&mut buf, 0); // language V1
        write_canonical_array(&mut buf, 3);
        write_uint_canonical(&mut buf, 100);
        write_uint_canonical(&mut buf, 200);
        write_uint_canonical(&mut buf, 300);

        let mut off = 0;
        let cm = decode_cost_models(&buf, &mut off, DecoderMode::Strict).unwrap();
        assert_eq!(cm.v1, Some(vec![100, 200, 300]));
        assert_eq!(cm.v2, None);
        assert_eq!(cm.v3, None);
    }

    #[test]
    fn decode_all_three_languages() {
        let mut buf = Vec::new();
        write_canonical_map(&mut buf, 3);
        for (lang_idx, values) in [(0u64, vec![1, 2]), (1, vec![3, 4]), (2, vec![5, 6])] {
            write_uint_canonical(&mut buf, lang_idx);
            write_canonical_array(&mut buf, values.len() as u64);
            for v in values {
                write_uint_canonical(&mut buf, v);
            }
        }

        let mut off = 0;
        let cm = decode_cost_models(&buf, &mut off, DecoderMode::Strict).unwrap();
        assert_eq!(cm.v1, Some(vec![1, 2]));
        assert_eq!(cm.v2, Some(vec![3, 4]));
        assert_eq!(cm.v3, Some(vec![5, 6]));
    }

    #[test]
    fn decode_negative_int_coefficient() {
        let mut buf = Vec::new();
        write_canonical_map(&mut buf, 1);
        write_uint_canonical(&mut buf, 0);
        write_canonical_array(&mut buf, 2);
        write_uint_canonical(&mut buf, 42);
        write_neg(&mut buf, -17);

        let mut off = 0;
        let cm = decode_cost_models(&buf, &mut off, DecoderMode::Strict).unwrap();
        assert_eq!(cm.v1, Some(vec![42, -17]));
    }

    #[test]
    fn strict_rejects_unknown_language() {
        let mut buf = Vec::new();
        write_canonical_map(&mut buf, 1);
        write_uint_canonical(&mut buf, 5); // unknown
        write_canonical_array(&mut buf, 1);
        write_uint_canonical(&mut buf, 999);

        let mut off = 0;
        let res = decode_cost_models(&buf, &mut off, DecoderMode::Strict);
        assert!(res.is_err(), "strict mode should reject unknown language");
    }

    #[test]
    fn lenient_accepts_unknown_language() {
        let mut buf = Vec::new();
        write_canonical_map(&mut buf, 1);
        write_uint_canonical(&mut buf, 5);
        write_canonical_array(&mut buf, 1);
        write_uint_canonical(&mut buf, 999);

        let mut off = 0;
        let cm = decode_cost_models(&buf, &mut off, DecoderMode::Lenient).unwrap();
        assert_eq!(cm.v1, None);
        assert_eq!(cm.unknown_languages.get(&5u8), Some(&vec![999]));
    }

    #[test]
    fn get_returns_expected_array() {
        let mut cm = CostModels::new();
        cm.v3 = Some(vec![10, 20, 30]);
        assert_eq!(cm.get(PlutusLanguage::V3), Some(&[10, 20, 30][..]));
        assert_eq!(cm.get(PlutusLanguage::V1), None);
    }

    #[test]
    fn decoder_deterministic() {
        let mut buf = Vec::new();
        write_canonical_map(&mut buf, 1);
        write_uint_canonical(&mut buf, 2);
        write_canonical_array(&mut buf, 4);
        for v in [1, 2, 3, 4] {
            write_uint_canonical(&mut buf, v);
        }

        let mut o1 = 0;
        let mut o2 = 0;
        let cm1 = decode_cost_models(&buf, &mut o1, DecoderMode::Strict).unwrap();
        let cm2 = decode_cost_models(&buf, &mut o2, DecoderMode::Strict).unwrap();
        assert_eq!(cm1, cm2);
        assert_eq!(o1, o2);
    }
}
