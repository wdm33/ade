// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN cardano-cli `query protocol-parameters` JSON parser
//! (PHASE4-N-F-G-A S2a — current protocol parameters source).
//!
//! Parses the oracle's `protocol-parameters` JSON (the `protocol_params_json`
//! preimage carried in the operator consensus-inputs bundle) into a canonical
//! [`ProtocolParameters`], so the recovered ledger can carry the **current**
//! protocol version + modeled parameters instead of `ProtocolParameters::default()`
//! (the stale `protocol_major = 2` the S2 PO-1 entry check exposed).
//!
//! **No float path (hard rule).** JSON number literals for the rational
//! unit-interval / non-negative-interval parameters (`poolPledgeInfluence`,
//! `monetaryExpansion`, `treasuryCut`) are preserved as strings via
//! `serde_json::value::RawValue` and converted to exact [`Rational`] via integer
//! decimal/scientific parsing. There is no `f64`, no `as f64`, and no serde float
//! deserialization anywhere in this module. A literal that cannot be represented
//! exactly (non-numeric, NaN/Inf, exponent overflow) fails closed.
//!
//! **S2a modeled surface (documented).** `ProtocolParameters` is Ade's Shelley–
//! Mary-shaped model. The fields parsed here are the ones it models that are
//! forge/header-relevant; fields the Conway oracle emits that are **outside
//! S2a's currently modeled `ProtocolParameters` / forge-header surface** are
//! ignored (not denied): `utxoCostPerByte`, `executionUnitPrices`, `maxValueSize`,
//! `maxCollateralInputs`, `minFeeRefScriptCostPerByte`, `maxBlockExecutionUnits`,
//! and the Conway governance parameters (`dRep*`, `committee*`, `govAction*`,
//! voting thresholds). These are relevant to full ledger compatibility later;
//! they are simply not S2a authority. `cost_models_cbor` is `None` for S2a — cost
//! models are Plutus budget semantics, not empty-block header/forge fidelity (the
//! node forge path closed here produces empty / self-accepted blocks and does not
//! evaluate Plutus); budget-exact cost-model ingestion is a separate concern.
//! `decentralization` is `0` (definitionally so in Conway). `network_id` is
//! derived from the bundle's `network_magic` (it is not a protocol-parameter).
//! `min_utxo_value` has no Conway source (replaced by `utxoCostPerByte`, outside
//! the modeled surface) and is not consumed on the empty-block forge path; it is
//! set to `0` rather than carried as a fabricated value.

use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::rational::Rational;
use ade_types::tx::Coin;
use serde::Deserialize;
use serde_json::value::RawValue;

/// Mainnet network magic; everything else maps to the testnet network id (0).
const MAINNET_NETWORK_MAGIC: u32 = 764_824_073;

/// Closed parser error surface. No `String` payloads in the load-bearing parts;
/// `field` discriminants are `&'static str` from a closed list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolParamsParseError {
    /// The protocol-parameters JSON is structurally invalid, a required modeled
    /// field is missing, or a modeled field has the wrong JSON type.
    JsonShape,
    /// A rational unit/non-negative-interval literal could not be parsed exactly
    /// into a `Rational` by integer arithmetic (non-numeric, NaN/Inf, or an
    /// exponent that overflows). Fail-closed: never a float fallback.
    InexactRational { field: &'static str },
}

/// Parse a cardano-cli `query protocol-parameters` JSON document into a canonical
/// [`ProtocolParameters`]. `network_magic` supplies `network_id` (not a
/// protocol-parameter). Fail-closed on any modeled-field shape error or inexact
/// rational; ignores fields outside S2a's modeled surface (see module docs).
pub fn parse_protocol_parameters_json(
    json: &[u8],
    network_magic: u32,
) -> Result<ProtocolParameters, ProtocolParamsParseError> {
    let cli: CliProtocolParameters =
        serde_json::from_slice(json).map_err(|_| ProtocolParamsParseError::JsonShape)?;

    let pool_influence =
        parse_exact_rational(cli.pool_pledge_influence.get(), "poolPledgeInfluence")?;
    let monetary_expansion =
        parse_exact_rational(cli.monetary_expansion.get(), "monetaryExpansion")?;
    let treasury_growth = parse_exact_rational(cli.treasury_cut.get(), "treasuryCut")?;

    let network_id: u8 = if network_magic == MAINNET_NETWORK_MAGIC {
        1
    } else {
        0
    };

    Ok(ProtocolParameters {
        min_fee_a: Coin(cli.tx_fee_per_byte),
        min_fee_b: Coin(cli.tx_fee_fixed),
        max_block_body_size: cli.max_block_body_size,
        max_tx_size: cli.max_tx_size,
        max_block_header_size: cli.max_block_header_size,
        key_deposit: Coin(cli.stake_address_deposit),
        pool_deposit: Coin(cli.stake_pool_deposit),
        e_max: cli.pool_retire_max_epoch,
        n_opt: cli.stake_pool_target_num,
        pool_influence,
        monetary_expansion,
        treasury_growth,
        protocol_major: cli.protocol_version.major,
        protocol_minor: cli.protocol_version.minor,
        // Conway has no `minUTxOValue` (replaced by `utxoCostPerByte`, outside
        // S2a's modeled surface); not consumed on the empty-block forge path.
        min_utxo_value: Coin(0),
        min_pool_cost: Coin(cli.min_pool_cost),
        // Definitionally 0 in Conway (the parameter was removed because it is
        // permanently 0). Not the Shelley-launch default of 1.
        decentralization: Rational::zero(),
        collateral_percent: cli.collateral_percentage,
        max_tx_ex_units_mem: cli.max_tx_execution_units.memory,
        max_tx_ex_units_cpu: cli.max_tx_execution_units.steps,
        network_id,
        // Plutus budget semantics — outside S2a's empty-block forge-header surface.
        cost_models_cbor: None,
    })
}

/// The modeled subset of cardano-cli protocol-parameters. Unknown fields are
/// intentionally NOT denied — Conway emits many parameters outside S2a's modeled
/// surface (see module docs). Integer fields deserialize via serde's integer path
/// (never float); rational fields are captured as raw literals (`RawValue`) and
/// parsed exactly downstream — never through `f64`.
#[derive(Deserialize)]
struct CliProtocolParameters {
    #[serde(rename = "txFeePerByte")]
    tx_fee_per_byte: u64,
    #[serde(rename = "txFeeFixed")]
    tx_fee_fixed: u64,
    #[serde(rename = "maxBlockBodySize")]
    max_block_body_size: u32,
    #[serde(rename = "maxTxSize")]
    max_tx_size: u32,
    #[serde(rename = "maxBlockHeaderSize")]
    max_block_header_size: u32,
    #[serde(rename = "stakeAddressDeposit")]
    stake_address_deposit: u64,
    #[serde(rename = "stakePoolDeposit")]
    stake_pool_deposit: u64,
    #[serde(rename = "poolRetireMaxEpoch")]
    pool_retire_max_epoch: u32,
    #[serde(rename = "stakePoolTargetNum")]
    stake_pool_target_num: u32,
    #[serde(rename = "minPoolCost")]
    min_pool_cost: u64,
    #[serde(rename = "collateralPercentage")]
    collateral_percentage: u16,
    #[serde(rename = "poolPledgeInfluence")]
    pool_pledge_influence: Box<RawValue>,
    #[serde(rename = "monetaryExpansion")]
    monetary_expansion: Box<RawValue>,
    #[serde(rename = "treasuryCut")]
    treasury_cut: Box<RawValue>,
    #[serde(rename = "protocolVersion")]
    protocol_version: CliProtocolVersion,
    #[serde(rename = "maxTxExecutionUnits")]
    max_tx_execution_units: CliExUnits,
}

#[derive(Deserialize)]
struct CliProtocolVersion {
    major: u32,
    minor: u32,
}

#[derive(Deserialize)]
struct CliExUnits {
    memory: u64,
    steps: u64,
}

/// Parse a JSON number literal (integer, decimal, or scientific) into an exact
/// [`Rational`] using integer arithmetic only. No `f64`, no float round-trip.
///
/// Algorithm (closed):
/// 1. Strip an optional leading sign.
/// 2. Split a `e`/`E` scientific exponent (a signed integer).
/// 3. Split the mantissa into integer + fraction digit runs (digits only).
/// 4. The exact value is `(int_digits ++ frac_digits) * 10^(exp - frac_len)`.
/// 5. Build `num/den` from that, reduced by [`Rational::new`].
///
/// Fail-closed (`InexactRational`) on: non-digit characters, an empty mantissa,
/// a malformed exponent, or a `10^k` factor that overflows `i128` (k > ~38). It
/// never approximates: the literal is taken at exact face value (the committed
/// `protocol_params_hash` binds the preimage, so this is faithful to the bundle).
fn parse_exact_rational(
    lit: &str,
    field: &'static str,
) -> Result<Rational, ProtocolParamsParseError> {
    let err = || ProtocolParamsParseError::InexactRational { field };
    let s = lit.trim();

    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };

    let (mantissa, exp): (&str, i32) = match s.find(['e', 'E']) {
        Some(i) => {
            let e: i32 = s[i + 1..].parse().map_err(|_| err())?;
            (&s[..i], e)
        }
        None => (s, 0),
    };

    let (int_part, frac_part) = match mantissa.find('.') {
        Some(i) => (&mantissa[..i], &mantissa[i + 1..]),
        None => (mantissa, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(err());
    }
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(err());
    }

    let mut digits = String::with_capacity(int_part.len() + frac_part.len());
    digits.push_str(int_part);
    digits.push_str(frac_part);
    let value: i128 = if digits.is_empty() {
        0
    } else {
        digits.parse().map_err(|_| err())?
    };

    let net_exp: i32 = exp - (frac_part.len() as i32);
    let (num, den): (i128, i128) = if net_exp >= 0 {
        let mul = pow10_i128(net_exp as u32).ok_or_else(err)?;
        (value.checked_mul(mul).ok_or_else(err)?, 1)
    } else {
        let den = pow10_i128((-net_exp) as u32).ok_or_else(err)?;
        (value, den)
    };
    let num = if neg {
        num.checked_neg().ok_or_else(err)?
    } else {
        num
    };

    Rational::new(num, den).ok_or_else(err)
}

/// `10^e` as `i128`, or `None` on overflow (e > ~38). Integer-only.
fn pow10_i128(e: u32) -> Option<i128> {
    let mut r: i128 = 1;
    for _ in 0..e {
        r = r.checked_mul(10)?;
    }
    Some(r)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// The isolated nfg_a private-net cardano-cli 11.0.1 sample, trimmed to the
    /// modeled fields + a representative subset of the ignored Conway fields (to
    /// prove they are ignored, not denied). The rational fields carry the exact
    /// literals the real query emits (`monetaryExpansion`/`treasuryCut` = 0.1,
    /// `poolPledgeInfluence` = 0). Per the S2a implementation note: this parser is
    /// designed against the committed isolated private-net cardano-cli 11.x sample;
    /// decimal/scientific JSON numbers are parsed exactly into Rational; binary
    /// float conversion is forbidden.
    const NFG_A_SAMPLE: &str = r#"{
        "collateralPercentage": 150,
        "committeeMaxTermLength": 200,
        "executionUnitPrices": {"priceMemory": 5.77e-2, "priceSteps": 7.21e-5},
        "maxBlockBodySize": 65536,
        "maxBlockHeaderSize": 1100,
        "maxTxExecutionUnits": {"memory": 140000000, "steps": 10000000000},
        "maxTxSize": 16384,
        "maxValueSize": 5000,
        "minPoolCost": 0,
        "monetaryExpansion": 0.1,
        "poolPledgeInfluence": 0,
        "poolRetireMaxEpoch": 18,
        "protocolVersion": {"major": 2, "minor": 0},
        "stakeAddressDeposit": 400000,
        "stakePoolDeposit": 0,
        "stakePoolTargetNum": 100,
        "treasuryCut": 0.1,
        "txFeeFixed": 0,
        "txFeePerByte": 1,
        "utxoCostPerByte": 4310
    }"#;

    #[test]
    fn nfg_a_sample_parses_to_expected_modeled_pparams() {
        let pp = parse_protocol_parameters_json(NFG_A_SAMPLE.as_bytes(), 42).unwrap();
        assert_eq!(pp.min_fee_a, Coin(1));
        assert_eq!(pp.min_fee_b, Coin(0));
        assert_eq!(pp.max_block_body_size, 65_536);
        assert_eq!(pp.max_tx_size, 16_384);
        assert_eq!(pp.max_block_header_size, 1_100);
        assert_eq!(pp.key_deposit, Coin(400_000));
        assert_eq!(pp.pool_deposit, Coin(0));
        assert_eq!(pp.e_max, 18);
        assert_eq!(pp.n_opt, 100);
        assert_eq!(pp.min_pool_cost, Coin(0));
        assert_eq!(pp.collateral_percent, 150);
        assert_eq!(pp.max_tx_ex_units_mem, 140_000_000);
        assert_eq!(pp.max_tx_ex_units_cpu, 10_000_000_000);
        assert_eq!(pp.protocol_major, 2);
        assert_eq!(pp.protocol_minor, 0);
        // Rationals parsed exactly from the JSON float literals.
        assert_eq!(pp.monetary_expansion, Rational::new(1, 10).unwrap());
        assert_eq!(pp.treasury_growth, Rational::new(1, 10).unwrap());
        assert_eq!(pp.pool_influence, Rational::zero());
        // Era-correct / derived / outside-surface dispositions.
        assert_eq!(pp.decentralization, Rational::zero());
        assert_eq!(pp.network_id, 0); // magic 42 => testnet
        assert_eq!(pp.cost_models_cbor, None);
    }

    #[test]
    fn mainnet_magic_yields_network_id_one() {
        let pp =
            parse_protocol_parameters_json(NFG_A_SAMPLE.as_bytes(), MAINNET_NETWORK_MAGIC).unwrap();
        assert_eq!(pp.network_id, 1);
    }

    #[test]
    fn missing_modeled_field_fails_closed() {
        let bad = NFG_A_SAMPLE.replace("\"txFeePerByte\": 1,", "");
        let err = parse_protocol_parameters_json(bad.as_bytes(), 42).unwrap_err();
        assert_eq!(err, ProtocolParamsParseError::JsonShape);
    }

    #[test]
    fn exact_rational_decimal_and_scientific_cases() {
        // Decimal.
        assert_eq!(
            parse_exact_rational("0.1", "f").unwrap(),
            Rational::new(1, 10).unwrap()
        );
        assert_eq!(
            parse_exact_rational("0.0577", "f").unwrap(),
            Rational::new(577, 10_000).unwrap()
        );
        // Scientific.
        assert_eq!(
            parse_exact_rational("7.21e-5", "f").unwrap(),
            Rational::new(721, 10_000_000).unwrap()
        );
        assert_eq!(
            parse_exact_rational("5.77e-2", "f").unwrap(),
            Rational::new(577, 10_000).unwrap()
        );
        // Integers + reduction.
        assert_eq!(parse_exact_rational("0", "f").unwrap(), Rational::zero());
        assert_eq!(
            parse_exact_rational("3", "f").unwrap(),
            Rational::from_integer(3)
        );
        assert_eq!(
            parse_exact_rational("0.5", "f").unwrap(),
            Rational::new(1, 2).unwrap()
        );
        // Positive exponent.
        assert_eq!(
            parse_exact_rational("1.5e2", "f").unwrap(),
            Rational::from_integer(150)
        );
    }

    #[test]
    fn exact_rational_rejects_non_numeric_and_overflow() {
        assert!(parse_exact_rational("abc", "f").is_err());
        assert!(parse_exact_rational("", "f").is_err());
        assert!(parse_exact_rational("0.1.2", "f").is_err());
        assert!(parse_exact_rational("NaN", "f").is_err());
        // Exponent that overflows the 10^k factor (k > ~38).
        assert!(parse_exact_rational("1e40", "f").is_err());
    }

    #[test]
    fn parser_is_deterministic_across_two_runs() {
        let a = parse_protocol_parameters_json(NFG_A_SAMPLE.as_bytes(), 42).unwrap();
        let b = parse_protocol_parameters_json(NFG_A_SAMPLE.as_bytes(), 42).unwrap();
        assert_eq!(a, b);
    }
}
