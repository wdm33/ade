// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN JSONL writer for `AdmissionLogEvent` (PHASE4-N-M-B S2).
//!
//! Hand-rolled JSON serializer over the closed event enum. One
//! JSON object per line, flushed after every emit so a SIGINT
//! mid-emit cannot produce a partial line. No serde-derive dep.
//! Mirrors `crate::live_log::writer::LiveLogWriter` shape; the two
//! writers stay physically isolated (different files, different
//! vocabularies).

use std::io::{self, Write};

use super::event::AdmissionLogEvent;

/// JSONL sink for `AdmissionLogEvent`. Wraps any `Write` impl
/// (a `File`, a `Vec<u8>` for tests, `io::stdout()`, etc.).
pub struct AdmissionLogWriter<W: Write> {
    sink: W,
}

impl<W: Write> AdmissionLogWriter<W> {
    pub fn new(sink: W) -> Self {
        Self { sink }
    }

    /// Serialize one event to a single line + flush. Atomic on
    /// the file system call boundary; a SIGINT after the write
    /// returns sees a complete line.
    pub fn emit(&mut self, event: &AdmissionLogEvent) -> io::Result<()> {
        let mut buf = String::new();
        encode_event(event, &mut buf);
        buf.push('\n');
        self.sink.write_all(buf.as_bytes())?;
        self.sink.flush()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }

    pub fn into_inner(self) -> W {
        self.sink
    }
}

fn encode_event(event: &AdmissionLogEvent, out: &mut String) {
    out.push('{');
    push_key_str(out, "event", event.discriminator());
    match event {
        AdmissionLogEvent::AdmissionStarted {
            peer_count,
            json_seed_path,
            wal_dir,
            consensus_inputs_fingerprint_hex,
        } => {
            out.push(',');
            push_key_u64(out, "peer_count", *peer_count as u64);
            out.push(',');
            push_key_str(out, "json_seed_path", json_seed_path);
            out.push(',');
            push_key_str(out, "wal_dir", wal_dir);
            out.push(',');
            push_key_str(
                out,
                "consensus_inputs_fingerprint_hex",
                consensus_inputs_fingerprint_hex,
            );
        }
        AdmissionLogEvent::SnapshotImported {
            seed_point_slot,
            imported_utxo_fp_hex,
            utxo_entry_count,
        } => {
            out.push(',');
            push_key_u64(out, "seed_point_slot", *seed_point_slot);
            out.push(',');
            push_key_str(out, "imported_utxo_fp_hex", imported_utxo_fp_hex);
            out.push(',');
            push_key_u64(out, "utxo_entry_count", *utxo_entry_count);
        }
        AdmissionLogEvent::BootstrapComplete {
            initial_ledger_fp_hex,
            chain_tip_slot,
            consensus_inputs_fingerprint_hex,
        } => {
            out.push(',');
            push_key_str(out, "initial_ledger_fp_hex", initial_ledger_fp_hex);
            out.push(',');
            push_key_u64(out, "chain_tip_slot", *chain_tip_slot);
            out.push(',');
            push_key_str(
                out,
                "consensus_inputs_fingerprint_hex",
                consensus_inputs_fingerprint_hex,
            );
        }
        AdmissionLogEvent::BlockReceived {
            peer,
            slot,
            block_hash_hex,
        } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "slot", *slot);
            out.push(',');
            push_key_str(out, "block_hash_hex", block_hash_hex);
        }
        AdmissionLogEvent::BlockAdmitted {
            slot,
            block_hash_hex,
            prev_hash_hex,
            post_fp_hex,
            consensus_inputs_fingerprint_hex,
        } => {
            out.push(',');
            push_key_u64(out, "slot", *slot);
            out.push(',');
            push_key_str(out, "block_hash_hex", block_hash_hex);
            out.push(',');
            push_key_str(out, "prev_hash_hex", prev_hash_hex);
            out.push(',');
            push_key_str(out, "post_fp_hex", post_fp_hex);
            out.push(',');
            push_key_str(
                out,
                "consensus_inputs_fingerprint_hex",
                consensus_inputs_fingerprint_hex,
            );
        }
        AdmissionLogEvent::AgreementVerdict {
            kind,
            slot,
            our_hash_hex,
            peer_hash_hex,
            peer_slot,
            tx_in_hex,
            consensus_inputs_fingerprint_hex,
        } => {
            out.push(',');
            push_key_str(out, "kind", kind);
            out.push(',');
            push_key_u64(out, "slot", *slot);
            if let Some(h) = our_hash_hex {
                out.push(',');
                push_key_str(out, "our_hash_hex", h);
            }
            if let Some(h) = peer_hash_hex {
                out.push(',');
                push_key_str(out, "peer_hash_hex", h);
            }
            if let Some(s) = peer_slot {
                out.push(',');
                push_key_u64(out, "peer_slot", *s);
            }
            if let Some(t) = tx_in_hex {
                out.push(',');
                push_key_str(out, "tx_in_hex", t);
            }
            out.push(',');
            push_key_str(
                out,
                "consensus_inputs_fingerprint_hex",
                consensus_inputs_fingerprint_hex,
            );
        }
        AdmissionLogEvent::AdmissionHalted { reason } => {
            out.push(',');
            push_key_str(out, "reason", reason.as_str());
        }
        AdmissionLogEvent::AdmissionShutdown { reason } => {
            out.push(',');
            push_key_str(out, "reason", reason.as_str());
        }
        // PHASE4-N-AO S9 (DC-EVIDENCE-04) closed fork-choice events.
        AdmissionLogEvent::NeedsForkChoice {
            peer,
            slot,
            block_hash_hex,
        } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "slot", *slot);
            out.push(',');
            push_key_str(out, "block_hash_hex", block_hash_hex);
        }
        AdmissionLogEvent::LcaDiscovered {
            peer,
            fork_anchor_slot,
            fork_anchor_hash_hex,
            candidate_header_count,
        } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "fork_anchor_slot", *fork_anchor_slot);
            out.push(',');
            push_key_str(out, "fork_anchor_hash_hex", fork_anchor_hash_hex);
            out.push(',');
            push_key_u64(out, "candidate_header_count", *candidate_header_count);
        }
        AdmissionLogEvent::CandidateFragmentBuilt {
            peer,
            anchor_slot,
            candidate_header_count,
        } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "anchor_slot", *anchor_slot);
            out.push(',');
            push_key_u64(out, "candidate_header_count", *candidate_header_count);
        }
        AdmissionLogEvent::ForkChoiceSelected {
            fork_switch_id,
            peer,
            result,
            winner_tip_slot,
            winner_tip_hash_hex,
            consensus_inputs_fingerprint_hex,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_str(out, "result", result.as_str());
            if let Some(s) = winner_tip_slot {
                out.push(',');
                push_key_u64(out, "winner_tip_slot", *s);
            }
            if let Some(h) = winner_tip_hash_hex {
                out.push(',');
                push_key_str(out, "winner_tip_hash_hex", h);
            }
            out.push(',');
            push_key_str(
                out,
                "consensus_inputs_fingerprint_hex",
                consensus_inputs_fingerprint_hex,
            );
        }
        AdmissionLogEvent::BranchFetchStarted {
            fork_switch_id,
            peer,
            fork_anchor_slot,
            winner_tip_slot,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "fork_anchor_slot", *fork_anchor_slot);
            out.push(',');
            push_key_u64(out, "winner_tip_slot", *winner_tip_slot);
        }
        AdmissionLogEvent::BranchFetchCompleted {
            fork_switch_id,
            peer,
            block_count,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "block_count", *block_count);
        }
        AdmissionLogEvent::BranchPrevalidated {
            fork_switch_id,
            peer,
            block_count,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "block_count", *block_count);
        }
        AdmissionLogEvent::ForkSwitchApplied {
            fork_switch_id,
            peer,
            new_tip_slot,
            new_tip_hash_hex,
            rollback_reason,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "new_tip_slot", *new_tip_slot);
            out.push(',');
            push_key_str(out, "new_tip_hash_hex", new_tip_hash_hex);
            out.push(',');
            push_key_str(out, "rollback_reason", rollback_reason);
        }
        AdmissionLogEvent::ForkSwitchFailed {
            fork_switch_id,
            peer,
            failure_code,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_str(out, "failure_code", failure_code.as_str());
        }
        AdmissionLogEvent::ForkSwitchSuperseded { fork_switch_id, peer } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
        }
        // PHASE4-N-AO S11 (DC-NODE-39) closed missing-bridge fail-closed event.
        AdmissionLogEvent::MissingBridge {
            peer,
            block_hash_hex,
            reason,
        } => {
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_str(out, "block_hash_hex", block_hash_hex);
            out.push(',');
            push_key_str(out, "reason", reason);
        }
        // PHASE4-N-AO S14 (DC-NODE-41) closed range re-fetch recovery events.
        AdmissionLogEvent::RangeRefetchStarted {
            fork_switch_id,
            peer,
            from_slot,
            to_slot,
            reason,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_u64(out, "from_slot", *from_slot);
            out.push(',');
            push_key_u64(out, "to_slot", *to_slot);
            out.push(',');
            push_key_str(out, "reason", reason);
        }
        AdmissionLogEvent::RangeRefetchCompleted {
            fork_switch_id,
            peer,
            outcome,
        } => {
            out.push(',');
            push_key_str(out, "fork_switch_id", fork_switch_id);
            out.push(',');
            push_key_str(out, "peer", peer);
            out.push(',');
            push_key_str(out, "outcome", outcome);
        }
        // MEM-MEASURE-A2 (OP-MEM-01) closed live memory-evidence events.
        AdmissionLogEvent::MemoryMeasure {
            point,
            slot,
            durable_tip_slot,
            durable_tip_fp_hex,
            rss_kib,
            rss_hwm_kib,
        } => {
            out.push(',');
            push_key_str(out, "point", point);
            out.push(',');
            push_key_u64(out, "slot", *slot);
            out.push(',');
            push_key_u64(out, "durable_tip_slot", *durable_tip_slot);
            out.push(',');
            push_key_str(out, "durable_tip_fp_hex", durable_tip_fp_hex);
            out.push(',');
            push_key_u64(out, "rss_kib", *rss_kib);
            out.push(',');
            push_key_u64(out, "rss_hwm_kib", *rss_hwm_kib);
        }
        AdmissionLogEvent::MemorySummary {
            sample_count,
            rss_p50_kib,
            rss_p95_kib,
            rss_peak_kib,
            rss_hwm_kib,
            replay_verdict,
        } => {
            out.push(',');
            push_key_u64(out, "sample_count", *sample_count);
            out.push(',');
            push_key_u64(out, "rss_p50_kib", *rss_p50_kib);
            out.push(',');
            push_key_u64(out, "rss_p95_kib", *rss_p95_kib);
            out.push(',');
            push_key_u64(out, "rss_peak_kib", *rss_peak_kib);
            out.push(',');
            push_key_u64(out, "rss_hwm_kib", *rss_hwm_kib);
            out.push(',');
            push_key_str(out, "replay_verdict", replay_verdict);
        }
    }
    out.push('}');
}

fn push_key_str(out: &mut String, key: &str, val: &str) {
    out.push('"');
    push_json_str_body(out, key);
    out.push_str("\":\"");
    push_json_str_body(out, val);
    out.push('"');
}

fn push_key_u64(out: &mut String, key: &str, val: u64) {
    out.push('"');
    push_json_str_body(out, key);
    out.push_str("\":");
    out.push_str(&val.to_string());
}

fn push_json_str_body(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

// Re-export discriminator strings as constants for the suppressor
// gate's positive grep.
#[allow(dead_code)]
const DISCRIMINATORS: &[&str] = &[
    "admission_started",
    "snapshot_imported",
    "bootstrap_complete",
    "block_received",
    "block_admitted",
    "agreement_verdict",
    "admission_halted",
    "admission_shutdown",
    // PHASE4-N-AO S9 (DC-EVIDENCE-04) closed fork-choice events.
    "needs_fork_choice",
    "lca_discovered",
    "candidate_fragment_built",
    "fork_choice_selected",
    "branch_fetch_started",
    "branch_fetch_completed",
    "branch_prevalidated",
    "fork_switch_applied",
    "fork_switch_failed",
    "fork_switch_superseded",
    // PHASE4-N-AO S11 (DC-NODE-39) closed missing-bridge fail-closed event.
    "missing_bridge",
    // PHASE4-N-AO S14 (DC-NODE-41) closed range re-fetch recovery events.
    "range_refetch_started",
    "range_refetch_completed",
    // MEM-MEASURE-A2 (OP-MEM-01) closed live memory-evidence events.
    "memory_measure",
    "memory_summary",
];

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use super::super::event::{AdmissionHaltReason, AdmissionShutdownReason};
    use std::io::BufRead;

    fn emit_to_vec(events: &[AdmissionLogEvent]) -> Vec<u8> {
        let buf: Vec<u8> = Vec::new();
        let mut w = AdmissionLogWriter::new(buf);
        for e in events {
            w.emit(e).expect("emit");
        }
        w.into_inner()
    }

    #[test]
    fn admission_log_writer_emits_memory_events() {
        let fp = "ab".repeat(32);
        let events = vec![
            AdmissionLogEvent::MemoryMeasure {
                point: "chain_sync_follow",
                slot: 120,
                durable_tip_slot: 120,
                durable_tip_fp_hex: fp.clone(),
                rss_kib: 4096,
                rss_hwm_kib: 6900,
            },
            AdmissionLogEvent::MemorySummary {
                sample_count: 7,
                rss_p50_kib: 4000,
                rss_p95_kib: 4200,
                rss_peak_kib: 4500,
                rss_hwm_kib: 6800,
                replay_verdict: "agreed",
            },
        ];
        let text = String::from_utf8(emit_to_vec(&events)).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(r#""event":"memory_measure""#));
        assert!(lines[0].contains(r#""point":"chain_sync_follow""#));
        assert!(lines[0].contains(r#""slot":120"#));
        assert!(lines[0].contains(r#""durable_tip_slot":120"#));
        assert!(lines[0].contains(&format!(r#""durable_tip_fp_hex":"{fp}""#)));
        assert!(lines[0].contains(r#""rss_kib":4096"#));
        assert!(lines[0].contains(r#""rss_hwm_kib":6900"#));
        assert!(lines[1].contains(r#""event":"memory_summary""#));
        assert!(lines[1].contains(r#""sample_count":7"#));
        assert!(lines[1].contains(r#""rss_p50_kib":4000"#));
        assert!(lines[1].contains(r#""rss_p95_kib":4200"#));
        assert!(lines[1].contains(r#""rss_peak_kib":4500"#));
        assert!(lines[1].contains(r#""rss_hwm_kib":6800"#));
        assert!(lines[1].contains(r#""replay_verdict":"agreed""#));
    }

    #[test]
    fn admission_log_writer_emits_one_object_per_line() {
        let fp = "ff".repeat(32);
        let events = vec![
            AdmissionLogEvent::AdmissionStarted {
                peer_count: 2,
                json_seed_path: "/tmp/seed.json".into(),
                wal_dir: "/tmp/wal".into(),
                consensus_inputs_fingerprint_hex: fp.clone(),
            },
            AdmissionLogEvent::SnapshotImported {
                seed_point_slot: 12345,
                imported_utxo_fp_hex: "deadbeef".repeat(8),
                utxo_entry_count: 128,
            },
            AdmissionLogEvent::BootstrapComplete {
                initial_ledger_fp_hex: "aa".repeat(32),
                chain_tip_slot: 12345,
                consensus_inputs_fingerprint_hex: fp.clone(),
            },
            AdmissionLogEvent::BlockReceived {
                peer: "127.0.0.1:3001".into(),
                slot: 12346,
                block_hash_hex: "bb".repeat(32),
            },
            AdmissionLogEvent::BlockAdmitted {
                slot: 12346,
                block_hash_hex: "bb".repeat(32),
                prev_hash_hex: "aa".repeat(32),
                post_fp_hex: "cc".repeat(32),
                consensus_inputs_fingerprint_hex: fp.clone(),
            },
            AdmissionLogEvent::AgreementVerdict {
                kind: "agreed",
                slot: 12346,
                our_hash_hex: Some("bb".repeat(32)),
                peer_hash_hex: Some("bb".repeat(32)),
                peer_slot: None,
                tx_in_hex: None,
                consensus_inputs_fingerprint_hex: fp.clone(),
            },
            AdmissionLogEvent::AdmissionShutdown {
                reason: AdmissionShutdownReason::SignalReceived,
            },
        ];
        let bytes = emit_to_vec(&events);
        let lines: Vec<&[u8]> = bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(lines.len(), 7);
        for (i, line) in lines.iter().enumerate() {
            let s = std::str::from_utf8(line).expect("utf8");
            assert!(s.starts_with('{'), "line {i} must start with {{");
            assert!(s.ends_with('}'), "line {i} must end with }}");
        }
    }

    #[test]
    fn admission_log_writer_serializes_admission_started_canonically() {
        let bytes = emit_to_vec(&[AdmissionLogEvent::AdmissionStarted {
            peer_count: 1,
            json_seed_path: "/seed.json".into(),
            wal_dir: "/wal".into(),
            consensus_inputs_fingerprint_hex: "00".repeat(32),
        }]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        assert_eq!(
            s,
            "{\"event\":\"admission_started\",\"peer_count\":1,\"json_seed_path\":\"/seed.json\",\"wal_dir\":\"/wal\",\"consensus_inputs_fingerprint_hex\":\"0000000000000000000000000000000000000000000000000000000000000000\"}\n"
        );
    }

    #[test]
    fn admission_log_writer_two_runs_are_byte_identical() {
        let events = vec![
            AdmissionLogEvent::AdmissionStarted {
                peer_count: 3,
                json_seed_path: "p".into(),
                wal_dir: "w".into(),
                consensus_inputs_fingerprint_hex: "ab".repeat(32),
            },
            AdmissionLogEvent::AdmissionHalted {
                reason: AdmissionHaltReason::Diverged,
            },
        ];
        let a = emit_to_vec(&events);
        let b = emit_to_vec(&events);
        assert_eq!(a, b);
    }

    #[test]
    fn admission_log_writer_emits_agreement_verdict_with_kind_field() {
        let bytes = emit_to_vec(&[AdmissionLogEvent::AgreementVerdict {
            kind: "diverged",
            slot: 100,
            our_hash_hex: Some("a1".repeat(32)),
            peer_hash_hex: Some("b2".repeat(32)),
            peer_slot: None,
            tx_in_hex: None,
            consensus_inputs_fingerprint_hex: "cd".repeat(32),
        }]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        assert!(s.contains("\"event\":\"agreement_verdict\""), "got: {s}");
        assert!(s.contains("\"kind\":\"diverged\""), "got: {s}");
        assert!(s.contains("\"slot\":100"), "got: {s}");
    }

    #[test]
    fn admission_log_writer_omits_optional_fields_when_none() {
        let bytes = emit_to_vec(&[AdmissionLogEvent::AgreementVerdict {
            kind: "input_not_found",
            slot: 0,
            our_hash_hex: None,
            peer_hash_hex: None,
            peer_slot: None,
            tx_in_hex: Some("deadbeef#0".into()),
            consensus_inputs_fingerprint_hex: "ef".repeat(32),
        }]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        assert!(!s.contains("our_hash_hex"), "got: {s}");
        assert!(!s.contains("peer_hash_hex"), "got: {s}");
        assert!(!s.contains("peer_slot"), "got: {s}");
        assert!(s.contains("\"tx_in_hex\":\"deadbeef#0\""), "got: {s}");
    }

    #[test]
    fn admission_log_writer_emits_range_refetch_events_with_closed_fields() {
        // PHASE4-N-AO S14 (DC-NODE-41): the started carries the range + the closed
        // trigger reason; the completed carries the closed outcome discriminator.
        let bytes = emit_to_vec(&[
            AdmissionLogEvent::RangeRefetchStarted {
                fork_switch_id: "abcd1234".into(),
                peer: "127.0.0.1:6002".into(),
                from_slot: 298,
                to_slot: 388,
                reason: "branch_gap",
            },
            AdmissionLogEvent::RangeRefetchCompleted {
                fork_switch_id: "abcd1234".into(),
                peer: "127.0.0.1:6002".into(),
                outcome: "admitted",
            },
        ]);
        let s = std::str::from_utf8(&bytes).expect("utf8");
        let lines: Vec<&str> = s.lines().collect();
        assert!(lines[0].contains("\"event\":\"range_refetch_started\""), "got: {}", lines[0]);
        assert!(lines[0].contains("\"from_slot\":298"), "got: {}", lines[0]);
        assert!(lines[0].contains("\"to_slot\":388"), "got: {}", lines[0]);
        assert!(lines[0].contains("\"reason\":\"branch_gap\""), "got: {}", lines[0]);
        assert!(lines[1].contains("\"event\":\"range_refetch_completed\""), "got: {}", lines[1]);
        assert!(lines[1].contains("\"outcome\":\"admitted\""), "got: {}", lines[1]);
    }

    #[test]
    fn admission_log_writer_lines_are_parseable_as_one_json_object_per_line() {
        let events = vec![
            AdmissionLogEvent::AdmissionStarted {
                peer_count: 1,
                json_seed_path: "p".into(),
                wal_dir: "w".into(),
                consensus_inputs_fingerprint_hex: "12".repeat(32),
            },
            AdmissionLogEvent::AdmissionShutdown {
                reason: AdmissionShutdownReason::UpstreamDropped,
            },
        ];
        let bytes = emit_to_vec(&events);
        let cursor = std::io::Cursor::new(bytes);
        for line in cursor.lines() {
            let s = line.expect("line");
            assert!(s.starts_with('{'));
            assert!(s.ends_with('}'));
            assert!(s.contains("\"event\":\""), "missing event: {s}");
            let mut depth = 0i32;
            for c in s.chars() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
            }
            assert_eq!(depth, 0, "unbalanced braces: {s}");
        }
    }
}
