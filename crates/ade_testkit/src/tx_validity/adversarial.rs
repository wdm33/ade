// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN adversarial tx mutators + synthetic adversarial builders (B2-S4).
//!
//! Non-authoritative. Two families, both judged by the BLUE
//! [`ade_ledger::tx_validity::tx_validity`]; the calling test asserts EVERY
//! mutation lands `Invalid` (the CE-B2-4 no-false-accept core):
//!
//! **(A) Witness mutations on REAL corpus txs** ([`mutate_witness`]). A real
//! on-wire Conway tx that carries a tx-derived required signer (explicit k14 /
//! withdrawal k5 / cert k4 / voter k19) is corrupted in one targeted way:
//!
//! - W1 remove a required witness → MissingRequiredSigner
//! - W2 flip a byte in a witness signature → WitnessInvalid
//! - W3 truncate a witness sig → WitnessInvalid (a wrong-size signature is a
//!   fail-closed `MalformedWitnessField`, routed to WitnessInvalid)
//! - W4 re-sign a different body → WitnessInvalid
//!
//! These run at `track_utxo = false`: tx-derived required-signer coverage and
//! supplied-witness verification both run unconditionally there, so a real tx
//! with a tx-derived requirement is a valid no-false-accept target. (Real
//! corpus txs without any tx-derived requirement are skipped — at
//! track_utxo=false removing their only witnesses is correctly NOT a
//! required-signer violation; those belong to family B.)
//!
//! **(B) Synthetic adversarial txs** (builders below) at `track_utxo = true`
//! over a controlled UTxO — the UTxO-dependent surface track_utxo=false defers:
//!
//! - S1 value imbalance (outputs + fee != inputs) → Phase1Invalid
//! - S2 missing input-payment witness → MissingRequiredSigner
//! - S3 unresolvable / dangling input → Phase1Invalid
//! - S4 forged input-payment witness (wrong sig) → WitnessInvalid
//!
//! Family-A mutators decode the witness-set vkey array with the same
//! `ade_codec::cbor` primitives the BLUE decoder uses, then rebuild it —
//! applying the mutation to EVERY entry covering a required signer (so the
//! requirement is genuinely violated even when several witnesses share a key
//! hash) — never blind byte flips. `BTreeMap`/`Vec` only; no `HashMap`, no
//! float, no clock.

use std::collections::BTreeSet;

use ade_codec::cbor::{self, canonical_width, ContainerEncoding};
use ade_codec::traits::{AdeEncode, CodecContext};
use ade_ledger::state::LedgerState;
use ade_ledger::tx_validity::{encode_tx_verdict_surface, tx_validity, TxRejectClass, TxValidityVerdict};
use ade_ledger::utxo::TxOut;
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32};

use ed25519_dalek::{Signer, SigningKey};

use super::extract::ExtractedTx;

const TEST_NETWORK: u8 = 1; // mainnet nibble, matching ProtocolParameters default
const EPOCH_576: EpochNo = EpochNo(576);

// ===========================================================================
// Family (A): witness mutations on real corpus txs
// ===========================================================================

/// The named witness mutations applied to a REAL corpus tx (family A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessMutation {
    /// W1 — drop one supplied vkey witness (the first), leaving a required
    /// signer uncovered.
    RemoveWitness,
    /// W2 — flip a byte inside a witness signature (right key, broken sig).
    FlipSignatureByte,
    /// W3 — truncate a witness signature so the witness-set no longer decodes
    /// to a fixed-size signature.
    TruncateSignature,
    /// W4 — replace a witness signature with one over a DIFFERENT message
    /// (the right key signing the wrong body).
    ReSignDifferentBody,
}

impl WitnessMutation {
    pub const ALL: [WitnessMutation; 4] = [
        WitnessMutation::RemoveWitness,
        WitnessMutation::FlipSignatureByte,
        WitnessMutation::TruncateSignature,
        WitnessMutation::ReSignDifferentBody,
    ];

    /// The reject class this mutation is documented to produce.
    ///
    /// W3 (TruncateSignature) lands `WitnessInvalid`, NOT `MalformedField`:
    /// a wrong-size signature is a `WitnessClosureError::MalformedWitnessField`,
    /// which the verdict taxonomy (`TxValidityError::class`) routes to
    /// `WitnessInvalid` because it is a witness-set field failure discovered
    /// during coverage, not a tx-body decode failure. This is still fully
    /// fail-closed (never Valid) — the slice §13 "wrong class acceptable if
    /// still fail-closed; document actual class" path.
    pub fn expected_class(self) -> TxRejectClass {
        match self {
            WitnessMutation::RemoveWitness => TxRejectClass::MissingRequiredSigner,
            WitnessMutation::FlipSignatureByte
            | WitnessMutation::ReSignDifferentBody
            | WitnessMutation::TruncateSignature => TxRejectClass::WitnessInvalid,
        }
    }
}

/// One decoded `[vkey, sig]` witness entry: its vkey, its signature, and
/// whether it covers a tx-derived required signer.
#[derive(Clone)]
struct Entry {
    vkey: Vec<u8>,
    sig: Vec<u8>,
    covers_required: bool,
}

/// The vkey-witness array decoded into entries, each tagged with whether it
/// covers a tx-derived required signer. Family-A mutators rebuild the array
/// from these entries, mutating EVERY entry that covers a requirement — a
/// single covering witness left intact would (correctly) keep the tx Valid,
/// which is not a no-false-accept target. Mutating all covering entries makes
/// the requirement genuinely uncovered/invalid.
struct WitnessEntries {
    entries: Vec<Entry>,
    /// How many entries cover a tx-derived required signer.
    covering: usize,
}

/// Decode the vkey-witness array (key 0, optionally tag(258)-wrapped) into
/// tagged entries. Returns `None` if there is no vkey-witness array, it is
/// indefinite-length (not produced by the reference), or it is empty.
fn decode_witness_entries(ws: &[u8], required: &BTreeSet<Hash28>) -> Option<WitnessEntries> {
    let mut offset = 0usize;
    let enc = cbor::read_map_header(ws, &mut offset).ok()?;
    let mut array_start: Option<usize> = None;

    let mut visit = |ws: &[u8], offset: &mut usize| -> Option<()> {
        let (key, _) = cbor::read_uint(ws, offset).ok()?;
        if key == 0 {
            if *offset < ws.len() && ((ws[*offset] >> 5) & 0x7) == 6 {
                cbor::read_tag(ws, offset).ok()?;
            }
            array_start = Some(*offset);
            cbor::skip_item(ws, offset).ok()?;
        } else {
            cbor::skip_item(ws, offset).ok()?;
        }
        Some(())
    };
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                visit(ws, &mut offset)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(ws, offset).ok()? {
                visit(ws, &mut offset)?;
            }
        }
    }

    let array_start = array_start?;
    let mut off = array_start;
    let count = match cbor::read_array_header(ws, &mut off).ok()? {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => return None,
    };
    if count == 0 {
        return None;
    }
    let mut entries = Vec::with_capacity(count as usize);
    let mut covering = 0usize;
    for _ in 0..count {
        cbor::read_array_header(ws, &mut off).ok()?; // [vkey, sig] header
        let (vkey, _) = cbor::read_bytes(ws, &mut off).ok()?;
        let (sig, _) = cbor::read_bytes(ws, &mut off).ok()?;
        let covers_required = required.contains(&ade_crypto::blake2b_224(&vkey));
        if covers_required {
            covering += 1;
        }
        entries.push(Entry {
            vkey,
            sig,
            covers_required,
        });
    }
    Some(WitnessEntries { entries, covering })
}

/// Re-encode a vkey-witness set `{0: [[vkey, sig], ...]}` from entries.
fn encode_witness_set(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    witness_set(entries)
}

/// Outcome of attempting a family-A witness mutation on a real corpus tx.
pub enum WitnessMutationOutcome {
    /// The tx is not a valid family-A target (no tx-derived required signer, or
    /// no vkey witness to corrupt) — skipped, with the reason.
    NotApplicable(&'static str),
    /// The mutated tx CBOR, ready to drive through `tx_validity`.
    Mutated(Vec<u8>),
}

/// Does this real tx carry a tx-derived required signer (explicit / withdrawal
/// / cert / voter)? Only such txs are family-A targets at track_utxo=false: for
/// a tx with no tx-derived requirement, removing its witnesses is correctly NOT
/// a required-signer violation in partial mode.
pub fn has_tx_derived_requirement(tx_cbor: &[u8]) -> bool {
    let body = match decode_body_only(tx_cbor) {
        Some(b) => b,
        None => return false,
    };
    let req = ade_ledger::tx_validity::tx_derived_required_signers(&body, CardanoEra::Conway);
    matches!(req, Ok(r) if !r.keys.is_empty())
}

/// Decode just the body of a full Conway tx `[body, ws, is_valid, aux]`.
fn decode_body_only(tx_cbor: &[u8]) -> Option<ConwayTxBody> {
    let mut offset = 0usize;
    cbor::read_array_header(tx_cbor, &mut offset).ok()?;
    ade_codec::conway::tx::decode_conway_tx_body(tx_cbor, &mut offset).ok()
}

/// Split a full Conway tx `[body, witness_set, is_valid, aux]` into the
/// preserved body slice, witness-set slice, and trailing (is_valid + aux) bytes.
fn split_full_tx(tx_cbor: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let mut offset = 0usize;
    cbor::read_array_header(tx_cbor, &mut offset).ok()?;
    let body_start = offset;
    ade_codec::conway::tx::decode_conway_tx_body(tx_cbor, &mut offset).ok()?;
    let body_end = offset;
    let ws_start = offset;
    cbor::skip_item(tx_cbor, &mut offset).ok()?;
    let ws_end = offset;
    let body = tx_cbor[body_start..body_end].to_vec();
    let ws = tx_cbor[ws_start..ws_end].to_vec();
    let tail = tx_cbor[ws_end..].to_vec();
    Some((body, ws, tail))
}

/// Reassemble a full Conway tx from a preserved body, a (possibly mutated)
/// witness set, and the original trailing bytes.
fn reassemble(body: &[u8], ws: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + body.len() + ws.len() + tail.len());
    out.push(0x84); // array(4)
    out.extend_from_slice(body);
    out.extend_from_slice(ws);
    out.extend_from_slice(tail);
    out
}

/// Apply one family-A witness mutation to a real corpus tx, returning fresh
/// full-tx CBOR (or `NotApplicable`). The witness set's vkey array is rebuilt
/// from its decoded entries; the mutation is applied to EVERY entry covering a
/// tx-derived required signer (so the requirement is genuinely violated even
/// when several witnesses share the same key hash) and the body + trailing
/// bytes are preserved.
pub fn mutate_witness(extracted: &ExtractedTx, mutation: WitnessMutation) -> WitnessMutationOutcome {
    let body_typed = match decode_body_only(&extracted.tx_cbor) {
        Some(b) => b,
        None => return WitnessMutationOutcome::NotApplicable("tx body does not decode"),
    };
    let required = match ade_ledger::tx_validity::tx_derived_required_signers(
        &body_typed,
        CardanoEra::Conway,
    ) {
        Ok(r) if !r.keys.is_empty() => r.keys,
        _ => return WitnessMutationOutcome::NotApplicable("no tx-derived required signer"),
    };
    let (body, ws, tail) = match split_full_tx(&extracted.tx_cbor) {
        Some(parts) => parts,
        None => return WitnessMutationOutcome::NotApplicable("tx does not split"),
    };
    let decoded = match decode_witness_entries(&ws, &required) {
        Some(d) if d.covering > 0 => d,
        _ => {
            return WitnessMutationOutcome::NotApplicable(
                "no witness covers a tx-derived required signer",
            )
        }
    };

    // Build the new entry list, applying the mutation to every covering entry.
    let mut new_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(decoded.entries.len());
    for e in &decoded.entries {
        if !e.covers_required {
            new_entries.push((e.vkey.clone(), e.sig.clone()));
            continue;
        }
        match mutation {
            WitnessMutation::RemoveWitness => {
                // Drop the covering entry entirely → requirement uncovered.
            }
            WitnessMutation::FlipSignatureByte => {
                if e.sig.is_empty() {
                    return WitnessMutationOutcome::NotApplicable("empty signature");
                }
                let mut sig = e.sig.clone();
                sig[0] ^= 0x01;
                new_entries.push((e.vkey.clone(), sig));
            }
            WitnessMutation::TruncateSignature => {
                if e.sig.len() <= 1 {
                    return WitnessMutationOutcome::NotApplicable(
                        "signature too short to truncate",
                    );
                }
                let mut sig = e.sig.clone();
                sig.truncate(sig.len() - 1);
                new_entries.push((e.vkey.clone(), sig));
            }
            WitnessMutation::ReSignDifferentBody => {
                // Keep the original vkey (coverage-by-hash still matches the
                // required key) but replace the signature with a real Ed25519
                // signature over a DIFFERENT message under a throwaway key. It
                // cannot verify against the real body hash for the matched key
                // → InvalidWitnessSignature (WitnessInvalid).
                new_entries.push((e.vkey.clone(), forged_signature_bytes(&extracted.tx_cbor)));
            }
        }
    }

    let mutated_ws = encode_witness_set(&new_entries);
    WitnessMutationOutcome::Mutated(reassemble(&body, &mutated_ws, &tail))
}

/// A deterministic, structurally-valid 64-byte Ed25519 signature over a fixed
/// message under a throwaway key. It will never verify for the corpus tx's
/// required key, so it forces a fail-closed `InvalidWitnessSignature`.
fn forged_signature_bytes(seed_src: &[u8]) -> Vec<u8> {
    let mut seed = [0x5Au8; 32];
    for (i, b) in seed_src.iter().take(32).enumerate() {
        seed[i] ^= *b;
    }
    let sk = SigningKey::from_bytes(&seed);
    sk.sign(b"adversarial-wrong-body").to_bytes().to_vec()
}

// ===========================================================================
// Family (B): synthetic adversarial txs at track_utxo = true
// ===========================================================================

/// The named synthetic adversarial mutations (family B), each over a fully
/// controlled UTxO so the UTxO-dependent surface is exercised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticMutation {
    /// S1 — value imbalance: outputs + fee != sum(inputs).
    ValueImbalance,
    /// S2 — the input payment-key witness is absent.
    MissingInputWitness,
    /// S3 — an input references a UTxO that does not exist.
    DanglingInput,
    /// S4 — the input payment-key witness carries a forged (non-verifying) sig.
    ForgedInputWitness,
}

impl SyntheticMutation {
    pub const ALL: [SyntheticMutation; 4] = [
        SyntheticMutation::ValueImbalance,
        SyntheticMutation::MissingInputWitness,
        SyntheticMutation::DanglingInput,
        SyntheticMutation::ForgedInputWitness,
    ];

    pub fn expected_class(self) -> TxRejectClass {
        match self {
            SyntheticMutation::ValueImbalance | SyntheticMutation::DanglingInput => {
                TxRejectClass::Phase1Invalid
            }
            SyntheticMutation::MissingInputWitness => TxRejectClass::MissingRequiredSigner,
            SyntheticMutation::ForgedInputWitness => TxRejectClass::WitnessInvalid,
        }
    }
}

/// Deterministic Ed25519 key material for the synthetic family.
struct SynthKey {
    signing: SigningKey,
    vkey: Vec<u8>,
    key_hash: Hash28,
}

impl SynthKey {
    fn from_seed(seed_byte: u8) -> Self {
        let signing = SigningKey::from_bytes(&[seed_byte; 32]);
        let vkey = signing.verifying_key().to_bytes().to_vec();
        let key_hash = ade_crypto::blake2b_224(&vkey);
        SynthKey {
            signing,
            vkey,
            key_hash,
        }
    }

    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.signing.sign(msg).to_bytes().to_vec()
    }
}

/// Enterprise key-hash address (type 0x6, network 1) for `key_hash`.
fn enterprise_keyhash_address(key_hash: &Hash28) -> Vec<u8> {
    let mut addr = Vec::with_capacity(29);
    addr.push((0x6 << 4) | TEST_NETWORK); // 0x61
    addr.extend_from_slice(&key_hash.0);
    addr
}

fn tx_in(byte: u8, index: u16) -> TxIn {
    TxIn {
        tx_hash: Hash32([byte; 32]),
        index,
    }
}

/// A controlled Conway tx body spending `input` paying out `out_coin` with
/// `fee`. The single output is an enterprise key-hash address.
fn synth_body(input: TxIn, out_coin: u64, fee: u64) -> ConwayTxBody {
    let mut inputs = BTreeSet::new();
    inputs.insert(input);
    let out_addr = enterprise_keyhash_address(&Hash28([0x0B; 28]));
    ConwayTxBody {
        inputs,
        outputs: vec![BabbageTxOut {
            address: out_addr,
            coin: Coin(out_coin),
            multi_asset: None,
            datum_option: None,
            script_ref: None,
        }],
        fee: Coin(fee),
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
    // Encoding a controlled body never fails; a panic here would be a test-
    // harness bug, not a node behavior, so unwrap is acceptable in GREEN code.
    body.ade_encode(&mut buf, &ctx)
        .expect("synthetic Conway body encodes");
    buf
}

/// Build a Conway witness-set CBOR `{0: [[vkey, sig], ...]}`.
fn witness_set(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut buf = Vec::new();
    if entries.is_empty() {
        // Empty witness set: an empty map `{}`.
        cbor::write_map_header(&mut buf, ContainerEncoding::Definite(0, canonical_width(0)));
        return buf;
    }
    cbor::write_map_header(&mut buf, ContainerEncoding::Definite(1, canonical_width(1)));
    cbor::write_uint_canonical(&mut buf, 0); // key 0 = vkey witnesses
    cbor::write_array_header(
        &mut buf,
        ContainerEncoding::Definite(entries.len() as u64, canonical_width(entries.len() as u64)),
    );
    for (vkey, sig) in entries {
        cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        cbor::write_bytes_canonical(&mut buf, vkey);
        cbor::write_bytes_canonical(&mut buf, sig);
    }
    buf
}

/// One synthetic adversarial case: the full tx CBOR plus the LedgerState
/// (track_utxo=true) it must be judged against.
pub struct SyntheticCase {
    pub tx_cbor: Vec<u8>,
    pub ledger: LedgerState,
}

/// A track_utxo=true Conway ledger at epoch 576 holding a single UTxO at
/// `input` paying `coin` to an enterprise address controlled by `payment_key`.
fn ledger_with_utxo(input: &TxIn, payment_key: &Hash28, coin: u64) -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l.track_utxo = true;
    let addr = enterprise_keyhash_address(payment_key);
    // A minimal preserved Conway output `[address, coin]` for the AlonzoPlus raw
    // slice. Only the `address` field is read for required-signer derivation;
    // value conservation is the surface S1 exercises against this UTxO's coin.
    let mut raw = Vec::new();
    cbor::write_array_header(&mut raw, ContainerEncoding::Definite(2, canonical_width(2)));
    cbor::write_bytes_canonical(&mut raw, &addr);
    cbor::write_uint_canonical(&mut raw, coin);
    l.utxo_state.utxos.insert(
        input.clone(),
        TxOut::AlonzoPlus {
            raw,
            address: addr,
            coin: Coin(coin),
        },
    );
    l
}

/// Build the synthetic adversarial case for `mutation`.
///
/// Baseline: a balanced tx spending one controlled input worth 5_000_000,
/// paying 4_800_000 out with 200_000 fee, signed by the input payment key.
/// Each mutation perturbs exactly one dimension.
pub fn build_synthetic(mutation: SyntheticMutation) -> SyntheticCase {
    let pay = SynthKey::from_seed(0x11);
    let input = tx_in(0xA0, 0);
    let input_value: u64 = 5_000_000;

    match mutation {
        SyntheticMutation::ValueImbalance => {
            // outputs(4_900_000) + fee(200_000) = 5_100_000 != inputs(5_000_000).
            let body = synth_body(input.clone(), 4_900_000, 200_000);
            let body_bytes = encode_body(&body);
            let h = ade_crypto::blake2b_256(&body_bytes);
            let ws = witness_set(&[(pay.vkey.clone(), pay.sign(&h.0))]);
            let tx = assemble(&body_bytes, &ws);
            let ledger = ledger_with_utxo(&input, &pay.key_hash, input_value);
            SyntheticCase { tx_cbor: tx, ledger }
        }
        SyntheticMutation::MissingInputWitness => {
            let body = synth_body(input.clone(), 4_800_000, 200_000);
            let body_bytes = encode_body(&body);
            // No witnesses at all: the input payment key is uncovered.
            let ws = witness_set(&[]);
            let tx = assemble(&body_bytes, &ws);
            let ledger = ledger_with_utxo(&input, &pay.key_hash, input_value);
            SyntheticCase { tx_cbor: tx, ledger }
        }
        SyntheticMutation::DanglingInput => {
            // The body spends `input`, but the ledger holds a DIFFERENT UTxO.
            let body = synth_body(input.clone(), 4_800_000, 200_000);
            let body_bytes = encode_body(&body);
            let h = ade_crypto::blake2b_256(&body_bytes);
            let ws = witness_set(&[(pay.vkey.clone(), pay.sign(&h.0))]);
            let tx = assemble(&body_bytes, &ws);
            // UTxO keyed at a different input → the spend input is unresolvable.
            let other = tx_in(0xB0, 0);
            let ledger = ledger_with_utxo(&other, &pay.key_hash, input_value);
            SyntheticCase { tx_cbor: tx, ledger }
        }
        SyntheticMutation::ForgedInputWitness => {
            let body = synth_body(input.clone(), 4_800_000, 200_000);
            let body_bytes = encode_body(&body);
            // The witness carries the RIGHT vkey but a signature over a wrong
            // message — coverage-by-hash matches, signature does not verify.
            let wrong = [0x00u8; 32];
            let ws = witness_set(&[(pay.vkey.clone(), pay.sign(&wrong))]);
            let tx = assemble(&body_bytes, &ws);
            let ledger = ledger_with_utxo(&input, &pay.key_hash, input_value);
            SyntheticCase { tx_cbor: tx, ledger }
        }
    }
}

/// Assemble a full Conway tx `[body, witness_set, true, null]`.
fn assemble(body_bytes: &[u8], ws_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body_bytes.len() + ws_bytes.len() + 3);
    out.push(0x84); // array(4)
    out.extend_from_slice(body_bytes);
    out.extend_from_slice(ws_bytes);
    out.push(0xf5); // is_valid = true
    out.push(0xf6); // aux = null
    out
}

// ===========================================================================
// Verdict helper shared by both families
// ===========================================================================

/// Drive `tx_validity` over `tx_cbor` atop `ledger` and return the verdict plus
/// its canonical surface bytes. The calling test asserts the verdict is never
/// `Valid`.
pub fn judge(ledger: &LedgerState, tx_cbor: &[u8]) -> (TxValidityVerdict, Vec<u8>) {
    let outcome = tx_validity(ledger, tx_cbor);
    let surface = encode_tx_verdict_surface(&outcome.verdict);
    (outcome.verdict, surface)
}

/// A track_utxo=false Conway ledger at epoch 576 — the family-A judging state
/// (same scope as the positive corpus).
pub fn ledger_partial_at_576() -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l
}
