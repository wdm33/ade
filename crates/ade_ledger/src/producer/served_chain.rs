// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// BLUE producer-side served-chain index (PHASE4-N-G S2).
//
// `ServedChainSnapshot` is the single canonical index from which the
// producer-side server reducers (chain-sync `RollForward` headers,
// block-fetch `Block { bytes }` payloads) source their wire bytes.
// The only entry path is `served_chain_admit`, which derives the
// `(slot, hash)` key from the AcceptedBlock bytes via `decode_block` —
// no caller-supplied "asserted hash" can mismatch the bytes.
//
// `CN-CONS-07` is preserved across the network seam: every byte that
// ever leaves this index originated as `AcceptedBlock.as_bytes()`,
// which only `self_accept` returning `Ok(...)` produces.
//
// BTreeMap-backed iteration is the only iteration; no `HashMap` /
// `HashSet` (DC-PROTO-07 transcript determinism foundation).

use std::collections::BTreeMap;

use ade_crypto::blake2b::blake2b_256;
use ade_types::{Hash32, SlotNo};

use crate::block_validity::{decode_block, BlockValidityError};
use crate::producer::AcceptedBlock;

/// Canonical, deterministic, append-only snapshot of `AcceptedBlock`
/// tokens keyed by `(slot, block_hash)`.
///
/// Does not derive `Eq` because `AcceptedBlock` only derives
/// `PartialEq` (its inner `Vec<u8>` derives both, but the surface
/// follows the parent type's choice).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServedChainSnapshot {
    blocks: BTreeMap<(SlotNo, Hash32), AcceptedBlock>,
}

/// Closed admit-error sum. No `String`-bearing variant; no
/// `#[non_exhaustive]`. `Eq` is unavailable because the embedded
/// `BlockValidityError` only derives `PartialEq`.
#[derive(Debug, Clone, PartialEq)]
pub enum ServedChainAdmitError {
    /// `decode_block` rejected the AcceptedBlock bytes. Should never
    /// fire for a real `AcceptedBlock` (the byte path through
    /// `self_accept` already proved decode-validity); the variant
    /// exists for strict totality.
    Decode(BlockValidityError),
    /// Two distinct AcceptedBlock byte sequences resolved to the same
    /// (slot, hash) key. Cryptographically unreachable under
    /// blake2b_256 header hashing; the variant exists to make the
    /// structural invariant explicit.
    KeyByteConflict { slot: SlotNo, hash: Hash32 },
}

impl ServedChainSnapshot {
    /// Build an empty served-chain snapshot.
    pub fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
        }
    }

    /// Number of admitted blocks.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Whether any block has been admitted.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Lookup the bytes admitted at `(slot, hash)`. Returns a slice
    /// of the `AcceptedBlock`'s own bytes — byte-identical to what
    /// `self_accept` saw.
    pub fn block_bytes(&self, slot: SlotNo, hash: &Hash32) -> Option<&[u8]> {
        self.blocks
            .get(&(slot, hash.clone()))
            .map(AcceptedBlock::as_bytes)
    }

    /// Iterate `(slot, hash, bytes)` in BTreeMap order over an
    /// inclusive range of keys.
    pub fn range_bytes(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> impl Iterator<Item = (SlotNo, &'_ Hash32, &'_ [u8])> + '_ {
        self.blocks
            .range(from..=to)
            .map(|((slot, hash), block)| (*slot, hash, block.as_bytes()))
    }

    /// Iterate every admitted block in BTreeMap order.
    pub fn iter(&self) -> impl Iterator<Item = (SlotNo, &'_ Hash32, &'_ [u8])> + '_ {
        self.blocks
            .iter()
            .map(|((slot, hash), block)| (*slot, hash, block.as_bytes()))
    }

    /// Deterministic fingerprint over the snapshot: blake2b_256 of
    /// the concatenated `(slot_be8 || hash || bytes)` triples in
    /// BTreeMap order. Two snapshots admitting the same blocks (in
    /// any admission order) have identical fingerprints — the
    /// replay-equivalence anchor S5's session-transcript replay
    /// uses.
    pub fn fingerprint(&self) -> Hash32 {
        let mut acc = Vec::new();
        for (slot, hash, bytes) in self.iter() {
            acc.extend_from_slice(&slot.0.to_be_bytes());
            acc.extend_from_slice(&hash.0);
            acc.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            acc.extend_from_slice(bytes);
        }
        Hash32(blake2b_256(&acc).0)
    }
}

/// Admit one `AcceptedBlock` into the served chain. Pure, total,
/// deterministic. Idempotent on byte-identity at the same key.
///
/// The `(slot, hash)` key is derived from `block.as_bytes()` via
/// `decode_block` — the caller cannot supply a mismatching key. The
/// only failure modes are (a) `decode_block` rejects the bytes
/// (unreachable for a real AcceptedBlock; the variant exists for
/// strict totality), and (b) two AcceptedBlock byte sequences happen
/// to resolve to the same `(slot, blake2b_256(header))` key —
/// cryptographically infeasible under blake2b_256, but the variant
/// exists to make the invariant explicit.
pub fn served_chain_admit(
    mut served: ServedChainSnapshot,
    block: AcceptedBlock,
) -> Result<ServedChainSnapshot, ServedChainAdmitError> {
    let decoded = decode_block(block.as_bytes()).map_err(ServedChainAdmitError::Decode)?;
    let key = (decoded.header_input.slot, decoded.block_hash);
    if let Some(existing) = served.blocks.get(&key) {
        if existing.as_bytes() != block.as_bytes() {
            return Err(ServedChainAdmitError::KeyByteConflict {
                slot: key.0,
                hash: key.1,
            });
        }
        // Idempotent: same key + same bytes → same snapshot.
        return Ok(served);
    }
    served.blocks.insert(key, block);
    Ok(served)
}
