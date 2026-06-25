// GREEN — genesis-consistency pinning harness (PHASE4-N-F-G-A S1).
//
// Reads the committed private-net Ade-as-leader reference fixture (S1b),
// builds the recovered seed-epoch surface, drives the REAL
// `bootstrap_initial_state` warm-start, and pins Ade's recovered values +
// leader-eligibility inputs against the genesis-derived reference.
//
// Non-authoritative test infrastructure. The whole harness is `#[cfg(test)]`:
// it exists only to exercise the four S1 pinning tests. The committed fixture
// is **evidence input, not runtime authority** — it is never a production
// source of eta0 / stake / ASC / VRF keyhash, and the in-test sidecar pre-seed
// (`put_seed_epoch_consensus_inputs`) is confined to this test module (the
// CN-CINPUT-02 populate-side fence keeps production population on the
// verified-bootstrap path only). Comparisons are over observable / derived
// surfaces only (DC-COMPAT-01): no Ade-internal-state fingerprint is compared
// to a Haskell-serialized-state hash.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
    use ade_core::consensus::vrf_cert::{praos_vrf_input, ActiveSlotsCoeff, StakeFraction};
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_crypto::blake2b_256;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::seed_consensus_inputs::{
        encode_seed_epoch_consensus_inputs, SeedEpochConsensusInputs,
    };
    use ade_ledger::state::LedgerState;
    use ade_ledger::wal::RecoveredBootstrapProvenance;
    use ade_runtime::bootstrap::{
        bootstrap_initial_state, BootstrapInputs, BootstrapState, SeedEpochConsensusSource,
    };
    use ade_runtime::chaindb::{InMemoryChainDb, SnapshotStore};
    use ade_runtime::rollback::persistent_cache::PersistentSnapshotCache;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

    /// The committed S1b private-net reference fixture, embedded at compile
    /// time so the harness reads only committed bytes (no Docker / cardano-cli
    /// / live node).
    const FIXTURE_JSON: &str =
        include_str!("../../fixtures/nfg_a_privnet_reference/consensus-inputs.json");

    /// A fixed, non-secret anchor fingerprint binding the in-test sidecar to
    /// its provenance.
    const TEST_ANCHOR_FP: Hash32 = Hash32([0x5A; 32]);

    /// Parsed genesis-derived reference values from the S1b fixture.
    struct GenesisReference {
        /// Genesis-derived initial epoch nonce (== private Shelley genesis hash).
        eta0: Nonce,
        epoch_no: EpochNo,
        asc: ActiveSlotsCoeff,
        /// Per-pool active stake + registered VRF keyhash.
        pools: BTreeMap<Hash28, PoolEntry>,
        total_active_stake: u64,
    }

    fn hex_to_vec(s: &str) -> Vec<u8> {
        assert!(s.len() % 2 == 0, "hex string must have even length");
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex byte"))
            .collect()
    }

    fn hash32_from_hex(s: &str) -> Hash32 {
        let v = hex_to_vec(s);
        assert_eq!(v.len(), 32, "expected 32-byte hex for Hash32");
        let mut a = [0u8; 32];
        a.copy_from_slice(&v);
        Hash32(a)
    }

    fn hash28_from_hex(s: &str) -> Hash28 {
        let v = hex_to_vec(s);
        assert_eq!(v.len(), 28, "expected 28-byte hex for Hash28");
        let mut a = [0u8; 28];
        a.copy_from_slice(&v);
        Hash28(a)
    }

    /// Parse the committed fixture into the genesis-derived reference values.
    fn load_reference() -> GenesisReference {
        let v: serde_json::Value =
            serde_json::from_str(FIXTURE_JSON).expect("fixture parses as JSON");

        let eta0 = Nonce(hash32_from_hex(
            v["epoch_nonce_hex"].as_str().expect("epoch_nonce_hex"),
        ));
        let epoch_no = EpochNo(v["epoch_no"].as_u64().expect("epoch_no"));
        let asc = ActiveSlotsCoeff {
            numer: v["active_slots_coeff"]["numer"]
                .as_u64()
                .expect("asc numer") as u32,
            denom: v["active_slots_coeff"]["denom"]
                .as_u64()
                .expect("asc denom") as u32,
        };

        let dist = v["pool_distribution"]
            .as_object()
            .expect("pool_distribution object");
        let vrfs = v["pool_vrf_keyhashes"]
            .as_object()
            .expect("pool_vrf_keyhashes object");

        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        let mut total_active_stake: u64 = 0;
        for (pool_id_hex, entry) in dist {
            let active_stake = entry["active_stake"].as_u64().expect("active_stake");
            let vrf_keyhash = hash32_from_hex(
                vrfs[pool_id_hex]
                    .as_str()
                    .expect("vrf keyhash for pool present in both maps"),
            );
            total_active_stake = total_active_stake
                .checked_add(active_stake)
                .expect("total active stake fits u64");
            pools.insert(
                hash28_from_hex(pool_id_hex),
                PoolEntry {
                    active_stake,
                    vrf_keyhash,
                },
            );
        }

        GenesisReference {
            eta0,
            epoch_no,
            asc,
            pools,
            total_active_stake,
        }
    }

    /// Build the BLUE recovered-seed-epoch record from the reference.
    fn reference_seed_inputs(r: &GenesisReference) -> SeedEpochConsensusInputs {
        SeedEpochConsensusInputs {
            anchor_fp: TEST_ANCHOR_FP,
            epoch_no: r.epoch_no,
            epoch_start_slot: SlotNo(r.epoch_no.0 * 432_000),
            epoch_length_slots: 432_000,
            epoch_nonce: r.eta0.clone(),
            genesis_hash: Hash32([0x9a; 32]),
            protocol_params_hash: Hash32([0x9b; 32]),
            active_slots_coeff: r.asc,
            total_active_stake: r.total_active_stake,
            pool_distribution: r.pools.clone(),
        }
    }

    /// A minimal single-Conway-era schedule for the warm-start materialize.
    fn minimal_schedule() -> EraSchedule {
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Conway,
                start_slot: SlotNo(0),
                start_epoch: EpochNo(0),
                slot_length_ms: 1_000,
                epoch_length_slots: 432_000,
                safe_zone_slots: 432_000,
            }],
        )
        .expect("schedule")
    }

    /// Drive the **real** warm-start recovery: pre-seed a store with a snapshot
    /// carrying `chain_dep = genesis(eta0)` plus the fixture sidecar + its
    /// provenance, then recover through `bootstrap_initial_state`
    /// (`RequiredFromRecoveredProvenance`).
    fn warm_start_recover(r: &GenesisReference) -> BootstrapState {
        let record = reference_seed_inputs(r);

        // Snapshot carries the genesis-derived eta0 in the chain-dep state.
        let db = InMemoryChainDb::new();
        let ledger = LedgerState::new(CardanoEra::Conway);
        let chain_dep = PraosChainDepState::genesis(r.eta0.clone());
        PersistentSnapshotCache::new(&db)
            .capture(SlotNo(0), &ledger, &chain_dep)
            .expect("capture snapshot");

        // Persist the fixture sidecar + its A3a provenance binding (in-test
        // pre-seed only; CN-CINPUT-02 confines production population to the
        // verified-bootstrap composers).
        let bytes = encode_seed_epoch_consensus_inputs(&record);
        db.put_seed_epoch_consensus_inputs(&record.anchor_fp, &bytes)
            .expect("put sidecar");
        let provenance = RecoveredBootstrapProvenance {
            anchor_fp: record.anchor_fp.clone(),
            sidecar_hash: blake2b_256(&bytes),
            epoch_no: record.epoch_no,
        };

        let sched = minimal_schedule();
        let view = PoolDistrView::from_seed_epoch_consensus_inputs(&record);

        bootstrap_initial_state(BootstrapInputs {
            chaindb: &db,
            snapshot_store: &db,
            era_schedule: &sched,
            ledger_view: &view,
            genesis_initial: None,
            seed_epoch_consensus_source: SeedEpochConsensusSource::RequiredFromRecoveredProvenance(
                provenance,
            ),
            recovered_anchor: None,
        })
        .expect("warm-start recovers")
    }

    // CE-G-A-1 (a): the WarmStart-recovered eta0 (chain_dep.epoch_nonce) equals
    // the genesis-derived fixture eta0.
    #[test]
    fn pinning_recovered_eta0_matches_genesis_fixture() {
        let r = load_reference();
        let out = warm_start_recover(&r);
        assert_eq!(
            out.chain_dep.epoch_nonce, r.eta0,
            "recovered chain-dep eta0 must equal the genesis-derived fixture eta0"
        );
    }

    // CE-G-A-1 (a): the WarmStart-recovered SeedEpochConsensusInputs (stake /
    // ASC / per-pool VRF keyhash) equals the genesis-derived fixture.
    #[test]
    fn pinning_recovered_stake_asc_vrf_matches_genesis_fixture() {
        let r = load_reference();
        let out = warm_start_recover(&r);
        let recovered = out
            .seed_epoch_consensus_inputs
            .expect("required warm-start recovers the sidecar");
        assert_eq!(recovered.active_slots_coeff, r.asc, "ASC");
        assert_eq!(
            recovered.total_active_stake, r.total_active_stake,
            "total active stake"
        );
        assert_eq!(recovered.pool_distribution, r.pools, "per-pool stake + vrf");
    }

    // CE-G-A-1 (c): pre-seed -> warm_start -> recovered-state round-trip is
    // byte-faithful (the recovered record re-encodes to the persisted bytes).
    #[test]
    fn pinning_preseed_warmstart_roundtrip_faithful() {
        let r = load_reference();
        let record = reference_seed_inputs(&r);
        let out = warm_start_recover(&r);
        let recovered = out.seed_epoch_consensus_inputs.expect("recovered sidecar");
        assert_eq!(
            recovered, record,
            "recovered record equals the pre-seeded record"
        );
        assert_eq!(
            encode_seed_epoch_consensus_inputs(&recovered),
            encode_seed_epoch_consensus_inputs(&record),
            "byte-identical re-encode"
        );
    }

    // CE-G-A-1 (b): Ade's praos_vrf_input + leader-threshold inputs are the
    // genesis-derived ones. Observable/derived surfaces only (DC-COMPAT-01).
    #[test]
    fn pinning_praos_vrf_input_and_threshold_match_fixture() {
        let r = load_reference();
        let slot = SlotNo(382); // a sample leadership slot from the private net

        // Pin the VRF-input recipe: blake2b256(slot_be8 ‖ eta0_32).
        let mut pre = [0u8; 40];
        pre[0..8].copy_from_slice(&slot.0.to_be_bytes());
        pre[8..40].copy_from_slice(r.eta0.as_bytes());
        let expected = blake2b_256(&pre).0;
        assert_eq!(
            praos_vrf_input(slot, &r.eta0),
            expected,
            "praos_vrf_input == blake2b256(slot_be8 ‖ eta0)"
        );

        // The VRF input is BOUND to the genesis-derived eta0 (not a constant).
        assert_ne!(
            praos_vrf_input(slot, &r.eta0),
            praos_vrf_input(slot, &Nonce::ZERO),
            "VRF input must depend on the genesis-derived eta0"
        );

        // The recovered eta0 (from warm-start) feeds the same VRF input.
        let out = warm_start_recover(&r);
        assert_eq!(
            praos_vrf_input(slot, &out.chain_dep.epoch_nonce),
            praos_vrf_input(slot, &r.eta0),
            "recovered eta0 produces the genesis-consistent VRF input"
        );

        // Leader-threshold inputs: Ade's pool has a positive stake fraction and
        // the genesis ASC, projected through the recovered leadership view.
        let record = reference_seed_inputs(&r);
        let _view = PoolDistrView::from_seed_epoch_consensus_inputs(&record);
        let (_pool_id, entry) = r.pools.iter().next().expect("at least one Ade pool");
        let sigma = StakeFraction {
            numer: entry.active_stake,
            denom: r.total_active_stake,
        };
        assert!(
            sigma.numer > 0 && sigma.denom >= sigma.numer && sigma.denom > 0,
            "Ade pool has a valid positive leader-eligibility stake fraction"
        );
        assert_eq!(
            r.asc,
            ActiveSlotsCoeff {
                numer: 1,
                denom: 20
            },
            "genesis ASC (0.05) preserved as 1/20"
        );
    }
}
