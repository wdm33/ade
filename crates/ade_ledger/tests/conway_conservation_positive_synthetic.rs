// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-B3-S5 (CE-B3-5), SYNTHETIC PORTION.
//
// THIS IS A SYNTHETIC POSITIVE CORPUS, NOT THE REAL-CORPUS AGREEMENT ORACLE.
//
// CE-B3-5 ultimately requires every REAL on-chain cert/withdrawal-bearing Conway
// tx to be Valid at track_utxo=true against the REAL resolved UTxO + cert/pool/DRep
// state for epoch 576. That oracle is environment-blocked: the epoch-576
// UTxO/ledger-state snapshot was deleted post-extraction and is NOT in this repo
// (see corpus/validity/conway_epoch576/README.md; the B2 positive corpus runs at
// track_utxo=false for the same reason — ade_testkit tx_validity ledger_at_576()
// builds an empty UTxO). Real corpus txs cannot be run at track_utxo=true here
// because their inputs would not resolve.
//
// What THIS file proves, and only this:
//   1. The BLUE conservation guard ACCEPTS balanced cert/withdrawal-bearing Conway
//      txs (consumed == produced) over a CONTROLLED resolved UTxO + seeded CertState
//      + canonical ConwayDepositParams at track_utxo=true semantics.
//   2. The per-tx verdict surface for that synthetic corpus is replay-deterministic:
//      two runs yield byte-identical surfaces.
//
// It does NOT prove agreement with the real epoch-576 chain. The real-corpus oracle
// remains an open obligation for CE-B3-5, blocked on absent snapshot data.

#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};

use ade_ledger::conway::validate_conway_state_backed;
use ade_ledger::delegation::CertState;
use ade_ledger::error::LedgerError;
use ade_ledger::pparams::ConwayDepositParams;
use ade_ledger::utxo::TxOut;
use ade_ledger::value::{MultiAsset, Value};
use ade_ledger::witness::WitnessInfo;

use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash32, SlotNo};

const MAINNET_PERCENT: u16 = 150;
const MAINNET_NET: u8 = 1;
const PV_CONWAY: u16 = 9;
const KEY_DEPOSIT: u64 = 2_000_000;

// --- canonical params (same shape as the S4 full-conservation test) -------

fn deposit_params() -> ConwayDepositParams {
    ConwayDepositParams {
        key_deposit: Coin(KEY_DEPOSIT),
        pool_deposit: Coin(500_000_000),
        drep_deposit: Coin(500_000_000),
        gov_action_deposit: Coin(100_000_000_000),
    }
}

fn mainnet_addr() -> Vec<u8> {
    let mut v = vec![0x61u8];
    v.extend_from_slice(&[0xaa; 28]);
    v
}

fn the_input() -> TxIn {
    TxIn {
        tx_hash: Hash32([0x01; 32]),
        index: 0,
    }
}

fn utxo_with(entries: &[(TxIn, u64)]) -> BTreeMap<TxIn, TxOut> {
    let mut u = BTreeMap::new();
    for (tx_in, coin) in entries {
        u.insert(
            tx_in.clone(),
            TxOut::ShelleyMary {
                address: mainnet_addr(),
                value: Value {
                    coin: Coin(*coin),
                    multi_asset: MultiAsset::new(),
                },
            },
        );
    }
    u
}

fn empty_witness() -> WitnessInfo {
    WitnessInfo {
        available_key_hashes: BTreeSet::new(),
        native_scripts: Vec::new(),
        has_plutus_v1: false,
        has_plutus_v2: false,
        has_plutus_v3: false,
        total_ex_units: Default::default(),
    }
}

fn base_body(input_coin_in_output: u64, fee: u64) -> ConwayTxBody {
    let mut inputs = BTreeSet::new();
    inputs.insert(the_input());
    ConwayTxBody {
        inputs,
        outputs: vec![BabbageTxOut {
            address: mainnet_addr(),
            coin: Coin(input_coin_in_output),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        }],
        fee: Coin(fee),
        ttl: Some(SlotNo(100)),
        certs: None,
        withdrawals: None,
        metadata_hash: None,
        validity_interval_start: None,
        mint: None,
        script_data_hash: None,
        collateral_inputs: None,
        required_signers: None,
        network_id: None,
        collateral_return: None,
        total_collateral: None,
        reference_inputs: None,
        voting_procedures: None,
        proposal_procedures: None,
        treasury_value: None,
        donation: None,
    }
}

fn run(
    body: &ConwayTxBody,
    utxo: &BTreeMap<TxIn, TxOut>,
    cert_state: &CertState,
) -> Result<(), LedgerError> {
    validate_conway_state_backed(
        body,
        utxo,
        &empty_witness(),
        MAINNET_PERCENT,
        MAINNET_NET,
        PV_CONWAY,
        (i64::MAX, i64::MAX),
        &deposit_params(),
        cert_state,
    )
}

// --- minimal CBOR builders (mirrors the S4 full-conservation test) --------

fn cbor_head(buf: &mut Vec<u8>, major: u8, value: u64) {
    let m = major << 5;
    if value < 24 {
        buf.push(m | value as u8);
    } else if value < 0x100 {
        buf.push(m | 24);
        buf.push(value as u8);
    } else if value < 0x1_0000 {
        buf.push(m | 25);
        buf.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value < 0x1_0000_0000 {
        buf.push(m | 26);
        buf.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        buf.push(m | 27);
        buf.extend_from_slice(&value.to_be_bytes());
    }
}

fn cbor_uint(buf: &mut Vec<u8>, v: u64) {
    cbor_head(buf, 0, v);
}

fn cbor_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    cbor_head(buf, 2, bytes.len() as u64);
    buf.extend_from_slice(bytes);
}

fn cbor_array(buf: &mut Vec<u8>, n: u64) {
    cbor_head(buf, 4, n);
}

fn cbor_map(buf: &mut Vec<u8>, n: u64) {
    cbor_head(buf, 5, n);
}

fn stake_credential(buf: &mut Vec<u8>, fill: u8) {
    cbor_array(buf, 2);
    cbor_uint(buf, 0); // key-hash credential type
    cbor_bytes(buf, &[fill; 28]);
}

fn encode_withdrawals(entries: &[(u8, u64)]) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_map(&mut buf, entries.len() as u64);
    for (fill, coin) in entries {
        let mut acct = [0u8; 29];
        acct[0] = 0xe1;
        for b in acct.iter_mut().skip(1) {
            *b = *fill;
        }
        cbor_bytes(&mut buf, &acct);
        cbor_uint(&mut buf, *coin);
    }
    buf
}

fn encode_certs(certs: &[Vec<u8>]) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, certs.len() as u64);
    for c in certs {
        buf.extend_from_slice(c);
    }
    buf
}

/// tag 0 — legacy registration (implicit key_deposit, NewDeposit(DepositParam)).
fn cert_legacy_registration(cred_fill: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 2);
    cbor_uint(&mut buf, 0);
    stake_credential(&mut buf, cred_fill);
    buf
}

/// tag 7 — explicit-deposit stake registration (NewDeposit(ExplicitInCert)).
fn cert_explicit_registration(cred_fill: u8, deposit: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 3);
    cbor_uint(&mut buf, 7);
    stake_credential(&mut buf, cred_fill);
    cbor_uint(&mut buf, deposit);
    buf
}

/// tag 11 — combined stake registration + delegation
/// (NewDeposit(ExplicitInCert)): `[11, credential, pool_hash28, deposit]`.
fn cert_reg_deleg(cred_fill: u8, pool_fill: u8, deposit: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 4);
    cbor_uint(&mut buf, 11);
    stake_credential(&mut buf, cred_fill);
    cbor_bytes(&mut buf, &[pool_fill; 28]);
    cbor_uint(&mut buf, deposit);
    buf
}

/// tag 16 — DRep registration (NewDeposit(ExplicitInCert)):
/// `[16, drep_credential, deposit, anchor?]` — anchor omitted.
fn cert_drep_registration(cred_fill: u8, deposit: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 3);
    cbor_uint(&mut buf, 16);
    stake_credential(&mut buf, cred_fill);
    cbor_uint(&mut buf, deposit);
    buf
}

// --- synthetic balanced corpus -------------------------------------------

/// One synthetic positive corpus entry: a balanced cert/withdrawal-bearing tx
/// over a controlled resolved UTxO and seeded CertState. Every entry MUST be
/// Valid (consumed == produced) under the canonical deposit params.
struct PositiveEntry {
    label: &'static str,
    body: ConwayTxBody,
    utxo: BTreeMap<TxIn, TxOut>,
    state: CertState,
}

/// The synthetic positive corpus. Coverage of conservation-relevant cert/
/// withdrawal shapes, each balanced by construction:
///   - tag 0  legacy stake registration  (NewDeposit = key_deposit param)
///   - tag 7  explicit stake registration (NewDeposit = explicit cert deposit)
///   - tag 16 DRep registration           (NewDeposit = explicit cert deposit)
///   - withdrawal-bearing tx (no cert)     (withdrawal on the consumed side)
///   - tag 11 combined reg + delegation    (NewDeposit = explicit cert deposit)
fn synthetic_positive_corpus() -> Vec<PositiveEntry> {
    let mut corpus = Vec::new();

    // tag 0 — legacy stake registration.
    //   consumed = input(3_000_000)
    //   produced = output(800_000) + fee(200_000) + new_deposit(2_000_000)
    {
        let mut body = base_body(800_000, 200_000);
        body.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
        corpus.push(PositiveEntry {
            label: "tag0_legacy_stake_registration",
            body,
            utxo: utxo_with(&[(the_input(), 3_000_000)]),
            state: CertState::new(),
        });
    }

    // tag 7 — explicit-deposit stake registration (deposit = 2_000_000).
    //   consumed = input(3_000_000)
    //   produced = output(800_000) + fee(200_000) + new_deposit(2_000_000)
    {
        let mut body = base_body(800_000, 200_000);
        body.certs = Some(encode_certs(&[cert_explicit_registration(0x12, 2_000_000)]));
        corpus.push(PositiveEntry {
            label: "tag7_explicit_stake_registration",
            body,
            utxo: utxo_with(&[(the_input(), 3_000_000)]),
            state: CertState::new(),
        });
    }

    // tag 16 — DRep registration (deposit = 500_000_000).
    //   consumed = input(501_000_000)
    //   produced = output(800_000) + fee(200_000) + new_deposit(500_000_000)
    {
        let mut body = base_body(800_000, 200_000);
        body.certs = Some(encode_certs(&[cert_drep_registration(0x13, 500_000_000)]));
        corpus.push(PositiveEntry {
            label: "tag16_drep_registration",
            body,
            utxo: utxo_with(&[(the_input(), 501_000_000)]),
            state: CertState::new(),
        });
    }

    // withdrawal-bearing tx (no cert).
    //   consumed = input(1_000_000) + withdrawal(1_000_000) = 2_000_000
    //   produced = output(1_800_000) + fee(200_000)         = 2_000_000
    {
        let mut body = base_body(1_800_000, 200_000);
        body.withdrawals = Some(encode_withdrawals(&[(0x22, 1_000_000)]));
        corpus.push(PositiveEntry {
            label: "withdrawal_bearing_no_cert",
            body,
            utxo: utxo_with(&[(the_input(), 1_000_000)]),
            state: CertState::new(),
        });
    }

    // tag 11 — combined stake registration + delegation (deposit = 2_000_000),
    // paired with a withdrawal to exercise both consumed-side terms together.
    //   consumed = input(3_000_000) + withdrawal(500_000) = 3_500_000
    //   produced = output(1_300_000) + fee(200_000) + new_deposit(2_000_000)
    //            = 3_500_000
    {
        let mut body = base_body(1_300_000, 200_000);
        body.certs = Some(encode_certs(&[cert_reg_deleg(0x14, 0x77, 2_000_000)]));
        body.withdrawals = Some(encode_withdrawals(&[(0x33, 500_000)]));
        corpus.push(PositiveEntry {
            label: "tag11_reg_deleg_with_withdrawal",
            body,
            utxo: utxo_with(&[(the_input(), 3_000_000)]),
            state: CertState::new(),
        });
    }

    corpus
}

/// Canonical per-tx verdict surface for the synthetic corpus. The guard returns
/// `Result<(), LedgerError>`; the byte-stable comparison surface is one byte per
/// tx: 0x00 = Valid, 0x01 = Invalid (rejected). Every positive entry must encode
/// to 0x00. Determinism of this surface across runs is the replay contract here.
fn verdict_surface(result: &Result<(), LedgerError>) -> u8 {
    match result {
        Ok(()) => 0x00,
        Err(_) => 0x01,
    }
}

/// Run the whole synthetic corpus and return the concatenated verdict surface.
fn run_corpus_surface(corpus: &[PositiveEntry]) -> Vec<u8> {
    let mut surface = Vec::with_capacity(corpus.len());
    for entry in corpus {
        let result = run(&entry.body, &entry.utxo, &entry.state);
        surface.push(verdict_surface(&result));
    }
    surface
}

// --- CE-B3-5 (synthetic portion) tests ------------------------------------

#[test]
fn synthetic_balanced_cert_withdrawal_txs_are_valid() {
    let corpus = synthetic_positive_corpus();
    assert!(
        corpus.len() >= 5,
        "synthetic positive corpus must cover at least 5 cert/withdrawal shapes"
    );
    for entry in &corpus {
        let result = run(&entry.body, &entry.utxo, &entry.state);
        // A balanced cert/withdrawal tx that is rejected here is an S4 accounting
        // bug, not a corpus defect — the harness surfaces it rather than softening.
        assert!(
            result.is_ok(),
            "synthetic balanced entry `{}` must be Valid, got {result:?}",
            entry.label
        );
    }
}

#[test]
fn synthetic_positive_verdict_stream_replays_byte_identical() {
    let first = run_corpus_surface(&synthetic_positive_corpus());
    let second = run_corpus_surface(&synthetic_positive_corpus());
    assert_eq!(
        first, second,
        "synthetic positive verdict stream must be byte-identical across runs"
    );
    // Every entry in the positive corpus is Valid, so the surface is all 0x00.
    assert!(
        first.iter().all(|&b| b == 0x00),
        "every synthetic positive verdict must be Valid (0x00), got {first:?}"
    );
}
