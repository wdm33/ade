// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-F-G-H S3 (C1 node-spine serve-to-peer operator harness).
//!
//! The env-gated test below is the operator EXECUTION HARNESS for the C1 serve
//! direction: a real Haskell follower dials Ade's `--listen` (S2), handshakes
//! against Ade's magic-aware serve table (S2b), ChainSync-discovers + BlockFetches
//! Ade's forged block. It remains skipped in CI and is NOT a runtime node mode.
//! It reuses the G-D rehearsal evidence path VERBATIM (`correlate` -> non-promotable
//! `PrivateRehearsalManifest`); the hermetic correlate->manifest wiring is already
//! proven by `c1_dry_run_correlate_to_rehearsal_envelope`
//! (node_c1_dry_run_rehearsal.rs), so this file adds no second hermetic test.
//!
//! Hard line: Ade self-accept != peer acceptance; a served block != peer
//! acceptance; wire success != peer acceptance. The follower is EXPECTED to accept
//! a protocol-valid block, but the only accepted/served claim comes from the
//! follower's log naming the EXACT forged hash, through `correlate`. C1 acceptance
//! != bounty completion; this flips NO RO-LIVE rule.
//!
//! Live run (env-gated by `ADE_LIVE_C1_SERVE=1`; see
//! docs/evidence/phase4-n-f-g-h-node-serve-README.md). Required env when enabled:
//!   - ADE_LIVE_C1_SERVE=1
//!   - ADE_LIVE_FORGED_BLOCK_HASH=<64-hex>
//!   - ADE_LIVE_FORGED_SLOT=<integer>
//!   - ADE_LIVE_NETWORK_MAGIC=<integer>   (42 for C1)
//!   - ADE_LIVE_PEER_LOG=/path/to/operator-captured-follower.log (JSONL)
//!   - ADE_LIVE_REHEARSAL_PEER_LOG_SHA256=<sha256sum of the committed log>
//!   - ADE_LIVE_REHEARSAL_MANIFEST_OUT=/path/to/phase4-n-f-g-h-node-serve-<run>.toml

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::env;
use std::path::PathBuf;

use ade_node::ba02_evidence::AdeForgeRecord;
use ade_node::rehearsal_evidence::{RehearsalEnvelope, RehearsalVenue};
use ade_node::rehearsal_pass::{
    correlate_peer_log_file_into_rehearsal, write_private_rehearsal_manifest,
};
use ade_types::Hash32;

/// Parse a 64-char hex string into a `Hash32` (operator-supplied forged hash).
fn parse_hex32(s: &str) -> Hash32 {
    assert_eq!(s.len(), 64, "forged block hash must be 64 hex chars");
    let mut arr = [0u8; 32];
    for (i, slot) in arr.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("valid hex");
    }
    Hash32(arr)
}

/// Operator execution harness — env-gated; CI skips it (env unset). NOT a node
/// mode. When enabled it reads the operator-captured HASKELL-FOLLOWER log (the
/// follower that dialed Ade's `--listen` and fetched Ade's served block),
/// correlates, wraps a correlate-produced payload into a non-promotable
/// `PrivateRehearsalManifest`, and writes it ONLY on a real `peer_served_block`
/// match for the exact forged hash. `NoEvidence` PANICS (no manifest). The live
/// run is `blocked_until_operator_c1_serve_executed` and flips NO RO-LIVE rule.
#[test]
fn node_c1_serve_live() {
    if env::var("ADE_LIVE_C1_SERVE").ok().as_deref() != Some("1") {
        eprintln!(
            "node_c1_serve_live: skipped (set ADE_LIVE_C1_SERVE=1 + ADE_LIVE_FORGED_BLOCK_HASH/SLOT, \
             ADE_LIVE_NETWORK_MAGIC, ADE_LIVE_PEER_LOG, ADE_LIVE_REHEARSAL_PEER_LOG_SHA256, \
             ADE_LIVE_REHEARSAL_MANIFEST_OUT to run the C1 node-spine serve dry-run per \
             docs/evidence/phase4-n-f-g-h-node-serve-README.md. PRIVATE-TESTNET REHEARSAL — \
             not bounty evidence, no RO-LIVE flip.)"
        );
        return;
    }
    let hash_hex = env::var("ADE_LIVE_FORGED_BLOCK_HASH").expect("ADE_LIVE_FORGED_BLOCK_HASH");
    let slot: u64 = env::var("ADE_LIVE_FORGED_SLOT")
        .expect("ADE_LIVE_FORGED_SLOT")
        .parse()
        .expect("ADE_LIVE_FORGED_SLOT integer");
    let magic: u32 = env::var("ADE_LIVE_NETWORK_MAGIC")
        .expect("ADE_LIVE_NETWORK_MAGIC")
        .parse()
        .expect("ADE_LIVE_NETWORK_MAGIC integer");
    let peer_log = PathBuf::from(env::var("ADE_LIVE_PEER_LOG").expect("ADE_LIVE_PEER_LOG"));
    let sha = env::var("ADE_LIVE_REHEARSAL_PEER_LOG_SHA256")
        .expect("ADE_LIVE_REHEARSAL_PEER_LOG_SHA256");
    let out = PathBuf::from(
        env::var("ADE_LIVE_REHEARSAL_MANIFEST_OUT").expect("ADE_LIVE_REHEARSAL_MANIFEST_OUT"),
    );

    // The envelope's peer_log_file is the committed basename of the follower log.
    let peer_log_file = peer_log
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "follower.log".to_string());
    let env_meta = RehearsalEnvelope {
        venue: RehearsalVenue::PrivateTestnetC1,
        peer_log_file,
        peer_log_file_sha256: sha,
    };
    let ade = AdeForgeRecord {
        forged_block_hash: parse_hex32(&hash_hex),
        slot,
        network_magic: magic,
    };

    match correlate_peer_log_file_into_rehearsal(&ade, &peer_log, env_meta)
        .expect("read operator-captured follower log")
    {
        Some(manifest) => {
            write_private_rehearsal_manifest(&manifest, &out).expect("write rehearsal manifest");
            println!(
                "C1 node-spine SERVE rehearsal manifest written to {} (NOT bounty evidence; no RO-LIVE flip):\n{}",
                out.display(),
                manifest.to_canonical_toml()
            );
        }
        None => panic!(
            "C1 serve dry-run produced NO evidence: the Haskell follower did NOT accept the \
             Ade-served block (no peer_served_block match for the forged hash). No manifest \
             written. Re-check stake / leadership / genesis-consistency / topology — Ade \
             self-accept + a served block are NOT acceptance."
        ),
    }
}
