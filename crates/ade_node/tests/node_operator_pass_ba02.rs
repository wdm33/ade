// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-F-G-C S2 (RO-LIVE-06 BA-02 evidence wiring).
//!
//! Exercises the RED operator-pass evidence I/O (`ade_node::ba02_pass`):
//! operator-captured peer-accept JSONL log file -> `parse_peer_accept_events`
//! (allow-list) -> `correlate` (sole `Ba02Manifest` ctor). The hermetic tests
//! use a SYNTHETIC fixture for reducer/mechanics ONLY — a synthetic fixture
//! CANNOT satisfy BA-02 (it is never placed under the committed BA-02 manifest
//! home and never satisfies `ci_check_ba02_evidence_manifest_schema.sh`). A
//! real BA-02 needs the env-gated live operator pass below against a peer that
//! can grant leadership; that is `blocked_until_operator_stake_available`.
//!
//! Honesty line: Ade self-accept != peer acceptance; a served block != peer
//! acceptance; wire success != peer acceptance. Only a real operator-captured
//! peer log naming the EXACT forged hash, through `correlate`, yields a
//! manifest.
//!
//! Live operator pass (env-gated by `ADE_LIVE_OPERATOR_TEST=1`, mirroring
//! `admission_live_operator_pass.rs`). Required env when enabled:
//!   - ADE_LIVE_OPERATOR_TEST=1
//!   - ADE_LIVE_FORGED_BLOCK_HASH=<64-hex>   (the Ade-forged block hash)
//!   - ADE_LIVE_FORGED_SLOT=<integer>
//!   - ADE_LIVE_NETWORK_MAGIC=<integer>
//!   - ADE_LIVE_PEER_LOG=/path/to/operator-captured-peer.log (JSONL)
//!   - ADE_LIVE_BA02_MANIFEST_OUT=/path/to/ade-evidence.json

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::env;
use std::fs;
use std::path::PathBuf;

use ade_node::ba02_evidence::{AdeForgeRecord, BA02Outcome};
use ade_node::ba02_pass::{correlate_peer_log_file, write_ba02_manifest};
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

fn forge_record(byte: u8, slot: u64) -> AdeForgeRecord {
    AdeForgeRecord {
        forged_block_hash: Hash32([byte; 32]),
        slot,
        network_magic: 1,
    }
}

/// CE-G-C-2: the RED evidence-I/O path reads a peer-log FILE and runs it
/// through the GREEN reducer — `Ba02Manifest` on a matching `peer_served_block`
/// fixture; `NoEvidence` on an Ade-internal-only fixture (self-accept /
/// forge_succeeded / block_received lines, which the allow-list drops). The
/// synthetic fixture is reducer/mechanics ONLY — it lives in a TempDir, never
/// under the committed BA-02 manifest home, and never satisfies the gate.
#[test]
fn correlate_wired_to_operator_peer_log() {
    let dir = TempDir::new().unwrap();

    // Matching operator-captured peer-accept log: the peer SERVED the forged
    // block (strongest signal) at the matching slot.
    let served = dir.path().join("peer-served.log");
    fs::write(
        &served,
        format!(
            "{{\"event\":\"peer_served_block\",\"block_hash_hex\":\"{}\",\"slot\":7,\"peer\":\"127.0.0.1:3001\"}}\n",
            hex32(0xAB)
        ),
    )
    .unwrap();
    let outcome = correlate_peer_log_file(&forge_record(0xAB, 7), &served).unwrap();
    match outcome {
        BA02Outcome::Ba02Manifest(m) => {
            assert_eq!(m.forged_block_hash_hex, hex32(0xAB));
            assert_eq!(m.matched_block_hash_hex, hex32(0xAB));
            assert_eq!(m.slot, 7);
        }
        other => panic!("expected Ba02Manifest on a matching served-block log, got {other:?}"),
    }

    // Ade-internal-only log: self-accept / forge_succeeded / block_received are
    // NOT acceptance — the allow-list drops them, so there is NO evidence.
    let internal = dir.path().join("ade-internal.log");
    fs::write(
        &internal,
        format!(
            "{{\"event\":\"self_accept\",\"block_hash_hex\":\"{h}\",\"slot\":7}}\n\
             {{\"event\":\"forge_succeeded\",\"block_hash_hex\":\"{h}\",\"slot\":7}}\n\
             {{\"event\":\"block_received\",\"block_hash_hex\":\"{h}\",\"slot\":7}}\n",
            h = hex32(0xAB)
        ),
    )
    .unwrap();
    let outcome = correlate_peer_log_file(&forge_record(0xAB, 7), &internal).unwrap();
    assert!(
        matches!(outcome, BA02Outcome::NoEvidence { .. }),
        "Ade-internal-only signals are NOT peer acceptance — expected NoEvidence, got {outcome:?}"
    );

    // A missing peer-log file fails closed (io::Error), never a synthesized
    // acceptance and never a silent NoEvidence.
    let missing = dir.path().join("does-not-exist.log");
    assert!(
        correlate_peer_log_file(&forge_record(0xAB, 7), &missing).is_err(),
        "a missing peer-log file must fail closed (io::Error), not synthesize a verdict"
    );
}

/// `correlate` over a file is a pure function of its bytes — the same fixture
/// file yields a byte-identical `BA02Outcome` (and canonical-JSON manifest)
/// across two reads (the existing RO-LIVE-06 determinism property, over the
/// file-I/O path).
#[test]
fn correlate_from_operator_log_file_is_deterministic() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("peer-served.log");
    fs::write(
        &log,
        format!(
            "{{\"event\":\"peer_served_block\",\"block_hash_hex\":\"{}\",\"slot\":42,\"peer\":\"p\"}}\n",
            hex32(0x5C)
        ),
    )
    .unwrap();
    let a = correlate_peer_log_file(&forge_record(0x5C, 42), &log).unwrap();
    let b = correlate_peer_log_file(&forge_record(0x5C, 42), &log).unwrap();
    assert_eq!(a, b, "same fixture file => byte-identical BA02Outcome");
    if let (BA02Outcome::Ba02Manifest(ma), BA02Outcome::Ba02Manifest(mb)) = (&a, &b) {
        assert_eq!(
            ma.to_canonical_json(),
            mb.to_canonical_json(),
            "canonical-JSON manifest is byte-identical across reads"
        );
    } else {
        panic!("expected Ba02Manifest on the matching fixture, got {a:?}");
    }
}

/// Live operator pass — env-gated; the hermetic CI run skips it (env unset).
/// When enabled it reads the operator-captured peer log, correlates against
/// the Ade forge record, and writes the manifest ONLY on a real
/// `correlate`-produced `Ba02Manifest`. A `NoEvidence` outcome PANICS (the
/// peer did not accept) — it never writes a manifest. This is the only path
/// to a real BA-02 manifest; it is `blocked_until_operator_stake_available`.
#[test]
fn node_operator_pass_ba02_live() {
    if env::var("ADE_LIVE_OPERATOR_TEST").ok().as_deref() != Some("1") {
        eprintln!(
            "node_operator_pass_ba02_live: skipped (set ADE_LIVE_OPERATOR_TEST=1 + the \
             ADE_LIVE_FORGED_BLOCK_HASH/SLOT, ADE_LIVE_NETWORK_MAGIC, ADE_LIVE_PEER_LOG, \
             ADE_LIVE_BA02_MANIFEST_OUT env to run the operator pass)."
        );
        return;
    }
    let hash_hex = env::var("ADE_LIVE_FORGED_BLOCK_HASH").expect("ADE_LIVE_FORGED_BLOCK_HASH");
    let slot: u64 = env::var("ADE_LIVE_FORGED_SLOT")
        .expect("ADE_LIVE_FORGED_SLOT")
        .parse()
        .expect("ADE_LIVE_FORGED_SLOT integer");
    let network_magic: u32 = env::var("ADE_LIVE_NETWORK_MAGIC")
        .expect("ADE_LIVE_NETWORK_MAGIC")
        .parse()
        .expect("ADE_LIVE_NETWORK_MAGIC integer");
    let peer_log = PathBuf::from(env::var("ADE_LIVE_PEER_LOG").expect("ADE_LIVE_PEER_LOG"));
    let out = PathBuf::from(env::var("ADE_LIVE_BA02_MANIFEST_OUT").expect("ADE_LIVE_BA02_MANIFEST_OUT"));

    let ade = AdeForgeRecord {
        forged_block_hash: parse_hex32(&hash_hex),
        slot,
        network_magic,
    };
    match correlate_peer_log_file(&ade, &peer_log).expect("read operator-captured peer log") {
        BA02Outcome::Ba02Manifest(m) => {
            write_ba02_manifest(&m, &out).expect("write BA-02 manifest");
            println!("BA-02 manifest written to {}: {}", out.display(), m.to_canonical_json());
        }
        BA02Outcome::NoEvidence { reason } => panic!(
            "live operator pass produced NO BA-02 evidence ({reason:?}): the peer did NOT accept \
             the Ade-forged block. No manifest written. Re-check operator stake / leadership / \
             genesis-consistency — Ade self-accept is NOT acceptance."
        ),
    }
}
