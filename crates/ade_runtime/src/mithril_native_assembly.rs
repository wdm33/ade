// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED native Mithril authority-transition assembly + persist
//! (MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1b).
//!
//! Assembles the COMPLETE authoritative seed — `LedgerState` +
//! `PraosChainDepState` + a NATIVE `LiveConsensusInputsCanonical` — from ONLY
//! the verified manifest report (the Stage-0 binding), the S1a
//! `NativeSnapshotNonUtxoState` (the decoded non-UTxO authority), the Stage-2
//! `tables` UTxO (`UTxOState`), and genesis constants. There is **no
//! cardano-cli, no JSON consensus-input bundle, no operator seed, and no
//! convenience fallback** on this path — the verified snapshot IS the source
//! (DC-MITHRIL-03).
//!
//! [`assemble_native_mithril_seed`] is the pure assembly: it maps each field
//! from its single declared source and enforces POINT COHERENCE as a TERMINAL
//! gate (S1a point == manifest certified point; S1a epoch == manifest epoch;
//! S1a network id == manifest-magic-derived id; the assembled anchor's
//! `seed_point` == the manifest point). Any mismatch / missing input is a
//! structured terminal error — NO authority is assembled and NOTHING is
//! persisted.
//!
//! [`bootstrap_from_native_mithril_snapshot`] is the native entry: it runs the
//! assembly, then routes the assembled seed through the SAME single closed
//! Mithril composition [`crate::mithril_bootstrap::bootstrap_from_mithril_snapshot`]
//! (which calls the single `bootstrap_initial_state` authority and persists the
//! seed-epoch sidecar + recovered-anchor point + WAL commit). The WAL append is
//! the SOLE point at which the anchor lineage becomes discoverable, so an
//! interrupted import (any write failing before the WAL commit) leaves NO
//! bootable partial authority state — the authority is visible ONLY after the
//! atomic commit.
//!
//! The CLI / JSON seed importers (`import_cardano_cli_json_utxo`,
//! `import_live_consensus_inputs`) stay as RED diagnostic / oracle tooling;
//! they do NOT participate on this native bootstrap path.

use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::praos_state::{Nonce as CoreNonce, PraosChainDepState};
use ade_crypto::blake2b::blake2b_256;
use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::consensus_input_extract::PraosNonces;
use ade_ledger::ledgerdb_state::NativeSnapshotNonUtxoState;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::snapshot::gov_state::encode_pparams;
use ade_ledger::state::{EpochState, LedgerState};
use ade_ledger::utxo::UTxOState;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

use crate::consensus_inputs::{
    canonical_from_raw, LiveConsensusInputsCanonical, LiveConsensusInputsRaw,
    PoolEntry as ConsensusPoolEntry,
};
use crate::mithril_bootstrap::{
    bootstrap_from_mithril_snapshot, MithrilBootstrapError, MithrilBootstrapOutput,
    MithrilSeedPointInputs,
};
use crate::seed_import::UtxoFingerprint;

/// The minimal subset of the verified manifest the native assembly binds the
/// snapshot to. It is the manifest's attested side (`network_magic`,
/// `genesis_hash`, the certified `point`) carried whole — NEVER the operator
/// JSON seed or a cardano-cli bundle. The caller obtains it from the RED
/// `import_mithril_manifest_from_bytes` report (the same manifest bytes are
/// fed to `bootstrap_from_mithril_snapshot`, which re-parses them and runs
/// `verify_mithril_binding` itself — this struct does not replace that check,
/// it is the assembly's coherence anchor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedManifestBinding {
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub certified_point: SeedPoint,
    /// The Mithril immutable-file range (provenance only; carried into the
    /// anchor's `SeedProvenance::Mithril` by `bootstrap_from_mithril_snapshot`).
    pub immutable_range: (u64, u64),
}

/// Genesis constants the native assembly consumes — the immutable
/// chain-configuration facts that are NOT in the LedgerDB `state` file and are
/// NOT operator-supplied seed material. Sourced from the Cardano genesis
/// (Shelley `maxLovelaceSupply`, the active-slots coefficient), never from a
/// cardano-cli query or the operator bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeGenesisConstants {
    pub max_lovelace_supply: u64,
    pub active_slots_coeff: ade_core::consensus::vrf_cert::ActiveSlotsCoeff,
}

/// The assembled native seed — exactly the four seed inputs the single closed
/// Mithril composition [`bootstrap_from_mithril_snapshot`] consumes, every one
/// derived from manifest / S1a / Stage-2 / genesis. Returned by
/// [`assemble_native_mithril_seed`] only after POINT COHERENCE has passed, so a
/// `NativeMithrilSeed` value is by construction coherent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeMithrilSeed {
    pub ledger: LedgerState,
    pub chain_dep: PraosChainDepState,
    pub consensus_inputs: LiveConsensusInputsCanonical,
    pub seed_point_inputs: MithrilSeedPointInputs,
}

/// Closed terminal error sum for the native assembly. Every variant is a
/// fail-closed halt BEFORE any authority is assembled or persisted — there is
/// no partial state and no fallback (DC-MITHRIL-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MithrilNativeAssemblyError {
    /// S1a's bound point (slot or hash) disagrees with the manifest certified
    /// point. The snapshot and the certificate attest different chain points.
    PointMismatch {
        s1a_slot: u64,
        manifest_slot: u64,
    },
    /// S1a's bound point hash disagrees with the manifest certified point hash
    /// (same slot, different block).
    PointHashMismatch,
    /// S1a's NES epoch disagrees with the manifest-certified epoch (S1a already
    /// cross-checks this against `manifest_epoch`; re-asserted here as the
    /// coherence gate before persist).
    EpochMismatch {
        s1a_epoch: u64,
        manifest_epoch: u64,
    },
    /// S1a's derived network id disagrees with the id derived from the manifest
    /// network magic (mainnet magic -> 1, any other -> 0).
    NetworkMismatch {
        s1a_network_id: u8,
        manifest_network_id: u8,
    },
    /// S1a did not decode a Conway-era snapshot. The native bootstrap assembles
    /// a Conway `LedgerState` only.
    NonConwayEra {
        decoded: CardanoEra,
    },
    /// The S1a epoch falls outside every era in the supplied schedule, so the
    /// epoch window (`epoch_start_slot` / `epoch_end_slot`) cannot be derived.
    EpochWindowUnresolved {
        epoch: u64,
    },
}

/// The manifest-magic-derived internal network id. Mirrors S1a's
/// `network_id_from_magic` (mainnet magic -> 1, any other (testnet) -> 0). Kept
/// in lock-step so the coherence check compares the SAME derivation S1a bound.
const MAINNET_NETWORK_MAGIC: u32 = 764_824_073;
fn network_id_from_magic(magic: u32) -> u8 {
    if magic == MAINNET_NETWORK_MAGIC {
        1
    } else {
        0
    }
}

/// Map the five S1a record-order Praos nonces onto the
/// `PraosChainDepState` nonce slots. Record order is
/// `[evolving, candidate, epoch(eta0), lab, last_epoch_block]`; the
/// `last_epoch_block` slot is the previous-epoch nonce. Op-cert counters are
/// empty and `last_*` are `None` (cold start — no header applied yet).
fn chain_dep_from_nonces(n: &PraosNonces) -> PraosChainDepState {
    let to_core = |x: &ade_ledger::consensus_input_extract::Nonce| CoreNonce(Hash32(x.0));
    PraosChainDepState {
        evolving_nonce: to_core(&n.evolving),
        candidate_nonce: to_core(&n.candidate),
        epoch_nonce: to_core(&n.epoch),
        previous_epoch_nonce: to_core(&n.last_epoch_block),
        lab_nonce: to_core(&n.lab),
        last_epoch_block: None,
        // The seed→seed+1 boundary combine operand (DC-EPOCH-16):
        // eta0(seed+1) = candidate ⭒ last_epoch_block_nonce. Seeded from the
        // imported snapshot's last-epoch-block nonce so the general epoch tick
        // reproduces the ECA-5 bridge value exactly.
        last_epoch_block_nonce: Some(to_core(&n.last_epoch_block)),
        last_slot: None,
        last_block_no: None,
        op_cert_counters: ade_core::consensus::praos_state::OpCertCounterMap::new(),
    }
}

/// Derive `[epoch_start_slot, epoch_end_slot]` for `epoch` from the era
/// schedule: locate the era whose `start_epoch <= epoch` (the latest such era),
/// then `start = era.start_slot + (epoch - era.start_epoch) * epoch_length` and
/// `end = start + epoch_length - 1`. Returns `None` (terminal upstream) if
/// `epoch` precedes the first era or the arithmetic overflows.
fn derive_epoch_window(schedule: &EraSchedule, epoch: EpochNo) -> Option<(SlotNo, SlotNo)> {
    let mut chosen: Option<&ade_core::consensus::era_schedule::EraSummary> = None;
    for era in schedule.eras() {
        if era.start_epoch.0 <= epoch.0 {
            chosen = Some(era);
        }
    }
    let era = chosen?;
    let epochs_in = epoch.0.checked_sub(era.start_epoch.0)?;
    let epoch_len = u64::from(era.epoch_length_slots);
    let offset = epochs_in.checked_mul(epoch_len)?;
    let start = era.start_slot.0.checked_add(offset)?;
    let end = start.checked_add(epoch_len.checked_sub(1)?)?;
    Some((SlotNo(start), SlotNo(end)))
}

/// The native protocol-params hash: blake2b over the canonical `encode_pparams`
/// of the S1a protocol parameters. The native path has no cardano-cli JSON
/// preimage (the operator-bundle path hashes the JSON text); the snapshot IS
/// the source, so the hash commits to the decoded params directly.
fn native_protocol_params_hash(pp: &ProtocolParameters) -> Hash32 {
    blake2b_256(&encode_pparams(pp))
}

/// Build the NATIVE `LiveConsensusInputsCanonical` from S1a + manifest +
/// genesis. The provenance text fields carry a fixed NATIVE marker (never a
/// cardano-cli command or node version); `source_tip` is the manifest point;
/// `protocol_params_json` is `None` (no JSON preimage on the native path).
/// The fingerprint is computed via the SOLE canonical-form authority
/// (`canonical_from_raw`), so it is byte-identical to any other route that
/// produces the same field set.
fn native_consensus_inputs(
    s1a: &NativeSnapshotNonUtxoState,
    binding: &VerifiedManifestBinding,
    genesis: &NativeGenesisConstants,
    epoch_start_slot: SlotNo,
    epoch_end_slot: SlotNo,
) -> LiveConsensusInputsCanonical {
    let mut pool_distribution: BTreeMap<Hash28, ConsensusPoolEntry> = BTreeMap::new();
    let mut pool_vrf_keyhashes: BTreeMap<Hash28, Hash32> = BTreeMap::new();
    for (pool_id, (stake, vrf)) in &s1a.pool_distr {
        pool_distribution.insert(pool_id.0.clone(), ConsensusPoolEntry { active_stake: *stake });
        pool_vrf_keyhashes.insert(pool_id.0.clone(), vrf.clone());
    }
    let raw = LiveConsensusInputsRaw {
        network_magic: binding.network_magic,
        genesis_hash: binding.genesis_hash.clone(),
        era: CardanoEra::Conway,
        epoch_no: s1a.epoch,
        epoch_start_slot,
        epoch_end_slot,
        active_slots_coeff: genesis.active_slots_coeff,
        epoch_nonce: CoreNonce(Hash32(s1a.praos_nonces.epoch.0)),
        pool_distribution,
        pool_vrf_keyhashes,
        protocol_params_hash: native_protocol_params_hash(&s1a.protocol_params),
        // Native provenance: the verified Mithril snapshot, NOT a cardano-cli
        // query. Fixed markers (no operator node version / query command) so the
        // canonical fingerprint stays deterministic without laundering operator
        // provenance onto the native path.
        source_cardano_node_version: NATIVE_SOURCE_MARKER.to_string(),
        source_query_command: NATIVE_SOURCE_MARKER.to_string(),
        source_tip_hash: binding.certified_point.block_hash.clone(),
        source_tip_slot: binding.certified_point.slot,
        protocol_params_json: None,
    };
    canonical_from_raw(raw)
}

/// Fixed native-provenance marker for the consensus-inputs source fields. The
/// snapshot is the authority; this is recorded provenance, never a cardano-cli
/// command or operator node version.
const NATIVE_SOURCE_MARKER: &str = "native-mithril-snapshot";

/// Assemble the COMPLETE native seed from manifest / S1a / Stage-2 / genesis and
/// enforce POINT COHERENCE (terminal before any persist). On success the
/// returned [`NativeMithrilSeed`] is coherent by construction; the caller feeds
/// it to [`bootstrap_from_native_mithril_snapshot`] (or, equivalently, directly
/// to [`bootstrap_from_mithril_snapshot`]) for the atomic persist.
///
/// Field sources (EXCLUSIVELY manifest / S1a / Stage-2 / genesis):
/// - `ledger.utxo_state` <- Stage-2 `utxo` (the `tables` UTxO);
/// - `ledger.cert_state` <- S1a `cert_state`;
/// - `ledger.epoch_state` <- {epoch <- S1a, slot <- manifest point, reserves +
///   treasury <- S1a, snapshots = cold-start empty, block_production <- S1a,
///   epoch_fees = 0};
/// - `ledger.protocol_params` <- S1a (incl. the `MinUtxoRule::PerByte`);
/// - `ledger.era` = Conway; `ledger.max_lovelace_supply` <- genesis;
///   `gov_state` <- the certified snapshot's imported Proposals + Committee (CONWAY-PROPOSAL-DEPOSIT-
///   EXPIRY S1); `conway_deposit_params = None`; `track_utxo = false`;
/// - `chain_dep` five nonces <- S1a; op-cert counters empty; `last_* = None`;
/// - `consensus_inputs` (native) <- manifest magic/genesis/point + S1a
///   epoch/eta0/pool_distr/params + genesis ASC + derived epoch window.
pub fn assemble_native_mithril_seed(
    s1a: &NativeSnapshotNonUtxoState,
    s1a_commitment: Hash32,
    utxo: UTxOState,
    binding: &VerifiedManifestBinding,
    genesis: &NativeGenesisConstants,
    era_schedule: &EraSchedule,
) -> Result<NativeMithrilSeed, MithrilNativeAssemblyError> {
    // --- POINT COHERENCE (terminal, before any assembly/persist). ---
    // S1a era must be Conway (the native bootstrap assembles a Conway ledger).
    if s1a.era != CardanoEra::Conway {
        return Err(MithrilNativeAssemblyError::NonConwayEra { decoded: s1a.era });
    }
    // S1a point == manifest certified point (slot then hash).
    if s1a.point.slot != binding.certified_point.slot {
        return Err(MithrilNativeAssemblyError::PointMismatch {
            s1a_slot: s1a.point.slot.0,
            manifest_slot: binding.certified_point.slot.0,
        });
    }
    if s1a.point.block_hash != binding.certified_point.block_hash {
        return Err(MithrilNativeAssemblyError::PointHashMismatch);
    }
    // S1a network id == manifest-magic-derived id.
    let manifest_network_id = network_id_from_magic(binding.network_magic);
    if s1a.network_id != manifest_network_id {
        return Err(MithrilNativeAssemblyError::NetworkMismatch {
            s1a_network_id: s1a.network_id,
            manifest_network_id,
        });
    }
    // S1a epoch == manifest epoch. S1a already cross-checks its NES epoch
    // against `manifest_epoch`; here the manifest epoch is the epoch the
    // schedule resolves for the certified slot — re-assert they agree.
    let manifest_epoch = era_schedule
        .locate(binding.certified_point.slot)
        .map(|loc| loc.epoch)
        .map_err(|_| MithrilNativeAssemblyError::EpochWindowUnresolved {
            epoch: s1a.epoch.0,
        })?;
    if s1a.epoch != manifest_epoch {
        return Err(MithrilNativeAssemblyError::EpochMismatch {
            s1a_epoch: s1a.epoch.0,
            manifest_epoch: manifest_epoch.0,
        });
    }

    // --- Epoch window (derived: era schedule + S1a epoch). ---
    let (epoch_start_slot, epoch_end_slot) = derive_epoch_window(era_schedule, s1a.epoch)
        .ok_or(MithrilNativeAssemblyError::EpochWindowUnresolved { epoch: s1a.epoch.0 })?;

    // --- Assemble the LedgerState. ---
    let epoch_state = EpochState {
        epoch: s1a.epoch,
        // slot <- manifest point (the certified tip the snapshot is bound to).
        slot: binding.certified_point.slot,
        // CE-3d: the mark/set/go stake snapshots decoded from the certified snapshot's esSnapshots —
        // the reward + leadership stake authority the EpochAccumulator seeds. NOT a cold-start empty
        // default: an empty `go` makes the accumulator compute zero member rewards for the first ~3
        // boundaries after bootstrap. Bound to the same certified point/network/era as the rest of S1a.
        snapshots: s1a.snapshots.clone(),
        reserves: s1a.reserves,
        treasury: s1a.treasury,
        block_production: pool_keyed_block_production(&s1a.block_production),
        // epoch_fees <- the certified snapshot fee pot (epoch (seed)'s fees accrued up to the snapshot
        // point). This is the nesBcur-analog for fees: the live follow adds the seed-to-end tail onto it,
        // so the FULL epoch (seed) fees rotate to nesBprev at the seed boundary and the FIRST native
        // boundary (seed+1 -> seed+2) draws the complete fee pot (NOT cold-start 0, which under-draws).
        epoch_fees: s1a.epoch_fees,
    };
    let ledger = LedgerState {
        utxo_state: utxo,
        epoch_state,
        protocol_params: s1a.protocol_params.clone(),
        era: CardanoEra::Conway,
        track_utxo: false,
        cert_state: s1a.cert_state.clone(),
        max_lovelace_supply: genesis.max_lovelace_supply,
        // CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1: seed the accumulator's gov_state from the CERTIFIED
        // snapshot's imported Proposals + Committee (NOT inferred). Identity-bound proposals (incl their
        // canonical vote maps), the active committee, and its quorum are the inputs the boundary
        // deposit-expiry negative proof reads. The remaining ConwayGovState fields are NOT imported in
        // S1 — they are sourced from pparams / cert-state in later slices (S3 sources gov_action_lifetime
        // + thresholds for NEWLY-submitted proposals; the imported proposals already carry their own
        // expires_after). The S4 evaluator FAILS CLOSED if a proposal's disposition needs any field not
        // populated here; the observed five refunds need only the committee gate (committee + quorum).
        gov_state: Some(ade_ledger::state::ConwayGovState {
            proposals: s1a.imported_gov.proposals.clone(),
            committee: s1a.imported_gov.committee.clone(),
            committee_quorum: s1a.imported_gov.committee_quorum.unwrap_or((1, 1)),
            drep_expiry: std::collections::BTreeMap::new(),
            gov_action_lifetime: 0,
            vote_delegations: std::collections::BTreeMap::new(),
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            committee_hot_keys: std::collections::BTreeMap::new(),
        }),
        conway_deposit_params: None,
    };

    // --- Assemble the PraosChainDepState (cold start). ---
    let chain_dep = chain_dep_from_nonces(&s1a.praos_nonces);

    // --- Assemble the NATIVE LiveConsensusInputsCanonical. ---
    let consensus_inputs =
        native_consensus_inputs(s1a, binding, genesis, epoch_start_slot, epoch_end_slot);

    // --- Seed-point inputs for the single closed Mithril composition. The
    //     anchor's seed_point IS the manifest certified point (the native path:
    //     the snapshot is manifest-bound, so the certified point is the anchor
    //     point; verify_mithril_binding inside bootstrap_from_mithril_snapshot
    //     re-checks anchor.seed_point == report.certified_point). The assembled
    //     anchor seed_point == the manifest point is the final coherence leg. ---
    let initial_ledger_fingerprint = ade_ledger::fingerprint::fingerprint(&ledger).combined;
    let imported_utxo_fingerprint =
        UtxoFingerprint(ade_ledger::fingerprint::fingerprint_utxo_v2(&ledger.utxo_state));
    let seed_point_inputs = MithrilSeedPointInputs {
        seed_slot: binding.certified_point.slot,
        seed_block_hash: binding.certified_point.block_hash.clone(),
        network_magic: binding.network_magic,
        genesis_hash: binding.genesis_hash.clone(),
        // The seed artifact hash binds the assembled seed to its source bytes;
        // on the native path it is the S1a commitment (the deterministic digest
        // over every decoded non-UTxO field), supplied by the caller from the
        // `decode_native_nonutxo_state` return.
        seed_artifact_hash: s1a_commitment,
        imported_utxo_fingerprint,
        initial_ledger_fingerprint,
    };

    Ok(NativeMithrilSeed {
        ledger,
        chain_dep,
        consensus_inputs,
        seed_point_inputs,
    })
}

/// Re-key S1a's `block_production` (`BTreeMap<PoolId, u64>`) into the ledger's
/// `BTreeMap<PoolId, u64>` shape. (Identity on the key type; an explicit clone
/// so the assembly never aliases the S1a borrow.)
fn pool_keyed_block_production(
    src: &BTreeMap<ade_types::tx::PoolId, u64>,
) -> BTreeMap<ade_types::tx::PoolId, u64> {
    src.clone()
}

/// The native Mithril bootstrap entry: assemble + persist ATOMICALLY (S1b).
///
/// Runs [`assemble_native_mithril_seed`] (point coherence terminal), then routes
/// the assembled seed through the SAME single closed composition
/// [`bootstrap_from_mithril_snapshot`] — the `bootstrap_initial_state` authority
/// + the seed-epoch sidecar + the recovered-anchor point + the WAL commit. The
/// CLI/JSON seed does NOT participate; the assembled seed is the sole source.
#[allow(clippy::too_many_arguments)]
pub fn bootstrap_from_native_mithril_snapshot<D, S>(
    s1a: &NativeSnapshotNonUtxoState,
    s1a_commitment: Hash32,
    utxo: UTxOState,
    binding: &VerifiedManifestBinding,
    genesis: &NativeGenesisConstants,
    manifest_bytes: &[u8],
    chaindb: &D,
    snapshot_store: &S,
    wal: &mut dyn ade_ledger::wal::WalStore,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<MithrilBootstrapOutput, NativeMithrilBootstrapError>
where
    D: crate::chaindb::ChainDb,
    S: crate::chaindb::SnapshotStore + ?Sized,
{
    let seed = assemble_native_mithril_seed(s1a, s1a_commitment, utxo, binding, genesis, era_schedule)
        .map_err(NativeMithrilBootstrapError::Assembly)?;
    bootstrap_from_mithril_snapshot(
        &seed.seed_point_inputs,
        seed.ledger,
        seed.chain_dep,
        manifest_bytes,
        &seed.consensus_inputs,
        chaindb,
        snapshot_store,
        wal,
        era_schedule,
        ledger_view,
    )
    .map_err(NativeMithrilBootstrapError::Bootstrap)
}

/// Closed error sum for the native Mithril bootstrap entry: the assembly /
/// coherence gate, or the single closed composition's verdict.
#[derive(Debug)]
pub enum NativeMithrilBootstrapError {
    /// The native assembly / point-coherence gate fail-closed (no authority
    /// assembled, nothing persisted).
    Assembly(MithrilNativeAssemblyError),
    /// The single closed Mithril composition returned an error (binding,
    /// bootstrap authority, or the atomic persist).
    Bootstrap(MithrilBootstrapError),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::BootstrapAnchorHash;
    use ade_core::consensus::EraSummary;
    use ade_ledger::consensus_input_extract::Nonce as LedgerNonce;
    use ade_ledger::delegation::CertState;
    use ade_ledger::ledgerdb_state::RewardNibbleObservation;
    use ade_ledger::pparams::MinUtxoRule;
    use ade_ledger::recovered_anchor_point::decode_recovered_anchor_point;
    use ade_ledger::utxo::UTxOState;
    use ade_ledger::wal::{replay_from_anchor, WalEntry, WalError, WalStore};
    use ade_types::tx::{Coin, PoolId};
    use ade_types::Hash28;

    #[test]
    fn seeded_chain_dep_tick_reproduces_bridge_eta0() {
        // DC-EPOCH-16 bridge equivalence (hermetic): seeding the chain-dep from
        // the imported snapshot nonces and applying the BLUE epoch tick reproduces
        // the ECA-5 bridge precompute eta0(seed+1) = blake2b(candidate || lastEpochBlock).
        use ade_core::consensus::{apply_nonce_input, NonceInput};
        let n = PraosNonces {
            evolving: LedgerNonce([0x11; 32]),
            candidate: LedgerNonce([0x22; 32]),
            epoch: LedgerNonce([0x33; 32]),
            lab: LedgerNonce([0x44; 32]),
            last_epoch_block: LedgerNonce([0x55; 32]),
        };
        let seeded = chain_dep_from_nonces(&n);
        // The combine operand is the imported last-epoch-block nonce.
        assert_eq!(
            seeded.last_epoch_block_nonce,
            Some(CoreNonce(Hash32([0x55; 32])))
        );
        let after = apply_nonce_input(
            &seeded,
            &NonceInput::EpochBoundary {
                new_epoch: EpochNo(1),
            },
        )
        .expect("epoch tick");
        // Independently compute the bridge formula blake2b(candidate || lastEpochBlock).
        let mut buf = [0u8; 64];
        buf[0..32].copy_from_slice(&[0x22; 32]);
        buf[32..64].copy_from_slice(&[0x55; 32]);
        assert_eq!(after.epoch_nonce, CoreNonce(blake2b_256(&buf)));
    }

    use crate::chaindb::{InMemoryChainDb, SnapshotStore};
    use crate::recovered_anchor::load_recovered_anchor_point;

    // The manifest's attested certified point Q. The S1a point is set to the
    // SAME point (the native path: the snapshot IS bound to the manifest), so
    // the coherence gate passes and the embedded `verify_mithril_binding`
    // (inside bootstrap_from_mithril_snapshot) also passes.
    const MANIFEST_SLOT: u64 = 23_013_663;
    const MANIFEST_BLOCK_HASH: [u8; 32] = [0x22; 32];
    const MANIFEST_GENESIS_HASH: [u8; 32] = [0x11; 32];
    const MANIFEST_NETWORK_MAGIC: u32 = 1;

    // Preprod-shaped epoch geometry (magic 1 is testnet -> network_id 0).
    const EPOCH_NO: u64 = 296;
    const EPOCH_LENGTH: u64 = 432_000;
    // start_slot of the era such that EPOCH_NO's window contains MANIFEST_SLOT.
    // era_start_epoch = 0, era_start_slot = 0 => epoch 296 window is
    // [296*432000, 297*432000-1] = [127_872_000, 128_303_999]. MANIFEST_SLOT
    // (23_013_663) does NOT fall there, so use an era anchored so that the
    // certified slot lands inside epoch 296.
    const ERA_START_EPOCH: u64 = EPOCH_NO;
    // Anchor the era at EPOCH_NO so MANIFEST_SLOT sits inside the first window.
    // Choose era_start_slot so MANIFEST_SLOT is within [start, start+len-1].
    const ERA_START_SLOT: u64 = MANIFEST_SLOT - (MANIFEST_SLOT % EPOCH_LENGTH);

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

    /// Minimal append-order in-memory `WalStore` double (always-Ok).
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
    impl WalStore for VecWal {
        fn append(&mut self, entry: WalEntry) -> Result<(), WalError> {
            self.entries.push(entry);
            Ok(())
        }
        fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
            Ok(self.entries.clone())
        }
    }

    /// A WAL double whose `append` ALWAYS fails — injects the interrupted-import
    /// (the WAL commit point cannot be written).
    struct FailingWal;
    impl WalStore for FailingWal {
        fn append(&mut self, _entry: WalEntry) -> Result<(), WalError> {
            Err(WalError::Io(std::io::ErrorKind::Other))
        }
        fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
            Ok(Vec::new())
        }
    }

    fn pool(id: u8) -> PoolId {
        PoolId(Hash28([id; 28]))
    }

    fn nonce(b: u8) -> LedgerNonce {
        LedgerNonce([b; 32])
    }

    /// A hermetic S1a `NativeSnapshotNonUtxoState` bound to the manifest point.
    // CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1: one identity-bound proposal + a committee for the threading
    // test, so a `Some(empty)` regression (proposal data silently dropped) FAILS rather than passes.
    const SAMPLE_GAS_TXID: [u8; 32] = [0xAB; 32];
    const SAMPLE_DEPOSIT: u64 = 100_000_000_000;
    const SAMPLE_RETURN_ADDR: [u8; 29] = [0xE0; 29];
    const SAMPLE_PROPOSED_IN: u64 = 1309;
    const SAMPLE_EXPIRES_AFTER: u64 = 1339;
    const SAMPLE_COMMITTEE_MEMBER: [u8; 28] = [0xC0; 28];
    const SAMPLE_QUORUM: (u64, u64) = (2, 3);

    fn sample_imported_gov() -> ade_ledger::ledgerdb_state::ImportedGovState {
        use ade_types::conway::governance::{GovAction, GovActionId, GovActionState};
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::tx::Coin;
        let mut committee = std::collections::BTreeMap::new();
        committee.insert(StakeCredential::KeyHash(Hash28(SAMPLE_COMMITTEE_MEMBER)), 1340u64);
        ade_ledger::ledgerdb_state::ImportedGovState {
            proposals: vec![GovActionState {
                action_id: GovActionId {
                    tx_hash: Hash32(SAMPLE_GAS_TXID),
                    index: 0,
                },
                committee_votes: Vec::new(), // 0 committee Yes ⇒ committee gate fails ⇒ provably unratifiable
                drep_votes: Vec::new(),
                spo_votes: Vec::new(),
                deposit: Coin(SAMPLE_DEPOSIT),
                return_addr: SAMPLE_RETURN_ADDR.to_vec(),
                gov_action: GovAction::TreasuryWithdrawals {
                    withdrawals: vec![(SAMPLE_RETURN_ADDR.to_vec(), Coin(400_000_000_000))],
                    policy_hash: None,
                },
                proposed_in: EpochNo(SAMPLE_PROPOSED_IN),
                expires_after: EpochNo(SAMPLE_EXPIRES_AFTER),
            }],
            committee,
            committee_quorum: Some(SAMPLE_QUORUM),
        }
    }

    fn s1a_state() -> NativeSnapshotNonUtxoState {
        let mut pp = ProtocolParameters::default();
        // Conway per-byte rule (S1a decodes coinsPerUTxOByte into PerByte).
        pp.min_utxo_rule = MinUtxoRule::PerByte(Coin(4310));
        // testnet magic 1 -> network_id 0; S1a binds it onto the params too.
        pp.network_id = 0;

        let mut pool_distr: BTreeMap<PoolId, (u64, Hash32)> = BTreeMap::new();
        pool_distr.insert(pool(0x01), (1_000u64, Hash32([0x07; 32])));
        pool_distr.insert(pool(0x05), (2_500u64, Hash32([0x08; 32])));

        // ECA-5: the MARK (seed+1) leadership — same pools/VRFs as nesPd (so the VRF cross-check passes),
        // distinct next-epoch stakes.
        let mut mark_pool_distr: BTreeMap<PoolId, (u64, Hash32)> = BTreeMap::new();
        mark_pool_distr.insert(pool(0x01), (1_100u64, Hash32([0x07; 32])));
        mark_pool_distr.insert(pool(0x05), (2_700u64, Hash32([0x08; 32])));

        let mut block_production: BTreeMap<PoolId, u64> = BTreeMap::new();
        block_production.insert(pool(0x01), 3);
        // nesBcur: current-epoch blocks so far (distinct from the nesBprev count above).
        let mut current_block_production: BTreeMap<PoolId, u64> = BTreeMap::new();
        current_block_production.insert(pool(0x01), 2);

        // CE-3d: a non-empty mark/set/go snapshot pipeline so the assembled ledger's
        // `epoch_state.snapshots` is verifiably threaded (not a cold-start empty default).
        use ade_ledger::epoch::{GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotState, StakeSnapshot};
        let stake_snapshot = |pid: PoolId, coin: u64| {
            let mut delegations = BTreeMap::new();
            delegations.insert((pid.0).clone(), (pid.clone(), Coin(coin)));
            let mut pool_stakes = BTreeMap::new();
            pool_stakes.insert(pid, Coin(coin));
            StakeSnapshot {
                delegations,
                pool_stakes,
            }
        };
        let snapshots = SnapshotState {
            mark: MarkSnapshot(stake_snapshot(pool(0x01), 1_100)),
            set: SetSnapshot(stake_snapshot(pool(0x01), 1_050)),
            go: GoSnapshot(stake_snapshot(pool(0x01), 1_000)),
        };

        NativeSnapshotNonUtxoState {
            era: CardanoEra::Conway,
            network_id: 0,
            epoch: EpochNo(EPOCH_NO),
            point: SeedPoint {
                slot: SlotNo(MANIFEST_SLOT),
                block_hash: Hash32(MANIFEST_BLOCK_HASH),
            },
            cert_state: CertState::new(),
            praos_nonces: PraosNonces {
                evolving: nonce(0x11),
                candidate: nonce(0x22),
                epoch: nonce(0x33),
                lab: nonce(0x44),
                last_epoch_block: nonce(0x55),
            },
            pool_distr,
            mark_pool_distr,
            snapshots,
            protocol_params: pp,
            reserves: Coin(13_000_000_000_000_000),
            treasury: Coin(1_000_000_000_000),
            block_production,
            current_block_production,
            reward_deltas: std::collections::BTreeMap::new(),
            rupd_delta_treasury: Coin(0),
            rupd_delta_reserves: Coin(0),
            epoch_fees: Coin(0),
            imported_gov: sample_imported_gov(),
            reward_nibble_observation: RewardNibbleObservation::Mixed,
        }
    }

    fn s1a_commitment_fixture() -> Hash32 {
        Hash32([0xAB; 32])
    }

    fn binding() -> VerifiedManifestBinding {
        VerifiedManifestBinding {
            network_magic: MANIFEST_NETWORK_MAGIC,
            genesis_hash: Hash32(MANIFEST_GENESIS_HASH),
            certified_point: SeedPoint {
                slot: SlotNo(MANIFEST_SLOT),
                block_hash: Hash32(MANIFEST_BLOCK_HASH),
            },
            immutable_range: (0, 4242),
        }
    }

    fn genesis() -> NativeGenesisConstants {
        NativeGenesisConstants {
            max_lovelace_supply: 45_000_000_000_000_000,
            active_slots_coeff: ActiveSlotsCoeff {
                numer: 5,
                denom: 100,
            },
        }
    }

    fn schedule() -> EraSchedule {
        EraSchedule::new(
            BootstrapAnchorHash(Hash32([0u8; 32])),
            0,
            vec![EraSummary {
                randomness_stabilisation_window_slots: None,
                era: CardanoEra::Conway,
                start_slot: SlotNo(ERA_START_SLOT),
                start_epoch: EpochNo(ERA_START_EPOCH),
                slot_length_ms: 1_000,
                epoch_length_slots: EPOCH_LENGTH as u32,
                safe_zone_slots: EPOCH_LENGTH as u32,
            }],
        )
        .expect("schedule")
    }

    /// A non-empty Stage-2 UTxO double — built via `from_map`, the exact entry
    /// path S1b consumes (the Stage-2 materialization is upstream; S1b takes the
    /// `UTxOState`). A single ada-only output keyed by a fixed TxIn.
    fn stage2_utxo() -> UTxOState {
        use ade_ledger::utxo::TxOut;
        use ade_ledger::value::Value;
        use ade_types::tx::TxIn;
        let mut m: BTreeMap<TxIn, TxOut> = BTreeMap::new();
        m.insert(
            TxIn {
                tx_hash: Hash32([0xCC; 32]),
                index: 0,
            },
            TxOut::ShelleyMary {
                address: vec![0x00; 29],
                value: Value::from_coin(Coin(1_710_000)),
            },
        );
        UTxOState::from_map(m)
    }

    /// A leadership view consistent with the native consensus inputs (cold-start
    /// composition never consumes it, but it is built faithfully).
    fn ledger_view(c: &LiveConsensusInputsCanonical) -> ade_ledger::consensus_view::PoolDistrView {
        let mut pools: BTreeMap<Hash28, ade_ledger::consensus_view::PoolEntry> = BTreeMap::new();
        let mut total = 0u64;
        for (k, v) in &c.pool_distribution {
            total = total.saturating_add(v.active_stake);
            let vrf = c
                .pool_vrf_keyhashes
                .get(k)
                .cloned()
                .unwrap_or(Hash32([0u8; 32]));
            pools.insert(
                k.clone(),
                ade_ledger::consensus_view::PoolEntry {
                    active_stake: v.active_stake,
                    vrf_keyhash: vrf,
                },
            );
        }
        ade_ledger::consensus_view::PoolDistrView::new(c.epoch_no, total, c.active_slots_coeff, pools)
    }

    #[test]
    fn native_assembled_seed_is_deterministic() {
        // Same verified inputs -> byte-identical assembled seed + commitments
        // (the LiveConsensusInputsCanonical fingerprint is part of the struct).
        let s1a = s1a_state();
        let a = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &binding(),
            &genesis(),
            &schedule(),
        )
        .expect("assemble a");
        let b = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &binding(),
            &genesis(),
            &schedule(),
        )
        .expect("assemble b");
        assert_eq!(a, b, "the assembled native seed is deterministic");
        // The canonical-inputs fingerprint (a commitment over every field) is
        // equal too — a stronger byte-level determinism witness.
        assert_eq!(
            a.consensus_inputs.fingerprint, b.consensus_inputs.fingerprint,
            "the native consensus-inputs fingerprint is deterministic"
        );
    }

    #[test]
    fn native_assembly_maps_each_field_from_its_source() {
        let s1a = s1a_state();
        let g = genesis();
        let bind = binding();
        let seed = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &bind,
            &g,
            &schedule(),
        )
        .expect("assemble");

        // LedgerState field sources.
        assert_eq!(seed.ledger.era, CardanoEra::Conway);
        assert!(!seed.ledger.track_utxo, "track_utxo = false");
        // CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1: gov_state is SEEDED from the imported proposals +
        // committee — never None, and never Some(empty) (a regression dropping proposal data must fail
        // HERE, not silently pass). Assert the meaningful imported state + a full identity-bound record.
        {
            use ade_types::shelley::cert::StakeCredential;
            let gov = seed
                .ledger
                .gov_state
                .as_ref()
                .expect("gov_state seeded from the imported proposals + committee");
            assert_eq!(gov.proposals.len(), 1, "imported proposal count preserved");
            assert_eq!(gov.committee_quorum, SAMPLE_QUORUM, "imported committee quorum preserved");
            assert_eq!(
                gov.committee.get(&StakeCredential::KeyHash(Hash28(SAMPLE_COMMITTEE_MEMBER))),
                Some(&1340u64),
                "imported committee member preserved",
            );
            let p = &gov.proposals[0];
            assert_eq!(p.action_id.tx_hash, Hash32(SAMPLE_GAS_TXID), "GovActionId tx_hash bound");
            assert_eq!(p.action_id.index, 0, "GovActionId index bound");
            assert_eq!(p.deposit.0, SAMPLE_DEPOSIT, "deposit bound");
            assert_eq!(p.return_addr, SAMPLE_RETURN_ADDR.to_vec(), "return_addr bound");
            assert_eq!(p.proposed_in.0, SAMPLE_PROPOSED_IN, "proposed_in bound");
            assert_eq!(p.expires_after.0, SAMPLE_EXPIRES_AFTER, "expires_after bound");
            assert!(
                matches!(
                    p.gov_action,
                    ade_types::conway::governance::GovAction::TreasuryWithdrawals { .. }
                ),
                "action kind bound",
            );
        }
        assert!(
            seed.ledger.conway_deposit_params.is_none(),
            "conway_deposit_params = None"
        );
        assert_eq!(seed.ledger.max_lovelace_supply, g.max_lovelace_supply);
        assert_eq!(seed.ledger.cert_state, s1a.cert_state);
        assert_eq!(seed.ledger.protocol_params, s1a.protocol_params);
        // epoch_state sources.
        assert_eq!(seed.ledger.epoch_state.epoch, s1a.epoch);
        assert_eq!(
            seed.ledger.epoch_state.slot, bind.certified_point.slot,
            "epoch_state.slot <- manifest point"
        );
        assert_eq!(seed.ledger.epoch_state.reserves, s1a.reserves);
        assert_eq!(seed.ledger.epoch_state.treasury, s1a.treasury);
        assert_eq!(seed.ledger.epoch_state.block_production, s1a.block_production);
        assert_eq!(seed.ledger.epoch_state.epoch_fees, Coin(0), "epoch_fees = 0");
        // CE-3d: the mark/set/go snapshots are threaded from S1a (the certified snapshot's
        // esSnapshots), NOT a cold-start empty default. This is what seeds the accumulator's `go`.
        assert_eq!(seed.ledger.epoch_state.snapshots, s1a.snapshots);
        assert!(
            !seed.ledger.epoch_state.snapshots.go.0.pool_stakes.is_empty(),
            "go snapshot must be non-empty (the reward/leadership stake authority)"
        );
        // UTxO <- Stage-2 (the single ada-only entry).
        assert_eq!(seed.ledger.utxo_state.len(), 1, "utxo_state <- Stage-2");

        // PraosChainDepState: five nonces <- S1a (record-order mapping).
        assert_eq!(seed.chain_dep.evolving_nonce, CoreNonce(Hash32([0x11; 32])));
        assert_eq!(seed.chain_dep.candidate_nonce, CoreNonce(Hash32([0x22; 32])));
        assert_eq!(seed.chain_dep.epoch_nonce, CoreNonce(Hash32([0x33; 32])));
        assert_eq!(seed.chain_dep.lab_nonce, CoreNonce(Hash32([0x44; 32])));
        assert_eq!(
            seed.chain_dep.previous_epoch_nonce,
            CoreNonce(Hash32([0x55; 32])),
            "previous_epoch_nonce <- S1a last_epoch_block"
        );
        assert!(seed.chain_dep.op_cert_counters.is_empty(), "op-cert counters empty");
        assert!(seed.chain_dep.last_slot.is_none(), "last_* = None (cold start)");

        // NATIVE LiveConsensusInputsCanonical sources.
        let ci = &seed.consensus_inputs;
        assert_eq!(ci.network_magic, bind.network_magic, "magic <- manifest");
        assert_eq!(ci.genesis_hash, bind.genesis_hash, "genesis_hash <- manifest");
        assert_eq!(ci.epoch_no, s1a.epoch, "epoch_no <- S1a");
        assert_eq!(ci.active_slots_coeff, g.active_slots_coeff, "ASC <- genesis");
        assert_eq!(
            ci.epoch_nonce,
            CoreNonce(Hash32([0x33; 32])),
            "epoch_nonce <- S1a eta0"
        );
        assert_eq!(ci.pool_distribution.len(), 2, "pool stake <- S1a pool_distr");
        assert_eq!(
            ci.pool_distribution.get(&Hash28([0x01; 28])).unwrap().active_stake,
            1_000
        );
        assert_eq!(
            ci.pool_vrf_keyhashes.get(&Hash28([0x01; 28])).unwrap(),
            &Hash32([0x07; 32]),
            "pool VRF <- S1a pool_distr"
        );
        assert_eq!(
            ci.source_tip_slot, bind.certified_point.slot,
            "source_tip <- manifest point"
        );
        assert_eq!(ci.source_tip_hash, bind.certified_point.block_hash);
        // No operator-bundle provenance on the native bundle.
        assert!(ci.protocol_params_json.is_none(), "no JSON preimage on native path");
        assert_eq!(ci.source_query_command, NATIVE_SOURCE_MARKER);
        assert_eq!(ci.source_cardano_node_version, NATIVE_SOURCE_MARKER);

        // seed_point inputs: the anchor seed_point IS the manifest point.
        assert_eq!(seed.seed_point_inputs.seed_slot, bind.certified_point.slot);
        assert_eq!(
            seed.seed_point_inputs.seed_block_hash,
            bind.certified_point.block_hash
        );
        assert_eq!(
            seed.seed_point_inputs.seed_artifact_hash,
            s1a_commitment_fixture(),
            "seed_artifact_hash <- S1a commitment"
        );
    }

    #[test]
    fn point_mismatch_is_terminal() {
        let mut s1a = s1a_state();
        s1a.point.slot = SlotNo(99_999_999);
        let err = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &binding(),
            &genesis(),
            &schedule(),
        )
        .expect_err("point slot mismatch must be terminal");
        assert!(matches!(err, MithrilNativeAssemblyError::PointMismatch { .. }));
    }

    #[test]
    fn point_hash_mismatch_is_terminal() {
        let mut s1a = s1a_state();
        s1a.point.block_hash = Hash32([0xAB; 32]);
        let err = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &binding(),
            &genesis(),
            &schedule(),
        )
        .expect_err("point hash mismatch must be terminal");
        assert!(matches!(err, MithrilNativeAssemblyError::PointHashMismatch));
    }

    #[test]
    fn wrong_era_is_terminal() {
        let mut s1a = s1a_state();
        s1a.era = CardanoEra::Babbage;
        let err = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &binding(),
            &genesis(),
            &schedule(),
        )
        .expect_err("non-Conway era must be terminal");
        assert!(matches!(err, MithrilNativeAssemblyError::NonConwayEra { .. }));
    }

    #[test]
    fn wrong_network_is_terminal() {
        // S1a network_id (1 = mainnet) disagrees with the manifest magic
        // (testnet 1 -> network_id 0).
        let mut s1a = s1a_state();
        s1a.network_id = 1;
        let err = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &binding(),
            &genesis(),
            &schedule(),
        )
        .expect_err("network mismatch must be terminal");
        assert!(matches!(
            err,
            MithrilNativeAssemblyError::NetworkMismatch {
                s1a_network_id: 1,
                manifest_network_id: 0
            }
        ));
    }

    #[test]
    fn epoch_mismatch_is_terminal() {
        // S1a epoch disagrees with the epoch the schedule resolves for the
        // certified slot.
        let mut s1a = s1a_state();
        s1a.epoch = EpochNo(EPOCH_NO + 1);
        let err = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &binding(),
            &genesis(),
            &schedule(),
        )
        .expect_err("epoch mismatch must be terminal");
        assert!(matches!(err, MithrilNativeAssemblyError::EpochMismatch { .. }));
    }

    #[test]
    fn native_bootstrap_persists_and_anchor_point_is_recoverable() {
        // The native entry persists ATOMICALLY on a fresh empty store: the
        // sidecar, the recovered-anchor point, and the WAL provenance (commit).
        // The imported anchor point is recoverable via load_recovered_anchor_point.
        let db = InMemoryChainDb::new();
        let mut wal = VecWal::new();
        let s1a = s1a_state();
        let sched = schedule();
        let g = genesis();
        let bind = binding();
        // Build the leadership view from the assembled inputs.
        let pre = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &bind,
            &g,
            &sched,
        )
        .expect("pre-assemble for view");
        let view = ledger_view(&pre.consensus_inputs);

        let out = bootstrap_from_native_mithril_snapshot(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &bind,
            &g,
            MANIFEST.as_bytes(),
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect("native bootstrap persists");

        assert!(out.tip.is_none(), "cold-start has no tip");
        assert!(matches!(
            out.anchor.seed_provenance,
            ade_ledger::bootstrap_anchor::SeedProvenance::Mithril { .. }
        ));
        // The anchor's seed_point IS the manifest point (final coherence leg).
        assert_eq!(out.anchor.seed_point.slot, SlotNo(MANIFEST_SLOT));
        assert_eq!(out.anchor.seed_point.block_hash, Hash32(MANIFEST_BLOCK_HASH));

        // The recovered anchor POINT is explicitly evidenced + recoverable,
        // keyed by the anchor's initial_ledger_fingerprint.
        let anchor_fp = out.anchor.initial_ledger_fingerprint.clone();
        let tip = load_recovered_anchor_point(&db, &anchor_fp)
            .expect("recovered anchor point loads");
        assert_eq!(tip.slot, SlotNo(MANIFEST_SLOT));
        assert_eq!(tip.hash, Hash32(MANIFEST_BLOCK_HASH));
        // And the raw record decodes + binds to the same anchor lineage.
        let raw = db
            .get_recovered_anchor_point(&anchor_fp)
            .expect("get")
            .expect("present");
        let rec = decode_recovered_anchor_point(&raw).expect("decode");
        assert_eq!(rec.anchor_fp, anchor_fp);

        // The seed-epoch sidecar is present, and the WAL provenance (the commit
        // point) makes the lineage discoverable.
        assert!(db
            .get_seed_epoch_consensus_inputs(&anchor_fp)
            .expect("get sidecar")
            .is_some());
        let entries = wal.read_all().expect("read_all");
        let bb = BTreeMap::new();
        let replay = replay_from_anchor(&anchor_fp, &entries, &bb).expect("replay");
        assert!(
            replay.provenance.is_some(),
            "the WAL provenance makes the anchor lineage discoverable"
        );
    }

    #[test]
    fn interrupted_persist_leaves_no_discoverable_anchor_lineage() {
        // Inject a WAL-append failure (the commit point). The bootstrap fails;
        // the anchor lineage is NOT discoverable (no WAL provenance), so a
        // warm-start recovers the store as "not imported" — no bootable partial
        // authority state.
        let db = InMemoryChainDb::new();
        let mut wal = FailingWal;
        let s1a = s1a_state();
        let sched = schedule();
        let g = genesis();
        let bind = binding();
        let pre = assemble_native_mithril_seed(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &bind,
            &g,
            &sched,
        )
        .expect("pre-assemble for view");
        let view = ledger_view(&pre.consensus_inputs);

        let err = bootstrap_from_native_mithril_snapshot(
            &s1a,
            s1a_commitment_fixture(),
            stage2_utxo(),
            &bind,
            &g,
            MANIFEST.as_bytes(),
            &db,
            &db,
            &mut wal,
            &sched,
            &view,
        )
        .expect_err("a failed WAL commit must fail the import");
        assert!(matches!(
            err,
            NativeMithrilBootstrapError::Bootstrap(
                MithrilBootstrapError::SeedConsensusProvenanceWal(_)
            )
        ));

        // No WAL provenance was committed -> the lineage is not discoverable.
        // The recovered-anchor-point key is the assembled anchor's
        // initial_ledger_fingerprint (recompute it from the assembled seed).
        let anchor_fp = pre.seed_point_inputs.initial_ledger_fingerprint.clone();
        let entries = wal.read_all().expect("read_all");
        assert!(entries.is_empty(), "the failing WAL committed nothing");
        let bb = BTreeMap::new();
        let replay = replay_from_anchor(&anchor_fp, &entries, &bb).expect("replay");
        assert!(
            replay.provenance.is_none(),
            "no WAL provenance -> the anchor lineage is NOT discoverable (no bootable partial state)"
        );
    }
}
