// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-N-G S5 — End-to-end session-transcript replay test.
//
// Drives the full producer-side server pipeline for one synthetic
// session:
//   1. Build broadcast queue from corpus-derived AcceptedBlock arrivals.
//   2. GREEN adapter (`drain_and_admit`) admits queue -> ServedChainSnapshot.
//   3. ServedChainLookups projects the snapshot to the BLUE reducer traits.
//   4. Drive a synthetic peer-message sequence through the chain-sync
//      + block-fetch server reducers.
//   5. Collect the outgoing wire-frame Vec<Vec<u8>>.
//
// Three invariants close at this slice:
//   * DC-PROTO-07 (transcript determinism): two identical runs produce
//     byte-identical outgoing frame sequences.
//   * DC-CONS-17 (served-bytes parity): every Block{bytes} payload in
//     the transcript equals the admitted AcceptedBlock.as_bytes() at
//     that (slot,hash) key.
//   * DC-CONS-18 (header-body wire coherence): every RollForward{header}
//     in the transcript matches the canonical header projection
//     `accepted_block_header_bytes` over the same AcceptedBlock that
//     the matching block-fetch range subsequently serves.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
use ade_ledger::block_validity::{accepted_block_header_bytes, decode_block};
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::producer::{self_accept, AcceptedBlock, ServedChainSnapshot};
use ade_ledger::state::LedgerState;
use ade_network::block_fetch::server::{
    producer_block_fetch_serve, BlockFetchServerStep, ProducerBlockFetchServerState,
};
use ade_network::chain_sync::server::{
    producer_chain_sync_advance_tip, producer_chain_sync_serve, ProducerChainSyncServerState,
    ServerStep,
};
use ade_network::codec::block_fetch::{encode_block_fetch_message, BlockFetchMessage, Range};
use ade_network::codec::chain_sync::{encode_chain_sync_message, ChainSyncMessage};
use ade_network::codec::block_fetch::Point as BlockFetchPoint;
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_runtime::producer::broadcast::BroadcastQueue;
use ade_runtime::producer::broadcast_to_served::drain_and_admit;
use ade_runtime::producer::served_chain_lookups::ServedChainLookups;
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

const EPOCH_576: EpochNo = EpochNo(576);
const EPOCH_577_START: u64 = 163_900_800;
const MAINNET_EPOCH_LENGTH: u64 = 432_000;

fn schedule() -> EraSchedule {
    let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
    let eras = vec![EraSummary {
        era: CardanoEra::Conway,
        start_slot: SlotNo(start_576),
        start_epoch: EPOCH_576,
        slot_length_ms: 1_000,
        epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
        safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
    }];
    EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras).expect("schedule")
}

fn corpus_view() -> (ConwayValidityCorpus, PoolDistrView) {
    let c = ConwayValidityCorpus::load().expect("corpus loads");
    let total = c.pd_total_active_stake;
    let asc = ActiveSlotsCoeff {
        numer: c.asc.numer as u32,
        denom: c.asc.denom as u32,
    };
    let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    for (pool_id, p) in &c.pools {
        let scale = total / p.sigma.denom;
        let active_stake = p.sigma.numer * scale;
        pools.insert(
            Hash28(*pool_id),
            PoolEntry {
                active_stake,
                vrf_keyhash: Hash32(p.vrf_keyhash),
            },
        );
    }
    (c, PoolDistrView::new(EPOCH_576, total, asc, pools))
}

fn build_accepted_n(n: usize) -> Vec<AcceptedBlock> {
    let (c, view) = corpus_view();
    let mut idxs: Vec<usize> = (0..c.blocks.len()).collect();
    idxs.sort_by_key(|&i| {
        let env = decode_block_envelope(&c.blocks[i]).expect("env");
        env.block_end - env.block_start
    });
    let block_bytes: Vec<Vec<u8>> = idxs.into_iter().take(n).map(|i| c.blocks[i].clone()).collect();
    let schedule = schedule();
    let ledger = {
        let mut l = LedgerState::new(CardanoEra::Conway);
        l.epoch_state.epoch = EPOCH_576;
        l
    };
    let chain_dep = {
        let mut s = PraosChainDepState::empty();
        s.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
        s.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
        s
    };
    block_bytes
        .iter()
        .map(|b| self_accept(b, &ledger, &chain_dep, &schedule, &view).expect("self_accept"))
        .collect()
}

fn cs_v() -> ChainSyncVersion { ChainSyncVersion::new(9) }
fn bf_v() -> BlockFetchVersion { BlockFetchVersion::new(9) }

/// Drive a single synthetic session through the full pipeline.
/// Returns the outgoing wire-frame sequence (encoded bytes).
fn run_session(
    arrivals: &[AcceptedBlock],
    cs_inputs: &[ChainSyncMessage],
    bf_inputs: &[BlockFetchMessage],
) -> Vec<Vec<u8>> {
    // 1. Admit arrivals into snapshot via the GREEN adapter.
    let mut queue = BroadcastQueue::new(arrivals.len().max(1));
    for a in arrivals {
        queue.enqueue(a.clone()).expect("enqueue");
    }
    let (snap, _q, _drained) = drain_and_admit(ServedChainSnapshot::new(), queue).expect("admit");
    let lookups = ServedChainLookups { snap: &snap };

    // 2. Drive chain-sync session.
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut cs_state = ProducerChainSyncServerState::new();
    for m in cs_inputs {
        let (s2, step) =
            producer_chain_sync_serve(cs_state, m.clone(), &lookups, cs_v()).expect("cs serve");
        cs_state = s2;
        match step {
            ServerStep::Reply(reply) => {
                frames.push(encode_chain_sync_message(&reply.into_message()));
            }
            ServerStep::Done => break,
        }
    }
    // Also flush any pending advance_tip; if cs_state ended in
    // CanAwait/MustReply, advance_tip drains any further blocks.
    loop {
        let (s2, maybe) = producer_chain_sync_advance_tip(cs_state, &lookups).expect("cs adv");
        cs_state = s2;
        match maybe {
            Some(reply) => frames.push(encode_chain_sync_message(&reply.into_message())),
            None => break,
        }
    }

    // 3. Drive block-fetch session.
    let mut bf_state = ProducerBlockFetchServerState::new();
    for m in bf_inputs {
        let (s2, step) =
            producer_block_fetch_serve(bf_state, m.clone(), &lookups, bf_v()).expect("bf serve");
        bf_state = s2;
        match step {
            BlockFetchServerStep::Replies(replies) => {
                for r in replies {
                    frames.push(encode_block_fetch_message(&r.into_message()));
                }
            }
            BlockFetchServerStep::Done => break,
        }
    }

    frames
}

#[test]
fn session_transcript_replay_byte_identical() {
    // DC-PROTO-07: two runs over identical canonical inputs produce
    // identical outgoing frame sequences.
    let arrivals = build_accepted_n(2);
    let cs_inputs = vec![
        ChainSyncMessage::RequestNext,
        ChainSyncMessage::RequestNext,
    ];
    let bf_inputs: Vec<BlockFetchMessage> = Vec::new();
    let a = run_session(&arrivals, &cs_inputs, &bf_inputs);
    let b = run_session(&arrivals, &cs_inputs, &bf_inputs);
    assert_eq!(a, b, "session transcript must replay byte-identical");
    assert!(!a.is_empty(), "transcript must be non-empty");
}

#[test]
fn session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes() {
    // DC-CONS-17: every Block{bytes} payload in the transcript equals
    // the admitted AcceptedBlock.as_bytes() at its (slot,hash) key.
    let arrivals = build_accepted_n(2);
    // Compute the (slot,hash) range that covers all admitted blocks.
    let mut keys: Vec<(SlotNo, Hash32)> = arrivals
        .iter()
        .map(|a| {
            let d = decode_block(a.as_bytes()).expect("decode");
            (d.header_input.slot, d.block_hash)
        })
        .collect();
    keys.sort();
    let from = keys.first().unwrap().clone();
    let to = keys.last().unwrap().clone();
    let bf_inputs = vec![BlockFetchMessage::RequestRange(Range {
        from: BlockFetchPoint::Block { slot: from.0, hash: from.1.clone() },
        to: BlockFetchPoint::Block { slot: to.0, hash: to.1.clone() },
    })];
    let frames = run_session(&arrivals, &[], &bf_inputs);

    // Recover the Block{bytes} payloads from the transcript by
    // decoding each frame as a BlockFetchMessage.
    use ade_network::codec::block_fetch::decode_block_fetch_message;
    let block_payloads: Vec<Vec<u8>> = frames
        .iter()
        .filter_map(|f| decode_block_fetch_message(f).ok())
        .filter_map(|m| match m {
            BlockFetchMessage::Block { bytes } => Some(bytes),
            _ => None,
        })
        .collect();

    let admitted_bytes_in_order: Vec<Vec<u8>> = {
        // Walk the admitted (slot,hash) keys in BTreeMap order (which
        // is how range_bytes iterates) and match each Block payload.
        let mut snap = ServedChainSnapshot::new();
        for a in &arrivals {
            snap = ade_ledger::producer::served_chain_admit(snap, a.clone()).expect("admit");
        }
        snap.range_bytes(from.clone(), to.clone())
            .map(|(_, _, bytes)| bytes.to_vec())
            .collect()
    };
    // DC-CONS-17 strengthened by CN-WIRE-08 (PHASE4-N-X): the served
    // wire payload is the tag-24 CBOR-in-CBOR wrap of the admitted
    // AcceptedBlock.as_bytes(); UNWRAPPING each via the shared authority
    // recovers the admitted bytes byte-for-byte. The bare [era,block] is
    // never served.
    use ade_network::codec::block_fetch::decompose_blockfetch_block;
    let served_inner: Vec<Vec<u8>> = block_payloads
        .iter()
        .map(|p| {
            assert_eq!(&p[0..2], &[0xd8, 0x18], "served payload must be tag-24 wrapped");
            decompose_blockfetch_block(p).expect("tag24 unwrap").to_vec()
        })
        .collect();
    assert_eq!(
        served_inner, admitted_bytes_in_order,
        "every served Block{{bytes}}, once tag-24 unwrapped, must equal the admitted AcceptedBlock.as_bytes() at the same key"
    );
}

#[test]
fn session_transcript_announced_header_matches_served_body_recipe() {
    // DC-CONS-18: every RollForward{header} in the transcript matches
    // the canonical header projection accepted_block_header_bytes
    // over the same AcceptedBlock the subsequent block-fetch range
    // serves.
    let arrivals = build_accepted_n(1);
    let cs_inputs = vec![ChainSyncMessage::RequestNext];
    // After chain-sync, query the same block via block-fetch.
    let decoded0 = decode_block(arrivals[0].as_bytes()).expect("decode");
    let key0 = (decoded0.header_input.slot, decoded0.block_hash.clone());
    let bf_inputs = vec![BlockFetchMessage::RequestRange(Range {
        from: BlockFetchPoint::Block { slot: key0.0, hash: key0.1.clone() },
        to: BlockFetchPoint::Block { slot: key0.0, hash: key0.1.clone() },
    })];
    let frames = run_session(&arrivals, &cs_inputs, &bf_inputs);

    use ade_network::codec::block_fetch::decode_block_fetch_message;
    use ade_network::codec::chain_sync::decode_chain_sync_message;

    // Pull the RollForward header bytes.
    let rolled_header: Vec<u8> = frames
        .iter()
        .filter_map(|f| decode_chain_sync_message(f).ok())
        .find_map(|m| match m {
            ChainSyncMessage::RollForward { header, .. } => Some(header),
            _ => None,
        })
        .expect("RollForward in transcript");

    // The canonical header projection over the same AcceptedBlock
    // MUST equal the rolled bytes.
    let canonical = accepted_block_header_bytes(&arrivals[0])
        .expect("project")
        .to_vec();
    assert_eq!(
        rolled_header, canonical,
        "RollForward header must equal accepted_block_header_bytes (DC-CONS-18)"
    );

    // The served Block bytes, once tag-24 unwrapped (CN-WIRE-08),
    // MUST equal the AcceptedBlock bytes (DC-CONS-17 through the wrap).
    let served_body: Vec<u8> = frames
        .iter()
        .filter_map(|f| decode_block_fetch_message(f).ok())
        .find_map(|m| match m {
            BlockFetchMessage::Block { bytes } => Some(bytes),
            _ => None,
        })
        .expect("Block in transcript");
    let served_inner =
        ade_network::codec::block_fetch::decompose_blockfetch_block(&served_body)
            .expect("served payload is tag-24 wrapped");
    assert_eq!(
        served_inner,
        arrivals[0].as_bytes(),
        "Block bytes, tag-24 unwrapped, must equal AcceptedBlock.as_bytes() (DC-CONS-17)"
    );
}
