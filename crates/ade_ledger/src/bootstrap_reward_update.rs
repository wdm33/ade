// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `BootstrapRewardUpdate` type + canonical CBOR codec (B3c / Option B).
//!
//! The bootstrap reward update is the snapshot-bound, manifest-committed replay INPUT that lets a
//! native-Mithril-bootstrapped node reproduce Cardano's epoch-boundary reward distribution at the
//! FIRST post-bootstrap replay-derived authority (the seed+2 leader schedule) — WITHOUT persisting a
//! mutated "post-RUPD seed" as though it existed at the bootstrap point.
//!
//! Cardano's temporal model (the one this preserves): at the epoch boundary the reward update is
//! applied (rewards distributed to reward accounts) and THEN the new stake snapshot is taken. So the
//! "go" snapshot that governs epoch seed+2 reflects the reward distribution applied at the
//! seed→seed+1 boundary, AFTER that epoch's withdrawals. The window driver therefore:
//! ```text
//!   seed ledger at bootstrap point (PRE-update)
//!   → replay canonical blocks through the boundary (incl. withdrawals)
//!   → apply this reward delta exactly ONCE
//!   → aggregate stake / derive leadership authority
//! ```
//! Warm-start reconstructs identically from the persisted PRE-update seed + the canonical replay
//! window + THIS persisted, bound delta — the durable-format cost of replay-equivalence, not a
//! mutated pseudo-state.
//!
//! Closed, version-gated, byte-canonical, commitment-bound (the same discipline as
//! `bootstrap_bridge` / `seed_consensus_inputs`): `encode_bootstrap_reward_update` /
//! `decode_bootstrap_reward_update` are the SOLE pub codec pair. Decode rejects unknown versions,
//! non-canonical or duplicate credential keys, an unknown credential tag, a commitment mismatch,
//! trailing bytes, and any non-byte-canonical encoding (re-encode != input). No `Default`, no
//! `#[non_exhaustive]`: the type system requires every field at construction.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_map_header, read_uint, write_array_header,
    write_bytes_canonical, write_map_header, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::Coin;
use ade_types::{EpochNo, Hash28, Hash32, SlotNo};

/// Pinned wire schema version. Decode rejects any other (fail-closed). v1 = the initial Option-B
/// bootstrap reward-update sidecar.
pub const BOOTSTRAP_RUPD_SCHEMA_VERSION: u32 = 1;

/// Domain separator for the canonical commitment (binds the bytes to THIS artifact + version).
const RUPD_COMMITMENT_DOMAIN: &[u8] = b"ade.b3c.bootstrap-reward-update.v1";

const FIELDS_OUTER: u64 = 7;
const CREDENTIAL_FIELDS: u64 = 2;

/// The credential discriminants, matching the ledger snapshot encoding
/// (`stake_credential_from_ledger_tag`): 0 = key hash, 1 = script hash.
const CRED_TAG_KEYHASH: u64 = 0;
const CRED_TAG_SCRIPTHASH: u64 = 1;

/// Closed record: the manifest-bound bootstrap reward update persisted as a fingerprint-bound
/// sidecar. All fields required at construction; no `Default`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapRewardUpdate {
    /// Binds this delta to the source bootstrap snapshot/manifest (the `BootstrapAnchor`
    /// fingerprint) — the same provenance binding the seed sidecar carries. Warm-start checks
    /// this matches the recovered anchor before consuming the delta.
    pub manifest_commitment: Hash32,
    /// The seed bootstrap point's slot (the Mithril snapshot point, INSIDE the seed epoch). The
    /// delta is the reward update computed up to this point and applied at the seed→seed+1 boundary.
    pub source_point_slot: SlotNo,
    /// The seed bootstrap point's block hash, paired with `source_point_slot`.
    pub source_point_hash: Hash32,
    /// The seed epoch whose END boundary this reward update is applied at: the seed+2 authority
    /// window replays this epoch's blocks, then applies this delta exactly once before the snapshot.
    /// The window driver applies the delta ONLY for the window ending at this boundary.
    pub target_epoch: EpochNo,
    /// The per-credential reward delta (the snapshot's Complete `nesRu` `rs` map, aggregated per
    /// credential). Keys join the dstate reward map by the SAME `StakeCredential` representation.
    /// `BTreeMap` ordering is the sole acceptable map ordering on an authority path.
    pub reward_delta: BTreeMap<StakeCredential, Coin>,
    /// `blake2b_256(domain ‖ body)` over EVERY field above (the manifest binding, the source point,
    /// the target boundary, and the full delta). Binds the durable bytes so warm-start proves
    /// byte-identical reconstruction; a tampered delta or rebinding fails closed.
    pub canonical_commitment: Hash32,
}

/// Closed error sum for `BootstrapRewardUpdate` encode/decode. Carries only non-secret primitives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapRupdError {
    /// CBOR primitive read error or non-byte-canonical encoding.
    MalformedCbor,
    /// Decoded schema version did not match `BOOTSTRAP_RUPD_SCHEMA_VERSION`.
    UnknownVersion { expected: u32, found: u32 },
    /// Decoded buffer did not match the expected closed CBOR shape.
    Structural { reason: &'static str },
    /// Unknown credential tag (not 0 = key hash / 1 = script hash).
    UnknownCredentialTag { found: u64 },
    /// A credential key was not strictly greater than its predecessor (not canonical order).
    NonCanonicalMapOrder,
    /// A credential key was repeated.
    DuplicateCredentialKey,
    /// `canonical_commitment` did not match the recomputed commitment over the body.
    CommitmentMismatch,
    /// Trailing bytes after the record.
    TrailingBytes { extra: usize },
}

/// Compute the canonical commitment over the body (every field EXCEPT the commitment itself).
pub fn bootstrap_rupd_commitment(
    manifest_commitment: &Hash32,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    target_epoch: EpochNo,
    reward_delta: &BTreeMap<StakeCredential, Coin>,
) -> Hash32 {
    let body = encode_rupd_body(
        manifest_commitment,
        source_point_slot,
        source_point_hash,
        target_epoch,
        reward_delta,
    );
    let mut domained = Vec::with_capacity(RUPD_COMMITMENT_DOMAIN.len() + body.len());
    domained.extend_from_slice(RUPD_COMMITMENT_DOMAIN);
    domained.extend_from_slice(&body);
    ade_crypto::blake2b_256(&domained)
}

/// Encode every field EXCEPT the version header + the commitment (the bytes the commitment binds).
fn encode_rupd_body(
    manifest_commitment: &Hash32,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    target_epoch: EpochNo,
    reward_delta: &BTreeMap<StakeCredential, Coin>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    write_bytes_canonical(&mut buf, &manifest_commitment.0);
    write_uint_canonical(&mut buf, source_point_slot.0);
    write_bytes_canonical(&mut buf, &source_point_hash.0);
    write_uint_canonical(&mut buf, target_epoch.0);
    let count = reward_delta.len() as u64;
    write_map_header(&mut buf, ContainerEncoding::Definite(count, canonical_width(count)));
    // `BTreeMap` iteration is in canonical `StakeCredential` order — the credential CBOR encoding
    // (tag 0 < tag 1, then hash bytes) is order-preserving w.r.t. `StakeCredential: Ord`.
    for (cred, coin) in reward_delta {
        write_credential(&mut buf, cred);
        write_uint_canonical(&mut buf, coin.0);
    }
    buf
}

/// Canonical CBOR encode. Sole pub encoder.
///
/// Wire shape (v1):
/// ```text
/// array(7) [
///   uint   BOOTSTRAP_RUPD_SCHEMA_VERSION (= 1),
///   bytes(32) manifest_commitment,
///   uint   source_point_slot,
///   bytes(32) source_point_hash,
///   uint   target_epoch,
///   map(N) { array(2)[uint cred_tag, bytes(28) cred_hash] => uint coin, ... },  // BTreeMap order
///   bytes(32) canonical_commitment,
/// ]
/// ```
pub fn encode_bootstrap_reward_update(rupd: &BootstrapRewardUpdate) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
    );
    write_uint_canonical(&mut buf, BOOTSTRAP_RUPD_SCHEMA_VERSION as u64);
    let body = encode_rupd_body(
        &rupd.manifest_commitment,
        rupd.source_point_slot,
        &rupd.source_point_hash,
        rupd.target_epoch,
        &rupd.reward_delta,
    );
    buf.extend_from_slice(&body);
    write_bytes_canonical(&mut buf, &rupd.canonical_commitment.0);
    buf
}

/// Canonical CBOR decode. Sole pub decoder. Fail-fast on unknown version, wrong shape, short hashes,
/// unknown credential tag, non-canonical or duplicate keys, commitment mismatch, trailing bytes, or
/// any non-byte-canonical encoding (re-encode != input).
pub fn decode_bootstrap_reward_update(
    bytes: &[u8],
) -> Result<BootstrapRewardUpdate, BootstrapRupdError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, FIELDS_OUTER, "outer")?;

    let version = read_u32_field(bytes, &mut o)?;
    if version != BOOTSTRAP_RUPD_SCHEMA_VERSION {
        return Err(BootstrapRupdError::UnknownVersion {
            expected: BOOTSTRAP_RUPD_SCHEMA_VERSION,
            found: version,
        });
    }

    let manifest_commitment = read_hash32(bytes, &mut o)?;
    let source_point_slot = SlotNo(read_u64_field(bytes, &mut o)?);
    let source_point_hash = read_hash32(bytes, &mut o)?;
    let target_epoch = EpochNo(read_u64_field(bytes, &mut o)?);
    let reward_delta = decode_reward_delta(bytes, &mut o)?;
    let canonical_commitment = read_hash32(bytes, &mut o)?;

    if o != bytes.len() {
        return Err(BootstrapRupdError::TrailingBytes {
            extra: bytes.len() - o,
        });
    }

    // Commitment binds the body; recompute and reject a mismatch fail-closed.
    let recomputed = bootstrap_rupd_commitment(
        &manifest_commitment,
        source_point_slot,
        &source_point_hash,
        target_epoch,
        &reward_delta,
    );
    if recomputed != canonical_commitment {
        return Err(BootstrapRupdError::CommitmentMismatch);
    }

    let decoded = BootstrapRewardUpdate {
        manifest_commitment,
        source_point_slot,
        source_point_hash,
        target_epoch,
        reward_delta,
        canonical_commitment,
    };

    // Byte-canonical: reject a structurally valid but non-minimally-encoded buffer.
    if encode_bootstrap_reward_update(&decoded) != bytes {
        return Err(BootstrapRupdError::MalformedCbor);
    }

    Ok(decoded)
}

fn write_credential(buf: &mut Vec<u8>, cred: &StakeCredential) {
    write_array_header(
        buf,
        ContainerEncoding::Definite(CREDENTIAL_FIELDS, canonical_width(CREDENTIAL_FIELDS)),
    );
    let (tag, hash) = match cred {
        StakeCredential::KeyHash(h) => (CRED_TAG_KEYHASH, h),
        StakeCredential::ScriptHash(h) => (CRED_TAG_SCRIPTHASH, h),
    };
    write_uint_canonical(buf, tag);
    write_bytes_canonical(buf, &hash.0);
}

fn read_credential_field(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<StakeCredential, BootstrapRupdError> {
    expect_definite_array(bytes, offset, CREDENTIAL_FIELDS, "credential")?;
    let tag = read_u64_field(bytes, offset)?;
    let hash = read_hash28(bytes, offset)?;
    match tag {
        CRED_TAG_KEYHASH => Ok(StakeCredential::KeyHash(hash)),
        CRED_TAG_SCRIPTHASH => Ok(StakeCredential::ScriptHash(hash)),
        found => Err(BootstrapRupdError::UnknownCredentialTag { found }),
    }
}

fn decode_reward_delta(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<BTreeMap<StakeCredential, Coin>, BootstrapRupdError> {
    let enc = read_map_header(bytes, offset)?;
    let count = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(BootstrapRupdError::Structural {
                reason: "indefinite-length map not allowed in reward_delta",
            })
        }
    };

    let mut delta: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
    let mut prev_key: Option<StakeCredential> = None;
    for _ in 0..count {
        let cred = read_credential_field(bytes, offset)?;
        if let Some(prev) = &prev_key {
            match cred.cmp(prev) {
                std::cmp::Ordering::Greater => {}
                std::cmp::Ordering::Equal => return Err(BootstrapRupdError::DuplicateCredentialKey),
                std::cmp::Ordering::Less => return Err(BootstrapRupdError::NonCanonicalMapOrder),
            }
        }
        let coin = Coin(read_u64_field(bytes, offset)?);
        prev_key = Some(cred.clone());
        delta.insert(cred, coin);
    }
    Ok(delta)
}

fn expect_definite_array(
    bytes: &[u8],
    offset: &mut usize,
    expected_len: u64,
    label: &'static str,
) -> Result<(), BootstrapRupdError> {
    let enc = read_array_header(bytes, offset)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        ContainerEncoding::Definite(_, _) => Err(BootstrapRupdError::Structural {
            reason: match label {
                "outer" => "outer array has wrong field count",
                "credential" => "credential array has wrong field count",
                _ => "array has wrong field count",
            },
        }),
        ContainerEncoding::Indefinite => Err(BootstrapRupdError::Structural {
            reason: "indefinite-length array not allowed in BootstrapRewardUpdate",
        }),
    }
}

fn read_u32_field(bytes: &[u8], offset: &mut usize) -> Result<u32, BootstrapRupdError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    if n > u32::MAX as u64 {
        return Err(BootstrapRupdError::Structural {
            reason: "u32 field overflowed",
        });
    }
    Ok(n as u32)
}

fn read_u64_field(bytes: &[u8], offset: &mut usize) -> Result<u64, BootstrapRupdError> {
    let (n, _w): (u64, IntWidth) = read_uint(bytes, offset)?;
    Ok(n)
}

fn read_hash28(bytes: &[u8], offset: &mut usize) -> Result<Hash28, BootstrapRupdError> {
    let (h, _w) = read_bytes(bytes, offset)?;
    if h.len() != 28 {
        return Err(BootstrapRupdError::Structural {
            reason: "expected 28-byte hash",
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&h);
    Ok(Hash28(arr))
}

fn read_hash32(bytes: &[u8], offset: &mut usize) -> Result<Hash32, BootstrapRupdError> {
    let (h, _w) = read_bytes(bytes, offset)?;
    if h.len() != 32 {
        return Err(BootstrapRupdError::Structural {
            reason: "expected 32-byte hash",
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&h);
    Ok(Hash32(arr))
}

impl From<ade_codec::CodecError> for BootstrapRupdError {
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

    fn key_cred(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }
    fn script_cred(b: u8) -> StakeCredential {
        StakeCredential::ScriptHash(Hash28([b; 28]))
    }

    fn sample() -> BootstrapRewardUpdate {
        let mut reward_delta = BTreeMap::new();
        reward_delta.insert(key_cred(0x01), Coin(1_000));
        reward_delta.insert(key_cred(0x05), Coin(2_500));
        reward_delta.insert(script_cred(0x02), Coin(9_999));
        let manifest_commitment = Hash32([0x44; 32]);
        let source_point_slot = SlotNo(115_676_685);
        let source_point_hash = Hash32([0x66; 32]);
        let target_epoch = EpochNo(1338);
        let canonical_commitment = bootstrap_rupd_commitment(
            &manifest_commitment,
            source_point_slot,
            &source_point_hash,
            target_epoch,
            &reward_delta,
        );
        BootstrapRewardUpdate {
            manifest_commitment,
            source_point_slot,
            source_point_hash,
            target_epoch,
            reward_delta,
            canonical_commitment,
        }
    }

    #[test]
    fn round_trips_byte_identical() {
        let s = sample();
        let bytes = encode_bootstrap_reward_update(&s);
        let decoded = decode_bootstrap_reward_update(&bytes).expect("decode");
        assert_eq!(decoded, s);
        assert_eq!(encode_bootstrap_reward_update(&decoded), bytes);
    }

    #[test]
    fn key_and_script_credentials_order_keyhash_first() {
        // KeyHash(tag 0) sorts before ScriptHash(tag 1) for the same hash bytes — the encoding is
        // order-preserving, so the BTreeMap iteration is the canonical map order.
        let mut delta = BTreeMap::new();
        delta.insert(script_cred(0x01), Coin(2));
        delta.insert(key_cred(0x01), Coin(1));
        let s = BootstrapRewardUpdate {
            reward_delta: delta,
            ..sample_recommitted(BTreeMap::new())
        };
        // recompute commitment for THIS delta
        let s = recommit(s);
        let bytes = encode_bootstrap_reward_update(&s);
        let decoded = decode_bootstrap_reward_update(&bytes).expect("decode");
        let mut it = decoded.reward_delta.keys();
        assert_eq!(it.next(), Some(&key_cred(0x01)), "key hash first");
        assert_eq!(it.next(), Some(&script_cred(0x01)), "script hash second");
    }

    #[test]
    fn decode_rejects_unknown_version() {
        let fresh = encode_bootstrap_reward_update(&sample());
        for bad in [0u64, 2, 99] {
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
            );
            write_uint_canonical(&mut buf, bad);
            buf.extend_from_slice(&fresh[2..]);
            match decode_bootstrap_reward_update(&buf) {
                Err(BootstrapRupdError::UnknownVersion { expected: 1, found })
                    if found == bad as u32 => {}
                other => panic!("expected UnknownVersion for v{bad}, got {other:?}"),
            }
        }
    }

    #[test]
    fn decode_rejects_commitment_mismatch() {
        let mut s = sample();
        s.canonical_commitment = Hash32([0xFF; 32]); // wrong commitment
        let bytes = encode_bootstrap_reward_update(&s);
        match decode_bootstrap_reward_update(&bytes) {
            Err(BootstrapRupdError::CommitmentMismatch) => {}
            other => panic!("expected CommitmentMismatch, got {other:?}"),
        }
    }

    #[test]
    fn tampered_reward_amount_breaks_the_commitment() {
        // Flip one reward amount but keep the stored commitment -> CommitmentMismatch.
        let mut s = sample();
        let bytes_ok = encode_bootstrap_reward_update(&s);
        // mutate the delta; the stored canonical_commitment now no longer matches.
        s.reward_delta.insert(key_cred(0x01), Coin(1_001));
        let bytes_bad = encode_bootstrap_reward_update(&s);
        assert_ne!(bytes_ok, bytes_bad);
        match decode_bootstrap_reward_update(&bytes_bad) {
            Err(BootstrapRupdError::CommitmentMismatch) => {}
            other => panic!("expected CommitmentMismatch, got {other:?}"),
        }
    }

    #[test]
    fn empty_delta_round_trips() {
        let manifest_commitment = Hash32([0x11; 32]);
        let source_point_slot = SlotNo(1);
        let source_point_hash = Hash32([0x22; 32]);
        let target_epoch = EpochNo(7);
        let reward_delta = BTreeMap::new();
        let canonical_commitment = bootstrap_rupd_commitment(
            &manifest_commitment,
            source_point_slot,
            &source_point_hash,
            target_epoch,
            &reward_delta,
        );
        let s = BootstrapRewardUpdate {
            manifest_commitment,
            source_point_slot,
            source_point_hash,
            target_epoch,
            reward_delta,
            canonical_commitment,
        };
        let bytes = encode_bootstrap_reward_update(&s);
        assert_eq!(decode_bootstrap_reward_update(&bytes).expect("decode"), s);
    }

    // helpers for the ordering test
    fn sample_recommitted(delta: BTreeMap<StakeCredential, Coin>) -> BootstrapRewardUpdate {
        let mut s = sample();
        s.reward_delta = delta;
        recommit(s)
    }
    fn recommit(mut s: BootstrapRewardUpdate) -> BootstrapRewardUpdate {
        s.canonical_commitment = bootstrap_rupd_commitment(
            &s.manifest_commitment,
            s.source_point_slot,
            &s.source_point_hash,
            s.target_epoch,
            &s.reward_delta,
        );
        s
    }
}
