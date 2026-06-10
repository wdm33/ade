// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Dedicated convergence-evidence sink (PHASE4-N-AJ AJ-S1).
//!
//! The convergence-evidence transcript (CE-AI-6) is a NARROW evidence file,
//! not a lifecycle log. This sink is the only way to write it: it exposes
//! exactly three emit methods — one per allowed event — and NO accessor to
//! the raw inner writer, so the file cannot become a dumping ground for
//! sched / forge / admission-lifecycle events.
//!
//! Closed convergence vocabulary = the 3-variant subset of the reused
//! [`AdmissionLogEvent`] (no new evidence enum; ¬AJ-3):
//!   - `block_received`    (each peer block considered, before drop/admit/refuse),
//!   - `block_admitted`    (per `pump_block` admit),
//!   - `agreement_verdict` (`verdict::derive` result).
//! All three are in `ci/ci_check_convergence_evidence_schema.sh`'s ALLOWED set.
//! The compiler closure (this sink, which has no method for any other variant)
//! is one half of the property; the file-tree half is
//! `ci/ci_check_convergence_evidence_vocabulary_closed.sh` (DC-ADMIT-04 /
//! DC-NODE-30).
//!
//! TCB: the *event selection* (which [`AdmissionLogEvent`] variant each method
//! builds) is GREEN and generic over `W: Write`; the file-backed instantiation
//! (`File::create` + the byte writes) is RED. With no path supplied the sink is
//! disabled — no file is opened or written, consensus behavior and existing
//! logs are unchanged (I-AJ-6 / AJ-S1 decision D-1). AJ-S1 is INERT: this sink
//! is built and unit-tested but not yet fed by `run_participant_sync` (AJ-S2).

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::admission_log::{AdmissionLogEvent, AdmissionLogWriter};

/// Opt-in, closed-vocabulary convergence-evidence sink. Wraps an optional
/// [`AdmissionLogWriter`]; `None` => disabled (every emit is a no-op).
pub struct ConvergenceEvidenceSink<W: Write> {
    inner: Option<AdmissionLogWriter<W>>,
}

impl ConvergenceEvidenceSink<File> {
    /// Open the file-backed sink (RED). `None` => disabled: no file is created
    /// and every emit is a no-op. `Some(p)` => `File::create(p)` + a writer.
    pub fn open(path: Option<&Path>) -> io::Result<Self> {
        let inner = match path {
            None => None,
            Some(p) => Some(AdmissionLogWriter::new(File::create(p)?)),
        };
        Ok(Self { inner })
    }
}

impl<W: Write> ConvergenceEvidenceSink<W> {
    /// Construct over an arbitrary sink (GREEN seam: tests use a shared buffer;
    /// AJ-S2 uses the file-backed [`ConvergenceEvidenceSink::open`]). Always
    /// enabled.
    pub fn with_writer(writer: AdmissionLogWriter<W>) -> Self {
        Self { inner: Some(writer) }
    }

    /// A disabled sink (no inner writer); every emit is a no-op.
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    /// Whether a sink is backing this writer (a path was supplied).
    pub fn is_enabled(&self) -> bool {
        self.inner.is_some()
    }

    /// Emit a `block_received` evidence line (each peer block considered).
    pub fn emit_block_received(
        &mut self,
        peer: &str,
        slot: u64,
        block_hash_hex: &str,
    ) -> io::Result<()> {
        self.emit(AdmissionLogEvent::BlockReceived {
            peer: peer.to_string(),
            slot,
            block_hash_hex: block_hash_hex.to_string(),
        })
    }

    /// Emit a `block_admitted` evidence line (per `pump_block` admit).
    pub fn emit_block_admitted(
        &mut self,
        slot: u64,
        block_hash_hex: &str,
        post_fp_hex: &str,
        consensus_inputs_fingerprint_hex: &str,
    ) -> io::Result<()> {
        self.emit(AdmissionLogEvent::BlockAdmitted {
            slot,
            block_hash_hex: block_hash_hex.to_string(),
            post_fp_hex: post_fp_hex.to_string(),
            consensus_inputs_fingerprint_hex: consensus_inputs_fingerprint_hex.to_string(),
        })
    }

    /// Emit an `agreement_verdict` evidence line (`verdict::derive` result).
    #[allow(clippy::too_many_arguments)]
    pub fn emit_agreement_verdict(
        &mut self,
        kind: &'static str,
        slot: u64,
        our_hash_hex: Option<String>,
        peer_hash_hex: Option<String>,
        peer_slot: Option<u64>,
        tx_in_hex: Option<String>,
        consensus_inputs_fingerprint_hex: &str,
    ) -> io::Result<()> {
        self.emit(AdmissionLogEvent::AgreementVerdict {
            kind,
            slot,
            our_hash_hex,
            peer_hash_hex,
            peer_slot,
            tx_in_hex,
            consensus_inputs_fingerprint_hex: consensus_inputs_fingerprint_hex.to_string(),
        })
    }

    /// Flush the underlying sink (no-op when disabled).
    pub fn flush(&mut self) -> io::Result<()> {
        match self.inner.as_mut() {
            Some(w) => w.flush(),
            None => Ok(()),
        }
    }

    /// PRIVATE single funnel — the three `emit_*` methods are its only callers,
    /// so no caller outside this module can construct a non-subset variant.
    /// Deliberately NOT `pub`, and there is NO accessor returning the inner
    /// [`AdmissionLogWriter`] (closed vocabulary; DC-ADMIT-04 / DC-NODE-30).
    fn emit(&mut self, event: AdmissionLogEvent) -> io::Result<()> {
        match self.inner.as_mut() {
            Some(w) => w.emit(&event),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A `Write` backed by a shared buffer the test can inspect — lets the test
    /// read the emitted bytes WITHOUT the sink exposing its inner writer.
    #[derive(Clone, Default)]
    struct SharedBuf(Rc<RefCell<Vec<u8>>>);
    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn h(b: u8) -> String {
        format!("{b:02x}").repeat(32)
    }

    #[test]
    fn convergence_evidence_absent_path_emits_no_file() {
        // No --convergence-evidence-path => open(None) => disabled: no file is
        // created and every emit is a no-op (consensus + existing logs
        // unchanged). Nothing is written anywhere.
        let mut sink = ConvergenceEvidenceSink::open(None).expect("open(None) is infallible");
        assert!(!sink.is_enabled());
        sink.emit_block_received("127.0.0.1:3001", 100, &h(0xaa)).unwrap();
        sink.emit_block_admitted(100, &h(0xaa), &h(0xbb), &h(0xcc)).unwrap();
        sink.emit_agreement_verdict("agreed", 100, Some(h(0xaa)), Some(h(0xaa)), Some(100), None, &h(0xcc))
            .unwrap();
        sink.flush().unwrap();
        assert!(!sink.is_enabled());
    }

    #[test]
    fn convergence_evidence_writer_emits_closed_vocabulary() {
        let buf = SharedBuf::default();
        let mut sink = ConvergenceEvidenceSink::with_writer(AdmissionLogWriter::new(buf.clone()));
        sink.emit_block_received("127.0.0.1:3001", 99, &h(0xaa)).unwrap();
        sink.emit_block_admitted(100, &h(0xaa), &h(0xbb), &h(0xcc)).unwrap();
        sink.emit_agreement_verdict("agreed", 100, Some(h(0xaa)), Some(h(0xaa)), Some(100), None, &h(0xcc))
            .unwrap();

        let out = String::from_utf8(buf.0.borrow().clone()).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "exactly three evidence lines, one per emit");
        assert!(lines[0].contains(r#""event":"block_received""#));
        assert!(lines[1].contains(r#""event":"block_admitted""#));
        assert!(lines[2].contains(r#""event":"agreement_verdict""#));

        // Closed vocabulary: NONE of the excluded admission-lifecycle literals
        // may appear — the convergence file is an evidence transcript, not a
        // lifecycle log.
        for forbidden in [
            "admission_started",
            "snapshot_imported",
            "bootstrap_complete",
            "admission_halted",
            "admission_shutdown",
        ] {
            assert!(
                !out.contains(forbidden),
                "forbidden literal {forbidden} leaked into convergence evidence"
            );
        }
    }
}
