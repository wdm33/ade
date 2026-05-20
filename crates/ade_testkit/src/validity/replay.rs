// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN replay harness (PHASE4-B1, B1-S6).
//!
//! Non-authoritative: drives the BLUE `block_validity` authority over every
//! block in the committed Conway-576 positive corpus and collects, per block,
//! the verdict plus its canonical verdict-surface bytes. Each block is
//! validated independently (the chain-dep state is reseeded per block with
//! `last_slot = None`, so corpus ordering does not couple the blocks). This is
//! the positive replay surface for `T-DET-01`/`DC-VAL-04`; it asserts nothing
//! itself — the calling tests assert agreement and replay-equivalence.

use ade_core::consensus::{BootstrapAnchorHash, EraSchedule, EraSummary, Nonce, PraosChainDepState};
use ade_ledger::block_validity::{block_validity, encode_verdict_surface, BlockValidityVerdict};
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::state::LedgerState;
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

use super::{pool_distr_view_from_corpus, ConwayValidityCorpus, CorpusViewError};

const EPOCH_576: EpochNo = EpochNo(576);
const EPOCH_577_START: u64 = 163_900_800;
const MAINNET_EPOCH_LENGTH: u64 = 432_000;

/// One block's replay outcome: the verdict and its canonical surface bytes.
pub struct BlockReplay {
    pub verdict: BlockValidityVerdict,
    pub surface: Vec<u8>,
}

/// The mainnet Conway era schedule positioned at epoch 576 — identical to the
/// B1-S4 composition test's schedule.
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

/// Praos state seeded with the corpus epoch nonce; `last_slot = None` so each
/// corpus block validates independently of the others.
fn state_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(Hash32(eta0));
    s.evolving_nonce = Nonce(Hash32(eta0));
    s
}

/// Ledger state pre-positioned at epoch 576 (structural body validation,
/// `track_utxo = false`); avoids firing an epoch-boundary transition.
fn ledger_at_576() -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l
}

/// Validate a single arbitrary block (e.g. an adversarial mutation of a corpus
/// block) against the corpus's per-block consensus recipe — the SAME inputs
/// `replay_block_validity` uses (eta0(576), epoch-576 ledger, the corpus
/// pool-distribution view, the mainnet Conway schedule). Returns the verdict
/// and its canonical surface bytes. Used by the B1-S7 adversarial harness so
/// every mutation is judged by exactly the recipe the positive corpus passes.
pub fn validate_block_against_corpus(
    corpus: &ConwayValidityCorpus,
    block_cbor: &[u8],
) -> Result<BlockReplay, CorpusViewError> {
    let view: PoolDistrView = pool_distr_view_from_corpus(corpus, EPOCH_576)?;
    let era_schedule = schedule();
    let ledger = ledger_at_576();
    let chain_dep = state_with_eta0(corpus.epoch_nonce);
    let outcome = block_validity(&ledger, &chain_dep, &era_schedule, &view, block_cbor);
    let surface = encode_verdict_surface(&outcome.verdict);
    Ok(BlockReplay {
        verdict: outcome.verdict,
        surface,
    })
}

/// Drive `block_validity` over every corpus block, validating each independently
/// against the per-block-reseeded chain-dep state, the corpus pool-distribution
/// view, and the mainnet Conway era schedule. Returns one `BlockReplay` per
/// block in corpus order.
pub fn replay_block_validity(
    corpus: &ConwayValidityCorpus,
) -> Result<Vec<BlockReplay>, CorpusViewError> {
    let view: PoolDistrView = pool_distr_view_from_corpus(corpus, EPOCH_576)?;
    let era_schedule = schedule();

    let mut out = Vec::with_capacity(corpus.blocks.len());
    for block_cbor in &corpus.blocks {
        let ledger = ledger_at_576();
        let chain_dep = state_with_eta0(corpus.epoch_nonce);
        let outcome = block_validity(&ledger, &chain_dep, &era_schedule, &view, block_cbor);
        let surface = encode_verdict_surface(&outcome.verdict);
        out.push(BlockReplay {
            verdict: outcome.verdict,
            surface,
        });
    }
    Ok(out)
}
