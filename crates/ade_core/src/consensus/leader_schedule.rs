// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Leader schedule query — pure function over
//! `(query, ledger_view, era_schedule, chain_dep_state)` producing
//! either a `LeaderScheduleAnswer` (threshold context for a known pool
//! at a slot within forecast horizon) or a typed `LeaderScheduleError`.
//!
//! The query itself does *not* decide whether a given pool leads a
//! slot — that depends on the actual VRF output bytes, which arrive at
//! header-validation time. The answer instead packages the canonical
//! threshold inputs (`stake_fraction`, `asc`, `expected_vrf_input`) so
//! the caller can call `is_leader_for_vrf_output` once it has a VRF
//! output in hand and get a deterministic decision.

use ade_crypto::vrf::VrfOutput;
use ade_types::{EpochNo, Hash28, SlotNo};

use crate::consensus::era_schedule::EraSchedule;
use crate::consensus::errors::LeaderScheduleError;
use crate::consensus::ledger_view::LedgerView;
use crate::consensus::praos_state::PraosChainDepState;
use crate::consensus::vrf_cert::{
    is_leader, vrf_input, ActiveSlotsCoeff, StakeFraction, VrfRole, VRF_INPUT_LEN,
};

/// Query: "for this `slot`, can `pool` lead?"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderScheduleQuery {
    pub slot: SlotNo,
    pub pool: Hash28,
}

/// Threshold context for one `(slot, pool)` query.
///
/// `leads` is intentionally absent: deciding whether a pool *actually*
/// leads requires the pool's VRF output for `expected_vrf_input`, which
/// arrives at header-validation time. Callers compose the bool
/// themselves via `is_leader_for_vrf_output(&answer, &vrf_output)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderScheduleAnswer {
    pub slot: SlotNo,
    pub pool: Hash28,
    pub epoch: EpochNo,
    /// The 41-byte VRF input `[slot_be (8) ‖ epoch_nonce (32) ‖ tag=LEADER (1)]`
    /// the pool's VRF key must produce a proof over to be valid.
    pub expected_vrf_input: [u8; VRF_INPUT_LEN],
    /// Pool active stake / total active stake, as canonical lovelace
    /// numerator and denominator. Caller guarantees denom > 0.
    pub stake_fraction: (u64, u64),
    /// Active-slots-coefficient surfaced through the ledger view at
    /// query time. Pinned into the answer so consumers reuse a single
    /// canonical `f` without re-asking the ledger.
    pub asc: ActiveSlotsCoeff,
}

/// Pure leader-schedule query.
///
/// Algorithm:
/// 1. `era_schedule.check_forecast_horizon` — past-horizon queries
///    are refused deterministically with `OutsideForecastRange`.
/// 2. `era_schedule.locate` — slot to `(era, epoch)` mapping;
///    `HFCError` is wrapped into `LeaderScheduleError::HFC`.
/// 3. The ledger view is consulted for `(pool_vrf_key, pool_active_stake,
///    total_active_stake, active_slots_coeff)`. Any missing piece is a
///    typed `UnknownPool` failure — N-B never guesses.
/// 4. The canonical 41-byte VRF input is built via
///    `vrf_cert::vrf_input(slot, epoch_nonce, LeaderEligibility)`.
///
/// No mutation: `state` is borrowed immutably and the function returns
/// a fresh `LeaderScheduleAnswer`. Replay equivalence is guaranteed by
/// construction since every input is a pure value.
pub fn query_leader_schedule(
    query: &LeaderScheduleQuery,
    ledger_view: &dyn LedgerView,
    era_schedule: &EraSchedule,
    state: &PraosChainDepState,
) -> Result<LeaderScheduleAnswer, LeaderScheduleError> {
    era_schedule
        .check_forecast_horizon(query.slot)
        .map_err(LeaderScheduleError::OutsideForecastRange)?;

    let location = era_schedule
        .locate(query.slot)
        .map_err(LeaderScheduleError::HFC)?;
    let epoch = location.epoch;

    // VRF key is the strongest "do we know this pool?" signal; resolve
    // it first so callers fast-fail uniformly on unknown pools.
    if ledger_view.pool_vrf_key(epoch, &query.pool).is_none() {
        return Err(LeaderScheduleError::UnknownPool);
    }
    let pool_stake = ledger_view
        .pool_active_stake(epoch, &query.pool)
        .ok_or(LeaderScheduleError::UnknownPool)?;
    let total_stake = ledger_view
        .total_active_stake(epoch)
        .ok_or(LeaderScheduleError::UnknownPool)?;
    if total_stake == 0 {
        return Err(LeaderScheduleError::UnknownPool);
    }
    let asc = ledger_view
        .active_slots_coeff(epoch)
        .ok_or(LeaderScheduleError::UnknownPool)?;

    let alpha = vrf_input(query.slot, &state.epoch_nonce, VrfRole::LeaderEligibility);

    Ok(LeaderScheduleAnswer {
        slot: query.slot,
        pool: query.pool.clone(),
        epoch,
        expected_vrf_input: alpha,
        stake_fraction: (pool_stake, total_stake),
        asc,
    })
}

/// Compose the final per-VRF-output leadership decision.
///
/// Delegates to `vrf_cert::is_leader` using the threshold context that
/// `query_leader_schedule` produced. Pure — no I/O, no allocation.
pub fn is_leader_for_vrf_output(answer: &LeaderScheduleAnswer, output: &VrfOutput) -> bool {
    let sigma = StakeFraction {
        numer: answer.stake_fraction.0,
        denom: answer.stake_fraction.1,
    };
    is_leader(output, sigma, answer.asc)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_crypto::vrf::VrfVerificationKey;
    use ade_types::{CardanoEra, Hash32};

    use crate::consensus::era_schedule::{BootstrapAnchorHash, EraSummary};
    use crate::consensus::errors::OutsideForecastRange;
    use crate::consensus::praos_state::Nonce;

    struct TestLedger {
        epoch: EpochNo,
        pool_known: Hash28,
        vk: VrfVerificationKey,
        pool_stake: u64,
        total_stake: u64,
        asc: ActiveSlotsCoeff,
    }

    impl LedgerView for TestLedger {
        fn total_active_stake(&self, epoch: EpochNo) -> Option<u64> {
            (epoch == self.epoch).then_some(self.total_stake)
        }
        fn pool_active_stake(&self, epoch: EpochNo, pool: &Hash28) -> Option<u64> {
            (epoch == self.epoch && pool == &self.pool_known).then_some(self.pool_stake)
        }
        fn pool_vrf_key(&self, epoch: EpochNo, pool: &Hash28) -> Option<VrfVerificationKey> {
            (epoch == self.epoch && pool == &self.pool_known).then(|| self.vk.clone())
        }
        fn active_slots_coeff(&self, epoch: EpochNo) -> Option<ActiveSlotsCoeff> {
            (epoch == self.epoch).then_some(self.asc)
        }
    }

    fn shelley_only_schedule() -> EraSchedule {
        let eras = vec![EraSummary {
            era: CardanoEra::Shelley,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }];
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
            .unwrap_or_else(|_| unreachable!("well-formed"))
    }

    fn pool() -> Hash28 {
        Hash28([0xAA; 28])
    }

    fn vk() -> VrfVerificationKey {
        VrfVerificationKey([0xBB; 32])
    }

    fn state_with_nonce(byte: u8) -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32([byte; 32]));
        s
    }

    fn ledger(asc: ActiveSlotsCoeff) -> TestLedger {
        TestLedger {
            epoch: EpochNo(0),
            pool_known: pool(),
            vk: vk(),
            pool_stake: 1_000,
            total_stake: 10_000,
            asc,
        }
    }

    #[test]
    fn query_uses_state_epoch_nonce_for_vrf_input() {
        let schedule = shelley_only_schedule();
        let state = state_with_nonce(0xCD);
        let view = ledger(ActiveSlotsCoeff { numer: 1, denom: 20 });
        let answer = query_leader_schedule(
            &LeaderScheduleQuery {
                slot: SlotNo(42),
                pool: pool(),
            },
            &view,
            &schedule,
            &state,
        )
        .unwrap();
        // Bytes 8..40 must mirror the epoch_nonce that lives in `state`;
        // anything else would mean we re-derived the nonce from another
        // source (forbidden by DC-CONS-04 / DC-CONSENSUS-02).
        assert_eq!(&answer.expected_vrf_input[8..40], &[0xCD; 32]);
        // Slot prefix is big-endian.
        assert_eq!(&answer.expected_vrf_input[0..8], &42u64.to_be_bytes());
        // LEADER tag.
        assert_eq!(answer.expected_vrf_input[40], 0x4C);
    }

    #[test]
    fn query_returns_unknown_pool_when_no_vrf_key() {
        let schedule = shelley_only_schedule();
        let state = state_with_nonce(0);
        let view = ledger(ActiveSlotsCoeff { numer: 1, denom: 20 });
        let res = query_leader_schedule(
            &LeaderScheduleQuery {
                slot: SlotNo(0),
                pool: Hash28([0xFF; 28]),
            },
            &view,
            &schedule,
            &state,
        );
        assert_eq!(res, Err(LeaderScheduleError::UnknownPool));
    }

    #[test]
    fn query_returns_outside_forecast_range_for_far_future() {
        let schedule = shelley_only_schedule();
        let state = state_with_nonce(0);
        let view = ledger(ActiveSlotsCoeff { numer: 1, denom: 20 });
        let beyond = SlotNo(u64::MAX);
        let res = query_leader_schedule(
            &LeaderScheduleQuery {
                slot: beyond,
                pool: pool(),
            },
            &view,
            &schedule,
            &state,
        );
        assert_eq!(
            res,
            Err(LeaderScheduleError::OutsideForecastRange(
                OutsideForecastRange {
                    requested: beyond,
                    horizon: SlotNo(129_600),
                }
            ))
        );
    }

    #[test]
    fn query_does_not_mutate_state() {
        // Compile-time guaranteed by `&PraosChainDepState`. We still
        // assert observed equality to detect any future signature drift.
        let schedule = shelley_only_schedule();
        let state = state_with_nonce(0x77);
        let snapshot = state.clone();
        let view = ledger(ActiveSlotsCoeff { numer: 1, denom: 20 });
        let _ = query_leader_schedule(
            &LeaderScheduleQuery {
                slot: SlotNo(1),
                pool: pool(),
            },
            &view,
            &schedule,
            &state,
        );
        assert_eq!(state, snapshot);
    }

    #[test]
    fn is_leader_for_vrf_output_delegates_to_vrf_cert() {
        // asc.numer == asc.denom: vrf_cert::is_leader returns true
        // regardless of output (every slot eligible). This proves the
        // helper threads asc + sigma through to the underlying function.
        let answer = LeaderScheduleAnswer {
            slot: SlotNo(0),
            pool: pool(),
            epoch: EpochNo(0),
            expected_vrf_input: [0u8; VRF_INPUT_LEN],
            stake_fraction: (1, 2),
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
        };
        let any_output = VrfOutput([0xFF; 64]);
        assert!(is_leader_for_vrf_output(&answer, &any_output));

        // asc.numer == 0: never leads.
        let zero = LeaderScheduleAnswer {
            asc: ActiveSlotsCoeff { numer: 0, denom: 1 },
            ..answer.clone()
        };
        assert!(!is_leader_for_vrf_output(&zero, &any_output));

        // sigma.numer == 0: never leads regardless of output.
        let no_stake = LeaderScheduleAnswer {
            stake_fraction: (0, 1),
            asc: ActiveSlotsCoeff { numer: 1, denom: 2 },
            ..answer
        };
        assert!(!is_leader_for_vrf_output(&no_stake, &any_output));
    }
}
