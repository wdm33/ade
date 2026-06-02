// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED shared serve-dispatch authority for the producer-side server
//! pump (PHASE4-N-F-G-H S1).
//!
//! The SINGLE serve-dispatch core both `--mode produce` and `--mode
//! node` drive: it maps an inbound N2N server-frame event over the
//! `ServedChainView` through the BLUE reducers
//! (`ade_network::{chain_sync,block_fetch}::server`) and relays the
//! typed `ServerReply` as an `OutboundCommand`. It is COORDINATOR-FREE
//! by construction (no `CoordinatorState`, no `coordinator_step`, no
//! producer evidence writer) so the `--mode node` spine can reuse it
//! without instantiating the producer coordinator. Per DC-NODE-07 there
//! is exactly one definition of this dispatch authority
//! (`ci/ci_check_single_serve_dispatch_authority.sh`).
//!
//! Extracted verbatim from `ade_node::produce_mode` (N-S-B B4 outbound
//! relay + N-T S4 served-block evidence); behavior is byte-identical.
//! The caller owns any coordinator/evidence bookkeeping around it
//! (e.g. `--mode produce` emits `ProducerLogEvent::BlockServed` from
//! the returned `ServedBlockEvidence`).

use std::collections::BTreeMap;

use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};

use crate::network::n2n_server::PerPeerN2nServerState;
use crate::network::outbound_command::PerPeerOutbound;
use crate::orchestrator::event::OrchestratorEvent;
use crate::producer::producer_log::PeerId;
use crate::producer::served_chain_handle::ServedChainView;

/// Per-peer N2N server session state, keyed by the coordinator
/// `PeerId`. Shared by both serve drivers (`--mode produce` and
/// `--mode node`).
pub type ServerPeerStates = BTreeMap<PeerId, PerPeerN2nServerState>;

/// **PHASE4-N-S-B B4** — closed dispatch-error surface for the
/// outbound-relay path. No `String` payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// Peer is not in the per-peer state map (PeerConnected
    /// never arrived or PeerDisconnected already cleared it).
    UnknownPeer { peer_id: u64 },
    /// Peer is in `peers_state` but the PerPeerOutbound map
    /// has no sender registered for that peer. Indicates a
    /// listener/driver synchronization bug.
    PeerOutboundMissing { peer_id: u64 },
    /// `mpsc::Sender::try_send` failed — either the channel
    /// is full (peer not draining fast enough) or the
    /// receiver was dropped (MuxPump task exited).
    SendFailure { peer_id: u64 },
    /// BLUE reducer rejected the inbound frame as malformed.
    ReducerError,
}

/// **PHASE4-N-T S4** — GREEN evidence observation of a block actually
/// present in the served snapshot for a block-fetch request range.
/// Collected into an owned `Vec` BEFORE any `.await` (never holds the
/// `watch::Ref` across an await). OBSERVES the served snapshot; it does
/// not re-decide what the BLUE serve reducer (`producer_block_fetch_serve`)
/// serves, and is never fabricated — `(slot, hash, bytes_len)` are read
/// from the snapshot. The caller decides what to do with it.
pub struct ServedBlockEvidence {
    pub peer_id: PeerId,
    pub slot: u64,
    pub hash: [u8; 32],
    pub bytes_len: u32,
}

/// Install per-peer N2N server state for a downstream-server peer on
/// `PeerConnected`. COORDINATOR-FREE: serve lifecycle only — the caller
/// owns any coordinator/evidence bookkeeping.
pub fn install_server_peer_state(
    peers_state: &mut ServerPeerStates,
    peer_id: PeerId,
    chain_sync_version: ChainSyncVersion,
    block_fetch_version: BlockFetchVersion,
) {
    peers_state.insert(
        peer_id,
        PerPeerN2nServerState::new(chain_sync_version, block_fetch_version),
    );
}

/// Remove per-peer N2N server state and the per-peer outbound sender on
/// `PeerDisconnected`. COORDINATOR-FREE: serve lifecycle only.
pub async fn remove_server_peer_state(
    peers_state: &mut ServerPeerStates,
    peer_outbound: &PerPeerOutbound,
    peer_id: PeerId,
) {
    peers_state.remove(&peer_id);
    peer_outbound
        .write()
        .await
        .remove(&crate::orchestrator::event::PeerId(peer_id.0));
}

/// **PHASE4-N-S-B B4** — outbound-relay-aware dispatch.
///
/// Uses the lower-level BLUE reducers (`producer_chain_sync_serve` /
/// `producer_block_fetch_serve`) directly so the reply is
/// typed `ServerReply`, not pre-encoded `Vec<u8>`. The typed
/// reply is wrapped in `OutboundCommand` and try_sent through
/// the per-peer outbound channel. No `Vec<u8>` byte tunnel.
///
/// On lookup or send failure, returns the closed
/// `DispatchError` variant; never panics.
pub async fn dispatch_server_frame_event_to_outbound(
    event: &OrchestratorEvent,
    peers_state: &mut ServerPeerStates,
    served_chain_view: &ServedChainView,
    peer_outbound: &PerPeerOutbound,
) -> Result<(usize, Vec<ServedBlockEvidence>), DispatchError> {
    use crate::network::outbound_command::OutboundCommand;
    use crate::producer::served_chain_lookups::ServedChainLookups;
    use ade_network::block_fetch::server::producer_block_fetch_serve;
    use ade_network::chain_sync::server::producer_chain_sync_serve;
    use ade_network::codec::block_fetch::{decode_block_fetch_message, BlockFetchMessage, Point};
    use ade_network::codec::chain_sync::decode_chain_sync_message;

    match event {
        OrchestratorEvent::PeerN2nServerChainSyncFrame { peer_id, bytes } => {
            let key = PeerId(peer_id.0);
            let state = peers_state
                .get(&key)
                .cloned()
                .ok_or(DispatchError::UnknownPeer { peer_id: peer_id.0 })?;
            let msg = decode_chain_sync_message(bytes).map_err(|_| DispatchError::ReducerError)?;
            let chain_sync_version = state.chain_sync_version;
            let block_fetch_version = state.block_fetch_version;
            let block_fetch_old = state.block_fetch;
            // Scope the `watch::Ref` (a `!Send` read guard) so it is dropped
            // BEFORE the outbound `.await` below, keeping this dispatch future
            // `Send` (the node-spine serve sibling spawns it). The reducer reads
            // the snapshot via `lookups` and returns `step`; the snapshot is not
            // needed for the send. (Mirrors the block-fetch arm's `drop(snap_ref)`;
            // produce_mode's behavior is unchanged — it just releases the read
            // guard a few statements sooner.)
            let (cs2, step) = {
                let snap_ref = served_chain_view.borrow();
                let lookups = ServedChainLookups { snap: &*snap_ref };
                producer_chain_sync_serve(state.chain_sync, msg, &lookups, chain_sync_version)
                    .map_err(|_| DispatchError::ReducerError)?
            };
            let mut sent = 0usize;
            if let ade_network::chain_sync::server::ServerStep::Reply(reply) = step {
                let cmd = OutboundCommand::ChainSync {
                    peer: crate::orchestrator::event::PeerId(peer_id.0),
                    reply,
                };
                let map = peer_outbound.read().await;
                let sender = map
                    .get(&crate::orchestrator::event::PeerId(peer_id.0))
                    .ok_or(DispatchError::PeerOutboundMissing { peer_id: peer_id.0 })?;
                sender
                    .try_send(cmd)
                    .map_err(|_| DispatchError::SendFailure { peer_id: peer_id.0 })?;
                sent = 1;
            }
            let updated_state = crate::network::n2n_server::PerPeerN2nServerState {
                chain_sync: cs2,
                block_fetch: block_fetch_old,
                chain_sync_version,
                block_fetch_version,
            };
            peers_state.insert(key, updated_state);
            Ok((sent, Vec::new()))
        }
        OrchestratorEvent::PeerN2nServerBlockFetchFrame { peer_id, bytes } => {
            let key = PeerId(peer_id.0);
            let state = peers_state
                .get(&key)
                .cloned()
                .ok_or(DispatchError::UnknownPeer { peer_id: peer_id.0 })?;
            let chain_sync_version = state.chain_sync_version;
            let block_fetch_version = state.block_fetch_version;
            let chain_sync_old = state.chain_sync;
            // Scope the `watch::Ref` (a `!Send` read guard) to this block so it is
            // dropped BEFORE the outbound `.await` below, keeping this dispatch
            // future `Send` (the node-spine serve sibling spawns it). Everything
            // read from the snapshot is captured as OWNED data (`step` +
            // `served_evidence`) before the block ends. The requested point range
            // is captured before `msg` is consumed by the reducer; a closed-end
            // RequestRange carries `(slot, hash)` for both endpoints (Origin has no
            // key in the snapshot's BTreeMap, so a range touching Origin observes
            // no present block — never over-claims). GREEN evidence: observe which
            // requested blocks are PRESENT in the served snapshot, reading real
            // `(slot, hash, bytes_len)`, never fabricated/zeroed.
            let (bf2, step, served_evidence) = {
                let snap_ref = served_chain_view.borrow();
                let msg =
                    decode_block_fetch_message(bytes).map_err(|_| DispatchError::ReducerError)?;
                let requested_range = match &msg {
                    BlockFetchMessage::RequestRange(r) => match (&r.from, &r.to) {
                        (
                            Point::Block { slot: fs, hash: fh },
                            Point::Block { slot: ts, hash: th },
                        ) => Some(((*fs, fh.clone()), (*ts, th.clone()))),
                        _ => None,
                    },
                    _ => None,
                };
                let lookups = ServedChainLookups { snap: &*snap_ref };
                let (bf2, step) = producer_block_fetch_serve(
                    state.block_fetch,
                    msg,
                    &lookups,
                    block_fetch_version,
                )
                .map_err(|_| DispatchError::ReducerError)?;
                let mut served_evidence: Vec<ServedBlockEvidence> = Vec::new();
                if let Some((from, to)) = requested_range {
                    for (s, h, b) in snap_ref.range_bytes(from, to) {
                        served_evidence.push(ServedBlockEvidence {
                            peer_id: PeerId(peer_id.0),
                            slot: s.0,
                            hash: h.0,
                            bytes_len: b.len() as u32,
                        });
                    }
                }
                (bf2, step, served_evidence)
            };
            let mut sent = 0usize;
            if let ade_network::block_fetch::server::BlockFetchServerStep::Replies(replies) = step {
                let map = peer_outbound.read().await;
                let sender = map
                    .get(&crate::orchestrator::event::PeerId(peer_id.0))
                    .ok_or(DispatchError::PeerOutboundMissing { peer_id: peer_id.0 })?;
                for reply in replies {
                    let cmd = OutboundCommand::BlockFetch {
                        peer: crate::orchestrator::event::PeerId(peer_id.0),
                        reply,
                    };
                    sender
                        .try_send(cmd)
                        .map_err(|_| DispatchError::SendFailure { peer_id: peer_id.0 })?;
                    sent += 1;
                }
            }
            let updated_state = crate::network::n2n_server::PerPeerN2nServerState {
                chain_sync: chain_sync_old,
                block_fetch: bf2,
                chain_sync_version,
                block_fetch_version,
            };
            peers_state.insert(key, updated_state);
            Ok((sent, served_evidence))
        }
        _ => Ok((0, Vec::new())),
    }
}
