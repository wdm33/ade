// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED live `--mode node` FirstRun -> NATIVE Mithril bootstrap route
//! (MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d).
//!
//! This is the INVOCATION wiring: it routes the live FirstRun inputs
//! (the verified Mithril manifest + the V2 LedgerDB `state` file + the
//! Stage-2 `tables` MemPack + the Cardano Shelley genesis) through the
//! UNCHANGED native chain already shipped by S1a/S1b/S1c:
//!   1. `import_mithril_manifest_from_bytes` -> the `VerifiedManifestBinding`;
//!   2. `decode_native_nonutxo_state` (S1a) -> `(s1a, s1a_commitment)`;
//!   3. `materialize_tables_to_utxo` (S1c) -> `UTxOState`;
//!   4. the Shelley genesis -> `NativeGenesisConstants` + the single-era
//!      Conway `EraSchedule` (see [`build_native_schedule`]);
//!   5. `bootstrap_from_native_mithril_snapshot` (S1b) -> the atomic persist.
//!
//! There is NO cardano-cli / JSON consensus-input bundle / operator seed on
//! this route; the verified snapshot IS the source. A forbidden flag
//! (`--json-seed-path` / `--consensus-inputs-path`) supplied ALONGSIDE the
//! native inputs is a structured terminal error (no ambiguous,
//! half-authoritative path) — see `first_run_mithril_bootstrap`.
//!
//! Every failure here is TERMINAL before authority visibility (the WAL
//! commit-point inside `bootstrap_from_native_mithril_snapshot` is the sole
//! discovery gate): a missing / mixed component, a manifest / point / network
//! / era mismatch, or a decode / materialize / assemble / persist failure all
//! leave NO bootable partial state.

use ade_core::consensus::era_schedule::{EraSchedule, EraSummary};
use ade_core::consensus::ledger_view::LedgerView;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_core::consensus::BootstrapAnchorHash;
use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::{decode_native_nonutxo_state, NativeNonUtxoError};
use ade_ledger::mithril_utxo_materialize::{materialize_tables_to_utxo, TxOutMaterializeError};
use ade_runtime::mithril_bootstrap::MithrilBootstrapOutput;
use ade_runtime::mithril_import::import_mithril_manifest_from_bytes;
use ade_runtime::mithril_native_assembly::{
    bootstrap_from_native_mithril_snapshot, NativeGenesisConstants, NativeMithrilBootstrapError,
    VerifiedManifestBinding,
};
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

/// The Conway era index `materialize_tables_to_utxo` requires (the snapshot is
/// a Conway V2 LedgerDB). Kept in lock-step with the materializer's own
/// `CONWAY_ERA_INDEX`.
const CONWAY_ERA_INDEX: usize = 6;

/// Mainnet network magic — the sole non-testnet network. Mirrors S1a's
/// `network_id_from_magic` derivation boundary.
const MAINNET_NETWORK_MAGIC: u32 = 764_824_073;
const PREPROD_NETWORK_MAGIC: u32 = 1;
const PREVIEW_NETWORK_MAGIC: u32 = 2;

/// Closed terminal error sum for the native FirstRun route. Every variant is a
/// fail-closed halt BEFORE the WAL commit-point (authority visibility): there
/// is no partial state and no fallback to the cardano-cli / JSON seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFirstRunError {
    /// A required native-route file (manifest / state / tables / shelley
    /// genesis) could not be read. Carries the path label + the io error kind
    /// (no path bytes).
    ComponentRead {
        component: &'static str,
        kind: std::io::ErrorKind,
    },
    /// The Mithril manifest could not be imported (malformed / unsupported
    /// artifact type / bad hash hex / inverted immutable range). Fail closed.
    ManifestImport(String),
    /// The Shelley genesis JSON could not be parsed, or a required field was
    /// missing / malformed (no implicit default, no stringly fallback).
    GenesisParse(NativeGenesisParseError),
    /// The manifest network magic is not one of the closed supported networks
    /// (mainnet / preprod / preview), so the Shelley era boundary — the per-
    /// network compatibility constant the single-era schedule is anchored at —
    /// is unknown. Fail closed rather than guess a boundary.
    UnknownNetworkBoundary { network_magic: u32 },
    /// Deriving the certified slot's epoch / window from the Shelley boundary +
    /// the genesis epoch length overflowed or placed the certified slot before
    /// the Shelley boundary. The certified point is incoherent with the network
    /// geometry. Fail closed.
    EpochGeometry { certified_slot: u64 },
    /// The S1a non-UTxO decode (`decode_native_nonutxo_state`) fail-closed —
    /// a non-Conway telescope, an epoch mismatch, a missing snapshot field, or
    /// an inconsistent VRF binding. Carries the closed `NativeNonUtxoError`
    /// debug. Fail closed (no default fill).
    NonUtxoDecode(String),
    /// The Stage-2 `tables` -> `UTxOState` materialization
    /// (`materialize_tables_to_utxo`) fail-closed — an unsupported TxOut tag /
    /// address form / script language / value tag / non-ascending key, or a
    /// non-Conway era. Carries the closed `TxOutMaterializeError` debug. Fail
    /// closed (no opaque keep-bytes fallback).
    UtxoMaterialize(String),
    /// `bootstrap_from_native_mithril_snapshot` (S1b) fail-closed — the native
    /// assembly / point-coherence gate, or the single closed composition's
    /// verdict (binding, bootstrap authority, sidecar persist, or WAL append).
    /// Carries the closed `NativeMithrilBootstrapError` debug. Fail closed — NO
    /// fallback, NO bootable partial state.
    NativeBootstrap(String),
    /// The inline EVIEW reduced-checkpoint build (S2 / DC-MITHRIL-08) fail-closed
    /// — `open` / `build_from` / `seal_bootstrap` over the materialized UTxO before
    /// it is consumed. Carries the closed `ReducedCheckpointError` debug. Fail
    /// closed — a build failure aborts bootstrap; NO bootable partial state.
    ReducedCheckpoint(String),
    /// The verified Mithril manifest does NOT bind to the selected `--network` profile (its
    /// network magic or Shelley genesis hash differs from the committed NetworkProfile) -- the
    /// snapshot is for a different network than selected. Fail closed. (Skipped on the advanced
    /// custom path, where `--shelley-genesis-path` supplies the constants directly.)
    NetworkMismatch(String),
}

/// Closed Shelley-genesis parse error surface for the native route. The
/// `field` discriminants are `&'static str` from a closed list (no `String`
/// payload on the load-bearing variants).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGenesisParseError {
    /// The genesis bytes are not a JSON object.
    JsonShape,
    /// A required field is absent.
    MissingField { name: &'static str },
    /// A required field has the wrong JSON type (e.g. a string for a numeric
    /// field) — no stringly fallback.
    MalformedType { name: &'static str },
    /// A required field carries an out-of-range / un-representable value (e.g.
    /// an `activeSlotsCoeff` that is not a finite positive decimal, or an
    /// `epochLength` exceeding `u32`).
    MalformedValue { name: &'static str },
}

/// The genesis facts the native route consumes: the `NativeGenesisConstants`
/// (`max_lovelace_supply` + `active_slots_coeff`) the S1b assembly takes, plus
/// the `epoch_length_slots` the single-era Conway schedule needs. Sourced from
/// the real Cardano `shelley-genesis.json`, never a cardano-cli query or the
/// operator bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeGenesisFacts {
    pub constants: NativeGenesisConstants,
    pub epoch_length_slots: u32,
}

/// Parse a real Cardano `shelley-genesis.json` into the native genesis facts.
///
/// Extracts `maxLovelaceSupply` (u64), `activeSlotsCoeff` (a JSON decimal, e.g.
/// `0.05`, converted to the exact `ActiveSlotsCoeff { numer, denom }` rational
/// WITHOUT any float arithmetic), and `epochLength` (u32). Fail-closed on a
/// missing / malformed field — no implicit default, no stringly fallback.
///
/// `activeSlotsCoeff` is read from its raw JSON token text (the serde_json
/// `Number`'s `to_string`) and converted decimal-string -> rational by counting
/// fractional digits, so no `f64` ever enters the authoritative state.
pub fn parse_native_shelley_genesis(
    json_bytes: &[u8],
) -> Result<NativeGenesisFacts, NativeGenesisParseError> {
    let json: serde_json::Value =
        serde_json::from_slice(json_bytes).map_err(|_| NativeGenesisParseError::JsonShape)?;
    let obj = json.as_object().ok_or(NativeGenesisParseError::JsonShape)?;

    let max_lovelace_supply = require_u64(obj, "maxLovelaceSupply")?;

    let epoch_length_u64 = require_u64(obj, "epochLength")?;
    let epoch_length_slots = u32::try_from(epoch_length_u64)
        .map_err(|_| NativeGenesisParseError::MalformedValue { name: "epochLength" })?;
    if epoch_length_slots == 0 {
        return Err(NativeGenesisParseError::MalformedValue { name: "epochLength" });
    }

    let active_slots_coeff = parse_active_slots_coeff(obj)?;

    Ok(NativeGenesisFacts {
        constants: NativeGenesisConstants {
            max_lovelace_supply,
            active_slots_coeff,
        },
        epoch_length_slots,
    })
}

/// Convert the Shelley genesis `activeSlotsCoeff` decimal token into the exact
/// `ActiveSlotsCoeff` rational. No float arithmetic: the JSON `Number`'s
/// canonical decimal text (e.g. "0.05") is split on '.', the integer + frac
/// digits give `numer = int*10^k + frac`, `denom = 10^k` (k = frac-digit
/// count), then reduced by gcd. Rejects a non-positive / non-finite / overly
/// long coefficient (fail-closed; the rational must fit `u32`).
fn parse_active_slots_coeff(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<ActiveSlotsCoeff, NativeGenesisParseError> {
    let name = "activeSlotsCoeff";
    let v = obj
        .get(name)
        .ok_or(NativeGenesisParseError::MissingField { name })?;
    let num = match v {
        serde_json::Value::Number(n) => n,
        _ => return Err(NativeGenesisParseError::MalformedType { name }),
    };
    // Canonical decimal text of the JSON number (no exponent for the genesis
    // coefficients we target; an exponent form is rejected below).
    let text = num.to_string();
    decimal_text_to_rational(&text)
        .ok_or(NativeGenesisParseError::MalformedValue { name })
}

/// Parse a plain decimal string (`"0.05"`, `"1"`, `"0.5"`) into a reduced
/// `ActiveSlotsCoeff { numer, denom }`. Returns `None` on an empty / non-digit /
/// exponent / sign-bearing / overflowing input, or a non-positive value.
fn decimal_text_to_rational(text: &str) -> Option<ActiveSlotsCoeff> {
    // Reject exponent / sign forms (genesis coefficients are plain decimals).
    if text.is_empty()
        || text.contains('e')
        || text.contains('E')
        || text.contains('-')
        || text.contains('+')
    {
        return None;
    }
    let (int_part, frac_part) = match text.split_once('.') {
        Some((i, f)) => (i, f),
        None => (text, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    // All remaining characters must be ASCII digits.
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let frac_digits = frac_part.len();
    // denom = 10^frac_digits; numer = int*denom + frac. Compute in u64 and
    // require the reduced pair to fit u32.
    let mut denom: u64 = 1;
    for _ in 0..frac_digits {
        denom = denom.checked_mul(10)?;
    }
    let int_val: u64 = if int_part.is_empty() {
        0
    } else {
        int_part.parse::<u64>().ok()?
    };
    let frac_val: u64 = if frac_part.is_empty() {
        0
    } else {
        frac_part.parse::<u64>().ok()?
    };
    let numer = int_val.checked_mul(denom)?.checked_add(frac_val)?;
    if numer == 0 {
        return None;
    }
    let g = gcd_u64(numer, denom);
    let numer_r = numer / g;
    let denom_r = denom / g;
    let numer_u32 = u32::try_from(numer_r).ok()?;
    let denom_u32 = u32::try_from(denom_r).ok()?;
    Some(ActiveSlotsCoeff {
        numer: numer_u32,
        denom: denom_u32,
    })
}

/// Euclidean gcd (deterministic integer arithmetic).
fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

/// The per-network Shelley era boundary `(start_epoch, start_slot)` — the fixed
/// compatibility constant the single-era Conway schedule is anchored against
/// (the absolute first slot of any post-Shelley epoch is
/// `start_slot + (epoch - start_epoch) * epoch_length`). CLOSED + enumerated
/// (mirrors `bootstrap_export::resolve_network_profile`); an unknown magic is
/// terminal upstream. The values are the reviewed committed boundaries (the
/// same table `genesis_parser_corpus.rs` pins as the oracle).
fn shelley_boundary_for_magic(network_magic: u32) -> Option<(EpochNo, SlotNo)> {
    match network_magic {
        // Mainnet Shelley hard fork: epoch 208, slot 4_492_800.
        MAINNET_NETWORK_MAGIC => Some((EpochNo(208), SlotNo(4_492_800))),
        // Preprod Shelley start: epoch 4, slot 86_400 (4 Byron epochs of 21_600).
        PREPROD_NETWORK_MAGIC => Some((EpochNo(4), SlotNo(86_400))),
        // Preview is Byron-free: epoch 0, slot 0 (epoch == slot / epoch_length).
        PREVIEW_NETWORK_MAGIC => Some((EpochNo(0), SlotNo(0))),
        _ => None,
    }
}

/// Resolve the certified slot's epoch from the network Shelley boundary +
/// the genesis epoch length:
/// `epoch = shelley_start_epoch + (certified_slot - shelley_start_slot) / epoch_length`.
/// This is the SAME slot->epoch arithmetic `EraSchedule::locate` applies; it is
/// the `manifest_epoch` cross-check S1a binds the decoded NES epoch against (a
/// disagreement is fail-closed inside the decoder). Returns `None` (terminal)
/// when the certified slot precedes the Shelley boundary or the arithmetic
/// overflows.
fn epoch_for_certified_slot(
    certified_slot: SlotNo,
    boundary: (EpochNo, SlotNo),
    epoch_length_slots: u32,
) -> Option<EpochNo> {
    let (start_epoch, start_slot) = boundary;
    let slots_in = certified_slot.0.checked_sub(start_slot.0)?;
    let epoch_len = u64::from(epoch_length_slots);
    if epoch_len == 0 {
        return None;
    }
    let epoch_offset = slots_in / epoch_len;
    let epoch = start_epoch.0.checked_add(epoch_offset)?;
    Some(EpochNo(epoch))
}

/// Build the single-era Conway `EraSchedule` the native assembly consumes,
/// anchored at the SNAPSHOT epoch's absolute geometry so that
/// `locate(certified_slot).epoch == snapshot_epoch` and
/// `derive_epoch_window(schedule, snapshot_epoch)` is the window that contains
/// the certified slot. The anchor is the per-network Shelley boundary +
/// the genesis epoch length (NOT the operator bundle). The era's `start_epoch`
/// is the snapshot epoch and its `start_slot` is that epoch's absolute first
/// slot, so a single Conway summary suffices for the snapshot-epoch window the
/// cold-start bootstrap needs.
///
/// Returns `None` (terminal upstream) if `EraSchedule::new` rejects the summary
/// (a zero epoch length — a caller bug, since the genesis epoch length is
/// validated non-zero in [`parse_native_shelley_genesis`]).
pub fn build_native_schedule(
    snapshot_epoch: EpochNo,
    boundary: (EpochNo, SlotNo),
    epoch_length_slots: u32,
) -> Option<EraSchedule> {
    let (start_epoch, start_slot) = boundary;
    // The snapshot epoch's absolute first slot under the network geometry.
    let epochs_in = snapshot_epoch.0.checked_sub(start_epoch.0)?;
    let epoch_len = u64::from(epoch_length_slots);
    let offset = epochs_in.checked_mul(epoch_len)?;
    let epoch_start_slot = start_slot.0.checked_add(offset)?;
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        epoch_start_slot,
        vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(epoch_start_slot),
            start_epoch: snapshot_epoch,
            slot_length_ms: 1_000,
            epoch_length_slots,
            safe_zone_slots: epoch_length_slots,
        }],
    )
    .ok()
}

/// The NATIVE FirstRun bootstrap: read the four native components, route them
/// through the UNCHANGED S1a/S1b/S1c chain, and persist atomically. Returns the
/// `MithrilBootstrapOutput` on success (durable artifacts persisted; the anchor
/// lineage discoverable via `load_recovered_anchor_point`).
///
/// `*_view_builder` builds the leadership `LedgerView` from the assembled
/// consensus inputs (the cold-start composition never consumes it, but it is
/// built faithfully — no placeholder). The caller supplies the persistent
/// `ChainDb` + `SnapshotStore` (the same store on this binary) + the `WalStore`
/// — they are REUSED, never re-opened.
#[allow(clippy::too_many_arguments)]
pub fn native_first_run_bootstrap<D, S, FView>(
    manifest_bytes: &[u8],
    state_cbor: &[u8],
    tables_bytes: &[u8],
    genesis_facts: NativeGenesisFacts,
    expected_network: Option<(u32, ade_types::Hash32)>,
    snapshot_dir: &std::path::Path,
    chaindb: &D,
    snapshot_store: &S,
    wal: &mut dyn ade_ledger::wal::WalStore,
    view_builder: FView,
) -> Result<MithrilBootstrapOutput, NativeFirstRunError>
where
    D: ade_runtime::chaindb::ChainDb,
    S: ade_runtime::chaindb::SnapshotStore + ?Sized,
    FView: FnOnce(&ade_runtime::consensus_inputs::LiveConsensusInputsCanonical) -> Box<dyn LedgerView>,
{
    // 1. Manifest -> the verified binding (the provenance authority:
    //    certified_point, network_magic, genesis_hash, immutable_range).
    let import = import_mithril_manifest_from_bytes(manifest_bytes)
        .map_err(|e| NativeFirstRunError::ManifestImport(format!("{e:?}")))?;
    let report = &import.report;
    let binding = VerifiedManifestBinding {
        network_magic: report.network_magic,
        genesis_hash: report.genesis_hash.clone(),
        certified_point: report.certified_point.clone(),
        immutable_range: report.immutable_range,
    };

    // 2. The verified manifest MUST bind to the selected network profile (--network): the
    //    committed network magic + Shelley genesis hash must equal the manifest's, else the
    //    snapshot is for a different network than selected -> terminal. `None` skips this on the
    //    advanced custom path (where --shelley-genesis-path supplies the constants directly).
    if let Some((expected_magic, expected_genesis_hash)) = expected_network.as_ref() {
        if binding.network_magic != *expected_magic
            || binding.genesis_hash != *expected_genesis_hash
        {
            return Err(NativeFirstRunError::NetworkMismatch(format!(
                "manifest network (magic {}, genesis hash {:?}) does not match the selected --network profile (magic {}, genesis hash {:?})",
                binding.network_magic, binding.genesis_hash, expected_magic, expected_genesis_hash
            )));
        }
    }

    // 3. The network Shelley boundary (closed per-magic constant) + the
    //    certified slot's epoch -> the `manifest_epoch` S1a cross-checks.
    let boundary = shelley_boundary_for_magic(binding.network_magic).ok_or(
        NativeFirstRunError::UnknownNetworkBoundary {
            network_magic: binding.network_magic,
        },
    )?;
    let manifest_epoch = epoch_for_certified_slot(
        binding.certified_point.slot,
        boundary,
        genesis_facts.epoch_length_slots,
    )
    .ok_or(NativeFirstRunError::EpochGeometry {
        certified_slot: binding.certified_point.slot.0,
    })?;

    // 4. S1a: decode the non-UTxO authority from the `state` file, bound to the
    //    manifest point / epoch / network. A wrong epoch / network / era /
    //    missing field is fail-closed inside the decoder.
    let point = SeedPoint {
        slot: binding.certified_point.slot,
        block_hash: binding.certified_point.block_hash.clone(),
    };
    let (s1a, s1a_commitment) = decode_native_nonutxo_state(
        state_cbor,
        point,
        manifest_epoch.0,
        binding.network_magic,
    )
    .map_err(|e: NativeNonUtxoError| NativeFirstRunError::NonUtxoDecode(format!("{e:?}")))?;

    // 5. S1c: materialize the Stage-2 `tables` into the authoritative UTxOState
    //    (Conway-bound, whole file — no sample cap on the live route).
    let utxo = materialize_tables_to_utxo(tables_bytes, CONWAY_ERA_INDEX, None)
        .map_err(|e: TxOutMaterializeError| NativeFirstRunError::UtxoMaterialize(format!("{e:?}")))?;

    // 5b. S2 (DC-MITHRIL-08): when the decoded cert-state carries delegations (the EVIEW
    //     package), build the live reduced checkpoint INLINE from the materialized UTxO
    //     BEFORE it is consumed by the bootstrap, so a Mithril-started node is boundary-usable
    //     (ECA derives + promotes the next-epoch authority from it). Sealed at the certified
    //     slot; gated on delegations so a no-EVIEW snapshot is byte-identical. Fail-closed: a
    //     build failure aborts before authority visibility.
    if !s1a.cert_state.delegation.delegations.is_empty() {
        use ade_ledger::reduced_utxo::{reduce_txout, ReducedStakeRef};
        let mut reduced: std::collections::BTreeMap<
            ade_types::tx::TxIn,
            (ade_types::tx::Coin, ReducedStakeRef),
        > = std::collections::BTreeMap::new();
        for (txin, txout) in utxo.utxos.iter() {
            reduced.insert(txin.clone(), reduce_txout(txout));
        }
        let checkpoint = ade_runtime::chaindb::ReducedUtxoCheckpoint::open(
            &snapshot_dir.join("reduced-checkpoint.redb"),
        )
        .map_err(|e| NativeFirstRunError::ReducedCheckpoint(format!("{e:?}")))?;
        checkpoint
            .build_from(&reduced)
            .map_err(|e| NativeFirstRunError::ReducedCheckpoint(format!("{e:?}")))?;
        checkpoint
            .seal_bootstrap(binding.certified_point.slot)
            .map_err(|e| NativeFirstRunError::ReducedCheckpoint(format!("{e:?}")))?;
    }

    // 6. Build the single-era Conway schedule anchored at the snapshot epoch's
    //    absolute geometry (the network boundary + the genesis epoch length).
    let era_schedule = build_native_schedule(s1a.epoch, boundary, genesis_facts.epoch_length_slots)
        .ok_or(NativeFirstRunError::EpochGeometry {
            certified_slot: binding.certified_point.slot.0,
        })?;

    // 7. Build the leadership view from the would-be assembled inputs. We do a
    //    pre-assembly to obtain the canonical inputs the view zips; the actual
    //    bootstrap re-runs the assembly (point coherence is deterministic, so
    //    the pre-assembly either also fails-closed identically or yields the
    //    same inputs). The cold-start composition never consumes the view.
    let pre = ade_runtime::mithril_native_assembly::assemble_native_mithril_seed(
        &s1a,
        s1a_commitment.clone(),
        utxo.clone(),
        &binding,
        &genesis_facts.constants,
        &era_schedule,
    )
    .map_err(|e| NativeFirstRunError::NativeBootstrap(format!("{e:?}")))?;
    let view = view_builder(&pre.consensus_inputs);

    // 8. S1b: assemble + atomically persist through the single closed
    //    composition. The WAL commit-point inside is the SOLE discovery gate;
    //    any failure before it leaves NO bootable partial authority state.
    bootstrap_from_native_mithril_snapshot(
        &s1a,
        s1a_commitment,
        utxo,
        &binding,
        &genesis_facts.constants,
        manifest_bytes,
        chaindb,
        snapshot_store,
        wal,
        &era_schedule,
        view.as_ref(),
    )
    .map_err(|e: NativeMithrilBootstrapError| {
        NativeFirstRunError::NativeBootstrap(format!("{e:?}"))
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn shelley_genesis_active_slots_coeff_decimal_to_rational() {
        // The real preprod shelley genesis carries activeSlotsCoeff = 0.05; it
        // must convert to the exact ActiveSlotsCoeff { 1, 20 } rational with NO
        // float arithmetic (0.05 = 5/100 reduced by gcd 5).
        assert_eq!(
            decimal_text_to_rational("0.05"),
            Some(ActiveSlotsCoeff { numer: 1, denom: 20 })
        );
        assert_eq!(
            decimal_text_to_rational("0.5"),
            Some(ActiveSlotsCoeff { numer: 1, denom: 2 })
        );
        assert_eq!(
            decimal_text_to_rational("1"),
            Some(ActiveSlotsCoeff { numer: 1, denom: 1 })
        );
        // Fail-closed on non-decimal / non-positive / exponent forms.
        assert_eq!(decimal_text_to_rational("0"), None);
        assert_eq!(decimal_text_to_rational("0.0"), None);
        assert_eq!(decimal_text_to_rational("5e-2"), None);
        assert_eq!(decimal_text_to_rational("-0.05"), None);
        assert_eq!(decimal_text_to_rational(""), None);
        assert_eq!(decimal_text_to_rational("abc"), None);
    }

    #[test]
    fn preprod_snapshot_epoch_window_contains_manifest_point() {
        // The real preprod snapshot tip slot is 126_400_064 (epoch 296). The
        // native schedule (preprod Shelley boundary epoch 4 / slot 86_400 +
        // genesis epoch length 432_000) must yield epoch 296 for that slot and
        // a window that CONTAINS it — the load-bearing S1d cross-check.
        let boundary = shelley_boundary_for_magic(PREPROD_NETWORK_MAGIC).expect("preprod boundary");
        let epoch_length = 432_000u32;
        let certified_slot = SlotNo(126_400_064);
        let epoch = epoch_for_certified_slot(certified_slot, boundary, epoch_length)
            .expect("epoch resolves");
        assert_eq!(epoch, EpochNo(296), "the snapshot tip slot is epoch 296");

        let sched = build_native_schedule(epoch, boundary, epoch_length).expect("schedule");
        // locate(certified_slot).epoch == the snapshot epoch.
        let loc = sched.locate(certified_slot).expect("locate");
        assert_eq!(loc.epoch, EpochNo(296));
        // The era summary's window contains the certified slot.
        let era = &sched.eras()[0];
        let win_start = era.start_slot.0;
        let win_end = win_start + u64::from(era.epoch_length_slots) - 1;
        assert!(
            win_start <= certified_slot.0 && certified_slot.0 <= win_end,
            "manifest point {} must fall inside the derived window [{}, {}]",
            certified_slot.0,
            win_start,
            win_end
        );
        assert_eq!(win_start, 126_230_400, "epoch 296 absolute start slot");
        assert_eq!(win_end, 126_662_399, "epoch 296 absolute end slot");
    }

    #[test]
    fn unknown_network_magic_has_no_boundary() {
        // An unknown magic has no closed Shelley boundary -> terminal upstream
        // (never a guessed boundary).
        assert!(shelley_boundary_for_magic(999_999).is_none());
    }

    #[test]
    fn parse_native_shelley_genesis_extracts_constants() {
        // A minimal real-shaped shelley genesis -> the native facts.
        let json = r#"{
            "maxLovelaceSupply": 45000000000000000,
            "activeSlotsCoeff": 0.05,
            "epochLength": 432000,
            "slotLength": 1,
            "systemStart": "2022-06-01T00:00:00Z"
        }"#;
        let facts = parse_native_shelley_genesis(json.as_bytes()).expect("parse");
        assert_eq!(facts.constants.max_lovelace_supply, 45_000_000_000_000_000);
        assert_eq!(
            facts.constants.active_slots_coeff,
            ActiveSlotsCoeff { numer: 1, denom: 20 }
        );
        assert_eq!(facts.epoch_length_slots, 432_000);

        // Missing field -> fail closed.
        let missing = r#"{ "activeSlotsCoeff": 0.05, "epochLength": 432000 }"#;
        assert_eq!(
            parse_native_shelley_genesis(missing.as_bytes()),
            Err(NativeGenesisParseError::MissingField {
                name: "maxLovelaceSupply"
            })
        );
        // Wrong type (stringly int) -> fail closed.
        let stringly = r#"{ "maxLovelaceSupply": "45000000000000000", "activeSlotsCoeff": 0.05, "epochLength": 432000 }"#;
        assert_eq!(
            parse_native_shelley_genesis(stringly.as_bytes()),
            Err(NativeGenesisParseError::MalformedType {
                name: "maxLovelaceSupply"
            })
        );
    }
}

fn require_u64(
    obj: &serde_json::Map<String, serde_json::Value>,
    name: &'static str,
) -> Result<u64, NativeGenesisParseError> {
    let v = obj
        .get(name)
        .ok_or(NativeGenesisParseError::MissingField { name })?;
    if !v.is_number() {
        return Err(NativeGenesisParseError::MalformedType { name });
    }
    v.as_u64()
        .ok_or(NativeGenesisParseError::MalformedValue { name })
}
