// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN closed admission-mode JSONL event vocabulary
//! (PHASE4-N-M-B S2).
//!
//! The admission mode's emitted log is a CLOSED sum of 8 variants.
//! Adding a new variant requires a code change here + a
//! corresponding allow-list update in
//! `ci/ci_check_admission_log_vocabulary_closed.sh`. Per
//! `[[feedback-shell-must-not-overstate-semantic-truth]]` the
//! admission-mode vocabulary is physically isolated from the
//! wire-only-mode vocabulary (`crate::live_log::event`):
//!   - admission-only literals MUST NOT appear in wire-only files,
//!   - wire-only-only literals MUST NOT appear in admission files,
//!   - shared literals (`node_started` / `node_shutdown`) MAY
//!     appear in both.
//!
//! Doctrine reference (memory):
//!   - `[[feedback-evidence-reducers-are-green-not-authority]]` —
//!     `AgreementVerdict` is GREEN evidence; this writer emits it
//!     as an evidence event, NOT a success / authority record.
//!     `Lagging` is evidence-state only.

/// Closed sum of admission-mode log event kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionLogEvent {
    /// Binary started in admission mode; peer count + key paths
    /// recorded (paths included for operator audit, not for
    /// authority). `consensus_inputs_fingerprint_hex` carries the
    /// canonical fingerprint of the imported LiveConsensusInputs
    /// bundle (DC-CONS-IN-02 / DC-ADMIT-10) so every downstream
    /// event in this transcript is bound to the operator oracle.
    AdmissionStarted {
        peer_count: u32,
        json_seed_path: String,
        wal_dir: String,
        consensus_inputs_fingerprint_hex: String,
    },
    /// JSON UTxO seed imported + persistent snapshot captured at
    /// the seed point. `imported_utxo_fp_hex` is the fingerprint
    /// of the imported UTxO map (DC-SEED-01); `utxo_entry_count`
    /// is the count of accepted entries (refscript fail-fasts are
    /// surfaced via `AdmissionHalted::BootstrapFatal`, not as a
    /// missing-count delta — see DC-ADMIT-09).
    SnapshotImported {
        seed_point_slot: u64,
        imported_utxo_fp_hex: String,
        utxo_entry_count: u64,
    },
    /// Warm-start bootstrap complete; runner is ready to consume
    /// peer-supplied blocks. `consensus_inputs_fingerprint_hex`
    /// (DC-ADMIT-10) binds this event — and every subsequent
    /// BlockAdmitted/AgreementVerdict — to the canonical
    /// consensus-inputs the runner was configured with.
    BootstrapComplete {
        initial_ledger_fp_hex: String,
        chain_tip_slot: u64,
        consensus_inputs_fingerprint_hex: String,
    },
    /// A block arrived from the peer. Pre-admit signal; the admit
    /// outcome is emitted separately as `BlockAdmitted` (success)
    /// or surfaced into `AgreementVerdict` (failure).
    BlockReceived {
        peer: String,
        slot: u64,
        block_hash_hex: String,
    },
    /// Block was admitted via `admit_via_block_validity` AND the
    /// per-admit WAL entry was successfully appended. `post_fp_hex`
    /// is the post-admit ledger fingerprint (the same fingerprint
    /// written to the WAL entry that just landed).
    /// `consensus_inputs_fingerprint_hex` binds the admit to the
    /// operator oracle (DC-ADMIT-10).
    BlockAdmitted {
        slot: u64,
        block_hash_hex: String,
        /// The admitted block's VALIDATED header `prev_hash` (parent linkage),
        /// from the same decode the reducer consumed — NEVER peer-supplied; the
        /// genesis predecessor is the all-zero hash. Capture-only fidelity for
        /// the post-switch branch-continuity verdict (S10, DC-EVIDENCE-05).
        prev_hash_hex: String,
        post_fp_hex: String,
        consensus_inputs_fingerprint_hex: String,
    },
    /// GREEN evidence emit: result of `verdict::derive`. `kind` is
    /// the closed-vocabulary discriminator from
    /// `verdict::verdict_kind`.
    /// `consensus_inputs_fingerprint_hex` binds the verdict to
    /// the operator oracle (DC-ADMIT-10).
    AgreementVerdict {
        kind: &'static str,
        slot: u64,
        our_hash_hex: Option<String>,
        peer_hash_hex: Option<String>,
        peer_slot: Option<u64>,
        tx_in_hex: Option<String>,
        consensus_inputs_fingerprint_hex: String,
    },
    /// Admission halted on a fatal evidence / I/O / bootstrap
    /// failure; the runner exits non-zero immediately after this
    /// event.
    AdmissionHalted { reason: AdmissionHaltReason },
    /// Admission shutting down on signal / clean upstream drop.
    AdmissionShutdown { reason: AdmissionShutdownReason },

    // PHASE4-N-AO S9 (DC-EVIDENCE-04): closed fork-choice convergence evidence --
    // observe-only taps proving the live SELECT path (candidate discovery -> LCA ->
    // selection -> branch proof -> fork-switch apply). NONE of these is ever read by
    // BLUE / selection / apply / fence logic; they only OBSERVE already-computed
    // authority outcomes.
    /// A competing block reached the `NeedsForkChoice` dispatch (S3).
    NeedsForkChoice {
        peer: String,
        slot: u64,
        block_hash_hex: String,
    },
    /// The durable last-common-ancestor was discovered by the S7 walk
    /// (`walk_to_durable_lca` OK). `candidate_header_count` is the depth of the
    /// competing branch above the LCA.
    LcaDiscovered {
        peer: String,
        fork_anchor_slot: u64,
        fork_anchor_hash_hex: String,
        candidate_header_count: u64,
    },
    /// The multi-header `CandidateFragment` was built (S2) from the LCA.
    CandidateFragmentBuilt {
        peer: String,
        anchor_slot: u64,
        candidate_header_count: u64,
    },
    /// `select_best_chain` decided (S3): `Win` (a competing branch is the winner) or
    /// `Loss`. `fork_switch_id` correlates a WIN to its single terminal apply event.
    ForkChoiceSelected {
        fork_switch_id: String,
        peer: String,
        result: ForkChoiceResult,
        winner_tip_slot: Option<u64>,
        winner_tip_hash_hex: Option<String>,
        consensus_inputs_fingerprint_hex: String,
    },
    /// S4 began block-fetching the winning branch (anchor -> winner_tip).
    BranchFetchStarted {
        fork_switch_id: String,
        peer: String,
        fork_anchor_slot: u64,
        winner_tip_slot: u64,
    },
    /// S4 completed the branch fetch (`block_count` bodies).
    BranchFetchCompleted {
        fork_switch_id: String,
        peer: String,
        block_count: u64,
    },
    /// S4 prevalidated the complete branch (bind + link + block_validity).
    BranchPrevalidated {
        fork_switch_id: String,
        peer: String,
        block_count: u64,
    },
    /// S4 applied the fork-switch -- the winning branch is durably adopted
    /// (`ForkSwitchOutcome::Adopted`). Terminal for its `fork_switch_id`.
    ForkSwitchApplied {
        fork_switch_id: String,
        peer: String,
        new_tip_slot: u64,
        new_tip_hash_hex: String,
        rollback_reason: &'static str,
    },
    /// S4 fork-switch proof failed (`ForkSwitchOutcome::ProofFailed`) -- the durable
    /// chain is unchanged. Terminal for its `fork_switch_id`. `failure_code` is a
    /// CLOSED enum (no free-form string).
    ForkSwitchFailed {
        fork_switch_id: String,
        peer: String,
        failure_code: ForkChoiceEvidenceFailure,
    },
    /// A provisional `fork_choice_selected{win}` was SUPERSEDED by a newer win on
    /// the same fork (the competing branch grew; the relay loop never applied this
    /// id). Terminal for its `fork_switch_id` -- so every win resolves to exactly
    /// one of applied / failed / superseded (DC-EVIDENCE-04), never dangling.
    ForkSwitchSuperseded {
        fork_switch_id: String,
        peer: String,
    },

    // PHASE4-N-AO S11 (DC-NODE-39): post-ForkChoiceWin missing-bridge fail-closed.
    /// A post-switch competing descendant could not be bridged to the durable
    /// adopted tip / a durable stored ancestor within k -- a STRUCTURED fail-closed
    /// outcome, NOT a silent no-op. `reason` is the closed `MissingBridgeReason`
    /// discriminator. Observe-only: the durable chain is byte-unchanged, the block
    /// is NOT admitted, and the forge fence is held. NEVER read by any authority
    /// path (selection / walk / apply / fence-decision).
    MissingBridge {
        peer: String,
        block_hash_hex: String,
        reason: &'static str,
    },
}

/// Closed result of `select_best_chain` for a competing candidate (S9 evidence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkChoiceResult {
    Win,
    Loss,
}

impl ForkChoiceResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Win => "win",
            Self::Loss => "loss",
        }
    }
}

/// Closed failure code for a fork-switch proof failure (S9 evidence). Maps the
/// `BranchProofError` surface to a closed code -- NO free-form error strings in
/// the evidence vocabulary (DC-EVIDENCE-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkChoiceEvidenceFailure {
    EmptyBranch,
    BodyUnavailable,
    BodyHeaderMismatch,
    BrokenParentLink,
    BodyInvalid,
    AnchorUnreachable,
}

impl ForkChoiceEvidenceFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyBranch => "empty_branch",
            Self::BodyUnavailable => "body_unavailable",
            Self::BodyHeaderMismatch => "body_header_mismatch",
            Self::BrokenParentLink => "broken_parent_link",
            Self::BodyInvalid => "body_invalid",
            Self::AnchorUnreachable => "anchor_unreachable",
        }
    }
}

/// Closed halt-reason sum. Each variant maps to a closed runner
/// exit code (B4 / B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionHaltReason {
    /// `AgreementVerdict::Diverged` observed — fatal.
    Diverged,
    /// `AgreementVerdict::InputNotFound` observed — fatal.
    InputNotFound,
    /// `WalStore::append` returned a fatal I/O error.
    WalAppendIo,
    /// Bootstrap / seed import / anchor mint failed irrecoverably.
    BootstrapFatal,
    /// Peer sent a block whose slot is outside
    /// `[epoch_start_slot, epoch_end_slot]` of the imported
    /// LiveConsensusInputs (DC-ADMIT-11 / ¬P-C2). The runner
    /// MUST NOT call `admit_via_block_validity`; the only
    /// outcome is a fail-closed halt.
    CrossEpochUse,
    /// Peer sent bytes the BLUE Conway decoder rejected. C
    /// tightens N-M-B's silent clean-exit path (DC-ADMIT-12 /
    /// ¬P-C9) into a halt: undecodable peer bytes are
    /// adversarial by default when no peer tip exists at the
    /// same slot for a `Diverged` verdict.
    PeerSentUndecodableBytes,
}

/// Closed shutdown-reason sum for clean exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionShutdownReason {
    /// SIGINT / SIGTERM received.
    SignalReceived,
    /// Upstream peer connection closed; runner drained.
    UpstreamDropped,
    /// Operator requested shutdown via control API (reserved;
    /// no callsite yet).
    OperatorRequested,
}

impl AdmissionLogEvent {
    /// Stable discriminator string emitted as the JSON `event`
    /// field. The set is closed — adding a variant means adding a
    /// discriminator + updating the CI gate.
    pub fn discriminator(&self) -> &'static str {
        match self {
            Self::AdmissionStarted { .. } => "admission_started",
            Self::SnapshotImported { .. } => "snapshot_imported",
            Self::BootstrapComplete { .. } => "bootstrap_complete",
            Self::BlockReceived { .. } => "block_received",
            Self::BlockAdmitted { .. } => "block_admitted",
            Self::AgreementVerdict { .. } => "agreement_verdict",
            Self::AdmissionHalted { .. } => "admission_halted",
            Self::AdmissionShutdown { .. } => "admission_shutdown",
            // PHASE4-N-AO S9 (DC-EVIDENCE-04) closed fork-choice events.
            Self::NeedsForkChoice { .. } => "needs_fork_choice",
            Self::LcaDiscovered { .. } => "lca_discovered",
            Self::CandidateFragmentBuilt { .. } => "candidate_fragment_built",
            Self::ForkChoiceSelected { .. } => "fork_choice_selected",
            Self::BranchFetchStarted { .. } => "branch_fetch_started",
            Self::BranchFetchCompleted { .. } => "branch_fetch_completed",
            Self::BranchPrevalidated { .. } => "branch_prevalidated",
            Self::ForkSwitchApplied { .. } => "fork_switch_applied",
            Self::ForkSwitchFailed { .. } => "fork_switch_failed",
            Self::ForkSwitchSuperseded { .. } => "fork_switch_superseded",
            // PHASE4-N-AO S11 (DC-NODE-39) closed missing-bridge fail-closed event.
            Self::MissingBridge { .. } => "missing_bridge",
        }
    }
}

impl AdmissionHaltReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Diverged => "diverged",
            Self::InputNotFound => "input_not_found",
            Self::WalAppendIo => "wal_append_io",
            Self::BootstrapFatal => "bootstrap_fatal",
            Self::CrossEpochUse => "cross_epoch_use",
            Self::PeerSentUndecodableBytes => "peer_sent_undecodable_bytes",
        }
    }
}

impl AdmissionShutdownReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SignalReceived => "signal_received",
            Self::UpstreamDropped => "upstream_dropped",
            Self::OperatorRequested => "operator_requested",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn admission_log_event_discriminator_round_trips_for_each_variant() {
        let cases: Vec<(AdmissionLogEvent, &'static str)> = vec![
            (
                AdmissionLogEvent::AdmissionStarted {
                    peer_count: 1,
                    json_seed_path: "x".into(),
                    wal_dir: "y".into(),
                    consensus_inputs_fingerprint_hex: "00".repeat(32),
                },
                "admission_started",
            ),
            (
                AdmissionLogEvent::SnapshotImported {
                    seed_point_slot: 0,
                    imported_utxo_fp_hex: "00".into(),
                    utxo_entry_count: 0,
                },
                "snapshot_imported",
            ),
            (
                AdmissionLogEvent::BootstrapComplete {
                    initial_ledger_fp_hex: "00".into(),
                    chain_tip_slot: 0,
                    consensus_inputs_fingerprint_hex: "00".repeat(32),
                },
                "bootstrap_complete",
            ),
            (
                AdmissionLogEvent::BlockReceived {
                    peer: "p".into(),
                    slot: 0,
                    block_hash_hex: "aa".into(),
                },
                "block_received",
            ),
            (
                AdmissionLogEvent::BlockAdmitted {
                    slot: 0,
                    block_hash_hex: "aa".into(),
                    prev_hash_hex: "cc".into(),
                    post_fp_hex: "bb".into(),
                    consensus_inputs_fingerprint_hex: "00".repeat(32),
                },
                "block_admitted",
            ),
            (
                AdmissionLogEvent::AgreementVerdict {
                    kind: "agreed",
                    slot: 0,
                    our_hash_hex: Some("aa".into()),
                    peer_hash_hex: Some("aa".into()),
                    peer_slot: None,
                    tx_in_hex: None,
                    consensus_inputs_fingerprint_hex: "00".repeat(32),
                },
                "agreement_verdict",
            ),
            (
                AdmissionLogEvent::AdmissionHalted {
                    reason: AdmissionHaltReason::Diverged,
                },
                "admission_halted",
            ),
            (
                AdmissionLogEvent::AdmissionShutdown {
                    reason: AdmissionShutdownReason::SignalReceived,
                },
                "admission_shutdown",
            ),
        ];
        for (e, expected) in cases {
            assert_eq!(e.discriminator(), expected);
        }
    }

    /// Compile-time exhaustiveness: adding a variant breaks this
    /// match until the new arm is filled in.
    #[test]
    fn admission_log_event_match_is_exhaustive() {
        let e = AdmissionLogEvent::AdmissionShutdown {
            reason: AdmissionShutdownReason::SignalReceived,
        };
        let _: &str = match &e {
            AdmissionLogEvent::AdmissionStarted { .. } => "admission_started",
            AdmissionLogEvent::SnapshotImported { .. } => "snapshot_imported",
            AdmissionLogEvent::BootstrapComplete { .. } => "bootstrap_complete",
            AdmissionLogEvent::BlockReceived { .. } => "block_received",
            AdmissionLogEvent::BlockAdmitted { .. } => "block_admitted",
            AdmissionLogEvent::AgreementVerdict { .. } => "agreement_verdict",
            AdmissionLogEvent::AdmissionHalted { .. } => "admission_halted",
            AdmissionLogEvent::AdmissionShutdown { .. } => "admission_shutdown",
            AdmissionLogEvent::NeedsForkChoice { .. } => "needs_fork_choice",
            AdmissionLogEvent::LcaDiscovered { .. } => "lca_discovered",
            AdmissionLogEvent::CandidateFragmentBuilt { .. } => "candidate_fragment_built",
            AdmissionLogEvent::ForkChoiceSelected { .. } => "fork_choice_selected",
            AdmissionLogEvent::BranchFetchStarted { .. } => "branch_fetch_started",
            AdmissionLogEvent::BranchFetchCompleted { .. } => "branch_fetch_completed",
            AdmissionLogEvent::BranchPrevalidated { .. } => "branch_prevalidated",
            AdmissionLogEvent::ForkSwitchApplied { .. } => "fork_switch_applied",
            AdmissionLogEvent::ForkSwitchFailed { .. } => "fork_switch_failed",
            AdmissionLogEvent::ForkSwitchSuperseded { .. } => "fork_switch_superseded",
            AdmissionLogEvent::MissingBridge { .. } => "missing_bridge",
        };
    }

    #[test]
    fn admission_log_event_agreement_verdict_carries_kind_discriminator() {
        // The `kind` field must be one of the closed
        // verdict::verdict_kind strings; the writer test in
        // writer.rs proves it lands as `"kind":"..."` in the JSON.
        let e = AdmissionLogEvent::AgreementVerdict {
            kind: "diverged",
            slot: 100,
            our_hash_hex: Some("a1".repeat(32)),
            peer_hash_hex: Some("b2".repeat(32)),
            peer_slot: None,
            tx_in_hex: None,
            consensus_inputs_fingerprint_hex: "00".repeat(32),
        };
        match e {
            AdmissionLogEvent::AgreementVerdict { kind, .. } => {
                assert!(matches!(kind, "agreed" | "lagging" | "diverged" | "input_not_found"));
            }
            _ => panic!("expected AgreementVerdict"),
        }
    }
}
