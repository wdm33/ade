// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED mithril-client import shell (PHASE4-N-Y S1).
//!
//! Parses an operator-supplied manifest describing a
//! **mithril-client-verified** snapshot + certificate into the
//! closed provenance fields {certificate_hash, certified_point,
//! immutable_range, genesis_hash, network_magic}.
//!
//! Boundary discipline ([[feedback-mithril-is-peer-infra-not-ade-authority]]):
//! - The mithril-client (documented acquisition infra) verifies the
//!   STM multisig. This shell performs **no semantic decision** and
//!   **never** re-verifies (or imports any STM-verify crate). It only
//!   moves the verified output into typed provenance fields.
//! - The authoritative accept/reject decision is the BLUE
//!   `verify_mithril_binding` predicate; this shell never decides
//!   whether the binding holds.
//!
//! A real mithril-client subprocess invocation is out of scope for
//! this slice — the operator records the client's verified output in
//! the manifest, mirroring how `consensus_inputs`/`seed_import`
//! consume operator-supplied JSON.

use std::fs;
use std::io;
use std::path::Path;

use ade_ledger::bootstrap_anchor::{MithrilManifestReport, SeedPoint, SeedProvenance};
use ade_types::{Hash32, SlotNo};

use super::json::{parse_mithril_manifest_json, RawMithrilManifest};

/// The closed artifact type the mithril-client snapshot manifest must
/// declare. Any other value fails closed (the importer does not
/// decide *binding*, but it does refuse a manifest it cannot map to a
/// snapshot provenance).
const SNAPSHOT_ARTIFACT_TYPE: &str = "cardano-database-snapshot";

/// Closed error sum for the mithril-import shell. Carries only
/// non-secret primitives. RED-side parse/structure errors only — the
/// BLUE binding verdict is a separate `MithrilImportError`.
#[derive(Debug)]
pub enum MithrilManifestError {
    /// IO failure reading the manifest file.
    Io(io::ErrorKind),
    /// JSON parse failure.
    Json(serde_json::Error),
    /// A hex hash field had wrong length or non-hex characters.
    BadHashHex { field: &'static str },
    /// The manifest declared an artifact type this shell does not map
    /// to a snapshot provenance.
    UnsupportedArtifactType,
    /// The immutable range was inverted (`lo > hi`).
    BadImmutableRange,
}

impl From<serde_json::Error> for MithrilManifestError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// The shell's output: the closed `SeedProvenance::Mithril` to record
/// in the anchor, plus the `MithrilManifestReport` (the Mithril cert's
/// attested side) the BLUE predicate cross-checks against the
/// independently-minted anchor. No semantic decision is made here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MithrilProvenanceImport {
    pub provenance: SeedProvenance,
    pub report: MithrilManifestReport,
}

/// Read + parse a mithril-client manifest file into the provenance
/// fields. File-IO variant.
pub fn import_mithril_manifest(
    path: &Path,
) -> Result<MithrilProvenanceImport, MithrilManifestError> {
    let bytes = fs::read(path).map_err(|e| MithrilManifestError::Io(e.kind()))?;
    import_mithril_manifest_from_bytes(&bytes)
}

/// In-memory variant: parse manifest JSON bytes into the provenance
/// fields. SOLE structural mapping from the verified manifest to the
/// closed provenance + observed anchor field-set.
pub fn import_mithril_manifest_from_bytes(
    bytes: &[u8],
) -> Result<MithrilProvenanceImport, MithrilManifestError> {
    let raw: RawMithrilManifest = parse_mithril_manifest_json(bytes)?;

    if raw.artifact_type != SNAPSHOT_ARTIFACT_TYPE {
        return Err(MithrilManifestError::UnsupportedArtifactType);
    }
    if raw.immutable_range.lo > raw.immutable_range.hi {
        return Err(MithrilManifestError::BadImmutableRange);
    }

    let certificate_hash = parse_hash32(&raw.certificate_hash_hex, "certificate_hash_hex")?;
    let genesis_hash = parse_hash32(&raw.genesis_hash_hex, "genesis_hash_hex")?;
    let block_hash = parse_hash32(&raw.certified_point.block_hash_hex, "certified_point.block_hash_hex")?;
    let certified_point = SeedPoint {
        slot: SlotNo(raw.certified_point.slot),
        block_hash,
    };
    let immutable_range = (raw.immutable_range.lo, raw.immutable_range.hi);

    let provenance = SeedProvenance::Mithril {
        certificate_hash: certificate_hash.clone(),
        certified_point: certified_point.clone(),
        immutable_range,
    };
    let report = MithrilManifestReport {
        network_magic: raw.network_magic,
        genesis_hash,
        certificate_hash,
        certified_point,
        immutable_range,
    };

    Ok(MithrilProvenanceImport { provenance, report })
}

fn parse_hash32(hex: &str, field: &'static str) -> Result<Hash32, MithrilManifestError> {
    if hex.len() != 64 {
        return Err(MithrilManifestError::BadHashHex { field });
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let pair = &hex[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(pair, 16)
            .map_err(|_| MithrilManifestError::BadHashHex { field })?;
    }
    Ok(Hash32(bytes))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"{
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
    fn manifest_maps_to_provenance_and_fields() {
        let out = import_mithril_manifest_from_bytes(MINIMAL.as_bytes()).expect("import");
        match &out.provenance {
            SeedProvenance::Mithril {
                certificate_hash,
                certified_point,
                immutable_range,
            } => {
                assert_eq!(*certificate_hash, Hash32([0x66; 32]));
                assert_eq!(certified_point.slot, SlotNo(23013663));
                assert_eq!(*immutable_range, (0, 4242));
            }
            other => panic!("expected Mithril provenance, got {other:?}"),
        }
        assert_eq!(out.report.network_magic, 1);
        assert_eq!(out.report.genesis_hash, Hash32([0x11; 32]));
        assert_eq!(out.report.certificate_hash, Hash32([0x66; 32]));
    }

    #[test]
    fn unsupported_artifact_type_fails_closed() {
        let bad = MINIMAL.replace(
            r#""artifact_type": "cardano-database-snapshot""#,
            r#""artifact_type": "mithril-stake-distribution""#,
        );
        match import_mithril_manifest_from_bytes(bad.as_bytes()) {
            Err(MithrilManifestError::UnsupportedArtifactType) => {}
            other => panic!("expected UnsupportedArtifactType, got {other:?}"),
        }
    }

    #[test]
    fn bad_hash_hex_fails_closed() {
        let bad = MINIMAL.replace(
            r#""certificate_hash_hex": "6666666666666666666666666666666666666666666666666666666666666666""#,
            r#""certificate_hash_hex": "zz""#,
        );
        match import_mithril_manifest_from_bytes(bad.as_bytes()) {
            Err(MithrilManifestError::BadHashHex { field: "certificate_hash_hex" }) => {}
            other => panic!("expected BadHashHex, got {other:?}"),
        }
    }

    #[test]
    fn inverted_immutable_range_fails_closed() {
        let bad = MINIMAL.replace(r#""immutable_range": { "lo": 0, "hi": 4242 }"#,
            r#""immutable_range": { "lo": 5000, "hi": 4242 }"#);
        match import_mithril_manifest_from_bytes(bad.as_bytes()) {
            Err(MithrilManifestError::BadImmutableRange) => {}
            other => panic!("expected BadImmutableRange, got {other:?}"),
        }
    }

    #[test]
    fn mithril_import_fail_closed_blocks_storage_init() {
        use ade_ledger::bootstrap_anchor::{
            verify_mithril_binding, BootstrapAnchor, MithrilImportError,
        };
        use crate::chaindb::InMemoryChainDb;
        use crate::chaindb::SnapshotStore;

        let import = import_mithril_manifest_from_bytes(MINIMAL.as_bytes()).expect("import");

        // The independently-minted anchor (from the operator's
        // --json-seed extraction) carries its OWN seed_point and
        // Mithril provenance. Here it was certified at a DIFFERENT
        // point than the manifest report attests — a mismatched
        // binding from two genuinely independent origins.
        let mismatched_point = SeedPoint {
            slot: SlotNo(99999999),
            block_hash: Hash32([0xAB; 32]),
        };
        let anchor_mismatch = BootstrapAnchor {
            network_magic: 1,
            genesis_hash: Hash32([0x11; 32]),
            seed_point: mismatched_point.clone(),
            seed_artifact_hash: Hash32([0x33; 32]),
            imported_utxo_fingerprint: Hash32([0x44; 32]),
            initial_ledger_fingerprint: Hash32([0x55; 32]),
            seed_provenance: SeedProvenance::Mithril {
                certificate_hash: Hash32([0x66; 32]),
                certified_point: mismatched_point,
                immutable_range: (0, 4242),
            },
        };

        let verdict = verify_mithril_binding(&import.report, &anchor_mismatch);
        assert_eq!(verdict, Err(MithrilImportError::CertifiedPointMismatch));

        // Storage must NOT initialize on a failed binding. We model
        // "storage init" as a put_snapshot; the gate only runs it on
        // an Ok verdict. After a fail-closed verdict the store stays
        // empty — no partial state.
        let store = InMemoryChainDb::new();
        if verdict.is_ok() {
            store.put_snapshot(SlotNo(0), b"initial-state").expect("put");
        }
        assert!(
            store.list_snapshot_slots().expect("list").is_empty(),
            "storage must not initialize on a mismatched Mithril binding"
        );

        // The positive control: an anchor minted at the same point
        // the manifest attests (still an independent origin) binds,
        // and only then does storage initialize.
        let anchor_match = BootstrapAnchor {
            seed_point: import.report.certified_point.clone(),
            seed_provenance: SeedProvenance::Mithril {
                certificate_hash: import.report.certificate_hash.clone(),
                certified_point: import.report.certified_point.clone(),
                immutable_range: import.report.immutable_range,
            },
            ..anchor_mismatch
        };
        let ok_verdict = verify_mithril_binding(&import.report, &anchor_match);
        assert_eq!(ok_verdict, Ok(()));
        let store_ok = InMemoryChainDb::new();
        if ok_verdict.is_ok() {
            store_ok.put_snapshot(SlotNo(0), b"initial-state").expect("put");
        }
        assert_eq!(store_ok.list_snapshot_slots().expect("list").len(), 1);
    }
}
