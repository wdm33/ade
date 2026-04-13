// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Conway mainnet Plutus-tx divergence reproducer.
//!
//! Standalone self-contained reproducer for the aiken upstream issue:
//! a Conway mainnet tx that the chain accepted (tx_idx NOT in its
//! block's `invalid_transactions` field) but aiken's
//! `eval_phase_two_raw` rejects with a validator trace pointing at
//! the `txInfoValidRange` / "expiration time" user-level check.
//!
//! Source:
//!   block_file:  blk_00073_chunk06188_idx00073.cbor
//!   tx_idx:      7
//!   tx_hash:     d97b843494511d57bfa7fba05ea40855de6663472b7c2fd8557a3b114054826f
//!
//! This test is marked `#[ignore]` because **it is currently expected
//! to fail** (that is the whole point — it demonstrates the bug). Run
//! with `cargo test -p ade_plutus --test conway_validity_range_reproducer
//! -- --ignored --nocapture` when tracking the upstream fix.
//!
//! When aiken's upstream patches the ScriptContext validity-range
//! issue, this test should pass. At that point, remove the `#[ignore]`
//! and it becomes a regression guard.

use ade_plutus::{eval_tx_phase_two, tx_eval::MAINNET_SLOT_CONFIG};

/// Full tx CBOR `[body, witness_set, is_valid=true, aux=null]`.
const TX_BODY_HEX: &str = include_str!(
    "../../../target/aiken_divergent_fixture/tx_body.hex"
);

/// `Vec<TransactionInput>` CBOR (2 inputs).
const INPUTS_HEX: &str = include_str!(
    "../../../target/aiken_divergent_fixture/inputs.hex"
);

/// `Vec<TransactionOutput>` CBOR (2 outputs — the script input + collateral).
const OUTPUTS_HEX: &str = include_str!(
    "../../../target/aiken_divergent_fixture/outputs.hex"
);

/// The pparams `cost_models` CBOR from the snapshot at the Conway
/// entry boundary — contains V1 / V2 / V3 coefficient tables as used
/// on mainnet at slot 133660855.
const COST_MODELS_HEX: &str = include_str!(
    "../../../target/aiken_divergent_fixture/cost_models.hex"
);

fn decode_hex(s: &str) -> Vec<u8> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = cleaned.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        out.push((hex_digit(bytes[i]) << 4) | hex_digit(bytes[i + 1]));
        i += 2;
    }
    out
}

fn hex_digit(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("bad hex digit: {b:?}"),
    }
}

fn split_array_items(cbor: &[u8]) -> Vec<Vec<u8>> {
    use ade_codec::cbor::{self, ContainerEncoding};
    let mut off = 0;
    let enc = cbor::read_array_header(cbor, &mut off).expect("array header");
    let mut items = Vec::new();
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let start = off;
                cbor::skip_item(cbor, &mut off).expect("skip item");
                items.push(cbor[start..off].to_vec());
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(cbor, off).expect("break") {
                let start = off;
                cbor::skip_item(cbor, &mut off).expect("skip item");
                items.push(cbor[start..off].to_vec());
            }
        }
    }
    items
}

#[test]
#[ignore = "currently-failing reproducer — demonstrates the aiken ScriptContext bug"]
fn conway_validity_range_divergence() {
    let tx_cbor = decode_hex(TX_BODY_HEX);
    let inputs_cbor = decode_hex(INPUTS_HEX);
    let outputs_cbor = decode_hex(OUTPUTS_HEX);
    let cost_models_cbor = decode_hex(COST_MODELS_HEX);

    let inputs = split_array_items(&inputs_cbor);
    let outputs = split_array_items(&outputs_cbor);
    assert_eq!(
        inputs.len(),
        outputs.len(),
        "inputs and outputs must zip 1:1"
    );

    let resolved_utxos: Vec<(Vec<u8>, Vec<u8>)> =
        inputs.into_iter().zip(outputs.into_iter()).collect();

    let result = eval_tx_phase_two(
        &tx_cbor,
        &resolved_utxos,
        Some(&cost_models_cbor),
        (10_000_000_000, 14_000_000),
        MAINNET_SLOT_CONFIG,
    );

    match result {
        Ok(res) => {
            // Expected path: once aiken upstream fixes the
            // ScriptContext bug, this branch fires and we can flip
            // this test from #[ignore] to a regression guard.
            eprintln!(
                "upstream appears fixed — {} script(s) evaluated",
                res.scripts.len()
            );
            for s in &res.scripts {
                assert!(s.success, "all scripts must pass on mainnet-accepted tx");
            }
        }
        Err(e) => {
            // Currently-observed state. Prints the full error chain so
            // maintainers can confirm the reproducer hits the same
            // signature they're debugging.
            eprintln!("DIVERGENCE OBSERVED (expected while upstream is unpatched)");
            eprintln!("  tx_hash: d97b843494511d57bfa7fba05ea40855de6663472b7c2fd8557a3b114054826f");
            eprintln!("  error:   {e:?}");
            panic!(
                "Conway mainnet tx rejected by eval_phase_two_raw — \
                 expected Ok because chain accepted this tx"
            );
        }
    }
}
