// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// B1-S5 truth test — the 14 real Conway-576 blocks are the oracle. A correct
// Praos single-VRF + KES construction validates all 14 as elected blocks; any
// leader-check failure means the VRF construction is wrong, never that the
// check should be relaxed.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_codec::cbor::{self, envelope::decode_block_envelope, ContainerEncoding};
use ade_core::consensus::{
    validate_and_apply_header, BootstrapAnchorHash, EraSchedule, EraSummary, HeaderApplied,
    HeaderInput, HeaderKes, HeaderValidationError, HeaderVrf, Nonce, PraosChainDepState,
};
use ade_crypto::blake2b::{blake2b_224, blake2b_256};
use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
use ade_testkit::validity::{pool_distr_view_from_corpus, ConwayValidityCorpus};
use ade_types::shelley::block::VrfData;
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

const EPOCH_576: EpochNo = EpochNo(576);
/// Mainnet slot at which epoch 577 begins (epoch 576 is `[start_576, this)`).
const EPOCH_577_START: u64 = 163_900_800;
const MAINNET_EPOCH_LENGTH: u64 = 432_000;

/// A single-era schedule that locates every corpus slot in epoch 576. Built so
/// `locate(slot).epoch == 576` for the corpus blocks (slots 163_900_639..784).
fn schedule() -> EraSchedule {
    let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
    let eras = vec![EraSummary {
        randomness_stabilisation_window_slots: None,
        era: CardanoEra::Conway,
        start_slot: SlotNo(start_576),
        start_epoch: EPOCH_576,
        slot_length_ms: 1_000,
        epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
        safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
    }];
    EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras)
        .expect("schedule is well-formed")
}

/// Genesis-like Praos state seeded with the corpus epoch nonce. `last_slot` is
/// `None` so the monotone-slot check admits the first corpus block.
fn state_with_eta0(eta0: [u8; 32]) -> PraosChainDepState {
    let mut s = PraosChainDepState::empty();
    s.epoch_nonce = Nonce(Hash32(eta0));
    s.evolving_nonce = Nonce(Hash32(eta0));
    s
}

/// One decoded corpus header, projected to a Praos `HeaderInput`.
struct CorpusHeader {
    input: HeaderInput,
    /// `blake2b_256(vrf_vkey)` — the binding the snapshot keyhash must match.
    vrf_keyhash: Hash32,
}

/// Build the Praos `HeaderInput` for the `i`-th corpus block.
fn corpus_header(corpus: &ConwayValidityCorpus, i: usize) -> CorpusHeader {
    let env_bytes = &corpus.blocks[i];
    let env = decode_block_envelope(env_bytes).expect("envelope decodes");
    let inner = &env_bytes[env.block_start..env.block_end];
    let block = ade_codec::conway::decode_conway_block(inner).expect("conway block decodes");
    let block = block.decoded();
    let hb = &block.header.body;

    // Extract the certified VRF output + proof from the combined vrf_result.
    let vrf_result = match &hb.vrf {
        VrfData::Combined { vrf_result } => vrf_result,
        VrfData::Split { .. } => panic!("Conway header must carry a combined VRF"),
    };
    let (output_bytes, proof_bytes) = parse_combined_vrf(vrf_result);

    let mut vk_arr = [0u8; 32];
    vk_arr.copy_from_slice(&hb.vrf_vkey);
    let vrf_vk = VrfVerificationKey(vk_arr);

    let mut out_arr = [0u8; 64];
    out_arr.copy_from_slice(&output_bytes);
    let mut proof_arr = [0u8; 80];
    proof_arr.copy_from_slice(&proof_bytes);

    // The header-body CBOR bytes are the KES message; the KES signature is the
    // inner bytes of the array(2) header's second element.
    let body_bytes = header_body_bytes(inner);
    let kes_signature = unwrap_cbor_bytes(&block.header.kes_signature);

    let issuer_pool = Hash28(blake2b_224(&hb.issuer_vkey).0);

    let kes = HeaderKes {
        issuer_vkey: hb.issuer_vkey.clone(),
        kes_vkey: hb.operational_cert.hot_vkey.clone(),
        kes_signature,
        op_cert_signature: hb.operational_cert.sigma.clone(),
        header_body_bytes: body_bytes,
    };

    let input = HeaderInput {
        prev_hash: Hash32([0u8; 32]),
        slot: SlotNo(hb.slot),
        block_no: BlockNo(hb.block_number),
        body_hash: hb.body_hash.clone(),
        issuer_pool,
        op_cert_kes_period: hb.operational_cert.kes_period,
        op_cert_counter: hb.operational_cert.sequence_number,
        vrf_vk,
        vrf: HeaderVrf::Praos {
            proof: VrfProof(proof_arr),
            output: VrfOutput(out_arr),
        },
        kes: Some(kes),
    };

    CorpusHeader {
        input,
        vrf_keyhash: Hash32(blake2b_256(&hb.vrf_vkey).0),
    }
}

/// The Babbage/Conway combined VRF cert is `array(2)[bytes(64) output, bytes(80) proof]`.
fn parse_combined_vrf(b: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut o = 0usize;
    match cbor::read_array_header(b, &mut o) {
        Ok(ContainerEncoding::Definite(2, _)) => {}
        _ => panic!("combined VRF must be array(2)"),
    }
    let (output, _) = cbor::read_bytes(b, &mut o).expect("vrf output bytes");
    let (proof, _) = cbor::read_bytes(b, &mut o).expect("vrf proof bytes");
    assert_eq!(output.len(), 64, "vrf output is 64 bytes");
    assert_eq!(proof.len(), 80, "vrf proof is 80 bytes");
    (output, proof)
}

/// The header-body CBOR bytes within an inner Conway block. The block is
/// `array(N)[ header, ... ]`; the header is `array(2)[ body, kes_sig ]`.
fn header_body_bytes(inner: &[u8]) -> Vec<u8> {
    let mut o = 0usize;
    let _ = cbor::read_array_header(inner, &mut o).expect("block array");
    let _ = cbor::read_array_header(inner, &mut o).expect("header array");
    let body_start = o;
    let (_, body_end) = cbor::skip_item(inner, &mut o).expect("header body item");
    inner[body_start..body_end].to_vec()
}

/// Unwrap a CBOR `bytes(..)` item, returning the inner byte string.
fn unwrap_cbor_bytes(b: &[u8]) -> Vec<u8> {
    let mut o = 0usize;
    let (bytes, _) = cbor::read_bytes(b, &mut o).expect("cbor bytes");
    bytes
}

/// Validate a single corpus header against a fresh state.
fn validate_block(
    corpus: &ConwayValidityCorpus,
    i: usize,
) -> Result<HeaderApplied, HeaderValidationError> {
    let ch = corpus_header(corpus, i);
    let view = pool_distr_view_from_corpus(corpus, EPOCH_576).expect("pool distr view");
    let state = state_with_eta0(corpus.epoch_nonce);
    validate_and_apply_header(&state, &ch.input, &view, &schedule())
}

#[test]
fn conway_corpus_headers_all_pass_leader_check() {
    let corpus = ConwayValidityCorpus::load().expect("corpus loads");
    assert_eq!(corpus.blocks.len(), 14, "expected 14 Conway-576 blocks");

    let view = pool_distr_view_from_corpus(&corpus, EPOCH_576).expect("pool distr view");

    for i in 0..corpus.blocks.len() {
        let ch = corpus_header(&corpus, i);

        // VRF keyhash binding (B1-S1 cross-check, re-asserted here).
        let registered = ade_core::consensus::ledger_view::LedgerView::pool_vrf_keyhash(
            &view,
            EPOCH_576,
            &ch.input.issuer_pool,
        )
        .expect("issuing pool is in the distribution");
        assert_eq!(
            registered, ch.vrf_keyhash,
            "block {i}: blake2b_256(vrf_vkey) must equal the registered keyhash"
        );

        let state = state_with_eta0(corpus.epoch_nonce);
        let res = validate_and_apply_header(&state, &ch.input, &view, &schedule());
        assert!(
            res.is_ok(),
            "block {i} (slot {}) failed header validation: {:?}",
            ch.input.slot.0,
            res.err()
        );
    }
}

#[test]
fn conway_corpus_nonce_contribution_is_deterministic() {
    let corpus = ConwayValidityCorpus::load().expect("corpus loads");
    let a = validate_block(&corpus, 0).expect("block 0 valid");
    let b = validate_block(&corpus, 0).expect("block 0 valid (replay)");
    assert_eq!(
        a.new_state.evolving_nonce, b.new_state.evolving_nonce,
        "same header twice must evolve the nonce identically"
    );
    assert_eq!(a, b, "validation is byte-identical across runs");
}

#[test]
fn praos_vrf_keyhash_mismatch_rejected() {
    let corpus = ConwayValidityCorpus::load().expect("corpus loads");
    let ch = corpus_header(&corpus, 0);
    let view = pool_distr_view_from_corpus(&corpus, EPOCH_576).expect("pool distr view");
    let state = state_with_eta0(corpus.epoch_nonce);

    // Tamper the header VRF key so its hash no longer matches the snapshot.
    let mut bad = ch.input.clone();
    let mut k = bad.vrf_vk.0;
    k[0] ^= 0xFF;
    bad.vrf_vk = VrfVerificationKey(k);

    match validate_and_apply_header(&state, &bad, &view, &schedule()) {
        Err(HeaderValidationError::VrfKeyhashMismatch { .. }) => {}
        other => panic!("expected VrfKeyhashMismatch, got {other:?}"),
    }
}

#[test]
fn praos_malformed_vrf_proof_rejected() {
    let corpus = ConwayValidityCorpus::load().expect("corpus loads");
    let ch = corpus_header(&corpus, 0);
    let view = pool_distr_view_from_corpus(&corpus, EPOCH_576).expect("pool distr view");
    let state = state_with_eta0(corpus.epoch_nonce);

    // Corrupt the VRF proof so the single-VRF verify fails.
    let mut bad = ch.input.clone();
    bad.vrf = match bad.vrf {
        HeaderVrf::Praos { proof, output } => {
            let mut p = proof.0;
            p[0] ^= 0xFF;
            HeaderVrf::Praos {
                proof: VrfProof(p),
                output,
            }
        }
        HeaderVrf::Tpraos { .. } => panic!("corpus is Praos"),
    };

    match validate_and_apply_header(&state, &bad, &view, &schedule()) {
        Err(HeaderValidationError::VrfCert(_)) => {}
        other => panic!("expected VrfCert error, got {other:?}"),
    }
}

#[test]
fn praos_malformed_kes_sig_rejected() {
    let corpus = ConwayValidityCorpus::load().expect("corpus loads");
    let ch = corpus_header(&corpus, 0);
    let view = pool_distr_view_from_corpus(&corpus, EPOCH_576).expect("pool distr view");
    let state = state_with_eta0(corpus.epoch_nonce);

    // Truncate the KES signature so the fixed-size field guard rejects it.
    let mut bad = ch.input.clone();
    if let Some(kes) = bad.kes.as_mut() {
        kes.kes_signature.truncate(447);
    }

    match validate_and_apply_header(&state, &bad, &view, &schedule()) {
        Err(HeaderValidationError::MalformedField(fe)) => {
            assert_eq!(fe.actual, 447);
            assert_eq!(fe.expected, 448);
        }
        other => panic!("expected MalformedField, got {other:?}"),
    }
}
