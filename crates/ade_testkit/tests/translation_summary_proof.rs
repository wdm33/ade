//! Integration test: Encoding-independent translation summary proof.
//!
//! Defines a structured state summary that both Ade's translation and
//! the oracle can produce identically. If summaries match, the gap
//! between Ade and oracle is serializer-specific, not semantic.
//!
//! This is the experiment that informs the CE-73 proof surface decision.

use ade_ledger::hfc::translate_era;
use ade_ledger::state::{EpochState, LedgerState};
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::utxo::UTxOState;
use ade_types::{CardanoEra, EpochNo, SlotNo};
use ade_types::tx::Coin;

/// Encoding-independent state summary.
///
/// Every field is a deterministic value that both Ade and the oracle
/// can produce without depending on CBOR encoding choices. If these
/// match across a translation boundary, the remaining CE-73 gap is
/// serializer-specific.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StateSummary {
    // Identity
    era: CardanoEra,
    epoch: u64,

    // Monetary
    treasury_lovelace: u64,
    reserves_lovelace: u64,

    // UTxO
    utxo_count: usize,

    // Protocol parameters (values, not encoding)
    pp_min_fee_a: u64,
    pp_min_fee_b: u64,
    pp_max_block_body_size: u32,
    pp_max_tx_size: u32,
    pp_max_block_header_size: u32,
    pp_key_deposit: u64,
    pp_pool_deposit: u64,
    pp_e_max: u32,
    pp_n_opt: u32,
    pp_pool_influence_num: i128,
    pp_pool_influence_den: i128,
    pp_monetary_expansion_num: i128,
    pp_monetary_expansion_den: i128,
    pp_treasury_growth_num: i128,
    pp_treasury_growth_den: i128,
    pp_protocol_major: u32,
    pp_protocol_minor: u32,
    pp_min_pool_cost: u64,
}

impl StateSummary {
    fn from_ledger_state(state: &LedgerState) -> Self {
        let pp = &state.protocol_params;
        Self {
            era: state.era,
            epoch: state.epoch_state.epoch.0,
            treasury_lovelace: state.epoch_state.treasury.0,
            reserves_lovelace: state.epoch_state.reserves.0,
            utxo_count: state.utxo_state.len(),
            pp_min_fee_a: pp.min_fee_a.0,
            pp_min_fee_b: pp.min_fee_b.0,
            pp_max_block_body_size: pp.max_block_body_size,
            pp_max_tx_size: pp.max_tx_size,
            pp_max_block_header_size: pp.max_block_header_size,
            pp_key_deposit: pp.key_deposit.0,
            pp_pool_deposit: pp.pool_deposit.0,
            pp_e_max: pp.e_max,
            pp_n_opt: pp.n_opt,
            pp_pool_influence_num: pp.pool_influence.numerator(),
            pp_pool_influence_den: pp.pool_influence.denominator(),
            pp_monetary_expansion_num: pp.monetary_expansion.numerator(),
            pp_monetary_expansion_den: pp.monetary_expansion.denominator(),
            pp_treasury_growth_num: pp.treasury_growth.numerator(),
            pp_treasury_growth_den: pp.treasury_growth.denominator(),
            pp_protocol_major: pp.protocol_major,
            pp_protocol_minor: pp.protocol_minor,
            pp_min_pool_cost: pp.min_pool_cost.0,
        }
    }

    /// Build from oracle-extracted values (from sub_state_summaries.toml
    /// and protocol_params_oracle.toml).
    fn from_oracle(
        era: CardanoEra,
        epoch: u64,
        treasury: u64,
        reserves: u64,
        utxo_count: usize,
        pp: &OracleParams,
    ) -> Self {
        Self {
            era,
            epoch,
            treasury_lovelace: treasury,
            reserves_lovelace: reserves,
            utxo_count,
            pp_min_fee_a: pp.min_fee_a,
            pp_min_fee_b: pp.min_fee_b,
            pp_max_block_body_size: pp.max_block_body_size,
            pp_max_tx_size: pp.max_tx_size,
            pp_max_block_header_size: pp.max_block_header_size,
            pp_key_deposit: pp.key_deposit,
            pp_pool_deposit: pp.pool_deposit,
            pp_e_max: pp.e_max,
            pp_n_opt: pp.n_opt,
            pp_pool_influence_num: pp.pool_influence_num,
            pp_pool_influence_den: pp.pool_influence_den,
            pp_monetary_expansion_num: pp.monetary_expansion_num,
            pp_monetary_expansion_den: pp.monetary_expansion_den,
            pp_treasury_growth_num: pp.treasury_growth_num,
            pp_treasury_growth_den: pp.treasury_growth_den,
            pp_protocol_major: pp.protocol_major,
            pp_protocol_minor: pp.protocol_minor,
            pp_min_pool_cost: pp.min_pool_cost,
        }
    }
}

struct OracleParams {
    min_fee_a: u64,
    min_fee_b: u64,
    max_block_body_size: u32,
    max_tx_size: u32,
    max_block_header_size: u32,
    key_deposit: u64,
    pool_deposit: u64,
    e_max: u32,
    n_opt: u32,
    pool_influence_num: i128,
    pool_influence_den: i128,
    monetary_expansion_num: i128,
    monetary_expansion_den: i128,
    treasury_growth_num: i128,
    treasury_growth_den: i128,
    protocol_major: u32,
    protocol_minor: u32,
    min_pool_cost: u64,
}

/// Oracle protocol params at the Shelley→Allegra HFC boundary (epoch 236).
fn oracle_params_shelley_allegra() -> OracleParams {
    OracleParams {
        min_fee_a: 44,
        min_fee_b: 155381,
        max_block_body_size: 65536,
        max_tx_size: 16384,
        max_block_header_size: 1100,
        key_deposit: 2_000_000,
        pool_deposit: 500_000_000,
        e_max: 18,
        n_opt: 500,
        pool_influence_num: 3,
        pool_influence_den: 10,
        monetary_expansion_num: 3,
        monetary_expansion_den: 1000,
        treasury_growth_num: 1,
        treasury_growth_den: 5,
        protocol_major: 3,  // Allegra = protocol version 3
        protocol_minor: 0,
        min_pool_cost: 340_000_000,
    }
}

fn make_shelley_state_at_hfc() -> LedgerState {
    // State matching oracle at Shelley→Allegra HFC (epoch 236)
    use ade_ledger::rational::Rational;

    LedgerState {
        utxo_state: UTxOState::new(),
        epoch_state: EpochState {
            epoch: EpochNo(236),
            slot: SlotNo(16_588_800),
            snapshots: ade_ledger::epoch::SnapshotState::new(),
            reserves: Coin(13_112_607_632_000_000),
            treasury: Coin(217_021_606_000_000),
            block_production: std::collections::BTreeMap::new(),
            epoch_fees: Coin(0),
        },
        protocol_params: ProtocolParameters {
            min_fee_a: Coin(44),
            min_fee_b: Coin(155_381),
            max_block_body_size: 65536,
            max_tx_size: 16384,
            max_block_header_size: 1100,
            key_deposit: Coin(2_000_000),
            pool_deposit: Coin(500_000_000),
            e_max: 18,
            n_opt: 500,  // mainnet value, not genesis default
            pool_influence: Rational::new(3, 10).unwrap(),
            monetary_expansion: Rational::new(3, 1000).unwrap(),
            treasury_growth: Rational::new(1, 5).unwrap(),
            protocol_major: 3,  // Allegra protocol version
            protocol_minor: 0,
            min_utxo_value: Coin(1_000_000),
            min_pool_cost: Coin(340_000_000),
            decentralization: Rational::new(8, 25).unwrap(),
        },
        era: CardanoEra::Shelley,
        track_utxo: false,
        cert_state: ade_ledger::delegation::CertState::new(),
        max_lovelace_supply: 45_000_000_000_000_000,
    }
}

#[test]
fn shelley_allegra_summary_matches_oracle() {
    let pre_state = make_shelley_state_at_hfc();
    let post_state = translate_era(&pre_state, CardanoEra::Allegra).unwrap();

    let ade_summary = StateSummary::from_ledger_state(&post_state);

    let oracle_summary = StateSummary::from_oracle(
        CardanoEra::Allegra,
        236,
        217_021_606_000_000,
        13_112_607_632_000_000,
        0, // UTxO empty in both (no loaded UTxO)
        &oracle_params_shelley_allegra(),
    );

    eprintln!("\n=== Shelley→Allegra Summary Proof ===");

    let mut mismatches = Vec::new();

    macro_rules! cmp_field {
        ($field:ident, $label:expr) => {
            if ade_summary.$field != oracle_summary.$field {
                mismatches.push(format!(
                    "{}: ade={:?}, oracle={:?}",
                    $label, ade_summary.$field, oracle_summary.$field
                ));
                eprintln!("  ✗ {}: ade={:?}, oracle={:?}", $label, ade_summary.$field, oracle_summary.$field);
            } else {
                eprintln!("  ✓ {}: {:?}", $label, ade_summary.$field);
            }
        };
    }

    cmp_field!(era, "era");
    cmp_field!(epoch, "epoch");
    cmp_field!(treasury_lovelace, "treasury");
    cmp_field!(reserves_lovelace, "reserves");
    cmp_field!(utxo_count, "utxo_count");
    cmp_field!(pp_min_fee_a, "min_fee_a");
    cmp_field!(pp_min_fee_b, "min_fee_b");
    cmp_field!(pp_max_block_body_size, "max_block_body_size");
    cmp_field!(pp_max_tx_size, "max_tx_size");
    cmp_field!(pp_max_block_header_size, "max_block_header_size");
    cmp_field!(pp_key_deposit, "key_deposit");
    cmp_field!(pp_pool_deposit, "pool_deposit");
    cmp_field!(pp_e_max, "e_max");
    cmp_field!(pp_n_opt, "n_opt");
    cmp_field!(pp_pool_influence_num, "pool_influence_num");
    cmp_field!(pp_pool_influence_den, "pool_influence_den");
    cmp_field!(pp_monetary_expansion_num, "monetary_expansion_num");
    cmp_field!(pp_monetary_expansion_den, "monetary_expansion_den");
    cmp_field!(pp_treasury_growth_num, "treasury_growth_num");
    cmp_field!(pp_treasury_growth_den, "treasury_growth_den");
    cmp_field!(pp_protocol_major, "protocol_major");
    cmp_field!(pp_protocol_minor, "protocol_minor");
    cmp_field!(pp_min_pool_cost, "min_pool_cost");

    if mismatches.is_empty() {
        eprintln!("\n  RESULT: All 22 fields match. Gap is serializer-specific, not semantic.");
    } else {
        eprintln!("\n  RESULT: {} mismatches found — semantic gap exists.", mismatches.len());
    }
    eprintln!("====================================\n");

    assert_eq!(ade_summary, oracle_summary, "summaries must match");
}
