// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED private-testnet rehearsal evidence I/O (PHASE4-N-F-G-D S2).
//!
//! Reads the operator-captured peer-accept JSONL log, runs it through the GREEN
//! `ba02_evidence::correlate` (via [`crate::ba02_pass::correlate_peer_log_file`]
//! — the SOLE `Ba02Manifest` constructor), and wraps a correlate-produced
//! payload in a [`PrivateRehearsalManifest`]. RED file I/O ONLY: it constructs
//! no evidence, synthesizes no acceptance, and uses no alternate correlator. A
//! `NoEvidence` outcome yields `None` (writes nothing); a missing/unreadable
//! peer-log file fails closed (`io::Error`) — never a synthesized manifest.
//! `[[feedback-shell-must-not-overstate-semantic-truth]]`.

use std::fs;
use std::io;
use std::path::Path;

use crate::ba02_evidence::AdeForgeRecord;
use crate::ba02_pass::correlate_peer_log_file;
use crate::rehearsal_evidence::{PrivateRehearsalManifest, RehearsalEnvelope};

/// Read the operator-captured peer-accept log, correlate it, and wrap a
/// correlate-produced manifest in the rehearsal envelope. `Ok(None)` iff
/// `correlate` returned `NoEvidence` (nothing to write). A missing/unreadable
/// file is an `io::Error` (fail closed), inherited from `correlate_peer_log_file`.
pub fn correlate_peer_log_file_into_rehearsal(
    ade: &AdeForgeRecord,
    peer_log_path: &Path,
    envelope: RehearsalEnvelope,
) -> io::Result<Option<PrivateRehearsalManifest>> {
    let outcome = correlate_peer_log_file(ade, peer_log_path)?;
    Ok(PrivateRehearsalManifest::from_correlate_outcome(
        &outcome, envelope,
    ))
}

/// Write a [`PrivateRehearsalManifest`] as canonical TOML to `out_path`. The
/// argument type is the gate: only a `PrivateRehearsalManifest` (which ONLY
/// [`PrivateRehearsalManifest::from_correlate_outcome`]'s `Ba02Manifest` arm
/// constructs) is writable — there is no path that emits a manifest from a
/// `NoEvidence` outcome or raw operator input, so a written rehearsal manifest
/// is ALWAYS correlate-produced.
pub fn write_private_rehearsal_manifest(
    manifest: &PrivateRehearsalManifest,
    out_path: &Path,
) -> io::Result<()> {
    fs::write(out_path, manifest.to_canonical_toml())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::ba02_evidence::{correlate, parse_peer_accept_events, BA02Outcome};
    use crate::rehearsal_evidence::RehearsalVenue;
    use ade_types::Hash32;

    fn forged_record() -> AdeForgeRecord {
        AdeForgeRecord {
            forged_block_hash: Hash32([0x11; 32]),
            slot: 124_140_368,
            network_magic: 42,
        }
    }

    fn forged_hex() -> String {
        "11".repeat(32)
    }

    fn envelope() -> RehearsalEnvelope {
        RehearsalEnvelope {
            venue: RehearsalVenue::PrivateTestnetC1,
            peer_log_file: "phase4-n-f-g-d-private-rehearsal-test-peer.log".to_string(),
            // 64-hex placeholder; the GATE re-verifies sha256 against the real
            // committed peer-log file (the no-synthetic binding).
            peer_log_file_sha256: "deadbeef".repeat(8),
        }
    }

    fn matching_peer_log() -> String {
        format!(
            "{{\"event\":\"peer_served_block\",\"block_hash_hex\":\"{}\",\"slot\":124140368,\"peer\":\"127.0.0.1:3010\"}}\n",
            forged_hex()
        )
    }

    #[test]
    fn rehearsal_envelope_wraps_correlate_produced_payload() {
        let ade = forged_record();
        let events = parse_peer_accept_events(&matching_peer_log());
        let outcome = correlate(&ade, &events);
        // The payload MUST be a correlate-produced Ba02Manifest.
        let expected_payload = match &outcome {
            BA02Outcome::Ba02Manifest(m) => m.clone(),
            other => panic!("expected a correlate-produced Ba02Manifest, got {:?}", other),
        };
        let manifest = PrivateRehearsalManifest::from_correlate_outcome(&outcome, envelope())
            .expect("a correlate-produced payload wraps into a rehearsal manifest");
        // The wrapped payload is correlate's output, verbatim.
        assert_eq!(manifest.ba02, expected_payload);
        // The serialized envelope carries the non-promotable rehearsal markers.
        let toml = manifest.to_canonical_toml();
        assert!(toml.contains("is_rehearsal = true"));
        assert!(toml.contains("not_bounty_evidence = true"));
        assert!(toml.contains("venue = \"private-testnet-c1\""));
        assert!(toml.contains(&format!(
            "peer_log_file_sha256 = \"{}\"",
            "deadbeef".repeat(8)
        )));
        assert!(toml.contains(&format!("forged_block_hash_hex = \"{}\"", forged_hex())));
    }

    #[test]
    fn rehearsal_correlate_no_evidence_writes_nothing() {
        let ade = forged_record();
        // A peer log naming a DIFFERENT hash at the forged slot => NoEvidence.
        let non_matching = format!(
            "{{\"event\":\"peer_served_block\",\"block_hash_hex\":\"{}\",\"slot\":124140368,\"peer\":\"127.0.0.1:3010\"}}\n",
            "22".repeat(32)
        );
        let dir = std::env::temp_dir();
        let log_path = dir.join("ade_gd_s2_no_evidence_peer.log");
        let out_path = dir.join("ade_gd_s2_no_evidence_manifest.toml");
        let _ = fs::remove_file(&out_path); // ensure clean
        fs::write(&log_path, non_matching).unwrap();

        let result = correlate_peer_log_file_into_rehearsal(&ade, &log_path, envelope()).unwrap();
        // NoEvidence => None => nothing to wrap.
        assert!(result.is_none(), "NoEvidence must yield no rehearsal manifest");
        // The write fn is never reached; the out path must not exist.
        assert!(!out_path.exists(), "NoEvidence writes nothing");

        let _ = fs::remove_file(&log_path);
    }

    #[test]
    fn rehearsal_envelope_is_structurally_distinct_from_ba02_manifest() {
        let ade = forged_record();
        let events = parse_peer_accept_events(&matching_peer_log());
        let outcome = correlate(&ade, &events);
        let manifest =
            PrivateRehearsalManifest::from_correlate_outcome(&outcome, envelope()).expect("manifest");
        let rehearsal_toml = manifest.to_canonical_toml();
        let bare_ba02_json = manifest.ba02.to_canonical_json();

        // The rehearsal envelope carries markers a bare Ba02Manifest lacks.
        assert!(rehearsal_toml.contains("is_rehearsal = true"));
        assert!(rehearsal_toml.contains("not_bounty_evidence = true"));
        assert!(!bare_ba02_json.contains("is_rehearsal"));
        assert!(!bare_ba02_json.contains("not_bounty_evidence"));

        // It does NOT satisfy the bounty schema's required-field set: at least
        // one bounty-required field (a peer-log capture field) is absent, so a
        // rehearsal manifest cannot satisfy the bounty gate even if mislocated.
        assert!(!rehearsal_toml.contains("accept_event_kind"));
        assert!(!rehearsal_toml.contains("peer_log_capture_command"));
    }
}
