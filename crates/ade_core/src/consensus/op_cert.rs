// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Op-cert counter monotonicity authority for Praos chain-dep state.
//!
//! A header's op-cert issue counter must be strictly greater than the
//! highest observed counter for the same `(pool_id, kes_period)`. The
//! `(pool, kes_period)` tuple keys the counter map so the same pool
//! keeps independent counter histories across KES rotation windows,
//! matching ouroboros-consensus `OCertIssueNumber` / `KESPeriod`
//! conventions.
//!
//! `apply_op_cert` is a thin wrapper around
//! `OpCertCounterMap::upsert_strict` that lifts the per-map update
//! into a `PraosChainDepState` -> `PraosChainDepState` transition.
//! Every field other than `op_cert_counters` is preserved unchanged.

use ade_types::Hash28;

use crate::consensus::errors::OpCertCounterError;
use crate::consensus::praos_state::PraosChainDepState;

/// One op-cert observation from a header.
///
/// `pool` is the pool's cold-key VRF hash (28 bytes). `kes_period` is
/// the KES period the cert was issued under. `counter` is the cert's
/// issue number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpCertObservation {
    pub pool: Hash28,
    pub kes_period: u64,
    pub counter: u64,
}

/// Apply an op-cert observation to the chain-dep state.
///
/// Returns a new state with the counter recorded if and only if
/// `counter > existing` (or no existing entry for the
/// `(pool, kes_period)` key). Equal or lower counters return
/// `OpCertCounterError::Regression { existing, attempted }`.
///
/// Pure: same `(state, observation)` -> same result, every time.
pub fn apply_op_cert(
    state: &PraosChainDepState,
    observation: &OpCertObservation,
) -> Result<PraosChainDepState, OpCertCounterError> {
    let mut new_state = state.clone();
    new_state.op_cert_counters.upsert_strict(
        observation.pool.clone(),
        observation.kes_period,
        observation.counter,
    )?;
    Ok(new_state)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_types::{BlockNo, EpochNo, Hash32, SlotNo};

    use crate::consensus::praos_state::Nonce;

    fn pool(byte: u8) -> Hash28 {
        Hash28([byte; 28])
    }

    fn populated_state() -> PraosChainDepState {
        let mut s = PraosChainDepState::empty();
        s.evolving_nonce = Nonce(Hash32([0xa1; 32]));
        s.candidate_nonce = Nonce(Hash32([0xa2; 32]));
        s.epoch_nonce = Nonce(Hash32([0xa3; 32]));
        s.previous_epoch_nonce = Nonce(Hash32([0xa4; 32]));
        s.lab_nonce = Nonce(Hash32([0xa5; 32]));
        s.last_epoch_block = Some(EpochNo(42));
        s.last_slot = Some(SlotNo(123_456));
        s.last_block_no = Some(BlockNo(7_800_000));
        s
    }

    #[test]
    fn apply_op_cert_inserts_first_observation() {
        let s = populated_state();
        let obs = OpCertObservation {
            pool: pool(1),
            kes_period: 100,
            counter: 0,
        };
        let next = apply_op_cert(&s, &obs).unwrap();
        assert_eq!(next.op_cert_counters.get(&pool(1), 100), Some(0));
        assert_eq!(next.op_cert_counters.len(), 1);
    }

    #[test]
    fn apply_op_cert_advances_existing_strictly() {
        let s = populated_state();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(1),
                kes_period: 100,
                counter: 0,
            },
        )
        .unwrap();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(1),
                kes_period: 100,
                counter: 1,
            },
        )
        .unwrap();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(1),
                kes_period: 100,
                counter: 5,
            },
        )
        .unwrap();
        assert_eq!(s.op_cert_counters.get(&pool(1), 100), Some(5));
        assert_eq!(s.op_cert_counters.len(), 1);
    }

    #[test]
    fn apply_op_cert_accepts_equal_counter_as_noop() {
        // PHASE4-N-M-FOLLOW: equal op-cert counter is the same
        // op-cert being re-used across blocks within a KES
        // period (normal pool operation per the Cardano
        // protocol). Apply MUST succeed and leave the counter
        // unchanged.
        let s = populated_state();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(2),
                kes_period: 7,
                counter: 4,
            },
        )
        .unwrap();
        let s2 = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(2),
                kes_period: 7,
                counter: 4,
            },
        )
        .unwrap();
        assert_eq!(s2.op_cert_counters.get(&pool(2), 7), Some(4));
    }

    #[test]
    fn apply_op_cert_rejects_lower_counter() {
        let s = populated_state();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(3),
                kes_period: 9,
                counter: 10,
            },
        )
        .unwrap();
        let err = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(3),
                kes_period: 9,
                counter: 9,
            },
        );
        assert_eq!(
            err,
            Err(OpCertCounterError::Regression {
                existing: 10,
                attempted: 9,
            })
        );
    }

    #[test]
    fn apply_op_cert_independent_kes_periods_dont_collide() {
        let s = populated_state();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(4),
                kes_period: 100,
                counter: 5,
            },
        )
        .unwrap();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(4),
                kes_period: 101,
                counter: 0,
            },
        )
        .unwrap();
        assert_eq!(s.op_cert_counters.get(&pool(4), 100), Some(5));
        assert_eq!(s.op_cert_counters.get(&pool(4), 101), Some(0));
        assert_eq!(s.op_cert_counters.len(), 2);
    }

    #[test]
    fn apply_op_cert_independent_pools_dont_collide() {
        let s = populated_state();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(5),
                kes_period: 50,
                counter: 7,
            },
        )
        .unwrap();
        let s = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(6),
                kes_period: 50,
                counter: 0,
            },
        )
        .unwrap();
        assert_eq!(s.op_cert_counters.get(&pool(5), 50), Some(7));
        assert_eq!(s.op_cert_counters.get(&pool(6), 50), Some(0));
        assert_eq!(s.op_cert_counters.len(), 2);
    }

    #[test]
    fn apply_op_cert_does_not_touch_nonces() {
        let s = populated_state();
        let next = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(7),
                kes_period: 1,
                counter: 1,
            },
        )
        .unwrap();
        assert_eq!(next.evolving_nonce, s.evolving_nonce);
        assert_eq!(next.candidate_nonce, s.candidate_nonce);
        assert_eq!(next.epoch_nonce, s.epoch_nonce);
        assert_eq!(next.previous_epoch_nonce, s.previous_epoch_nonce);
        assert_eq!(next.lab_nonce, s.lab_nonce);
        assert_eq!(next.last_epoch_block, s.last_epoch_block);
    }

    #[test]
    fn apply_op_cert_does_not_touch_last_slot_or_block_no() {
        let s = populated_state();
        let next = apply_op_cert(
            &s,
            &OpCertObservation {
                pool: pool(8),
                kes_period: 1,
                counter: 1,
            },
        )
        .unwrap();
        assert_eq!(next.last_slot, s.last_slot);
        assert_eq!(next.last_block_no, s.last_block_no);
    }
}
