// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3f-2-pre (DC-EVIEW-09) — the bootstrap manifest that binds the
//! seed and the cert-state import as ONE package.
//!
//! The seed (`SeedEpochConsensusInputs`) is the compact per-POOL active epoch consensus
//! view; the cert state (`CertState` = `DelegationState` + `PoolState`) is the
//! per-CREDENTIAL ledger continuation state needed to derive LATER epoch views. They are
//! DIFFERENT authority surfaces and stay SEPARATELY TYPED — the seed record is NOT
//! widened. They are bound only through this canonical [`BootstrapManifest`], which
//! carries the network + era, the exact source chain point, the source/checkpoint
//! commitment, and the canonical-bytes HASH of EACH artifact.
//!
//! The cert-state artifact is the COMPLETE canonical `CertState` produced by the
//! existing `encode_cert_state` codec (reused verbatim — never hand-reconstructed loose
//! delegation/reward maps), so it carries the registration/lifecycle facts the codec
//! requires, not just `delegations + rewards`.
//!
//! FAIL-CLOSED (non-negotiable), BEFORE any bootstrap state becomes durable: a seed
//! whose bytes do not hash to the manifest's `seed_hash`, a cert state whose bytes do
//! not hash to `cert_state_hash`, a network/era that does not match, or a malformed
//! manifest / cert state, are all rejected. A seed without its manifest-bound cert state
//! (or vice versa) cannot pass — verification requires BOTH artifacts and the manifest.

use ade_core::consensus::events::Point;
use ade_crypto::blake2b::blake2b_256;
use ade_types::{CardanoEra, Hash32};

use crate::delegation::CertState;
use crate::snapshot::cert_state::decode_cert_state;

const MANIFEST_DOMAIN: u8 = 0xB0; // version/domain tag for the canonical encoding (v1, 142 bytes)
const MANIFEST_DOMAIN_V2: u8 = 0xB1; // v2 (174 bytes): adds the bound UTxO seed hash

/// Binds the seed + cert-state artifacts of one bootstrap into a single canonical,
/// self-describing package. Every field is a binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapManifest {
    pub network_magic: u32,
    pub era: CardanoEra,
    /// The exact source chain point (slot + block hash) the package was captured at.
    pub source_point: Point,
    /// `blake2b_256` of the canonical seed (`SeedEpochConsensusInputs`) bytes.
    pub seed_hash: Hash32,
    /// `blake2b_256` of the canonical cert-state (`encode_cert_state`) bytes.
    pub cert_state_hash: Hash32,
    /// The source/checkpoint commitment (e.g. the oracle ledger-state / anchor hash).
    pub source_commitment: Hash32,
    /// `blake2b_256` of the canonical UTxO/ledger seed (`query utxo --whole-utxo` JSON), bound in
    /// manifest v2 (BOOTSTRAP-ANCHOR-PACKAGE). `None` for v1 manifests (the ECA recovery path stays
    /// byte-identical). Some => the package is admission-ready (the seed sibling is bound).
    pub utxo_seed_hash: Option<Hash32>,
}

/// Why a bootstrap package is rejected. Every variant is fail-closed BEFORE durability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapManifestError {
    /// The manifest bytes are not a well-formed canonical manifest.
    MalformedManifest,
    /// The seed bytes do not hash to the manifest's `seed_hash`.
    SeedHashMismatch,
    /// The cert-state bytes do not hash to the manifest's `cert_state_hash`.
    CertStateHashMismatch,
    /// The manifest's network does not match the bootstrap's expected network.
    NetworkMismatch,
    /// The manifest's era does not match the bootstrap's expected era.
    EraMismatch,
    /// The (hash-verified) cert-state bytes do not decode as a canonical `CertState`.
    CertStateDecode,
}

impl BootstrapManifest {
    /// The canonical, deterministic byte encoding (fixed field order + width). Injective.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(1 + 4 + 1 + 8 + 32 * 5);
        // v2 (domain 0xB1) when the UTxO seed is bound; v1 (0xB0) otherwise -- the v1 encoding is
        // byte-identical to the original 142-byte manifest (the ECA recovery path is unchanged).
        b.push(if self.utxo_seed_hash.is_some() {
            MANIFEST_DOMAIN_V2
        } else {
            MANIFEST_DOMAIN
        });
        b.extend_from_slice(&self.network_magic.to_be_bytes());
        b.push(self.era as u8);
        b.extend_from_slice(&self.source_point.slot.0.to_be_bytes());
        b.extend_from_slice(&self.source_point.hash.0);
        b.extend_from_slice(&self.seed_hash.0);
        b.extend_from_slice(&self.cert_state_hash.0);
        b.extend_from_slice(&self.source_commitment.0);
        if let Some(h) = &self.utxo_seed_hash {
            b.extend_from_slice(&h.0);
        }
        b
    }

    /// Decode a canonical manifest. Fail-closed on a wrong domain tag, wrong length, or
    /// an unknown era tag.
    pub fn decode(bytes: &[u8]) -> Result<BootstrapManifest, BootstrapManifestError> {
        // v1 (0xB0): 1+4+1+8+32*4 = 142. v2 (0xB1): +32 (utxo_seed_hash) = 174.
        let has_utxo = match bytes.first() {
            Some(&MANIFEST_DOMAIN) => false,
            Some(&MANIFEST_DOMAIN_V2) => true,
            _ => return Err(BootstrapManifestError::MalformedManifest),
        };
        if bytes.len() != if has_utxo { 174 } else { 142 } {
            return Err(BootstrapManifestError::MalformedManifest);
        }
        let network_magic = u32::from_be_bytes(bytes[1..5].try_into().unwrap());
        let era = era_from_tag(bytes[5]).ok_or(BootstrapManifestError::MalformedManifest)?;
        let slot = u64::from_be_bytes(bytes[6..14].try_into().unwrap());
        let h = |off: usize| {
            let mut a = [0u8; 32];
            a.copy_from_slice(&bytes[off..off + 32]);
            Hash32(a)
        };
        Ok(BootstrapManifest {
            network_magic,
            era,
            source_point: Point { slot: ade_types::primitives::SlotNo(slot), hash: h(14) },
            seed_hash: h(46),
            cert_state_hash: h(78),
            source_commitment: h(110),
            utxo_seed_hash: if has_utxo { Some(h(142)) } else { None },
        })
    }
}

fn era_from_tag(tag: u8) -> Option<CardanoEra> {
    CardanoEra::ALL.into_iter().find(|e| *e as u8 == tag)
}

/// Verify a bootstrap package and import the cert state. THE fail-closed gate:
/// decode the manifest; require it match the expected network + era; require the seed
/// and cert-state bytes to hash to the manifest's committed hashes; then decode the
/// (now hash-bound) cert state with the existing canonical codec. Returns the complete
/// `CertState` to seed the bootstrap `LedgerState`, or a fail-closed error -- NEVER a
/// partially-trusted or empty cert state on a binding failure.
pub fn verify_and_import_cert_state(
    manifest_bytes: &[u8],
    seed_bytes: &[u8],
    cert_state_bytes: &[u8],
    expected_network: u32,
    expected_era: CardanoEra,
) -> Result<(BootstrapManifest, CertState), BootstrapManifestError> {
    let manifest = BootstrapManifest::decode(manifest_bytes)?;
    if manifest.network_magic != expected_network {
        return Err(BootstrapManifestError::NetworkMismatch);
    }
    if manifest.era != expected_era {
        return Err(BootstrapManifestError::EraMismatch);
    }
    if blake2b_256(seed_bytes) != manifest.seed_hash {
        return Err(BootstrapManifestError::SeedHashMismatch);
    }
    if blake2b_256(cert_state_bytes) != manifest.cert_state_hash {
        return Err(BootstrapManifestError::CertStateHashMismatch);
    }
    let cert_state =
        decode_cert_state(cert_state_bytes).map_err(|_| BootstrapManifestError::CertStateDecode)?;
    Ok((manifest, cert_state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::cert_state::encode_cert_state;
    use ade_types::primitives::SlotNo;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId};
    use ade_types::Hash28;

    const NET: u32 = 2;

    fn sample_cert_state() -> CertState {
        let mut cs = CertState::new();
        cs.delegation
            .delegations
            .insert(StakeCredential::KeyHash(Hash28([0x11; 28])), PoolId(Hash28([0x22; 28])));
        cs.delegation
            .rewards
            .insert(StakeCredential::KeyHash(Hash28([0x11; 28])), Coin(500_000));
        cs
    }

    fn manifest_for(seed: &[u8], cert: &[u8]) -> BootstrapManifest {
        BootstrapManifest {
            network_magic: NET,
            era: CardanoEra::Conway,
            source_point: Point { slot: SlotNo(115_000_000), hash: Hash32([0xaa; 32]) },
            seed_hash: blake2b_256(seed),
            cert_state_hash: blake2b_256(cert),
            source_commitment: Hash32([0xcc; 32]),
            utxo_seed_hash: None,
        }
    }

    #[test]
    fn v2_manifest_binds_utxo_seed_hash_backward_compatibly() {
        let mut m = manifest_for(b"seed", b"cert");
        // v1 (no UTxO seed): 142 bytes, byte-identical to the original manifest.
        let v1 = m.canonical_bytes();
        assert_eq!(v1.len(), 142);
        assert_eq!(BootstrapManifest::decode(&v1).unwrap(), m);
        // v2 (UTxO seed bound): 174 bytes, decodes back with the bound hash.
        m.utxo_seed_hash = Some(Hash32([0x5e; 32]));
        let v2 = m.canonical_bytes();
        assert_eq!(v2.len(), 174);
        let d = BootstrapManifest::decode(&v2).unwrap();
        assert_eq!(d, m);
        assert_eq!(d.utxo_seed_hash, Some(Hash32([0x5e; 32])));
    }

    #[test]
    fn manifest_round_trips_canonically() {
        let m = manifest_for(b"seed-bytes", b"cert-bytes");
        assert_eq!(BootstrapManifest::decode(&m.canonical_bytes()).unwrap(), m);
    }

    #[test]
    fn verify_and_import_happy_path() {
        let seed = b"the-canonical-seed-bytes";
        let cert_bytes = encode_cert_state(&sample_cert_state());
        let m = manifest_for(seed, &cert_bytes);
        let (manifest, cs) =
            verify_and_import_cert_state(&m.canonical_bytes(), seed, &cert_bytes, NET, CardanoEra::Conway)
                .expect("happy path");
        assert_eq!(manifest, m);
        assert_eq!(cs, sample_cert_state(), "the COMPLETE cert state is imported via the codec");
    }

    #[test]
    fn seed_hash_mismatch_fails_closed() {
        let cert_bytes = encode_cert_state(&sample_cert_state());
        let m = manifest_for(b"the-real-seed", &cert_bytes);
        // a DIFFERENT seed than the manifest commits to.
        let r = verify_and_import_cert_state(&m.canonical_bytes(), b"a-tampered-seed", &cert_bytes, NET, CardanoEra::Conway);
        assert_eq!(r, Err(BootstrapManifestError::SeedHashMismatch));
    }

    #[test]
    fn cert_state_hash_mismatch_fails_closed() {
        let seed = b"the-seed";
        let cert_bytes = encode_cert_state(&sample_cert_state());
        let m = manifest_for(seed, &cert_bytes);
        // a DIFFERENT cert state than the manifest commits to.
        let other = encode_cert_state(&CertState::new());
        let r = verify_and_import_cert_state(&m.canonical_bytes(), seed, &other, NET, CardanoEra::Conway);
        assert_eq!(r, Err(BootstrapManifestError::CertStateHashMismatch));
    }

    #[test]
    fn network_and_era_mismatch_fail_closed() {
        let seed = b"s";
        let cert = encode_cert_state(&sample_cert_state());
        let m = manifest_for(seed, &cert);
        assert_eq!(
            verify_and_import_cert_state(&m.canonical_bytes(), seed, &cert, 1, CardanoEra::Conway),
            Err(BootstrapManifestError::NetworkMismatch)
        );
        assert_eq!(
            verify_and_import_cert_state(&m.canonical_bytes(), seed, &cert, NET, CardanoEra::Babbage),
            Err(BootstrapManifestError::EraMismatch)
        );
    }

    #[test]
    fn malformed_manifest_fails_closed() {
        assert_eq!(BootstrapManifest::decode(&[]), Err(BootstrapManifestError::MalformedManifest));
        assert_eq!(BootstrapManifest::decode(&[0x00; 142]), Err(BootstrapManifestError::MalformedManifest)); // wrong domain
        let mut short = manifest_for(b"s", b"c").canonical_bytes();
        short.pop();
        assert_eq!(BootstrapManifest::decode(&short), Err(BootstrapManifestError::MalformedManifest));
    }

    #[test]
    fn malformed_cert_state_fails_closed_after_hash_ok() {
        // a cert-state blob that the manifest commits to (hash matches) but does NOT
        // decode -> CertStateDecode (fail-closed, never an empty cert state).
        let seed = b"s";
        let bad_cert = b"not-a-canonical-cert-state";
        let m = manifest_for(seed, bad_cert);
        let r = verify_and_import_cert_state(&m.canonical_bytes(), seed, bad_cert, NET, CardanoEra::Conway);
        assert_eq!(r, Err(BootstrapManifestError::CertStateDecode));
    }
}
