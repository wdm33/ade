//! MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d — the NATIVE `--mode node` FirstRun
//! route against the REAL preprod snapshot. Exercises the production native
//! chain (`native_firstrun::native_first_run_bootstrap`) end-to-end over the
//! real V2 LedgerDB `state` + the Stage-2 `tables` + the real Cardano Shelley
//! genesis + a point-coherent manifest: the native bootstrap must invoke the
//! S1a/S1b/S1c chain, persist the durable artifacts ATOMICALLY, and leave the
//! anchor lineage discoverable via `load_recovered_anchor_point`. The persisted
//! anchor's point == the snapshot point, and NO cardano-cli / JSON seed
//! participates. RED: reads the restored snapshot; SKIPS when absent.
//!
//! The `.mithril-scratch` snapshot is epoch 296, tip slot 126_400_064, preprod
//! (genesis hash 162d29c4..., magic 1). No real Mithril manifest exists for
//! THIS exact snapshot point (the committed docs/evidence/mithril-manifest.json
//! certifies a DIFFERENT point, slot 125928174 / epoch 295), so — as the S1d
//! contract sanctions when no real manifest is available — a point-COHERENT
//! manifest matching the snapshot point/epoch/network is CONSTRUCTED for the
//! run (the manifest's block hash is the snapshot-point hash bound through; the
//! S1a decoder does not re-derive it from the state file).

use ade_core::consensus::ledger_view::LedgerView;
use ade_ledger::wal::{WalEntry, WalError, WalStore};
use ade_node::native_firstrun::native_first_run_bootstrap;
use ade_runtime::chaindb::{InMemoryChainDb, SnapshotStore};
use ade_runtime::consensus_inputs::LiveConsensusInputsCanonical;
use ade_runtime::recovered_anchor::load_recovered_anchor_point;
use ade_types::SlotNo;

const LEDGER_DIR: &str = "/home/ts/Code/rust/ade/.mithril-scratch/restore-ancillary/db/ledger";
const PREPROD_MAGIC: u32 = 1;
// The reviewed preprod Shelley-genesis hash (bootstrap_export::resolve_network_profile).
const PREPROD_GENESIS_HASH_HEX: &str =
    "162d29c4e1cf6b8a84f2d692e67a3ac6bc7851bc3e6e4afe64d15778bed8bd86";
// A deterministic placeholder for the snapshot-point block hash. The S1a
// decoder binds it THROUGH (it does not decode the tip hash from the state
// file), so the constructed manifest is point-coherent by construction.
const SNAPSHOT_BLOCK_HASH_HEX: &str =
    "abababababababababababababababababababababababababababababababab";

/// Minimal append-order in-memory `WalStore` double.
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

/// Locate the highest-slot ledger snapshot folder under the restore dir.
fn snapshot_slot_dir() -> Option<(u64, std::path::PathBuf)> {
    let dir = std::env::var("ADE_MITHRIL_LEDGER_DIR").unwrap_or_else(|_| LEDGER_DIR.to_string());
    let mut slots: Vec<u64> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .flatten()
            .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse::<u64>().ok()))
            .collect(),
        Err(_) => {
            eprintln!("SKIP: no Mithril restore at {dir}");
            return None;
        }
    };
    if slots.is_empty() {
        eprintln!("SKIP: no ledger snapshot in {dir}");
        return None;
    }
    slots.sort();
    let slot = *slots.last().unwrap();
    Some((slot, std::path::PathBuf::from(format!("{dir}/{slot}"))))
}

/// The shelley genesis bytes (the native route's required metadata file).
fn shelley_genesis_bytes() -> Option<Vec<u8>> {
    let path = std::env::var("ADE_SHELLEY_GENESIS").unwrap_or_else(|_| {
        "/home/ts/Code/rust/ade/.cardano-node-preprod/config/shelley-genesis.json".to_string()
    });
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(_) => {
            eprintln!("SKIP: no shelley genesis at {path}");
            None
        }
    }
}

/// Construct a point-coherent Mithril manifest for the snapshot tip slot.
fn coherent_manifest(slot: u64) -> String {
    format!(
        r#"{{
            "artifact_type": "cardano-database-snapshot",
            "certificate_hash_hex": "6666666666666666666666666666666666666666666666666666666666666666",
            "network_magic": {PREPROD_MAGIC},
            "genesis_hash_hex": "{PREPROD_GENESIS_HASH_HEX}",
            "certified_point": {{
                "slot": {slot},
                "block_hash_hex": "{SNAPSHOT_BLOCK_HASH_HEX}"
            }},
            "immutable_range": {{ "lo": 0, "hi": 5829 }},
            "source_mithril_client_version": "constructed-point-coherent-manifest (S1d test)",
            "source_command": "constructed-point-coherent (NO real manifest for this snapshot point)"
        }}"#
    )
}

/// Build a leadership view from the assembled canonical inputs (faithful zip;
/// the cold-start composition never consumes it).
fn view_builder(c: &LiveConsensusInputsCanonical) -> Box<dyn LedgerView> {
    let mut pools: std::collections::BTreeMap<ade_types::Hash28, ade_ledger::consensus_view::PoolEntry> =
        std::collections::BTreeMap::new();
    let mut total = 0u64;
    for (k, v) in &c.pool_distribution {
        total = total.saturating_add(v.active_stake);
        let vrf = c
            .pool_vrf_keyhashes
            .get(k)
            .cloned()
            .unwrap_or(ade_types::Hash32([0u8; 32]));
        pools.insert(
            k.clone(),
            ade_ledger::consensus_view::PoolEntry {
                active_stake: v.active_stake,
                vrf_keyhash: vrf,
            },
        );
    }
    Box::new(ade_ledger::consensus_view::PoolDistrView::new(
        c.epoch_no,
        total,
        c.active_slots_coeff,
        pools,
    ))
}

#[test]
fn native_first_run_real_snapshot_invokes_bootstrap_and_persists() {
    let (slot, dir) = match snapshot_slot_dir() {
        Some(x) => x,
        None => return,
    };
    let state_cbor = match std::fs::read(dir.join("state")) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("SKIP: no state file in {}", dir.display());
            return;
        }
    };
    let tables_bytes = match std::fs::read(dir.join("tables")) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("SKIP: no tables file in {}", dir.display());
            return;
        }
    };
    let shelley_genesis = match shelley_genesis_bytes() {
        Some(d) => d,
        None => return,
    };
    let manifest = coherent_manifest(slot);
    eprintln!(
        "S1d native FirstRun over the REAL snapshot: slot {slot}, state {} bytes, tables {} bytes",
        state_cbor.len(),
        tables_bytes.len()
    );

    let db = InMemoryChainDb::new();
    let mut wal = VecWal {
        entries: Vec::new(),
    };
    let snapshot_dir = tempfile::tempdir().expect("snapshot dir");

    let out = native_first_run_bootstrap(
        manifest.as_bytes(),
        &state_cbor,
        &tables_bytes,
        &shelley_genesis,
        snapshot_dir.path(),
        &db,
        &db,
        &mut wal,
        view_builder,
    )
    .expect("native FirstRun must invoke the native bootstrap and persist over real bytes");

    // S2 (DC-MITHRIL-08): the preprod snapshot's cert-state carries delegations, so the native
    // FirstRun built the live reduced checkpoint INLINE — a Mithril-started node is
    // boundary-usable (ECA derives the next-epoch view from it), not inert at the wall.
    let cp_path = snapshot_dir.path().join("reduced-checkpoint.redb");
    assert!(cp_path.exists(), "native FirstRun must build the reduced checkpoint inline");
    ade_runtime::chaindb::ReducedUtxoCheckpoint::open(&cp_path)
        .expect("the inline-built reduced checkpoint must open");

    // The native bootstrap produced a MithrilBootstrapOutput with a Mithril
    // anchor whose seed_point IS the snapshot point.
    assert!(out.tip.is_none(), "cold-start has no tip");
    assert!(matches!(
        out.anchor.seed_provenance,
        ade_ledger::bootstrap_anchor::SeedProvenance::Mithril { .. }
    ));
    assert_eq!(
        out.anchor.seed_point.slot,
        SlotNo(slot),
        "the persisted anchor point == the snapshot point"
    );

    // The anchor lineage is discoverable via load_recovered_anchor_point
    // (the durable artifacts persisted: sidecar + recovered-anchor point +
    // the WAL provenance commit).
    let anchor_fp = out.anchor.initial_ledger_fingerprint.clone();
    let recovered =
        load_recovered_anchor_point(&db, &anchor_fp).expect("recovered anchor point loads");
    assert_eq!(recovered.slot, SlotNo(slot), "recovered point == snapshot slot");

    // The seed-epoch sidecar persisted, anchor-fp-keyed.
    assert!(SnapshotStore::get_seed_epoch_consensus_inputs(&db, &anchor_fp)
        .expect("get sidecar")
        .is_some());

    // The WAL recorded the provenance commit (the sole discovery gate).
    assert_eq!(
        wal.read_all().expect("read_all").len(),
        1,
        "exactly one seed-epoch provenance entry committed"
    );
    eprintln!(
        "S1d native FirstRun PERSISTED: anchor_fp={:?}, seed_point_slot={}",
        anchor_fp, slot
    );
}

#[test]
fn native_first_run_real_snapshot_wrong_network_is_terminal() {
    // Same real state/tables/genesis, but a manifest whose magic (mainnet)
    // disagrees with the state's derived network id (preprod testnet) => a
    // terminal coherence failure before any persist (nothing discoverable).
    let (slot, dir) = match snapshot_slot_dir() {
        Some(x) => x,
        None => return,
    };
    let state_cbor = match std::fs::read(dir.join("state")) {
        Ok(d) => d,
        Err(_) => return,
    };
    let tables_bytes = match std::fs::read(dir.join("tables")) {
        Ok(d) => d,
        Err(_) => return,
    };
    let shelley_genesis = match shelley_genesis_bytes() {
        Some(d) => d,
        None => return,
    };
    // Mainnet magic 764824073 -> network_id 1; the state derives testnet (0).
    let mainnet_manifest = format!(
        r#"{{
            "artifact_type": "cardano-database-snapshot",
            "certificate_hash_hex": "6666666666666666666666666666666666666666666666666666666666666666",
            "network_magic": 764824073,
            "genesis_hash_hex": "{PREPROD_GENESIS_HASH_HEX}",
            "certified_point": {{ "slot": {slot}, "block_hash_hex": "{SNAPSHOT_BLOCK_HASH_HEX}" }},
            "immutable_range": {{ "lo": 0, "hi": 5829 }},
            "source_mithril_client_version": "wrong-network (S1d test)",
            "source_command": "wrong-network"
        }}"#
    );

    let db = InMemoryChainDb::new();
    let mut wal = VecWal {
        entries: Vec::new(),
    };
    let snapshot_dir = tempfile::tempdir().expect("snapshot dir");
    let r = native_first_run_bootstrap(
        mainnet_manifest.as_bytes(),
        &state_cbor,
        &tables_bytes,
        &shelley_genesis,
        snapshot_dir.path(),
        &db,
        &db,
        &mut wal,
        view_builder,
    );
    assert!(
        r.is_err(),
        "a wrong-network manifest must be terminal (network-id coherence)"
    );
    // Nothing committed -> no discoverable anchor lineage.
    assert!(
        wal.read_all().expect("read_all").is_empty(),
        "no WAL provenance on a fail-closed coherence path (no bootable partial state)"
    );
}
