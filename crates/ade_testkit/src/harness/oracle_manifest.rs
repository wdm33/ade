use serde::{Deserialize, Serialize};
use std::fmt;

use super::{Era, HarnessError};

/// Structured oracle provenance metadata that MUST accompany every differential run.
///
/// This captures the exact tool versions, data sources, and scope used to produce
/// reference oracle data, ensuring full reproducibility of differential comparisons.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OracleManifest {
    /// cardano-node version used to produce oracle data.
    pub cardano_node_version: String,
    /// ouroboros-consensus / cardano-ledger version.
    pub consensus_version: String,
    /// Extractor tool name.
    pub extractor_tool: String,
    /// Git commit hash of the extractor tool.
    pub extractor_commit: String,
    /// ImmutableDB snapshot identity (Mithril epoch + hash).
    pub db_snapshot_id: String,
    /// Start slot of the block sequence (inclusive).
    pub start_slot: u64,
    /// End slot of the block sequence (inclusive).
    pub end_slot: u64,
    /// Era scope — which eras are covered.
    pub era_scope: Vec<Era>,
    /// Comparison surface: what is being compared.
    pub comparison_surface: ComparisonSurface,
    /// SHA-256 digests of output artifacts.
    pub artifact_digests: Vec<ArtifactDigest>,
    /// Extraction date (YYYY-MM-DD).
    pub extraction_date: String,
}

/// What ledger surface is being compared in a differential run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComparisonSurface {
    ExtLedgerStateHash,
    UTxOSetHash,
    DelegationMapHash,
    RewardAccountsHash,
    ProtocolParametersHash,
    Custom(String),
}

/// A SHA-256 digest paired with the artifact path it describes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactDigest {
    /// Path to the artifact (relative to corpus root).
    pub artifact_path: String,
    /// SHA-256 hex digest of the artifact contents.
    pub sha256: String,
}

/// A violation found during oracle manifest validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestViolation {
    /// A required string field is empty.
    EmptyField { field: String },
    /// start_slot > end_slot.
    InvalidSlotRange { start: u64, end: u64 },
    /// era_scope is empty.
    EmptyEraScope,
    /// No artifact digests provided.
    NoArtifactDigests,
    /// An artifact digest has an empty field.
    ArtifactEmptyField { index: usize, field: String },
}

impl fmt::Display for ManifestViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestViolation::EmptyField { field } => {
                write!(f, "required field '{field}' is empty")
            }
            ManifestViolation::InvalidSlotRange { start, end } => {
                write!(f, "start_slot ({start}) > end_slot ({end})")
            }
            ManifestViolation::EmptyEraScope => write!(f, "era_scope must not be empty"),
            ManifestViolation::NoArtifactDigests => {
                write!(f, "at least one artifact digest is required")
            }
            ManifestViolation::ArtifactEmptyField { index, field } => {
                write!(f, "artifact_digests[{index}]: field '{field}' is empty")
            }
        }
    }
}

/// Validate an `OracleManifest` for structural completeness.
///
/// Returns a list of violations. An empty list means the manifest is valid.
pub fn validate_oracle_manifest(manifest: &OracleManifest) -> Vec<ManifestViolation> {
    let mut violations = Vec::new();

    let string_fields: &[(&str, &str)] = &[
        ("cardano_node_version", &manifest.cardano_node_version),
        ("consensus_version", &manifest.consensus_version),
        ("extractor_tool", &manifest.extractor_tool),
        ("extractor_commit", &manifest.extractor_commit),
        ("db_snapshot_id", &manifest.db_snapshot_id),
        ("extraction_date", &manifest.extraction_date),
    ];

    for (field_name, value) in string_fields {
        if value.is_empty() {
            violations.push(ManifestViolation::EmptyField {
                field: field_name.to_string(),
            });
        }
    }

    if manifest.start_slot > manifest.end_slot {
        violations.push(ManifestViolation::InvalidSlotRange {
            start: manifest.start_slot,
            end: manifest.end_slot,
        });
    }

    if manifest.era_scope.is_empty() {
        violations.push(ManifestViolation::EmptyEraScope);
    }

    if manifest.artifact_digests.is_empty() {
        violations.push(ManifestViolation::NoArtifactDigests);
    }

    for (i, digest) in manifest.artifact_digests.iter().enumerate() {
        if digest.artifact_path.is_empty() {
            violations.push(ManifestViolation::ArtifactEmptyField {
                index: i,
                field: "artifact_path".to_string(),
            });
        }
        if digest.sha256.is_empty() {
            violations.push(ManifestViolation::ArtifactEmptyField {
                index: i,
                field: "sha256".to_string(),
            });
        }
    }

    violations
}

/// Parse a TOML string into an `OracleManifest`.
pub fn parse_oracle_manifest(toml_content: &str) -> Result<OracleManifest, HarnessError> {
    toml::from_str(toml_content)
        .map_err(|e| HarnessError::ParseError(format!("oracle manifest TOML parse error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> OracleManifest {
        OracleManifest {
            cardano_node_version: "10.6.2".to_string(),
            consensus_version: "0.20.0.0".to_string(),
            extractor_tool: "ade-extractor".to_string(),
            extractor_commit: "abc123def456".to_string(),
            db_snapshot_id: "epoch-530-deadbeef".to_string(),
            start_slot: 4492800,
            end_slot: 16588799,
            era_scope: vec![Era::Shelley, Era::Allegra],
            comparison_surface: ComparisonSurface::ExtLedgerStateHash,
            artifact_digests: vec![ArtifactDigest {
                artifact_path: "shelley/state_hashes.toml".to_string(),
                sha256: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                    .to_string(),
            }],
            extraction_date: "2026-03-18".to_string(),
        }
    }

    #[test]
    fn valid_manifest_no_violations() {
        let manifest = sample_manifest();
        let violations = validate_oracle_manifest(&manifest);
        assert!(
            violations.is_empty(),
            "unexpected violations: {violations:?}"
        );
    }

    #[test]
    fn detects_empty_cardano_node_version() {
        let mut manifest = sample_manifest();
        manifest.cardano_node_version = String::new();
        let violations = validate_oracle_manifest(&manifest);
        assert!(violations.iter().any(|v| matches!(
            v,
            ManifestViolation::EmptyField { field } if field == "cardano_node_version"
        )));
    }

    #[test]
    fn detects_invalid_slot_range() {
        let mut manifest = sample_manifest();
        manifest.start_slot = 100;
        manifest.end_slot = 50;
        let violations = validate_oracle_manifest(&manifest);
        assert!(violations
            .iter()
            .any(|v| matches!(v, ManifestViolation::InvalidSlotRange { .. })));
    }

    #[test]
    fn equal_slots_valid() {
        let mut manifest = sample_manifest();
        manifest.start_slot = 100;
        manifest.end_slot = 100;
        let violations = validate_oracle_manifest(&manifest);
        assert!(
            violations.is_empty(),
            "start == end should be valid: {violations:?}"
        );
    }

    #[test]
    fn detects_empty_era_scope() {
        let mut manifest = sample_manifest();
        manifest.era_scope = Vec::new();
        let violations = validate_oracle_manifest(&manifest);
        assert!(violations
            .iter()
            .any(|v| matches!(v, ManifestViolation::EmptyEraScope)));
    }

    #[test]
    fn detects_no_artifact_digests() {
        let mut manifest = sample_manifest();
        manifest.artifact_digests = Vec::new();
        let violations = validate_oracle_manifest(&manifest);
        assert!(violations
            .iter()
            .any(|v| matches!(v, ManifestViolation::NoArtifactDigests)));
    }

    #[test]
    fn detects_artifact_empty_path() {
        let mut manifest = sample_manifest();
        manifest.artifact_digests[0].artifact_path = String::new();
        let violations = validate_oracle_manifest(&manifest);
        assert!(violations.iter().any(|v| matches!(
            v,
            ManifestViolation::ArtifactEmptyField { index: 0, field } if field == "artifact_path"
        )));
    }

    #[test]
    fn detects_artifact_empty_sha256() {
        let mut manifest = sample_manifest();
        manifest.artifact_digests[0].sha256 = String::new();
        let violations = validate_oracle_manifest(&manifest);
        assert!(violations.iter().any(|v| matches!(
            v,
            ManifestViolation::ArtifactEmptyField { index: 0, field } if field == "sha256"
        )));
    }

    #[test]
    fn multiple_violations_collected() {
        let mut manifest = sample_manifest();
        manifest.cardano_node_version = String::new();
        manifest.consensus_version = String::new();
        manifest.start_slot = 999;
        manifest.end_slot = 1;
        manifest.era_scope = Vec::new();
        let violations = validate_oracle_manifest(&manifest);
        // At least: 2 empty fields + slot range + empty era scope
        assert!(
            violations.len() >= 4,
            "expected >= 4 violations: {violations:?}"
        );
    }

    #[test]
    fn toml_roundtrip() {
        let manifest = sample_manifest();
        let serialized = toml::to_string(&manifest).unwrap();
        let reparsed: OracleManifest = toml::from_str(&serialized).unwrap();
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn parse_oracle_manifest_valid() {
        let manifest = sample_manifest();
        let serialized = toml::to_string(&manifest).unwrap();
        let parsed = parse_oracle_manifest(&serialized).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn parse_oracle_manifest_invalid_toml() {
        let result = parse_oracle_manifest("not valid toml [[[[");
        assert!(result.is_err());
        match result.unwrap_err() {
            HarnessError::ParseError(msg) => assert!(msg.contains("TOML")),
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn comparison_surface_custom_variant() {
        let manifest = OracleManifest {
            comparison_surface: ComparisonSurface::Custom("my-custom-surface".to_string()),
            ..sample_manifest()
        };
        let serialized = toml::to_string(&manifest).unwrap();
        let reparsed: OracleManifest = toml::from_str(&serialized).unwrap();
        assert_eq!(manifest.comparison_surface, reparsed.comparison_surface);
    }

    #[test]
    fn manifest_violation_display() {
        let v = ManifestViolation::EmptyField {
            field: "cardano_node_version".to_string(),
        };
        let s = format!("{v}");
        assert!(s.contains("cardano_node_version"));
        assert!(s.contains("empty"));

        let v2 = ManifestViolation::InvalidSlotRange {
            start: 100,
            end: 50,
        };
        let s2 = format!("{v2}");
        assert!(s2.contains("100"));
        assert!(s2.contains("50"));
    }
}
