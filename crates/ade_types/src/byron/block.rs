// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::Hash32;

/// Byron Epoch Boundary Block (EBB) — HFC era tag 0.
///
/// EBBs are genesis/epoch boundary blocks that contain stakeholder
/// address lists. They have a simpler structure than regular blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronEbbBlock {
    pub header: ByronEbbHeader,
    /// Body: indefinite-length array of stakeholder address hashes.
    /// Carried as opaque CBOR — not semantically parsed in Phase 1.
    pub body: Vec<u8>,
    /// Extra data (attributes map). Opaque CBOR.
    pub extra: Vec<u8>,
}

/// Header of a Byron EBB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronEbbHeader {
    pub protocol_magic: u32,
    pub prev_hash: Hash32,
    pub body_proof: Hash32,
    /// Epoch index from the consensus data.
    pub epoch: u64,
    /// Chain difficulty from the consensus data (wrapped in array(1) on wire).
    pub chain_difficulty: u64,
    /// Header extra data. Opaque CBOR.
    pub extra_data: Vec<u8>,
}

/// Byron regular block — HFC era tag 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronRegularBlock {
    pub header: ByronRegularHeader,
    /// Body: array(4) [tx_payload, ssc_payload, dlg_payload, upd_payload].
    /// Carried as opaque CBOR — not semantically parsed in Phase 1.
    pub body: Vec<u8>,
    /// Extra data (attributes map). Opaque CBOR.
    pub extra: Vec<u8>,
}

/// Header of a Byron regular block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronRegularHeader {
    pub protocol_magic: u32,
    pub prev_hash: Hash32,
    /// Body proof: 4-element array [tx_proof, ssc_proof, dlg_proof, upd_proof].
    /// Carried as opaque CBOR for Phase 1.
    pub body_proof: Vec<u8>,
    pub consensus_data: ByronConsensusData,
    /// Header extra data. Opaque CBOR.
    pub extra_data: Vec<u8>,
}

/// Consensus data from a Byron regular block header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByronConsensusData {
    pub epoch: u64,
    pub slot_in_epoch: u64,
    /// Delegator public key (64 bytes, Ed25519 extended).
    pub delegator_pubkey: Vec<u8>,
    /// Chain difficulty (wrapped in array(1) on wire).
    pub chain_difficulty: u64,
    /// Block signature. Opaque CBOR.
    pub block_sig: Vec<u8>,
}
