// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! One-shot fixture emitter for the aiken upstream bug report.
//!
//! Walks `corpus/contiguous/conway/` starting from the pre-Conway
//! snapshot, looking for the first tx where our Plutus evaluator
//! returns `PlutusFailed` but the block's `invalid_transactions`
//! does NOT list that tx index. That divergence is our "our fail,
//! chain accept" signature — the aiken ScriptContext inheritance
//! issue we want to report upstream.
//!
//! On match, writes a self-contained reproducer directory:
//!   target/aiken_divergent_fixture/
//!     tx_body.hex       — the full tx CBOR (body+witness+true+aux)
//!     inputs.hex        — Vec<TransactionInput> CBOR (aiken format)
//!     outputs.hex       — Vec<TransactionOutput> CBOR (aiken format)
//!     cost_models.hex   — the pparam cost_models CBOR (optional)
//!     meta.txt          — block filename, tx_idx, tx_hash, error
//!
//! These files drop directly into aiken's
//! `crates/uplc/src/tx/tests.rs::test_eval_*` pattern — the
//! maintainers can adopt them verbatim as a regression test.
//!
//! Marked `#[ignore]` because it runs for minutes and writes outside
//! `target/` at a path the upstream reporter reads.

use std::path::PathBuf;

use ade_codec::cbor::envelope::decode_block_envelope;
use ade_ledger::plutus_eval::{
    assemble_full_tx_cbor, build_resolved_utxos, decode_invalid_tx_indices,
};
use ade_ledger::rules::{apply_block_with_verdicts, TxOutcome};
use ade_testkit::harness::snapshot_loader::LoadedSnapshot;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
}

fn output_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("aiken_divergent_fixture")
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[test]
#[ignore = "generator — writes target/aiken_divergent_fixture/ for upstream reporting"]
fn emit_first_conway_divergent_tx() {
    let snap_path = corpus_root()
        .join("snapshots")
        .join("snapshot_133660855.tar.gz");
    let era_dir = corpus_root().join("contiguous").join("conway");
    let index_path = era_dir.join("blocks.json");

    if !snap_path.exists() {
        eprintln!("[skip] Conway snapshot missing");
        return;
    }
    if !index_path.exists() {
        eprintln!("[skip] Conway blocks.json missing");
        return;
    }

    let snap = LoadedSnapshot::from_tarball(&snap_path).unwrap();
    let mut state = snap.to_ledger_state();
    let cost_models_cbor = state.protocol_params.cost_models_cbor.clone();

    let index_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap()).unwrap();
    let blocks = index_json
        .get("blocks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    eprintln!("scanning {} Conway blocks for first divergence…", blocks.len());

    // We need per-block access to tx body/witness slices + pre-block
    // state so we can rebuild the exact inputs aiken was given. The
    // `apply_block_with_verdicts` call gives us the verdicts but eats
    // the state transition — so we snapshot state BEFORE applying and
    // decode the block ourselves to slice bytes.

    for entry in &blocks {
        let filename = entry["file"].as_str().unwrap();
        let raw = std::fs::read(era_dir.join(filename)).unwrap();
        let env = decode_block_envelope(&raw).unwrap();
        let inner = &raw[env.block_start..env.block_end];

        let pre_state = state.clone();

        let result = match apply_block_with_verdicts(&pre_state, env.era, inner) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  apply failed at {filename}: {e}");
                break;
            }
        };

        // Look for divergent fail
        let invalid = &result.invalid_tx_indices;
        let mut divergent_idx: Option<usize> = None;
        for v in &result.tx_verdicts {
            if matches!(v.outcome, TxOutcome::PlutusFailed { .. })
                && !invalid.contains(&(v.tx_index as u64))
            {
                divergent_idx = Some(v.tx_index);
                break;
            }
        }

        if let Some(tx_idx) = divergent_idx {
            eprintln!(
                "FOUND divergent tx: block={filename}, tx_idx={tx_idx}"
            );
            emit_fixture(&pre_state, inner, env.era, filename, tx_idx, cost_models_cbor.as_deref());
            return;
        }

        state = result.new_state;
    }

    eprintln!("no divergent tx found in scanned blocks");
}

fn emit_fixture(
    pre_state: &ade_ledger::state::LedgerState,
    block_inner: &[u8],
    era: ade_types::CardanoEra,
    block_filename: &str,
    tx_idx: usize,
    cost_models_cbor: Option<&[u8]>,
) {
    use ade_codec::{babbage, conway, cbor};
    use ade_codec::cbor::ContainerEncoding;

    // Decode block to slice out tx_bodies + witness_sets.
    let (tx_bodies_bytes, witness_sets_bytes) = match era {
        ade_types::CardanoEra::Alonzo => {
            let b = ade_codec::alonzo::decode_alonzo_block(block_inner).unwrap();
            let d = b.decoded();
            (d.tx_bodies.clone(), d.witness_sets.clone())
        }
        ade_types::CardanoEra::Babbage => {
            let b = babbage::decode_babbage_block(block_inner).unwrap();
            let d = b.decoded();
            (d.tx_bodies.clone(), d.witness_sets.clone())
        }
        ade_types::CardanoEra::Conway => {
            let b = conway::decode_conway_block(block_inner).unwrap();
            let d = b.decoded();
            (d.tx_bodies.clone(), d.witness_sets.clone())
        }
        _ => panic!("unsupported era for divergent-tx extraction"),
    };

    // Walk tx_bodies to slice out our tx_idx body.
    let (body_bytes, witness_bytes) =
        slice_tx(&tx_bodies_bytes, &witness_sets_bytes, tx_idx).unwrap();

    // Decode the body minimally to get inputs + collateral + refs.
    let (inputs, collateral, refs) = decode_input_sets(era, &body_bytes);

    // Assemble full tx CBOR (body + witness + is_valid=true + null aux).
    let tx_cbor = assemble_full_tx_cbor(&body_bytes, &witness_bytes);

    // Resolve UTxOs.
    let resolved = build_resolved_utxos(
        &inputs,
        collateral.as_ref(),
        refs.as_ref(),
        &pre_state.utxo_state.utxos,
        era,
    )
    .unwrap_or_default();

    // Rebuild inputs.cbor + outputs.cbor in aiken's test format:
    //   inputs.cbor  = Vec<TransactionInput>  CBOR
    //   outputs.cbor = Vec<TransactionOutput> CBOR
    let mut inputs_cbor = Vec::new();
    let mut outputs_cbor = Vec::new();
    let n = resolved.len() as u64;
    cbor::write_array_header(
        &mut inputs_cbor,
        ContainerEncoding::Definite(n, cbor::canonical_width(n)),
    );
    cbor::write_array_header(
        &mut outputs_cbor,
        ContainerEncoding::Definite(n, cbor::canonical_width(n)),
    );
    for (i, o) in &resolved {
        inputs_cbor.extend_from_slice(i);
        outputs_cbor.extend_from_slice(o);
    }

    // tx_hash for reference.
    let tx_hash = ade_crypto::blake2b_256(&body_bytes);

    // Write files.
    let out = output_root();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("tx_body.hex"), to_hex(&tx_cbor)).unwrap();
    std::fs::write(out.join("inputs.hex"), to_hex(&inputs_cbor)).unwrap();
    std::fs::write(out.join("outputs.hex"), to_hex(&outputs_cbor)).unwrap();
    if let Some(cm) = cost_models_cbor {
        std::fs::write(out.join("cost_models.hex"), to_hex(cm)).unwrap();
    }
    let meta = format!(
        "era: {era:?}\nblock_file: {block_filename}\ntx_idx: {tx_idx}\ntx_hash: {}\nresolved_utxo_count: {}\nbody_size_bytes: {}\nwitness_size_bytes: {}\n",
        to_hex(&tx_hash.0),
        resolved.len(),
        body_bytes.len(),
        witness_bytes.len(),
    );
    std::fs::write(out.join("meta.txt"), &meta).unwrap();

    eprintln!("\n=== Fixture written to {} ===\n{meta}", out.display());
}

fn slice_tx(
    tx_bodies: &[u8],
    witness_sets: &[u8],
    tx_idx: usize,
) -> Option<(Vec<u8>, Vec<u8>)> {
    use ade_codec::cbor::{self, ContainerEncoding};
    let mut bo = 0;
    let body_enc = cbor::read_array_header(tx_bodies, &mut bo).ok()?;
    let body_count = match body_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => u64::MAX,
    };
    let mut wo = 0;
    let wit_enc = cbor::read_array_header(witness_sets, &mut wo).ok()?;
    let _wit_count = match wit_enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => u64::MAX,
    };

    for i in 0..body_count {
        let body_start = bo;
        let _ = cbor::skip_item(tx_bodies, &mut bo).ok()?;
        let body_end = bo;
        let wit_start = wo;
        let _ = cbor::skip_item(witness_sets, &mut wo).ok()?;
        let wit_end = wo;
        if i as usize == tx_idx {
            return Some((
                tx_bodies[body_start..body_end].to_vec(),
                witness_sets[wit_start..wit_end].to_vec(),
            ));
        }
    }
    None
}

fn decode_input_sets(
    era: ade_types::CardanoEra,
    body_bytes: &[u8],
) -> (
    std::collections::BTreeSet<ade_types::tx::TxIn>,
    Option<std::collections::BTreeSet<ade_types::tx::TxIn>>,
    Option<std::collections::BTreeSet<ade_types::tx::TxIn>>,
) {
    let mut off = 0;
    match era {
        ade_types::CardanoEra::Alonzo => {
            let b = ade_codec::alonzo::tx::decode_alonzo_tx_body(body_bytes, &mut off).unwrap();
            (b.inputs, b.collateral_inputs, None)
        }
        ade_types::CardanoEra::Babbage => {
            let b = ade_codec::babbage::tx::decode_babbage_tx_body(body_bytes, &mut off).unwrap();
            (b.inputs, b.collateral_inputs, b.reference_inputs)
        }
        ade_types::CardanoEra::Conway => {
            let b = ade_codec::conway::tx::decode_conway_tx_body(body_bytes, &mut off).unwrap();
            (b.inputs, b.collateral_inputs, b.reference_inputs)
        }
        _ => unreachable!(),
    }
}

// Silence unused-import warnings when the test is compiled but not run.
#[allow(dead_code)]
fn _keep_imports() {
    let _ = decode_invalid_tx_indices;
}
