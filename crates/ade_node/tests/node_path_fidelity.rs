//! PHASE4-N-F-G-D S1 — path-fidelity proof (CE-G-D-1, clause 1 of
//! CN-REHEARSAL-FIDELITY-01).
//!
//! Proves the `--mode node` accepted-block path's consensus-inputs ingestion is
//! venue-agnostic: a private / epoch-0-shaped extraction and a synced-preprod-tip
//! shaped extraction both import through the SINGLE shared
//! `import_live_consensus_inputs_from_bytes` (the exact function
//! `node_lifecycle.rs` consumes), with no venue parameter or branch. This is the
//! OQ1 proof — the C1 dry-run uses the identical import path as the eventual
//! preprod bounty pass; "fast slots" come from the operator-authored private
//! genesis stake allocation (operator input), never from a private-only code path.

use ade_runtime::consensus_inputs::import_live_consensus_inputs_from_bytes;

/// A synced-preprod-tip-shaped bundle: current epoch (200), an in-window tip,
/// ASC 1/20 — what the eventual preprod bounty pass extracts at the live tip.
const PREPROD_TIP_SHAPE: &str = r#"{
    "network_magic": 1,
    "genesis_hash_hex": "00000000000000000000000000000000000000000000000000000000000000aa",
    "era": "conway",
    "epoch_no": 200,
    "epoch_start_slot": 86400000,
    "epoch_end_slot": 86832000,
    "active_slots_coeff": {"numer": 1, "denom": 20},
    "epoch_nonce_hex": "00000000000000000000000000000000000000000000000000000000000000bb",
    "pool_distribution": {
        "00000000000000000000000000000000000000000000000000000001": {"active_stake": 123456789}
    },
    "pool_vrf_keyhashes": {
        "00000000000000000000000000000000000000000000000000000001": "00000000000000000000000000000000000000000000000000000000000000cc"
    },
    "protocol_params_hash_hex": "00000000000000000000000000000000000000000000000000000000000000dd",
    "source_cardano_node_version": "cardano-node 11.0.1",
    "source_query_command": "cardano-cli conway query stake-snapshot --testnet-magic 1",
    "source_tip_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ee",
    "source_tip_slot": 86400500
}"#;

/// A private / epoch-0-shaped bundle: the genesis epoch (0), the origin tip
/// (slot 0, within `[epoch_start_slot, epoch_end_slot]`), operator-controlled
/// ~all stake, ASC 1/2 (fast leadership). SAME envelope, different DATA only —
/// extracted from a fresh private follower peer via the same cardano-cli queries.
const PRIVATE_EPOCH0_SHAPE: &str = r#"{
    "network_magic": 42,
    "genesis_hash_hex": "00000000000000000000000000000000000000000000000000000000000000aa",
    "era": "conway",
    "epoch_no": 0,
    "epoch_start_slot": 0,
    "epoch_end_slot": 432000,
    "active_slots_coeff": {"numer": 1, "denom": 2},
    "epoch_nonce_hex": "00000000000000000000000000000000000000000000000000000000000000bb",
    "pool_distribution": {
        "00000000000000000000000000000000000000000000000000000001": {"active_stake": 1000000000000}
    },
    "pool_vrf_keyhashes": {
        "00000000000000000000000000000000000000000000000000000001": "00000000000000000000000000000000000000000000000000000000000000cc"
    },
    "protocol_params_hash_hex": "00000000000000000000000000000000000000000000000000000000000000dd",
    "source_cardano_node_version": "cardano-node 11.0.1",
    "source_query_command": "cardano-cli conway query protocol-state --testnet-magic 42",
    "source_tip_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ee",
    "source_tip_slot": 0
}"#;

/// CE-G-D-1 path fidelity: both venue shapes import through the SAME shared
/// importer. There is exactly one `import_live_consensus_inputs_from_bytes`
/// function and it takes no venue parameter — so "both shapes import Ok" is the
/// proof that the C1 dry-run cannot diverge from the preprod accepted-block path.
#[test]
fn node_accepted_block_consensus_inputs_via_shared_import() {
    // Same function, preprod-tip shape.
    let tip = import_live_consensus_inputs_from_bytes(PREPROD_TIP_SHAPE.as_bytes())
        .expect("preprod-tip-shaped bundle imports via the shared importer");

    // Same function, private/epoch-0 shape — no venue branch, no from-genesis
    // constructor, no private-only path. If THIS failed, the fix would be in the
    // shared importer (preprod would gain the same acceptance), never a
    // private-only workaround (N0).
    let priv0 = import_live_consensus_inputs_from_bytes(PRIVATE_EPOCH0_SHAPE.as_bytes())
        .expect("private/epoch-0-shaped bundle imports via the SAME shared importer");

    // They are genuinely different venue shapes (the importer parsed the distinct
    // values, did not silently coerce) — yet ONE function accepted both.
    assert_eq!(tip.epoch_no.0, 200, "preprod-tip shape carries the current epoch");
    assert_eq!(priv0.epoch_no.0, 0, "private shape carries the genesis epoch");
    assert_eq!(
        priv0.epoch_start_slot.0, 0,
        "private shape starts at the genesis slot (origin tip in-window)"
    );
    assert_ne!(
        tip.epoch_no.0, priv0.epoch_no.0,
        "the two bundles are distinct venue shapes, both accepted by the one importer"
    );
}
