// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;

use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, EpochNo, Hash28, SlotNo};

/// Structured ledger error — typed, comparable, canonically serializable.
///
/// All variants carry typed structs with typed fields. No `String`, no
/// `&'static str` detail bags. `PartialEq` derived on everything for
/// mechanical comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum LedgerError {
    // UTxO domain
    InputNotFound(InputNotFoundError),
    DuplicateInput(DuplicateInputError),

    // Value domain
    Conservation(ConservationError),
    NegativeValue(NegativeValueError),
    InsufficientFee(FeeError),

    // Witness domain
    MissingWitness(WitnessError),
    InvalidWitness(WitnessError),
    BootstrapWitnessMismatch(WitnessError),

    // Validity domain
    ExpiredTransaction(ValidityError),
    TransactionNotYetValid(ValidityError),

    // Script domain
    NativeScriptFailed(ScriptError),

    // Minting domain
    MintWithoutPolicy(MintError),

    // Certificate domain
    InvalidCertificate(CertificateError),

    // Epoch domain
    EpochTransition(EpochError),

    // HFC domain
    Translation(TranslationError),

    // Rule authority — era/rule not yet enforced (deterministic refusal, never Ok)
    RuleNotYetEnforced(RuleNotYetEnforcedError),

    // Structural domain (Alonzo+)
    StructuralViolation(StructuralError),

    // Late-era state-backed validation (Alonzo+) — O-27 obligations
    BadInputs(BadInputsError),
    NoCollateralInputs,
    InsufficientCollateral(InsufficientCollateralError),
    CollateralContainsNonADA,
    IncorrectTotalCollateral(IncorrectTotalCollateralError),

    // Late-era state-backed validation (Babbage+/Conway+) — O-28 obligations
    NonDisjointRefInputs(NonDisjointRefInputsError),
    MissingRequiredDatums(MissingRequiredDatumsError),
    MissingRequiredSigners(MissingRequiredSignersError),
    WrongNetworkInTxBody(WrongNetworkError),
    WrongNetworkInOutput(WrongNetworkOutputError),

    // Phase-1 tx-level budget cap (Alonzo+) — O-30.3
    ExUnitsTooBigUTxO(ExUnitsTooBigError),

    // Phase-2 Plutus failures (Alonzo+) — O-32.1
    // Mirrors Haskell `AlonzoUtxosPredFailure::ValidationTagMismatch
    // FailedUnexpectedly` and `CollectErrors` respectively.
    //
    // These are the ONLY two LedgerError categories that
    // `phase::classify_failure_phase` routes to Phase2. All other
    // variants are Phase1 (tx rejected, no state delta). Phase2
    // means the tx stays in the block with a collateral-only state
    // delta applied via `phase::apply_phase_2_failure`.
    PlutusExecutionFailed(PlutusExecutionError),
    PlutusContextBuildFailed(PlutusContextBuildError),

    // Codec passthrough
    Decoding(DecodingError),
}

// ---------------------------------------------------------------------------
// UTxO domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct InputNotFoundError {
    pub tx_in: TxIn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateInputError {
    pub tx_in: TxIn,
}

// ---------------------------------------------------------------------------
// Late-era state-backed validation errors (Alonzo+)
//
// Mirror the Haskell cardano-ledger error constructors:
//   - BadInputs           <- BadInputsUTxO              (Shelley UTXO, reused)
//   - NoCollateralInputs  <- NoCollateralInputs         (Alonzo Utxo)
//   - InsufficientCollateral <- InsufficientCollateral  (Alonzo Utxo)
//   - CollateralContainsNonADA <- CollateralContainsNonADA (Alonzo Utxo)
//   - IncorrectTotalCollateral <- IncorrectTotalCollateralField (Babbage Utxo)
//
// See docs/active/S-27_obligation_discharge.md for citations.
// ---------------------------------------------------------------------------

/// Set of transaction inputs that are not present in the UTxO.
///
/// Covers spend inputs (all eras), collateral inputs (Alonzo+), and
/// reference inputs (Babbage+). The Haskell cardano-ledger treats all
/// three with the same constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadInputsError {
    pub missing: BTreeSet<TxIn>,
}

/// Collateral percent rule: `100 * balance < collateral_percent * fee`.
///
/// `balance` is signed (`i128`) to mirror the Haskell `DeltaCoin`-backed
/// `Integer` and to tolerate adversarial fees without overflow.
/// `required` is the ceiling-rounded required amount, reporting-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsufficientCollateralError {
    pub balance: i128,
    pub required: u64,
    pub percent: u16,
    pub fee: u64,
}

/// Babbage's `totalCollateral` declaration did not match the computed balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncorrectTotalCollateralError {
    pub balance: i128,
    pub declared: u64,
}

/// Conway (PV ≥ 9): a `TxIn` appears in both `inputs` and
/// `reference_inputs`. Mirrors `BabbageNonDisjointRefInputs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonDisjointRefInputsError {
    pub intersection: BTreeSet<TxIn>,
}

/// Alonzo+: one or more required datum hashes could not be matched by
/// any witness-provided datum. Mirrors `MissingRequiredDatums`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRequiredDatumsError {
    pub missing: BTreeSet<ade_types::Hash32>,
}

/// Alonzo+: one or more hashes in `required_signers` are not matched
/// by any vkey witness. Mirrors `MissingVKeyWitnessesUTXOW` when
/// caused specifically by `required_signers` shortfall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRequiredSignersError {
    pub missing: BTreeSet<Hash28>,
}

/// Alonzo+: `network_id` tx body field (key 15) does not match the
/// current network. Mirrors `WrongNetworkInTxBody`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrongNetworkError {
    pub declared: u8,
    pub current: u8,
}

/// Shelley+: an output's address network byte does not match the
/// current network. Mirrors `WrongNetwork`. Reported one output at a
/// time (Haskell reports a set; Ade's equivalent carries one and
/// callers gather as needed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrongNetworkOutputError {
    pub address_first_byte: u8,
    pub current: u8,
}

/// Alonzo+: sum of redeemer `ex_units` across the tx exceeds
/// `ppMaxTxExUnits`. Mirrors `ExUnitsTooBigUTxO` (Alonzo
/// `AlonzoUtxoPredFailure`, CBOR tag 15). Pointwise check:
/// declared exceeds cap in mem, cpu, or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExUnitsTooBigError {
    pub declared_mem: i64,
    pub declared_cpu: i64,
    pub max_mem: i64,
    pub max_cpu: i64,
}

/// Phase-2: one or more Plutus scripts failed during CEK
/// evaluation or exhausted their declared `ex_units` budget.
/// Mirrors Haskell `ValidationTagMismatch (IsValid True)
/// FailedUnexpectedly`.
///
/// Triggers the collateral-consumption state delta via
/// `phase::apply_phase_2_failure`. The tx stays in the block but
/// only collateral is consumed; regular outputs, certs, mint,
/// withdrawals are NOT applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlutusExecutionError {
    /// Index of the failing redeemer in the tx witness set.
    pub redeemer_index: u32,
    /// Whether the failure was budget exhaustion (true) or a
    /// CEK-level error term (false).
    pub budget_exhausted: bool,
}

/// Phase-2: ScriptContext / redeemer / cost-model couldn't be
/// constructed for one or more scripts. Mirrors Haskell
/// `CollectErrors` sub-variants (`NoRedeemer`, `NoWitness`,
/// `NoCostModel`, `BadTranslation`).
///
/// Like `PlutusExecutionFailed`, triggers the collateral-only
/// state delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlutusContextBuildError {
    pub reason: PlutusContextBuildReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlutusContextBuildReason {
    /// Redeemer referenced by a script is missing from the
    /// witness set.
    MissingRedeemer,
    /// Witness required for ScriptContext construction is absent.
    MissingWitness,
    /// Cost model for the script's language version is not
    /// present in protocol parameters.
    MissingCostModel,
    /// A tx field could not be translated into PlutusData form
    /// (e.g., address too long, invalid datum, etc.).
    BadTranslation,
}

// ---------------------------------------------------------------------------
// Value domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ConservationError {
    pub consumed_coin: Coin,
    pub produced_coin: Coin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NegativeValueError {
    pub coin: Coin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeeError {
    pub required: Coin,
    pub provided: Coin,
}

// ---------------------------------------------------------------------------
// Witness domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct WitnessError {
    pub key_hash: Hash28,
    pub algorithm: WitnessAlgorithm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WitnessAlgorithm {
    Ed25519,
    Ed25519Extended,
    Bootstrap,
}

// ---------------------------------------------------------------------------
// Validity domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ValidityError {
    pub current_slot: SlotNo,
    pub bound: SlotNo,
}

// ---------------------------------------------------------------------------
// Script domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptError {
    pub script_hash: Hash28,
    pub reason: NativeScriptFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScriptFailure {
    MissingRequiredSignature { key_hash: Hash28 },
    TimelockNotSatisfied { required_slot: SlotNo, current_slot: SlotNo },
    ThresholdNotMet { required: u32, provided: u32 },
}

// ---------------------------------------------------------------------------
// Minting domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct MintError {
    pub policy_id: Hash28,
}

// ---------------------------------------------------------------------------
// Certificate domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CertificateError {
    pub cert_index: u16,
    pub reason: CertFailureReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CertFailureReason {
    StakeAlreadyRegistered,
    StakeNotRegistered,
    PoolAlreadyRegistered,
    PoolNotRegistered,
    InvalidPoolParams,
}

// ---------------------------------------------------------------------------
// Epoch domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct EpochError {
    pub epoch: EpochNo,
    pub era: CardanoEra,
    pub reason: EpochFailureReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EpochFailureReason {
    RewardOverflow,
    InvalidParameterUpdate,
    SnapshotRotationFailure,
}

// ---------------------------------------------------------------------------
// HFC domain errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct TranslationError {
    pub from_era: CardanoEra,
    pub to_era: CardanoEra,
    pub reason: TranslationFailureReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranslationFailureReason {
    InvalidSourceState,
    MissingGenesisParameter,
    UtxoConversionFailure,
}

// ---------------------------------------------------------------------------
// Structural domain errors (Alonzo+)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct StructuralError {
    pub era: CardanoEra,
    pub reason: StructuralFailureReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructuralFailureReason {
    /// script_data_hash present but collateral_inputs absent (Alonzo+)
    MissingCollateral,
    /// collateral_return present but collateral_inputs absent (Babbage+)
    CollateralReturnWithoutCollateral,
    /// total_collateral present but collateral_inputs absent (Babbage+)
    TotalCollateralWithoutCollateral,
    /// collateral_inputs present in pre-Alonzo era
    CollateralInPreAlonzoEra,
    /// reference_inputs present in pre-Babbage era
    ReferenceInputsInPreBabbageEra,
    /// voting_procedures present in pre-Conway era
    GovernanceFieldInPreConwayEra,
    /// proposal_procedures present in pre-Conway era
    ProposalFieldInPreConwayEra,
    /// donation present in pre-Conway era
    DonationInPreConwayEra,
    /// treasury_value present in pre-Conway era
    TreasuryInPreConwayEra,
    /// transaction has no inputs
    EmptyInputs,
    /// transaction has no outputs
    EmptyOutputs,
    /// transaction fee is zero
    ZeroFee,
    /// an output has zero coin value
    ZeroCoinOutput,
}

// ---------------------------------------------------------------------------
// Rule authority errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RuleNotYetEnforcedError {
    pub era: CardanoEra,
    pub rule: RuleName,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleName {
    ApplyBlock,
    EpochBoundary,
    EraTranslation,
}

// ---------------------------------------------------------------------------
// Codec passthrough errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct DecodingError {
    pub offset: usize,
    pub reason: DecodingFailureReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodingFailureReason {
    InvalidStructure,
    UnexpectedType,
    TrailingBytes,
}

// ---------------------------------------------------------------------------
// Display implementations
// ---------------------------------------------------------------------------

impl core::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LedgerError::InputNotFound(e) => {
                write!(f, "input not found: {:?}#{}", e.tx_in.tx_hash, e.tx_in.index)
            }
            LedgerError::DuplicateInput(e) => {
                write!(f, "duplicate input: {:?}#{}", e.tx_in.tx_hash, e.tx_in.index)
            }
            LedgerError::Conservation(e) => {
                write!(
                    f,
                    "conservation violation: consumed {} != produced {}",
                    e.consumed_coin, e.produced_coin
                )
            }
            LedgerError::NegativeValue(e) => {
                write!(f, "negative value: {}", e.coin)
            }
            LedgerError::InsufficientFee(e) => {
                write!(
                    f,
                    "insufficient fee: required {} but provided {}",
                    e.required, e.provided
                )
            }
            LedgerError::MissingWitness(e) => {
                write!(f, "missing witness for key hash {:?}", e.key_hash)
            }
            LedgerError::InvalidWitness(e) => {
                write!(f, "invalid witness for key hash {:?}", e.key_hash)
            }
            LedgerError::BootstrapWitnessMismatch(e) => {
                write!(f, "bootstrap witness mismatch for key hash {:?}", e.key_hash)
            }
            LedgerError::ExpiredTransaction(e) => {
                write!(
                    f,
                    "transaction expired: current slot {} past ttl {}",
                    e.current_slot.0, e.bound.0
                )
            }
            LedgerError::TransactionNotYetValid(e) => {
                write!(
                    f,
                    "transaction not yet valid: current slot {} before start {}",
                    e.current_slot.0, e.bound.0
                )
            }
            LedgerError::NativeScriptFailed(e) => {
                write!(f, "native script failed: {:?}", e.script_hash)
            }
            LedgerError::MintWithoutPolicy(e) => {
                write!(f, "mint without policy: {:?}", e.policy_id)
            }
            LedgerError::InvalidCertificate(e) => {
                write!(f, "invalid certificate at index {}: {:?}", e.cert_index, e.reason)
            }
            LedgerError::EpochTransition(e) => {
                write!(f, "epoch transition error at epoch {}: {:?}", e.epoch.0, e.reason)
            }
            LedgerError::Translation(e) => {
                write!(
                    f,
                    "translation error {} -> {}: {:?}",
                    e.from_era, e.to_era, e.reason
                )
            }
            LedgerError::StructuralViolation(e) => {
                write!(f, "structural violation in {}: {:?}", e.era, e.reason)
            }
            LedgerError::RuleNotYetEnforced(e) => {
                write!(f, "rule {:?} not yet enforced for era {}", e.rule, e.era)
            }
            LedgerError::Decoding(e) => {
                write!(f, "decoding error at offset {}: {:?}", e.offset, e.reason)
            }
            LedgerError::BadInputs(e) => {
                write!(f, "bad inputs: {} missing from UTxO", e.missing.len())
            }
            LedgerError::NoCollateralInputs => {
                write!(f, "no collateral inputs provided")
            }
            LedgerError::InsufficientCollateral(e) => {
                write!(
                    f,
                    "insufficient collateral: balance {} < required {} (percent {} of fee {})",
                    e.balance, e.required, e.percent, e.fee
                )
            }
            LedgerError::CollateralContainsNonADA => {
                write!(f, "collateral contains non-ADA assets without collateral return")
            }
            LedgerError::IncorrectTotalCollateral(e) => {
                write!(
                    f,
                    "incorrect total_collateral: declared {} != balance {}",
                    e.declared, e.balance
                )
            }
            LedgerError::NonDisjointRefInputs(e) => {
                write!(
                    f,
                    "reference inputs overlap spend inputs: {} offending",
                    e.intersection.len()
                )
            }
            LedgerError::MissingRequiredDatums(e) => {
                write!(f, "missing required datums: {} unmatched", e.missing.len())
            }
            LedgerError::MissingRequiredSigners(e) => {
                write!(f, "missing required signers: {} unmatched", e.missing.len())
            }
            LedgerError::WrongNetworkInTxBody(e) => {
                write!(
                    f,
                    "tx body network_id {} does not match current network {}",
                    e.declared, e.current
                )
            }
            LedgerError::WrongNetworkInOutput(e) => {
                write!(
                    f,
                    "output address network nibble {} does not match current network {}",
                    e.address_first_byte & 0x0f,
                    e.current
                )
            }
            LedgerError::ExUnitsTooBigUTxO(e) => {
                write!(
                    f,
                    "ex_units too big: declared (mem={}, cpu={}) exceeds cap (mem={}, cpu={})",
                    e.declared_mem, e.declared_cpu, e.max_mem, e.max_cpu
                )
            }
            LedgerError::PlutusExecutionFailed(e) => {
                if e.budget_exhausted {
                    write!(
                        f,
                        "plutus execution failed: redeemer index {} exhausted declared ex_units",
                        e.redeemer_index
                    )
                } else {
                    write!(
                        f,
                        "plutus execution failed: redeemer index {} produced error term",
                        e.redeemer_index
                    )
                }
            }
            LedgerError::PlutusContextBuildFailed(e) => {
                write!(f, "plutus context build failed: {:?}", e.reason)
            }
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<ade_codec::CodecError> for LedgerError {
    fn from(e: ade_codec::CodecError) -> Self {
        let (offset, reason) = match e {
            ade_codec::CodecError::InvalidCborStructure { offset, .. } => {
                (offset, DecodingFailureReason::InvalidStructure)
            }
            ade_codec::CodecError::UnexpectedCborType { offset, .. } => {
                (offset, DecodingFailureReason::UnexpectedType)
            }
            ade_codec::CodecError::TrailingBytes { consumed, .. } => {
                (consumed, DecodingFailureReason::TrailingBytes)
            }
            ade_codec::CodecError::UnexpectedEof { offset, .. } => {
                (offset, DecodingFailureReason::InvalidStructure)
            }
            ade_codec::CodecError::UnknownEraTag { .. } => {
                (0, DecodingFailureReason::UnexpectedType)
            }
            ade_codec::CodecError::InvalidLength { offset, .. } => {
                (offset, DecodingFailureReason::InvalidStructure)
            }
        };
        LedgerError::Decoding(DecodingError { offset, reason })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::Hash32;

    #[test]
    fn ledger_error_equality() {
        let a = LedgerError::InputNotFound(InputNotFoundError {
            tx_in: TxIn {
                tx_hash: Hash32([0xaa; 32]),
                index: 0,
            },
        });
        let b = LedgerError::InputNotFound(InputNotFoundError {
            tx_in: TxIn {
                tx_hash: Hash32([0xaa; 32]),
                index: 0,
            },
        });
        let c = LedgerError::InputNotFound(InputNotFoundError {
            tx_in: TxIn {
                tx_hash: Hash32([0xbb; 32]),
                index: 0,
            },
        });
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn conservation_error_display() {
        let e = LedgerError::Conservation(ConservationError {
            consumed_coin: Coin(1_000_000),
            produced_coin: Coin(2_000_000),
        });
        let s = format!("{e}");
        assert!(s.contains("1000000"));
        assert!(s.contains("2000000"));
    }

    #[test]
    fn rule_not_yet_enforced_display() {
        let e = LedgerError::RuleNotYetEnforced(RuleNotYetEnforcedError {
            era: CardanoEra::ByronRegular,
            rule: RuleName::ApplyBlock,
        });
        let s = format!("{e}");
        assert!(s.contains("ApplyBlock"));
        assert!(s.contains("byron_regular"));
    }

    #[test]
    fn codec_error_conversion() {
        let codec_err = ade_codec::CodecError::InvalidCborStructure {
            offset: 42,
            detail: "test",
        };
        let ledger_err: LedgerError = codec_err.into();
        match ledger_err {
            LedgerError::Decoding(e) => {
                assert_eq!(e.offset, 42);
                assert_eq!(e.reason, DecodingFailureReason::InvalidStructure);
            }
            _ => std::unreachable!(),
        }
    }

    #[test]
    fn all_witness_algorithms_comparable() {
        assert_ne!(WitnessAlgorithm::Ed25519, WitnessAlgorithm::Bootstrap);
        assert_eq!(
            WitnessAlgorithm::Ed25519Extended,
            WitnessAlgorithm::Ed25519Extended
        );
    }
}
