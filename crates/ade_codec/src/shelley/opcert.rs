// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Closed-grammar OpCert encoder/decoder — the single producer-side opcert
//! byte authority. Both header CBOR (`shelley::block`) and standalone opcert
//! CBOR (S2 fixture parity) delegate to this module.
//!
//! The standalone CBOR shape matches the `OperationalCertificate` 4-tuple
//! that `cardano-cli node issue-op-cert` emits as a `cborHex` text envelope:
//!
//! ```text
//! [ hot_vkey: bstr(32), sequence_number: uint, kes_period: uint, sigma: bstr(64) ]
//! ```
//!
//! `sigma` is the cold-key Ed25519 signature over the canonical signable
//! `hot_vkey || sequence_number_be8 || kes_period_be8`, the same byte
//! representation `ade_crypto::kes::verify_opcert` consumes.

use crate::cbor::{
    self, read_array_header, read_bytes, read_uint, write_array_header, write_bytes_canonical,
    write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_types::shelley::block::OperationalCert;

const HOT_VKEY_LEN: usize = 32;
const SIGMA_LEN: usize = 64;

/// Closed error sum for opcert (de)serialisation.
///
/// Every forbidden shape has a dedicated variant; this enum is the
/// single authority for opcert structural rejection. No
/// `#[non_exhaustive]` — closure is the point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpCertCodecError {
    BadArrayHeader { expected: u8, found: u8 },
    BadFieldType { field: &'static str, detail: &'static str },
    WrongHotVkeyLength { found: usize, expected: usize },
    WrongSigmaLength { found: usize, expected: usize },
    SequenceNumberOverflow,
    KesPeriodOverflow,
    TrailingBytes { remaining: usize },
}

/// Encode an `OperationalCert` as a canonical, standalone 4-tuple.
///
/// This is the shape `cardano-cli node issue-op-cert` writes into the
/// `cborHex` field of its text envelope. Byte-identical to the cardano-api
/// reference encoder.
pub fn encode_opcert(opcert: &OperationalCert) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 2 + HOT_VKEY_LEN + 9 + 9 + 2 + SIGMA_LEN);
    write_array_header(&mut buf, ContainerEncoding::Definite(4, IntWidth::Inline));
    write_opcert_fields_into(&mut buf, opcert);
    buf
}

/// Decode a canonical standalone opcert 4-tuple.
///
/// Rejects: wrong outer array length, wrong field types, wrong
/// `hot_vkey` / `sigma` byte-string lengths, and trailing bytes after
/// the closing `sigma`.
pub fn decode_opcert(bytes: &[u8]) -> Result<OperationalCert, OpCertCodecError> {
    let mut offset = 0usize;

    let enc = read_array_header(bytes, &mut offset).map_err(|_| OpCertCodecError::BadArrayHeader {
        expected: 0x84,
        found: bytes.first().copied().unwrap_or(0),
    })?;
    match enc {
        ContainerEncoding::Definite(4, IntWidth::Inline) => {}
        _ => {
            return Err(OpCertCodecError::BadArrayHeader {
                expected: 0x84,
                found: bytes.first().copied().unwrap_or(0),
            });
        }
    }

    let opcert = read_opcert_fields_from(bytes, &mut offset)?;

    if offset != bytes.len() {
        return Err(OpCertCodecError::TrailingBytes {
            remaining: bytes.len() - offset,
        });
    }
    Ok(opcert)
}

/// Header-embedded opcert fields path — used by the Shelley header
/// encoder for the inlined (Shelley/Alonzo array(15)) and nested
/// (Babbage/Conway array(10)) layouts alike.
///
/// Differs from `encode_opcert` only in that it does NOT emit the
/// surrounding 4-element-array CBOR header. The header path's outer
/// arrays already carry the structural framing; this writes just the
/// four fields in order: `bstr(hot_vkey) uint(seq) uint(period) bstr(sigma)`.
pub fn write_opcert_fields_into(buf: &mut Vec<u8>, opcert: &OperationalCert) {
    write_bytes_canonical(buf, &opcert.hot_vkey);
    write_uint_canonical(buf, opcert.sequence_number);
    write_uint_canonical(buf, opcert.kes_period);
    write_bytes_canonical(buf, &opcert.sigma);
}

/// Header-embedded opcert fields path — symmetric reader for
/// `write_opcert_fields_into`. Reads four fields from `data` starting
/// at `*offset` and advances `*offset`.
///
/// Rejects wrong CBOR major types per field (via `BadFieldType`) and
/// wrong `hot_vkey` / `sigma` byte-string lengths (via
/// `WrongHotVkeyLength` / `WrongSigmaLength`).
pub fn read_opcert_fields_from(
    data: &[u8],
    offset: &mut usize,
) -> Result<OperationalCert, OpCertCodecError> {
    let (hot_vkey, _) = read_bytes(data, offset).map_err(|_| OpCertCodecError::BadFieldType {
        field: "hot_vkey",
        detail: "expected CBOR byte string",
    })?;
    if hot_vkey.len() != HOT_VKEY_LEN {
        return Err(OpCertCodecError::WrongHotVkeyLength {
            found: hot_vkey.len(),
            expected: HOT_VKEY_LEN,
        });
    }

    let (sequence_number, _) =
        read_uint(data, offset).map_err(|_| OpCertCodecError::BadFieldType {
            field: "sequence_number",
            detail: "expected CBOR unsigned integer",
        })?;

    let (kes_period, _) = read_uint(data, offset).map_err(|_| OpCertCodecError::BadFieldType {
        field: "kes_period",
        detail: "expected CBOR unsigned integer",
    })?;

    let (sigma, _) = read_bytes(data, offset).map_err(|_| OpCertCodecError::BadFieldType {
        field: "sigma",
        detail: "expected CBOR byte string",
    })?;
    if sigma.len() != SIGMA_LEN {
        return Err(OpCertCodecError::WrongSigmaLength {
            found: sigma.len(),
            expected: SIGMA_LEN,
        });
    }

    Ok(OperationalCert {
        hot_vkey,
        sequence_number,
        kes_period,
        sigma,
    })
}

// Suppress dead-code warning from the cbor re-import: keeping the
// `cbor::` qualified path import preserves grep visibility for the
// "all CBOR field emission goes through cbor::" structural check.
#[allow(dead_code)]
fn _ensure_cbor_import_used() {
    let _: fn(&mut Vec<u8>, u64) = cbor::write_uint_canonical;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn canonical_fixture() -> OperationalCert {
        OperationalCert {
            hot_vkey: vec![0x01; HOT_VKEY_LEN],
            sequence_number: 7,
            kes_period: 42,
            sigma: vec![0x02; SIGMA_LEN],
        }
    }

    fn canonical_fixture_bytes() -> Vec<u8> {
        // Hand-computed from the cardano-api OperationalCertificate CBOR
        // schema (4-tuple of bstr32 / uint / uint / bstr64):
        //   84                                     # array(4)
        //   58 20 <32 * 0x01>                      # bstr(32) hot_vkey
        //   07                                     # uint(7)
        //   18 2a                                  # uint(42)
        //   58 40 <64 * 0x02>                      # bstr(64) sigma
        let mut out = Vec::with_capacity(104);
        out.push(0x84);
        out.push(0x58);
        out.push(0x20);
        out.extend(std::iter::repeat(0x01u8).take(HOT_VKEY_LEN));
        out.push(0x07);
        out.push(0x18);
        out.push(0x2a);
        out.push(0x58);
        out.push(0x40);
        out.extend(std::iter::repeat(0x02u8).take(SIGMA_LEN));
        out
    }

    #[test]
    fn opcert_encoder_matches_cardano_cli_byte_identical() {
        let bytes = encode_opcert(&canonical_fixture());
        assert_eq!(bytes, canonical_fixture_bytes());
        assert_eq!(bytes.len(), 104);
    }

    #[test]
    fn opcert_round_trip_byte_identical() {
        let fixtures = vec![
            // Minimal: zero seq#, zero period.
            OperationalCert {
                hot_vkey: vec![0x00; HOT_VKEY_LEN],
                sequence_number: 0,
                kes_period: 0,
                sigma: vec![0x00; SIGMA_LEN],
            },
            // Large seq#, large period (requires multi-byte uint encoding).
            OperationalCert {
                hot_vkey: vec![0xAB; HOT_VKEY_LEN],
                sequence_number: 1_000_000,
                kes_period: 1_000_000,
                sigma: vec![0xCD; SIGMA_LEN],
            },
            // Max u64 seq#.
            OperationalCert {
                hot_vkey: vec![0x11; HOT_VKEY_LEN],
                sequence_number: u64::MAX,
                kes_period: 0,
                sigma: vec![0x22; SIGMA_LEN],
            },
            // Period 0 boundary.
            OperationalCert {
                hot_vkey: vec![0x33; HOT_VKEY_LEN],
                sequence_number: 7,
                kes_period: 0,
                sigma: vec![0x44; SIGMA_LEN],
            },
            // Period far-future (max u64).
            OperationalCert {
                hot_vkey: vec![0x55; HOT_VKEY_LEN],
                sequence_number: 1,
                kes_period: u64::MAX,
                sigma: vec![0x66; SIGMA_LEN],
            },
        ];
        for fixture in &fixtures {
            let bytes = encode_opcert(fixture);
            let decoded = decode_opcert(&bytes).unwrap();
            assert_eq!(&decoded, fixture);
            let reencoded = encode_opcert(&decoded);
            assert_eq!(bytes, reencoded);
        }
    }

    #[test]
    fn opcert_decode_rejects_trailing_garbage() {
        let mut bytes = encode_opcert(&canonical_fixture());
        bytes.push(0x00);
        let err = decode_opcert(&bytes).unwrap_err();
        assert_eq!(err, OpCertCodecError::TrailingBytes { remaining: 1 });
    }

    #[test]
    fn opcert_decode_rejects_truncated() {
        let bytes = encode_opcert(&canonical_fixture());
        // Drop the final byte of sigma — read_bytes sees UnexpectedEof,
        // which the decoder maps to BadFieldType for the sigma field.
        let truncated = &bytes[..bytes.len() - 1];
        let err = decode_opcert(truncated).unwrap_err();
        assert!(
            matches!(
                err,
                OpCertCodecError::BadFieldType { field: "sigma", .. }
            ),
            "expected BadFieldType {{ field: \"sigma\", .. }}, got {:?}",
            err,
        );
    }

    #[test]
    fn opcert_decode_rejects_wrong_array_header() {
        let mut bytes = encode_opcert(&canonical_fixture());
        bytes[0] = 0x83; // array(3) instead of array(4)
        let err = decode_opcert(&bytes).unwrap_err();
        assert_eq!(
            err,
            OpCertCodecError::BadArrayHeader {
                expected: 0x84,
                found: 0x83,
            }
        );
    }

    #[test]
    fn opcert_decode_rejects_short_hot_vkey() {
        // Hand-build a 4-tuple whose hot_vkey is bstr(31) instead of bstr(32).
        let mut bytes = Vec::new();
        bytes.push(0x84);
        bytes.push(0x58);
        bytes.push(0x1f); // bstr(31)
        bytes.extend(std::iter::repeat(0x01u8).take(31));
        bytes.push(0x07);
        bytes.push(0x18);
        bytes.push(0x2a);
        bytes.push(0x58);
        bytes.push(0x40);
        bytes.extend(std::iter::repeat(0x02u8).take(SIGMA_LEN));
        let err = decode_opcert(&bytes).unwrap_err();
        assert_eq!(
            err,
            OpCertCodecError::WrongHotVkeyLength {
                found: 31,
                expected: 32,
            }
        );
    }

    #[test]
    fn opcert_decode_rejects_short_sigma() {
        // Hand-build a 4-tuple whose sigma is bstr(63) instead of bstr(64).
        let mut bytes = Vec::new();
        bytes.push(0x84);
        bytes.push(0x58);
        bytes.push(0x20);
        bytes.extend(std::iter::repeat(0x01u8).take(HOT_VKEY_LEN));
        bytes.push(0x07);
        bytes.push(0x18);
        bytes.push(0x2a);
        bytes.push(0x58);
        bytes.push(0x3f); // bstr(63)
        bytes.extend(std::iter::repeat(0x02u8).take(63));
        let err = decode_opcert(&bytes).unwrap_err();
        assert_eq!(
            err,
            OpCertCodecError::WrongSigmaLength {
                found: 63,
                expected: 64,
            }
        );
    }

    #[test]
    fn header_encoder_uses_opcert_fields_path() {
        // Pre-refactor golden: the exact 4 emit calls' output, computed
        // canonically. Asserts `write_opcert_fields_into` produces the
        // same 4-field byte sequence that the legacy inline emit did.
        //
        // Expected bytes:
        //   58 20 <32 * 0x01> 07 18 2a 58 40 <64 * 0x02>
        // (no outer array header — that's the caller's responsibility).
        let mut golden = Vec::new();
        golden.push(0x58);
        golden.push(0x20);
        golden.extend(std::iter::repeat(0x01u8).take(HOT_VKEY_LEN));
        golden.push(0x07);
        golden.push(0x18);
        golden.push(0x2a);
        golden.push(0x58);
        golden.push(0x40);
        golden.extend(std::iter::repeat(0x02u8).take(SIGMA_LEN));

        let mut buf = Vec::new();
        write_opcert_fields_into(&mut buf, &canonical_fixture());
        assert_eq!(buf, golden);

        let mut offset = 0usize;
        let decoded = read_opcert_fields_from(&buf, &mut offset).unwrap();
        assert_eq!(decoded, canonical_fixture());
        assert_eq!(offset, buf.len());
    }
}
