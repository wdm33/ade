// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

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
            LedgerError::RuleNotYetEnforced(e) => {
                write!(f, "rule {:?} not yet enforced for era {}", e.rule, e.era)
            }
            LedgerError::Decoding(e) => {
                write!(f, "decoding error at offset {}: {:?}", e.offset, e.reason)
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
