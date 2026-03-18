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

/// Classification of where divergence occurred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceKind {
    Decoding,
    StateTransition,
    Hashing,
    Serialization,
    ProtocolSequencing,
}

/// Full first-divergence report with localization.
///
/// Contains all information needed to reproduce and diagnose a divergence
/// between the Ade ledger and the Cardano reference implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstDivergenceReport {
    /// Block number where divergence was first detected.
    pub block_number: u64,
    /// Slot number of the divergent block.
    pub slot: u64,
    /// Era of the divergent block.
    pub era: Era,
    /// Which comparison surface diverged.
    pub comparison_surface: String,
    /// Pre-state fingerprint (state hash before applying the block).
    pub pre_state_hash: StateHash,
    /// Post-state fingerprint from oracle.
    pub expected_post_state_hash: StateHash,
    /// Post-state fingerprint from Ade.
    pub actual_post_state_hash: StateHash,
    /// Classification of the divergence.
    pub divergence_kind: DivergenceKind,
    /// Path to the raw block CBOR that triggered divergence.
    pub block_cbor_path: String,
    /// Minimal reproduction instructions.
    pub repro_command: String,
}

/// Metadata for a block in a rich differential sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMeta {
    /// Era of this block.
    pub era: Era,
    /// Slot number.
    pub slot: u64,
    /// Block number (height).
    pub block_number: u64,
    /// Path to the CBOR file on disk.
    pub cbor_path: String,
    /// Raw CBOR bytes.
    pub cbor: Vec<u8>,
}

/// Full differential report from a rich ledger sequence comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialReport {
    /// Path to the oracle manifest used for this run.
    pub oracle_manifest_path: String,
    /// Total number of blocks in the input sequence.
    pub total_blocks: usize,
    /// Number of blocks actually compared before stopping.
    pub blocks_compared: usize,
    /// The first divergence found, if any.
    pub first_divergence: Option<FirstDivergenceReport>,
    /// True if all compared blocks matched the oracle reference.
    pub replay_equivalent: bool,
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

/// Apply blocks sequentially through a `LedgerApplicator` and compare
/// state hashes after each block against a reference sequence, producing
/// a rich `DifferentialReport` with full localization on divergence.
///
/// Unlike `diff_ledger_sequence`, this accepts `BlockMeta` entries carrying
/// era, slot, block number, and CBOR path metadata, enabling actionable
/// first-divergence reports.
pub fn diff_ledger_sequence_rich(
    applicator: &mut dyn LedgerApplicator,
    blocks: &[BlockMeta],
    reference: &LedgerHashSequence,
    oracle_manifest_path: &str,
) -> Result<DifferentialReport, HarnessError> {
    if blocks.len() != reference.state_hashes.len() {
        return Err(HarnessError::ValidationError(format!(
            "block count ({}) does not match reference hash count ({})",
            blocks.len(),
            reference.state_hashes.len()
        )));
    }

    let total_blocks = blocks.len();
    let mut pre_state_hash = StateHash([0u8; 32]);

    // Try to capture initial state hash; if the applicator supports it,
    // use it as the pre-state for the first block.
    if let Ok(h) = applicator.current_state_hash() {
        pre_state_hash = h;
    }

    for (i, block) in blocks.iter().enumerate() {
        applicator.apply_block(block.era, &block.cbor)?;
        let actual = applicator.current_state_hash()?;
        let expected = reference.state_hashes[i];

        if actual != expected {
            let repro_command = format!(
                "cargo test --package ade_testkit -- --exact replay_block_{}_{}_{}",
                block.era.as_str(),
                block.slot,
                block.block_number
            );

            return Ok(DifferentialReport {
                oracle_manifest_path: oracle_manifest_path.to_string(),
                total_blocks,
                blocks_compared: i + 1,
                first_divergence: Some(FirstDivergenceReport {
                    block_number: block.block_number,
                    slot: block.slot,
                    era: block.era,
                    comparison_surface: "ExtLedgerStateHash".to_string(),
                    pre_state_hash,
                    expected_post_state_hash: expected,
                    actual_post_state_hash: actual,
                    divergence_kind: DivergenceKind::StateTransition,
                    block_cbor_path: block.cbor_path.clone(),
                    repro_command,
                }),
                replay_equivalent: false,
            });
        }

        pre_state_hash = actual;
    }

    Ok(DifferentialReport {
        oracle_manifest_path: oracle_manifest_path.to_string(),
        total_blocks,
        blocks_compared: total_blocks,
        first_divergence: None,
        replay_equivalent: true,
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
        let blocks = vec![(Era::Shelley, vec![0u8; 10]), (Era::Shelley, vec![0u8; 10])];
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

    #[test]
    fn divergence_kind_roundtrip_json() {
        for kind in [
            DivergenceKind::Decoding,
            DivergenceKind::StateTransition,
            DivergenceKind::Hashing,
            DivergenceKind::Serialization,
            DivergenceKind::ProtocolSequencing,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let parsed: DivergenceKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, parsed);
        }
    }

    #[test]
    fn first_divergence_report_roundtrip_json() {
        let report = FirstDivergenceReport {
            block_number: 4490511,
            slot: 4492800,
            era: Era::Shelley,
            comparison_surface: "ExtLedgerStateHash".to_string(),
            pre_state_hash: StateHash([0xaa; 32]),
            expected_post_state_hash: StateHash([0xbb; 32]),
            actual_post_state_hash: StateHash([0xcc; 32]),
            divergence_kind: DivergenceKind::StateTransition,
            block_cbor_path: "shelley/block_4492800.cbor".to_string(),
            repro_command: "cargo test replay_shelley".to_string(),
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: FirstDivergenceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, parsed);
    }

    #[test]
    fn block_meta_roundtrip_json() {
        let meta = BlockMeta {
            era: Era::Allegra,
            slot: 16588800,
            block_number: 5000000,
            cbor_path: "allegra/block_5000000.cbor".to_string(),
            cbor: vec![0x82, 0x00, 0x01],
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: BlockMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta.era, parsed.era);
        assert_eq!(meta.slot, parsed.slot);
        assert_eq!(meta.block_number, parsed.block_number);
        assert_eq!(meta.cbor_path, parsed.cbor_path);
        assert_eq!(meta.cbor, parsed.cbor);
    }

    #[test]
    fn differential_report_matching_rich() {
        let hash_init = StateHash([0u8; 32]);
        let hash_a = StateHash([1u8; 32]);
        let hash_b = StateHash([2u8; 32]);

        struct RichFakeApplicator {
            // index 0 = initial state, index 1+ = post-block states
            hashes: Vec<StateHash>,
            index: usize,
        }
        impl LedgerApplicator for RichFakeApplicator {
            fn apply_block(&mut self, _era: Era, _cbor: &[u8]) -> Result<(), HarnessError> {
                self.index += 1;
                Ok(())
            }
            fn current_state_hash(&self) -> Result<StateHash, HarnessError> {
                Ok(self.hashes[self.index])
            }
            fn reset_to(&mut self, _hash: &StateHash) -> Result<(), HarnessError> {
                Ok(())
            }
        }

        let blocks = vec![
            BlockMeta {
                era: Era::Shelley,
                slot: 4492800,
                block_number: 4490511,
                cbor_path: "shelley/blk0.cbor".to_string(),
                cbor: vec![0u8; 10],
            },
            BlockMeta {
                era: Era::Shelley,
                slot: 4492820,
                block_number: 4490512,
                cbor_path: "shelley/blk1.cbor".to_string(),
                cbor: vec![0u8; 10],
            },
        ];

        let reference = LedgerHashSequence {
            era: Era::Shelley,
            state_hashes: vec![hash_a, hash_b],
        };

        let mut applicator = RichFakeApplicator {
            hashes: vec![hash_init, hash_a, hash_b],
            index: 0,
        };

        let report =
            diff_ledger_sequence_rich(&mut applicator, &blocks, &reference, "manifest.toml")
                .unwrap();

        assert!(report.replay_equivalent);
        assert!(report.first_divergence.is_none());
        assert_eq!(report.total_blocks, 2);
        assert_eq!(report.blocks_compared, 2);
        assert_eq!(report.oracle_manifest_path, "manifest.toml");
    }

    #[test]
    fn differential_report_divergent_rich() {
        let hash_init = StateHash([0u8; 32]);
        let hash_a = StateHash([1u8; 32]);
        let hash_b = StateHash([2u8; 32]);
        let hash_wrong = StateHash([9u8; 32]);

        struct RichFakeApplicator {
            hashes: Vec<StateHash>,
            index: usize,
        }
        impl LedgerApplicator for RichFakeApplicator {
            fn apply_block(&mut self, _era: Era, _cbor: &[u8]) -> Result<(), HarnessError> {
                self.index += 1;
                Ok(())
            }
            fn current_state_hash(&self) -> Result<StateHash, HarnessError> {
                Ok(self.hashes[self.index])
            }
            fn reset_to(&mut self, _hash: &StateHash) -> Result<(), HarnessError> {
                Ok(())
            }
        }

        let blocks = vec![
            BlockMeta {
                era: Era::Shelley,
                slot: 4492800,
                block_number: 4490511,
                cbor_path: "shelley/blk0.cbor".to_string(),
                cbor: vec![0u8; 10],
            },
            BlockMeta {
                era: Era::Shelley,
                slot: 4492820,
                block_number: 4490512,
                cbor_path: "shelley/blk1.cbor".to_string(),
                cbor: vec![0u8; 10],
            },
        ];

        let reference = LedgerHashSequence {
            era: Era::Shelley,
            state_hashes: vec![hash_a, hash_b],
        };

        let mut applicator = RichFakeApplicator {
            hashes: vec![hash_init, hash_a, hash_wrong],
            index: 0,
        };

        let report =
            diff_ledger_sequence_rich(&mut applicator, &blocks, &reference, "manifest.toml")
                .unwrap();

        assert!(!report.replay_equivalent);
        assert_eq!(report.blocks_compared, 2);
        let div = report.first_divergence.unwrap();
        assert_eq!(div.block_number, 4490512);
        assert_eq!(div.slot, 4492820);
        assert_eq!(div.era, Era::Shelley);
        assert_eq!(div.expected_post_state_hash, hash_b);
        assert_eq!(div.actual_post_state_hash, hash_wrong);
        assert_eq!(div.pre_state_hash, hash_a);
        assert_eq!(div.divergence_kind, DivergenceKind::StateTransition);
        assert_eq!(div.block_cbor_path, "shelley/blk1.cbor");
        assert!(div.repro_command.contains("replay_block"));
    }

    #[test]
    fn rich_mismatched_block_count_returns_error() {
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
        let blocks: Vec<BlockMeta> = vec![];
        let mut applicator = NoOpApplicator;

        let result = diff_ledger_sequence_rich(&mut applicator, &blocks, &reference, "m.toml");
        assert!(matches!(result, Err(HarnessError::ValidationError(_))));
    }

    #[test]
    fn differential_report_roundtrip_json() {
        let report = DifferentialReport {
            oracle_manifest_path: "manifest.toml".to_string(),
            total_blocks: 100,
            blocks_compared: 50,
            first_divergence: Some(FirstDivergenceReport {
                block_number: 4490511,
                slot: 4492800,
                era: Era::Shelley,
                comparison_surface: "ExtLedgerStateHash".to_string(),
                pre_state_hash: StateHash([0xaa; 32]),
                expected_post_state_hash: StateHash([0xbb; 32]),
                actual_post_state_hash: StateHash([0xcc; 32]),
                divergence_kind: DivergenceKind::Hashing,
                block_cbor_path: "shelley/blk.cbor".to_string(),
                repro_command: "cargo test replay".to_string(),
            }),
            replay_equivalent: false,
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: DifferentialReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report.oracle_manifest_path, parsed.oracle_manifest_path);
        assert_eq!(report.total_blocks, parsed.total_blocks);
        assert_eq!(report.blocks_compared, parsed.blocks_compared);
        assert_eq!(report.replay_equivalent, parsed.replay_equivalent);
        assert_eq!(report.first_divergence, parsed.first_divergence);
    }
}
