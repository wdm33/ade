// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN per-tx extraction from the committed Conway-576 corpus blocks.
//!
//! Non-authoritative test infrastructure. Decodes each corpus block, splits
//! its parallel transaction segments (`tx_bodies`, `witness_sets`, the
//! per-index `auxiliary_data` map, and the `invalid_transactions` index set),
//! and reassembles each on-wire Conway transaction
//! `[transaction_body, transaction_witness_set, is_valid, auxiliary_data/null]`
//! preserving the body bytes byte-for-byte so the BLUE `tx_validity` recomputes
//! `tx_id = blake2b_256(body_slice)` over the SAME bytes the chain hashed.
//!
//! `BTreeMap`/`Vec` only — no `HashMap`.

use std::collections::BTreeMap;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::cbor::{self, ContainerEncoding};
use ade_codec::conway::decode_conway_block;

/// One extracted on-wire Conway transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedTx {
    /// Source block index in corpus order.
    pub block_index: usize,
    /// Transaction index within the source block.
    pub tx_index: usize,
    /// The reassembled on-wire tx CBOR `[body, witness_set, is_valid, aux]`,
    /// with the body bytes preserved byte-for-byte from the block.
    pub tx_cbor: Vec<u8>,
    /// The `is_valid` flag (false iff the tx index is in the block's
    /// `invalid_transactions` set).
    pub is_valid: bool,
}

/// Errors raised while extracting txs from a corpus block.
#[derive(Debug)]
pub enum ExtractError {
    /// The era-tagged envelope or inner block failed to decode.
    BlockDecode(ade_codec::CodecError),
    /// A parallel segment (bodies / witness sets / aux map) failed to split.
    Segment(ade_codec::CodecError),
    /// The block carried a different number of witness sets than tx bodies.
    SegmentCountMismatch { bodies: usize, witness_sets: usize },
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractError::BlockDecode(e) => write!(f, "block decode: {e}"),
            ExtractError::Segment(e) => write!(f, "segment split: {e}"),
            ExtractError::SegmentCountMismatch {
                bodies,
                witness_sets,
            } => write!(
                f,
                "segment count mismatch: {bodies} bodies vs {witness_sets} witness sets"
            ),
        }
    }
}

impl std::error::Error for ExtractError {}

/// Split a CBOR array buffer into the preserved byte slices of its elements.
/// The buffer is exactly one array (the block's `tx_bodies` / `witness_sets`).
fn split_array_elements(data: &[u8]) -> Result<Vec<Vec<u8>>, ade_codec::CodecError> {
    let mut offset = 0usize;
    let enc = cbor::read_array_header(data, &mut offset)?;
    let mut out = Vec::new();
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let (s, e) = cbor::skip_item(data, &mut offset)?;
                out.push(data[s..e].to_vec());
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset)? {
                let (s, e) = cbor::skip_item(data, &mut offset)?;
                out.push(data[s..e].to_vec());
            }
        }
    }
    Ok(out)
}

/// Parse the block's auxiliary-data map (`{ tx_index => aux_data }`) into a
/// map from tx index to the preserved aux-data byte slice. A tx index with no
/// metadata is absent from the map (its on-wire `aux_data` is `null`).
fn split_aux_map(data: &[u8]) -> BTreeMap<u64, Vec<u8>> {
    let mut out: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    let mut offset = 0usize;
    let enc = match cbor::read_map_header(data, &mut offset) {
        Ok(e) => e,
        // A diagnostic split: a malformed/absent metadata map yields no aux
        // (every tx defaults to `null`), exactly as the chain encodes it.
        Err(_) => return out,
    };
    let read_pair = |off: &mut usize, out: &mut BTreeMap<u64, Vec<u8>>| -> bool {
        let key = match cbor::read_uint(data, off) {
            Ok((k, _)) => k,
            Err(_) => return false,
        };
        let (s, e) = match cbor::skip_item(data, off) {
            Ok(r) => r,
            Err(_) => return false,
        };
        out.insert(key, data[s..e].to_vec());
        true
    };
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                if !read_pair(&mut offset, &mut out) {
                    break;
                }
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset).unwrap_or(true) {
                if !read_pair(&mut offset, &mut out) {
                    break;
                }
            }
        }
    }
    out
}

/// Decode the `invalid_transactions` index set (Alonzo+ field 5).
fn invalid_indices(invalid_txs_cbor: Option<&[u8]>) -> std::collections::BTreeSet<u64> {
    ade_ledger::plutus_eval::decode_invalid_tx_indices(invalid_txs_cbor)
}

/// Extract every on-wire Conway transaction from a single era-tagged block
/// envelope, reassembling `[body, witness_set, is_valid, aux]` per index with
/// the body bytes preserved. `block_index` is the block's position in the
/// corpus stream (recorded on each [`ExtractedTx`]).
pub fn extract_block_txs(
    block_cbor: &[u8],
    block_index: usize,
) -> Result<Vec<ExtractedTx>, ExtractError> {
    // The corpus stores era-tagged `[era, block]` envelopes; strip the
    // envelope to the inner Conway block bytes before decoding.
    let env = decode_block_envelope(block_cbor).map_err(ExtractError::BlockDecode)?;
    let inner = &block_cbor[env.block_start..env.block_end];
    let preserved = decode_conway_block(inner).map_err(ExtractError::BlockDecode)?;
    let block = preserved.decoded();

    let bodies = split_array_elements(&block.tx_bodies).map_err(ExtractError::Segment)?;
    let witness_sets = split_array_elements(&block.witness_sets).map_err(ExtractError::Segment)?;
    if bodies.len() != witness_sets.len() {
        return Err(ExtractError::SegmentCountMismatch {
            bodies: bodies.len(),
            witness_sets: witness_sets.len(),
        });
    }
    let aux = split_aux_map(&block.metadata);
    let invalid = invalid_indices(block.invalid_txs.as_deref());

    let mut out = Vec::with_capacity(bodies.len());
    for (i, (body, witness_set)) in bodies.iter().zip(witness_sets.iter()).enumerate() {
        let is_valid = !invalid.contains(&(i as u64));
        let aux_slice = aux.get(&(i as u64)).map(|v| v.as_slice());
        let tx_cbor =
            ade_ledger::plutus_eval::assemble_full_tx_with(body, witness_set, is_valid, aux_slice);
        out.push(ExtractedTx {
            block_index,
            tx_index: i,
            tx_cbor,
            is_valid,
        });
    }
    Ok(out)
}

/// Extract every on-wire Conway transaction from every block in the corpus
/// stream, in `(block_index, tx_index)` order.
pub fn extract_corpus_txs(blocks: &[Vec<u8>]) -> Result<Vec<ExtractedTx>, ExtractError> {
    let mut out = Vec::new();
    for (block_index, block_cbor) in blocks.iter().enumerate() {
        out.extend(extract_block_txs(block_cbor, block_index)?);
    }
    Ok(out)
}
