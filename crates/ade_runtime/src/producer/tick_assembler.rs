// Core Contract:
// - Deterministic: same inputs => byte-identical ProducerTick output
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Pure GREEN function: stitches signed artifacts into a canonical
//   ProducerTick, never invokes signing primitives, never reads I/O.

//! GREEN tick assembler (PHASE4-N-C S6).
//!
//! `assemble_tick` is observably deterministic: identical
//! `(slot, base_state, mempool, inputs)` inputs MUST produce
//! byte-identical [`ProducerTick`] values across two replays. Captured
//! RED outputs round-trip; no nondeterminism enters via the tick
//! assembler. Closure properties enforced mechanically by
//! `ci/ci_check_scheduler_closure.sh`.

use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_crypto::ed25519::Ed25519VerificationKey;
use ade_crypto::kes::{KesPeriod, KesSignature};
use ade_crypto::vrf::{vrf_proof_to_hash, VrfProof};
use ade_ledger::mempool::admit::MempoolState;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::producer::state::ProducerTick;
use ade_ledger::state::LedgerState;
use ade_types::primitives::SlotNo;
use ade_types::shelley::block::{OperationalCert, PrevHash, ProtocolVersion};
use ade_types::BlockNo;

/// Closed RED-supplied inputs the assembler stitches into a canonical
/// [`ProducerTick`]. Carries signed artifacts only; no private keys.
#[derive(Debug, Clone, PartialEq)]
pub struct TickInputs {
    pub vrf_proof: VrfProof,
    pub kes_period: KesPeriod,
    pub kes_signature: KesSignature,
    pub opcert: OperationalCert,
    pub cold_vk: Ed25519VerificationKey,
    pub vrf_vkey: Vec<u8>,
    pub leader_answer: LeaderScheduleAnswer,
    pub pparams: ProtocolParameters,
    pub mempool_tx_bytes: Vec<Vec<u8>>,
    pub prev_opcert_counter: Option<u64>,
    pub block_number: BlockNo,
    pub prev_hash: PrevHash,
    pub protocol_version: ProtocolVersion,
}

/// Closed assembler-error sum. No `String`-bearing variants — replay
/// surface stays byte-stable.
#[derive(Debug, Clone, PartialEq)]
pub enum TickAssemblyError {
    /// `vrf_proof_to_hash` failed structurally.
    VrfProofMalformed { detail: &'static str },
    /// `inputs.mempool_tx_bytes.len()` does not match
    /// `mempool.accepted().len()`.
    MempoolWidthMismatch {
        tx_bytes: usize,
        accepted_ids: usize,
    },
}

/// Pure GREEN function: stitch signed artifacts + mempool snapshot into a
/// canonical [`ProducerTick`]. Two identical `(slot, base_state, mempool,
/// inputs)` inputs MUST produce byte-identical outputs (`==`).
pub fn assemble_tick(
    slot: u64,
    base_state: &LedgerState,
    mempool: &MempoolState,
    inputs: &TickInputs,
) -> Result<ProducerTick, TickAssemblyError> {
    if inputs.mempool_tx_bytes.len() != mempool.accepted().len() {
        return Err(TickAssemblyError::MempoolWidthMismatch {
            tx_bytes: inputs.mempool_tx_bytes.len(),
            accepted_ids: mempool.accepted().len(),
        });
    }

    let vrf_output = vrf_proof_to_hash(&inputs.vrf_proof).map_err(|_| {
        TickAssemblyError::VrfProofMalformed {
            detail: "vrf_proof_to_hash rejected the supplied proof bytes",
        }
    })?;

    Ok(ProducerTick {
        slot: SlotNo(slot),
        base_state: base_state.clone(),
        mempool: mempool.clone(),
        mempool_tx_bytes: inputs.mempool_tx_bytes.clone(),
        pparams: inputs.pparams.clone(),
        leader_answer: inputs.leader_answer.clone(),
        vrf_proof: inputs.vrf_proof.clone(),
        vrf_output,
        vrf_vkey: inputs.vrf_vkey.clone(),
        kes_period: inputs.kes_period,
        kes_signature: inputs.kes_signature.clone(),
        opcert: inputs.opcert.clone(),
        cold_vk: inputs.cold_vk.clone(),
        prev_opcert_counter: inputs.prev_opcert_counter,
        block_number: inputs.block_number,
        prev_hash: inputs.prev_hash.clone(),
        protocol_version: inputs.protocol_version,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_core::consensus::vrf_cert::{ActiveSlotsCoeff, ExpectedVrfInput};
    use ade_crypto::kes::SUM6_KES_SIG_LEN;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32};
    use cardano_crypto::vrf::VrfDraft03;

    fn keypair_proof(slot: u64) -> VrfProof {
        let (sk, _vk) = VrfDraft03::keypair_from_seed(&[0xA5; 32]);
        let mut alpha = Vec::with_capacity(16);
        alpha.extend_from_slice(&slot.to_be_bytes());
        alpha.extend_from_slice(b"vrf-input-stub");
        let proof_bytes = VrfDraft03::prove(&sk, &alpha).expect("prove succeeds");
        VrfProof(proof_bytes)
    }

    fn leader_answer() -> LeaderScheduleAnswer {
        LeaderScheduleAnswer {
            slot: SlotNo(100),
            pool: Hash28([0xAA; 28]),
            epoch: EpochNo(0),
            expected_vrf_input: ExpectedVrfInput::Praos([0u8; 32]),
            stake_fraction: (1, 2),
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
        }
    }

    fn base_inputs() -> TickInputs {
        let opcert = OperationalCert {
            hot_vkey: vec![0x11; 32],
            sequence_number: 3,
            kes_period: 42,
            sigma: vec![0x22; 64],
        };
        TickInputs {
            vrf_proof: keypair_proof(100),
            kes_period: KesPeriod(42),
            kes_signature: KesSignature([0u8; SUM6_KES_SIG_LEN]),
            opcert,
            cold_vk: Ed25519VerificationKey::from_bytes(&[0x33; 32]).unwrap(),
            vrf_vkey: vec![0x44; 32],
            leader_answer: leader_answer(),
            pparams: Default::default(),
            mempool_tx_bytes: Vec::new(),
            prev_opcert_counter: None,
            block_number: BlockNo(1),
            prev_hash: PrevHash::Block(Hash32([0u8; 32])),
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
        }
    }

    fn base_state() -> LedgerState {
        LedgerState::new(CardanoEra::Conway)
    }

    #[test]
    fn tick_assembler_deterministic_over_captured_red_outputs() {
        let st = base_state();
        let m = MempoolState::new(base_state());
        let inputs = base_inputs();
        let t1 = assemble_tick(100, &st, &m, &inputs).unwrap();
        let t2 = assemble_tick(100, &st, &m, &inputs).unwrap();
        assert_eq!(t1, t2, "assemble_tick must be observably deterministic");
    }

    #[test]
    fn tick_assembler_rejects_mempool_width_mismatch() {
        let st = base_state();
        let m = MempoolState::new(base_state());
        let mut inputs = base_inputs();
        inputs.mempool_tx_bytes = vec![vec![0x80], vec![0x80], vec![0x80]];
        let err = assemble_tick(100, &st, &m, &inputs).unwrap_err();
        assert_eq!(
            err,
            TickAssemblyError::MempoolWidthMismatch {
                tx_bytes: 3,
                accepted_ids: 0,
            }
        );
    }

    #[test]
    fn tick_assembler_rejects_malformed_vrf_proof() {
        let st = base_state();
        let m = MempoolState::new(base_state());
        let mut inputs = base_inputs();
        // VrfDraft03::proof_to_hash reads bytes[0..32] as the gamma
        // compressed Edwards point. The encoding (y=2, sign bit set in
        // byte 31) names the affine y-coordinate 2 with positive sign;
        // the only valid sign for y=2 is the cleared bit (point (x,2)
        // with x even), so decompression fails with InvalidPoint and
        // the assembler surfaces `VrfProofMalformed`.
        let mut bad = [0u8; 80];
        bad[0] = 0x02;
        bad[31] = 0x80;
        inputs.vrf_proof = VrfProof(bad);
        let err = assemble_tick(100, &st, &m, &inputs).unwrap_err();
        match err {
            TickAssemblyError::VrfProofMalformed { .. } => {}
            other => panic!("expected VrfProofMalformed, got {other:?}"),
        }
    }
}
