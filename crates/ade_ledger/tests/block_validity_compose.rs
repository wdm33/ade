// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// B1-S4 — `block_validity` composition. Proves:
//   - header-before-body fail-fast ordering (`DC-VAL-03`);
//   - the body-hash binding is a real WIRED check (E5 negative test,
//     `CN-CONS-04`);
//   - total state evolution: Invalid leaves both input states unchanged
//     (`DC-VAL-05`);
//   - a valid block evolves both authoritative states (`DC-VAL-02/05`).
//
// The 14 real Conway-576 blocks are the oracle (on-chain inclusion).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_core::consensus::{
    BootstrapAnchorHash, EraSchedule, EraSummary, Nonce, PraosChainDepState,
};
use ade_ledger::block_validity::{block_validity, BlockRejectClass, BlockValidityVerdict};
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::state::LedgerState;
use ade_testkit::validity::{pool_distr_view_from_corpus, ConwayValidityCorpus};
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

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
    EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
        .expect("schedule is well-formed")
}

/// Genesis-like Praos state seeded with the corpus epoch nonce; `last_slot`
/// is `None` so the first corpus block is admissible.
fn state_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(Hash32(eta0));
    s.evolving_nonce = Nonce(Hash32(eta0));
    s
}

/// Ledger state pre-positioned at epoch 576 so block application does not fire
/// an epoch-boundary transition (out of this slice's scope).
fn ledger_at_576() -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l
}

fn corpus() -> ConwayValidityCorpus {
    ConwayValidityCorpus::load().expect("corpus loads")
}

fn view(corpus: &ConwayValidityCorpus) -> PoolDistrView {
    pool_distr_view_from_corpus(corpus, EPOCH_576).expect("pool distr view")
}

/// The inner Conway block byte span within an envelope.
fn inner_span(env_bytes: &[u8]) -> (usize, usize) {
    let env = decode_block_envelope(env_bytes).expect("envelope decodes");
    (env.block_start, env.block_end)
}

/// Flip one byte inside the block body so the header is untouched but the
/// recomputed body hash changes. Scans the inner block from the tail (the body
/// segments follow the header) for the first flip that still decodes yet alters
/// the body hash — a content byte, never a CBOR length/type byte.
fn flip_body_byte(env_bytes: &[u8]) -> Vec<u8> {
    use ade_ledger::block_validity::decode_block;
    let (start, end) = inner_span(env_bytes);
    let base = decode_block(env_bytes).expect("base block decodes");
    for idx in (start..end).rev() {
        let mut bad = env_bytes.to_vec();
        bad[idx] ^= 0x01;
        if let Ok(d) = decode_block(&bad) {
            if d.computed_body_hash != base.computed_body_hash {
                return bad;
            }
        }
    }
    panic!("no structure-preserving body flip found");
}

#[test]
fn header_before_body_fail_fast() {
    // A chain-dep state whose `last_slot` is already past the block's slot
    // forces the header authority to reject (SlotBeforeLastApplied) BEFORE any
    // body work. Pair it with an altered body that would otherwise be rejected
    // by the body-hash binding / body authority — we must still see
    // HeaderInvalid, proving the body path is never reached.
    let corpus = corpus();
    let view = view(&corpus);
    let ledger = ledger_at_576();

    let mut chain_dep = state_with_eta0(corpus.epoch_nonce);
    chain_dep.last_slot = Some(SlotNo(u64::MAX - 1)); // past every corpus slot

    let altered = flip_body_byte(&corpus.blocks[0]);

    let outcome = block_validity(&ledger, &chain_dep, &schedule(), &view, &altered);
    match outcome.verdict {
        BlockValidityVerdict::Invalid { class, .. } => {
            assert_eq!(
                class,
                BlockRejectClass::HeaderInvalid,
                "header must fail first; body path must not run"
            );
        }
        BlockValidityVerdict::Valid { .. } => panic!("expected Invalid(HeaderInvalid)"),
    }
}

#[test]
fn altered_body_rejected_by_hash_binding() {
    // E5 negative test: a real corpus block with one body byte flipped (header
    // intact) must be rejected by the WIRED body-hash binding.
    let corpus = corpus();
    let view = view(&corpus);
    let ledger = ledger_at_576();
    let chain_dep = state_with_eta0(corpus.epoch_nonce);

    let altered = flip_body_byte(&corpus.blocks[0]);

    let outcome = block_validity(&ledger, &chain_dep, &schedule(), &view, &altered);
    match outcome.verdict {
        BlockValidityVerdict::Invalid { class, .. } => {
            assert_eq!(
                class,
                BlockRejectClass::BodyHashMismatch,
                "altered body must be caught by the body-hash binding"
            );
        }
        BlockValidityVerdict::Valid { .. } => {
            panic!("altered body must not be Valid — binding is unwired")
        }
    }
}

#[test]
fn invalid_block_leaves_state_unchanged() {
    // For an Invalid outcome both input states must be returned byte-identical.
    let corpus = corpus();
    let view = view(&corpus);
    let ledger = ledger_at_576();
    let chain_dep = state_with_eta0(corpus.epoch_nonce);

    let altered = flip_body_byte(&corpus.blocks[0]);

    let outcome = block_validity(&ledger, &chain_dep, &schedule(), &view, &altered);
    assert!(matches!(
        outcome.verdict,
        BlockValidityVerdict::Invalid { .. }
    ));
    assert_eq!(outcome.ledger, ledger, "ledger must be unchanged on Invalid");
    assert_eq!(
        outcome.chain_dep, chain_dep,
        "chain_dep must be unchanged on Invalid"
    );
}

#[test]
fn valid_block_evolves_both_states() {
    // The lightest corpus block, validated atop a fresh state, yields Valid and
    // advances both authoritative states.
    let corpus = corpus();
    let view = view(&corpus);

    // Pick the block whose inner CBOR is smallest (fewest/cheapest txs).
    let lightest = (0..corpus.blocks.len())
        .min_by_key(|&i| {
            let (s, e) = inner_span(&corpus.blocks[i]);
            e - s
        })
        .expect("corpus is non-empty");

    let ledger = ledger_at_576();
    let chain_dep = state_with_eta0(corpus.epoch_nonce);

    let outcome = block_validity(&ledger, &chain_dep, &schedule(), &view, &corpus.blocks[lightest]);

    match outcome.verdict {
        BlockValidityVerdict::Valid { block_no, .. } => {
            // chain_dep advanced past the input state.
            assert!(
                outcome.chain_dep.last_slot.is_some(),
                "chain_dep last_slot must advance"
            );
            assert_eq!(outcome.chain_dep.last_block_no, Some(block_no));
            assert_ne!(
                outcome.chain_dep, chain_dep,
                "chain_dep must evolve on Valid"
            );
            // ledger advanced its slot.
            assert_ne!(outcome.ledger, ledger, "ledger must evolve on Valid");
        }
        BlockValidityVerdict::Invalid { class, error } => {
            panic!("lightest corpus block must be Valid, got {class:?}: {error:?}");
        }
    }
}
