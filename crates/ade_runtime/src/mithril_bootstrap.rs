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
use ade_types::{Hash32, SlotNo};

use crate::bootstrap::{bootstrap_initial_state, BootstrapError, BootstrapInputs};
use crate::bootstrap_anchor::{mint, MintInputs};
use crate::chaindb::{ChainDb, ChainTip, SnapshotStore};
use crate::mithril_import::{import_mithril_manifest_from_bytes, MithrilManifestError};
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
    chaindb: &D,
    snapshot_store: &S,
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

    let (ledger, chain_dep, tip) = bootstrap_initial_state(BootstrapInputs {
        chaindb,
        snapshot_store,
        era_schedule,
        ledger_view,
        genesis_initial: Some((seed_ledger, seed_chain_dep)),
    })
    .map_err(MithrilBootstrapError::Bootstrap)?;

    Ok(MithrilBootstrapOutput {
        ledger,
        chain_dep,
        tip,
        anchor,
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
    use ade_types::{CardanoEra, EpochNo, Hash28};

    use crate::chaindb::InMemoryChainDb;

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
        let asc = ActiveSlotsCoeff { numer: 5, denom: 100 };
        let pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        PoolDistrView::new(EPOCH_576, 1, asc, pools)
    }

    fn seed_state() -> (LedgerState, PraosChainDepState) {
        let ledger = LedgerState::new(CardanoEra::Conway);
        let chain_dep = PraosChainDepState::genesis(Nonce(Hash32([0xCD; 32])));
        (ledger, chain_dep)
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
        let sched = schedule();
        let view = empty_view();
        let (ledger, chain_dep) = seed_state();

        let mut inputs = matching_seed_point_inputs();
        inputs.seed_slot = SlotNo(99_999_999);
        inputs.seed_block_hash = Hash32([0xAB; 32]);

        let err = bootstrap_from_mithril_snapshot(
            &inputs,
            ledger,
            chain_dep,
            MANIFEST.as_bytes(),
            &db,
            &db,
            &sched,
            &view,
        )
        .expect_err("mismatched seed_point must fail closed before storage init");
        assert!(matches!(err, MithrilBootstrapError::Binding(_)));
        assert!(
            db.list_snapshot_slots().expect("list").is_empty(),
            "storage must not initialize before a verified binding"
        );
    }

    #[test]
    fn mithril_bootstrap_fails_closed_on_seed_point_mismatch() {
        // Operator seed-point P ≠ manifest certified_point Q →
        // Binding(CertifiedPointMismatch); no bootstrap_initial_state
        // side effect (store stays empty). Load-bearing: if seed_point
        // were sourced from the manifest, P would equal Q and this
        // would never fail.
        let db = InMemoryChainDb::new();
        let sched = schedule();
        let view = empty_view();
        let (ledger, chain_dep) = seed_state();

        let mut inputs = matching_seed_point_inputs();
        inputs.seed_slot = SlotNo(99_999_999);
        inputs.seed_block_hash = Hash32([0xAB; 32]);
        // The operator's independent point is genuinely different from
        // what the manifest attests.
        assert_ne!(inputs.seed_slot, SlotNo(MANIFEST_SLOT));

        let err = bootstrap_from_mithril_snapshot(
            &inputs,
            ledger,
            chain_dep,
            MANIFEST.as_bytes(),
            &db,
            &db,
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
    }

    #[test]
    fn mithril_bootstrap_succeeds_when_seed_point_matches() {
        // Operator seed-point == manifest certified_point → bootstrap
        // proceeds; anchor records SeedProvenance::Mithril.
        let db = InMemoryChainDb::new();
        let sched = schedule();
        let view = empty_view();
        let (ledger, chain_dep) = seed_state();
        let inputs = matching_seed_point_inputs();

        let out = bootstrap_from_mithril_snapshot(
            &inputs,
            ledger,
            chain_dep,
            MANIFEST.as_bytes(),
            &db,
            &db,
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
}
