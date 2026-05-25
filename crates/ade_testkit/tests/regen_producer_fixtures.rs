// Regen helper for PHASE4-N-C S3 producer replay fixtures.
//
// Invocation:
//   cargo test -p ade_testkit --test regen_producer_fixtures -- --ignored --nocapture
//
// Prints byte-literal constants suitable for pasting into
// `crates/ade_testkit/src/producer/fixtures.rs`. The helper recomputes
// the cold-key signature (deterministic ed25519 per RFC-8032) and the
// expected forged-block bytes from the fixed seeds pinned in
// `fixtures.rs`. Running the helper a second time yields byte-identical
// output — the only path that ever pulls RED signing primitives.

#![allow(clippy::unwrap_used)]

use ade_ledger::producer::forge::forge_block;
use ade_testkit::producer::fixtures;
use ed25519_dalek::{Signer, SigningKey as DalekSk};

fn print_byte_literal(name: &str, bytes: &[u8]) {
    println!("pub const {}: &[u8] = &[", name);
    for line in bytes.chunks(12) {
        let parts: Vec<String> = line.iter().map(|b| format!("0x{:02x}", b)).collect();
        println!("    {},", parts.join(", "));
    }
    println!("];");
}

fn print_byte_array(name: &str, n: usize, bytes: &[u8]) {
    println!("pub const {}: [u8; {}] = [", name, n);
    for line in bytes.chunks(15) {
        let parts: Vec<String> = line.iter().map(|b| format!("0x{:02x}", b)).collect();
        println!("    {},", parts.join(", "));
    }
    println!("];");
}

#[test]
#[ignore]
fn regen_producer_fixture_artifacts() {
    let cold = DalekSk::from_bytes(&fixtures::COLD_SEED);
    let cold_vk_bytes = *cold.verifying_key().as_bytes();

    let mut signable = Vec::with_capacity(48);
    signable.extend_from_slice(&fixtures::HOT_VKEY_BYTES);
    signable.extend_from_slice(&fixtures::OPCERT_SEQUENCE_NUMBER.to_be_bytes());
    signable.extend_from_slice(&fixtures::OPCERT_KES_PERIOD.to_be_bytes());
    let sigma = cold.sign(&signable).to_bytes();

    println!("// ---- COLD_VK_BYTES (regenerated) ----");
    print_byte_array("COLD_VK_BYTES", 32, &cold_vk_bytes);
    println!();
    println!("// ---- OPCERT_SIGMA (regenerated) ----");
    print_byte_array("OPCERT_SIGMA", 64, &sigma);
    println!();

    // Forge the positive fixture once and print its bytes.
    let fixture = fixtures::fixture_empty_mempool_leader();
    let (forged, _) = forge_block(&fixture.ticks[0]).expect("empty-mempool leader must forge");
    println!("// ---- EXPECTED_FORGED_EMPTY_MEMPOOL_LEADER (regenerated, {} bytes) ----", forged.bytes.len());
    print_byte_literal("EXPECTED_FORGED_EMPTY_MEMPOOL_LEADER", &forged.bytes);
}
