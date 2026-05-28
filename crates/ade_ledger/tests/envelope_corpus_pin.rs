// PHASE4-N-V S1 — corpus pin for the canonical block-envelope encoder.
//
// `ade_codec` is BLUE and cannot depend on `ade_testkit` (the corpus crate),
// so the encode-against-real-data pin lives here in `ade_ledger`, which has
// `ade_testkit` as a dev-dependency. Proves `encode_block_envelope` is a
// byte-exact inverse of `decode_block_envelope` on a real Conway block —
// pinning the `82 07` envelope head and the exact shape against ground truth
// (resolves OQ1). CE-V-3.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ade_codec::cbor::envelope::{decode_block_envelope, encode_block_envelope};
use ade_testkit::validity::ConwayValidityCorpus;
use ade_types::CardanoEra;

#[test]
fn encode_block_envelope_reencodes_corpus_block_identically() {
    let corpus = ConwayValidityCorpus::load().expect("corpus loads");
    assert!(!corpus.blocks.is_empty(), "corpus must be non-empty");

    for (i, b) in corpus.blocks.iter().enumerate() {
        let env = decode_block_envelope(b)
            .unwrap_or_else(|e| panic!("corpus block {i} must have a valid envelope: {e:?}"));
        assert_eq!(env.era, CardanoEra::Conway, "corpus is Conway-era");
        let inner = &b[env.block_start..env.block_end];
        let reencoded = encode_block_envelope(env.era, inner);
        assert_eq!(
            reencoded, *b,
            "encode_block_envelope(decode(b)) must byte-equal corpus block {i}"
        );
        // Head pin: every Conway envelope is `82 07 ..`.
        assert_eq!(&b[..2], &[0x82, 0x07], "corpus block {i} head is 82 07");
    }
}
