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
/// Apply a block and return the verdict plus any epoch boundary accounting.
///
/// If the block triggers an epoch boundary, the accounting struct contains
/// the full decomposition (deltaR1, deltaR2, deltaT1, deltaT2, etc.).
/// If no boundary fires, accounting is None.
pub fn apply_block_with_accounting(
    state: &LedgerState,
    era: CardanoEra,
    block_cbor: &[u8],
) -> Result<(LedgerState, BlockVerdict, Option<EpochBoundaryAccounting>), LedgerError> {
    // Pre-decode the block to get the slot for epoch detection
    let slot = match era {
        CardanoEra::ByronEbb | CardanoEra::ByronRegular => {
            let (s, v) = apply_block_classified(state, era, block_cbor)?;
            return Ok((s, v, None));
        }
        _ => {
            let decoded = match era {
                CardanoEra::Shelley => shelley::decode_shelley_block(block_cbor)?,
                CardanoEra::Allegra => allegra::decode_allegra_block(block_cbor)?,
                CardanoEra::Mary => mary::decode_mary_block(block_cbor)?,
                CardanoEra::Alonzo => alonzo::decode_alonzo_block(block_cbor)?,
                CardanoEra::Babbage => babbage::decode_babbage_block(block_cbor)?,
                CardanoEra::Conway => conway::decode_conway_block(block_cbor)?,
                _ => {
                    let (s, v) = apply_block_classified(state, era, block_cbor)?;
                    return Ok((s, v, None));
                }
            };
            SlotNo(decoded.decoded().header.body.slot)
        }
    };

    // Check for epoch boundary, capture accounting if it fires. Routes through THE dispatcher
    // (RVBP B1): a track_utxo=false Conway follower crosses reduced (no accounting); everything
    // else runs the full boundary. This is the same dispatch the two live crossers use.
    let mut accounting = None;
    let pre_boundary_state = if let Some(new_epoch) = crate::state::detect_epoch_transition(
        state.epoch_state.epoch, slot,
    ) {
        let (new_state, acct) = dispatch_epoch_boundary(state, new_epoch)?;
        accounting = acct;
        new_state
    } else {
        state.clone()
    };

    // Apply block normally on the (possibly post-boundary) state
    let (final_state, verdict) = apply_block_classified(&pre_boundary_state, era, block_cbor)?;
    Ok((final_state, verdict, accounting))
}

/// Cross a Conway epoch boundary in the REDUCED-VALIDATION plane (`track_utxo=false`). A reduced follower has no
/// base-UTxO stake authority here, so it advances ONLY the audited structural facts — epoch progression and the
/// block-production window rollover — and produces NO authority:
///   - stake snapshots UNAVAILABLE (`EpochStakeSnapshots::ReducedUnavailable`): no mark/set/go bytes, so nothing
///     can be persisted, fingerprinted, or rehydrated as authority (N-RVB-1, gate 1);
///   - NO certificate/pool lifecycle (POOLREAP is unavailable in the reduced plane, N-RVB-3): the cert/gov state
///     is reset to its empty structural absence rather than carried unchanged — a reduced boundary must not
///     leave a full `CertState`/gov state that a future consumer could mistake for advanced (post-POOLREAP)
///     lifecycle (deviation 2; the typed result is [`crate::reduced_boundary::ReducedBoundaryProjection`] with
///     `ReducedCertProjection::Unavailable`). Safe: a `track_utxo=false` follower never applies certs
///     (`apply_block_classified` carries cert/gov structurally and evolves neither) and reads leadership from the
///     accumulator's `PoolDistrView`, never this `LedgerState`.
/// No rewards, pots, or governance enactment — those are the FULL `EpochAccumulator`'s sole authority. Pure.
pub fn apply_reduced_epoch_boundary(state: &LedgerState, new_epoch: ade_types::EpochNo) -> LedgerState {
    let mut reduced = state.clone();
    reduced.epoch_state.epoch = new_epoch;
    // No fabricated snapshot — the reduced plane crossed WITHOUT stake authority (gate 1 / N-RVB-1).
    reduced.epoch_state.snapshots = crate::epoch::EpochStakeSnapshots::ReducedUnavailable;
    // No advanced certificate/pool or governance lifecycle — UNAVAILABLE BY TYPE, never a cleared/empty
    // full-state field that could be mistaken for advanced authority (N-RVB-3, deviation 2). "Reduced follower +
    // a normal CertState/gov present" is now unrepresentable; a full-truth reader fails closed.
    reduced.cert_state = crate::state::CertStateProjection::ReducedUnavailable;
    reduced.gov_state = crate::state::GovStateProjection::ReducedUnavailable;
    // The block-production window rolled over: the new epoch starts with a fresh (empty) window and zero fees.
    reduced.epoch_state.block_production = std::collections::BTreeMap::new();
    reduced.epoch_state.epoch_fees = ade_types::tx::Coin(0);
    reduced
}

/// THE epoch-boundary dispatcher — routes by validation plane BEFORE any full boundary execution
/// (RVBP B1). A `track_utxo=false` Conway follower crosses via the REDUCED projection
/// ([`apply_reduced_epoch_boundary`]: epoch/slot + block-window rollover only; snapshots/cert/gov =
/// `ReducedUnavailable`, no accounting) and NEVER runs [`apply_epoch_boundary_full`] — the reward/pot/
/// POOLREAP/governance authority belongs solely to the `EpochAccumulator`, so a reduced follower must
/// not fabricate it. Every other state (full replay / oracle / `track_utxo=true`) runs the authoritative
/// full boundary. Both live crossers (`apply_shelley_era_block_with_verdicts`,
/// `apply_shelley_era_block_classified`) and the accounting entry point route through THIS function, so
/// no production `track_utxo=false` Conway path can reach the full boundary. Returns the post-boundary
/// state and the accounting (`None` on the reduced plane — there is no authoritative accounting to emit).
fn dispatch_epoch_boundary(
    state: &LedgerState,
    new_epoch: ade_types::EpochNo,
) -> Result<(LedgerState, Option<EpochBoundaryAccounting>), LedgerError> {
    if state.era == CardanoEra::Conway && !state.track_utxo {
        Ok((apply_reduced_epoch_boundary(state, new_epoch), None))
    } else {
        let (new_state, acct) = apply_epoch_boundary_full(state, new_epoch)?;
        Ok((new_state, Some(acct)))
    }
}

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
            BlockVerdict { tx_count: 0, plutus_deferred_count: 0, non_plutus_count: 0, native_script_passed: 0, native_script_failed: 0, state_backed_phase1_rejected: 0, plutus_eval_passed: 0, plutus_eval_failed: 0, plutus_eval_ineligible: 0 },
        )),
        CardanoEra::ByronRegular => {
            let preserved = byron::decode_byron_regular_block(block_cbor)?;
            let block = preserved.decoded();
            let new_state = crate::byron::validate_byron_block(state, block)?;
            Ok((
                new_state,
                BlockVerdict { tx_count: 0, plutus_deferred_count: 0, non_plutus_count: 0, native_script_passed: 0, native_script_failed: 0, state_backed_phase1_rejected: 0, plutus_eval_passed: 0, plutus_eval_failed: 0, plutus_eval_ineligible: 0 },
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
                    native_script_passed: 0, native_script_failed: 0,
                    state_backed_phase1_rejected: 0,
                    plutus_eval_passed: 0, plutus_eval_failed: 0,
                    plutus_eval_ineligible: 0,
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

/// Apply a block and return `BlockApplyResult` — state transition,
/// block-level verdict counters, AND per-tx verdicts.
///
/// This is the S-32-item-7 surface: callers that need to diff tx-by-tx
/// against an oracle (CE-88) use this. The existing `apply_block` /
/// `apply_block_classified` entry points are unchanged and retain
/// their tuple return shapes for other callers.
///
/// Per-tx verdicts are only populated for Alonzo/Babbage/Conway blocks
/// with `track_utxo=true` — pre-Alonzo or unresolved blocks return
/// `tx_verdicts: Vec::new()`.
pub fn apply_block_with_verdicts(
    state: &LedgerState,
    era: CardanoEra,
    block_cbor: &[u8],
) -> Result<BlockApplyResult, LedgerError> {
    // For Byron and empty-tx cases, reuse the existing classified path.
    if matches!(era, CardanoEra::ByronEbb | CardanoEra::ByronRegular) {
        let (new_state, verdict) = apply_block_classified(state, era, block_cbor)?;
        return Ok(BlockApplyResult {
            new_state,
            verdict,
            tx_verdicts: Vec::new(),
            invalid_tx_indices: std::collections::BTreeSet::new(),
        });
    }

    // Decode the block once; run the full classified pipeline PLUS
    // per-tx verdict collection when the composer path activates.
    let decoded = match era {
        CardanoEra::Shelley => shelley::decode_shelley_block(block_cbor)?,
        CardanoEra::Allegra => allegra::decode_allegra_block(block_cbor)?,
        CardanoEra::Mary => mary::decode_mary_block(block_cbor)?,
        CardanoEra::Alonzo => alonzo::decode_alonzo_block(block_cbor)?,
        CardanoEra::Babbage => babbage::decode_babbage_block(block_cbor)?,
        CardanoEra::Conway => conway::decode_conway_block(block_cbor)?,
        _ => {
            let (new_state, verdict) = apply_block_classified(state, era, block_cbor)?;
            return Ok(BlockApplyResult {
                new_state,
                verdict,
                tx_verdicts: Vec::new(),
                invalid_tx_indices: std::collections::BTreeSet::new(),
            });
        }
    };
    let block = decoded.decoded();
    apply_shelley_era_block_with_verdicts(state, block, era)
}

fn apply_shelley_era_block_with_verdicts(
    state: &LedgerState,
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
) -> Result<BlockApplyResult, LedgerError> {
    let slot = SlotNo(block.header.body.slot);

    let mut current_state = state.clone();
    if let Some(new_epoch) = crate::state::detect_epoch_transition(
        current_state.epoch_state.epoch,
        slot,
    ) {
        // RVBP B1: dispatch by validation plane BEFORE full boundary execution. A track_utxo=false Conway
        // follower crosses via the reduced projection (cert/gov/snapshots = ReducedUnavailable) and never
        // reaches the full reward/pot/POOLREAP/gov boundary — that authority is the accumulator's alone.
        let (new_state, _accounting) = dispatch_epoch_boundary(&current_state, new_epoch)?;
        current_state = new_state;
    }

    let mut verdict = decode_validate_tx_bodies(block, era)?;

    // Conway vkey-witness + required-signer closure (CE-B2-1). Runs
    // against the pre-block UTxO; tx-derived sources are checked
    // regardless of track_utxo (the slice doc track_utxo note).
    if era == CardanoEra::Conway {
        verify_conway_witness_closure(block, &current_state)?;
    }

    let utxo_state = if current_state.track_utxo {
        track_utxo(block, era, &current_state.utxo_state)?
    } else {
        current_state.utxo_state.clone()
    };

    // Run the composer + Plutus-eval dispatch, capturing per-tx verdicts.
    let tx_verdicts = if current_state.track_utxo
        && matches!(
            era,
            CardanoEra::Alonzo | CardanoEra::Babbage | CardanoEra::Conway
        )
    {
        let (stats, verdicts) =
            run_phase_one_composers(block, era, &current_state)?;
        verdict.state_backed_phase1_rejected = stats.rejected;
        verdict.plutus_eval_passed = stats.plutus_eval_passed;
        verdict.plutus_eval_failed = stats.plutus_eval_failed;
        verdict.plutus_eval_ineligible = stats.plutus_eval_ineligible;
        verdicts
    } else {
        Vec::new()
    };

    let (cert_state, gov_state) = if current_state.track_utxo {
        let (cs, gs) = process_block_certificates(block, era, &current_state)?;
        (
            crate::state::CertStateProjection::Authoritative(cs),
            crate::state::GovStateProjection::Authoritative(gs),
        )
    } else {
        // Reduced follower (track_utxo=false): carry the capability-typed cert/gov FORWARD unchanged — after a
        // reduced boundary these are `ReducedUnavailable`, so no normal CertState/gov is ever exposed.
        (current_state.cert_state.clone(), current_state.gov_state.clone())
    };

    let mut epoch_state = current_state.epoch_state;
    epoch_state.slot = slot;

    let invalid_tx_indices = crate::plutus_eval::decode_invalid_tx_indices(
        block.invalid_txs.as_deref(),
    );

    Ok(BlockApplyResult {
        new_state: LedgerState {
            utxo_state,
            epoch_state,
            protocol_params: current_state.protocol_params,
            era,
            track_utxo: current_state.track_utxo,
            cert_state,
            max_lovelace_supply: current_state.max_lovelace_supply,
            // PHASE4-B5: governance state is carried forward (cert-accumulated
            // when tracked), no longer nulled at every block apply.
            gov_state,
            conway_deposit_params: current_state.conway_deposit_params.clone(),
        },
        verdict,
        tx_verdicts,
        invalid_tx_indices,
    })
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
        // RVBP B1: dispatch by validation plane BEFORE full boundary execution — a track_utxo=false Conway
        // follower crosses via the reduced projection and never reaches the full boundary (accumulator authority).
        let (new_state, _accounting) = dispatch_epoch_boundary(&current_state, new_epoch)?;
        current_state = new_state;
    }

    let mut verdict = decode_validate_tx_bodies(block, era)?;

    // Conway vkey-witness + required-signer closure (CE-B2-1). Runs
    // against the pre-block UTxO; tx-derived sources are checked
    // regardless of track_utxo (the slice doc track_utxo note).
    if era == CardanoEra::Conway {
        verify_conway_witness_closure(block, &current_state)?;
    }

    // Track UTxO only when explicitly enabled.
    let utxo_state = if current_state.track_utxo {
        track_utxo(block, era, &current_state.utxo_state)?
    } else {
        current_state.utxo_state.clone()
    };

    // Run state-backed Phase 1 composer for Alonzo+ eras.
    // Only when track_utxo is on (otherwise UTxO resolution is impossible).
    // Runs against the PRE-block UTxO — the composer's input-resolution
    // invariant is evaluated at the block boundary, not per-tx mid-block.
    if current_state.track_utxo
        && matches!(
            era,
            CardanoEra::Alonzo | CardanoEra::Babbage | CardanoEra::Conway
        )
    {
        let (stats, _tx_verdicts) = run_phase_one_composers(block, era, &current_state)?;
        verdict.state_backed_phase1_rejected = stats.rejected;
        verdict.plutus_eval_passed = stats.plutus_eval_passed;
        verdict.plutus_eval_failed = stats.plutus_eval_failed;
        verdict.plutus_eval_ineligible = stats.plutus_eval_ineligible;
    }

    // Process certificates to accumulate delegation/pool state and (PHASE4-B5)
    // governance state.
    let (cert_state, gov_state) = if current_state.track_utxo {
        let (cs, gs) = process_block_certificates(block, era, &current_state)?;
        (
            crate::state::CertStateProjection::Authoritative(cs),
            crate::state::GovStateProjection::Authoritative(gs),
        )
    } else {
        // Reduced follower (track_utxo=false): carry the capability-typed cert/gov FORWARD unchanged — after a
        // reduced boundary these are `ReducedUnavailable`, so no normal CertState/gov is ever exposed.
        (current_state.cert_state.clone(), current_state.gov_state.clone())
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
            max_lovelace_supply: current_state.max_lovelace_supply,
        // PHASE4-B5: governance state carried forward (cert-accumulated when
        // tracked), no longer nulled at every block apply.
        gov_state,
        conway_deposit_params: current_state.conway_deposit_params.clone(),
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
pub(crate) fn track_utxo(
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

/// Conway vkey-witness + required-signer closure over a block's txs
/// (PHASE4-B2-S1, CE-B2-1).
///
/// For each Conway tx:
/// 1. Derive the closed required-signer set
///    ([`crate::tx_validity::required_signers`]). The tx-derived sources
///    (explicit / withdrawal / certificate / governance voter) are
///    derived **regardless of `track_utxo`** — they need no UTxO. The
///    input/collateral payment-key sources are added only when
///    `track_utxo` is on and the spent/collateral outputs resolve in the
///    pre-block UTxO (the slice doc track_utxo note: full input-cred
///    coverage is exercised on real UTxO in B2-S3; here the B1 corpus
///    runs `track_utxo=false`, so only tx-derived coverage applies).
/// 2. Verify every required key hash is covered by a witness whose
///    Ed25519 signature over the PRESERVED tx body hash verifies
///    ([`crate::tx_validity::verify_required_witnesses`]). Fail-closed.
///
/// Returns the FIRST failure as a `LedgerError`; an all-covered block
/// returns `Ok(())`. Pure over `(block, utxo)`; no I/O.
fn verify_conway_witness_closure(
    block: &ade_types::shelley::block::ShelleyBlock,
    state: &LedgerState,
) -> Result<(), LedgerError> {
    use crate::tx_validity::{
        required_signers, tx_derived_required_signers, verify_required_witnesses, ResolvedInputs,
        ResolvedOutput, VKeyWitnessRef,
    };

    if block.tx_count == 0 {
        return Ok(());
    }

    let track = state.track_utxo;
    let utxo = &state.utxo_state.utxos;

    // Decode witness sets once (each is a Vec<VKeyWitness> from key 0).
    let witness_sets = shelley_witness_sets(block)?;

    let mut body_offset = 0;
    let body_data = &block.tx_bodies;
    let body_enc = cbor::read_array_header(body_data, &mut body_offset)?;

    let mut tx_idx = 0usize;
    let mut process = |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
        let body_start = *offset;
        let body = ade_codec::conway::tx::decode_conway_tx_body(data, offset)?;
        let body_end = *offset;
        let body_wire = &data[body_start..body_end];
        let body_hash = ade_crypto::blake2b_256(body_wire);

        // 1. Required signers.
        let required = if track {
            // Resolve spend + collateral inputs against the pre-block UTxO.
            // An input that does not resolve makes the closed function
            // fail-fast (UnresolvableInput) UNLESS we cannot see it because
            // it predates the replay window — in that case full input-cred
            // coverage is not provable here, so fall back to the tx-derived
            // subset (the real-UTxO coverage proof lives in B2-S3).
            let mut resolved = ResolvedInputs::new();
            let mut all_resolved = true;
            for input in body.inputs.iter().chain(
                body.collateral_inputs.iter().flat_map(|c| c.iter()),
            ) {
                match utxo.get(input) {
                    Some(out) => {
                        resolved.insert(
                            input.clone(),
                            ResolvedOutput {
                                address: out.address_bytes().to_vec(),
                            },
                        );
                    }
                    None => {
                        all_resolved = false;
                        break;
                    }
                }
            }
            if all_resolved {
                required_signers(&body, &resolved, CardanoEra::Conway)
                    .map_err(LedgerError::RequiredSignerDerivation)?
            } else {
                tx_derived_required_signers(&body, CardanoEra::Conway)
                    .map_err(LedgerError::RequiredSignerDerivation)?
            }
        } else {
            tx_derived_required_signers(&body, CardanoEra::Conway)
                .map_err(LedgerError::RequiredSignerDerivation)?
        };

        // 2. Coverage over the preserved body hash.
        let empty: Vec<VKeyWitnessRef> = Vec::new();
        let witnesses = witness_sets.get(tx_idx).unwrap_or(&empty);
        verify_required_witnesses(&body_hash, &required, witnesses)
            .map_err(LedgerError::WitnessClosure)?;

        tx_idx += 1;
        Ok(())
    };

    match body_enc {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process(body_data, &mut body_offset)?;
            }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(body_data, body_offset)? {
                process(body_data, &mut body_offset)?;
            }
        }
    }

    Ok(())
}

/// Decode each tx's vkey witnesses (witness-set key 0) into the
/// tx_validity witness shape. Reuses the Shelley witness-set decoder
/// (Conway witness sets carry vkey witnesses under key 0 in the same
/// `[vkey, signature]` shape).
fn shelley_witness_sets(
    block: &ade_types::shelley::block::ShelleyBlock,
) -> Result<Vec<Vec<crate::tx_validity::VKeyWitnessRef>>, LedgerError> {
    let raw = crate::shelley::decode_conway_vkey_witness_sets(&block.witness_sets)?;
    Ok(raw
        .into_iter()
        .map(|set| {
            set.into_iter()
                .map(|w| crate::tx_validity::VKeyWitnessRef {
                    vkey: w.vkey,
                    signature: w.signature,
                })
                .collect()
        })
        .collect())
}

/// Epoch boundary transition (T-25A.1 + T-25A.3).
///
/// Performs:
/// 1. Snapshot rotation (mark/set/go)
/// 2. Pool retirements effective at this epoch
/// 3. Reward computation and distribution
/// 4. Treasury/reserves update
///
/// Idempotent: only called once per epoch boundary crossing.
pub fn apply_epoch_boundary_full(
    state: &LedgerState,
    new_epoch: ade_types::EpochNo,
) -> Result<(LedgerState, EpochBoundaryAccounting), crate::governance::GovernanceTerminal> {
    // The full-ledger path is mainnet/Shelley-schedule (epoch detection uses SHELLEY_EPOCH_LENGTH),
    // so its monetary-expansion expected-blocks denominator is the mainnet `432_000 × 1/20 = 21_600`.
    // The accumulator path (preview / multi-network) sources the REAL per-era epoch length from the
    // era schedule instead — see `SelectedBlockCtx::active_slots_per_epoch`.
    // CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S1: a Conway boundary REQUIRES the point-bound base UTxO for the
    // mark (never a reward-only mark). The full-ledger path derives it from its OWN tracked UTxO at the boundary
    // point; a Conway boundary WITHOUT a tracked UTxO fails closed inside the boundary fn
    // (`BoundaryBaseStakeRequired`) rather than construct a reward-only mark. Pre-Conway eras keep the legacy stub.
    let boundary_base_stake = derive_boundary_base_stake(state);
    apply_epoch_boundary_with_registrations(
        state,
        new_epoch,
        None,
        boundary_base_stake.as_ref(),
        crate::state::SHELLEY_EPOCH_LENGTH / 20,
    )
}

/// Derive the point-bound canonical per-credential base-UTxO stake for an epoch boundary from a FULL-ledger
/// state's OWN tracked UTxO (CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S1). Sums `Base(cred)` UTxO coin per
/// credential at the pre-boundary slot — the exact input `build_boundary_mark_snapshot` needs. `Some` only for a
/// Conway boundary WITH a tracked UTxO (`track_utxo=true`); `None` otherwise (pre-Conway keeps the legacy stub;
/// a reduced `track_utxo=false` follower has no full UTxO to derive from and is non-authoritative here).
pub fn derive_boundary_base_stake(
    state: &LedgerState,
) -> Option<crate::epoch_accumulator::BoundaryBaseStake> {
    if state.era != ade_types::CardanoEra::Conway || !state.track_utxo {
        return None;
    }
    let mut base: std::collections::BTreeMap<ade_types::shelley::cert::StakeCredential, ade_types::tx::Coin> =
        std::collections::BTreeMap::new();
    for out in state.utxo_state.utxos.values() {
        if let (coin, crate::reduced_utxo::ReducedStakeRef::Base(cred)) = crate::reduced_utxo::reduce_txout(out) {
            let e = base.entry(cred).or_insert(ade_types::tx::Coin(0));
            e.0 = e.0.saturating_add(coin.0);
        }
    }
    Some(crate::epoch_accumulator::BoundaryBaseStake {
        boundary_point: state.epoch_state.slot,
        canonical_credential_stake: base,
    })
}

/// Apply epoch boundary with an optional override for the credential registration set.
///
/// When `registration_override` is None, uses the PRE state's registrations.
/// When provided, uses the override set for the delta_t2 computation. This allows
/// passing the POST snapshot's registration set, which is closer to the oracle's
/// DState at the boundary tick.
pub fn apply_epoch_boundary_with_registrations(
    state: &LedgerState,
    new_epoch: ade_types::EpochNo,
    registration_override: Option<&std::collections::BTreeMap<ade_types::shelley::cert::StakeCredential, ()>>,
    // The point-bound canonical per-credential base-UTxO stake for the boundary
    // (CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S1). The mark is built INSIDE this fn, AFTER `applyRUpd`, from
    // this base + the staged POST-RUPD reward accounts — never a caller-precomputed (pre-RUPD) mark. The
    // accumulator caller supplies it from the reduced checkpoint; the direct/full-ledger caller derives it from
    // its full UTxO at the same point. `None` on a CONWAY boundary is a structured `BoundaryBaseStakeRequired`
    // terminal (never a reward-only mark); pre-Conway eras keep the legacy reward-only stub (now post-RUPD).
    boundary_base_stake: Option<&crate::epoch_accumulator::BoundaryBaseStake>,
    // The network's expected block-producing slots per epoch = `epochLength × activeSlotCoeff`
    // (preview 86_400 × 1/20 = 4_320; mainnet/preprod 432_000 × 1/20 = 21_600). The monetary-
    // expansion performance factor is `eta = min(1, blocksMade / floor((1-d) × this))`. Passed in
    // from the caller (the accumulator advancer derives it from the era schedule's real per-era
    // epoch length) — NOT a hardcoded mainnet constant. Preview's shorter epoch making this 5×
    // too large was the CE-3d reward-magnitude residual.
    active_slots_per_epoch: u64,
) -> Result<(LedgerState, EpochBoundaryAccounting), crate::governance::GovernanceTerminal> {
    // 1. Reward computation from PRE-rotation go snapshot
    //    Rewards must be computed before rotation — after rotation,
    //    the go snapshot becomes the old set (which may be empty).
    let reserves = state.epoch_state.reserves;
    let treasury = state.epoch_state.treasury;

    // The authoritative snapshots. This is the FULL (authoritative) boundary path; a reduced-validation
    // projection has no stake authority and never reaches this fn, so require `Authoritative` and fail closed
    // with `FullBoundaryStateRequired` rather than read a fabricated snapshot (REDUCED-VALIDATION-BOUNDARY-PLANE).
    let snaps = state.epoch_state.snapshots.as_authoritative().ok_or(
        crate::governance::GovernanceTerminal::FullBoundaryStateRequired {
            boundary_point: state.epoch_state.slot,
        },
    )?;
    // The authoritative cert + governance state. This is the FULL boundary path; a reduced projection carries no
    // cert/gov lifecycle, so require `Authoritative` and fail closed (never a fabricated read) — cert/gov are
    // unavailable by type on the reduced plane (RVBP).
    let cert = state.cert_state.require_full(state.epoch_state.slot)?;
    let gov = state.gov_state.require_full(state.epoch_state.slot)?;

    // 0. CRE S4.3 — plan the Conway governance epoch FIRST, from the immutable pre-boundary view, BEFORE any
    // reward/treasury/proposal mutation. A terminal (potentially-ratifiable / malformed / dormant-required)
    // returns HERE with zero resulting-state change. This is the SINGLE governance authority shared by the
    // accumulator-follow and direct-replay boundaries — neither caller decides proposal removal or deposit
    // routing independently. The plan is APPLIED once below, into the constructed next state.
    let gov_plan: Option<crate::governance::ConwayGovernanceEpochPlan> =
        if state.era == ade_types::CardanoEra::Conway {
            if let Some(gov) = gov.as_ref() {
                let drep_stake = crate::governance::derive_drep_voting_stake(
                    &gov.vote_delegations,
                    &snaps.mark.0,
                );
                let committee_quorum = crate::rational::Rational::new(
                    gov.committee_quorum.0 as i128,
                    gov.committee_quorum.1.max(1) as i128,
                )
                .unwrap_or_else(crate::rational::Rational::one);
                let input = crate::governance::ConwayGovernanceEpochInput {
                    proposals: &gov.proposals,
                    drep_stake: &drep_stake,
                    pool_stake: &snaps.go.0.pool_stakes,
                    committee_members: &gov.committee,
                    committee_quorum: &committee_quorum,
                    pool_thresholds: &gov.pool_voting_thresholds,
                    drep_thresholds: &gov.drep_voting_thresholds,
                    committee_hot_keys: &gov.committee_hot_keys,
                    drep_expiry: &gov.drep_expiry,
                    num_dormant: &gov.num_dormant,
                    current_pparams: &state.protocol_params,
                    current_prev_pparam_action: &gov.prev_pparam_action,
                    new_epoch: new_epoch.0,
                };
                // Freeze the registration view used for deposit routing: the PRE-boundary registrations
                // (no caller re-checks registration against a partially-updated state).
                let registrations = &cert.delegation.registrations;
                Some(crate::governance::plan_conway_governance_epoch(&input, |c| {
                    registrations.contains_key(c)
                })?)
            } else {
                None
            }
        } else {
            None
        };

    // --- Shelley eta: decentralization-adjusted monetary expansion ---
    // eta = min(1, blocksMade / expectedBlocks) when d < 0.8
    // eta = 1 when d >= 0.8
    // expectedBlocks = floor((1-d) * epochLength * activeSlotCoeff)
    let d = &state.protocol_params.decentralization;
    let d_threshold = crate::rational::Rational::new(4, 5)
        .unwrap_or_else(crate::rational::Rational::one);

    let total_blocks_produced: u64 = state.epoch_state.block_production
        .values().copied().sum();

    // Compute eta as Rational for precision
    let eta = if d.numerator() * d_threshold.denominator()
        >= d_threshold.numerator() * d.denominator()
    {
        // d >= 0.8: eta = 1 (highly centralized, use full expansion)
        crate::rational::Rational::one()
    } else {
        // expectedBlocks = floor((1-d) * epochLength * activeSlotCoeff)
        // = floor((1-d) * active_slots_per_epoch)  [preview 4_320, mainnet/preprod 21_600]
        let one_minus_d = crate::rational::Rational::one()
            .checked_sub(d)
            .unwrap_or_else(crate::rational::Rational::one);
        let epoch_slots =
            crate::rational::Rational::from_integer(active_slots_per_epoch as i128);
        let expected_rat = one_minus_d.checked_mul(&epoch_slots)
            .unwrap_or_else(crate::rational::Rational::one);
        let expected_blocks = expected_rat.floor().max(1) as u64;

        if total_blocks_produced >= expected_blocks {
            crate::rational::Rational::one()
        } else if expected_blocks > 0 {
            crate::rational::Rational::new(
                total_blocks_produced as i128, expected_blocks as i128,
            ).unwrap_or_else(crate::rational::Rational::one)
        } else {
            crate::rational::Rational::one()
        }
    };

    // deltaR1 = floor(eta * rho * reserves)
    let delta_r1 = {
        let reserves_rat = crate::rational::Rational::from_integer(reserves.0 as i128);
        let rho = &state.protocol_params.monetary_expansion;
        reserves_rat.checked_mul(rho)
            .and_then(|r| r.checked_mul(&eta))
            .map(|r| {
                let f = r.floor();
                if f < 0 { 0u64 } else { f as u64 }
            })
            .unwrap_or(0u64)
    };

    // total_reward = deltaR1 + epoch_fees
    let total_reward = ade_types::tx::Coin(
        delta_r1.saturating_add(state.epoch_state.epoch_fees.0)
    );

    // deltaT1 = floor(total_reward * tau)
    let treasury_delta = {
        let total_rat = crate::rational::Rational::from_integer(total_reward.0 as i128);
        let delta = total_rat.checked_mul(&state.protocol_params.treasury_growth);
        match delta {
            Some(d) => {
                let floored = d.floor();
                if floored < 0 { 0u64 } else { floored as u64 }
            }
            None => 0u64,
        }
    };

    // 2. Pool reward allocation from PRE-rotation go snapshot
    let pool_reward_pot = total_reward.0.saturating_sub(treasury_delta);
    let go = &snaps.go;

    // Total active stake = sum of delegated pool stakes from go snapshot.
    let total_active_stake: u64 = go.0.pool_stakes.values()
        .map(|c| c.0)
        .fold(0u64, |a, b| a.saturating_add(b));

    // totalStake: the denominator for sigma and pledge ratio in maxPool.
    //
    // Haskell source (confirmed): `totalStake = circulation es maxSupply`
    // where `circulation (EpochState acnt _ _ _) supply = supply <-> casReserves acnt`
    // i.e. totalStake = maxLovelaceSupply - reserves. Same for ALL protocol versions.
    //
    // Pre-Mary (Shelley/Allegra, PV < 4): totalStake = activeStake.
    //   Proven exact for Allegra epoch 236→237 (99.1% + MIR = 100.0%).
    //   The Haskell source uses circulation for all eras, but Allegra empirically
    //   matches activeStake. The PV < 4 branch may reflect a different code path
    //   in the pre-Mary Haskell implementation (before the SnapShot refactor).
    //
    // Mary+ (PV >= 4): totalStake = circulation = maxLovelaceSupply - reserves.
    //   Confirmed from: (1) FreeVars_totalStake in Mary epoch 267 mid-epoch dump,
    //   (2) Haskell source: `circulation` function in PulsingReward.hs.
    //   Alonzo 310→311: 99.95%. Babbage 406→407: 97.97%. Conway 528→529: 100.38%.
    //
    // Dual-denominator (PV 4+, confirmed from Haskell source + oracle data):
    //   sigma  = poolStake / totalStake (circulation) — for maxPool bracket
    //   sigmaA = poolStake / totalActiveStake          — for apparentPerformance
    //   apparentPerformance is NOT capped at 1.0 (over-performing pools get more than maxPool)
    //   Confirmed: PREALL/circ/actv/noc gives 100.0000% for Babbage 406→407 and Conway 528→529.
    let total_stake: u64 = if state.protocol_params.protocol_major < 4 {
        // Shelley (2) / Allegra (3): use activeStake
        total_active_stake
    } else {
        // Mary (4+): use circulation = maxLovelaceSupply - reserves
        state.max_lovelace_supply.saturating_sub(reserves.0)
    };

    // Allocate rewards to pools that have params
    let mut total_pool_rewards = 0u64;
    let mut total_member_rewards = 0u64;
    let mut rewarded_pool_count = 0usize;
    let mut reward_deltas = std::collections::BTreeMap::new();
    let mut _sum_f = 0u64; // sum of raw f values (floor(maxPool*perf))
    let mut _sum_max_pool = 0u64; // sum of maxPool values (before perf multiply)

    eprintln!("  [epoch_boundary] protocol_major={} total_stake={} active_stake={} pool_pot={} go_pools={} cert_pools={}",
        state.protocol_params.protocol_major, total_stake, total_active_stake, pool_reward_pot,
        go.0.pool_stakes.len(), cert.pool.pools.len());

    if total_stake > 0 && total_active_stake > 0 && pool_reward_pot > 0 {
        for (pool_id, pool_stake) in &go.0.pool_stakes {
            let params = match cert.pool.pools.get(pool_id) {
                Some(p) => p,
                None => continue,
            };

            // Pool performance = blocks_produced / expected_blocks_for_this_pool
            // expected_for_pool = expected_total * (pool_stake / total_stake)
            let blocks_produced = state.epoch_state.block_production
                .get(pool_id)
                .copied()
                .unwrap_or(0);
            if blocks_produced == 0 {
                continue; // Zero performance → zero reward
            }

            // Gather delegator stakes for this pool
            let delegator_stakes: std::collections::BTreeMap<ade_types::Hash28, ade_types::tx::Coin> =
                go.0.delegations.iter()
                    .filter(|(_, (pid, _))| pid == pool_id)
                    .map(|(cred, (_, coin))| (cred.clone(), *coin))
                    .collect();

            let margin = crate::rational::Rational::new(
                params.margin.0 as i128,
                params.margin.1 as i128,
            ).unwrap_or_else(crate::rational::Rational::zero);

            // sigma = pool_stake / totalStake (circulation) — for maxPool bracket
            // sigmaA = pool_stake / totalActiveStake — for apparentPerformance
            let sigma = crate::rational::Rational::new(
                pool_stake.0 as i128, total_stake as i128,
            ).unwrap_or_else(crate::rational::Rational::zero);

            // apparentPerformance = beta / sigmaA (Haskell: mkApparentPerformance)
            //   beta = blocks / totalBlocks
            //   sigmaA = poolStake / totalActiveStake
            //   perf = beta / sigmaA = blocks * totalActiveStake / (totalBlocks * poolStake)
            //   NOT capped at 1.0 — over-performing pools earn more than maxPool.
            //   Confirmed: uncapped + activeStake gives 100.0000% for Babbage/Conway.
            let perf_denom = if state.protocol_params.protocol_major < 4 {
                total_active_stake // Allegra: same as totalStake (both use activeStake)
            } else {
                total_active_stake // Mary+: sigmaA uses activeStake (NOT circulation)
            };
            let performance = if total_blocks_produced > 0 && pool_stake.0 > 0 {
                crate::rational::Rational::new(
                    (blocks_produced as i128) * (perf_denom as i128),
                    (total_blocks_produced as i128) * (pool_stake.0 as i128),
                ).unwrap_or_else(crate::rational::Rational::one)
            } else {
                crate::rational::Rational::one()
            };

            // Shelley maxPoolReward (two-step with separate floors):
            //   maxPool = floor(R / (1+a0) * (sigma' + s'*a0*(sigma'-s'*(z-sigma')/z)))
            //   poolReward = floor(maxPool * apparentPerformance)
            // where sigma' = min(sigma, z), s' = min(s, z), z = 1/k
            let a0 = &state.protocol_params.pool_influence;
            let k = state.protocol_params.n_opt as i128;
            let z = crate::rational::Rational::new(1, k)
                .unwrap_or_else(crate::rational::Rational::zero);

            // sigma' = min(sigma, z) — cap at saturation
            let sigma_prime = if sigma.numerator() * z.denominator() > z.numerator() * sigma.denominator() {
                z.clone()
            } else {
                sigma.clone()
            };

            // s' = min(pledge/total_stake, z)
            let s = crate::rational::Rational::new(
                params.pledge.0 as i128, total_stake as i128,
            ).unwrap_or_else(crate::rational::Rational::zero);
            let s_prime = if s.numerator() * z.denominator() > z.numerator() * s.denominator() {
                z.clone()
            } else {
                s
            };

            // Shelley maxPool bracket (matches Haskell exactly):
            //   factor4 = (z - σ') / z
            //   factor3 = (σ' - s' × factor4) / z
            //   bracket = σ' + s' × a0 × factor3
            let bracket = {
                let factor4 = z.checked_sub(&sigma_prime)
                    .and_then(|d| d.checked_div(&z));
                let factor3 = factor4.and_then(|f4| {
                    s_prime.checked_mul(&f4)
                        .and_then(|sf4| sigma_prime.checked_sub(&sf4))
                        .and_then(|num| num.checked_div(&z))
                });
                let pledge_bonus = factor3.and_then(|f3| {
                    s_prime.checked_mul(a0)
                        .and_then(|r| r.checked_mul(&f3))
                });
                pledge_bonus.and_then(|pb| sigma_prime.checked_add(&pb))
            };

            // Step 1: maxPool = floor(R / (1+a0) * bracket)
            let one_plus_a0 = crate::rational::Rational::one()
                .checked_add(a0)
                .unwrap_or_else(crate::rational::Rational::one);

            let max_pool = if let Some(br) = bracket {
                let pot_rat = crate::rational::Rational::from_integer(pool_reward_pot as i128);
                pot_rat.checked_mul(&br)
                    .and_then(|r| r.checked_div(&one_plus_a0))
                    .map(|r| r.floor().max(0) as u64)
                    .unwrap_or_else(|| {
                        (pool_reward_pot as u128 * pool_stake.0 as u128
                            / total_stake as u128 * 10 / 13) as u64
                    })
            } else {
                (pool_reward_pot as u128 * pool_stake.0 as u128
                    / total_stake as u128 * 10 / 13) as u64
            };

            if max_pool == 0 {
                continue;
            }

            // Shelley pledge satisfaction: if pledge > sum(owner_stakes) → maxPool = 0
            // Haskell uses full go snapshot stake (not filtered by pool): an owner's
            // total active stake counts toward pledge regardless of their delegation.
            // Only apply for Mary+ (protocol_major >= 4) where owner parsing is reliable.
            // Pre-Mary: owner encoding differs, skip the check (matches proven formula).
            if state.protocol_params.protocol_major >= 4
                && !params.owners.is_empty()
                && params.pledge.0 > 0
            {
                let owner_stake: u64 = params.owners.iter()
                    .map(|owner| {
                        go.0.delegations.get(owner)
                            .map(|(_, c)| c.0)
                            .unwrap_or(0)
                    })
                    .sum();
                if params.pledge.0 > owner_stake {
                    continue;
                }
            }

            // Step 2: poolReward = floor(maxPool * apparentPerformance)
            let pool_max = {
                let max_rat = crate::rational::Rational::from_integer(max_pool as i128);
                max_rat.checked_mul(&performance)
                    .map(|r| r.floor().max(0) as u64)
                    .unwrap_or(max_pool)
            };

            if pool_max == 0 {
                continue;
            }

            // Shelley reward split (matches Haskell cardano-ledger exactly):
            //
            // leaderReward = c + floor((f-c) * (m + (1-m)*s_op/σ))
            //   where s_op = operator's own stake in the pool
            //   Bundles the operator's margin AND their pro-rata member share
            //   into a single floor operation.
            //
            // memberReward(t) = floor((f-c) * (1-m) * t / σ)
            //   Applied to each delegator EXCEPT the operator (who already
            //   got their share via leaderReward).

            // Identify operator credential from reward_account
            let op_cred: Option<ade_types::Hash28> = if params.reward_account.len() >= 29 {
                let mut cred_bytes = [0u8; 28];
                cred_bytes.copy_from_slice(&params.reward_account[1..29]);
                Some(ade_types::Hash28(cred_bytes))
            } else {
                None
            };

            _sum_f += pool_max;
            _sum_max_pool += max_pool;
            if performance.numerator() >= performance.denominator() {
                // (performance uncapped — over-performing pools earn more than maxPool)
            }

            // hardforkBabbageForgoRewardPrefilter: at PV ≤ 6, leader/member
            // rewards are only distributed to registered accounts. Unregistered
            // accounts' shares stay in the pool residual (dr2 → reserves).
            // At PV > 6 (Babbage+), rewards are computed for ALL accounts;
            // unregistered rewards are routed to treasury via delta_t2 in applyRUpd.
            let pv_prefilter = state.protocol_params.protocol_major <= 6;
            // For PV≤6 pre-filter, use registration_override if provided (closest
            // to the DState when the pulser actually ran), otherwise fall back to
            // cert.delegation.registrations. The delta_t2 check in
            // applyRUpd uses the same registration source for consistency.

            // Registration check helper: uses override set if provided,
            // falls back to PRE state registrations.
            let is_cred_registered = |h: &ade_types::Hash28| -> bool {
                // Reward-account hash from address bytes — no key/script discriminant
                // is encoded at this boundary; the CertState registrations map is
                // keyed the same way, so KeyHash is the consistent projection.
                let sc = ade_types::shelley::cert::StakeCredential::KeyHash(h.clone());
                if let Some(override_regs) = registration_override {
                    override_regs.contains_key(&sc)
                } else {
                    cert.delegation.registrations.contains_key(&sc)
                }
            };

            if pool_max <= params.cost.0 {
                // Pool reward doesn't cover cost — operator gets all of it
                if pv_prefilter {
                    let op_registered = op_cred.as_ref()
                        .map(|oc| is_cred_registered(oc))
                        .unwrap_or(false);
                    if !op_registered {
                        rewarded_pool_count += 1;
                        continue;
                    }
                }
                if let Some(ref oc) = op_cred {
                    let entry = reward_deltas.entry(oc.clone())
                        .or_insert(ade_types::tx::Coin(0));
                    entry.0 = entry.0.saturating_add(pool_max);
                }
                total_pool_rewards = total_pool_rewards.saturating_add(pool_max);
                rewarded_pool_count += 1;
                continue;
            }

            let f_minus_c = pool_max - params.cost.0;
            let one_minus_m = crate::rational::Rational::one()
                .checked_sub(&margin)
                .unwrap_or_else(crate::rational::Rational::one);

            // Operator's stake share s/σ in the leader reward. Cardano's `s` is the POOL OWNERS'
            // stake (the `poolOwners` set), summed from the go snapshot — NOT the reward account's own
            // delegation. The owners are also EXCLUDED from member rewards below (their share rides in
            // the leader term); paying a non-owner reward account as both leader and member, and an
            // owner as a member, mis-attributes the per-account split (the totals are unchanged, so the
            // pots are not). Gated to Mary+ where owner parsing is reliable; pre-Mary keeps the proven
            // reward-account projection.
            let use_owners = state.protocol_params.protocol_major >= 4 && !params.owners.is_empty();
            let owner_stake: u64 = if use_owners {
                params.owners.iter()
                    .map(|o| go.0.delegations.get(o).map(|(_, c)| c.0).unwrap_or(0))
                    .sum()
            } else {
                op_cred.as_ref()
                    .and_then(|oc| go.0.delegations.get(oc))
                    .map(|(_, c)| c.0)
                    .unwrap_or(0)
            };
            let op_share = crate::rational::Rational::new(
                owner_stake as i128, pool_stake.0 as i128,
            ).unwrap_or_else(crate::rational::Rational::zero);

            // leaderReward = c + floor((f-c) * (m + (1-m)*s_op/σ))
            let leader_term = margin.checked_add(
                &one_minus_m.checked_mul(&op_share)
                    .unwrap_or_else(crate::rational::Rational::zero)
            ).unwrap_or(margin.clone());
            let leader_reward = params.cost.0 + crate::rational::Rational::from_integer(f_minus_c as i128)
                .checked_mul(&leader_term)
                .map(|r| r.floor().max(0) as u64)
                .unwrap_or(0);

            // Route leader reward: at PV≤6, only distribute if operator is registered
            let distribute_leader = if pv_prefilter {
                op_cred.as_ref()
                    .map(|oc| is_cred_registered(oc))
                    .unwrap_or(false)
            } else {
                true
            };

            if distribute_leader {
                if let Some(ref oc) = op_cred {
                    let entry = reward_deltas.entry(oc.clone())
                        .or_insert(ade_types::tx::Coin(0));
                    entry.0 = entry.0.saturating_add(leader_reward);
                }
                total_pool_rewards = total_pool_rewards.saturating_add(leader_reward);
            }

            // memberReward(t) = floor((f-c) * (1-m) * t / σ)
            // for each delegator EXCEPT the operator
            let member_factor = crate::rational::Rational::from_integer(f_minus_c as i128)
                .checked_mul(&one_minus_m)
                .unwrap_or_else(crate::rational::Rational::zero);

            let mut member_distributed = 0u64;
            if pool_stake.0 > 0 {
                for (cred, stake) in &delegator_stakes {
                    // Skip the pool OWNERS — their stake rides in the leader term (s), so cardano
                    // excludes them from member rewards. Pre-Mary (no reliable owners) falls back to
                    // excluding the reward account, matching the proven pre-Mary path.
                    let is_owner = if use_owners {
                        params.owners.contains(cred)
                    } else {
                        op_cred.as_ref() == Some(cred)
                    };
                    if is_owner { continue; }
                    if stake.0 == 0 { continue; }
                    // PV≤6 pre-filter: skip unregistered members
                    if pv_prefilter && !is_cred_registered(cred) {
                        continue;
                    }
                    let share = crate::rational::Rational::new(
                        stake.0 as i128, pool_stake.0 as i128,
                    ).unwrap_or_else(crate::rational::Rational::zero);
                    let member_reward = member_factor.checked_mul(&share)
                        .map(|r| r.floor().max(0) as u64)
                        .unwrap_or(0);
                    if member_reward > 0 {
                        member_distributed += member_reward;
                        let entry = reward_deltas.entry(cred.clone()).or_insert(ade_types::tx::Coin(0));
                        entry.0 = entry.0.saturating_add(member_reward);
                    }
                }
            }

            if !distribute_leader {
                // Count leader reward as part of pool processing for pool count,
                // but don't add to total (stays in dr2)
            }
            total_member_rewards = total_member_rewards.saturating_add(member_distributed);
            rewarded_pool_count += 1;
        }
    }


    // deltaT2: rewards to unregistered credentials go to treasury.
    // Haskell applyRUpd: treasury receives deltaT + frTotalUnregistered.
    // frTotalUnregistered = rewards for credentials NOT in the DState accounts map.
    // This applies at ALL protocol versions (not just PV ≤ 6).
    // hardforkBabbageForgoRewardPrefilter only affects leader reward COLLECTION,
    // not the final applyRUpd filtering.
    let mut delta_t2 = 0u64;
    let mut delegation = cert.delegation.clone();

    for (cred, reward) in &reward_deltas {
        // The reward distribution keys by Hash28 (no key/script discriminant), but registrations carry
        // it. Resolve to the REGISTERED stake credential — key-hash first, then script-hash — and credit
        // THAT one. A script-hash staker is registered as ScriptHash; projecting every hash to KeyHash
        // (the old behaviour) failed the lookup for script stakers and routed their reward to the
        // treasury (a treasury-vs-member-rewards split error). A given hash is registered as at most one
        // discriminant; if neither is registered the reward goes to the treasury (deltaT2), as before.
        let as_key = ade_types::shelley::cert::StakeCredential::KeyHash(cred.clone());
        let as_script = ade_types::shelley::cert::StakeCredential::ScriptHash(cred.clone());
        let registered = if let Some(override_regs) = registration_override {
            if override_regs.contains_key(&as_key) {
                Some(as_key)
            } else if override_regs.contains_key(&as_script) {
                Some(as_script)
            } else {
                None
            }
        } else if delegation.registrations.contains_key(&as_key) {
            Some(as_key)
        } else if delegation.registrations.contains_key(&as_script) {
            Some(as_script)
        } else {
            None
        };
        match registered {
            Some(stake_cred) => {
                let entry = delegation.rewards
                    .entry(stake_cred)
                    .or_insert(ade_types::tx::Coin(0));
                entry.0 = entry.0.saturating_add(reward.0);
            }
            None => {
                delta_t2 = delta_t2.saturating_add(reward.0);
            }
        }
    }

    let _ = (rewarded_pool_count, total_pool_rewards, total_member_rewards, total_stake);

    // 3. Snapshot rotation (AFTER `applyRUpd`). The mark reads the STAGED POST-RUPD reward accounts
    //    (`delegation`, already updated at the applyRUpd step above) + the point-bound EXACT base UTxO — never
    //    the pre-RUPD view (the pre-RUPD mark was the CE-3d go-stake residual −343,260,172,883). Delegations are
    //    read here PRE-POOLREAP (POOLREAP runs below), matching cardano's SNAP-before-POOLREAP order. A CONWAY
    //    boundary WITHOUT the base input is a structured terminal (never a reward-only mark), zero mutation.
    let new_mark = match boundary_base_stake {
        Some(base) => crate::epoch_accumulator::build_boundary_mark_snapshot(
            base,
            crate::epoch_accumulator::PostRupdRewards::after_rupd(&delegation),
        ),
        None => {
            // ANY Conway boundary that reaches this fn REQUIRES the base — fail closed rather than fabricate a
            // reward-only mark (the pre-RUPD/base-less mark was the CE-3d go-stake residual). The reduced-validation
            // path (`track_utxo=false`) never reaches here: `dispatch_epoch_boundary` routes every `track_utxo=false`
            // Conway boundary to `apply_reduced_epoch_boundary` (which emits no mark at all), so the reward-only stub
            // is UNREPRESENTABLE for Conway — the gate is on the era alone, not `&& track_utxo`, closing the
            // footgun for any future direct caller rather than relying on the routing to keep it unreachable. Only
            // pre-Conway eras (which have no base-stake authority) keep the legacy stub, reading POST-RUPD `delegation`.
            if state.era == ade_types::CardanoEra::Conway {
                return Err(crate::governance::GovernanceTerminal::BoundaryBaseStakeRequired {
                    boundary_point: state.epoch_state.slot,
                });
            }
            crate::epoch::StakeSnapshot {
                delegations: delegation.delegations.iter()
                    .map(|(cred, pool)| {
                        let stake = delegation.rewards
                            .get(cred)
                            .copied()
                            .unwrap_or(ade_types::tx::Coin(0));
                        (cred.hash().clone(), (pool.clone(), stake))
                    })
                    .collect(),
                pool_stakes: {
                    let mut ps = std::collections::BTreeMap::new();
                    for pool in delegation.delegations.values() {
                        ps.entry(pool.clone()).or_insert(ade_types::tx::Coin(0));
                    }
                    ps
                },
            }
        }
    };
    let rotated = crate::epoch::rotate_snapshots(
        snaps,
        new_mark,
    );

    // 4. POOLREAP — the single canonical cardano transition (Shelley PoolReap.hs:132-241, which
    //    Conway reuses), consolidated here so the full-ledger and accumulator paths share ONE order
    //    whose halves cannot silently fail to compose:
    //      (a) adopt staged future-pool re-registrations,
    //      (b) reap the pools retiring at EXACTLY this epoch (== e, never <= e),
    //      (c) refund each reaped pool's deposit to its OWN reward-account credential by the real
    //          key/script discriminant (registered → that reward account, unregistered → treasury),
    //      (d) clear the reaped pools' delegators, then
    //      (e) remove the reaped pools from the active set + the retiring schedule.
    let mut pool_state = cert.pool.clone();
    let mut poolreap_to_treasury = 0u64;
    let pool_deposit = state.protocol_params.pool_deposit.0;

    // (a) Future-pool adoption: a staged re-registration whose pool is still active becomes the
    //     active params; an orphan future (no matching active pool) is dropped.
    let adopted = std::mem::take(&mut pool_state.future_pools);
    for (pool_id, params) in adopted {
        if pool_state.pools.contains_key(&pool_id) {
            pool_state.pools.insert(pool_id, params);
        }
    }

    // (b) The pools scheduled to retire at EXACTLY this epoch.
    let retired: std::collections::BTreeSet<ade_types::tx::PoolId> = pool_state
        .retiring
        .iter()
        .filter(|(_, retire_epoch)| retire_epoch.0 == new_epoch.0)
        .map(|(pool_id, _)| pool_id.clone())
        .collect();

    // (c) Refund each reaped pool's deposit to its own reward-account credential, decoded with the
    //     real discriminant (byte 0 bit 4 = 0x10 ⇒ ScriptHash). Registered → credit its reward
    //     balance; unregistered → treasury. A malformed (≠29-byte) reward account refunds nowhere.
    for pool_id in &retired {
        if let Some(params) = pool_state.pools.get(pool_id) {
            if let Some(stake_cred) =
                crate::epoch_accumulator::reward_account_credential(&params.reward_account)
            {
                if delegation.registrations.contains_key(&stake_cred) {
                    let entry = delegation
                        .rewards
                        .entry(stake_cred)
                        .or_insert(ade_types::tx::Coin(0));
                    entry.0 = entry.0.saturating_add(pool_deposit);
                } else {
                    poolreap_to_treasury = poolreap_to_treasury.saturating_add(pool_deposit);
                }
            }
        }
    }

    // (d) Clear the reaped pools' delegators — a credential delegated to a reaped pool is
    //     un-delegated so it cannot silently reattach if that pool id re-registers later.
    if !retired.is_empty() {
        delegation
            .delegations
            .retain(|_cred, pool_id| !retired.contains(pool_id));
    }

    // (e) Remove the reaped pools from the active set + the retiring schedule.
    for pool_id in &retired {
        pool_state.pools.remove(pool_id);
        pool_state.retiring.remove(pool_id);
    }

    // 4b. CRE S4.3 — APPLY the pre-computed governance plan into the boundary result. The plan was validated
    // up-front (a terminal already returned before any mutation). S4.3a applies the next proposal set + the
    // explicit deposit returns ONLY: a registered return-address credential is credited to its reward account;
    // a deregistered one routes to the TREASURY (unclaimed). No enactment (pparams/committee/constitution/root
    // unchanged) — a threshold-passing proposal is terminal (returned above), never enacted here. This
    // expired-deposit REFUND now runs on BOTH the accumulator and direct-replay boundaries; the replay path
    // previously dropped expired proposals with no refund (a path-dependent partial-finalization bug).
    let mut gov_deposit_to_treasury = 0u64;
    let new_gov_state = match (gov.as_ref(), gov_plan.as_ref()) {
        (Some(gov), Some(plan)) => {
            for ret in &plan.deposit_returns {
                match ret {
                    crate::governance::DepositReturn::ToRewardAccount { credential, amount, .. } => {
                        let bal = delegation
                            .rewards
                            .entry(credential.clone())
                            .or_insert(ade_types::tx::Coin(0));
                        bal.0 = bal.0.saturating_add(amount.0);
                    }
                    crate::governance::DepositReturn::ToTreasury { amount, .. } => {
                        gov_deposit_to_treasury = gov_deposit_to_treasury.saturating_add(amount.0);
                    }
                    crate::governance::DepositReturn::NoDeposit { .. } => {}
                }
            }
            // Committee/quorum/thresholds/delegations/dormancy carry forward unchanged; the removed proposals
            // leave the set (original order preserved by the planner). CRE S4.3c: the previous-pparam-action root
            // ADVANCES to the enacted winner when the plan enacts (`Set`), else carries forward (`Unchanged`).
            Some(crate::state::ConwayGovState {
                proposals: plan.proposals.clone(),
                committee: gov.committee.clone(),
                committee_quorum: gov.committee_quorum,
                drep_expiry: gov.drep_expiry.clone(),
                gov_action_lifetime: gov.gov_action_lifetime,
                vote_delegations: gov.vote_delegations.clone(),
                pool_voting_thresholds: gov.pool_voting_thresholds.clone(),
                drep_voting_thresholds: gov.drep_voting_thresholds.clone(),
                committee_hot_keys: gov.committee_hot_keys.clone(),
                num_dormant: gov.num_dormant.clone(),
                prev_pparam_action: match &plan.prev_pparam_action {
                    crate::governance::PrevPParamActionDelta::Set(id) => {
                        crate::state::PreviousPParamAction::Enacted(id.clone())
                    }
                    crate::governance::PrevPParamActionDelta::Unchanged => gov.prev_pparam_action.clone(),
                },
            })
        }
        _ => gov.clone(),
    };

    // 5. Update reserves and treasury per Shelley spec:
    //    deltaR2 = pool_pot - sum(all_computed_rewards)  [undistributed returns to reserves]
    //    reserves' = reserves - deltaR1 + deltaR2
    //    treasury' = treasury + deltaT1 + deltaT2  [deltaT2 = filtered undeliverable rewards]
    let sum_rewards = total_pool_rewards.saturating_add(total_member_rewards);
    let delta_r2 = pool_reward_pot.saturating_sub(sum_rewards);
    let new_reserves = ade_types::tx::Coin(
        reserves.0
            .saturating_sub(delta_r1)
            .saturating_add(delta_r2)
    );
    let new_treasury = ade_types::tx::Coin(
        treasury.0
            .saturating_add(treasury_delta)
            .saturating_add(delta_t2)
            .saturating_add(poolreap_to_treasury)
            .saturating_add(gov_deposit_to_treasury)
    );

    let cert_state = crate::delegation::CertState {
        delegation,
        pool: pool_state,
    };

    let eta_num = eta.numerator().unsigned_abs() as u64;
    let eta_den = eta.denominator().unsigned_abs() as u64;

    let accounting = EpochBoundaryAccounting {
        delta_r1,
        delta_r2,
        delta_t1: treasury_delta,
        delta_t2,
        total_reward: total_reward.0,
        pool_reward_pot,
        sum_rewards,
        rewarded_pool_count: rewarded_pool_count as u64,
        eta_numerator: eta_num,
        eta_denominator: eta_den.max(1),
        epoch_fees: state.epoch_state.epoch_fees.0,
        // MIR: zeroed here — populated by the caller when MIR data is available.
        // MIR cannot be computed from the reward formula alone; it requires
        // parsing the InstantaneousRewards from the ledger state.
        mir_reserves_to_treasury: 0,
        mir_reserves_to_accounts: 0,
        mir_treasury_to_accounts: 0,
    };

    let new_state = LedgerState {
        utxo_state: state.utxo_state.clone(),
        epoch_state: crate::state::EpochState {
            epoch: new_epoch,
            slot: state.epoch_state.slot,
            // The FULL boundary produces authoritative snapshots (base + post-RUPD rewards).
            snapshots: crate::epoch::EpochStakeSnapshots::Authoritative(rotated),
            reserves: new_reserves,
            treasury: new_treasury,
            block_production: std::collections::BTreeMap::new(),
            epoch_fees: ade_types::tx::Coin(0),
        },
        // CRE S4.3c: a supported exec-units enactment writes the new Tx/block memory limits (steps preserved) in
        // the SAME boundary construction as the proposal removal + deposit returns + root advance; otherwise the
        // parameters carry forward unchanged. One atomic result — no half of the enactment can land alone.
        protocol_params: match gov_plan.as_ref().map(|p| &p.pparams) {
            Some(crate::governance::PParamsDelta::Set(pp)) => (**pp).clone(),
            _ => state.protocol_params.clone(),
        },
        era: state.era,
        track_utxo: state.track_utxo,
        cert_state: crate::state::CertStateProjection::Authoritative(cert_state),
        max_lovelace_supply: state.max_lovelace_supply,
        gov_state: crate::state::GovStateProjection::Authoritative(new_gov_state),
        conway_deposit_params: state.conway_deposit_params.clone(),
    };

    Ok((new_state, accounting))
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

/// Detailed accounting of an epoch boundary transition.
///
/// Decomposes reserves and treasury changes into four distinct flows:
///
/// 1. **Reward distribution**: reserves → reward pot → pools → accounts + treasury
///    - delta_r1: monetary expansion from reserves
///    - delta_r2: undistributed rewards returned to reserves
///    - delta_t1: treasury's share (tau) of the reward pot
///    - delta_t2: rewards to unregistered credentials redirected to treasury
///    - sum_rewards: total computed pool rewards (operator + member)
///
/// 2. **MIR reserves→treasury**: direct transfer, separate from rewards
///    - mir_reserves_to_treasury
///
/// 3. **MIR reserves→accounts**: reserves directly to individual staker accounts
///    - mir_reserves_to_accounts
///
/// 4. **MIR treasury→accounts**: treasury directly to individual staker accounts
///    - mir_treasury_to_accounts
///
/// These flows must never be collapsed into a single number. The accounting
/// identity `implied_sum = reserves_decrease - treasury_increase + fees`
/// conflates reward distribution with MIR effects and will produce false
/// divergences if MIR is non-zero.
///
/// All values in lovelace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochBoundaryAccounting {
    // --- Reward distribution ---
    /// floor(min(1, eta) * rho * reserves) — monetary expansion from reserves
    pub delta_r1: u64,
    /// pool_pot - sum_rewards — undistributed remainder returned to reserves
    pub delta_r2: u64,
    /// floor(total_reward * tau) — treasury's share of the reward pot
    pub delta_t1: u64,
    /// sum of rewards filtered for unregistered credentials → treasury
    pub delta_t2: u64,
    /// total_reward = delta_r1 + epoch_fees
    pub total_reward: u64,
    /// pool_reward_pot = total_reward - delta_t1
    pub pool_reward_pot: u64,
    /// sum of all computed pool rewards (operator + member)
    pub sum_rewards: u64,
    /// number of pools that received rewards
    pub rewarded_pool_count: u64,
    /// eta = min(1, blocksMade / expectedBlocks)
    pub eta_numerator: u64,
    pub eta_denominator: u64,
    /// epoch fees added to reward pot
    pub epoch_fees: u64,

    // --- MIR (Move Instantaneous Rewards) ---
    // Protocol-authorized transfers separate from ordinary rewards.
    // Accumulated during the epoch via MIR certificates, applied at boundary.
    /// MIR: reserves → treasury (direct transfer, not via reward pot)
    pub mir_reserves_to_treasury: u64,
    /// MIR: reserves → individual staker accounts (bypasses reward pot)
    pub mir_reserves_to_accounts: u64,
    /// MIR: treasury → individual staker accounts
    pub mir_treasury_to_accounts: u64,
}

/// Process certificates from a block to accumulate delegation/pool state.
///
/// For each tx body with a `certs` field (key 4), decode the certificates
/// and apply them to the certificate state using `apply_cert`.
pub(crate) fn process_block_certificates(
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
    state: &LedgerState,
) -> Result<(crate::delegation::CertState, Option<crate::state::ConwayGovState>), LedgerError> {
    // This path runs only at track_utxo=true (full state present) — require the authoritative cert/gov and fail
    // closed rather than accommodate a reduced projection (RVBP; cert/gov unavailable by type on the reduced plane).
    if block.tx_count == 0 {
        return Ok((
            state.cert_state.require_full(state.epoch_state.slot)?.clone(),
            state.gov_state.require_full(state.epoch_state.slot)?.clone(),
        ));
    }

    let mut cert_state = state.cert_state.require_full(state.epoch_state.slot)?.clone();
    // PHASE4-B5: governance state is threaded alongside cert-state. When present
    // (Conway, governance tracked) gov-affecting certs accumulate into it; when
    // absent (None) governance is not tracked and the gov half is skipped — the
    // same gating as track_utxo for cert-state.
    let mut gov_state = state.gov_state.require_full(state.epoch_state.slot)?.clone();
    // Lazy env: only DRep register/update (tags 16/18) consult it, so an absent
    // drep_activity fails fast exactly when an expiry must be computed, never for
    // env-free gov certs.
    let gov_env = state.gov_cert_env().ok();
    let mut offset = 0;
    let data = &block.tx_bodies;
    let enc = cbor::read_array_header(data, &mut offset)?;
    let key_deposit = state.protocol_params.key_deposit;

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
                // Capture cert bytes and accumulate fail-closed: a decode or
                // apply error halts the block transition (this path runs only at
                // track_utxo, i.e. with full state present — there is no
                // reduced-state replay accommodation to swallow for).
                let cert_start = *offset;
                let (_, cert_end) = cbor::skip_item(data, offset)?;
                let cert_bytes = &data[cert_start..cert_end];
                let (cs, gs) = accumulate_tx_certs(
                    era,
                    cert_bytes,
                    &cert_state,
                    &gov_state,
                    key_deposit,
                    gov_env.as_ref(),
                )?;
                cert_state = cs;
                gov_state = gs;
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

    Ok((cert_state, gov_state))
}

/// Accumulate one tx's certificate array into cert-state and governance state,
/// era-dispatched and fail-closed (PHASE4-B4-S3/S4 cert-state; PHASE4-B5-S3
/// governance).
///
/// Conway certs decode through the owner-complete closed grammar
/// (`decode_conway_certs`); each applies to B4-owned `CertState` via
/// `apply_conway_cert` **and**, when governance is tracked (`gov_state` present),
/// to `ConwayGovState` via `apply_conway_gov_cert` — replacing B4's
/// observe-and-drop with application. Shelley..Babbage use the Shelley decoder +
/// `apply_cert` (no governance). A decode error, a cert-state apply error, or a
/// governance apply error (e.g. a DRep expiry needed with `drep_activity` absent)
/// propagates as a structured `LedgerError` — never swallowed.
///
/// `gov_env` is consulted only by DRep register/update (tags 16/18); env-free gov
/// certs accumulate regardless. `gov_state == None` means governance is not
/// tracked for this replay (the gov half is skipped, paralleling track_utxo
/// gating for cert-state).
fn accumulate_tx_certs(
    era: CardanoEra,
    cert_bytes: &[u8],
    cert_state: &crate::delegation::CertState,
    gov_state: &Option<crate::state::ConwayGovState>,
    key_deposit: ade_types::tx::Coin,
    gov_env: Option<&crate::state::GovCertEnv>,
) -> Result<(crate::delegation::CertState, Option<crate::state::ConwayGovState>), LedgerError> {
    let mut state = cert_state.clone();
    let mut gov = gov_state.clone();
    if era == CardanoEra::Conway {
        let certs = ade_codec::conway::cert::decode_conway_certs(cert_bytes)?;
        for (idx, cert) in certs.iter().enumerate() {
            let env = crate::delegation::ConwayCertEnv {
                key_deposit,
                cert_index: idx as u16,
            };
            let outcome = crate::delegation::apply_conway_cert(&state, cert, &env)?;
            state = outcome.state;
            // PHASE4-B5: apply the governance half (vote-delegation / committee /
            // DRep) into ConwayGovState when governance is tracked. A gov apply
            // error propagates and halts the block transition.
            if let Some(g) = gov.as_ref() {
                gov = Some(crate::gov_cert::apply_conway_gov_cert(g, cert, gov_env)?);
            }
        }
    } else {
        let certs = ade_codec::shelley::cert::decode_certificates(cert_bytes)?;
        for (idx, cert) in certs.iter().enumerate() {
            state = crate::delegation::apply_cert(&state, cert, key_deposit, idx as u16)?;
        }
    }
    Ok((state, gov))
}

/// Locate per-output byte slices in an already-parsed Alonzo+ tx body.
///
/// Re-walks the body CBOR to find map key 1 (outputs) and returns the
/// start/end offsets of each output within `body_bytes`. Used to
/// preserve raw output CBOR in `TxOut::AlonzoPlus` — the structured
/// decoder already returned the outputs as parsed values, but aiken's
/// Plutus ScriptContext construction needs the byte-identical wire form.
fn locate_alonzo_plus_output_slices(
    body_bytes: &[u8],
) -> Result<Vec<(usize, usize)>, LedgerError> {
    let mut off = 0;
    let enc = cbor::read_map_header(body_bytes, &mut off)?;
    let map_len = match enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => {
            return Err(ade_codec::error::CodecError::InvalidCborStructure {
                offset: 0,
                detail: "Alonzo+ tx body must be definite-length map",
            }
            .into());
        }
    };

    let mut slices: Vec<(usize, usize)> = Vec::new();
    for _ in 0..map_len {
        let (key, _) = cbor::read_uint(body_bytes, &mut off)?;
        if key == 1 {
            // outputs array — slice each element.
            let arr_enc = cbor::read_array_header(body_bytes, &mut off)?;
            match arr_enc {
                cbor::ContainerEncoding::Definite(n, _) => {
                    for _ in 0..n {
                        let start = off;
                        let _ = cbor::skip_item(body_bytes, &mut off)?;
                        slices.push((start, off));
                    }
                }
                cbor::ContainerEncoding::Indefinite => {
                    while !cbor::is_break(body_bytes, off)? {
                        let start = off;
                        let _ = cbor::skip_item(body_bytes, &mut off)?;
                        slices.push((start, off));
                    }
                    off += 1; // consume break
                }
            }
            // Keep scanning — we've captured the outputs; skip other keys.
            continue;
        }
        let _ = cbor::skip_item(body_bytes, &mut off)?;
    }
    Ok(slices)
}

/// Extract inputs and outputs from a decoded tx body.
/// Apply a single Conway transaction to a UTxO state — the canonical UTxO
/// transition the block path uses (`track_utxo`'s inner closure), factored for
/// one tx so the standalone `tx_validity` path (PHASE4-B2-S2) and the block
/// body path produce byte-identical UTxO outputs. Consumes the body's inputs
/// and produces its outputs under `tx_id` (computed from preserved body bytes).
///
/// `body_bytes` MUST be the preserved body slice `tx_id` was hashed over, so
/// the produced `AlonzoPlus` outputs carry their byte-exact `raw` slices.
pub fn apply_conway_tx_to_utxo(
    utxo: &crate::utxo::UTxOState,
    body: &ade_types::conway::tx::ConwayTxBody,
    body_bytes: &[u8],
    tx_id: &ade_types::Hash32,
) -> Result<crate::utxo::UTxOState, LedgerError> {
    let mut new_utxo = utxo.clone();
    for input in &body.inputs {
        new_utxo.utxos.remove(input);
    }
    let slices = locate_alonzo_plus_output_slices(body_bytes)?;
    for (idx, (out, (s, e))) in body.outputs.iter().zip(slices).enumerate() {
        let tx_in = ade_types::tx::TxIn {
            tx_hash: tx_id.clone(),
            index: idx as u16,
        };
        new_utxo.utxos.insert(
            tx_in,
            crate::utxo::TxOut::AlonzoPlus {
                raw: body_bytes[s..e].to_vec(),
                address: out.address.clone(),
                coin: out.coin,
            },
        );
    }
    Ok(new_utxo)
}

pub(crate) fn extract_inputs_outputs_from_tx(
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
            let body_start = *offset;
            let tx = ade_codec::alonzo::tx::decode_alonzo_tx_body(data, offset)?;
            let body_end = *offset;
            let body_bytes = &data[body_start..body_end];
            let slices = locate_alonzo_plus_output_slices(body_bytes)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx
                .outputs
                .into_iter()
                .zip(slices.into_iter())
                .map(|(o, (s, e))| crate::utxo::TxOut::AlonzoPlus {
                    raw: body_bytes[s..e].to_vec(),
                    address: o.address,
                    coin: o.coin,
                })
                .collect();
            Ok((inputs, outputs))
        }
        CardanoEra::Babbage => {
            let body_start = *offset;
            let tx = ade_codec::babbage::tx::decode_babbage_tx_body(data, offset)?;
            let body_end = *offset;
            let body_bytes = &data[body_start..body_end];
            let slices = locate_alonzo_plus_output_slices(body_bytes)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx
                .outputs
                .into_iter()
                .zip(slices.into_iter())
                .map(|(o, (s, e))| crate::utxo::TxOut::AlonzoPlus {
                    raw: body_bytes[s..e].to_vec(),
                    address: o.address,
                    coin: o.coin,
                })
                .collect();
            Ok((inputs, outputs))
        }
        CardanoEra::Conway => {
            let body_start = *offset;
            let tx = ade_codec::conway::tx::decode_conway_tx_body(data, offset)?;
            let body_end = *offset;
            let body_bytes = &data[body_start..body_end];
            let slices = locate_alonzo_plus_output_slices(body_bytes)?;
            let inputs: Vec<_> = tx.inputs.into_iter().collect();
            let outputs = tx
                .outputs
                .into_iter()
                .zip(slices.into_iter())
                .map(|(o, (s, e))| crate::utxo::TxOut::AlonzoPlus {
                    raw: body_bytes[s..e].to_vec(),
                    address: o.address,
                    coin: o.coin,
                })
                .collect();
            Ok((inputs, outputs))
        }
        _ => {
            let _ = cbor::skip_item(data, offset)?;
            Ok((Vec::new(), Vec::new()))
        }
    }
}

/// Per-transaction outcome class. Maps each tx's combined Phase-1 +
/// Plutus verdict into a small sum type the diff-against-oracle harness
/// can compare against. The S-32 discharge doc promised this surface
/// (item 7): callers need per-tx pass/fail as values, not just block-
/// level aggregate counters.
#[derive(Debug, Clone, PartialEq)]
pub enum TxOutcome {
    /// Tx passed all state-backed Phase-1 checks and, if it carried
    /// Plutus scripts, every script ran to completion successfully.
    Passed,
    /// Tx's Phase-1 composer returned `BadInputs` — not all inputs
    /// resolve in the pre-block UTxO. We treat this as "not classifiable"
    /// rather than a pass/fail verdict, mirroring the silent-skip policy
    /// of the UTxO tracker itself.
    InputsUnresolved,
    /// Phase-1 state-backed check failed. The full LedgerError is
    /// preserved so the harness can diff against oracle error classes.
    Phase1Rejected {
        reason: crate::error::LedgerError,
    },
    /// Phase-1 passed; tx carries Plutus scripts; aiken returned a
    /// successful evaluation for every script.
    PlutusPassed {
        /// Aggregate cpu across all scripts in the tx.
        cpu: i64,
        /// Aggregate mem across all scripts in the tx.
        mem: i64,
        /// Number of scripts executed.
        script_count: usize,
    },
    /// Phase-1 passed; tx carries Plutus scripts; aiken returned an
    /// error for at least one script.
    PlutusFailed { reason: String },
    /// Phase-1 passed; tx carries Plutus scripts but at least one
    /// input / collateral / reference input didn't resolve in the
    /// pre-block UTxO. Distinct from `InputsUnresolved` because
    /// Phase-1 did resolve the spend-set but the Plutus evaluator
    /// needs additional context (ref inputs / collateral).
    PlutusIneligible,
    /// Tx was processed but didn't go through the Alonzo+ composer
    /// path (e.g., pre-Alonzo era or `track_utxo=false`). No verdict
    /// claim is made — the harness should treat this as "out of scope."
    Skipped,
}

/// Single-transaction verdict emitted by `apply_block_with_verdicts`.
#[derive(Debug, Clone, PartialEq)]
pub struct TxVerdict {
    /// 0-based tx index within the block.
    pub tx_index: usize,
    /// Classification of this tx's outcome.
    pub outcome: TxOutcome,
}

/// Full apply-block result with per-tx verdicts. Returned by the new
/// `apply_block_with_verdicts` entry point; existing callers of
/// `apply_block_classified` keep their `(LedgerState, BlockVerdict)`
/// tuple shape unchanged.
#[derive(Debug, Clone)]
pub struct BlockApplyResult {
    pub new_state: LedgerState,
    pub verdict: BlockVerdict,
    /// Per-tx outcomes in block order. Empty for blocks that don't
    /// go through the Alonzo+ composer path (pre-Alonzo or
    /// track_utxo=false), since per-tx classification only runs when
    /// the state-backed composer runs.
    pub tx_verdicts: Vec<TxVerdict>,
    /// Tx indices in the block's `invalid_transactions` field
    /// (Alonzo+). Empty for pre-Alonzo blocks (field doesn't exist)
    /// or blocks with no invalid txs. A tx at index `i` is a phase-2
    /// failure (is_valid=false) iff `i` is in this set.
    /// Exposed so oracle-diff harnesses can cross-reference our Plutus
    /// verdicts against the chain's ground truth.
    pub invalid_tx_indices: std::collections::BTreeSet<u64>,
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
    /// Plutus-bearing txs identified by witness-set inspection.
    /// Actual eval outcome lives in the `plutus_eval_*` counters
    /// below; this is retained as a classification signal (how many
    /// txs the Plutus dispatch had jurisdiction over).
    pub plutus_deferred_count: u64,
    /// Non-Plutus txs (native scripts evaluated, or no scripts).
    pub non_plutus_count: u64,
    /// Native scripts evaluated and passed.
    pub native_script_passed: u64,
    /// Native scripts evaluated and failed (structural — tx still accepted
    /// because witness-level script failure is a Phase 2 ledger rule, not
    /// a structural rejection at this level).
    pub native_script_failed: u64,
    /// Alonzo+ txs rejected by the state-backed Phase-1 composer.
    /// Only incremented when `track_utxo=true` and all inputs resolve; txs
    /// whose inputs predate the replay window are silently skipped (same
    /// policy as the UTxO tracker). 0 for pre-Alonzo eras.
    pub state_backed_phase1_rejected: u64,
    /// Plutus txs that `ade_plutus::eval_tx_phase_two` ran to completion.
    /// Zero on pre-Alonzo / when `track_utxo=false` / when no inputs
    /// resolve (the tx lands on `PlutusEvalOutcome::Ineligible`).
    pub plutus_eval_passed: u64,
    /// Plutus txs aiken returned a failure for (decode / budget / script
    /// failure / context build). Surfaces CE-89's "every Plutus verdict
    /// reaches the evaluator" contract — anything here is reported to
    /// downstream consumers instead of being deferred.
    pub plutus_eval_failed: u64,
    /// Plutus-carrying txs that couldn't be evaluated because at least
    /// one input / collateral / reference input didn't resolve in the
    /// pre-block UTxO. Diagnostic surface: a positive count here means
    /// the pipeline CAN see Plutus txs but the UTxO window doesn't hold
    /// their predecessors.
    pub plutus_eval_ineligible: u64,
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
            native_script_passed: 0,
            native_script_failed: 0,
            state_backed_phase1_rejected: 0,
            plutus_eval_passed: 0,
            plutus_eval_failed: 0,
            plutus_eval_ineligible: 0,
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
    let mut native_script_passed = 0u64;
    let mut native_script_failed = 0u64;
    let mut tx_idx = 0usize;

    let mut process_one = |body_data: &[u8], body_offset: &mut usize| -> Result<(), LedgerError> {
        // Decode and structurally validate the tx body
        let body_posture = decode_and_validate_single_tx(body_data, body_offset, era)?;

        // Get witness info for this tx (if available)
        let witness_info = witness_infos.get(tx_idx);

        // Determine authoritative script verdict using witness confirmation (CE-77)
        let has_plutus_in_witnesses = witness_info
            .map(|w| w.has_plutus())
            .unwrap_or(false);

        // Plutus-bearing tx classification — actual eval outcome is
        // counted separately in `plutus_eval_{passed,failed,ineligible}`
        // by run_phase_one_composers when track_utxo=true.
        let is_deferred = has_plutus_in_witnesses
            || body_posture == crate::scripts::ScriptPosture::PlutusPresentDeferred;

        if is_deferred {
            plutus_deferred_count += 1;
        } else {
            // Evaluate native scripts if present
            if let Some(w) = witness_info {
                for script in &w.native_scripts {
                    let verdict = crate::scripts::evaluate_native_script(
                        script,
                        &w.available_key_hashes,
                        current_slot,
                    );
                    match verdict {
                        crate::scripts::ScriptVerdict::NativeScriptPassed => {
                            native_script_passed += 1;
                        }
                        crate::scripts::ScriptVerdict::NativeScriptFailed(_) => {
                            native_script_failed += 1;
                        }
                        crate::scripts::ScriptVerdict::PlutusPassed { .. }
                        | crate::scripts::ScriptVerdict::PlutusFailed { .. } => {
                            // Plutus verdicts do not arise from
                            // evaluate_native_script (native scripts
                            // never produce Plutus verdicts). The
                            // match is exhaustive for future-proofing.
                        }
                    }
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
        native_script_passed,
        native_script_failed,
        state_backed_phase1_rejected: 0,
        plutus_eval_passed: 0,
        plutus_eval_failed: 0,
        plutus_eval_ineligible: 0,
    })
}

/// Same as `run_phase_one_composers` but returns per-rejection error
/// variant names (one entry per non-BadInputs failure). Used by
/// diagnostic tests to characterize what the composer is catching.
pub fn run_phase_one_composers_diagnostic(
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
    state: &LedgerState,
) -> Result<Vec<String>, LedgerError> {
    if block.tx_count == 0 {
        return Ok(Vec::new());
    }
    let witness_infos = crate::witness::decode_witness_infos(&block.witness_sets)?;

    let mut body_offset = 0;
    let body_data = &block.tx_bodies;
    let body_enc = cbor::read_array_header(body_data, &mut body_offset)?;

    let pp = &state.protocol_params;
    let collateral_percent = pp.collateral_percent;
    let current_network = pp.network_id;
    let max_ex_units: (i64, i64) =
        (pp.max_tx_ex_units_mem as i64, pp.max_tx_ex_units_cpu as i64);
    let utxo = &state.utxo_state.utxos;

    let mut rejections = Vec::new();
    let mut tx_idx = 0usize;
    let empty_wi = crate::witness::WitnessInfo {
        available_key_hashes: std::collections::BTreeSet::new(),
        native_scripts: Vec::new(),
        has_plutus_v1: false,
        has_plutus_v2: false,
        has_plutus_v3: false,
        total_ex_units: crate::witness::TotalExUnits { mem: 0, cpu: 0 },
    };

    let mut process_one = |data: &[u8], offset: &mut usize| -> Result<(), LedgerError> {
        let wi = witness_infos.get(tx_idx).unwrap_or(&empty_wi);
        let result = match era {
            CardanoEra::Alonzo => {
                let body = ade_codec::alonzo::tx::decode_alonzo_tx_body(data, offset)?;
                crate::alonzo::validate_alonzo_state_backed(
                    &body, utxo, wi, collateral_percent, current_network, max_ex_units,
                )
            }
            CardanoEra::Babbage => {
                let body = ade_codec::babbage::tx::decode_babbage_tx_body(data, offset)?;
                crate::babbage::validate_babbage_state_backed(
                    &body, utxo, wi, collateral_percent, current_network, max_ex_units,
                )
            }
            CardanoEra::Conway => {
                let body = ade_codec::conway::tx::decode_conway_tx_body(data, offset)?;
                let deposit_params = state
                    .conway_deposit_view()
                    .map_err(LedgerError::ValidationEnvironment)?;
                crate::conway::validate_conway_state_backed(
                    &body, utxo, wi, collateral_percent, current_network,
                    pp.protocol_major as u16, max_ex_units, &deposit_params,
                    state.cert_state.require_full(state.epoch_state.slot)?,
                )
            }
            _ => Ok(()),
        };
        match result {
            Ok(()) | Err(crate::error::LedgerError::BadInputs(_)) => {}
            Err(e) => {
                rejections.push(format!("tx#{tx_idx}: {e:?}"));
            }
        }
        tx_idx += 1;
        Ok(())
    };

    match body_enc {
        cbor::ContainerEncoding::Definite(n, _) => {
            for _ in 0..n { process_one(body_data, &mut body_offset)?; }
        }
        cbor::ContainerEncoding::Indefinite => {
            while !cbor::is_break(body_data, body_offset)? {
                process_one(body_data, &mut body_offset)?;
            }
        }
    }

    Ok(rejections)
}

/// Counts returned by the composer + Plutus-eval integrated pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ComposerStats {
    pub rejected: u64,
    pub plutus_eval_passed: u64,
    pub plutus_eval_failed: u64,
    pub plutus_eval_ineligible: u64,
}

/// Walk the block's tx bodies + witness sets in parallel, invoking the
/// per-era state-backed composer against the pre-block UTxO. Runs
/// `ade_plutus::eval_tx_phase_two` inline for any Plutus tx whose Phase-1
/// checks passed and whose inputs fully resolve in the UTxO snapshot.
///
/// A tx whose composer returns `BadInputs` is silently skipped — inputs
/// may predate the replay window, mirroring the UTxO-tracker policy.
/// Any other error increments `rejected`.
///
/// Plutus txs land on `PlutusEvalOutcome::Ineligible` (silent — not
/// counted as pass or fail) when any input doesn't resolve. Successful
/// aiken runs increment `plutus_eval_passed`; aiken errors bump
/// `plutus_eval_failed`.
///
/// Assumes era is Alonzo/Babbage/Conway and `state.track_utxo == true`.
fn run_phase_one_composers(
    block: &ade_types::shelley::block::ShelleyBlock,
    era: CardanoEra,
    state: &LedgerState,
) -> Result<(ComposerStats, Vec<TxVerdict>), LedgerError> {
    if block.tx_count == 0 {
        return Ok((ComposerStats::default(), Vec::new()));
    }

    let witness_infos = crate::witness::decode_witness_infos(&block.witness_sets)?;

    let mut body_offset = 0;
    let body_data = &block.tx_bodies;
    let body_enc = cbor::read_array_header(body_data, &mut body_offset)?;

    // Parallel-walk witness sets to capture each tx's raw witness-set slice.
    let mut witness_offset = 0;
    let witness_data = &block.witness_sets;
    let witness_enc = cbor::read_array_header(witness_data, &mut witness_offset)?;
    let witness_count = match witness_enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => u64::MAX,
    };

    let pp = &state.protocol_params;
    let collateral_percent = pp.collateral_percent;
    let current_network = pp.network_id;
    let max_ex_units: (i64, i64) =
        (pp.max_tx_ex_units_mem as i64, pp.max_tx_ex_units_cpu as i64);
    let utxo = &state.utxo_state.utxos;

    // Conway requires its canonical deposit params present; assemble the view
    // once and fail fast for the whole block if the environment is missing it.
    // Pre-Conway eras never read this and carry `None`.
    let conway_deposit_params = if era == CardanoEra::Conway {
        Some(
            state
                .conway_deposit_view()
                .map_err(LedgerError::ValidationEnvironment)?,
        )
    } else {
        None
    };

    let mut stats = ComposerStats::default();
    let mut tx_verdicts: Vec<TxVerdict> = Vec::new();
    let mut tx_idx = 0usize;
    let empty_wi = crate::witness::WitnessInfo {
        available_key_hashes: std::collections::BTreeSet::new(),
        native_scripts: Vec::new(),
        has_plutus_v1: false,
        has_plutus_v2: false,
        has_plutus_v3: false,
        total_ex_units: crate::witness::TotalExUnits { mem: 0, cpu: 0 },
    };

    // Budget per tx for aiken. We reuse the pparams tx-level cap as the
    // initial budget — phase-1 has already verified the tx stays within it,
    // so this is the right upper bound for aiken too.
    let budget = (pp.max_tx_ex_units_cpu, pp.max_tx_ex_units_mem);

    let tx_count = match body_enc {
        cbor::ContainerEncoding::Definite(n, _) => n,
        cbor::ContainerEncoding::Indefinite => u64::MAX,
    };
    let mut witness_remaining = witness_count;

    loop {
        // Termination: definite → we've consumed tx_count entries;
        // indefinite → break byte in body.
        if matches!(body_enc, cbor::ContainerEncoding::Definite(_, _))
            && tx_idx as u64 >= tx_count
        {
            break;
        }
        if matches!(body_enc, cbor::ContainerEncoding::Indefinite)
            && cbor::is_break(body_data, body_offset)?
        {
            break;
        }

        let wi = witness_infos.get(tx_idx).unwrap_or(&empty_wi);

        // Capture body slice.
        let body_start = body_offset;

        // Run the phase-1 composer by decoding the body. This advances
        // body_offset to the end of this tx's body.
        let (phase_one_result, body_tx_meta) = decode_and_phase_one(
            era,
            body_data,
            &mut body_offset,
            utxo,
            wi,
            collateral_percent,
            current_network,
            max_ex_units,
            pp.protocol_major as u16,
            conway_deposit_params.as_ref(),
            state.cert_state.require_full(state.epoch_state.slot)?,
        )?;
        let body_end = body_offset;

        // Advance witness cursor in parallel. Capture witness slice.
        let witness_start = witness_offset;
        if witness_remaining > 0 {
            let _ = cbor::skip_item(witness_data, &mut witness_offset)?;
            witness_remaining = witness_remaining.saturating_sub(1);
        }
        let witness_end = witness_offset;

        match phase_one_result {
            Ok(()) => {
                // Phase-1 passed. Try Plutus eval if the tx carries any
                // Plutus script.
                if wi.has_plutus() {
                    let outcome = crate::plutus_eval::try_evaluate_tx(
                        &body_data[body_start..body_end],
                        &witness_data[witness_start..witness_end],
                        &body_tx_meta.inputs,
                        body_tx_meta.collateral_inputs.as_ref(),
                        body_tx_meta.reference_inputs.as_ref(),
                        utxo,
                        era,
                        budget,
                        pp.cost_models_cbor.as_deref(),
                    );
                    let verdict_outcome = match outcome {
                        crate::plutus_eval::PlutusEvalOutcome::Ineligible => {
                            stats.plutus_eval_ineligible =
                                stats.plutus_eval_ineligible.saturating_add(1);
                            TxOutcome::PlutusIneligible
                        }
                        crate::plutus_eval::PlutusEvalOutcome::Passed {
                            total_cpu,
                            total_mem,
                            script_count,
                        } => {
                            stats.plutus_eval_passed =
                                stats.plutus_eval_passed.saturating_add(1);
                            TxOutcome::PlutusPassed {
                                cpu: total_cpu,
                                mem: total_mem,
                                script_count,
                            }
                        }
                        crate::plutus_eval::PlutusEvalOutcome::Failed { reason } => {
                            stats.plutus_eval_failed =
                                stats.plutus_eval_failed.saturating_add(1);
                            TxOutcome::PlutusFailed { reason }
                        }
                    };
                    tx_verdicts.push(TxVerdict { tx_index: tx_idx, outcome: verdict_outcome });
                } else {
                    // Phase-1 passed, no Plutus scripts.
                    tx_verdicts.push(TxVerdict {
                        tx_index: tx_idx,
                        outcome: TxOutcome::Passed,
                    });
                }
            }
            Err(crate::error::LedgerError::BadInputs(_)) => {
                // Silent skip for Phase-1 accounting (replay-window policy).
                // For diagnostic accounting: if this was a Plutus tx, its
                // unresolved inputs are also the reason we can't eval, so
                // count it as plutus_eval_ineligible. This distinguishes
                // "Plutus tx we never saw" from "Plutus tx we couldn't
                // feed to aiken."
                if wi.has_plutus() {
                    stats.plutus_eval_ineligible =
                        stats.plutus_eval_ineligible.saturating_add(1);
                    tx_verdicts.push(TxVerdict {
                        tx_index: tx_idx,
                        outcome: TxOutcome::PlutusIneligible,
                    });
                } else {
                    tx_verdicts.push(TxVerdict {
                        tx_index: tx_idx,
                        outcome: TxOutcome::InputsUnresolved,
                    });
                }
            }
            Err(e) => {
                stats.rejected = stats.rejected.saturating_add(1);
                tx_verdicts.push(TxVerdict {
                    tx_index: tx_idx,
                    outcome: TxOutcome::Phase1Rejected { reason: e },
                });
            }
        }

        tx_idx += 1;
    }

    Ok((stats, tx_verdicts))
}

/// Phase-1 call per era, returning both the result and the minimal tx
/// metadata the Plutus-eval path needs (input sets).
struct TxInputSets {
    inputs: std::collections::BTreeSet<ade_types::tx::TxIn>,
    collateral_inputs: Option<std::collections::BTreeSet<ade_types::tx::TxIn>>,
    reference_inputs: Option<std::collections::BTreeSet<ade_types::tx::TxIn>>,
}

#[allow(clippy::too_many_arguments)]
fn decode_and_phase_one(
    era: CardanoEra,
    data: &[u8],
    offset: &mut usize,
    utxo: &impl crate::utxo::UtxoStore,
    wi: &crate::witness::WitnessInfo,
    collateral_percent: u16,
    current_network: u8,
    max_ex_units: (i64, i64),
    protocol_major: u16,
    conway_deposit_params: Option<&crate::pparams::ConwayDepositParams>,
    cert_state: &crate::delegation::CertState,
) -> Result<(Result<(), LedgerError>, TxInputSets), LedgerError> {
    match era {
        CardanoEra::Alonzo => {
            let body = ade_codec::alonzo::tx::decode_alonzo_tx_body(data, offset)?;
            let r = crate::alonzo::validate_alonzo_state_backed(
                &body, utxo, wi, collateral_percent, current_network, max_ex_units,
            );
            let meta = TxInputSets {
                inputs: body.inputs.clone(),
                collateral_inputs: body.collateral_inputs.clone(),
                reference_inputs: None,
            };
            Ok((r, meta))
        }
        CardanoEra::Babbage => {
            let body = ade_codec::babbage::tx::decode_babbage_tx_body(data, offset)?;
            let r = crate::babbage::validate_babbage_state_backed(
                &body, utxo, wi, collateral_percent, current_network, max_ex_units,
            );
            let meta = TxInputSets {
                inputs: body.inputs.clone(),
                collateral_inputs: body.collateral_inputs.clone(),
                reference_inputs: body.reference_inputs.clone(),
            };
            Ok((r, meta))
        }
        CardanoEra::Conway => {
            let body = ade_codec::conway::tx::decode_conway_tx_body(data, offset)?;
            let deposit_params = conway_deposit_params.ok_or(
                LedgerError::ValidationEnvironment(
                    crate::error::ValidationEnvironmentError::MissingConwayDepositParams,
                ),
            )?;
            let r = crate::conway::validate_conway_state_backed(
                &body, utxo, wi, collateral_percent, current_network,
                protocol_major, max_ex_units, deposit_params, cert_state,
            );
            let meta = TxInputSets {
                inputs: body.inputs.clone(),
                collateral_inputs: body.collateral_inputs.clone(),
                reference_inputs: body.reference_inputs.clone(),
            };
            Ok((r, meta))
        }
        _ => {
            // Shouldn't be called for other eras; skip item and return Ok.
            let _ = cbor::skip_item(data, offset)?;
            Ok((Ok(()), TxInputSets {
                inputs: std::collections::BTreeSet::new(),
                collateral_inputs: None,
                reference_inputs: None,
            }))
        }
    }
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

    #[test]
    fn bad_inputs_are_silently_skipped() {
        // Build a minimal Alonzo block with a tx whose inputs are not in UTxO
        // but track_utxo=true. Composer returns BadInputs; wiring must skip it
        // without incrementing state_backed_phase1_rejected (replay-window policy).
        use std::collections::BTreeSet;
        let mut body = ade_types::alonzo::tx::AlonzoTxBody {
            inputs: BTreeSet::new(),
            outputs: Vec::new(),
            fee: ade_types::tx::Coin(0),
            ttl: None,
            certs: None,
            withdrawals: None,
            update: None,
            metadata_hash: None,
            validity_interval_start: None,
            mint: None,
            script_data_hash: None,
            collateral_inputs: None,
            required_signers: None,
            network_id: None,
        };
        // Insert one input missing from the (empty) UTxO.
        body.inputs.insert(ade_types::tx::TxIn {
            tx_hash: ade_types::Hash32([0x11; 32]),
            index: 0,
        });

        let utxo = std::collections::BTreeMap::new();
        let wi = crate::witness::WitnessInfo {
            available_key_hashes: BTreeSet::new(),
            native_scripts: Vec::new(),
            has_plutus_v1: false,
            has_plutus_v2: false,
            has_plutus_v3: false,
            total_ex_units: crate::witness::TotalExUnits { mem: 0, cpu: 0 },
        };
        let res = crate::alonzo::validate_alonzo_state_backed(
            &body, &utxo, &wi, 150, 1, (14_000_000, 10_000_000_000),
        );
        assert!(
            matches!(res, Err(crate::error::LedgerError::BadInputs(_))),
            "composer must return BadInputs when input predates UTxO",
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod cert_state_dispatch {
    //! PHASE4-B4-S3/S4 (CE-B4-3, CE-B4-4): era-dispatched, fail-closed
    //! cert-state accumulation. Conway certs route through the owner-complete
    //! `decode_conway_certs` + `apply_conway_cert`; decode and apply errors
    //! propagate; governance effects are owner-tagged out of B4 scope.
    use super::accumulate_tx_certs;
    use crate::delegation::CertState;
    use crate::error::{LedgerError, ValidationEnvironmentError};
    use crate::state::{ConwayGovState, GovCertEnv};
    use ade_types::tx::Coin;
    use ade_types::CardanoEra;

    fn cbor_uint(buf: &mut Vec<u8>, major: u8, v: u64) {
        let m = major << 5;
        if v < 24 {
            buf.push(m | v as u8);
        } else if v < 0x100 {
            buf.push(m | 24);
            buf.push(v as u8);
        } else {
            buf.push(m | 25);
            buf.extend_from_slice(&(v as u16).to_be_bytes());
        }
    }
    fn arr(b: &mut Vec<u8>, n: u64) {
        cbor_uint(b, 4, n);
    }
    fn uint(b: &mut Vec<u8>, v: u64) {
        cbor_uint(b, 0, v);
    }
    fn cred(b: &mut Vec<u8>, marker: u8) {
        arr(b, 2);
        uint(b, 0);
        cbor_uint(b, 2, 28);
        b.extend_from_slice(&[marker; 28]);
    }
    fn h28(b: &mut Vec<u8>, marker: u8) {
        cbor_uint(b, 2, 28);
        b.extend_from_slice(&[marker; 28]);
    }
    fn cert_array(cert: Vec<u8>) -> Vec<u8> {
        let mut b = Vec::new();
        arr(&mut b, 1);
        b.extend_from_slice(&cert);
        b
    }
    fn reg_cert(marker: u8) -> Vec<u8> {
        let mut b = Vec::new();
        arr(&mut b, 2);
        uint(&mut b, 0);
        cred(&mut b, marker);
        b
    }
    fn deleg_cert(cred_m: u8, pool_m: u8) -> Vec<u8> {
        let mut b = Vec::new();
        arr(&mut b, 3);
        uint(&mut b, 2);
        cred(&mut b, cred_m);
        h28(&mut b, pool_m);
        b
    }
    fn drep_reg_cert(marker: u8) -> Vec<u8> {
        let mut b = Vec::new();
        arr(&mut b, 4);
        uint(&mut b, 16);
        cred(&mut b, marker);
        uint(&mut b, 500_000_000);
        b.push(0xf6); // anchor = null
        b
    }
    fn tag_only_cert(tag: u64) -> Vec<u8> {
        let mut b = Vec::new();
        arr(&mut b, 1);
        uint(&mut b, tag);
        b
    }
    const KD: Coin = Coin(2_000_000);

    /// REDUCED-VALIDATION-BOUNDARY-PLANE (gate 1 / N-RVB-1 + deviation 2): a reduced follower (`track_utxo=false`)
    /// crossing a Conway boundary advances epoch + rolls the block-production window but produces NO stake
    /// snapshot at all (`ReducedUnavailable`) AND carries no advanced certificate/pool or governance lifecycle —
    /// never a fabricated mark, never a full `CertState` that could be mistaken for post-POOLREAP state.
    #[test]
    fn reduced_epoch_boundary_produces_no_mark_or_cert_lifecycle() {
        use super::apply_reduced_epoch_boundary;
        use crate::state::LedgerState;
        use ade_types::tx::PoolId;
        use ade_types::{EpochNo, Hash28};

        let mut state = LedgerState::new(CardanoEra::Conway); // track_utxo=false by default (the reduced plane)
        state.epoch_state.epoch = EpochNo(500);
        state
            .epoch_state
            .block_production
            .insert(PoolId(Hash28([0x11; 28])), 42);
        state.epoch_state.epoch_fees = Coin(9_999);
        // Give it cert + gov content to prove the reduced boundary does NOT carry them across (deviation 2).
        state
            .cert_state
            .as_authoritative_mut()
            .expect("authoritative cert state in test")
            .delegation
            .registrations
            .insert(ade_types::shelley::cert::StakeCredential::KeyHash(Hash28([0x22; 28])), Coin(2_000_000));

        let reduced = apply_reduced_epoch_boundary(&state, EpochNo(501));
        assert_eq!(reduced.epoch_state.epoch, EpochNo(501), "epoch advances to the new epoch");
        assert!(
            reduced.epoch_state.snapshots.is_reduced(),
            "the reduced plane produces NO mark/set/go — ReducedUnavailable (gate 1 / N-RVB-1)",
        );
        assert!(
            reduced.epoch_state.snapshots.as_authoritative().is_none(),
            "stake authority is unavailable on the reduced plane (fails closed, never a fabricated snapshot)",
        );
        assert!(
            reduced.epoch_state.block_production.is_empty(),
            "the block-production window rolled over into a fresh (empty) new epoch",
        );
        assert_eq!(reduced.epoch_state.epoch_fees, Coin(0), "new-epoch fees reset");
        // Deviation 2: cert/gov are unavailable BY TYPE across the reduced boundary — not an empty full
        // CertState/gov, but `ReducedUnavailable` (a reduced follower ratified/enacted no cert or governance).
        assert!(
            reduced.cert_state.is_reduced(),
            "no full CertState carried across the reduced boundary (CertStateProjection::ReducedUnavailable)",
        );
        assert!(
            reduced.gov_state.is_reduced(),
            "no governance lifecycle carried across the reduced boundary (GovStateProjection::ReducedUnavailable)",
        );
    }

    #[test]
    fn era_dispatch_conway_accumulates_via_conway_path() {
        let bytes = cert_array(reg_cert(1));
        let (out, _gov) = accumulate_tx_certs(CardanoEra::Conway, &bytes, &CertState::new(), &None, KD, None).unwrap();
        assert_eq!(out.delegation.registrations.len(), 1, "Conway reg accumulated");
    }

    #[test]
    fn era_dispatch_shelley_accumulates_via_shelley_path() {
        let bytes = cert_array(reg_cert(1));
        let (out, _gov) = accumulate_tx_certs(CardanoEra::Shelley, &bytes, &CertState::new(), &None, KD, None).unwrap();
        assert_eq!(out.delegation.registrations.len(), 1, "Shelley reg accumulated");
    }

    #[test]
    fn conway_decode_error_is_fail_closed() {
        // Cert array header claims 1 element but no cert follows → decode error.
        let mut bytes = Vec::new();
        arr(&mut bytes, 1);
        let res = accumulate_tx_certs(CardanoEra::Conway, &bytes, &CertState::new(), &None, KD, None);
        assert!(res.is_err(), "truncated cert array must fail closed, not swallow");
    }

    #[test]
    fn conway_unknown_tag_is_fail_closed() {
        let bytes = cert_array(tag_only_cert(19));
        let res = accumulate_tx_certs(CardanoEra::Conway, &bytes, &CertState::new(), &None, KD, None);
        assert!(res.is_err(), "unknown cert tag must fail closed");
    }

    #[test]
    fn conway_removed_tag_is_fail_closed() {
        let bytes = cert_array(tag_only_cert(5));
        let res = accumulate_tx_certs(CardanoEra::Conway, &bytes, &CertState::new(), &None, KD, None);
        assert!(
            matches!(res, Err(LedgerError::EraInvalidCertificate(_))),
            "removed tag 5 must reject as era-invalid, not swallow",
        );
    }

    #[test]
    fn conway_apply_error_is_fail_closed() {
        // Delegation for an unregistered credential → apply error must PROPAGATE
        // (this is the swallow that B4 removes — CE-B4-4).
        let bytes = cert_array(deleg_cert(1, 2));
        let res = accumulate_tx_certs(CardanoEra::Conway, &bytes, &CertState::new(), &None, KD, None);
        assert!(res.is_err(), "apply error must propagate, not be swallowed as non-fatal");
    }

    #[test]
    fn conway_governance_cert_routed_out_of_scope() {
        // A DRep registration mutates ConwayGovState, never the B4-owned
        // delegation/pool CertState. With gov_state = None (governance not
        // tracked here) the gov half is skipped; CertState stays unchanged
        // either way — owner exclusivity (DC-LEDGER-08 strengthened by
        // DC-LEDGER-09).
        let bytes = cert_array(drep_reg_cert(1));
        let before = CertState::new();
        let (out, _gov) = accumulate_tx_certs(CardanoEra::Conway, &bytes, &before, &None, KD, None).unwrap();
        assert_eq!(out, before, "governance cert leaves B4-owned cert-state unchanged");
    }

    fn empty_gov() -> ConwayGovState {
        ConwayGovState {
            prev_pparam_action: crate::state::PreviousPParamAction::Unversioned,
            proposals: Vec::new(),
            committee: std::collections::BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: std::collections::BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: std::collections::BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: std::collections::BTreeMap::new(),
            num_dormant: crate::state::DormantEpochs::Unversioned,
        }
    }

    /// PHASE4-B5: the block-path accumulator now APPLIES the governance half — a
    /// DRep registration lands in gov_state.drep_expiry (was observed-and-dropped
    /// in B4). Proves the wiring accumulates, not just compiles.
    #[test]
    fn gov_accumulation_applies_drep_registration_into_gov_state() {
        let bytes = cert_array(drep_reg_cert(1));
        let gov = Some(empty_gov());
        let env = GovCertEnv { current_epoch: 576, drep_activity: 20 };
        let (_cs, gov_out) =
            accumulate_tx_certs(CardanoEra::Conway, &bytes, &CertState::new(), &gov, KD, Some(&env))
                .unwrap();
        let g = gov_out.expect("governance tracked");
        let cred = ade_types::shelley::cert::StakeCredential::KeyHash(ade_types::Hash28([1u8; 28]));
        assert_eq!(
            g.drep_expiry.get(&cred),
            Some(&(576 + 20)),
            "DRep expiry accumulated into gov_state from the block path",
        );
    }

    /// CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION S1: the epoch boundary BUILDS the new MARK from the point-bound
    /// base UTxO ([`BoundaryBaseStake`]) + the STAGED POST-RUPD reward accounts — `stake = base + reward` per
    /// credential, retaining per-credential `delegations`. And the NO-FALLBACK gate: a CONWAY boundary WITHOUT
    /// the base input is a structured `BoundaryBaseStakeRequired` terminal (never a reward-only mark).
    #[test]
    fn epoch_boundary_builds_mark_from_base_plus_postrupd_reward_and_requires_base_for_conway() {
        use super::apply_epoch_boundary_with_registrations;
        use crate::state::LedgerState;
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::tx::PoolId;
        use ade_types::{EpochNo, Hash28};
        use std::collections::BTreeMap;

        let mut state = LedgerState::new(CardanoEra::Conway);
        state.epoch_state.epoch = EpochNo(500);
        // ANY Conway boundary reaching this fn REQUIRES the base — a `None` is the structured terminal (the
        // reduced track_utxo=false path never reaches here; `dispatch_epoch_boundary` routes it to the reduced
        // boundary, so the reward-only stub is unrepresentable for Conway). track_utxo=true is the full path.
        state.track_utxo = true;

        // A delegated credential with a POST-RUPD reward balance (no blocks → native RUPD is zero, so the held
        // 700 is the post-RUPD balance); its base UTxO (300) is supplied via BoundaryBaseStake.
        let pool_a = PoolId(Hash28([0x11; 28]));
        let cred_a = StakeCredential::KeyHash(Hash28([0xA1; 28]));
        state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").delegation.delegations.insert(cred_a.clone(), pool_a.clone());
        state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").delegation.rewards.insert(cred_a.clone(), Coin(700));
        let mut base = BTreeMap::new();
        base.insert(cred_a.clone(), Coin(300));
        let base_stake = crate::epoch_accumulator::BoundaryBaseStake {
            boundary_point: ade_types::SlotNo(0),
            canonical_credential_stake: base,
        };

        // Some(base) -> the mark is BUILT: stake = base (300) + post-RUPD reward (700) = 1000, delegations kept.
        let (built, _) =
            apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&base_stake), 21_600)
                .unwrap();
        assert_eq!(
            built.epoch_state.snapshots.as_authoritative().unwrap().mark.0.delegations.get(&Hash28([0xA1; 28])).map(|(_, c)| c.0),
            Some(1000),
            "the mark stake is base (300) + post-RUPD reward (700)"
        );
        assert_eq!(
            built.epoch_state.snapshots.as_authoritative().unwrap().mark.0.pool_stakes.get(&pool_a).map(|c| c.0),
            Some(1000),
            "pool_stakes aggregates the per-credential base+reward"
        );

        // None on a CONWAY boundary -> structured terminal, NEVER a reward-only mark (the no-fallback gate).
        let terminal = apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, None, 21_600);
        assert!(
            matches!(
                terminal,
                Err(crate::governance::GovernanceTerminal::BoundaryBaseStakeRequired { .. })
            ),
            "a Conway boundary without BoundaryBaseStake fails structurally, never a reward-only mark",
        );
    }

    /// LIVE-LEDGER-EPOCH-TRANSITION (CE-3d): the monetary-expansion performance factor `eta` uses the
    /// NETWORK's epoch length via `active_slots_per_epoch`, NOT a hardcoded mainnet constant. Preview's
    /// epoch is 86_400 slots (active_slots = 4_320) vs mainnet's 432_000 (21_600); for the SAME
    /// under-target block count the preview boundary must draw ~5× the reserves into the reward pot
    /// (eta is 5× larger), where the old hardcode made the two identical. Guards the CE-3d reward-
    /// magnitude residual (the preview boundary previously under-expanded 5×).
    #[test]
    fn monetary_expansion_tracks_network_epoch_length() {
        use super::apply_epoch_boundary_with_registrations;
        use crate::rational::Rational;
        use crate::state::LedgerState;
        use ade_types::tx::PoolId;
        use ade_types::{EpochNo, Hash28};

        let mut state = LedgerState::new(CardanoEra::Conway);
        state.epoch_state.epoch = EpochNo(500);
        state.epoch_state.reserves = Coin(1_000_000_000_000_000); // 1e15
        state.epoch_state.treasury = Coin(0);
        // Fully decentralized (d = 0) so eta = blocksMade / expectedBlocks (not the d >= 0.8 -> 1 cap),
        // with a block count well below either expected-blocks target so eta < 1 on BOTH networks.
        state.protocol_params.decentralization = Rational::zero();
        state.protocol_params.monetary_expansion = Rational::new(3, 1000).unwrap();
        state.protocol_params.treasury_growth = Rational::new(1, 5).unwrap();
        state
            .epoch_state
            .block_production
            .insert(PoolId(Hash28([0x11; 28])), 100);

        // The go snapshot is empty -> no member rewards; the pool pot returns to reserves, so the
        // treasury increase is exactly floor(deltaR1 * tau) -- a clean readout of the eta-scaled pot. The
        // empty cert-state has no delegations, so an EMPTY base yields an empty mark (this test reads pots, not
        // the mark) — Conway requires the base input, so pass an empty one rather than `None` (which terminals).
        let empty_base = crate::epoch_accumulator::BoundaryBaseStake {
            boundary_point: ade_types::SlotNo(0),
            canonical_credential_stake: std::collections::BTreeMap::new(),
        };
        let preview =
            apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&empty_base), 4_320)
                .unwrap()
                .0;
        let mainnet =
            apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&empty_base), 21_600)
                .unwrap()
                .0;

        let t_preview = preview.epoch_state.treasury.0;
        let t_mainnet = mainnet.epoch_state.treasury.0;
        assert!(t_preview > 0 && t_mainnet > 0, "both expand (eta < 1, not zero)");
        assert!(
            t_preview > t_mainnet,
            "preview's shorter epoch expands MORE for the same blocks (the bug made them equal): \
             preview={t_preview} mainnet={t_mainnet}"
        );
        // eta_preview / eta_mainnet = 21_600 / 4_320 = 5 -> the pot (and treasury cut) is ~5× larger.
        let ratio_x100 = t_preview.saturating_mul(100) / t_mainnet;
        assert!(
            (495..=505).contains(&ratio_x100),
            "expansion ratio must be ~5x (21600/4320): got {}.{:02}x (preview={t_preview} mainnet={t_mainnet})",
            ratio_x100 / 100,
            ratio_x100 % 100,
        );
    }

    /// CRE S4.3a: a ratifiable NoConfidence (empty DRep/SPO stake ⇒ those gates skip; committee gate skipped
    /// for NoConfidence ⇒ thresholds pass) is TERMINAL at the boundary on both paths — S4.3a performs no
    /// enactment, so the boundary must NOT silently dissolve the committee (the pre-S4.3 partial-enactment path).
    /// Atomic committee dissolution for the supported subset lands in S4.3c; `apply_committee_enactment` stays
    /// unit-tested in `governance.rs`.
    #[test]
    fn epoch_boundary_ratifiable_noconfidence_is_terminal_pending_enactment() {
        use crate::state::LedgerState;
        use super::apply_epoch_boundary_full;
        use ade_types::conway::governance::{GovAction, GovActionId, GovActionState};
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::{EpochNo, Hash28, Hash32};

        let mut state = LedgerState::new(CardanoEra::Conway);
        state.epoch_state.epoch = EpochNo(500);
        let mut gov = empty_gov();
        gov.committee = [
            (StakeCredential::KeyHash(Hash28([0xA0; 28])), 600u64),
            (StakeCredential::ScriptHash(Hash28([0xA1; 28])), 600u64),
        ]
        .into_iter()
        .collect();
        gov.proposals = vec![GovActionState {
            action_id: GovActionId { tx_hash: Hash32([0x01; 32]), index: 0 },
            committee_votes: Vec::new(),
            drep_votes: Vec::new(),
            spo_votes: Vec::new(),
            deposit: Coin(0),
            return_addr: Vec::new(),
            gov_action: GovAction::NoConfidence { prev_action: None },
            proposed_in: EpochNo(499),
            expires_after: EpochNo(510),
        }];
        state.gov_state = crate::state::GovStateProjection::Authoritative(Some(gov));

        // CRE S4.3a: a threshold-passing (potentially-ratifiable) action is TERMINAL at the boundary on BOTH the
        // replay and accumulator paths — S4.3a performs NO enactment. The boundary must NOT silently dissolve the
        // committee (that was the pre-S4.3 partial-enactment path). Committee dissolution
        // (`apply_committee_enactment`) is unit-tested in `governance.rs` and wired atomically for the supported
        // action subset in S4.3c.
        let err = apply_epoch_boundary_full(&state, EpochNo(501)).unwrap_err();
        assert!(
            matches!(
                err,
                crate::governance::GovernanceTerminal::UnsupportedRatifiedAction {
                    kind: crate::governance::UnsupportedActionKind::NotParameterChange,
                    ..
                }
            ),
            "a ratified NoConfidence (non-ParameterChange) terminals the boundary, got {err:?}",
        );
    }

    /// CE-3a (LIVE-LEDGER-EPOCH-TRANSITION S3, DC-EPOCH-21): the single canonical POOLREAP now lives
    /// inside `apply_epoch_boundary_with_registrations` — future-pool adoption → reap (== e) → deposit
    /// refund by the real reward-account discriminant → delegation-clear → pool/retiring removal. Each
    /// case is built on `LedgerState::new(Conway)` (reserves/treasury 0, empty go snapshot, gov None) so
    /// the reward path contributes nothing: treasury moves only by the deposit-refund split and rewards
    /// only by it, isolating the POOLREAP effect under test.
    mod poolreap_ce3a {
        use super::super::apply_epoch_boundary_with_registrations;
        use crate::delegation::PoolParams;
        use crate::state::LedgerState;
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::tx::{Coin, PoolId};
        use ade_types::{CardanoEra, EpochNo, Hash28, Hash32};

        const POOL_DEPOSIT: u64 = 500_000_000;

        fn state_at(epoch: u64) -> LedgerState {
            let mut state = LedgerState::new(CardanoEra::Conway);
            state.epoch_state.epoch = EpochNo(epoch);
            state.protocol_params.pool_deposit = Coin(POOL_DEPOSIT);
            state
        }

        fn pid(b: u8) -> PoolId {
            PoolId(Hash28([b; 28]))
        }

        /// An empty point-bound base. These tests exercise POOLREAP (reap/refund/delegation-clear), not the mark,
        /// but a Conway boundary requires the base input (else it terminals) — so supply an empty one.
        fn base0() -> crate::epoch_accumulator::BoundaryBaseStake {
            crate::epoch_accumulator::BoundaryBaseStake {
                boundary_point: ade_types::SlotNo(0),
                canonical_credential_stake: std::collections::BTreeMap::new(),
            }
        }

        /// A pool whose reward account is `header ‖ [cred_byte;28]` — header 0xE0 = key-hash stake,
        /// 0xF0 = script-hash stake.
        fn pool_with_account(id: u8, header: u8, cred_byte: u8) -> PoolParams {
            let mut reward_account = vec![header];
            reward_account.extend_from_slice(&[cred_byte; 28]);
            PoolParams {
                pool_id: pid(id),
                vrf_hash: Hash32([id; 32]),
                pledge: Coin(0),
                cost: Coin(0),
                margin: (0, 1),
                reward_account,
                owners: vec![],
            }
        }

        // (a) reap pools retiring at EXACTLY the crossed epoch — never `> e`, and never `< e` (the
        //     stale case is what distinguishes the strict `== e` from a `<= e`: a `<= e` would wrongly
        //     reap the stale one).
        #[test]
        fn poolreap_reaps_exact_epoch_only() {
            let mut state = state_at(500);
            let pools = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.pools;
            pools.insert(pid(0xA1), pool_with_account(0xA1, 0xE0, 0x01));
            pools.insert(pid(0xB2), pool_with_account(0xB2, 0xE0, 0x02));
            pools.insert(pid(0xC3), pool_with_account(0xC3, 0xE0, 0x03));
            let retiring = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.retiring;
            retiring.insert(pid(0xA1), EpochNo(501)); // == e  → reaped
            retiring.insert(pid(0xB2), EpochNo(502)); // > e   → kept
            retiring.insert(pid(0xC3), EpochNo(499)); // < e (stale) → kept under `==`, reaped under `<=`

            let (out, _ac) =
                apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&base0()), 21_600)
                    .unwrap();
            let pool = &out.cert_state.as_authoritative().expect("authoritative cert state in test").pool;

            assert!(
                !pool.pools.contains_key(&pid(0xA1)),
                "retire == e is reaped"
            );
            assert!(!pool.retiring.contains_key(&pid(0xA1)));
            assert!(
                pool.pools.contains_key(&pid(0xB2)),
                "retire > e kept (== e, not <= e)"
            );
            assert_eq!(pool.retiring.get(&pid(0xB2)), Some(&EpochNo(502)));
            assert!(
                pool.pools.contains_key(&pid(0xC3)),
                "a STALE retire < e is kept under strict `== e` (a `<= e` would wrongly reap it)"
            );
            assert_eq!(pool.retiring.get(&pid(0xC3)), Some(&EpochNo(499)));
        }

        // (b) registered operator reward account → refund; unregistered → treasury.
        #[test]
        fn poolreap_refund_registered_else_treasury() {
            let registered = StakeCredential::KeyHash(Hash28([0x11; 28]));
            let mut state = state_at(500);
            let pools = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.pools;
            pools.insert(pid(0xA1), pool_with_account(0xA1, 0xE0, 0x11));
            pools.insert(pid(0xB2), pool_with_account(0xB2, 0xE0, 0x22));
            let retiring = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.retiring;
            retiring.insert(pid(0xA1), EpochNo(501));
            retiring.insert(pid(0xB2), EpochNo(501));
            let regs = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").delegation.registrations;
            regs.insert(registered.clone(), Coin(2_000_000));

            let (out, _ac) =
                apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&base0()), 21_600)
                    .unwrap();

            assert_eq!(
                out.cert_state.as_authoritative().expect("authoritative cert state in test").delegation.rewards.get(&registered),
                Some(&Coin(POOL_DEPOSIT)),
                "registered operator account is refunded the pool deposit",
            );
            assert_eq!(
                out.epoch_state.treasury,
                Coin(POOL_DEPOSIT),
                "the unregistered operator's deposit is the only treasury movement",
            );
        }

        // (c) THE regression: a delegation to a reaped pool is cleared (the dead-clear bug).
        #[test]
        fn poolreap_clears_reaped_pool_delegations() {
            let c = StakeCredential::KeyHash(Hash28([0xCC; 28]));
            let e = StakeCredential::KeyHash(Hash28([0xEE; 28]));
            let mut state = state_at(500);
            let pools = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.pools;
            pools.insert(pid(0xA1), pool_with_account(0xA1, 0xE0, 0x01));
            pools.insert(pid(0xB2), pool_with_account(0xB2, 0xE0, 0x02));
            let retiring = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.retiring;
            retiring.insert(pid(0xA1), EpochNo(501));
            let regs = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").delegation.registrations;
            regs.insert(c.clone(), Coin(2_000_000));
            regs.insert(e.clone(), Coin(2_000_000));
            let delegs = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").delegation.delegations;
            delegs.insert(c.clone(), pid(0xA1));
            delegs.insert(e.clone(), pid(0xB2));

            let (out, _ac) =
                apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&base0()), 21_600)
                    .unwrap();
            let deleg = &out.cert_state.as_authoritative().expect("authoritative cert state in test").delegation;

            assert_eq!(
                deleg.delegations.get(&c),
                None,
                "delegation to reaped pool cleared"
            );
            assert_eq!(
                deleg.delegations.get(&e),
                Some(&pid(0xB2)),
                "surviving delegation kept"
            );
            assert!(
                deleg.registrations.contains_key(&c),
                "delegator registration preserved"
            );
        }

        // (d) a script-hash (0xF0) reward account refunds to a ScriptHash credential, not KeyHash.
        #[test]
        fn poolreap_script_hash_reward_account_refunds_to_script_cred() {
            let script_cred = StakeCredential::ScriptHash(Hash28([0x33; 28]));
            let key_cred = StakeCredential::KeyHash(Hash28([0x33; 28]));
            let mut state = state_at(500);
            let pools = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.pools;
            pools.insert(pid(0xA1), pool_with_account(0xA1, 0xF0, 0x33));
            let retiring = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.retiring;
            retiring.insert(pid(0xA1), EpochNo(501));
            let regs = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").delegation.registrations;
            regs.insert(script_cred.clone(), Coin(2_000_000));

            let (out, _ac) =
                apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&base0()), 21_600)
                    .unwrap();
            let rewards = &out.cert_state.as_authoritative().expect("authoritative cert state in test").delegation.rewards;

            assert_eq!(
                rewards.get(&script_cred),
                Some(&Coin(POOL_DEPOSIT)),
                "a 0xF0 reward account refunds to its ScriptHash credential by the real discriminant",
            );
            assert_eq!(
                rewards.get(&key_cred),
                None,
                "the refund is NOT mis-routed to a KeyHash projection of the same 28 bytes",
            );
            assert_eq!(
                out.epoch_state.treasury,
                Coin(0),
                "a registered script reward account is refunded, not sent to treasury",
            );
        }

        // (e) future-pool adoption still fires at the boundary (and an orphan future is dropped).
        #[test]
        fn poolreap_adopts_future_pool_params() {
            let mut active = pool_with_account(0xA1, 0xE0, 0x01);
            active.pledge = Coin(100);
            let mut staged = pool_with_account(0xA1, 0xE0, 0x01);
            staged.pledge = Coin(200);
            let mut state = state_at(500);
            state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.pools.insert(pid(0xA1), active);
            let future = &mut state.cert_state.as_authoritative_mut().expect("authoritative cert state in test").pool.future_pools;
            future.insert(pid(0xA1), staged);
            // An orphan future (no matching active pool) must be dropped, not adopted.
            future.insert(pid(0xB2), pool_with_account(0xB2, 0xE0, 0x02));

            let (out, _ac) =
                apply_epoch_boundary_with_registrations(&state, EpochNo(501), None, Some(&base0()), 21_600)
                    .unwrap();
            let pool = &out.cert_state.as_authoritative().expect("authoritative cert state in test").pool;

            assert_eq!(
                pool.pools.get(&pid(0xA1)).map(|p| p.pledge),
                Some(Coin(200)),
                "the staged re-registration params are adopted into the active set",
            );
            assert!(
                pool.future_pools.is_empty(),
                "future_pools drained on adoption"
            );
            assert!(
                !pool.pools.contains_key(&pid(0xB2)),
                "orphan future dropped, not adopted"
            );
        }
    }

    /// A governance apply error (DRep expiry needed, drep_activity absent) halts
    /// accumulation fail-closed — never a defaulted expiry, never a swallow.
    #[test]
    fn gov_apply_error_halts_accumulation() {
        let bytes = cert_array(drep_reg_cert(1));
        let gov = Some(empty_gov());
        let res = accumulate_tx_certs(CardanoEra::Conway, &bytes, &CertState::new(), &gov, KD, None);
        assert!(
            matches!(
                res,
                Err(LedgerError::ValidationEnvironment(
                    ValidationEnvironmentError::MissingDRepActivityParam
                ))
            ),
            "DRep accumulation with absent env must halt fail-closed",
        );
    }
}
