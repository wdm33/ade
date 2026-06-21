// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED Conway-genesis bootstrap entry (PHASE4-N-Y S4).
//!
//! Routes a controlled Conway genesis through the **same** single
//! closed bootstrap authority [`crate::bootstrap::bootstrap_initial_state`]
//! (CN-NODE-01) — never a parallel storage-init path. The genesis
//! file read / parse is the RED [`crate::producer::genesis_parser`]
//! (CN-GENESIS-01); the genesis→initial-state transform is the BLUE
//! [`ade_ledger::genesis_source::genesis_initial_state`]
//! (DC-GENESIS-SRC-01). This shell only composes them and mints the
//! `BootstrapAnchor`.
//!
//! Provenance: a controlled genesis records
//! `SeedProvenance::CardanoCliJson` on the anchor — the genesis JSON
//! is the cardano-cli/operator-supplied seed artifact. No new
//! provenance variant is introduced (cluster §7); no `*Anchor` trait
//! / plugin seam.

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::bootstrap_anchor::{BootstrapAnchor, SeedProvenance};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::genesis_source::{genesis_initial_state, ConwayGenesisConfig, GenesisSourceError};
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{WalError, WalStore};
use ade_types::{Hash32, SlotNo};

use crate::bootstrap::{
    bootstrap_initial_state, BootstrapError, BootstrapInputs, BootstrapState,
    SeedEpochConsensusSource,
};
use crate::bootstrap_anchor::{mint, MintInputs};
use crate::chaindb::{ChainDb, ChainDbError, ChainTip, SnapshotStore};
use crate::consensus_inputs::LiveConsensusInputsCanonical;
use crate::seed_consensus_merge::SeedConsensusMergeError;
use crate::seed_epoch_lineage::SeedEpochLineagePersistError;
use crate::seed_import::UtxoFingerprint;

/// Closed error sum for the genesis-bootstrap entry. RED-side
/// composition errors only; the BLUE transform's fail-closed verdict
/// and the bootstrap authority's verdict are carried through.
#[derive(Debug)]
pub enum GenesisBootstrapError {
    /// The BLUE genesis→initial-state transform fail-closed (e.g. a
    /// non-Conway genesis → `GenesisSourceError::NonConwayEra`).
    GenesisSource(GenesisSourceError),
    /// The single closed bootstrap authority returned an error.
    Bootstrap(BootstrapError),
    /// The seed-epoch consensus-inputs merge fail-closed (a pool present
    /// in exactly one of the stake / VRF-keyhash maps). A2: no provenance
    /// gap is tolerated, so the bootstrap fails.
    SeedConsensusMerge(SeedConsensusMergeError),
    /// Persisting the seed-epoch consensus-inputs sidecar failed. A2: a
    /// bootstrap that cannot record its consensus-input provenance fails
    /// rather than silently proceed.
    SeedConsensusPersist(ChainDbError),
    /// Appending the seed-epoch consensus-inputs WAL provenance entry
    /// failed (A3a). The WAL append is the import's commit point; if it
    /// cannot be written the bootstrap fails rather than leave the
    /// sidecar without a provenance record.
    SeedConsensusProvenanceWal(WalError),
}

/// The genesis-bootstrap entry's typed output: the cold-start state
/// triple the authority produced, plus the minted `BootstrapAnchor`
/// recording the genesis provenance.
#[derive(Debug)]
pub struct GenesisBootstrapOutput {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub tip: Option<ChainTip>,
    pub anchor: BootstrapAnchor,
}

/// SOLE genesis-bootstrap routing entry. Composes the BLUE genesis
/// transform, the anchor mint, and the single closed
/// `bootstrap_initial_state` authority. The genesis pair enters the
/// authority **only** via `BootstrapInputs.genesis_initial`.
///
/// `network_magic` / `genesis_hash` are the parsed genesis identity
/// (RED, from `parse_shelley_genesis` + the genesis file digest);
/// they are recorded on the anchor but never decide the cold-start
/// branch. The chaindb / snapshot store are the (empty) cold-start
/// backends; a non-empty store routes the authority to warm-start,
/// which is S2/S3's path, not this one.
#[allow(clippy::too_many_arguments)]
pub fn bootstrap_from_conway_genesis<D, S>(
    conway_genesis: &ConwayGenesisConfig,
    network_magic: u32,
    genesis_hash: Hash32,
    genesis_artifact_hash: Hash32,
    seed_consensus_inputs: &LiveConsensusInputsCanonical,
    chaindb: &D,
    snapshot_store: &S,
    wal: &mut dyn WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<GenesisBootstrapOutput, GenesisBootstrapError>
where
    D: ChainDb,
    S: SnapshotStore + ?Sized,
{
    let (genesis_ledger, genesis_chain_dep) =
        genesis_initial_state(conway_genesis).map_err(GenesisBootstrapError::GenesisSource)?;

    let imported_utxo_fingerprint = fingerprint(&genesis_ledger).utxo;
    let initial_ledger_fingerprint = fingerprint(&genesis_ledger).combined;

    let anchor = mint(MintInputs {
        network_magic,
        genesis_hash,
        seed_slot: SlotNo(0),
        seed_block_hash: Hash32([0u8; 32]),
        seed_artifact_hash: genesis_artifact_hash,
        imported_utxo_fingerprint: UtxoFingerprint(imported_utxo_fingerprint),
        initial_ledger_fingerprint,
        seed_provenance: SeedProvenance::CardanoCliJson,
    });

    let BootstrapState {
        ledger,
        chain_dep,
        tip,
        ..
    } = bootstrap_initial_state(BootstrapInputs {
        chaindb,
        snapshot_store,
        era_schedule,
        ledger_view,
        genesis_initial: Some((genesis_ledger, genesis_chain_dep)),
        // A3b: genesis composition is a cold-start; no recovered
        // sidecar to demand (the composer writes the sidecar after
        // bootstrap, it does not consume one).
        seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
        // AK-S1: cold-start composition resolves no recovered anchor (the
        // composer persists the anchor-point record after bootstrap; the
        // warm-start recover path is what loads + resolves it).
        recovered_anchor: None,
    })
    .map_err(GenesisBootstrapError::Bootstrap)?;

    persist_seed_epoch_consensus_inputs(snapshot_store, wal, &anchor, seed_consensus_inputs)?;

    Ok(GenesisBootstrapOutput {
        ledger,
        chain_dep,
        tip,
        anchor,
    })
}

/// Build the anchor-bound seed-epoch consensus-inputs sidecar from the
/// verified-bootstrap canonical inputs and persist it through the
/// dedicated anchor-fp-keyed `SnapshotStore` surface (A2). `anchor_fp`
/// is the anchor's `initial_ledger_fingerprint`; `epoch_no` is the
/// canonical inputs' seed epoch. Fail-closed on a merge gap or a store
/// write failure — a bootstrap without recorded consensus-input
/// provenance is not allowed to proceed.
fn persist_seed_epoch_consensus_inputs<S>(
    snapshot_store: &S,
    wal: &mut dyn WalStore,
    anchor: &BootstrapAnchor,
    seed_consensus_inputs: &LiveConsensusInputsCanonical,
) -> Result<(), GenesisBootstrapError>
where
    S: SnapshotStore + ?Sized,
{
    crate::seed_epoch_lineage::persist_seed_epoch_consensus_inputs(
        snapshot_store,
        wal,
        anchor,
        seed_consensus_inputs,
    )
    .map_err(|e| match e {
        SeedEpochLineagePersistError::Merge(x) => GenesisBootstrapError::SeedConsensusMerge(x),
        SeedEpochLineagePersistError::Persist(x) => GenesisBootstrapError::SeedConsensusPersist(x),
        SeedEpochLineagePersistError::ProvenanceWal(x) => {
            GenesisBootstrapError::SeedConsensusProvenanceWal(x)
        }
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use ade_core::consensus::praos_state::Nonce;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::genesis_source::GenesisInitialFund;
    use ade_ledger::utxo::TxOut;
    use ade_types::tx::{Coin, TxIn};
    use ade_types::{CardanoEra, EpochNo, Hash28, SlotNo};

    use ade_ledger::wal::{
        decode_wal_entry, replay_from_anchor, RecoveredBootstrapProvenance, WalEntry,
    };

    use crate::chaindb::InMemoryChainDb;

    /// Minimal append-order in-memory `WalStore` double: enough to
    /// thread through the composer and read back the appended
    /// provenance entry.
    struct VecWal {
        entries: Vec<WalEntry>,
    }
    impl VecWal {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }
    }
    impl ade_ledger::wal::WalStore for VecWal {
        fn append(&mut self, entry: WalEntry) -> Result<(), ade_ledger::wal::WalError> {
            self.entries.push(entry);
            Ok(())
        }
        fn read_all(&self) -> Result<Vec<WalEntry>, ade_ledger::wal::WalError> {
            Ok(self.entries.clone())
        }
    }

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

    fn empty_view() -> PoolDistrView {
        let asc = ActiveSlotsCoeff {
            numer: 5,
            denom: 100,
        };
        let pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        PoolDistrView::new(EPOCH_576, 1, asc, pools)
    }

    fn conway_config() -> ConwayGenesisConfig {
        ConwayGenesisConfig {
            era: CardanoEra::Conway,
            initial_nonce: Nonce(Hash32([0xCD; 32])),
            initial_funds: vec![GenesisInitialFund {
                tx_in: TxIn {
                    tx_hash: Hash32([0x01; 32]),
                    index: 0,
                },
                tx_out: TxOut::ShelleyMary {
                    address: vec![0xAA; 29],
                    value: ade_ledger::value::Value::from_coin(Coin(1_000_000)),
                },
            }],
        }
    }

    fn seed_inputs() -> LiveConsensusInputsCanonical {
        let mut stake = BTreeMap::new();
        stake.insert(Hash28([0x01; 28]), 1_000u64);
        stake.insert(Hash28([0x05; 28]), 2_500u64);
        let mut vrfs = BTreeMap::new();
        vrfs.insert(Hash28([0x01; 28]), Hash32([0x07; 32]));
        vrfs.insert(Hash28([0x05; 28]), Hash32([0x08; 32]));
        crate::seed_consensus_merge::test_canonical_inputs(EPOCH_576, stake, vrfs)
    }

    #[test]
    fn conway_genesis_bootstrap_through_single_authority() {
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let sched = schedule();
        let view = empty_view();
        let cfg = conway_config();
        let inputs = seed_inputs();

        let out = bootstrap_from_conway_genesis(
            &cfg,
            /*network_magic=*/ 1,
            /*genesis_hash=*/ Hash32([0x11; 32]),
            /*genesis_artifact_hash=*/ Hash32([0x22; 32]),
            &inputs,
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect("genesis bootstrap");

        // The genesis pair entered via the cold-start branch of the
        // single authority: no tip (cold-start), nonce seeded from the
        // genesis-derived value, anchor records the genesis provenance.
        assert!(out.tip.is_none(), "cold-start has no tip");
        assert_eq!(out.chain_dep.epoch_nonce, Nonce(Hash32([0xCD; 32])));
        assert_eq!(out.ledger.utxo_state.len(), 1);
        assert_eq!(out.anchor.seed_provenance, SeedProvenance::CardanoCliJson);
        assert_eq!(out.anchor.network_magic, 1);

        // The authority's cold-start output equals the BLUE transform's
        // output — the genesis pair is not re-derived on any side path.
        let (direct_ledger, direct_chain_dep) =
            genesis_initial_state(&cfg).expect("blue transform");
        assert_eq!(
            fingerprint(&out.ledger).combined,
            fingerprint(&direct_ledger).combined
        );
        assert_eq!(out.chain_dep, direct_chain_dep);
        assert_eq!(
            out.anchor.initial_ledger_fingerprint,
            fingerprint(&direct_ledger).combined
        );
    }

    #[test]
    fn non_conway_genesis_bootstrap_fails_closed() {
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let sched = schedule();
        let view = empty_view();
        let inputs = seed_inputs();
        let mut cfg = conway_config();
        cfg.era = CardanoEra::Babbage;

        let err = bootstrap_from_conway_genesis(
            &cfg,
            1,
            Hash32([0x11; 32]),
            Hash32([0x22; 32]),
            &inputs,
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect_err("non-conway must fail closed");
        assert!(matches!(
            err,
            GenesisBootstrapError::GenesisSource(GenesisSourceError::NonConwayEra {
                found: CardanoEra::Babbage
            })
        ));
        // No storage initialized on the fail-closed path.
        assert!(db.list_snapshot_slots().expect("list").is_empty());
        // And no WAL provenance entry on the fail-closed path.
        assert!(wal.read_all().expect("read_all").is_empty());
    }

    #[test]
    fn bootstrap_persists_anchor_keyed_seed_consensus_inputs() {
        use ade_ledger::consensus_view::PoolEntry as BluePoolEntry;
        use ade_ledger::seed_consensus_inputs::{
            decode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };

        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let sched = schedule();
        let view = empty_view();
        let cfg = conway_config();
        let inputs = seed_inputs();

        let out = bootstrap_from_conway_genesis(
            &cfg,
            1,
            Hash32([0x11; 32]),
            Hash32([0x22; 32]),
            &inputs,
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect("genesis bootstrap");

        // The sidecar is keyed by the anchor's initial_ledger_fingerprint.
        let anchor_fp = out.anchor.initial_ledger_fingerprint.clone();
        let stored = db
            .get_seed_epoch_consensus_inputs(&anchor_fp)
            .expect("get sidecar")
            .expect("sidecar present after bootstrap");

        // It decodes via the A1 sole codec to the expected merged record:
        // the two RED maps (stake, vrf) zipped into the single BLUE map,
        // total_active_stake = the saturating sum.
        let decoded = decode_seed_epoch_consensus_inputs(&stored).expect("decode via A1");
        let mut expected_pools = BTreeMap::new();
        expected_pools.insert(
            Hash28([0x01; 28]),
            BluePoolEntry {
                active_stake: 1_000,
                vrf_keyhash: Hash32([0x07; 32]),
            },
        );
        expected_pools.insert(
            Hash28([0x05; 28]),
            BluePoolEntry {
                active_stake: 2_500,
                vrf_keyhash: Hash32([0x08; 32]),
            },
        );
        let expected = SeedEpochConsensusInputs {
            anchor_fp: anchor_fp.clone(),
            epoch_no: EPOCH_576,
            epoch_start_slot: inputs.epoch_start_slot,
            epoch_length_slots: inputs.epoch_length_slots().expect("valid epoch window"),
            epoch_nonce: inputs.epoch_nonce.clone(),
            genesis_hash: inputs.genesis_hash.clone(),
            protocol_params_hash: inputs.protocol_params_hash.clone(),
            active_slots_coeff: inputs.active_slots_coeff,
            total_active_stake: 3_500,
            pool_distribution: expected_pools,
        };
        assert_eq!(decoded, expected);
        assert_eq!(decoded.anchor_fp, out.anchor.initial_ledger_fingerprint);

        // The sidecar lives in its own namespace — it created no
        // slot-keyed snapshot.
        assert!(db.list_snapshot_slots().expect("list").is_empty());

        // A3a: the WAL recorded the provenance entry AFTER the sidecar
        // put (the commit point), with sidecar_hash bound to the exact
        // stored A1 bytes. This is the put-then-append ordering: the
        // sidecar is present (asserted above) AND the WAL entry exists.
        let entries = wal.read_all().expect("read_all");
        assert_eq!(entries.len(), 1, "exactly one provenance entry");
        match &entries[0] {
            WalEntry::SeedEpochConsensusInputsImported {
                anchor_fp: a,
                sidecar_hash,
                epoch_no,
            } => {
                assert_eq!(*a, anchor_fp);
                assert_eq!(*epoch_no, EPOCH_576);
                assert_eq!(*sidecar_hash, ade_crypto::blake2b_256(&stored));
            }
            other => panic!("expected provenance entry, got {other:?}"),
        }

        // The appended entry is itself canonically encodable, and replay
        // surfaces the provenance view bound to this anchor.
        let _ = decode_wal_entry(&ade_ledger::wal::encode_wal_entry(&entries[0]))
            .expect("entry round-trips");
        let bb = BTreeMap::new();
        let replay = replay_from_anchor(&anchor_fp, &entries, &bb).expect("replay");
        assert_eq!(
            replay.provenance,
            Some(RecoveredBootstrapProvenance {
                anchor_fp: anchor_fp.clone(),
                sidecar_hash: ade_crypto::blake2b_256(&stored),
                epoch_no: EPOCH_576,
            })
        );
    }
}
