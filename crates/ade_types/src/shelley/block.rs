// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::Hash32;

/// Post-Byron block structure shared by Shelley through Conway.
///
/// Block is either array(4) (Shelley/Allegra/Mary) or array(5) (Alonzo+).
/// Header body is either array(15) (Shelley-Alonzo) or array(10) (Babbage-Conway).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyBlock {
    pub header: ShelleyHeader,
    /// Number of transactions (from tx_bodies array length).
    pub tx_count: u64,
    /// Transaction bodies sequence. Opaque CBOR.
    pub tx_bodies: Vec<u8>,
    /// Transaction witness sets. Opaque CBOR.
    pub witness_sets: Vec<u8>,
    /// Transaction metadata map. Opaque CBOR.
    pub metadata: Vec<u8>,
    /// Invalid transactions (present in Alonzo+). Absent for Shelley/Allegra/Mary.
    pub invalid_txs: Option<Vec<u8>>,
}

/// Block header: array(2) [header_body, kes_signature].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyHeader {
    pub body: ShelleyHeaderBody,
    /// KES signature. Opaque CBOR bytes.
    pub kes_signature: Vec<u8>,
}

/// Header body fields common across all post-Byron eras.
///
/// For Shelley-Alonzo: array(15) with inlined operational cert + protocol version,
///   and split VRF certs (nonce_vrf + leader_vrf).
/// For Babbage-Conway: array(10) with nested cert + version,
///   and combined vrf_result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyHeaderBody {
    pub block_number: u64,
    pub slot: u64,
    pub prev_hash: Hash32,
    /// Issuer verification key (32 bytes).
    pub issuer_vkey: Vec<u8>,
    /// VRF verification key (32 bytes).
    pub vrf_vkey: Vec<u8>,
    /// VRF data — format varies by era.
    pub vrf: VrfData,
    pub body_size: u64,
    pub body_hash: Hash32,
    pub operational_cert: OperationalCert,
    pub protocol_version: ProtocolVersion,
}

/// VRF data — encoding format varies by era.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VrfData {
    /// Shelley-Alonzo: separate nonce and leader VRF certificates.
    Split {
        nonce_vrf: Vec<u8>,
        leader_vrf: Vec<u8>,
    },
    /// Babbage-Conway: single combined VRF result.
    Combined { vrf_result: Vec<u8> },
}

/// Operational certificate fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalCert {
    /// Hot KES verification key (32 bytes).
    pub hot_vkey: Vec<u8>,
    pub sequence_number: u64,
    pub kes_period: u64,
    /// KES signature. Opaque bytes.
    pub sigma: Vec<u8>,
}

/// Protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u64,
    pub minor: u64,
}
