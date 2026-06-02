// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-F-G-D S3 (C1 dry-run operator-gated scaffold).
//!
//! The env-gated test below is the operator EXECUTION HARNESS for the C1
//! private-testnet rehearsal evidence path; it remains skipped in CI and is NOT
//! a runtime node mode (no binary arm produces a rehearsal manifest). It
//! exercises the SAME accepted-block path as the preprod bounty pass (S1
//! fences path fidelity) and produces a NON-PROMOTABLE `PrivateRehearsalManifest`
//! (S2 fences the evidence artifact).
//!
//! Hard line: Ade self-accept != peer acceptance; a served block != peer
//! acceptance; wire success != peer acceptance. Only a real operator-captured
//! Haskell peer log naming the EXACT forged hash, through `correlate`, yields a
//! manifest. C1 acceptance != bounty completion; this flips NO RO-LIVE rule.
//!
//! Live run (env-gated by `ADE_LIVE_C1_DRY_RUN=1`, mirroring
//! `node_operator_pass_ba02_live`). Required env when enabled:
//!   - ADE_LIVE_C1_DRY_RUN=1
//!   - ADE_LIVE_FORGED_BLOCK_HASH=<64-hex>
//!   - ADE_LIVE_FORGED_SLOT=<integer>
//!   - ADE_LIVE_NETWORK_MAGIC=<integer>
//!   - ADE_LIVE_PEER_LOG=/path/to/operator-captured-peer.log (JSONL)
//!   - ADE_LIVE_REHEARSAL_PEER_LOG_SHA256=<sha256sum of the committed peer log>
//!   - ADE_LIVE_REHEARSAL_MANIFEST_OUT=/path/to/phase4-n-f-g-d-private-rehearsal-<run>.toml

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::env;
use std::fs;
use std::path::PathBuf;

use ade_node::ba02_evidence::AdeForgeRecord;
use ade_node::rehearsal_evidence::{RehearsalEnvelope, RehearsalVenue};
use ade_node::rehearsal_pass::{
    correlate_peer_log_file_into_rehearsal, write_private_rehearsal_manifest,
};
use ade_types::Hash32;
use tempfile::TempDir;

/// 64-char lowercase hex of a byte repeated 32 times (`Hash32([byte; 32])`).
fn hex32(byte: u8) -> String {
    format!("{:02x}", byte).repeat(32)
}

/// Parse a 64-char hex string into a `Hash32` (operator-supplied forged hash).
fn parse_hex32(s: &str) -> Hash32 {
    assert_eq!(s.len(), 64, "forged block hash must be 64 hex chars");
    let mut arr = [0u8; 32];
    for (i, slot) in arr.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("valid hex");
    }
    Hash32(arr)
}

fn forge_record(byte: u8, slot: u64, magic: u32) -> AdeForgeRecord {
    AdeForgeRecord {
        forged_block_hash: Hash32([byte; 32]),
        slot,
        network_magic: magic,
    }
}

fn served_log(hash_hex: &str, slot: u64) -> String {
    format!(
        "{{\"event\":\"peer_served_block\",\"block_hash_hex\":\"{}\",\"slot\":{},\"peer\":\"127.0.0.1:3010\"}}\n",
        hash_hex, slot
    )
}

/// The operator computes `peer_log_file_sha256` via `sha256sum`; the rehearsal
/// gate re-verifies it against the committed file. Hermetic tests pass a
/// placeholder (the wrapper does not verify the sha256 — the gate does).
fn envelope(sha: &str) -> RehearsalEnvelope {
    RehearsalEnvelope {
        venue: RehearsalVenue::PrivateTestnetC1,
        peer_log_file: "phase4-n-f-g-d-private-rehearsal-test-peer.log".to_string(),
        peer_log_file_sha256: sha.to_string(),
    }
}

/// CE-G-D-3 hermetic wiring: file -> `correlate_peer_log_file_into_rehearsal` ->
/// `Some(manifest)` -> `write_private_rehearsal_manifest` -> read back, assert
/// the rehearsal markers + the correlate-produced payload. The `NoEvidence` path
/// yields `None` and writes nothing; a missing peer log fails closed. The output
/// lives in a `TempDir`, never the committed rehearsal home — the gate stays
/// vacuous (no synthetic manifest).
#[test]
fn c1_dry_run_correlate_to_rehearsal_envelope() {
    let dir = TempDir::new().unwrap();

    // Matching peer-served-block fixture at the forged (hash, slot).
    let log = dir.path().join("peer.log");
    fs::write(&log, served_log(&hex32(0xAB), 7)).unwrap();
    let out = dir.path().join("phase4-n-f-g-d-private-rehearsal-TEST.toml");

    let ade = forge_record(0xAB, 7, 42);
    let manifest = correlate_peer_log_file_into_rehearsal(&ade, &log, envelope(&hex32(0xEE)))
        .unwrap()
        .expect("matching peer log => a correlate-produced rehearsal manifest");
    write_private_rehearsal_manifest(&manifest, &out).unwrap();

    let written = fs::read_to_string(&out).unwrap();
    assert!(written.contains("is_rehearsal = true"));
    assert!(written.contains("not_bounty_evidence = true"));
    assert!(written.contains("venue = \"private-testnet-c1\""));
    assert!(written.contains(&format!("forged_block_hash_hex = \"{}\"", hex32(0xAB))));
    assert!(written.contains(&format!("matched_block_hash_hex = \"{}\"", hex32(0xAB))));
    assert!(written.contains("slot = 7"));
    assert!(written.contains("peer_accept_source = \"served_block\""));

    // NoEvidence path: a peer log naming a DIFFERENT hash => None => nothing written.
    let bad_log = dir.path().join("peer-nomatch.log");
    fs::write(&bad_log, served_log(&hex32(0x22), 7)).unwrap();
    let bad_out = dir.path().join("phase4-n-f-g-d-private-rehearsal-NOMATCH.toml");
    let none =
        correlate_peer_log_file_into_rehearsal(&ade, &bad_log, envelope(&hex32(0xEE))).unwrap();
    assert!(none.is_none(), "NoEvidence must yield no rehearsal manifest");
    assert!(!bad_out.exists(), "NoEvidence writes nothing");

    // A missing peer log fails closed (io::Error), never a synthesized manifest.
    let missing = dir.path().join("does-not-exist.log");
    assert!(
        correlate_peer_log_file_into_rehearsal(&ade, &missing, envelope(&hex32(0xEE))).is_err(),
        "a missing peer-log file must fail closed (io::Error)"
    );
}

/// Operator execution harness — env-gated; CI skips it (env unset). NOT a node
/// mode. When enabled it reads the operator-captured peer log, correlates,
/// wraps a correlate-produced payload into a `PrivateRehearsalManifest`, and
/// writes it ONLY on a real match. `NoEvidence` PANICS (no manifest). The live
/// run is `blocked_until_operator_c1_net_executed` and flips NO RO-LIVE rule.
#[test]
fn node_c1_dry_run_rehearsal_live() {
    if env::var("ADE_LIVE_C1_DRY_RUN").ok().as_deref() != Some("1") {
        eprintln!(
            "node_c1_dry_run_rehearsal_live: skipped (set ADE_LIVE_C1_DRY_RUN=1 + \
             ADE_LIVE_FORGED_BLOCK_HASH/SLOT, ADE_LIVE_NETWORK_MAGIC, ADE_LIVE_PEER_LOG, \
             ADE_LIVE_REHEARSAL_PEER_LOG_SHA256, ADE_LIVE_REHEARSAL_MANIFEST_OUT to run the C1 \
             dry-run. PRIVATE-TESTNET REHEARSAL — not bounty evidence, no RO-LIVE flip.)"
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

    // The envelope's peer_log_file is the committed basename of the peer log.
    let peer_log_file = peer_log
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "peer.log".to_string());
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
        .expect("read operator-captured peer log")
    {
        Some(manifest) => {
            write_private_rehearsal_manifest(&manifest, &out).expect("write rehearsal manifest");
            println!(
                "C1 private-testnet REHEARSAL manifest written to {} (NOT bounty evidence; no RO-LIVE flip):\n{}",
                out.display(),
                manifest.to_canonical_toml()
            );
        }
        None => panic!(
            "C1 dry-run produced NO evidence: the Haskell peer did NOT accept the Ade-forged \
             block. No manifest written. Re-check operator stake / leadership / \
             genesis-consistency — Ade self-accept is NOT acceptance."
        ),
    }
}
