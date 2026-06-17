// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED Mithril-snapshot bootstrap entry (PHASE4-N-Z S1).
//!
//! Routes a Mithril-sourced seed through the **same** single closed
//! bootstrap authority [`crate::bootstrap::bootstrap_initial_state`]
//! (CN-NODE-01) — never a parallel storage-init path. Mirrors
//! [`crate::genesis_bootstrap::bootstrap_from_conway_genesis`] in
//! shape: a composition-only RED shell with a closed error surface
//! and no new authority.
//!
//! DC-MITHRIL-02 — the load-bearing rule: the anchor's `seed_point`
//! (`seed_slot` / `seed_block_hash`) is minted from the
//! **operator-provided** [`MithrilSeedPointInputs`], an origin
//! structurally independent of the Mithril manifest. The manifest
//! import only populates `seed_provenance` (`SeedProvenance::Mithril`,
//! recording what the cert attests). `verify_mithril_binding` then
//! cross-checks the manifest's attested `certified_point` against the
//! independently-supplied `anchor.seed_point`; a disagreement fails
//! closed **before** any `bootstrap_initial_state` call, so no storage
//! initializes on a mismatched binding (CN-MITHRIL-01).

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::bootstrap_anchor::{verify_mithril_binding, BootstrapAnchor, MithrilImportError};
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
use crate::mithril_import::{import_mithril_manifest_from_bytes, MithrilManifestError};
use crate::seed_consensus_merge::SeedConsensusMergeError;
use crate::seed_epoch_lineage::SeedEpochLineagePersistError;
use crate::seed_import::UtxoFingerprint;

/// Operator-provided seed-point extraction inputs — the origin that is
/// **structurally independent** of the Mithril manifest. A separate
/// struct from the manifest bytes by construction (DC-MITHRIL-02): the
/// anchor's `seed_point` is minted from `seed_slot` / `seed_block_hash`
/// here, never from `import.report.*` / `manifest.certified_point` /
/// the `SeedProvenance::Mithril` fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MithrilSeedPointInputs {
    pub seed_slot: SlotNo,
    pub seed_block_hash: Hash32,
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub seed_artifact_hash: Hash32,
    pub imported_utxo_fingerprint: UtxoFingerprint,
    pub initial_ledger_fingerprint: Hash32,
}

/// Closed error sum for the Mithril-bootstrap entry. RED-side
/// composition errors only: the manifest parse, the BLUE binding
/// verdict, and the single bootstrap authority's verdict are each
/// carried through their own variant.
#[derive(Debug)]
pub enum MithrilBootstrapError {
    /// The RED manifest parse fail-closed (malformed manifest).
    Import(MithrilManifestError),
    /// The BLUE binding predicate fail-closed (a field mismatch
    /// between the manifest's attested side and the independently
    /// minted anchor) — no storage init.
    Binding(MithrilImportError),
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

/// The Mithril-bootstrap entry's typed output: the cold-start state
/// triple the authority produced, plus the minted `BootstrapAnchor`
/// recording the Mithril provenance.
#[derive(Debug)]
pub struct MithrilBootstrapOutput {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub tip: Option<ChainTip>,
    pub anchor: BootstrapAnchor,
}

/// SOLE Mithril-bootstrap routing entry. Composes the RED manifest
/// import, the anchor mint, the BLUE `verify_mithril_binding`
/// cross-check, and the single closed `bootstrap_initial_state`
/// authority — in that order.
///
/// `seed_point_inputs` is the operator's independent seed-point
/// extraction; `manifest_bytes` is the Mithril manifest. The two are
/// separate parameters by construction (DC-MITHRIL-02). The seed pair
/// `(seed_ledger, seed_chain_dep)` is the operator-supplied cold-start
/// state; it enters the authority only via `BootstrapInputs.genesis_initial`.
#[allow(clippy::too_many_arguments)]
pub fn bootstrap_from_mithril_snapshot<D, S>(
    seed_point_inputs: &MithrilSeedPointInputs,
    seed_ledger: LedgerState,
    seed_chain_dep: PraosChainDepState,
    manifest_bytes: &[u8],
    seed_consensus_inputs: &LiveConsensusInputsCanonical,
    chaindb: &D,
    snapshot_store: &S,
    wal: &mut dyn WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<MithrilBootstrapOutput, MithrilBootstrapError>
where
    D: ChainDb,
    S: SnapshotStore + ?Sized,
{
    let import = import_mithril_manifest_from_bytes(manifest_bytes)
        .map_err(MithrilBootstrapError::Import)?;

    let anchor = mint(MintInputs {
        network_magic: seed_point_inputs.network_magic,
        genesis_hash: seed_point_inputs.genesis_hash.clone(),
        seed_slot: seed_point_inputs.seed_slot,
        seed_block_hash: seed_point_inputs.seed_block_hash.clone(),
        seed_artifact_hash: seed_point_inputs.seed_artifact_hash.clone(),
        imported_utxo_fingerprint: seed_point_inputs.imported_utxo_fingerprint.clone(),
        initial_ledger_fingerprint: seed_point_inputs.initial_ledger_fingerprint.clone(),
        seed_provenance: import.provenance,
    });

    verify_mithril_binding(&import.report, &anchor).map_err(MithrilBootstrapError::Binding)?;

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
        genesis_initial: Some((seed_ledger, seed_chain_dep)),
        // A3b: Mithril composition is a cold-start; no recovered
        // sidecar to demand (the composer writes it after bootstrap).
        seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired,
        // AK-S1: cold-start composition resolves no recovered anchor (the
        // composer persists the anchor-point record after bootstrap, like the
        // sidecar; the warm-start recover path is what loads + resolves it).
        recovered_anchor: None,
    })
    .map_err(MithrilBootstrapError::Bootstrap)?;

    persist_seed_epoch_consensus_inputs(snapshot_store, wal, &anchor, seed_consensus_inputs)?;

    Ok(MithrilBootstrapOutput {
        ledger,
        chain_dep,
        tip,
        anchor,
    })
}

/// Build the anchor-bound seed-epoch consensus-inputs sidecar from the
/// verified-bootstrap canonical inputs and persist it through the
/// dedicated anchor-fp-keyed `SnapshotStore` surface (A2). Runs only
/// after `verify_mithril_binding` has passed (this helper is called from
/// the success tail of `bootstrap_from_mithril_snapshot`). `anchor_fp`
/// is the anchor's `initial_ledger_fingerprint`; `epoch_no` is the
/// canonical inputs' seed epoch. Fail-closed on a merge gap or a store
/// write failure.
fn persist_seed_epoch_consensus_inputs<S>(
    snapshot_store: &S,
    wal: &mut dyn WalStore,
    anchor: &BootstrapAnchor,
    seed_consensus_inputs: &LiveConsensusInputsCanonical,
) -> Result<(), MithrilBootstrapError>
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
        SeedEpochLineagePersistError::Merge(x) => MithrilBootstrapError::SeedConsensusMerge(x),
        SeedEpochLineagePersistError::Persist(x) => MithrilBootstrapError::SeedConsensusPersist(x),
        SeedEpochLineagePersistError::ProvenanceWal(x) => {
            MithrilBootstrapError::SeedConsensusProvenanceWal(x)
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
    use ade_ledger::bootstrap_anchor::SeedProvenance;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::wal::{replay_from_anchor, RecoveredBootstrapProvenance, WalEntry};
    use ade_types::{CardanoEra, EpochNo, Hash28};

    use crate::chaindb::InMemoryChainDb;

    /// Minimal append-order in-memory `WalStore` double.
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

    // The manifest's attested certified_point Q. The matching tests
    // set the operator seed_point to this; the mismatch test sets it
    // to a genuinely different point P.
    const MANIFEST_SLOT: u64 = 23_013_663;
    const MANIFEST_BLOCK_HASH: [u8; 32] = [0x22; 32];
    const MANIFEST_CERT_HASH: [u8; 32] = [0x66; 32];
    const MANIFEST_GENESIS_HASH: [u8; 32] = [0x11; 32];
    const MANIFEST_NETWORK_MAGIC: u32 = 1;

    const MANIFEST: &str = r#"{
        "artifact_type": "cardano-database-snapshot",
        "certificate_hash_hex": "6666666666666666666666666666666666666666666666666666666666666666",
        "network_magic": 1,
        "genesis_hash_hex": "1111111111111111111111111111111111111111111111111111111111111111",
        "certified_point": {
            "slot": 23013663,
            "block_hash_hex": "2222222222222222222222222222222222222222222222222222222222222222"
        },
        "immutable_range": { "lo": 0, "hi": 4242 },
        "source_mithril_client_version": "mithril-client 0.10.0",
        "source_command": "mithril-client cardano-db download latest"
    }"#;

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

    fn seed_state() -> (LedgerState, PraosChainDepState) {
        let ledger = LedgerState::new(CardanoEra::Conway);
        let chain_dep = PraosChainDepState::genesis(Nonce(Hash32([0xCD; 32])));
        (ledger, chain_dep)
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

    /// Operator seed-point inputs whose `seed_slot` / `seed_block_hash`
    /// equal the manifest's attested certified_point (the agreeing
    /// case). Independent origin — these fields are typed in by the
    /// operator, not read from the manifest.
    fn matching_seed_point_inputs() -> MithrilSeedPointInputs {
        MithrilSeedPointInputs {
            seed_slot: SlotNo(MANIFEST_SLOT),
            seed_block_hash: Hash32(MANIFEST_BLOCK_HASH),
            network_magic: MANIFEST_NETWORK_MAGIC,
            genesis_hash: Hash32(MANIFEST_GENESIS_HASH),
            seed_artifact_hash: Hash32([0x33; 32]),
            imported_utxo_fingerprint: UtxoFingerprint(Hash32([0x44; 32])),
            initial_ledger_fingerprint: Hash32([0x55; 32]),
        }
    }

    #[test]
    fn mithril_bootstrap_verifies_before_storage_init() {
        // Operator seed-point P ≠ manifest certified_point Q: the
        // binding must fail, and the store must stay empty. This
        // proves verify_mithril_binding runs and must be Ok before
        // bootstrap_initial_state writes anything (call-order).
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let sched = schedule();
        let view = empty_view();
        let (ledger, chain_dep) = seed_state();

        let mut inputs = matching_seed_point_inputs();
        inputs.seed_slot = SlotNo(99_999_999);
        inputs.seed_block_hash = Hash32([0xAB; 32]);
        let cinputs = seed_inputs();

        let err = bootstrap_from_mithril_snapshot(
            &inputs,
            ledger,
            chain_dep,
            MANIFEST.as_bytes(),
            &cinputs,
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect_err("mismatched seed_point must fail closed before storage init");
        assert!(matches!(err, MithrilBootstrapError::Binding(_)));
        assert!(
            db.list_snapshot_slots().expect("list").is_empty(),
            "storage must not initialize before a verified binding"
        );
        // No WAL provenance entry on the pre-binding fail-closed path.
        assert!(wal.read_all().expect("read_all").is_empty());
    }

    #[test]
    fn mithril_bootstrap_fails_closed_on_seed_point_mismatch() {
        // Operator seed-point P ≠ manifest certified_point Q →
        // Binding(CertifiedPointMismatch); no bootstrap_initial_state
        // side effect (store stays empty). Load-bearing: if seed_point
        // were sourced from the manifest, P would equal Q and this
        // would never fail.
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let sched = schedule();
        let view = empty_view();
        let (ledger, chain_dep) = seed_state();

        let mut inputs = matching_seed_point_inputs();
        inputs.seed_slot = SlotNo(99_999_999);
        inputs.seed_block_hash = Hash32([0xAB; 32]);
        let cinputs = seed_inputs();
        // The operator's independent point is genuinely different from
        // what the manifest attests.
        assert_ne!(inputs.seed_slot, SlotNo(MANIFEST_SLOT));

        let err = bootstrap_from_mithril_snapshot(
            &inputs,
            ledger,
            chain_dep,
            MANIFEST.as_bytes(),
            &cinputs,
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect_err("seed-point mismatch must fail closed");
        assert!(matches!(
            err,
            MithrilBootstrapError::Binding(MithrilImportError::CertifiedPointMismatch)
        ));
        assert!(
            db.list_snapshot_slots().expect("list").is_empty(),
            "no bootstrap_initial_state side effect on a mismatched binding"
        );
        // No WAL provenance entry on the mismatched-binding path.
        assert!(wal.read_all().expect("read_all").is_empty());
    }

    #[test]
    fn mithril_bootstrap_succeeds_when_seed_point_matches() {
        // Operator seed-point == manifest certified_point → bootstrap
        // proceeds; anchor records SeedProvenance::Mithril.
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let sched = schedule();
        let view = empty_view();
        let (ledger, chain_dep) = seed_state();
        let inputs = matching_seed_point_inputs();
        let cinputs = seed_inputs();

        let out = bootstrap_from_mithril_snapshot(
            &inputs,
            ledger,
            chain_dep,
            MANIFEST.as_bytes(),
            &cinputs,
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect("matching seed-point binds and bootstraps");

        assert!(out.tip.is_none(), "cold-start has no tip");
        assert_eq!(out.chain_dep.epoch_nonce, Nonce(Hash32([0xCD; 32])));
        assert!(matches!(
            out.anchor.seed_provenance,
            SeedProvenance::Mithril { .. }
        ));
        assert_eq!(out.anchor.seed_point.slot, SlotNo(MANIFEST_SLOT));
        assert_eq!(out.anchor.network_magic, MANIFEST_NETWORK_MAGIC);
        assert_eq!(MANIFEST_CERT_HASH, [0x66; 32]);
    }

    #[test]
    fn bootstrap_persists_anchor_keyed_seed_consensus_inputs() {
        use ade_ledger::consensus_view::PoolEntry as BluePoolEntry;
        use ade_ledger::seed_consensus_inputs::{
            decode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
        };

        // After a verified Mithril binding, the sidecar is persisted
        // anchor-keyed and decodes via the A1 sole codec to the merged
        // record.
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let sched = schedule();
        let view = empty_view();
        let (ledger, chain_dep) = seed_state();
        let inputs = matching_seed_point_inputs();
        let cinputs = seed_inputs();

        let out = bootstrap_from_mithril_snapshot(
            &inputs,
            ledger,
            chain_dep,
            MANIFEST.as_bytes(),
            &cinputs,
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect("matching seed-point binds and bootstraps");

        let anchor_fp = out.anchor.initial_ledger_fingerprint.clone();
        let stored = db
            .get_seed_epoch_consensus_inputs(&anchor_fp)
            .expect("get sidecar")
            .expect("sidecar present after Mithril bootstrap");

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
            epoch_start_slot: cinputs.epoch_start_slot,
            epoch_length_slots: cinputs.epoch_length_slots().expect("valid epoch window"),
            epoch_nonce: cinputs.epoch_nonce.clone(),
            active_slots_coeff: cinputs.active_slots_coeff,
            total_active_stake: 3_500,
            pool_distribution: expected_pools,
        };
        assert_eq!(decoded, expected);
        assert_eq!(decoded.anchor_fp, out.anchor.initial_ledger_fingerprint);

        // The sidecar created no slot-keyed snapshot — disjoint namespace.
        assert!(db.list_snapshot_slots().expect("list").is_empty());

        // A3a: the WAL recorded the provenance entry after the sidecar
        // put (commit point); replay surfaces the bound view.
        let entries = wal.read_all().expect("read_all");
        assert_eq!(entries.len(), 1, "exactly one provenance entry");
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
