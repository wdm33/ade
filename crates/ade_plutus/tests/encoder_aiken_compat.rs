// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! S-32 item 4 — real-mainnet-tx integration prerequisite.
//!
//! Proves that `ade_codec`'s new Alonzo+ TxOut encoders (feat commit
//! `a176a18`) emit bytes that aiken's phase-2 entry point
//! (`eval_phase_two_raw`) can consume via its `resolved_utxos`
//! argument. aiken parses each output byte-slice through
//! `pallas_primitives::conway::TransactionOutput::decode_fragment`
//! (see aiken/crates/uplc/src/tx.rs:150–155), so round-tripping our
//! encoder output through pallas's decoder is the minimal
//! compatibility contract we must satisfy.
//!
//! What this test does NOT do: full tx eval. A complete mainnet tx
//! requires:
//!   (1) an encoded Alonzo+ tx body (no encoder yet in ade_codec —
//!       separate work item),
//!   (2) a valid witness-set, is-valid flag, and aux-data wrapper,
//!   (3) resolved UTxOs for every input, collateral input, and
//!       reference input.
//! Those are tracked as follow-up work. This test closes the
//! encoder's half of the contract: bytes that land in aiken's
//! `resolved_utxos` parameter must decode.

use ade_codec::babbage::tx::{encode_babbage_tx_out_array, encode_babbage_tx_out_map};
use ade_codec::traits::{AdeEncode, CodecContext};
use ade_types::alonzo::tx::AlonzoTxOut;
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::tx::Coin;
use ade_types::{CardanoEra, Hash32};

use pallas_primitives::Fragment;
use pallas_primitives::conway::TransactionOutput;

fn alonzo_ctx() -> CodecContext {
    CodecContext {
        era: CardanoEra::Alonzo,
    }
}

fn babbage_ctx() -> CodecContext {
    CodecContext {
        era: CardanoEra::Babbage,
    }
}

// ---------------------------------------------------------------------------
// Alonzo (Legacy / array form) — must decode as PseudoTransactionOutput::Legacy
// ---------------------------------------------------------------------------

#[test]
fn alonzo_txout_coin_only_decodes_via_pallas() {
    let out = AlonzoTxOut {
        address: vec![0x60, 0x01, 0x02, 0x03, 0x04],
        coin: Coin(1_000_000),
        multi_asset: None,
        datum_hash: None,
    };
    let mut buf = Vec::new();
    out.ade_encode(&mut buf, &alonzo_ctx())
        .expect("alonzo encode must succeed");

    let decoded =
        TransactionOutput::decode_fragment(&buf).expect("pallas must parse alonzo array form");
    assert!(
        matches!(decoded, TransactionOutput::Legacy(_)),
        "alonzo array form must decode to Legacy variant",
    );
}

#[test]
fn alonzo_txout_with_datum_hash_decodes_via_pallas() {
    let out = AlonzoTxOut {
        address: vec![0x60, 0x01, 0x02, 0x03, 0x04],
        coin: Coin(42),
        multi_asset: None,
        datum_hash: Some(Hash32([0xAA; 32])),
    };
    let mut buf = Vec::new();
    out.ade_encode(&mut buf, &alonzo_ctx())
        .expect("alonzo+datum encode must succeed");

    let decoded = TransactionOutput::decode_fragment(&buf)
        .expect("pallas must parse alonzo array form with datum");
    match decoded {
        TransactionOutput::Legacy(legacy) => {
            assert_eq!(
                legacy.datum_hash.map(|h| h.to_vec()),
                Some(vec![0xAA; 32]),
                "datum hash must survive round-trip",
            );
        }
        _ => panic!("alonzo with datum must decode to Legacy"),
    }
}

// ---------------------------------------------------------------------------
// Babbage map form — must decode as PseudoTransactionOutput::PostAlonzo
// ---------------------------------------------------------------------------

#[test]
fn babbage_txout_map_coin_only_decodes_via_pallas() {
    let out = BabbageTxOut {
        address: vec![0x60, 0x01, 0x02, 0x03, 0x04],
        coin: Coin(1_000_000),
        multi_asset: None,
        datum_option: None,
        script_ref: None,
    };
    let mut buf = Vec::new();
    out.ade_encode(&mut buf, &babbage_ctx())
        .expect("babbage map encode must succeed");

    let decoded =
        TransactionOutput::decode_fragment(&buf).expect("pallas must parse babbage map form");
    assert!(
        matches!(decoded, TransactionOutput::PostAlonzo(_)),
        "babbage map form must decode to PostAlonzo variant",
    );
}

#[test]
fn babbage_txout_map_with_datum_hash_decodes_via_pallas() {
    // PseudoDatumOption::Hash = [0, bstr(32)] — minimal valid datum_option
    // whose inner bytes are semantically meaningful (not opaque fill).
    let mut datum_cbor = vec![0x82, 0x00, 0x58, 0x20];
    datum_cbor.extend_from_slice(&[0xCC; 32]);

    let out = BabbageTxOut {
        address: vec![0x60, 0x01, 0x02, 0x03, 0x04],
        coin: Coin(2_000_000),
        multi_asset: None,
        datum_option: Some(datum_cbor),
        script_ref: None,
    };
    let mut buf = Vec::new();
    out.ade_encode(&mut buf, &babbage_ctx())
        .expect("babbage map encode with datum_hash must succeed");

    let decoded = TransactionOutput::decode_fragment(&buf)
        .expect("pallas must parse babbage map form with datum_hash");
    match decoded {
        TransactionOutput::PostAlonzo(post) => {
            let addr: &[u8] = post.address.as_ref();
            assert_eq!(addr, &[0x60, 0x01, 0x02, 0x03, 0x04]);
            assert!(post.datum_option.is_some(), "datum_option must round-trip");
            assert!(post.script_ref.is_none(), "script_ref must stay None");
        }
        _ => panic!("babbage map with datum must decode to PostAlonzo"),
    }
}

// ---------------------------------------------------------------------------
// Babbage array form — legacy 2-field / with-datum must decode as Legacy
// (matches pallas's variant-dispatch on array vs map at the outer level).
// ---------------------------------------------------------------------------

#[test]
fn babbage_txout_array_coin_only_decodes_via_pallas() {
    let out = BabbageTxOut {
        address: vec![0x60, 0x01, 0x02, 0x03, 0x04],
        coin: Coin(500_000),
        multi_asset: None,
        datum_option: None,
        script_ref: None,
    };
    let mut buf = Vec::new();
    encode_babbage_tx_out_array(&mut buf, &out).expect("babbage array encode must succeed");

    let decoded =
        TransactionOutput::decode_fragment(&buf).expect("pallas must parse babbage array form");
    assert!(
        matches!(decoded, TransactionOutput::Legacy(_)),
        "babbage array form must decode to Legacy variant (pallas dispatches on array vs map)",
    );
}

// ---------------------------------------------------------------------------
// Compile-time shim — proves the same encoder used by run_phase_one_composers
// is the same one covered by the compat tests above (i.e. no risk of wiring
// divergence).
// ---------------------------------------------------------------------------

#[test]
fn babbage_trait_and_map_free_fn_agree() {
    // AdeEncode on BabbageTxOut delegates to encode_babbage_tx_out_map.
    // Byte equality guards against future drift between the two paths.
    let babbage = BabbageTxOut {
        address: vec![0x60, 0xcc, 0xdd],
        coin: Coin(9),
        multi_asset: None,
        datum_option: None,
        script_ref: None,
    };
    let mut via_trait = Vec::new();
    babbage
        .ade_encode(&mut via_trait, &babbage_ctx())
        .expect("trait encode");
    let mut via_fn = Vec::new();
    encode_babbage_tx_out_map(&mut via_fn, &babbage).expect("fn encode");
    assert_eq!(via_trait, via_fn, "babbage map encoder paths must agree");
}
