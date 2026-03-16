use serde::{Deserialize, Serialize};
use std::fmt;

use super::{Era, HarnessError};

/// Opaque 32-byte state hash for ledger state comparison.
///
/// Represents a hash of the ledger state at a block boundary.
/// This is version-scoped oracle evidence — valid only against
/// the pinned cardano-node version used for extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StateHash(pub [u8; 32]);

impl StateHash {
    /// Create a StateHash from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self, HarnessError> {
        let hex = hex.strip_prefix("0x").unwrap_or(hex);
        if hex.len() != 64 {
            return Err(HarnessError::ParseError(format!(
                "state hash hex must be 64 characters, got {}",
                hex.len()
            )));
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let s = std::str::from_utf8(chunk)
                .map_err(|e| HarnessError::ParseError(format!("invalid hex: {e}")))?;
            bytes[i] = u8::from_str_radix(s, 16)
                .map_err(|e| HarnessError::ParseError(format!("invalid hex byte: {e}")))?;
        }
        Ok(StateHash(bytes))
    }

    /// Encode as lowercase hex string.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl fmt::Display for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Trait for applying blocks to a ledger state and computing state hashes.
///
/// Implementations will use real ledger rules. The harness uses this trait
/// to obtain project-side state hashes for comparison against reference
/// oracle data.
pub trait LedgerApplicator {
    fn apply_block(&mut self, era: Era, cbor: &[u8]) -> Result<(), HarnessError>;
    fn current_state_hash(&self) -> Result<StateHash, HarnessError>;
    fn reset_to(&mut self, state_hash: &StateHash) -> Result<(), HarnessError>;
}

/// Stub applicator that returns `NotYetImplemented` for all operations.
pub struct StubLedgerApplicator;

impl LedgerApplicator for StubLedgerApplicator {
    fn apply_block(&mut self, era: Era, _cbor: &[u8]) -> Result<(), HarnessError> {
        Err(HarnessError::NotYetImplemented(format!(
            "ledger block application for {era} not yet implemented"
        )))
    }

    fn current_state_hash(&self) -> Result<StateHash, HarnessError> {
        Err(HarnessError::NotYetImplemented(
            "ledger state hash not yet implemented".to_string(),
        ))
    }

    fn reset_to(&mut self, _state_hash: &StateHash) -> Result<(), HarnessError> {
        Err(HarnessError::NotYetImplemented(
            "ledger reset not yet implemented".to_string(),
        ))
    }
}

/// A point of divergence in a ledger state hash sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerDivergence {
    /// Block index within the sequence where divergence was detected.
    pub block_index: usize,
    /// Expected state hash from reference oracle.
    pub expected: StateHash,
    /// Actual state hash produced by project code.
    pub actual: StateHash,
}

/// Report from comparing a sequence of ledger state hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerDiffReport {
    /// The first divergence found, if any.
    pub first_divergence: Option<LedgerDivergence>,
}

impl LedgerDiffReport {
    /// Returns true if no divergence was found.
    pub fn is_match(&self) -> bool {
        self.first_divergence.is_none()
    }
}

/// A parsed reference hash sequence from oracle extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerHashSequence {
    pub era: Era,
    pub state_hashes: Vec<StateHash>,
}

/// Apply blocks sequentially through a `LedgerApplicator` and compare
/// state hashes after each block against a reference sequence.
///
/// Reports the first divergence found. Stops at the first mismatch.
pub fn diff_ledger_sequence(
    applicator: &mut dyn LedgerApplicator,
    blocks: &[(Era, Vec<u8>)],
    reference: &LedgerHashSequence,
) -> Result<LedgerDiffReport, HarnessError> {
    if blocks.len() != reference.state_hashes.len() {
        return Err(HarnessError::ValidationError(format!(
            "block count ({}) does not match reference hash count ({})",
            blocks.len(),
            reference.state_hashes.len()
        )));
    }

    for (i, (era, cbor)) in blocks.iter().enumerate() {
        applicator.apply_block(*era, cbor)?;
        let actual = applicator.current_state_hash()?;
        let expected = reference.state_hashes[i];

        if actual != expected {
            return Ok(LedgerDiffReport {
                first_divergence: Some(LedgerDivergence {
                    block_index: i,
                    expected,
                    actual,
                }),
            });
        }
    }

    Ok(LedgerDiffReport {
        first_divergence: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_hash_from_hex_valid() {
        let hex = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let hash = StateHash::from_hex(hex).unwrap();
        assert_eq!(hash.to_hex(), hex);
    }

    #[test]
    fn state_hash_from_hex_with_prefix() {
        let hex = "0xabcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let hash = StateHash::from_hex(hex).unwrap();
        assert_eq!(hash.0[0], 0xab);
    }

    #[test]
    fn state_hash_from_hex_wrong_length() {
        let result = StateHash::from_hex("abcd");
        assert!(result.is_err());
    }

    #[test]
    fn state_hash_display() {
        let hash = StateHash([0u8; 32]);
        assert_eq!(
            format!("{hash}"),
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn state_hash_ordering() {
        let a = StateHash([0u8; 32]);
        let mut b_bytes = [0u8; 32];
        b_bytes[31] = 1;
        let b = StateHash(b_bytes);
        assert!(a < b);
    }

    #[test]
    fn stub_applicator_apply_block() {
        let mut stub = StubLedgerApplicator;
        let result = stub.apply_block(Era::Byron, &[]);
        match result {
            Err(HarnessError::NotYetImplemented(msg)) => {
                assert!(msg.contains("byron"));
            }
            other => panic!("expected NotYetImplemented, got {other:?}"),
        }
    }

    #[test]
    fn stub_applicator_current_state_hash() {
        let stub = StubLedgerApplicator;
        let result = stub.current_state_hash();
        assert!(matches!(result, Err(HarnessError::NotYetImplemented(_))));
    }

    #[test]
    fn stub_applicator_reset_to() {
        let mut stub = StubLedgerApplicator;
        let hash = StateHash([0u8; 32]);
        let result = stub.reset_to(&hash);
        assert!(matches!(result, Err(HarnessError::NotYetImplemented(_))));
    }

    #[test]
    fn self_comparison_ledger_sequence() {
        let hash_a = StateHash([1u8; 32]);
        let hash_b = StateHash([2u8; 32]);

        struct FakeApplicator {
            hashes: Vec<StateHash>,
            index: usize,
        }
        impl LedgerApplicator for FakeApplicator {
            fn apply_block(&mut self, _era: Era, _cbor: &[u8]) -> Result<(), HarnessError> {
                self.index += 1;
                Ok(())
            }
            fn current_state_hash(&self) -> Result<StateHash, HarnessError> {
                Ok(self.hashes[self.index - 1])
            }
            fn reset_to(&mut self, _hash: &StateHash) -> Result<(), HarnessError> {
                Ok(())
            }
        }

        let reference = LedgerHashSequence {
            era: Era::Shelley,
            state_hashes: vec![hash_a, hash_b],
        };
        let blocks = vec![
            (Era::Shelley, vec![0u8; 10]),
            (Era::Shelley, vec![0u8; 10]),
        ];
        let mut applicator = FakeApplicator {
            hashes: vec![hash_a, hash_b],
            index: 0,
        };

        let report = diff_ledger_sequence(&mut applicator, &blocks, &reference).unwrap();
        assert!(report.is_match(), "expected matching sequence");
    }

    #[test]
    fn ledger_diff_report_roundtrip_json() {
        let report = LedgerDiffReport {
            first_divergence: Some(LedgerDivergence {
                block_index: 5,
                expected: StateHash([0xaa; 32]),
                actual: StateHash([0xbb; 32]),
            }),
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: LedgerDiffReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, parsed);
    }

    #[test]
    fn mismatched_block_count_returns_error() {
        struct NoOpApplicator;
        impl LedgerApplicator for NoOpApplicator {
            fn apply_block(&mut self, _: Era, _: &[u8]) -> Result<(), HarnessError> {
                Ok(())
            }
            fn current_state_hash(&self) -> Result<StateHash, HarnessError> {
                Ok(StateHash([0; 32]))
            }
            fn reset_to(&mut self, _: &StateHash) -> Result<(), HarnessError> {
                Ok(())
            }
        }

        let reference = LedgerHashSequence {
            era: Era::Byron,
            state_hashes: vec![StateHash([0; 32])],
        };
        let blocks = vec![];
        let mut applicator = NoOpApplicator;

        let result = diff_ledger_sequence(&mut applicator, &blocks, &reference);
        assert!(matches!(result, Err(HarnessError::ValidationError(_))));
    }
}
