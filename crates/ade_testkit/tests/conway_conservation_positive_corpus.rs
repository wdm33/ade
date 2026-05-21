// PHASE4-B3-S5 (CE-B3-5) — REAL-corpus positive conservation oracle.
//
// Drives the BLUE `tx_validity` over every real cert/withdrawal-bearing Conway tx
// in the committed epoch-576 corpus at track_utxo=true, against the real resolved
// input UTxOs, and asserts each is Valid (consumed == produced holds for real
// on-chain txs) with a byte-identical verdict stream across two runs.
//
// DATA-GATED: the resolved input set lives in
// `corpus/validity/conway_epoch576/resolved_inputs.json` (79 inputs — spend +
// collateral + reference — for the 29 cert/withdrawal-bearing txs). It was produced
// by `cardano-cli query utxo` against a node whose cloned ImmutableDB was truncated
// to slot 163900587 (the block immediately before the first corpus block), plus the
// intra-corpus outputs created by earlier corpus txs (see
// corpus/tools/extract_conway_resolved_inputs.md). When the fixture is absent (e.g.
// the epoch-576 ledger snapshot is not reproduced), the test SKIPS loudly rather than
// failing — guard correctness + no-false-accept are covered by conway_conservation_full
// (S4) and conway_conservation_adversarial (S6).
//
// Plutus-script-bearing txs (script_data_hash present) are carved out: their phase-2
// goes through aiken, which has a known Conway divergence (CE-88, externally blocked).
// That is orthogonal to value-conservation, which is what this oracle establishes.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use ade_ledger::tx_validity::{encode_tx_verdict_surface, tx_validity, TxValidityVerdict};
use ade_ledger::utxo::{TxOut, UTxOState};
use ade_ledger::value::{MultiAsset, Value};
use ade_testkit::tx_validity::extract_corpus_txs;
use ade_testkit::validity::corpus::ConwayValidityCorpus;

use ade_codec::conway::tx::decode_conway_tx_body;
use ade_ledger::pparams::ConwayOnlyDepositParams;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, Hash32};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("validity")
        .join("conway_epoch576")
        .join("resolved_inputs.json")
}

#[derive(serde::Deserialize)]
struct ResolvedInput {
    tx_hash: String,
    index: u16,
    coin: u64,
    address: String,
}

#[derive(serde::Deserialize)]
struct ResolvedInputs {
    inputs: Vec<ResolvedInput>,
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex_to_hash32(s: &str) -> Hash32 {
    let b = hex_to_bytes(s);
    let mut h = [0u8; 32];
    h.copy_from_slice(&b);
    Hash32(h)
}

fn load_resolved_utxo() -> Option<BTreeMap<TxIn, TxOut>> {
    let path = fixture_path();
    let text = std::fs::read_to_string(&path).ok()?;
    let parsed: ResolvedInputs = serde_json::from_str(&text).expect("resolved_inputs.json parse");
    let mut map = BTreeMap::new();
    for ri in parsed.inputs {
        let tx_in = TxIn {
            tx_hash: hex_to_hash32(&ri.tx_hash),
            index: ri.index,
        };
        map.insert(
            tx_in,
            TxOut::ShelleyMary {
                address: hex_to_bytes(&ri.address),
                value: Value {
                    coin: Coin(ri.coin),
                    multi_asset: MultiAsset::new(),
                },
            },
        );
    }
    Some(map)
}

fn ledger_576_with_utxo(utxo: BTreeMap<TxIn, TxOut>) -> ade_ledger::state::LedgerState {
    let mut l = ade_ledger::state::LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = ade_types::EpochNo(576);
    l.track_utxo = true;
    l.utxo_state = UTxOState { utxos: utxo };
    // Mainnet Conway deposit params. These do not affect the corpus txs (their
    // certs are all Neutral delegations — no deposit/refund terms), but the
    // canonical surface must be present for any Conway state at track_utxo=true.
    l.conway_deposit_params = Some(ConwayOnlyDepositParams {
        drep_deposit: Coin(500_000_000),
        gov_action_deposit: Coin(100_000_000_000),
        drep_activity: 20,
    });
    l
}

/// A cert/withdrawal-bearing tx whose verdict is determined by phase-1 +
/// value-conservation (no phase-2 Plutus). Plutus-script-bearing txs
/// (`script_data_hash` present) are carved out: their phase-2 evaluation goes
/// through aiken, which has a known Conway divergence (CE-88, externally
/// blocked — `project_aiken_main_bug_unfixed`). That is orthogonal to the
/// conservation oracle this test establishes.
fn is_conservation_relevant(tx_cbor: &[u8]) -> bool {
    let mut off = 0usize;
    if ade_codec::cbor::read_array_header(tx_cbor, &mut off).is_err() {
        return false;
    }
    match decode_conway_tx_body(tx_cbor, &mut off) {
        Ok(b) => (b.certs.is_some() || b.withdrawals.is_some()) && b.script_data_hash.is_none(),
        Err(_) => false,
    }
}

#[test]
fn real_cert_withdrawal_txs_are_valid_at_track_utxo_true() {
    let Some(utxo) = load_resolved_utxo() else {
        eprintln!(
            "SKIP real_cert_withdrawal_txs_are_valid_at_track_utxo_true: \
             corpus/validity/conway_epoch576/resolved_inputs.json absent \
             (epoch-576 UTxO snapshot not in this environment). Guard correctness \
             is covered by conway_conservation_full + conway_conservation_adversarial."
        );
        return;
    };

    let corpus = ConwayValidityCorpus::load().expect("load corpus");
    let txs = extract_corpus_txs(&corpus.blocks).expect("extract");
    let ledger = ledger_576_with_utxo(utxo);

    let mut checked = 0usize;
    for t in &txs {
        if !is_conservation_relevant(&t.tx_cbor) {
            continue;
        }
        checked += 1;
        let outcome = tx_validity(&ledger, &t.tx_cbor);
        match &outcome.verdict {
            TxValidityVerdict::Valid { .. } => {}
            TxValidityVerdict::Invalid { class, error } => {
                panic!(
                    "real cert/withdrawal tx b{} i{} rejected at track_utxo=true: \
                     class={:?} error={:?}",
                    t.block_index, t.tx_index, class, error
                );
            }
        }
    }
    assert!(checked > 0, "no cert/withdrawal-bearing txs found in corpus");
    eprintln!("real positive conservation oracle: {checked} non-Plutus cert/withdrawal txs Valid at track_utxo=true");
}

#[test]
fn real_positive_verdict_stream_replays_byte_identical() {
    let Some(utxo) = load_resolved_utxo() else {
        eprintln!("SKIP real_positive_verdict_stream_replays_byte_identical: fixture absent");
        return;
    };
    let corpus = ConwayValidityCorpus::load().expect("load corpus");
    let txs = extract_corpus_txs(&corpus.blocks).expect("extract");
    let ledger = ledger_576_with_utxo(utxo);

    let run = || -> Vec<Vec<u8>> {
        txs.iter()
            .filter(|t| is_conservation_relevant(&t.tx_cbor))
            .map(|t| encode_tx_verdict_surface(&tx_validity(&ledger, &t.tx_cbor).verdict))
            .collect()
    };
    assert_eq!(run(), run(), "verdict stream must replay byte-identically");
}
