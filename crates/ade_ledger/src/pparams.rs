// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeMap;
use ade_types::tx::Coin;
use ade_types::Hash28;
use crate::error::{
    EpochError, EpochFailureReason, LedgerError,
};
use crate::rational::Rational;

/// Full protocol parameters for Shelley through Mary eras.
///
/// All monetary values in lovelace. All rationals use exact integer arithmetic.
/// No floating point anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolParameters {
    // -- Fee parameters --

    /// Fee coefficient 'a' (per-byte fee, in lovelace).
    pub min_fee_a: Coin,
    /// Fee constant 'b' (fixed fee, in lovelace).
    pub min_fee_b: Coin,

    // -- Size limits --

    /// Maximum block body size in bytes.
    pub max_block_body_size: u32,
    /// Maximum transaction size in bytes.
    pub max_tx_size: u32,
    /// Maximum block header size in bytes.
    pub max_block_header_size: u32,

    // -- Staking parameters --

    /// Key deposit (lovelace required to register a staking key).
    pub key_deposit: Coin,
    /// Pool deposit (lovelace required to register a pool).
    pub pool_deposit: Coin,
    /// Maximum epoch for pool retirement (how far in advance).
    pub e_max: u32,
    /// Desired number of stake pools (k, used for saturation).
    pub n_opt: u32,

    // -- Reward parameters --

    /// Pool influence factor (a0) — pledge influence on rewards.
    /// Stored as Rational for exact arithmetic.
    pub pool_influence: Rational,
    /// Monetary expansion rate (rho) — fraction of reserves to rewards per epoch.
    pub monetary_expansion: Rational,
    /// Treasury growth rate (tau) — fraction of rewards going to treasury.
    pub treasury_growth: Rational,

    // -- Protocol version --

    /// Major protocol version.
    pub protocol_major: u32,
    /// Minor protocol version.
    pub protocol_minor: u32,

    // -- UTxO parameters --

    /// Minimum UTxO value (in lovelace).
    pub min_utxo_value: Coin,

    // -- Pool parameters --

    /// Minimum pool cost (in lovelace).
    pub min_pool_cost: Coin,

    // -- Decentralization --

    /// Decentralization parameter (d).
    /// d = 1 means fully federated (all BFT blocks).
    /// d = 0 means fully decentralized (all pool blocks).
    /// Removed in Babbage (permanently 0).
    pub decentralization: Rational,
}

impl Default for ProtocolParameters {
    fn default() -> Self {
        // Shelley mainnet genesis defaults
        ProtocolParameters {
            min_fee_a: Coin(44),
            min_fee_b: Coin(155_381),
            max_block_body_size: 65_536,
            max_tx_size: 16_384,
            max_block_header_size: 1_100,
            key_deposit: Coin(2_000_000),
            pool_deposit: Coin(500_000_000),
            e_max: 18,
            n_opt: 150,
            // a0 = 3/10 in Shelley genesis
            pool_influence: Rational::new(3, 10).unwrap_or_else(Rational::zero),
            // rho = 3/1000
            monetary_expansion: Rational::new(3, 1000).unwrap_or_else(Rational::zero),
            // tau = 2/10
            treasury_growth: Rational::new(2, 10).unwrap_or_else(Rational::zero),
            protocol_major: 2,
            protocol_minor: 0,
            min_utxo_value: Coin(1_000_000),
            min_pool_cost: Coin(340_000_000),
            // d = 1 at Shelley launch (fully federated); decreases over time to 0
            decentralization: Rational::new(1, 1).unwrap_or_else(Rational::zero),
        }
    }
}

/// A proposed protocol parameter update.
///
/// Each field is optional — only fields that are being updated are Some.
/// Applied atomically at the epoch boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolParameterUpdate {
    pub min_fee_a: Option<Coin>,
    pub min_fee_b: Option<Coin>,
    pub max_block_body_size: Option<u32>,
    pub max_tx_size: Option<u32>,
    pub max_block_header_size: Option<u32>,
    pub key_deposit: Option<Coin>,
    pub pool_deposit: Option<Coin>,
    pub e_max: Option<u32>,
    pub n_opt: Option<u32>,
    pub pool_influence: Option<Rational>,
    pub monetary_expansion: Option<Rational>,
    pub treasury_growth: Option<Rational>,
    pub protocol_major: Option<u32>,
    pub protocol_minor: Option<u32>,
    pub min_utxo_value: Option<Coin>,
    pub min_pool_cost: Option<Coin>,
    pub decentralization: Option<Rational>,
}

impl ProtocolParameterUpdate {
    /// Empty update (no changes).
    pub fn empty() -> Self {
        ProtocolParameterUpdate {
            min_fee_a: None,
            min_fee_b: None,
            max_block_body_size: None,
            max_tx_size: None,
            max_block_header_size: None,
            key_deposit: None,
            pool_deposit: None,
            e_max: None,
            n_opt: None,
            pool_influence: None,
            monetary_expansion: None,
            treasury_growth: None,
            protocol_major: None,
            protocol_minor: None,
            min_utxo_value: None,
            min_pool_cost: None,
            decentralization: None,
        }
    }
}

/// Apply protocol parameter updates at an epoch boundary.
///
/// In Cardano, genesis delegates propose parameter updates during an epoch.
/// At the boundary, if a quorum of proposals agree on a parameter, it is applied.
///
/// This function takes the current parameters and a set of proposals
/// (keyed by genesis delegate hash). It requires a strict majority (> 50%)
/// for any parameter change.
///
/// Returns the updated parameters, or an error if the update is invalid.
pub fn apply_parameter_updates(
    current: &ProtocolParameters,
    proposals: &BTreeMap<Hash28, ProtocolParameterUpdate>,
    quorum_threshold: usize,
    current_epoch: ade_types::EpochNo,
    current_era: ade_types::CardanoEra,
) -> Result<ProtocolParameters, LedgerError> {
    if proposals.is_empty() {
        return Ok(current.clone());
    }

    // Collect all proposed values for each field and find quorum agreement
    let merged = merge_proposals(proposals, quorum_threshold);

    // Validate and apply the merged update
    apply_update(current, &merged, current_epoch, current_era)
}

/// Merge multiple proposals: for each field, if >= quorum_threshold
/// proposals agree on the same value, that value is adopted.
fn merge_proposals(
    proposals: &BTreeMap<Hash28, ProtocolParameterUpdate>,
    quorum_threshold: usize,
) -> ProtocolParameterUpdate {
    let mut result = ProtocolParameterUpdate::empty();

    // min_fee_a
    result.min_fee_a = find_quorum_coin(
        proposals.values().filter_map(|p| p.min_fee_a),
        quorum_threshold,
    );

    // min_fee_b
    result.min_fee_b = find_quorum_coin(
        proposals.values().filter_map(|p| p.min_fee_b),
        quorum_threshold,
    );

    // max_block_body_size
    result.max_block_body_size = find_quorum_u32(
        proposals.values().filter_map(|p| p.max_block_body_size),
        quorum_threshold,
    );

    // max_tx_size
    result.max_tx_size = find_quorum_u32(
        proposals.values().filter_map(|p| p.max_tx_size),
        quorum_threshold,
    );

    // max_block_header_size
    result.max_block_header_size = find_quorum_u32(
        proposals.values().filter_map(|p| p.max_block_header_size),
        quorum_threshold,
    );

    // key_deposit
    result.key_deposit = find_quorum_coin(
        proposals.values().filter_map(|p| p.key_deposit),
        quorum_threshold,
    );

    // pool_deposit
    result.pool_deposit = find_quorum_coin(
        proposals.values().filter_map(|p| p.pool_deposit),
        quorum_threshold,
    );

    // e_max
    result.e_max = find_quorum_u32(
        proposals.values().filter_map(|p| p.e_max),
        quorum_threshold,
    );

    // n_opt
    result.n_opt = find_quorum_u32(
        proposals.values().filter_map(|p| p.n_opt),
        quorum_threshold,
    );

    // protocol_major
    result.protocol_major = find_quorum_u32(
        proposals.values().filter_map(|p| p.protocol_major),
        quorum_threshold,
    );

    // protocol_minor
    result.protocol_minor = find_quorum_u32(
        proposals.values().filter_map(|p| p.protocol_minor),
        quorum_threshold,
    );

    // min_utxo_value
    result.min_utxo_value = find_quorum_coin(
        proposals.values().filter_map(|p| p.min_utxo_value),
        quorum_threshold,
    );

    // min_pool_cost
    result.min_pool_cost = find_quorum_coin(
        proposals.values().filter_map(|p| p.min_pool_cost),
        quorum_threshold,
    );

    // Rational fields: pool_influence, monetary_expansion, treasury_growth, decentralization
    // For simplicity, these use the first proposal that reaches quorum
    result.decentralization = find_quorum_rational(
        proposals.values().filter_map(|p| p.decentralization.as_ref()),
        quorum_threshold,
    );

    result.pool_influence = find_quorum_rational(
        proposals.values().filter_map(|p| p.pool_influence.as_ref()),
        quorum_threshold,
    );

    result.monetary_expansion = find_quorum_rational(
        proposals.values().filter_map(|p| p.monetary_expansion.as_ref()),
        quorum_threshold,
    );

    result.treasury_growth = find_quorum_rational(
        proposals.values().filter_map(|p| p.treasury_growth.as_ref()),
        quorum_threshold,
    );

    result
}

/// Find quorum agreement among Coin proposals.
fn find_quorum_coin(values: impl Iterator<Item = Coin>, threshold: usize) -> Option<Coin> {
    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    for v in values {
        *counts.entry(v.0).or_insert(0) += 1;
    }
    for (val, count) in &counts {
        if *count >= threshold {
            return Some(Coin(*val));
        }
    }
    None
}

/// Find quorum agreement among u32 proposals.
fn find_quorum_u32(values: impl Iterator<Item = u32>, threshold: usize) -> Option<u32> {
    let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
    for v in values {
        *counts.entry(v).or_insert(0) += 1;
    }
    for (val, count) in &counts {
        if *count >= threshold {
            return Some(*val);
        }
    }
    None
}

/// Find quorum agreement among Rational proposals.
fn find_quorum_rational<'a>(
    values: impl Iterator<Item = &'a Rational>,
    threshold: usize,
) -> Option<Rational> {
    let collected: Vec<&Rational> = values.collect();
    // Count occurrences by equality
    let mut groups: Vec<(&Rational, usize)> = Vec::new();
    for val in &collected {
        let mut found = false;
        for (existing, count) in &mut groups {
            if *existing == *val {
                *count += 1;
                found = true;
                break;
            }
        }
        if !found {
            groups.push((val, 1));
        }
    }
    for (val, count) in &groups {
        if *count >= threshold {
            return Some((*val).clone());
        }
    }
    None
}

/// Apply a merged update to current parameters.
fn apply_update(
    current: &ProtocolParameters,
    update: &ProtocolParameterUpdate,
    current_epoch: ade_types::EpochNo,
    current_era: ade_types::CardanoEra,
) -> Result<ProtocolParameters, LedgerError> {
    let mut pp = current.clone();

    if let Some(v) = update.min_fee_a {
        pp.min_fee_a = v;
    }
    if let Some(v) = update.min_fee_b {
        pp.min_fee_b = v;
    }
    if let Some(v) = update.max_block_body_size {
        if v == 0 {
            return Err(LedgerError::EpochTransition(EpochError {
                epoch: current_epoch,
                era: current_era,
                reason: EpochFailureReason::InvalidParameterUpdate,
            }));
        }
        pp.max_block_body_size = v;
    }
    if let Some(v) = update.max_tx_size {
        if v == 0 {
            return Err(LedgerError::EpochTransition(EpochError {
                epoch: current_epoch,
                era: current_era,
                reason: EpochFailureReason::InvalidParameterUpdate,
            }));
        }
        pp.max_tx_size = v;
    }
    if let Some(v) = update.max_block_header_size {
        pp.max_block_header_size = v;
    }
    if let Some(v) = update.key_deposit {
        pp.key_deposit = v;
    }
    if let Some(v) = update.pool_deposit {
        pp.pool_deposit = v;
    }
    if let Some(v) = update.e_max {
        pp.e_max = v;
    }
    if let Some(v) = update.n_opt {
        if v == 0 {
            return Err(LedgerError::EpochTransition(EpochError {
                epoch: current_epoch,
                era: current_era,
                reason: EpochFailureReason::InvalidParameterUpdate,
            }));
        }
        pp.n_opt = v;
    }
    if let Some(ref v) = update.pool_influence {
        if !v.is_non_negative() {
            return Err(LedgerError::EpochTransition(EpochError {
                epoch: current_epoch,
                era: current_era,
                reason: EpochFailureReason::InvalidParameterUpdate,
            }));
        }
        pp.pool_influence = v.clone();
    }
    if let Some(ref v) = update.monetary_expansion {
        if !v.is_non_negative() {
            return Err(LedgerError::EpochTransition(EpochError {
                epoch: current_epoch,
                era: current_era,
                reason: EpochFailureReason::InvalidParameterUpdate,
            }));
        }
        pp.monetary_expansion = v.clone();
    }
    if let Some(ref v) = update.treasury_growth {
        if !v.is_non_negative() {
            return Err(LedgerError::EpochTransition(EpochError {
                epoch: current_epoch,
                era: current_era,
                reason: EpochFailureReason::InvalidParameterUpdate,
            }));
        }
        pp.treasury_growth = v.clone();
    }
    if let Some(v) = update.protocol_major {
        pp.protocol_major = v;
    }
    if let Some(v) = update.protocol_minor {
        pp.protocol_minor = v;
    }
    if let Some(v) = update.min_utxo_value {
        pp.min_utxo_value = v;
    }
    if let Some(v) = update.min_pool_cost {
        pp.min_pool_cost = v;
    }
    if let Some(ref v) = update.decentralization {
        if !v.is_non_negative() {
            return Err(LedgerError::EpochTransition(EpochError {
                epoch: current_epoch,
                era: current_era,
                reason: EpochFailureReason::InvalidParameterUpdate,
            }));
        }
        pp.decentralization = v.clone();
    }

    Ok(pp)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::{CardanoEra, EpochNo};

    fn make_genesis_key(byte: u8) -> Hash28 {
        Hash28([byte; 28])
    }

    // -----------------------------------------------------------------------
    // Default parameters
    // -----------------------------------------------------------------------

    #[test]
    fn default_params_are_shelley_mainnet() {
        let pp = ProtocolParameters::default();
        assert_eq!(pp.min_fee_a, Coin(44));
        assert_eq!(pp.min_fee_b, Coin(155_381));
        assert_eq!(pp.max_tx_size, 16_384);
        assert_eq!(pp.key_deposit, Coin(2_000_000));
        assert_eq!(pp.pool_deposit, Coin(500_000_000));
        assert_eq!(pp.e_max, 18);
        assert_eq!(pp.n_opt, 150);
        assert_eq!(pp.protocol_major, 2);
        assert_eq!(pp.min_utxo_value, Coin(1_000_000));
        assert_eq!(pp.min_pool_cost, Coin(340_000_000));
    }

    // -----------------------------------------------------------------------
    // apply_parameter_updates
    // -----------------------------------------------------------------------

    #[test]
    fn no_proposals_returns_current() {
        let pp = ProtocolParameters::default();
        let proposals = BTreeMap::new();
        let result = apply_parameter_updates(
            &pp,
            &proposals,
            1,
            EpochNo(1),
            CardanoEra::Shelley,
        )
        .unwrap();
        assert_eq!(result, pp);
    }

    #[test]
    fn single_proposal_with_quorum_one() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();
        let mut update = ProtocolParameterUpdate::empty();
        update.min_fee_a = Some(Coin(50));
        proposals.insert(make_genesis_key(0x01), update);

        let result = apply_parameter_updates(
            &pp,
            &proposals,
            1,
            EpochNo(1),
            CardanoEra::Shelley,
        )
        .unwrap();

        assert_eq!(result.min_fee_a, Coin(50));
        // Other params unchanged
        assert_eq!(result.min_fee_b, pp.min_fee_b);
    }

    #[test]
    fn quorum_not_met_keeps_current() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();

        let mut update1 = ProtocolParameterUpdate::empty();
        update1.min_fee_a = Some(Coin(50));
        proposals.insert(make_genesis_key(0x01), update1);

        let mut update2 = ProtocolParameterUpdate::empty();
        update2.min_fee_a = Some(Coin(60)); // Different value!
        proposals.insert(make_genesis_key(0x02), update2);

        // Quorum requires 2, but the two proposals disagree
        let result = apply_parameter_updates(
            &pp,
            &proposals,
            2,
            EpochNo(1),
            CardanoEra::Shelley,
        )
        .unwrap();

        // No quorum → original value kept
        assert_eq!(result.min_fee_a, pp.min_fee_a);
    }

    #[test]
    fn quorum_met_for_agreeing_proposals() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();

        let mut update1 = ProtocolParameterUpdate::empty();
        update1.max_tx_size = Some(32_768);
        proposals.insert(make_genesis_key(0x01), update1);

        let mut update2 = ProtocolParameterUpdate::empty();
        update2.max_tx_size = Some(32_768); // Same value
        proposals.insert(make_genesis_key(0x02), update2);

        let result = apply_parameter_updates(
            &pp,
            &proposals,
            2,
            EpochNo(1),
            CardanoEra::Shelley,
        )
        .unwrap();

        assert_eq!(result.max_tx_size, 32_768);
    }

    #[test]
    fn zero_max_tx_size_rejected() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();
        let mut update = ProtocolParameterUpdate::empty();
        update.max_tx_size = Some(0);
        proposals.insert(make_genesis_key(0x01), update);

        let result = apply_parameter_updates(
            &pp,
            &proposals,
            1,
            EpochNo(1),
            CardanoEra::Shelley,
        );

        assert!(matches!(
            result,
            Err(LedgerError::EpochTransition(EpochError {
                reason: EpochFailureReason::InvalidParameterUpdate,
                ..
            }))
        ));
    }

    #[test]
    fn zero_n_opt_rejected() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();
        let mut update = ProtocolParameterUpdate::empty();
        update.n_opt = Some(0);
        proposals.insert(make_genesis_key(0x01), update);

        let result = apply_parameter_updates(
            &pp,
            &proposals,
            1,
            EpochNo(1),
            CardanoEra::Shelley,
        );

        assert!(matches!(
            result,
            Err(LedgerError::EpochTransition(_))
        ));
    }

    #[test]
    fn negative_monetary_expansion_rejected() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();
        let mut update = ProtocolParameterUpdate::empty();
        update.monetary_expansion = Some(Rational::new(-1, 10).unwrap());
        proposals.insert(make_genesis_key(0x01), update);

        let result = apply_parameter_updates(
            &pp,
            &proposals,
            1,
            EpochNo(1),
            CardanoEra::Shelley,
        );

        assert!(matches!(
            result,
            Err(LedgerError::EpochTransition(_))
        ));
    }

    #[test]
    fn multiple_fields_updated_atomically() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();
        let mut update = ProtocolParameterUpdate::empty();
        update.min_fee_a = Some(Coin(50));
        update.min_fee_b = Some(Coin(200_000));
        update.max_tx_size = Some(32_768);
        update.key_deposit = Some(Coin(3_000_000));
        proposals.insert(make_genesis_key(0x01), update);

        let result = apply_parameter_updates(
            &pp,
            &proposals,
            1,
            EpochNo(5),
            CardanoEra::Allegra,
        )
        .unwrap();

        assert_eq!(result.min_fee_a, Coin(50));
        assert_eq!(result.min_fee_b, Coin(200_000));
        assert_eq!(result.max_tx_size, 32_768);
        assert_eq!(result.key_deposit, Coin(3_000_000));
    }

    #[test]
    fn rational_update_applied() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();
        let mut update = ProtocolParameterUpdate::empty();
        update.pool_influence = Some(Rational::new(1, 5).unwrap());
        proposals.insert(make_genesis_key(0x01), update);

        let result = apply_parameter_updates(
            &pp,
            &proposals,
            1,
            EpochNo(1),
            CardanoEra::Shelley,
        )
        .unwrap();

        assert_eq!(result.pool_influence, Rational::new(1, 5).unwrap());
    }

    #[test]
    fn apply_parameter_updates_deterministic() {
        let pp = ProtocolParameters::default();
        let mut proposals = BTreeMap::new();
        let mut update = ProtocolParameterUpdate::empty();
        update.min_fee_a = Some(Coin(50));
        proposals.insert(make_genesis_key(0x01), update);

        let r1 = apply_parameter_updates(&pp, &proposals, 1, EpochNo(1), CardanoEra::Shelley).unwrap();
        let r2 = apply_parameter_updates(&pp, &proposals, 1, EpochNo(1), CardanoEra::Shelley).unwrap();
        assert_eq!(r1, r2);
    }
}
