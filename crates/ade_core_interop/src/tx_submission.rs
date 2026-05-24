// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S4 (CE-N-E-6, code half): GREEN adapter + accumulator + bridge
// from `ade_network::tx_submission::InventoryEvent` (N-A) into
// `ade_ledger::mempool::IngressEvent` (N-E), with per-peer accumulation
// and deterministic canonicalization.
//
// This module is purely deterministic: the production RED socket loop that
// drives a real cardano-node peer is the operator-action half (see
// docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md). Everything here is
// mechanically testable from synthetic InventoryEvents.

use ade_ledger::mempool::{
    canonicalize_peer_streams, AdmitOutcome, IngressEvent, IngressSource, MempoolState,
    PeerId, PeerSubmissionQueue,
};
use ade_ledger::state::LedgerState;
use ade_network::tx_submission::InventoryEvent;
use ade_testkit::mempool::replay_ingress_trace;

/// Map a single `InventoryEvent` from a known peer source to zero or
/// more `IngressEvent`s. Pure; no I/O. Only `TxsDelivered` carries
/// tx bytes; all other events emit an empty Vec.
pub fn event_to_ingress(event: &InventoryEvent, src: IngressSource) -> Vec<IngressEvent> {
    match event {
        InventoryEvent::TxsDelivered { tx_bytes } => tx_bytes
            .iter()
            .cloned()
            .map(|b| IngressEvent::new(src, b))
            .collect(),
        InventoryEvent::ServerOpened
        | InventoryEvent::IdsRequested { .. }
        | InventoryEvent::IdsDelivered { .. }
        | InventoryEvent::TxsRequested { .. } => Vec::new(),
    }
}

/// Per-peer accumulator over an `InventoryEvent` stream. Pure; observes
/// each event into a per-peer FIFO of tx_bytes, drainable into a
/// `PeerSubmissionQueue`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerAccumulator {
    peer: PeerId,
    txs: Vec<Vec<u8>>,
}

impl PeerAccumulator {
    pub fn new(peer: PeerId) -> Self {
        Self {
            peer,
            txs: Vec::new(),
        }
    }

    pub fn observe(&mut self, event: &InventoryEvent) {
        if let InventoryEvent::TxsDelivered { tx_bytes } = event {
            for b in tx_bytes {
                self.txs.push(b.clone());
            }
        }
    }

    pub fn drain(self) -> PeerSubmissionQueue {
        PeerSubmissionQueue {
            peer: self.peer,
            source: IngressSource::N2N,
            txs: self.txs,
        }
    }

    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }
}

/// Given per-peer `InventoryEvent` streams from N2N tx-submission2,
/// build per-peer queues, canonicalize via
/// `canonicalize_peer_streams`, and replay through
/// `replay_ingress_trace`. Pure function of the inputs.
///
/// The production socket loop is the only RED layer; this function is
/// the deterministic GREEN bridge that the loop's collected output is
/// fed into.
pub fn ingest_n2n_events(
    base: LedgerState,
    per_peer: &[(PeerId, Vec<InventoryEvent>)],
) -> (MempoolState, Vec<AdmitOutcome>) {
    let queues: Vec<PeerSubmissionQueue> = per_peer
        .iter()
        .map(|(peer, events)| {
            let mut acc = PeerAccumulator::new(peer.clone());
            for ev in events {
                acc.observe(ev);
            }
            acc.drain()
        })
        .collect();

    let canonical = canonicalize_peer_streams(&queues);
    replay_ingress_trace(base, &canonical)
}
