// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Nonce evolution authority for Praos chain-dep state (DC-EPOCH-16).
//!
//! Two indivisible transitions exhaust how Praos chain-dep nonces evolve on
//! the live follow path, mirroring ouroboros-consensus
//! `reupdateChainDepState` + the epoch tick:
//!
//! - `HeaderContribution` — one validated followed header. From a single
//!   authoritative input `{slot, prev_block_hash, vrf_nonce_output,
//!   freeze_boundary}` BLUE computes, atomically:
//!     * `evolving'  = evolving ⭒ nonceValue(vrf_nonce_output)`
//!     * `lab'       = Nonce(prev_block_hash)`   (`prevHashToNonce`)
//!     * `candidate' = evolving'` while `slot < freeze_boundary`, else the
//!       candidate is frozen (carried unchanged).
//! - `EpochBoundary` — the epoch tick (first tick into the new epoch):
//!     * `epoch_nonce'            = candidate ⭒ last_epoch_block_nonce`
//!     * `previous_epoch_nonce'   = epoch_nonce`
//!     * `last_epoch_block_nonce' = lab`
//!     * `evolving` and `candidate` carry through UNCHANGED (no reset).
//!
//! `⭒` is `Nonce(blake2b256(a ‖ b))`. The Praos boundary combine carries NO
//! extraEntropy operand (unlike TPraos TICKN). The combine operand
//! `last_epoch_block_nonce` is an explicit optional: an absent operand (a
//! legacy `array(9)` chain-dep store, or a pre-seed state) fails the boundary
//! closed (`MissingLastEpochBlockNonce`) rather than fabricate a nonce. The
//! separable `CandidateFreeze` of the prior model is retired — candidate
//! tracking is folded into the indivisible per-header step so it can never be
//! called out of order or omitted on the follow path.
//!
//! Op-cert counters, `last_block_no`, and the bookkeeping `last_epoch_block`
//! (an `EpochNo`) are carried by the transitions here; op-cert maintenance
//! lives in the op-cert authority.

use ade_crypto::blake2b::blake2b_256;
use ade_crypto::vrf::VrfOutput;
use ade_types::{EpochNo, Hash32, SlotNo};

use crate::consensus::errors::NonceEvolutionError;
use crate::consensus::praos_state::{Nonce, PraosChainDepState};

/// One input to `apply_nonce_input`.
///
/// Closed enum — every authoritative Praos nonce transition on the follow
/// path is one of these two shapes (DC-EPOCH-16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonceInput {
    /// One validated followed header. `slot` must be strictly greater than
    /// `state.last_slot` (if set). From this single input BLUE computes the
    /// evolving, lab, and candidate nonces — the candidate tracks the
    /// evolving nonce until `slot >= freeze_boundary`, then freezes.
    HeaderContribution {
        slot: SlotNo,
        /// The header's previous-block hash; `lab' = Nonce(prev_block_hash)`.
        prev_block_hash: Hash32,
        /// The Praos nonce-role VRF output (the range-extended `vrfNonceValue`
        /// in the high 32 bytes); mixed into the evolving nonce.
        vrf_nonce_output: VrfOutput,
        /// `firstSlotNextEpoch − RandomnessStabilisationWindow`. The candidate
        /// tracks the evolving nonce while `slot < freeze_boundary` and is
        /// frozen at/after it. Computed by the shell from the era geometry +
        /// `RSW = ceil(4k/f)`.
        freeze_boundary: SlotNo,
    },
    /// The epoch tick: promote the frozen candidate into the new epoch nonce
    /// via `candidate ⭒ last_epoch_block_nonce`, rotate `previous_epoch_nonce`
    /// / `last_epoch_block_nonce`, and carry `evolving` and `candidate`
    /// through unchanged.
    EpochBoundary { new_epoch: EpochNo },
}

/// Deterministic, total nonce evolution.
///
/// `fn(state, input) -> Result<new_state, error>` — no partial mutation, no
/// ambient state. Every output byte is a deterministic function of the input
/// bytes plus the prior state bytes.
pub fn apply_nonce_input(
    state: &PraosChainDepState,
    input: &NonceInput,
) -> Result<PraosChainDepState, NonceEvolutionError> {
    match input {
        NonceInput::HeaderContribution {
            slot,
            prev_block_hash,
            vrf_nonce_output,
            freeze_boundary,
        } => apply_header_contribution(
            state,
            *slot,
            prev_block_hash,
            vrf_nonce_output,
            *freeze_boundary,
        ),
        NonceInput::EpochBoundary { new_epoch } => apply_epoch_boundary(state, *new_epoch),
    }
}

/// `⭒` — the Praos nonce combine: `Nonce(blake2b256(a ‖ b))`. The left
/// operand's bytes precede the right operand's bytes.
fn combine(a: &Nonce, b: &Nonce) -> Nonce {
    let mut buf = [0u8; 64];
    buf[0..32].copy_from_slice(a.as_bytes());
    buf[32..64].copy_from_slice(b.as_bytes());
    Nonce(blake2b_256(&buf))
}

/// The 32-byte Praos nonce value carried in the high bytes of the nonce-role
/// VRF output (`header_validate` places `vrfNonceValue` there).
fn nonce_value(vrf_nonce_output: &VrfOutput) -> Nonce {
    let mut b = [0u8; 32];
    b.copy_from_slice(&vrf_nonce_output.0[0..32]);
    Nonce(Hash32(b))
}

fn apply_header_contribution(
    state: &PraosChainDepState,
    slot: SlotNo,
    prev_block_hash: &Hash32,
    vrf_nonce_output: &VrfOutput,
    freeze_boundary: SlotNo,
) -> Result<PraosChainDepState, NonceEvolutionError> {
    if let Some(last) = state.last_slot {
        if slot.0 <= last.0 {
            return Err(NonceEvolutionError::SlotBeforeLast {
                last,
                attempted: slot,
            });
        }
    }

    // evolving' = evolving ⭒ nonceValue(vrf_nonce_output)
    let evolving_next = combine(&state.evolving_nonce, &nonce_value(vrf_nonce_output));
    // lab' = prevHashToNonce(prev_block_hash)
    let lab_next = Nonce(prev_block_hash.clone());
    // The candidate tracks the evolving nonce until the stabilisation window,
    // then freezes (carried unchanged) — the UPDN/reupdateChainDepState rule.
    let candidate_next = if slot.0 < freeze_boundary.0 {
        evolving_next.clone()
    } else {
        state.candidate_nonce.clone()
    };

    let mut new_state = state.clone();
    new_state.evolving_nonce = evolving_next;
    new_state.lab_nonce = lab_next;
    new_state.candidate_nonce = candidate_next;
    new_state.last_slot = Some(slot);
    Ok(new_state)
}

fn apply_epoch_boundary(
    state: &PraosChainDepState,
    new_epoch: EpochNo,
) -> Result<PraosChainDepState, NonceEvolutionError> {
    if state.epoch_nonce == Nonce::ZERO && state.candidate_nonce == Nonce::ZERO {
        return Err(NonceEvolutionError::UninitialisedEpochNonce);
    }
    // Explicit operand presence — never fabricate the combine operand.
    let last_epoch_block_nonce = state
        .last_epoch_block_nonce
        .as_ref()
        .ok_or(NonceEvolutionError::MissingLastEpochBlockNonce)?;

    let mut new_state = state.clone();
    // epoch_nonce' = candidate ⭒ last_epoch_block_nonce  (Praos: no extraEntropy)
    new_state.epoch_nonce = combine(&state.candidate_nonce, last_epoch_block_nonce);
    new_state.previous_epoch_nonce = state.epoch_nonce.clone();
    // Rotate: this epoch's last-applied-block nonce becomes the next boundary's
    // combine operand.
    new_state.last_epoch_block_nonce = Some(state.lab_nonce.clone());
    // Bookkeeping: the just-finished epoch number.
    new_state.last_epoch_block = Some(EpochNo(new_epoch.0.saturating_sub(1)));
    // evolving and candidate carry through UNCHANGED (no reset).
    Ok(new_state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_types::{BlockNo, Hash28};

    fn vrf(prefix: u8) -> VrfOutput {
        let mut b = [0u8; 64];
        for (i, x) in b.iter_mut().enumerate() {
            *x = prefix.wrapping_add(i as u8);
        }
        VrfOutput(b)
    }

    /// A seeded, non-empty, valid-looking chain-dep state (distinct nonces,
    /// `last_epoch_block_nonce` present).
    fn seeded() -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.evolving_nonce = Nonce(Hash32([0x10; 32]));
        s.candidate_nonce = Nonce(Hash32([0x20; 32]));
        s.epoch_nonce = Nonce(Hash32([0x30; 32]));
        s.previous_epoch_nonce = Nonce(Hash32([0x40; 32]));
        s.lab_nonce = Nonce(Hash32([0x50; 32]));
        s.last_epoch_block_nonce = Some(Nonce(Hash32([0x60; 32])));
        s
    }

    fn hc(slot: u64, prev: u8, v: u8, freeze_boundary: u64) -> NonceInput {
        NonceInput::HeaderContribution {
            slot: SlotNo(slot),
            prev_block_hash: Hash32([prev; 32]),
            vrf_nonce_output: vrf(v),
            freeze_boundary: SlotNo(freeze_boundary),
        }
    }

    fn tick() -> NonceInput {
        NonceInput::EpochBoundary {
            new_epoch: EpochNo(8),
        }
    }

    #[test]
    fn header_contribution_rejects_non_monotonic_slot() {
        let mut s = seeded();
        s.last_slot = Some(SlotNo(100));
        assert_eq!(
            apply_nonce_input(&s, &hc(100, 0xAA, 1, 1_000_000)),
            Err(NonceEvolutionError::SlotBeforeLast {
                last: SlotNo(100),
                attempted: SlotNo(100),
            })
        );
        assert_eq!(
            apply_nonce_input(&s, &hc(99, 0xAA, 1, 1_000_000)),
            Err(NonceEvolutionError::SlotBeforeLast {
                last: SlotNo(100),
                attempted: SlotNo(99),
            })
        );
    }

    #[test]
    fn header_contribution_advances_evolving_sets_lab_deterministically() {
        let s = seeded();
        let next = apply_nonce_input(&s, &hc(10, 0xAB, 0x10, 1_000_000)).unwrap();
        assert_ne!(next.evolving_nonce, s.evolving_nonce);
        // lab' = Nonce(prev_block_hash)
        assert_eq!(next.lab_nonce, Nonce(Hash32([0xAB; 32])));
        assert_eq!(next.last_slot, Some(SlotNo(10)));
        let again = apply_nonce_input(&s, &hc(10, 0xAB, 0x10, 1_000_000)).unwrap();
        assert_eq!(next, again);
    }

    #[test]
    fn header_contribution_tracks_candidate_before_freeze_boundary() {
        let s = seeded();
        // slot < freeze_boundary => candidate tracks the new evolving nonce.
        let next = apply_nonce_input(&s, &hc(10, 0xAB, 0x10, 100)).unwrap();
        assert_eq!(next.candidate_nonce, next.evolving_nonce);
        assert_ne!(next.candidate_nonce, s.candidate_nonce);
    }

    #[test]
    fn header_contribution_freezes_candidate_at_freeze_boundary() {
        let s = seeded();
        // slot >= freeze_boundary => candidate frozen; evolving still advances.
        let next = apply_nonce_input(&s, &hc(100, 0xAB, 0x10, 100)).unwrap();
        assert_eq!(next.candidate_nonce, s.candidate_nonce);
        assert_ne!(next.evolving_nonce, s.evolving_nonce);
    }

    #[test]
    fn header_contribution_does_not_touch_epoch_or_operand() {
        let s = seeded();
        let next = apply_nonce_input(&s, &hc(10, 0xAB, 0x10, 1_000_000)).unwrap();
        assert_eq!(next.epoch_nonce, s.epoch_nonce);
        assert_eq!(next.previous_epoch_nonce, s.previous_epoch_nonce);
        assert_eq!(next.last_epoch_block_nonce, s.last_epoch_block_nonce);
    }

    #[test]
    fn epoch_boundary_combines_candidate_with_last_epoch_block_nonce() {
        let s = seeded();
        let leb = s.last_epoch_block_nonce.clone().unwrap();
        let expected = combine(&s.candidate_nonce, &leb);
        let next = apply_nonce_input(&s, &tick()).unwrap();
        assert_eq!(next.epoch_nonce, expected);
    }

    #[test]
    fn epoch_boundary_does_not_reset_evolving_or_candidate() {
        let s = seeded();
        let next = apply_nonce_input(&s, &tick()).unwrap();
        assert_eq!(next.evolving_nonce, s.evolving_nonce);
        assert_eq!(next.candidate_nonce, s.candidate_nonce);
    }

    #[test]
    fn epoch_boundary_rotates_last_epoch_block_nonce_from_lab() {
        let s = seeded();
        let next = apply_nonce_input(&s, &tick()).unwrap();
        assert_eq!(next.last_epoch_block_nonce, Some(s.lab_nonce.clone()));
        // lab itself is preserved across the tick (only headers change it).
        assert_eq!(next.lab_nonce, s.lab_nonce);
    }

    #[test]
    fn epoch_boundary_sets_previous_epoch_nonce() {
        let s = seeded();
        let next = apply_nonce_input(&s, &tick()).unwrap();
        assert_eq!(next.previous_epoch_nonce, s.epoch_nonce);
    }

    #[test]
    fn epoch_boundary_fails_closed_on_missing_operand() {
        let mut s = seeded();
        s.last_epoch_block_nonce = None; // legacy array(9) / pre-seed
        assert_eq!(
            apply_nonce_input(&s, &tick()),
            Err(NonceEvolutionError::MissingLastEpochBlockNonce)
        );
    }

    #[test]
    fn epoch_boundary_rejects_uninitialised() {
        let s = PraosChainDepState::empty(); // all ZERO, operand None
        assert_eq!(
            apply_nonce_input(
                &s,
                &NonceInput::EpochBoundary {
                    new_epoch: EpochNo(1),
                }
            ),
            Err(NonceEvolutionError::UninitialisedEpochNonce)
        );
    }

    #[test]
    fn epoch_boundary_preserves_op_cert_counters_and_block_no() {
        let mut s = seeded();
        s.op_cert_counters
            .upsert_strict(Hash28([0x05; 28]), 4, 9)
            .unwrap();
        s.last_block_no = Some(BlockNo(7_800_000));
        let next = apply_nonce_input(&s, &tick()).unwrap();
        assert_eq!(next.op_cert_counters, s.op_cert_counters);
        assert_eq!(next.last_block_no, s.last_block_no);
    }

    #[test]
    fn combine_is_blake2b_of_left_then_right() {
        let a = Nonce(Hash32([0x11; 32]));
        let b = Nonce(Hash32([0x22; 32]));
        let mut buf = [0u8; 64];
        buf[0..32].copy_from_slice(&[0x11; 32]);
        buf[32..64].copy_from_slice(&[0x22; 32]);
        assert_eq!(combine(&a, &b), Nonce(blake2b_256(&buf)));
        // Order matters: reversed operands hash differently.
        let mut rev = [0u8; 64];
        rev[0..32].copy_from_slice(&[0x22; 32]);
        rev[32..64].copy_from_slice(&[0x11; 32]);
        assert_ne!(combine(&a, &b), Nonce(blake2b_256(&rev)));
    }

    #[test]
    fn evolving_mix_uses_high_32_bytes_of_vrf_output() {
        let mut s = PraosChainDepState::empty();
        s.evolving_nonce = Nonce(Hash32([0x77; 32]));
        let mut vb = [0u8; 64];
        for (i, x) in vb.iter_mut().enumerate() {
            *x = i as u8;
        }
        let next = apply_nonce_input(
            &s,
            &NonceInput::HeaderContribution {
                slot: SlotNo(1),
                prev_block_hash: Hash32([0; 32]),
                vrf_nonce_output: VrfOutput(vb),
                freeze_boundary: SlotNo(1_000_000),
            },
        )
        .unwrap();
        let mut buf = [0u8; 64];
        buf[0..32].copy_from_slice(&[0x77; 32]);
        buf[32..64].copy_from_slice(&vb[0..32]);
        assert_eq!(next.evolving_nonce, Nonce(blake2b_256(&buf)));
    }
}
