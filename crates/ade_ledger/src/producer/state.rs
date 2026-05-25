// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Canonical producer-tick value (BLUE).
//!
//! `ProducerTick` is the only sanctioned input to `forge_block`. Every
//! input the forge function needs is carried explicitly as a value: no
//! ambient state, no implicit ledger reads, no clock, no rand, no
//! private-key bytes. Replay corpora carry `ProducerTick` values
//! directly; the producer-side RED -> BLUE boundary is one tick.
//!
//! Closure properties enforced mechanically by
//! `ci/ci_check_forge_purity.sh` (guards 4 + 5) and
//! `ci/ci_check_no_private_keys_in_corpus.sh` (guards 2 + 3):
//! - No `#[non_exhaustive]` on this struct.
//! - No private-key fields (`*SigningKey`, `KesSecret`, `ColdSigningKey`).

use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_crypto::ed25519::Ed25519VerificationKey;
use ade_crypto::kes::{KesPeriod, KesSignature};
use ade_crypto::vrf::{VrfOutput, VrfProof};
use ade_types::primitives::SlotNo;
use ade_types::shelley::block::{OperationalCert, ProtocolVersion};
use ade_types::{BlockNo, Hash32};

use crate::mempool::admit::MempoolState;
use crate::pparams::ProtocolParameters;
use crate::state::LedgerState;

/// A canonical producer tick — every input `forge_block` needs as an
/// explicit value. Private-key fields are absent by construction.
#[derive(Debug, Clone, PartialEq)]
pub struct ProducerTick {
    pub slot: SlotNo,
    /// The ledger state the tick was produced against; `forge_block`'s
    /// admit-prefix check rebuilds the mempool snapshot from it.
    pub base_state: LedgerState,
    /// Captured mempool snapshot. Its `accepted()` list is the canonical
    /// accumulating order forge must respect.
    pub mempool: MempoolState,
    /// Ordered preserved-byte tx CBOR slices that produced this mempool
    /// snapshot, one per index in `mempool.accepted()`. Carried
    /// explicitly because `MempoolState` only retains `Hash32` ids —
    /// forge must have the wire bytes to assemble the body buckets.
    pub mempool_tx_bytes: Vec<Vec<u8>>,
    pub pparams: ProtocolParameters,
    /// Leader-schedule context the validator's `is_leader_for_vrf_output`
    /// consumes — produced by `query_leader_schedule` (N-B). Forge does
    /// not derive thresholds itself; it composes this answer with the
    /// supplied VRF output.
    pub leader_answer: LeaderScheduleAnswer,
    pub vrf_proof: VrfProof,
    pub vrf_output: VrfOutput,
    /// Header-encoded VRF verification key (32 bytes).
    pub vrf_vkey: Vec<u8>,
    pub kes_period: KesPeriod,
    pub kes_signature: KesSignature,
    pub opcert: OperationalCert,
    /// The operator's cold verification key — supplied by RED, validated
    /// by BLUE via `opcert_validate`. Doubles as the canonical
    /// `issuer_vkey` for the forged header body (validator computes
    /// `issuer_pool = blake2b_224(issuer_vkey)`).
    pub cold_vk: Ed25519VerificationKey,
    /// Durable per-(cold-key, node) opcert counter; `None` only for the
    /// first opcert this node has ever produced.
    pub prev_opcert_counter: Option<u64>,
    pub block_number: BlockNo,
    pub prev_hash: Hash32,
    pub protocol_version: ProtocolVersion,
}
