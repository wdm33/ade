// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! CE-B2-1 — Conway vkey-witness + required-signer closure (PHASE4-B2-S1).
//!
//! Synthetic fixtures with full control: a Conway tx body + a mini-UTxO
//! (controlled input addresses → known required key hashes) + real
//! Ed25519 key material (deterministic, seeded). Every hard gate from the
//! slice doc §11 is proven here. Correctness lives in this slice: a
//! false-accept is release-blocking, so each gate is a real check, never
//! softened.

#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};

use ade_codec::cbor::{self, canonical_width, ContainerEncoding};
use ade_codec::traits::{AdeEncode, CodecContext};
use ade_ledger::tx_validity::{
    required_signers, tx_derived_required_signers, verify_required_witnesses, RequiredSignerError,
    RequiredSigners, ResolvedInputs, ResolvedOutput, SignerSource, VKeyWitnessRef, WitnessClosureError,
};
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, Hash28, Hash32};

use ed25519_dalek::{Signer, SigningKey};

// ---------------------------------------------------------------------------
// Deterministic Ed25519 key material
// ---------------------------------------------------------------------------

/// A test key pair: signing key + the Cardano key hash (Blake2b-224 of vkey).
struct TestKey {
    signing: SigningKey,
    vkey: Vec<u8>,
    key_hash: Hash28,
}

impl TestKey {
    /// Deterministic key from a fixed 32-byte seed.
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

    /// A vkey witness over `msg` (the preserved tx body hash bytes).
    fn witness_over(&self, msg: &[u8]) -> VKeyWitnessRef {
        let sig = self.signing.sign(msg).to_bytes().to_vec();
        VKeyWitnessRef {
            vkey: self.vkey.clone(),
            signature: sig,
        }
    }
}

// ---------------------------------------------------------------------------
// Conway body / address / cert / withdrawal / voter builders
// ---------------------------------------------------------------------------

const TEST_NETWORK: u8 = 1; // mainnet nibble

/// Enterprise address (type 0x6, payment = key-hash) for `key_hash`.
fn enterprise_keyhash_address(key_hash: &Hash28) -> Vec<u8> {
    let mut addr = Vec::with_capacity(29);
    addr.push((0x6 << 4) | TEST_NETWORK); // 0x61
    addr.extend_from_slice(&key_hash.0);
    addr
}

/// Enterprise address with a SCRIPT-hash payment credential (type 0x7).
fn enterprise_scripthash_address(script_hash: &Hash28) -> Vec<u8> {
    let mut addr = Vec::with_capacity(29);
    addr.push((0x7 << 4) | TEST_NETWORK); // 0x71
    addr.extend_from_slice(&script_hash.0);
    addr
}

/// Reward account (type 0xE, key-hash) for a stake key hash.
fn reward_account_keyhash(key_hash: &Hash28) -> Vec<u8> {
    let mut acct = Vec::with_capacity(29);
    acct.push((0xE << 4) | TEST_NETWORK); // 0xE1
    acct.extend_from_slice(&key_hash.0);
    acct
}

fn tx_in(byte: u8, index: u16) -> TxIn {
    TxIn {
        tx_hash: Hash32([byte; 32]),
        index,
    }
}

/// withdrawals = {+ reward_account => coin}. One entry.
fn withdrawals_cbor(reward_account: &[u8], coin: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor::write_map_header(&mut buf, ContainerEncoding::Definite(1, canonical_width(1)));
    cbor::write_bytes_canonical(&mut buf, reward_account);
    cbor::write_uint_canonical(&mut buf, coin);
    buf
}

/// credential = [0, addr_keyhash] (key-hash credential).
fn credential_keyhash_cbor(buf: &mut Vec<u8>, key_hash: &Hash28) {
    cbor::write_array_header(buf, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_uint_canonical(buf, 0);
    cbor::write_bytes_canonical(buf, &key_hash.0);
}

/// certs = [+ certificate]. Builds a single stake-deregistration cert
/// (tag 1, stake_credential) requiring the stake key's vkey.
fn certs_stake_deregistration(stake_key: &Hash28) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, canonical_width(1)));
    // certificate = [1, stake_credential]
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_uint_canonical(&mut buf, 1);
    credential_keyhash_cbor(&mut buf, stake_key);
    buf
}

/// certs with a SCRIPT-hash stake credential (tag 1) — not a vkey signer.
fn certs_stake_deregistration_script(script_hash: &Hash28) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(1, canonical_width(1)));
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_uint_canonical(&mut buf, 1);
    // credential = [1, script_hash]
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_uint_canonical(&mut buf, 1);
    cbor::write_bytes_canonical(&mut buf, &script_hash.0);
    buf
}

/// voting_procedures = {+ voter => {+ gov_action_id => voting_procedure}}.
/// One DRep key-hash voter (tag 2) with one InfoAction vote.
fn voting_procedures_drep(drep_key: &Hash28) -> Vec<u8> {
    let mut buf = Vec::new();
    cbor::write_map_header(&mut buf, ContainerEncoding::Definite(1, canonical_width(1)));
    // voter = [2, addr_keyhash]
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_uint_canonical(&mut buf, 2);
    cbor::write_bytes_canonical(&mut buf, &drep_key.0);
    // value: {+ gov_action_id => voting_procedure} — one entry
    cbor::write_map_header(&mut buf, ContainerEncoding::Definite(1, canonical_width(1)));
    // gov_action_id = [transaction_id, index]
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_bytes_canonical(&mut buf, &[0xAB; 32]);
    cbor::write_uint_canonical(&mut buf, 0);
    // voting_procedure = [vote, anchor/nil]
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_uint_canonical(&mut buf, 1); // vote = Yes
    buf.push(0xf6); // nil anchor
    buf
}

/// A minimal Conway tx body spending `input` with one output.
fn base_body(input: TxIn) -> ConwayTxBody {
    let mut inputs = BTreeSet::new();
    inputs.insert(input);
    ConwayTxBody {
        inputs,
        outputs: vec![BabbageTxOut {
            address: vec![0x61, 0x02, 0x03, 0x04, 0x05],
            coin: Coin(1_000_000),
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

fn body_hash(body: &ConwayTxBody) -> Hash32 {
    ade_crypto::blake2b_256(&encode_body(body))
}

/// A `ResolvedInputs` resolving `input` to an enterprise key-hash address.
fn resolved_for(input: &TxIn, payment_key: &Hash28) -> ResolvedInputs {
    let mut r = ResolvedInputs::new();
    r.insert(
        input.clone(),
        ResolvedOutput {
            address: enterprise_keyhash_address(payment_key),
        },
    );
    r
}

// ---------------------------------------------------------------------------
// HARD GATE: all required signers covered → Valid
// ---------------------------------------------------------------------------

#[test]
fn all_required_covered_is_valid() {
    let pay = TestKey::from_seed(0x11);
    let explicit = TestKey::from_seed(0x22);
    let withdraw = TestKey::from_seed(0x33);
    let cert = TestKey::from_seed(0x44);
    let voter = TestKey::from_seed(0x55);

    let input = tx_in(0xA0, 0);
    let mut body = base_body(input.clone());
    let mut signers = BTreeSet::new();
    signers.insert(explicit.key_hash.clone());
    body.required_signers = Some(signers);
    body.withdrawals = Some(withdrawals_cbor(
        &reward_account_keyhash(&withdraw.key_hash),
        500,
    ));
    body.certs = Some(certs_stake_deregistration(&cert.key_hash));
    body.voting_procedures = Some(voting_procedures_drep(&voter.key_hash));

    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();

    // Every one of the five tx/input sources contributed exactly its key.
    assert!(required.keys.contains(&pay.key_hash));
    assert!(required.keys.contains(&explicit.key_hash));
    assert!(required.keys.contains(&withdraw.key_hash));
    assert!(required.keys.contains(&cert.key_hash));
    assert!(required.keys.contains(&voter.key_hash));

    let h = body_hash(&body);
    let witnesses = vec![
        pay.witness_over(&h.0),
        explicit.witness_over(&h.0),
        withdraw.witness_over(&h.0),
        cert.witness_over(&h.0),
        voter.witness_over(&h.0),
    ];
    assert_eq!(verify_required_witnesses(&h, &required, &witnesses), Ok(()));
}

// ---------------------------------------------------------------------------
// HARD GATE: each missing source → MissingRequiredSigner{source}
// ---------------------------------------------------------------------------

#[test]
fn missing_input_payment_witness_rejected() {
    let pay = TestKey::from_seed(0x11);
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);

    // No witnesses at all → the input payment key is uncovered.
    match verify_required_witnesses(&h, &required, &[]) {
        Err(WitnessClosureError::MissingRequiredSigner { key_hash, source }) => {
            assert_eq!(key_hash, pay.key_hash);
            assert_eq!(source, SignerSource::InputPaymentKey);
        }
        other => panic!("expected MissingRequiredSigner{{InputPaymentKey}}, got {other:?}"),
    }
}

#[test]
fn missing_explicit_required_signer_rejected() {
    let pay = TestKey::from_seed(0x11);
    let explicit = TestKey::from_seed(0x22);
    let input = tx_in(0xA0, 0);
    let mut body = base_body(input.clone());
    let mut signers = BTreeSet::new();
    signers.insert(explicit.key_hash.clone());
    body.required_signers = Some(signers);

    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);
    // Cover the input but NOT the explicit signer.
    let witnesses = vec![pay.witness_over(&h.0)];
    match verify_required_witnesses(&h, &required, &witnesses) {
        Err(WitnessClosureError::MissingRequiredSigner { key_hash, source }) => {
            assert_eq!(key_hash, explicit.key_hash);
            assert_eq!(source, SignerSource::ExplicitRequiredSigner);
        }
        other => panic!("expected MissingRequiredSigner{{ExplicitRequiredSigner}}, got {other:?}"),
    }
}

#[test]
fn missing_withdrawal_witness_rejected() {
    let pay = TestKey::from_seed(0x11);
    let withdraw = TestKey::from_seed(0x33);
    let input = tx_in(0xA0, 0);
    let mut body = base_body(input.clone());
    body.withdrawals = Some(withdrawals_cbor(
        &reward_account_keyhash(&withdraw.key_hash),
        500,
    ));

    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);
    let witnesses = vec![pay.witness_over(&h.0)];
    match verify_required_witnesses(&h, &required, &witnesses) {
        Err(WitnessClosureError::MissingRequiredSigner { key_hash, source }) => {
            assert_eq!(key_hash, withdraw.key_hash);
            assert_eq!(source, SignerSource::WithdrawalKey);
        }
        other => panic!("expected MissingRequiredSigner{{WithdrawalKey}}, got {other:?}"),
    }
}

#[test]
fn missing_certificate_witness_rejected() {
    let pay = TestKey::from_seed(0x11);
    let cert = TestKey::from_seed(0x44);
    let input = tx_in(0xA0, 0);
    let mut body = base_body(input.clone());
    body.certs = Some(certs_stake_deregistration(&cert.key_hash));

    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);
    let witnesses = vec![pay.witness_over(&h.0)];
    match verify_required_witnesses(&h, &required, &witnesses) {
        Err(WitnessClosureError::MissingRequiredSigner { key_hash, source }) => {
            assert_eq!(key_hash, cert.key_hash);
            assert_eq!(source, SignerSource::CertificateKey);
        }
        other => panic!("expected MissingRequiredSigner{{CertificateKey}}, got {other:?}"),
    }
}

#[test]
fn missing_governance_voter_witness_rejected() {
    let pay = TestKey::from_seed(0x11);
    let voter = TestKey::from_seed(0x55);
    let input = tx_in(0xA0, 0);
    let mut body = base_body(input.clone());
    body.voting_procedures = Some(voting_procedures_drep(&voter.key_hash));

    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);
    let witnesses = vec![pay.witness_over(&h.0)];
    match verify_required_witnesses(&h, &required, &witnesses) {
        Err(WitnessClosureError::MissingRequiredSigner { key_hash, source }) => {
            assert_eq!(key_hash, voter.key_hash);
            assert_eq!(source, SignerSource::GovernanceVoter);
        }
        other => panic!("expected MissingRequiredSigner{{GovernanceVoter}}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// HARD GATE: malformed / forged signatures fail closed
// ---------------------------------------------------------------------------

#[test]
fn wrong_size_signature_rejected() {
    let pay = TestKey::from_seed(0x11);
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);

    // A witness for the right key but with a truncated (63-byte) signature.
    let mut w = pay.witness_over(&h.0);
    w.signature.truncate(63);
    match verify_required_witnesses(&h, &required, &[w]) {
        Err(WitnessClosureError::MalformedWitnessField { .. }) => {}
        other => panic!("expected MalformedWitnessField (fail-closed), got {other:?}"),
    }
}

#[test]
fn wrong_size_vkey_rejected() {
    let pay = TestKey::from_seed(0x11);
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);

    let mut w = pay.witness_over(&h.0);
    w.vkey.push(0x00); // 33-byte vkey
    match verify_required_witnesses(&h, &required, &[w]) {
        Err(WitnessClosureError::MalformedWitnessField { .. }) => {}
        other => panic!("expected MalformedWitnessField (fail-closed), got {other:?}"),
    }
}

#[test]
fn signature_over_wrong_body_rejected() {
    let pay = TestKey::from_seed(0x11);
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);

    // Sign a DIFFERENT message than the body hash; the right key signs the
    // wrong bytes. Coverage by hash matches, signature does not verify.
    let wrong = [0x00u8; 32];
    let w = pay.witness_over(&wrong);
    match verify_required_witnesses(&h, &required, &[w]) {
        Err(WitnessClosureError::InvalidWitnessSignature { key_hash }) => {
            assert_eq!(key_hash, pay.key_hash);
        }
        other => panic!("expected InvalidWitnessSignature, got {other:?}"),
    }
}

#[test]
fn witness_correct_key_wrong_body_rejected() {
    // Distinct from the previous test: build a second, different body and
    // sign ITS hash with the right key, then verify against the first body.
    let pay = TestKey::from_seed(0x11);
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    let mut other_body = base_body(input.clone());
    other_body.fee = Coin(999_999); // different bytes → different hash

    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);
    let other_h = body_hash(&other_body);
    assert_ne!(h, other_h);

    let w = pay.witness_over(&other_h.0); // right key, wrong body
    match verify_required_witnesses(&h, &required, &[w]) {
        Err(WitnessClosureError::InvalidWitnessSignature { key_hash }) => {
            assert_eq!(key_hash, pay.key_hash);
        }
        other => panic!("expected InvalidWitnessSignature, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// HARD GATE: an extra irrelevant witness never substitutes
// ---------------------------------------------------------------------------

#[test]
fn extra_irrelevant_witness_does_not_substitute() {
    let pay = TestKey::from_seed(0x11);
    let stranger = TestKey::from_seed(0x99);
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let h = body_hash(&body);

    // A valid witness from an unrelated key — must NOT cover the required
    // payment key.
    let witnesses = vec![stranger.witness_over(&h.0)];
    match verify_required_witnesses(&h, &required, &witnesses) {
        Err(WitnessClosureError::MissingRequiredSigner { key_hash, source }) => {
            assert_eq!(key_hash, pay.key_hash);
            assert_eq!(source, SignerSource::InputPaymentKey);
        }
        other => panic!("expected MissingRequiredSigner, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// HARD GATE: unresolvable input is fail-fast (never silent skip)
// ---------------------------------------------------------------------------

#[test]
fn unresolvable_input_is_fail_fast() {
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    // Empty ResolvedInputs — the spend input cannot be resolved.
    let resolved = ResolvedInputs::new();
    match required_signers(&body, &resolved, CardanoEra::Conway) {
        Err(RequiredSignerError::UnresolvableInput { input: i }) => {
            assert_eq!(i, input);
        }
        other => panic!("expected UnresolvableInput fail-fast, got {other:?}"),
    }
}

#[test]
fn unresolvable_collateral_input_is_fail_fast() {
    let pay = TestKey::from_seed(0x11);
    let input = tx_in(0xA0, 0);
    let collateral = tx_in(0xC0, 0);
    let mut body = base_body(input.clone());
    let mut col = BTreeSet::new();
    col.insert(collateral.clone());
    body.collateral_inputs = Some(col);

    // Resolve the spend input but NOT the collateral input.
    let resolved = resolved_for(&input, &pay.key_hash);
    match required_signers(&body, &resolved, CardanoEra::Conway) {
        Err(RequiredSignerError::UnresolvableInput { input: i }) => {
            assert_eq!(i, collateral);
        }
        other => panic!("expected UnresolvableInput for collateral, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// HARD GATE: script-credential input is NOT a vkey signer (no over-require)
// ---------------------------------------------------------------------------

#[test]
fn script_credential_input_not_a_vkey_signer() {
    let script = Hash28([0x7C; 28]); // a script hash, not a key hash
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());

    let mut resolved = ResolvedInputs::new();
    resolved.insert(
        input.clone(),
        ResolvedOutput {
            address: enterprise_scripthash_address(&script),
        },
    );
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();

    // The script-hash payment credential contributes NO required vkey
    // signer; the only required set is empty → no witnesses needed.
    assert!(required.keys.is_empty(), "script input must not require a vkey signer");
    let h = body_hash(&body);
    assert_eq!(verify_required_witnesses(&h, &required, &[]), Ok(()));
}

#[test]
fn script_credential_certificate_not_a_vkey_signer() {
    let pay = TestKey::from_seed(0x11);
    let script = Hash28([0x7C; 28]);
    let input = tx_in(0xA0, 0);
    let mut body = base_body(input.clone());
    body.certs = Some(certs_stake_deregistration_script(&script));

    let resolved = resolved_for(&input, &pay.key_hash);
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();

    // Only the input payment key is required; the script-cred cert adds none.
    assert!(required.keys.contains(&pay.key_hash));
    assert!(!required.provenance.iter().any(|(s, _)| *s == SignerSource::CertificateKey));
}

// ---------------------------------------------------------------------------
// Collateral payment key contributes (script-tx coverage source)
// ---------------------------------------------------------------------------

#[test]
fn collateral_payment_key_is_required() {
    let pay = TestKey::from_seed(0x11);
    let col_key = TestKey::from_seed(0x66);
    let input = tx_in(0xA0, 0);
    let collateral = tx_in(0xC0, 0);
    let mut body = base_body(input.clone());
    let mut col = BTreeSet::new();
    col.insert(collateral.clone());
    body.collateral_inputs = Some(col);

    let mut resolved = resolved_for(&input, &pay.key_hash);
    resolved.insert(
        collateral.clone(),
        ResolvedOutput {
            address: enterprise_keyhash_address(&col_key.key_hash),
        },
    );
    let required = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    assert!(required
        .provenance
        .iter()
        .any(|(s, k)| *s == SignerSource::CollateralPaymentKey && *k == col_key.key_hash));

    let h = body_hash(&body);
    // Missing the collateral witness → rejected naming CollateralPaymentKey.
    let witnesses = vec![pay.witness_over(&h.0)];
    match verify_required_witnesses(&h, &required, &witnesses) {
        Err(WitnessClosureError::MissingRequiredSigner { key_hash, source }) => {
            assert_eq!(key_hash, col_key.key_hash);
            assert_eq!(source, SignerSource::CollateralPaymentKey);
        }
        other => panic!("expected MissingRequiredSigner{{CollateralPaymentKey}}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// tx-derived subset is UTxO-free (track_utxo=false body-path surface)
// ---------------------------------------------------------------------------

#[test]
fn tx_derived_subset_excludes_input_sources() {
    let explicit = TestKey::from_seed(0x22);
    let input = tx_in(0xA0, 0);
    let mut body = base_body(input);
    let mut signers = BTreeSet::new();
    signers.insert(explicit.key_hash.clone());
    body.required_signers = Some(signers);

    let required = tx_derived_required_signers(&body, CardanoEra::Conway).unwrap();
    // The explicit signer is present; no input/collateral source appears.
    assert!(required.keys.contains(&explicit.key_hash));
    assert!(!required.provenance.iter().any(|(s, _)| matches!(
        s,
        SignerSource::InputPaymentKey | SignerSource::CollateralPaymentKey
    )));
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn closure_is_deterministic() {
    let pay = TestKey::from_seed(0x11);
    let cert = TestKey::from_seed(0x44);
    let input = tx_in(0xA0, 0);
    let mut body = base_body(input.clone());
    body.certs = Some(certs_stake_deregistration(&cert.key_hash));
    let resolved = resolved_for(&input, &pay.key_hash);

    let r1 = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    let r2 = required_signers(&body, &resolved, CardanoEra::Conway).unwrap();
    assert_eq!(r1, r2);

    let h = body_hash(&body);
    let witnesses = vec![pay.witness_over(&h.0)]; // cert witness intentionally absent
    let v1 = verify_required_witnesses(&h, &r1, &witnesses);
    let v2 = verify_required_witnesses(&h, &r2, &witnesses);
    assert_eq!(format!("{v1:?}"), format!("{v2:?}"));
    // And the same failure both times: the cert key is the first uncovered.
    assert!(matches!(v1, Err(WitnessClosureError::MissingRequiredSigner { .. })));
}

// ---------------------------------------------------------------------------
// Closed enumeration / non-Conway era guard
// ---------------------------------------------------------------------------

#[test]
fn non_conway_era_is_unsupported() {
    let input = tx_in(0xA0, 0);
    let body = base_body(input.clone());
    let resolved = ResolvedInputs::new();
    match required_signers(&body, &resolved, CardanoEra::Babbage) {
        Err(RequiredSignerError::UnsupportedEra { era }) => {
            assert_eq!(era, CardanoEra::Babbage);
        }
        other => panic!("expected UnsupportedEra, got {other:?}"),
    }
}

#[test]
fn empty_requirements_is_trivially_valid() {
    // A body with no inputs resolved and no tx-derived sources: the
    // tx-derived subset is empty, so verification passes with no witnesses.
    let input = tx_in(0xA0, 0);
    let body = base_body(input);
    let required: RequiredSigners = tx_derived_required_signers(&body, CardanoEra::Conway).unwrap();
    assert!(required.keys.is_empty());
    let h = body_hash(&body);
    assert_eq!(
        verify_required_witnesses(&h, &required, &[]),
        Ok(())
    );
    // Sanity: the resolved map below is unused but documents intent.
    let _ = BTreeMap::<TxIn, ()>::new();
}
