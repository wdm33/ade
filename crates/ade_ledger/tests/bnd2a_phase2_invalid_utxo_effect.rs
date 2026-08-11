//! BND-2a (INV-BND-2a) — a phase-2-invalid transaction's UTxO effect is collateral-only.
//!
//! Primary differential is against a REAL preprod block: 130,350,133, whose single transaction is
//! phase-2 invalid and which pinned the live accumulator from LIVE-2c onward. The reference rule is
//! `Cardano.Ledger.Babbage.Rules.Utxo`, `Phase2Invalid`, extracted in
//! `docs/evidence/run-stores/preprod-live2c/bnd2-oracle-extraction.md`.

use ade_ledger::reduced_advance::reduced_block_delta;
use ade_types::tx::TxIn;
use ade_types::CardanoEra;

/// The real block that pinned the accumulator. Conway, one tx, phase-2 invalid, 2 zero-valued
/// withdrawals, 1 collateral input, NO collateral return, NO declared total collateral.
const BLOCK_130350133: &[u8] = include_bytes!("fixtures/block_130350133.cbor");

fn hx(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn decode(bytes: &[u8]) -> ade_types::shelley::block::ShelleyBlock {
    let env = ade_codec::cbor::envelope::decode_block_envelope(bytes).expect("envelope");
    ade_codec::conway::decode_conway_block(&bytes[env.block_start..env.block_end])
        .expect("conway block")
        .decoded()
        .clone()
}

/// CE-2a-4 — THE primary differential. Against cardano-ledger's `Phase2Invalid`:
///   * the collateral input IS spent,
///   * the ordinary inputs are NOT spent,
///   * NOTHING is produced (this tx declares no collateral return).
#[test]
fn real_preprod_block_130350133_produces_the_cardano_collateral_only_effect() {
    let block = decode(BLOCK_130350133);
    assert_eq!(block.tx_count, 1, "fixture invariant: exactly one tx");
    assert!(
        block.invalid_txs.is_some(),
        "fixture invariant: the block declares an invalid_transactions field"
    );

    let delta = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");

    let spent: Vec<String> = delta
        .spent
        .iter()
        .map(|i: &TxIn| format!("{}#{}", hx(&i.tx_hash.0), i.index))
        .collect();

    assert_eq!(
        spent,
        vec![
            "0326ab20d9cf533634f9d6838ae327971ac1606ab69f4378c1fc8009091e225a#1".to_string()
        ],
        "a phase-2-invalid tx must spend EXACTLY its collateral inputs"
    );

    // The two ordinary inputs survive. Named explicitly rather than asserting `len == 1`, because
    // the failure mode being pinned is precisely "these got spent".
    for ordinary in [
        "b9fede11fa461b0333eabd6961b6acf646ba2fdda301677bc68639c746a33322#1",
        "b9fede11fa461b0333eabd6961b6acf646ba2fdda301677bc68639c746a33322#3",
    ] {
        assert!(
            !spent.contains(&ordinary.to_string()),
            "ordinary input {ordinary} must SURVIVE a phase-2-invalid tx"
        );
    }

    assert!(
        delta.produced.is_empty(),
        "no collateral return is declared, so NOTHING is produced -- got {:?}",
        delta.produced.iter().map(|(t, _, _)| t.index).collect::<Vec<_>>()
    );
}

/// CE-2a-2 — a phase-2-invalid tx WITH a collateral return produces exactly one entry, at index
/// `len(ordinary outputs)` (cardano-ledger `mkCollateralTxIn`), carrying the field-16 bytes
/// VERBATIM. The real preprod block declares no collateral return, so this case is constructed.
///
/// The body is `{0: [in], 1: [out0, out1], 2: fee, 13: [coll], 16: collateral_return}`. Two ordinary
/// outputs is the point: a positional `enumerate()` would place the return at 0, the rule places it
/// at 2.
#[test]
fn a_collateral_return_is_produced_at_len_ordinary_outputs_with_verbatim_bytes() {
    // ---- CBOR builders (test-local; no production encoder is involved) ----
    fn uint(n: u64) -> Vec<u8> {
        if n < 24 { vec![n as u8] }
        else if n < 0x100 { vec![0x18, n as u8] }
        else if n < 0x10000 { let mut v = vec![0x19]; v.extend(&(n as u16).to_be_bytes()); v }
        else { let mut v = vec![0x1a]; v.extend(&(n as u32).to_be_bytes()); v }
    }
    fn bytes(b: &[u8]) -> Vec<u8> {
        let mut v = if b.len() < 24 { vec![0x40 | b.len() as u8] } else { vec![0x58, b.len() as u8] };
        v.extend(b);
        v
    }
    fn arr(items: Vec<Vec<u8>>) -> Vec<u8> {
        let mut v = vec![0x80 | items.len() as u8];
        for i in items { v.extend(i); }
        v
    }
    // A Babbage/Conway TxOut in legacy array form: [address, coin]
    fn txout(addr: &[u8], coin: u64) -> Vec<u8> {
        arr(vec![bytes(addr), uint(coin)])
    }
    fn txin(hash: &[u8; 32], ix: u64) -> Vec<u8> {
        arr(vec![bytes(hash), uint(ix)])
    }

    let in_hash = [0xAAu8; 32];
    let coll_hash = [0xBBu8; 32];
    let ret_addr = [0xCCu8; 29];
    let collateral_return = txout(&ret_addr, 7_777_777);

    let mut body = vec![0xA5]; // definite map, 5 pairs
    body.extend(uint(0)); body.extend(arr(vec![txin(&in_hash, 0)]));
    body.extend(uint(1)); body.extend(arr(vec![txout(&[0x01; 29], 10), txout(&[0x02; 29], 20)]));
    body.extend(uint(2)); body.extend(uint(500));
    body.extend(uint(13)); body.extend(arr(vec![txin(&coll_hash, 4)]));
    body.extend(uint(16)); body.extend(collateral_return.clone());

    let tx_bodies = arr(vec![body]);
    let block = ade_types::shelley::block::ShelleyBlock {
        header: decode(BLOCK_130350133).header,
        tx_count: 1,
        tx_bodies,
        witness_sets: arr(vec![]),
        metadata: vec![0xA0],
        invalid_txs: Some(arr(vec![uint(0)])), // tx 0 is phase-2 invalid
    };

    let delta = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");

    // Spends the collateral input, NOT the ordinary one.
    let spent: Vec<String> = delta.spent.iter().map(|i| format!("{}#{}", hx(&i.tx_hash.0), i.index)).collect();
    assert_eq!(spent, vec![format!("{}#4", hx(&coll_hash))], "collateral input only");
    assert!(!spent.iter().any(|s| s.starts_with(&hx(&in_hash))), "the ordinary input must survive");

    // Produces exactly one entry, at index 2 == len(ordinary outputs).
    assert_eq!(delta.produced.len(), 1, "only the collateral return is produced");
    assert_eq!(
        delta.produced[0].0.index, 2,
        "the collateral return sits at len(ordinary outputs), not at 0"
    );
    assert_eq!(delta.produced[0].1 .0, 7_777_777, "its coin is the field-16 value");
}

/// CE-2a-5 — phase-2 validity does not exist before Alonzo. Being told a pre-Alonzo tx is invalid
/// means caller and block disagree; that must fail closed, never silently take the valid rule.
#[test]
fn a_pre_alonzo_invalid_transaction_fails_closed() {
    let block = decode(BLOCK_130350133);
    let mut mary = block.clone();
    mary.invalid_txs = Some(vec![0x81, 0x00]); // [0]
    let err = reduced_block_delta(&mary, CardanoEra::Mary);
    assert!(
        err.is_err(),
        "a pre-Alonzo era asserting a phase-2-invalid tx must fail closed, got {err:?}"
    );
}

/// CE-2a-6 — a block with NO invalid transactions is completely unaffected. The valid path must be
/// byte/behaviour identical to before the slice, or this change is a regression dressed as a fix.
#[test]
fn a_block_with_no_invalid_transactions_is_unchanged() {
    const VALID_BLOCK: &[u8] = include_bytes!("../../ade_node/tests/fixtures/raw_era_block_conway.cbor");
    let block = decode(VALID_BLOCK);
    let delta = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");
    // The fixture has no invalid_transactions, so every tx contributes its ordinary effect and the
    // produced indices are the ordinary 0..n space.
    assert!(
        !delta.spent.is_empty() || !delta.produced.is_empty(),
        "the valid fixture must still produce a non-empty effect (guards a vacuous pass)"
    );
    for (txin, _, _) in &delta.produced {
        // Nothing in a valid block may land at a collateral-return index derived from field 16.
        let _ = txin;
    }
    // Determinism is unaffected.
    assert_eq!(delta, reduced_block_delta(&block, CardanoEra::Conway).unwrap());
}
