// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Forward-sync durable lifecycle (PHASE4-N-Y S2).
//!
//! Two-driver split mirroring `session` / `mux_pump`:
//!   - [`reducer`] — GREEN lifecycle reducer. Composes the BLUE admit
//!     chokepoint (`admit_via_block_validity`) and emits a closed
//!     [`SyncEffect`] plan whose `AdvanceTip` is unreachable until the
//!     block's `StoreBlockBytes` + `AppendWal` precede it (DC-SYNC-01).
//!   - [`pump`] — RED driver applying the plan in order against the
//!     persistent `ChainDb` + WAL, refusing to advance the tip before
//!     the durability writes return Ok.

pub mod pump;
pub mod reducer;

pub use pump::{pump_block, NoCheckpointSink, PumpError, PumpTip, SnapshotSink};
pub use reducer::{forward_sync_step, AdmitPlan, ForwardSyncState, SyncEffect};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod replay_tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::receive::ReceiveState;
    use ade_ledger::state::LedgerState;
    use ade_ledger::wal::{encode_wal_entry, WalEntry, WalError, WalStore};
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    use crate::chaindb::InMemoryChainDb;
    use crate::rollback::cadence::SnapshotCadence;

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

    fn corpus_view() -> (ConwayValidityCorpus, ade_ledger::consensus_view::PoolDistrView) {
        use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
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

    fn pick_lightest(c: &ConwayValidityCorpus) -> Vec<u8> {
        let idx = (0..c.blocks.len())
            .min_by_key(|&i| {
                let env = decode_block_envelope(&c.blocks[i]).expect("env");
                env.block_end - env.block_start
            })
            .expect("non-empty");
        c.blocks[idx].clone()
    }

    #[derive(Default)]
    struct VecWal {
        entries: Vec<WalEntry>,
    }
    impl WalStore for VecWal {
        fn append(&mut self, entry: WalEntry) -> Result<(), WalError> {
            self.entries.push(entry);
            Ok(())
        }
        fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
            Ok(self.entries.clone())
        }
    }

    /// Run the forward-sync pump over an ordered block sequence and
    /// return `(post-state ledger fingerprint, concatenated canonical
    /// WAL bytes)` — the replay-equivalence surface.
    fn run_sync(eta0: [u8; 32], seq: &[Vec<u8>]) -> (Hash32, Vec<u8>) {
        let (_, view) = corpus_view();
        let sched = schedule();
        let mut state = fresh_state(eta0);
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::default();
        for bytes in seq {
            pump_block(
                &mut state,
                &db,
                &mut wal,
                &NoCheckpointSink,
                bytes,
                &sched,
                &view,
            )
            .expect("pump");
        }
        let fp = ade_ledger::fingerprint::fingerprint(&state.receive.ledger).combined;
        let mut wal_bytes = Vec::new();
        for e in wal.read_all().expect("read_all") {
            wal_bytes.extend_from_slice(&encode_wal_entry(&e));
        }
        (fp, wal_bytes)
    }

    #[test]
    fn forward_sync_replay_two_runs_byte_identical() {
        let (c, _view) = corpus_view();
        // Synthetic-but-representative in-tree sequence: a corpus block
        // (the lightest Conway block) replayed from the same anchor.
        // S2's replay property is two-run byte-identity over the same
        // ordered sequence; a real preprod snapshot→tip capture is S5
        // operator-evidence scope.
        let seq = vec![pick_lightest(&c)];

        let (fp1, wal1) = run_sync(c.epoch_nonce, &seq);
        let (fp2, wal2) = run_sync(c.epoch_nonce, &seq);

        assert_eq!(fp1, fp2, "post-state fingerprint must be byte-identical");
        assert_eq!(wal1, wal2, "WAL bytes must be byte-identical");
    }
}
