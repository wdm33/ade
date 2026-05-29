// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE BootstrapAnchor type + canonical CBOR codec (PHASE4-N-M-A S2).
//!
//! `BootstrapAnchor` is the closed-record provenance bundle minted
//! at oracle-seed import. Every WAL entry chains back to the
//! anchor's `initial_ledger_fingerprint` (DC-WAL-02); the anchor
//! itself records the import inputs (oracle source, seed slot,
//! artifact hash, imported-UTxO fingerprint).
//!
//! CN-ANCHOR-01: this module's `encode_bootstrap_anchor` /
//! `decode_bootstrap_anchor` is the SOLE pub fn pair encoding /
//! decoding `BootstrapAnchor` in the workspace. No `Default` impl
//! and no `#[non_exhaustive]`: the type-system requires all 6
//! fields at construction (¬P-A3).
//!
//! DC-ANCHOR-01: canonical CBOR round-trip preserves byte-identity.
//! `ANCHOR_SCHEMA_VERSION = 2` is written into the encoded form; decode
//! rejects unknown versions fail-fast.
//!
//! CN-MITHRIL-01 / DC-MITHRIL-01 (PHASE4-N-Y S1): the anchor records
//! how its seed was sourced via the closed `SeedProvenance` field —
//! either the cardano-cli JSON path (`CardanoCliJson`) or a verified
//! Mithril snapshot (`Mithril { .. }`). The Mithril variant records
//! the mithril-client-verified certificate hash + certified point +
//! immutable range as closed provenance; the STM multisig is never a
//! BLUE trust root (verified by the RED mithril-client). Schema bumped
//! 1→2 (additive 8th outer field).

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_uint, write_array_header,
    write_bytes_canonical, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_types::{Hash32, SlotNo};

use super::error::BootstrapAnchorError;

/// Pinned wire schema version. Decode rejects any other.
pub const ANCHOR_SCHEMA_VERSION: u32 = 2;

const FIELDS_OUTER: u64 = 8;
const SEED_POINT_FIELDS: u64 = 2;

/// Closed discriminants for the `seed_provenance` field. Written as
/// the first element of the provenance array.
const PROVENANCE_TAG_CARDANO_CLI_JSON: u64 = 0;
const PROVENANCE_TAG_MITHRIL: u64 = 1;

/// Closed record: the provenance bundle for an oracle-seed
/// bootstrap. All 6 fields required at construction; no
/// `Default`, no `#[non_exhaustive]`.
///
/// Per memory [[feedback-oracle-seed-then-ade-owns]] the anchor
/// records what was imported (cardano-node oracle at point P);
/// after this point Ade owns the runtime representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapAnchor {
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub seed_point: SeedPoint,
    pub seed_artifact_hash: Hash32,
    pub imported_utxo_fingerprint: Hash32,
    pub initial_ledger_fingerprint: Hash32,
    pub seed_provenance: SeedProvenance,
}

/// Closed point reference (slot + block hash). Mirrors the
/// chain-sync `Point::Block` shape but is its own type to keep
/// the anchor independent of the wire codec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedPoint {
    pub slot: SlotNo,
    pub block_hash: Hash32,
}

/// Closed provenance of the imported seed. Makes the seed's origin
/// explicit and illegal states unrepresentable (DC-MITHRIL-01). The
/// `Mithril` variant records the mithril-client-verified certificate
/// hash plus the certified point + immutable range it binds — the
/// STM signature itself is never re-verified or trusted in BLUE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedProvenance {
    /// Seed imported from a cardano-cli JSON dump (CN-SEED-01 path).
    CardanoCliJson,
    /// Seed bound to a verified Mithril certificate.
    Mithril {
        certificate_hash: Hash32,
        certified_point: SeedPoint,
        immutable_range: (u64, u64),
    },
}

/// Canonical CBOR encode. CN-ANCHOR-01: sole pub encoder.
///
/// Wire shape:
/// ```text
/// array(8) [
///   uint   ANCHOR_SCHEMA_VERSION (= 2),
///   uint   network_magic,
///   bytes(32) genesis_hash,
///   array(2) [ uint seed_point.slot, bytes(32) block_hash ],
///   bytes(32) seed_artifact_hash,
///   bytes(32) imported_utxo_fingerprint,
///   bytes(32) initial_ledger_fingerprint,
///   seed_provenance:
///     CardanoCliJson  => array(1) [ uint 0 ]
///     Mithril         => array(5) [ uint 1, bytes(32) certificate_hash,
///                                   array(2) [ uint slot, bytes(32) block_hash ],
///                                   uint immutable_range.0, uint immutable_range.1 ],
/// ]
/// ```
pub fn encode_bootstrap_anchor(anchor: &BootstrapAnchor) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
    );
    write_uint_canonical(&mut buf, ANCHOR_SCHEMA_VERSION as u64);
    write_uint_canonical(&mut buf, anchor.network_magic as u64);
    write_bytes_canonical(&mut buf, &anchor.genesis_hash.0);

    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(SEED_POINT_FIELDS, canonical_width(SEED_POINT_FIELDS)),
    );
    write_uint_canonical(&mut buf, anchor.seed_point.slot.0);
    write_bytes_canonical(&mut buf, &anchor.seed_point.block_hash.0);

    write_bytes_canonical(&mut buf, &anchor.seed_artifact_hash.0);
    write_bytes_canonical(&mut buf, &anchor.imported_utxo_fingerprint.0);
    write_bytes_canonical(&mut buf, &anchor.initial_ledger_fingerprint.0);
    encode_seed_provenance(&mut buf, &anchor.seed_provenance);
    buf
}

fn encode_seed_provenance(buf: &mut Vec<u8>, provenance: &SeedProvenance) {
    match provenance {
        SeedProvenance::CardanoCliJson => {
            write_array_header(buf, ContainerEncoding::Definite(1, canonical_width(1)));
            write_uint_canonical(buf, PROVENANCE_TAG_CARDANO_CLI_JSON);
        }
        SeedProvenance::Mithril {
            certificate_hash,
            certified_point,
            immutable_range,
        } => {
            write_array_header(buf, ContainerEncoding::Definite(5, canonical_width(5)));
            write_uint_canonical(buf, PROVENANCE_TAG_MITHRIL);
            write_bytes_canonical(buf, &certificate_hash.0);
            write_array_header(
                buf,
                ContainerEncoding::Definite(SEED_POINT_FIELDS, canonical_width(SEED_POINT_FIELDS)),
            );
            write_uint_canonical(buf, certified_point.slot.0);
            write_bytes_canonical(buf, &certified_point.block_hash.0);
            write_uint_canonical(buf, immutable_range.0);
            write_uint_canonical(buf, immutable_range.1);
        }
    }
}

/// Canonical CBOR decode. CN-ANCHOR-01: sole pub decoder. Fail-
/// fast on unknown schema version, wrong array width, or short
/// hash bytes.
pub fn decode_bootstrap_anchor(
    bytes: &[u8],
) -> Result<BootstrapAnchor, BootstrapAnchorError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, FIELDS_OUTER, "outer")?;

    let version = read_u32_field(bytes, &mut o)?;
    if version != ANCHOR_SCHEMA_VERSION {
        return Err(BootstrapAnchorError::UnknownVersion {
            expected: ANCHOR_SCHEMA_VERSION,
            found: version,
        });
    }

    let network_magic = read_u32_field(bytes, &mut o)?;
    let genesis_hash = read_hash32(bytes, &mut o)?;

    expect_definite_array(bytes, &mut o, SEED_POINT_FIELDS, "seed_point")?;
    let slot = read_u64_field(bytes, &mut o)?;
    let block_hash = read_hash32(bytes, &mut o)?;
    let seed_point = SeedPoint {
        slot: SlotNo(slot),
        block_hash,
    };

    let seed_artifact_hash = read_hash32(bytes, &mut o)?;
    let imported_utxo_fingerprint = read_hash32(bytes, &mut o)?;
    let initial_ledger_fingerprint = read_hash32(bytes, &mut o)?;
    let seed_provenance = decode_seed_provenance(bytes, &mut o)?;

    if o != bytes.len() {
        return Err(BootstrapAnchorError::TrailingBytes {
            extra: bytes.len() - o,
        });
    }

    Ok(BootstrapAnchor {
        network_magic,
        genesis_hash,
        seed_point,
        seed_artifact_hash,
        imported_utxo_fingerprint,
        initial_ledger_fingerprint,
        seed_provenance,
    })
}

fn decode_seed_provenance(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<SeedProvenance, BootstrapAnchorError> {
    let enc = read_array_header(bytes, offset)?;
    let len = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(BootstrapAnchorError::Structural {
                reason: "indefinite-length array not allowed in seed_provenance",
            })
        }
    };
    let tag = read_u64_field(bytes, offset)?;
    match tag {
        PROVENANCE_TAG_CARDANO_CLI_JSON => {
            if len != 1 {
                return Err(BootstrapAnchorError::Structural {
                    reason: "CardanoCliJson provenance has wrong field count",
                });
            }
            Ok(SeedProvenance::CardanoCliJson)
        }
        PROVENANCE_TAG_MITHRIL => {
            if len != 5 {
                return Err(BootstrapAnchorError::Structural {
                    reason: "Mithril provenance has wrong field count",
                });
            }
            let certificate_hash = read_hash32(bytes, offset)?;
            expect_definite_array(bytes, offset, SEED_POINT_FIELDS, "seed_point")?;
            let slot = read_u64_field(bytes, offset)?;
            let block_hash = read_hash32(bytes, offset)?;
            let lo = read_u64_field(bytes, offset)?;
            let hi = read_u64_field(bytes, offset)?;
            Ok(SeedProvenance::Mithril {
                certificate_hash,
                certified_point: SeedPoint {
                    slot: SlotNo(slot),
                    block_hash,
                },
                immutable_range: (lo, hi),
            })
        }
        _ => Err(BootstrapAnchorError::Structural {
            reason: "unknown seed_provenance discriminant",
        }),
    }
}

fn expect_definite_array(
    bytes: &[u8],
    offset: &mut usize,
    expected_len: u64,
    label: &'static str,
) -> Result<(), BootstrapAnchorError> {
    let enc = read_array_header(bytes, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        ContainerEncoding::Definite(n, _) => Err(BootstrapAnchorError::Structural {
            reason: match (label, n) {
                ("outer", _) => "outer array has wrong field count",
                ("seed_point", _) => "seed_point array has wrong field count",
                _ => "array has wrong field count",
            },
        }),
        ContainerEncoding::Indefinite => Err(BootstrapAnchorError::Structural {
            reason: "indefinite-length array not allowed in BootstrapAnchor",
        }),
    }
}

fn read_u32_field(bytes: &[u8], offset: &mut usize) -> Result<u32, BootstrapAnchorError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    if n > u32::MAX as u64 {
        return Err(BootstrapAnchorError::Structural {
            reason: "u32 field overflowed",
        });
    }
    Ok(n as u32)
}

fn read_u64_field(bytes: &[u8], offset: &mut usize) -> Result<u64, BootstrapAnchorError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    Ok(n)
}

fn read_hash32(bytes: &[u8], offset: &mut usize) -> Result<Hash32, BootstrapAnchorError> {
    let (h, _w) = read_bytes(bytes, offset)?;
    if h.len() != 32 {
        return Err(BootstrapAnchorError::Structural {
            reason: "expected 32-byte hash",
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&h);
    Ok(Hash32(arr))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample() -> BootstrapAnchor {
        BootstrapAnchor {
            network_magic: 1,
            genesis_hash: Hash32([0x11; 32]),
            seed_point: SeedPoint {
                slot: SlotNo(23013663),
                block_hash: Hash32([0x22; 32]),
            },
            seed_artifact_hash: Hash32([0x33; 32]),
            imported_utxo_fingerprint: Hash32([0x44; 32]),
            initial_ledger_fingerprint: Hash32([0x55; 32]),
            seed_provenance: SeedProvenance::CardanoCliJson,
        }
    }

    fn sample_mithril() -> BootstrapAnchor {
        BootstrapAnchor {
            seed_provenance: SeedProvenance::Mithril {
                certificate_hash: Hash32([0x66; 32]),
                certified_point: SeedPoint {
                    slot: SlotNo(23013663),
                    block_hash: Hash32([0x22; 32]),
                },
                immutable_range: (0, 4242),
            },
            ..sample()
        }
    }

    #[test]
    fn bootstrap_anchor_round_trips_via_canonical_cbor() {
        let a = sample();
        let bytes = encode_bootstrap_anchor(&a);
        let decoded = decode_bootstrap_anchor(&bytes).expect("decode");
        assert_eq!(decoded, a);
    }

    #[test]
    fn bootstrap_anchor_encode_two_runs_byte_identical() {
        let a = sample();
        assert_eq!(encode_bootstrap_anchor(&a), encode_bootstrap_anchor(&a));
    }

    #[test]
    fn bootstrap_anchor_decode_rejects_unknown_version() {
        // Craft an outer array with version=99 then splice the rest
        // of a fresh v2 encoding. Outer-array header is 1 byte
        // (0x88 for array(8)); version (=2) is the next 1 byte
        // (0x02). So the payload after the version starts at index 2.
        let mut buf = Vec::new();
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
        );
        write_uint_canonical(&mut buf, 99);
        let fresh = encode_bootstrap_anchor(&sample());
        buf.extend_from_slice(&fresh[2..]);
        match decode_bootstrap_anchor(&buf) {
            Err(BootstrapAnchorError::UnknownVersion { expected: 2, found: 99 }) => {}
            other => panic!("expected UnknownVersion, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_anchor_v2_round_trips_and_rejects_unknown_version() {
        // v2 round-trips for both provenance variants.
        for a in [sample(), sample_mithril()] {
            let bytes = encode_bootstrap_anchor(&a);
            let decoded = decode_bootstrap_anchor(&bytes).expect("v2 decode");
            assert_eq!(decoded, a);
        }
        // A v2 encoding decodes; a v3 (and any other) version is
        // rejected fail-fast.
        let fresh = encode_bootstrap_anchor(&sample());
        assert!(decode_bootstrap_anchor(&fresh).is_ok());
        for bad_version in [0u64, 1, 3, 99] {
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
            );
            write_uint_canonical(&mut buf, bad_version);
            buf.extend_from_slice(&fresh[2..]);
            match decode_bootstrap_anchor(&buf) {
                Err(BootstrapAnchorError::UnknownVersion { expected: 2, found })
                    if found == bad_version as u32 => {}
                other => panic!("expected UnknownVersion for v{bad_version}, got {other:?}"),
            }
        }
    }

    #[test]
    fn bootstrap_anchor_decode_rejects_trailing_bytes() {
        let mut bytes = encode_bootstrap_anchor(&sample());
        bytes.push(0xFF);
        match decode_bootstrap_anchor(&bytes) {
            Err(BootstrapAnchorError::TrailingBytes { extra: 1 }) => {}
            other => panic!("expected TrailingBytes, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_anchor_decode_rejects_short_buffer() {
        let bytes = encode_bootstrap_anchor(&sample());
        for trunc in [0usize, 1, 5, 32, bytes.len() - 1] {
            let res = decode_bootstrap_anchor(&bytes[..trunc]);
            assert!(res.is_err(), "must fail at trunc={trunc}");
        }
    }

    #[test]
    fn bootstrap_anchor_decode_rejects_wrong_outer_array_length() {
        // Hand-craft an array(6) instead of array(7).
        let mut buf = Vec::new();
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(6, canonical_width(6)),
        );
        // Fill with junk that would otherwise decode.
        for _ in 0..6 {
            write_uint_canonical(&mut buf, 0);
        }
        let res = decode_bootstrap_anchor(&buf);
        match res {
            Err(BootstrapAnchorError::Structural { .. }) => {}
            other => panic!("expected Structural, got {other:?}"),
        }
    }

    #[test]
    fn bootstrap_anchor_decode_rejects_short_hash() {
        // Build a valid array(7), version=1, magic=1, then a
        // 31-byte (not 32-byte) genesis_hash.
        let mut buf = Vec::new();
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
        );
        write_uint_canonical(&mut buf, ANCHOR_SCHEMA_VERSION as u64); // version
        write_uint_canonical(&mut buf, 1); // network_magic
        write_bytes_canonical(&mut buf, &[0u8; 31]); // genesis_hash (short)
        // We don't need to fill the rest; decode will hit Structural first.
        let res = decode_bootstrap_anchor(&buf);
        match res {
            Err(BootstrapAnchorError::Structural { .. }) => {}
            other => panic!("expected Structural, got {other:?}"),
        }
    }

    #[test]
    fn seed_point_carries_slot_and_block_hash() {
        // Compile-time exhaustiveness probe — if SeedPoint adds a
        // field, this fails to compile until updated.
        let p = SeedPoint {
            slot: SlotNo(42),
            block_hash: Hash32([0x77; 32]),
        };
        match p {
            SeedPoint {
                slot,
                block_hash,
            } => {
                assert_eq!(slot, SlotNo(42));
                assert_eq!(block_hash.0[0], 0x77);
            }
        }
    }

    #[test]
    fn bootstrap_anchor_match_is_exhaustive() {
        let a = sample();
        let BootstrapAnchor {
            network_magic,
            genesis_hash,
            seed_point,
            seed_artifact_hash,
            imported_utxo_fingerprint,
            initial_ledger_fingerprint,
            seed_provenance,
        } = a;
        assert_eq!(network_magic, 1);
        assert_eq!(genesis_hash.0[0], 0x11);
        assert_eq!(seed_point.slot, SlotNo(23013663));
        assert_eq!(seed_artifact_hash.0[0], 0x33);
        assert_eq!(imported_utxo_fingerprint.0[0], 0x44);
        assert_eq!(initial_ledger_fingerprint.0[0], 0x55);
        assert_eq!(seed_provenance, SeedProvenance::CardanoCliJson);
    }
}
