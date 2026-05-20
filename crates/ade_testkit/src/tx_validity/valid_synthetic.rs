// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN VALID synthetic tx builders for the mempool admission gate (B2-S5).
//!
//! The B2-S4 [`super::adversarial`] family builds INvalid synthetic txs over a
//! controlled UTxO. The mempool admission gate (CE-B2-5) also needs VALID
//! synthetic txs — admit-able against a controlled track_utxo=true ledger — and
//! a DEPENDENT pair (tx B spending tx A's output) to prove that admission
//! re-validates against the accumulating state. Those builders live here, reusing
//! the same deterministic keygen / enterprise-address / canonical-encode recipe.
//!
//! Non-authoritative. `BTreeSet`/`Vec` only; no `HashMap`, no float, no clock.

use std::collections::BTreeSet;

use ade_codec::cbor::{self, canonical_width, ContainerEncoding};
use ade_codec::traits::{AdeEncode, CodecContext};
use ade_ledger::state::LedgerState;
use ade_ledger::utxo::TxOut;
use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32};

use ed25519_dalek::{Signer, SigningKey};

const TEST_NETWORK: u8 = 1; // mainnet nibble, matching ProtocolParameters default
const EPOCH_576: EpochNo = EpochNo(576);

/// Deterministic Ed25519 key material (mirrors `adversarial::SynthKey`).
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

/// A controlled Conway tx body spending `input`, paying `out_coin` to an
/// enterprise key-hash address controlled by `out_key_hash`, with `fee`.
fn synth_body(input: TxIn, out_key_hash: &Hash28, out_coin: u64, fee: u64) -> ConwayTxBody {
    let mut inputs = BTreeSet::new();
    inputs.insert(input);
    ConwayTxBody {
        inputs,
        outputs: vec![BabbageTxOut {
            address: enterprise_keyhash_address(out_key_hash),
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

/// The blake2b-256 tx id of a preserved Conway body — the key under which its
/// outputs land in the UTxO (`apply_conway_tx_to_utxo`).
fn body_tx_id(body_bytes: &[u8]) -> Hash32 {
    ade_crypto::blake2b_256(body_bytes)
}

/// A track_utxo=true Conway ledger at epoch 576 holding a single UTxO at
/// `input` paying `coin` to an enterprise address controlled by `payment_key`.
fn ledger_with_utxo(input: &TxIn, payment_key: &Hash28, coin: u64) -> LedgerState {
    let mut l = LedgerState::new(CardanoEra::Conway);
    l.epoch_state.epoch = EPOCH_576;
    l.track_utxo = true;
    // Conway states carry their canonical deposit params (mainnet values);
    // tx_validity's view assembly requires them present.
    l.conway_deposit_params = Some(ade_ledger::pparams::ConwayOnlyDepositParams {
        drep_deposit: Coin(500_000_000),
        gov_action_deposit: Coin(100_000_000_000),
    });
    let addr = enterprise_keyhash_address(payment_key);
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

/// A single valid synthetic case: the full tx CBOR plus the track_utxo=true
/// `LedgerState` it admits against.
pub struct ValidCase {
    pub tx_cbor: Vec<u8>,
    pub ledger: LedgerState,
}

/// Build ONE valid synthetic tx: spend a single controlled input worth
/// 5_000_000, paying 4_800_000 to a controlled enterprise output with 200_000
/// fee, signed by the input payment key. `tx_validity(&case.ledger, &tx_cbor)`
/// is `Valid` — the admit positive-path target.
pub fn build_valid() -> ValidCase {
    let pay = SynthKey::from_seed(0x21);
    let out_key = SynthKey::from_seed(0x22);
    let input = tx_in(0xC0, 0);
    let input_value: u64 = 5_000_000;

    let body = synth_body(input.clone(), &out_key.key_hash, 4_800_000, 200_000);
    let body_bytes = encode_body(&body);
    let h = body_tx_id(&body_bytes);
    let ws = witness_set(&[(pay.vkey.clone(), pay.sign(&h.0))]);
    let tx_cbor = assemble(&body_bytes, &ws);
    let ledger = ledger_with_utxo(&input, &pay.key_hash, input_value);
    ValidCase { tx_cbor, ledger }
}

/// A dependent pair: tx A spends a controlled input; tx B spends A's only
/// output. B is valid ONLY against the state AFTER A is applied (the
/// accumulating UTxO), and unresolvable against the base ledger.
pub struct DependentPair {
    /// The base ledger (track_utxo=true) holding only A's input UTxO.
    pub ledger: LedgerState,
    /// tx A: spends the base UTxO, produces an output keyed by A's tx id.
    pub tx_a: Vec<u8>,
    /// tx B: spends A's output (`TxIn { A.tx_id, 0 }`).
    pub tx_b: Vec<u8>,
}

/// Build the dependent pair. tx A spends a controlled input worth 5_000_000,
/// paying 4_800_000 to a key the harness controls, with 200_000 fee. tx B then
/// spends A's `index 0` output (4_800_000), paying 4_600_000 with 200_000 fee.
/// B's input is `TxIn { tx_hash: blake2b_256(A.body), index: 0 }`, which exists
/// in the UTxO only after A is applied.
pub fn build_dependent_pair() -> DependentPair {
    let a_in_key = SynthKey::from_seed(0x31); // controls A's input
    let a_out_key = SynthKey::from_seed(0x32); // controls A's output == B's input
    let b_out_key = SynthKey::from_seed(0x33); // controls B's output
    let a_input = tx_in(0xD0, 0);
    let a_input_value: u64 = 5_000_000;

    // tx A.
    let a_body = synth_body(a_input.clone(), &a_out_key.key_hash, 4_800_000, 200_000);
    let a_body_bytes = encode_body(&a_body);
    let a_id = body_tx_id(&a_body_bytes);
    let a_ws = witness_set(&[(a_in_key.vkey.clone(), a_in_key.sign(&a_id.0))]);
    let tx_a = assemble(&a_body_bytes, &a_ws);

    // tx B spends A's output (TxIn { A.tx_id, 0 }), signed by A's output key.
    let b_input = TxIn {
        tx_hash: a_id,
        index: 0,
    };
    let b_body = synth_body(b_input, &b_out_key.key_hash, 4_600_000, 200_000);
    let b_body_bytes = encode_body(&b_body);
    let b_id = body_tx_id(&b_body_bytes);
    let b_ws = witness_set(&[(a_out_key.vkey.clone(), a_out_key.sign(&b_id.0))]);
    let tx_b = assemble(&b_body_bytes, &b_ws);

    let ledger = ledger_with_utxo(&a_input, &a_in_key.key_hash, a_input_value);
    DependentPair {
        ledger,
        tx_a,
        tx_b,
    }
}
