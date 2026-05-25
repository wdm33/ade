// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// `admit` is the Tier-1 mempool admission gate (CE-B2-5). It is a THIN gate over
// the BLUE `tx_validity`: a tx is admitted iff `tx_validity(accumulating, tx)` is
// `Valid` — NO FALSE ACCEPT. On Valid the tx id is appended and the accumulating
// state is replaced by the applied state; on Invalid the mempool is returned
// UNCHANGED with the structured reason. Re-validation is always against the
// CURRENT accumulating state, never a stale snapshot, so an intra-mempool
// dependent tx (B spending A's output) validates correctly once A is admitted.

use ade_types::Hash32;

use crate::state::LedgerState;
use crate::tx_validity::{tx_validity, TxRejectClass, TxValidityError, TxValidityVerdict};

/// The mempool's authoritative state: the admitted tx ids in admission order
/// and the ledger state after applying every admitted tx (the "accumulating"
/// state that the next `admit` re-validates against).
#[derive(Debug, Clone, PartialEq)]
pub struct MempoolState {
    accepted: Vec<Hash32>,
    accumulating: LedgerState,
}

impl MempoolState {
    /// A fresh mempool over a base ledger state. The accumulating state starts
    /// equal to `base`; nothing is admitted yet.
    pub fn new(base: LedgerState) -> Self {
        MempoolState {
            accepted: Vec::new(),
            accumulating: base,
        }
    }

    /// The admitted tx ids in admission order.
    pub fn accepted(&self) -> &[Hash32] {
        &self.accepted
    }

    /// The accumulating ledger state (base + every admitted tx applied).
    pub fn accumulating(&self) -> &LedgerState {
        &self.accumulating
    }
}

/// The closed admission outcome. `Admitted` carries the admitted tx id;
/// `Rejected` carries the coarse class plus the full structured validity reason.
// `Eq` is omitted because `TxValidityError` embeds a `PartialEq`-only
// `LedgerError`; this mirrors `TxValidityVerdict` and is a structural fact, not
// an open surface.
#[derive(Debug, Clone, PartialEq)]
pub enum AdmitOutcome {
    Admitted {
        tx_id: Hash32,
    },
    Rejected {
        class: TxRejectClass,
        error: TxValidityError,
    },
}

/// Tier-1 admission: admit `tx_cbor` iff `tx_validity(accumulating, tx)` is
/// `Valid`.
///
/// - `Valid { tx_id, applied }` → a new `MempoolState` with `tx_id` appended and
///   `accumulating` replaced by `applied`; outcome `Admitted { tx_id }`.
/// - `Invalid { class, error }` → the mempool is returned UNCHANGED (a clone of
///   the input); outcome `Rejected { class, error }`. No false accept.
///
/// Total and pure: every input produces exactly one `(MempoolState, AdmitOutcome)`
/// with no partial mutation and no I/O. Re-validation is against `mempool`'s
/// CURRENT accumulating state.
pub fn admit(mempool: &MempoolState, tx_cbor: &[u8]) -> (MempoolState, AdmitOutcome) {
    let outcome = tx_validity(&mempool.accumulating, tx_cbor);
    match outcome.verdict {
        TxValidityVerdict::Valid { tx_id, applied } => {
            let mut accepted = mempool.accepted.clone();
            accepted.push(tx_id.clone());
            let next = MempoolState {
                accepted,
                accumulating: applied,
            };
            (next, AdmitOutcome::Admitted { tx_id })
        }
        TxValidityVerdict::Invalid { class, error } => {
            // Mempool UNCHANGED on any Invalid verdict — no false accept.
            (mempool.clone(), AdmitOutcome::Rejected { class, error })
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::state::LedgerState;
    use ade_types::CardanoEra;

    /// Pin admit's prefix property as documentation-as-test:
    /// admitting a malformed slice leaves the mempool's accepted list
    /// untouched, so two sequential rejects compose with no permutation
    /// of the (vacuous) accumulating order. Re-validation runs against
    /// the CURRENT accumulating state — never a stale snapshot — which
    /// is the prefix-respecting property the forge replay relies on
    /// (DC-LEDGER-12).
    #[test]
    fn admit_prefix_property_documented() {
        let base = LedgerState::new(CardanoEra::Conway);
        let m0 = MempoolState::new(base);
        let bad = vec![0x80u8];

        let (m1, o1) = admit(&m0, &bad);
        assert!(matches!(o1, AdmitOutcome::Rejected { .. }));
        assert_eq!(m1.accepted(), m0.accepted());
        assert_eq!(m1.accumulating(), m0.accumulating());

        let (m2, o2) = admit(&m1, &bad);
        assert!(matches!(o2, AdmitOutcome::Rejected { .. }));
        assert_eq!(m2.accepted(), m0.accepted());
        assert_eq!(m2.accumulating(), m0.accumulating());
    }
}
