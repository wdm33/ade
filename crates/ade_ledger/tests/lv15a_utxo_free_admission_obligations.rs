// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! LV-15a (DC-TXV-09, DC-VAL-07): admission obligations that need neither the UTxO nor protocol
//! parameters — the validity interval, every network id, bootstrap attribute size, and the header's
//! declared body size. Rejection only.

#![allow(clippy::unwrap_used, clippy::panic)]

use ade_codec::cbor::{self, canonical_width, ContainerEncoding};
use ade_codec::traits::{AdeEncode, CodecContext};
use ade_ledger::error::LedgerError;
use ade_ledger::late_era_validation::check_address_network;
use ade_ledger::rules::apply_block_with_verdicts;
use ade_ledger::state::{mainnet_shelley_schedule, LedgerState};
use ade_ledger::tx_validity::utxo_free::{
    address_network, check_tx_networks, check_utxo_free_tx, check_validity_interval,
    read_bootstrap_attributes, OutputPosition, UtxoFreeRejection,
};
use ade_ledger::tx_validity::verdict::TxValidityError;
use ade_ledger::tx_validity::{tx_validity, TxValidityVerdict};
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, Hash32, SlotNo};

const PREPROD: u8 = 0;
const MAINNET: u8 = 1;

const TIMELOCK_BLOCK: &[u8] = include_bytes!("fixtures/lv13_timelock_block.cbor");
const BOOTSTRAP_BLOCK: &[u8] = include_bytes!("fixtures/lv13_bootstrap_block.cbor");
const REFNATIVE_BLOCK: &[u8] = include_bytes!("fixtures/lv13_refnative_block.cbor");

/// A real preprod bootstrap output address (slot 129,817,028): derivation payload 28 bytes, network
/// magic 1.
const REAL_BOOTSTRAP: &str = "82d818584583581c62f1b7e85815d841f7d14df7f371c0205af4cd70c4e5ed14e97f5b01a201581e581ca77ffa049fbf5686fef5cc09150125bf342415cd206f868beac1c61d024101001a36f92c9a";

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// `[tag24(bytes [root, {attrs}, 0]), crc32]`.
fn byron_address(attrs: &[(u64, Vec<u8>)]) -> Vec<u8> {
    let mut payload = Vec::new();
    cbor::write_array_header(&mut payload, ContainerEncoding::Definite(3, canonical_width(3)));
    cbor::write_bytes_canonical(&mut payload, &[0x11; 28]);
    let n = attrs.len() as u64;
    cbor::write_map_header(&mut payload, ContainerEncoding::Definite(n, canonical_width(n)));
    for (k, v) in attrs {
        cbor::write_uint_canonical(&mut payload, *k);
        cbor::write_bytes_canonical(&mut payload, v);
    }
    cbor::write_uint_canonical(&mut payload, 0);
    let mut a = Vec::new();
    cbor::write_array_header(&mut a, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_tag(&mut a, 24, canonical_width(24));
    cbor::write_bytes_canonical(&mut a, &payload);
    cbor::write_uint_canonical(&mut a, u64::from(crc32(&payload)));
    a
}

/// Key 1's value: the derivation-path payload as a CBOR byte string.
fn derivation_path(len: usize) -> Vec<u8> {
    let mut v = Vec::new();
    cbor::write_bytes_canonical(&mut v, &vec![0x22; len]);
    v
}

fn magic() -> Vec<u8> {
    vec![0x01]
}

fn out(address: Vec<u8>) -> BabbageTxOut {
    BabbageTxOut { address, coin: Coin(2_000_000), multi_asset: None, datum_option: None, script_ref: None }
}

fn enterprise(network: u8) -> Vec<u8> {
    let mut a = vec![0x60 | network];
    a.extend_from_slice(&[0x33; 28]);
    a
}

fn reward_account(network: u8) -> Vec<u8> {
    let mut a = vec![0xE0 | network];
    a.extend_from_slice(&[0x44; 28]);
    a
}

fn withdrawals(accounts: &[Vec<u8>]) -> Vec<u8> {
    let mut b = Vec::new();
    let n = accounts.len() as u64;
    cbor::write_map_header(&mut b, ContainerEncoding::Definite(n, canonical_width(n)));
    for a in accounts {
        cbor::write_bytes_canonical(&mut b, a);
        cbor::write_uint_canonical(&mut b, 0);
    }
    b
}

fn body(network: u8) -> ConwayTxBody {
    ConwayTxBody {
        inputs: [TxIn { tx_hash: Hash32([0xA0; 32]), index: 0 }].into_iter().collect(),
        outputs: vec![out(enterprise(network))],
        fee: Coin(200_000),
        ttl: None,
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

fn inner(envelope: &[u8]) -> Vec<u8> {
    let env = cbor::envelope::decode_block_envelope(envelope).unwrap();
    envelope[env.block_start..env.block_end].to_vec()
}

/// Byte range of header-body item `k` in an inner Conway block `[[header_body, sig], ..]`.
fn header_item(inner: &[u8], k: usize) -> (usize, usize) {
    let mut o = 0;
    for _ in 0..3 {
        cbor::read_array_header(inner, &mut o).unwrap();
    }
    for _ in 0..k {
        cbor::skip_item(inner, &mut o).unwrap();
    }
    cbor::skip_item(inner, &mut o).unwrap()
}

/// Rewrite header-body uint item `k` in place at the same CBOR width.
fn patch_header_uint(inner: &mut [u8], k: usize, value: u64) {
    let (s, e) = header_item(inner, k);
    let mut o = s;
    let (_, width) = cbor::read_uint(inner, &mut o).unwrap();
    let mut enc = Vec::new();
    cbor::write_uint(&mut enc, value, width);
    assert_eq!(enc.len(), e - s, "same width");
    inner[s..e].copy_from_slice(&enc);
}

const HEADER_SLOT: usize = 1;
const HEADER_BODY_SIZE: usize = 6;

fn state_at(slot: u64, track_utxo: bool, network: u8) -> LedgerState {
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = track_utxo;
    state.protocol_params.network_id = network;
    state.conway_deposit_params = Some(ade_ledger::pparams::ConwayOnlyDepositParams {
        drep_deposit: Coin(500_000_000),
        gov_action_deposit: Coin(100_000_000_000),
        drep_activity: 20,
    });
    state.epoch_state.epoch = mainnet_shelley_schedule().locate(SlotNo(slot)).unwrap().epoch;
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));
    state
}

fn apply(state: &LedgerState, inner: &[u8]) -> Result<(), LedgerError> {
    apply_block_with_verdicts(state, CardanoEra::Conway, inner, &mainnet_shelley_schedule()).map(|_| ())
}

fn slot_of(inner: &[u8]) -> u64 {
    ade_codec::conway::decode_conway_block(inner).unwrap().decoded().header.body.slot
}

fn tx_bodies(inner: &[u8]) -> Vec<ConwayTxBody> {
    let decoded = ade_codec::conway::decode_conway_block(inner).unwrap();
    let data = &decoded.decoded().tx_bodies;
    let mut o = 0;
    let n = match cbor::read_array_header(data, &mut o).unwrap() {
        ContainerEncoding::Definite(n, _) => Some(n),
        ContainerEncoding::Indefinite => None,
    };
    let mut out = Vec::new();
    loop {
        match n {
            Some(n) if out.len() as u64 == n => break,
            None if cbor::is_break(data, o).unwrap() => break,
            _ => {}
        }
        out.push(ade_codec::conway::tx::decode_conway_tx_body(data, &mut o).unwrap());
    }
    out
}

#[test]
fn lv15a_the_validity_interval_is_half_open() {
    let mut b = body(PREPROD);
    b.validity_interval_start = Some(SlotNo(100));
    b.ttl = Some(SlotNo(200));
    for (slot, ok) in [(99, false), (100, true), (150, true), (199, true), (200, false), (201, false)] {
        assert_eq!(check_validity_interval(&b, SlotNo(slot)).is_ok(), ok, "slot {slot}");
    }
    assert_eq!(
        check_validity_interval(&b, SlotNo(200)),
        Err(UtxoFreeRejection::OutsideValidityInterval {
            invalid_before: Some(100),
            invalid_hereafter: Some(200),
            slot: 200
        })
    );
    let mut only_start = body(PREPROD);
    only_start.validity_interval_start = Some(SlotNo(100));
    assert!(check_validity_interval(&only_start, SlotNo(99)).is_err());
    assert!(check_validity_interval(&only_start, SlotNo(u64::MAX)).is_ok());
    let mut only_end = body(PREPROD);
    only_end.ttl = Some(SlotNo(200));
    assert!(check_validity_interval(&only_end, SlotNo(0)).is_ok());
    assert!(check_validity_interval(&only_end, SlotNo(200)).is_err());
    assert!(check_validity_interval(&body(PREPROD), SlotNo(u64::MAX)).is_ok());
}

#[test]
fn lv15a_each_network_check_rejects_the_wrong_network() {
    assert_eq!(check_tx_networks(&body(PREPROD), PREPROD), Ok(()));
    assert_eq!(
        check_tx_networks(&body(MAINNET), PREPROD),
        Err(UtxoFreeRejection::WrongNetworkInOutput {
            output: OutputPosition::Output(0),
            network: MAINNET,
            expected: PREPROD
        })
    );

    let mut ret = body(PREPROD);
    ret.collateral_return = Some(out(enterprise(MAINNET)));
    assert_eq!(
        check_tx_networks(&ret, PREPROD),
        Err(UtxoFreeRejection::WrongNetworkInOutput {
            output: OutputPosition::CollateralReturn,
            network: MAINNET,
            expected: PREPROD
        })
    );

    let mut w = body(PREPROD);
    w.withdrawals = Some(withdrawals(&[reward_account(PREPROD), reward_account(MAINNET)]));
    assert_eq!(
        check_tx_networks(&w, PREPROD),
        Err(UtxoFreeRejection::WrongNetworkWithdrawal { network: MAINNET, expected: PREPROD })
    );
    w.withdrawals = Some(withdrawals(&[reward_account(PREPROD)]));
    assert_eq!(check_tx_networks(&w, PREPROD), Ok(()));

    let mut field = body(PREPROD);
    field.network_id = Some(MAINNET);
    assert_eq!(
        check_tx_networks(&field, PREPROD),
        Err(UtxoFreeRejection::WrongNetworkInTxBody { declared: MAINNET, expected: PREPROD })
    );
    field.network_id = Some(PREPROD);
    assert_eq!(check_tx_networks(&field, PREPROD), Ok(()));
}

#[test]
fn lv15a_a_bootstrap_address_network_is_its_magic_attribute() {
    let real = unhex(REAL_BOOTSTRAP);
    // The test's CRC agrees with the chain's, so the rebuilt variants below are well-formed.
    let mut o = 0;
    cbor::read_array_header(&real, &mut o).unwrap();
    cbor::read_tag(&real, &mut o).unwrap();
    let (payload, _) = cbor::read_bytes(&real, &mut o).unwrap();
    let (crc, _) = cbor::read_uint(&real, &mut o).unwrap();
    assert_eq!(u64::from(crc32(&payload)), crc);

    let attrs = read_bootstrap_attributes(&real).unwrap();
    assert!(attrs.has_network_magic);
    assert_eq!((attrs.derivation_payload_len, attrs.unknown_len), (28, 0));
    assert_eq!(address_network(&real), Ok(PREPROD));

    let mut b = body(PREPROD);
    b.outputs.push(out(real.clone()));
    assert_eq!(check_tx_networks(&b, PREPROD), Ok(()));
    assert!(matches!(
        check_tx_networks(&b, MAINNET),
        Err(UtxoFreeRejection::WrongNetworkInOutput { network: PREPROD, .. })
    ));

    // The same shape without the magic is a mainnet address: wrong on preprod.
    let no_magic = byron_address(&[(1, derivation_path(28))]);
    assert_eq!(address_network(&no_magic), Ok(MAINNET));
    let mut c = body(PREPROD);
    c.collateral_return = Some(out(no_magic.clone()));
    assert_eq!(
        check_tx_networks(&c, PREPROD),
        Err(UtxoFreeRejection::WrongNetworkInOutput {
            output: OutputPosition::CollateralReturn,
            network: MAINNET,
            expected: PREPROD
        })
    );

    let garbage = vec![0x82, 0x00, 0x00];
    let mut g = body(PREPROD);
    g.outputs.push(out(garbage.clone()));
    assert_eq!(
        check_tx_networks(&g, PREPROD),
        Err(UtxoFreeRejection::MalformedBootstrapAddress { output: OutputPosition::Output(1) })
    );

    // The full path's output network check agrees, one definition of an address's network.
    assert!(check_address_network(&real, PREPROD).is_ok());
    assert!(check_address_network(&no_magic, PREPROD).is_err());
    assert!(check_address_network(&no_magic, MAINNET).is_ok());
    assert!(check_address_network(&garbage, PREPROD).is_err());
}

#[test]
fn lv15a_bootstrap_attributes_may_total_64_bytes() {
    let at = byron_address(&[(1, derivation_path(40)), (2, magic()), (5, vec![0x55; 24])]);
    let over = byron_address(&[(1, derivation_path(40)), (2, magic()), (5, vec![0x55; 25])]);
    let attrs = read_bootstrap_attributes(&at).unwrap();
    assert_eq!((attrs.derivation_payload_len, attrs.unknown_len), (40, 24));

    let mut b = body(PREPROD);
    b.outputs.push(out(at));
    assert_eq!(check_tx_networks(&b, PREPROD), Ok(()));
    let mut c = body(PREPROD);
    c.outputs.push(out(over));
    assert_eq!(
        check_tx_networks(&c, PREPROD),
        Err(UtxoFreeRejection::BootstrapAttributesTooBig { output: OutputPosition::Output(1), size: 65 })
    );
}

#[test]
fn lv15a_the_declared_body_size_must_equal_the_body() {
    for block in [TIMELOCK_BLOCK, BOOTSTRAP_BLOCK, REFNATIVE_BLOCK] {
        let real = inner(block);
        let slot = slot_of(&real);
        for track in [false, true] {
            assert_eq!(apply(&state_at(slot, track, PREPROD), &real), Ok(()), "the real block applies");
        }
        let (s, _) = header_item(&real, HEADER_BODY_SIZE);
        let mut o = s;
        let (declared, _) = cbor::read_uint(&real, &mut o).unwrap();
        for wrong in [declared - 1, declared + 1] {
            let mut patched = real.clone();
            patch_header_uint(&mut patched, HEADER_BODY_SIZE, wrong);
            for track in [false, true] {
                assert_eq!(
                    apply(&state_at(slot, track, PREPROD), &patched),
                    Err(LedgerError::UtxoFreeObligation(UtxoFreeRejection::WrongBlockBodySize {
                        declared: wrong,
                        actual: declared
                    }))
                );
            }
        }
    }
}

#[test]
fn lv15a_the_block_path_rejects_a_block_outside_a_tx_validity_interval() {
    let mut upper = 0;
    let mut lower = 0;
    for block in [TIMELOCK_BLOCK, BOOTSTRAP_BLOCK, REFNATIVE_BLOCK] {
        let real = inner(block);
        for b in tx_bodies(&real) {
            // At invalid_hereafter the tx is no longer valid; one slot before its start, not yet.
            let moves = [
                b.ttl.map(|h| h.0),
                b.validity_interval_start.map(|s| s.0 - 1),
            ];
            for (i, target) in moves.into_iter().enumerate() {
                let Some(target) = target else { continue };
                let mut patched = real.clone();
                patch_header_uint(&mut patched, HEADER_SLOT, target);
                for track in [false, true] {
                    match apply(&state_at(target, track, PREPROD), &patched) {
                        Err(LedgerError::UtxoFreeObligation(UtxoFreeRejection::OutsideValidityInterval {
                            slot,
                            ..
                        })) => assert_eq!(slot, target),
                        other => panic!("slot {target}: expected OutsideValidityInterval, got {other:?}"),
                    }
                }
                if i == 0 { upper += 1 } else { lower += 1 }
            }
        }
    }
    assert!(upper > 0 && lower > 0, "the fixtures exercise both bounds (upper {upper}, lower {lower})");
}

#[test]
fn lv15a_the_block_path_checks_the_network_in_both_modes() {
    let real = inner(BOOTSTRAP_BLOCK);
    let slot = slot_of(&real);
    for track in [false, true] {
        assert_eq!(apply(&state_at(slot, track, PREPROD), &real), Ok(()));
        assert!(matches!(
            apply(&state_at(slot, track, MAINNET), &real),
            Err(LedgerError::UtxoFreeObligation(UtxoFreeRejection::WrongNetworkInOutput { .. }))
        ));
    }
}

#[test]
fn lv15a_the_standalone_path_checks_the_network() {
    let encode = |b: &ConwayTxBody| {
        let mut buf = Vec::new();
        b.ade_encode(&mut buf, &CodecContext { era: CardanoEra::Conway }).unwrap();
        buf
    };
    let tx = |b: &ConwayTxBody| {
        let mut t = vec![0x84];
        t.extend_from_slice(&encode(b));
        t.extend_from_slice(&[0xa0, 0xf5, 0xf6]);
        t
    };
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.protocol_params.network_id = PREPROD;
    assert!(matches!(tx_validity(&state, &tx(&body(PREPROD))).verdict, TxValidityVerdict::Valid { .. }));
    match tx_validity(&state, &tx(&body(MAINNET))).verdict {
        TxValidityVerdict::Invalid { error, .. } => assert_eq!(
            error,
            TxValidityError::Phase1(LedgerError::UtxoFreeObligation(UtxoFreeRejection::WrongNetworkInOutput {
                output: OutputPosition::Output(0),
                network: MAINNET,
                expected: PREPROD
            }))
        ),
        TxValidityVerdict::Valid { .. } => panic!("a mainnet output is valid on preprod"),
    }
}

#[test]
fn lv15a_the_obligations_compose() {
    let mut b = body(PREPROD);
    b.ttl = Some(SlotNo(10));
    assert!(check_utxo_free_tx(&b, SlotNo(9), PREPROD).is_ok());
    assert!(matches!(
        check_utxo_free_tx(&b, SlotNo(10), PREPROD),
        Err(UtxoFreeRejection::OutsideValidityInterval { .. })
    ));
    b.outputs = vec![out(enterprise(MAINNET))];
    assert!(matches!(
        check_utxo_free_tx(&b, SlotNo(9), PREPROD),
        Err(UtxoFreeRejection::WrongNetworkInOutput { .. })
    ));
}
