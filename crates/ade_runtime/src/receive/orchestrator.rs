// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED per-peer N2N receive orchestrator (PHASE4-N-H S4).
//!
//! Pure state-driver — no socket I/O. Decodes inbound chain-sync
//! (client role) and block-fetch (client role) wire frames via the
//! existing PHASE4-N-A codecs, lifts them via S3's GREEN adapter,
//! and calls the BLUE [`receive_apply`] reducer.
//!
//! Multi-peer: per-peer state is independent; the single shared
//! `ChainDb` is the only cross-peer coordination point. Two peers
//! receiving the same block both succeed (`InMemoryChainDb` /
//! `PersistentChainDb` are idempotent on byte-identity).
//!
//! Key-boundary doctrine: this module MUST NOT import from
//! `crate::producer::signing` / `crate::producer::broadcast` /
//! `crate::producer::scheduler`. Enforced by
//! `ci/ci_check_receive_orchestrator_no_producer_dep.sh`.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::receive::{
    receive_apply, ChainDbWrite, ReceiveEffect, ReceiveError, ReceiveState,
};
use ade_network::codec::block_fetch::decode_block_fetch_message;
use ade_network::codec::chain_sync::decode_chain_sync_message;
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_network::codec::CodecError;

use crate::receive::events_to_state::{lift_block_fetch_event, lift_chain_sync_signal};
use crate::receive::{ChainDbWriter as _ChainDbWriterMarker};

/// Per-peer receive-side state for one connected upstream peer.
pub struct PerPeerReceiveState {
    pub receive_state: ReceiveState,
    pub chain_sync_version: ChainSyncVersion,
    pub block_fetch_version: BlockFetchVersion,
}

impl PerPeerReceiveState {
    pub fn new(
        receive_state: ReceiveState,
        chain_sync_version: ChainSyncVersion,
        block_fetch_version: BlockFetchVersion,
    ) -> Self {
        Self {
            receive_state,
            chain_sync_version,
            block_fetch_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReceiveDispatchError {
    ChainSyncDecode(CodecError),
    BlockFetchDecode(CodecError),
    Receive(ReceiveError),
}

/// Decode + lift + apply one inbound chain-sync frame.
///
/// Returns `Ok(Some(effect))` if the frame produced a state-changing
/// event; `Ok(None)` if the frame was a non-state-changing variant
/// (Intersected / NoIntersection). The decode goes through chain-sync
/// at the message level — but the receive side actually consumes
/// `ForkChoiceSignal` values from the BLUE state machine, not raw
/// `ChainSyncMessage`. For pragmatic single-frame dispatch we accept
/// pre-encoded `ChainSyncMessage` frames and translate Server-agency
/// messages into the equivalent ForkChoiceSignal value below.
pub fn dispatch_chain_sync_inbound<W: ChainDbWrite>(
    state: &mut PerPeerReceiveState,
    frame: &[u8],
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<ReceiveEffect>, ReceiveDispatchError> {
    let msg =
        decode_chain_sync_message(frame).map_err(ReceiveDispatchError::ChainSyncDecode)?;
    let sig = message_to_fork_choice_signal(msg);
    let event = match sig {
        Some(s) => lift_chain_sync_signal(s),
        None => None,
    };
    apply_event(
        state,
        event,
        chain_write,
        era_schedule,
        ledger_view,
    )
}

/// Decode + lift + apply one inbound block-fetch frame.
pub fn dispatch_block_fetch_inbound<W: ChainDbWrite>(
    state: &mut PerPeerReceiveState,
    frame: &[u8],
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<ReceiveEffect>, ReceiveDispatchError> {
    let msg =
        decode_block_fetch_message(frame).map_err(ReceiveDispatchError::BlockFetchDecode)?;
    let ev = message_to_batch_delivery_event(msg);
    let event = match ev {
        Some(e) => lift_block_fetch_event(e),
        None => None,
    };
    apply_event(
        state,
        event,
        chain_write,
        era_schedule,
        ledger_view,
    )
}

fn apply_event<W: ChainDbWrite>(
    state: &mut PerPeerReceiveState,
    event: Option<ade_ledger::receive::ReceiveEvent>,
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<ReceiveEffect>, ReceiveDispatchError> {
    match event {
        Some(e) => {
            let effect = receive_apply(
                &mut state.receive_state,
                e,
                chain_write,
                era_schedule,
                ledger_view,
                None,
            )
            .map_err(ReceiveDispatchError::Receive)?;
            Ok(Some(effect))
        }
        None => Ok(None),
    }
}

fn message_to_fork_choice_signal(
    msg: ade_network::codec::chain_sync::ChainSyncMessage,
) -> Option<ade_network::chain_sync::signal::ForkChoiceSignal> {
    use ade_network::chain_sync::signal::ForkChoiceSignal;
    use ade_network::codec::chain_sync::ChainSyncMessage;
    match msg {
        ChainSyncMessage::RollForward { header, tip } => Some(ForkChoiceSignal::RollForward {
            header_bytes: header,
            tip,
        }),
        ChainSyncMessage::RollBackward { point, tip } => {
            Some(ForkChoiceSignal::RollBackward { point, tip })
        }
        ChainSyncMessage::IntersectFound { point, tip } => {
            Some(ForkChoiceSignal::Intersected { point, tip })
        }
        ChainSyncMessage::IntersectNotFound { tip } => {
            Some(ForkChoiceSignal::NoIntersection { tip })
        }
        ChainSyncMessage::RequestNext
        | ChainSyncMessage::AwaitReply
        | ChainSyncMessage::FindIntersect { .. }
        | ChainSyncMessage::Done => None,
    }
}

fn message_to_batch_delivery_event(
    msg: ade_network::codec::block_fetch::BlockFetchMessage,
) -> Option<ade_network::block_fetch::event::BatchDeliveryEvent> {
    use ade_network::block_fetch::event::BatchDeliveryEvent;
    use ade_network::codec::block_fetch::BlockFetchMessage;
    match msg {
        BlockFetchMessage::StartBatch => Some(BatchDeliveryEvent::BatchStarted),
        BlockFetchMessage::NoBlocks => Some(BatchDeliveryEvent::NoBlocks),
        BlockFetchMessage::Block { bytes } => Some(BatchDeliveryEvent::BlockDelivered {
            block_bytes: bytes,
        }),
        BlockFetchMessage::BatchDone => Some(BatchDeliveryEvent::BatchCompleted),
        BlockFetchMessage::RequestRange(_) | BlockFetchMessage::ClientDone => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::ReceiveState;
    use ade_ledger::state::LedgerState;
    use ade_network::codec::chain_sync::{
        encode_chain_sync_message, ChainSyncMessage, Point as CsPoint, Tip as CsTip,
    };
    use ade_network::codec::block_fetch::{encode_block_fetch_message, BlockFetchMessage};
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    use crate::chaindb::InMemoryChainDb;
    use crate::receive::ChainDbWriter;

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Conway,
                start_slot: SlotNo(start_576),
                start_epoch: EPOCH_576,
                slot_length_ms: 1_000,
                epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
                safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
            }],
        )
        .expect("schedule")
    }

    fn corpus_view() -> (ConwayValidityCorpus, PoolDistrView) {
        let c = ConwayValidityCorpus::load().expect("corpus");
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            let scale = total / p.sigma.denom;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake: p.sigma.numer * scale,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        (c, PoolDistrView::new(EPOCH_576, total, asc, pools))
    }

    fn fresh_state(eta0: [u8; 32]) -> ReceiveState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        ReceiveState::new(ledger, chain_dep)
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    fn build_per_peer(eta0: [u8; 32]) -> PerPeerReceiveState {
        PerPeerReceiveState::new(
            fresh_state(eta0),
            ChainSyncVersion::new(9),
            BlockFetchVersion::new(9),
        )
    }

    #[test]
    fn dispatch_chain_sync_inbound_decodes_then_caches() {
        let (c, view) = corpus_view();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let mut state = build_per_peer(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        let frame = encode_chain_sync_message(&ChainSyncMessage::RollForward {
            header: bytes.clone(),
            tip: CsTip {
                point: CsPoint::Block {
                    slot: decoded.header_input.slot,
                    hash: decoded.block_hash.clone(),
                },
                block_no: decoded.header_input.block_no.0,
            },
        });
        let effect = dispatch_chain_sync_inbound(
            &mut state,
            &frame,
            &mut writer,
            &schedule(),
            &view,
        )
        .expect("dispatch")
        .expect("Some effect");
        match effect {
            ReceiveEffect::Cached { .. } => {}
            other => panic!("expected Cached, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_block_fetch_inbound_decodes_then_admits() {
        let (c, view) = corpus_view();
        let bytes = pick_lightest(&c);
        let decoded = decode_block(&bytes).expect("decode");
        let mut state = build_per_peer(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);

        // Cache the header first.
        let cs_frame = encode_chain_sync_message(&ChainSyncMessage::RollForward {
            header: bytes.clone(),
            tip: CsTip {
                point: CsPoint::Block {
                    slot: decoded.header_input.slot,
                    hash: decoded.block_hash.clone(),
                },
                block_no: decoded.header_input.block_no.0,
            },
        });
        dispatch_chain_sync_inbound(&mut state, &cs_frame, &mut writer, &schedule(), &view)
            .expect("cache");

        // Now deliver the body.
        let bf_frame = encode_block_fetch_message(&BlockFetchMessage::Block {
            bytes: bytes.clone(),
        });
        let effect = dispatch_block_fetch_inbound(
            &mut state,
            &bf_frame,
            &mut writer,
            &schedule(),
            &view,
        )
        .expect("dispatch")
        .expect("Some effect");
        match effect {
            ReceiveEffect::Admitted { .. } => {}
            other => panic!("expected Admitted, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_chain_sync_inbound_threads_negotiated_version() {
        // Build state with an unusual version; assert it survives in
        // state (the reducer doesn't gate on chain-sync version
        // today, so we can't observe per-call threading directly —
        // but we can pin the per-peer state holds it).
        let (c, _view) = corpus_view();
        let state = PerPeerReceiveState::new(
            fresh_state(c.epoch_nonce),
            ChainSyncVersion::new(42),
            BlockFetchVersion::new(42),
        );
        assert_eq!(state.chain_sync_version.get(), 42);
        assert_eq!(state.block_fetch_version.get(), 42);
    }

    #[test]
    fn dispatch_rejects_undecodable_input() {
        let (c, view) = corpus_view();
        let mut state = build_per_peer(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        let garbage = vec![0xFFu8; 4];
        let err = dispatch_chain_sync_inbound(
            &mut state,
            &garbage,
            &mut writer,
            &schedule(),
            &view,
        )
        .expect_err("must reject");
        match err {
            ReceiveDispatchError::ChainSyncDecode(_) => {}
            other => panic!("expected ChainSyncDecode, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_filters_non_state_changing_events() {
        let (c, view) = corpus_view();
        let mut state = build_per_peer(c.epoch_nonce);
        let db = InMemoryChainDb::new();
        let mut writer = ChainDbWriter::new(&db);
        // IntersectFound is server-agency but maps to Intersected
        // (non-state-changing → None).
        let frame = encode_chain_sync_message(&ChainSyncMessage::IntersectFound {
            point: CsPoint::Origin,
            tip: CsTip {
                point: CsPoint::Origin,
                block_no: 0,
            },
        });
        let effect = dispatch_chain_sync_inbound(
            &mut state,
            &frame,
            &mut writer,
            &schedule(),
            &view,
        )
        .expect("dispatch");
        assert!(effect.is_none(), "IntersectFound must be filtered");

        // BatchStarted (block-fetch) likewise.
        let frame = encode_block_fetch_message(&BlockFetchMessage::StartBatch);
        let effect =
            dispatch_block_fetch_inbound(&mut state, &frame, &mut writer, &schedule(), &view)
                .expect("dispatch");
        assert!(effect.is_none(), "BatchStarted must be filtered");
    }
}

// Suppress unused-import warning on the marker; needed only to
// transit the `_ChainDbWriterMarker` import name.
#[allow(dead_code)]
fn _writer_marker(_w: &_ChainDbWriterMarker<'_, crate::chaindb::InMemoryChainDb>) {}
