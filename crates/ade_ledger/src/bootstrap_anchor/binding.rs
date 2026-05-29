// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE Mithril import binding predicate (PHASE4-N-Y S1).
//!
//! CN-MITHRIL-01 / DC-MITHRIL-01: a Mithril-sourced seed may
//! bootstrap only after the certified point + immutable range +
//! genesis hash + network magic bind to the anchor and the
//! recomputed `seed_artifact_hash` matches; any mismatch fails
//! closed before storage init.
//!
//! This predicate is **pure and total**: no I/O, no clock, no
//! `HashMap`, no float, no `String`/`anyhow` errors. The Mithril STM
//! multisig is verified by the RED mithril-client (acquisition
//! infra) — Ade never re-verifies it here and never treats the cert
//! signature as a BLUE trust root. BLUE only binds the
//! mithril-client-reported content to the anchor field-set and
//! recomputes the seed artifact hash.

use ade_types::Hash32;

use super::anchor::{SeedPoint, SeedProvenance};

/// Closed observed field-set the binding predicate checks the
/// `Mithril` provenance against. These are the values BLUE derives
/// independently (anchor field-set + recompute) — the predicate
/// proves the provenance the anchor carries agrees with them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MithrilAnchorFields {
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub certified_point: SeedPoint,
    pub immutable_range: (u64, u64),
    /// The RED mithril-client also reports these for content-binding;
    /// they must equal the values recorded in the anchor provenance.
    pub reported_certificate_hash: Hash32,
    pub reported_network_magic: u32,
    pub reported_genesis_hash: Hash32,
    pub reported_certified_point: SeedPoint,
    pub reported_immutable_range: (u64, u64),
}

/// Closed error sum for the Mithril binding predicate. All variants
/// are fail-fast and carry only non-secret primitives. Distinct
/// variant per checked field (CN-MITHRIL-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MithrilImportError {
    NetworkMagicMismatch,
    GenesisHashMismatch,
    CertifiedPointMismatch,
    ImmutableRangeMismatch,
    SeedArtifactHashMismatch,
    UnsupportedArtifactType,
}

/// Pure binding predicate. Returns `Ok(())` only when the Mithril
/// provenance recorded in the anchor binds to every observed anchor
/// field and the recomputed seed artifact hash matches.
///
/// `recomputed_seed_artifact_hash` is the Blake2b-256 digest BLUE
/// recomputes over the imported seed artifact; it must equal the
/// anchor's `seed_artifact_hash` (passed as `anchor_seed_artifact_hash`).
///
/// `UnsupportedArtifactType` is returned for any non-`Mithril`
/// provenance — the predicate only binds Mithril-sourced seeds.
pub fn verify_mithril_binding(
    provenance: &SeedProvenance,
    fields: &MithrilAnchorFields,
    anchor_seed_artifact_hash: &Hash32,
    recomputed_seed_artifact_hash: &Hash32,
) -> Result<(), MithrilImportError> {
    let (certificate_hash, certified_point, immutable_range) = match provenance {
        SeedProvenance::Mithril {
            certificate_hash,
            certified_point,
            immutable_range,
        } => (certificate_hash, certified_point, immutable_range),
        SeedProvenance::CardanoCliJson => {
            return Err(MithrilImportError::UnsupportedArtifactType)
        }
    };

    if *certificate_hash != fields.reported_certificate_hash {
        return Err(MithrilImportError::CertifiedPointMismatch);
    }
    if fields.network_magic != fields.reported_network_magic {
        return Err(MithrilImportError::NetworkMagicMismatch);
    }
    if fields.genesis_hash != fields.reported_genesis_hash {
        return Err(MithrilImportError::GenesisHashMismatch);
    }
    if *certified_point != fields.certified_point
        || fields.certified_point != fields.reported_certified_point
    {
        return Err(MithrilImportError::CertifiedPointMismatch);
    }
    if *immutable_range != fields.immutable_range
        || fields.immutable_range != fields.reported_immutable_range
    {
        return Err(MithrilImportError::ImmutableRangeMismatch);
    }
    if anchor_seed_artifact_hash != recomputed_seed_artifact_hash {
        return Err(MithrilImportError::SeedArtifactHashMismatch);
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

    fn provenance() -> SeedProvenance {
        SeedProvenance::Mithril {
            certificate_hash: Hash32([0x66; 32]),
            certified_point: point(),
            immutable_range: (0, 4242),
        }
    }

    fn fields() -> MithrilAnchorFields {
        MithrilAnchorFields {
            network_magic: 1,
            genesis_hash: Hash32([0x11; 32]),
            certified_point: point(),
            immutable_range: (0, 4242),
            reported_certificate_hash: Hash32([0x66; 32]),
            reported_network_magic: 1,
            reported_genesis_hash: Hash32([0x11; 32]),
            reported_certified_point: point(),
            reported_immutable_range: (0, 4242),
        }
    }

    fn artifact_hash() -> Hash32 {
        Hash32([0x33; 32])
    }

    #[test]
    fn mithril_anchor_binding_is_deterministic() {
        let a = verify_mithril_binding(&provenance(), &fields(), &artifact_hash(), &artifact_hash());
        let b = verify_mithril_binding(&provenance(), &fields(), &artifact_hash(), &artifact_hash());
        assert_eq!(a, b);
        assert_eq!(a, Ok(()));
    }

    #[test]
    fn mithril_anchor_rejects_field_mismatch() {
        // 1. Network magic flipped.
        let mut f = fields();
        f.reported_network_magic = 2;
        assert_eq!(
            verify_mithril_binding(&provenance(), &f, &artifact_hash(), &artifact_hash()),
            Err(MithrilImportError::NetworkMagicMismatch)
        );

        // 2. Genesis hash flipped.
        let mut f = fields();
        f.reported_genesis_hash = Hash32([0xEE; 32]);
        assert_eq!(
            verify_mithril_binding(&provenance(), &f, &artifact_hash(), &artifact_hash()),
            Err(MithrilImportError::GenesisHashMismatch)
        );

        // 3. Certified point flipped.
        let mut f = fields();
        f.reported_certified_point = SeedPoint {
            slot: SlotNo(999),
            block_hash: Hash32([0x22; 32]),
        };
        assert_eq!(
            verify_mithril_binding(&provenance(), &f, &artifact_hash(), &artifact_hash()),
            Err(MithrilImportError::CertifiedPointMismatch)
        );

        // 4. Immutable range flipped.
        let mut f = fields();
        f.reported_immutable_range = (0, 9999);
        assert_eq!(
            verify_mithril_binding(&provenance(), &f, &artifact_hash(), &artifact_hash()),
            Err(MithrilImportError::ImmutableRangeMismatch)
        );

        // 5. Seed artifact hash flipped (recompute disagrees).
        assert_eq!(
            verify_mithril_binding(&provenance(), &fields(), &artifact_hash(), &Hash32([0x99; 32])),
            Err(MithrilImportError::SeedArtifactHashMismatch)
        );
    }

    #[test]
    fn non_mithril_provenance_is_unsupported() {
        assert_eq!(
            verify_mithril_binding(
                &SeedProvenance::CardanoCliJson,
                &fields(),
                &artifact_hash(),
                &artifact_hash()
            ),
            Err(MithrilImportError::UnsupportedArtifactType)
        );
    }
}
