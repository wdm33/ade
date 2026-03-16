// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::Hash32;

/// Shelley-era block — HFC era tag 2.
///
/// Structure: array(4) [header, tx_bodies, witness_sets, metadata]
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
}

/// Shelley block header: array(2) [header_body, kes_signature].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyHeader {
    pub body: ShelleyHeaderBody,
    /// KES signature. Opaque CBOR bytes.
    pub kes_signature: Vec<u8>,
}

/// Shelley header body: array(15) with inlined operational cert and protocol version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelleyHeaderBody {
    pub block_number: u64,
    pub slot: u64,
    pub prev_hash: Hash32,
    /// Issuer verification key (32 bytes).
    pub issuer_vkey: Vec<u8>,
    /// VRF verification key (32 bytes).
    pub vrf_vkey: Vec<u8>,
    /// Nonce VRF certificate. Opaque CBOR (array(2)).
    pub nonce_vrf: Vec<u8>,
    /// Leader VRF certificate. Opaque CBOR (array(2)).
    pub leader_vrf: Vec<u8>,
    pub body_size: u64,
    pub body_hash: Hash32,
    pub operational_cert: OperationalCert,
    pub protocol_version: ProtocolVersion,
}

/// Operational certificate fields (inlined in header body array).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalCert {
    /// Hot KES verification key (32 bytes).
    pub hot_vkey: Vec<u8>,
    pub sequence_number: u64,
    pub kes_period: u64,
    /// KES signature. Opaque bytes.
    pub sigma: Vec<u8>,
}

/// Protocol version (inlined in header body array).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u64,
    pub minor: u64,
}
