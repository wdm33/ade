// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Dedicated convergence-evidence sink + context (PHASE4-N-AJ).
//!
//! The convergence-evidence transcript (CE-AI-6) is a NARROW evidence file, not
//! a lifecycle log. [`ConvergenceEvidenceSink`] is the only way to write it: it
//! exposes exactly three emit methods — one per allowed event — and NO accessor
//! to the raw inner writer, so the file cannot become a dumping ground for
//! sched / forge / admission-lifecycle events.
//!
//! Closed convergence vocabulary = the 3-variant subset of the reused
//! [`AdmissionLogEvent`] (no new evidence vocabulary enum; ¬AJ-3):
//! `block_received` / `block_admitted` / `agreement_verdict` — all in
//! `ci/ci_check_convergence_evidence_schema.sh`'s ALLOWED set.
//!
//! **Hard line: evidence observes authority; evidence never becomes authority.**
//! [`ConvergenceEvidence`] (AJ-S2) bundles the sink + the oracle binding + the
//! followed-peer label and emits, as a GREEN side-output, what the participant
//! path already decided via `pump_block` (admit) + `verdict::derive`. A write
//! error is **surfaced** ([`EvidenceEmitResult::FailedAndPoisoned`]) and marks
//! the transcript incomplete — it is NEVER swallowed as success and NEVER
//! propagated into the authoritative loop.
//!
//! TCB: the event *selection* (which [`AdmissionLogEvent`] variant + the
//! `verdict::derive` mapping) is GREEN and generic over `W: Write`; the
//! file-backed instantiation (`File::create` + the byte writes) is RED.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use ade_ledger::receive::events::TipPoint;
use ade_network::codec::chain_sync::{Point as ChainSyncPoint, Tip};
use ade_types::{Hash32, SlotNo};

use crate::admission::verdict::{derive, verdict_kind, AgreementVerdict, BlockAdmitOutcome};
use crate::admission_log::{AdmissionLogEvent, AdmissionLogWriter};

/// Lowercase hex of a byte slice. Local copy (mirrors `admission::runner` /
/// `wire_only`) so the convergence transcript is byte-identical to the
/// admission transcript's hash fields.
fn hex_lowercase(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// The outcome of one evidence emit. A write error is **never** silently
/// swallowed as success: it surfaces as `FailedAndPoisoned` (and is also
/// retained in the sink's `poisoned` flag). Non-fatal to authority, but never
/// invisible to evidence status.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceEmitResult {
    /// The event was written to the configured sink.
    Written,
    /// No sink configured (`--convergence-evidence-path` absent); no-op.
    Disabled,
    /// A write error occurred; the sink is now poisoned and the transcript is
    /// incomplete / unusable for CE-AI-6. NON-FATAL to authority — the caller
    /// continues — but MUST NOT be treated as success.
    FailedAndPoisoned,
}

/// Opt-in, closed-vocabulary convergence-evidence sink. Wraps an optional
/// [`AdmissionLogWriter`]; `None` => disabled (every emit is a no-op).
pub struct ConvergenceEvidenceSink {
    inner: Option<AdmissionLogWriter<Box<dyn Write>>>,
    poisoned: bool,
}

impl ConvergenceEvidenceSink {
    /// Open the file-backed sink (RED). `None` => disabled: no file is created
    /// and every emit is a no-op. `Some(p)` => `File::create(p)` + a writer.
    pub fn open(path: Option<&Path>) -> io::Result<Self> {
        let inner: Option<AdmissionLogWriter<Box<dyn Write>>> = match path {
            None => None,
            Some(p) => Some(AdmissionLogWriter::new(Box::new(File::create(p)?))),
        };
        Ok(Self { inner, poisoned: false })
    }

    /// Construct over an arbitrary boxed sink (GREEN seam: tests use a shared
    /// buffer; production uses the file-backed [`ConvergenceEvidenceSink::open`]).
    /// Always enabled.
    pub fn with_writer(sink: Box<dyn Write>) -> Self {
        Self { inner: Some(AdmissionLogWriter::new(sink)), poisoned: false }
    }

    /// A disabled sink (no inner writer); every emit is a no-op.
    pub fn disabled() -> Self {
        Self { inner: None, poisoned: false }
    }

    /// Whether a sink is backing this writer (a path was supplied).
    pub fn is_enabled(&self) -> bool {
        self.inner.is_some()
    }

    /// Whether a write error has poisoned the transcript (incomplete / unusable
    /// for CE-AI-6).
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Emit a `block_received` evidence line (each peer block considered).
    pub fn emit_block_received(
        &mut self,
        peer: &str,
        slot: u64,
        block_hash_hex: &str,
    ) -> EvidenceEmitResult {
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
    ) -> EvidenceEmitResult {
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
    ) -> EvidenceEmitResult {
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

    /// PRIVATE single funnel — the three `emit_*` methods are its only callers,
    /// so no caller outside this module can construct a non-subset variant.
    /// Deliberately NOT `pub`, and there is NO accessor returning the inner
    /// [`AdmissionLogWriter`] (closed vocabulary; DC-ADMIT-04 / DC-NODE-30).
    /// On a write error it poisons + returns `FailedAndPoisoned` — it never
    /// returns a success value for a failed write.
    fn emit(&mut self, event: AdmissionLogEvent) -> EvidenceEmitResult {
        if self.poisoned {
            return EvidenceEmitResult::FailedAndPoisoned;
        }
        match self.inner.as_mut() {
            None => EvidenceEmitResult::Disabled,
            Some(w) => match w.emit(&event) {
                Ok(()) => EvidenceEmitResult::Written,
                Err(_) => {
                    self.poisoned = true;
                    EvidenceEmitResult::FailedAndPoisoned
                }
            },
        }
    }
}

/// AJ-S2 GREEN evidence context: the dedicated sink + the DC-ADMIT-10 oracle
/// binding + the followed-peer label, plus an `incomplete` accumulator. The
/// participant loop calls the `emit_*` methods; a write failure flips
/// `incomplete` (and the sink's `poisoned`) without ever disrupting authority.
pub struct ConvergenceEvidence {
    sink: ConvergenceEvidenceSink,
    consensus_inputs_fingerprint_hex: String,
    peer_label: String,
    incomplete: bool,
}

impl ConvergenceEvidence {
    /// `fingerprint` is the DC-ADMIT-10 oracle binding (the imported bundle's
    /// `canonical.fingerprint`, or the recovered-oracle ledger fingerprint in a
    /// warm-start). `peer_label` is the followed peer addr.
    pub fn new(sink: ConvergenceEvidenceSink, fingerprint: &Hash32, peer_label: String) -> Self {
        Self {
            sink,
            consensus_inputs_fingerprint_hex: hex_lowercase(&fingerprint.0),
            peer_label,
            incomplete: false,
        }
    }

    /// The transcript is incomplete (a write failed) — the operator must NOT
    /// commit it for CE-AI-6.
    pub fn is_incomplete(&self) -> bool {
        self.incomplete || self.sink.is_poisoned()
    }

    fn note(&mut self, r: EvidenceEmitResult) {
        if r == EvidenceEmitResult::FailedAndPoisoned {
            self.incomplete = true;
        }
    }

    /// `block_received` — evidence of **peer input** (not local admission), for
    /// every considered peer block, before drop/admit/refuse.
    pub fn emit_block_received(&mut self, slot: u64, block_hash: &Hash32) {
        let r = self
            .sink
            .emit_block_received(&self.peer_label, slot, &hex_lowercase(&block_hash.0));
        self.note(r);
    }

    /// `block_admitted` (proof of local durable admission) + `agreement_verdict`
    /// (GREEN comparison vs the observed peer tip). Called ONLY after a
    /// successful `pump_block`. `peer_tip` is the observed followed peer tip
    /// (`None` => `Origin`).
    pub fn emit_admit_and_verdict(
        &mut self,
        slot: u64,
        block_hash: &Hash32,
        post_fp: &Hash32,
        peer_tip: Option<TipPoint>,
    ) {
        let r1 = self.sink.emit_block_admitted(
            slot,
            &hex_lowercase(&block_hash.0),
            &hex_lowercase(&post_fp.0),
            &self.consensus_inputs_fingerprint_hex,
        );
        self.note(r1);

        let outcome = BlockAdmitOutcome::Valid {
            slot: SlotNo(slot),
            block_hash: block_hash.clone(),
            post_fp: post_fp.clone(),
        };
        let tip = match peer_tip {
            Some(tp) => Tip {
                point: ChainSyncPoint::Block { slot: tp.slot, hash: tp.hash },
                block_no: tp.block_no,
            },
            None => Tip { point: ChainSyncPoint::Origin, block_no: 0 },
        };
        let v = derive(&outcome, &tip);
        // Mirror admission::runner::emit_verdict's verdict->fields mapping.
        let (vslot, our_h, peer_h, peer_slot, tx_in) = match &v {
            AgreementVerdict::Agreed { slot, hash } => {
                (slot.0, Some(hex_lowercase(&hash.0)), Some(hex_lowercase(&hash.0)), None, None)
            }
            AgreementVerdict::Lagging { our_slot, peer_slot } => {
                (our_slot.0, None, None, Some(peer_slot.0), None)
            }
            AgreementVerdict::Diverged { slot, our_hash, peer_hash } => (
                slot.0,
                Some(hex_lowercase(&our_hash.0)),
                Some(hex_lowercase(&peer_hash.0)),
                None,
                None,
            ),
            AgreementVerdict::InputNotFound { tx_in_hex } => {
                (0, None, None, None, Some(tx_in_hex.clone()))
            }
        };
        let r2 = self.sink.emit_agreement_verdict(
            verdict_kind(&v),
            vslot,
            our_h,
            peer_h,
            peer_slot,
            tx_in,
            &self.consensus_inputs_fingerprint_hex,
        );
        self.note(r2);
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

    /// A `Write` that always errors — exercises the poison path.
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::Other, "boom"))
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
        // No --convergence-evidence-path => open(None) => disabled: every emit
        // is a no-op returning Disabled. No file, no bytes anywhere.
        let mut sink = ConvergenceEvidenceSink::open(None).expect("open(None) is infallible");
        assert!(!sink.is_enabled());
        assert_eq!(
            sink.emit_block_received("127.0.0.1:3001", 100, &h(0xaa)),
            EvidenceEmitResult::Disabled
        );
        assert_eq!(
            sink.emit_block_admitted(100, &h(0xaa), &h(0xbb), &h(0xcc)),
            EvidenceEmitResult::Disabled
        );
        assert!(!sink.is_poisoned());
    }

    #[test]
    fn convergence_evidence_writer_emits_closed_vocabulary() {
        let buf = SharedBuf::default();
        let mut sink = ConvergenceEvidenceSink::with_writer(Box::new(buf.clone()));
        assert_eq!(
            sink.emit_block_received("127.0.0.1:3001", 99, &h(0xaa)),
            EvidenceEmitResult::Written
        );
        assert_eq!(
            sink.emit_block_admitted(100, &h(0xaa), &h(0xbb), &h(0xcc)),
            EvidenceEmitResult::Written
        );
        assert_eq!(
            sink.emit_agreement_verdict("agreed", 100, Some(h(0xaa)), Some(h(0xaa)), None, None, &h(0xcc)),
            EvidenceEmitResult::Written
        );

        let out = String::from_utf8(buf.0.borrow().clone()).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "exactly three evidence lines, one per emit");
        assert!(lines[0].contains(r#""event":"block_received""#));
        assert!(lines[1].contains(r#""event":"block_admitted""#));
        assert!(lines[2].contains(r#""event":"agreement_verdict""#));
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

    #[test]
    fn convergence_evidence_write_failure_poisons_and_is_surfaced() {
        // A failing sink => emit returns FailedAndPoisoned (NOT a success value),
        // is_poisoned() latches, and every later emit also fails. Never swallowed.
        let mut sink = ConvergenceEvidenceSink::with_writer(Box::new(FailingWriter));
        assert_eq!(
            sink.emit_block_received("p", 1, &h(0x01)),
            EvidenceEmitResult::FailedAndPoisoned
        );
        assert!(sink.is_poisoned());
        assert_eq!(
            sink.emit_block_admitted(2, &h(0x02), &h(0x03), &h(0x04)),
            EvidenceEmitResult::FailedAndPoisoned
        );
    }

    #[test]
    fn convergence_evidence_context_marks_incomplete_on_write_failure() {
        let mut ev = ConvergenceEvidence::new(
            ConvergenceEvidenceSink::with_writer(Box::new(FailingWriter)),
            &Hash32([0xCC; 32]),
            "127.0.0.1:3001".to_string(),
        );
        assert!(!ev.is_incomplete());
        ev.emit_block_received(100, &Hash32([0xAA; 32]));
        assert!(ev.is_incomplete(), "a write failure marks the transcript incomplete");
    }
}
