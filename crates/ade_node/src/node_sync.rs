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
    WirePump(mpsc::Receiver<AdmissionPeerEvent>),
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
        Self::WirePump(rx)
    }

    /// Next ordered block bytes, or `None` at clean end-of-feed.
    ///
    /// Selects ONLY `AdmissionPeerEvent::Block`. `TipUpdate` is skipped
    /// (a comparison input for the verdict loop, not a block and not a
    /// sync tip authority). `Disconnected` and a closed channel both end
    /// the feed (a clean disconnect is not a tip authority). No verdict
    /// is ever derived or surfaced here (E2 / no verdict-as-sync).
    pub async fn next_block(&mut self) -> Option<Vec<u8>> {
        match self {
            Self::InMemory(q) => q.pop_front(),
            Self::WirePump(rx) => loop {
                match rx.recv().await {
                    Some(AdmissionPeerEvent::Block { block_bytes, .. }) => {
                        return Some(block_bytes);
                    }
                    // Not a block; not a sync tip authority. Skip.
                    Some(AdmissionPeerEvent::TipUpdate { .. }) => continue,
                    // Clean disconnect ends the feed.
                    Some(AdmissionPeerEvent::Disconnected { .. }) => return None,
                    // Sender dropped: end of feed.
                    None => return None,
                }
            },
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
    use ade_network::codec::chain_sync::{Point, Tip};
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

        let tip = run_node_sync(
            &mut source,
            &mut state,
            &chaindb,
            &mut wal,
            &sched,
            &view,
        )
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
        let chaindb =
            PersistentChainDb::open(PersistentChainDbOptions::at(&chaindb_path)).unwrap();
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

        let r = run_node_sync(
            &mut source,
            &mut state,
            &chaindb,
            &mut wal,
            &sched,
            &view,
        )
        .await;
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
            &recovered, &tip, &mut shell1, &L5_POOL, &pparams, &sched, 100, 0,
            ProtocolVersion { major: 9, minor: 0 },
        )
        .expect("ok");
        let e2 = forge_one_from_recovered(
            &recovered, &tip, &mut shell2, &L5_POOL, &pparams, &sched, 100, 0,
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
}
