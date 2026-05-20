// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! CE-B2-2 — `tx_validity` composition + verdict taxonomy (PHASE4-B2-S2).
//!
//! Synthetic `LedgerState` + synthetic Conway transactions with full UTxO
//! control and real (deterministic, seeded) Ed25519 key material — the same
//! fixture style B2-S1 used. Non-Plutus transactions: phase-2 dispatch is
//! wired but full Plutus eval is not exercised here (CE-88/aiken carve-out).
//!
//! Correctness is release-blocking: a false-accept is the cluster's #1
//! prohibition, so no assertion is softened.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use ade_codec::cbor::{self, canonical_width, ContainerEncoding};
use ade_codec::traits::{AdeEncode, CodecContext};
use ade_ledger::rules::apply_conway_tx_to_utxo;
use ade_ledger::state::LedgerState;
use ade_ledger::tx_validity::verdict::{TxRejectClass, TxValidityError};
use ade_ledger::tx_validity::witness::{WitnessClosureError, WitnessField};
use ade_ledger::tx_validity::{tx_validity, TxValidityVerdict};
use ade_ledger::utxo::TxOut;
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, Hash28, Hash32};

use ed25519_dalek::{Signer, SigningKey};

// ---------------------------------------------------------------------------
// Deterministic Ed25519 key material (mirrors B2-S1)
// ---------------------------------------------------------------------------

struct TestKey {
    signing: SigningKey,
    vkey: Vec<u8>,
    key_hash: Hash28,
}

impl TestKey {
    fn from_seed(seed_byte: u8) -> Self {
        let seed = [seed_byte; 32];
        let signing = SigningKey::from_bytes(&seed);
        let vkey = signing.verifying_key().to_bytes().to_vec();
        let key_hash = ade_crypto::blake2b_224(&vkey);
        TestKey {
            signing,
            vkey,
            key_hash,
        }
    }

    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.signing.sign(msg).to_bytes().to_vec()
    }
}

const TEST_NETWORK: u8 = 1;

/// Enterprise address (type 0x6, payment = key-hash) for `key_hash`.
fn enterprise_keyhash_address(key_hash: &Hash28) -> Vec<u8> {
    let mut addr = Vec::with_capacity(29);
    addr.push((0x6 << 4) | TEST_NETWORK);
    addr.extend_from_slice(&key_hash.0);
    addr
}

fn tx_in(byte: u8, index: u16) -> TxIn {
    TxIn {
        tx_hash: Hash32([byte; 32]),
        index,
    }
}

/// A minimal Conway tx body spending `input` with one key-hash output.
fn base_body(input: TxIn, out_payment: &Hash28) -> ConwayTxBody {
    let mut inputs = BTreeSet::new();
    inputs.insert(input);
    ConwayTxBody {
        inputs,
        outputs: vec![BabbageTxOut {
            address: enterprise_keyhash_address(out_payment),
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

fn encode_body(body: &ConwayTxBody) -> Vec<u8> {
    let mut buf = Vec::new();
    let ctx = CodecContext {
        era: CardanoEra::Conway,
    };
    body.ade_encode(&mut buf, &ctx).unwrap();
    buf
}

/// Witness set map `{0: [[vkey, sig], ...]}` for the given witnesses.
fn witness_set_cbor(witnesses: &[(&[u8], &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor::write_map_header(&mut buf, ContainerEncoding::Definite(1, canonical_width(1)));
    cbor::write_uint_canonical(&mut buf, 0);
    cbor::write_array_header(
        &mut buf,
        ContainerEncoding::Definite(witnesses.len() as u64, canonical_width(witnesses.len() as u64)),
    );
    for (vkey, sig) in witnesses {
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        cbor::write_bytes_canonical(&mut buf, vkey);
        cbor::write_bytes_canonical(&mut buf, sig);
    }
    buf
}

/// An empty witness set `{}`.
fn empty_witness_set_cbor() -> Vec<u8> {
    let mut buf = Vec::new();
    cbor::write_map_header(&mut buf, ContainerEncoding::Definite(0, canonical_width(0)));
    buf
}

/// Assemble a full Conway tx CBOR `[body, witness_set, true, nil]`.
fn full_tx_cbor(body_bytes: &[u8], witness_set: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(0x84); // array(4)
    buf.extend_from_slice(body_bytes);
    buf.extend_from_slice(witness_set);
    buf.push(0xf5); // is_valid = true
    buf.push(0xf6); // aux_data = nil
    buf
}

/// The body-byte slice `tx_validity` will hash, lifted exactly the way the
/// decoder lifts it: element 0 of the full tx array.
fn body_slice_of(tx_cbor: &[u8]) -> Vec<u8> {
    let mut offset = 0;
    let _ = cbor::read_array_header(tx_cbor, &mut offset).unwrap();
    let start = offset;
    let _ = ade_codec::conway::tx::decode_conway_tx_body(tx_cbor, &mut offset).unwrap();
    tx_cbor[start..offset].to_vec()
}

/// A synthetic Conway `LedgerState` (track_utxo=true) whose UTxO resolves
/// `input` to an enterprise key-hash address for `payment_key`.
fn state_with_input(input: &TxIn, payment_key: &Hash28) -> LedgerState {
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = true;
    let raw = {
        // A self-contained AlonzoPlus output whose `raw` is its own minimal
        // CBOR encoding (address + coin map) — only address_bytes() is read by
        // the required-signer derivation, so the raw form is not load-bearing.
        let out = BabbageTxOut {
            address: enterprise_keyhash_address(payment_key),
            coin: Coin(1_000_000),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        };
        let mut buf = Vec::new();
        out.ade_encode(
            &mut buf,
            &CodecContext {
                era: CardanoEra::Conway,
            },
        )
        .unwrap();
        buf
    };
    state.utxo_state.utxos.insert(
        input.clone(),
        TxOut::AlonzoPlus {
            raw,
            address: enterprise_keyhash_address(payment_key),
            coin: Coin(1_000_000),
        },
    );
    state
}

// ---------------------------------------------------------------------------
// valid_tx_is_valid_and_applies
// ---------------------------------------------------------------------------

#[test]
fn valid_tx_is_valid_and_applies() {
    let pay = TestKey::from_seed(0x11);
    let out_key = TestKey::from_seed(0x99);
    let input = tx_in(0xA0, 0);

    let state = state_with_input(&input, &pay.key_hash);
    let body = base_body(input.clone(), &out_key.key_hash);
    let body_bytes = encode_body(&body);

    // Sign over the tx id (= blake2b_256 of the preserved body bytes).
    let tx_id = ade_crypto::blake2b_256(&body_bytes);
    let sig = pay.sign(&tx_id.0);
    let wset = witness_set_cbor(&[(&pay.vkey, &sig)]);
    let tx = full_tx_cbor(&body_bytes, &wset);

    let outcome = tx_validity(&state, &tx);
    match &outcome.verdict {
        TxValidityVerdict::Valid { tx_id: vid, applied } => {
            assert_eq!(*vid, tx_id, "tx_id must be the preserved-body hash");
            // The spend evolved the state: input consumed, output produced.
            assert!(
                !applied.utxo_state.utxos.contains_key(&input),
                "spent input must be consumed",
            );
            let produced = TxIn {
                tx_hash: tx_id.clone(),
                index: 0,
            };
            assert!(
                applied.utxo_state.utxos.contains_key(&produced),
                "tx output must be produced under the tx id",
            );
            // `applied` field on the outcome mirrors the Valid verdict's state.
            assert_eq!(&outcome.applied, applied);
            // Exactly the spend changed: same UTxO minus input plus output.
            let expected = apply_conway_tx_to_utxo(
                &state.utxo_state,
                &body,
                &body_bytes,
                &tx_id,
            )
            .unwrap();
            assert_eq!(applied.utxo_state, expected);
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// phase1_failure_short_circuits_phase2
// ---------------------------------------------------------------------------

#[test]
fn phase1_failure_short_circuits_phase2() {
    let pay = TestKey::from_seed(0x11);
    let out_key = TestKey::from_seed(0x99);
    let input = tx_in(0xA0, 0);

    let state = state_with_input(&input, &pay.key_hash);
    let body = base_body(input.clone(), &out_key.key_hash);
    let body_bytes = encode_body(&body);

    // Phase-1 fails: the required input payment witness is absent. (Empty
    // witness set.) A non-Plutus tx never reaches phase-2 anyway, but we also
    // assert the verdict is a witness-class phase-1 rejection — proving the
    // composition halted in phase-1 and did NOT produce a Valid verdict.
    let tx = full_tx_cbor(&body_bytes, &empty_witness_set_cbor());

    let outcome = tx_validity(&state, &tx);
    match &outcome.verdict {
        TxValidityVerdict::Invalid { class, error } => {
            assert_eq!(*class, TxRejectClass::MissingRequiredSigner);
            assert!(
                matches!(
                    error,
                    TxValidityError::Witness(WitnessClosureError::MissingRequiredSigner { .. })
                ),
                "phase-1 witness closure must be the failing stage, got {error:?}",
            );
        }
        other => panic!("expected phase-1 Invalid, got {other:?}"),
    }

    // Phase-2 never ran: the state is unchanged (no collateral consumed, no
    // outputs produced). If phase-2 had run on a phase-1-invalid tx this would
    // differ from the input.
    assert_eq!(outcome.applied, state, "phase-1-invalid tx must not be mutated");
}

// ---------------------------------------------------------------------------
// invalid_tx_leaves_state_unchanged
// ---------------------------------------------------------------------------

#[test]
fn invalid_tx_leaves_state_unchanged() {
    let pay = TestKey::from_seed(0x11);
    let out_key = TestKey::from_seed(0x99);
    let input = tx_in(0xA0, 0);

    let state = state_with_input(&input, &pay.key_hash);
    let body = base_body(input.clone(), &out_key.key_hash);
    let body_bytes = encode_body(&body);
    let tx_id = ade_crypto::blake2b_256(&body_bytes);

    // A forged witness: correct key hash, but signed over the WRONG message.
    let bad_sig = pay.sign(b"not the tx body hash");
    let wset = witness_set_cbor(&[(&pay.vkey, &bad_sig)]);
    let tx = full_tx_cbor(&body_bytes, &wset);

    let outcome = tx_validity(&state, &tx);
    match &outcome.verdict {
        TxValidityVerdict::Invalid { class, error } => {
            assert_eq!(*class, TxRejectClass::WitnessInvalid);
            assert!(
                matches!(
                    error,
                    TxValidityError::Witness(WitnessClosureError::InvalidWitnessSignature {
                        key_hash,
                    }) if *key_hash == pay.key_hash
                ),
                "expected InvalidWitnessSignature, got {error:?}",
            );
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
    let _ = tx_id;
    assert_eq!(outcome.applied, state, "Invalid outcome must clone input state");
}

// ---------------------------------------------------------------------------
// tx_id_uses_preserved_bytes
// ---------------------------------------------------------------------------

#[test]
fn tx_id_uses_preserved_bytes() {
    let pay = TestKey::from_seed(0x11);
    let out_key = TestKey::from_seed(0x99);
    let input = tx_in(0xA0, 0);

    let state = state_with_input(&input, &pay.key_hash);
    let body = base_body(input.clone(), &out_key.key_hash);
    let body_bytes = encode_body(&body);
    let tx_id = ade_crypto::blake2b_256(&body_bytes);
    let sig = pay.sign(&tx_id.0);
    let wset = witness_set_cbor(&[(&pay.vkey, &sig)]);
    let tx = full_tx_cbor(&body_bytes, &wset);

    // The hash MUST be over the body slice lifted from the FULL tx CBOR — not
    // a re-encode. Assert the lifted slice equals our encoded body and that the
    // verdict's tx_id is blake2b_256 of exactly those preserved bytes.
    let lifted = body_slice_of(&tx);
    assert_eq!(lifted, body_bytes, "lifted body slice must be byte-identical");
    let expected = ade_crypto::blake2b_256(&lifted);

    let outcome = tx_validity(&state, &tx);
    match outcome.verdict {
        TxValidityVerdict::Valid { tx_id: vid, .. } => {
            assert_eq!(vid, expected, "tx_id must equal blake2b_256(preserved bytes)");
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// class_mapping_is_total
// ---------------------------------------------------------------------------

#[test]
fn class_mapping_is_total() {
    use ade_ledger::block_validity::{FieldError, FieldKind};
    use ade_ledger::error::LedgerError;

    // Every TxValidityError variant maps to a class. Exercising one
    // representative per variant proves the mapping is total and the closed
    // taxonomy is exhaustive (the match in `class()` has no catch-all).
    let cases: Vec<(TxValidityError, TxRejectClass)> = vec![
        (
            TxValidityError::Decode(LedgerError::from(
                ade_codec::CodecError::InvalidCborStructure {
                    offset: 0,
                    detail: "x",
                },
            )),
            TxRejectClass::MalformedField,
        ),
        (
            TxValidityError::Witness(WitnessClosureError::MissingRequiredSigner {
                key_hash: Hash28([0u8; 28]),
                source: ade_ledger::tx_validity::SignerSource::InputPaymentKey,
            }),
            TxRejectClass::MissingRequiredSigner,
        ),
        (
            TxValidityError::Witness(WitnessClosureError::InvalidWitnessSignature {
                key_hash: Hash28([0u8; 28]),
            }),
            TxRejectClass::WitnessInvalid,
        ),
        (
            TxValidityError::Witness(WitnessClosureError::MalformedWitnessField {
                which: WitnessField::Signature,
                key_hash: Hash28([0u8; 28]),
            }),
            TxRejectClass::WitnessInvalid,
        ),
        (
            TxValidityError::Phase1(LedgerError::from(
                ade_codec::CodecError::InvalidCborStructure {
                    offset: 0,
                    detail: "x",
                },
            )),
            TxRejectClass::Phase1Invalid,
        ),
        (
            TxValidityError::Phase2(LedgerError::from(
                ade_codec::CodecError::InvalidCborStructure {
                    offset: 0,
                    detail: "x",
                },
            )),
            TxRejectClass::Phase2Invalid,
        ),
        (
            TxValidityError::MalformedField(FieldError {
                field: FieldKind::VkeyWitness,
                expected: 32,
                actual: 31,
            }),
            TxRejectClass::MalformedField,
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.class(), expected, "class() must be total for {error:?}");
    }
}

// ---------------------------------------------------------------------------
// determinism
// ---------------------------------------------------------------------------

#[test]
fn tx_validity_is_deterministic() {
    let pay = TestKey::from_seed(0x11);
    let out_key = TestKey::from_seed(0x99);
    let input = tx_in(0xA0, 0);

    let state = state_with_input(&input, &pay.key_hash);
    let body = base_body(input.clone(), &out_key.key_hash);
    let body_bytes = encode_body(&body);
    let tx_id = ade_crypto::blake2b_256(&body_bytes);
    let sig = pay.sign(&tx_id.0);
    let wset = witness_set_cbor(&[(&pay.vkey, &sig)]);
    let tx = full_tx_cbor(&body_bytes, &wset);

    let a = tx_validity(&state, &tx);
    let b = tx_validity(&state, &tx);
    assert_eq!(a.verdict, b.verdict, "same (state, tx) → identical verdict");
    assert_eq!(a.applied, b.applied, "same (state, tx) → identical applied state");
}
