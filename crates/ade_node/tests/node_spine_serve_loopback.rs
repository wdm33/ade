// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration tests — PHASE4-N-U S3 (DC-NODE-13) + PHASE4-N-F-G-H/K
//! (DC-NODE-07 / DC-NODE-09).
//!
//! Hermetic node-spine serve-to-peer loopback over the DURABLE CHAIN
//! PROJECTION. A self-accepted corpus block is made durable in a ChainDb
//! (the same `StoredBlock` form `pump_block` writes), the node-spine serve
//! task (`run_node_serve_task`) is bound on an ephemeral `127.0.0.1:0`
//! port reading an `Arc<dyn ChainDb>`, and Ade's OWN consume client
//! (`dial_for_admission` + `run_admission_wire_pump`) acts as the
//! in-process follower: it completes the N2N handshake, discovers the
//! served tip via ChainSync, and BlockFetches the body. The fetched body
//! must be byte-identical to the durable block (DC-CONS-17).
//!
//! PHASE4-N-U S3: the serve source is the durable ChainDb PROJECTION
//! (`ChainDbServedSource`), NOT the retired `ServedChainHandle` /
//! `ServedChainView` accumulator. The durable chain is extend-only, so it
//! is coherent (A→B, never B without A) and serving survives restart. A
//! reducer-level test proves the coherent-history projection over two
//! durable blocks; a structural test proves the serve strictly follows
//! the durable chain (no phantom accumulator).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::producer::AcceptedBlock;
use ade_ledger::state::LedgerState;
use ade_network::block_fetch::server::{
    producer_block_fetch_serve, BlockFetchServerStep, ProducerBlockFetchServerState,
};
use ade_network::chain_sync::server::{
    producer_chain_sync_serve, ProducerChainSyncServerState, ServerStep,
};
use ade_network::codec::block_fetch::{
    compose_blockfetch_block, BlockFetchMessage, Point as BfPoint, Range,
};
use ade_network::codec::chain_sync::{ChainSyncMessage, Point};
use ade_network::codec::version::{BlockFetchVersion, ChainSyncVersion};
use ade_node::admission::bootstrap::build_n2n_version_table;
use ade_node::node_lifecycle::{bind_serve_listener, run_node_serve_task, ServeStartError};
use ade_runtime::admission::{dial_for_admission, run_admission_wire_pump, AdmissionPeerEvent};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb, StoredBlock};
use ade_runtime::network::ChainDbServedSource;
use ade_runtime::producer::chain_evolution::ChainEvolution;
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use tokio::sync::{mpsc, watch};

const EPOCH_576: EpochNo = EpochNo(576);
const EPOCH_577_START: u64 = 163_900_800;
const MAINNET_EPOCH_LENGTH: u64 = 432_000;

/// PHASE4-N-F-G-H S2b: a non-mainnet (C1-style) network magic. The node serve
/// listener advertises THIS magic (via `n2n_supported_for_magic`) and the
/// follower proposes the same — proving the serve handshake is magic-aware (a
/// mainnet-only serve table would refuse this peer).
const C1_MAGIC: u32 = 42;

// --- corpus -> self-accepted block helpers (mirror `produce_loopback.rs`) ---

fn schedule() -> EraSchedule {
    let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(start_576),
            start_epoch: EPOCH_576,
            slot_length_ms: 1_000,
            epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
            safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
        }],
    )
    .expect("schedule is well-formed")
}

fn corpus() -> ConwayValidityCorpus {
    ConwayValidityCorpus::load().expect("corpus loads")
}

fn view(c: &ConwayValidityCorpus) -> PoolDistrView {
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
    PoolDistrView::new(EPOCH_576, total, asc, pools)
}

fn ledger_at_576() -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l
}

fn chain_dep_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(Hash32(eta0));
    s.evolving_nonce = Nonce(Hash32(eta0));
    s
}

fn inner_span(env_bytes: &[u8]) -> (usize, usize) {
    let env = decode_block_envelope(env_bytes).expect("envelope decodes");
    (env.block_start, env.block_end)
}

fn pick_lightest(c: &ConwayValidityCorpus) -> &[u8] {
    let idx = (0..c.blocks.len())
        .min_by_key(|&i| {
            let (s, e) = inner_span(&c.blocks[i]);
            e - s
        })
        .expect("corpus is non-empty");
    &c.blocks[idx]
}

/// The two lightest corpus blocks, ordered ascending by slot. Used to build a
/// two-block durable chain (a lower-slot "ingested predecessor" A and a
/// higher-slot "forged successor" B) for the coherent-history projection test.
fn pick_two_lightest_by_slot(c: &ConwayValidityCorpus) -> (Vec<u8>, Vec<u8>) {
    let mut idxs: Vec<usize> = (0..c.blocks.len()).collect();
    idxs.sort_by_key(|&i| {
        let (s, e) = inner_span(&c.blocks[i]);
        e - s
    });
    let mut a = c.blocks[idxs[0]].clone();
    let mut b = c.blocks[idxs[1]].clone();
    let sa = decode_block(&a).expect("decode a").header_input.slot;
    let sb = decode_block(&b).expect("decode b").header_input.slot;
    assert_ne!(sa, sb, "the two corpus blocks must occupy distinct slots");
    if sa > sb {
        std::mem::swap(&mut a, &mut b);
    }
    (a, b)
}

fn seed_from_corpus(c: &ConwayValidityCorpus) -> ChainEvolution {
    ChainEvolution::seed(
        ledger_at_576(),
        chain_dep_with_eta0(c.epoch_nonce),
        None,
        schedule(),
        view(c),
        Nonce(Hash32(c.epoch_nonce)),
    )
}

/// Self-accept a corpus block through the BLUE block_validity + self_accept
/// authorities (inside `advance`).
fn self_accept_corpus_block(c: &ConwayValidityCorpus, block_bytes: &[u8]) -> AcceptedBlock {
    let (_evo, accepted) = seed_from_corpus(c)
        .advance(block_bytes)
        .expect("validator-accepted corpus block advances");
    accepted
}

/// Make a self-accepted block DURABLE in an in-memory ChainDb the same way
/// `pump_block` does: a `StoredBlock { hash, slot, bytes }` keyed by the decoded
/// `(slot, block_hash)`, bytes = `accepted.as_bytes()` verbatim. This is the
/// test's stand-in for S1's durable admit; the serve task then PROJECTS this
/// durable chain (DC-NODE-13). Returns an `Arc<dyn ChainDb>` ready for
/// `run_node_serve_task`.
fn durable_chaindb(accepted: &[&AcceptedBlock]) -> Arc<dyn ChainDb> {
    let db = InMemoryChainDb::new();
    for acc in accepted {
        let decoded = decode_block(acc.as_bytes()).expect("durable block decodes");
        db.put_block(&StoredBlock {
            hash: decoded.block_hash,
            slot: decoded.header_input.slot,
            bytes: acc.as_bytes().to_vec(),
        })
        .expect("put_block");
    }
    Arc::new(db)
}

// --- tests ---

/// CE-7 (DC-NODE-13): the node-spine serve task PROJECTS the durable chain to a
/// real (in-process) N2N follower via ChainSync + BlockFetch; the fetched body
/// is byte-identical to the durable block (DC-CONS-17). The block is served
/// because it is DURABLE — not because a live `AcceptedBlock` token is held in an
/// accumulator (the retired G-R path).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn served_view_projects_durable_chain() {
    let corpus = corpus();
    let block_bytes = pick_lightest(&corpus).to_vec();
    let accepted = self_accept_corpus_block(&corpus, &block_bytes);

    // Make the self-accepted block durable, then serve the durable projection.
    let serve_chaindb = durable_chaindb(&[&accepted]);

    let listener = bind_serve_listener("127.0.0.1:0")
        .await
        .expect("bind node-spine serve listener");
    let serve_addr = listener.local_addr().expect("serve local addr");
    let (stop_tx, stop_rx) = watch::channel(false);
    let serve = tokio::spawn(run_node_serve_task(listener, serve_chaindb, C1_MAGIC, stop_rx));

    // Follower = Ade's OWN consume client (dial + N2N handshake + chain-sync +
    // block-fetch). The serve advertises the C1 magic via n2n_supported_for_magic
    // (S2b), so the follower proposes the same NON-mainnet magic.
    let our_versions = build_n2n_version_table(C1_MAGIC);
    let (transport, version) = dial_for_admission(serve_addr, our_versions)
        .await
        .expect("follower dials + N2N-handshakes the node-spine serve");
    let (ev_tx, mut ev_rx) = mpsc::channel::<AdmissionPeerEvent>(64);
    tokio::spawn(run_admission_wire_pump(
        transport,
        serve_addr.to_string(),
        Point::Origin,
        version,
        C1_MAGIC,
        ev_tx,
    ));

    let fetched = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match ev_rx.recv().await {
                Some(AdmissionPeerEvent::Block { block_bytes, .. }) => return Some(block_bytes),
                Some(_) => continue,
                None => return None,
            }
        }
    })
    .await
    .expect("follower receives a block-fetch reply within the timeout")
    .expect("follower received a Block event (not a premature disconnect)");

    assert_eq!(
        &fetched[..],
        &block_bytes[..],
        "the body the follower block-fetched (tag-24 stripped at the receive boundary, CN-WIRE-12) \
         must be byte-identical to the DURABLE block served by the projection (DC-CONS-17)"
    );

    let _ = stop_tx.send(true);
    let _ = serve.await;
}

/// CE-7 (DC-NODE-13): coherent-history projection. Given a durable chain A→B (a
/// lower-slot ingested predecessor A and a higher-slot successor B), the serve
/// projection over the durable ChainDb advertises A before B (`next_after` walk:
/// None→A→B→None) and a BlockFetch RequestRange[A,B] returns [StartBatch,
/// Block(A), Block(B), BatchDone] in order — never B without A. Reducer-level so
/// it is deterministic over the real corpus bytes (no async-follower timing).
#[test]
fn follower_fetches_coherent_history_incl_ingested_predecessor() {
    let corpus = corpus();
    let (bytes_a, bytes_b) = pick_two_lightest_by_slot(&corpus);
    let acc_a = self_accept_corpus_block(&corpus, &bytes_a);
    let acc_b = self_accept_corpus_block(&corpus, &bytes_b);
    let serve_chaindb = durable_chaindb(&[&acc_a, &acc_b]);
    let src = ChainDbServedSource::new(serve_chaindb.as_ref());

    let da = decode_block(&bytes_a).expect("decode a");
    let db = decode_block(&bytes_b).expect("decode b");
    let key_a = (da.header_input.slot, da.block_hash.clone());
    let key_b = (db.header_input.slot, db.block_hash.clone());

    // ChainSync next_after walk over the projection: A is advertised first
    // (lowest key), then B, then nothing — a follower cannot reach B without
    // first crossing A.
    use ade_network::chain_sync::server::ServedHeaderLookup;
    let first = src.next_after(None).expect("A is advertised first");
    assert_eq!((first.slot, first.hash.clone()), key_a, "lowest durable key is A");
    let second = src
        .next_after(Some(key_a.clone()))
        .expect("B is advertised after A");
    assert_eq!((second.slot, second.hash.clone()), key_b, "next after A is B");
    assert!(
        src.next_after(Some(key_b.clone())).is_none(),
        "nothing is advertised past the durable tip B"
    );

    // BlockFetch RequestRange[A,B] over the projection: StartBatch, Block(A),
    // Block(B), BatchDone — A precedes B; the payloads are the tag-24 wrap of the
    // durable bytes (DC-CONS-17 / CN-WIRE-08).
    let state = ProducerBlockFetchServerState::new();
    let (_st, step) = producer_block_fetch_serve(
        state,
        BlockFetchMessage::RequestRange(Range {
            from: BfPoint::Block { slot: key_a.0, hash: key_a.1.clone() },
            to: BfPoint::Block { slot: key_b.0, hash: key_b.1.clone() },
        }),
        &src,
        BlockFetchVersion::new(9),
    )
    .expect("range serve");
    match step {
        BlockFetchServerStep::Replies(replies) => {
            let msgs: Vec<BlockFetchMessage> = replies.into_iter().map(|r| r.into_message()).collect();
            assert!(
                matches!(msgs.first(), Some(BlockFetchMessage::StartBatch)),
                "first reply is StartBatch"
            );
            assert!(
                matches!(msgs.last(), Some(BlockFetchMessage::BatchDone)),
                "last reply is BatchDone"
            );
            let blocks: Vec<&Vec<u8>> = msgs
                .iter()
                .filter_map(|m| match m {
                    BlockFetchMessage::Block { bytes } => Some(bytes),
                    _ => None,
                })
                .collect();
            assert_eq!(blocks.len(), 2, "exactly A and B are served (coherent A→B)");
            assert_eq!(
                blocks[0], &compose_blockfetch_block(&bytes_a),
                "A is served first (never B without A)"
            );
            assert_eq!(
                blocks[1], &compose_blockfetch_block(&bytes_b),
                "B is served second, after A"
            );
        }
        other => panic!("expected Replies, got {other:?}"),
    }
}

/// CE-7 (DC-NODE-13): the served view STRICTLY follows the durable chain — the
/// G-R in-memory accumulator is retired. The serve projection over an EMPTY
/// durable ChainDb advertises nothing (no tip, no next, an out-of-chain point
/// does not intersect, a range yields no blocks): there is no phantom accumulator
/// holding blocks that are not durable. A block is served iff it is durable.
#[test]
fn served_view_retires_accumulator() {
    use ade_network::chain_sync::server::ServedHeaderLookup;

    let empty = InMemoryChainDb::new();
    let src = ChainDbServedSource::new(&empty);
    assert!(src.tip().is_none(), "empty durable chain -> no served tip");
    assert!(src.next_after(None).is_none(), "empty durable chain -> nothing advertised");
    assert!(
        src.intersect(&[Point::Block { slot: SlotNo(123), hash: Hash32([7u8; 32]) }]).is_none(),
        "a non-durable point does not intersect (no accumulator)"
    );

    // A chain-sync RequestNext over the empty projection parks (AwaitReply) — it
    // never invents a block, because the serve source IS the durable chain.
    let (_st, step) = producer_chain_sync_serve(
        ProducerChainSyncServerState::new(),
        ChainSyncMessage::RequestNext,
        &src,
        ChainSyncVersion::new(9),
    )
    .expect("serve");
    match step {
        ServerStep::Reply(reply) => assert!(
            matches!(reply.into_message(), ChainSyncMessage::AwaitReply),
            "empty durable chain -> AwaitReply (no phantom block)"
        ),
        other => panic!("expected Reply, got {other:?}"),
    }
}

/// CE-G-H-2 ("no silent live-serve claim"): a serve-start bind failure under
/// `--listen` is surfaced as a STRUCTURED `ServeStartError`, never silently
/// swallowed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_serve_start_failure_is_surfaced_not_silent() {
    let occupier = bind_serve_listener("127.0.0.1:0")
        .await
        .expect("occupy an ephemeral port");
    let occupied = occupier.local_addr().expect("occupied local addr").to_string();

    match bind_serve_listener(&occupied).await {
        Err(ServeStartError::Bind(_)) => { /* expected: surfaced, fail-closed */ }
        Err(other) => panic!("expected ServeStartError::Bind on an occupied port, got {other:?}"),
        Ok(_) => panic!(
            "serve-start bind on an occupied port MUST fail (no silent live-serve claim) — \
             a second listener bound the same active port"
        ),
    }

    match bind_serve_listener("definitely-not-a-socket-addr").await {
        Err(ServeStartError::InvalidAddr(_)) => {}
        other => panic!("expected ServeStartError::InvalidAddr, got {other:?}"),
    }

    drop(occupier);
}

// --- PHASE4-N-F-G-K S1 (DC-NODE-09): serve lifetime decoupled from feed end ---

/// CE-G-K-1 (DC-NODE-09): the node-spine serve task's lifetime is owned by the
/// shutdown watch, NOT the upstream feed. With the shutdown watch held FALSE (the
/// feed-end case), the serve task stays alive and a follower that dials LATE still
/// BlockFetches the durable block from the projection.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn serve_task_outlives_feed_end_and_serves_late_fetch() {
    let corpus = corpus();
    let block_bytes = pick_lightest(&corpus).to_vec();
    let accepted = self_accept_corpus_block(&corpus, &block_bytes);
    let serve_chaindb = durable_chaindb(&[&accepted]);

    let listener = bind_serve_listener("127.0.0.1:0")
        .await
        .expect("bind node-spine serve listener");
    let serve_addr = listener.local_addr().expect("serve local addr");
    // This watch stands in for the operator `shutdown` watch the On-arm clones
    // into the serve task (DC-NODE-09). It stays FALSE across the feed-end + the
    // late fetch — the serve task must NOT terminate on its own.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let serve = tokio::spawn(run_node_serve_task(listener, serve_chaindb, C1_MAGIC, shutdown_rx));

    tokio::task::yield_now().await;
    assert!(
        !serve.is_finished(),
        "DC-NODE-09: a clean feed-end (shutdown false) must NOT terminate the serve task"
    );

    let our_versions = build_n2n_version_table(C1_MAGIC);
    let (transport, version) = dial_for_admission(serve_addr, our_versions)
        .await
        .expect("late follower dials + N2N-handshakes the still-alive serve");
    let (ev_tx, mut ev_rx) = mpsc::channel::<AdmissionPeerEvent>(64);
    tokio::spawn(run_admission_wire_pump(
        transport,
        serve_addr.to_string(),
        Point::Origin,
        version,
        C1_MAGIC,
        ev_tx,
    ));
    let fetched = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match ev_rx.recv().await {
                Some(AdmissionPeerEvent::Block { block_bytes, .. }) => return Some(block_bytes),
                Some(_) => continue,
                None => return None,
            }
        }
    })
    .await
    .expect("late follower receives a block-fetch reply within the timeout")
    .expect("late follower received a Block event (not a premature disconnect)");
    assert_eq!(
        &fetched[..],
        &block_bytes[..],
        "the late follower fetched the durable block byte-identically \
         (tag-24 stripped at the receive boundary, CN-WIRE-12)"
    );

    assert!(
        !serve.is_finished(),
        "DC-NODE-09: the serve task remains alive after a late fetch (no feed-end coupling)"
    );

    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(5), serve)
        .await
        .expect("serve task terminates promptly on shutdown (no hang)")
        .expect("serve task joins cleanly");
}

/// CE-G-K-2 (DC-NODE-09): the longer-lived serve task terminates cleanly when the
/// operator shutdown watch flips — no hang, no leaked task.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serve_task_terminates_on_shutdown_no_hang() {
    let serve_chaindb: Arc<dyn ChainDb> = Arc::new(InMemoryChainDb::new());
    let listener = bind_serve_listener("127.0.0.1:0")
        .await
        .expect("bind node-spine serve listener");
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let serve = tokio::spawn(run_node_serve_task(listener, serve_chaindb, C1_MAGIC, shutdown_rx));

    tokio::task::yield_now().await;
    assert!(
        !serve.is_finished(),
        "serve task must stay alive until shutdown (not tied to feed/peer presence)"
    );

    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(5), serve)
        .await
        .expect("serve task terminates within the timeout on shutdown (no hang)")
        .expect("serve task joins cleanly (no leak)");
}
