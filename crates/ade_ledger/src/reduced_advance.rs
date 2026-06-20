// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3b-2 (DC-EVIEW-04b) — the per-block reduced UTxO delta.
//!
//! [`reduced_block_delta`] is a FAITHFUL MIRROR of the ledger's own `track_utxo`
//! (`rules.rs`): it iterates the block's `tx_bodies` through the SAME
//! `extract_inputs_outputs_from_tx`, removes the same spent `TxIn`s, computes the
//! same `tx_hash = blake2b_256(tx_body_wire_bytes)`, and produces the same
//! `(tx_hash, output_index)` keys — but it emits a bounded DELTA `(spent, produced)`
//! with the produced outputs REDUCED to `(Coin, ReducedStakeRef)` (S3b-1) instead of
//! mutating a full UTxO. So the reduced UTxO the windowed advance maintains is, by
//! construction, the reduced projection of the ledger transition's own UTxO — a
//! single authority, not a parallel reimplementation. The equality
//! `reduced_block_delta == reduce(track_utxo)` is proven on a REAL Conway block.
//!
//! The window driver (`ade_runtime`) applies each block's delta to the durable
//! reduced-UTxO checkpoint (DC-EVIEW-04) and advances the cert/delegation/reward
//! state via the ledger's own `process_block_certificates`. (At Conway the S3a
//! `PointerMap` is unused — pointer outputs reduce to `NonContributing` — so the
//! advance does not populate it.)

use std::collections::BTreeMap;

use ade_codec::cbor;
use ade_types::tx::{Coin, TxIn};
use ade_types::CardanoEra;

use crate::error::LedgerError;
use crate::reduced_utxo::{reduce_txout, ReducedStakeRef};
use crate::rules::extract_inputs_outputs_from_tx;

/// The bounded per-block reduced UTxO delta: inputs spent + reduced outputs produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReducedBlockDelta {
    pub spent: Vec<TxIn>,
    pub produced: Vec<(TxIn, Coin, ReducedStakeRef)>,
}

/// Compute a block's reduced UTxO delta, mirroring `track_utxo` exactly (same
/// extraction, same tx-hash, same produced keys) but reducing the outputs.
pub fn reduced_block_delta(
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
) -> Result<ReducedBlockDelta, LedgerError> {
    // `track_utxo` THREADS a single mutating UTxO across the block's txs, so a tx may
    // spend an output produced by an EARLIER tx in the SAME block (intra-block chained
    // spend), and that output ends ABSENT. To be a faithful mirror, accumulate the
    // produced set in a map and CANCEL any output spent later in the block; an input
    // that hits the intra-block produced set never becomes a `spent` (it was never in
    // the prior checkpoint). The emitted delta is then the NET block effect.
    let mut produced: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
    let mut spent: Vec<TxIn> = Vec::new();
    if block.tx_count == 0 {
        return Ok(ReducedBlockDelta::default());
    }
    let data = &block.tx_bodies;
    let mut offset = 0usize;
    match cbor::read_array_header(data, &mut offset)? {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_one_tx(data, &mut offset, era, &mut spent, &mut produced)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset)? {
                process_one_tx(data, &mut offset, era, &mut spent, &mut produced)?;
            }
        }
    }
    Ok(ReducedBlockDelta {
        spent,
        produced: produced
            .into_iter()
            .map(|(txin, (coin, reduced))| (txin, coin, reduced))
            .collect(),
    })
}

fn process_one_tx(
    data: &[u8],
    offset: &mut usize,
    era: CardanoEra,
    spent: &mut Vec<TxIn>,
    produced: &mut BTreeMap<TxIn, (Coin, ReducedStakeRef)>,
) -> Result<(), LedgerError> {
    let body_start = *offset;
    let (inputs, outputs) = extract_inputs_outputs_from_tx(data, offset, era)?;
    let body_end = *offset;
    let wire_bytes = &data[body_start..body_end];

    // Inputs are processed BEFORE this tx's outputs are added (mirrors track_utxo's
    // remove-then-insert per tx), so a tx can only cancel outputs from EARLIER txs.
    for input in inputs {
        if produced.remove(&input).is_none() {
            // not produced earlier in this block -> a real prior-checkpoint spend.
            spent.push(input);
        }
        // else: produced-then-spent within the block -> phantom cancelled (no spent).
    }
    // tx_hash = Blake2b-256(tx_body_wire_bytes) -- identical to track_utxo.
    let tx_hash = ade_crypto::blake2b::blake2b_256(wire_bytes);
    for (idx, out) in outputs.into_iter().enumerate() {
        let txin = TxIn {
            tx_hash: tx_hash.clone(),
            index: idx as u16,
        };
        produced.insert(txin, reduce_txout(&out));
    }
    Ok(())
}

/// Advance the cert / delegation / pool / reward state through one block, reusing the
/// ledger's OWN `process_block_certificates` (the single authority — NOT a parallel
/// reimplementation). The caller carries a [`LedgerState`](crate::state::LedgerState)
/// whose `cert_state` / `gov_state` / params are the current window accumulation; the
/// returned `(CertState, gov)` is the post-block state to carry forward. (S3c reads the
/// resulting delegation map + reward balances to aggregate per-pool stake.)
pub fn advance_cert_state(
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
    state: &crate::state::LedgerState,
) -> Result<(crate::delegation::CertState, Option<crate::state::ConwayGovState>), LedgerError> {
    crate::rules::process_block_certificates(block, era, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::track_utxo;
    use crate::utxo::UTxOState;
    use std::collections::BTreeMap;

    // The REAL Conway block captured from the live preprod peer (public chain data),
    // reused from the ade_node admission fixture. Proving the equality on a REAL block
    // (not synthetic CBOR) is required by the project's real-interop discipline.
    const RAW_CONWAY_BLOCK: &[u8] =
        include_bytes!("../../ade_node/tests/fixtures/raw_era_block_conway.cbor");

    fn decode_fixture_block() -> ade_types::shelley::block::ShelleyBlock {
        let env =
            ade_codec::cbor::envelope::decode_block_envelope(RAW_CONWAY_BLOCK).expect("envelope");
        assert_eq!(env.era, CardanoEra::Conway, "fixture is a Conway block");
        let inner = &RAW_CONWAY_BLOCK[env.block_start..env.block_end];
        ade_codec::conway::decode_conway_block(inner)
            .expect("decode conway block")
            .decoded()
            .clone()
    }

    // THE rigor proof: reduced_block_delta produces EXACTLY the reduced projection of
    // the ledger's own track_utxo, on a real Conway block. track_utxo over an empty
    // UTxO yields the block's produced outputs (spent inputs aren't present, skipped
    // gracefully); reduced_block_delta.produced must equal those, reduced.
    #[test]
    fn reduced_delta_equals_reduce_of_track_utxo_on_real_conway_block() {
        let block = decode_fixture_block();
        let era = CardanoEra::Conway;

        // the ledger's own apply (full TxOut), over an empty UTxO.
        let full = track_utxo(&block, era, &UTxOState::new()).expect("track_utxo");
        let mut expected: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        for (txin, out) in full.utxos.iter() {
            expected.insert(txin.clone(), reduce_txout(out));
        }

        // the reduced delta.
        let delta = reduced_block_delta(&block, era).expect("reduced_block_delta");
        let mut got: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        for (txin, coin, reduced) in &delta.produced {
            got.insert(txin.clone(), (*coin, reduced.clone()));
        }

        assert!(!got.is_empty(), "the real block must produce outputs");
        assert_eq!(
            got, expected,
            "reduced_block_delta.produced must equal reduce(track_utxo) byte-for-byte"
        );
        // the produced keys are exactly the ledger's produced UTxO keys.
        assert_eq!(got.len(), full.utxos.iter().count());
    }

    // THE regression for the intra-block chained-spend bug (security review): a block
    // whose SECOND tx spends an output PRODUCED BY THE FIRST tx in the SAME block. The
    // ledger's track_utxo threads the UTxO, so that output ends ABSENT; reduced_block_delta
    // must cancel the phantom and match. (Built by appending a minimal Conway tx2 that
    // spends (tx1_hash, 0) to the real fixture's tx1.)
    #[test]
    fn intra_block_chained_spend_cancels_phantom_matches_track_utxo() {
        let block = decode_fixture_block();
        let era = CardanoEra::Conway;

        // capture tx1's exact body bytes + hash (the first tx of the real block).
        let data = &block.tx_bodies;
        let mut off = 0usize;
        let _ = cbor::read_array_header(data, &mut off).unwrap();
        let tx1_start = off;
        let _ = extract_inputs_outputs_from_tx(data, &mut off, era).unwrap();
        let tx1_bytes = data[tx1_start..off].to_vec();
        let tx1_hash = ade_crypto::blake2b::blake2b_256(&tx1_bytes);

        // a minimal Conway tx body that spends (tx1_hash, 0): map{0:[[hash,0]], 1:[], 2:0}.
        let mut tx2 = vec![0xa3u8, 0x00, 0x81, 0x82, 0x58, 0x20];
        tx2.extend_from_slice(&tx1_hash.0);
        tx2.extend_from_slice(&[0x00, 0x01, 0x80, 0x02, 0x00]);

        // a 2-tx block: array(2) [tx1, tx2].
        let mut tx_bodies = vec![0x82u8];
        tx_bodies.extend_from_slice(&tx1_bytes);
        tx_bodies.extend_from_slice(&tx2);
        let mut chained = block.clone();
        chained.tx_count = 2;
        chained.tx_bodies = tx_bodies;

        // sanity: track_utxo (the ground truth) does NOT retain the chained-spent output.
        let full = track_utxo(&chained, era, &UTxOState::new()).unwrap();
        let spent_key = TxIn { tx_hash: tx1_hash.clone(), index: 0 };
        let mut expected: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        for (txin, out) in full.utxos.iter() {
            expected.insert(txin.clone(), reduce_txout(out));
        }
        assert!(!expected.contains_key(&spent_key), "track_utxo drops the chained-spent output");

        // reduced_block_delta must MATCH track_utxo: the phantom is cancelled.
        let delta = reduced_block_delta(&chained, era).unwrap();
        let mut got: BTreeMap<TxIn, (Coin, ReducedStakeRef)> = BTreeMap::new();
        for (txin, coin, r) in &delta.produced {
            got.insert(txin.clone(), (*coin, r.clone()));
        }
        assert!(
            !got.contains_key(&spent_key),
            "the produced-then-spent output must be CANCELLED, not a phantom"
        );
        assert!(
            !delta.spent.contains(&spent_key),
            "an intra-block-produced input is not a prior-checkpoint spend"
        );
        assert_eq!(got, expected, "reduced_block_delta == reduce(track_utxo) with chaining");
    }

    // Determinism: same block -> identical delta across calls.
    #[test]
    fn reduced_block_delta_is_deterministic() {
        let block = decode_fixture_block();
        assert_eq!(
            reduced_block_delta(&block, CardanoEra::Conway).unwrap(),
            reduced_block_delta(&block, CardanoEra::Conway).unwrap()
        );
    }

    // The cert advance reuses the ledger's own process_block_certificates over a real
    // Conway block without error; the returned cert_state is carried forward by the
    // window driver (S3b-2) and read by S3c. (Proves the single-authority wiring.)
    #[test]
    fn advance_cert_state_over_real_block_does_not_error() {
        let block = decode_fixture_block();
        let state = crate::state::LedgerState::new(CardanoEra::Conway);
        let (_cert_state, _gov) =
            advance_cert_state(&block, CardanoEra::Conway, &state).expect("cert advance ok");
    }

    // An empty block yields an empty delta.
    #[test]
    fn empty_block_yields_empty_delta() {
        let mut block = decode_fixture_block();
        block.tx_count = 0;
        let delta = reduced_block_delta(&block, CardanoEra::Conway).unwrap();
        assert_eq!(delta, ReducedBlockDelta::default());
    }
}
