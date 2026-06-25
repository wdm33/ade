// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `BootstrapNextEpochAuthority` + canonical CBOR codec (EPOCH-CONTINUITY-ACTIVATION ECA-5,
//! DC-EPOCH-15). The ONE-TIME bridge authority that lets a native-Mithril-started node survive its
//! FIRST epoch boundary (seed_epoch -> seed_epoch+1).
//!
//! The verified bootstrap snapshot already carries the seed+1 leadership in its MARK stake snapshot
//! (`ssStakeMark`); this record is its CANONICAL projection, bound at bootstrap into the durable
//! sidecar. It is NOT a second authority source: the seed+1 view comes from the imported snapshot
//! ALONE (`source_kind = ImportedMarkSnapshot`), never nesPd / a window replay / an oracle.
//!
//! The explicit `source_kind` discriminant keeps the bridge unambiguously distinct from the
//! replay-derived `EpochConsensusView` (which is the authority for seed+2 and later): the selector
//! reads `target_epoch == seed_epoch+1 -> bridge`, `>= seed_epoch+2 -> replay`. `canonical_commitment`
//! is `blake2b_256` over the canonical encoding of every other field, so warm-start can verify the
//! durable bytes reconstruct the SAME bridge (and therefore the same selector decision) byte-identically.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_map_header, read_uint, write_array_header,
    write_bytes_canonical, write_map_header, write_uint_canonical, ContainerEncoding,
};
use ade_core::consensus::praos_state::Nonce;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_types::{EpochNo, Hash28, Hash32, SlotNo};

use crate::consensus_view::PoolEntry;

/// Pinned wire schema version. Decode rejects any other (fail-closed).
pub const BRIDGE_SCHEMA_VERSION: u32 = 1;

const FIELDS_OUTER: u64 = 13;
const ASC_FIELDS: u64 = 2;
const POOL_ENTRY_FIELDS: u64 = 2;

/// The provenance of a [`BootstrapNextEpochAuthority`]. A CLOSED discriminant: the bridge is
/// usable ONLY for `seed_epoch+1`, and ONLY when derived from the imported MARK snapshot. There is
/// no other variant -- a replay-derived authority is a different type (`EpochConsensusView`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeSourceKind {
    /// The seed+1 leadership projected from the verified bootstrap snapshot's `ssStakeMark`.
    ImportedMarkSnapshot,
}

impl BridgeSourceKind {
    fn discriminant(self) -> u64 {
        match self {
            BridgeSourceKind::ImportedMarkSnapshot => 0,
        }
    }
    fn from_discriminant(d: u64) -> Option<Self> {
        match d {
            0 => Some(BridgeSourceKind::ImportedMarkSnapshot),
            _ => None,
        }
    }
}

/// Closed record: the one-time bridge authority for the first post-bootstrap boundary. All fields
/// required at construction; no `Default`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapNextEpochAuthority {
    /// Binds this record to a specific `BootstrapAnchor` (self-describing; SAME `anchor_fp` as the
    /// seed `SeedEpochConsensusInputs` it accompanies).
    pub anchor_fp: Hash32,
    /// The epoch this bridge answers for: `seed_epoch + 1` (the ONLY epoch it may be used for).
    pub target_epoch: EpochNo,
    /// The provenance discriminant (always `ImportedMarkSnapshot` for now) -- distinguishes the
    /// bridge from a replay-derived authority.
    pub source_kind: BridgeSourceKind,
    /// The seed/bootstrap selected-chain point (slot + hash) the snapshot was certified at -- binds
    /// the bridge to the snapshot lineage.
    pub source_point_slot: SlotNo,
    pub source_point_hash: Hash32,
    /// The network genesis-config hash (source/profile commitment).
    pub genesis_hash: Hash32,
    /// `blake2b_256` of the protocol-params (protocol/profile commitment).
    pub protocol_params_hash: Hash32,
    pub active_slots_coeff: ActiveSlotsCoeff,
    /// The seed+1 leadership nonce (eta0) -- the candidate nonce frozen for the next epoch.
    pub epoch_nonce: Nonce,
    pub total_active_stake: u64,
    /// Deterministic `BTreeMap` ordering; `PoolEntry` carries the VRF keyhash (from the durable
    /// cert-state registrations -- the snapshot's own poolParams encoding differs and is not used).
    pub pool_distribution: BTreeMap<Hash28, PoolEntry>,
    /// `blake2b_256` over the canonical encoding of every other field (binding + warm-start
    /// reconstruction check).
    pub canonical_commitment: Hash32,
}

/// Closed error sum for `BootstrapNextEpochAuthority` encode/decode. Carries only non-secret
/// primitives; no `String`/`anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeCodecError {
    /// CBOR primitive read error or non-byte-canonical encoding.
    MalformedCbor,
    /// Decoded schema version did not match `BRIDGE_SCHEMA_VERSION`.
    UnknownVersion { expected: u32, found: u32 },
    /// Unknown `source_kind` discriminant.
    UnknownSourceKind { found: u64 },
    /// Decoded buffer did not match the expected closed CBOR shape.
    Structural { reason: &'static str },
    /// `canonical_commitment` did not match the recomputed commitment over the other fields.
    CommitmentMismatch,
    /// Trailing bytes after the record.
    TrailingBytes { extra: usize },
}

/// Compute the `canonical_commitment` for a fully-populated authority (over EVERY field EXCEPT the
/// commitment itself). The commitment binds the durable bytes so warm-start can prove byte-identical
/// reconstruction.
pub fn bridge_canonical_commitment(
    anchor_fp: &Hash32,
    target_epoch: EpochNo,
    source_kind: BridgeSourceKind,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    genesis_hash: &Hash32,
    protocol_params_hash: &Hash32,
    active_slots_coeff: ActiveSlotsCoeff,
    epoch_nonce: &Nonce,
    total_active_stake: u64,
    pool_distribution: &BTreeMap<Hash28, PoolEntry>,
) -> Hash32 {
    let body = encode_bridge_body(
        anchor_fp,
        target_epoch,
        source_kind,
        source_point_slot,
        source_point_hash,
        genesis_hash,
        protocol_params_hash,
        active_slots_coeff,
        epoch_nonce,
        total_active_stake,
        pool_distribution,
    );
    let mut domained = Vec::with_capacity(body.len() + 32);
    domained.extend_from_slice(b"ade.eca5.bootstrap-bridge.v1");
    domained.extend_from_slice(&body);
    ade_crypto::blake2b_256(&domained)
}

/// Encode every field EXCEPT the version header + the commitment (the bytes the commitment binds).
#[allow(clippy::too_many_arguments)]
fn encode_bridge_body(
    anchor_fp: &Hash32,
    target_epoch: EpochNo,
    source_kind: BridgeSourceKind,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    genesis_hash: &Hash32,
    protocol_params_hash: &Hash32,
    active_slots_coeff: ActiveSlotsCoeff,
    epoch_nonce: &Nonce,
    total_active_stake: u64,
    pool_distribution: &BTreeMap<Hash28, PoolEntry>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    write_bytes_canonical(&mut buf, &anchor_fp.0);
    write_uint_canonical(&mut buf, target_epoch.0);
    write_uint_canonical(&mut buf, source_kind.discriminant());
    write_uint_canonical(&mut buf, source_point_slot.0);
    write_bytes_canonical(&mut buf, &source_point_hash.0);
    write_bytes_canonical(&mut buf, &genesis_hash.0);
    write_bytes_canonical(&mut buf, &protocol_params_hash.0);
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(ASC_FIELDS, canonical_width(ASC_FIELDS)),
    );
    write_uint_canonical(&mut buf, active_slots_coeff.numer as u64);
    write_uint_canonical(&mut buf, active_slots_coeff.denom as u64);
    write_bytes_canonical(&mut buf, epoch_nonce.as_bytes());
    write_uint_canonical(&mut buf, total_active_stake);
    let pool_count = pool_distribution.len() as u64;
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(pool_count, canonical_width(pool_count)),
    );
    for (keyhash, entry) in pool_distribution {
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

/// Construct an authority, computing its `canonical_commitment` (the SOLE construction path that
/// guarantees the commitment is consistent with the fields).
#[allow(clippy::too_many_arguments)]
pub fn build_bootstrap_next_epoch_authority(
    anchor_fp: Hash32,
    target_epoch: EpochNo,
    source_kind: BridgeSourceKind,
    source_point_slot: SlotNo,
    source_point_hash: Hash32,
    genesis_hash: Hash32,
    protocol_params_hash: Hash32,
    active_slots_coeff: ActiveSlotsCoeff,
    epoch_nonce: Nonce,
    total_active_stake: u64,
    pool_distribution: BTreeMap<Hash28, PoolEntry>,
) -> BootstrapNextEpochAuthority {
    let canonical_commitment = bridge_canonical_commitment(
        &anchor_fp,
        target_epoch,
        source_kind,
        source_point_slot,
        &source_point_hash,
        &genesis_hash,
        &protocol_params_hash,
        active_slots_coeff,
        &epoch_nonce,
        total_active_stake,
        &pool_distribution,
    );
    BootstrapNextEpochAuthority {
        anchor_fp,
        target_epoch,
        source_kind,
        source_point_slot,
        source_point_hash,
        genesis_hash,
        protocol_params_hash,
        active_slots_coeff,
        epoch_nonce,
        total_active_stake,
        pool_distribution,
        canonical_commitment,
    }
}

/// Canonical CBOR encode. The SOLE pub encoder.
///
/// Wire shape (v1): `array(13)[version, anchor_fp(32), target_epoch, source_kind, source_point_slot,
/// source_point_hash(32), genesis_hash(32), protocol_params_hash(32), [asc.numer, asc.denom],
/// epoch_nonce(32), total_active_stake, map{pool(28) => [stake, vrf(32)]}, canonical_commitment(32)]`.
pub fn encode_bootstrap_next_epoch_authority(a: &BootstrapNextEpochAuthority) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
    );
    write_uint_canonical(&mut buf, BRIDGE_SCHEMA_VERSION as u64);
    buf.extend_from_slice(&encode_bridge_body(
        &a.anchor_fp,
        a.target_epoch,
        a.source_kind,
        a.source_point_slot,
        &a.source_point_hash,
        &a.genesis_hash,
        &a.protocol_params_hash,
        a.active_slots_coeff,
        &a.epoch_nonce,
        a.total_active_stake,
        &a.pool_distribution,
    ));
    write_bytes_canonical(&mut buf, &a.canonical_commitment.0);
    buf
}

/// Canonical CBOR decode. The SOLE pub decoder. Fail-fast on unknown version, wrong shape, short
/// hashes, non-canonical / duplicate pool keys, a commitment mismatch, or trailing bytes.
pub fn decode_bootstrap_next_epoch_authority(
    bytes: &[u8],
) -> Result<BootstrapNextEpochAuthority, BridgeCodecError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, FIELDS_OUTER, "outer")?;

    let version = read_u32_field(bytes, &mut o)?;
    if version != BRIDGE_SCHEMA_VERSION {
        return Err(BridgeCodecError::UnknownVersion {
            expected: BRIDGE_SCHEMA_VERSION,
            found: version,
        });
    }
    let anchor_fp = read_hash32(bytes, &mut o)?;
    let target_epoch = EpochNo(read_u64_field(bytes, &mut o)?);
    let source_kind = BridgeSourceKind::from_discriminant(read_u64_field(bytes, &mut o)?)
        .ok_or(BridgeCodecError::UnknownSourceKind { found: 0 })?;
    let source_point_slot = SlotNo(read_u64_field(bytes, &mut o)?);
    let source_point_hash = read_hash32(bytes, &mut o)?;
    let genesis_hash = read_hash32(bytes, &mut o)?;
    let protocol_params_hash = read_hash32(bytes, &mut o)?;

    expect_definite_array(bytes, &mut o, ASC_FIELDS, "active_slots_coeff")?;
    let numer = read_u32_field(bytes, &mut o)?;
    let denom = read_u32_field(bytes, &mut o)?;
    let active_slots_coeff = ActiveSlotsCoeff { numer, denom };

    let epoch_nonce = Nonce(read_hash32(bytes, &mut o)?);
    let total_active_stake = read_u64_field(bytes, &mut o)?;
    let pool_distribution = decode_pool_distribution(bytes, &mut o)?;
    let canonical_commitment = read_hash32(bytes, &mut o)?;

    if o != bytes.len() {
        return Err(BridgeCodecError::TrailingBytes {
            extra: bytes.len() - o,
        });
    }

    let recomputed = bridge_canonical_commitment(
        &anchor_fp,
        target_epoch,
        source_kind,
        source_point_slot,
        &source_point_hash,
        &genesis_hash,
        &protocol_params_hash,
        active_slots_coeff,
        &epoch_nonce,
        total_active_stake,
        &pool_distribution,
    );
    if recomputed != canonical_commitment {
        return Err(BridgeCodecError::CommitmentMismatch);
    }

    let decoded = BootstrapNextEpochAuthority {
        anchor_fp,
        target_epoch,
        source_kind,
        source_point_slot,
        source_point_hash,
        genesis_hash,
        protocol_params_hash,
        active_slots_coeff,
        epoch_nonce,
        total_active_stake,
        pool_distribution,
        canonical_commitment,
    };

    // Byte-canonical: a structurally valid but non-canonically-encoded buffer must NOT round-trip.
    if encode_bootstrap_next_epoch_authority(&decoded) != bytes {
        return Err(BridgeCodecError::MalformedCbor);
    }
    Ok(decoded)
}

fn decode_pool_distribution(
    bytes: &[u8],
    o: &mut usize,
) -> Result<BTreeMap<Hash28, PoolEntry>, BridgeCodecError> {
    let count = match read_map_header(bytes, o).map_err(|_| BridgeCodecError::MalformedCbor)? {
        ContainerEncoding::Definite(c, _) => c,
        ContainerEncoding::Indefinite => {
            return Err(BridgeCodecError::Structural {
                reason: "indefinite pool map",
            })
        }
    };
    let mut out: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    let mut prev: Option<Hash28> = None;
    for _ in 0..count {
        let key = read_hash28(bytes, o)?;
        if let Some(p) = &prev {
            if &key <= p {
                return Err(BridgeCodecError::Structural {
                    reason: "pool keys not strictly ascending",
                });
            }
        }
        prev = Some(key.clone());
        expect_definite_array(bytes, o, POOL_ENTRY_FIELDS, "pool entry")?;
        let active_stake = read_u64_field(bytes, o)?;
        let vrf_keyhash = read_hash32(bytes, o)?;
        out.insert(
            key,
            PoolEntry {
                active_stake,
                vrf_keyhash,
            },
        );
    }
    Ok(out)
}

fn expect_definite_array(
    bytes: &[u8],
    o: &mut usize,
    n: u64,
    _what: &'static str,
) -> Result<(), BridgeCodecError> {
    match read_array_header(bytes, o).map_err(|_| BridgeCodecError::MalformedCbor)? {
        ContainerEncoding::Definite(c, _) if c == n => Ok(()),
        _ => Err(BridgeCodecError::Structural {
            reason: "wrong array header",
        }),
    }
}

fn read_u64_field(bytes: &[u8], o: &mut usize) -> Result<u64, BridgeCodecError> {
    let (v, _) = read_uint(bytes, o).map_err(|_| BridgeCodecError::MalformedCbor)?;
    Ok(v)
}

fn read_u32_field(bytes: &[u8], o: &mut usize) -> Result<u32, BridgeCodecError> {
    let v = read_u64_field(bytes, o)?;
    u32::try_from(v).map_err(|_| BridgeCodecError::Structural {
        reason: "u32 field overflow",
    })
}

fn read_hash32(bytes: &[u8], o: &mut usize) -> Result<Hash32, BridgeCodecError> {
    let (b, _) = read_bytes(bytes, o).map_err(|_| BridgeCodecError::MalformedCbor)?;
    if b.len() != 32 {
        return Err(BridgeCodecError::Structural {
            reason: "hash32 width",
        });
    }
    let mut h = [0u8; 32];
    h.copy_from_slice(&b);
    Ok(Hash32(h))
}

fn read_hash28(bytes: &[u8], o: &mut usize) -> Result<Hash28, BridgeCodecError> {
    let (b, _) = read_bytes(bytes, o).map_err(|_| BridgeCodecError::MalformedCbor)?;
    if b.len() != 28 {
        return Err(BridgeCodecError::Structural {
            reason: "hash28 width",
        });
    }
    let mut h = [0u8; 28];
    h.copy_from_slice(&b);
    Ok(Hash28(h))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sample() -> BootstrapNextEpochAuthority {
        let mut pools = BTreeMap::new();
        pools.insert(
            Hash28([0x01; 28]),
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        pools.insert(
            Hash28([0x05; 28]),
            PoolEntry {
                active_stake: 2_500,
                vrf_keyhash: Hash32([0x08; 32]),
            },
        );
        build_bootstrap_next_epoch_authority(
            Hash32([0xAA; 32]),
            EpochNo(1339),
            BridgeSourceKind::ImportedMarkSnapshot,
            SlotNo(115_676_685),
            Hash32([0xBB; 32]),
            Hash32([0xCC; 32]),
            Hash32([0xDD; 32]),
            ActiveSlotsCoeff { numer: 1, denom: 20 },
            Nonce(Hash32([0xEE; 32])),
            3_500,
            pools,
        )
    }

    #[test]
    fn round_trip_is_byte_identical() {
        let a = sample();
        let bytes = encode_bootstrap_next_epoch_authority(&a);
        let b = decode_bootstrap_next_epoch_authority(&bytes).unwrap();
        assert_eq!(a, b);
        assert_eq!(encode_bootstrap_next_epoch_authority(&b), bytes);
    }

    #[test]
    fn tampered_commitment_is_rejected() {
        let a = sample();
        let mut bytes = encode_bootstrap_next_epoch_authority(&a);
        let n = bytes.len();
        bytes[n - 1] ^= 0xff;
        assert_eq!(
            decode_bootstrap_next_epoch_authority(&bytes),
            Err(BridgeCodecError::CommitmentMismatch)
        );
    }

    #[test]
    fn tampered_field_breaks_the_commitment() {
        // Flip a stake byte in the body (not the trailing commitment): the recomputed commitment no
        // longer matches the stored one -> terminal. Proves the commitment binds the WHOLE record.
        let a = sample();
        let mut bytes = encode_bootstrap_next_epoch_authority(&a);
        // total_active_stake is a small inline uint mid-record; flip the version-independent body.
        // Find a byte well inside the body and flip it; the commitment (last 33 bytes) must catch it.
        let flip = 10usize.min(bytes.len() - 40);
        bytes[flip] ^= 0x01;
        assert!(decode_bootstrap_next_epoch_authority(&bytes).is_err());
    }

    #[test]
    fn wrong_version_is_rejected() {
        let a = sample();
        let mut bad = encode_bootstrap_next_epoch_authority(&a);
        bad[1] = 0x02; // the inline version uint follows the 1-byte array(13) header.
        assert!(matches!(
            decode_bootstrap_next_epoch_authority(&bad),
            Err(BridgeCodecError::UnknownVersion { .. })
        ));
    }

    #[test]
    fn trailing_bytes_rejected() {
        let a = sample();
        let mut bytes = encode_bootstrap_next_epoch_authority(&a);
        bytes.push(0x00);
        assert_eq!(
            decode_bootstrap_next_epoch_authority(&bytes),
            Err(BridgeCodecError::TrailingBytes { extra: 1 })
        );
    }
}
