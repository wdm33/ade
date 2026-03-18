use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

use super::{Era, HarnessError};

/// A permanent regression entry for a discrepant input.
///
/// Each entry captures a specific block or transaction that caused a divergence
/// between Ade and the Cardano reference implementation, along with the expected
/// verdict and fix status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionEntry {
    /// Unique corpus identifier (e.g., "REG-2B-001").
    pub corpus_id: String,
    /// Raw CBOR bytes of the discrepant input (hex-encoded for TOML/JSON).
    pub raw_bytes_hex: String,
    /// Source description (where this input came from).
    pub source: String,
    /// Era in which the discrepancy was observed.
    pub era: Era,
    /// Expected verdict (Accept or Reject with reason).
    pub expected_verdict: ExpectedVerdict,
    /// Git commit hash that fixed the discrepancy (empty if unfixed).
    pub fix_commit: String,
    /// Date the entry was added (YYYY-MM-DD).
    pub added_date: String,
    /// Description of what went wrong.
    pub description: String,
}

/// Whether a discrepant input should be accepted or rejected by the node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExpectedVerdict {
    Accept,
    Reject { reason: String },
}

/// A collection of regression entries forming the discrepant-input corpus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegressionCorpus {
    pub entries: Vec<RegressionEntry>,
}

/// A violation found during regression corpus validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorpusViolation {
    /// A required string field is empty.
    EmptyField { corpus_id: String, field: String },
    /// Duplicate corpus ID found.
    DuplicateId { corpus_id: String },
    /// raw_bytes_hex contains non-hex characters or has odd length.
    InvalidHex { corpus_id: String, detail: String },
}

impl fmt::Display for CorpusViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorpusViolation::EmptyField { corpus_id, field } => {
                write!(f, "'{corpus_id}': required field '{field}' is empty")
            }
            CorpusViolation::DuplicateId { corpus_id } => {
                write!(f, "duplicate corpus ID: '{corpus_id}'")
            }
            CorpusViolation::InvalidHex { corpus_id, detail } => {
                write!(f, "'{corpus_id}': invalid hex in raw_bytes_hex: {detail}")
            }
        }
    }
}

/// Parse a TOML string into a `RegressionCorpus`.
pub fn parse_regression_corpus(toml_content: &str) -> Result<RegressionCorpus, HarnessError> {
    toml::from_str(toml_content)
        .map_err(|e| HarnessError::ParseError(format!("regression corpus TOML parse error: {e}")))
}

/// Validate a `RegressionCorpus` for structural completeness.
///
/// Checks:
/// - Unique corpus IDs across all entries
/// - Non-empty required string fields (corpus_id, raw_bytes_hex, source, added_date, description)
/// - Valid hex encoding in raw_bytes_hex (even length, valid hex characters)
///
/// Returns a list of violations. An empty list means the corpus is valid.
pub fn validate_regression_corpus(corpus: &RegressionCorpus) -> Vec<CorpusViolation> {
    let mut violations = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for entry in &corpus.entries {
        let id = &entry.corpus_id;

        // Check for duplicate IDs
        if !seen_ids.insert(id.clone()) {
            violations.push(CorpusViolation::DuplicateId {
                corpus_id: id.clone(),
            });
        }

        // Check required string fields
        let string_fields: &[(&str, &str)] = &[
            ("corpus_id", &entry.corpus_id),
            ("raw_bytes_hex", &entry.raw_bytes_hex),
            ("source", &entry.source),
            ("added_date", &entry.added_date),
            ("description", &entry.description),
        ];

        for (field_name, value) in string_fields {
            if value.is_empty() {
                violations.push(CorpusViolation::EmptyField {
                    corpus_id: id.clone(),
                    field: field_name.to_string(),
                });
            }
        }

        // Validate hex encoding
        if !entry.raw_bytes_hex.is_empty() {
            if entry.raw_bytes_hex.len() % 2 != 0 {
                violations.push(CorpusViolation::InvalidHex {
                    corpus_id: id.clone(),
                    detail: "odd number of hex characters".to_string(),
                });
            } else if !entry.raw_bytes_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                violations.push(CorpusViolation::InvalidHex {
                    corpus_id: id.clone(),
                    detail: "contains non-hex characters".to_string(),
                });
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> RegressionEntry {
        RegressionEntry {
            corpus_id: "REG-2B-001".to_string(),
            raw_bytes_hex: "820082008200".to_string(),
            source: "mainnet block 4492800".to_string(),
            era: Era::Shelley,
            expected_verdict: ExpectedVerdict::Accept,
            fix_commit: String::new(),
            added_date: "2026-03-18".to_string(),
            description: "State hash mismatch on first Shelley block".to_string(),
        }
    }

    fn sample_corpus() -> RegressionCorpus {
        RegressionCorpus {
            entries: vec![sample_entry()],
        }
    }

    #[test]
    fn valid_corpus_no_violations() {
        let corpus = sample_corpus();
        let violations = validate_regression_corpus(&corpus);
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:?}"
        );
    }

    #[test]
    fn empty_corpus_no_violations() {
        let corpus = RegressionCorpus {
            entries: Vec::new(),
        };
        let violations = validate_regression_corpus(&corpus);
        assert!(violations.is_empty());
    }

    #[test]
    fn detects_duplicate_ids() {
        let corpus = RegressionCorpus {
            entries: vec![sample_entry(), sample_entry()],
        };
        let violations = validate_regression_corpus(&corpus);
        assert!(violations.iter().any(
            |v| matches!(v, CorpusViolation::DuplicateId { corpus_id } if corpus_id == "REG-2B-001")
        ));
    }

    #[test]
    fn detects_empty_corpus_id() {
        let mut entry = sample_entry();
        entry.corpus_id = String::new();
        let corpus = RegressionCorpus {
            entries: vec![entry],
        };
        let violations = validate_regression_corpus(&corpus);
        assert!(violations.iter().any(|v| matches!(
            v,
            CorpusViolation::EmptyField { field, .. } if field == "corpus_id"
        )));
    }

    #[test]
    fn detects_empty_raw_bytes_hex() {
        let mut entry = sample_entry();
        entry.raw_bytes_hex = String::new();
        let corpus = RegressionCorpus {
            entries: vec![entry],
        };
        let violations = validate_regression_corpus(&corpus);
        assert!(violations.iter().any(|v| matches!(
            v,
            CorpusViolation::EmptyField { field, .. } if field == "raw_bytes_hex"
        )));
    }

    #[test]
    fn detects_odd_hex_length() {
        let mut entry = sample_entry();
        entry.raw_bytes_hex = "abc".to_string();
        let corpus = RegressionCorpus {
            entries: vec![entry],
        };
        let violations = validate_regression_corpus(&corpus);
        assert!(violations.iter().any(|v| matches!(
            v,
            CorpusViolation::InvalidHex { detail, .. } if detail.contains("odd")
        )));
    }

    #[test]
    fn detects_non_hex_characters() {
        let mut entry = sample_entry();
        entry.raw_bytes_hex = "gg1122".to_string();
        let corpus = RegressionCorpus {
            entries: vec![entry],
        };
        let violations = validate_regression_corpus(&corpus);
        assert!(violations.iter().any(|v| matches!(
            v,
            CorpusViolation::InvalidHex { detail, .. } if detail.contains("non-hex")
        )));
    }

    #[test]
    fn reject_verdict_roundtrip() {
        let mut entry = sample_entry();
        entry.expected_verdict = ExpectedVerdict::Reject {
            reason: "invalid VRF proof".to_string(),
        };
        let corpus = RegressionCorpus {
            entries: vec![entry],
        };
        let serialized = toml::to_string(&corpus).unwrap();
        let reparsed: RegressionCorpus = toml::from_str(&serialized).unwrap();
        assert_eq!(corpus, reparsed);
    }

    #[test]
    fn toml_roundtrip() {
        let corpus = sample_corpus();
        let serialized = toml::to_string(&corpus).unwrap();
        let reparsed = parse_regression_corpus(&serialized).unwrap();
        assert_eq!(corpus, reparsed);
    }

    #[test]
    fn parse_invalid_toml() {
        let result = parse_regression_corpus("not valid toml [[[[");
        assert!(result.is_err());
        match result.unwrap_err() {
            HarnessError::ParseError(msg) => assert!(msg.contains("TOML")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn corpus_violation_display() {
        let v = CorpusViolation::EmptyField {
            corpus_id: "REG-2B-001".to_string(),
            field: "source".to_string(),
        };
        let s = format!("{v}");
        assert!(s.contains("REG-2B-001"));
        assert!(s.contains("source"));
        assert!(s.contains("empty"));

        let v2 = CorpusViolation::DuplicateId {
            corpus_id: "REG-2B-001".to_string(),
        };
        let s2 = format!("{v2}");
        assert!(s2.contains("duplicate"));
        assert!(s2.contains("REG-2B-001"));
    }

    #[test]
    fn fix_commit_empty_is_valid() {
        // fix_commit is optional — empty means unfixed
        let corpus = sample_corpus();
        assert!(corpus.entries[0].fix_commit.is_empty());
        let violations = validate_regression_corpus(&corpus);
        assert!(violations.is_empty());
    }
}
