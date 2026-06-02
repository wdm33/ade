// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration tests — PHASE4-N-F-G-H S2 (DC-NODE-07).
//!
//! Hermetic node-spine serve-to-peer loopback. A self-accepted corpus block is
//! pre-loaded into a `ServedChainView` (via the single `ServedChainHandle::
//! push_atomic` authority), the node-spine serve sibling (`run_node_serve_task`)
//! is bound on an ephemeral `127.0.0.1:0` port, and Ade's OWN consume client
//! (`dial_for_admission` + `run_admission_wire_pump`) acts as the in-process
//! follower: it completes the N2N handshake, discovers the served tip via
//! ChainSync, and BlockFetches the body. The fetched body must be byte-identical
//! to the served self-accepted block (DC-CONS-17). The follower's `RequestNext`
//! arrives AFTER the block is already in the served view — the request-driven
//! case (no proactive `advance_tip`; that is out of S2 scope, a separate cluster
//! if a real C1 follower proves it necessary).
//!
//! A second test proves the "no silent live-serve claim" invariant: a serve-start
//! bind failure under `--listen` is surfaced as a STRUCTURED `ServeStartError`,
//! never silently swallowed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::time::Duration;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::state::LedgerState;
use ade_network::codec::block_fetch::decompose_blockfetch_block;
use ade_network::codec::chain_sync::Point;
use ade_network::handshake::version_table::MAINNET_NETWORK_MAGIC;
use ade_node::admission::bootstrap::build_n2n_version_table;
use ade_node::node_lifecycle::{bind_serve_listener, run_node_serve_task, ServeStartError};
use ade_runtime::admission::{dial_for_admission, run_admission_wire_pump, AdmissionPeerEvent};
use ade_runtime::producer::chain_evolution::ChainEvolution;
use ade_runtime::producer::served_chain_handle::ServedChainHandle;
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};
use tokio::sync::{mpsc, watch};

const EPOCH_576: EpochNo = EpochNo(576);
const EPOCH_577_START: u64 = 163_900_800;
const MAINNET_EPOCH_LENGTH: u64 = 432_000;

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

// --- tests ---

/// CE-G-H-2: the node-spine serve sibling serves an already-served self-accepted
/// block to a real (in-process) N2N follower via ChainSync + BlockFetch; the
/// fetched body is byte-identical to the served block (DC-CONS-17).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn node_spine_serve_loopback_follower_fetches_self_accepted_block() {
    let corpus = corpus();
    let block_bytes = pick_lightest(&corpus).to_vec();

    // Real self-accept path: the corpus block threaded through the BLUE
    // block_validity + self_accept authorities inside `advance`.
    let (_evo, accepted) = seed_from_corpus(&corpus)
        .advance(&block_bytes)
        .expect("validator-accepted corpus block advances");

    // Pre-load the served view BEFORE the follower requests (request-driven: the
    // block is already present at request time). `handle` stays alive for the
    // whole test (it owns the watch sender feeding the view the serve reads).
    let (handle, serve_view) = ServedChainHandle::new();
    let _tip = handle.push_atomic(accepted).expect("push_atomic");

    // Node-spine serve: bind (the fail-fast serve-start surface) on an ephemeral
    // port, then spawn the serve sibling reading the served view.
    let listener = bind_serve_listener("127.0.0.1:0")
        .await
        .expect("bind node-spine serve listener");
    let serve_addr = listener.local_addr().expect("serve local addr");
    let (stop_tx, stop_rx) = watch::channel(false);
    let serve = tokio::spawn(run_node_serve_task(listener, serve_view, stop_rx));

    // Follower = Ade's OWN consume client (dial + N2N handshake + chain-sync +
    // block-fetch). The serve advertises the static `N2N_SUPPORTED` responder
    // table (mainnet magic), so the follower proposes the matching magic.
    let our_versions = build_n2n_version_table(MAINNET_NETWORK_MAGIC);
    let (transport, version) = dial_for_admission(serve_addr, our_versions)
        .await
        .expect("follower dials + N2N-handshakes the node-spine serve");
    let (ev_tx, mut ev_rx) = mpsc::channel::<AdmissionPeerEvent>(64);
    tokio::spawn(run_admission_wire_pump(
        transport,
        serve_addr.to_string(),
        Point::Origin,
        version,
        MAINNET_NETWORK_MAGIC,
        ev_tx,
    ));

    // The follower discovers the served tip via ChainSync (IntersectFound /
    // RollForward) and fetches the body via BlockFetch -> emits Block{bytes}.
    let fetched = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            match ev_rx.recv().await {
                Some(AdmissionPeerEvent::Block { block_bytes, .. }) => return Some(block_bytes),
                // Drain TipUpdate / Disconnected; keep waiting for the body.
                Some(_) => continue,
                None => return None,
            }
        }
    })
    .await
    .expect("follower receives a block-fetch reply within the timeout")
    .expect("follower received a Block event (not a premature disconnect)");

    // The wire pump emits the raw `MsgBlock` payload (tag-24-wrapped per
    // CN-WIRE-08); unwrap it and assert the inner block is byte-identical to the
    // served self-accepted block (DC-CONS-17), mirroring produce_loopback's
    // `block_fetch_payload_is_self_accepted_bytes`.
    let inner = decompose_blockfetch_block(&fetched)
        .expect("the follower's block-fetch payload is tag-24-wrapped (CN-WIRE-08)");
    assert_eq!(
        inner,
        &block_bytes[..],
        "the body the follower block-fetched (tag-24-unwrapped) must be \
         byte-identical to the served self-accepted block (DC-CONS-17)"
    );

    let _ = stop_tx.send(true);
    let _ = serve.await;
    // `handle` is dropped here (after the serve task has stopped).
    drop(handle);
}

/// CE-G-H-2 ("no silent live-serve claim"): a serve-start bind failure under
/// `--listen` is surfaced as a STRUCTURED `ServeStartError`, never silently
/// swallowed. The On-arm fail-fasts on this (it never proceeds claiming live
/// serve capability while serving is disabled).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn node_serve_start_failure_is_surfaced_not_silent() {
    // Occupy a port, then attempt to bind the node serve on the SAME address.
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

    // A non-parseable `--listen` value is surfaced as a structured InvalidAddr.
    match bind_serve_listener("definitely-not-a-socket-addr").await {
        Err(ServeStartError::InvalidAddr(_)) => {}
        other => panic!("expected ServeStartError::InvalidAddr, got {other:?}"),
    }

    drop(occupier);
}
