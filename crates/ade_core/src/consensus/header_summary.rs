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
    /// VRF nonce-role proof.
    pub vrf_nonce_proof: VrfProof,
    /// VRF leader-role proof.
    pub vrf_leader_proof: VrfProof,
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
