// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! LV-14 (DC-TXV-08): every supplied vkey and bootstrap witness verifies, required or not
//! (cardano-ledger `validateVerifiedWits`), separately from required-signer coverage.

#![allow(clippy::unwrap_used, clippy::panic)]

use ade_codec::cbor::{self, canonical_width, ContainerEncoding};
use ade_codec::traits::{AdeEncode, CodecContext};
use ade_ledger::error::LedgerError;
use ade_ledger::rules::apply_block_with_verdicts;
use ade_ledger::state::{mainnet_shelley_schedule, LedgerState};
use ade_ledger::tx_validity::verdict::{TxRejectClass, TxValidityError};
use ade_ledger::tx_validity::witness::{WitnessClosureError, WitnessField};
use ade_ledger::tx_validity::{tx_validity, verify_supplied_witnesses, TxValidityVerdict, VKeyWitnessRef};
use ade_ledger::utxo::TxOut;
use ade_ledger::witness::{bootstrap_key_hash, decode_witness_info_single, BootstrapWitnessRef};
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, Hash28, Hash32, SlotNo};

use ed25519_dalek::{Signer, SigningKey};

struct Key {
    signing: SigningKey,
    vkey: Vec<u8>,
    hash: Hash28,
}

fn key(seed: u8) -> Key {
    let signing = SigningKey::from_bytes(&[seed; 32]);
    let vkey = signing.verifying_key().to_bytes().to_vec();
    let hash = ade_crypto::blake2b_224(&vkey);
    Key { signing, vkey, hash }
}

fn sign(k: &Key, msg: &[u8]) -> Vec<u8> {
    k.signing.sign(msg).to_bytes().to_vec()
}

fn key_address(h: &Hash28) -> Vec<u8> {
    let mut a = vec![0x61];
    a.extend_from_slice(&h.0);
    a
}

fn input() -> TxIn {
    TxIn { tx_hash: Hash32([0xA0; 32]), index: 0 }
}

fn body() -> ConwayTxBody {
    ConwayTxBody {
        inputs: [input()].into_iter().collect(),
        outputs: vec![BabbageTxOut {
            address: key_address(&Hash28([0x99; 28])),
            coin: Coin(800_000),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        }],
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

fn encode(b: &ConwayTxBody) -> Vec<u8> {
    let mut buf = Vec::new();
    b.ade_encode(&mut buf, &CodecContext { era: CardanoEra::Conway }).unwrap();
    buf
}

fn arr(buf: &mut Vec<u8>, n: usize) {
    cbor::write_array_header(buf, ContainerEncoding::Definite(n as u64, canonical_width(n as u64)));
}

/// `{0: [[vkey, sig], ..], 1: [native scripts]?}`.
fn witness_set(vkeys: &[(&[u8], &[u8])], native: &[Vec<u8>]) -> Vec<u8> {
    let mut b = Vec::new();
    let n = 1 + usize::from(!native.is_empty());
    cbor::write_map_header(&mut b, ContainerEncoding::Definite(n as u64, canonical_width(n as u64)));
    cbor::write_uint_canonical(&mut b, 0);
    arr(&mut b, vkeys.len());
    for (v, s) in vkeys {
        arr(&mut b, 2);
        cbor::write_bytes_canonical(&mut b, v);
        cbor::write_bytes_canonical(&mut b, s);
    }
    if !native.is_empty() {
        cbor::write_uint_canonical(&mut b, 1);
        arr(&mut b, native.len());
        for s in native {
            b.extend_from_slice(s);
        }
    }
    b
}

fn tx(body_bytes: &[u8], ws: &[u8]) -> Vec<u8> {
    let mut t = vec![0x84];
    t.extend_from_slice(body_bytes);
    t.extend_from_slice(ws);
    t.extend_from_slice(&[0xf5, 0xf6]);
    t
}

/// Full mode: the input resolves to `payer`'s key address, so `payer` is a required signer.
fn full_state(payer: &Hash28) -> LedgerState {
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = true;
    state.conway_deposit_params = Some(ade_ledger::pparams::ConwayOnlyDepositParams {
        drep_deposit: Coin(500_000_000),
        gov_action_deposit: Coin(100_000_000_000),
        drep_activity: 20,
    });
    let out = BabbageTxOut {
        address: key_address(payer),
        coin: Coin(1_000_000),
        multi_asset: None,
        datum_option: None,
        script_ref: None,
    };
    let mut raw = Vec::new();
    out.ade_encode(&mut raw, &CodecContext { era: CardanoEra::Conway }).unwrap();
    state
        .utxo_state
        .utxos
        .insert(input(), TxOut::AlonzoPlus { raw, address: key_address(payer), coin: Coin(1_000_000) });
    state
}

fn reduced_state() -> LedgerState {
    LedgerState::new(CardanoEra::Conway)
}

fn witness_error(state: &LedgerState, t: &[u8]) -> Option<(TxRejectClass, TxValidityError)> {
    match tx_validity(state, t).verdict {
        TxValidityVerdict::Invalid { class, error } => Some((class, error)),
        TxValidityVerdict::Valid { .. } => None,
    }
}

// ---------------------------------------------------------------------------
// CE-LV14-1: a bad signature on a non-required witness
// ---------------------------------------------------------------------------

#[test]
fn lv14_a_bad_signature_on_a_non_required_witness_rejects_in_both_modes() {
    let (payer, extra) = (key(0x11), key(0x22));
    let b = encode(&body());
    let id = ade_crypto::blake2b_256(&b);
    let good = sign(&payer, &id.0);
    let bad = sign(&extra, b"not this body");
    let t = tx(&b, &witness_set(&[(&payer.vkey, &good), (&extra.vkey, &bad)], &[]));
    let expected = TxValidityError::Witness(WitnessClosureError::InvalidSuppliedWitnessSignature {
        key_hash: extra.hash.clone(),
    });
    for state in [full_state(&payer.hash), reduced_state()] {
        assert_eq!(witness_error(&state, &t), Some((TxRejectClass::WitnessInvalid, expected.clone())));
    }
    // The same tx with that witness's signature made good is accepted.
    let fixed = sign(&extra, &id.0);
    let t = tx(&b, &witness_set(&[(&payer.vkey, &good), (&extra.vkey, &fixed)], &[]));
    for state in [full_state(&payer.hash), reduced_state()] {
        assert_eq!(witness_error(&state, &t), None);
    }
}

// ---------------------------------------------------------------------------
// CE-LV14-3: required coverage stays its own check
// ---------------------------------------------------------------------------

#[test]
fn lv14_required_signer_failures_keep_their_own_errors() {
    let payer = key(0x11);
    let b = encode(&body());
    let bad = sign(&payer, b"not this body");
    let t = tx(&b, &witness_set(&[(&payer.vkey, &bad)], &[]));
    assert_eq!(
        witness_error(&full_state(&payer.hash), &t).map(|(_, e)| e),
        Some(TxValidityError::Witness(WitnessClosureError::InvalidWitnessSignature { key_hash: payer.hash.clone() }))
    );
    let t = tx(&b, &witness_set(&[], &[]));
    assert!(matches!(
        witness_error(&full_state(&payer.hash), &t).map(|(_, e)| e),
        Some(TxValidityError::Witness(WitnessClosureError::MissingRequiredSigner { .. }))
    ));
    // In reduced mode the payer is not a derivable requirement, but its bad signature is still caught,
    // by the supplied-witness check.
    let t = tx(&b, &witness_set(&[(&payer.vkey, &bad)], &[]));
    assert_eq!(
        witness_error(&reduced_state(), &t).map(|(_, e)| e),
        Some(TxValidityError::Witness(WitnessClosureError::InvalidSuppliedWitnessSignature { key_hash: payer.hash }))
    );
}

// ---------------------------------------------------------------------------
// CE-LV14-4: malformed fields
// ---------------------------------------------------------------------------

#[test]
fn lv14_wrong_size_keys_and_signatures_are_malformed() {
    let k = key(0x33);
    let h = Hash32([7; 32]);
    let sig = sign(&k, &h.0);
    let short_key = VKeyWitnessRef { vkey: k.vkey[..31].to_vec(), signature: sig.clone() };
    let short_sig = VKeyWitnessRef { vkey: k.vkey.clone(), signature: sig[..63].to_vec() };
    assert!(matches!(
        verify_supplied_witnesses(&h, &[short_key], &[]),
        Err(WitnessClosureError::MalformedWitnessField { which: WitnessField::VerificationKey, .. })
    ));
    assert!(matches!(
        verify_supplied_witnesses(&h, &[short_sig], &[]),
        Err(WitnessClosureError::MalformedWitnessField { which: WitnessField::Signature, .. })
    ));
    let boot = |vkey: Vec<u8>, signature: Vec<u8>| BootstrapWitnessRef {
        vkey,
        signature,
        chain_code: vec![0; 32],
        attributes: vec![0xa0],
    };
    assert!(matches!(
        verify_supplied_witnesses(&h, &[], &[boot(k.vkey[..31].to_vec(), sig.clone())]),
        Err(WitnessClosureError::MalformedWitnessField { which: WitnessField::VerificationKey, .. })
    ));
    assert!(matches!(
        verify_supplied_witnesses(&h, &[], &[boot(k.vkey.clone(), sig[..63].to_vec())]),
        Err(WitnessClosureError::MalformedWitnessField { which: WitnessField::Signature, .. })
    ));
    assert_eq!(verify_supplied_witnesses(&h, &[], &[boot(k.vkey.clone(), sig)]), Ok(()));
}

// ---------------------------------------------------------------------------
// CE-LV14-5: a native script satisfied only by a badly signed key
// ---------------------------------------------------------------------------

#[test]
fn lv14_a_native_script_on_a_badly_signed_key_does_not_carry_the_tx() {
    let (payer, signer) = (key(0x11), key(0x44));
    let mut script = Vec::new();
    arr(&mut script, 2);
    cbor::write_uint_canonical(&mut script, 0);
    cbor::write_bytes_canonical(&mut script, &signer.hash.0);
    let mut pre = vec![0x00];
    pre.extend_from_slice(&script);
    let policy = ade_crypto::blake2b_224(&pre);
    let mut bd = body();
    let mut mint = Vec::new();
    cbor::write_map_header(&mut mint, ContainerEncoding::Definite(1, canonical_width(1)));
    cbor::write_bytes_canonical(&mut mint, &policy.0);
    cbor::write_map_header(&mut mint, ContainerEncoding::Definite(1, canonical_width(1)));
    cbor::write_bytes_canonical(&mut mint, &[]);
    cbor::write_uint_canonical(&mut mint, 1);
    bd.mint = Some(mint);
    let b = encode(&bd);
    let id = ade_crypto::blake2b_256(&b);
    let good_payer = sign(&payer, &id.0);
    // The script passes by membership; the signer's signature is over the wrong message.
    let bad = sign(&signer, b"not this body");
    let t = tx(&b, &witness_set(&[(&payer.vkey, &good_payer), (&signer.vkey, &bad)], &[script.clone()]));
    let w = decode_witness_info_single(&witness_set(&[(&signer.vkey, &bad)], &[script])).unwrap();
    assert_eq!(
        ade_ledger::tx_validity::script_witnesses::reduced_verdict(&bd, &w),
        ade_ledger::tx_validity::script_witnesses::ReducedScriptVerdict::Undetermined,
        "the script itself passes on key-hash membership"
    );
    assert_eq!(
        witness_error(&reduced_state(), &t).map(|(_, e)| e),
        Some(TxValidityError::Witness(WitnessClosureError::InvalidSuppliedWitnessSignature { key_hash: signer.hash }))
    );
}

// ---------------------------------------------------------------------------
// Real preprod transactions: CE-LV14-2, CE-LV14-6
// ---------------------------------------------------------------------------

const TIMELOCK_BLOCK: &[u8] = include_bytes!("fixtures/lv13_timelock_block.cbor");
const BOOTSTRAP_BLOCK: &[u8] = include_bytes!("fixtures/lv13_bootstrap_block.cbor");
const BOOTSTRAP_TX: usize = 1;

fn inner(envelope: &[u8]) -> Vec<u8> {
    let env = ade_codec::cbor::envelope::decode_block_envelope(envelope).unwrap();
    envelope[env.block_start..env.block_end].to_vec()
}

fn elements(data: &[u8]) -> Vec<(usize, usize)> {
    let mut o = 0;
    let n = match cbor::read_array_header(data, &mut o).unwrap() {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => panic!("fixtures use definite arrays"),
    };
    (0..n).map(|_| cbor::skip_item(data, &mut o).unwrap()).collect()
}

/// (body bytes, witness-set bytes) of tx `i`.
fn real_tx(envelope: &[u8], i: usize) -> (Vec<u8>, Vec<u8>) {
    let inner = inner(envelope);
    let block = ade_codec::conway::decode_conway_block(&inner).unwrap();
    let blk = block.decoded();
    let (bs, be) = elements(&blk.tx_bodies)[i];
    let (ws, we) = elements(&blk.witness_sets)[i];
    (blk.tx_bodies[bs..be].to_vec(), blk.witness_sets[ws..we].to_vec())
}

fn block_state(inner: &[u8]) -> LedgerState {
    let slot = ade_codec::conway::decode_conway_block(inner).unwrap().decoded().header.body.slot;
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.epoch_state.epoch = mainnet_shelley_schedule().locate(SlotNo(slot)).unwrap().epoch;
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));
    state
}

/// Flip the last byte of the first 64-byte signature inside `ws` that sits in `block`.
fn flip_a_signature(block: &mut [u8], ws: &[u8], after: &[u8]) -> usize {
    let at = block.windows(ws.len()).position(|w| w == ws).unwrap();
    // A signature is a 64-byte bstr (0x58 0x40) following `after` (a 32-byte key's bytes).
    let k = ws.windows(after.len()).position(|w| w == after).unwrap() + after.len();
    assert_eq!(&ws[k..k + 2], &[0x58, 0x40], "a 64-byte signature follows the key");
    let pos = at + k + 2 + 63;
    block[pos] ^= 0x01;
    pos
}

#[test]
fn lv14_a_real_bootstrap_witness_verifies_and_a_flipped_one_rejects_in_both_paths() {
    let (body_bytes, ws) = real_tx(BOOTSTRAP_BLOCK, BOOTSTRAP_TX);
    let w = decode_witness_info_single(&ws).unwrap();
    assert!(!w.bootstrap_witnesses.is_empty());
    let id = ade_crypto::blake2b_256(&body_bytes);
    let vk: Vec<VKeyWitnessRef> = ade_ledger::shelley::decode_conway_vkey_witness_set_single(&ws)
        .unwrap()
        .into_iter()
        .map(|x| VKeyWitnessRef { vkey: x.vkey, signature: x.signature })
        .collect();
    assert_eq!(verify_supplied_witnesses(&id, &vk, &w.bootstrap_witnesses), Ok(()));

    let boot = &w.bootstrap_witnesses[0];
    let expected = WitnessClosureError::InvalidBootstrapWitnessSignature {
        key_hash: bootstrap_key_hash(&boot.vkey, &boot.chain_code, &boot.attributes),
    };
    let mut envelope = BOOTSTRAP_BLOCK.to_vec();
    flip_a_signature(&mut envelope, &ws, &boot.vkey);
    let bad_inner = inner(&envelope);
    let state = block_state(&bad_inner);
    let first = apply_block_with_verdicts(&state, CardanoEra::Conway, &bad_inner, &mainnet_shelley_schedule()).err();
    assert_eq!(first, Some(LedgerError::WitnessClosure(expected.clone())));
    let again = apply_block_with_verdicts(&state, CardanoEra::Conway, &bad_inner, &mainnet_shelley_schedule()).err();
    assert_eq!(first, again, "replay gives the same structured error");

    let (b2, ws2) = real_tx(&envelope, BOOTSTRAP_TX);
    assert_eq!(
        witness_error(&state, &tx(&b2, &ws2)).map(|(_, e)| e),
        Some(TxValidityError::Witness(expected))
    );
    // The unmodified block applies.
    let good = inner(BOOTSTRAP_BLOCK);
    assert!(apply_block_with_verdicts(&block_state(&good), CardanoEra::Conway, &good, &mainnet_shelley_schedule()).is_ok());
}

#[test]
fn lv14_a_real_vkey_signature_flipped_gives_the_same_error_on_both_paths() {
    let (body_bytes, ws) = real_tx(TIMELOCK_BLOCK, 0);
    let vk = ade_ledger::shelley::decode_conway_vkey_witness_set_single(&ws).unwrap();
    assert!(!vk.is_empty());
    let target = vk[0].vkey.clone();
    let mut envelope = TIMELOCK_BLOCK.to_vec();
    flip_a_signature(&mut envelope, &ws, &target);
    let bad_inner = inner(&envelope);
    let state = block_state(&bad_inner);
    let block_err = apply_block_with_verdicts(&state, CardanoEra::Conway, &bad_inner, &mainnet_shelley_schedule())
        .err()
        .expect("the flipped signature must reject the block");
    let (b2, ws2) = real_tx(&envelope, 0);
    assert_eq!(b2, body_bytes, "the body is untouched");
    let LedgerError::WitnessClosure(e) = &block_err else {
        panic!("expected a witness rejection, got {block_err:?}");
    };
    assert!(matches!(
        e,
        WitnessClosureError::InvalidWitnessSignature { .. } | WitnessClosureError::InvalidSuppliedWitnessSignature { .. }
    ));
    assert_eq!(
        witness_error(&state, &tx(&b2, &ws2)).map(|(_, e)| e),
        Some(TxValidityError::Witness(e.clone()))
    );
}
