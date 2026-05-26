// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN [`LiveLedgerView`] — `LedgerView` impl backed by an
//! imported, fingerprinted [`LiveConsensusInputsCanonical`]
//! (PHASE4-N-M-C S2).
//!
//! The view is constructed deterministically from the canonical
//! form and answers BLUE consensus's `LedgerView` queries directly
//! from the imported fields. The epoch-window guard is enforced
//! here at the per-query boundary (DC-VIEW-01):
//!   - any `LedgerView` query whose `epoch` argument is not
//!     `canonical.epoch_no` returns `None` — BLUE then fails
//!     closed via `BlockValidityError::MissingConsensusInput`,
//!     which the admission runner classifies as a body-side
//!     `Invalid` outcome.
//!
//! The per-block slot-window guard (slot ∉ [start, end]) lives in
//! the admission runner — it intercepts before
//! `admit_via_block_validity` is invoked and emits
//! `AdmissionHalted { reason: CrossEpochUse }` (DC-ADMIT-11).

use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_types::{EpochNo, Hash28, Hash32};

use super::canonical::LiveConsensusInputsCanonical;

/// `LedgerView` implementation over an operator-imported,
/// fingerprinted consensus-inputs bundle. Per-query
/// epoch-window-guard semantics: returns `None` for any epoch
/// other than `canonical.epoch_no`.
#[derive(Debug, Clone)]
pub struct LiveLedgerView {
    inputs: LiveConsensusInputsCanonical,
}

impl LiveLedgerView {
    pub fn new(inputs: LiveConsensusInputsCanonical) -> Self {
        Self { inputs }
    }

    /// Canonical inputs (read-only).
    pub fn inputs(&self) -> &LiveConsensusInputsCanonical {
        &self.inputs
    }

    /// Canonical fingerprint — the load-bearing handle every
    /// admission JSONL block-event references (DC-ADMIT-10).
    pub fn fingerprint(&self) -> &Hash32 {
        &self.inputs.fingerprint
    }
}

impl LedgerView for LiveLedgerView {
    fn total_active_stake(&self, epoch: EpochNo) -> Option<u64> {
        if epoch != self.inputs.epoch_no {
            return None;
        }
        let mut sum: u64 = 0;
        for entry in self.inputs.pool_distribution.values() {
            sum = sum.saturating_add(entry.active_stake);
        }
        Some(sum)
    }

    fn pool_active_stake(&self, epoch: EpochNo, pool: &Hash28) -> Option<u64> {
        if epoch != self.inputs.epoch_no {
            return None;
        }
        self.inputs.pool_distribution.get(pool).map(|p| p.active_stake)
    }

    fn pool_vrf_keyhash(&self, epoch: EpochNo, pool: &Hash28) -> Option<Hash32> {
        if epoch != self.inputs.epoch_no {
            return None;
        }
        self.inputs.pool_vrf_keyhashes.get(pool).cloned()
    }

    fn active_slots_coeff(&self, epoch: EpochNo) -> Option<ActiveSlotsCoeff> {
        if epoch != self.inputs.epoch_no {
            return None;
        }
        Some(self.inputs.active_slots_coeff)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use super::super::canonical::import_live_consensus_inputs_from_bytes;
    use ade_types::Hash28;

    const MINIMAL: &str = r#"{
        "network_magic": 1,
        "genesis_hash_hex": "00000000000000000000000000000000000000000000000000000000000000aa",
        "era": "conway",
        "epoch_no": 200,
        "epoch_start_slot": 86400000,
        "epoch_end_slot": 86832000,
        "active_slots_coeff": {"numer": 1, "denom": 20},
        "epoch_nonce_hex": "00000000000000000000000000000000000000000000000000000000000000bb",
        "pool_distribution": {
            "00000000000000000000000000000000000000000000000000000001": {"active_stake": 100},
            "00000000000000000000000000000000000000000000000000000002": {"active_stake": 300}
        },
        "pool_vrf_keyhashes": {
            "00000000000000000000000000000000000000000000000000000001": "00000000000000000000000000000000000000000000000000000000000000cc",
            "00000000000000000000000000000000000000000000000000000002": "00000000000000000000000000000000000000000000000000000000000000dd"
        },
        "protocol_params_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ee",
        "source_cardano_node_version": "cardano-node 11.0.1",
        "source_query_command": "cardano-cli conway query stake-distribution --testnet-magic 1",
        "source_tip_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ff",
        "source_tip_slot": 86400500
    }"#;

    fn view() -> LiveLedgerView {
        let canonical = import_live_consensus_inputs_from_bytes(MINIMAL.as_bytes()).unwrap();
        LiveLedgerView::new(canonical)
    }

    fn pool1() -> Hash28 {
        let mut b = [0u8; 28];
        b[27] = 0x01;
        Hash28(b)
    }
    fn pool2() -> Hash28 {
        let mut b = [0u8; 28];
        b[27] = 0x02;
        Hash28(b)
    }
    fn pool_unknown() -> Hash28 {
        Hash28([0xee; 28])
    }

    #[test]
    fn in_window_epoch_answers_total_active_stake() {
        let v = view();
        assert_eq!(v.total_active_stake(EpochNo(200)), Some(400));
    }

    #[test]
    fn out_of_window_epoch_returns_none() {
        let v = view();
        assert_eq!(v.total_active_stake(EpochNo(199)), None);
        assert_eq!(v.total_active_stake(EpochNo(201)), None);
        assert_eq!(v.pool_active_stake(EpochNo(199), &pool1()), None);
        assert_eq!(v.pool_vrf_keyhash(EpochNo(199), &pool1()), None);
        assert_eq!(v.active_slots_coeff(EpochNo(199)), None);
    }

    #[test]
    fn in_window_per_pool_lookups_return_imported_values() {
        let v = view();
        assert_eq!(v.pool_active_stake(EpochNo(200), &pool1()), Some(100));
        assert_eq!(v.pool_active_stake(EpochNo(200), &pool2()), Some(300));
        assert!(v.pool_vrf_keyhash(EpochNo(200), &pool1()).is_some());
        assert_eq!(
            v.active_slots_coeff(EpochNo(200)),
            Some(ActiveSlotsCoeff { numer: 1, denom: 20 })
        );
    }

    #[test]
    fn in_window_unknown_pool_returns_none() {
        let v = view();
        assert_eq!(v.pool_active_stake(EpochNo(200), &pool_unknown()), None);
        assert_eq!(v.pool_vrf_keyhash(EpochNo(200), &pool_unknown()), None);
    }

    #[test]
    fn fingerprint_accessor_matches_canonical() {
        let v = view();
        assert_eq!(v.fingerprint(), &v.inputs().fingerprint);
    }
}
