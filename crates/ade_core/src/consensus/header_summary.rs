// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Header validation inputs and outputs.
//!
//! `HeaderInput` is the structured projection a validator consumes — the
//! caller (GREEN N-A glue or a test driver) has already parsed raw header
//! bytes and surfaced every field this slice's transition needs.
//!
//! `ValidatedHeaderSummary` is the output produced when a header
//! validates; downstream consumers (fork-choice S-B8, rollback S-B9)
//! treat it as a closed canonical projection. The summary records
//! `body_hash` so a later body-admission step can verify
//! header→body binding — this slice never fetches or hashes the body.

use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
use ade_types::{BlockNo, Hash28, Hash32, SlotNo};

/// The VRF payload of a header — closed over the two consensus protocols.
///
/// Shelley..Alonzo run **TPraos**: two role-tagged VRF proofs (nonce-role and
/// leader-role), each verified over its own role-tagged input.
///
/// Babbage and Conway run **Praos**: ONE combined-VRF proof. Both the leader
/// value and the nonce contribution are derived from its single certified
/// output, so the header also carries that output for binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderVrf {
    /// Shelley..Alonzo two-proof model.
    Tpraos {
        nonce_proof: VrfProof,
        leader_proof: VrfProof,
    },
    /// Babbage, Conway single combined-VRF model. `output` is the certified
    /// 64-byte output carried in the header; validation re-verifies the proof
    /// and binds the recomputed output to it.
    Praos { proof: VrfProof, output: VrfOutput },
}

/// KES + operational-certificate material needed to authenticate a Praos
/// header. Absent for TPraos inputs (KES was checked at body admission in the
/// N-B era model); present for Praos so header validation is complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderKes {
    /// Issuer cold verification key (Ed25519, 32 bytes) — signs the op-cert.
    pub issuer_vkey: Vec<u8>,
    /// Hot KES verification key (32 bytes) — the op-cert subject and KES root.
    pub kes_vkey: Vec<u8>,
    /// KES signature bytes (Sum6KES raw, 448 bytes) over the header body.
    pub kes_signature: Vec<u8>,
    /// Operational-certificate cold-key signature (Ed25519, 64 bytes).
    pub op_cert_signature: Vec<u8>,
    /// The exact header-body CBOR bytes the KES signature was produced over.
    pub header_body_bytes: Vec<u8>,
}

/// Everything a validator needs to admit a header. Constructed by the
/// network layer (S-A4 chain-sync) or by a test driver.
///
/// This is NOT the raw on-wire shape — it is a structured projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderInput {
    pub slot: SlotNo,
    pub block_no: BlockNo,
    pub body_hash: Hash32,
    pub issuer_pool: Hash28,
    pub op_cert_kes_period: u64,
    pub op_cert_counter: u64,
    /// VRF verification key registered for this pool in the snapshot
    /// (lookup belongs to the caller via `LedgerView`).
    pub vrf_vk: VrfVerificationKey,
    /// VRF payload — TPraos (two proofs) or Praos (one proof + output).
    pub vrf: HeaderVrf,
    /// KES + op-cert material. Present for Praos headers (fully authenticated
    /// here); `None` for TPraos headers (legacy N-B model).
    pub kes: Option<HeaderKes>,
}

/// The output produced when a header validates — consumed by
/// fork-choice (S-B8) and rollback (S-B9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedHeaderSummary {
    pub slot: SlotNo,
    pub block_no: BlockNo,
    pub body_hash: Hash32,
    pub issuer_pool: Hash28,
    pub op_cert_counter: u64,
    pub vrf_leader_output: VrfOutput,
}
