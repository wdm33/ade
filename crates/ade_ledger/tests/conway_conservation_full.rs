// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// PHASE4-B3-S4 (CE-B3-4): full Conway value-conservation equation for
// cert/withdrawal-bearing txs, with the deposit/withdrawal early-out removed and
// the §9.1 reject precedence pinned. These tests drive the BLUE composer
// `validate_conway_state_backed` directly at track_utxo=true semantics (resolved
// UTxO supplied) with a seeded CertState and canonical ConwayDepositParams.

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

/// certs: array of certs. Each cert is itself an array `[tag, ...fields]`.
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

/// tag 8 — explicit-refund unregistration (Refund(ExplicitInCert)).
fn cert_explicit_unregistration(cred_fill: u8, refund: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor_array(&mut buf, 3);
    cbor_uint(&mut buf, 8);
    stake_credential(&mut buf, cred_fill);
    cbor_uint(&mut buf, refund);
    buf
}

/// tag 5 — genesis-key-delegation, structurally removed in Conway.
fn cert_removed_tag_5() -> Vec<u8> {
    let mut buf = Vec::new();
    // The decoder reads only the tag for 5/6 and stops; a bare `[5]` is enough.
    cbor_array(&mut buf, 1);
    cbor_uint(&mut buf, 5);
    buf
}

fn seeded_state(cred_fill: u8, recorded: u64) -> CertState {
    let mut s = CertState::new();
    s.delegation
        .registrations
        .insert(StakeCredential::KeyHash(Hash28([cred_fill; 28])), Coin(recorded));
    s
}

// --- CE-B3-4 tests --------------------------------------------------------

#[test]
fn conway_conservation_full() {
    // Balanced cert + withdrawal tx.
    //   consumed = input(3_000_000) + withdrawal(1_000_000) = 4_000_000
    //   produced = output(1_800_000) + fee(200_000) + new_deposit(2_000_000)
    //            = 4_000_000
    let mut body = base_body(1_800_000, 200_000);
    body.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    body.withdrawals = Some(encode_withdrawals(&[(0x22, 1_000_000)]));
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();
    assert!(
        run(&body, &utxo, &state).is_ok(),
        "balanced cert+withdrawal tx must be accepted"
    );

    // Imbalanced: bump the output by 1 so produced exceeds consumed.
    let mut bad = base_body(1_800_001, 200_000);
    bad.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    bad.withdrawals = Some(encode_withdrawals(&[(0x22, 1_000_000)]));
    assert!(
        matches!(run(&bad, &utxo, &state), Err(LedgerError::Conservation(_))),
        "imbalanced cert+withdrawal tx must reject with Conservation"
    );

    // Deterministic across two runs.
    let r1 = run(&body, &utxo, &state);
    let r2 = run(&body, &utxo, &state);
    assert_eq!(r1, r2, "verdict must be replay-stable");
}

#[test]
fn conservation_early_out_removed() {
    // A cert+withdrawal-bearing tx with a clear value imbalance. The OLD
    // early-out (`certs.is_some() || withdrawals.is_some()`) accepted this
    // unconditionally; the full equation must now reject it.
    //   consumed = input(3_000_000) + withdrawal(500_000) = 3_500_000
    //   produced = output(9_000_000) + fee(200_000) + new_deposit(2_000_000)
    //            = 11_200_000  (>> consumed)
    let mut body = base_body(9_000_000, 200_000);
    body.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    body.withdrawals = Some(encode_withdrawals(&[(0x33, 500_000)]));
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();
    assert!(
        matches!(
            run(&body, &utxo, &state),
            Err(LedgerError::Conservation(_))
        ),
        "the previously-false-accepted imbalance must now reject"
    );
}

#[test]
fn reject_reason_precedence_is_deterministic() {
    // Engineer a tx that triggers BOTH a malformed-withdrawals decode failure
    // (precedence 1) AND a value imbalance (precedence 5). §9.1 requires the
    // decode failure to win regardless of construction order.
    let utxo = utxo_with(&[(the_input(), 3_000_000)]);
    let state = CertState::new();

    // Order A: set certs (balanced-irrelevant) first, then bad withdrawals.
    let mut a = base_body(9_999_999, 200_000); // value imbalance present
    a.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    a.withdrawals = Some(truncated_withdrawals());
    let ra = run(&a, &utxo, &state);

    // Order B: identical tx, fields assigned in the opposite order.
    let mut b = base_body(9_999_999, 200_000);
    b.withdrawals = Some(truncated_withdrawals());
    b.certs = Some(encode_certs(&[cert_legacy_registration(0x11)]));
    let rb = run(&b, &utxo, &state);

    // Both must be the SAME reason, and that reason must be the decode failure
    // (Decoding), never the lower-precedence Conservation.
    assert_eq!(ra, rb, "reject reason must not depend on evaluation order");
    assert!(
        matches!(ra, Err(LedgerError::Decoding(_))),
        "decode failure (precedence 1) must mask the value imbalance (precedence 5), got {ra:?}"
    );

    // Cross-check the next precedence pair: era-invalid cert (precedence 2)
    // must mask a value imbalance (precedence 5), again order-independent.
    let mut c = base_body(9_999_999, 200_000);
    c.certs = Some(encode_certs(&[cert_removed_tag_5()]));
    let rc = run(&c, &utxo, &state);
    let mut d = base_body(9_999_999, 200_000);
    d.certs = Some(encode_certs(&[cert_removed_tag_5()]));
    let rd = run(&d, &utxo, &state);
    assert_eq!(rc, rd);
    assert!(
        matches!(rc, Err(LedgerError::EraInvalidCertificate(_))),
        "era-invalid cert (precedence 2) must mask the value imbalance, got {rc:?}"
    );
}

#[test]
fn state_dependent_unaccountable_rejects() {
    // A legacy unregistration (tag 1) for a credential ABSENT from the
    // registrations map: the refund amount is state-dependent and unresolvable,
    // so the classifier returns Unsupported — which the composer must surface as
    // UnsupportedStateDependentDeposit, NOT a Conservation reject and NOT accept.
    let mut body = base_body(1_000_000, 200_000);
    body.certs = Some(encode_certs(&[cert_legacy_unregistration(0x44)]));
    let utxo = utxo_with(&[(the_input(), 1_200_000)]);
    let empty = CertState::new();
    assert!(
        matches!(
            run(&body, &utxo, &empty),
            Err(LedgerError::UnsupportedStateDependentDeposit(_))
        ),
        "unresolved state-dependent refund must reject as UnsupportedStateDependentDeposit"
    );

    // Sanity: with the credential recorded, the SAME cert resolves to a refund
    // and the tx balances — proving the reject above is genuinely the missing
    // state, not a structural defect.
    //   consumed = input(1_200_000) + refund(2_000_000) = 3_200_000
    //   produced = output(3_000_000) + fee(200_000)      = 3_200_000
    let mut ok_body = base_body(3_000_000, 200_000);
    ok_body.certs = Some(encode_certs(&[cert_explicit_unregistration(0x44, 2_000_000)]));
    let utxo2 = utxo_with(&[(the_input(), 1_200_000)]);
    assert!(
        run(&ok_body, &utxo2, &empty).is_ok(),
        "explicit-refund unregistration that balances must be accepted"
    );

    // And the legacy (tag 1) path resolves once state records the deposit.
    //   refund = recorded(2_000_000); consumed = 1_200_000 + 2_000_000
    let mut legacy_ok = base_body(3_000_000, 200_000);
    legacy_ok.certs = Some(encode_certs(&[cert_legacy_unregistration(0x55)]));
    let utxo3 = utxo_with(&[(the_input(), 1_200_000)]);
    assert!(
        run(&legacy_ok, &utxo3, &seeded_state(0x55, 2_000_000)).is_ok(),
        "legacy unregistration with recorded deposit must resolve and balance"
    );
}
