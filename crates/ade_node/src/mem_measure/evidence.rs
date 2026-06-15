//! GREEN: the MEM-MEASURE evidence record, its validator, and the
//! replay-fingerprint pairing.
//!
//! The record pairs a memory measurement (RED RSS observations) with a REPLAY
//! FINGERPRINT + VERDICT over the authoritative workload output. The verdict is
//! computed ONLY from the fingerprint pairing — never from the RSS numbers. A
//! measurement whose replay verdict is not `Agreed` is INVALID evidence: a
//! low-memory run that silently changed the authoritative output proves
//! nothing. This is the load-bearing MEM-MEASURE discipline. Pure: no clock,
//! no RNG, no `HashMap`, no float, no I/O.

use serde::{Deserialize, Serialize};

/// The replay verdict pairing two runs of the same workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayVerdict {
    /// The two authoritative fingerprints are byte-identical — valid evidence.
    Agreed,
    /// They differ — the measurement perturbed an authoritative output; INVALID.
    Diverged,
}

/// One MEM-MEASURE evidence record. The same schema is populated hermetically
/// (A1, `venue = "hermetic"`) and live (A2, `venue = "C2-LOCAL"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemEvidenceRecord {
    /// Stable scenario label (e.g. `mem-measure-a1-bounded-ingress`).
    pub scenario_id: String,
    /// Git SHA the measurement was taken at (`UNSET` if unrecorded).
    pub git_sha: String,
    /// `debug` | `release`.
    pub build_profile: String,
    /// `hermetic` (A1) | `C2-LOCAL` (A2) | ...
    pub venue: String,
    /// Bootstrap / recovered anchor identity (hermetic: a fixed label).
    pub anchor: String,
    /// Authoritative state fingerprint before the workload.
    pub tip_before: String,
    /// Authoritative state fingerprint after the workload.
    pub tip_after: String,
    /// WAL / checkpoint fingerprint (hermetic: the authoritative-output fp; A2
    /// replaces it with the real persisted-state fingerprint).
    pub wal_checkpoint_fp: String,
    /// blake2b-256 of the canonical ordered workload input.
    pub workload_hash: String,
    /// Observational (RED): p50 RSS in kiB, or `None` if unsampled.
    pub rss_p50_kib: Option<u64>,
    /// Observational (RED): p95 RSS in kiB.
    pub rss_p95_kib: Option<u64>,
    /// Observational (RED): peak RSS in kiB.
    pub rss_peak_kib: Option<u64>,
    /// Number of RSS samples taken.
    pub rss_sample_count: usize,
    /// blake2b-256 of the canonical authoritative output.
    pub final_fingerprint: String,
    /// Verdict from re-running the same workload (fingerprint pairing).
    pub replay_verdict: ReplayVerdict,
}

/// A structural / discipline defect in an evidence record. Empty list = valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDefect {
    /// `scenario_id` is empty.
    EmptyScenarioId,
    /// `workload_hash` is empty.
    EmptyWorkloadHash,
    /// `final_fingerprint` is empty.
    EmptyFinalFingerprint,
    /// The replay verdict is not `Agreed` — the measurement is invalid evidence.
    VerdictNotAgreed,
    /// `p50 > p95` or `p95 > peak` — the observational percentiles are mis-shaped.
    PercentileShapeViolated,
}

/// blake2b-256 lowercase-hex of arbitrary canonical bytes. The single
/// fingerprint authority for MEM-MEASURE evidence.
pub fn fingerprint_hex(canonical_bytes: &[u8]) -> String {
    hex_lower(&ade_crypto::blake2b::blake2b_256(canonical_bytes).0)
}

/// Pair two authoritative fingerprints from runs of the SAME workload. `Agreed`
/// iff byte-identical. RSS pressure must not change the fingerprint; if it does,
/// the verdict is `Diverged` and the evidence is invalid.
pub fn pair_replay(first_fp: &str, second_fp: &str) -> ReplayVerdict {
    if first_fp == second_fp {
        ReplayVerdict::Agreed
    } else {
        ReplayVerdict::Diverged
    }
}

/// Validate a record's STRUCTURE and its replay discipline. The result does NOT
/// depend on the RSS magnitudes — only on their internal SHAPE
/// (`p50 <= p95 <= peak`) and on the verdict being `Agreed`. Memory numbers are
/// evidence, never a gate.
pub fn validate_evidence(rec: &MemEvidenceRecord) -> Vec<EvidenceDefect> {
    let mut defects = Vec::new();
    if rec.scenario_id.is_empty() {
        defects.push(EvidenceDefect::EmptyScenarioId);
    }
    if rec.workload_hash.is_empty() {
        defects.push(EvidenceDefect::EmptyWorkloadHash);
    }
    if rec.final_fingerprint.is_empty() {
        defects.push(EvidenceDefect::EmptyFinalFingerprint);
    }
    if rec.replay_verdict != ReplayVerdict::Agreed {
        defects.push(EvidenceDefect::VerdictNotAgreed);
    }
    // Percentile SHAPE only (ordering), never magnitude.
    if let (Some(p50), Some(p95)) = (rec.rss_p50_kib, rec.rss_p95_kib) {
        if p50 > p95 {
            defects.push(EvidenceDefect::PercentileShapeViolated);
        }
    }
    if let (Some(p95), Some(peak)) = (rec.rss_p95_kib, rec.rss_peak_kib) {
        if p95 > peak {
            defects.push(EvidenceDefect::PercentileShapeViolated);
        }
    }
    defects
}

/// Lowercase hex. Self-contained (no external hex dependency).
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn agreed_record() -> MemEvidenceRecord {
        MemEvidenceRecord {
            scenario_id: "mem-measure-a1-test".into(),
            git_sha: "UNSET".into(),
            build_profile: "debug".into(),
            venue: "hermetic".into(),
            anchor: "hermetic:bounded-ingress".into(),
            tip_before: "hermetic:pre".into(),
            tip_after: "abcd".into(),
            wal_checkpoint_fp: "abcd".into(),
            workload_hash: "1234".into(),
            rss_p50_kib: Some(100),
            rss_p95_kib: Some(200),
            rss_peak_kib: Some(300),
            rss_sample_count: 3,
            final_fingerprint: "abcd".into(),
            replay_verdict: ReplayVerdict::Agreed,
        }
    }

    #[test]
    fn well_formed_agreed_record_validates() {
        assert!(validate_evidence(&agreed_record()).is_empty());
    }

    #[test]
    fn diverged_verdict_is_invalid_evidence() {
        let mut r = agreed_record();
        r.replay_verdict = ReplayVerdict::Diverged;
        assert!(validate_evidence(&r).contains(&EvidenceDefect::VerdictNotAgreed));
    }

    #[test]
    fn validator_ignores_rss_magnitude() {
        let mut low = agreed_record();
        low.rss_p50_kib = Some(1);
        low.rss_p95_kib = Some(2);
        low.rss_peak_kib = Some(3);

        let mut high = agreed_record();
        high.rss_p50_kib = Some(9_000_000);
        high.rss_p95_kib = Some(9_500_000);
        high.rss_peak_kib = Some(9_999_999);

        // Wildly different magnitudes, identical (empty) validation result.
        assert_eq!(validate_evidence(&low), validate_evidence(&high));
        assert!(validate_evidence(&low).is_empty());
    }

    #[test]
    fn percentile_shape_violation_flagged() {
        let mut r = agreed_record();
        r.rss_p50_kib = Some(500); // p50 > p95
        r.rss_p95_kib = Some(200);
        assert!(validate_evidence(&r).contains(&EvidenceDefect::PercentileShapeViolated));
    }

    #[test]
    fn pair_replay_agreed_on_identical_fp() {
        assert_eq!(pair_replay("aa", "aa"), ReplayVerdict::Agreed);
    }

    #[test]
    fn pair_replay_diverged_on_different_fp() {
        assert_eq!(pair_replay("aa", "bb"), ReplayVerdict::Diverged);
    }

    #[test]
    fn fingerprint_hex_is_deterministic() {
        let a = fingerprint_hex(b"hello");
        assert_eq!(a, fingerprint_hex(b"hello"));
        assert_eq!(a.len(), 64, "blake2b-256 = 32 bytes = 64 hex chars");
        assert_ne!(fingerprint_hex(b"hello"), fingerprint_hex(b"world"));
    }

    #[test]
    fn record_roundtrips_jsonl() {
        let r = agreed_record();
        let line = serde_json::to_string(&r).unwrap();
        let back: MemEvidenceRecord = serde_json::from_str(&line).unwrap();
        assert_eq!(r, back);
    }
}
