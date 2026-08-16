//! BND-2d (INV-BND-2d) — the block NAMES the collateral bindings the UTxO authority must retain.
//!
//! The positioning contract (`docs/clusters/PREPROD-LIVE-2-FORGE-READINESS/SLICE-BND-2d-...md`):
//! a collateral value is authoritative in `[create(x), B)` where `B` is the block that spends it,
//! so the LAST instant the authority can answer is the moment it applies `B`. This half is the BLUE
//! derivation that tells the authority WHICH bindings to keep at that instant. It introduces no
//! second reader of body field 13 — `extract_tx_utxo_effect` remains the single derivation
//! (INV-BND-2a), and for a phase-2-invalid tx its `spends` IS the collateral input list.

use ade_ledger::reduced_advance::reduced_block_delta;
use ade_types::CardanoEra;

/// The real block that pinned the live accumulator: Conway, one tx, phase-2 invalid, one collateral
/// input `0326ab20…#1`, no collateral return, no declared total collateral.
const BLOCK_130350133: &[u8] = include_bytes!("fixtures/block_130350133.cbor");
/// A real Conway block with NO invalid transactions — the scope control.
const VALID_BLOCK: &[u8] = include_bytes!("../../ade_node/tests/fixtures/raw_era_block_conway.cbor");

fn hx(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn decode(bytes: &[u8]) -> ade_types::shelley::block::ShelleyBlock {
    let env = ade_codec::cbor::envelope::decode_block_envelope(bytes).expect("envelope");
    ade_codec::conway::decode_conway_block(&bytes[env.block_start..env.block_end])
        .expect("conway block")
        .decoded()
        .clone()
}

/// CE-2d-1 (derivation half) — on the REAL failing block, the delta names exactly the one binding
/// the authority is about to destroy, and marks it as one the authority itself must value
/// (`None` — it was not created by this block).
#[test]
fn the_real_failing_block_names_its_collateral_binding_for_retention() {
    let block = decode(BLOCK_130350133);
    let delta = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");

    let named: Vec<(String, Option<u64>)> = delta
        .collateral_consumed
        .iter()
        .map(|(i, v)| (format!("{}#{}", hx(&i.tx_hash.0), i.index), v.map(|c| c.0)))
        .collect();

    assert_eq!(
        named,
        vec![(
            "0326ab20d9cf533634f9d6838ae327971ac1606ab69f4378c1fc8009091e225a#1".to_string(),
            None
        )],
        "the block must name its one collateral binding, unvalued (it predates this block)"
    );

    // The named binding is EXACTLY what the block spends — the retention cannot name an input the
    // authority does not remove, or it would keep a live entry alive under a second key.
    assert_eq!(
        delta.collateral_consumed.len(),
        delta.spent.len(),
        "for this block every spend IS the collateral spend"
    );
    assert_eq!(delta.collateral_consumed[0].0, delta.spent[0]);
}

/// CE-2d-9 — SCOPE. Only collateral of a phase-2-INVALID tx is named. A block of ordinary traffic
/// spends plenty of inputs and must name none of them: retaining every spend would turn a bounded
/// retention into a second, unbounded UTxO history.
#[test]
fn an_ordinary_spend_is_never_named_for_retention() {
    let block = decode(VALID_BLOCK);
    let delta = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");
    assert!(
        !delta.spent.is_empty(),
        "the control block must actually spend inputs, or this proves nothing"
    );
    assert!(
        delta.collateral_consumed.is_empty(),
        "a block with no phase-2-invalid tx names NO collateral binding, got {:?}",
        delta.collateral_consumed
    );
}

/// The naming is deterministic and in canonical order — it is durable input to a store write.
#[test]
fn the_named_bindings_are_deterministic() {
    let block = decode(BLOCK_130350133);
    let a = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");
    let b = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");
    assert_eq!(a.collateral_consumed, b.collateral_consumed);
    assert_eq!(a, b, "the whole delta replays identically");
}

/// The pre-existing effect is UNCHANGED — `spent` / `produced` are what BND-2a proved, so the
/// reduced UTxO a store records does not move. Only the naming is additive.
#[test]
fn the_reduced_utxo_effect_itself_is_unchanged() {
    let block = decode(BLOCK_130350133);
    let delta = reduced_block_delta(&block, CardanoEra::Conway).expect("delta");
    let spent: Vec<String> = delta
        .spent
        .iter()
        .map(|i| format!("{}#{}", hx(&i.tx_hash.0), i.index))
        .collect();
    assert_eq!(
        spent,
        vec!["0326ab20d9cf533634f9d6838ae327971ac1606ab69f4378c1fc8009091e225a#1".to_string()]
    );
    assert!(delta.produced.is_empty(), "no collateral return is declared");
}
