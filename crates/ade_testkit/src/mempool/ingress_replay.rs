// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S2 (DC-MEM-04 + DC-MEM-01 strengthening): GREEN harness
// that wraps the existing B-track adversarial corpus in synthetic
// `IngressEvent`s and replays them through `mempool_ingress`.
//
// The fold is a literal pass over `mempool_ingress` (single-step per
// OQ-6); no batching, no out-of-order interleaving.

use ade_ledger::mempool::{mempool_ingress, AdmitOutcome, IngressEvent, IngressSource, MempoolState};
use ade_ledger::state::LedgerState;
use ade_ledger::tx_validity::TxRejectClass;

use crate::tx_validity::{build_synthetic, build_valid, SyntheticMutation};

/// Closed expected-outcome variant for a B-track case wrapped as an
/// `IngressEvent`. Used by the agreement and rejection-preservation tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedOutcome {
    /// The valid synthetic case must Admit.
    Admit,
    /// An adversarial mutation must Reject with this class.
    Reject(TxRejectClass),
}

/// One B-track case lifted into an ingress trace: the `event` to feed,
/// the `base` ledger to feed it against, and the `expected` outcome.
#[derive(Debug, Clone)]
pub struct BTrackCase {
    pub event: IngressEvent,
    pub base: LedgerState,
    pub expected: ExpectedOutcome,
}

/// Wrap a raw tx CBOR byte string as an `IngressEvent`. The bytes are
/// passed verbatim (PreservedCbor end-to-end).
pub fn wrap_as_ingress(source: IngressSource, tx_cbor: Vec<u8>) -> IngressEvent {
    IngressEvent::new(source, tx_cbor)
}

/// Lift the full B-track corpus (the valid case + every
/// `SyntheticMutation`) into an `IngressEvent` trace under the given
/// `source` variant. The tx bytes are reused verbatim from the B-track
/// generators; only the envelope is new.
pub fn b_track_corpus_as_ingress(source: IngressSource) -> Vec<BTrackCase> {
    let mut cases = Vec::new();

    let valid = build_valid();
    cases.push(BTrackCase {
        event: wrap_as_ingress(source, valid.tx_cbor),
        base: valid.ledger,
        expected: ExpectedOutcome::Admit,
    });

    for mutation in SyntheticMutation::ALL {
        let case = build_synthetic(mutation);
        cases.push(BTrackCase {
            event: wrap_as_ingress(source, case.tx_cbor),
            base: case.ledger,
            expected: ExpectedOutcome::Reject(mutation.expected_class()),
        });
    }

    cases
}

/// Replay an ordered `IngressEvent` trace against a single `base` ledger
/// state, folding `mempool_ingress` over the trace. The result is the
/// final `MempoolState` plus the ordered sequence of admission outcomes.
///
/// Single-step per OQ-6 — no batching, no out-of-order interleaving.
/// Pure: identical `(base, events)` always yield identical
/// `(MempoolState, Vec<AdmitOutcome>)`.
pub fn replay_ingress_trace(
    base: LedgerState,
    events: &[IngressEvent],
) -> (MempoolState, Vec<AdmitOutcome>) {
    let mut mempool = MempoolState::new(base);
    let mut outcomes = Vec::with_capacity(events.len());
    for event in events {
        let (next, outcome) = mempool_ingress(&mempool, event);
        mempool = next;
        outcomes.push(outcome);
    }
    (mempool, outcomes)
}
