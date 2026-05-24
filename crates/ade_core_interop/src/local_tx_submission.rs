// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S5 (CE-N-E-7, code half): GREEN adapter + accumulator + bridge
// from `ade_network::n2c::local_tx_submission::LocalTxSubmissionEvent` (N-A)
// into `ade_ledger::mempool::IngressEvent` (N-E), under
// `IngressSource::N2C`.
//
// The actual UDS socket loop driving cardano-cli is the operator-action
// half (see docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md). Everything
// here is mechanically testable from synthetic LocalTxSubmissionEvents.

use ade_ledger::mempool::{
    canonicalize_peer_streams, AdmitOutcome, IngressEvent, IngressSource, MempoolState,
    PeerId, PeerSubmissionQueue,
};
use ade_ledger::state::LedgerState;
use ade_network::n2c::local_tx_submission::LocalTxSubmissionEvent;
use ade_testkit::mempool::replay_ingress_trace;

/// Map a single `LocalTxSubmissionEvent` from a known client to zero
/// or one `IngressEvent`. Only `TxSubmitted` carries tx bytes;
/// `TxAccepted` / `TxRejected` are server-to-client responses with no
/// bytes to admit.
pub fn local_event_to_ingress(event: &LocalTxSubmissionEvent) -> Vec<IngressEvent> {
    match event {
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes } => {
            vec![IngressEvent::new(IngressSource::N2C, tx_bytes.clone())]
        }
        LocalTxSubmissionEvent::TxAccepted
        | LocalTxSubmissionEvent::TxRejected { .. } => Vec::new(),
    }
}

/// Per-client accumulator over a `LocalTxSubmissionEvent` stream.
/// Pure; observes each event into a per-client FIFO of tx_bytes,
/// drainable into a `PeerSubmissionQueue` with N2C source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAccumulator {
    client: PeerId,
    txs: Vec<Vec<u8>>,
}

impl ClientAccumulator {
    pub fn new(client: PeerId) -> Self {
        Self {
            client,
            txs: Vec::new(),
        }
    }

    pub fn observe(&mut self, event: &LocalTxSubmissionEvent) {
        if let LocalTxSubmissionEvent::TxSubmitted { tx_bytes } = event {
            self.txs.push(tx_bytes.clone());
        }
    }

    pub fn drain(self) -> PeerSubmissionQueue {
        PeerSubmissionQueue {
            peer: self.client,
            source: IngressSource::N2C,
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

/// Given per-client `LocalTxSubmissionEvent` streams from N2C
/// local-tx-submission, build per-client queues, canonicalize via
/// `canonicalize_peer_streams`, and replay through
/// `replay_ingress_trace`. Pure function of the inputs.
pub fn ingest_n2c_events(
    base: LedgerState,
    per_client: &[(PeerId, Vec<LocalTxSubmissionEvent>)],
) -> (MempoolState, Vec<AdmitOutcome>) {
    let queues: Vec<PeerSubmissionQueue> = per_client
        .iter()
        .map(|(client, events)| {
            let mut acc = ClientAccumulator::new(client.clone());
            for ev in events {
                acc.observe(ev);
            }
            acc.drain()
        })
        .collect();

    let canonical = canonicalize_peer_streams(&queues);
    replay_ingress_trace(base, &canonical)
}
