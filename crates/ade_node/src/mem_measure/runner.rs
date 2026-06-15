//! GREEN/RED seam: the hermetic MEM-MEASURE-A1 measurement runner.
//!
//! Drives the GREEN bounded-ingress workload under RED RSS sampling and emits a
//! paired [`MemEvidenceRecord`]. The authoritative output (the bounded fold
//! result) is fingerprinted; the workload is replayed once; the verdict pairs
//! the two fingerprints. RSS is sampled around the fold and recorded into the
//! record's OBSERVATIONAL fields only — the verdict NEVER depends on RSS. This
//! is the hermetic template A2 mirrors against a live `--mode node` run.

use ade_ledger::mempool::{AdmitOutcome, IngressEvent, IngressSource, MempoolState};
use ade_ledger::state::LedgerState;

use super::bounded_admission::{replay_bounded_ingress_trace, BoundedOutcome, ShedReason};
use super::evidence::{fingerprint_hex, pair_replay, MemEvidenceRecord};
use super::rss_sampler::RssWindow;

/// The compile-time build profile, for the evidence record.
pub fn current_build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// Canonical bytes of the ordered inbound trace (the workload input): for each
/// event, the source discriminant, then the big-endian tx-byte length, then the
/// tx bytes verbatim.
fn workload_bytes(events: &[IngressEvent]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for e in events {
        bytes.push(match e.source() {
            IngressSource::N2N => 0u8,
            IngressSource::N2C => 1u8,
        });
        bytes.extend_from_slice(&(e.tx_bytes().len() as u64).to_be_bytes());
        bytes.extend_from_slice(e.tx_bytes());
    }
    bytes
}

/// Canonical digest of the authoritative output: the admitted tx ids in
/// admission order, a separator, then a per-event disposition byte. Independent
/// of any volatile reject-class layout — `accepted()` already records WHICH txs
/// admitted; the disposition byte only distinguishes forward-admit /
/// forward-reject / shed-count / shed-byte.
fn authoritative_digest(mempool: &MempoolState, outcomes: &[BoundedOutcome]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for id in mempool.accepted() {
        bytes.extend_from_slice(&id.0);
    }
    bytes.push(0xFF);
    for o in outcomes {
        let tag = match o {
            BoundedOutcome::Forwarded(AdmitOutcome::Admitted { .. }) => 0u8,
            BoundedOutcome::Forwarded(AdmitOutcome::Rejected { .. }) => 1u8,
            BoundedOutcome::Shed(ShedReason::CountBudgetExhausted) => 2u8,
            BoundedOutcome::Shed(ShedReason::ByteBudgetExhausted) => 3u8,
        };
        bytes.push(tag);
    }
    bytes
}

/// Run the hermetic bounded-ingress workload under RSS observation and produce
/// a paired evidence record (`venue = "hermetic"`). The workload is folded
/// twice; the two authoritative fingerprints are paired into the verdict. RSS
/// is sampled around the folds and stored in the observational fields only.
pub fn run_hermetic_bounded_ingress_measurement(
    scenario_id: &str,
    git_sha: &str,
    base: LedgerState,
    events: &[IngressEvent],
) -> MemEvidenceRecord {
    let workload_hash = fingerprint_hex(&workload_bytes(events));

    let mut rss = RssWindow::new();
    rss.observe_now();

    // First run — the measured authoritative output.
    let (mempool1, outcomes1) = replay_bounded_ingress_trace(base.clone(), events);
    rss.observe_now();
    let final_fp_1 = fingerprint_hex(&authoritative_digest(&mempool1, &outcomes1));

    // Replay — must reproduce the same authoritative output under (possibly
    // different) memory conditions.
    let (mempool2, outcomes2) = replay_bounded_ingress_trace(base, events);
    rss.observe_now();
    let final_fp_2 = fingerprint_hex(&authoritative_digest(&mempool2, &outcomes2));

    let replay_verdict = pair_replay(&final_fp_1, &final_fp_2);

    MemEvidenceRecord {
        scenario_id: scenario_id.to_string(),
        git_sha: git_sha.to_string(),
        build_profile: current_build_profile().to_string(),
        venue: "hermetic".to_string(),
        anchor: "hermetic:bounded-ingress".to_string(),
        tip_before: "hermetic:pre".to_string(),
        tip_after: final_fp_1.clone(),
        wal_checkpoint_fp: final_fp_1.clone(),
        workload_hash,
        rss_p50_kib: rss.p50_kib(),
        rss_p95_kib: rss.p95_kib(),
        rss_peak_kib: rss.peak_kib(),
        rss_sample_count: rss.count(),
        final_fingerprint: final_fp_1,
        replay_verdict,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::mem_measure::evidence::{validate_evidence, ReplayVerdict};
    use ade_ledger::mempool::{IngressEvent, IngressSource};
    use ade_ledger::state::LedgerState;
    use ade_testkit::mempool::{b_track_corpus_as_ingress, ExpectedOutcome};

    fn corpus_events() -> (LedgerState, Vec<IngressEvent>) {
        // The valid B-track case's base, fed a junk + valid + junk trace: a real
        // authoritative output to fingerprint + replay. The bound is irrelevant
        // here (three events, well under budget).
        let cases = b_track_corpus_as_ingress(IngressSource::N2N);
        let valid = cases
            .into_iter()
            .find(|c| matches!(c.expected, ExpectedOutcome::Admit))
            .expect("corpus has a valid case");
        let events = vec![
            IngressEvent::new(IngressSource::N2N, vec![0x80]),
            valid.event.clone(),
            IngressEvent::new(IngressSource::N2N, vec![0x81]),
        ];
        (valid.base, events)
    }

    #[test]
    fn hermetic_measurement_verdict_is_agreed() {
        let (base, events) = corpus_events();
        let rec = run_hermetic_bounded_ingress_measurement("a1-test", "UNSET", base, &events);
        assert_eq!(rec.replay_verdict, ReplayVerdict::Agreed);
        assert!(
            validate_evidence(&rec).is_empty(),
            "the hermetic record must be valid evidence"
        );
        assert_eq!(rec.venue, "hermetic");
        assert_eq!(rec.final_fingerprint.len(), 64);
    }

    #[test]
    fn hermetic_measurement_is_replay_stable() {
        let (base, events) = corpus_events();
        let r1 =
            run_hermetic_bounded_ingress_measurement("a1-test", "UNSET", base.clone(), &events);
        let r2 = run_hermetic_bounded_ingress_measurement("a1-test", "UNSET", base, &events);
        // Authoritative fields identical regardless of RSS variation between runs.
        assert_eq!(r1.final_fingerprint, r2.final_fingerprint);
        assert_eq!(r1.workload_hash, r2.workload_hash);
        assert_eq!(r1.tip_after, r2.tip_after);
    }

    #[test]
    fn hermetic_measurement_records_rss_on_linux() {
        let (base, events) = corpus_events();
        let rec = run_hermetic_bounded_ingress_measurement("a1-test", "UNSET", base, &events);
        if cfg!(target_os = "linux") {
            assert!(rec.rss_sample_count > 0);
            assert!(rec.rss_peak_kib.is_some());
        }
    }
}
