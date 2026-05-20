// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN tx-validity test harness (PHASE4-B2, B2-S3).
//!
//! Non-authoritative: extracts every on-wire Conway transaction from the
//! committed Conway-576 corpus blocks (see [`extract`]) and drives the BLUE
//! [`ade_ledger::tx_validity::tx_validity`] over each, collecting the verdict
//! plus its canonical verdict-surface bytes. Each tx is judged against a fresh
//! Conway `LedgerState` at epoch 576 with `track_utxo = false` — the SAME scope
//! as B1's positive corpus (structural + supplied-witness sig validity +
//! tx-derived required-signer coverage; input-credential coverage needs the
//! UTxO and is proven synthetically in B2-S1). It asserts nothing itself — the
//! calling tests assert agreement and replay-equivalence.

pub mod extract;

pub use extract::{extract_block_txs, extract_corpus_txs, ExtractError, ExtractedTx};

use ade_ledger::state::LedgerState;
use ade_ledger::tx_validity::{encode_tx_verdict_surface, tx_validity, TxValidityVerdict};
use ade_types::{CardanoEra, EpochNo};

const EPOCH_576: EpochNo = EpochNo(576);

/// One tx's replay outcome: the verdict and its canonical surface bytes.
pub struct TxReplay {
    pub block_index: usize,
    pub tx_index: usize,
    pub is_valid_flag: bool,
    pub verdict: TxValidityVerdict,
    pub surface: Vec<u8>,
}

/// Ledger state pre-positioned at epoch 576 (structural validation,
/// `track_utxo = false` — same boundary as B1's positive corpus). Each tx is
/// validated against a fresh clone so corpus ordering does not couple txs.
fn ledger_at_576() -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l
}

/// Drive `tx_validity` over a single arbitrary on-wire Conway tx against the
/// corpus's epoch-576 Conway ledger (`track_utxo = false`). Returns the verdict
/// and its canonical surface bytes. Used by the calling tests and (later) the
/// B2-S4 adversarial harness so every tx is judged by the same recipe.
pub fn validate_tx_against_corpus(extracted: &ExtractedTx) -> TxReplay {
    let ledger = ledger_at_576();
    let outcome = tx_validity(&ledger, &extracted.tx_cbor);
    let surface = encode_tx_verdict_surface(&outcome.verdict);
    TxReplay {
        block_index: extracted.block_index,
        tx_index: extracted.tx_index,
        is_valid_flag: extracted.is_valid,
        verdict: outcome.verdict,
        surface,
    }
}

/// Extract every tx from the corpus blocks and drive `tx_validity` over each,
/// in `(block_index, tx_index)` order. Returns one [`TxReplay`] per tx.
pub fn replay_tx_validity(blocks: &[Vec<u8>]) -> Result<Vec<TxReplay>, ExtractError> {
    let txs = extract_corpus_txs(blocks)?;
    Ok(txs.iter().map(validate_tx_against_corpus).collect())
}
