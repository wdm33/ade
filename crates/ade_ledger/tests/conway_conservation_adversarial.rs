// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-B3-S6 (CE-B3-6): adversarial Conway value-conservation corpus. Every
// family below MUST reject — with the §9.1 precedence class, never accepted.
// The corpus is built from synthetic / controlled cert/withdrawal-bearing txs
// over a controlled resolved UTxO (track_utxo=true semantics), seeded CertState,
// and canonical ConwayDepositParams — the same pattern as conway_conservation_full
// (B3-S4) and the B2 family-(B) controlled-UTxO synthetic-mutation harness. The
// real epoch-576 snapshot is absent in this environment, so no real on-chain
// input resolution is attempted (corpus/validity/conway_epoch576/README.md).
//
// This is a TEST-ONLY (GREEN) slice exercising the BLUE guard
// `validate_conway_state_backed`; it changes no BLUE logic.

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
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, TxIn};
use ade_types::{Hash28, Hash32, SlotNo};

const MAINNET_PERCENT: u16 = 150;
const MAINNET_NET: u8 = 1;
const PV_CONWAY: u16 = 9;
const KEY_DEPOSIT: u64 = 2_000_000;

// --- canonical params ----------------------------------------------------

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

fn the_input() -> TxIn {
    TxIn {
        tx_hash: Hash32([0x01; 32]),
        index: 0,
    }
}

fn base_body(output_coin: u64, fee: u64) -> ConwayTxBody {
    let mut inputs = BTreeSet::new();
    inputs.insert(the_input());
    ConwayTxBody {
        inputs,
        outputs: vec![BabbageTxOut {
            address: mainnet_addr(),
            coin: Coin(output_coin),
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

// --- minimal CBOR builders -----------------------------------------------

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

/// withdrawals: definite map { 29-byte reward account => coin }
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

fn truncated_withdrawals() -> Vec<u8> {
    // Declares a 1-entry map then ends before the value — a structured decode
    // failure (highest-precedence reject).
    let mut buf = Vec::new();
    cbor_map(&mut buf, 1);
    let mut acct = [0u8; 29];
    acct[0] = 0xe1;
    cbor_bytes(&mut buf, &acct);
    // value omitted
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

/// tag 0 — legacy registration (implicit key_deposit, NewDeposit).
fn cert_legacy_registration(cred_fill: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 2);
    cbor_uint(&mut buf, 0);
    stake_credential(&mut buf, cred_fill);
    buf
}

/// tag 1 — legacy unregistration (state-dependent refund).
fn cert_legacy_unregistration(cred_fill: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 2);
    cbor_uint(&mut buf, 1);
    stake_credential(&mut buf, cred_fill);
    buf
}

/// tag 5 — genesis-key-delegation, structurally removed in Conway.
fn cert_removed_tag_5() -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 1);
    cbor_uint(&mut buf, 5);
    buf
}

/// tag >= 19 — outside the Conway certificate grammar; decode failure.
fn cert_unknown_tag(tag: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 2);
    cbor_uint(&mut buf, tag);
    stake_credential(&mut buf, 0x99);
    buf
}

fn seeded_state(cred_fill: u8, recorded: u64) -> CertState {
    let mut s = CertState::new();
    s.delegation
        .registrations
        .insert(StakeCredential(Hash28([cred_fill; 28])), Coin(recorded));
    s
}

// -------------------------------------------------------------------------
// Family 1 — value-imbalanced via deposit/refund mutation.
// -------------------------------------------------------------------------

#[test]
fn adversarial_imbalanced_via_deposit() {
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();

    // Balanced control: legacy registration adds new_deposit(key_deposit).
    //   consumed = input(3_000_000)
    //   produced = output(800_000) + fee(200_000) + new_deposit(2_000_000)
    let mut control = base_body(800_000, 200_000);
    control.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    assert!(
        run(&control, &utxo, &state).is_ok(),
        "balanced cert-bearing control must be accepted (proves the mutation, not the shape)"
    );

    // Mutation A: drop the registration cert entirely so the new_deposit term
    // vanishes while outputs assume it was charged → produced < consumed.
    let mut no_cert = base_body(800_000, 200_000);
    no_cert.certs = None;
    assert!(
        matches!(run(&no_cert, &utxo, &state), Err(LedgerError::Conservation(_))),
        "removing the deposit-bearing cert must reject as Conservation, got {:?}",
        run(&no_cert, &utxo, &state)
    );

    // Mutation B: keep the cert but bump the output by 1 so produced exceeds
    // consumed by the deposit-accounted equation.
    let mut bumped = base_body(800_001, 200_000);
    bumped.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    assert!(
        matches!(run(&bumped, &utxo, &state), Err(LedgerError::Conservation(_))),
        "deposit-mutated imbalance must reject as Conservation, got {:?}",
        run(&bumped, &utxo, &state)
    );
}

// -------------------------------------------------------------------------
// Family 2 — value-imbalanced via withdrawal mutation.
// -------------------------------------------------------------------------

#[test]
fn adversarial_imbalanced_via_withdrawal() {
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();

    // Balanced control: withdrawal(1_000_000) is a consumed term.
    //   consumed = input(3_000_000) + withdrawal(1_000_000) = 4_000_000
    //   produced = output(3_800_000) + fee(200_000)         = 4_000_000
    let mut control = base_body(3_800_000, 200_000);
    control.withdrawals = Some(encode_withdrawals(&[(0x22, 1_000_000)]));
    assert!(
        run(&control, &utxo, &state).is_ok(),
        "balanced withdrawal-bearing control must be accepted"
    );

    // Mutation: shrink the withdrawal so consumed falls below produced.
    let mut shrunk = base_body(3_800_000, 200_000);
    shrunk.withdrawals = Some(encode_withdrawals(&[(0x22, 1)]));
    assert!(
        matches!(run(&shrunk, &utxo, &state), Err(LedgerError::Conservation(_))),
        "withdrawal-mutated imbalance must reject as Conservation, got {:?}",
        run(&shrunk, &utxo, &state)
    );
}

// -------------------------------------------------------------------------
// Family 3 — unknown cert tag (>= 19): decode failure, precedence over
// conservation.
// -------------------------------------------------------------------------

#[test]
fn adversarial_unknown_cert_tag_rejects_as_decode() {
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();

    // The body is ALSO grossly value-imbalanced. §9.1 precedence 1 (decode)
    // must mask precedence 5 (conservation): the verdict is a decode failure.
    let mut body = base_body(9_999_999, 200_000);
    body.certs = Some(encode_certs(&[cert_unknown_tag(19)]));
    assert!(
        matches!(run(&body, &utxo, &state), Err(LedgerError::Decoding(_))),
        "unknown cert tag must reject as Decoding (masking the imbalance), got {:?}",
        run(&body, &utxo, &state)
    );

    // A higher unknown tag behaves identically.
    let mut body2 = base_body(9_999_999, 200_000);
    body2.certs = Some(encode_certs(&[cert_unknown_tag(99)]));
    assert!(
        matches!(run(&body2, &utxo, &state), Err(LedgerError::Decoding(_))),
        "unknown cert tag 99 must reject as Decoding, got {:?}",
        run(&body2, &utxo, &state)
    );
}

// -------------------------------------------------------------------------
// Family 4 — removed tag (5/6): era-invalid cert, precedence over
// conservation.
// -------------------------------------------------------------------------

#[test]
fn adversarial_removed_tag_rejects_as_era_invalid() {
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();

    // Also value-imbalanced; §9.1 precedence 2 (era-invalid) must mask
    // precedence 5 (conservation).
    let mut body = base_body(9_999_999, 200_000);
    body.certs = Some(encode_certs(&[cert_removed_tag_5()]));
    assert!(
        matches!(run(&body, &utxo, &state), Err(LedgerError::EraInvalidCertificate(_))),
        "removed tag 5 must reject as EraInvalidCertificate (masking the imbalance), got {:?}",
        run(&body, &utxo, &state)
    );
}

// -------------------------------------------------------------------------
// Family 5 — truncated/malformed withdrawals: decode failure, precedence
// over conservation.
// -------------------------------------------------------------------------

#[test]
fn adversarial_truncated_withdrawals_rejects_as_decode() {
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();

    // Also value-imbalanced; §9.1 precedence 1 (decode) must mask precedence 5.
    let mut body = base_body(9_999_999, 200_000);
    body.withdrawals = Some(truncated_withdrawals());
    assert!(
        matches!(run(&body, &utxo, &state), Err(LedgerError::Decoding(_))),
        "truncated withdrawals must reject as Decoding (masking the imbalance), got {:?}",
        run(&body, &utxo, &state)
    );
}

// -------------------------------------------------------------------------
// Family 6 — mis-charged pool deposit / state-dependent unaccountable:
// legacy unregistration (tag 1) for a credential absent from registrations →
// UnsupportedStateDependentDeposit (NOT accepted, NOT Conservation).
// -------------------------------------------------------------------------

#[test]
fn adversarial_state_dependent_unaccountable_rejects() {
    // Credential 0x44 is NOT in the (empty) registrations map, so the refund is
    // unresolvable. The classifier returns Unsupported; the composer surfaces it
    // ahead of any conservation reject.
    let mut body = base_body(1_000_000, 200_000);
    body.certs = Some(encode_certs(&[cert_legacy_unregistration(0x44)]));
    let utxo = utxo_with(&[(the_input(), 1_200_000)]);
    let empty = CertState::new();
    assert!(
        matches!(
            run(&body, &utxo, &empty),
            Err(LedgerError::UnsupportedStateDependentDeposit(_))
        ),
        "unresolved state-dependent refund must reject as UnsupportedStateDependentDeposit, got {:?}",
        run(&body, &utxo, &empty)
    );

    // Control: with the credential recorded, the SAME legacy cert resolves to a
    // refund and the tx balances — proving the reject is the missing state, not
    // a structural defect.
    //   consumed = input(1_200_000) + refund(2_000_000) = 3_200_000
    //   produced = output(3_000_000) + fee(200_000)      = 3_200_000
    let mut ok_body = base_body(3_000_000, 200_000);
    ok_body.certs = Some(encode_certs(&[cert_legacy_unregistration(0x55)]));
    let utxo2 = utxo_with(&[(the_input(), 1_200_000)]);
    assert!(
        run(&ok_body, &utxo2, &seeded_state(0x55, 2_000_000)).is_ok(),
        "legacy unregistration with recorded deposit must resolve and balance"
    );
}

// -------------------------------------------------------------------------
// No-false-accept: across the WHOLE adversarial set, no mutation is Valid.
// -------------------------------------------------------------------------

/// Every adversarial (tx, utxo, state) the corpus exercises. A single shared
/// list so the no-false-accept and determinism sweeps see the identical set.
fn adversarial_corpus() -> Vec<(ConwayTxBody, BTreeMap<TxIn, TxOut>, CertState)> {
    let utxo3 = utxo_with(&[(the_input(), 3_000_000)]);
    let utxo12 = utxo_with(&[(the_input(), 1_200_000)]);
    let empty = CertState::new();
    let mut corpus = Vec::new();

    // F1: deposit-mutated (cert dropped).
    let mut f1a = base_body(800_000, 200_000);
    f1a.certs = None;
    corpus.push((f1a, utxo3.clone(), empty.clone()));
    // F1: deposit-mutated (output bumped).
    let mut f1b = base_body(800_001, 200_000);
    f1b.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    corpus.push((f1b, utxo3.clone(), empty.clone()));

    // F2: withdrawal-mutated.
    let mut f2 = base_body(3_800_000, 200_000);
    f2.withdrawals = Some(encode_withdrawals(&[(0x22, 1)]));
    corpus.push((f2, utxo3.clone(), empty.clone()));

    // F3: unknown cert tag.
    let mut f3 = base_body(9_999_999, 200_000);
    f3.certs = Some(encode_certs(&[cert_unknown_tag(19)]));
    corpus.push((f3, utxo3.clone(), empty.clone()));

    // F4: removed tag 5.
    let mut f4 = base_body(9_999_999, 200_000);
    f4.certs = Some(encode_certs(&[cert_removed_tag_5()]));
    corpus.push((f4, utxo3.clone(), empty.clone()));

    // F5: truncated withdrawals.
    let mut f5 = base_body(9_999_999, 200_000);
    f5.withdrawals = Some(truncated_withdrawals());
    corpus.push((f5, utxo3.clone(), empty.clone()));

    // F6: state-dependent unaccountable.
    let mut f6 = base_body(1_000_000, 200_000);
    f6.certs = Some(encode_certs(&[cert_legacy_unregistration(0x44)]));
    corpus.push((f6, utxo12, empty));

    corpus
}

#[test]
fn adversarial_no_false_accept() {
    for (i, (body, utxo, state)) in adversarial_corpus().into_iter().enumerate() {
        let verdict = run(&body, &utxo, &state);
        assert!(
            verdict.is_err(),
            "adversarial corpus entry {i} must NOT be accepted, got {verdict:?}"
        );
    }
}

#[test]
fn adversarial_verdicts_replay_byte_identical() {
    let corpus = adversarial_corpus();
    let run1: Vec<Result<(), LedgerError>> = corpus
        .iter()
        .map(|(b, u, s)| run(b, u, s))
        .collect();
    let run2: Vec<Result<(), LedgerError>> = corpus
        .iter()
        .map(|(b, u, s)| run(b, u, s))
        .collect();
    assert_eq!(
        run1, run2,
        "adversarial verdict stream must replay byte-identically across two runs"
    );
}
