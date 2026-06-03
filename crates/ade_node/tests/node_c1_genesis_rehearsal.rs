// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Integration test — PHASE4-N-F-G-J S5 (C1 genesis-successor rehearsal,
//! operator-gated).
//!
//! The env-gated test below is the operator EXECUTION HARNESS for the C1
//! private-testnet GENESIS-SUCCESSOR rehearsal: the node cold-starts from a
//! recovered-lineage WarmStart with NO persisted block tip, forges block 0 +
//! `PrevHash::Genesis` (S4 reachability + S3 null-prev), and a follower Haskell
//! peer validates/fetches the null-prev block. It is skipped in CI and is NOT a
//! runtime node mode. It exercises the SAME `--mode node` cold-start path
//! (`ci_check_node_path_fidelity.sh` fences fidelity) and produces a
//! NON-PROMOTABLE `PrivateRehearsalManifest` under the genesis-rehearsal home
//! `docs/evidence/phase4-n-f-g-j-genesis-rehearsal-*.toml`. The hermetic
//! mechanics + genesis binding live in `forge_succeeds.rs`
//! (`genesis_rehearsal_manifest_binds_block_zero_genesis`).
//!
//! Hard line: Ade self-accept != peer acceptance; a served block != peer
//! acceptance; wire success != peer acceptance. Only a real operator-captured
//! Haskell peer log naming the EXACT forged genesis-block hash, through
//! `correlate`, yields a manifest. C1 acceptance != bounty completion; this
//! flips NO RO-LIVE rule. The live run is
//! `blocked_until_operator_c1_genesis_successor_rehearsal`.
//!
//! Live run (env-gated by `ADE_LIVE_C1_GENESIS_REHEARSAL=1`). Required env when
//! enabled:
//!   - ADE_LIVE_C1_GENESIS_REHEARSAL=1
//!   - ADE_LIVE_FORGED_BLOCK_HASH=<64-hex of the Ade-forged genesis block>
//!   - ADE_LIVE_FORGED_SLOT=<integer>
//!   - ADE_LIVE_NETWORK_MAGIC=<integer>
//!   - ADE_LIVE_PEER_LOG=/path/to/operator-captured-peer.log (JSONL)
//!   - ADE_LIVE_REHEARSAL_PEER_LOG_SHA256=<sha256sum of the committed peer log>
//!   - ADE_LIVE_REHEARSAL_MANIFEST_OUT=/path/to/phase4-n-f-g-j-genesis-rehearsal-<run>.toml

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
    assert_eq!(s.len(), 64, "forged genesis block hash must be 64 hex chars");
    let mut arr = [0u8; 32];
    for (i, slot) in arr.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("valid hex");
    }
    Hash32(arr)
}

/// Operator execution harness — env-gated; CI skips it (env unset). NOT a node
/// mode. When enabled it reads the operator-captured follower log for the
/// cold-start GENESIS-SUCCESSOR block, correlates, and writes a
/// `PrivateRehearsalManifest` ONLY on a real match. `NoEvidence` PANICS (no
/// manifest). The live run is
/// `blocked_until_operator_c1_genesis_successor_rehearsal` and flips NO
/// RO-LIVE rule.
#[test]
fn node_c1_genesis_rehearsal_live() {
    if env::var("ADE_LIVE_C1_GENESIS_REHEARSAL").ok().as_deref() != Some("1") {
        eprintln!(
            "node_c1_genesis_rehearsal_live: skipped (set ADE_LIVE_C1_GENESIS_REHEARSAL=1 + \
             ADE_LIVE_FORGED_BLOCK_HASH/SLOT, ADE_LIVE_NETWORK_MAGIC, ADE_LIVE_PEER_LOG, \
             ADE_LIVE_REHEARSAL_PEER_LOG_SHA256, ADE_LIVE_REHEARSAL_MANIFEST_OUT to run the C1 \
             genesis-successor rehearsal. PRIVATE-TESTNET REHEARSAL — not bounty evidence, no \
             RO-LIVE flip; blocked_until_operator_c1_genesis_successor_rehearsal.)"
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
    let sha =
        env::var("ADE_LIVE_REHEARSAL_PEER_LOG_SHA256").expect("ADE_LIVE_REHEARSAL_PEER_LOG_SHA256");
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
        .expect("read operator-captured follower log")
    {
        Some(manifest) => {
            write_private_rehearsal_manifest(&manifest, &out).expect("write rehearsal manifest");
            println!(
                "C1 GENESIS-SUCCESSOR rehearsal manifest written to {} (NOT bounty evidence; no RO-LIVE flip):\n{}",
                out.display(),
                manifest.to_canonical_toml()
            );
        }
        None => panic!(
            "C1 genesis rehearsal produced NO evidence: the Haskell follower did NOT accept the \
             Ade-forged genesis-successor (block 0 + null prev_hash). No manifest written. \
             Re-check operator stake / leadership / genesis-consistency / the null-prev wire \
             encoding — Ade self-accept is NOT acceptance."
        ),
    }
}
