// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED operator-pass BA-02 evidence I/O (PHASE4-N-F-G-C S2).
//!
//! Reads the operator-captured peer-accept JSONL log from disk and runs it
//! through the GREEN [`crate::ba02_evidence`] reducer
//! ([`parse_peer_accept_events`] + [`correlate`] — the SOLE [`Ba02Manifest`]
//! constructor). This module is RED file I/O ONLY: it constructs no evidence,
//! derives no acceptance, and never coerces a non-acceptance line. `correlate`
//! stays the sole authority; a [`Ba02Manifest`] is a CLAIM ABOUT authority,
//! not authority (`[[feedback-evidence-reducers-are-green-not-authority]]`).
//!
//! Honesty line (`[[feedback-shell-must-not-overstate-semantic-truth]]`): Ade
//! self-accept != peer acceptance; a served block != peer acceptance; wire
//! success != peer acceptance. The ONLY input that can yield a manifest is a
//! real operator-captured peer log naming the EXACT forged hash, through
//! `correlate`. A missing/unreadable peer-log file fails closed (`io::Error`),
//! never a synthesized acceptance.

use std::fs;
use std::io;
use std::path::Path;

use crate::ba02_evidence::{
    correlate, parse_peer_accept_events, AdeForgeRecord, BA02Outcome, Ba02Manifest,
};

/// Read an operator-captured peer-accept JSONL log file and correlate it
/// against the Ade forge record: file bytes -> [`parse_peer_accept_events`]
/// (allow-list) -> [`correlate`] (sole [`Ba02Manifest`] ctor). Pure
/// pass-through; no acceptance is synthesized. A missing/unreadable file is an
/// [`io::Error`] (fail-closed), NOT a `NoEvidence` and NOT a manifest.
pub fn correlate_peer_log_file(
    ade: &AdeForgeRecord,
    peer_log_path: &Path,
) -> io::Result<BA02Outcome> {
    let log = fs::read_to_string(peer_log_path)?;
    let events = parse_peer_accept_events(&log);
    Ok(correlate(ade, &events))
}

/// Write a [`correlate`]-produced [`Ba02Manifest`] as canonical JSON to
/// `out_path`. The argument type is the gate: only a [`Ba02Manifest`] (which
/// ONLY `correlate`'s exact-match arm constructs) is writable — there is no
/// path here that emits a manifest from a `NoEvidence` outcome or from raw
/// operator input, so a written manifest is ALWAYS `correlate`-produced.
pub fn write_ba02_manifest(manifest: &Ba02Manifest, out_path: &Path) -> io::Result<()> {
    fs::write(out_path, manifest.to_canonical_json())
}
