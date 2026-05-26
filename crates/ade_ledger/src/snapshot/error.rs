// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE closed error sums for the snapshot encoder/decoder
//! (PHASE4-N-J S1).

use ade_codec::CodecError;
use ade_types::{CardanoEra, Hash32};

/// Closed snapshot-encode-error sum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotEncodeError {
    /// Pre-Conway era encountered. PHASE4-N-J ships Conway-only.
    EraNotSupported { era: CardanoEra },
}

/// Closed snapshot-decode-error sum.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotDecodeError {
    /// Bytes failed CBOR parsing. Carries the upstream codec error.
    Cbor(CodecError),
    /// Version tag does not match the supported version (== 1 in
    /// this cluster). Decoder rejects before decoding the payload.
    UnknownVersion { expected: u32, found: u32 },
    /// Embedded source fingerprint does not match the recomputed
    /// fingerprint on the decoded state. Corruption / schema drift.
    FingerprintMismatch { expected: Hash32, actual: Hash32 },
    /// Bytes embed a pre-Conway era. Same scope discipline as
    /// the encoder.
    EraNotSupported { era: CardanoEra },
    /// Structural rejection: the decoder reached a state that's
    /// inconsistent with the encoder's canonical shape (wrong
    /// array length, missing field, etc.). Carries a static tag
    /// rather than a String so the sum stays closed.
    Structural { reason: StructuralReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralReason {
    ArrayLengthMismatch,
    MapLengthExceeded,
    UnexpectedNull,
    UnexpectedNonNull,
    NonceLengthMismatch,
    PoolIdLengthMismatch,
    Hash32LengthMismatch,
    Hash28LengthMismatch,
    EraTagOutOfRange,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_encode_error_round_trips_through_pattern_match() {
        let errs = vec![SnapshotEncodeError::EraNotSupported {
            era: CardanoEra::ByronEbb,
        }];
        for e in errs {
            match e {
                SnapshotEncodeError::EraNotSupported { .. } => {}
            }
        }
    }

    #[test]
    fn snapshot_decode_error_round_trips_through_pattern_match() {
        let errs = vec![
            SnapshotDecodeError::UnknownVersion {
                expected: 1,
                found: 2,
            },
            SnapshotDecodeError::FingerprintMismatch {
                expected: Hash32([0u8; 32]),
                actual: Hash32([1u8; 32]),
            },
            SnapshotDecodeError::EraNotSupported {
                era: CardanoEra::ByronEbb,
            },
            SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            },
        ];
        for e in errs {
            match e {
                SnapshotDecodeError::Cbor(_) => {}
                SnapshotDecodeError::UnknownVersion { .. } => {}
                SnapshotDecodeError::FingerprintMismatch { .. } => {}
                SnapshotDecodeError::EraNotSupported { .. } => {}
                SnapshotDecodeError::Structural { .. } => {}
            }
        }
    }

    #[test]
    fn structural_reason_round_trips_through_pattern_match() {
        let reasons = [
            StructuralReason::ArrayLengthMismatch,
            StructuralReason::MapLengthExceeded,
            StructuralReason::UnexpectedNull,
            StructuralReason::UnexpectedNonNull,
            StructuralReason::NonceLengthMismatch,
            StructuralReason::PoolIdLengthMismatch,
            StructuralReason::Hash32LengthMismatch,
            StructuralReason::Hash28LengthMismatch,
            StructuralReason::EraTagOutOfRange,
        ];
        for r in reasons {
            match r {
                StructuralReason::ArrayLengthMismatch => {}
                StructuralReason::MapLengthExceeded => {}
                StructuralReason::UnexpectedNull => {}
                StructuralReason::UnexpectedNonNull => {}
                StructuralReason::NonceLengthMismatch => {}
                StructuralReason::PoolIdLengthMismatch => {}
                StructuralReason::Hash32LengthMismatch => {}
                StructuralReason::Hash28LengthMismatch => {}
                StructuralReason::EraTagOutOfRange => {}
            }
        }
    }
}
