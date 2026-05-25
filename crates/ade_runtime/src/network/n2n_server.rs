// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// RED per-peer N2N server session driver (PHASE4-N-G S6).
//
// Pure state-machine driver — no socket I/O. Decodes inbound mini-
// protocol frames, calls the BLUE reducers via the GREEN
// `ServedChainLookups` adapter, encodes outgoing frames.
//
// Key-boundary doctrine: this module MUST NOT import from
// `crate::producer::signing`. Enforced by
// `ci/ci_check_n2n_server_no_signing_dep.sh`. Per-peer state is
// independent; cross-peer coordination is only via the shared
// `&ServedChainSnapshot`.

use ade_ledger::producer::ServedChainSnapshot;
use ade_network::codec::CodecError;
use ade_network::block_fetch::server::{
    producer_block_fetch_serve, BlockFetchServerStep, ProducerBlockFetchServerError,
    ProducerBlockFetchServerState,
};
use ade_network::chain_sync::server::{
    producer_chain_sync_advance_tip, producer_chain_sync_serve, ProducerChainSyncServerState,
    ProducerServerError, ServerStep,
};
use ade_network::codec::block_fetch::{decode_block_fetch_message, encode_block_fetch_message};
use ade_network::codec::chain_sync::{decode_chain_sync_message, encode_chain_sync_message};
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};

use crate::producer::served_chain_lookups::ServedChainLookups;

/// Per-peer N2N server state for a single connected peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerPeerN2nServerState {
    pub chain_sync: ProducerChainSyncServerState,
    pub block_fetch: ProducerBlockFetchServerState,
    pub chain_sync_version: ChainSyncVersion,
    pub block_fetch_version: BlockFetchVersion,
}

impl PerPeerN2nServerState {
    pub fn new(cs_version: ChainSyncVersion, bf_version: BlockFetchVersion) -> Self {
        Self {
            chain_sync: ProducerChainSyncServerState::new(),
            block_fetch: ProducerBlockFetchServerState::new(),
            chain_sync_version: cs_version,
            block_fetch_version: bf_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    ChainSyncDecode(CodecError),
    BlockFetchDecode(CodecError),
    ChainSync(ProducerServerError),
    BlockFetch(ProducerBlockFetchServerError),
}

/// Process one inbound chain-sync frame.
/// Returns updated state, optional encoded outgoing frame, and whether
/// the session has terminated (client said Done).
pub fn dispatch_chain_sync_frame(
    mut state: PerPeerN2nServerState,
    frame: &[u8],
    snap: &ServedChainSnapshot,
) -> Result<(PerPeerN2nServerState, Option<Vec<u8>>, bool), DispatchError> {
    let msg = decode_chain_sync_message(frame).map_err(DispatchError::ChainSyncDecode)?;
    let lookups = ServedChainLookups { snap };
    let (cs2, step) =
        producer_chain_sync_serve(state.chain_sync, msg, &lookups, state.chain_sync_version)
            .map_err(DispatchError::ChainSync)?;
    state.chain_sync = cs2;
    match step {
        ServerStep::Done => Ok((state, None, true)),
        ServerStep::Reply(reply) => {
            let bytes = encode_chain_sync_message(&reply.into_message());
            Ok((state, Some(bytes), false))
        }
    }
}

/// Process one inbound block-fetch frame.
/// Returns updated state, a (possibly multi-element) outgoing frame
/// sequence, and whether the session has terminated.
pub fn dispatch_block_fetch_frame(
    mut state: PerPeerN2nServerState,
    frame: &[u8],
    snap: &ServedChainSnapshot,
) -> Result<(PerPeerN2nServerState, Vec<Vec<u8>>, bool), DispatchError> {
    let msg = decode_block_fetch_message(frame).map_err(DispatchError::BlockFetchDecode)?;
    let lookups = ServedChainLookups { snap };
    let (bf2, step) = producer_block_fetch_serve(
        state.block_fetch,
        msg,
        &lookups,
        state.block_fetch_version,
    )
    .map_err(DispatchError::BlockFetch)?;
    state.block_fetch = bf2;
    match step {
        BlockFetchServerStep::Done => Ok((state, Vec::new(), true)),
        BlockFetchServerStep::Replies(replies) => {
            let frames: Vec<Vec<u8>> = replies
                .into_iter()
                .map(|r| encode_block_fetch_message(&r.into_message()))
                .collect();
            Ok((state, frames, false))
        }
    }
}

/// Poll for a deferred chain-sync RollForward after broadcast
/// admission. Called by the orchestrator once per peer after each
/// `drain_and_admit`.
pub fn poll_chain_sync_advance(
    mut state: PerPeerN2nServerState,
    snap: &ServedChainSnapshot,
) -> Result<(PerPeerN2nServerState, Option<Vec<u8>>), DispatchError> {
    let lookups = ServedChainLookups { snap };
    let (cs2, maybe) =
        producer_chain_sync_advance_tip(state.chain_sync, &lookups).map_err(DispatchError::ChainSync)?;
    state.chain_sync = cs2;
    Ok((state, maybe.map(|r| encode_chain_sync_message(&r.into_message()))))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_network::codec::chain_sync::ChainSyncMessage;

    fn vs() -> (ChainSyncVersion, BlockFetchVersion) {
        (ChainSyncVersion::new(9), BlockFetchVersion::new(9))
    }

    #[test]
    fn dispatch_chain_sync_frame_request_next_yields_outgoing_frame() {
        // Empty snapshot → RequestNext should yield an AwaitReply
        // outgoing frame.
        let (cs_v, bf_v) = vs();
        let state = PerPeerN2nServerState::new(cs_v, bf_v);
        let snap = ServedChainSnapshot::new();
        let req_next = encode_chain_sync_message(&ChainSyncMessage::RequestNext);
        let (_state2, frame, done) =
            dispatch_chain_sync_frame(state, &req_next, &snap).unwrap();
        assert!(!done);
        let frame = frame.expect("outgoing frame");
        let decoded = decode_chain_sync_message(&frame).unwrap();
        assert_eq!(decoded, ChainSyncMessage::AwaitReply);
    }

    #[test]
    fn dispatch_chain_sync_frame_threads_negotiated_version_to_reducer() {
        // Build state with a deliberately unusual version; the
        // dispatcher must thread it (not read from a global).
        let cs_v = ChainSyncVersion::new(42);
        let bf_v = BlockFetchVersion::new(42);
        let state = PerPeerN2nServerState::new(cs_v, bf_v);
        assert_eq!(state.chain_sync_version.get(), 42);
        // Drive a Done (any client-legal msg works) to confirm no
        // panic with the unusual version.
        let snap = ServedChainSnapshot::new();
        let done_frame = encode_chain_sync_message(&ChainSyncMessage::Done);
        let (_state2, _frame, ended) = dispatch_chain_sync_frame(state, &done_frame, &snap).unwrap();
        assert!(ended);
    }

    #[test]
    fn dispatch_block_fetch_frame_request_range_yields_batch_frames() {
        use ade_network::codec::block_fetch::{BlockFetchMessage, Point, Range};
        use ade_types::{Hash32, SlotNo};
        let (cs_v, bf_v) = vs();
        let state = PerPeerN2nServerState::new(cs_v, bf_v);
        // Empty snapshot + range over a fabricated point → NoBlocks.
        let snap = ServedChainSnapshot::new();
        let range = Range {
            from: Point::Block { slot: SlotNo(1), hash: Hash32([0u8; 32]) },
            to: Point::Block { slot: SlotNo(2), hash: Hash32([0u8; 32]) },
        };
        let req = encode_block_fetch_message(&BlockFetchMessage::RequestRange(range));
        let (_state2, frames, done) = dispatch_block_fetch_frame(state, &req, &snap).unwrap();
        assert!(!done);
        assert_eq!(frames.len(), 1);
        let decoded = decode_block_fetch_message(&frames[0]).unwrap();
        assert_eq!(decoded, BlockFetchMessage::NoBlocks);
    }

    #[test]
    fn dispatch_chain_sync_frame_rejects_undecodable_input() {
        let (cs_v, bf_v) = vs();
        let state = PerPeerN2nServerState::new(cs_v, bf_v);
        let snap = ServedChainSnapshot::new();
        let garbage = vec![0xFFu8; 4];
        let err = dispatch_chain_sync_frame(state, &garbage, &snap).expect_err("must reject");
        match err {
            DispatchError::ChainSyncDecode(_) => {}
            other => panic!("expected ChainSyncDecode, got {other:?}"),
        }
    }

    #[test]
    fn poll_chain_sync_advance_idle_yields_none() {
        let (cs_v, bf_v) = vs();
        let state = PerPeerN2nServerState::new(cs_v, bf_v);
        let snap = ServedChainSnapshot::new();
        let (_state2, maybe) = poll_chain_sync_advance(state, &snap).unwrap();
        assert!(maybe.is_none(), "idle peer + empty snap → no advance");
    }
}
