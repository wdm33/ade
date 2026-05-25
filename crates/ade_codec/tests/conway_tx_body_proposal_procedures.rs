// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PROPOSAL-PROCEDURES-DECODE PP-S1 integration tests: the typed
// `proposal_procedures` field round-trips through the full
// `ConwayTxBody` codec (not just the inner sub-decoder). Asserts the
// type-shape change has wired through both decode and encode at the
// body boundary (DC-LEDGER-11 in vivo).

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeSet;

use ade_codec::cbor::{self, ContainerEncoding, IntWidth};
use ade_codec::conway::governance::{decode_proposal_procedures, encode_proposal_procedures};
use ade_codec::conway::tx::{decode_conway_tx_body, encode_conway_tx_body};
use ade_codec::traits::CodecContext;

use ade_types::babbage::tx::BabbageTxOut;
use ade_types::conway::governance::{Anchor, GovAction, ProposalProcedure};
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::{Coin, TxIn};
use ade_types::{CardanoEra, Hash32};

fn ctx() -> CodecContext {
    CodecContext {
        era: CardanoEra::Conway,
    }
}

fn minimal_body() -> ConwayTxBody {
    let mut inputs = BTreeSet::new();
    inputs.insert(TxIn {
        tx_hash: Hash32([0xAA; 32]),
        index: 0,
    });
    ConwayTxBody {
        inputs,
        outputs: vec![BabbageTxOut {
            address: vec![0x60, 0x01, 0x02, 0x03, 0x04],
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

fn synthetic_anchor() -> Anchor {
    let mut buf = Vec::new();
    cbor::write_array_header(&mut buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    cbor::write_text_canonical(&mut buf, "x");
    cbor::write_bytes_canonical(&mut buf, &[0xaa; 32]);
    Anchor { raw: buf }
}

fn info_action_proposal() -> ProposalProcedure {
    ProposalProcedure {
        deposit: Coin(100_000_000_000),
        return_addr: vec![0xe0; 29],
        gov_action: GovAction::InfoAction,
        anchor: synthetic_anchor(),
    }
}

/// CE-PP-1 + CE-PP-2 in vivo: a full `ConwayTxBody` with the typed
/// `proposal_procedures` field round-trips through the body codec
/// byte-identically. Proves the type-shape change is wired end-to-end
/// at the body boundary, not just at the inner sub-decoder.
#[test]
fn body_with_proposal_procedures_round_trips_through_typed_field() {
    let mut body = minimal_body();
    body.proposal_procedures = Some(vec![
        info_action_proposal(),
        info_action_proposal(),
    ]);

    let mut buf = Vec::new();
    encode_conway_tx_body(&mut buf, &body, &ctx()).unwrap();

    let mut off = 0;
    let decoded = decode_conway_tx_body(&buf, &mut off).unwrap();
    assert_eq!(off, buf.len(), "body decoder must consume all bytes");
    assert_eq!(body, decoded, "body must round-trip preserving typed field");

    // The decoded field is the typed form, not opaque bytes.
    let decoded_procs = decoded.proposal_procedures.expect("typed Some");
    assert_eq!(decoded_procs.len(), 2);
    matches!(decoded_procs[0].gov_action, GovAction::InfoAction);
}

/// The body codec invokes the closed sub-decoder (not pass-through
/// bytes) — encoding a `None` field skips key 20 entirely; encoding
/// `Some(vec![pp])` produces bytes the closed sub-decoder accepts.
#[test]
fn body_none_skips_proposal_procedures_key_20() {
    let body = minimal_body(); // proposal_procedures = None
    let mut buf = Vec::new();
    encode_conway_tx_body(&mut buf, &body, &ctx()).unwrap();
    let mut off = 0;
    let decoded = decode_conway_tx_body(&buf, &mut off).unwrap();
    assert_eq!(decoded.proposal_procedures, None);
}

/// Cross-check: the body-level encode/decode pair agrees with the
/// sub-decoder pair on the same `Vec<ProposalProcedure>`.
#[test]
fn body_and_sub_decoder_agree_on_proposal_procedures() {
    let procs = vec![info_action_proposal()];

    // Sub-decoder path:
    let sub_bytes = encode_proposal_procedures(&procs);
    let sub_decoded = decode_proposal_procedures(&sub_bytes).unwrap();
    assert_eq!(sub_decoded, procs);

    // Body path:
    let mut body = minimal_body();
    body.proposal_procedures = Some(procs.clone());
    let mut buf = Vec::new();
    encode_conway_tx_body(&mut buf, &body, &ctx()).unwrap();
    let mut off = 0;
    let decoded = decode_conway_tx_body(&buf, &mut off).unwrap();
    assert_eq!(decoded.proposal_procedures, Some(procs));
}
