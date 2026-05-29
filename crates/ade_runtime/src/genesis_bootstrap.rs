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
use ade_ledger::genesis_source::{
    genesis_initial_state, ConwayGenesisConfig, GenesisSourceError,
};
use ade_ledger::state::LedgerState;
use ade_types::{Hash32, SlotNo};

use crate::bootstrap::{bootstrap_initial_state, BootstrapError, BootstrapInputs};
use crate::bootstrap_anchor::{mint, MintInputs};
use crate::chaindb::{ChainDb, ChainTip, SnapshotStore};
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
    chaindb: &D,
    snapshot_store: &S,
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

    let (ledger, chain_dep, tip) = bootstrap_initial_state(BootstrapInputs {
        chaindb,
        snapshot_store,
        era_schedule,
        ledger_view,
        genesis_initial: Some((genesis_ledger, genesis_chain_dep)),
    })
    .map_err(GenesisBootstrapError::Bootstrap)?;

    Ok(GenesisBootstrapOutput {
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
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::genesis_source::GenesisInitialFund;
    use ade_ledger::utxo::TxOut;
    use ade_types::tx::{Coin, TxIn};
    use ade_types::{CardanoEra, EpochNo, Hash28, SlotNo};

    use crate::chaindb::InMemoryChainDb;

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
        let asc = ActiveSlotsCoeff { numer: 5, denom: 100 };
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

    #[test]
    fn conway_genesis_bootstrap_through_single_authority() {
        let db = InMemoryChainDb::new();
        let sched = schedule();
        let view = empty_view();
        let cfg = conway_config();

        let out = bootstrap_from_conway_genesis(
            &cfg,
            /*network_magic=*/ 1,
            /*genesis_hash=*/ Hash32([0x11; 32]),
            /*genesis_artifact_hash=*/ Hash32([0x22; 32]),
            &db,
            &db,
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
        let sched = schedule();
        let view = empty_view();
        let mut cfg = conway_config();
        cfg.era = CardanoEra::Babbage;

        let err = bootstrap_from_conway_genesis(
            &cfg,
            1,
            Hash32([0x11; 32]),
            Hash32([0x22; 32]),
            &db,
            &db,
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
    }
}
