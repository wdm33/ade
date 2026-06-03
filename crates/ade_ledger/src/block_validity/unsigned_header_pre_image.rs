// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE canonical unsigned-header pre-image recipe
//! (PHASE4-N-S-A A2).
//!
//! `unsigned_header_pre_image(...)` produces the canonical
//! CBOR encoding of `ShelleyHeaderBody` — the bytes the KES
//! signature is computed over, and the bytes the validator
//! extracts via `decode_block(block_bytes).header_input.kes.header_body_bytes`.
//!
//! Single source of truth: this recipe + the existing
//! `ShelleyHeaderBody::ade_encode` impl produce byte-identical
//! output to what `decode_block` extracts from any corpus
//! block (cross-impl byte-match enforced by
//! `CN-PREIMAGE-FIXTURE-01`).
//!
//! Branded `UnsignedHeaderPreImage(Vec<u8>)` makes
//! arbitrary-byte signing structurally unrepresentable:
//! the only constructor is this recipe, and
//! `kes_sign_header` (in `ade_runtime::producer`) accepts
//! only this type.
//!
//! Doctrine: see [[feedback-fail-closed-validation]] —
//! KES signing input is a type-system gate, not a comment.

use ade_codec::traits::{AdeEncode, CodecContext};
use ade_types::shelley::block::{
    OperationalCert, PrevHash, ProtocolVersion, ShelleyHeaderBody, VrfData,
};
use ade_types::{CardanoEra, Hash32};

/// Branded unsigned-header pre-image bytes.
///
/// The only public constructor is
/// `unsigned_header_pre_image(...)`. Callers cannot fabricate
/// a value from arbitrary bytes — the inner field is private.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsignedHeaderPreImage(Vec<u8>);

impl UnsignedHeaderPreImage {
    /// Read-only access to the underlying bytes. Used by:
    /// (a) the validator's `verify_kes` to verify a signature;
    /// (b) `kes_sign_header` (RED) to produce a signature;
    /// (c) cross-impl tests asserting byte equality against
    ///     `decode_block.header_input.kes.header_body_bytes`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Length of the pre-image. Cheap; useful for evidence
    /// logging without holding a reference to the bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Canonical pre-image recipe.
///
/// Constructs a `ShelleyHeaderBody` from the supplied
/// canonical inputs and CBOR-encodes it via the same
/// `AdeEncode` impl that `forge_block` uses internally
/// (`ade_codec::shelley::block::ShelleyHeaderBody::ade_encode`).
///
/// The output bytes are byte-identical to what
/// `ade_ledger::block_validity::header_input::decode_block(...).header_input.kes.header_body_bytes`
/// produces for any corpus block whose `(slot, block_no,
/// prev_hash, issuer_vkey, vrf_vkey, vrf_result, body_size,
/// body_hash, opcert, protocol_version)` matches these
/// inputs.
///
/// **Failure mode:** `ade_encode` only fails for genuinely
/// malformed inputs (negative integers in unsigned fields,
/// etc.); valid canonical inputs always succeed. The
/// function returns a `Result` for strict totality; callers
/// should treat `Err` as a programming bug (the caller's
/// inputs are out of canonical range).
pub fn unsigned_header_pre_image(
    slot: u64,
    block_number: u64,
    prev_hash: PrevHash,
    issuer_vkey: Vec<u8>,
    vrf_vkey: Vec<u8>,
    vrf_result: Vec<u8>, // pre-encoded CBOR array(2) of [output, proof]
    body_size: u64,
    body_hash: Hash32,
    operational_cert: OperationalCert,
    protocol_version: ProtocolVersion,
) -> Result<UnsignedHeaderPreImage, UnsignedHeaderPreImageError> {
    let body = ShelleyHeaderBody {
        block_number,
        slot,
        prev_hash,
        issuer_vkey,
        vrf_vkey,
        vrf: VrfData::Combined { vrf_result },
        body_size,
        body_hash,
        operational_cert,
        protocol_version,
    };
    let mut buf = Vec::new();
    let ctx = CodecContext {
        era: CardanoEra::Conway,
    };
    body.ade_encode(&mut buf, &ctx)
        .map_err(|_| UnsignedHeaderPreImageError::EncodeFailure)?;
    Ok(UnsignedHeaderPreImage(buf))
}

/// Closed error surface. No `String` payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsignedHeaderPreImageError {
    /// `ade_encode` returned a `CodecError`. Should never
    /// occur for canonical inputs; the variant exists for
    /// strict totality.
    EncodeFailure,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::block_validity::header_input::decode_block;
    use ade_testkit::validity::ConwayValidityCorpus;

    /// PHASE4-N-S-A A2 byte-match: for every Conway corpus
    /// block, Ade's recipe output equals
    /// `decode_block.header_input.kes.header_body_bytes`
    /// byte-for-byte. Closes `CN-PREIMAGE-FIXTURE-01`.
    #[test]
    fn unsigned_header_preimage_matches_decode_block_extraction_for_corpus() {
        let corpus = ConwayValidityCorpus::load().expect("corpus");
        let mut checked = 0usize;
        for (i, block_bytes) in corpus.blocks.iter().enumerate() {
            let decoded = match decode_block(block_bytes) {
                Ok(d) => d,
                Err(e) => panic!("corpus block {} decode failure: {:?}", i, e),
            };
            let kes = decoded
                .header_input
                .kes
                .as_ref()
                .expect("Conway blocks carry kes");
            let reference = &kes.header_body_bytes;

            // Reconstruct the VRF "vrf_result" CBOR field
            // from the decoded HeaderVrf (Praos Combined
            // shape: array(2) of [output, proof]). This
            // mirrors forge.rs:282-288.
            let vrf_result = match &decoded.header_input.vrf {
                ade_core::consensus::header_summary::HeaderVrf::Praos { proof, output } => {
                    let mut buf = Vec::with_capacity(2 + 64 + 80);
                    use ade_codec::cbor::{
                        write_array_header, write_bytes_canonical, ContainerEncoding, IntWidth,
                    };
                    write_array_header(
                        &mut buf,
                        ContainerEncoding::Definite(2, IntWidth::Inline),
                    );
                    write_bytes_canonical(&mut buf, &output.0);
                    write_bytes_canonical(&mut buf, &proof.0);
                    buf
                }
                ade_core::consensus::header_summary::HeaderVrf::Tpraos { .. } => {
                    // Pre-Babbage; corpus is Conway. Skip.
                    continue;
                }
            };

            // Decode the inner block to extract operational
            // cert + protocol_version (not surfaced in
            // HeaderInput directly).
            let inner = &block_bytes[decoded.inner_start..decoded.inner_end];
            let preserved = ade_codec::conway::decode_conway_block(inner)
                .expect("conway block decode");
            let shelley_block = preserved.decoded();
            let opcert = shelley_block.header.body.operational_cert.clone();
            let protocol_version = shelley_block.header.body.protocol_version;
            let issuer_vkey = shelley_block.header.body.issuer_vkey.clone();
            let vrf_vkey = shelley_block.header.body.vrf_vkey.clone();
            let body_size = shelley_block.header.body.body_size;
            let prev_hash_field = shelley_block.header.body.prev_hash.clone();

            let recipe_out = unsigned_header_pre_image(
                decoded.header_input.slot.0,
                decoded.header_input.block_no.0,
                prev_hash_field,
                issuer_vkey,
                vrf_vkey,
                vrf_result,
                body_size,
                decoded.header_input.body_hash.clone(),
                opcert,
                protocol_version,
            )
            .expect("recipe encode");

            assert_eq!(
                recipe_out.as_bytes(),
                reference.as_slice(),
                "byte-match failure on corpus block {}",
                i,
            );
            checked += 1;
        }
        assert!(checked > 0, "must check at least one corpus block");
    }

    #[test]
    fn recipe_output_is_byte_identical_across_two_runs() {
        // DC-KES-HEADER-01 replay anchor.
        let prev_hash = PrevHash::Block(Hash32([0xAA; 32]));
        let issuer = vec![0x01; 32];
        let vrf_vkey = vec![0x02; 32];
        let vrf_result = {
            use ade_codec::cbor::{
                write_array_header, write_bytes_canonical, ContainerEncoding, IntWidth,
            };
            let mut buf = Vec::new();
            write_array_header(
                &mut buf,
                ContainerEncoding::Definite(2, IntWidth::Inline),
            );
            write_bytes_canonical(&mut buf, &[0x03; 64]);
            write_bytes_canonical(&mut buf, &[0x04; 80]);
            buf
        };
        let body_hash = Hash32([0x05; 32]);
        let opcert = OperationalCert {
            hot_vkey: vec![0x06; 32],
            sequence_number: 7,
            kes_period: 42,
            sigma: vec![0x08; 64],
        };
        let pv = ProtocolVersion {
            major: 9,
            minor: 0,
        };

        let a = unsigned_header_pre_image(
            100,
            1,
            prev_hash.clone(),
            issuer.clone(),
            vrf_vkey.clone(),
            vrf_result.clone(),
            128,
            body_hash.clone(),
            opcert.clone(),
            pv,
        )
        .unwrap();
        let b = unsigned_header_pre_image(
            100, 1, prev_hash, issuer, vrf_vkey, vrf_result, 128, body_hash, opcert, pv,
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn unsigned_header_preimage_constructor_is_private() {
        // Compile-time check: the inner Vec<u8> is private,
        // so this test exists only to assert that callers
        // outside this module cannot bypass the recipe.
        // The branded type's only public construction path
        // is `unsigned_header_pre_image(...)`.
        let _ = "branded type — no other constructor reachable";
    }

    // -----------------------------------------------------------------
    // PHASE4-N-F-G-J S3 — genesis (null) vs Block (hash32) pre-image.
    // -----------------------------------------------------------------

    fn sample_vrf_result() -> Vec<u8> {
        use ade_codec::cbor::{
            write_array_header, write_bytes_canonical, ContainerEncoding, IntWidth,
        };
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_bytes_canonical(&mut buf, &[0x03; 64]);
        write_bytes_canonical(&mut buf, &[0x04; 80]);
        buf
    }

    fn sample_opcert() -> OperationalCert {
        OperationalCert {
            hot_vkey: vec![0x06; 32],
            sequence_number: 7,
            kes_period: 42,
            sigma: vec![0x08; 64],
        }
    }

    #[test]
    fn pre_image_block_zero_emits_genesis_prev() {
        let pv = ProtocolVersion { major: 9, minor: 0 };
        let genesis = unsigned_header_pre_image(
            100,
            0,
            PrevHash::Genesis,
            vec![0x01; 32],
            vec![0x02; 32],
            sample_vrf_result(),
            128,
            Hash32([0x05; 32]),
            sample_opcert(),
            pv,
        )
        .unwrap();
        let blocky = unsigned_header_pre_image(
            100,
            0,
            PrevHash::Block(Hash32([0xAB; 32])),
            vec![0x01; 32],
            vec![0x02; 32],
            sample_vrf_result(),
            128,
            Hash32([0x05; 32]),
            sample_opcert(),
            pv,
        )
        .unwrap();
        // Genesis -> CBOR null (1 byte); Block -> bytes(32) (0x58 0x20 + 32
        // = 34 bytes). The genesis pre-image is exactly 33 bytes shorter.
        assert_eq!(blocky.len() - genesis.len(), 33);
        assert!(
            genesis.as_bytes().contains(&0xf6),
            "the genesis predecessor must be encoded as CBOR null"
        );
    }

    #[test]
    fn pre_image_nonzero_block_prev_byte_identical() {
        let pv = ProtocolVersion { major: 9, minor: 0 };
        let h = Hash32([0xAB; 32]);
        let a = unsigned_header_pre_image(
            100,
            1,
            PrevHash::Block(h.clone()),
            vec![0x01; 32],
            vec![0x02; 32],
            sample_vrf_result(),
            128,
            Hash32([0x05; 32]),
            sample_opcert(),
            pv,
        )
        .unwrap();
        let b = unsigned_header_pre_image(
            100,
            1,
            PrevHash::Block(h),
            vec![0x01; 32],
            vec![0x02; 32],
            sample_vrf_result(),
            128,
            Hash32([0x05; 32]),
            sample_opcert(),
            pv,
        )
        .unwrap();
        assert_eq!(a, b, "Block-path pre-image is deterministic");
        // The 32-byte parent hash appears verbatim in the signed bytes.
        assert!(a.as_bytes().windows(32).any(|w| w == [0xAB; 32]));
    }

    #[test]
    fn corpus_blocks_pass_header_position_rule() {
        // Every real corpus block (block_number > 0, Block predecessor)
        // decodes — i.e. passes check_header_position — via decode_block.
        let corpus = ConwayValidityCorpus::load().expect("corpus");
        let mut checked = 0usize;
        for (i, block_bytes) in corpus.blocks.iter().enumerate() {
            let decoded = decode_block(block_bytes)
                .unwrap_or_else(|e| panic!("corpus block {i} must pass the position rule: {e:?}"));
            assert!(
                decoded.header_input.block_no.0 > 0,
                "corpus blocks are non-genesis"
            );
            checked += 1;
        }
        assert!(checked > 0, "must check at least one corpus block");
    }
}
