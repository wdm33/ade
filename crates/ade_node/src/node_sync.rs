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
use ade_ledger::consensus_view::PoolDistrView;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::wal::WalStore;
use ade_runtime::admission::AdmissionPeerEvent;
use ade_runtime::bootstrap::BootstrapState;
use ade_runtime::chaindb::{ChainDb, ChainTip, SnapshotStore};
use ade_runtime::forward_sync::{pump_block, ForwardSyncState, NoCheckpointSink, PumpTip};
use ade_runtime::producer::coordinator::CoordinatorEvent;
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_runtime::rollback::PersistentSnapshotCache;
use ade_types::shelley::block::ProtocolVersion;
use ade_types::{BlockNo, Hash28, SlotNo};
use tokio::sync::mpsc;

use crate::produce_mode::{run_real_forge, ForgeRequestContext};

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
    },
    /// Deterministic in-memory ordered feed (hermetic test / loopback).
    /// Exactly the "a live socket is not required" shape `pump_block`
    /// was designed for.
    InMemory(VecDeque<Vec<u8>>),
}

impl NodeBlockSource {
    /// Build an in-memory source from an ordered block-bytes sequence.
    pub fn in_memory(blocks: Vec<Vec<u8>>) -> Self {
        Self::InMemory(VecDeque::from(blocks))
    }

    /// Wrap one peer's wire-pump event receiver as the source.
    pub fn from_wire_pump(rx: mpsc::Receiver<AdmissionPeerEvent>) -> Self {
        Self::WirePump {
            rx,
            lookahead: VecDeque::new(),
            disconnected: false,
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
    ) {
        use tokio::sync::mpsc::error::TryRecvError;
        loop {
            match rx.try_recv() {
                Ok(AdmissionPeerEvent::Block { block_bytes, .. }) => {
                    lookahead.push_back(block_bytes);
                }
                Ok(AdmissionPeerEvent::TipUpdate { .. }) => continue,
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
            Self::InMemory(q) => q.pop_front(),
            Self::WirePump {
                rx,
                lookahead,
                disconnected,
            } => {
                // Top up the content-blind lookahead from whatever is
                // immediately available (non-blocking), then hand back one
                // block. No `.await` on the channel — a batch boundary
                // (open-but-empty) yields `None` so the sync driver returns
                // and the loop can re-plan / idle / shut down cleanly.
                if lookahead.is_empty() && !*disconnected {
                    Self::pump_lookahead(rx, lookahead, disconnected);
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
            Self::InMemory(q) => !q.is_empty(),
            Self::WirePump {
                rx,
                lookahead,
                disconnected,
            } => {
                if lookahead.is_empty() && !*disconnected {
                    Self::pump_lookahead(rx, lookahead, disconnected);
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
            Self::InMemory(q) => q.is_empty(),
            Self::WirePump {
                lookahead,
                disconnected,
                ..
            } => *disconnected && lookahead.is_empty(),
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
            Self::InMemory(_) => {}
            Self::WirePump {
                rx,
                lookahead,
                disconnected,
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
                        Some(AdmissionPeerEvent::TipUpdate { .. }) => continue,
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
/// Returns the reused `CoordinatorEvent` (`ForgeSucceeded` /
/// `ForgeNotLeader` / `ForgeFailed`), or a typed `NodeForgeError` when the
/// recovered base cannot host a forge.
#[allow(clippy::too_many_arguments)]
pub fn forge_one_from_recovered(
    recovered: &BootstrapState,
    selected_tip: &ChainTip,
    shell: &mut ProducerShell,
    pool_id: &Hash28,
    pparams: &ProtocolParameters,
    era_schedule: &EraSchedule,
    slot: u64,
    kes_period: u32,
    protocol_version: ProtocolVersion,
) -> Result<CoordinatorEvent, NodeForgeError> {
    // Fail-closed: the leadership view MUST be the recovered surface.
    let recovered_inputs = recovered
        .seed_epoch_consensus_inputs
        .as_ref()
        .ok_or(NodeForgeError::MissingRecoveredConsensusInputs)?;

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
        &recovered.chain_dep,
    ) {
        Ok(a) => a,
        Err(_) => {
            return Ok(CoordinatorEvent::ForgeNotLeader {
                slot,
                vrf_output_fingerprint: [0u8; 8],
            });
        }
    };

    // Forge base from recovered state + the selected tip. block_number is
    // tip+1: the recovered chain_dep's last_block_no is the tip's block
    // number (0/None ⇒ first forged block is number 1).
    let next_block_number = recovered
        .chain_dep
        .last_block_no
        .map(|b| b.0 + 1)
        .unwrap_or(1);
    let _ = selected_tip.slot; // tip identity is the prev_hash below.
    let vrf_vk = shell.vrf_verification_key();

    let ctx = ForgeRequestContext {
        eta0: &recovered.chain_dep.epoch_nonce,
        vrf_vk: &vrf_vk,
        leader_schedule_answer: &answer,
        pparams,
        base_state: &recovered.ledger,
        chain_dep_state: &recovered.chain_dep,
        era_schedule,
        pool_distr_view: &pool_distr_view,
        block_number: BlockNo(next_block_number),
        prev_hash: selected_tip.hash.clone(),
        protocol_version,
        prev_opcert_counter: None,
    };

    // Single-shot forge through the reused engine. Its result variants
    // (ForgeSucceeded / ForgeNotLeader / ForgeFailed) are returned as-is;
    // there is no fallback path.
    Ok(run_real_forge(slot, kes_period, &ctx, shell))
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

    use crate::node_lifecycle::{run_relay_loop, ForgeActivation, NodeLifecycleError};
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
        let event = forge_one_from_recovered(
            &recovered,
            &tip,
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
        let e1 = forge_one_from_recovered(
            &recovered,
            &tip,
            &mut shell1,
            &L5_POOL,
            &pparams,
            &sched,
            100,
            0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("ok");
        let e2 = forge_one_from_recovered(
            &recovered,
            &tip,
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
            &tip,
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
        let mut state = fresh_state([0xCD; 32]);
        // Open WirePump: Continuing (never ended) + NoWorkReady (no block), so
        // the planner reaches ForgeTick (a feed-end would suppress forge).
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
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
        let mut state = fresh_state([0xCD; 32]);
        let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
        let mut source = NodeBlockSource::from_wire_pump(block_rx);
        let (sd_tx, mut sd_rx) = watch::channel(false);

        let sched = l5_era_schedule();
        let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
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
        let mut state = fresh_state([0xCD; 32]);
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
            let mut state = fresh_state([0xCD; 32]);
            // Open WirePump (Continuing) so the forge branch is reachable.
            let (block_tx, block_rx) = mpsc::channel::<AdmissionPeerEvent>(4);
            let mut source = NodeBlockSource::from_wire_pump(block_rx);
            let (sd_tx, mut sd_rx) = watch::channel(false);

            let sched = l5_era_schedule();
            let recovered = l5_recovered_state(Some(l5_recovered_inputs()));
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
}
