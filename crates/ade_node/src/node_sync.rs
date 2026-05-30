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
use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::wal::WalStore;
use ade_runtime::admission::AdmissionPeerEvent;
use ade_runtime::chaindb::{ChainDb, SnapshotStore};
use ade_runtime::forward_sync::{pump_block, ForwardSyncState, NoCheckpointSink, PumpTip};
use ade_runtime::rollback::PersistentSnapshotCache;
use tokio::sync::mpsc;

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
}
