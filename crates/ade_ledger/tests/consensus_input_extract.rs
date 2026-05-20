// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! B1-S1 cross-validation: the committed Conway-576 corpus is self-consistent —
//! every block issuer resolves to a pool present in `consensus_inputs.json`, the
//! pinned eta0 matches, and the active-slots coefficient is 1/20.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_codec::conway::decode_conway_block;
use ade_crypto::blake2b_224;
use ade_testkit::validity::ConwayValidityCorpus;

const PINNED_ETA0: &str = "d4d5f9dc027133f27b2551711c078e7d34575daba4aa0ee7c82e4b9dd9f55c51";

fn hex32(s: &str) -> [u8; 32] {
    let bytes = s.as_bytes();
    assert_eq!(bytes.len(), 64);
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
    }
    out
}

#[test]
fn corpus_loads_and_is_self_consistent() {
    let corpus = ConwayValidityCorpus::load().expect("load conway_epoch576 corpus");

    // eta0 matches the pinned epoch-576 value.
    assert_eq!(corpus.epoch_nonce, hex32(PINNED_ETA0));

    // Active-slots coefficient is the mainnet constant 1/20.
    assert_eq!(corpus.asc.numer, 1);
    assert_eq!(corpus.asc.denom, 20);

    // The corpus carries the 14 real Conway-576 tail blocks.
    assert_eq!(corpus.blocks.len(), 14);

    // Every block issuer's pool id resolves to a pool in consensus_inputs.json.
    for block in &corpus.blocks {
        let env = decode_block_envelope(block).expect("envelope");
        let inner = &block[env.block_start..env.block_end];
        let decoded = decode_conway_block(inner).expect("conway block");
        let issuer_vkey = &decoded.decoded().header.body.issuer_vkey;
        let pool_id = blake2b_224(issuer_vkey).0;
        assert!(
            corpus.pools.contains_key(&pool_id),
            "block issuer pool {} absent from corpus pools",
            hex(&pool_id)
        );
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
