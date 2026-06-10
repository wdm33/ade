// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE RecoveredAnchorPoint type + canonical CBOR codec (PHASE4-N-AK AK-S1).
//!
//! `RecoveredAnchorPoint` is the closed, version-gated, byte-canonical record of
//! the bootstrap anchor POINT — the `(slot, block_hash)` the recovered store was
//! seeded at — persisted as replayable recovery provenance and bound to the
//! recovered anchor fingerprint (`anchor_fp`). It is the durable restart
//! authority for the live-follow start tip (DC-NODE-31): on warm-start the
//! recover path loads + fail-closed verifies it, and `bootstrap_initial_state`
//! resolves the FindIntersect start from it so a bare-anchor recovery starts at
//! the anchor, not Origin.
//!
//! This is a SEPARATE additive record from `SeedEpochConsensusInputs` (the
//! seed-epoch consensus sidecar): it lives in its own anchor-fp-keyed
//! `SnapshotStore` surface and does NOT touch the seed-epoch sidecar's shape,
//! schema version, or its `sidecar_hash`/provenance binding. Both records share
//! the same `anchor_fp` key (`BootstrapAnchor.initial_ledger_fingerprint`), so a
//! store discoverable via the seed-epoch sidecar lineage also carries this
//! record — or fails closed.
//!
//! CN-CINPUT-01 analog: `encode_recovered_anchor_point` /
//! `decode_recovered_anchor_point` is the SOLE pub fn pair encoding/decoding
//! `RecoveredAnchorPoint`. No `Default` impl and no `#[non_exhaustive]`: the type
//! system requires all fields at construction. `RECOVERED_ANCHOR_POINT_SCHEMA_VERSION
//! = 1` is written into the encoded form; decode rejects unknown versions
//! fail-fast, rejects short hash bytes, rejects trailing bytes, and verifies a
//! byte-canonical round-trip (re-encode == input).

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_uint, write_array_header,
    write_bytes_canonical, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_types::{Hash32, SlotNo};

/// Pinned wire schema version. Decode rejects any other (fail-closed).
pub const RECOVERED_ANCHOR_POINT_SCHEMA_VERSION: u32 = 1;

const FIELDS_OUTER: u64 = 4;

/// Closed record: the persisted bootstrap anchor point, bound to the recovered
/// anchor fingerprint. All fields required at construction; no `Default`, no
/// `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredAnchorPoint {
    /// Binds this record to a specific `BootstrapAnchor`
    /// (`initial_ledger_fingerprint`), self-describing — the warm-start load
    /// fails closed if this disagrees with the recovered `anchor_fp`.
    pub anchor_fp: Hash32,
    /// The anchor block's slot (`BootstrapAnchor.seed_point.slot`).
    pub slot: SlotNo,
    /// The anchor block's hash (`BootstrapAnchor.seed_point.block_hash`). A
    /// zero/null hash denotes a genesis seed point (truly Origin) — the
    /// live-follow resolver treats it as Origin, never as a usable start point.
    pub block_hash: Hash32,
}

/// Closed error sum for `RecoveredAnchorPoint` encode/decode. Carries only
/// non-secret primitives; no `String`/`anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveredAnchorPointError {
    /// CBOR primitive read error or non-byte-canonical encoding.
    MalformedCbor,
    /// Decoded schema version did not match `RECOVERED_ANCHOR_POINT_SCHEMA_VERSION`.
    UnknownVersion { expected: u32, found: u32 },
    /// Decoded buffer did not match the expected closed CBOR shape
    /// (wrong array header, wrong hash byte width, field overflow).
    Structural { reason: &'static str },
    /// Trailing bytes after the expected record structure.
    TrailingBytes { extra: usize },
}

/// Canonical CBOR encode. Sole pub encoder.
///
/// Wire shape:
/// ```text
/// array(4) [
///   uint   RECOVERED_ANCHOR_POINT_SCHEMA_VERSION (= 1),
///   bytes(32) anchor_fp,
///   uint   slot,
///   bytes(32) block_hash,
/// ]
/// ```
pub fn encode_recovered_anchor_point(record: &RecoveredAnchorPoint) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
    );
    write_uint_canonical(&mut buf, RECOVERED_ANCHOR_POINT_SCHEMA_VERSION as u64);
    write_bytes_canonical(&mut buf, &record.anchor_fp.0);
    write_uint_canonical(&mut buf, record.slot.0);
    write_bytes_canonical(&mut buf, &record.block_hash.0);
    buf
}

/// Canonical CBOR decode. Sole pub decoder. Fail-fast on unknown schema
/// version, wrong array shape, short hash bytes, trailing bytes, or any
/// non-byte-canonical encoding (re-encode != input).
pub fn decode_recovered_anchor_point(
    bytes: &[u8],
) -> Result<RecoveredAnchorPoint, RecoveredAnchorPointError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, FIELDS_OUTER, "outer")?;

    let version = read_u32_field(bytes, &mut o)?;
    if version != RECOVERED_ANCHOR_POINT_SCHEMA_VERSION {
        return Err(RecoveredAnchorPointError::UnknownVersion {
            expected: RECOVERED_ANCHOR_POINT_SCHEMA_VERSION,
            found: version,
        });
    }

    let anchor_fp = read_hash32(bytes, &mut o)?;
    let slot = SlotNo(read_u64_field(bytes, &mut o)?);
    let block_hash = read_hash32(bytes, &mut o)?;

    if o != bytes.len() {
        return Err(RecoveredAnchorPointError::TrailingBytes {
            extra: bytes.len() - o,
        });
    }

    let decoded = RecoveredAnchorPoint {
        anchor_fp,
        slot,
        block_hash,
    };

    // Byte-canonical: a structurally valid but non-canonically-encoded buffer
    // (e.g. a uint written wider than minimal) decodes to the same value but
    // re-encodes to different bytes. Reject it fail-closed.
    if encode_recovered_anchor_point(&decoded) != bytes {
        return Err(RecoveredAnchorPointError::MalformedCbor);
    }

    Ok(decoded)
}

fn expect_definite_array(
    bytes: &[u8],
    offset: &mut usize,
    expected_len: u64,
    label: &'static str,
) -> Result<(), RecoveredAnchorPointError> {
    let enc = read_array_header(bytes, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        ContainerEncoding::Definite(_, _) => Err(RecoveredAnchorPointError::Structural {
            reason: match label {
                "outer" => "outer array has wrong field count",
                _ => "array has wrong field count",
            },
        }),
        ContainerEncoding::Indefinite => Err(RecoveredAnchorPointError::Structural {
            reason: "indefinite-length array not allowed in RecoveredAnchorPoint",
        }),
    }
}

fn read_u32_field(bytes: &[u8], offset: &mut usize) -> Result<u32, RecoveredAnchorPointError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    if n > u32::MAX as u64 {
        return Err(RecoveredAnchorPointError::Structural {
            reason: "u32 field overflowed",
        });
    }
    Ok(n as u32)
}

fn read_u64_field(bytes: &[u8], offset: &mut usize) -> Result<u64, RecoveredAnchorPointError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    Ok(n)
}

fn read_hash32(bytes: &[u8], offset: &mut usize) -> Result<Hash32, RecoveredAnchorPointError> {
    let (h, _w) = read_bytes(bytes, offset)?;
    if h.len() != 32 {
        return Err(RecoveredAnchorPointError::Structural {
            reason: "expected 32-byte hash",
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&h);
    Ok(Hash32(arr))
}

impl From<ade_codec::CodecError> for RecoveredAnchorPointError {
    fn from(_e: ade_codec::CodecError) -> Self {
        Self::MalformedCbor
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample() -> RecoveredAnchorPoint {
        RecoveredAnchorPoint {
            anchor_fp: Hash32([0x42; 32]),
            slot: SlotNo(188),
            block_hash: Hash32([0x2e; 32]),
        }
    }

    #[test]
    fn recovered_anchor_point_round_trips_byte_identical() {
        let s = sample();
        let bytes = encode_recovered_anchor_point(&s);
        let decoded = decode_recovered_anchor_point(&bytes).expect("decode");
        assert_eq!(decoded, s);
        // encode -> decode -> encode = identical bytes.
        assert_eq!(encode_recovered_anchor_point(&decoded), bytes);
    }

    #[test]
    fn recovered_anchor_point_encode_two_runs_byte_identical() {
        let s = sample();
        assert_eq!(
            encode_recovered_anchor_point(&s),
            encode_recovered_anchor_point(&s)
        );
    }

    #[test]
    fn recovered_anchor_point_decode_rejects_unknown_version() {
        // The outer array header is 1 byte (0x84 for array(4)); the version
        // (=1) is the next 1 byte (0x01). So the body after the version starts
        // at index 2.
        let fresh = encode_recovered_anchor_point(&sample());
        for bad_version in [0u64, 2, 99] {
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
            );
            write_uint_canonical(&mut buf, bad_version);
            buf.extend_from_slice(&fresh[2..]);
            match decode_recovered_anchor_point(&buf) {
                Err(RecoveredAnchorPointError::UnknownVersion { expected: 1, found })
                    if found == bad_version as u32 => {}
                other => panic!("expected UnknownVersion for v{bad_version}, got {other:?}"),
            }
        }
    }

    #[test]
    fn recovered_anchor_point_decode_rejects_trailing_bytes() {
        let mut bytes = encode_recovered_anchor_point(&sample());
        bytes.push(0xFF);
        match decode_recovered_anchor_point(&bytes) {
            Err(RecoveredAnchorPointError::TrailingBytes { extra: 1 }) => {}
            other => panic!("expected TrailingBytes, got {other:?}"),
        }
    }

    #[test]
    fn recovered_anchor_point_decode_rejects_short_buffer() {
        let bytes = encode_recovered_anchor_point(&sample());
        for trunc in [0usize, 1, 5, 32, bytes.len() - 1] {
            assert!(
                decode_recovered_anchor_point(&bytes[..trunc]).is_err(),
                "must fail at trunc={trunc}"
            );
        }
    }

    #[test]
    fn recovered_anchor_point_decode_rejects_short_hash() {
        // Build a valid array(4), version=1, then a 31-byte (not 32) anchor_fp.
        let mut buf = Vec::new();
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
        );
        write_uint_canonical(&mut buf, RECOVERED_ANCHOR_POINT_SCHEMA_VERSION as u64);
        write_bytes_canonical(&mut buf, &[0u8; 31]);
        match decode_recovered_anchor_point(&buf) {
            Err(RecoveredAnchorPointError::Structural { .. }) => {}
            other => panic!("expected Structural, got {other:?}"),
        }
    }

    #[test]
    fn recovered_anchor_point_match_is_exhaustive() {
        // Compile-time exhaustiveness probe — if RecoveredAnchorPoint adds a
        // field, this fails to compile until updated.
        let RecoveredAnchorPoint {
            anchor_fp,
            slot,
            block_hash,
        } = sample();
        assert_eq!(anchor_fp.0[0], 0x42);
        assert_eq!(slot, SlotNo(188));
        assert_eq!(block_hash.0[0], 0x2e);
    }

    #[test]
    fn recovered_anchor_point_zero_hash_round_trips() {
        // A genesis (zero-hash) seed point is a representable record — the
        // Origin distinction lives in the resolver, not the codec.
        let s = RecoveredAnchorPoint {
            anchor_fp: Hash32([0x11; 32]),
            slot: SlotNo(0),
            block_hash: Hash32([0u8; 32]),
        };
        let bytes = encode_recovered_anchor_point(&s);
        assert_eq!(decode_recovered_anchor_point(&bytes).expect("decode"), s);
    }
}
