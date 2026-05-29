// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE Mithril import binding predicate (PHASE4-N-Y S1, S7).
//!
//! CN-MITHRIL-01 / DC-MITHRIL-01: a Mithril-sourced seed may
//! bootstrap only after the Mithril certificate's attested
//! `{network_magic, genesis_hash, certified_point, certificate_hash}`
//! bind to the independently-minted `BootstrapAnchor`'s
//! `{network_magic, genesis_hash, seed_point, provenance}`; any
//! mismatch fails closed before storage init.
//!
//! The two sides come from genuinely different origins: the
//! `MithrilManifestReport` is what the Mithril cert attests (the
//! manifest-reported side), and the `BootstrapAnchor` is minted from
//! the operator's `--json-seed` + genesis extraction. The predicate
//! cross-checks them — never a value against itself.
//!
//! This predicate is **pure and total**: no I/O, no clock, no
//! `HashMap`, no float, no `String`/`anyhow` errors. The Mithril STM
//! multisig is verified by the RED mithril-client (acquisition
//! infra) — Ade never re-verifies it here and never treats the cert
//! signature as a BLUE trust root.

use ade_types::Hash32;

use super::anchor::{BootstrapAnchor, SeedPoint, SeedProvenance};

/// Closed manifest-reported side of the binding: exactly what the
/// Mithril certificate attests, as parsed by the RED mithril-import
/// shell. This is the ONLY side the report carries — there is no
/// duplicated/fabricated counterpart. The predicate cross-checks it
/// against the independently-minted `BootstrapAnchor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MithrilManifestReport {
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub certificate_hash: Hash32,
    pub certified_point: SeedPoint,
    /// Recorded as provenance only — the Mithril snapshot's
    /// immutable-file range has no independent counterpart on the
    /// `--json-seed` anchor, so it is NOT self-checked.
    pub immutable_range: (u64, u64),
}

/// Closed error sum for the Mithril binding predicate. All variants
/// are fail-fast and carry only non-secret primitives. Distinct
/// variant per checked field (CN-MITHRIL-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MithrilImportError {
    NetworkMagicMismatch,
    GenesisHashMismatch,
    CertifiedPointMismatch,
    CertificateHashMismatch,
    UnsupportedArtifactType,
}

/// Pure binding predicate. Returns `Ok(())` only when the Mithril
/// manifest report agrees with the independently-minted anchor on
/// every cross-checkable field.
///
/// Cross-checks (two independent origins — never a value vs itself):
/// - `report.network_magic == anchor.network_magic`
/// - `report.genesis_hash == anchor.genesis_hash`
/// - `report.certified_point == anchor.seed_point` (the load-bearing
///   check: the cert's attested point vs the point the seed was
///   extracted at)
/// - the anchor's provenance is `Mithril { certificate_hash, .. }`
///   with `certificate_hash == report.certificate_hash`
///
/// `UnsupportedArtifactType` is returned for any non-`Mithril`
/// anchor provenance — the predicate only binds Mithril-sourced
/// seeds. `report.immutable_range` is provenance-only (no check).
pub fn verify_mithril_binding(
    report: &MithrilManifestReport,
    anchor: &BootstrapAnchor,
) -> Result<(), MithrilImportError> {
    let anchor_certificate_hash = match &anchor.seed_provenance {
        SeedProvenance::Mithril {
            certificate_hash, ..
        } => certificate_hash,
        SeedProvenance::CardanoCliJson => {
            return Err(MithrilImportError::UnsupportedArtifactType)
        }
    };

    if report.network_magic != anchor.network_magic {
        return Err(MithrilImportError::NetworkMagicMismatch);
    }
    if report.genesis_hash != anchor.genesis_hash {
        return Err(MithrilImportError::GenesisHashMismatch);
    }
    if report.certified_point != anchor.seed_point {
        return Err(MithrilImportError::CertifiedPointMismatch);
    }
    if report.certificate_hash != *anchor_certificate_hash {
        return Err(MithrilImportError::CertificateHashMismatch);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_types::SlotNo;

    fn point() -> SeedPoint {
        SeedPoint {
            slot: SlotNo(23013663),
            block_hash: Hash32([0x22; 32]),
        }
    }

    /// A `--json-seed`-minted anchor: its `network_magic`,
    /// `genesis_hash`, `seed_point` and Mithril provenance come from
    /// the operator's seed extraction — an independent origin from
    /// the manifest report below.
    fn anchor() -> BootstrapAnchor {
        BootstrapAnchor {
            network_magic: 1,
            genesis_hash: Hash32([0x11; 32]),
            seed_point: point(),
            seed_artifact_hash: Hash32([0x33; 32]),
            imported_utxo_fingerprint: Hash32([0x44; 32]),
            initial_ledger_fingerprint: Hash32([0x55; 32]),
            seed_provenance: SeedProvenance::Mithril {
                certificate_hash: Hash32([0x66; 32]),
                certified_point: point(),
                immutable_range: (0, 4242),
            },
        }
    }

    /// The Mithril cert's attested side — agrees with `anchor()`.
    fn report() -> MithrilManifestReport {
        MithrilManifestReport {
            network_magic: 1,
            genesis_hash: Hash32([0x11; 32]),
            certificate_hash: Hash32([0x66; 32]),
            certified_point: point(),
            immutable_range: (0, 4242),
        }
    }

    #[test]
    fn mithril_anchor_binding_is_deterministic() {
        let a = verify_mithril_binding(&report(), &anchor());
        let b = verify_mithril_binding(&report(), &anchor());
        assert_eq!(a, b);
        assert_eq!(a, Ok(()));
    }

    /// The load-bearing cross-check the old tautological code could
    /// not express: the manifest is certified at a point ≠ the
    /// `--json-seed` anchor's `seed_point`. Under the old code both
    /// sides were populated from the SAME manifest value, so this
    /// could never fail. Now it fails closed.
    #[test]
    fn mithril_binding_rejects_certified_point_other_than_seed_point() {
        let mut r = report();
        r.certified_point = SeedPoint {
            // A point genuinely different from the anchor's seed_point.
            slot: SlotNo(99999999),
            block_hash: Hash32([0xAB; 32]),
        };
        assert_ne!(r.certified_point, anchor().seed_point);
        assert_eq!(
            verify_mithril_binding(&r, &anchor()),
            Err(MithrilImportError::CertifiedPointMismatch)
        );
    }

    #[test]
    fn mithril_anchor_rejects_field_mismatch() {
        // 1. Network magic differs between report and anchor.
        let mut r = report();
        r.network_magic = 2;
        assert_eq!(
            verify_mithril_binding(&r, &anchor()),
            Err(MithrilImportError::NetworkMagicMismatch)
        );

        // 2. Genesis hash differs.
        let mut r = report();
        r.genesis_hash = Hash32([0xEE; 32]);
        assert_eq!(
            verify_mithril_binding(&r, &anchor()),
            Err(MithrilImportError::GenesisHashMismatch)
        );

        // 3. Certified point differs from the anchor's seed_point.
        let mut r = report();
        r.certified_point = SeedPoint {
            slot: SlotNo(999),
            block_hash: Hash32([0x22; 32]),
        };
        assert_eq!(
            verify_mithril_binding(&r, &anchor()),
            Err(MithrilImportError::CertifiedPointMismatch)
        );

        // 4. Certificate hash differs from the anchor's provenance
        //    cert hash — its own distinct variant (the S1 WARN: this
        //    used to wrongly return CertifiedPointMismatch).
        let mut r = report();
        r.certificate_hash = Hash32([0xCD; 32]);
        assert_eq!(
            verify_mithril_binding(&r, &anchor()),
            Err(MithrilImportError::CertificateHashMismatch)
        );
    }

    #[test]
    fn non_mithril_provenance_is_unsupported() {
        let mut a = anchor();
        a.seed_provenance = SeedProvenance::CardanoCliJson;
        assert_eq!(
            verify_mithril_binding(&report(), &a),
            Err(MithrilImportError::UnsupportedArtifactType)
        );
    }
}
