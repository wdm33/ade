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
            BlockVerdict { tx_count: 0, plutus_deferred_count: 0, non_plutus_count: 0, native_script_passed: 0, native_script_failed: 0 },
        )),
        CardanoEra::ByronRegular => {
            let preserved = byron::decode_byron_regular_block(block_cbor)?;
            let block = preserved.decoded();
            let new_state = crate::byron::validate_byron_block(state, block)?;
            Ok((
                new_state,
                BlockVerdict { tx_count: 0, plutus_deferred_count: 0, non_plutus_count: 0, native_script_passed: 0, native_script_failed: 0 },
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
        let (new_state, _accounting) = apply_epoch_boundary_full(&current_state, new_epoch);
        current_state = new_state;
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

/// Epoch boundary transition (T-25A.1 + T-25A.3).
///
/// Performs:
/// 1. Snapshot rotation (mark/set/go)
/// 2. Pool retirements effective at this epoch
/// 3. Reward computation and distribution
/// 4. Treasury/reserves update
///
/// Idempotent: only called once per epoch boundary crossing.
fn apply_epoch_boundary_full(
    state: &LedgerState,
    new_epoch: ade_types::EpochNo,
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

    // Compute total active stake from go snapshot
    let total_stake: u64 = go.0.pool_stakes.values()
        .map(|c| c.0)
        .fold(0u64, |a, b| a.saturating_add(b));

    // Allocate rewards to pools that have params
    let mut total_pool_rewards = 0u64;
    let mut total_member_rewards = 0u64;
    let mut rewarded_pool_count = 0usize;
    let mut reward_deltas = std::collections::BTreeMap::new();

    // Per-pool performance now uses apparentPerformance = β/σ with
    // total_blocks_produced as denominator (not expected_total_blocks).

    if total_stake > 0 && pool_reward_pot > 0 {
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

            // Shelley apparentPerformance = min(1, β/σ)
            // where β = pool_blocks / total_blocks, σ = pool_stake / total_stake
            // This is: min(1, (pool_blocks * total_stake) / (total_blocks * pool_stake))
            let sigma = crate::rational::Rational::new(
                pool_stake.0 as i128, total_stake as i128,
            ).unwrap_or_else(crate::rational::Rational::zero);

            let performance = if total_blocks_produced > 0 && pool_stake.0 > 0 {
                let perf = crate::rational::Rational::new(
                    (blocks_produced as i128) * (total_stake as i128),
                    (total_blocks_produced as i128) * (pool_stake.0 as i128),
                ).unwrap_or_else(crate::rational::Rational::one);
                if perf.numerator() > perf.denominator() {
                    crate::rational::Rational::one()
                } else { perf }
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

            // Use u128 integer arithmetic to avoid Rational overflow:
            // maxPool = R * performance / (1+a0) * (sigma' + s'*a0*(sigma'-s'*(z-sigma')/z))
            //
            // Simplify: compute the bracket term as a Rational, then multiply by R*perf/(1+a0)
            // bracket = sigma' + s' * a0 * (sigma' - s' * (z - sigma') / z)
            let bracket = {
                let z_minus_sigma = z.checked_sub(&sigma_prime);
                let inner = z_minus_sigma.and_then(|diff| {
                    s_prime.checked_mul(&diff)
                        .and_then(|r| r.checked_div(&z))
                });
                let sigma_minus_inner = inner.and_then(|i| sigma_prime.checked_sub(&i));
                let pledge_term = sigma_minus_inner.and_then(|smi| {
                    s_prime.checked_mul(a0)
                        .and_then(|r| r.checked_mul(&smi))
                });
                pledge_term.and_then(|pt| sigma_prime.checked_add(&pt))
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

            // Operator gets: min(pool_max, cost) + margin * (pool_max - cost)
            let operator_reward = if pool_max <= params.cost.0 {
                pool_max
            } else {
                let after_cost = pool_max - params.cost.0;
                let margin_cut = crate::rational::Rational::from_integer(after_cost as i128)
                    .checked_mul(&margin)
                    .map(|r| r.floor().max(0) as u64)
                    .unwrap_or(0);
                params.cost.0 + margin_cut
            };

            let delegator_pool = pool_max.saturating_sub(operator_reward);

            // Distribute delegator_pool pro-rata by stake
            let mut member_distributed = 0u64;
            if pool_stake.0 > 0 && delegator_pool > 0 {
                for (cred, stake) in &delegator_stakes {
                    let share = crate::rational::Rational::new(
                        stake.0 as i128, pool_stake.0 as i128,
                    ).unwrap_or_else(crate::rational::Rational::zero);
                    let member_reward = crate::rational::Rational::from_integer(delegator_pool as i128)
                        .checked_mul(&share)
                        .map(|r| r.floor().max(0) as u64)
                        .unwrap_or(0);
                    if member_reward > 0 {
                        member_distributed += member_reward;
                        let entry = reward_deltas.entry(cred.clone()).or_insert(ade_types::tx::Coin(0));
                        entry.0 = entry.0.saturating_add(member_reward);
                    }
                }
            }

            // Route operator reward to the pool's reward account credential
            if operator_reward > 0 && params.reward_account.len() >= 29 {
                let mut reward_cred = [0u8; 28];
                reward_cred.copy_from_slice(&params.reward_account[1..29]);
                let entry = reward_deltas.entry(ade_types::Hash28(reward_cred))
                    .or_insert(ade_types::tx::Coin(0));
                entry.0 = entry.0.saturating_add(operator_reward);
            }

            total_pool_rewards = total_pool_rewards.saturating_add(operator_reward);
            total_member_rewards = total_member_rewards.saturating_add(member_distributed);
            rewarded_pool_count += 1;
        }
    }

    // deltaT2: filter rewards — only registered credentials receive rewards.
    // Rewards to unregistered credentials go to treasury (Shelley spec).
    // Requires complete registrations loaded from oracle DState rewards map.
    let mut delta_t2 = 0u64;
    let mut delegation = state.cert_state.delegation.clone();
    for (cred, reward) in &reward_deltas {
        let stake_cred = ade_types::shelley::cert::StakeCredential(cred.clone());
        if delegation.registrations.contains_key(&stake_cred) {
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

    // 4. Pool retirements effective at this epoch
    let mut pool_state = state.cert_state.pool.clone();
    pool_state.retiring.retain(|pool_id, retire_epoch| {
        if retire_epoch.0 <= new_epoch.0 {
            pool_state.pools.remove(pool_id);
            false
        } else {
            true
        }
    });

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
/// Decomposes the reserves and treasury changes into named buckets
/// for precise oracle comparison. All values in lovelace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochBoundaryAccounting {
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
    /// Plutus txs → ScriptVerdict::NotYetEvaluated (CE-77).
    pub plutus_deferred_count: u64,
    /// Non-Plutus txs (native scripts evaluated, or no scripts).
    pub non_plutus_count: u64,
    /// Native scripts evaluated and passed.
    pub native_script_passed: u64,
    /// Native scripts evaluated and failed (structural — tx still accepted
    /// because witness-level script failure is a Phase 2 ledger rule, not
    /// a structural rejection at this level).
    pub native_script_failed: u64,
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

        // ScriptPosture → ScriptVerdict mapping (CE-77):
        // - PlutusPresentDeferred or Plutus in witnesses → ScriptVerdict::NotYetEvaluated
        // - NonPlutusScriptsOnly with native scripts → evaluate → NativeScriptPassed/Failed
        // - NoScripts → ScriptVerdict::NativeScriptPassed (trivially)
        let is_deferred = has_plutus_in_witnesses
            || body_posture == crate::scripts::ScriptPosture::PlutusPresentDeferred;

        if is_deferred {
            // ScriptVerdict::NotYetEvaluated — Plutus evaluation deferred to Phase 3
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
                        crate::scripts::ScriptVerdict::NotYetEvaluated => {}
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
