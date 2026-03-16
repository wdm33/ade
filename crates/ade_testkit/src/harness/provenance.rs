use serde::{Deserialize, Serialize};
use std::fmt;

use super::HarnessError;

/// A single entry in a DC-REF-01 provenance manifest.
///
/// All fields are mandatory per DC-REF-01. String fields must be non-empty.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestEntry {
    /// Path to the reference artifact relative to the manifest directory.
    pub file: String,
    /// Path to the source block/data in the golden corpus.
    pub source_block: String,
    /// Cardano era name (lowercase).
    pub era: String,
    /// Artifact type (e.g., "block_fields", "state_hash", "transcript").
    #[serde(rename = "type")]
    pub entry_type: String,
    /// Name of the extraction tool (e.g., "cardano-cli").
    pub extraction_tool: String,
    /// Version of the extraction tool.
    pub extraction_tool_version: String,
    /// Git revision of the extraction tool.
    pub extraction_tool_git_rev: String,
    /// Cardano node version used for extraction.
    pub cardano_node_version: String,
    /// Network magic number (764824073 for mainnet).
    pub network_magic: u64,
    /// Protocol version at the time of extraction.
    pub protocol_version: String,
    /// Method used for extraction (e.g., "cardano-cli debug decode block").
    pub extraction_method: String,
    /// Date of extraction (YYYY-MM-DD).
    pub extraction_date: String,
    /// Source data type (e.g., "ImmutableDB", "LedgerDB").
    pub source_type: String,
    /// Reproducibility instructions.
    pub reproducibility: String,
    /// SHA-256 checksum of the reference artifact file.
    pub sha256: String,
}

/// A DC-REF-01 provenance manifest containing entries for reference artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    #[serde(default)]
    pub entries: Vec<ManifestEntry>,
}

/// A violation of DC-REF-01 provenance requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceViolation {
    /// A required string field is empty.
    EmptyField { entry_file: String, field: String },
    /// SHA-256 checksum does not match file content.
    ChecksumMismatch {
        entry_file: String,
        expected: String,
        actual: String,
    },
    /// A file exists in the reference directory but has no manifest entry.
    UntrackedFile { file: String },
}

impl fmt::Display for ProvenanceViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvenanceViolation::EmptyField { entry_file, field } => {
                write!(f, "empty field '{field}' in entry for '{entry_file}'")
            }
            ProvenanceViolation::ChecksumMismatch {
                entry_file,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "checksum mismatch for '{entry_file}': expected {expected}, got {actual}"
                )
            }
            ProvenanceViolation::UntrackedFile { file } => {
                write!(f, "untracked file not in manifest: '{file}'")
            }
        }
    }
}

/// Parse a TOML string into a `Manifest`.
pub fn parse_manifest(toml_content: &str) -> Result<Manifest, HarnessError> {
    toml::from_str(toml_content)
        .map_err(|e| HarnessError::ParseError(format!("manifest TOML parse error: {e}")))
}

/// Validate that all DC-REF-01 required fields are non-empty.
///
/// Returns a list of violations. An empty list means the manifest is valid.
/// This validates structural completeness only — SHA-256 checksum verification
/// against actual files is performed by `ci/ci_check_ref_provenance.sh`.
pub fn validate_manifest(manifest: &Manifest) -> Vec<ProvenanceViolation> {
    let mut violations = Vec::new();

    for entry in &manifest.entries {
        let file = &entry.file;

        let string_fields: &[(&str, &str)] = &[
            ("file", &entry.file),
            ("source_block", &entry.source_block),
            ("era", &entry.era),
            ("type", &entry.entry_type),
            ("extraction_tool", &entry.extraction_tool),
            ("extraction_tool_version", &entry.extraction_tool_version),
            ("extraction_tool_git_rev", &entry.extraction_tool_git_rev),
            ("cardano_node_version", &entry.cardano_node_version),
            ("protocol_version", &entry.protocol_version),
            ("extraction_method", &entry.extraction_method),
            ("extraction_date", &entry.extraction_date),
            ("source_type", &entry.source_type),
            ("reproducibility", &entry.reproducibility),
            ("sha256", &entry.sha256),
        ];

        for (field_name, value) in string_fields {
            if value.is_empty() {
                violations.push(ProvenanceViolation::EmptyField {
                    entry_file: file.clone(),
                    field: field_name.to_string(),
                });
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_toml() -> &'static str {
        r#"
[[entries]]
file = "byron/chunk00000_blk00000.json"
source_block = "corpus/golden/byron/blocks/chunk00000_blk00000.cbor"
era = "byron"
type = "block_fields"
extraction_tool = "cardano-cli"
extraction_tool_version = "10.6.2.0"
extraction_tool_git_rev = "0d697f14"
cardano_node_version = "10.6.2"
network_magic = 764824073
protocol_version = "10.0"
extraction_method = "cardano-cli debug decode block"
extraction_date = "2026-03-15"
source_type = "ImmutableDB"
reproducibility = "Run cardano-cli debug decode block on golden CBOR"
sha256 = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
"#
    }

    #[test]
    fn parse_valid_manifest() {
        let manifest = parse_manifest(sample_manifest_toml()).unwrap();
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].era, "byron");
        assert_eq!(manifest.entries[0].network_magic, 764824073);
    }

    #[test]
    fn parse_empty_manifest() {
        let manifest = parse_manifest("").unwrap();
        assert!(manifest.entries.is_empty());
    }

    #[test]
    fn validate_complete_manifest_no_violations() {
        let manifest = parse_manifest(sample_manifest_toml()).unwrap();
        let violations = validate_manifest(&manifest);
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:?}"
        );
    }

    #[test]
    fn validate_empty_manifest_no_violations() {
        let manifest = Manifest {
            entries: Vec::new(),
        };
        let violations = validate_manifest(&manifest);
        assert!(violations.is_empty());
    }

    #[test]
    fn validate_detects_empty_field() {
        let manifest = Manifest {
            entries: vec![ManifestEntry {
                file: "test.json".to_string(),
                source_block: "".to_string(), // empty — violation
                era: "byron".to_string(),
                entry_type: "block_fields".to_string(),
                extraction_tool: "cardano-cli".to_string(),
                extraction_tool_version: "10.6.2.0".to_string(),
                extraction_tool_git_rev: "0d697f14".to_string(),
                cardano_node_version: "10.6.2".to_string(),
                network_magic: 764824073,
                protocol_version: "10.0".to_string(),
                extraction_method: "cardano-cli debug decode block".to_string(),
                extraction_date: "2026-03-15".to_string(),
                source_type: "ImmutableDB".to_string(),
                reproducibility: "test".to_string(),
                sha256: "abc123".to_string(),
            }],
        };
        let violations = validate_manifest(&manifest);
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0],
            ProvenanceViolation::EmptyField {
                entry_file: "test.json".to_string(),
                field: "source_block".to_string(),
            }
        );
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let result = parse_manifest("this is not valid toml [[[[");
        assert!(result.is_err());
        match result.unwrap_err() {
            HarnessError::ParseError(msg) => assert!(msg.contains("TOML")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn provenance_violation_display() {
        let v = ProvenanceViolation::EmptyField {
            entry_file: "test.json".to_string(),
            field: "era".to_string(),
        };
        let s = format!("{v}");
        assert!(s.contains("empty field"));
        assert!(s.contains("era"));
        assert!(s.contains("test.json"));
    }

    #[test]
    fn manifest_roundtrip_toml() {
        let manifest = parse_manifest(sample_manifest_toml()).unwrap();
        let serialized = toml::to_string(&manifest).unwrap();
        let reparsed = parse_manifest(&serialized).unwrap();
        assert_eq!(manifest, reparsed);
    }
}
