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

    // Check for epoch boundary, capture accounting if it fires
    let mut accounting = None;
    let pre_boundary_state = if let Some(new_epoch) = crate::state::detect_epoch_transition(
        state.epoch_state.epoch, slot,
    ) {
        let (new_state, acct) = apply_epoch_boundary_full(state, new_epoch);
        accounting = Some(acct);
        new_state
    } else {
        state.clone()
    };

    // Apply block normally on the (possibly post-boundary) state
    let (final_state, verdict) = apply_block_classified(&pre_boundary_state, era, block_cbor)?;
    Ok((final_state, verdict, accounting))
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
        let (new_state, _accounting) = apply_epoch_boundary_full(&current_state, new_epoch);
        current_state = new_state;
    }

    let mut verdict = decode_validate_tx_bodies(block, era)?;

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

    let cert_state = if current_state.track_utxo {
        process_block_certificates(block, era, &current_state)?
    } else {
        current_state.cert_state.clone()
    };

    let mut epoch_state = current_state.epoch_state;
    epoch_state.slot = slot;

    Ok(BlockApplyResult {
        new_state: LedgerState {
            utxo_state,
            epoch_state,
            protocol_params: current_state.protocol_params,
            era,
            track_utxo: current_state.track_utxo,
            cert_state,
            max_lovelace_supply: current_state.max_lovelace_supply,
            gov_state: None,
        },
        verdict,
        tx_verdicts,
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
        let (new_state, _accounting) = apply_epoch_boundary_full(&current_state, new_epoch);
        current_state = new_state;
    }

    let mut verdict = decode_validate_tx_bodies(block, era)?;

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
            max_lovelace_supply: current_state.max_lovelace_supply,
        gov_state: None,
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
) -> (LedgerState, EpochBoundaryAccounting) {
    apply_epoch_boundary_with_registrations(state, new_epoch, None)
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
) -> (LedgerState, EpochBoundaryAccounting) {
    // 1. Reward computation from PRE-rotation go snapshot
    //    Rewards must be computed before rotation — after rotation,
    //    the go snapshot becomes the old set (which may be empty).
    let reserves = state.epoch_state.reserves;
    let treasury = state.epoch_state.treasury;

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
        // expectedBlocks = floor((1-d) * 432000 * 1/20)
        // = floor((1-d) * 21600)
        let one_minus_d = crate::rational::Rational::one()
            .checked_sub(d)
            .unwrap_or_else(crate::rational::Rational::one);
        let epoch_slots = crate::rational::Rational::from_integer(21600);
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
    let go = &state.epoch_state.snapshots.go;

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
        go.0.pool_stakes.len(), state.cert_state.pool.pools.len());

    if total_stake > 0 && total_active_stake > 0 && pool_reward_pot > 0 {
        for (pool_id, pool_stake) in &go.0.pool_stakes {
            let params = match state.cert_state.pool.pools.get(pool_id) {
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
            // state.cert_state.delegation.registrations. The delta_t2 check in
            // applyRUpd uses the same registration source for consistency.

            // Registration check helper: uses override set if provided,
            // falls back to PRE state registrations.
            let is_cred_registered = |h: &ade_types::Hash28| -> bool {
                let sc = ade_types::shelley::cert::StakeCredential(h.clone());
                if let Some(override_regs) = registration_override {
                    override_regs.contains_key(&sc)
                } else {
                    state.cert_state.delegation.registrations.contains_key(&sc)
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

            // Operator's stake share: Haskell uses the full go snapshot stake
            // (not filtered by pool) so an operator's stake counts even if they
            // delegate elsewhere. This matches s/σ in the leader reward formula.
            let op_stake = op_cred.as_ref()
                .and_then(|oc| go.0.delegations.get(oc))
                .map(|(_, c)| c.0)
                .unwrap_or(0);
            let op_share = crate::rational::Rational::new(
                op_stake as i128, pool_stake.0 as i128,
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
                    // Skip operator — already got their share via leaderReward
                    if op_cred.as_ref() == Some(cred) { continue; }
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
    let mut delegation = state.cert_state.delegation.clone();

    for (cred, reward) in &reward_deltas {
        let stake_cred = ade_types::shelley::cert::StakeCredential(cred.clone());
        let is_registered = if let Some(override_regs) = registration_override {
            override_regs.contains_key(&stake_cred)
        } else {
            delegation.registrations.contains_key(&stake_cred)
        };
        if is_registered {
            let entry = delegation.rewards
                .entry(stake_cred)
                .or_insert(ade_types::tx::Coin(0));
            entry.0 = entry.0.saturating_add(reward.0);
        } else {
            delta_t2 = delta_t2.saturating_add(reward.0);
        }
    }

    let _ = (rewarded_pool_count, total_pool_rewards, total_member_rewards, total_stake);

    // 3. Snapshot rotation (AFTER reward computation)
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
            let mut ps = std::collections::BTreeMap::new();
            for pool in state.cert_state.delegation.delegations.values() {
                ps.entry(pool.clone()).or_insert(ade_types::tx::Coin(0));
            }
            ps
        },
    };
    let rotated = crate::epoch::rotate_snapshots(
        &state.epoch_state.snapshots,
        new_mark,
    );

    // 4. Pool retirements effective at this epoch (POOLREAP)
    //
    // When a pool retires:
    // - Pool is removed from the registered pools map
    // - Pool deposit is returned to the operator's reward account
    // - If the operator's reward account is unregistered, deposit goes to treasury
    let mut pool_state = state.cert_state.pool.clone();
    let mut poolreap_to_treasury = 0u64;
    let pool_deposit = state.protocol_params.pool_deposit.0;
    pool_state.retiring.retain(|pool_id, retire_epoch| {
        if retire_epoch.0 <= new_epoch.0 {
            // Check if the pool operator's reward account is registered
            if let Some(params) = pool_state.pools.get(pool_id) {
                if params.reward_account.len() >= 29 {
                    let mut cred = [0u8; 28];
                    cred.copy_from_slice(&params.reward_account[1..29]);
                    let stake_cred = ade_types::shelley::cert::StakeCredential(
                        ade_types::Hash28(cred));
                    if delegation.registrations.contains_key(&stake_cred) {
                        // Registered — return deposit to reward account
                        let entry = delegation.rewards
                            .entry(stake_cred)
                            .or_insert(ade_types::tx::Coin(0));
                        entry.0 = entry.0.saturating_add(pool_deposit);
                    } else {
                        // Unregistered — deposit goes to treasury
                        poolreap_to_treasury += pool_deposit;
                    }
                }
            }
            pool_state.pools.remove(pool_id);
            false
        } else {
            true
        }
    });

    // 4b. Conway governance: ratification, enactment, expiry
    let mut governance_treasury_withdrawn = 0u64;
    let new_gov_state = if state.era == ade_types::CardanoEra::Conway {
        if let Some(ref gov) = state.gov_state {
            // Compute DRep stake distribution from vote delegations + mark snapshot.
            // The mark snapshot is the most recent (current epoch), closest to the
            // Haskell DRepPulser's InstantStake. Using go would be 2 epochs stale.
            let mark = &state.epoch_state.snapshots.mark;
            let mut drep_stake: crate::governance::DRepStakeDistribution =
                std::collections::BTreeMap::new();
            for (cred, drep) in &gov.vote_delegations {
                let stake = mark.0.delegations.get(cred)
                    .map(|(_, c)| c.0)
                    .unwrap_or(0);
                if stake > 0 {
                    *drep_stake.entry(drep.clone()).or_insert(0) += stake;
                }
            }

            let committee_quorum = crate::rational::Rational::new(
                gov.committee_quorum.0 as i128,
                gov.committee_quorum.1.max(1) as i128,
            ).unwrap_or_else(crate::rational::Rational::one);

            // Ratification uses the ENDING epoch (before boundary), not the new epoch.
            // Proposals with expires_after >= ending_epoch are still active.
            let ending_epoch = new_epoch.0.saturating_sub(1);
            let result = crate::governance::evaluate_ratification(
                &gov.proposals,
                &drep_stake,
                &go.0.pool_stakes,
                &gov.committee,
                &committee_quorum,
                &gov.pool_voting_thresholds,
                &gov.drep_voting_thresholds,
                ending_epoch,
                &gov.committee_hot_keys,
                &gov.drep_expiry,
            );

            let effects = crate::governance::enact_proposals(&result.ratified);

            // Deposit refunds: enacted proposal deposits returned from treasury.
            // Only enacted proposals have deposits refunded from treasury.
            // Expired proposal deposits are returned from the deposit pot, not treasury.
            let enacted_deposit_refunds: u64 = effects.deposits_returned.iter()
                .map(|(_, c)| c.0)
                .sum();

            // Deposit refunds for enacted proposals: in Conway, proposal deposits
            // go to the governance deposit pot. When enacted, deposits are returned
            // from that pot, not from treasury. Test both.
            // Treasury outflows from governance:
            // 1. Treasury withdrawals from enacted TreasuryWithdrawal proposals
            // 2. Deposit refunds NOT from treasury (deposits come from deposit pot)
            governance_treasury_withdrawn = effects.treasury_withdrawn;
            eprintln!("  [governance] ratified={} expired={} remaining={} treasury_withdrawn={} ADA deposit_refunds={} ADA",
                result.ratified.len(), result.expired.len(), result.remaining.len(),
                effects.treasury_withdrawn / 1_000_000, enacted_deposit_refunds / 1_000_000);

            Some(crate::state::ConwayGovState {
                proposals: result.remaining,
                committee: gov.committee.clone(),
                committee_quorum: gov.committee_quorum,
                drep_expiry: gov.drep_expiry.clone(),
                gov_action_lifetime: gov.gov_action_lifetime,
                vote_delegations: gov.vote_delegations.clone(),
                pool_voting_thresholds: gov.pool_voting_thresholds.clone(),
                drep_voting_thresholds: gov.drep_voting_thresholds.clone(),
                committee_hot_keys: gov.committee_hot_keys.clone(),
            })
        } else {
            None
        }
    } else {
        state.gov_state.clone()
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
            .saturating_sub(governance_treasury_withdrawn)
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
            snapshots: rotated,
            reserves: new_reserves,
            treasury: new_treasury,
            block_production: std::collections::BTreeMap::new(),
            epoch_fees: ade_types::tx::Coin(0),
        },
        protocol_params: state.protocol_params.clone(),
        era: state.era,
        track_utxo: state.track_utxo,
        cert_state,
        max_lovelace_supply: state.max_lovelace_supply,
        gov_state: new_gov_state,
    };

    (new_state, accounting)
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
                crate::conway::validate_conway_state_backed(
                    &body, utxo, wi, collateral_percent, current_network,
                    pp.protocol_major as u16, max_ex_units,
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
    utxo: &std::collections::BTreeMap<ade_types::tx::TxIn, crate::utxo::TxOut>,
    wi: &crate::witness::WitnessInfo,
    collateral_percent: u16,
    current_network: u8,
    max_ex_units: (i64, i64),
    protocol_major: u16,
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
            let r = crate::conway::validate_conway_state_backed(
                &body, utxo, wi, collateral_percent, current_network,
                protocol_major, max_ex_units,
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
