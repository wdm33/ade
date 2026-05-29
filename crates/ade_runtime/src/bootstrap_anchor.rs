// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN BootstrapAnchor mint (PHASE4-N-M-A S2).
//!
//! Sole authority composing the import inputs (network, genesis,
//! seed point, seed artifact hash, imported UTxO fingerprint,
//! initial ledger fingerprint) into a canonical Ade
//! `BootstrapAnchor`. CN-ANCHOR-01.
//!
//! Doctrine: per memory [[feedback-oracle-seed-then-ade-owns]] the
//! anchor records the bootstrap input artifact provenance. After
//! mint, Ade owns the runtime representation — the anchor is the
//! single point of truth for "what we imported from the oracle".

use ade_ledger::bootstrap_anchor::{BootstrapAnchor, SeedPoint, SeedProvenance};
use ade_types::{Hash32, SlotNo};

use crate::seed_import::UtxoFingerprint;

/// Closed input bundle for [`mint`]. All fields required; no
/// `Default` impl; no `#[non_exhaustive]`. Construction failure
/// is a compile error, not a runtime error.
///
/// `seed_provenance` records how the seed was sourced (cardano-cli
/// JSON or a verified Mithril snapshot, PHASE4-N-Y S1). The caller
/// supplies it; the cardano-cli path passes
/// `SeedProvenance::CardanoCliJson`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintInputs {
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub seed_slot: SlotNo,
    pub seed_block_hash: Hash32,
    pub seed_artifact_hash: Hash32,
    pub imported_utxo_fingerprint: UtxoFingerprint,
    pub initial_ledger_fingerprint: Hash32,
    pub seed_provenance: SeedProvenance,
}

/// SOLE authority: mint a `BootstrapAnchor` from typed import
/// inputs. CN-ANCHOR-01.
pub fn mint(inputs: MintInputs) -> BootstrapAnchor {
    BootstrapAnchor {
        network_magic: inputs.network_magic,
        genesis_hash: inputs.genesis_hash,
        seed_point: SeedPoint {
            slot: inputs.seed_slot,
            block_hash: inputs.seed_block_hash,
        },
        seed_artifact_hash: inputs.seed_artifact_hash,
        imported_utxo_fingerprint: inputs.imported_utxo_fingerprint.0,
        initial_ledger_fingerprint: inputs.initial_ledger_fingerprint,
        seed_provenance: inputs.seed_provenance,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use ade_ledger::bootstrap_anchor::{decode_bootstrap_anchor, encode_bootstrap_anchor};

    fn sample_inputs() -> MintInputs {
        MintInputs {
            network_magic: 1,
            genesis_hash: Hash32([0x11; 32]),
            seed_slot: SlotNo(23013663),
            seed_block_hash: Hash32([0x22; 32]),
            seed_artifact_hash: Hash32([0x33; 32]),
            imported_utxo_fingerprint: UtxoFingerprint(Hash32([0x44; 32])),
            initial_ledger_fingerprint: Hash32([0x55; 32]),
            seed_provenance: SeedProvenance::CardanoCliJson,
        }
    }

    #[test]
    fn mint_composes_inputs_byte_identically() {
        let a = mint(sample_inputs());
        let b = mint(sample_inputs());
        assert_eq!(a, b);
        assert_eq!(encode_bootstrap_anchor(&a), encode_bootstrap_anchor(&b));
    }

    #[test]
    fn mint_then_round_trip_via_canonical_cbor() {
        let a = mint(sample_inputs());
        let bytes = encode_bootstrap_anchor(&a);
        let decoded = decode_bootstrap_anchor(&bytes).expect("decode");
        assert_eq!(decoded, a);
    }

    #[test]
    fn mint_carries_seed_point_correctly() {
        let inputs = sample_inputs();
        let a = mint(inputs.clone());
        assert_eq!(a.seed_point.slot, inputs.seed_slot);
        assert_eq!(a.seed_point.block_hash, inputs.seed_block_hash);
    }

    #[test]
    fn mint_propagates_utxo_fingerprint_into_anchor() {
        let inputs = sample_inputs();
        let a = mint(inputs.clone());
        assert_eq!(a.imported_utxo_fingerprint, inputs.imported_utxo_fingerprint.0);
    }
}
