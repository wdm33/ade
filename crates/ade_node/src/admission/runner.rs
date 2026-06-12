// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED admission runner (PHASE4-N-M-B S4).
//!
//! Sole entry point composing N-M-A storage + N-L wire + Conway
//! BLUE authority + the GREEN verdict reducer into one tokio
//! event loop. The runner takes a closed peer-event channel
//! (`AdmissionPeerEvent`) so it can be unit-tested without a real
//! cardano-node peer; the production wiring (B5) starts a wire
//! pump that produces the same events.
//!
//! TCB color: RED. Owns no new authority. All decision-making
//! routes through:
//!   - `admit_via_block_validity` (BLUE, CN-CONS-08),
//!   - `WalStore::append` (DC-WAL-01),
//!   - `verdict::derive` (GREEN evidence reducer).
//!
//! Hard invariants:
//!   - exactly one `pub fn run_admission` (CI gate
//!     `ci_check_admission_runner_closure.sh`),
//!   - exactly one `AdmittedBlock` ↦ exactly one WAL append ↦
//!     exactly one `AgreementVerdict` emit (DC-ADMIT-02/03/05),
//!   - `Diverged` / `InputNotFound` halt fatal IMMEDIATELY
//!     (DC-ADMIT-06),
//!   - admission JSONL vocabulary is closed
//!     (DC-ADMIT-04, enforced via the bidirectional gate).

use std::io::Write;
use std::sync::Arc;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::block_validity::decode_block;
use ade_ledger::block_validity::verdict::BlockValidityError;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::receive::admit_via_block_validity;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{BlockVerdictTag, WalEntry, WalStore};
use ade_network::codec::chain_sync::Tip;
use ade_types::{EpochNo, Hash32, SlotNo};
use tokio::sync::{mpsc, watch, Mutex};

use super::verdict::{
    derive as derive_verdict, verdict_kind, AgreementVerdict, BlockAdmitOutcome,
    InvalidAdmitReason,
};
use crate::admission_log::{
    AdmissionHaltReason, AdmissionLogEvent, AdmissionLogWriter, AdmissionShutdownReason,
};

/// Live agreement diverged exit code.
pub const EXIT_LIVE_AGREEMENT_DIVERGED: i32 = 30;
/// Comparison-input not found exit code (evidence source gap).
pub const EXIT_LIVE_INPUT_NOT_FOUND: i32 = 31;
/// Peer block slot outside the imported consensus-inputs epoch
/// window — DC-ADMIT-11 / ¬P-C2.
pub const EXIT_LIVE_CROSS_EPOCH_USE: i32 = 32;
/// WAL append I/O fatal exit code.
pub const EXIT_LIVE_WAL_APPEND_IO: i32 = 33;
/// Peer sent bytes the BLUE decoder rejected with no peer tip at
/// the same slot — DC-ADMIT-12 / ¬P-C9. Reserved for C3; the C2
/// runner does not yet emit this exit code.
pub const EXIT_LIVE_PEER_SENT_UNDECODABLE: i32 = 34;

/// Closed exit-code sum. Maps to the binary's exit-code constants
/// (mirroring `wire_only::EXIT_LIVE_PASS_PEER_FAILURE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionExitCode {
    /// Clean shutdown (signal / upstream drop / channel closed).
    Ok,
    /// `AgreementVerdict::Diverged` observed.
    Diverged,
    /// `AgreementVerdict::InputNotFound` observed.
    InputNotFound,
    /// Peer block slot outside the imported consensus-inputs
    /// epoch window (DC-ADMIT-11).
    CrossEpochUse,
    /// `WalStore::append` returned a fatal I/O error.
    WalAppendIo,
    /// Peer sent undecodable bytes (DC-ADMIT-12, C3-wired).
    PeerSentUndecodableBytes,
}

impl AdmissionExitCode {
    /// Numeric exit code surfaced to the OS by the binary wrapper.
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::Diverged => EXIT_LIVE_AGREEMENT_DIVERGED,
            Self::InputNotFound => EXIT_LIVE_INPUT_NOT_FOUND,
            Self::CrossEpochUse => EXIT_LIVE_CROSS_EPOCH_USE,
            Self::WalAppendIo => EXIT_LIVE_WAL_APPEND_IO,
            Self::PeerSentUndecodableBytes => EXIT_LIVE_PEER_SENT_UNDECODABLE,
        }
    }
}

/// Closed peer-event sum delivered by the wire pump (or by the B6
/// hermetic loopback). All fields are typed; no free-text payload.
#[derive(Debug, Clone)]
pub enum AdmissionPeerEvent {
    /// A block arrived from the peer. The runner runs
    /// `admit_via_block_validity` then emits `BlockAdmitted` (on
    /// success) or `AgreementVerdict::Diverged` (on validity reject).
    Block { peer: String, block_bytes: Vec<u8> },
    /// The peer's chain-sync tip changed. Used as the comparison
    /// input by the next `verdict::derive` call.
    TipUpdate { peer: String, tip: Tip },
    /// Peer connection closed cleanly. Surfaced for clean shutdown
    /// when all peers have disconnected.
    Disconnected { peer: String },
}

/// Closed input bundle for [`run_admission`]. All fields are
/// required; no `Default` impl; no `#[non_exhaustive]`.
pub struct AdmissionInputs<'a, W, S>
where
    W: Write + Send + 'static,
    S: WalStore,
{
    pub writer: AdmissionLogWriter<W>,
    pub wal_store: S,
    pub anchor_initial_ledger_fp: Hash32,
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub era_schedule: &'a EraSchedule,
    pub ledger_view: &'a dyn LedgerView,
    pub peer_events: mpsc::Receiver<AdmissionPeerEvent>,
    pub shutdown: watch::Receiver<bool>,
    pub peer_count: u32,
    pub json_seed_path: String,
    pub wal_dir: String,
    pub initial_chain_tip_slot: u64,
    /// Canonical fingerprint of the imported LiveConsensusInputs
    /// (DC-CONS-IN-02). Bound into every BlockAdmitted /
    /// AgreementVerdict / BootstrapComplete / AdmissionStarted
    /// emit (DC-ADMIT-10) so the JSONL transcript is closed over
    /// the operator oracle.
    pub consensus_inputs_fingerprint: Hash32,
    /// Epoch the imported consensus-inputs pertain to. Used to
    /// label the cross-epoch guard's halt and is the only epoch
    /// the underlying [`LedgerView`] answers for (DC-VIEW-01).
    pub consensus_inputs_epoch: EpochNo,
    /// First slot of the imported epoch (inclusive). Peer blocks
    /// with slot `< this` are rejected pre-admit with
    /// [`AdmissionExitCode::CrossEpochUse`] (DC-ADMIT-11).
    pub consensus_inputs_epoch_start_slot: SlotNo,
    /// Last slot of the imported epoch (inclusive). Peer blocks
    /// with slot `> this` are rejected pre-admit with
    /// [`AdmissionExitCode::CrossEpochUse`] (DC-ADMIT-11).
    pub consensus_inputs_epoch_end_slot: SlotNo,
}

/// SOLE admission entry point (CN-ADMIT-01).
///
/// Drives the admission loop until one of:
///   - shutdown signal received → `Ok`
///   - all peers disconnected → `Ok`
///   - peer-event channel closed → `Ok`
///   - `AgreementVerdict::Diverged` observed → `Diverged`
///   - `AgreementVerdict::InputNotFound` observed → `InputNotFound`
///   - `WalStore::append` failed → `WalAppendIo`
pub async fn run_admission<W, S>(mut inputs: AdmissionInputs<'_, W, S>) -> AdmissionExitCode
where
    W: Write + Send + 'static,
    S: WalStore,
{
    let writer = Arc::new(Mutex::new(inputs.writer));
    let consensus_fp_hex = hex_lowercase(&inputs.consensus_inputs_fingerprint.0);

    emit(
        &writer,
        AdmissionLogEvent::AdmissionStarted {
            peer_count: inputs.peer_count,
            json_seed_path: inputs.json_seed_path.clone(),
            wal_dir: inputs.wal_dir.clone(),
            consensus_inputs_fingerprint_hex: consensus_fp_hex.clone(),
        },
    )
    .await;

    emit(
        &writer,
        AdmissionLogEvent::BootstrapComplete {
            initial_ledger_fp_hex: format!("{}", inputs.anchor_initial_ledger_fp),
            chain_tip_slot: inputs.initial_chain_tip_slot,
            consensus_inputs_fingerprint_hex: consensus_fp_hex.clone(),
        },
    )
    .await;

    // Latest known peer tip. The runner uses this for the next
    // verdict::derive call; updated by `TipUpdate` events.
    let mut latest_peer_tip: Tip = Tip {
        point: ade_network::codec::chain_sync::Point::Origin,
        block_no: 0,
    };

    // Tail post_fp for chain continuity (DC-WAL-02). Initialized
    // from the anchor; updated after each successful WAL append.
    let mut tail_post_fp = inputs.anchor_initial_ledger_fp.clone();

    // Mutable ledger / chain_dep traversed by each successful admit.
    let mut ledger = inputs.ledger;
    let mut chain_dep = inputs.chain_dep;

    // Connected peer count for clean-shutdown detection.
    let mut connected: u32 = inputs.peer_count;

    loop {
        tokio::select! {
            _ = inputs.shutdown.changed() => {
                if *inputs.shutdown.borrow() {
                    emit(
                        &writer,
                        AdmissionLogEvent::AdmissionShutdown {
                            reason: AdmissionShutdownReason::SignalReceived,
                        },
                    )
                    .await;
                    return AdmissionExitCode::Ok;
                }
            }
            event = inputs.peer_events.recv() => {
                match event {
                    None => {
                        emit(
                            &writer,
                            AdmissionLogEvent::AdmissionShutdown {
                                reason: AdmissionShutdownReason::UpstreamDropped,
                            },
                        )
                        .await;
                        return AdmissionExitCode::Ok;
                    }
                    Some(AdmissionPeerEvent::TipUpdate { tip, .. }) => {
                        latest_peer_tip = tip;
                    }
                    Some(AdmissionPeerEvent::Disconnected { .. }) => {
                        if connected > 0 {
                            connected -= 1;
                        }
                        if connected == 0 {
                            emit(
                                &writer,
                                AdmissionLogEvent::AdmissionShutdown {
                                    reason: AdmissionShutdownReason::UpstreamDropped,
                                },
                            )
                            .await;
                            return AdmissionExitCode::Ok;
                        }
                    }
                    Some(AdmissionPeerEvent::Block { peer, block_bytes }) => {
                        // C2 pre-admit epoch-window guard
                        // (DC-ADMIT-11). Decode just enough to
                        // recover the block slot; if it is
                        // outside the imported epoch window, halt
                        // immediately WITHOUT calling
                        // admit_via_block_validity (¬P-C2).
                        if let Ok(slot) = peek_block_slot(&block_bytes) {
                            if slot < inputs.consensus_inputs_epoch_start_slot
                                || slot > inputs.consensus_inputs_epoch_end_slot
                            {
                                let _ = peer;
                                emit(
                                    &writer,
                                    AdmissionLogEvent::AdmissionHalted {
                                        reason: AdmissionHaltReason::CrossEpochUse,
                                    },
                                )
                                .await;
                                return AdmissionExitCode::CrossEpochUse;
                            }
                        }

                        let outcome = process_block(
                            &block_bytes,
                            &ledger,
                            &chain_dep,
                            inputs.era_schedule,
                            inputs.ledger_view,
                        );

                        match outcome {
                            ProcessedBlock::Admitted {
                                slot,
                                block_hash,
                                prev_hash,
                                next_ledger,
                                next_chain_dep,
                            } => {
                                emit(
                                    &writer,
                                    AdmissionLogEvent::BlockReceived {
                                        peer: peer.clone(),
                                        slot: slot.0,
                                        block_hash_hex: hex_lowercase(&block_hash.0),
                                    },
                                )
                                .await;
                                let post_fp = fingerprint(&next_ledger).combined;
                                let entry = WalEntry::AdmitBlock {
                                    prior_fp: tail_post_fp.clone(),
                                    block_hash: block_hash.clone(),
                                    slot,
                                    verdict: BlockVerdictTag::Valid,
                                    post_fp: post_fp.clone(),
                                };
                                match inputs.wal_store.append(entry) {
                                    Ok(()) => {}
                                    Err(e) => {
                                        emit(
                                            &writer,
                                            AdmissionLogEvent::AdmissionHalted {
                                                reason: AdmissionHaltReason::WalAppendIo,
                                            },
                                        )
                                        .await;
                                        let _ = e;
                                        return AdmissionExitCode::WalAppendIo;
                                    }
                                }
                                tail_post_fp = post_fp.clone();
                                ledger = next_ledger;
                                chain_dep = next_chain_dep;

                                emit(
                                    &writer,
                                    AdmissionLogEvent::BlockAdmitted {
                                        slot: slot.0,
                                        block_hash_hex: hex_lowercase(&block_hash.0),
                                        prev_hash_hex: hex_lowercase(&prev_hash.0),
                                        post_fp_hex: format!("{}", post_fp),
                                        consensus_inputs_fingerprint_hex: consensus_fp_hex
                                            .clone(),
                                    },
                                )
                                .await;

                                let block_admit = BlockAdmitOutcome::Valid {
                                    slot,
                                    block_hash: block_hash.clone(),
                                    post_fp,
                                };
                                let verdict = derive_verdict(&block_admit, &latest_peer_tip);
                                emit_verdict(&writer, &verdict, &consensus_fp_hex).await;
                                if let Some(halt) = halt_for_verdict(&verdict) {
                                    emit(
                                        &writer,
                                        AdmissionLogEvent::AdmissionHalted { reason: halt },
                                    )
                                    .await;
                                    return halt_to_exit(halt);
                                }
                            }
                            ProcessedBlock::Invalid {
                                slot,
                                block_hash,
                                reason,
                            } => {
                                emit(
                                    &writer,
                                    AdmissionLogEvent::BlockReceived {
                                        peer: peer.clone(),
                                        slot: slot.0,
                                        block_hash_hex: hex_lowercase(&block_hash.0),
                                    },
                                )
                                .await;
                                let block_admit = BlockAdmitOutcome::Invalid {
                                    slot,
                                    block_hash,
                                    reason,
                                };
                                let verdict = derive_verdict(&block_admit, &latest_peer_tip);
                                emit_verdict(&writer, &verdict, &consensus_fp_hex).await;
                                if let Some(halt) = halt_for_verdict(&verdict) {
                                    emit(
                                        &writer,
                                        AdmissionLogEvent::AdmissionHalted { reason: halt },
                                    )
                                    .await;
                                    return halt_to_exit(halt);
                                }
                            }
                            ProcessedBlock::Undecodable => {
                                // C3 strengthening of N-M-B's
                                // silent-clean-exit path
                                // (DC-ADMIT-12 / ¬P-C9). The peer
                                // fed us bytes the Conway BLUE
                                // decoder rejected. Adversarial by
                                // default:
                                //   - If a peer tip exists at any
                                //     slot, emit Diverged at that
                                //     slot (we couldn't decode, so
                                //     hashes can't be compared —
                                //     surface as Diverged with our
                                //     hash = zero-hash sentinel +
                                //     peer hash from tip).
                                //   - If no peer tip exists yet,
                                //     emit
                                //     AdmissionHalted {
                                //       reason: PeerSentUndecodableBytes
                                //     }.
                                // In NO case do we return Ok or
                                // emit InputNotFound (¬P-C7).
                                match &latest_peer_tip.point {
                                    ade_network::codec::chain_sync::Point::Block {
                                        slot: peer_slot,
                                        hash: peer_hash,
                                    } => {
                                        let verdict = AgreementVerdict::Diverged {
                                            slot: *peer_slot,
                                            our_hash: Hash32([0u8; 32]),
                                            peer_hash: peer_hash.clone(),
                                        };
                                        emit_verdict(&writer, &verdict, &consensus_fp_hex)
                                            .await;
                                        emit(
                                            &writer,
                                            AdmissionLogEvent::AdmissionHalted {
                                                reason: AdmissionHaltReason::Diverged,
                                            },
                                        )
                                        .await;
                                        return AdmissionExitCode::Diverged;
                                    }
                                    ade_network::codec::chain_sync::Point::Origin => {
                                        emit(
                                            &writer,
                                            AdmissionLogEvent::AdmissionHalted {
                                                reason:
                                                    AdmissionHaltReason::PeerSentUndecodableBytes,
                                            },
                                        )
                                        .await;
                                        return AdmissionExitCode::PeerSentUndecodableBytes;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Processed outcome of `admit_via_block_validity` over peer bytes.
enum ProcessedBlock {
    Admitted {
        slot: SlotNo,
        block_hash: Hash32,
        /// The admitted block's validated header `prev_hash` (parent linkage;
        /// genesis predecessor as the all-zero hash) — capture-only fidelity for
        /// the post-switch branch-continuity verdict (S10, DC-EVIDENCE-05).
        prev_hash: Hash32,
        next_ledger: LedgerState,
        next_chain_dep: PraosChainDepState,
    },
    Invalid {
        slot: SlotNo,
        block_hash: Hash32,
        reason: InvalidAdmitReason,
    },
    Undecodable,
}

fn process_block(
    block_bytes: &[u8],
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> ProcessedBlock {
    // The BlockFetch protocol delivers each block's body as the
    // FULL wrapped CBOR item `tag(24, bytes(.cbor [era_tag,
    // era_block]))`. `decode_block` expects the UNWRAPPED inner
    // bytes (the `[era_tag, era_block]` array). Strip the tag-24
    // envelope before handing to the BLUE decoder. (PHASE4-N-M-FRAG
    // surfaced this: prior to the FRAG fix the wire pump exited
    // before any block reached `decode_block`, so the missing
    // unwrap was masked.)
    // CN-WIRE-08: strip the BlockFetch tag-24 CBOR-in-CBOR envelope via
    // the single shared ade_codec authority (no hand-rolled tag-24 parse
    // in RED). `unwrap_tag24` returns a zero-copy borrow of the inner
    // `[era, block]` storage bytes that `decode_block` consumes.
    let inner = match ade_codec::unwrap_tag24(block_bytes) {
        Ok(b) => b,
        Err(_) => return ProcessedBlock::Undecodable,
    };
    let decoded = match decode_block(inner) {
        Ok(d) => d,
        Err(_) => return ProcessedBlock::Undecodable,
    };
    let slot = decoded.header_input.slot;
    let block_hash = decoded.block_hash.clone();
    let prev_hash = decoded
        .prev_hash
        .block_hash()
        .cloned()
        .unwrap_or(Hash32([0; 32]));
    match admit_via_block_validity(inner, ledger, chain_dep, era_schedule, ledger_view) {
        Ok(out) => ProcessedBlock::Admitted {
            slot,
            block_hash,
            prev_hash,
            next_ledger: out.ledger,
            next_chain_dep: out.chain_dep,
        },
        Err(e) => {
            // PHASE4-N-M-FRAG operator diagnostic.
            // Surface the typed validity error so the operator
            // can see WHICH stage of admit_via_block_validity
            // rejected the block. Stays in RED scope.
            eprintln!(
                "admission_admit_rejected: slot={} block_hash={} error={e:?}",
                slot.0,
                hex_lowercase(&block_hash.0),
            );
            ProcessedBlock::Invalid {
                slot,
                block_hash,
                reason: classify_validity_error(&e),
            }
        }
    }
}

fn classify_validity_error(e: &BlockValidityError) -> InvalidAdmitReason {
    match e {
        BlockValidityError::Header(_) => InvalidAdmitReason::Header,
        BlockValidityError::HeaderPositionInvalid { .. } => InvalidAdmitReason::Header,
        BlockValidityError::Body(_) => InvalidAdmitReason::Body,
        BlockValidityError::MalformedField(_) => InvalidAdmitReason::MalformedField,
        BlockValidityError::BodyHashMismatch { .. } => InvalidAdmitReason::BodyHashMismatch,
        BlockValidityError::MissingConsensusInput(_) => InvalidAdmitReason::Body,
    }
}

fn halt_for_verdict(v: &AgreementVerdict) -> Option<AdmissionHaltReason> {
    match v {
        AgreementVerdict::Diverged { .. } => Some(AdmissionHaltReason::Diverged),
        AgreementVerdict::InputNotFound { .. } => Some(AdmissionHaltReason::InputNotFound),
        AgreementVerdict::Agreed { .. } | AgreementVerdict::Lagging { .. } => None,
    }
}

fn halt_to_exit(reason: AdmissionHaltReason) -> AdmissionExitCode {
    match reason {
        AdmissionHaltReason::Diverged => AdmissionExitCode::Diverged,
        AdmissionHaltReason::InputNotFound => AdmissionExitCode::InputNotFound,
        AdmissionHaltReason::WalAppendIo => AdmissionExitCode::WalAppendIo,
        AdmissionHaltReason::BootstrapFatal => AdmissionExitCode::Ok,
        AdmissionHaltReason::CrossEpochUse => AdmissionExitCode::CrossEpochUse,
        AdmissionHaltReason::PeerSentUndecodableBytes => {
            AdmissionExitCode::PeerSentUndecodableBytes
        }
    }
}

async fn emit_verdict<W: Write + Send + 'static>(
    writer: &Arc<Mutex<AdmissionLogWriter<W>>>,
    v: &AgreementVerdict,
    consensus_fp_hex: &str,
) {
    let kind = verdict_kind(v);
    let (slot, our_h, peer_h, peer_slot, tx_in) = match v {
        AgreementVerdict::Agreed { slot, hash } => (
            slot.0,
            Some(hex_lowercase(&hash.0)),
            Some(hex_lowercase(&hash.0)),
            None,
            None,
        ),
        AgreementVerdict::Lagging { our_slot, peer_slot } => {
            (our_slot.0, None, None, Some(peer_slot.0), None)
        }
        AgreementVerdict::Diverged {
            slot,
            our_hash,
            peer_hash,
        } => (
            slot.0,
            Some(hex_lowercase(&our_hash.0)),
            Some(hex_lowercase(&peer_hash.0)),
            None,
            None,
        ),
        AgreementVerdict::InputNotFound { tx_in_hex } => (0, None, None, None, Some(tx_in_hex.clone())),
    };
    emit(
        writer,
        AdmissionLogEvent::AgreementVerdict {
            kind,
            slot,
            our_hash_hex: our_h,
            peer_hash_hex: peer_h,
            peer_slot,
            tx_in_hex: tx_in,
            consensus_inputs_fingerprint_hex: consensus_fp_hex.to_string(),
        },
    )
    .await;
}

/// Decode just enough of a peer's block CBOR to recover the slot
/// number, for the C2 pre-admit epoch-window guard
/// (DC-ADMIT-11). Returns the slot on success; returns an empty
/// error on any decode failure (the runner does not branch on the
/// decode-error class — undecodable bytes fall through to the
/// normal `process_block` path, where C3 tightens the handling).
fn peek_block_slot(block_bytes: &[u8]) -> Result<SlotNo, ()> {
    decode_block(block_bytes)
        .map(|d| d.header_input.slot)
        .map_err(|_| ())
}

async fn emit<W: Write + Send + 'static>(
    writer: &Arc<Mutex<AdmissionLogWriter<W>>>,
    event: AdmissionLogEvent,
) {
    let mut w = writer.lock().await;
    let _ = w.emit(&event);
}

fn hex_lowercase(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xF) as usize] as char);
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::ledger_view::LedgerView;
    use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
    use ade_ledger::wal::{WalEntry, WalError};
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    /// An in-memory WalStore for unit tests.
    struct VecWalStore {
        entries: Vec<WalEntry>,
    }
    impl VecWalStore {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }
    }
    impl WalStore for VecWalStore {
        fn append(&mut self, entry: WalEntry) -> Result<(), WalError> {
            self.entries.push(entry);
            Ok(())
        }
        fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
            Ok(self.entries.clone())
        }
    }

    /// Minimal LedgerView for tests. The runner tests never feed
    /// real block bytes through `admit_via_block_validity`, so the
    /// view's methods return `None` — they're never called.
    struct NoopLedgerView;
    impl LedgerView for NoopLedgerView {
        fn total_active_stake(&self, _epoch: EpochNo) -> Option<u64> {
            None
        }
        fn pool_active_stake(&self, _epoch: EpochNo, _pool: &Hash28) -> Option<u64> {
            None
        }
        fn pool_vrf_keyhash(&self, _epoch: EpochNo, _pool: &Hash28) -> Option<Hash32> {
            None
        }
        fn active_slots_coeff(
            &self,
            _epoch: EpochNo,
        ) -> Option<ade_core::consensus::vrf_cert::ActiveSlotsCoeff> {
            None
        }
    }

    fn make_schedule() -> EraSchedule {
        EraSchedule::new(
            ade_core::consensus::BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![ade_core::consensus::EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 432_000,
            }],
        )
        .expect("schedule")
    }

    fn make_inputs<'a>(
        peer_events: mpsc::Receiver<AdmissionPeerEvent>,
        shutdown: watch::Receiver<bool>,
        schedule: &'a EraSchedule,
        view: &'a NoopLedgerView,
    ) -> AdmissionInputs<'a, Vec<u8>, VecWalStore> {
        AdmissionInputs {
            writer: AdmissionLogWriter::new(Vec::<u8>::new()),
            wal_store: VecWalStore::new(),
            anchor_initial_ledger_fp: Hash32([0xAA; 32]),
            ledger: LedgerState::new(CardanoEra::Conway),
            chain_dep: PraosChainDepState::genesis(Nonce::ZERO),
            era_schedule: schedule,
            ledger_view: view,
            peer_events,
            shutdown,
            peer_count: 1,
            json_seed_path: "/seed.json".into(),
            wal_dir: "/wal".into(),
            initial_chain_tip_slot: 0,
            consensus_inputs_fingerprint: Hash32([0xCC; 32]),
            consensus_inputs_epoch: EpochNo(0),
            consensus_inputs_epoch_start_slot: SlotNo(0),
            consensus_inputs_epoch_end_slot: SlotNo(u64::MAX),
        }
    }

    #[tokio::test]
    async fn run_admission_emits_shutdown_on_signal() {
        let (_tx, rx) = mpsc::channel::<AdmissionPeerEvent>(8);
        let (sh_tx, sh_rx) = watch::channel(false);
        let schedule = make_schedule();
        let view = NoopLedgerView;
        let inputs = make_inputs(rx, sh_rx, &schedule, &view);
        // Schedule the shutdown signal on the executor; await the
        // runner inline (avoids the `'static` bound `tokio::spawn`
        // would require for the `&schedule` / `&view` references).
        let _signaler = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let _ = sh_tx.send(true);
        });
        let exit = run_admission(inputs).await;
        assert_eq!(exit, AdmissionExitCode::Ok);
    }

    #[tokio::test]
    async fn run_admission_emits_shutdown_on_channel_close() {
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(8);
        let (_sh_tx, sh_rx) = watch::channel(false);
        let schedule = make_schedule();
        let view = NoopLedgerView;
        let inputs = make_inputs(rx, sh_rx, &schedule, &view);
        drop(tx);
        let exit = run_admission(inputs).await;
        assert_eq!(exit, AdmissionExitCode::Ok);
    }

    #[tokio::test]
    async fn run_admission_disconnect_to_zero_peers_clean_exit() {
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(8);
        let (_sh_tx, sh_rx) = watch::channel(false);
        let schedule = make_schedule();
        let view = NoopLedgerView;
        let inputs = make_inputs(rx, sh_rx, &schedule, &view);
        tx.send(AdmissionPeerEvent::Disconnected {
            peer: "p".into(),
        })
        .await
        .unwrap();
        let exit = run_admission(inputs).await;
        assert_eq!(exit, AdmissionExitCode::Ok);
    }

    #[test]
    fn exit_code_constants_round_trip_to_i32() {
        assert_eq!(
            AdmissionExitCode::Diverged.as_i32(),
            EXIT_LIVE_AGREEMENT_DIVERGED
        );
        assert_eq!(
            AdmissionExitCode::InputNotFound.as_i32(),
            EXIT_LIVE_INPUT_NOT_FOUND
        );
        assert_eq!(
            AdmissionExitCode::WalAppendIo.as_i32(),
            EXIT_LIVE_WAL_APPEND_IO
        );
        assert_eq!(AdmissionExitCode::Ok.as_i32(), 0);
    }

    #[test]
    fn halt_for_verdict_only_diverged_or_input_not_found_halts() {
        assert!(halt_for_verdict(&AgreementVerdict::Agreed {
            slot: SlotNo(0),
            hash: Hash32([0u8; 32])
        })
        .is_none());
        assert!(halt_for_verdict(&AgreementVerdict::Lagging {
            our_slot: SlotNo(0),
            peer_slot: SlotNo(0)
        })
        .is_none());
        assert_eq!(
            halt_for_verdict(&AgreementVerdict::Diverged {
                slot: SlotNo(0),
                our_hash: Hash32([0u8; 32]),
                peer_hash: Hash32([0u8; 32])
            }),
            Some(AdmissionHaltReason::Diverged)
        );
        assert_eq!(
            halt_for_verdict(&AgreementVerdict::InputNotFound {
                tx_in_hex: "x".into()
            }),
            Some(AdmissionHaltReason::InputNotFound)
        );
    }
}
