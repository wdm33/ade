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
//! `bootstrap_bridge` / `seed_consensus_inputs`): `encode_seed_reward_authority` /
//! `decode_seed_reward_authority` are the SOLE pub codec pair. The durable record is a closed sum,
//! [`SeedRewardAuthority`] (LV-2, DC-EPOCH-41): either cardano's finished result, imported, or the
//! typed statement that there was none yet and the native calculation owns the seed boundary.
//! Decode rejects unknown versions,
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
/// bootstrap reward-update sidecar (rs only); v2 = adds `delta_treasury` + `delta_reserves` so the
/// seed-boundary apply sets the pots byte-exact (the pre-seed fees it cannot re-derive natively); v3 =
/// adds `delta_fees` (the RewardUpdate's `feeSS` magnitude) so the seed-boundary apply reduces the fee
/// pot by `feeSS` EXACTLY as cardano does when it applies the reward update — before the SNAP freezes
/// the fee pot for the next boundary. Without it the seed epoch's `feeSS` double-counts into the seed+2
/// reward's fee input. v4 (LV-2, DC-EPOCH-41) = the record is a closed SUM,
/// [`SeedRewardAuthority`]: the variant tag follows the version and is inside the commitment. A v3
/// record is refused, never reinterpreted — a v3 record with empty deltas cannot be told apart from
/// a snapshot whose reward update had not been computed yet, and guessing is the defect.
pub const BOOTSTRAP_RUPD_SCHEMA_VERSION: u32 = 4;

/// Domain separator for the canonical commitment (binds the bytes to THIS artifact + version).
const RUPD_COMMITMENT_DOMAIN: &[u8] = b"ade.lv2.seed-reward-authority.v4";

/// The variant discriminants. On the wire directly after the version; inside the commitment.
const VARIANT_IMPORTED: u64 = 0;
const VARIANT_NATIVE: u64 = 1;

/// Outer arity of the `Imported` variant: version, variant, 8 body fields, commitment.
const FIELDS_OUTER: u64 = 11;
/// Outer arity of the `Native` variant: version, variant, 5 body fields, commitment.
const FIELDS_OUTER_NATIVE: u64 = 8;
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
    /// The snapshot `nesRu`'s `deltaT` — the treasury INCREASE this reward update applies (the
    /// pot's tau share plus any unregistered-reward sweep). Added to the treasury at the seed
    /// boundary so the accumulator's pots are byte-exact without re-deriving from pre-seed fees.
    pub delta_treasury: Coin,
    /// The snapshot `nesRu`'s `deltaR` MAGNITUDE — the reserves DECREASE this reward update applies
    /// (the net draw after the undistributed return). Subtracted from reserves at the seed boundary.
    pub delta_reserves: Coin,
    /// The snapshot `nesRu`'s `deltaF` MAGNITUDE — the fee pot DECREASE (`feeSS`) this reward update
    /// applies. Cardano reduces the fee pot by `feeSS` when it APPLIES the reward update (Shelley
    /// `epoch.tex`: "The fee pot will be reduced by feeSS"), THEN the SNAP freezes the fee pot. The
    /// seed-boundary apply subtracts this from `epoch_fees` before rotating it to `prev_epoch_fees`, so
    /// the seed epoch's `feeSS` does not double-count into the seed+2 reward's fee input.
    pub delta_fees: Coin,
    /// The per-credential reward delta (the snapshot's Complete `nesRu` `rs` map, aggregated per
    /// credential). Keys join the dstate reward map by the SAME `StakeCredential` representation.
    /// `BTreeMap` ordering is the sole acceptable map ordering on an authority path.
    pub reward_delta: BTreeMap<StakeCredential, Coin>,
    /// `blake2b_256(domain ‖ body)` over EVERY field above (the manifest binding, the source point,
    /// the target boundary, and the full delta). Binds the durable bytes so warm-start proves
    /// byte-identical reconstruction; a tampered delta or rebinding fails closed.
    pub canonical_commitment: Hash32,
}

/// The seed snapshot's reward update had NOT been computed when the snapshot was taken (cardano
/// `nesRu = SNothing`). The seed→seed+1 boundary's reward is then owed by the NATIVE calculation, run
/// at that boundary from the imported `nesBprev` and this record's `fee_ss`. Carries the same
/// provenance binding as [`BootstrapRewardUpdate`] and its own commitment. No `Default`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSeedReward {
    /// Binds this record to the source bootstrap snapshot/manifest.
    pub manifest_commitment: Hash32,
    /// The seed bootstrap point's slot.
    pub source_point_slot: SlotNo,
    /// The seed bootstrap point's block hash.
    pub source_point_hash: Hash32,
    /// The seed epoch whose END boundary this authority governs.
    pub target_epoch: EpochNo,
    /// The snapshot's `ssFee` — the fees of epoch (seed-1). It is the one input of the seed
    /// boundary's reward that cannot be rebuilt from post-seed blocks, and it is still inside the
    /// imported fee pot, which the seed boundary reduces by exactly this amount.
    pub fee_ss: Coin,
    /// `blake2b_256(domain ‖ variant ‖ body)` over every field above.
    pub canonical_commitment: Hash32,
}

/// DC-EPOCH-41: WHO owns the reward paid at the seed→seed+1 boundary. Exactly one of the two, chosen
/// by a typed fact about the seed snapshot — never by an empty map. An absent reward update is not
/// representable as a zero-valued present one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedRewardAuthority {
    /// cardano finished the calculation before the snapshot (`nesRu = Complete`). Its result is
    /// applied exactly once (DC-EPOCH-18).
    Imported(BootstrapRewardUpdate),
    /// cardano had not started (`nesRu = SNothing`). Ade computes it at the seed boundary.
    Native(NativeSeedReward),
}

impl SeedRewardAuthority {
    /// The seed epoch whose END boundary this authority governs.
    pub fn target_epoch(&self) -> EpochNo {
        match self {
            SeedRewardAuthority::Imported(r) => r.target_epoch,
            SeedRewardAuthority::Native(n) => n.target_epoch,
        }
    }

    /// The manifest binding (the sidecar key).
    pub fn manifest_commitment(&self) -> &Hash32 {
        match self {
            SeedRewardAuthority::Imported(r) => &r.manifest_commitment,
            SeedRewardAuthority::Native(n) => &n.manifest_commitment,
        }
    }
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
    /// The variant tag is neither `Imported` (0) nor `Native` (1).
    UnknownVariant { found: u64 },
}

/// Compute the canonical commitment over the body (every field EXCEPT the commitment itself).
#[allow(clippy::too_many_arguments)]
pub fn bootstrap_rupd_commitment(
    manifest_commitment: &Hash32,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    target_epoch: EpochNo,
    delta_treasury: Coin,
    delta_reserves: Coin,
    delta_fees: Coin,
    reward_delta: &BTreeMap<StakeCredential, Coin>,
) -> Hash32 {
    let body = encode_rupd_body(
        manifest_commitment,
        source_point_slot,
        source_point_hash,
        target_epoch,
        delta_treasury,
        delta_reserves,
        delta_fees,
        reward_delta,
    );
    commit_variant(VARIANT_IMPORTED, &body)
}

/// Compute the canonical commitment of a [`NativeSeedReward`] over every field except the commitment.
pub fn native_seed_reward_commitment(
    manifest_commitment: &Hash32,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    target_epoch: EpochNo,
    fee_ss: Coin,
) -> Hash32 {
    let body = encode_native_body(
        manifest_commitment,
        source_point_slot,
        source_point_hash,
        target_epoch,
        fee_ss,
    );
    commit_variant(VARIANT_NATIVE, &body)
}

/// `blake2b_256(domain ‖ uint(variant) ‖ body)`. The variant is inside the commitment, so a record
/// cannot be re-labelled from one authority to the other without breaking it.
fn commit_variant(variant: u64, body: &[u8]) -> Hash32 {
    let mut domained = Vec::with_capacity(RUPD_COMMITMENT_DOMAIN.len() + 1 + body.len());
    domained.extend_from_slice(RUPD_COMMITMENT_DOMAIN);
    write_uint_canonical(&mut domained, variant);
    domained.extend_from_slice(body);
    ade_crypto::blake2b_256(&domained)
}

fn encode_native_body(
    manifest_commitment: &Hash32,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    target_epoch: EpochNo,
    fee_ss: Coin,
) -> Vec<u8> {
    let mut buf = Vec::new();
    write_bytes_canonical(&mut buf, &manifest_commitment.0);
    write_uint_canonical(&mut buf, source_point_slot.0);
    write_bytes_canonical(&mut buf, &source_point_hash.0);
    write_uint_canonical(&mut buf, target_epoch.0);
    write_uint_canonical(&mut buf, fee_ss.0);
    buf
}

/// Encode every field EXCEPT the version header + the commitment (the bytes the commitment binds).
#[allow(clippy::too_many_arguments)]
fn encode_rupd_body(
    manifest_commitment: &Hash32,
    source_point_slot: SlotNo,
    source_point_hash: &Hash32,
    target_epoch: EpochNo,
    delta_treasury: Coin,
    delta_reserves: Coin,
    delta_fees: Coin,
    reward_delta: &BTreeMap<StakeCredential, Coin>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    write_bytes_canonical(&mut buf, &manifest_commitment.0);
    write_uint_canonical(&mut buf, source_point_slot.0);
    write_bytes_canonical(&mut buf, &source_point_hash.0);
    write_uint_canonical(&mut buf, target_epoch.0);
    write_uint_canonical(&mut buf, delta_treasury.0);
    write_uint_canonical(&mut buf, delta_reserves.0);
    write_uint_canonical(&mut buf, delta_fees.0);
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

/// Canonical CBOR encode of the seed reward authority. With [`decode_seed_reward_authority`], the
/// SOLE pub codec pair for the durable record.
///
/// Wire shape (v4):
/// ```text
/// Imported: array(11) [ uint 4, uint 0, <the 8 Imported body fields below>, bytes(32) commitment ]
/// Native:   array(8)  [ uint 4, uint 1, bytes(32) manifest_commitment, uint source_point_slot,
///                       bytes(32) source_point_hash, uint target_epoch, uint fee_ss,
///                       bytes(32) commitment ]
/// ```
pub fn encode_seed_reward_authority(authority: &SeedRewardAuthority) -> Vec<u8> {
    match authority {
        SeedRewardAuthority::Imported(rupd) => encode_bootstrap_reward_update(rupd),
        SeedRewardAuthority::Native(native) => {
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(
                    FIELDS_OUTER_NATIVE,
                    canonical_width(FIELDS_OUTER_NATIVE),
                ),
            );
            write_uint_canonical(&mut buf, BOOTSTRAP_RUPD_SCHEMA_VERSION as u64);
            write_uint_canonical(&mut buf, VARIANT_NATIVE);
            buf.extend_from_slice(&encode_native_body(
                &native.manifest_commitment,
                native.source_point_slot,
                &native.source_point_hash,
                native.target_epoch,
                native.fee_ss,
            ));
            write_bytes_canonical(&mut buf, &native.canonical_commitment.0);
            buf
        }
    }
}

/// Canonical CBOR decode of the seed reward authority. Fail-fast on an unknown version (checked
/// BEFORE the shape, so a v3 record is refused as `UnknownVersion`), an unknown variant, a wrong
/// shape for the variant, a commitment mismatch, trailing bytes, or any non-byte-canonical encoding.
pub fn decode_seed_reward_authority(
    bytes: &[u8],
) -> Result<SeedRewardAuthority, BootstrapRupdError> {
    let mut o = 0usize;
    let arity = match read_array_header(bytes, &mut o)? {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(BootstrapRupdError::Structural {
                reason: "indefinite-length array not allowed in BootstrapRewardUpdate",
            })
        }
    };
    let version = read_u32_field(bytes, &mut o)?;
    if version != BOOTSTRAP_RUPD_SCHEMA_VERSION {
        return Err(BootstrapRupdError::UnknownVersion {
            expected: BOOTSTRAP_RUPD_SCHEMA_VERSION,
            found: version,
        });
    }
    let variant = read_u64_field(bytes, &mut o)?;
    match variant {
        VARIANT_IMPORTED => {
            if arity != FIELDS_OUTER {
                return Err(BootstrapRupdError::Structural {
                    reason: "outer array has wrong field count",
                });
            }
            decode_imported_fields(bytes, o).map(SeedRewardAuthority::Imported)
        }
        VARIANT_NATIVE => {
            if arity != FIELDS_OUTER_NATIVE {
                return Err(BootstrapRupdError::Structural {
                    reason: "outer array has wrong field count",
                });
            }
            let manifest_commitment = read_hash32(bytes, &mut o)?;
            let source_point_slot = SlotNo(read_u64_field(bytes, &mut o)?);
            let source_point_hash = read_hash32(bytes, &mut o)?;
            let target_epoch = EpochNo(read_u64_field(bytes, &mut o)?);
            let fee_ss = Coin(read_u64_field(bytes, &mut o)?);
            let canonical_commitment = read_hash32(bytes, &mut o)?;
            if o != bytes.len() {
                return Err(BootstrapRupdError::TrailingBytes {
                    extra: bytes.len() - o,
                });
            }
            let recomputed = native_seed_reward_commitment(
                &manifest_commitment,
                source_point_slot,
                &source_point_hash,
                target_epoch,
                fee_ss,
            );
            if recomputed != canonical_commitment {
                return Err(BootstrapRupdError::CommitmentMismatch);
            }
            let decoded = SeedRewardAuthority::Native(NativeSeedReward {
                manifest_commitment,
                source_point_slot,
                source_point_hash,
                target_epoch,
                fee_ss,
                canonical_commitment,
            });
            if encode_seed_reward_authority(&decoded) != bytes {
                return Err(BootstrapRupdError::MalformedCbor);
            }
            Ok(decoded)
        }
        found => Err(BootstrapRupdError::UnknownVariant { found }),
    }
}

/// Canonical CBOR encode of the `Imported` variant's record. Callers outside this module encode
/// through [`encode_seed_reward_authority`].
///
/// Wire shape (v4, `Imported`):
/// ```text
/// array(11) [
///   uint   BOOTSTRAP_RUPD_SCHEMA_VERSION (= 4),
///   uint   0 (the Imported variant),
///   bytes(32) manifest_commitment,
///   uint   source_point_slot,
///   bytes(32) source_point_hash,
///   uint   target_epoch,
///   uint   delta_treasury,
///   uint   delta_reserves,
///   uint   delta_fees,
///   map(N) { array(2)[uint cred_tag, bytes(28) cred_hash] => uint coin, ... },  // BTreeMap order
///   bytes(32) canonical_commitment,
/// ]
/// ```
fn encode_bootstrap_reward_update(rupd: &BootstrapRewardUpdate) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
    );
    write_uint_canonical(&mut buf, BOOTSTRAP_RUPD_SCHEMA_VERSION as u64);
    write_uint_canonical(&mut buf, VARIANT_IMPORTED);
    let body = encode_rupd_body(
        &rupd.manifest_commitment,
        rupd.source_point_slot,
        &rupd.source_point_hash,
        rupd.target_epoch,
        rupd.delta_treasury,
        rupd.delta_reserves,
        rupd.delta_fees,
        &rupd.reward_delta,
    );
    buf.extend_from_slice(&body);
    write_bytes_canonical(&mut buf, &rupd.canonical_commitment.0);
    buf
}

/// Decode the `Imported` variant's fields, starting just past the variant tag at `o`. Fail-fast on
/// short hashes, unknown credential tag, non-canonical or duplicate keys, commitment mismatch,
/// trailing bytes, or any non-byte-canonical encoding (re-encode != input).
fn decode_imported_fields(
    bytes: &[u8],
    mut o: usize,
) -> Result<BootstrapRewardUpdate, BootstrapRupdError> {
    let manifest_commitment = read_hash32(bytes, &mut o)?;
    let source_point_slot = SlotNo(read_u64_field(bytes, &mut o)?);
    let source_point_hash = read_hash32(bytes, &mut o)?;
    let target_epoch = EpochNo(read_u64_field(bytes, &mut o)?);
    let delta_treasury = Coin(read_u64_field(bytes, &mut o)?);
    let delta_reserves = Coin(read_u64_field(bytes, &mut o)?);
    let delta_fees = Coin(read_u64_field(bytes, &mut o)?);
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
        delta_treasury,
        delta_reserves,
        delta_fees,
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
        delta_treasury,
        delta_reserves,
        delta_fees,
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

    /// The pre-LV-2 tests below exercise the `Imported` record. They decode through the one pub
    /// decoder and require the `Imported` variant.
    fn decode_bootstrap_reward_update(
        bytes: &[u8],
    ) -> Result<BootstrapRewardUpdate, BootstrapRupdError> {
        match decode_seed_reward_authority(bytes)? {
            SeedRewardAuthority::Imported(r) => Ok(r),
            SeedRewardAuthority::Native(_) => panic!("expected an Imported record"),
        }
    }

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
        let delta_treasury = Coin(2_744_247_724_989);
        let delta_reserves = Coin(3_108_895_499_648);
        // The confirmed feeSS magnitude of the 1338 judge-snapshot bootstrap RewardUpdate.
        let delta_fees = Coin(1_157_103_223);
        let canonical_commitment = bootstrap_rupd_commitment(
            &manifest_commitment,
            source_point_slot,
            &source_point_hash,
            target_epoch,
            delta_treasury,
            delta_reserves,
            delta_fees,
            &reward_delta,
        );
        BootstrapRewardUpdate {
            manifest_commitment,
            source_point_slot,
            source_point_hash,
            target_epoch,
            delta_treasury,
            delta_reserves,
            delta_fees,
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
        // Every earlier version is rejected on the version gate, v3 included: a v3 record with empty
        // deltas cannot be told apart from "not computed yet" (LV-2).
        for bad in [0u64, 1, 2, 3, 99] {
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(FIELDS_OUTER, canonical_width(FIELDS_OUTER)),
            );
            write_uint_canonical(&mut buf, bad);
            buf.extend_from_slice(&fresh[2..]);
            match decode_bootstrap_reward_update(&buf) {
                Err(BootstrapRupdError::UnknownVersion { expected: 4, found })
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
        let delta_treasury = Coin(0);
        let delta_reserves = Coin(0);
        let delta_fees = Coin(0);
        let reward_delta = BTreeMap::new();
        let canonical_commitment = bootstrap_rupd_commitment(
            &manifest_commitment,
            source_point_slot,
            &source_point_hash,
            target_epoch,
            delta_treasury,
            delta_reserves,
            delta_fees,
            &reward_delta,
        );
        let s = BootstrapRewardUpdate {
            manifest_commitment,
            source_point_slot,
            source_point_hash,
            target_epoch,
            delta_treasury,
            delta_reserves,
            delta_fees,
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
            s.delta_treasury,
            s.delta_reserves,
            s.delta_fees,
            &s.reward_delta,
        );
        s
    }

    #[test]
    fn v3_round_trips_delta_fees() {
        // The v3 codec carries the RewardUpdate feeSS magnitude byte-identically through encode/decode.
        let s = sample();
        assert_eq!(s.delta_fees, Coin(1_157_103_223));
        let bytes = encode_bootstrap_reward_update(&s);
        let decoded = decode_bootstrap_reward_update(&bytes).expect("decode");
        assert_eq!(decoded.delta_fees, Coin(1_157_103_223), "delta_fees survives the round trip");
        assert_eq!(decoded, s);
    }

    #[test]
    fn v3_rejects_genuine_v2_blob() {
        // A genuine v2 wire blob (array(9): the v2 field set, version 2, NO delta_fees) must fail closed
        // — a pre-fix store is never silently reinterpreted; re-bootstrap is the only migration. The
        // version is read before the shape, so it is refused as an unknown version.
        let s = sample();
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(9, canonical_width(9)));
        write_uint_canonical(&mut buf, 2); // version 2
        write_bytes_canonical(&mut buf, &s.manifest_commitment.0);
        write_uint_canonical(&mut buf, s.source_point_slot.0);
        write_bytes_canonical(&mut buf, &s.source_point_hash.0);
        write_uint_canonical(&mut buf, s.target_epoch.0);
        write_uint_canonical(&mut buf, s.delta_treasury.0);
        write_uint_canonical(&mut buf, s.delta_reserves.0);
        // v2 has NO delta_fees field here — straight to the reward-delta map.
        write_map_header(&mut buf, ContainerEncoding::Definite(0, canonical_width(0)));
        write_bytes_canonical(&mut buf, &s.canonical_commitment.0);
        match decode_seed_reward_authority(&buf) {
            Err(BootstrapRupdError::UnknownVersion { expected: 4, found: 2 }) => {}
            other => panic!("expected a fail-closed UnknownVersion for a v2 blob, got {other:?}"),
        }
    }

    #[test]
    fn tampered_delta_fees_breaks_the_commitment() {
        // Flip delta_fees but keep the stored commitment -> CommitmentMismatch: the fee-buffer magnitude
        // is bootstrap-authoritative and commitment-bound.
        let mut s = sample();
        let bytes_ok = encode_bootstrap_reward_update(&s);
        s.delta_fees = Coin(s.delta_fees.0 + 1);
        let bytes_bad = encode_bootstrap_reward_update(&s);
        assert_ne!(bytes_ok, bytes_bad);
        match decode_bootstrap_reward_update(&bytes_bad) {
            Err(BootstrapRupdError::CommitmentMismatch) => {}
            other => panic!("expected CommitmentMismatch, got {other:?}"),
        }
    }

    // ---- LV-2 (DC-EPOCH-41): the record is a closed sum ---------------------------------------

    fn native_sample() -> NativeSeedReward {
        let manifest_commitment = Hash32([0x44; 32]);
        let source_point_slot = SlotNo(129_813_427);
        let source_point_hash = Hash32([0x66; 32]);
        let target_epoch = EpochNo(304);
        // The preprod snapshot's ssFee at the seed (epoch 303's fees).
        let fee_ss = Coin(23_699_540_134);
        NativeSeedReward {
            canonical_commitment: native_seed_reward_commitment(
                &manifest_commitment,
                source_point_slot,
                &source_point_hash,
                target_epoch,
                fee_ss,
            ),
            manifest_commitment,
            source_point_slot,
            source_point_hash,
            target_epoch,
            fee_ss,
        }
    }

    /// CE-LV2-7: both variants round-trip byte-identically and decode back to the SAME variant.
    #[test]
    fn both_variants_round_trip_as_themselves() {
        let imported = SeedRewardAuthority::Imported(sample());
        let native = SeedRewardAuthority::Native(native_sample());
        for authority in [&imported, &native] {
            let bytes = encode_seed_reward_authority(authority);
            let decoded = decode_seed_reward_authority(&bytes).expect("decode");
            assert_eq!(&decoded, authority);
            assert_eq!(encode_seed_reward_authority(&decoded), bytes);
        }
        assert_ne!(
            encode_seed_reward_authority(&imported),
            encode_seed_reward_authority(&native)
        );
        assert_eq!(native.target_epoch(), EpochNo(304));
        assert_eq!(imported.target_epoch(), EpochNo(1338));
    }

    /// CE-LV2-7: a genuine v3 record — array(10), version 3, no variant tag — is refused with the
    /// typed `UnknownVersion`. It is never read as an `Imported` record, empty or otherwise.
    #[test]
    fn a_v3_record_is_refused_not_reinterpreted() {
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(10, canonical_width(10)));
        write_uint_canonical(&mut buf, 3);
        write_bytes_canonical(&mut buf, &[0x11; 32]);
        write_uint_canonical(&mut buf, 129_813_427);
        write_bytes_canonical(&mut buf, &[0x22; 32]);
        write_uint_canonical(&mut buf, 304);
        write_uint_canonical(&mut buf, 0); // delta_treasury
        write_uint_canonical(&mut buf, 0); // delta_reserves
        write_uint_canonical(&mut buf, 0); // delta_fees
        write_map_header(&mut buf, ContainerEncoding::Definite(0, canonical_width(0)));
        write_bytes_canonical(&mut buf, &[0x33; 32]);
        match decode_seed_reward_authority(&buf) {
            Err(BootstrapRupdError::UnknownVersion { expected: 4, found: 3 }) => {}
            other => panic!("expected UnknownVersion for a v3 record, got {other:?}"),
        }
    }

    /// CE-LV2-7: the variant is INSIDE the commitment. The same five binding fields committed as
    /// `Native` do not verify as an empty `Imported` record, and re-labelling the wire tag alone
    /// breaks the record.
    #[test]
    fn the_variant_is_inside_the_commitment() {
        let n = native_sample();
        let as_empty_imported = bootstrap_rupd_commitment(
            &n.manifest_commitment,
            n.source_point_slot,
            &n.source_point_hash,
            n.target_epoch,
            Coin(0),
            Coin(0),
            Coin(0),
            &BTreeMap::new(),
        );
        assert_ne!(n.canonical_commitment, as_empty_imported);

        // Hash the Native body under the OTHER variant tag: it must not equal the real commitment.
        let body = encode_native_body(
            &n.manifest_commitment,
            n.source_point_slot,
            &n.source_point_hash,
            n.target_epoch,
            n.fee_ss,
        );
        assert_eq!(commit_variant(VARIANT_NATIVE, &body), n.canonical_commitment);
        assert_ne!(commit_variant(VARIANT_IMPORTED, &body), n.canonical_commitment);

        // Flip only the wire variant tag (byte 2: header, version, variant) -> not a valid record.
        let mut bytes = encode_seed_reward_authority(&SeedRewardAuthority::Native(n));
        assert_eq!(bytes[2], VARIANT_NATIVE as u8);
        bytes[2] = VARIANT_IMPORTED as u8;
        assert!(decode_seed_reward_authority(&bytes).is_err());
    }

    #[test]
    fn a_tampered_fee_ss_breaks_the_commitment() {
        let mut n = native_sample();
        n.fee_ss = Coin(n.fee_ss.0 - 1);
        match decode_seed_reward_authority(&encode_seed_reward_authority(
            &SeedRewardAuthority::Native(n),
        )) {
            Err(BootstrapRupdError::CommitmentMismatch) => {}
            other => panic!("expected CommitmentMismatch, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_variant_is_refused() {
        let mut bytes = encode_seed_reward_authority(&SeedRewardAuthority::Native(native_sample()));
        bytes[2] = 2;
        match decode_seed_reward_authority(&bytes) {
            Err(BootstrapRupdError::UnknownVariant { found: 2 }) => {}
            other => panic!("expected UnknownVariant, got {other:?}"),
        }
    }
}
