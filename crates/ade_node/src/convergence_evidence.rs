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
use crate::admission_log::{
    AdmissionLogEvent, AdmissionLogWriter, ForkChoiceEvidenceFailure, ForkChoiceResult,
};
use crate::mem_measure::rss_sampler::{
    sample_private_dirty_kib, sample_rss_anon_kib, sample_vm_hwm_kib, sample_vm_rss_kib, RssWindow,
};

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
        prev_hash_hex: &str,
        post_fp_hex: &str,
        consensus_inputs_fingerprint_hex: &str,
    ) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::BlockAdmitted {
            slot,
            block_hash_hex: block_hash_hex.to_string(),
            prev_hash_hex: prev_hash_hex.to_string(),
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

    // PHASE4-N-AO S9 (DC-EVIDENCE-04): closed fork-choice evidence emitters. Each
    // constructs ONE closed variant and funnels it through `emit` — observe-only,
    // derived from already-computed authority outcomes (the sink never reads back).
    #[allow(clippy::too_many_arguments)]
    pub fn emit_needs_fork_choice(&mut self, peer: &str, slot: u64, block_hash_hex: &str) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::NeedsForkChoice {
            peer: peer.to_string(),
            slot,
            block_hash_hex: block_hash_hex.to_string(),
        })
    }
    pub fn emit_lca_discovered(&mut self, peer: &str, fork_anchor_slot: u64, fork_anchor_hash_hex: &str, candidate_header_count: u64) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::LcaDiscovered {
            peer: peer.to_string(),
            fork_anchor_slot,
            fork_anchor_hash_hex: fork_anchor_hash_hex.to_string(),
            candidate_header_count,
        })
    }
    pub fn emit_candidate_fragment_built(&mut self, peer: &str, anchor_slot: u64, candidate_header_count: u64) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::CandidateFragmentBuilt {
            peer: peer.to_string(),
            anchor_slot,
            candidate_header_count,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn emit_fork_choice_selected(&mut self, fork_switch_id: &str, peer: &str, result: ForkChoiceResult, winner_tip_slot: Option<u64>, winner_tip_hash_hex: Option<String>, consensus_inputs_fingerprint_hex: &str) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::ForkChoiceSelected {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            result,
            winner_tip_slot,
            winner_tip_hash_hex,
            consensus_inputs_fingerprint_hex: consensus_inputs_fingerprint_hex.to_string(),
        })
    }
    pub fn emit_branch_fetch_started(&mut self, fork_switch_id: &str, peer: &str, fork_anchor_slot: u64, winner_tip_slot: u64) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::BranchFetchStarted {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            fork_anchor_slot,
            winner_tip_slot,
        })
    }
    pub fn emit_branch_fetch_completed(&mut self, fork_switch_id: &str, peer: &str, block_count: u64) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::BranchFetchCompleted {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            block_count,
        })
    }
    pub fn emit_branch_prevalidated(&mut self, fork_switch_id: &str, peer: &str, block_count: u64) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::BranchPrevalidated {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            block_count,
        })
    }
    pub fn emit_fork_switch_applied(&mut self, fork_switch_id: &str, peer: &str, new_tip_slot: u64, new_tip_hash_hex: &str) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::ForkSwitchApplied {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            new_tip_slot,
            new_tip_hash_hex: new_tip_hash_hex.to_string(),
            rollback_reason: "fork_choice_win",
        })
    }
    pub fn emit_fork_switch_failed(&mut self, fork_switch_id: &str, peer: &str, failure_code: ForkChoiceEvidenceFailure) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::ForkSwitchFailed {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            failure_code,
        })
    }
    pub fn emit_fork_switch_superseded(&mut self, fork_switch_id: &str, peer: &str) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::ForkSwitchSuperseded {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
        })
    }

    /// PHASE4-N-AO S11 (DC-NODE-39): a post-switch competing descendant could not be
    /// bridged to the durable adopted tip / a durable stored ancestor within k --
    /// structured fail-closed evidence. `reason` is the closed `MissingBridgeReason`
    /// discriminator (no free-form strings). Observe-only.
    pub fn emit_missing_bridge(&mut self, peer: &str, block_hash_hex: &str, reason: &'static str) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::MissingBridge {
            peer: peer.to_string(),
            block_hash_hex: block_hash_hex.to_string(),
            reason,
        })
    }

    /// PHASE4-N-AO S14 (DC-NODE-41): closed range re-fetch recovery emitters --
    /// observe-only, constructed from already-computed authority outcomes (the
    /// `RangeRefetch` request + the `RangeRefetchOutcome` the BLUE admit produced).
    pub fn emit_range_refetch_started(&mut self, fork_switch_id: &str, peer: &str, from_slot: u64, to_slot: u64, reason: &'static str) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::RangeRefetchStarted {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            from_slot,
            to_slot,
            reason,
        })
    }
    pub fn emit_range_refetch_completed(&mut self, fork_switch_id: &str, peer: &str, outcome: &'static str) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::RangeRefetchCompleted {
            fork_switch_id: fork_switch_id.to_string(),
            peer: peer.to_string(),
            outcome,
        })
    }

    /// MEM-MEASURE-A2 (OP-MEM-01): closed live memory-evidence emitters. Each constructs
    /// ONE closed variant and funnels it through `emit` -- observe-only; the sink never
    /// reads RSS back, and RSS magnitude never gates.
    #[allow(clippy::too_many_arguments)]
    pub fn emit_memory_measure(
        &mut self,
        point: &'static str,
        slot: u64,
        durable_tip_slot: u64,
        durable_tip_fp_hex: &str,
        rss_kib: u64,
        rss_hwm_kib: u64,
        rss_anon_kib: u64,
        private_dirty_kib: u64,
    ) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::MemoryMeasure {
            point,
            slot,
            durable_tip_slot,
            durable_tip_fp_hex: durable_tip_fp_hex.to_string(),
            rss_kib,
            rss_hwm_kib,
            rss_anon_kib,
            private_dirty_kib,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn emit_memory_summary(
        &mut self,
        sample_count: u64,
        rss_p50_kib: u64,
        rss_p95_kib: u64,
        rss_peak_kib: u64,
        rss_hwm_kib: u64,
        owned_rss_anon_p50_kib: u64,
        owned_rss_anon_peak_kib: u64,
        owned_private_dirty_p50_kib: u64,
        owned_private_dirty_peak_kib: u64,
        replay_verdict: &'static str,
    ) -> EvidenceEmitResult {
        self.emit(AdmissionLogEvent::MemorySummary {
            sample_count,
            rss_p50_kib,
            rss_p95_kib,
            rss_peak_kib,
            rss_hwm_kib,
            owned_rss_anon_p50_kib,
            owned_rss_anon_peak_kib,
            owned_private_dirty_p50_kib,
            owned_private_dirty_peak_kib,
            replay_verdict,
        })
    }

    /// PRIVATE single funnel — the `emit_*` methods are its only callers,
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
    incomplete: bool,
    /// MEM-MEASURE-A2: RSS samples accumulated over the run (GREEN accumulator;
    /// the /proc read happens in `emit_memory_measure` via the RED sampler). Used
    /// to derive the run's p50/p95/peak for the `memory_summary`.
    rss: RssWindow,
    /// MEM-OPT-OPS S3: parallel OWNED windows — RssAnon (the OP-MEM-02 metric) +
    /// Private_Dirty (informational) — for the owned summary percentiles.
    rss_anon: RssWindow,
    private_dirty: RssWindow,
}

impl ConvergenceEvidence {
    /// `fingerprint` is the DC-ADMIT-10 oracle binding (the imported bundle's
    /// `canonical.fingerprint`, or the recovered-oracle ledger fingerprint in a
    /// warm-start). Per-event peer attribution is carried by each `emit_*` call
    /// (DC-NODE-34), not a single fixed label.
    pub fn new(sink: ConvergenceEvidenceSink, fingerprint: &Hash32) -> Self {
        Self {
            sink,
            consensus_inputs_fingerprint_hex: hex_lowercase(&fingerprint.0),
            incomplete: false,
            rss: RssWindow::new(),
            rss_anon: RssWindow::new(),
            private_dirty: RssWindow::new(),
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
    pub fn emit_block_received(&mut self, peer: &str, slot: u64, block_hash: &Hash32) {
        // DC-NODE-34 (peer identity preserved): block_received MUST carry the
        // PER-BLOCK source peer, not a fixed sink label -- otherwise a multi-peer
        // run mis-attributes every peer's blocks to the first peer (the evidence
        // artifact that masked live multi-peer SELECT). `peer_label` remains the
        // warm-start followed-peer default for single-peer / agreement contexts.
        let r = self
            .sink
            .emit_block_received(peer, slot, &hex_lowercase(&block_hash.0));
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
        prev_hash: &Hash32,
        post_fp: &Hash32,
        peer_tip: Option<TipPoint>,
    ) {
        let r1 = self.sink.emit_block_admitted(
            slot,
            &hex_lowercase(&block_hash.0),
            &hex_lowercase(&prev_hash.0),
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

    // PHASE4-N-AO S9 (DC-EVIDENCE-04): observe-only fork-choice taps. Each wraps the
    // closed sink emitter + `note`s a write failure (which flips `incomplete`, never
    // alters authority). Take primitive values so this module stays decoupled from
    // the selector/Point types; the taps extract from the authority outcomes.
    pub fn emit_needs_fork_choice(&mut self, peer: &str, slot: u64, block_hash: &Hash32) {
        let r = self
            .sink
            .emit_needs_fork_choice(peer, slot, &hex_lowercase(&block_hash.0));
        self.note(r);
    }
    pub fn emit_lca_discovered(
        &mut self,
        peer: &str,
        anchor_slot: u64,
        anchor_hash: &Hash32,
        header_count: u64,
    ) {
        let r = self.sink.emit_lca_discovered(
            peer,
            anchor_slot,
            &hex_lowercase(&anchor_hash.0),
            header_count,
        );
        self.note(r);
    }
    pub fn emit_candidate_fragment_built(&mut self, peer: &str, anchor_slot: u64, header_count: u64) {
        let r = self
            .sink
            .emit_candidate_fragment_built(peer, anchor_slot, header_count);
        self.note(r);
    }
    pub fn emit_fork_choice_selected(
        &mut self,
        fork_switch_id: &str,
        peer: &str,
        result: ForkChoiceResult,
        winner_tip_slot: Option<u64>,
        winner_tip_hash: Option<&Hash32>,
    ) {
        let r = self.sink.emit_fork_choice_selected(
            fork_switch_id,
            peer,
            result,
            winner_tip_slot,
            winner_tip_hash.map(|h| hex_lowercase(&h.0)),
            &self.consensus_inputs_fingerprint_hex,
        );
        self.note(r);
    }
    pub fn emit_branch_fetch_started(
        &mut self,
        fork_switch_id: &str,
        peer: &str,
        anchor_slot: u64,
        winner_tip_slot: u64,
    ) {
        let r = self
            .sink
            .emit_branch_fetch_started(fork_switch_id, peer, anchor_slot, winner_tip_slot);
        self.note(r);
    }
    pub fn emit_branch_fetch_completed(&mut self, fork_switch_id: &str, peer: &str, block_count: u64) {
        let r = self
            .sink
            .emit_branch_fetch_completed(fork_switch_id, peer, block_count);
        self.note(r);
    }
    pub fn emit_branch_prevalidated(&mut self, fork_switch_id: &str, peer: &str, block_count: u64) {
        let r = self
            .sink
            .emit_branch_prevalidated(fork_switch_id, peer, block_count);
        self.note(r);
    }
    pub fn emit_fork_switch_applied(
        &mut self,
        fork_switch_id: &str,
        peer: &str,
        new_tip_slot: u64,
        new_tip_hash: &Hash32,
    ) {
        let r = self.sink.emit_fork_switch_applied(
            fork_switch_id,
            peer,
            new_tip_slot,
            &hex_lowercase(&new_tip_hash.0),
        );
        self.note(r);
    }
    pub fn emit_fork_switch_failed(
        &mut self,
        fork_switch_id: &str,
        peer: &str,
        failure_code: ForkChoiceEvidenceFailure,
    ) {
        let r = self
            .sink
            .emit_fork_switch_failed(fork_switch_id, peer, failure_code);
        self.note(r);
    }
    pub fn emit_fork_switch_superseded(&mut self, fork_switch_id: &str, peer: &str) {
        let r = self.sink.emit_fork_switch_superseded(fork_switch_id, peer);
        self.note(r);
    }

    /// PHASE4-N-AO S11 (DC-NODE-39): observe-only missing-bridge tap. The per-block
    /// source `peer` (DC-NODE-34) + the un-bridgeable competing block's hash + the
    /// closed `reason`. A write failure flips `incomplete`; never alters authority.
    pub fn emit_missing_bridge(&mut self, peer: &str, block_hash: &Hash32, reason: &'static str) {
        let r = self
            .sink
            .emit_missing_bridge(peer, &hex_lowercase(&block_hash.0), reason);
        self.note(r);
    }

    /// PHASE4-N-AO S14 (DC-NODE-41): observe-only range re-fetch recovery taps. The
    /// `started` carries the requested range + the closed trigger reason; the
    /// `completed` carries the closed `RangeRefetchOutcome` discriminator. A write
    /// failure flips `incomplete`; never alters authority.
    pub fn emit_range_refetch_started(&mut self, fork_switch_id: &str, peer: &str, from_slot: u64, to_slot: u64, reason: &'static str) {
        let r = self
            .sink
            .emit_range_refetch_started(fork_switch_id, peer, from_slot, to_slot, reason);
        self.note(r);
    }
    pub fn emit_range_refetch_completed(&mut self, fork_switch_id: &str, peer: &str, outcome: &'static str) {
        let r = self
            .sink
            .emit_range_refetch_completed(fork_switch_id, peer, outcome);
        self.note(r);
    }

    /// MEM-MEASURE-A2 (OP-MEM-01): observe-only memory-evidence taps. `emit_memory_measure`
    /// samples process RSS via the RED rss_sampler (the /proc read), records it for the run
    /// summary, and emits the sample paired with the durable tip fingerprint observed at a
    /// closed measurement point. RSS magnitude never gates; a write failure flips
    /// `incomplete`, never alters authority. Off-Linux (no VmRSS) the sample is skipped.
    pub fn emit_memory_measure(
        &mut self,
        point: &'static str,
        slot: u64,
        durable_tip_slot: u64,
        durable_tip_fp: &Hash32,
    ) {
        if let Some(s) = sample_vm_rss_kib() {
            self.rss.record(s);
            let anon = sample_rss_anon_kib();
            let dirty = sample_private_dirty_kib();
            if let Some(a) = anon {
                self.rss_anon.record(a);
            }
            if let Some(d) = dirty {
                self.private_dirty.record(d);
            }
            let r = self.sink.emit_memory_measure(
                point,
                slot,
                durable_tip_slot,
                &hex_lowercase(&durable_tip_fp.0),
                s.0,
                sample_vm_hwm_kib().map(|h| h.0).unwrap_or(0),
                anon.map(|a| a.0).unwrap_or(0),
                dirty.map(|d| d.0).unwrap_or(0),
            );
            self.note(r);
        }
    }

    /// Emit the run-level memory summary: p50/p95/peak over the run's samples + the
    /// `replay_verdict` (`agreed` iff the run completed with no Diverged verdict/halt, so
    /// the durable chain is replay-equivalent by the enforced DC-WAL-03).
    pub fn emit_memory_summary(&mut self, replay_verdict: &'static str) {
        let r = self.sink.emit_memory_summary(
            self.rss.count() as u64,
            self.rss.p50_kib().unwrap_or(0),
            self.rss.p95_kib().unwrap_or(0),
            self.rss.peak_kib().unwrap_or(0),
            // All-time VmHWM: records the import peak even after the allocator
            // returns the pages (MEM-OPT-OPS S2).
            sample_vm_hwm_kib().map(|h| h.0).unwrap_or(0),
            // OWNED summary (MEM-OPT-OPS S3): RssAnon (OP-MEM-02 metric) + Private_Dirty.
            self.rss_anon.p50_kib().unwrap_or(0),
            self.rss_anon.peak_kib().unwrap_or(0),
            self.private_dirty.p50_kib().unwrap_or(0),
            self.private_dirty.peak_kib().unwrap_or(0),
            replay_verdict,
        );
        self.note(r);
    }
}

/// PHASE4-N-AO S9 (DC-EVIDENCE-04): the bounded deterministic `fork_switch_id` that
/// correlates one decide->apply cycle. Derived from the canonical tuple already in
/// `PendingForkSwitch` (winning_peer + fork_anchor + winner_tip) -- a blake2b-256
/// hex PREFIX, never free-form text. Same inputs at decide + apply => same id.
pub fn fork_switch_id(
    winning_peer: &str,
    anchor_slot: u64,
    anchor_hash: &Hash32,
    winner_tip_slot: u64,
    winner_tip_hash: &Hash32,
) -> String {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(winning_peer.as_bytes());
    buf.extend_from_slice(&anchor_slot.to_be_bytes());
    buf.extend_from_slice(&anchor_hash.0);
    buf.extend_from_slice(&winner_tip_slot.to_be_bytes());
    buf.extend_from_slice(&winner_tip_hash.0);
    let h = ade_crypto::blake2b::blake2b_256(&buf);
    hex_lowercase(&h.0[..8])
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

    // PHASE4-N-AO S9 (DC-EVIDENCE-04): the S4-apply acceptance -- a
    // fork_choice_selected{win} is followed by EXACTLY ONE terminal
    // (fork_switch_applied | fork_switch_failed), correlated by fork_switch_id.
    #[test]
    fn fork_choice_win_paired_with_exactly_one_terminal_applied() {
        let buf = SharedBuf::default();
        let mut ev = ConvergenceEvidence::new(
            ConvergenceEvidenceSink::with_writer(Box::new(buf.clone())),
            &Hash32([0xCC; 32]),
        );
        let win_hash = Hash32([0xAB; 32]);
        let fsid = fork_switch_id("peer-1", 100, &Hash32([0x11; 32]), 130, &win_hash);
        ev.emit_needs_fork_choice("peer-1", 130, &win_hash);
        ev.emit_lca_discovered("peer-1", 100, &Hash32([0x11; 32]), 3);
        ev.emit_candidate_fragment_built("peer-1", 100, 3);
        ev.emit_fork_choice_selected(&fsid, "peer-1", ForkChoiceResult::Win, Some(130), Some(&win_hash));
        ev.emit_branch_fetch_started(&fsid, "peer-1", 100, 130);
        ev.emit_branch_fetch_completed(&fsid, "peer-1", 3);
        ev.emit_branch_prevalidated(&fsid, "peer-1", 3);
        ev.emit_fork_switch_applied(&fsid, "peer-1", 130, &win_hash);

        let text = String::from_utf8(buf.0.borrow().clone()).unwrap();
        let wins = text
            .lines()
            .filter(|l| l.contains("\"fork_choice_selected\"") && l.contains("\"result\":\"win\"") && l.contains(&fsid))
            .count();
        assert_eq!(wins, 1, "exactly one win for this fork_switch_id");
        let terminals: Vec<&str> = text
            .lines()
            .filter(|l| {
                (l.contains("\"fork_switch_applied\"")
                    || l.contains("\"fork_switch_failed\"")
                    || l.contains("\"fork_switch_superseded\""))
                    && l.contains(&fsid)
            })
            .collect();
        assert_eq!(terminals.len(), 1, "a win => EXACTLY ONE terminal (applied|failed), never dangling/double");
        assert!(terminals[0].contains("\"fork_switch_applied\""), "this win adopted -> applied");
        assert!(terminals[0].contains("\"rollback_reason\":\"fork_choice_win\""), "applied carries the closed rollback_reason");
    }

    #[test]
    fn fork_choice_win_failed_terminal_carries_closed_code() {
        let buf = SharedBuf::default();
        let mut ev = ConvergenceEvidence::new(
            ConvergenceEvidenceSink::with_writer(Box::new(buf.clone())),
            &Hash32([0xCC; 32]),
        );
        let wh = Hash32([0xAB; 32]);
        let fsid = fork_switch_id("peer-2", 50, &Hash32([0x22; 32]), 80, &wh);
        ev.emit_fork_choice_selected(&fsid, "peer-2", ForkChoiceResult::Win, Some(80), Some(&wh));
        ev.emit_fork_switch_failed(&fsid, "peer-2", ForkChoiceEvidenceFailure::BodyInvalid);
        let text = String::from_utf8(buf.0.borrow().clone()).unwrap();
        let terminals: Vec<&str> = text
            .lines()
            .filter(|l| {
                (l.contains("\"fork_switch_applied\"")
                    || l.contains("\"fork_switch_failed\"")
                    || l.contains("\"fork_switch_superseded\""))
                    && l.contains(&fsid)
            })
            .collect();
        assert_eq!(terminals.len(), 1, "exactly one terminal");
        assert!(
            terminals[0].contains("\"failure_code\":\"body_invalid\""),
            "the failure terminal carries a CLOSED code, never a free-form string"
        );
    }

    #[test]
    fn superseded_win_pairs_to_superseded_terminal() {
        // Two wins on the SAME fork (the competing branch grew, so winner_tip and
        // thus fork_switch_id change). The first is superseded by the second; the
        // second applies. Each fork_switch_id resolves to EXACTLY ONE terminal --
        // no dangling win (the relay loop only applies the FINAL pending).
        let buf = SharedBuf::default();
        let mut ev = ConvergenceEvidence::new(
            ConvergenceEvidenceSink::with_writer(Box::new(buf.clone())),
            &Hash32([0xCC; 32]),
        );
        let anchor = Hash32([0x11; 32]);
        let tip1 = Hash32([0xA1; 32]);
        let tip2 = Hash32([0xA2; 32]);
        let fsid1 = fork_switch_id("peer-1", 100, &anchor, 120, &tip1);
        let fsid2 = fork_switch_id("peer-1", 100, &anchor, 130, &tip2);
        assert_ne!(fsid1, fsid2, "a growing winner_tip => distinct ids");
        ev.emit_fork_choice_selected(&fsid1, "peer-1", ForkChoiceResult::Win, Some(120), Some(&tip1));
        ev.emit_fork_choice_selected(&fsid2, "peer-1", ForkChoiceResult::Win, Some(130), Some(&tip2));
        ev.emit_fork_switch_superseded(&fsid1, "peer-1"); // win 1 overtaken by win 2
        ev.emit_fork_switch_applied(&fsid2, "peer-1", 130, &tip2); // win 2 adopted
        let text = String::from_utf8(buf.0.borrow().clone()).unwrap();
        let terminals = |fsid: &str| {
            text.lines()
                .filter(|l| {
                    (l.contains("\"fork_switch_applied\"")
                        || l.contains("\"fork_switch_failed\"")
                        || l.contains("\"fork_switch_superseded\""))
                        && l.contains(fsid)
                })
                .count()
        };
        assert_eq!(terminals(&fsid1), 1, "the superseded win => EXACTLY ONE terminal (superseded)");
        assert_eq!(terminals(&fsid2), 1, "the applied win => EXACTLY ONE terminal (applied)");
    }

    #[test]
    fn fork_switch_id_is_deterministic_and_bounded() {
        let a = fork_switch_id("p", 1, &Hash32([0x01; 32]), 2, &Hash32([0x02; 32]));
        let b = fork_switch_id("p", 1, &Hash32([0x01; 32]), 2, &Hash32([0x02; 32]));
        assert_eq!(a, b, "same canonical tuple => same id");
        assert_eq!(a.len(), 16, "bounded 16-hex (8-byte) prefix");
        let c = fork_switch_id("q", 1, &Hash32([0x01; 32]), 2, &Hash32([0x02; 32]));
        assert_ne!(a, c, "different peer => different id");
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
            sink.emit_block_admitted(100, &h(0xaa), &h(0xde), &h(0xbb), &h(0xcc)),
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
            sink.emit_block_admitted(100, &h(0xaa), &h(0xde), &h(0xbb), &h(0xcc)),
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
            sink.emit_block_admitted(2, &h(0x02), &h(0x05), &h(0x03), &h(0x04)),
            EvidenceEmitResult::FailedAndPoisoned
        );
    }

    #[test]
    fn convergence_evidence_context_marks_incomplete_on_write_failure() {
        let mut ev = ConvergenceEvidence::new(
            ConvergenceEvidenceSink::with_writer(Box::new(FailingWriter)),
            &Hash32([0xCC; 32]),
        );
        assert!(!ev.is_incomplete());
        ev.emit_block_received("127.0.0.1:3001", 100, &Hash32([0xAA; 32]));
        assert!(ev.is_incomplete(), "a write failure marks the transcript incomplete");
    }
}
