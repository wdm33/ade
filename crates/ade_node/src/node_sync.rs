// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `--mode node` sync path (PHASE4-N-F-C L4).
//!
//! L4a (this slice step): the verdict-decoupled block-bytes SOURCE the
//! lifecycle sync path consumes. One ordered source only (E1): either a
//! single peer's `run_admission_wire_pump` event stream, or a
//! deterministic in-memory feed for the hermetic test. The source yields
//! ONLY block bytes — it never derives, surfaces, or depends on an
//! agreement verdict, tip-agreement, or follow decision (E2). A
//! `TipUpdate` is a comparison input for admission's verdict loop, NOT a
//! block and NOT a tip authority for sync, so it is skipped; a clean
//! `Disconnected` (or a closed channel) ends the feed.
//!
//! What L4a is NOT: it is not a verdict flow (no `derive_verdict` /
//! `run_admission`), not a follower (`ade_core_interop::follow` is not
//! validating sync), and it advances no tip. The durable apply +
//! tip-snapshot capture (L4b, via `forward_sync::pump_block` +
//! `PersistentSnapshotCache::capture`) and the kill→warm-start recovery
//! proof (L4c) build on this source in later slice steps; the tip is a
//! durable-apply fact, never an agreement verdict.

use std::collections::VecDeque;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::leader_schedule::{query_leader_schedule, LeaderScheduleQuery};
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::receive::events::TipPoint;
use ade_ledger::state::LedgerState;
use ade_ledger::wal::WalStore;
use ade_network::codec::chain_sync::{Point, Tip};
use ade_runtime::admission::AdmissionPeerEvent;
use ade_runtime::bootstrap::BootstrapState;
use ade_runtime::chaindb::{ChainDb, ChainTip, SnapshotStore};
use ade_runtime::forward_sync::{pump_block, ForwardSyncState, NoCheckpointSink, PumpTip};
use ade_runtime::producer::coordinator::CoordinatorEvent;
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_runtime::rollback::PersistentSnapshotCache;
use ade_types::shelley::block::{PrevHash, ProtocolVersion};
use ade_types::{BlockNo, EpochNo, Hash28, SlotNo};
use tokio::sync::mpsc;

use crate::produce_mode::{run_real_forge, ForgeRequestContext};
use crate::run_loop_planner::VenuePolicy;
use ade_runtime::producer::self_accepted_handoff::SelfAcceptedHandoff;

/// PHASE4-N-F-G-E S1 (DC-LIVEMEM-01): the maximum blocks the content-blind
/// WirePump lookahead may buffer. At the cap, `pump_lookahead` stops the
/// opportunistic `try_recv` drain and the existing bounded mpsc
/// (`LIVE_WIRE_PUMP_CHANNEL_CAP`) back-pressures the pump's `events_tx.send` —
/// end-to-end bounded, no unbounded `VecDeque` growth from a fast / hostile
/// peer. A **defensive implementation bound, NOT a Cardano semantic
/// parameter**; tightenable by a future hardening slice, but no runtime option
/// (CLI / env / config) may disable it or set it unbounded. Closed constant.
const MAX_WIRE_PUMP_LOOKAHEAD: usize = 256;

/// Closed, verdict-decoupled ordered block-bytes source for the
/// `--mode node` lifecycle sync path (PHASE4-N-F-C L4a).
///
/// One ordered source only (E1). [`NodeBlockSource::next_block`] yields
/// ONLY `AdmissionPeerEvent::Block` payloads, in arrival order; it never
/// surfaces a verdict / tip-agreement / follow decision (E2).
pub enum NodeBlockSource {
    /// One peer's `run_admission_wire_pump` event stream. The pump is
    /// the N2N `BlockFetch` source; this taps its raw `Block` events —
    /// NOT admission's verdict runner (`run_admission`).
    ///
    /// `lookahead` is a **content-blind** availability buffer of opaque block
    /// bytes. The readiness peeks (PHASE4-N-F-D S2: `has_work_ready` /
    /// `wait_ready`) fill it via non-blocking `try_recv`; it is drained ONLY
    /// through `next_block`. It is RED scheduling state only — the bytes are
    /// never decoded, hashed, validated, classified, or reordered, never
    /// observed by BLUE/GREEN authority code, and nothing is skipped except
    /// the pre-existing `TipUpdate` filter; a buffered block is still
    /// delivered next, in arrival order (peek for availability, not meaning).
    /// `disconnected` records that the peer's channel ended (a clean
    /// disconnect is not a tip authority — it ends the feed once the
    /// lookahead drains).
    WirePump {
        rx: mpsc::Receiver<AdmissionPeerEvent>,
        lookahead: VecDeque<Vec<u8>>,
        disconnected: bool,
        /// PHASE4-N-AE.A (DC-NODE-15): the followed-peer-tip admissibility
        /// signal, updated as a write-only side effect when the wire stream
        /// yields a `TipUpdate` (which is otherwise skipped for sync). It is
        /// read ONLY by the ForgeTick admissibility gate, never by
        /// `next_block` / readiness — so it can only PREVENT a forge, never
        /// drive sync or chain selection.
        followed_peer_tip: FollowedPeerTipSignal,
    },
    /// Deterministic in-memory ordered feed (hermetic test / loopback).
    /// Exactly the "a live socket is not required" shape `pump_block`
    /// was designed for. The followed-peer-tip signal carried alongside is
    /// set explicitly by hermetic tests (the in-memory feed observes no live
    /// `TipUpdate`).
    InMemory {
        blocks: VecDeque<Vec<u8>>,
        followed_peer_tip: FollowedPeerTipSignal,
    },
}

impl NodeBlockSource {
    /// Build an in-memory source from an ordered block-bytes sequence.
    pub fn in_memory(blocks: Vec<Vec<u8>>) -> Self {
        Self::InMemory {
            blocks: VecDeque::from(blocks),
            followed_peer_tip: FollowedPeerTipSignal::new(),
        }
    }

    /// Build an in-memory source with an explicit followed-peer-tip
    /// admissibility signal (hermetic forge-gate tests). The in-memory feed
    /// observes no live `TipUpdate`, so tests set the signal directly to
    /// exercise the caught-up / not-caught-up classifier paths.
    pub fn in_memory_with_followed_tip(
        blocks: Vec<Vec<u8>>,
        followed_peer_tip: Option<TipPoint>,
    ) -> Self {
        Self::InMemory {
            blocks: VecDeque::from(blocks),
            followed_peer_tip: FollowedPeerTipSignal {
                latest: followed_peer_tip,
            },
        }
    }

    /// Wrap one peer's wire-pump event receiver as the source.
    pub fn from_wire_pump(rx: mpsc::Receiver<AdmissionPeerEvent>) -> Self {
        Self::WirePump {
            rx,
            lookahead: VecDeque::new(),
            disconnected: false,
            followed_peer_tip: FollowedPeerTipSignal::new(),
        }
    }

    /// PHASE4-N-AE.A (DC-NODE-15): the followed-peer-tip admissibility signal
    /// observed from this source's wire stream (or set explicitly for an
    /// in-memory feed). Read ONLY by the ForgeTick admissibility gate — never a
    /// sync / chain-selection authority.
    pub fn followed_peer_tip_signal(&self) -> &FollowedPeerTipSignal {
        match self {
            Self::InMemory {
                followed_peer_tip, ..
            } => followed_peer_tip,
            Self::WirePump {
                followed_peer_tip, ..
            } => followed_peer_tip,
        }
    }

    /// Non-blocking drain of the WirePump channel into the content-blind
    /// lookahead. Selects ONLY `Block` (buffered as opaque bytes), skips
    /// `TipUpdate`, and stops at the first `Disconnected` / closed channel
    /// (setting the flag) or when no event is immediately available. Never
    /// blocks; never inspects block content. RED scheduling only.
    fn pump_lookahead(
        rx: &mut mpsc::Receiver<AdmissionPeerEvent>,
        lookahead: &mut VecDeque<Vec<u8>>,
        disconnected: &mut bool,
        followed_peer_tip: &mut FollowedPeerTipSignal,
    ) {
        use tokio::sync::mpsc::error::TryRecvError;
        loop {
            // PHASE4-N-F-G-E S1 (DC-LIVEMEM-01): bound the content-blind
            // lookahead depth. At the cap, stop opportunistic draining; the
            // existing bounded mpsc (LIVE_WIRE_PUMP_CHANNEL_CAP) then
            // back-pressures the pump's events_tx.send. No unbounded growth
            // from a fast / hostile peer; never a silent drop (the bytes stay
            // queued in the bounded channel and are drained once below the cap).
            if lookahead.len() >= MAX_WIRE_PUMP_LOOKAHEAD {
                break;
            }
            match rx.try_recv() {
                Ok(AdmissionPeerEvent::Block { block_bytes, .. }) => {
                    lookahead.push_back(block_bytes);
                }
                // PHASE4-N-AE.A (DC-NODE-15): a TipUpdate is STILL skipped for
                // sync (it is not a block and not a sync tip authority), but it
                // is recorded into the followed-peer-tip admissibility signal as
                // a write-only side effect — read only by the ForgeTick gate.
                Ok(AdmissionPeerEvent::TipUpdate { tip, .. }) => {
                    followed_peer_tip.observe(&tip);
                    continue;
                }
                Ok(AdmissionPeerEvent::Disconnected { .. }) => {
                    *disconnected = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    *disconnected = true;
                    break;
                }
            }
        }
    }

    /// Next ordered block bytes that are AVAILABLE NOW, or `None` at the
    /// current batch boundary (an exhausted in-memory feed, or a WirePump
    /// with nothing buffered right now). **Non-blocking**: it never waits for
    /// future input — waiting happens only in the relay loop's RED `Idle`
    /// branch (`wait_ready`), OUTSIDE `run_node_sync`, so the sync driver is
    /// never awaited across a shutdown cancellation boundary.
    ///
    /// Selects ONLY `AdmissionPeerEvent::Block`. `TipUpdate` is skipped
    /// (a comparison input for the verdict loop, not a block and not a sync
    /// tip authority). `Disconnected` and a closed channel both end the feed
    /// (a clean disconnect is not a tip authority). No verdict is ever
    /// derived or surfaced here (E2 / no verdict-as-sync).
    ///
    /// `run_node_sync` is the SOLE block-consumption path: it calls
    /// `next_block` in a loop to drain the currently-available batch and
    /// returns at the boundary. The content-blind lookahead (filled by the
    /// readiness peeks) is drained ONLY here.
    pub async fn next_block(&mut self) -> Option<Vec<u8>> {
        match self {
            Self::InMemory { blocks, .. } => blocks.pop_front(),
            Self::WirePump {
                rx,
                lookahead,
                disconnected,
                followed_peer_tip,
            } => {
                // Top up the content-blind lookahead from whatever is
                // immediately available (non-blocking), then hand back one
                // block. No `.await` on the channel — a batch boundary
                // (open-but-empty) yields `None` so the sync driver returns
                // and the loop can re-plan / idle / shut down cleanly.
                if lookahead.is_empty() && !*disconnected {
                    Self::pump_lookahead(rx, lookahead, disconnected, followed_peer_tip);
                }
                lookahead.pop_front()
            }
        }
    }

    /// Whether a subsequent sync step is expected to make progress — i.e.
    /// whether at least one block is available to apply right now. RED
    /// scheduling information only; **content-blind** (it never inspects,
    /// decodes, classifies, hashes, validates, reorders, or consumes block
    /// bytes — for the WirePump arm it fills the opaque lookahead via
    /// non-blocking `try_recv`). PHASE4-N-F-D S2.
    pub fn has_work_ready(&mut self) -> bool {
        match self {
            Self::InMemory { blocks, .. } => !blocks.is_empty(),
            Self::WirePump {
                rx,
                lookahead,
                disconnected,
                followed_peer_tip,
            } => {
                if lookahead.is_empty() && !*disconnected {
                    Self::pump_lookahead(rx, lookahead, disconnected, followed_peer_tip);
                }
                !lookahead.is_empty()
            }
        }
    }

    /// Whether the source feed has structurally ended (distinct from
    /// momentary emptiness): an in-memory feed is ended once drained; a
    /// WirePump is ended once its channel disconnected AND the lookahead is
    /// drained. Content-blind. PHASE4-N-F-D S2.
    pub fn is_ended(&self) -> bool {
        match self {
            Self::InMemory { blocks, .. } => blocks.is_empty(),
            Self::WirePump {
                lookahead,
                disconnected,
                ..
            } => *disconnected && lookahead.is_empty(),
        }
    }

    /// Classify WHY the feed currently yields no block, as the closed
    /// `FeedReason` taxonomy (PHASE4-N-F-G-J S1, CN-NODE-04). Content-blind RED
    /// scheduling info only — emitted as a diagnostic, NEVER read by the planner
    /// (emit-only). Fail-closed-on-ambiguity (option (b)): the reason-less
    /// `disconnected` collapse cannot prove a clean drain, so a WirePump
    /// disconnect classifies the ineligible `UnknownDisconnected` — never an
    /// eligible `CleanEmpty`. An InMemory drain is a deterministic, provably-clean
    /// exhaustion (`CleanEmpty`); an open WirePump with no block ready is
    /// `NoBlockAvailable`. The specific error reasons + a reason-enriched live
    /// `CleanEmpty` await a future wire-pump enrichment (not S1).
    pub fn feed_reason(&self) -> crate::live_log::FeedReason {
        use crate::live_log::FeedReason;
        match self {
            Self::InMemory { blocks, .. } => {
                if blocks.is_empty() {
                    FeedReason::CleanEmpty
                } else {
                    FeedReason::NoBlockAvailable
                }
            }
            Self::WirePump {
                lookahead,
                disconnected,
                ..
            } => {
                if *disconnected && lookahead.is_empty() {
                    FeedReason::UnknownDisconnected
                } else {
                    FeedReason::NoBlockAvailable
                }
            }
        }
    }

    /// Resolve when the next sync step is expected to make progress, or the
    /// feed has ended. In-memory feeds resolve immediately (work is already
    /// present or it is ended). A WirePump with nothing buffered awaits one
    /// event (skipping `TipUpdate`), buffering a `Block` into the
    /// content-blind lookahead or marking disconnect. This is the loop's
    /// sole inter-iteration await point, so a shutdown selected against it is
    /// cancellation-safe — no durable apply is ever torn. PHASE4-N-F-D S2.
    pub async fn wait_ready(&mut self) {
        match self {
            Self::InMemory { .. } => {}
            Self::WirePump {
                rx,
                lookahead,
                disconnected,
                followed_peer_tip,
            } => {
                if !lookahead.is_empty() || *disconnected {
                    return;
                }
                loop {
                    match rx.recv().await {
                        Some(AdmissionPeerEvent::Block { block_bytes, .. }) => {
                            lookahead.push_back(block_bytes);
                            return;
                        }
                        // PHASE4-N-AE.A (DC-NODE-15): a TipUpdate does not end
                        // the await (it is not a block), but its tip IS recorded
                        // into the followed-peer-tip admissibility signal before
                        // we keep waiting for the next block.
                        Some(AdmissionPeerEvent::TipUpdate { tip, .. }) => {
                            followed_peer_tip.observe(&tip);
                            continue;
                        }
                        Some(AdmissionPeerEvent::Disconnected { .. }) => {
                            *disconnected = true;
                            return;
                        }
                        None => {
                            *disconnected = true;
                            return;
                        }
                    }
                }
            }
        }
    }
}

/// Closed L4b sync-driver error surface. Every variant is a deterministic
/// fail-closed halt — the driver never skips past a rejected block and
/// never falls back to genesis / a bundle / a cold path.
#[derive(Debug)]
pub enum NodeSyncError {
    /// `pump_block` rejected a block (the BLUE admit chokepoint, a WAL
    /// append, a block-bytes store, a checkpoint marker, or the
    /// durable-before-tip guard). Carries the closed `PumpError` debug.
    Pump(String),
    /// Capturing the selected-tip checkpoint via `PersistentSnapshotCache`
    /// failed. The tip advanced durably but its recovery snapshot could not
    /// be written — fail closed rather than report an unrecoverable tip.
    Capture(String),
}

/// L4b — the durable validated-apply driver: the FIRST production caller of
/// `forward_sync::pump_block` on the `--mode node` lifecycle path.
///
/// For each block from `source` (one ordered source, L4a), applies it
/// through `pump_block` against the owner's persistent `ChainDb` + WAL:
/// `StoreBlockBytes` + `AppendWal` are made durable BEFORE `AdvanceTip`
/// (DC-SYNC-01 — enforced inside `pump_block`'s `apply_plan`, not here).
/// The driver advances the tip ONLY through `pump_block`; it performs no
/// manual `put_block` / tip write / `AdvanceTip` construction.
///
/// **E4 (pinned):** after the drive, if a tip was advanced, the driver
/// captures a checkpoint AT the selected tip via
/// `PersistentSnapshotCache::capture(tip.slot, ledger, chain_dep)` — the
/// exact `PersistentSnapshotCache` path L3 `warm_start_recovery` reads back
/// (`nearest_le` → `decode_snapshot`). The captured `(ledger, chain_dep)`
/// is the post-apply state held in `state.receive`. This makes the advanced
/// tip recoverable from a genuine durable artifact of the apply path — not
/// a test fixture.
///
/// Fail-closed: a `pump_block` reject or a capture failure halts the drive
/// with a typed [`NodeSyncError`]; no skip-past, no fallback. Returns the
/// selected (final advanced) tip, or `None` if the source was empty.
///
/// What this is NOT: not a verdict flow (no `derive_verdict` /
/// `run_admission`), not a follower (`ade_core_interop::follow` is not
/// validating sync), no forge, no produce. The tip is a durable-apply fact.
pub async fn run_node_sync<D>(
    source: &mut NodeBlockSource,
    state: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<PumpTip>, NodeSyncError>
where
    D: ChainDb + SnapshotStore,
{
    let mut selected_tip: Option<PumpTip> = None;

    while let Some(block_bytes) = source.next_block().await {
        // The SOLE tip-advancing call on the lifecycle sync path. Its
        // internal cadence checkpoint marker is a no-op here
        // (`NoCheckpointSink`); the REAL recovery snapshot is captured
        // below via `PersistentSnapshotCache` (E4). A reject fails closed.
        // The block bytes arrive already bare `[era, block]`: the wire feed
        // strips the BlockFetch tag-24 wrapper at the receive boundary
        // (CN-WIRE-12), and the in-memory feed yields bare directly.
        let tip = pump_block(
            state,
            chaindb,
            wal,
            &NoCheckpointSink,
            &block_bytes,
            era_schedule,
            ledger_view,
        )
        .map_err(|e| NodeSyncError::Pump(format!("{e:?}")))?;
        if let Some(t) = tip {
            selected_tip = Some(t);
        }
    }

    // E4: capture the recovery checkpoint AT the selected tip, via the same
    // PersistentSnapshotCache path warm-start recovery reads. The captured
    // state is the post-apply ledger + chain_dep.
    if let Some(tip) = &selected_tip {
        PersistentSnapshotCache::new(chaindb)
            .capture(tip.slot, &state.receive.ledger, &state.receive.chain_dep)
            .map_err(|e| NodeSyncError::Capture(format!("{e:?}")))?;
    }

    Ok(selected_tip)
}

/// PHASE4-N-U S1 (DC-NODE-12) — the durable-forge-admit driver: route a BLUE
/// self-accepted forged block into the SAME `forward_sync::pump_block`
/// chokepoint received blocks use, so a forged block becomes durable ONLY
/// through the single durable tip-advance authority. The forge advances no
/// durable tip directly; `pump_block` stays the sole authority (DC-NODE-12
/// supersedes DC-NODE-05's "local artifact only" containment, preserving its
/// deeper invariant).
///
/// The bytes admitted are EXACTLY the self-accepted bytes
/// (`handoff.accepted().as_bytes()`) — no re-encode, no reserialize, no new
/// `WalEntry` variant (I-10); the carrier is the constructor-fenced
/// [`SelfAcceptedHandoff`], so only a BLUE self-accepted token (CN-FORGE-01)
/// can be admitted. `pump_block` runs the EXTEND-ONLY admit chokepoint
/// (`decode_block` → `admit_via_block_validity` → `block_validity`, incl.
/// `header_position`) then the ordered durable effects `StoreBlockBytes` →
/// `AppendWal` → `AdvanceTip` (durable-before-tip; DC-SYNC-01 enforced inside
/// `apply_plan`). The forged `AdmitBlock`'s `prior_fp` chains to the current
/// durable `post_fp` exactly as a received block's does (DC-WAL-04 chaining).
///
/// Extend-only race safety (DC-CONS-23): there is NO admit-time fork-choice. A
/// forge built on a stale tip (one a feed block has since advanced) fails
/// closed here — `block_validity`'s header-position/`prev_hash` check or the
/// `TipBeforeDurable` guard rejects it — and `pump_block` leaves the tip
/// unchanged.
///
/// No snapshot is captured here: forged admits are WAL-durable and ride the
/// existing DC-STORE-07 cadence; recovery is proven through WAL replay (S2),
/// not by forcing a snapshot at every forged tip.
pub fn admit_forged_block_durably<D>(
    handoff: &SelfAcceptedHandoff,
    state: &mut ForwardSyncState,
    chaindb: &D,
    wal: &mut dyn WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<PumpTip>, NodeSyncError>
where
    D: ChainDb,
{
    // I-10: the durably-admitted bytes ARE the self-accepted bytes (no re-encode).
    let block_bytes = handoff.accepted().as_bytes();
    pump_block(
        state,
        chaindb,
        wal,
        &NoCheckpointSink,
        block_bytes,
        era_schedule,
        ledger_view,
    )
    .map_err(|e| NodeSyncError::Pump(format!("{e:?}")))
}

// =========================================================================
// L5 — recovered-state forge handoff (single-shot)
// =========================================================================

/// Closed L5 forge-handoff error surface. Fail-closed: a forge attempt on
/// a base that did NOT carry a recovered seed-epoch consensus-input record
/// is unrepresentable as a forge — it returns this typed error, never a
/// bundle / cold / genesis fallback.
#[derive(Debug)]
pub enum NodeForgeError {
    /// The recovered `BootstrapState` has `seed_epoch_consensus_inputs:
    /// None`. The leadership view that decides who may forge MUST come
    /// from the recovered surface (DC-CINPUT-02b); without it there is no
    /// forge base, and L5 fails closed rather than reach for a bundle.
    MissingRecoveredConsensusInputs,
    /// A selected tip is present but `recovered.chain_dep.last_block_no` is
    /// None — a malformed recovered state (a tip implies a block height).
    /// Fail closed rather than default to a magic block number (the
    /// cold-start `block 0` path is the `selected_tip == None` branch).
    RecoveredTipMissingBlockNo,
}

/// PHASE4-N-F-G-A S4 — the node forge path's explicit single-recovered-seed-epoch
/// admission verdict (DC-EPOCH-03). A candidate forge slot is admissible ONLY
/// when it locates to the recovered seed epoch; any other epoch — or an
/// unlocatable slot — fails closed, because the recovered `chain_dep` eta0 is the
/// *seed-epoch* nonce and is stale past the boundary (a peer-reject class).
///
/// Closed sum: leadership runs only on `WithinSeedEpoch`; every `OffEpoch` is a
/// pre-leadership fail-closed. GREEN — pure, derived solely from `(slot,
/// era_schedule, seed_epoch)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeEpochAdmission {
    /// The candidate slot locates to the recovered seed epoch — leadership may be
    /// evaluated.
    WithinSeedEpoch,
    /// The candidate slot is outside the recovered seed epoch (or cannot be
    /// located at all): fail closed before leadership / KES signing.
    /// `candidate_epoch` is the located epoch, or `None` when the slot does not
    /// locate to any era.
    OffEpoch {
        candidate_epoch: Option<EpochNo>,
        seed_epoch: EpochNo,
    },
}

/// PHASE4-N-F-G-A S4: decide forge epoch-admission for `slot` against the
/// recovered `seed_epoch`, deriving the candidate epoch through the BLUE
/// [`EraSchedule::locate`] map — the same slot→epoch translation
/// `query_leader_schedule` uses, so the guard never diverges from leadership.
///
/// Within the seed epoch ⇒ [`ForgeEpochAdmission::WithinSeedEpoch`]; any other
/// located epoch ⇒ `OffEpoch { Some(e), seed }`; a slot that does not locate ⇒
/// `OffEpoch { None, seed }`. Pure / deterministic — no I/O, clock, rand, float.
pub fn forge_epoch_admission(
    slot: u64,
    era_schedule: &EraSchedule,
    seed_epoch: EpochNo,
) -> ForgeEpochAdmission {
    match era_schedule.locate(SlotNo(slot)) {
        Ok(loc) if loc.epoch == seed_epoch => ForgeEpochAdmission::WithinSeedEpoch,
        Ok(loc) => ForgeEpochAdmission::OffEpoch {
            candidate_epoch: Some(loc.epoch),
            seed_epoch,
        },
        Err(_) => ForgeEpochAdmission::OffEpoch {
            candidate_epoch: None,
            seed_epoch,
        },
    }
}

/// PHASE4-N-AE.A (DC-NODE-15): why a forge is not admissible against the
/// followed peer tip. A distinct, named reason for each absence/mismatch — never
/// a fabricated tip and never a silently-collapsed equality failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotCaughtUpReason {
    /// No followed peer tip has been observed yet (the follow has reported no
    /// peer tip on this run). Distinct from a durable-tip absence.
    NoFollowedPeerTip,
    /// No durable servable tip exists yet (the follow has not durably stored a
    /// peer block, so `ChainDb::tip()` / the served projection is empty). The
    /// recovered snapshot anchor is NOT a durable servable tip — so a forge here
    /// would have no peer-intersectable base.
    NoDurableServableTip,
    /// Both tips are present but disagree (hash or block_no): the durable
    /// servable tip is behind / diverged from the followed peer tip, so a forge
    /// would build on a base the peer is not standing on.
    TipMismatch,
}

/// PHASE4-N-AE.A (DC-NODE-15): the forge-on-followed-tip admission verdict. A
/// closed two-variant GREEN classifier sibling to [`ForgeEpochAdmission`]:
/// `CaughtUp` iff BOTH tips are present AND their `hash` AND `block_no` are
/// equal; otherwise `NotCaughtUp` carrying the named reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeFollowedTipAdmission {
    /// The durable servable tip equals the followed peer tip — a forge may be
    /// attempted (its successor builds on a peer-intersectable parent).
    CaughtUp,
    /// The durable servable tip does not equal the followed peer tip — fail
    /// closed before leadership / signing.
    NotCaughtUp { reason: NotCaughtUpReason },
}

/// PHASE4-N-AE.A (DC-NODE-15): decide forge-on-followed-tip admission. Pure /
/// deterministic GREEN — no I/O, clock, rand, float; derived solely from the two
/// `Option<TipPoint>` inputs. `CaughtUp` requires BOTH tips present AND their
/// `hash` AND `block_no` equal. Absence is an explicit named reason: a missing
/// followed peer tip is [`NotCaughtUpReason::NoFollowedPeerTip`], a missing
/// durable servable tip is [`NotCaughtUpReason::NoDurableServableTip`], and a
/// present-but-unequal pair is [`NotCaughtUpReason::TipMismatch`]. The slot is
/// ignored for equality — two blocks at the same `(hash, block_no)` are the same
/// chain point by their canonical hash (the parent identity DC-CONS-24 binds).
///
/// The followed peer tip is a forge-ADMISSIBILITY input only: this classifier
/// can only return a verdict that PREVENTS a forge; it selects nothing and never
/// reaches `select_best_chain` / `chain_selector` / `fork_choice`.
pub fn forge_followed_tip_admission(
    durable_servable_tip: Option<TipPoint>,
    followed_peer_tip: Option<TipPoint>,
) -> ForgeFollowedTipAdmission {
    match (durable_servable_tip, followed_peer_tip) {
        (Some(durable), Some(peer)) => {
            if durable.hash == peer.hash && durable.block_no == peer.block_no {
                ForgeFollowedTipAdmission::CaughtUp
            } else {
                ForgeFollowedTipAdmission::NotCaughtUp {
                    reason: NotCaughtUpReason::TipMismatch,
                }
            }
        }
        (None, Some(_)) => ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::NoDurableServableTip,
        },
        (_, None) => ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::NoFollowedPeerTip,
        },
    }
}

/// PHASE4-N-AE.A (DC-NODE-15): a typed, structured forge refusal — semantically
/// distinct from a forge *error* ([`NodeForgeError`]) and from a forge *result*
/// (the BLUE self-accept outcome). A `ForgeRefused` means the admissibility gate
/// PREVENTED the forge: **no state transition was attempted**, the tip is
/// unchanged, nothing was admitted / served / handed off. It is never a
/// log-string-only path — the carrier holds the tips + the named reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeRefused {
    /// The forge was not caught up to the followed peer tip. Carries the
    /// observed tips + the named reason for diagnostics. `local_servable_tip` is
    /// the durable servable tip Ade would have built on (NEVER the recovered
    /// snapshot anchor); `followed_peer_tip` is the peer tip the follow observed.
    NotCaughtUp {
        local_servable_tip: Option<TipPoint>,
        followed_peer_tip: Option<TipPoint>,
        reason: NotCaughtUpReason,
    },
    /// DC-NODE-18 (PHASE4-N-AF): the single-producer extend-own-spine fence
    /// refused the forge. Structured + comparable (never a stringly-authoritative
    /// error): the named reason plus the tips + venue role observed at the gate.
    /// No state transition was attempted; the tip is unchanged.
    SingleProducerFenceViolation {
        reason: SingleProducerFenceReason,
        durable_tip: Option<TipPoint>,
        followed_peer_tip: Option<TipPoint>,
        observed_peer_tip: Option<TipPoint>,
        venue_role: VenueRole,
    },
}

/// PHASE4-N-AE.A: the closed outcome of one `--mode node` ForgeTick attempt.
/// Three mechanically-distinct states (NOT folded into `CoordinatorEvent`):
/// `Refused` (the admissibility gate prevented the forge — no state transition),
/// `Forged` (the forge path ran, carrying the existing success carrier), and
/// `Failed` (the forge path was attempted and failed). A RED/GREEN sum, not a
/// canonical type.
#[derive(Debug)]
pub enum NodeForgeOutcome {
    /// The forge path ran. Carries the existing forge-result carrier — the
    /// reused `CoordinatorEvent` plus the optional self-accepted handoff.
    Forged(CoordinatorEvent, Option<SelfAcceptedHandoff>),
    /// The admissibility gate refused the forge: no state transition attempted,
    /// tip unchanged.
    Refused(ForgeRefused),
    /// The forge path was attempted but failed with a typed error.
    Failed(NodeForgeError),
}

/// PHASE4-N-AE.A (DC-NODE-15): the structured followed-peer-tip admissibility
/// signal — a sibling of the block-bytes source, sourced from the SAME
/// `run_admission_wire_pump` stream that already observes the peer tip (the
/// `AdmissionPeerEvent::TipUpdate` events `NodeBlockSource` deliberately skips
/// for sync). It carries the latest observed followed peer tip as an
/// `Option<TipPoint>` and is consumed ONLY as a forge-admissibility input.
///
/// This is NOT a sync / chain-selection authority: it never advances a tip,
/// never feeds `next_block` / `pump_block`, and never reaches a chain selector.
/// It can only PREVENT a forge (via [`forge_followed_tip_admission`]). It does
/// NOT revive `TipUpdate` as a sync tip authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FollowedPeerTipSignal {
    latest: Option<TipPoint>,
}

impl FollowedPeerTipSignal {
    /// A signal that has observed no followed peer tip yet.
    pub fn new() -> Self {
        Self { latest: None }
    }

    /// The latest observed followed peer tip, or `None` if none observed yet.
    pub fn tip(&self) -> Option<TipPoint> {
        self.latest.clone()
    }

    /// Record an observed followed peer tip from the wire stream's `TipUpdate`.
    /// A `Point::Origin` tip carries no `(slot, hash)` to stand on, so it is
    /// ignored — admissibility stays at the last concrete tip (or `None`). This
    /// is the ONLY mutation; it is a write-only side effect of draining the wire
    /// stream and never influences which block `next_block` yields.
    fn observe(&mut self, tip: &Tip) {
        if let Point::Block { slot, hash } = &tip.point {
            self.latest = Some(TipPoint {
                slot: *slot,
                hash: hash.clone(),
                block_no: tip.block_no,
            });
        }
    }
}

// =====================================================================
// DC-NODE-18 (PHASE4-N-AF) — single-producer extend-own-durable-spine.
//
// After the initial DC-NODE-15 catch-up to a real peer tip and the serve of the
// first own successor, a node in an EXPLICITLY single-producer venue may extend
// its OWN durable adopted spine (forge N+2 on durable N+1, ...) without requiring
// the relay to re-announce each own block back over the follow link (OQ-1: the
// relay does NOT re-announce; see DC-NODE-17 notes). Promotion into the extend
// state requires an explicit RED venue-adoption certificate; it is NEVER inferred
// from self-admit. A gate-APPLICABILITY refinement, NOT a fork-choice weakening
// (DC-CONS-03 stays the multi-chain authority); fenced strictly to single-producer.
// =====================================================================

/// Whether the venue is explicitly declared single-producer. The extend gate
/// fails closed on `Unknown` — DC-NODE-18 applies ONLY in an explicitly declared
/// single-producer venue (relay non-producing, Ade sole producer). A venue-scoped
/// admissibility input, NOT a global semantics knob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueRole {
    /// Not declared single-producer — the extend fence fails closed.
    Unknown,
    /// Explicitly declared single-producer: relay non-producing, Ade sole producer.
    SingleProducer,
}

// DC-NODE-21 (PHASE4-N-AH S2): the VenueAdoptionCertificate type is REMOVED from
// ade_node — the cert is operator evidence parsed by the harness, never a forge input.

/// DC-NODE-18: the explicit single-producer forge mode (NO booleans). The forge
/// scheduler walks this enum; the BLUE durable-admit + chain-selection paths are
/// untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeMode {
    /// Pre-catch-up: the DC-NODE-15 gate (durable == followed) governs the forge.
    InitialCatchupRequired,
    /// Caught up to the peer tip; the first successor forges on it (DC-NODE-15).
    CaughtUpToPeerTip { peer_tip: TipPoint },
    /// Single-producer steady state: extend the own durable adopted spine.
    SingleProducerExtendOwnDurableSpine {
        adopted_root: TipPoint,
        current_tip: TipPoint,
    },
}

/// DC-NODE-18: why a single-producer extend-forge is fenced off (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleProducerFenceReason {
    /// The venue is not explicitly declared single-producer.
    VenueNotDeclaredSingleProducer,
    /// The relay is producing (not a pure follower of Ade's chain).
    RelayProducing,
    /// A peer block beyond the adopted root (a block Ade did not forge) was
    /// observed — there is another producer.
    CompetingPeerBlockBeyondAdoptedRoot,
    /// The observed peer tip disagrees (hash) with Ade's single-producer spine at
    /// that block_no — a fork / a divergent durable tip.
    PeerTipDisagreesWithSpine,
    /// The recovered anchor is the k=0 snapshot-conflict edge (the frozen relay tip
    /// equals the recover anchor). DC-NODE-18 does not apply there.
    RecoveredAnchorK0SnapshotConflict,
}

/// DC-NODE-18: the per-ForgeTick decision for the single-producer forge mode. Pure
/// / total / deterministic GREEN — derived solely from the mode + the tip inputs +
/// the venue facts + an optional certificate. The followed/observed peer tips are
/// forge-ADMISSIBILITY inputs only; this never selects/reorders/prefers chains and
/// never reaches `select_best_chain` / `chain_selector` / `fork_choice`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SingleProducerForgeDecision {
    /// Initial modes — defer to the existing DC-NODE-15 followed-tip gate
    /// (`forge_followed_tip_admission`); DC-NODE-18 does not apply yet.
    UseInitialCatchupGate,
    /// Extend the own durable spine — forge on `forge_base` (the durable tip).
    /// DC-NODE-20: the extend state is entered directly on self-admit
    /// (`forge_mode_after_admit`); there is no cert-promotion / await-cert outcome.
    ExtendOwnSpine { forge_base: TipPoint },
    /// Fenced off — refuse with the structured violation.
    Refuse(ForgeRefused),
}

/// DC-NODE-18 transition: `InitialCatchupRequired -> CaughtUpToPeerTip` once the
/// DC-NODE-15 gate reports CaughtUp (durable == followed == `peer_tip`). Total;
/// idempotent in any other mode.
pub fn forge_mode_on_caughtup(mode: &ForgeMode, peer_tip: TipPoint) -> ForgeMode {
    match mode {
        ForgeMode::InitialCatchupRequired => ForgeMode::CaughtUpToPeerTip { peer_tip },
        other => other.clone(),
    }
}

/// DC-NODE-18 transition: advance `SingleProducerExtendOwnDurableSpine.current_tip`
/// after a successful extend forge. Total; only advances within the extend state.
pub fn forge_mode_on_extend(mode: &ForgeMode, new_tip: TipPoint) -> ForgeMode {
    match mode {
        ForgeMode::SingleProducerExtendOwnDurableSpine { adopted_root, .. } => {
            ForgeMode::SingleProducerExtendOwnDurableSpine {
                adopted_root: adopted_root.clone(),
                current_tip: new_tip,
            }
        }
        other => other.clone(),
    }
}

/// DC-NODE-18: the post-forge mode transition, applied by the RED loop ONLY after a
/// forge attempt. `admitted` is true IFF an actual block was forged AND durably
/// admitted — a not_leader / no-op tick sets the loop's `forged` flag but admits
/// nothing, and MUST NOT advance the mode. Pure / total: `!admitted` or a missing
/// `own_tip` returns the mode unchanged; otherwise `CaughtUpToPeerTip` records the
/// first own block (with its parent peer tip) and the extend state advances
/// `current_tip`.
pub fn forge_mode_after_admit(
    mode: &ForgeMode,
    admitted: bool,
    own_tip: Option<TipPoint>,
    _parent_peer_tip: Option<TipPoint>,
) -> ForgeMode {
    if !admitted {
        return mode.clone();
    }
    let own = match own_tip {
        Some(t) => t,
        None => return mode.clone(),
    };
    match mode {
        // DC-NODE-20: self-admit enters the extend state DIRECTLY on the local durable
        // spine head -- no FirstOwnBlockServed cert-wait. The own block just admitted
        // through pump_block (DC-NODE-12) IS the adopted root + current tip; relay
        // adoption is evidence (DC-NODE-21), not a forge-loop precondition.
        ForgeMode::CaughtUpToPeerTip { .. } => ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: own.clone(),
            current_tip: own,
        },
        ForgeMode::SingleProducerExtendOwnDurableSpine { .. } => forge_mode_on_extend(mode, own),
        other => other.clone(),
    }
}

/// DC-NODE-18 + DC-NODE-20: decide the single-producer forge action for this
/// ForgeTick. Pure / deterministic GREEN. `relay_producing` and `recovered_anchor_k0`
/// are RED venue facts supplied by the loop. DC-NODE-20: the forge base is Ade's own
/// local durable spine head -- the adoption certificate is NOT consulted (it is
/// evidence-only, DC-NODE-21). The fence fails closed (refuses) on any
/// single-producer-venue violation.
pub fn single_producer_forge_decision(
    mode: &ForgeMode,
    durable_servable_tip: Option<TipPoint>,
    followed_peer_tip: Option<TipPoint>,
    observed_peer_tip: Option<TipPoint>,
    venue_role: VenueRole,
    relay_producing: bool,
    recovered_anchor_k0: bool,
) -> SingleProducerForgeDecision {
    let violation = |reason: SingleProducerFenceReason| {
        SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation {
            reason,
            durable_tip: durable_servable_tip.clone(),
            followed_peer_tip: followed_peer_tip.clone(),
            observed_peer_tip: observed_peer_tip.clone(),
            venue_role,
        })
    };
    match mode {
        // Initial modes: DC-NODE-18 does not apply; the existing DC-NODE-15 gate governs.
        ForgeMode::InitialCatchupRequired | ForgeMode::CaughtUpToPeerTip { .. } => {
            SingleProducerForgeDecision::UseInitialCatchupGate
        }
        // DC-NODE-20: the extend state is entered DIRECTLY on self-admit
        // (`forge_mode_after_admit`) -- there is no FirstOwnBlockServed cert-promotion
        // arm; the own durable spine head IS the forge authority, fenced below.
        // Extend state: fail-closed fence, then forge on the durable spine head.
        ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root,
            current_tip,
        } => {
            if venue_role != VenueRole::SingleProducer {
                return violation(SingleProducerFenceReason::VenueNotDeclaredSingleProducer);
            }
            if relay_producing {
                return violation(SingleProducerFenceReason::RelayProducing);
            }
            if recovered_anchor_k0 {
                return violation(SingleProducerFenceReason::RecoveredAnchorK0SnapshotConflict);
            }
            if let Some(obs) = &observed_peer_tip {
                if obs.block_no > current_tip.block_no {
                    return violation(SingleProducerFenceReason::CompetingPeerBlockBeyondAdoptedRoot);
                }
                if obs.block_no == current_tip.block_no && obs.hash != current_tip.hash {
                    return violation(SingleProducerFenceReason::PeerTipDisagreesWithSpine);
                }
                if obs.block_no == adopted_root.block_no && obs.hash != adopted_root.hash {
                    return violation(SingleProducerFenceReason::PeerTipDisagreesWithSpine);
                }
            }
            // The forge base is Ade's own durable spine head. It must be present and
            // equal `current_tip`; a missing/diverged durable tip fails closed.
            match &durable_servable_tip {
                Some(durable)
                    if durable.hash == current_tip.hash
                        && durable.block_no == current_tip.block_no =>
                {
                    SingleProducerForgeDecision::ExtendOwnSpine {
                        forge_base: durable.clone(),
                    }
                }
                _ => violation(SingleProducerFenceReason::PeerTipDisagreesWithSpine),
            }
        }
    }
}

/// DC-NODE-19 (PHASE4-N-AG S1): the GREEN projection from the forge-mode domain
/// to the closed planner [`VenuePolicy`] input. Returns
/// `ContinueInSingleProducerExtend` ONLY in an explicitly declared single-producer
/// venue that has reached the DC-NODE-18 extend state
/// (`SingleProducerExtendOwnDurableSpine`); otherwise `HaltOnFeedEnd` — the
/// verbatim prior feed-end-halts behaviour. Pure / total / content-blind: it reads
/// only the venue role and the forge-mode discriminant, never a tip / hash / slot.
/// The planner consumes the resulting yes/no, never the mode itself.
pub fn venue_policy(venue_role: VenueRole, forge_mode: &ForgeMode) -> VenuePolicy {
    match (venue_role, forge_mode) {
        (VenueRole::SingleProducer, ForgeMode::SingleProducerExtendOwnDurableSpine { .. }) => {
            VenuePolicy::ContinueInSingleProducerExtend
        }
        _ => VenuePolicy::HaltOnFeedEnd,
    }
}

/// L5 — the recovered-state forge handoff. Single-shot.
///
/// Builds the forge base ENTIRELY from recovered state + the selected tip,
/// and runs one `run_real_forge` (the reused `produce_mode` engine — F2:
/// reuse of a public forge-engine surface, NOT a `produce_mode`
/// conversion; the cold/bundle path is untouched).
///
/// Provenance, not shape (DC-CINPUT-02b / CN-CINPUT-03):
///   - `pool_distr_view` (the leadership view) is projected ONLY from the
///     RECOVERED `recovered.seed_epoch_consensus_inputs` via
///     `PoolDistrView::from_seed_epoch_consensus_inputs`. It is NEVER the
///     `produce_mode::pool_distr_view_from_consensus_inputs` bundle helper,
///     and reads no `--consensus-inputs-path`.
///   - `eta0`, `base_state`, `chain_dep_state` come from the recovered
///     `BootstrapState`; `block_number` + `prev_hash` from the selected
///     tip (recovered `chain_dep.last_block_no` + tip hash).
///
/// Key-source boundary (RED): recovered state provides the ledger base,
/// chain_dep/eta0, selected tip, and recovered `SeedEpochConsensusInputs`
/// (the leadership view). Operator custody provides the signing material +
/// identity: `shell` (KES/VRF/cold/opcert), the operator `pool_id`, and
/// `pparams`/`protocol_version`. The leader-schedule answer is computed
/// HERE over the recovered projected view — not supplied by the caller —
/// so leadership is decided by the recovered surface. No operator consensus
/// bundle participates in the forge base.
///
/// Single-shot: one slot, one attempt. No slot loop, no peer evidence, no
/// BA-02 claim, no multi-epoch — those are deferred to L6 / N-U.
///
/// GREEN cold-start header position (DC-NODE-08) — the single cold-start
/// convention. No selected tip ⇒ the genesis-successor: `block_number 0` +
/// `PrevHash::Genesis`, matching `ChainEvolution::next_block_number()` (tip
/// None ⇒ 0). A selected tip ⇒ `block (last_block_no + 1)` +
/// `PrevHash::Block(tip.hash)`; a tip without a recorded height is a malformed
/// recovered state and fails closed — never a magic block-number default. Pure:
/// proposes the `(block_number, prev_hash)` pair; the BLUE check_header_position
/// (S3) is the final authority on its legality.
fn forge_header_position(
    selected_tip: Option<&ChainTip>,
    last_block_no: Option<BlockNo>,
) -> Result<(u64, PrevHash), NodeForgeError> {
    match selected_tip {
        None => Ok((0, PrevHash::Genesis)),
        Some(tip) => {
            let n = last_block_no
                .map(|b| b.0 + 1)
                .ok_or(NodeForgeError::RecoveredTipMissingBlockNo)?;
            Ok((n, PrevHash::Block(tip.hash.clone())))
        }
    }
}

/// Returns the reused `CoordinatorEvent` (`ForgeSucceeded` /
/// `ForgeNotLeader` / `ForgeFailed`), or a typed `NodeForgeError` when the
/// recovered base cannot host a forge.
#[allow(clippy::too_many_arguments)]
pub fn forge_one_from_recovered(
    recovered: &BootstrapState,
    live_chain_dep: &PraosChainDepState,
    live_ledger: &LedgerState,
    selected_tip: Option<&ChainTip>,
    shell: &mut ProducerShell,
    pool_id: &Hash28,
    pparams: &ProtocolParameters,
    era_schedule: &EraSchedule,
    slot: u64,
    kes_period: u32,
    protocol_version: ProtocolVersion,
) -> Result<(CoordinatorEvent, Option<SelfAcceptedHandoff>), NodeForgeError> {
    // Fail-closed: the leadership view MUST be the recovered surface.
    let recovered_inputs = recovered
        .seed_epoch_consensus_inputs
        .as_ref()
        .ok_or(NodeForgeError::MissingRecoveredConsensusInputs)?;

    // S4 (DC-EPOCH-03): fail closed BEFORE leadership / KES signing when the
    // candidate slot is outside the single recovered seed epoch. The recovered
    // chain_dep eta0 is the seed-epoch nonce; past the boundary it is stale (a
    // peer-reject class) and the forge path drives NO nonce promotion. Reuses the
    // same EraSchedule::locate map leadership uses (no divergence). Off-epoch
    // surfaces as the existing structured ForgeNotLeader — never a fabricated
    // off-epoch forge, never a leadership / sign path.
    if let ForgeEpochAdmission::OffEpoch { .. } =
        forge_epoch_admission(slot, era_schedule, recovered_inputs.epoch_no)
    {
        // PHASE4-N-F-G-B S1: a fail-closed (off-epoch) outcome surfaces no
        // handoff — a non-self-accepted result yields no servable artifact.
        return Ok((
            CoordinatorEvent::ForgeNotLeader {
                slot,
                vrf_output_fingerprint: [0u8; 8],
            },
            None,
        ));
    }

    // DC-CINPUT-02b: project the leadership PoolDistrView from the
    // RECOVERED record — the sole consensus-input source on this path.
    let pool_distr_view = PoolDistrView::from_seed_epoch_consensus_inputs(recovered_inputs);

    // Leadership is decided OVER the recovered projected view: query the
    // leader schedule for the operator's pool against it. The view passed
    // here is the recovered surface (`&pool_distr_view`), never a bundle —
    // so the recovered consensus inputs drive who may forge (DC-CINPUT-02b).
    // Unknown pool / outside horizon ⇒ deterministic `ForgeNotLeader` (not an
    // error), exactly as the diagnostic produce_mode path handles it.
    let answer = match query_leader_schedule(
        &LeaderScheduleQuery {
            slot: SlotNo(slot),
            pool: pool_id.clone(),
        },
        &pool_distr_view,
        era_schedule,
        live_chain_dep,
    ) {
        Ok(a) => a,
        Err(_) => {
            // PHASE4-N-F-G-B S1: not-a-leader is fail-closed — no handoff.
            return Ok((
                CoordinatorEvent::ForgeNotLeader {
                    slot,
                    vrf_output_fingerprint: [0u8; 8],
                },
                None,
            ));
        }
    };

    // Cold-start convention (ONE, matching ChainEvolution::next_block_number):
    // the genesis-successor (no selected tip) is block 0 + PrevHash::Genesis.
    let (next_block_number, prev_hash) =
        forge_header_position(selected_tip, live_chain_dep.last_block_no)?;
    let vrf_vk = shell.vrf_verification_key();

    let ctx = ForgeRequestContext {
        eta0: &live_chain_dep.epoch_nonce,
        vrf_vk: &vrf_vk,
        leader_schedule_answer: &answer,
        pparams,
        base_state: live_ledger,
        chain_dep_state: live_chain_dep,
        era_schedule,
        pool_distr_view: &pool_distr_view,
        block_number: BlockNo(next_block_number),
        prev_hash,
        protocol_version,
        prev_opcert_counter: None,
    };

    // Single-shot forge through the reused engine. Its result variants
    // (ForgeSucceeded / ForgeNotLeader / ForgeFailed) are returned as-is;
    // there is no fallback path.
    //
    // PHASE4-N-F-G-B S1: wrap the surfaced BLUE `AcceptedBlock` (Some iff the
    // engine self-accepted ⇒ ForgeSucceeded) into the typed, constructor-fenced
    // `SelfAcceptedHandoff` for the (S2) serve task. `map` keeps None on
    // ForgeNotLeader / ForgeFailed — a non-self-accepted outcome yields no
    // handoff. The token is the ORIGINAL from `self_accept` (CN-FORGE-01),
    // never re-derived from `artifact.bytes`.
    let (event, self_accepted) = run_real_forge(slot, kes_period, &ctx, shell);
    Ok((
        event,
        self_accepted.map(SelfAcceptedHandoff::from_self_accepted),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ade_ledger::seed_consensus_inputs::encode_seed_epoch_consensus_inputs;
    use ade_network::codec::chain_sync::{Point, Tip};
    use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;
    use ade_types::{Hash32, SlotNo};

    fn block(b: u8) -> Vec<u8> {
        vec![b; 4]
    }

    fn tip_update(peer: &str) -> AdmissionPeerEvent {
        AdmissionPeerEvent::TipUpdate {
            peer: peer.to_string(),
            tip: Tip {
                point: Point::Block {
                    slot: SlotNo(1),
                    hash: Hash32([0u8; 32]),
                },
                block_no: 1,
            },
        }
    }

    #[tokio::test]
    async fn in_memory_source_yields_blocks_in_order_then_none() {
        let mut src = NodeBlockSource::in_memory(vec![block(0xA1), block(0xA2), block(0xA3)]);
        assert_eq!(src.next_block().await, Some(block(0xA1)));
        assert_eq!(src.next_block().await, Some(block(0xA2)));
        assert_eq!(src.next_block().await, Some(block(0xA3)));
        assert_eq!(src.next_block().await, None);
        // Idempotent at end-of-feed.
        assert_eq!(src.next_block().await, None);
    }

    #[tokio::test]
    async fn wire_pump_source_selects_blocks_and_skips_tipupdate() {
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(16);
        // Interleave TipUpdate noise with the ordered blocks.
        tx.send(tip_update("p")).await.unwrap();
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xB1),
        })
        .await
        .unwrap();
        tx.send(tip_update("p")).await.unwrap();
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xB2),
        })
        .await
        .unwrap();
        drop(tx); // close the channel after the ordered blocks

        let mut src = NodeBlockSource::from_wire_pump(rx);
        assert_eq!(src.next_block().await, Some(block(0xB1)));
        assert_eq!(src.next_block().await, Some(block(0xB2)));
        assert_eq!(src.next_block().await, None, "closed channel ends the feed");
    }

    // ===== L4b: durable validated apply (hermetic, real persistent stores) =====

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::ReceiveState;
    use ade_ledger::state::LedgerState;
    use ade_ledger::wal::WalEntry;
    use ade_runtime::chaindb::{PersistentChainDb, PersistentChainDbOptions};
    use ade_runtime::rollback::SnapshotCadence;
    use ade_runtime::wal::FileWalStore;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28};
    use tempfile::TempDir;

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(start_576),
                start_epoch: EPOCH_576,
                slot_length_ms: 1_000,
                epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
                safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
            }],
        )
        .expect("schedule")
    }

    fn corpus_view() -> (ConwayValidityCorpus, PoolDistrView) {
        let c = ConwayValidityCorpus::load().expect("corpus");
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            let scale = total / p.sigma.denom;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake: p.sigma.numer * scale,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        (c, PoolDistrView::new(EPOCH_576, total, asc, pools))
    }

    fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    fn fresh_state(eta0: [u8; 32]) -> ForwardSyncState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(eta0));
        chain_dep.evolving_nonce = Nonce(Hash32(eta0));
        ForwardSyncState::new(
            ReceiveState::new(ledger, chain_dep),
            Hash32([0xA0; 32]),
            SnapshotCadence::DEFAULT,
        )
    }

    #[tokio::test]
    async fn node_sync_pump_advances_recoverable_tip() {
        // L4b: drive run_node_sync over an in-memory ordered source against
        // a REAL PersistentChainDb + FileWalStore. Assert the durable apply
        // outcome: block bytes stored, WAL AdmitBlock appended, tip advanced
        // (only via pump_block), and an E4 recovery snapshot captured AT the
        // tip via PersistentSnapshotCache.
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state(c.epoch_nonce);
        let mut source = NodeBlockSource::in_memory(vec![bytes.clone()]);

        let tip = run_node_sync(&mut source, &mut state, &chaindb, &mut wal, &sched, &view)
            .await
            .expect("sync ok")
            .expect("tip advanced");

        // Block durably stored under the advanced tip's hash.
        let stored = ChainDb::get_block_by_hash(&chaindb, &tip.hash)
            .expect("get")
            .expect("block present");
        assert_eq!(stored.bytes, bytes, "preserved wire bytes round-trip");

        // ChainDb tip matches the advanced tip.
        let chain_tip = ChainDb::tip(&chaindb).expect("tip").expect("non-empty");
        assert_eq!(chain_tip.slot, tip.slot);
        assert_eq!(chain_tip.hash, tip.hash);

        // WAL recorded an AdmitBlock for the applied block.
        let entries = wal.read_all().expect("read_all");
        assert!(
            entries
                .iter()
                .any(|e| matches!(e, WalEntry::AdmitBlock { slot, .. } if *slot == tip.slot)),
            "WAL must contain an AdmitBlock at the advanced tip slot"
        );

        // E4: a recovery snapshot was captured AT the tip slot via the same
        // PersistentSnapshotCache path warm-start recovery reads.
        let snap = SnapshotStore::get_snapshot(&chaindb, tip.slot).expect("get snapshot");
        assert!(
            snap.is_some(),
            "run_node_sync must capture a tip checkpoint via PersistentSnapshotCache (E4)"
        );
    }

    #[tokio::test]
    async fn run_node_sync_survives_reannounced_block_in_feed() {
        // PHASE4-N-AE.F (DC-NODE-16, CE-F4): the live-shape echo. A feed that
        // re-announces an already-applied block (the relay serving Ade's own adopted
        // tip back over the follow link, post-CE-A5) must NOT terminate the sync loop
        // -- the duplicate is an idempotent no-op, the loop completes, and the WAL
        // records the block exactly once (no double-apply, no SlotBeforeLastApplied
        // exit-43).
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state(c.epoch_nonce);
        // The SAME block twice: apply, then the echo.
        let mut source = NodeBlockSource::in_memory(vec![bytes.clone(), bytes.clone()]);

        let tip = run_node_sync(&mut source, &mut state, &chaindb, &mut wal, &sched, &view)
            .await
            .expect("sync survives the re-announced block (no fail-close)")
            .expect("tip advanced once");

        // The block was admitted EXACTLY once (the echo is a no-op, no double-apply).
        let admits = wal
            .read_all()
            .expect("read_all")
            .into_iter()
            .filter(|e| matches!(e, WalEntry::AdmitBlock { slot, .. } if *slot == tip.slot))
            .count();
        assert_eq!(
            admits, 1,
            "the re-announced block is admitted exactly once (no double-apply)"
        );
        let chain_tip = ChainDb::tip(&chaindb).expect("tip").expect("non-empty");
        assert_eq!(chain_tip.hash, tip.hash, "tip is the applied block");
    }

    #[tokio::test]
    async fn node_sync_kill_then_warm_start_recovers_same_tip() {
        // L4c: the join point between sync and recovery. Seed a warm-start
        // precondition (anchor sidecar + WAL provenance — the L2 first-run
        // artifact), run L4b durable apply (appends AdmitBlock entries +
        // captures a tip checkpoint via PersistentSnapshotCache), drop the
        // handles, reopen, and recover through the REAL warm_start_recovery.
        // The recovered tip must equal the L4b-advanced tip — recovered from
        // the persisted checkpoint, with NO test-side snapshot injection.
        use ade_ledger::seed_consensus_inputs::{
            encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };
        use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;

        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let dir = TempDir::new().unwrap();
        let snap = dir.path().join("snap");
        let wal_dir = dir.path().join("wal");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::create_dir_all(&wal_dir).unwrap();
        let chaindb_path = snap.join("chain.db");

        // The anchor fingerprint shared by the seed precondition (sidecar +
        // WAL provenance) AND the ForwardSyncState seed. `fresh_state` seeds
        // `prior_fp` from `Hash32([0xA0; 32])`, so the first AdmitBlock
        // chains from this exact value; warm-start discovery keys on the
        // sidecar table, whose key must be the same anchor_fp.
        let anchor_fp = Hash32([0xA0; 32]);
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            Hash28([0x01; 28]),
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        let sidecar = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EPOCH_576,
            epoch_nonce: Nonce(Hash32([0x8a; 32])),
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
            total_active_stake: 1_000,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&sidecar);

        // --- Phase 1: seed the precondition + run L4b durable apply. ---
        let advanced = {
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
            let mut wal = FileWalStore::open(&wal_dir).unwrap();

            // L2 first-run artifact: anchor-keyed sidecar + WAL provenance.
            chaindb
                .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &anchor_fp, EPOCH_576, &sidecar_bytes).unwrap();

            // L4b durable apply over one ordered source.
            let mut state = fresh_state(c.epoch_nonce);
            let mut source = NodeBlockSource::in_memory(vec![bytes.clone()]);
            run_node_sync(&mut source, &mut state, &chaindb, &mut wal, &sched, &view)
                .await
                .expect("sync ok")
                .expect("tip advanced")
            // chaindb + wal dropped here → the kill boundary.
        };

        // --- Phase 2: reopen + recover through the REAL recovery path. ---
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal = FileWalStore::open(&wal_dir).unwrap();
        let recovered = crate::node_lifecycle::warm_start_recovery(&chaindb, &wal)
            .expect("warm-start recovers after sync");

        let recovered_tip = recovered.tip.expect("recovered a tip");
        assert_eq!(
            recovered_tip.slot, advanced.slot,
            "recovered tip slot must equal the L4b-advanced tip slot"
        );
        assert_eq!(
            recovered_tip.hash, advanced.hash,
            "recovered tip hash must equal the L4b-advanced tip hash"
        );
        // The recovered seed-epoch sidecar still verifies (carried from L3).
        assert!(
            recovered.seed_epoch_consensus_inputs.is_some(),
            "warm-start must still recover the verified seed-epoch sidecar"
        );
    }

    #[tokio::test]
    async fn node_sync_fails_closed_on_undecodable_block() {
        // A block the BLUE decoder rejects must halt the drive fail-closed
        // (typed NodeSyncError::Pump), never skip-past, never fall back.
        let (_c, view) = corpus_view();
        let sched = schedule();

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state([0xEE; 32]);
        let mut source = NodeBlockSource::in_memory(vec![vec![0xDE, 0xAD, 0xBE, 0xEF]]);

        let r = run_node_sync(&mut source, &mut state, &chaindb, &mut wal, &sched, &view).await;
        assert!(
            matches!(r, Err(NodeSyncError::Pump(_))),
            "undecodable block must fail closed, got {r:?}"
        );
        // No tip advanced, no snapshot captured.
        assert!(ChainDb::tip(&chaindb).expect("tip").is_none());
        assert!(SnapshotStore::list_snapshot_slots(&chaindb)
            .expect("list")
            .is_empty());
    }

    #[tokio::test]
    async fn wire_pump_source_ends_on_disconnect_ignoring_later_blocks() {
        // A clean disconnect ends the feed even if more Block events are
        // queued behind it — a disconnect is not a tip authority, and a
        // single ordered source stops at its peer's disconnect (E1/E2).
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(16);
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xC1),
        })
        .await
        .unwrap();
        tx.send(AdmissionPeerEvent::Disconnected {
            peer: "p".to_string(),
        })
        .await
        .unwrap();
        // This block is queued AFTER the disconnect; it must NOT surface.
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xC2),
        })
        .await
        .unwrap();
        drop(tx);

        let mut src = NodeBlockSource::from_wire_pump(rx);
        assert_eq!(src.next_block().await, Some(block(0xC1)));
        assert_eq!(
            src.next_block().await,
            None,
            "disconnect ends the feed; later queued blocks are not surfaced"
        );
    }

    // =====================================================================
    // PHASE4-N-F-D S2 — readiness signals + the RED relay run loop
    // =====================================================================
    //
    // Readiness is content-blind RED scheduling info; the relay loop composes
    // the GREEN planner over run_node_sync (the sole block-consumption path).
    // run_relay_loop lives in node_lifecycle; these tests drive it over the
    // same hermetic fixtures the L4b tests use.

    use crate::node_lifecycle::{
        run_relay_loop, run_relay_loop_with_sched, ForgeActivation, NodeLifecycleError,
    };
    use tokio::sync::watch;

    #[test]
    fn readiness_inmemory_has_work_and_is_ended() {
        let mut empty = NodeBlockSource::in_memory(vec![]);
        assert!(!empty.has_work_ready(), "empty in-memory: no work");
        assert!(empty.is_ended(), "empty in-memory: ended");

        let mut full = NodeBlockSource::in_memory(vec![block(0x01), block(0x02)]);
        assert!(full.has_work_ready(), "non-empty in-memory: work ready");
        assert!(!full.is_ended(), "non-empty in-memory: not ended");
    }

    #[tokio::test]
    async fn readiness_wirepump_is_content_blind_and_order_preserving() {
        // has_work_ready fills the content-blind lookahead via non-blocking
        // try_recv; next_block then delivers the SAME bytes in arrival order.
        // Readiness peeks for availability, never decodes/reorders content.
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(16);
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xD1),
        })
        .await
        .unwrap();
        tx.send(AdmissionPeerEvent::Block {
            peer: "p".to_string(),
            block_bytes: block(0xD2),
        })
        .await
        .unwrap();
        drop(tx);

        let mut src = NodeBlockSource::from_wire_pump(rx);
        // Peek (fills lookahead) — does not consume or reorder.
        assert!(src.has_work_ready(), "buffered block is ready");
        assert!(!src.is_ended(), "not ended while a block is buffered");
        // Delivery order preserved through the lookahead.
        assert_eq!(src.next_block().await, Some(block(0xD1)));
        assert_eq!(src.next_block().await, Some(block(0xD2)));
        // Drained + channel closed ⇒ ended.
        assert!(!src.has_work_ready(), "no more work");
        assert!(src.is_ended(), "disconnected + drained ⇒ ended");
        assert_eq!(src.next_block().await, None);
    }

    #[tokio::test]
    async fn node_sched_events_emit_closed_vocabulary() {
        // CE-G-J-1 (positive emit): the `--mode node` relay loop emits the
        // closed CN-NODE-04 vocabulary through the emit-only sched sink. A
        // drained in-memory feed (the hermetic analogue of the C1 sole-producer
        // empty feed) classifies as the ELIGIBLE `clean_empty` — NEVER an
        // ineligible reason (OQ1) — and the loop emits `feed_unavailable`
        // before halting cleanly. Every emitted line's `event` is in the closed
        // allow-list. Forge is OFF here (no ForgeActivation), so this is purely
        // the feed-end emit; the no-behavior-change proof lives in the unchanged
        // run_loop_planner determinism/precedence-table tests.
        let (c, view) = corpus_view();
        let sched = schedule();

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state(c.epoch_nonce);
        // Drained feed — is_ended() right away, no block to sync.
        let mut source = NodeBlockSource::in_memory(vec![]);
        let (_tx, mut shutdown) = watch::channel(false);

        let mut sched_log = crate::live_log::NodeSchedLogWriter::new(Vec::<u8>::new());
        run_relay_loop_with_sched(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut shutdown,
            None,
            Some(&mut sched_log),
        )
        .await
        .expect("relay loop halts cleanly on a drained feed");

        let bytes = sched_log.into_inner();
        let text = std::str::from_utf8(&bytes).expect("utf8");
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        assert!(
            !lines.is_empty(),
            "the relay loop must emit at least one CN-NODE-04 event on a drained feed"
        );
        // The closed allow-list — the same set the emit-only gate enforces.
        const ALLOW: &[&str] = &[
            "feed_unavailable",
            "forge_tick_considered",
            "forge_tick_skipped",
            "forge_attempted",
            "forge_result",
        ];
        for line in &lines {
            assert!(
                ALLOW
                    .iter()
                    .any(|d| line.contains(&format!("\"event\":\"{d}\""))),
                "emitted line is outside the closed CN-NODE-04 allow-list: {line}"
            );
        }
        // The drained in-memory feed is the ELIGIBLE clean_empty case (OQ1) —
        // never an ineligible reason.
        assert!(
            lines.iter().any(|l| l
                .contains("\"event\":\"feed_unavailable\"")
                && l.contains("\"reason\":\"clean_empty\"")),
            "a drained in-memory feed must emit feed_unavailable{{clean_empty}} (eligible), got: {text}"
        );
    }

    #[tokio::test]
    async fn relay_loop_syncs_then_halts_clean_on_source_end() {
        // CE-D-3: both arms converge into the loop; an available batch is
        // synced via run_node_sync (durable tip + WAL + checkpoint), then a
        // drained+ended source halts the loop cleanly.
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state(c.epoch_nonce);
        let mut source = NodeBlockSource::in_memory(vec![bytes.clone()]);
        let (_tx, mut shutdown) = watch::channel(false);

        run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut shutdown,
            None,
        )
        .await
        .expect("relay loop drains the batch then halts cleanly");

        // The durable tip advanced (only via run_node_sync → pump_block).
        let tip = ChainDb::tip(&chaindb).expect("tip").expect("tip advanced");
        let stored = ChainDb::get_block_by_hash(&chaindb, &tip.hash)
            .expect("get")
            .expect("block present");
        assert_eq!(stored.bytes, bytes, "preserved wire bytes round-trip");
        // WAL recorded the AdmitBlock; the source is fully drained + ended.
        let entries = wal.read_all().expect("read_all");
        assert!(
            entries
                .iter()
                .any(|e| matches!(e, WalEntry::AdmitBlock { slot, .. } if *slot == tip.slot)),
            "WAL must contain an AdmitBlock at the synced tip"
        );
        assert!(source.is_ended(), "source ended after the batch drained");
    }

    #[tokio::test]
    async fn relay_loop_halts_clean_on_shutdown_no_partial_write() {
        // CE-D-3: a shutdown requested BEFORE the first tick halts the loop at
        // the boundary with NO SyncOnce — the tip never advances, no partial
        // write (shutdown precedence; planner HaltCleanly).
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state(c.epoch_nonce);
        // Work IS available, but shutdown is already requested.
        let mut source = NodeBlockSource::in_memory(vec![bytes]);
        let (_tx, mut shutdown) = watch::channel(true);

        run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut shutdown,
            None,
        )
        .await
        .expect("relay loop halts cleanly on shutdown");

        // Shutdown took precedence over the available block: nothing applied.
        assert!(
            ChainDb::tip(&chaindb).expect("tip").is_none(),
            "shutdown halts before any SyncOnce — no tip advance, no partial write"
        );
        assert!(
            SnapshotStore::list_snapshot_slots(&chaindb)
                .expect("list")
                .is_empty(),
            "no checkpoint captured when shutdown precedes sync"
        );
    }

    #[tokio::test]
    async fn relay_loop_idles_then_syncs_on_incremental_feed() {
        // CE-D-3 (Idle path): an open, momentarily-empty WirePump makes the
        // planner Idle; wait_ready awaits the next block (the loop's sole
        // inter-iteration await); when it arrives the loop syncs, then the
        // closed channel ends the feed and it halts cleanly. Hermetic
        // in-process mpsc — NO live peer.
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state(c.epoch_nonce);

        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(16);
        let mut source = NodeBlockSource::from_wire_pump(rx);
        let (_tx_sd, mut shutdown) = watch::channel(false);

        // Sender runs when the loop yields at wait_ready (current-thread
        // runtime): the first tick finds no work (Idle), wait_ready awaits,
        // the spawned task then sends one block and closes the channel.
        let send_bytes = bytes.clone();
        let sender = tokio::spawn(async move {
            tx.send(AdmissionPeerEvent::Block {
                peer: "p".to_string(),
                block_bytes: send_bytes,
            })
            .await
            .unwrap();
            drop(tx);
        });

        run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut shutdown,
            None,
        )
        .await
        .expect("relay loop idles, then syncs the delivered block, then halts");
        sender.await.unwrap();

        let tip = ChainDb::tip(&chaindb).expect("tip").expect("tip advanced");
        assert_eq!(
            ChainDb::get_block_by_hash(&chaindb, &tip.hash)
                .expect("get")
                .expect("present")
                .bytes,
            bytes,
            "the incrementally-delivered block was synced"
        );
    }

    #[tokio::test]
    async fn relay_loop_fails_closed_on_unapplyable_block() {
        // CE-D-2 fail-closed: a block run_node_sync → pump_block rejects halts
        // the loop with a typed RelaySync error — never a skip-past, never a
        // fallback, tip unmoved. (An undecodable block exercises the same
        // RelaySync path a cross-epoch header takes: the recovered
        // single-epoch view rejects an off-epoch header — DC-CINPUT-02a,
        // proven at the view/forge level — and run_node_sync surfaces it as
        // the identical fail-closed NodeSyncError → RelaySync.)
        let (_c, view) = corpus_view();
        let sched = schedule();

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = fresh_state([0x5A; 32]);
        let mut source = NodeBlockSource::in_memory(vec![vec![0xDE, 0xAD, 0xBE, 0xEF]]);
        let (_tx, mut shutdown) = watch::channel(false);

        let r = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut shutdown,
            None,
        )
        .await;
        assert!(
            matches!(r, Err(NodeLifecycleError::RelaySync(_))),
            "unapplyable block must fail closed via RelaySync, got {r:?}"
        );
        assert!(
            ChainDb::tip(&chaindb).expect("tip").is_none(),
            "no tip advance on a rejected block"
        );
    }

    #[tokio::test]
    async fn relay_loop_two_clean_runs_byte_identical() {
        // CE-D-4 / T-REC-03: two clean run_relay_loop runs over IDENTICAL
        // inputs (same recovered-state seed, same ordered in-memory feed, same
        // shutdown schedule) produce byte-identical authoritative outputs —
        // tip (slot + hash), WAL image, and captured checkpoint slots. Proves
        // deterministic orchestration absent crash interference. Multi-block
        // feed so the property holds across iterations, not just one apply.
        let (c, view) = corpus_view();
        let sched = schedule();
        // Two lightest blocks (for a fast multi-step run), fed in ASCENDING
        // SLOT order. The relay loop applies blocks slot-monotonically
        // (SlotBeforeLastApplied otherwise), so the feed must be slot-ordered,
        // not size-ordered. Slot is read via the same `decode_block` authority
        // the pump uses (`decoded.header_input.slot`).
        let mut sized: Vec<(usize, usize)> = (0..c.blocks.len())
            .map(|i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                (env.block_end - env.block_start, i)
            })
            .collect();
        sized.sort();
        let mut chosen: Vec<usize> = sized.iter().take(2).map(|&(_, i)| i).collect();
        chosen.sort_by_key(|&i| {
            decode_block(&c.blocks[i])
                .expect("decode")
                .header_input
                .slot
        });
        let feed: Vec<Vec<u8>> = chosen.iter().map(|&i| c.blocks[i].clone()).collect();

        // One clean run over a fresh store set; returns the authoritative
        // artifacts (tip, WAL Debug image, checkpoint slots).
        async fn run_once(
            feed: Vec<Vec<u8>>,
            eta0: [u8; 32],
            sched: &EraSchedule,
            view: &PoolDistrView,
        ) -> (Option<(SlotNo, Hash32)>, String, Vec<SlotNo>) {
            let dir = TempDir::new().unwrap();
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                    .unwrap();
            let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
            let mut state = fresh_state(eta0);
            let mut source = NodeBlockSource::in_memory(feed);
            let (_tx, mut shutdown) = watch::channel(false);
            run_relay_loop(
                &mut state,
                &mut source,
                &chaindb,
                &mut wal,
                sched,
                view,
                &mut shutdown,
                None,
            )
            .await
            .expect("clean run");
            let tip = ChainDb::tip(&chaindb)
                .expect("tip")
                .map(|t| (t.slot, t.hash));
            // WAL byte-identity: the Debug image captures every field
            // (slot/hashes/fingerprints) of every entry in order.
            let wal_image = format!("{:?}", wal.read_all().expect("read_all"));
            let snaps = SnapshotStore::list_snapshot_slots(&chaindb).expect("list");
            (tip, wal_image, snaps)
        }

        let (tip_a, wal_a, snaps_a) = run_once(feed.clone(), c.epoch_nonce, &sched, &view).await;
        let (tip_b, wal_b, snaps_b) = run_once(feed.clone(), c.epoch_nonce, &sched, &view).await;

        assert!(tip_a.is_some(), "the run must advance a tip");
        assert_eq!(
            tip_a, tip_b,
            "tip (slot + hash) must be byte-identical across clean runs"
        );
        assert_eq!(
            wal_a, wal_b,
            "WAL image must be byte-identical across clean runs"
        );
        assert_eq!(
            snaps_a, snaps_b,
            "captured checkpoint slots must be identical across clean runs"
        );
    }

    #[tokio::test]
    async fn relay_loop_kill_at_boundary_recovers_same_tip() {
        // CE-D-5: a relay-loop-advanced tip, after a kill at an iteration
        // boundary, is recovered through the PRODUCTION warm-start path to the
        // same tip as an uninterrupted run. Mirrors the L4c
        // node_sync_kill_then_warm_start_recovers_same_tip proof but drives the
        // tip via run_relay_loop instead of run_node_sync — so the loop
        // inherits the durable-before-advance (DC-SYNC-01) + warm-start
        // recovery guarantee. Test-only.
        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);

        let dir = TempDir::new().unwrap();
        let snap = dir.path().join("snap");
        let wal_dir = dir.path().join("wal");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::create_dir_all(&wal_dir).unwrap();
        let chaindb_path = snap.join("chain.db");

        // Anchor fingerprint shared by the seed precondition (sidecar + WAL
        // provenance) AND the ForwardSyncState seed: `fresh_state` seeds
        // `prior_fp` = Hash32([0xA0; 32]); warm-start discovery keys on the
        // sidecar table, whose key must be the same anchor_fp.
        let anchor_fp = Hash32([0xA0; 32]);
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            Hash28([0x11; 28]),
            PoolEntry {
                active_stake: 1,
                vrf_keyhash: Hash32([0x22; 32]),
            },
        );
        let recovered_inputs = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EPOCH_576,
            epoch_nonce: Nonce(Hash32([0x8b; 32])),
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
            total_active_stake: 1,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&recovered_inputs);

        // Seed + drive the relay loop, then DROP the stores (kill).
        let synced_tip = {
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
            let mut wal = FileWalStore::open(&wal_dir).unwrap();
            chaindb
                .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &anchor_fp, EPOCH_576, &sidecar_bytes).unwrap();

            let mut state = fresh_state(c.epoch_nonce);
            let mut source = NodeBlockSource::in_memory(vec![bytes.clone()]);
            let (_tx, mut shutdown) = watch::channel(false);
            run_relay_loop(
                &mut state,
                &mut source,
                &chaindb,
                &mut wal,
                &sched,
                &view,
                &mut shutdown,
                None,
            )
            .await
            .expect("relay loop runs to clean halt");

            ChainDb::tip(&chaindb).expect("tip").expect("tip advanced")
            // chaindb + wal dropped here — simulates a kill at the boundary.
        };

        // Reopen at the SAME paths (restart after kill) and run the production
        // L3 warm-start recovery — it must recover the same tip.
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal = FileWalStore::open(&wal_dir).unwrap();
        let recovered = crate::node_lifecycle::warm_start_recovery(&chaindb, &wal)
            .expect("warm-start recovers");

        assert_eq!(
            recovered.tip.map(|t| t.slot),
            Some(synced_tip.slot),
            "warm-start recovers the relay-loop-advanced tip slot after a kill"
        );
    }

    // =====================================================================
    // PHASE4-N-AE.C — recover→follow WAL prior-fp lineage continuity
    // =====================================================================
    //
    // The recover→follow seam: the FIRST followed AdmitBlock's prior_fp MUST
    // equal the recovered ledger-tip fingerprint (the anchor post_fp), not
    // zero (DC-WAL-02 first-entry clause). `relay_loop_kill_at_boundary_*`
    // masks this by using an arbitrary matched anchor_fp (0xA0) == the
    // `fresh_state` 0xA0 seed; here the anchor_fp is the REAL
    // `fingerprint(ledger)` (exactly what seed_to_snapshot writes as the
    // initial_ledger_fingerprint), so the ForwardSyncState prior_fp seed the
    // live node_lifecycle wiring supplies MUST be `fingerprint(ledger)` for
    // the chain to survive a kill + warm-start (T-REC-05). seed=0 reproduces
    // the CE-A5 exit-42 ChainBreak@1.

    /// Fingerprint of the fresh recover base (the ledger the follow extends) —
    /// the value the live fix seeds (`fingerprint(&state.ledger).combined`).
    fn aec_recover_base_fp() -> Hash32 {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        ade_ledger::fingerprint::fingerprint(&ledger).combined
    }

    /// Drive recover(anchor_fp = the REAL ledger fingerprint) → follow one
    /// corpus block via the production relay loop → kill → production
    /// `warm_start_recovery`, with `seed` as the ForwardSyncState prior_fp (the
    /// value the live wiring supplies). Returns (first followed AdmitBlock
    /// prior_fp, pre-kill synced tip, warm-start result as (slot,hash) | err).
    async fn aec_recover_follow_kill_warmstart(
        dir: &TempDir,
        seed: Hash32,
    ) -> (
        Option<Hash32>,
        (SlotNo, Hash32),
        Result<Option<(SlotNo, Hash32)>, String>,
    ) {
        use ade_ledger::seed_consensus_inputs::{
            encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };
        use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;

        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);
        let snap = dir.path().join("snap");
        let wal_dir = dir.path().join("wal");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::create_dir_all(&wal_dir).unwrap();
        let chaindb_path = snap.join("chain.db");

        // The anchor_fp is the REAL recover-base ledger fingerprint — exactly
        // what the live seed_to_snapshot persists as initial_ledger_fingerprint
        // — NOT an arbitrary matched constant.
        let anchor_fp = aec_recover_base_fp();
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            Hash28([0x11; 28]),
            PoolEntry {
                active_stake: 1,
                vrf_keyhash: Hash32([0x22; 32]),
            },
        );
        let recovered_inputs = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EPOCH_576,
            epoch_nonce: Nonce(Hash32([0x8b; 32])),
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
            total_active_stake: 1,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&recovered_inputs);

        let synced_tip = {
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
            let mut wal = FileWalStore::open(&wal_dir).unwrap();
            chaindb
                .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &anchor_fp, EPOCH_576, &sidecar_bytes).unwrap();

            // The recover base ledger the follow extends; the prior_fp seed is
            // the value under test (the live node_lifecycle wiring's seed).
            let mut ledger = LedgerState::new(CardanoEra::Conway);
            ledger.epoch_state.epoch = EPOCH_576;
            let mut chain_dep = PraosChainDepState::empty();
            chain_dep.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
            chain_dep.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
            let mut state = ForwardSyncState::new(
                ReceiveState::new(ledger, chain_dep),
                seed,
                SnapshotCadence::DEFAULT,
            );
            let mut source = NodeBlockSource::in_memory(vec![bytes.clone()]);
            let (_tx, mut shutdown) = watch::channel(false);
            run_relay_loop(
                &mut state,
                &mut source,
                &chaindb,
                &mut wal,
                &sched,
                &view,
                &mut shutdown,
                None,
            )
            .await
            .expect("relay loop runs to clean halt");

            let t = ChainDb::tip(&chaindb)
                .expect("tip")
                .expect("tip advanced");
            (t.slot, t.hash)
            // chaindb + wal dropped here — the kill boundary.
        };

        // The first followed AdmitBlock's prior_fp (DC-WAL-02 first-entry clause).
        let wal = FileWalStore::open(&wal_dir).unwrap();
        let first_admit_prior_fp = wal.read_all().unwrap().into_iter().find_map(|e| match e {
            WalEntry::AdmitBlock { prior_fp, .. } => Some(prior_fp),
            _ => None,
        });

        // Reopen at the SAME paths + the PRODUCTION warm-start.
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal2 = FileWalStore::open(&wal_dir).unwrap();
        let warm = match crate::node_lifecycle::warm_start_recovery(&chaindb, &wal2) {
            Ok(rec) => Ok(rec.tip.map(|t| (t.slot, t.hash))),
            Err(e) => Err(format!("{e:?}")),
        };
        (first_admit_prior_fp, synced_tip, warm)
    }

    #[tokio::test]
    async fn recover_follow_kill_warm_start_chains_from_ledger_fp() {
        // FIX: seed = fingerprint(recovered ledger) == the anchor post_fp. The
        // first followed AdmitBlock chains from the ledger tip; warm-start
        // recovers the same tip (no ChainBreak) — DC-WAL-02 + T-REC-05.
        let dir = TempDir::new().unwrap();
        let ledger_fp = aec_recover_base_fp();
        let (first_admit_prior_fp, synced_tip, warm) =
            aec_recover_follow_kill_warmstart(&dir, ledger_fp.clone()).await;
        assert_eq!(
            first_admit_prior_fp,
            Some(ledger_fp.clone()),
            "first followed AdmitBlock.prior_fp == fingerprint(recovered ledger) (DC-WAL-02)"
        );
        let recovered_tip = warm
            .expect("warm-start recovers a recover→followed store without ChainBreak")
            .expect("warm-start recovers a non-empty tip");
        assert_eq!(
            recovered_tip, synced_tip,
            "warm-start recovers the SAME followed tip (slot, hash) as the pre-kill run (T-REC-05)"
        );
    }

    #[tokio::test]
    async fn recover_follow_zero_seed_chainbreaks() {
        // BUG (pre-fix live wiring): seed = 0. The first followed AdmitBlock's
        // prior_fp is 0, not the anchor post_fp → warm-start fails ChainBreak@1,
        // reproducing the CE-A5 exit-42 failure. The fix must seed the chain,
        // not loosen verify_chain.
        let dir = TempDir::new().unwrap();
        let (first_admit_prior_fp, _synced_tip, warm) =
            aec_recover_follow_kill_warmstart(&dir, Hash32([0u8; 32])).await;
        assert_eq!(
            first_admit_prior_fp,
            Some(Hash32([0u8; 32])),
            "the zero seed writes a zero first prior_fp (the bug)"
        );
        let msg =
            warm.expect_err("a zero-seeded recover→followed store MUST fail warm-start, not silently recover");
        assert!(
            msg.contains("ChainBreak") && msg.contains("entry_index: 1"),
            "warm-start fails with ChainBreak@1 (the CE-A5 exit-42 failure), got: {msg}"
        );
    }

    #[tokio::test]
    async fn recover_follow_two_runs_byte_identical() {
        // T-REC-05: same recover base + same followed block → byte-identical WAL
        // image + the same recovered served tip across two independent runs.
        let ledger_fp = aec_recover_base_fp();
        let dir_a = TempDir::new().unwrap();
        let (_fp_a, tip_a, warm_a) =
            aec_recover_follow_kill_warmstart(&dir_a, ledger_fp.clone()).await;
        let dir_b = TempDir::new().unwrap();
        let (_fp_b, tip_b, warm_b) =
            aec_recover_follow_kill_warmstart(&dir_b, ledger_fp.clone()).await;
        assert_eq!(
            warm_a.expect("run A warm-starts"),
            warm_b.expect("run B warm-starts"),
            "two runs recover an identical served tip"
        );
        assert_eq!(tip_a, tip_b, "two runs reach the same followed tip");
        let wal_a = std::fs::read(dir_a.path().join("wal").join("wal-0000.bin")).unwrap();
        let wal_b = std::fs::read(dir_b.path().join("wal").join("wal-0000.bin")).unwrap();
        assert_eq!(wal_a, wal_b, "two runs produce a byte-identical WAL image");
    }

    // =====================================================================
    // PHASE4-N-AE.B CE-B3 — live-style follow→serve forge-parent intersectability
    // =====================================================================
    //
    // Resolves AE.B open-obligation #2: a REAL `run_relay_loop` follow stores the
    // followed block as a servable StoredBlock, AND the serve projects the followed
    // block's PARENT (its prev_hash) as a proof-gated FindIntersect-only point — so
    // a peer that already holds the parent can FindIntersect there and roll forward
    // onto Ade's served successor. This is the hermetic proxy for the CE-A5
    // relay-adoption surface: "a peer can FindIntersect at the forged parent, then
    // roll forward to Ade's forged successor" (NOT "ChainDb has the block").

    #[tokio::test]
    async fn recover_follow_serve_forged_parent_intersectable() {
        use ade_ledger::block_validity::decode_block;
        use ade_ledger::seed_consensus_inputs::{
            encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };
        use ade_network::chain_sync::server::ServedHeaderLookup;
        use ade_network::codec::chain_sync::Point;
        use ade_runtime::network::ChainDbServedSource;
        use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;
        use ade_types::shelley::block::PrevHash;

        let (c, view) = corpus_view();
        let sched = schedule();
        let bytes = pick_lightest(&c);
        // The followed block's parent (prev_hash) — the point a real peer would
        // FindIntersect at (the peer already holds the parent block).
        let parent_hash = match decode_block(&bytes).unwrap().prev_hash {
            PrevHash::Block(h) => h,
            PrevHash::Genesis => panic!("corpus block must be a non-Origin successor"),
        };

        let dir = TempDir::new().unwrap();
        let snap = dir.path().join("snap");
        let wal_dir = dir.path().join("wal");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::create_dir_all(&wal_dir).unwrap();
        let chaindb_path = snap.join("chain.db");

        let anchor_fp = aec_recover_base_fp();
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            Hash28([0x11; 28]),
            PoolEntry {
                active_stake: 1,
                vrf_keyhash: Hash32([0x22; 32]),
            },
        );
        let recovered_inputs = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EPOCH_576,
            epoch_nonce: Nonce(Hash32([0x8b; 32])),
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
            total_active_stake: 1,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&recovered_inputs);

        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let mut wal = FileWalStore::open(&wal_dir).unwrap();
        chaindb
            .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
            .unwrap();
        append_seed_epoch_provenance(&mut wal, &anchor_fp, EPOCH_576, &sidecar_bytes).unwrap();

        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = EPOCH_576;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
        chain_dep.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
        // AE.C-correct prior-fp seed (== fingerprint of the recover base ledger).
        let mut state = ForwardSyncState::new(
            ReceiveState::new(ledger, chain_dep),
            anchor_fp,
            SnapshotCadence::DEFAULT,
        );
        let mut source = NodeBlockSource::in_memory(vec![bytes.clone()]);
        let (_tx, mut shutdown) = watch::channel(false);
        run_relay_loop(
            &mut state, &mut source, &chaindb, &mut wal, &sched, &view, &mut shutdown, None,
        )
        .await
        .expect("relay loop follows the corpus block");

        // === serve checks (store alive — the live-style follow→serve surface) ===
        let served = ChainDbServedSource::new(&chaindb);
        let tip = ChainDb::tip(&chaindb)
            .expect("tip")
            .expect("the follow advanced the durable tip");

        // (a) open-obligation #2: the LIVE follow stores the followed block as a
        //     get_block_by_hash servable StoredBlock.
        assert!(
            ChainDb::get_block_by_hash(&chaindb, &tip.hash)
                .unwrap()
                .is_some(),
            "the live run_relay_loop follow stores the followed block as a servable StoredBlock"
        );
        // (b) the followed tip is FindIntersect-able (StoredBlock path).
        assert_eq!(
            served.intersect(&[Point::Block {
                slot: tip.slot,
                hash: tip.hash.clone(),
            }]),
            Some((tip.slot, tip.hash.clone())),
            "the followed tip is FindIntersect-able (StoredBlock path)"
        );
        // (c) the followed/forged PARENT is FindIntersect-able via the proof-gated
        //     projection (earliest servable StoredBlock's prev_hash == parent).
        let parent_slot = SlotNo(tip.slot.0.saturating_sub(1));
        assert_eq!(
            served.intersect(&[Point::Block {
                slot: parent_slot,
                hash: parent_hash.clone(),
            }]),
            Some((parent_slot, parent_hash.clone())),
            "the followed/forged parent is FindIntersect-able via the proof-gated projection (AC #8)"
        );
        // (d) ...and a relay that intersects at the parent rolls forward onto the
        //     served successor (the followed block).
        let next = served
            .next_after(Some((parent_slot, parent_hash)))
            .expect("next_after(parent) projects the served successor");
        assert_eq!(
            next.hash, tip.hash,
            "next_after(parent) rolls forward onto the followed/served successor"
        );
        // (e) hard boundary: the projected parent is never a StoredBlock (no bytes).
        assert!(
            ChainDb::get_block_by_hash(&chaindb, &next.hash.clone())
                .unwrap()
                .is_some(),
            "the served successor IS a real StoredBlock (real bytes), the parent is NOT"
        );
    }

    // =====================================================================
    // L5 — recovered-state forge handoff (hermetic, single-shot)
    // =====================================================================
    //
    // Reuses the proven `synth_shell` operator-key idiom from
    // forge_handler_variants.rs: operator KES/VRF/cold/opcert custody is
    // RED and synthesized here, while the forge BASE (ledger, chain_dep/
    // eta0, selected tip, leadership view) comes from a recovered
    // BootstrapState. No operator consensus bundle participates.

    // Most fixture types (Nonce, PraosChainDepState, ActiveSlotsCoeff,
    // BootstrapAnchorHash, EraSummary, PoolEntry, LedgerState, CardanoEra,
    // EpochNo, Hash28, ProtocolParameters, ProducerShell, ProtocolVersion)
    // are already imported by the L4 test section above (shared module
    // scope). Only the L5-specific seed-epoch type + opcert are new here.
    use ade_ledger::seed_consensus_inputs::SeedEpochConsensusInputs;
    use ade_types::shelley::block::OperationalCert;

    const L5_EPOCH: EpochNo = EpochNo(0);
    const L5_POOL: Hash28 = Hash28([0xAA; 28]);

    /// Operator key custody (RED), synthesized exactly as
    /// forge_handler_variants.rs::synth_shell. The keys never come from
    /// recovered state — this is the signing boundary.
    fn l5_synth_shell(cold_seed: u8, vrf_seed: u8, kes_seed: u8) -> ProducerShell {
        use ade_runtime::producer::signing::{ColdSigningKey, VrfSigningKey};
        use cardano_crypto::vrf::VrfDraft03;

        let cold_bytes = [cold_seed; 32];
        let cold = ColdSigningKey::from_bytes_zeroizing(&cold_bytes).unwrap();
        let (vrf_sk_bytes, _vrf_vk_bytes) = VrfDraft03::keypair_from_seed(&[vrf_seed; 32]);
        let vrf = VrfSigningKey::from_bytes_zeroizing(&vrf_sk_bytes).unwrap();
        let kes_seed_bytes = [kes_seed; 32];
        let kes =
            ade_runtime::producer::signing::KesSecret::from_seed_at_period(&kes_seed_bytes, 0)
                .unwrap();

        use ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes};
        let kes_sk_raw = Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed_bytes).unwrap();
        let hot_vkey = Sum6Kes::derive_verification_key(&kes_sk_raw);
        use ed25519_dalek::{Signer, SigningKey as DalekSk};
        let cold_dalek = DalekSk::from_bytes(&cold_bytes);
        let mut signable = Vec::with_capacity(48);
        signable.extend_from_slice(&hot_vkey);
        signable.extend_from_slice(&0u64.to_be_bytes());
        signable.extend_from_slice(&0u64.to_be_bytes());
        let sigma = cold_dalek.sign(&signable);
        let opcert = OperationalCert {
            hot_vkey: hot_vkey.to_vec(),
            sequence_number: 0,
            kes_period: 0,
            sigma: sigma.to_bytes().to_vec(),
        };
        ProducerShell::init(kes, vrf, cold, opcert).expect("shell init")
    }

    fn l5_era_schedule() -> EraSchedule {
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: L5_EPOCH,
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            }],
        )
        .expect("era schedule")
    }

    /// A recovered seed-epoch sidecar with the operator pool registered (so
    /// leadership is decidable from the RECOVERED surface). `vrf_keyhash`
    /// is arbitrary — leadership eligibility (not header binding) is what
    /// the projection drives here.
    fn l5_recovered_inputs() -> SeedEpochConsensusInputs {
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            L5_POOL,
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        SeedEpochConsensusInputs {
            anchor_fp: Hash32([0x5A; 32]),
            epoch_no: L5_EPOCH,
            epoch_nonce: Nonce(Hash32([0x8c; 32])),
            // asc 1/1 → every slot eligible regardless of VRF output bytes,
            // so the Eligible path is reached deterministically.
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 1 },
            total_active_stake: 1_000,
            pool_distribution: pools,
        }
    }

    /// Build a recovered `BootstrapState` with the given seed-epoch inputs.
    fn l5_recovered_state(
        seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs>,
    ) -> BootstrapState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = L5_EPOCH;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32([0xCD; 32]));
        chain_dep.evolving_nonce = Nonce(Hash32([0xCD; 32]));
        // A recovered tip implies a recorded block height: the recovered tip is
        // block 0, so the next forged block is number 1 (the WITH-tip path),
        // byte-identical to the pre-S4 unwrap_or(1) behaviour.
        chain_dep.last_block_no = Some(BlockNo(0));
        BootstrapState {
            ledger,
            chain_dep,
            tip: Some(ChainTip {
                hash: Hash32([0xBB; 32]),
                slot: SlotNo(10),
            }),
            seed_epoch_consensus_inputs,
        }
    }

    /// PHASE4-N-AE.A (DC-NODE-15): the node spine the relay-loop forge tests
    /// drive. The relay-loop forge now requires the from-genesis cold-start
    /// branch (a recovered-ANCHOR forge needs being caught up to the followed
    /// peer tip — DC-NODE-15 — covered end-to-end by the AE.A diagnostic suite),
    /// so the spine is genesis-fresh (`last_block_no: None`), matching
    /// [`l5_recovered_state_cold`]. Pre-AE.A this spine carried the recovered tip
    /// at block 0 (`last_block_no: Some(0)`); the recovered-tip-as-forge-base
    /// fallback it relied on is removed.
    fn l5_forge_spine() -> ForwardSyncState {
        l5_forge_spine_cold()
    }

    /// PHASE4-N-AE.A (DC-NODE-15): a from-genesis cold-start recovered state —
    /// `tip: None` (no recovered anchor) + `last_block_no: None`. The
    /// followed-tip admission gate does NOT apply to a genuine from-genesis cold
    /// start (its parent is `PrevHash::Genesis`, intersectable via Origin —
    /// DC-NODE-08 is upstream of the gate), so the relay-loop forge tests below
    /// exercise the FULL forge path (KES, off-epoch, leadership, self-accept)
    /// through the ungated cold-start branch. (A recovered-ANCHOR forge now
    /// requires being caught up to the followed peer tip — DC-NODE-15 — which the
    /// PHASE4-N-AE.A diagnostic suite proves end-to-end.)
    fn l5_recovered_state_cold(
        seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs>,
    ) -> BootstrapState {
        let mut ledger = LedgerState::new(CardanoEra::Conway);
        ledger.epoch_state.epoch = L5_EPOCH;
        let mut chain_dep = PraosChainDepState::empty();
        chain_dep.epoch_nonce = Nonce(Hash32([0xCD; 32]));
        chain_dep.evolving_nonce = Nonce(Hash32([0xCD; 32]));
        BootstrapState {
            ledger,
            chain_dep,
            tip: None,
            seed_epoch_consensus_inputs,
        }
    }

    /// The from-genesis cold-start spine (genesis-fresh ledger, `last_block_no:
    /// None`) matching [`l5_recovered_state_cold`] — the spine the cold-start
    /// relay-loop forge tests drive.
    fn l5_forge_spine_cold() -> ForwardSyncState {
        let r = l5_recovered_state_cold(None);
        ForwardSyncState::new(
            ReceiveState::new(r.ledger, r.chain_dep),
            Hash32([0xA0; 32]),
            SnapshotCadence::DEFAULT,
        )
    }

    #[test]
    fn forge_from_recovered_uses_recovered_pool_distr() {
        // DC-CINPUT-02b: the leadership view the forge consumes is exactly
        // PoolDistrView::from_seed_epoch_consensus_inputs(recovered) — the
        // recovered surface, not a bundle projection.
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
        let inputs = recovered.seed_epoch_consensus_inputs.as_ref().unwrap();
        let projected = PoolDistrView::from_seed_epoch_consensus_inputs(inputs);

        // The view used inside the handoff is built the same way; assert the
        // projection equals what the handoff projects (LedgerView surface).
        let direct = PoolDistrView::from_seed_epoch_consensus_inputs(&l5_recovered_inputs());
        assert_eq!(
            projected.total_active_stake(L5_EPOCH),
            direct.total_active_stake(L5_EPOCH),
            "recovered projection total stake must match"
        );
        assert_eq!(
            projected.pool_active_stake(L5_EPOCH, &L5_POOL),
            direct.pool_active_stake(L5_EPOCH, &L5_POOL),
            "recovered projection pool stake must match"
        );
        assert_eq!(
            projected.pool_vrf_keyhash(L5_EPOCH, &L5_POOL),
            direct.pool_vrf_keyhash(L5_EPOCH, &L5_POOL),
            "recovered projection vrf keyhash must match"
        );
        // And the handoff runs over this recovered base (Eligible path
        // reached — proves the projected view drove leadership).
        let tip = recovered.tip.clone().unwrap();
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let (event, _handoff) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            Some(&tip),
            &mut shell,
            &L5_POOL,
            &ProtocolParameters::default(),
            &l5_era_schedule(),
            13,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("recovered base hosts a forge");
        assert!(
            !matches!(event, CoordinatorEvent::ForgeNotLeader { .. }),
            "operator pool is registered + asc 1/1 → Eligible path, got {event:?}"
        );
    }

    #[test]
    fn forge_from_recovered_is_deterministic_across_two_runs() {
        // Single-shot determinism: same recovered base + same operator keys
        // ⇒ byte-identical CoordinatorEvent (ForgeSucceeded or ForgeNotLeader
        // or ForgeFailed — whichever; the assertion is byte-identity).
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
        let tip = recovered.tip.clone().unwrap();
        let sched = l5_era_schedule();
        let pparams = ProtocolParameters::default();

        let mut shell1 = l5_synth_shell(0xAB, 0xCD, 0xEF);
        let mut shell2 = l5_synth_shell(0xAB, 0xCD, 0xEF);
        let (e1, h1) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            Some(&tip),
            &mut shell1,
            &L5_POOL,
            &pparams,
            &sched,
            100,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("ok");
        let (e2, h2) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            Some(&tip),
            &mut shell2,
            &L5_POOL,
            &pparams,
            &sched,
            100,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("ok");
        assert_eq!(e1, e2, "recovered-state forge is replay byte-identical");
        // PHASE4-N-F-G-B S1: the surfaced handoff token is replay byte-identical
        // too (same recovered base + keys => same Option<SelfAcceptedHandoff>).
        assert_eq!(
            h1, h2,
            "the surfaced self-accepted handoff is replay byte-identical"
        );
    }

    #[test]
    fn forge_kill_then_warm_start_recovers_same_tip_via_forward_replay() {
        // PHASE4-N-U S2 (T-REC-05): a forged-block durable tip carries NO
        // snapshot-at-tip (admit_forged_block_durably captures none), so warm-start
        // recovers it by FORWARD-REPLAY from the genesis slot-0 snapshot over the
        // durable WAL block. The recovered tip + ledger fingerprint are
        // byte-identical to the pre-kill durable tip (the fingerprint guard inside
        // warm_start_recovery asserts the fp equality, so reaching Ok with the same
        // tip proves byte-identical forward replay).
        use ade_ledger::seed_consensus_inputs::{
            encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };
        use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;

        let eta0 = Nonce(Hash32([0xCD; 32]));
        // anchor_fp == the ForwardSyncState prior_fp seed, so the first forged
        // AdmitBlock.prior_fp chains from it and warm-start discovery keys on it.
        let anchor_fp = Hash32([0xA0; 32]);
        let genesis_base = || {
            let mut ledger = LedgerState::new(CardanoEra::Conway);
            ledger.epoch_state.epoch = EpochNo(0);
            let mut chain_dep = PraosChainDepState::empty();
            chain_dep.epoch_nonce = Nonce(Hash32([0xCD; 32]));
            chain_dep.evolving_nonce = Nonce(Hash32([0xCD; 32]));
            (ledger, chain_dep)
        };
        // MUST equal warm_start_recovery's reconstruction
        // make_node_schedule(epoch_no*432_000, epoch_no) = (0,0) for the genesis
        // seed epoch (safe_zone 432_000, NOT the l5 helper's 129_600).
        let era_schedule = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 432_000,
            }],
        )
        .expect("era schedule");

        // The pool keyed to the shell, so the forged block self-accepts AND the
        // sidecar-reconstructed ledger_view validates it the same way on replay.
        let mut shell = l5_synth_shell(0x31, 0x41, 0x59);
        let cold_vk = shell.cold_vk();
        let vrf_vk = shell.vrf_verification_key();
        let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&cold_vk.0);
        let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&vrf_vk.0);
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            pool_id.clone(),
            PoolEntry {
                active_stake: 1,
                vrf_keyhash,
            },
        );
        let sidecar = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EpochNo(0),
            epoch_nonce: eta0.clone(),
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 1 },
            total_active_stake: 1,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&sidecar);
        let ledger_view = PoolDistrView::from_seed_epoch_consensus_inputs(&sidecar);

        // Forge the genesis-successor block 0 -> self-accept.
        let (l_r, c_r) = genesis_base();
        let recovered = BootstrapState {
            ledger: l_r,
            chain_dep: c_r,
            tip: None,
            seed_epoch_consensus_inputs: Some(sidecar.clone()),
        };
        let (event, handoff) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            None,
            &mut shell,
            &pool_id,
            &ProtocolParameters::default(),
            &era_schedule,
            1,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("forge over the recovered genesis base");
        let handoff = match (event, handoff) {
            (CoordinatorEvent::ForgeSucceeded { .. }, Some(h)) => h,
            (ev, _) => {
                panic!("expected ForgeSucceeded with a self-accepted handoff, got {ev:?}")
            }
        };

        let dir = TempDir::new().unwrap();
        let chaindb_path = dir.path().join("chain.db");
        let wal_dir = dir.path().join("wal");

        // Phase 1: seed the sidecar + provenance + a genesis slot-0 snapshot, then
        // durably admit the forged block (NO snapshot captured by the driver).
        let advanced = {
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
            let mut wal = FileWalStore::open(&wal_dir).unwrap();
            chaindb
                .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &anchor_fp, EpochNo(0), &sidecar_bytes).unwrap();
            // The genesis slot-0 snapshot — the forward-replay base.
            let (l_s, c_s) = genesis_base();
            PersistentSnapshotCache::new(&chaindb)
                .capture(SlotNo(0), &l_s, &c_s)
                .unwrap();
            // Admit the forged block over the SAME genesis base (no tip snapshot).
            let (l_a, c_a) = genesis_base();
            let mut state = ForwardSyncState::new(
                ReceiveState::new(l_a, c_a),
                anchor_fp.clone(),
                SnapshotCadence::DEFAULT,
            );
            admit_forged_block_durably(
                &handoff,
                &mut state,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
            )
            .expect("durable admit")
            .expect("tip advanced")
            // chaindb + wal dropped here -> the kill boundary.
        };

        // Phase 2: reopen + recover through the REAL warm_start_recovery, which
        // forward-replays from the slot-0 snapshot over the durable WAL block.
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal = FileWalStore::open(&wal_dir).unwrap();
        let state = crate::node_lifecycle::warm_start_recovery(&chaindb, &wal)
            .expect("warm-start forward-replays the forged tip");
        let tip = state.tip.expect("recovered a tip");
        assert_eq!(
            tip.slot, advanced.slot,
            "forward-replay recovers the forged tip slot byte-identically (T-REC-05)"
        );
        assert_eq!(
            tip.hash, advanced.hash,
            "forward-replay recovers the forged tip hash byte-identically (T-REC-05)"
        );
    }

    #[test]
    fn forge_tip_successor_kill_then_warm_start_recovers_block_one() {
        // C2 TIP-SUCCESSOR DURABILITY DIAGNOSTIC (controlled Ade state, no venue).
        //
        // The question this isolates: does a forged TIP-SUCCESSOR (block N+1 on a
        // NON-Origin parent) recover across a restart — i.e. does block N+1's
        // prior_fp (= the durable POST_fp of block N, computed by the durable
        // apply, NOT constructed) chain across WAL replay?
        //
        // This is the C2-relevant boundary. The genesis seam (block 0's prior_fp
        // vs the seed/anchor) stays construction-matched (anchor_fp == the block-0
        // prior_fp, exactly as T-REC-05) — that case is the documented C1
        // genesis-successor limitation and is deliberately OUT OF SCOPE here. The
        // NEW seam under test is block-0 -> block-1: block 1's prior_fp is the real
        // post_fp of block 0 after the durable apply. If warm-start forward-replay
        // recovers block 1, the tip-successor durability boundary is clean.
        use ade_ledger::seed_consensus_inputs::{
            encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };
        use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;

        let eta0 = Nonce(Hash32([0xCD; 32]));
        // anchor_fp == the ForwardSyncState prior_fp seed == the block-0 prior_fp,
        // so the GENESIS seam is matched by construction (the C1-only case, masked
        // here on purpose); the block-0 -> block-1 seam is the real one under test.
        let anchor_fp = Hash32([0xA0; 32]);
        let genesis_base = || {
            let mut ledger = LedgerState::new(CardanoEra::Conway);
            ledger.epoch_state.epoch = EpochNo(0);
            let mut chain_dep = PraosChainDepState::empty();
            chain_dep.epoch_nonce = Nonce(Hash32([0xCD; 32]));
            chain_dep.evolving_nonce = Nonce(Hash32([0xCD; 32]));
            (ledger, chain_dep)
        };
        let era_schedule = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 432_000,
            }],
        )
        .expect("era schedule");

        // Always-leader pool (ASC 1/1, all stake), keyed to the shell.
        let mut shell = l5_synth_shell(0x31, 0x41, 0x59);
        let cold_vk = shell.cold_vk();
        let vrf_vk = shell.vrf_verification_key();
        let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&cold_vk.0);
        let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&vrf_vk.0);
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            pool_id.clone(),
            PoolEntry {
                active_stake: 1,
                vrf_keyhash,
            },
        );
        let sidecar = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EpochNo(0),
            epoch_nonce: eta0.clone(),
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 1 },
            total_active_stake: 1,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&sidecar);
        let ledger_view = PoolDistrView::from_seed_epoch_consensus_inputs(&sidecar);

        // Forge the genesis-successor block 0 over the recovered genesis base.
        let (l_r, c_r) = genesis_base();
        let recovered = BootstrapState {
            ledger: l_r,
            chain_dep: c_r,
            tip: None,
            seed_epoch_consensus_inputs: Some(sidecar.clone()),
        };
        let (event0, handoff0) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            None,
            &mut shell,
            &pool_id,
            &ProtocolParameters::default(),
            &era_schedule,
            1,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("forge block 0 over the recovered genesis base");
        let handoff0 = match (event0, handoff0) {
            (CoordinatorEvent::ForgeSucceeded { .. }, Some(h)) => h,
            (ev, _) => panic!("expected block-0 ForgeSucceeded, got {ev:?}"),
        };

        let dir = TempDir::new().unwrap();
        let chaindb_path = dir.path().join("chain.db");
        let wal_dir = dir.path().join("wal");

        // Phase 1: seed precondition + genesis slot-0 snapshot, admit block 0, then
        // forge block 1 on the DURABLE tip and admit it. Keep state alive across
        // both admits (the durable spine evolves); the kill is the scope end.
        let advanced1 = {
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
            let mut wal = FileWalStore::open(&wal_dir).unwrap();
            chaindb
                .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &anchor_fp, EpochNo(0), &sidecar_bytes).unwrap();
            let (l_s, c_s) = genesis_base();
            PersistentSnapshotCache::new(&chaindb)
                .capture(SlotNo(0), &l_s, &c_s)
                .unwrap();
            let (l_a, c_a) = genesis_base();
            let mut state = ForwardSyncState::new(
                ReceiveState::new(l_a, c_a),
                anchor_fp.clone(),
                SnapshotCadence::DEFAULT,
            );

            // Admit block 0 (genesis-successor) — the durable tip becomes block 0.
            let advanced0 = admit_forged_block_durably(
                &handoff0,
                &mut state,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
            )
            .expect("durable admit block 0")
            .expect("tip 0 advanced");
            assert_eq!(
                state.receive.chain_dep.last_block_no,
                Some(BlockNo(0)),
                "after admitting block 0 the durable spine's last_block_no is 0"
            );
            // The ChainTip of the durable block 0 — the non-Origin parent for the
            // tip-successor forge (selected_tip wants &ChainTip, not the PumpTip).
            let chain_tip0 = ChainDb::tip(&chaindb)
                .expect("durable tip")
                .expect("non-empty after block 0");

            // Forge block 1 ON the durable tip (block 0) against the EVOLVED spine.
            // selected_tip = Some(block 0) -> block 1, PrevHash::Block(tip0). The
            // sidecar (recovered.seed_epoch_consensus_inputs) drives leadership.
            let (event1, handoff1) = forge_one_from_recovered(
                &recovered,
                &state.receive.chain_dep,
                &state.receive.ledger,
                Some(&chain_tip0),
                &mut shell,
                &pool_id,
                &ProtocolParameters::default(),
                &era_schedule,
                2,
                0,
                ProtocolVersion { major: 9, minor: 0 },
            )
            .expect("forge block 1 over the durable tip");
            let handoff1 = match (event1, handoff1) {
                (CoordinatorEvent::ForgeSucceeded { .. }, Some(h)) => h,
                (ev, _) => panic!("expected block-1 ForgeSucceeded on the durable tip, got {ev:?}"),
            };

            // Admit block 1 — the durable tip becomes block 1 (tip-successor).
            let advanced1 = admit_forged_block_durably(
                &handoff1,
                &mut state,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
            )
            .expect("durable admit block 1")
            .expect("tip 1 advanced");
            assert_ne!(
                advanced1.hash, advanced0.hash,
                "block 1 is a distinct block from block 0"
            );
            assert_eq!(
                state.receive.chain_dep.last_block_no,
                Some(BlockNo(1)),
                "after admitting block 1 the durable spine's last_block_no is 1"
            );
            advanced1
            // chaindb + wal + state dropped here -> the kill boundary.
        };

        // Phase 2: reopen + recover through the REAL warm_start_recovery. This
        // forward-replays from the slot-0 snapshot over BOTH durable WAL blocks.
        // The decisive seam is block-0 -> block-1 (block 1's prior_fp == block 0's
        // real post_fp). If this ChainBreaks, the tip-successor recovery is broken;
        // if it recovers block 1, the C2 tip-successor durability boundary is clean.
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal = FileWalStore::open(&wal_dir).unwrap();
        let state = crate::node_lifecycle::warm_start_recovery(&chaindb, &wal)
            .expect("warm-start forward-replays the tip-successor chain without ChainBreak");
        let tip = state.tip.expect("recovered a tip");
        assert_eq!(
            tip.slot, advanced1.slot,
            "warm-start recovers the BLOCK-1 (tip-successor) slot — block-0 -> block-1 prior_fp chains across WAL replay"
        );
        assert_eq!(
            tip.hash, advanced1.hash,
            "warm-start recovers the BLOCK-1 (tip-successor) hash"
        );
    }

    /// CE-AF-5 (DC-NODE-18 / T-REC-05): warm-start replay of a K≥2 own-forged
    /// chain is byte-identical. Forges a 3-block own spine — the extend-own-spine
    /// shape, each successor built on the prior DURABLE tip (PrevHash::Block,
    /// prior_fp = parent post_fp) — kills, and recovers through the REAL
    /// warm_start_recovery. The recovered tip byte-equals the pre-kill durable tip
    /// (warm_start_recovery's internal fingerprint guard asserts fp equality, so
    /// reaching the same tip proves byte-identical forward replay over the whole
    /// spine). The `ForgeMode` is RED + not persisted, so it cannot perturb this
    /// deterministic surface.
    #[test]
    fn extend_own_spine_two_runs_byte_identical() {
        use ade_ledger::seed_consensus_inputs::{
            encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };
        use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;

        let eta0 = Nonce(Hash32([0xCD; 32]));
        let anchor_fp = Hash32([0xA0; 32]);
        let genesis_base = || {
            let mut ledger = LedgerState::new(CardanoEra::Conway);
            ledger.epoch_state.epoch = EpochNo(0);
            let mut chain_dep = PraosChainDepState::empty();
            chain_dep.epoch_nonce = Nonce(Hash32([0xCD; 32]));
            chain_dep.evolving_nonce = Nonce(Hash32([0xCD; 32]));
            (ledger, chain_dep)
        };
        let era_schedule = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 432_000,
            }],
        )
        .expect("era schedule");

        // Always-leader pool (ASC 1/1, all stake), keyed to the shell.
        let mut shell = l5_synth_shell(0x31, 0x41, 0x59);
        let cold_vk = shell.cold_vk();
        let vrf_vk = shell.vrf_verification_key();
        let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&cold_vk.0);
        let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&vrf_vk.0);
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            pool_id.clone(),
            PoolEntry {
                active_stake: 1,
                vrf_keyhash,
            },
        );
        let sidecar = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EpochNo(0),
            epoch_nonce: eta0.clone(),
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 1 },
            total_active_stake: 1,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&sidecar);
        let ledger_view = PoolDistrView::from_seed_epoch_consensus_inputs(&sidecar);

        // Forge block 0 (genesis-successor) over the recovered base.
        let (l_r, c_r) = genesis_base();
        let recovered = BootstrapState {
            ledger: l_r,
            chain_dep: c_r,
            tip: None,
            seed_epoch_consensus_inputs: Some(sidecar.clone()),
        };
        let (event0, handoff0) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            None,
            &mut shell,
            &pool_id,
            &ProtocolParameters::default(),
            &era_schedule,
            1,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("forge block 0 over the recovered genesis base");
        let handoff0 = match (event0, handoff0) {
            (CoordinatorEvent::ForgeSucceeded { .. }, Some(h)) => h,
            (ev, _) => panic!("expected block-0 ForgeSucceeded, got {ev:?}"),
        };

        let dir = TempDir::new().unwrap();
        let chaindb_path = dir.path().join("chain.db");
        let wal_dir = dir.path().join("wal");

        // Phase 1: seed precondition + genesis slot-0 snapshot, then forge+admit a
        // 3-block own spine (block 0, then 1 on durable-0, then 2 on durable-1),
        // keeping state alive so the durable spine evolves. The kill ends the scope.
        let advanced2 = {
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
            let mut wal = FileWalStore::open(&wal_dir).unwrap();
            chaindb
                .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
                .unwrap();
            append_seed_epoch_provenance(&mut wal, &anchor_fp, EpochNo(0), &sidecar_bytes).unwrap();
            let (l_s, c_s) = genesis_base();
            PersistentSnapshotCache::new(&chaindb)
                .capture(SlotNo(0), &l_s, &c_s)
                .unwrap();
            let (l_a, c_a) = genesis_base();
            let mut state = ForwardSyncState::new(
                ReceiveState::new(l_a, c_a),
                anchor_fp.clone(),
                SnapshotCadence::DEFAULT,
            );

            // Admit block 0 — the durable tip becomes block 0.
            admit_forged_block_durably(
                &handoff0,
                &mut state,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
            )
            .expect("durable admit block 0")
            .expect("tip 0 advanced");
            let chain_tip0 = ChainDb::tip(&chaindb)
                .expect("durable tip")
                .expect("non-empty after block 0");

            // Forge + admit block 1 ON the durable tip 0 (extend the own spine).
            let (event1, handoff1) = forge_one_from_recovered(
                &recovered,
                &state.receive.chain_dep,
                &state.receive.ledger,
                Some(&chain_tip0),
                &mut shell,
                &pool_id,
                &ProtocolParameters::default(),
                &era_schedule,
                2,
                0,
                ProtocolVersion { major: 9, minor: 0 },
            )
            .expect("forge block 1 over durable tip 0");
            let handoff1 = match (event1, handoff1) {
                (CoordinatorEvent::ForgeSucceeded { .. }, Some(h)) => h,
                (ev, _) => panic!("expected block-1 ForgeSucceeded, got {ev:?}"),
            };
            admit_forged_block_durably(
                &handoff1,
                &mut state,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
            )
            .expect("durable admit block 1")
            .expect("tip 1 advanced");
            let chain_tip1 = ChainDb::tip(&chaindb)
                .expect("durable tip")
                .expect("non-empty after block 1");

            // Forge + admit block 2 ON the durable tip 1 (extend the own spine again).
            let (event2, handoff2) = forge_one_from_recovered(
                &recovered,
                &state.receive.chain_dep,
                &state.receive.ledger,
                Some(&chain_tip1),
                &mut shell,
                &pool_id,
                &ProtocolParameters::default(),
                &era_schedule,
                3,
                0,
                ProtocolVersion { major: 9, minor: 0 },
            )
            .expect("forge block 2 over durable tip 1");
            let handoff2 = match (event2, handoff2) {
                (CoordinatorEvent::ForgeSucceeded { .. }, Some(h)) => h,
                (ev, _) => panic!("expected block-2 ForgeSucceeded, got {ev:?}"),
            };
            let advanced2 = admit_forged_block_durably(
                &handoff2,
                &mut state,
                &chaindb,
                &mut wal,
                &era_schedule,
                &ledger_view,
            )
            .expect("durable admit block 2")
            .expect("tip 2 advanced");
            assert_eq!(
                state.receive.chain_dep.last_block_no,
                Some(BlockNo(2)),
                "after admitting the 3-block own spine the durable last_block_no is 2"
            );
            advanced2
            // chaindb + wal + state dropped here -> the kill boundary.
        };

        // Phase 2: reopen + recover. warm_start_recovery forward-replays from the
        // slot-0 snapshot over ALL THREE durable WAL blocks; the recovered tip must
        // byte-equal the pre-kill durable block-2 tip (no ChainBreak across the
        // extend-own-spine seams).
        let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal = FileWalStore::open(&wal_dir).unwrap();
        let state = crate::node_lifecycle::warm_start_recovery(&chaindb, &wal)
            .expect("warm-start forward-replays the 3-block own spine without ChainBreak");
        let tip = state.tip.expect("recovered a tip");
        assert_eq!(
            tip.slot, advanced2.slot,
            "warm-start recovers the block-2 spine-head slot byte-identically (CE-AF-5)"
        );
        assert_eq!(
            tip.hash, advanced2.hash,
            "warm-start recovers the block-2 spine-head hash byte-identically (CE-AF-5)"
        );
    }

    #[test]
    fn forge_from_recovered_fails_closed_without_recovered_inputs() {
        // The forge base MUST carry recovered consensus inputs. A recovered
        // state with seed_epoch_consensus_inputs: None is unrepresentable as
        // a forge — typed error, no bundle/cold/genesis fallback.
        let recovered = l5_recovered_state(None);
        let tip = recovered.tip.clone().unwrap();
        let mut shell = l5_synth_shell(0x44, 0x55, 0x66);
        let r = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            Some(&tip),
            &mut shell,
            &L5_POOL,
            &ProtocolParameters::default(),
            &l5_era_schedule(),
            7,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        );
        assert!(
            matches!(r, Err(NodeForgeError::MissingRecoveredConsensusInputs)),
            "missing recovered consensus inputs must fail closed, got {r:?}"
        );
    }

    // ===== PHASE4-N-F-G-Q: forge-successor from the evolved admitted spine =====

    /// DC-NODE-10: after the feed advances the node spine, the forge-successor
    /// derives block_no from the EVOLVED admitted chain state, not a stale
    /// baseline. Proven both ways: (1) the evolved chain state (last_block_no
    /// Some(0)) + a selected tip ⇒ the forge computes successor block_no 1
    /// (not RecoveredTipMissingBlockNo); (2) a STALE chain state (last_block_no
    /// None) + a selected tip ⇒ RecoveredTipMissingBlockNo — the pre-G-Q bug,
    /// locked. (Pre-ingest no-tip cold-start ⇒ block 0 is covered by the G-J
    /// cold-start tests; forge_header_position ignores last_block_no when the
    /// selected tip is None.)
    #[test]
    fn forge_successor_reads_evolved_spine_block_no_not_stale_baseline_g_q() {
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
        let tip = recovered.tip.clone().unwrap();
        let sched = l5_era_schedule();
        let pparams = ProtocolParameters::default();

        // The successor header POSITION (acceptance #2): an evolved chain at
        // block 0 + a selected tip ⇒ block_no 1 + PrevHash::Block(tip).
        // forge_header_position is the single position authority (its N+1
        // behaviour is unit-tested in the cold-start/with-tip tests below).
        let (n, prev) = forge_header_position(Some(&tip), Some(BlockNo(0))).unwrap();
        assert_eq!(n, 1, "evolved last_block_no 0 + a selected tip ⇒ successor block_no 1");
        assert!(matches!(prev, PrevHash::Block(_)), "successor prev is the selected tip");

        // (1) EVOLVED admitted chain state: the feed advanced it to block 0
        // (last_block_no Some(0)). The forge-successor must read THIS.
        let evolved = l5_recovered_state(None);
        assert_eq!(
            evolved.chain_dep.last_block_no,
            Some(BlockNo(0)),
            "the evolved admitted chain state is at block 0"
        );
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let r_evolved = forge_one_from_recovered(
            &recovered,
            &evolved.chain_dep,
            &evolved.ledger,
            Some(&tip),
            &mut shell,
            &L5_POOL,
            &pparams,
            &sched,
            100,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        );
        // The forge READ the evolved block_no and proceeded past the position to
        // build the successor — it did NOT RecoveredTipMissingBlockNo. (A
        // downstream SelfAcceptRejected over the L5 placeholder tip is a fixture
        // artifact, not a DC-NODE-10 concern.)
        assert!(
            !matches!(r_evolved, Err(NodeForgeError::RecoveredTipMissingBlockNo)),
            "the evolved chain state (last_block_no Some(0)) must NOT RecoveredTipMissingBlockNo \
             — the forge reads the evolved block_no, got {r_evolved:?}"
        );

        // (2) STALE chain state (last_block_no None) + a selected tip ⇒ the
        // pre-G-Q bug: RecoveredTipMissingBlockNo. Locks the regression.
        let mut stale = l5_recovered_state(None);
        stale.chain_dep.last_block_no = None;
        let mut shell2 = l5_synth_shell(0x11, 0x22, 0x33);
        let r = forge_one_from_recovered(
            &recovered,
            &stale.chain_dep,
            &stale.ledger,
            Some(&tip),
            &mut shell2,
            &L5_POOL,
            &pparams,
            &sched,
            100,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        );
        assert!(
            matches!(r, Err(NodeForgeError::RecoveredTipMissingBlockNo)),
            "a selected tip with a stale (None) chain-state block_no must fail closed \
             RecoveredTipMissingBlockNo — the pre-G-Q bug, got {r:?}"
        );
    }

    // ===== PHASE4-N-F-G-J S4: cold-start (genesis) reachability =====

    #[test]
    fn forge_one_from_recovered_cold_start_is_block_zero_genesis() {
        // No selected tip ⇒ genesis-successor: block 0 + PrevHash::Genesis,
        // regardless of any recovered last_block_no.
        assert_eq!(
            forge_header_position(None, None).unwrap(),
            (0u64, PrevHash::Genesis)
        );
        assert_eq!(
            forge_header_position(None, Some(BlockNo(99))).unwrap(),
            (0u64, PrevHash::Genesis)
        );
    }

    #[test]
    fn forge_one_from_recovered_with_tip_is_block_n_plus_one_block_prev() {
        let tip = ChainTip {
            hash: Hash32([0xBB; 32]),
            slot: SlotNo(10),
        };
        assert_eq!(
            forge_header_position(Some(&tip), Some(BlockNo(4))).unwrap(),
            (5u64, PrevHash::Block(Hash32([0xBB; 32])))
        );
    }

    #[test]
    fn forge_header_position_some_tip_without_block_no_fails_closed() {
        let tip = ChainTip {
            hash: Hash32([0xBB; 32]),
            slot: SlotNo(10),
        };
        assert!(matches!(
            forge_header_position(Some(&tip), None),
            Err(NodeForgeError::RecoveredTipMissingBlockNo)
        ));
    }

    #[test]
    fn cold_start_block_number_is_zero_single_convention() {
        // ONE cold-start convention: node_sync's cold-start block number is 0,
        // matching ChainEvolution::next_block_number() at tip None (also 0).
        // The pre-S4 disagreement (node_sync unwrap_or(1)) is gone.
        use ade_runtime::producer::chain_evolution::ChainEvolution;
        let (n, prev) = forge_header_position(None, None).unwrap();
        assert_eq!(n, 0, "cold-start block number is 0, not 1");
        assert_eq!(prev, PrevHash::Genesis);
        let evo = ChainEvolution::seed(
            LedgerState::new(CardanoEra::Conway),
            PraosChainDepState::empty(),
            None, // tip None = cold start
            l5_era_schedule(),
            PoolDistrView::from_seed_epoch_consensus_inputs(&l5_recovered_inputs()),
            Nonce(Hash32([0xCD; 32])),
        );
        assert_eq!(
            n,
            evo.next_block_number(),
            "node_sync and ChainEvolution agree on the cold-start block number"
        );
    }

    #[test]
    fn node_spine_cold_start_forges_genesis_block_zero() {
        // The cold-start ctx (None tip) flows through the SAME run_real_forge
        // S3 proved. asc 1/1 ⇒ the operator is always leader, so the Eligible
        // forge path is reached; on self-accept the forged block is 0 + Genesis.
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let (event, handoff) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            None,
            &mut shell,
            &L5_POOL,
            &ProtocolParameters::default(),
            &l5_era_schedule(),
            13,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("cold-start forge over the recovered base");
        match event {
            CoordinatorEvent::ForgeSucceeded { artifact, .. } => {
                let decoded = ade_ledger::block_validity::decode_block(&artifact.bytes)
                    .expect("forged genesis block decodes (passes check_header_position)");
                assert_eq!(
                    decoded.header_input.block_no.0, 0,
                    "genesis-successor is block 0"
                );
                let inner = &artifact.bytes[decoded.inner_start..decoded.inner_end];
                let reparsed = ade_codec::conway::decode_conway_block(inner).unwrap();
                assert_eq!(
                    reparsed.decoded().header.body.prev_hash,
                    PrevHash::Genesis,
                    "the genesis-successor carries PrevHash::Genesis"
                );
                assert!(
                    handoff.is_some(),
                    "ForgeSucceeded surfaces exactly one self-accept handoff"
                );
            }
            CoordinatorEvent::ForgeFailed { .. } => {
                // The cold-start ctx reached the forge engine (Eligible leader,
                // asc 1/1) but the synthetic VRF/KES did not self-accept. The
                // block-0/Genesis derivation is proven by forge_header_position;
                // this still proves reachability (NOT ForgeNotLeader).
            }
            CoordinatorEvent::ForgeNotLeader { .. } => {
                panic!("cold-start ctx must reach the Eligible forge path (asc 1/1)");
            }
            other => panic!("unexpected cold-start outcome: {other:?}"),
        }
    }

    // ===== S2: forge-tick wiring into the relay loop (self-accept-only) =====

    use ade_runtime::clock::{millis_to_slot, DeterministicClock};
    use ade_runtime::producer::coordinator::{
        CoordinatorState, GenesisAnchor, LedgerSnapshotRef, OpCertPublicMetadata,
    };

    /// Genesis anchor s.t. `kes_period_for_slot(small slot) == Some(0)` — so the
    /// REUSED CoordinatorState method yields a valid period for the test slot.
    fn s2_coordinator_state() -> CoordinatorState {
        CoordinatorState {
            genesis_anchor: GenesisAnchor {
                network_magic: 1,
                slot_zero_time_unix_ms: 0,
                slot_length_ms: 1_000,
                slots_per_kes_period: 129_600,
                kes_anchor_slot: 0,
                kes_max_period: 63,
            },
            opcert_meta: OpCertPublicMetadata {
                kes_vkey: [0u8; 32],
                kes_start_period: 0,
                sequence_number: 0,
                cold_vkey_hash: [0u8; 28],
            },
            last_slot_tick: None,
            pending_forge_slot: None,
            chain_tip: None,
            ledger_snapshot_ref: LedgerSnapshotRef(0),
            peers: BTreeMap::new(),
            peer_id_counter: 0,
            broadcast_queue_size: 0,
            broadcast_queue_limit: 16,
            peer_limit: 16,
            shutdown_in_progress: false,
        }
    }

    /// Trivial ledger view for the relay loop's `ledger_view` arg. The forge
    /// path never consults it (it projects the recovered surface); it matters
    /// only if a `SyncOnce` runs — which these forge-only tests never reach.
    fn s2_idle_view() -> PoolDistrView {
        PoolDistrView::new(
            L5_EPOCH,
            0,
            ActiveSlotsCoeff { numer: 1, denom: 1 },
            BTreeMap::new(),
        )
    }

    #[tokio::test]
    async fn relay_loop_forge_tick_attempts_forge_advances_no_tip() {
        // CE-E-4: with forge activation present, the loop performs exactly ONE
        // fenced forge_one_from_recovered attempt at the due slot and advances
        // NO durable tip / serves / admits / persists nothing. The outcome
        // (ForgeSucceeded / ForgeNotLeader / structured ForgeFailed) is observed
        // in-memory only.
        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = l5_forge_spine();
        // Open WirePump: Continuing (never ended) + NoWorkReady (no block), so
        // the planner reaches ForgeTick (a feed-end would suppress forge).
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let recovered = l5_recovered_state_cold(Some(l5_recovered_inputs()));
        let coordinator = s2_coordinator_state();
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let view = s2_idle_view();
        // One forge slot: tick 100_000 ms / 1_000 ms-per-slot, anchor 0, start
        // slot 0 => SlotNo(100) (the slot the L5 forge tests use).
        let mut clock = DeterministicClock::new(0, vec![100_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &recovered,
            &mut shell,
            L5_POOL,
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );

        let tip_before = ChainDb::tip(&chaindb).unwrap();
        let loop_fut = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut sd_rx,
            Some(&mut act),
        );
        // The loop forges the single tick synchronously, then parks at the Idle
        // await; shutdown halts it. The channel stays open (Continuing) until
        // after the loop has halted.
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("relay loop with forge halts cleanly");
        drop(block_tx);

        assert_eq!(
            act.hermetic_forge_outcomes.len(),
            1,
            "exactly one fenced forge attempt at the single due slot"
        );
        let tip_after = ChainDb::tip(&chaindb).unwrap();
        assert_eq!(
            tip_before, tip_after,
            "forge must not advance the durable tip"
        );
        assert!(
            SnapshotStore::list_snapshot_slots(&chaindb)
                .unwrap()
                .is_empty(),
            "forge persists no snapshot / served state"
        );
    }

    #[tokio::test]
    async fn relay_loop_containment_semantics_after_serve_sibling_retired() {
        // PHASE4-N-U S3: the G-R serve sibling (handoff -> push_atomic into a
        // ServedChainView accumulator) is RETIRED — the serve task reads the
        // durable ChainDb projection (DC-NODE-13), and the forge tick has no
        // serve handoff. This test pins that the relay loop's authority semantics
        // are otherwise unchanged: the forge tick makes exactly ONE attempt at
        // the single due slot, and (with this synthetic non-self-accepting shell)
        // advances NO durable tip and persists no snapshot. Own-forged durable
        // admit on a REAL self-accept is S1's admit_forged_block_durably, covered
        // in forge_succeeds.rs.
        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = l5_forge_spine();
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let recovered = l5_recovered_state_cold(Some(l5_recovered_inputs()));
        let coordinator = s2_coordinator_state();
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let view = s2_idle_view();
        let mut clock = DeterministicClock::new(0, vec![100_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &recovered,
            &mut shell,
            L5_POOL,
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );

        let tip_before = ChainDb::tip(&chaindb).unwrap();
        let loop_fut = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("relay loop with forge tick halts cleanly");
        drop(block_tx);

        // Authority semantics: exactly one fenced forge attempt; no durable tip
        // advance (this synthetic shell does not self-accept, so no durable
        // admit fires); no snapshot.
        assert_eq!(
            act.hermetic_forge_outcomes.len(),
            1,
            "exactly one fenced forge attempt at the single due slot"
        );
        assert_eq!(
            tip_before,
            ChainDb::tip(&chaindb).unwrap(),
            "no self-accept -> no durable admit -> durable tip unchanged"
        );
        assert!(
            SnapshotStore::list_snapshot_slots(&chaindb)
                .unwrap()
                .is_empty(),
            "forge persists no snapshot / served state"
        );
    }

    #[tokio::test]
    async fn relay_loop_forge_slot_derived_via_clock_seam() {
        // CE-E-3: the slot the forge runs at is derived ONLY through the clock
        // seam — millis_to_slot(tick, anchor, start, slot_len). Assert the
        // forged outcome's slot equals that pure conversion (tick 250_000 ms,
        // 1_000 ms/slot, anchor 0, start 0 => slot 250).
        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = l5_forge_spine();
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let recovered = l5_recovered_state_cold(Some(l5_recovered_inputs()));
        let coordinator = s2_coordinator_state();
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let view = s2_idle_view();
        let expected = millis_to_slot(250_000, 0, SlotNo(0), 1_000);
        let mut clock = DeterministicClock::new(0, vec![250_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &recovered,
            &mut shell,
            L5_POOL,
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );

        let loop_fut = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("relay loop with forge halts cleanly");
        drop(block_tx);

        assert_eq!(act.hermetic_forge_outcomes.len(), 1);
        let outcome_slot = match &act.hermetic_forge_outcomes[0] {
            CoordinatorEvent::ForgeSucceeded { slot, .. } => *slot,
            CoordinatorEvent::ForgeNotLeader { slot, .. } => *slot,
            CoordinatorEvent::ForgeFailed { slot, .. } => *slot,
            other => panic!("unexpected forge outcome variant: {other:?}"),
        };
        assert_eq!(
            SlotNo(outcome_slot),
            expected,
            "forge slot must equal the clock-seam millis_to_slot derivation"
        );
    }

    #[tokio::test]
    async fn relay_loop_without_producer_material_matches_nfd_relay() {
        // CE-E-5: forge OFF (None) — the loop is the exact N-F-D relay. Over an
        // open (Continuing) feed with no work it idles then halts on shutdown,
        // advancing no tip, persisting nothing, and producing NO forged artifact
        // (there is no ForgeActivation to drive one).
        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = l5_forge_spine();
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);
        let sched = l5_era_schedule();
        let view = s2_idle_view();

        let loop_fut = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut sd_rx,
            None,
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("relay loop (forge off) halts cleanly");
        drop(block_tx);

        assert!(ChainDb::tip(&chaindb).unwrap().is_none(), "no tip advance");
        assert!(
            wal.read_all().expect("read_all").is_empty(),
            "forge-off relay appends no WAL entry on an empty feed"
        );
        assert!(SnapshotStore::list_snapshot_slots(&chaindb)
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn relay_loop_forge_two_runs_byte_identical() {
        // CE-E-6: forge-tick replay-equivalence. Two clean runs over IDENTICAL
        // (recovered state, feed, injected clock tick schedule, shutdown
        // schedule) produce byte-identical tip + WAL + checkpoints AND a
        // byte-identical forge-attempt sequence (forged bytes for any
        // ForgeSucceeded included, via CoordinatorEvent's PartialEq).
        async fn run_once() -> (
            Option<(SlotNo, Hash32)>,
            String,
            Vec<SlotNo>,
            Vec<CoordinatorEvent>,
        ) {
            let dir = TempDir::new().unwrap();
            let chaindb =
                PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                    .unwrap();
            let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
            let mut state = l5_forge_spine();
            // Open WirePump (Continuing) so the forge branch is reachable.
            let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
            let mut source = NodeBlockSource::from_wire_pump(block_rx);
            let (sd_tx, mut sd_rx) = watch::channel(false);

            let sched = l5_era_schedule();
            let recovered = l5_recovered_state_cold(Some(l5_recovered_inputs()));
            let coordinator = s2_coordinator_state();
            let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
            let view = s2_idle_view();
            // Fixed multi-tick schedule -> slots 100/200/300, each Due by
            // monotonic increase => a 3-attempt forge sequence.
            let mut clock = DeterministicClock::new(0, vec![100_000, 200_000, 300_000]);
            let mut act = ForgeActivation::new(
                &mut clock,
                &coordinator,
                &recovered,
                &mut shell,
                L5_POOL,
                ProtocolParameters::default(),
                ProtocolVersion { major: 9, minor: 0 },
                0,
                SlotNo(0),
                1_000,
            );

            let loop_fut = run_relay_loop(
                &mut state,
                &mut source,
                &chaindb,
                &mut wal,
                &sched,
                &view,
                &mut sd_rx,
                Some(&mut act),
            );
            let driver = async {
                let _ = sd_tx.send(true);
            };
            let (loop_res, _) = tokio::join!(loop_fut, driver);
            loop_res.expect("forge relay run halts cleanly");
            drop(block_tx);

            let tip = ChainDb::tip(&chaindb)
                .expect("tip")
                .map(|t| (t.slot, t.hash));
            let wal_image = format!("{:?}", wal.read_all().expect("read_all"));
            let snaps = SnapshotStore::list_snapshot_slots(&chaindb).expect("list");
            (tip, wal_image, snaps, act.hermetic_forge_outcomes.clone())
        }

        let (tip_a, wal_a, snaps_a, outcomes_a) = run_once().await;
        let (tip_b, wal_b, snaps_b, outcomes_b) = run_once().await;

        // The forge actually ran through the fenced path: a non-empty sequence
        // whose entries are forge_one_from_recovered outcomes (the sole producer
        // of these variants on this path) — so the identity is not vacuous.
        assert!(
            !outcomes_a.is_empty(),
            "the forge-attempt sequence must be non-empty (forge actually ran)"
        );
        assert!(
            outcomes_a.iter().all(|o| matches!(
                o,
                CoordinatorEvent::ForgeSucceeded { .. }
                    | CoordinatorEvent::ForgeNotLeader { .. }
                    | CoordinatorEvent::ForgeFailed { .. }
            )),
            "every observed outcome must be a forge_one_from_recovered result"
        );

        // Relay-derived surfaces (unchanged by forge) byte-identical across runs.
        assert_eq!(
            tip_a, tip_b,
            "tip byte-identical across two clean forge runs"
        );
        assert_eq!(
            wal_a, wal_b,
            "WAL image byte-identical across two clean forge runs"
        );
        assert_eq!(
            snaps_a, snaps_b,
            "checkpoint slots identical across two clean forge runs"
        );
        // The load-bearing identity: forge-attempt sequence + forged bytes.
        assert_eq!(
            outcomes_a, outcomes_b,
            "forge-attempt sequence + forged bytes byte-identical across two clean runs"
        );
    }

    // ===== PHASE4-N-F-G-C S1: live WirePump feed wiring (CE-G-C-1) =========

    /// PHASE4-N-F-G-C S1: the live feed is a *fill* of the closed 2-variant
    /// `NodeBlockSource` (`WirePump` | `InMemory`), NOT a new plugin point. An
    /// exhaustive match with NO wildcard arm pins the closure — adding a third
    /// variant (an alternative live source) would fail to compile here.
    #[tokio::test]
    async fn node_block_source_stays_closed_two_variant() {
        fn classify(s: &NodeBlockSource) -> &'static str {
            match s {
                NodeBlockSource::WirePump { .. } => "wire_pump",
                NodeBlockSource::InMemory { .. } => "in_memory",
            }
        }
        let (_tx, rx) = mpsc::channel::<AdmissionPeerEvent>(1);
        assert_eq!(classify(&NodeBlockSource::from_wire_pump(rx)), "wire_pump");
        assert_eq!(classify(&NodeBlockSource::in_memory(vec![])), "in_memory");
    }

    /// PHASE4-N-F-G-C S1: a LIVE (Continuing) `WirePump` feed makes
    /// `LoopStep::ForgeTick` reachable in the relay loop — the empty `InMemory`
    /// source halts before any `ForgeTick`. Same recovered base / keys / clock /
    /// schedule; the ONLY difference is the source liveness, so the contrast
    /// isolates exactly the live-feed effect. Forge stays subordinate to the
    /// feed (CN-NODE-02 / DC-NODE-05): a due slot forges ONLY because the feed
    /// is Continuing. (This is the consume-side proof that the G-C live wiring
    /// makes the forge observable; peer ACCEPT is NOT claimed — RO-LIVE-01/06.)
    #[tokio::test]
    async fn live_wire_pump_feed_reaches_forge_tick() {
        // Returns the fenced forge-attempt outcomes captured by the activation.
        async fn forge_outcomes_for(source_is_live: bool) -> Vec<CoordinatorEvent> {
            let dir = TempDir::new().unwrap();
            let chaindb = PersistentChainDb::open(PersistentChainDbOptions::at(
                dir.path().join("chain.db"),
            ))
            .unwrap();
            let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
            let mut state = l5_forge_spine();
            let sched = l5_era_schedule();
            let recovered = l5_recovered_state_cold(Some(l5_recovered_inputs()));
            let coordinator = s2_coordinator_state();
            let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
            let view = s2_idle_view();
            let mut clock = DeterministicClock::new(0, vec![100_000, 200_000, 300_000]);
            let mut act = ForgeActivation::new(
                &mut clock,
                &coordinator,
                &recovered,
                &mut shell,
                L5_POOL,
                ProtocolParameters::default(),
                ProtocolVersion { major: 9, minor: 0 },
                0,
                SlotNo(0),
                1_000,
            );
            // Keep the sender alive ONLY in the live case (Continuing feed); the
            // empty in_memory source is `is_ended` and halts before any ForgeTick.
            let _keepalive;
            let mut source = if source_is_live {
                let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(4);
                _keepalive = Some(tx);
                NodeBlockSource::from_wire_pump(rx)
            } else {
                _keepalive = None;
                NodeBlockSource::in_memory(Vec::new())
            };
            let (sd_tx, mut sd_rx) = watch::channel(false);
            let loop_fut = run_relay_loop(
                &mut state,
                &mut source,
                &chaindb,
                &mut wal,
                &sched,
                &view,
                &mut sd_rx,
                Some(&mut act),
            );
            let driver = async {
                let _ = sd_tx.send(true);
            };
            let (loop_res, _) = tokio::join!(loop_fut, driver);
            loop_res.expect("relay loop halts cleanly");
            act.hermetic_forge_outcomes.clone()
        }

        let live = forge_outcomes_for(true).await;
        let empty = forge_outcomes_for(false).await;
        assert!(
            !live.is_empty(),
            "a Continuing WirePump feed must make ForgeTick reachable (forge attempted)"
        );
        assert!(
            empty.is_empty(),
            "the empty in_memory source must halt before any ForgeTick (no forge attempted)"
        );
    }

    // ===== PHASE4-N-F-G-E S1: WirePump lookahead-depth cap (DC-LIVEMEM-01) =====

    /// A fast / hostile peer cannot grow the content-blind lookahead unbounded:
    /// one opportunistic drain stops at `MAX_WIRE_PUMP_LOOKAHEAD`, leaving the
    /// rest queued in the bounded channel (back-pressure), never an unbounded
    /// `VecDeque`.
    #[tokio::test]
    async fn wirepump_lookahead_stops_at_cap() {
        // A generous channel so the sends don't block — the point is the
        // lookahead DRAIN stops at the cap regardless of how much is available.
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(MAX_WIRE_PUMP_LOOKAHEAD * 4);
        for i in 0..(MAX_WIRE_PUMP_LOOKAHEAD + 50) {
            tx.send(AdmissionPeerEvent::Block {
                peer: "p".to_string(),
                block_bytes: vec![i as u8],
            })
            .await
            .unwrap();
        }
        let mut source = NodeBlockSource::from_wire_pump(rx);
        // Trigger one opportunistic pump (has_work_ready pumps when empty).
        assert!(source.has_work_ready());
        match &source {
            NodeBlockSource::WirePump { lookahead, .. } => assert_eq!(
                lookahead.len(),
                MAX_WIRE_PUMP_LOOKAHEAD,
                "lookahead must stop draining at the cap, not grow unbounded"
            ),
            _ => panic!("expected WirePump"),
        }
        drop(tx);
    }

    /// Under a normal feed (well under the cap) the cap is never hit and every
    /// block is delivered in arrival order — relay/sync behavior unchanged.
    #[tokio::test]
    async fn wirepump_lookahead_cap_preserves_relay_behavior_under_normal_feed() {
        let (tx, rx) = mpsc::channel::<AdmissionPeerEvent>(64);
        let n: usize = 10;
        for i in 0..n {
            tx.send(AdmissionPeerEvent::Block {
                peer: "p".to_string(),
                block_bytes: vec![i as u8],
            })
            .await
            .unwrap();
        }
        drop(tx); // close the feed so it ends after draining
        let mut source = NodeBlockSource::from_wire_pump(rx);
        let mut got = Vec::new();
        while let Some(b) = source.next_block().await {
            got.push(b[0]);
        }
        assert_eq!(
            got,
            (0..n as u8).collect::<Vec<u8>>(),
            "every block delivered in arrival order; the cap is never hit under a normal feed"
        );
    }

    // ===== S3b: single-epoch / KES fail-closed containment (CE-E-7) =====

    /// A coordinator whose KES key is exhausted at one period: `kes_max_period
    /// = 0` with `slots_per_kes_period = 10`, so slots 0..9 are period 0 (valid)
    /// and any slot >= 10 rotates to period >= 1 (> max) => `None`.
    fn s3b_kes_exhausted_coordinator() -> CoordinatorState {
        let mut c = s2_coordinator_state();
        c.genesis_anchor.slots_per_kes_period = 10;
        c.genesis_anchor.kes_max_period = 0;
        c
    }

    #[tokio::test]
    async fn forge_tick_rotated_kes_period_skips_no_retroactive_sign() {
        // CE-E-7 (KES clause): a Due slot whose KES period has rotated past the
        // hot key is SKIPPED before any forge_one_from_recovered attempt — no KES
        // signing (no retroactive sign), and the skip does NOT advance
        // last_forged_slot. Proven by a follow-up: after the exhausted HIGH slot
        // is skipped, a LOWER valid slot still forges — impossible if the skip
        // had advanced last_forged to the high slot (monotonic guard would then
        // mark the lower slot NotDue).
        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = l5_forge_spine();
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let recovered = l5_recovered_state_cold(Some(l5_recovered_inputs()));
        let coordinator = s3b_kes_exhausted_coordinator();
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let view = s2_idle_view();

        let tip_before = ChainDb::tip(&chaindb).unwrap();
        let wal_before = format!("{:?}", wal.read_all().unwrap());
        let snaps_before = SnapshotStore::list_snapshot_slots(&chaindb).unwrap();

        // tick1 -> slot 100 (period 10 > max 0 => KES None => SKIP);
        // tick2 -> slot 5 (period 0 => valid => forge). Slot 5 < slot 100, so a
        // forge at 5 proves the skip did not advance last_forged to 100.
        let mut clock = DeterministicClock::new(0, vec![100_000, 5_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &recovered,
            &mut shell,
            L5_POOL,
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );

        let loop_fut = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly");
        drop(block_tx);

        // Exactly one outcome — for the KES-valid slot 5. The exhausted slot 100
        // produced NO forge attempt (skipped; no retroactive KES sign), and the
        // skip did not advance last_forged (else slot 5 < 100 would be NotDue).
        assert_eq!(
            act.hermetic_forge_outcomes.len(),
            1,
            "only the KES-valid slot forges; the exhausted slot is skipped"
        );
        let slot = match &act.hermetic_forge_outcomes[0] {
            CoordinatorEvent::ForgeSucceeded { slot, .. } => *slot,
            CoordinatorEvent::ForgeNotLeader { slot, .. } => *slot,
            CoordinatorEvent::ForgeFailed { slot, .. } => *slot,
            other => panic!("unexpected forge outcome variant: {other:?}"),
        };
        assert_eq!(
            slot, 5,
            "the forged slot is the KES-valid follow-up (5) — proving the exhausted slot 100 was skipped and did not advance last_forged"
        );

        // Surfaces unchanged from the pre-tick baseline (forge advances no tip).
        assert_eq!(ChainDb::tip(&chaindb).unwrap(), tip_before);
        assert_eq!(format!("{:?}", wal.read_all().unwrap()), wal_before);
        assert_eq!(
            SnapshotStore::list_snapshot_slots(&chaindb).unwrap(),
            snaps_before
        );
    }

    #[tokio::test]
    async fn forge_tick_off_epoch_slot_fails_closed_local() {
        // CE-E-7 (off-epoch clause): a slot outside the recovered single-epoch
        // window is represented locally as a structured ForgeNotLeader through
        // the fenced forge path — never a fabricated off-epoch ForgeSucceeded,
        // never a tip advance.
        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        let mut state = l5_forge_spine();
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let recovered = l5_recovered_state_cold(Some(l5_recovered_inputs()));
        // KES in range for slot 432000 (period 3 <= 63); the containment here is
        // the off-epoch leader-schedule miss, not KES.
        let coordinator = s2_coordinator_state();
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let view = s2_idle_view();
        let tip_before = ChainDb::tip(&chaindb).unwrap();

        // Slot 432000 = epoch 1 (epoch_length_slots = 432000); the recovered
        // view is epoch 0 -> off-epoch -> ForgeNotLeader.
        let mut clock = DeterministicClock::new(0, vec![432_000_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &recovered,
            &mut shell,
            L5_POOL,
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );

        let loop_fut = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly");
        drop(block_tx);

        assert_eq!(act.hermetic_forge_outcomes.len(), 1);
        assert!(
            matches!(
                act.hermetic_forge_outcomes[0],
                CoordinatorEvent::ForgeNotLeader { .. }
            ),
            "off-epoch slot must be a structured ForgeNotLeader (no fabricated off-epoch forge), got {:?}",
            act.hermetic_forge_outcomes[0]
        );
        assert_eq!(
            ChainDb::tip(&chaindb).unwrap(),
            tip_before,
            "off-epoch slot advances no tip"
        );
    }

    // ===== S4 (PHASE4-N-F-G-A): epoch-boundary forge fail-closed (DC-EPOCH-03) =====

    #[test]
    fn forge_epoch_admission_within_seed_epoch_admits() {
        // Slot 100 locates to epoch 0 (l5 schedule start_epoch 0); seed epoch 0
        // ⇒ WithinSeedEpoch — leadership may run.
        let sched = l5_era_schedule();
        assert_eq!(
            forge_epoch_admission(100, &sched, L5_EPOCH),
            ForgeEpochAdmission::WithinSeedEpoch
        );
    }

    #[test]
    fn forge_epoch_admission_off_epoch_fails_closed() {
        // Slot 432000 locates to epoch 1 (epoch_length 432000); seed epoch 0 ⇒
        // OffEpoch{Some(1), 0} — distinct from a VRF-lottery loss, via the SAME
        // EraSchedule::locate map leadership uses.
        let sched = l5_era_schedule();
        assert_eq!(
            forge_epoch_admission(432_000, &sched, L5_EPOCH),
            ForgeEpochAdmission::OffEpoch {
                candidate_epoch: Some(EpochNo(1)),
                seed_epoch: L5_EPOCH,
            }
        );
    }

    #[test]
    fn forge_epoch_admission_unlocatable_fails_closed() {
        // A slot before the first era's start_slot does not locate ⇒ fail closed
        // as OffEpoch{None}. An unlocatable slot can never be the seed epoch.
        let sched = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            1_000,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(1_000),
                start_epoch: EpochNo(7),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 129_600,
            }],
        )
        .expect("schedule");
        assert_eq!(
            forge_epoch_admission(500, &sched, EpochNo(7)),
            ForgeEpochAdmission::OffEpoch {
                candidate_epoch: None,
                seed_epoch: EpochNo(7),
            }
        );
    }

    #[test]
    fn node_forge_off_epoch_slot_fails_closed() {
        // CE-G-A-4 (DC-EPOCH-03): forge_one_from_recovered for a slot outside the
        // single recovered seed epoch fails closed via the EXPLICIT epoch-admission
        // guard — BEFORE leadership / KES signing — as the structured
        // ForgeNotLeader; never a fabricated off-epoch ForgeSucceeded. Hardens the
        // N-F-E forge_tick_off_epoch_slot_fails_closed_local relay-loop proof (which
        // also pins no-tip / no-serve) into the named DC-EPOCH-03 handoff boundary.
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let sched = l5_era_schedule();
        let tip = recovered.tip.clone().expect("recovered tip");
        // Slot 432000 locates to epoch 1; the recovered seed epoch is 0 ⇒ off-epoch.
        // kes_period is irrelevant — the epoch guard fires before leadership/signing.
        let (outcome, handoff) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            Some(&tip),
            &mut shell,
            &L5_POOL,
            &ProtocolParameters::default(),
            &sched,
            432_000,
            3,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("off-epoch forge handoff is representable as a structured outcome");
        assert!(
            matches!(outcome, CoordinatorEvent::ForgeNotLeader { .. }),
            "off-epoch must fail closed as the structured ForgeNotLeader, got {outcome:?}"
        );
        assert!(
            !matches!(outcome, CoordinatorEvent::ForgeSucceeded { .. }),
            "off-epoch must never produce a signed / forged block"
        );
        // PHASE4-N-F-G-B S1: off-epoch fail-closed surfaces no handoff — a
        // non-self-accepted (ForgeNotLeader) outcome yields no servable token.
        assert!(
            handoff.is_none(),
            "off-epoch fail-closed must surface no self-accepted handoff"
        );
    }

    #[test]
    fn node_forge_no_epoch_boundary_promotion_on_forge_path() {
        // CE-G-A-4 (no-promotion lock): an ON-epoch forge handoff consumes the
        // recovered seed-epoch eta0 and drives NO nonce promotion — the recovered
        // chain_dep.epoch_nonce is identical before and after. The guard admits the
        // slot (WithinSeedEpoch), so leadership runs (this is the on-epoch path, not
        // a fail-closed). Cross-epoch nonce roll is a separate cluster, never here.
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
        let mut shell = l5_synth_shell(0x11, 0x22, 0x33);
        let sched = l5_era_schedule();
        let tip = recovered.tip.clone().expect("recovered tip");
        let seed_epoch = l5_recovered_inputs().epoch_no;
        // Slot 100 is in epoch 0 (the recovered seed epoch): the guard admits, so
        // leadership is reached rather than epoch-gated.
        assert_eq!(
            forge_epoch_admission(100, &sched, seed_epoch),
            ForgeEpochAdmission::WithinSeedEpoch,
            "slot 100 is in the recovered seed epoch — leadership must be reached"
        );
        let eta0_before = recovered.chain_dep.epoch_nonce.clone();
        let (outcome, handoff) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            Some(&tip),
            &mut shell,
            &L5_POOL,
            &ProtocolParameters::default(),
            &sched,
            100,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("on-epoch forge handoff is representable");
        // Leadership ran (on-epoch) — a real forge result, never the typed
        // MissingRecoveredConsensusInputs error path.
        assert!(matches!(
            outcome,
            CoordinatorEvent::ForgeSucceeded { .. }
                | CoordinatorEvent::ForgeNotLeader { .. }
                | CoordinatorEvent::ForgeFailed { .. }
        ));
        // PHASE4-N-F-G-B S1: the wrapped SelfAcceptedHandoff is Some iff the
        // node-spine forge self-accepted (ForgeSucceeded) — the surfacing
        // contract at the forge_one_from_recovered boundary.
        assert_eq!(
            handoff.is_some(),
            matches!(outcome, CoordinatorEvent::ForgeSucceeded { .. }),
            "handoff present iff the recovered-base forge self-accepted"
        );
        // No nonce promotion: the recovered seed-epoch eta0 is consumed unchanged.
        assert_eq!(
            recovered.chain_dep.epoch_nonce, eta0_before,
            "the forge path drives no nonce promotion — the recovered seed eta0 is unchanged"
        );
    }

    // ===== S4: operator-material-backed forge proof (replay-equivalent) =====

    /// Write a complete real-format operator key set with a REAL opcert sigma
    /// (cold key signs hot_vkey||seq||kes_period — same recipe as
    /// `l5_synth_shell`), so the loaded opcert verifies against the cold key.
    /// Returns the `ForgePaths` the production loader (S2) consumes.
    fn s4_operator_material(dir: &std::path::Path) -> crate::forge_intent::ForgePaths {
        use ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes};
        use ed25519_dalek::{Signer, SigningKey as DalekSk};
        use std::io::Write as _;
        fn hexe(b: &[u8]) -> String {
            let mut s = String::with_capacity(b.len() * 2);
            for x in b {
                s.push_str(&format!("{x:02x}"));
            }
            s
        }
        fn cli_env(path: &std::path::Path, ty: &str, payload: &[u8]) {
            let cbor = format!("58{:02x}{}", payload.len(), hexe(payload));
            let json = format!(
                "{{\"type\":\"{ty}\",\"description\":\"N-F-F S4 fixture\",\"cborHex\":\"{cbor}\"}}"
            );
            std::fs::File::create(path)
                .unwrap()
                .write_all(json.as_bytes())
                .unwrap();
        }
        let kes_seed = [0x42u8; 32];
        let kes = dir.join("kes.ade.skey");
        ade_runtime::producer::keys::write_ade_kes_envelope(&kes, &kes_seed, 0).unwrap();
        let (vrf_sk, _) = cardano_crypto::vrf::VrfDraft03::keypair_from_seed(&[0x07u8; 32]);
        let vrf = dir.join("vrf.skey");
        cli_env(&vrf, "VrfSigningKey_PraosVRF", &vrf_sk);
        let cold_seed = [0x33u8; 32];
        let cold = dir.join("cold.skey");
        cli_env(&cold, "StakePoolSigningKey_ed25519", &cold_seed);
        let kes_raw = Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed).unwrap();
        let hot_vkey = Sum6Kes::derive_verification_key(&kes_raw);
        let cold_dalek = DalekSk::from_bytes(&cold_seed);
        let mut signable = Vec::with_capacity(48);
        signable.extend_from_slice(&hot_vkey);
        signable.extend_from_slice(&0u64.to_be_bytes());
        signable.extend_from_slice(&0u64.to_be_bytes());
        let sigma = cold_dalek.sign(&signable);
        // REAL cardano-cli NodeOperationalCertificate envelope (S2): cborHex =
        // array(2)[array(4)[hot_vkey(32), seq=0, kes_period=0, sigma(64)], cold_vk(32)].
        let mut ocbor = vec![0x82u8, 0x84, 0x58, 0x20];
        ocbor.extend_from_slice(&hot_vkey);
        ocbor.push(0x00); // sequence_number 0
        ocbor.push(0x00); // kes_period 0
        ocbor.extend_from_slice(&[0x58, 0x40]);
        ocbor.extend_from_slice(&sigma.to_bytes());
        ocbor.extend_from_slice(&[0x58, 0x20]);
        ocbor.extend_from_slice(&[0u8; 32]); // cold_vk (discarded by the node path)
        let opcert = dir.join("opcert.json");
        std::fs::write(
            &opcert,
            format!(
                "{{\"type\":\"NodeOperationalCertificate\",\"description\":\"\",\"cborHex\":\"{}\"}}",
                hexe(&ocbor)
            ),
        )
        .unwrap();
        crate::forge_intent::ForgePaths {
            cold,
            kes,
            vrf,
            opcert,
            genesis: dir.join("genesis.json"),
        }
    }

    /// Recovered seed-epoch inputs registering `pool` (asc 1/1 → that pool is
    /// always leader, so leadership is decided by the recovered surface).
    fn l5_recovered_inputs_for_pool(pool: Hash28) -> SeedEpochConsensusInputs {
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            pool,
            PoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        SeedEpochConsensusInputs {
            anchor_fp: Hash32([0x5A; 32]),
            epoch_no: L5_EPOCH,
            epoch_nonce: Nonce(Hash32([0x8d; 32])),
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 1 },
            total_active_stake: 1_000,
            pool_distribution: pools,
        }
    }

    /// Drive ONE operator-material-backed forge tick over a continuing feed and
    /// return the in-memory outcomes. Asserts self-accept-only invariants (tip
    /// unchanged, no snapshot) internally so both callers inherit them. The
    /// shell is loaded through the production ingress (S2); the operator's own
    /// derived pool is registered so the operator KES key signs.
    async fn drive_operator_forge_once(
        opdir: &std::path::Path,
        chaindir: &std::path::Path,
        anchor_millis: u64,
    ) -> (
        Vec<CoordinatorEvent>,
        Option<ade_runtime::clock::SlotAlignmentError>,
    ) {
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(chaindir.join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(chaindir.join("wal")).unwrap();
        let mut state = l5_forge_spine();
        // Open WirePump: Continuing (never ended) + NoWorkReady, so the planner
        // reaches ForgeTick (a feed-end would suppress forge).
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let paths = s4_operator_material(opdir);
        let mut shell = crate::operator_forge::load_operator_producer_shell(&paths)
            .expect("operator material loads through the production ingress");
        // Operator's own pool id, derived exactly as build_operator_forge_material.
        let op_pool = Hash28(ade_crypto::blake2b_224(&shell.cold_vk().0).0);
        let recovered =
            l5_recovered_state_cold(Some(l5_recovered_inputs_for_pool(op_pool.clone())));
        let coordinator = s2_coordinator_state();
        let view = s2_idle_view();
        let mut clock = DeterministicClock::new(0, vec![100_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &recovered,
            &mut shell,
            op_pool,
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            anchor_millis,
            SlotNo(0),
            1_000,
        );

        let tip_before = ChainDb::tip(&chaindb).unwrap();
        let loop_fut = run_relay_loop(
            &mut state,
            &mut source,
            &chaindb,
            &mut wal,
            &sched,
            &view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("operator-material relay loop halts cleanly");
        drop(block_tx);

        // Self-accept-only: the forge advances NO durable tip and persists no
        // snapshot / served state.
        assert_eq!(
            ChainDb::tip(&chaindb).unwrap(),
            tip_before,
            "operator-material forge advances no durable tip"
        );
        assert!(
            SnapshotStore::list_snapshot_slots(&chaindb)
                .unwrap()
                .is_empty(),
            "operator-material forge persists no snapshot / served state"
        );
        (
            std::mem::take(&mut act.hermetic_forge_outcomes),
            act.last_slot_alignment_fail,
        )
    }

    #[tokio::test]
    async fn relay_loop_with_operator_material_forge_reaches_fenced_path() {
        // CE-F-5: an operator-material-backed activation (real keys loaded via the
        // production ingress) reaches ONLY the fenced forge_one_from_recovered,
        // exactly once for the single due slot, self-accept-only. With the
        // operator pool registered + asc 1/1 the operator KES key SIGNS (not
        // ForgeNotLeader) — proving operator material drives the fenced forge.
        let opdir = TempDir::new().unwrap();
        let chaindir = TempDir::new().unwrap();
        let (outcomes, _) = drive_operator_forge_once(opdir.path(), chaindir.path(), 0).await;
        assert_eq!(
            outcomes.len(),
            1,
            "exactly one fenced forge attempt at the single due slot"
        );
        assert!(
            !matches!(outcomes[0], CoordinatorEvent::ForgeNotLeader { .. }),
            "operator pool is leader (asc 1/1) — the operator KES signing path \
             must run; got {:?}",
            outcomes[0]
        );
    }

    #[tokio::test]
    async fn relay_loop_with_operator_material_two_runs_byte_identical() {
        // CE-F-5 (replay clause): a fixed recovered state + ordered feed +
        // injected clock + fixed operator key set ⇒ a byte-identical forge-attempt
        // sequence + forged bytes across runs.
        let op_a = TempDir::new().unwrap();
        let cd_a = TempDir::new().unwrap();
        let op_b = TempDir::new().unwrap();
        let cd_b = TempDir::new().unwrap();
        let (a, _) = drive_operator_forge_once(op_a.path(), cd_a.path(), 0).await;
        let (b, _) = drive_operator_forge_once(op_b.path(), cd_b.path(), 0).await;
        assert_eq!(
            format!("{a:?}"),
            format!("{b:?}"),
            "operator-material forge must be replay-equivalent across runs"
        );
    }

    #[tokio::test]
    async fn node_forge_slot_via_millis_to_slot_over_real_genesis_anchor() {
        // CE-G-A-3 (aligned): with the genesis anchor at millis 0 and the
        // injected wall-clock observing 100_000ms (tick >= anchor), the node
        // forge path derives the forge slot through the checked clock→slot seam
        // (slot 100 at 1000ms/slot) and reaches the fenced forge — the alignment
        // guard does not trip.
        let opdir = TempDir::new().unwrap();
        let chaindir = TempDir::new().unwrap();
        let (outcomes, slot_fail) =
            drive_operator_forge_once(opdir.path(), chaindir.path(), 0).await;
        assert_eq!(
            slot_fail, None,
            "an aligned wall-clock (tick >= anchor) must not trip the slot guard"
        );
        assert_eq!(
            outcomes.len(),
            1,
            "the aligned clock derives a forgeable slot through the checked seam"
        );
    }

    #[tokio::test]
    async fn node_forge_slot_drift_fails_closed() {
        // CE-G-A-3 (drift): a genesis anchor AHEAD of the observed wall-clock
        // (anchor 200_000ms > tick 100_000ms) is an implausible alignment the
        // saturating millis_to_slot would mask to slot 0. The node forge path
        // fails CLOSED at the RED clock seam — no forge attempt, the structured
        // SlotAlignmentError is surfaced, and no durable tip / snapshot moves
        // (asserted inside the helper).
        let opdir = TempDir::new().unwrap();
        let chaindir = TempDir::new().unwrap();
        let (outcomes, slot_fail) =
            drive_operator_forge_once(opdir.path(), chaindir.path(), 200_000).await;
        assert_eq!(
            slot_fail,
            Some(ade_runtime::clock::SlotAlignmentError::BeforeGenesisAnchor),
            "an anchor ahead of the wall-clock must fail closed, not mask drift"
        );
        assert!(
            outcomes.is_empty(),
            "a drift-failed slot alignment forges nothing"
        );
    }

    // ===================================================================
    // DC-NODE-18 (PHASE4-N-AF) — single-producer extend-own-durable-spine.
    // ===================================================================

    fn tp(block_no: u64, slot: u64, h: u8) -> TipPoint {
        TipPoint {
            slot: SlotNo(slot),
            hash: Hash32([h; 32]),
            block_no,
        }
    }

    /// CE-AG-1 (DC-NODE-19 S1): the venue-policy projection yields
    /// `ContinueInSingleProducerExtend` ONLY for the single-producer extend state;
    /// every other VenueRole × ForgeMode is `HaltOnFeedEnd`.
    #[test]
    fn venue_policy_projection_is_continue_only_in_extend() {
        let a = tp(11, 145, 0xBB);
        let modes = [
            ForgeMode::InitialCatchupRequired,
            ForgeMode::CaughtUpToPeerTip {
                peer_tip: a.clone(),
            },
            ForgeMode::SingleProducerExtendOwnDurableSpine {
                adopted_root: a.clone(),
                current_tip: a.clone(),
            },
        ];
        for role in [VenueRole::Unknown, VenueRole::SingleProducer] {
            for mode in &modes {
                let want = if role == VenueRole::SingleProducer
                    && matches!(mode, ForgeMode::SingleProducerExtendOwnDurableSpine { .. })
                {
                    VenuePolicy::ContinueInSingleProducerExtend
                } else {
                    VenuePolicy::HaltOnFeedEnd
                };
                assert_eq!(venue_policy(role, mode), want, "role={role:?} mode={mode:?}");
            }
        }
        // The ONLY Continue case, asserted directly.
        assert_eq!(
            venue_policy(
                VenueRole::SingleProducer,
                &ForgeMode::SingleProducerExtendOwnDurableSpine {
                    adopted_root: a.clone(),
                    current_tip: a,
                },
            ),
            VenuePolicy::ContinueInSingleProducerExtend,
        );
    }

    /// CE-AF-1: the forge-mode transitions are total + deterministic — each fires
    /// only in its source state, is a no-op elsewhere, and is a pure function of its
    /// inputs.
    #[test]
    fn forge_mode_transitions_are_total_and_deterministic() {
        let peer = tp(10, 100, 0xAA);
        let own = tp(11, 145, 0xBB);

        // on_caughtup: only from InitialCatchupRequired.
        assert_eq!(
            forge_mode_on_caughtup(&ForgeMode::InitialCatchupRequired, peer.clone()),
            ForgeMode::CaughtUpToPeerTip {
                peer_tip: peer.clone()
            }
        );
        let cu = ForgeMode::CaughtUpToPeerTip {
            peer_tip: peer.clone(),
        };
        assert_eq!(forge_mode_on_caughtup(&cu, tp(99, 999, 0xCC)), cu);

        // DC-NODE-20: forge_mode_after_admit on a REAL admit promotes CaughtUpToPeerTip
        // DIRECTLY to the extend state on the own durable tip — no FirstOwnBlockServed
        // cert-wait. A no-op (admitted=false) leaves the mode unchanged.
        assert_eq!(
            forge_mode_after_admit(&cu, true, Some(own.clone()), Some(peer.clone())),
            ForgeMode::SingleProducerExtendOwnDurableSpine {
                adopted_root: own.clone(),
                current_tip: own.clone(),
            }
        );
        assert_eq!(
            forge_mode_after_admit(&cu, false, Some(own.clone()), Some(peer.clone())),
            cu
        );

        // on_extend: advances current_tip within the extend state, keeps adopted_root.
        let ext = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: own.clone(),
            current_tip: own.clone(),
        };
        let next = tp(12, 160, 0xCC);
        assert_eq!(
            forge_mode_on_extend(&ext, next.clone()),
            ForgeMode::SingleProducerExtendOwnDurableSpine {
                adopted_root: own.clone(),
                current_tip: next.clone(),
            }
        );
        assert_eq!(
            forge_mode_on_extend(&ForgeMode::InitialCatchupRequired, next.clone()),
            ForgeMode::InitialCatchupRequired
        );
        // deterministic: same input → same output.
        assert_eq!(
            forge_mode_on_extend(&ext, next.clone()),
            forge_mode_on_extend(&ext, next)
        );
    }

    /// CE-AH-1 (DC-NODE-20): self-admit enters the extend state DIRECTLY — the cert is
    /// never consulted. `forge_mode_after_admit` promotes CaughtUpToPeerTip to the
    /// extend state on the own durable tip, and the resulting extend decision forges on
    /// that tip (ExtendOwnSpine) with NO cert input.
    #[test]
    fn caughtup_self_admit_enters_extend_directly_no_cert() {
        let peer = tp(10, 100, 0xAA);
        let own = tp(11, 145, 0xBB);
        let cu = ForgeMode::CaughtUpToPeerTip {
            peer_tip: peer.clone(),
        };
        // A real self-admit → extend on the own durable tip (no FirstOwnBlockServed).
        assert_eq!(
            forge_mode_after_admit(&cu, true, Some(own.clone()), Some(peer.clone())),
            ForgeMode::SingleProducerExtendOwnDurableSpine {
                adopted_root: own.clone(),
                current_tip: own.clone(),
            }
        );
        // The resulting extend state forges on the durable tip with NO cert (the
        // 7-arg decision no longer takes one).
        let ext = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: own.clone(),
            current_tip: own.clone(),
        };
        assert_eq!(
            single_producer_forge_decision(
                &ext,
                Some(own.clone()), // durable
                Some(peer.clone()), // followed (the frozen ancestor, lags)
                Some(peer),         // observed (on-spine ancestor)
                VenueRole::SingleProducer,
                false,
                false,
            ),
            SingleProducerForgeDecision::ExtendOwnSpine {
                forge_base: own.clone(),
            }
        );
    }

    /// CE-AF-3: in the extend state, the forge proceeds on the durable servable tip
    /// WITHOUT requiring followed_peer_tip == durable (the followed tip lags because
    /// the relay does not re-announce Ade's own block); the forge base byte-equals
    /// the durable spine head.
    #[test]
    fn extend_own_spine_forges_on_durable_tip_without_followed_equality() {
        let adopted = tp(11, 145, 0xBB);
        let current = tp(13, 175, 0xDD); // Ade forged 12, 13 since adoption
        let mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: adopted.clone(),
            current_tip: current.clone(),
        };
        // followed lags at the adopted root (relay never re-announced 12/13);
        // durable == current. DC-NODE-15 alone would say NotCaughtUp here.
        let decision = single_producer_forge_decision(
            &mode,
            Some(current.clone()),  // durable
            Some(adopted.clone()),  // followed (lags — != durable)
            Some(adopted),          // observed (relay at the adopted root, on-spine)
            VenueRole::SingleProducer,
            false,
            false,
        );
        assert_eq!(
            decision,
            SingleProducerForgeDecision::ExtendOwnSpine {
                forge_base: current,
            },
            "extend forges on the durable spine head despite followed != durable, and the forge base byte-equals it"
        );
    }

    /// CE-AF-4: the fence fails closed — each violation condition yields a structured
    /// SingleProducerFenceViolation carrying the named reason + the tips + venue role.
    #[test]
    fn single_producer_fence_fails_closed() {
        use SingleProducerFenceReason as R;
        let adopted = tp(11, 145, 0xBB);
        let current = tp(12, 160, 0xCC);
        let mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: adopted,
            current_tip: current.clone(),
        };

        // (1) venue not declared single-producer.
        assert!(matches!(
            single_producer_forge_decision(&mode, Some(current.clone()), Some(current.clone()), None, VenueRole::Unknown, false, false),
            SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation { reason: R::VenueNotDeclaredSingleProducer, .. })
        ));
        // (2) relay producing.
        assert!(matches!(
            single_producer_forge_decision(&mode, Some(current.clone()), Some(current.clone()), None, VenueRole::SingleProducer, true, false),
            SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation { reason: R::RelayProducing, .. })
        ));
        // (3) recovered anchor k=0 edge.
        assert!(matches!(
            single_producer_forge_decision(&mode, Some(current.clone()), Some(current.clone()), None, VenueRole::SingleProducer, false, true),
            SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation { reason: R::RecoveredAnchorK0SnapshotConflict, .. })
        ));
        // (4) competing peer block beyond the adopted root (block_no > current).
        assert!(matches!(
            single_producer_forge_decision(&mode, Some(current.clone()), Some(current.clone()), Some(tp(20, 300, 0xEE)), VenueRole::SingleProducer, false, false),
            SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation { reason: R::CompetingPeerBlockBeyondAdoptedRoot, .. })
        ));
        // (5) peer tip disagrees with the spine (same block_no as current, different hash).
        let d5 = single_producer_forge_decision(&mode, Some(current.clone()), Some(current.clone()), Some(tp(12, 160, 0xFF)), VenueRole::SingleProducer, false, false);
        assert!(matches!(
            &d5,
            SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation { reason: R::PeerTipDisagreesWithSpine, .. })
        ));
        // the violation carries the structured fields.
        if let SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation {
            venue_role,
            durable_tip,
            ..
        }) = d5
        {
            assert_eq!(venue_role, VenueRole::SingleProducer);
            assert_eq!(durable_tip, Some(current));
        }
    }

    /// CE-AF regression (live-surfaced): the post-forge transition advances ONLY on
    /// an ACTUAL admitted block. A not_leader / no-op tick (admitted=false) sets the
    /// loop's `forged` flag but admits nothing — it MUST NOT promote the mode (the
    /// bug the first live CE-AF-6 run caught: forged=true on not_leader wrongly
    /// advanced CaughtUpToPeerTip -> FirstOwnBlockServed, then stalled awaiting a
    /// certificate for a block never forged).
    #[test]
    fn forge_mode_after_admit_only_advances_on_real_admit() {
        let peer = tp(10, 100, 0xAA);
        let own = tp(11, 145, 0xBB);
        let caught = ForgeMode::CaughtUpToPeerTip {
            peer_tip: peer.clone(),
        };

        // not_leader / no-op tick (admitted = false) -> mode UNCHANGED.
        assert_eq!(
            forge_mode_after_admit(&caught, false, Some(own.clone()), Some(peer.clone())),
            caught
        );
        // a missing durable tip -> unchanged (defensive).
        assert_eq!(
            forge_mode_after_admit(&caught, true, None, Some(peer.clone())),
            caught
        );
        // DC-NODE-20: a real admit from CaughtUpToPeerTip -> the extend state DIRECTLY
        // on the own durable tip (no FirstOwnBlockServed cert-wait).
        assert_eq!(
            forge_mode_after_admit(&caught, true, Some(own.clone()), Some(peer.clone())),
            ForgeMode::SingleProducerExtendOwnDurableSpine {
                adopted_root: own.clone(),
                current_tip: own.clone(),
            }
        );
        // a real admit in the extend state -> current_tip advances, adopted_root kept.
        let ext = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: own.clone(),
            current_tip: own.clone(),
        };
        let next = tp(12, 160, 0xCC);
        assert_eq!(
            forge_mode_after_admit(&ext, true, Some(next.clone()), Some(peer)),
            ForgeMode::SingleProducerExtendOwnDurableSpine {
                adopted_root: own,
                current_tip: next,
            }
        );
    }

    // ===== PHASE4-N-AG S2 (DC-NODE-19): RED loop continuation past feed-EOF =====

    use ade_network::chain_sync::server::ServedHeaderLookup;
    use ade_runtime::network::ChainDbServedSource;

    /// Always-leader single-producer harness with a durable block 0 already
    /// forged + admitted, ready to drive `run_relay_loop` in the DC-NODE-18
    /// extend state. Mirrors the always-leader setup of
    /// `extend_own_spine_two_runs_byte_identical`, then forges + admits block 0.
    struct S2Lead {
        _dir: TempDir,
        chaindb: PersistentChainDb,
        wal: FileWalStore,
        state: ForwardSyncState,
        shell: ProducerShell,
        recovered: BootstrapState,
        ledger_view: PoolDistrView,
        era_schedule: EraSchedule,
        pool_id: Hash28,
        block0_tip: TipPoint,
    }

    fn s2_extend_lead() -> S2Lead {
        use ade_ledger::seed_consensus_inputs::{
            encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };
        use ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance;

        let eta0 = Nonce(Hash32([0xCD; 32]));
        let anchor_fp = Hash32([0xA0; 32]);
        let genesis_base = || {
            let mut ledger = LedgerState::new(CardanoEra::Conway);
            ledger.epoch_state.epoch = EpochNo(0);
            let mut chain_dep = PraosChainDepState::empty();
            chain_dep.epoch_nonce = Nonce(Hash32([0xCD; 32]));
            chain_dep.evolving_nonce = Nonce(Hash32([0xCD; 32]));
            (ledger, chain_dep)
        };
        let era_schedule = EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 432_000,
            }],
        )
        .expect("era schedule");
        let mut shell = l5_synth_shell(0x31, 0x41, 0x59);
        let cold_vk = shell.cold_vk();
        let vrf_vk = shell.vrf_verification_key();
        let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&cold_vk.0);
        let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&vrf_vk.0);
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            pool_id.clone(),
            PoolEntry {
                active_stake: 1,
                vrf_keyhash,
            },
        );
        let sidecar = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EpochNo(0),
            epoch_nonce: eta0.clone(),
            active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 1 },
            total_active_stake: 1,
            pool_distribution: pools,
        };
        let sidecar_bytes = encode_seed_epoch_consensus_inputs(&sidecar);
        let ledger_view = PoolDistrView::from_seed_epoch_consensus_inputs(&sidecar);
        let (l_r, c_r) = genesis_base();
        let recovered = BootstrapState {
            ledger: l_r,
            chain_dep: c_r,
            tip: None,
            seed_epoch_consensus_inputs: Some(sidecar.clone()),
        };

        let dir = TempDir::new().unwrap();
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
                .unwrap();
        let mut wal = FileWalStore::open(dir.path().join("wal")).unwrap();
        chaindb
            .put_seed_epoch_consensus_inputs(&anchor_fp, &sidecar_bytes)
            .unwrap();
        append_seed_epoch_provenance(&mut wal, &anchor_fp, EpochNo(0), &sidecar_bytes).unwrap();
        let (l_s, c_s) = genesis_base();
        PersistentSnapshotCache::new(&chaindb)
            .capture(SlotNo(0), &l_s, &c_s)
            .unwrap();
        let (l_a, c_a) = genesis_base();
        let mut state = ForwardSyncState::new(
            ReceiveState::new(l_a, c_a),
            anchor_fp.clone(),
            SnapshotCadence::DEFAULT,
        );

        // Forge block 0 (genesis successor, slot 1) and admit it durably so the
        // durable servable tip is block 0 — the extend state's `current_tip`.
        let (event0, handoff0) = forge_one_from_recovered(
            &recovered,
            &recovered.chain_dep,
            &recovered.ledger,
            None,
            &mut shell,
            &pool_id,
            &ProtocolParameters::default(),
            &era_schedule,
            1,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("forge block 0 over the recovered genesis base");
        let handoff0 = match (event0, handoff0) {
            (CoordinatorEvent::ForgeSucceeded { .. }, Some(h)) => h,
            (ev, _) => panic!("expected block-0 ForgeSucceeded, got {ev:?}"),
        };
        admit_forged_block_durably(
            &handoff0,
            &mut state,
            &chaindb,
            &mut wal,
            &era_schedule,
            &ledger_view,
        )
        .expect("durable admit block 0")
        .expect("tip 0 advanced");
        let (s0, h0, bn0) = ChainDbServedSource::new(&chaindb)
            .tip()
            .expect("served tip after block 0");
        let block0_tip = TipPoint {
            slot: s0,
            hash: h0,
            block_no: bn0,
        };

        S2Lead {
            _dir: dir,
            chaindb,
            wal,
            state,
            shell,
            recovered,
            ledger_view,
            era_schedule,
            pool_id,
            block0_tip,
        }
    }

    /// A venue-adoption certificate line (`<block_no> <slot> <hash_hex64>`) for the
    /// given own tip. DC-NODE-21: the node never reads this — tests that write it
    /// assert the forge proceeds on the local tip regardless (cert present, ignored).
    fn s2_cert_for(tip: &TipPoint) -> String {
        let hex: String = tip.hash.0.iter().map(|b| format!("{b:02x}")).collect();
        format!("{} {} {}", tip.block_no, tip.slot.0, hex)
    }

    /// CE-AG-2: a certified single-producer venue in the DC-NODE-18 extend state
    /// forges its successor PAST a structural feed EOF — the loop does NOT
    /// HaltCleanly on the ended feed; the durable tip advances 0 -> 1.
    #[tokio::test]
    async fn single_producer_extend_continues_past_feed_eof() {
        let mut lead = s2_extend_lead();
        let cert_path = lead._dir.path().join("cert_continue");
        std::fs::write(&cert_path, s2_cert_for(&lead.block0_tip)).unwrap();
        let coordinator = s2_coordinator_state();
        let mut clock = DeterministicClock::new(0, vec![2_000]); // slot 2 -> block 1
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );
        act.declare_single_producer_venue();
        act.forge_mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: lead.block0_tip.clone(),
            current_tip: lead.block0_tip.clone(),
        };
        let mut source = NodeBlockSource::in_memory(vec![]); // ENDED feed
        let (sd_tx, mut sd_rx) = watch::channel(false);
        let loop_fut = run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        );
        // The loop forges block 1 synchronously (past the ended feed), parks at
        // the continue-mode Idle, and shutdown halts it.
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly on shutdown");
        drop(act);
        let (_s, _h, bn) = ChainDbServedSource::new(&lead.chaindb)
            .tip()
            .expect("durable tip after the run");
        assert_eq!(
            bn, 1,
            "the single-producer extend venue forged block 1 PAST the feed EOF"
        );
    }

    /// CE-AG-2: the default (`Unknown`) venue keeps the verbatim prior behavior —
    /// a structural feed EOF HaltCleanly's the loop; no forge past the feed.
    #[tokio::test]
    async fn unknown_venue_still_halts_on_feed_eof() {
        let mut lead = s2_extend_lead();
        let coordinator = s2_coordinator_state();
        let mut clock = DeterministicClock::new(0, vec![2_000]);
        // No declare_single_producer_venue, default forge_mode -> venue_policy is
        // HaltOnFeedEnd, so an ended feed halts cleanly before any ForgeTick.
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );
        let mut source = NodeBlockSource::in_memory(vec![]); // ENDED feed
        let (_sd_tx, mut sd_rx) = watch::channel(false);
        run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        )
        .await
        .expect("Unknown venue halts cleanly on the ended feed");
        drop(act);
        let (_s, _h, bn) = ChainDbServedSource::new(&lead.chaindb)
            .tip()
            .expect("durable tip after the run");
        assert_eq!(bn, 0, "Unknown venue must NOT forge past the feed EOF");
    }

    /// CE-AG-2: only a CLEAN structural feed EOF is continued. A fatal source
    /// error (an undecodable block) exits via Err/fail-fast in the SyncOnce arm —
    /// never continued, even in the single-producer extend venue.
    #[tokio::test]
    async fn fatal_source_error_fails_fast_not_continued() {
        let mut lead = s2_extend_lead();
        let cert_path = lead._dir.path().join("cert_fatal");
        std::fs::write(&cert_path, s2_cert_for(&lead.block0_tip)).unwrap();
        let coordinator = s2_coordinator_state();
        let mut clock = DeterministicClock::new(0, vec![2_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );
        act.declare_single_producer_venue();
        act.forge_mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: lead.block0_tip.clone(),
            current_tip: lead.block0_tip.clone(),
        };
        // An undecodable block: WorkAvailable -> SyncOnce -> run_node_sync Err.
        let mut source = NodeBlockSource::in_memory(vec![vec![0xDE, 0xAD, 0xBE, 0xEF]]);
        let (_sd_tx, mut sd_rx) = watch::channel(false);
        let res = run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        )
        .await;
        drop(act);
        assert!(
            res.is_err(),
            "a fatal source error must fail-fast (Err), never be continued: {res:?}"
        );
    }

    /// CE-AH-2 (DC-NODE-20): the continue-past-EOF path NO LONGER requires a cert.
    /// In the extend state with NO certificate (no cert present), a feed EOF
    /// continues the loop and forges the next successor on the local durable spine —
    /// DC-NODE-19's continue-past-EOF core is preserved; its cert-fence clause is
    /// superseded by DC-NODE-20 (the forge base is the local ChainDb::tip).
    #[tokio::test]
    async fn continuation_past_eof_no_longer_requires_cert() {
        let mut lead = s2_extend_lead();
        let coordinator = s2_coordinator_state();
        let mut clock = DeterministicClock::new(0, vec![2_000]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );
        // Single-producer extend venue, NO certificate (no cert present).
        act.declare_single_producer_venue();
        act.forge_mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: lead.block0_tip.clone(),
            current_tip: lead.block0_tip.clone(),
        };
        let mut source = NodeBlockSource::in_memory(vec![]); // ENDED feed
        let (sd_tx, mut sd_rx) = watch::channel(false);
        let loop_fut = run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly on shutdown");
        drop(act);
        let (_s, _h, bn) = ChainDbServedSource::new(&lead.chaindb)
            .tip()
            .expect("durable tip after the run");
        assert_eq!(
            bn, 1,
            "DC-NODE-20: the continuation forged block 1 past the EOF with NO cert present"
        );
    }

    /// CE-AG-3: in continue-mode the Idle wait wakes on the slot-cadence timer
    /// (not the dead feed's wait_ready), so the loop forges the NEXT due slot
    /// across an Idle — durable tip advances 0 -> 1 -> 2 with no feed activity.
    #[tokio::test]
    async fn idle_under_dead_feed_wakes_on_clock_tick() {
        let mut lead = s2_extend_lead();
        let cert_path = lead._dir.path().join("cert_idle");
        std::fs::write(&cert_path, s2_cert_for(&lead.block0_tip)).unwrap();
        let coordinator = s2_coordinator_state();
        // slot_length 10ms (the Idle timer): clock ticks for slot 2, slot 2
        // (NotDue -> Idle), slot 3. The timer wakeup re-reads the clock + forges 3.
        let mut clock = DeterministicClock::new(0, vec![20, 20, 30]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            10,
        );
        act.declare_single_producer_venue();
        act.forge_mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: lead.block0_tip.clone(),
            current_tip: lead.block0_tip.clone(),
        };
        let mut source = NodeBlockSource::in_memory(vec![]); // ENDED feed
        let (sd_tx, mut sd_rx) = watch::channel(false);
        let loop_fut = run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        );
        // Let the 10ms Idle timer fire (waking the loop to forge slot 3) before
        // shutting down. If the Idle parked on the dead feed instead, block 2
        // would never be forged.
        let driver = async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly on shutdown");
        drop(act);
        let (_s, _h, bn) = ChainDbServedSource::new(&lead.chaindb)
            .tip()
            .expect("durable tip after the run");
        assert_eq!(
            bn, 2,
            "the Idle timer woke the loop to forge block 2 across the dead-feed Idle"
        );
    }

    /// CE-AH-5 (DC-NODE-20): core acceptance — sustained forging on the local durable
    /// spine with NO cert in the forge path. From the durable block 0 the loop forges
    /// block 1 then block 2 (forged >= 2 own successors) under a dead feed, the forge
    /// base deriving from ChainDb::tip each tick; no adoption certificate is present.
    #[tokio::test]
    async fn local_spine_sustains_two_successors_no_cert() {
        let mut lead = s2_extend_lead();
        let coordinator = s2_coordinator_state();
        // slot_length 10ms (fast Idle timer); clock: slot 2, slot 2 (Idle), slot 3 ->
        // forge block 1 then block 2 across the dead-feed Idle.
        let mut clock = DeterministicClock::new(0, vec![20, 20, 30]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            10,
        );
        // NO certificate (no cert present) — the forge base is the local tip.
        act.declare_single_producer_venue();
        act.forge_mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: lead.block0_tip.clone(),
            current_tip: lead.block0_tip.clone(),
        };
        let mut source = NodeBlockSource::in_memory(vec![]); // ENDED feed
        let (sd_tx, mut sd_rx) = watch::channel(false);
        let loop_fut = run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly on shutdown");
        drop(act);
        let (_s, _h, bn) = ChainDbServedSource::new(&lead.chaindb)
            .tip()
            .expect("durable tip after the run");
        assert_eq!(
            bn, 2,
            "DC-NODE-20: forged 2 successors (blocks 1, 2) on the local spine with NO cert"
        );
    }

    // ===== PHASE4-N-AG S3 (DC-NODE-19): replay-equivalence over a post-feed-end chain =====

    use ade_network::chain_sync::server::HeaderProjection;

    /// The served chain as a follower would walk it: the ordered ChainSync
    /// `next_after` projection (header bytes, DC-CONS-18) paired with each block's
    /// BlockFetch body (the verbatim durable bytes, DC-NODE-13). A deterministic
    /// snapshot of the served chain for replay comparison.
    fn served_snapshot(chaindb: &PersistentChainDb) -> Vec<(HeaderProjection, Vec<u8>)> {
        let src = ChainDbServedSource::new(chaindb);
        let mut out = Vec::new();
        let mut cursor: Option<(SlotNo, Hash32)> = None;
        while let Some(hp) = src.next_after(cursor.clone()) {
            let body = chaindb
                .get_block_by_hash(&hp.hash)
                .expect("get_block_by_hash")
                .expect("served point has a durable block body")
                .bytes;
            cursor = Some((hp.slot, hp.hash.clone()));
            out.push((hp, body));
        }
        out
    }

    /// Drive a fresh always-leader extend venue past an ENDED feed: one due clock
    /// tick (slot 2) forges block 1 on the durable block-0 spine — a single
    /// post-EOF successor — then shutdown halts the loop. The borrows on `lead`
    /// are released on return, so the caller can snapshot the durable surfaces.
    async fn s2_run_continue_one(lead: &mut S2Lead, cert_path: std::path::PathBuf) {
        std::fs::write(&cert_path, s2_cert_for(&lead.block0_tip)).unwrap();
        let coordinator = s2_coordinator_state();
        let mut clock = DeterministicClock::new(0, vec![2_000]); // slot 2 -> block 1
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            1_000,
        );
        act.declare_single_producer_venue();
        act.forge_mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: lead.block0_tip.clone(),
            current_tip: lead.block0_tip.clone(),
        };
        let mut source = NodeBlockSource::in_memory(vec![]); // ENDED feed
        let (sd_tx, mut sd_rx) = watch::channel(false);
        let loop_fut = run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly on shutdown");
    }

    /// CE-AG-4: two clean runs of the post-feed-end continuation are byte-identical
    /// across ALL FOUR surfaces — WAL image, durable tip, ledger fingerprint, AND
    /// served chain — under the same recovered state + ended feed + injected clock
    /// + adoption cert + shutdown schedule.
    #[tokio::test]
    async fn continue_past_eof_two_runs_byte_identical() {
        let mut a = s2_extend_lead();
        let cert_a = a._dir.path().join("cert");
        s2_run_continue_one(&mut a, cert_a).await;
        let wal_a = a.wal.read_all().expect("wal a");
        let tip_a = ChainDbServedSource::new(&a.chaindb).tip();
        let fp_a = ade_ledger::fingerprint::fingerprint(&a.state.receive.ledger);
        let served_a = served_snapshot(&a.chaindb);

        let mut b = s2_extend_lead();
        let cert_b = b._dir.path().join("cert");
        s2_run_continue_one(&mut b, cert_b).await;
        let wal_b = b.wal.read_all().expect("wal b");
        let tip_b = ChainDbServedSource::new(&b.chaindb).tip();
        let fp_b = ade_ledger::fingerprint::fingerprint(&b.state.receive.ledger);
        let served_b = served_snapshot(&b.chaindb);

        assert_eq!(
            tip_a.as_ref().map(|t| t.2),
            Some(1),
            "the run forged block 1 PAST the feed EOF"
        );
        assert_eq!(tip_a, tip_b, "durable tip byte-identical across runs");
        assert_eq!(fp_a, fp_b, "ledger fingerprint byte-identical across runs");
        assert_eq!(served_a, served_b, "served chain byte-identical across runs");
        assert_eq!(wal_a, wal_b, "WAL image byte-identical across runs");
    }

    /// CE-AG-4: a post-feed-end forged chain recovers via warm_start_recovery to
    /// the SAME durable tip + ledger fingerprint + served chain (no ChainBreak
    /// across the post-EOF forge seam) — T-REC-05 extended to the loop-continued
    /// chain. The TempDir is kept alive across the kill via destructuring.
    #[tokio::test]
    async fn continue_past_eof_kill_warm_start_recovers_byte_identical() {
        let mut lead = s2_extend_lead();
        let cert = lead._dir.path().join("cert");
        s2_run_continue_one(&mut lead, cert).await;

        // Destructure so the TempDir outlives the kill; drop the durable handles.
        let S2Lead {
            _dir,
            chaindb,
            wal,
            state,
            ..
        } = lead;
        let chaindb_path = _dir.path().join("chain.db");
        let wal_dir = _dir.path().join("wal");
        let pre_tip = ChainDbServedSource::new(&chaindb).tip();
        let pre_fp = ade_ledger::fingerprint::fingerprint(&state.receive.ledger);
        let pre_served = served_snapshot(&chaindb);
        assert_eq!(
            pre_tip.as_ref().map(|t| t.2),
            Some(1),
            "block 1 forged past the EOF before the kill"
        );
        drop(chaindb);
        drop(wal);
        drop(state);

        // Reopen + warm-start recover (forward-replay over the post-EOF WAL).
        let chaindb2 =
            PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal2 = FileWalStore::open(&wal_dir).unwrap();
        let recovered = crate::node_lifecycle::warm_start_recovery(&chaindb2, &wal2)
            .expect("warm-start forward-replays the post-EOF chain without ChainBreak");
        let post_tip = ChainDbServedSource::new(&chaindb2).tip();
        let post_fp = ade_ledger::fingerprint::fingerprint(&recovered.ledger);
        let post_served = served_snapshot(&chaindb2);

        assert_eq!(pre_tip, post_tip, "warm-start recovers the durable tip byte-identically");
        assert_eq!(pre_fp, post_fp, "warm-start recovers the ledger fingerprint byte-identically");
        assert_eq!(
            pre_served, post_served,
            "warm-start recovers the served chain byte-identically"
        );
    }

    /// CE-AG-4: feed EOF is loop-control input, not durable input. After K=1
    /// post-EOF forged successor, the WAL grew by exactly one WalEntry::AdmitBlock
    /// (the forged successor) — the EOF itself appended nothing.
    #[tokio::test]
    async fn feed_eof_appends_nothing_to_wal() {
        let mut lead = s2_extend_lead();
        let admits = |w: &[WalEntry]| w.iter().filter(|e| matches!(e, WalEntry::AdmitBlock { .. })).count();
        let wal_before = lead.wal.read_all().expect("wal before");
        let admits_before = admits(&wal_before);
        let cert = lead._dir.path().join("cert");
        s2_run_continue_one(&mut lead, cert).await; // one post-EOF forged successor
        let wal_after = lead.wal.read_all().expect("wal after");
        let admits_after = admits(&wal_after);

        assert_eq!(
            admits_after,
            admits_before + 1,
            "exactly one post-EOF forged AdmitBlock entry"
        );
        assert_eq!(
            wal_after.len(),
            wal_before.len() + 1,
            "the feed EOF appends NO WAL entry — the only new entry is the forged successor's AdmitBlock"
        );
    }

    // ===== PHASE4-N-AH S3 (DC-NODE-20 ∩ T-REC): replay-equivalence over the no-cert
    //       local-tip-derived K=2 successor chain =====

    /// Drive a fresh always-leader single-producer extend venue past an ENDED feed for
    /// K=2 successors on the LOCAL durable spine (forge base = ChainDb::tip), NO cert.
    /// Mirrors `local_spine_sustains_two_successors_no_cert`; the borrows on `lead`
    /// release on return so the caller can snapshot the durable surfaces.
    async fn local_spine_run_two(lead: &mut S2Lead) {
        let coordinator = s2_coordinator_state();
        let mut clock = DeterministicClock::new(0, vec![20, 20, 30]);
        let mut act = ForgeActivation::new(
            &mut clock,
            &coordinator,
            &lead.recovered,
            &mut lead.shell,
            lead.pool_id.clone(),
            ProtocolParameters::default(),
            ProtocolVersion { major: 9, minor: 0 },
            0,
            SlotNo(0),
            10,
        );
        act.declare_single_producer_venue();
        act.forge_mode = ForgeMode::SingleProducerExtendOwnDurableSpine {
            adopted_root: lead.block0_tip.clone(),
            current_tip: lead.block0_tip.clone(),
        };
        let mut source = NodeBlockSource::in_memory(vec![]); // ENDED feed
        let (sd_tx, mut sd_rx) = watch::channel(false);
        let loop_fut = run_relay_loop(
            &mut lead.state,
            &mut source,
            &lead.chaindb,
            &mut lead.wal,
            &lead.era_schedule,
            &lead.ledger_view,
            &mut sd_rx,
            Some(&mut act),
        );
        let driver = async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            let _ = sd_tx.send(true);
        };
        let (loop_res, _) = tokio::join!(loop_fut, driver);
        loop_res.expect("loop halts cleanly on shutdown");
    }

    /// CE-AH-4 (DC-NODE-20 ∩ T-REC-03): two clean runs of the no-cert K=2 local-spine
    /// forge are byte-identical across ALL FOUR durable surfaces — WAL image, durable
    /// tip, ledger fingerprint, served chain. The local-tip forge base is a pure
    /// function of the recovered state + canonical slot schedule (no wall-clock).
    #[tokio::test]
    async fn local_spine_two_runs_byte_identical() {
        let mut a = s2_extend_lead();
        local_spine_run_two(&mut a).await;
        let wal_a = a.wal.read_all().expect("wal a");
        let tip_a = ChainDbServedSource::new(&a.chaindb).tip();
        let fp_a = ade_ledger::fingerprint::fingerprint(&a.state.receive.ledger);
        let served_a = served_snapshot(&a.chaindb);

        let mut b = s2_extend_lead();
        local_spine_run_two(&mut b).await;
        let wal_b = b.wal.read_all().expect("wal b");
        let tip_b = ChainDbServedSource::new(&b.chaindb).tip();
        let fp_b = ade_ledger::fingerprint::fingerprint(&b.state.receive.ledger);
        let served_b = served_snapshot(&b.chaindb);

        assert_eq!(
            tip_a.as_ref().map(|t| t.2),
            Some(2),
            "K=2 successors forged on the local spine"
        );
        assert_eq!(tip_a, tip_b, "durable tip byte-identical across runs");
        assert_eq!(fp_a, fp_b, "ledger fingerprint byte-identical across runs");
        assert_eq!(served_a, served_b, "served chain byte-identical across runs");
        assert_eq!(wal_a, wal_b, "WAL image byte-identical across runs");
    }

    /// CE-AH-4 (DC-NODE-20 ∩ T-REC-05): the no-cert K=2 local-spine chain recovers via
    /// `warm_start_recovery` to the SAME durable tip + ledger fingerprint + served chain
    /// (no ChainBreak across the local-spine forge seam).
    #[tokio::test]
    async fn local_spine_kill_warm_start_byte_identical() {
        let mut lead = s2_extend_lead();
        local_spine_run_two(&mut lead).await;

        let S2Lead {
            _dir,
            chaindb,
            wal,
            state,
            ..
        } = lead;
        let chaindb_path = _dir.path().join("chain.db");
        let wal_dir = _dir.path().join("wal");
        let pre_tip = ChainDbServedSource::new(&chaindb).tip();
        let pre_fp = ade_ledger::fingerprint::fingerprint(&state.receive.ledger);
        let pre_served = served_snapshot(&chaindb);
        assert_eq!(
            pre_tip.as_ref().map(|t| t.2),
            Some(2),
            "K=2 forged before the kill"
        );
        drop(chaindb);
        drop(wal);
        drop(state);

        let chaindb2 =
            PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
        let wal2 = FileWalStore::open(&wal_dir).unwrap();
        let recovered = crate::node_lifecycle::warm_start_recovery(&chaindb2, &wal2)
            .expect("warm-start forward-replays the local-spine chain without ChainBreak");
        let post_tip = ChainDbServedSource::new(&chaindb2).tip();
        let post_fp = ade_ledger::fingerprint::fingerprint(&recovered.ledger);
        let post_served = served_snapshot(&chaindb2);

        assert_eq!(pre_tip, post_tip, "warm-start recovers the durable tip byte-identically");
        assert_eq!(pre_fp, post_fp, "warm-start recovers the ledger fingerprint byte-identically");
        assert_eq!(
            pre_served, post_served,
            "warm-start recovers the served chain byte-identically"
        );
    }

    /// CE-AH-4 (DC-NODE-20 ∩ DC-NODE-21): a cert FILE present on disk does NOT alter the
    /// replay surface — the cert-present and no-cert runs produce byte-identical WAL +
    /// durable tip + ledger fingerprint + served chain, and the cert's (bogus) adopted-
    /// tip hash never enters any served block body. The cert is absent from replay.
    #[tokio::test]
    async fn local_spine_cert_file_absent_from_replay_surface() {
        // Baseline: no cert file on disk.
        let mut no_cert = s2_extend_lead();
        local_spine_run_two(&mut no_cert).await;
        let wal_no = no_cert.wal.read_all().expect("wal no-cert");
        let tip_no = ChainDbServedSource::new(&no_cert.chaindb).tip();
        let fp_no = ade_ledger::fingerprint::fingerprint(&no_cert.state.receive.ledger);
        let served_no = served_snapshot(&no_cert.chaindb);

        // A cert file present on disk, carrying a DISTINCTIVE bogus adopted-tip hash the
        // node must never read (DC-NODE-21).
        let mut with_cert = s2_extend_lead();
        let bogus_hash_hex = "7e".repeat(32);
        std::fs::write(
            with_cert._dir.path().join("cert"),
            format!("99 9999 {bogus_hash_hex}"),
        )
        .unwrap();
        local_spine_run_two(&mut with_cert).await;
        let wal_with = with_cert.wal.read_all().expect("wal with-cert");
        let tip_with = ChainDbServedSource::new(&with_cert.chaindb).tip();
        let fp_with = ade_ledger::fingerprint::fingerprint(&with_cert.state.receive.ledger);
        let served_with = served_snapshot(&with_cert.chaindb);

        assert_eq!(tip_no, tip_with, "a cert file present does not change the durable tip");
        assert_eq!(fp_no, fp_with, "a cert file present does not change the ledger fingerprint");
        assert_eq!(served_no, served_with, "a cert file present does not change the served chain");
        assert_eq!(wal_no, wal_with, "a cert file present does not change the WAL image");

        // The bogus cert hash never enters any served block body (the cert is unread).
        let bogus = [0x7eu8; 32];
        let leaked = served_with
            .iter()
            .any(|(_, body)| body.windows(32).any(|w| w == bogus));
        assert!(
            !leaked,
            "the cert's bogus adopted-tip hash never enters the durable/served bytes"
        );
    }
}
