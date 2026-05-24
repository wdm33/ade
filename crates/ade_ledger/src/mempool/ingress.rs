// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-N-E S1 (DC-MEM-03): the single BLUE chokepoint into `admit` from
// wire ingress. Tx bytes enter the mempool only as `IngressEvent`s; the
// `source` variant is metadata only and MUST NOT change the verdict.
// `mempool_ingress` is a pure pass-through to `admit` over the event's bytes.

use crate::mempool::admit::{admit, AdmitOutcome, MempoolState};

/// Closed source discriminant: which transport carried the tx into the
/// mempool. Evidence/policy/replay metadata only — `mempool_ingress` MUST
/// NOT branch on this in the verdict path (CI-enforced).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IngressSource {
    N2N,
    N2C,
}

/// Canonical typed entry into the mempool. `tx_bytes` flows verbatim
/// (PreservedCbor end-to-end) through to `admit`; no normalization,
/// no decoding, no re-encoding at this layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressEvent {
    source: IngressSource,
    tx_bytes: Vec<u8>,
}

impl IngressEvent {
    pub fn new(source: IngressSource, tx_bytes: Vec<u8>) -> Self {
        Self { source, tx_bytes }
    }

    pub fn source(&self) -> IngressSource {
        self.source
    }

    pub fn tx_bytes(&self) -> &[u8] {
        &self.tx_bytes
    }
}

/// The single BLUE chokepoint from wire ingress into the mempool.
///
/// A pure function of `(mempool, event)`; the `event.source` is recorded
/// for evidence/policy/replay but MUST NOT affect the verdict. Equivalently:
/// `mempool_ingress(s, IngressEvent { source: N2N, b })` ==
/// `mempool_ingress(s, IngressEvent { source: N2C, b })` for all `(s, b)`.
///
/// Total and pure: every input produces exactly one `(MempoolState, AdmitOutcome)`
/// with no partial mutation and no I/O. Re-validation is against `mempool`'s
/// current accumulating state — the same property `admit` already proves.
pub fn mempool_ingress(
    mempool: &MempoolState,
    event: &IngressEvent,
) -> (MempoolState, AdmitOutcome) {
    admit(mempool, event.tx_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Adversarial-corpus-bearing tests live in
    // `crates/ade_ledger/tests/mempool_ingress.rs` (integration tests) because
    // they need `ade_testkit`, which depends on `ade_ledger` (cycle if pulled
    // into lib tests). These inline tests cover only what the lib alone can
    // observe: byte preservation, source closure.

    #[test]
    fn ingress_preserves_tx_bytes_verbatim() {
        // No normalization, no truncation, no re-encoding at the ingress layer.
        let raw: Vec<u8> = (0u8..=255u8).collect();
        let event = IngressEvent::new(IngressSource::N2N, raw.clone());
        assert_eq!(event.tx_bytes(), raw.as_slice());
        assert_eq!(event.source(), IngressSource::N2N);
    }

    #[test]
    fn ingress_source_is_closed_two_variants() {
        // Exhaustive match — compile fails if a new variant is added without
        // the cluster doc's review. N-E's invariant N-E-8 + DC-MEM-03 hinge
        // on this surface staying closed.
        for src in [IngressSource::N2N, IngressSource::N2C] {
            match src {
                IngressSource::N2N | IngressSource::N2C => {}
            }
        }
    }
}
