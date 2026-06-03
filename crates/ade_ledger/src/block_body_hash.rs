// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Canonical body-hash authority. The only function in the workspace
//! that computes the Cardano block body-hash recipe; both
//! `block_validity::header_input` (validator recomputation) and
//! `producer::forge::forge_block` (producer emission) call this.

use ade_crypto::blake2b_256;
use ade_types::shelley::block::ShelleyBlock;
use ade_types::Hash32;

/// Compute the Cardano block body hash from the four preserved CBOR
/// byte buckets. For Alonzo+ (Conway included) the recipe is:
///
/// ```text
///   body_hash = blake2b_256(
///       blake2b_256(tx_bodies)
///    || blake2b_256(witness_sets)
///    || blake2b_256(metadata)
///    || blake2b_256(invalid_txs OR empty bytes)
///   )
/// ```
///
/// Each input slice is the PRESERVED CBOR for the corresponding bucket,
/// including its outer array/map header (as carried in
/// `ShelleyBlock.{tx_bodies, witness_sets, metadata, invalid_txs}`).
/// `invalid_txs == None` is hashed as the empty byte string.
pub fn block_body_hash_from_buckets(
    tx_bodies: &[u8],
    witness_sets: &[u8],
    metadata: &[u8],
    invalid_txs: Option<&[u8]>,
) -> Hash32 {
    let h_tx = blake2b_256(tx_bodies).0;
    let h_ws = blake2b_256(witness_sets).0;
    let h_md = blake2b_256(metadata).0;
    let h_iv = blake2b_256(invalid_txs.unwrap_or(&[])).0;
    let mut concat = [0u8; 128];
    concat[0..32].copy_from_slice(&h_tx);
    concat[32..64].copy_from_slice(&h_ws);
    concat[64..96].copy_from_slice(&h_md);
    concat[96..128].copy_from_slice(&h_iv);
    Hash32(blake2b_256(&concat).0)
}

/// Compute the body hash of a `ShelleyBlock` value. Thin wrapper over
/// `block_body_hash_from_buckets` that destructures the block's four
/// bucket fields.
pub fn block_body_hash(block: &ShelleyBlock) -> Hash32 {
    block_body_hash_from_buckets(
        &block.tx_bodies,
        &block.witness_sets,
        &block.metadata,
        block.invalid_txs.as_deref(),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::shelley::block::{
        OperationalCert, ProtocolVersion, ShelleyHeader, ShelleyHeaderBody, VrfData,
    };

    fn fixture_block(invalid_txs: Option<Vec<u8>>) -> ShelleyBlock {
        ShelleyBlock {
            header: ShelleyHeader {
                body: ShelleyHeaderBody {
                    block_number: 1,
                    slot: 100,
                    prev_hash: ade_types::shelley::block::PrevHash::Block(Hash32([0u8; 32])),
                    issuer_vkey: vec![0u8; 32],
                    vrf_vkey: vec![0u8; 32],
                    vrf: VrfData::Combined {
                        vrf_result: vec![0x82, 0x40, 0x40],
                    },
                    body_size: 0,
                    body_hash: Hash32([0u8; 32]),
                    operational_cert: OperationalCert {
                        hot_vkey: vec![0u8; 32],
                        sequence_number: 0,
                        kes_period: 0,
                        sigma: vec![0u8; 64],
                    },
                    protocol_version: ProtocolVersion { major: 9, minor: 0 },
                },
                kes_signature: Vec::new(),
            },
            tx_count: 0,
            tx_bodies: vec![0x80],
            witness_sets: vec![0x80],
            metadata: vec![0xa0],
            invalid_txs,
        }
    }

    #[test]
    fn block_body_hash_pinned_recipe_byte_identical() {
        let tx_bodies = &[0x80u8][..];
        let witness_sets = &[0x80u8][..];
        let metadata = &[0xa0u8][..];
        let invalid_txs: Option<&[u8]> = None;

        let h_tx = blake2b_256(tx_bodies).0;
        let h_ws = blake2b_256(witness_sets).0;
        let h_md = blake2b_256(metadata).0;
        let h_iv = blake2b_256(&[]).0;
        let mut concat = [0u8; 128];
        concat[0..32].copy_from_slice(&h_tx);
        concat[32..64].copy_from_slice(&h_ws);
        concat[64..96].copy_from_slice(&h_md);
        concat[96..128].copy_from_slice(&h_iv);
        let expected = Hash32(blake2b_256(&concat).0);

        let actual = block_body_hash_from_buckets(tx_bodies, witness_sets, metadata, invalid_txs);
        assert_eq!(actual, expected);
    }

    #[test]
    fn block_body_hash_from_block_equals_from_buckets() {
        let block = fixture_block(Some(vec![0x80]));
        let from_block = block_body_hash(&block);
        let from_buckets = block_body_hash_from_buckets(
            &block.tx_bodies,
            &block.witness_sets,
            &block.metadata,
            block.invalid_txs.as_deref(),
        );
        assert_eq!(from_block, from_buckets);
    }

    #[test]
    fn block_body_hash_none_invalid_txs_equals_empty_bucket() {
        let block_none = fixture_block(None);
        let block_empty = fixture_block(Some(Vec::new()));
        assert_eq!(
            block_body_hash(&block_none),
            block_body_hash(&block_empty),
        );
    }
}
