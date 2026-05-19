// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Nonce evolution authority for Praos chain-dep state.
//!
//! Three transitions exhaust how Praos chain-dep state nonces evolve:
//!
//! - `HeaderContribution` — a validated header within the current
//!   epoch contributes its nonce-role VRF output into the evolving
//!   nonce via `blake2b256(evolving ‖ vrf_output[0..32])`.
//! - `CandidateFreeze`    — at the canonical 8k/f stability slot, the
//!   evolving nonce is frozen into the candidate nonce.
//! - `EpochBoundary`      — the candidate becomes the epoch nonce; the
//!   prior epoch nonce becomes the previous-epoch nonce; the evolving
//!   nonce is reseeded to the new epoch's candidate.
//!
//! Op-cert counters, `lab_nonce`, `last_block_no` are preserved
//! unchanged by every transition here. Op-cert maintenance lives in
//! S-B5; `lab_nonce` lives in S-B7.

use ade_crypto::blake2b::blake2b_256;
use ade_crypto::vrf::VrfOutput;
use ade_types::{EpochNo, SlotNo};

use crate::consensus::errors::NonceEvolutionError;
use crate::consensus::praos_state::{Nonce, PraosChainDepState};

/// One input to `apply_nonce_input`.
///
/// Closed enum — every authoritative nonce transition is one of these
/// three shapes. Adding a fourth variant would be a registry-level
/// strengthening, not a runtime concern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonceInput {
    /// A validated header within the current epoch contributes its
    /// VRF *nonce-role* output. `slot` must be strictly greater than
    /// `state.last_slot` (if set).
    HeaderContribution {
        slot: SlotNo,
        vrf_output: VrfOutput,
    },
    /// At the canonical Cardano-Praos "8k/f slot" within an epoch, the
    /// evolving nonce is frozen into the candidate nonce. The same
    /// header that triggers the freeze MUST already have been applied
    /// via `HeaderContribution`; this variant is independent of the
    /// header payload.
    CandidateFreeze { at_slot: SlotNo, epoch: EpochNo },
    /// At an epoch boundary, the candidate nonce becomes the new
    /// epoch nonce, the prior epoch nonce becomes
    /// `previous_epoch_nonce`, and `evolving_nonce` is re-seeded from
    /// the (just-promoted) candidate.
    EpochBoundary {
        new_epoch: EpochNo,
        /// epoch number of the just-finished epoch — recorded into
        /// `PraosChainDepState::last_epoch_block`.
        last_block_of_prev_epoch: Option<EpochNo>,
    },
}

/// Deterministic, total nonce evolution.
///
/// The shape is exactly `fn(state, input) -> Result<new_state, error>`
/// — no partial mutation, no ambient state. Every byte of the output
/// is a deterministic function of the input bytes plus the prior
/// state bytes.
pub fn apply_nonce_input(
    state: &PraosChainDepState,
    input: &NonceInput,
) -> Result<PraosChainDepState, NonceEvolutionError> {
    match input {
        NonceInput::HeaderContribution { slot, vrf_output } => {
            apply_header_contribution(state, *slot, vrf_output)
        }
        NonceInput::CandidateFreeze { at_slot: _, epoch: _ } => Ok(apply_candidate_freeze(state)),
        NonceInput::EpochBoundary {
            new_epoch: _,
            last_block_of_prev_epoch,
        } => apply_epoch_boundary(state, *last_block_of_prev_epoch),
    }
}

fn apply_header_contribution(
    state: &PraosChainDepState,
    slot: SlotNo,
    vrf_output: &VrfOutput,
) -> Result<PraosChainDepState, NonceEvolutionError> {
    if let Some(last) = state.last_slot {
        if slot.0 <= last.0 {
            return Err(NonceEvolutionError::SlotBeforeLast {
                last,
                attempted: slot,
            });
        }
    }

    // Fixed 64-byte buffer: 32 evolving bytes ‖ vrf_output[0..32].
    // Order matches ouroboros-consensus: prior evolving on the left,
    // VRF output prefix on the right.
    let mut buf = [0u8; 64];
    buf[0..32].copy_from_slice(state.evolving_nonce.as_bytes());
    buf[32..64].copy_from_slice(&vrf_output.0[0..32]);
    let next = blake2b_256(&buf);

    let mut new_state = state.clone();
    new_state.evolving_nonce = Nonce(next);
    new_state.last_slot = Some(slot);
    Ok(new_state)
}

fn apply_candidate_freeze(state: &PraosChainDepState) -> PraosChainDepState {
    let mut new_state = state.clone();
    new_state.candidate_nonce = state.evolving_nonce.clone();
    new_state
}

fn apply_epoch_boundary(
    state: &PraosChainDepState,
    last_block_of_prev_epoch: Option<EpochNo>,
) -> Result<PraosChainDepState, NonceEvolutionError> {
    if state.epoch_nonce == Nonce::ZERO && state.candidate_nonce == Nonce::ZERO {
        return Err(NonceEvolutionError::UninitialisedEpochNonce);
    }
    let mut new_state = state.clone();
    new_state.previous_epoch_nonce = state.epoch_nonce.clone();
    new_state.epoch_nonce = state.candidate_nonce.clone();
    new_state.evolving_nonce = state.candidate_nonce.clone();
    new_state.last_epoch_block = last_block_of_prev_epoch;
    Ok(new_state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_types::{BlockNo, Hash28, Hash32};

    fn vrf_with(prefix: u8) -> VrfOutput {
        let mut bytes = [0u8; 64];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = prefix.wrapping_add(i as u8);
        }
        VrfOutput(bytes)
    }

    fn state_with_evolving(byte: u8) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.evolving_nonce = Nonce(Hash32([byte; 32]));
        s.epoch_nonce = Nonce(Hash32([0x11; 32]));
        s.previous_epoch_nonce = Nonce(Hash32([0x22; 32]));
        s.lab_nonce = Nonce(Hash32([0x33; 32]));
        s
    }

    #[test]
    fn header_contribution_rejects_non_monotonic_slot() {
        let mut s = state_with_evolving(0x00);
        s.last_slot = Some(SlotNo(100));
        let err = apply_nonce_input(
            &s,
            &NonceInput::HeaderContribution {
                slot: SlotNo(100),
                vrf_output: vrf_with(1),
            },
        );
        assert_eq!(
            err,
            Err(NonceEvolutionError::SlotBeforeLast {
                last: SlotNo(100),
                attempted: SlotNo(100),
            })
        );

        let err2 = apply_nonce_input(
            &s,
            &NonceInput::HeaderContribution {
                slot: SlotNo(99),
                vrf_output: vrf_with(1),
            },
        );
        assert_eq!(
            err2,
            Err(NonceEvolutionError::SlotBeforeLast {
                last: SlotNo(100),
                attempted: SlotNo(99),
            })
        );
    }

    #[test]
    fn header_contribution_advances_evolving_nonce_deterministically() {
        let s = state_with_evolving(0x00);
        let input = NonceInput::HeaderContribution {
            slot: SlotNo(1),
            vrf_output: vrf_with(0x10),
        };
        let a = apply_nonce_input(&s, &input).unwrap();
        let b = apply_nonce_input(&s, &input).unwrap();
        assert_eq!(a, b);
        assert_ne!(a.evolving_nonce, s.evolving_nonce);
        assert_eq!(a.last_slot, Some(SlotNo(1)));
    }

    #[test]
    fn header_contribution_does_not_touch_epoch_nonce() {
        let s = state_with_evolving(0x00);
        let next = apply_nonce_input(
            &s,
            &NonceInput::HeaderContribution {
                slot: SlotNo(1),
                vrf_output: vrf_with(0x10),
            },
        )
        .unwrap();
        assert_eq!(next.epoch_nonce, s.epoch_nonce);
        assert_eq!(next.previous_epoch_nonce, s.previous_epoch_nonce);
        assert_eq!(next.candidate_nonce, s.candidate_nonce);
        assert_eq!(next.lab_nonce, s.lab_nonce);
    }

    #[test]
    fn candidate_freeze_copies_evolving_to_candidate() {
        let s = state_with_evolving(0xAB);
        let next = apply_nonce_input(
            &s,
            &NonceInput::CandidateFreeze {
                at_slot: SlotNo(123),
                epoch: EpochNo(7),
            },
        )
        .unwrap();
        assert_eq!(next.candidate_nonce, s.evolving_nonce);
        assert_eq!(next.evolving_nonce, s.evolving_nonce);
        assert_eq!(next.epoch_nonce, s.epoch_nonce);
        assert_eq!(next.previous_epoch_nonce, s.previous_epoch_nonce);
    }

    #[test]
    fn candidate_freeze_does_not_advance_slot() {
        let mut s = state_with_evolving(0xCD);
        s.last_slot = Some(SlotNo(500));
        let next = apply_nonce_input(
            &s,
            &NonceInput::CandidateFreeze {
                at_slot: SlotNo(999),
                epoch: EpochNo(7),
            },
        )
        .unwrap();
        assert_eq!(next.last_slot, Some(SlotNo(500)));
    }

    #[test]
    fn epoch_boundary_promotes_candidate_to_epoch_nonce() {
        let mut s = state_with_evolving(0x00);
        s.candidate_nonce = Nonce(Hash32([0x44; 32]));
        let next = apply_nonce_input(
            &s,
            &NonceInput::EpochBoundary {
                new_epoch: EpochNo(8),
                last_block_of_prev_epoch: Some(EpochNo(7)),
            },
        )
        .unwrap();
        assert_eq!(next.epoch_nonce, s.candidate_nonce);
        assert_eq!(next.evolving_nonce, s.candidate_nonce);
        assert_eq!(next.last_epoch_block, Some(EpochNo(7)));
    }

    #[test]
    fn epoch_boundary_rotates_previous_epoch_nonce() {
        let mut s = state_with_evolving(0x00);
        s.candidate_nonce = Nonce(Hash32([0x44; 32]));
        let prior_epoch = s.epoch_nonce.clone();
        let next = apply_nonce_input(
            &s,
            &NonceInput::EpochBoundary {
                new_epoch: EpochNo(8),
                last_block_of_prev_epoch: Some(EpochNo(7)),
            },
        )
        .unwrap();
        assert_eq!(next.previous_epoch_nonce, prior_epoch);
    }

    #[test]
    fn epoch_boundary_rejects_uninitialised_candidate() {
        let s = PraosChainDepState::empty();
        let err = apply_nonce_input(
            &s,
            &NonceInput::EpochBoundary {
                new_epoch: EpochNo(1),
                last_block_of_prev_epoch: None,
            },
        );
        assert_eq!(err, Err(NonceEvolutionError::UninitialisedEpochNonce));
    }

    #[test]
    fn epoch_boundary_preserves_op_cert_counters() {
        let mut s = state_with_evolving(0x00);
        s.candidate_nonce = Nonce(Hash32([0x44; 32]));
        s.op_cert_counters
            .upsert_strict(Hash28([0x05; 28]), 4, 9)
            .unwrap();
        s.last_block_no = Some(BlockNo(7_800_000));
        let next = apply_nonce_input(
            &s,
            &NonceInput::EpochBoundary {
                new_epoch: EpochNo(8),
                last_block_of_prev_epoch: Some(EpochNo(7)),
            },
        )
        .unwrap();
        assert_eq!(next.op_cert_counters, s.op_cert_counters);
        assert_eq!(next.last_block_no, s.last_block_no);
    }

    #[test]
    fn epoch_boundary_preserves_lab_nonce() {
        let mut s = state_with_evolving(0x00);
        s.candidate_nonce = Nonce(Hash32([0x44; 32]));
        let prior_lab = s.lab_nonce.clone();
        let next = apply_nonce_input(
            &s,
            &NonceInput::EpochBoundary {
                new_epoch: EpochNo(8),
                last_block_of_prev_epoch: Some(EpochNo(7)),
            },
        )
        .unwrap();
        assert_eq!(next.lab_nonce, prior_lab);
    }

    #[test]
    fn blake2b_input_order_is_evolving_then_vrf_output_first_32() {
        // Pin the byte order: evolving (32) ‖ vrf_output[0..32] (32)
        // -> blake2b_256 = new evolving nonce.
        let evolving_bytes = [0x77u8; 32];
        let mut vrf_bytes = [0u8; 64];
        for (i, b) in vrf_bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut state = PraosChainDepState::empty();
        state.evolving_nonce = Nonce(Hash32(evolving_bytes));

        let next = apply_nonce_input(
            &state,
            &NonceInput::HeaderContribution {
                slot: SlotNo(1),
                vrf_output: VrfOutput(vrf_bytes),
            },
        )
        .unwrap();

        // Independently compute the expected hash.
        let mut combined = [0u8; 64];
        combined[0..32].copy_from_slice(&evolving_bytes);
        combined[32..64].copy_from_slice(&vrf_bytes[0..32]);
        let expected = blake2b_256(&combined);
        assert_eq!(next.evolving_nonce, Nonce(expected));

        // Reversed order would give a different hash — confirms we're
        // not silently using `vrf_output[0..32] ‖ evolving`.
        let mut reversed = [0u8; 64];
        reversed[0..32].copy_from_slice(&vrf_bytes[0..32]);
        reversed[32..64].copy_from_slice(&evolving_bytes);
        let reversed_hash = blake2b_256(&reversed);
        assert_ne!(next.evolving_nonce, Nonce(reversed_hash));
    }
}
