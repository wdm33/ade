// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE SeedEpochConsensusInputs type + canonical CBOR codec (PHASE4-N-F-A A1).
//!
//! `SeedEpochConsensusInputs` is the closed, version-gated, byte-canonical
//! record of the seed-epoch consensus inputs established during verified
//! bootstrap (PoolDistr, ASC, per-pool VRF keyhash, total active stake) for
//! the single seed `epoch_no`. eta0 stays in `chain_dep`; `epoch_no` ties the
//! two together. It carries `anchor_fp` so the record is self-describing and
//! bound to a specific `BootstrapAnchor` (storage shape: fingerprint-bound
//! sidecar — Option A; the anchor is NOT bumped and does NOT embed this).
//!
//! CN-CINPUT-01 (candidate): this module's
//! `encode_seed_epoch_consensus_inputs` / `decode_seed_epoch_consensus_inputs`
//! is the SOLE pub fn pair encoding/decoding `SeedEpochConsensusInputs`. No
//! `Default` impl and no `#[non_exhaustive]`: the type system requires all
//! fields at construction. `SEED_CINPUT_SCHEMA_VERSION = 1` is written into the
//! encoded form; decode rejects unknown versions fail-fast, rejects
//! non-canonical or duplicated pool-map keys, rejects trailing bytes, and
//! verifies byte-canonical round-trip (re-encode == input).

use std::collections::BTreeMap;

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_map_header, read_uint, write_array_header,
    write_bytes_canonical, write_map_header, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_core::consensus::praos_state::Nonce;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_types::{EpochNo, Hash28, Hash32};

use crate::consensus_view::PoolEntry;

/// Pinned wire schema version. Decode rejects any other (fail-closed: an old v1
/// sidecar omitted `epoch_nonce`, so it decodes as `UnknownVersion` — never a
/// default-to-zero eta0). Bumped 1 -> 2 by PHASE4-N-F-G-N (T-REC-04 / DC-CINPUT-03).
pub const SEED_CINPUT_SCHEMA_VERSION: u32 = 2;

const FIELDS_OUTER: u64 = 7;
const ASC_FIELDS: u64 = 2;
const POOL_ENTRY_FIELDS: u64 = 2;

/// Closed record: the seed-epoch consensus inputs persisted as a
/// fingerprint-bound sidecar. All fields required at construction; no
/// `Default`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedEpochConsensusInputs {
    /// Binds this record to a specific `BootstrapAnchor` (self-describing).
    pub anchor_fp: Hash32,
    /// The single seed epoch these inputs are valid for.
    pub epoch_no: EpochNo,
    /// Praos epoch nonce (eta0) for `epoch_no`. Added by PHASE4-N-F-G-N so the
    /// recovered consensus inputs carry eta0 EXPLICITLY (T-REC-04); the forge VRF
    /// input is `praos_vrf_input(slot, epoch_nonce)`, and WarmStart overlays this
    /// onto `chain_dep.epoch_nonce` (DC-CINPUT-03).
    pub epoch_nonce: Nonce,
    pub active_slots_coeff: ActiveSlotsCoeff,
    pub total_active_stake: u64,
    /// Deterministic `BTreeMap` ordering; `PoolEntry` carries the VRF keyhash,
    /// so there is no separate `pool_vrf_keyhashes` map.
    pub pool_distribution: BTreeMap<Hash28, PoolEntry>,
}

/// Closed error sum for `SeedEpochConsensusInputs` encode/decode. Carries only
/// non-secret primitives; no `String`/`anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedConsensusInputsError {
    /// CBOR primitive read error or non-byte-canonical encoding.
    MalformedCbor,
    /// Decoded schema version did not match `SEED_CINPUT_SCHEMA_VERSION`.
    UnknownVersion { expected: u32, found: u32 },
    /// Decoded buffer did not match the expected closed CBOR shape
    /// (wrong array/map header, wrong hash byte width, field overflow).
    Structural { reason: &'static str },
    /// A pool-map key was not strictly greater than its predecessor
    /// (the map is not in canonical `BTreeMap` key order).
    NonCanonicalMapOrder,
    /// A pool-map key was repeated.
    DuplicatePoolKey,
    /// Trailing bytes after the expected record structure.
    TrailingBytes { extra: usize },
}

/// Canonical CBOR encode. CN-CINPUT-01: sole pub encoder.
///
/// Wire shape:
/// ```text
/// array(6) [
///   uint   SEED_CINPUT_SCHEMA_VERSION (= 1),
///   bytes(32) anchor_fp,
///   uint   epoch_no,
///   array(2) [ uint asc.numer, uint asc.denom ],
///   uint   total_active_stake,
///   map(K) {                                  // K = pool count, BTreeMap order
///     bytes(28) pool_keyhash =>
///       array(2) [ uint active_stake, bytes(32) vrf_keyhash ],
///     ...
///   },
/// ]
/// ```
pub fn encode_seed_epoch_consensus_inputs(inputs: &SeedEpochConsensusInputs) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
    );
    write_uint_canonical(&mut buf, SEED_CINPUT_SCHEMA_VERSION as u64);
    write_bytes_canonical(&mut buf, &inputs.anchor_fp.0);
    write_uint_canonical(&mut buf, inputs.epoch_no.0);
    write_bytes_canonical(&mut buf, inputs.epoch_nonce.as_bytes());

    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(ASC_FIELDS, canonical_width(ASC_FIELDS)),
    );
    write_uint_canonical(&mut buf, inputs.active_slots_coeff.numer as u64);
    write_uint_canonical(&mut buf, inputs.active_slots_coeff.denom as u64);

    write_uint_canonical(&mut buf, inputs.total_active_stake);

    let pool_count = inputs.pool_distribution.len() as u64;
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(pool_count, canonical_width(pool_count)),
    );
    // `BTreeMap` iteration is in canonical key order — the sole acceptable
    // map ordering on an authority path.
    for (keyhash, entry) in &inputs.pool_distribution {
        write_bytes_canonical(&mut buf, &keyhash.0);
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(POOL_ENTRY_FIELDS, canonical_width(POOL_ENTRY_FIELDS)),
        );
        write_uint_canonical(&mut buf, entry.active_stake);
        write_bytes_canonical(&mut buf, &entry.vrf_keyhash.0);
    }
    buf
}

/// Canonical CBOR decode. CN-CINPUT-01: sole pub decoder. Fail-fast on
/// unknown schema version, wrong array/map shape, short hash bytes,
/// non-canonical or duplicate pool keys, trailing bytes, or any
/// non-byte-canonical encoding (re-encode != input).
pub fn decode_seed_epoch_consensus_inputs(
    bytes: &[u8],
) -> Result<SeedEpochConsensusInputs, SeedConsensusInputsError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, FIELDS_OUTER, "outer")?;

    let version = read_u32_field(bytes, &mut o)?;
    if version != SEED_CINPUT_SCHEMA_VERSION {
        return Err(SeedConsensusInputsError::UnknownVersion {
            expected: SEED_CINPUT_SCHEMA_VERSION,
            found: version,
        });
    }

    let anchor_fp = read_hash32(bytes, &mut o)?;
    let epoch_no = EpochNo(read_u64_field(bytes, &mut o)?);
    let epoch_nonce = Nonce(read_hash32(bytes, &mut o)?);

    expect_definite_array(bytes, &mut o, ASC_FIELDS, "active_slots_coeff")?;
    let numer = read_u32_field(bytes, &mut o)?;
    let denom = read_u32_field(bytes, &mut o)?;
    let active_slots_coeff = ActiveSlotsCoeff { numer, denom };

    let total_active_stake = read_u64_field(bytes, &mut o)?;

    let pool_distribution = decode_pool_distribution(bytes, &mut o)?;

    if o != bytes.len() {
        return Err(SeedConsensusInputsError::TrailingBytes {
            extra: bytes.len() - o,
        });
    }

    let decoded = SeedEpochConsensusInputs {
        anchor_fp,
        epoch_no,
        epoch_nonce,
        active_slots_coeff,
        total_active_stake,
        pool_distribution,
    };

    // Byte-canonical: a structurally valid but non-canonically-encoded buffer
    // (e.g. a uint written wider than minimal) decodes to the same value but
    // re-encodes to different bytes. Reject it fail-closed.
    if encode_seed_epoch_consensus_inputs(&decoded) != bytes {
        return Err(SeedConsensusInputsError::MalformedCbor);
    }

    Ok(decoded)
}

fn decode_pool_distribution(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<BTreeMap<Hash28, PoolEntry>, SeedConsensusInputsError> {
    let enc = read_map_header(bytes, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(SeedConsensusInputsError::Structural {
                reason: "indefinite-length map not allowed in pool_distribution",
            })
        }
    };

    let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    let mut prev_key: Option<Hash28> = None;
    for _ in 0..count {
        let keyhash = read_hash28(bytes, offset)?;
        if let Some(prev) = &prev_key {
            match keyhash.0.cmp(&prev.0) {
                std::cmp::Ordering::Greater => {}
                std::cmp::Ordering::Equal => {
                    return Err(SeedConsensusInputsError::DuplicatePoolKey)
                }
                std::cmp::Ordering::Less => {
                    return Err(SeedConsensusInputsError::NonCanonicalMapOrder)
                }
            }
        }

        expect_definite_array(bytes, offset, POOL_ENTRY_FIELDS, "pool_entry")?;
        let active_stake = read_u64_field(bytes, offset)?;
        let vrf_keyhash = read_hash32(bytes, offset)?;

        prev_key = Some(keyhash.clone());
        pools.insert(
            keyhash,
            PoolEntry {
                active_stake,
                vrf_keyhash,
            },
        );
    }
    Ok(pools)
}

fn expect_definite_array(
    bytes: &[u8],
    offset: &mut usize,
    expected_len: u64,
    label: &'static str,
) -> Result<(), SeedConsensusInputsError> {
    let enc = read_array_header(bytes, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        ContainerEncoding::Definite(_, _) => Err(SeedConsensusInputsError::Structural {
            reason: match label {
                "outer" => "outer array has wrong field count",
                "active_slots_coeff" => "active_slots_coeff array has wrong field count",
                "pool_entry" => "pool_entry array has wrong field count",
                _ => "array has wrong field count",
            },
        }),
        ContainerEncoding::Indefinite => Err(SeedConsensusInputsError::Structural {
            reason: "indefinite-length array not allowed in SeedEpochConsensusInputs",
        }),
    }
}

fn read_u32_field(bytes: &[u8], offset: &mut usize) -> Result<u32, SeedConsensusInputsError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    if n > u32::MAX as u64 {
        return Err(SeedConsensusInputsError::Structural {
            reason: "u32 field overflowed",
        });
    }
    Ok(n as u32)
}

fn read_u64_field(bytes: &[u8], offset: &mut usize) -> Result<u64, SeedConsensusInputsError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    Ok(n)
}

fn read_hash28(bytes: &[u8], offset: &mut usize) -> Result<Hash28, SeedConsensusInputsError> {
    let (h, _w) = read_bytes(bytes, offset)?;
    if h.len() != 28 {
        return Err(SeedConsensusInputsError::Structural {
            reason: "expected 28-byte hash",
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&h);
    Ok(Hash28(arr))
}

fn read_hash32(bytes: &[u8], offset: &mut usize) -> Result<Hash32, SeedConsensusInputsError> {
    let (h, _w) = read_bytes(bytes, offset)?;
    if h.len() != 32 {
        return Err(SeedConsensusInputsError::Structural {
            reason: "expected 32-byte hash",
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&h);
    Ok(Hash32(arr))
}

impl From<ade_codec::CodecError> for SeedConsensusInputsError {
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

    fn pool_key(b: u8) -> Hash28 {
        Hash28([b; 28])
    }

    fn entry(stake: u64, vrf: u8) -> PoolEntry {
        PoolEntry {
            active_stake: stake,
            vrf_keyhash: Hash32([vrf; 32]),
        }
    }

    fn sample() -> SeedEpochConsensusInputs {
        let mut pool_distribution = BTreeMap::new();
        pool_distribution.insert(pool_key(0x01), entry(1_000, 0x07));
        pool_distribution.insert(pool_key(0x05), entry(2_500, 0x08));
        pool_distribution.insert(pool_key(0xAA), entry(999_999, 0x09));
        SeedEpochConsensusInputs {
            anchor_fp: Hash32([0x44; 32]),
            epoch_no: EpochNo(576),
            epoch_nonce: Nonce(Hash32([0x55; 32])),
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 20 },
            total_active_stake: 1_003_499,
            pool_distribution,
        }
    }

    #[test]
    fn seed_epoch_consensus_inputs_round_trips_byte_identical() {
        let s = sample();
        let bytes = encode_seed_epoch_consensus_inputs(&s);
        let decoded = decode_seed_epoch_consensus_inputs(&bytes).expect("decode");
        assert_eq!(decoded, s);
        // encode -> decode -> encode = identical bytes.
        let reencoded = encode_seed_epoch_consensus_inputs(&decoded);
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn seed_cinput_decode_rejects_unknown_version() {
        // Splice a bad version header in front of an otherwise-valid body.
        // The outer array header is 1 byte (0x87 for array(7)); the version
        // (=2) is the next 1 byte (0x02). So the body after the version starts
        // at index 2. PHASE4-N-F-G-N: the current version is 2; bad_version=1 is
        // the old v1 sidecar (which omitted epoch_nonce) — proving it fails
        // closed (UnknownVersion), never a default-to-zero eta0.
        let fresh = encode_seed_epoch_consensus_inputs(&sample());
        for bad_version in [0u64, 1, 3, 99] {
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
            );
            write_uint_canonical(&mut buf, bad_version);
            buf.extend_from_slice(&fresh[2..]);
            match decode_seed_epoch_consensus_inputs(&buf) {
                Err(SeedConsensusInputsError::UnknownVersion { expected: 2, found })
                    if found == bad_version as u32 => {}
                other => panic!("expected UnknownVersion for v{bad_version}, got {other:?}"),
            }
        }
    }

    #[test]
    fn seed_cinput_encoding_is_btreemap_ordered() {
        // Two records with the SAME pools inserted in OPPOSITE orders must
        // encode to identical bytes — `BTreeMap` ordering is deterministic and
        // insertion-order-independent.
        let mut ascending = BTreeMap::new();
        ascending.insert(pool_key(0x01), entry(1_000, 0x07));
        ascending.insert(pool_key(0x05), entry(2_500, 0x08));
        ascending.insert(pool_key(0xAA), entry(999_999, 0x09));

        let mut descending = BTreeMap::new();
        descending.insert(pool_key(0xAA), entry(999_999, 0x09));
        descending.insert(pool_key(0x05), entry(2_500, 0x08));
        descending.insert(pool_key(0x01), entry(1_000, 0x07));

        let a = SeedEpochConsensusInputs {
            pool_distribution: ascending,
            ..sample()
        };
        let b = SeedEpochConsensusInputs {
            pool_distribution: descending,
            ..sample()
        };
        assert_eq!(
            encode_seed_epoch_consensus_inputs(&a),
            encode_seed_epoch_consensus_inputs(&b),
        );
    }

    #[test]
    fn seed_cinput_decode_rejects_noncanonical_or_duplicate_keys() {
        // Hand-build a two-entry map whose keys are out of canonical order
        // (0x05 before 0x01) -> NonCanonicalMapOrder.
        let out_of_order = build_with_pool_map(&[
            (pool_key(0x05), entry(2_500, 0x08)),
            (pool_key(0x01), entry(1_000, 0x07)),
        ]);
        match decode_seed_epoch_consensus_inputs(&out_of_order) {
            Err(SeedConsensusInputsError::NonCanonicalMapOrder) => {}
            other => panic!("expected NonCanonicalMapOrder, got {other:?}"),
        }

        // Hand-build a two-entry map with a repeated key -> DuplicatePoolKey.
        let duplicate = build_with_pool_map(&[
            (pool_key(0x01), entry(1_000, 0x07)),
            (pool_key(0x01), entry(2_500, 0x08)),
        ]);
        match decode_seed_epoch_consensus_inputs(&duplicate) {
            Err(SeedConsensusInputsError::DuplicatePoolKey) => {}
            other => panic!("expected DuplicatePoolKey, got {other:?}"),
        }
    }

    /// Encode a record whose pool map is written verbatim from `entries` (in
    /// the given order, with `entries.len()` as the map count) so a test can
    /// craft out-of-order / duplicate-key maps the encoder would never emit.
    fn build_with_pool_map(entries: &[(Hash28, PoolEntry)]) -> Vec<u8> {
        let s = sample();
        let mut buf = Vec::new();
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
        );
        write_uint_canonical(&mut buf, SEED_CINPUT_SCHEMA_VERSION as u64);
        write_bytes_canonical(&mut buf, &s.anchor_fp.0);
        write_uint_canonical(&mut buf, s.epoch_no.0);
        write_bytes_canonical(&mut buf, s.epoch_nonce.as_bytes());
        write_array_header(
            &mut buf,
            ContainerEncoding::Definite(ASC_FIELDS, canonical_width(ASC_FIELDS)),
        );
        write_uint_canonical(&mut buf, s.active_slots_coeff.numer as u64);
        write_uint_canonical(&mut buf, s.active_slots_coeff.denom as u64);
        write_uint_canonical(&mut buf, s.total_active_stake);
        let count = entries.len() as u64;
        write_map_header(
            &mut buf,
            ContainerEncoding::Definite(count, canonical_width(count)),
        );
        for (keyhash, e) in entries {
            write_bytes_canonical(&mut buf, &keyhash.0);
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(POOL_ENTRY_FIELDS, canonical_width(POOL_ENTRY_FIELDS)),
            );
            write_uint_canonical(&mut buf, e.active_stake);
            write_bytes_canonical(&mut buf, &e.vrf_keyhash.0);
        }
        buf
    }
}
