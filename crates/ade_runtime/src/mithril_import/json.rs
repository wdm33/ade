// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED mithril-client manifest deserializer (PHASE4-N-Y S1).
//!
//! Pure structural deserialization of the operator-supplied manifest
//! describing a **mithril-client-verified** snapshot + certificate
//! into typed serde structs. The mithril-client (documented
//! acquisition infra) performs STM multisig verification; this
//! manifest mirrors how `consensus_inputs`/`seed_import` parse
//! operator-supplied JSON. Validation (hash widths, range
//! consistency) lives in `importer.rs`; this module only ensures the
//! JSON parses as the declared shape.
//!
//! The manifest is operator-produced (they run the mithril-client and
//! record its verified output). The expected shape is fixed; there is
//! no partial-import fallback.

use serde::Deserialize;

/// The mithril-client-verified snapshot manifest. All fields are
/// mandatory; `deny_unknown_fields` rejects operator-introduced
/// typos before the typed-import layer is reached.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawMithrilManifest {
    /// Artifact type the mithril-client produced. The importer
    /// accepts only the snapshot artifact; anything else fails
    /// closed (`UnsupportedArtifactType`).
    pub artifact_type: String,
    /// Hex-encoded 32-byte Mithril certificate hash (the cert the
    /// mithril-client verified). Recorded as closed provenance only —
    /// never re-verified as a BLUE trust root.
    pub certificate_hash_hex: String,
    /// Cardano network magic the snapshot pertains to.
    pub network_magic: u32,
    /// Hex-encoded 32-byte genesis hash.
    pub genesis_hash_hex: String,
    /// The certified chain point the certificate attests.
    pub certified_point: RawCertifiedPoint,
    /// Immutable-file range `[lo, hi]` the snapshot covers.
    pub immutable_range: RawImmutableRange,
    /// Source mithril-client version string.
    pub source_mithril_client_version: String,
    /// Aggregator / command that produced the manifest.
    pub source_command: String,
}

/// The certified chain point: slot + 32-byte block hash hex.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawCertifiedPoint {
    pub slot: u64,
    pub block_hash_hex: String,
}

/// Inclusive immutable-file range `[lo, hi]`.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawImmutableRange {
    pub lo: u64,
    pub hi: u64,
}

/// SOLE pub fn converting manifest JSON bytes into the structural
/// intermediate. Downstream `importer::parse_mithril_manifest`
/// consumes the result.
pub fn parse_mithril_manifest_json(bytes: &[u8]) -> Result<RawMithrilManifest, serde_json::Error> {
    serde_json::from_slice(bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    pub(crate) const MINIMAL: &str = r#"{
        "artifact_type": "cardano-database-snapshot",
        "certificate_hash_hex": "6666666666666666666666666666666666666666666666666666666666666666",
        "network_magic": 1,
        "genesis_hash_hex": "1111111111111111111111111111111111111111111111111111111111111111",
        "certified_point": {
            "slot": 23013663,
            "block_hash_hex": "2222222222222222222222222222222222222222222222222222222222222222"
        },
        "immutable_range": { "lo": 0, "hi": 4242 },
        "source_mithril_client_version": "mithril-client 0.10.0",
        "source_command": "mithril-client cardano-db download latest"
    }"#;

    #[test]
    fn minimal_manifest_parses() {
        let raw = parse_mithril_manifest_json(MINIMAL.as_bytes()).expect("parse ok");
        assert_eq!(raw.artifact_type, "cardano-database-snapshot");
        assert_eq!(raw.network_magic, 1);
        assert_eq!(raw.certified_point.slot, 23013663);
        assert_eq!(raw.immutable_range, RawImmutableRange { lo: 0, hi: 4242 });
    }

    #[test]
    fn missing_required_field_is_error() {
        let bad = MINIMAL.replace(r#""network_magic": 1,"#, "");
        let err = parse_mithril_manifest_json(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("network_magic"));
    }

    #[test]
    fn unknown_field_is_rejected() {
        let bad = MINIMAL.replace(
            r#""source_command": "mithril-client cardano-db download latest""#,
            r#""source_command": "mithril-client cardano-db download latest", "extra": 1"#,
        );
        let err = parse_mithril_manifest_json(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("extra"));
    }
}
