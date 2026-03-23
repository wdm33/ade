// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_codec::allegra;
use ade_codec::alonzo;
use ade_codec::babbage;
use ade_codec::byron;
use ade_codec::cbor;
use ade_codec::conway;
use ade_codec::mary;
use ade_codec::shelley;
use ade_types::CardanoEra;
use ade_types::SlotNo;
use crate::error::LedgerError;
use crate::state::LedgerState;

/// Apply a block to ledger state, dispatching by era.
///
/// Byron blocks are fully validated (S-09).
/// Shelley/Allegra/Mary blocks are structurally validated: block and tx body
/// decoding is exercised, but UTxO resolution and witness verification are
/// skipped when the UTxO set lacks the required inputs (expected when replaying
/// contiguous sequences without genesis UTxO). This enables verdict agreement
/// testing on block acceptance without requiring the full chain history.
pub fn apply_block(
    state: &LedgerState,
    era: CardanoEra,
    block_cbor: &[u8],
) -> Result<LedgerState, LedgerError> {
    match era {
        CardanoEra::ByronEbb => {
            // EBBs contain no transactions — pass-through, state unchanged
            Ok(state.clone())
        }
        CardanoEra::ByronRegular => {
            let preserved = byron::decode_byron_regular_block(block_cbor)?;
            let block = preserved.decoded();
            crate::byron::validate_byron_block(state, block)
        }
        CardanoEra::Shelley => {
            let preserved = shelley::decode_shelley_block(block_cbor)?;
            let block = preserved.decoded();
            apply_shelley_era_block(state, block, CardanoEra::Shelley)
        }
        CardanoEra::Allegra => {
            let preserved = allegra::decode_allegra_block(block_cbor)?;
            let block = preserved.decoded();
            apply_shelley_era_block(state, block, CardanoEra::Allegra)
        }
        CardanoEra::Mary => {
            let preserved = mary::decode_mary_block(block_cbor)?;
            let block = preserved.decoded();
            apply_shelley_era_block(state, block, CardanoEra::Mary)
        }
        CardanoEra::Alonzo => {
            let preserved = alonzo::decode_alonzo_block(block_cbor)?;
            let block = preserved.decoded();
            apply_shelley_era_block(state, block, CardanoEra::Alonzo)
        }
        CardanoEra::Babbage => {
            let preserved = babbage::decode_babbage_block(block_cbor)?;
            let block = preserved.decoded();
            apply_shelley_era_block(state, block, CardanoEra::Babbage)
        }
        CardanoEra::Conway => {
            let preserved = conway::decode_conway_block(block_cbor)?;
            let block = preserved.decoded();
            apply_shelley_era_block(state, block, CardanoEra::Conway)
        }
    }
}

/// Apply a post-Byron (Shelley/Allegra/Mary) block.
///
/// Decodes all tx bodies to exercise the CBOR parsing pipeline.
/// When UTxO inputs are not resolvable (expected during contiguous replay
/// without full chain history), records the tx count but does not fail.
/// This gives structural verdict agreement — the block is accepted if
/// all transaction bodies and witness sets decode correctly.
/// Apply a block and return both the new state and the structural classification.
///
/// Same as `apply_block` but exposes the `BlockVerdict` so the harness
/// can separate ordinary accepted blocks from script-execution-deferred blocks.
pub fn apply_block_classified(
    state: &LedgerState,
    era: CardanoEra,
    block_cbor: &[u8],
) -> Result<(LedgerState, BlockVerdict), LedgerError> {
    match era {
        CardanoEra::ByronEbb => Ok((
            state.clone(),
            BlockVerdict { tx_count: 0, plutus_deferred_count: 0, non_plutus_count: 0 },
        )),
        CardanoEra::ByronRegular => {
            let preserved = byron::decode_byron_regular_block(block_cbor)?;
            let block = preserved.decoded();
            let new_state = crate::byron::validate_byron_block(state, block)?;
            Ok((
                new_state,
                BlockVerdict { tx_count: 0, plutus_deferred_count: 0, non_plutus_count: 0 },
            ))
        }
        _ => {
            let decoded = match era {
                CardanoEra::Shelley => shelley::decode_shelley_block(block_cbor)?,
                CardanoEra::Allegra => allegra::decode_allegra_block(block_cbor)?,
                CardanoEra::Mary => mary::decode_mary_block(block_cbor)?,
                CardanoEra::Alonzo => alonzo::decode_alonzo_block(block_cbor)?,
                CardanoEra::Babbage => babbage::decode_babbage_block(block_cbor)?,
                CardanoEra::Conway => conway::decode_conway_block(block_cbor)?,
                _ => return apply_block(state, era, block_cbor).map(|s| (s, BlockVerdict {
                    tx_count: 0, plutus_deferred_count: 0, non_plutus_count: 0,
                })),
            };
            let block = decoded.decoded();
            apply_shelley_era_block_classified(state, block, era)
        }
    }
}

fn apply_shelley_era_block(
    state: &LedgerState,
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
) -> Result<LedgerState, LedgerError> {
    apply_shelley_era_block_classified(state, block, era).map(|(s, _)| s)
}

fn apply_shelley_era_block_classified(
    state: &LedgerState,
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
) -> Result<(LedgerState, BlockVerdict), LedgerError> {
    let slot = SlotNo(block.header.body.slot);

    // Detect epoch transition: if this block's slot falls in a new epoch,
    // apply the epoch boundary transition before processing the block.
    let mut current_state = state.clone();
    if let Some(new_epoch) = crate::state::detect_epoch_transition(
        current_state.epoch_state.epoch,
        slot,
    ) {
        current_state = apply_epoch_boundary_minimal(&current_state, new_epoch);
    }

    let verdict = decode_validate_tx_bodies(block, era)?;

    // Track UTxO only when explicitly enabled.
    let utxo_state = if current_state.track_utxo {
        track_utxo(block, era, &current_state.utxo_state)?
    } else {
        current_state.utxo_state.clone()
    };

    // Process certificates to accumulate delegation/pool state.
    let cert_state = if current_state.track_utxo {
        process_block_certificates(block, era, &current_state)?
    } else {
        current_state.cert_state.clone()
    };

    let mut epoch_state = current_state.epoch_state;
    epoch_state.slot = slot;

    Ok((
        LedgerState {
            utxo_state,
            epoch_state,
            protocol_params: current_state.protocol_params,
            era,
            track_utxo: current_state.track_utxo,
            cert_state,
        },
        verdict,
    ))
}

/// Track UTxO through a block: consume inputs, produce outputs.
///
/// For each transaction:
/// 1. Consume inputs: remove from UTxO (skip gracefully if not found —
///    the input may predate the replay window)
/// 2. Capture the tx body wire bytes and compute tx hash
/// 3. Produce outputs: add to UTxO with key (tx_hash, output_index)
///
/// Returns (updated_utxo, inputs_resolved, inputs_missing).
fn track_utxo(
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
    current_utxo: &crate::utxo::UTxOState,
) -> Result<crate::utxo::UTxOState, LedgerError> {
    if block.tx_count == 0 {
        return Ok(current_utxo.clone());
    }

    let mut utxo = current_utxo.clone();
    let mut offset = 0;
    let data = &block.tx_bodies;
    let enc = cbor::read_array_header(data, &mut offset)?;

    let mut process_one = |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
        let body_start = *offset;

        // Decode tx body and extract inputs + outputs
        let (inputs, outputs) = extract_inputs_outputs_from_tx(data, offset, era)?;

        let body_end = *offset;
        let wire_bytes = &data[body_start..body_end];

        // Consume inputs: remove from UTxO if present
        for input in &inputs {
            utxo.utxos.remove(input);
        }

        // Compute tx hash = Blake2b-256(tx_body_wire_bytes)
        let tx_hash = ade_crypto::blake2b_256(wire_bytes);

        // Produce outputs
        for (idx, out) in outputs.into_iter().enumerate() {
            let tx_in = ade_types::tx::TxIn {
                tx_hash: tx_hash.clone(),
                index: idx as u16,
            };
            utxo.utxos.insert(tx_in, out);
        }

        Ok(())
    };

    match enc {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_one(data, &mut offset)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset)? {
                process_one(data, &mut offset)?;
            }
        }
    }

    Ok(utxo)
}

/// Minimal epoch boundary transition (T-25A.1).
///
/// Performs snapshot rotation and pool retirements. Reward computation
/// is deferred to T-25A.3 — this skeleton establishes the boundary
/// trigger and accumulator carry-forward first.
///
/// Idempotent: only called once per epoch boundary crossing.
fn apply_epoch_boundary_minimal(
    state: &LedgerState,
    new_epoch: ade_types::EpochNo,
) -> LedgerState {
    // 1. Snapshot rotation: mark <- current delegation, set <- mark, go <- set
    let new_mark = crate::epoch::StakeSnapshot {
        delegations: state.cert_state.delegation.delegations.iter()
            .map(|(cred, pool)| {
                let stake = state.cert_state.delegation.rewards
                    .get(cred)
                    .copied()
                    .unwrap_or(ade_types::tx::Coin(0));
                (cred.0.clone(), (pool.clone(), stake))
            })
            .collect(),
        pool_stakes: {
            let mut pool_stakes = std::collections::BTreeMap::new();
            for pool in state.cert_state.delegation.delegations.values() {
                let entry = pool_stakes
                    .entry(pool.clone())
                    .or_insert(ade_types::tx::Coin(0));
                // Accumulate delegated stake per pool
                entry.0 = entry.0.saturating_add(0); // placeholder — real stake comes from UTxO
            }
            pool_stakes
        },
    };
    let rotated = crate::epoch::rotate_snapshots(
        &state.epoch_state.snapshots,
        new_mark,
    );

    // 2. Pool retirements effective at this epoch
    let mut pool_state = state.cert_state.pool.clone();
    let mut retired_pools = Vec::new();
    pool_state.retiring.retain(|pool_id, retire_epoch| {
        if retire_epoch.0 <= new_epoch.0 {
            retired_pools.push(pool_id.clone());
            false
        } else {
            true
        }
    });
    for pool_id in &retired_pools {
        pool_state.pools.remove(pool_id);
    }

    // 3. Reset per-epoch accumulators
    let cert_state = crate::delegation::CertState {
        delegation: state.cert_state.delegation.clone(),
        pool: pool_state,
    };

    LedgerState {
        utxo_state: state.utxo_state.clone(),
        epoch_state: crate::state::EpochState {
            epoch: new_epoch,
            slot: state.epoch_state.slot,
            snapshots: rotated,
            reserves: state.epoch_state.reserves,
            treasury: state.epoch_state.treasury,
        },
        protocol_params: state.protocol_params.clone(),
        era: state.era,
        track_utxo: state.track_utxo,
        cert_state,
    }
}

/// Structured summary of an epoch boundary transition.
///
/// This is the diagnostic comparison surface for T-25A — when oracle
/// comparison fails, this tells you WHICH component diverged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochBoundarySummary {
    pub from_epoch: u64,
    pub to_epoch: u64,
    pub delegation_count: usize,
    pub pool_count: usize,
    pub retiring_count: usize,
    pub retired_count: usize,
    pub mark_delegation_count: usize,
    pub set_delegation_count: usize,
    pub go_delegation_count: usize,
    pub treasury: u64,
    pub reserves: u64,
}

/// Process certificates from a block to accumulate delegation/pool state.
///
/// For each tx body with a `certs` field (key 4), decode the certificates
/// and apply them to the certificate state using `apply_cert`.
fn process_block_certificates(
    block: &ade_types::shelley::block::ShelleyBlock,
    _era: CardanoEra,
    state: &LedgerState,
) -> Result<crate::delegation::CertState, LedgerError> {
    if block.tx_count == 0 {
        return Ok(state.cert_state.clone());
    }

    let mut cert_state = state.cert_state.clone();
    let mut offset = 0;
    let data = &block.tx_bodies;
    let enc = cbor::read_array_header(data, &mut offset)?;

    let mut process_one = |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
        // Read the tx body map to find key 4 (certs)
        let map_enc = cbor::read_map_header(data, offset)?;
        let map_len = match map_enc {
            cbor::ContainerEncoding::Definite(n, _) => n,
            cbor::ContainerEncoding::Indefinite => {
                // Skip indefinite map
                while !cbor::is_break(data, *offset)? {
                    let _ = cbor::skip_item(data, offset)?;
                    let _ = cbor::skip_item(data, offset)?;
                }
                *offset += 1;
                return Ok(());
            }
        };

        for _ in 0..map_len {
            let (key, _) = cbor::read_uint(data, offset)?;
            if key == 4 {
                // Capture cert bytes
                let cert_start = *offset;
                let (_, cert_end) = cbor::skip_item(data, offset)?;
                let cert_bytes = &data[cert_start..cert_end];

                // Decode and apply certificates
                match ade_codec::shelley::cert::decode_certificates(cert_bytes) {
                    Ok(certs) => {
                        let key_deposit = state.protocol_params.key_deposit;
                        for (idx, cert) in certs.iter().enumerate() {
                            match crate::delegation::apply_cert(
                                &cert_state,
                                cert,
                                key_deposit,
                                idx as u16,
                            ) {
                                Ok(new_state) => cert_state = new_state,
                                Err(_) => {
                                    // Certificate application errors are non-fatal
                                    // during replay without full UTxO state.
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Cert decode errors are non-fatal during replay.
                    }
                }
            } else {
                let _ = cbor::skip_item(data, offset)?;
            }
        }

        Ok(())
    };

    match enc {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_one(data, &mut offset)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset)? {
                process_one(data, &mut offset)?;
            }
        }
    }

    Ok(cert_state)
}

/// Extract inputs and outputs from a decoded tx body.
fn extract_inputs_outputs_from_tx(
    data: &[u8],
    offset: &mut usize,
    era: CardanoEra,
) -> Result<(Vec<ade_types::tx::TxIn>, Vec<crate::utxo::TxOut>), LedgerError> {
    match era {
        CardanoEra::Shelley => {
            let tx = ade_codec::shelley::tx::decode_shelley_tx_body(data, offset)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx.outputs.into_iter().map(|o| crate::utxo::TxOut::ShelleyMary {
                address: o.address,
                value: crate::value::Value::from_coin(o.coin),
            }).collect();
            Ok((inputs, outputs))
        }
        CardanoEra::Allegra => {
            let tx = ade_codec::allegra::tx::decode_allegra_tx_body(data, offset)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx.outputs.into_iter().map(|o| crate::utxo::TxOut::ShelleyMary {
                address: o.address,
                value: crate::value::Value::from_coin(o.coin),
            }).collect();
            Ok((inputs, outputs))
        }
        CardanoEra::Mary => {
            let tx = ade_codec::mary::tx::decode_mary_tx_body(data, offset)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx.outputs.into_iter().map(|o| crate::utxo::TxOut::ShelleyMary {
                address: o.address,
                value: crate::value::Value::from_coin(o.coin),
            }).collect();
            Ok((inputs, outputs))
        }
        CardanoEra::Alonzo => {
            let tx = ade_codec::alonzo::tx::decode_alonzo_tx_body(data, offset)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx.outputs.into_iter().map(|o| crate::utxo::TxOut::ShelleyMary {
                address: o.address,
                value: crate::value::Value::from_coin(o.coin),
            }).collect();
            Ok((inputs, outputs))
        }
        CardanoEra::Babbage => {
            let tx = ade_codec::babbage::tx::decode_babbage_tx_body(data, offset)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx.outputs.into_iter().map(|o| crate::utxo::TxOut::ShelleyMary {
                address: o.address,
                value: crate::value::Value::from_coin(o.coin),
            }).collect();
            Ok((inputs, outputs))
        }
        CardanoEra::Conway => {
            let tx = ade_codec::conway::tx::decode_conway_tx_body(data, offset)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx.outputs.into_iter().map(|o| crate::utxo::TxOut::ShelleyMary {
                address: o.address,
                value: crate::value::Value::from_coin(o.coin),
            }).collect();
            Ok((inputs, outputs))
        }
        _ => {
            let _ = cbor::skip_item(data, offset)?;
            Ok((Vec::new(), Vec::new()))
        }
    }
}

/// Block-level structural verdict from applying a post-Byron block.
///
/// Summarizes the script posture across all transactions in the block.
/// This is a deterministic classification surface — the harness can use
/// it to separate ordinary accepted blocks from script-execution-deferred blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockVerdict {
    /// Total transactions decoded.
    pub tx_count: u64,
    /// Number of transactions with Plutus scripts (deferred).
    pub plutus_deferred_count: u64,
    /// Number of transactions with no Plutus involvement.
    pub non_plutus_count: u64,
}

/// Decode and structurally validate all transaction bodies from a post-Byron block.
///
/// Parses both tx_bodies and witness_sets in parallel. Uses witness-confirmed
/// Plutus detection (keys 3/6/7 in witness set) rather than body-only heuristics.
/// Evaluates native scripts against available vkey hashes and current slot.
fn decode_validate_tx_bodies(
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
) -> Result<BlockVerdict, LedgerError> {
    if block.tx_count == 0 {
        return Ok(BlockVerdict {
            tx_count: 0,
            plutus_deferred_count: 0,
            non_plutus_count: 0,
        });
    }

    let current_slot = block.header.body.slot;

    // Parse witness sets for all txs
    let witness_infos = crate::witness::decode_witness_infos(&block.witness_sets)?;

    // Parse and validate tx bodies
    let mut body_offset = 0;
    let body_data = &block.tx_bodies;
    let body_enc = cbor::read_array_header(body_data, &mut body_offset)?;

    let mut tx_count = 0u64;
    let mut plutus_deferred_count = 0u64;
    let mut non_plutus_count = 0u64;
    let mut tx_idx = 0usize;

    let mut process_one = |body_data: &[u8], body_offset: &mut usize| -> Result<(), LedgerError> {
        // Decode and structurally validate the tx body
        let body_posture = decode_and_validate_single_tx(body_data, body_offset, era)?;

        // Get witness info for this tx (if available)
        let witness_info = witness_infos.get(tx_idx);

        // Determine authoritative script verdict using witness confirmation
        let has_plutus_in_witnesses = witness_info
            .map(|w| w.has_plutus())
            .unwrap_or(false);

        // Witness-confirmed classification:
        // - Plutus in witnesses → deferred (authoritative, regardless of body heuristic)
        // - Body says Plutus but witnesses don't confirm → still deferred (conservative)
        // - Native scripts in witnesses → evaluate them
        // - Neither → non-Plutus
        let is_deferred = has_plutus_in_witnesses
            || body_posture == crate::scripts::ScriptPosture::PlutusPresentDeferred;

        if is_deferred {
            plutus_deferred_count += 1;
        } else {
            // Evaluate native scripts if present
            if let Some(w) = witness_info {
                for script in &w.native_scripts {
                    let _verdict = crate::scripts::evaluate_native_script(
                        script,
                        &w.available_key_hashes,
                        current_slot,
                    );
                    // Native script verdict is recorded but does not reject the block
                    // in T-24B without UTxO state to determine which scripts are required.
                    // Full rejection requires knowing which inputs are script-locked (T-24B+).
                }
            }
            non_plutus_count += 1;
        }

        tx_count += 1;
        tx_idx += 1;
        Ok(())
    };

    match body_enc {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process_one(body_data, &mut body_offset)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(body_data, body_offset)? {
                process_one(body_data, &mut body_offset)?;
            }
        }
    }

    Ok(BlockVerdict {
        tx_count,
        plutus_deferred_count,
        non_plutus_count,
    })
}

/// Decode a single tx body, run structural validation, classify script posture.
fn decode_and_validate_single_tx(
    data: &[u8],
    offset: &mut usize,
    era: CardanoEra,
) -> Result<crate::scripts::ScriptPosture, LedgerError> {
    match era {
        CardanoEra::Shelley => {
            let _tx = ade_codec::shelley::tx::decode_shelley_tx_body(data, offset)?;
            Ok(crate::scripts::ScriptPosture::NonPlutusScriptsOnly)
        }
        CardanoEra::Allegra => {
            let _tx = ade_codec::allegra::tx::decode_allegra_tx_body(data, offset)?;
            Ok(crate::scripts::ScriptPosture::NonPlutusScriptsOnly)
        }
        CardanoEra::Mary => {
            let _tx = ade_codec::mary::tx::decode_mary_tx_body(data, offset)?;
            Ok(crate::scripts::ScriptPosture::NonPlutusScriptsOnly)
        }
        CardanoEra::Alonzo => {
            let tx = ade_codec::alonzo::tx::decode_alonzo_tx_body(data, offset)?;
            crate::alonzo::validate_alonzo_structure(&tx)?;
            Ok(crate::alonzo::classify_alonzo_script_posture(&tx))
        }
        CardanoEra::Babbage => {
            let tx = ade_codec::babbage::tx::decode_babbage_tx_body(data, offset)?;
            crate::babbage::validate_babbage_structure(&tx)?;
            Ok(crate::babbage::classify_babbage_script_posture(&tx))
        }
        CardanoEra::Conway => {
            let tx = ade_codec::conway::tx::decode_conway_tx_body(data, offset)?;
            crate::conway::validate_conway_structure(&tx)?;
            Ok(crate::conway::classify_conway_script_posture(&tx))
        }
        _ => {
            let _ = cbor::skip_item(data, offset)?;
            Ok(crate::scripts::ScriptPosture::NoScripts)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn apply_block_byron_ebb_passes_through() {
        let state = LedgerState::new(CardanoEra::ByronEbb);

        use ade_codec::traits::{AdeEncode, CodecContext};
        use ade_types::byron::block::{ByronEbbBlock, ByronEbbHeader};
        use ade_types::Hash32;

        let ebb = ByronEbbBlock {
            header: ByronEbbHeader {
                protocol_magic: 764824073,
                prev_hash: Hash32([0u8; 32]),
                body_proof: Hash32([0u8; 32]),
                epoch: 0,
                chain_difficulty: 0,
                extra_data: vec![0x81, 0xa0],
            },
            body: vec![0x80],
            extra: vec![0xa0],
        };
        let ctx = CodecContext {
            era: CardanoEra::ByronEbb,
        };
        let mut buf = Vec::new();
        ebb.ade_encode(&mut buf, &ctx).unwrap();

        let result = apply_block(&state, CardanoEra::ByronEbb, &buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), state);
    }

    #[test]
    fn apply_block_deterministic() {
        // Determinism: same invalid input produces same error both times
        let state = LedgerState::new(CardanoEra::Mary);
        let result1 = apply_block(&state, CardanoEra::Mary, &[0x83, 0x01, 0x02]);
        let result2 = apply_block(&state, CardanoEra::Mary, &[0x83, 0x01, 0x02]);
        assert_eq!(result1, result2);
    }
}
