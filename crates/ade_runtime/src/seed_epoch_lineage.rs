// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED shared seed-epoch anchor-lineage persistence (PHASE4-N-F-G-I).
//!
//! The SINGLE persistence of the anchor-fp-keyed seed-epoch consensus-inputs
//! sidecar + its WAL provenance commit, shared by every bootstrap that records
//! a seed-epoch lineage: the Mithril production composition
//! (`crate::mithril_bootstrap`), the controlled Conway-genesis bootstrap
//! (`crate::genesis_bootstrap`), AND the operator admission/pre-seed bootstrap
//! (`ade_node::admission::bootstrap`).
//!
//! Extracted verbatim from the two former (their bodies were byte-identical
//! modulo the wrapping error). The admission bootstrap mints the same
//! `BootstrapAnchor` and now persists the same lineage, so a `--mode node`
//! WarmStart can recover a forge-capable store seeded purely from the shared
//! `--json-seed` + `import_live_consensus_inputs` path. This is NOT a
//! bootstrap-from-genesis: the lineage derives ONLY from the minted anchor
//! (its `initial_ledger_fingerprint`) + the imported
//! `LiveConsensusInputsCanonical` (its seed epoch) — never from a
//! genesis-derived eta0/stake/ASC constructor.

use ade_ledger::bootstrap_anchor::BootstrapAnchor;
use ade_ledger::seed_consensus_inputs::encode_seed_epoch_consensus_inputs;
use ade_ledger::wal::{WalError, WalStore};
use ade_types::EpochNo;

use crate::chaindb::{ChainDbError, SnapshotStore};
use crate::consensus_inputs::LiveConsensusInputsCanonical;
use crate::seed_consensus_merge::{merge_seed_epoch_consensus_inputs, SeedConsensusMergeError};
use crate::seed_consensus_provenance::append_seed_epoch_provenance;

/// Closed error sum for the shared seed-epoch lineage persistence. Each
/// caller maps these into its own bootstrap error so its public surface is
/// unchanged.
#[derive(Debug)]
pub enum SeedEpochLineagePersistError {
    /// The seed-epoch consensus-inputs merge fail-closed (a pool present in
    /// exactly one of the stake / VRF-keyhash maps). No provenance gap is
    /// tolerated, so the bootstrap fails.
    Merge(SeedConsensusMergeError),
    /// Persisting the anchor-fp-keyed sidecar failed — a bootstrap that
    /// cannot record its consensus-input provenance fails rather than
    /// silently proceed.
    Persist(ChainDbError),
    /// Appending the seed-epoch WAL provenance entry (the import's commit
    /// point) failed — fail rather than leave the sidecar without a
    /// provenance record.
    ProvenanceWal(WalError),
}

/// Build the anchor-bound seed-epoch consensus-inputs sidecar from the
/// minted anchor + the imported canonical inputs and persist it.
///
/// Ordering (A3a): sidecar put (durable) FIRST, then the WAL provenance
/// append (the commit point). A crash between the two leaves the sidecar
/// with no provenance entry — recovered as "not imported" — preserving the
/// fail-closed warm-start recovery semantics. `anchor_fp` is the anchor's
/// `initial_ledger_fingerprint`; `epoch_no` is the canonical inputs' seed
/// epoch.
pub fn persist_seed_epoch_consensus_inputs<S>(
    snapshot_store: &S,
    wal: &mut dyn WalStore,
    anchor: &BootstrapAnchor,
    seed_consensus_inputs: &LiveConsensusInputsCanonical,
) -> Result<(), SeedEpochLineagePersistError>
where
    S: SnapshotStore + ?Sized,
{
    let anchor_fp = anchor.initial_ledger_fingerprint.clone();
    let epoch_no: EpochNo = seed_consensus_inputs.epoch_no;
    let record =
        merge_seed_epoch_consensus_inputs(anchor_fp.clone(), epoch_no, seed_consensus_inputs)
            .map_err(SeedEpochLineagePersistError::Merge)?;
    let bytes = encode_seed_epoch_consensus_inputs(&record);
    snapshot_store
        .put_seed_epoch_consensus_inputs(&anchor_fp, &bytes)
        .map_err(SeedEpochLineagePersistError::Persist)?;
    append_seed_epoch_provenance(wal, &anchor_fp, epoch_no, &bytes)
        .map_err(SeedEpochLineagePersistError::ProvenanceWal)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ade_ledger::bootstrap_anchor::SeedProvenance;
    use ade_ledger::wal::{replay_from_anchor, WalEntry};
    use ade_types::{Hash28, Hash32, SlotNo};

    use crate::bootstrap_anchor::{mint, MintInputs};
    use crate::chaindb::InMemoryChainDb;
    use crate::seed_import::UtxoFingerprint;

    const EPOCH: EpochNo = EpochNo(576);

    /// Minimal in-memory `WalStore` double for unit coverage.
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

    fn sample_anchor() -> BootstrapAnchor {
        mint(MintInputs {
            network_magic: 42,
            genesis_hash: Hash32([0x11; 32]),
            seed_slot: SlotNo(0),
            seed_block_hash: Hash32([0u8; 32]),
            seed_artifact_hash: Hash32([0x22; 32]),
            imported_utxo_fingerprint: UtxoFingerprint(Hash32([0x33; 32])),
            initial_ledger_fingerprint: Hash32([0x42; 32]),
            seed_provenance: SeedProvenance::CardanoCliJson,
        })
    }

    fn sample_inputs() -> LiveConsensusInputsCanonical {
        let mut stake = BTreeMap::new();
        stake.insert(Hash28([0x01; 28]), 1_000u64);
        let mut vrfs = BTreeMap::new();
        vrfs.insert(Hash28([0x01; 28]), Hash32([0x07; 32]));
        crate::seed_consensus_merge::test_canonical_inputs(EPOCH, stake, vrfs)
    }

    /// PHASE4-N-F-G-I CE-G-I-1: the shared persist authority — which the
    /// admission/pre-seed bootstrap now invokes — writes BOTH the anchor-fp-keyed
    /// sidecar AND a replay-recoverable WAL provenance entry: exactly the lineage
    /// a `--mode node` WarmStart recovery requires (its absence was the
    /// WarmStartNoAnchorLineage gap).
    #[test]
    fn persist_writes_anchor_keyed_sidecar_and_recoverable_wal_provenance() {
        let db = InMemoryChainDb::new();
        let mut wal = VecWal {
            entries: Vec::new(),
        };
        let anchor = sample_anchor();
        let inputs = sample_inputs();

        persist_seed_epoch_consensus_inputs(&db, &mut wal, &anchor, &inputs).expect("persist");

        let anchor_fp = anchor.initial_ledger_fingerprint.clone();
        // The sidecar is present, keyed by the anchor's initial_ledger_fingerprint.
        let stored = db
            .get_seed_epoch_consensus_inputs(&anchor_fp)
            .expect("get sidecar")
            .expect("sidecar present after persist");
        assert!(!stored.is_empty());

        // The WAL provenance (the commit point) replays back, bound to the same
        // anchor + the imported seed epoch.
        let entries = wal.read_all().expect("read_all");
        let bb = BTreeMap::new();
        let out = replay_from_anchor(&anchor_fp, &entries, &bb).expect("replay");
        let prov = out.provenance.expect("provenance recovered");
        assert_eq!(prov.anchor_fp, anchor_fp);
        assert_eq!(prov.epoch_no, EPOCH);
    }
}
