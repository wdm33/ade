// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-T S5 — loopback integration test (CE-T-11 / DC-PROD-03).
//!
//! Proves N-T's end-to-end wiring: a self-acceptable block, threaded
//! through the real `ChainEvolution::advance` path and pushed via
//! `ServedChainHandle::push_atomic`, is present in the served snapshot
//! and reads back byte-identically through the serve-path lookup
//! `ServedChainSnapshot::block_bytes(slot, &hash)`, with two-run replay
//! determinism over the served-snapshot fingerprint.
//!
//! Tier achieved: **Tier B** (S5.md fallback). Tier A — driving
//! `produce_mode::run_real_forge` to `ForgeSucceeded` with a synthetic
//! eligible-leader fixture — was attempted with the consistent setup
//! S5.md §"Tier A" prescribes (VRF keyhash binding =
//! `blake2b_256(vrf_vk)`, ASC 1/1 + positive stake, base ledger /
//! pool_distr / era_schedule all at epoch 0, `chain_dep.epoch_nonce ==
//! ctx.eta0`). It did NOT self-accept: the placeholder first-pass forge
//! emits an empty-transaction-set Conway body that fails to re-decode
//! (`decode_block` -> `Body(Decoding(InvalidStructure))` at body offset
//! 0) *before* the KES-signs-unsigned-header second pass — the empty-body
//! forge structure is not yet round-trippable through the in-process
//! Conway block decoder. This is the N-R-A honest residual (forged
//! empty-body self-acceptance remains unproven in-process); fixing it
//! would require touching BLUE forge authorities, which S5 forbids.
//! A corpus block is a real, validator-accepted block — equivalent to a
//! forged one for the serve+fetch+replay path proven here.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::state::LedgerState;
use ade_network::block_fetch::server::{
    producer_block_fetch_serve, BlockFetchServerStep, ProducerBlockFetchServerState,
};
use ade_network::codec::block_fetch::{
    decompose_blockfetch_block, BlockFetchMessage, Point, Range,
};
use ade_network::codec::version::BlockFetchVersion;
use ade_runtime::producer::chain_evolution::ChainEvolution;
use ade_runtime::producer::self_accepted_handoff::SelfAcceptedHandoff;
use ade_runtime::producer::served_chain_handle::{ServedChainHandle, ServedChainView};
use ade_runtime::producer::served_chain_lookups::ServedChainLookups;
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

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
    .expect("schedule is well-formed")
}

fn corpus() -> ConwayValidityCorpus {
    ConwayValidityCorpus::load().expect("corpus loads")
}

/// Project the corpus into the in-crate `PoolDistrView` — the same
/// recipe used by `self_accept.rs`/`chain_evolution.rs` test helpers.
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

#[test]
fn forge_to_served_block_fetch_roundtrip() {
    let corpus = corpus();
    let block_bytes = pick_lightest(&corpus).to_vec();

    // Real advance path: self_accept-cleared block threaded through the
    // BLUE block_validity + self_accept authorities inside `advance`.
    let (_evo2, accepted) = seed_from_corpus(&corpus)
        .advance(&block_bytes)
        .expect("validator-accepted corpus block advances");

    // Single push_atomic authority — the same lookup the block-fetch
    // serve path uses.
    let (handle, view) = ServedChainHandle::new();
    let tip = handle.push_atomic(accepted).expect("push");

    let snap = view.borrow();
    let got = snap.block_bytes(tip.slot, &tip.hash).expect("present");
    assert_eq!(
        got,
        &block_bytes[..],
        "served block reads back byte-identically through block_bytes(slot, &hash)"
    );

    // The served key is exactly the block's own decoded (slot, hash).
    let decoded = decode_block(&block_bytes).expect("decode");
    assert_eq!(tip.slot, decoded.header_input.slot);
    assert_eq!(tip.hash, decoded.block_hash);
}

#[test]
fn served_snapshot_two_run_replay_byte_identical() {
    let corpus = corpus();
    let block_bytes = pick_lightest(&corpus).to_vec();

    let run = |bytes: &[u8]| -> Hash32 {
        let (_evo, accepted) = seed_from_corpus(&corpus)
            .advance(bytes)
            .expect("corpus block advances");
        let (handle, view) = ServedChainHandle::new();
        handle.push_atomic(accepted).expect("push");
        let snap = view.borrow();
        snap.fingerprint()
    };

    let fp_a = run(&block_bytes);
    let fp_b = run(&block_bytes);
    assert_eq!(
        fp_a, fp_b,
        "two seed->advance->push runs must yield byte-identical served-snapshot fingerprints"
    );
}

// =========================================================================
// PHASE4-N-F-G-B S3 — node-spine block-fetch payload proof (hermetic loopback)
// =========================================================================

/// Serve a single `(slot, hash)` over a node-spine `ServedChainView` via the
/// reused block-fetch reducer + `ServedChainLookups`, returning the `MsgBlock`
/// wire payload (tag-24 wrapped). Hermetic: drives `producer_block_fetch_serve`
/// directly — no real listener / socket / peer (that is G-C).
fn serve_block_fetch_payload(view: &ServedChainView, slot: SlotNo, hash: Hash32) -> Vec<u8> {
    let snap = view.borrow();
    let look = ServedChainLookups { snap: &snap };
    let range = Range {
        from: Point::Block {
            slot,
            hash: hash.clone(),
        },
        to: Point::Block { slot, hash },
    };
    let (_state, step) = producer_block_fetch_serve(
        ProducerBlockFetchServerState::new(),
        BlockFetchMessage::RequestRange(range),
        &look,
        BlockFetchVersion::new(9),
    )
    .expect("serve");
    match step {
        BlockFetchServerStep::Replies(replies) => replies
            .into_iter()
            .find_map(|r| match r.into_message() {
                BlockFetchMessage::Block { bytes } => Some(bytes),
                _ => None,
            })
            .expect("a Block reply for the served range"),
        other => panic!("expected Replies, got {other:?}"),
    }
}

#[test]
fn block_fetch_payload_is_self_accepted_bytes() {
    // Admit a self-accepted block via the S2 node-spine path
    // (SelfAcceptedHandoff::into_accepted() -> push_atomic), then serve it via
    // the reused block-fetch reducer over the node-spine view; the
    // tag-24-unwrapped payload is the self-accept input bytes byte-for-byte.
    let corpus = corpus();
    let block_bytes = pick_lightest(&corpus).to_vec();
    let (_evo, accepted) = seed_from_corpus(&corpus)
        .advance(&block_bytes)
        .expect("corpus block self-accepts + advances");

    let handoff = SelfAcceptedHandoff::from_self_accepted(accepted);
    let (handle, view) = ServedChainHandle::new();
    let tip = handle
        .push_atomic(handoff.into_accepted())
        .expect("node-spine admit via into_accepted()");

    let payload = serve_block_fetch_payload(&view, tip.slot, tip.hash.clone());
    let inner = decompose_blockfetch_block(&payload).expect("served payload is tag24-wrapped");
    assert_eq!(
        inner,
        &block_bytes[..],
        "served block-fetch payload (tag24-unwrapped) is the self-accept input bytes"
    );
}

#[test]
fn block_fetch_tag24_round_trips_to_self_accept_input() {
    // The served MsgBlock payload is bare tag-24 (0xd8 0x18); its inner bytes
    // decode via the canonical block-envelope authority back to the self-accept
    // input (wrap<->decode symmetry on the node spine).
    let corpus = corpus();
    let block_bytes = pick_lightest(&corpus).to_vec();
    let (_evo, accepted) = seed_from_corpus(&corpus)
        .advance(&block_bytes)
        .expect("corpus block self-accepts + advances");

    let handoff = SelfAcceptedHandoff::from_self_accepted(accepted);
    let (handle, view) = ServedChainHandle::new();
    let tip = handle
        .push_atomic(handoff.into_accepted())
        .expect("node-spine admit via into_accepted()");

    let payload = serve_block_fetch_payload(&view, tip.slot, tip.hash.clone());
    assert_eq!(
        &payload[0..2],
        &[0xd8, 0x18],
        "served payload must start with tag(24)"
    );
    let inner = decompose_blockfetch_block(&payload).expect("tag24 unwrap");
    let env = decode_block_envelope(inner).expect("inner decodes as [era, block]");
    assert_eq!(env.era, CardanoEra::Conway);
    assert_eq!(inner, &block_bytes[..]);
}
