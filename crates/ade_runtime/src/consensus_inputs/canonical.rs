// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN canonical-form + Blake2b-256 fingerprint of operator
//! consensus-inputs bundles (PHASE4-N-M-C S1b).
//!
//! Lifts a [`LiveConsensusInputsRaw`] (the typed-validated form
//! C1a produces) into [`LiveConsensusInputsCanonical`]: same
//! field set, plus a fingerprint computed over a canonical CBOR
//! encoding of every field in declared order.
//!
//! Rules:
//!   - DC-CONS-IN-02 — canonical fingerprint determinism + the
//!     load-bearing handle for every admission JSONL block-event
//!     (DC-ADMIT-10 wires the consumer side in C2).
//!
//! Companion to importer.rs: `import_live_consensus_inputs` (the
//! SOLE Canonical-returning authority per CN-CONS-IN-01) is the
//! one-line composition `raw -> canonical_from_raw` exposed
//! beside the raw importer.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use ade_codec::cbor::{
    canonical_width, write_array_header, write_bytes_canonical, write_map_header,
    write_text_canonical, write_uint_canonical, ContainerEncoding,
};
use ade_core::consensus::praos_state::Nonce;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_crypto::blake2b::blake2b_256;
use ade_ledger::pparams::ProtocolParameters;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

use super::importer::{
    import_live_consensus_inputs_raw_from_bytes, LiveConsensusInputsImportError,
    LiveConsensusInputsRaw, PoolEntry,
};
use super::protocol_params::{parse_protocol_parameters_json, ProtocolParamsParseError};

/// Canonical form of an operator consensus-inputs bundle —
/// identical field shape to [`LiveConsensusInputsRaw`], plus a
/// Blake2b-256 fingerprint computed over a canonical CBOR
/// encoding of every field in declared order.
///
/// The `fingerprint` is the load-bearing handle every admission
/// JSONL block-event references (DC-ADMIT-10, wired in C2). Two
/// imports of the same JSON bytes MUST yield byte-identical
/// fingerprints (DC-CONS-IN-02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveConsensusInputsCanonical {
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub era: CardanoEra,
    pub epoch_no: EpochNo,
    pub epoch_start_slot: SlotNo,
    pub epoch_end_slot: SlotNo,
    pub active_slots_coeff: ActiveSlotsCoeff,
    pub epoch_nonce: Nonce,
    pub pool_distribution: BTreeMap<Hash28, PoolEntry>,
    pub pool_vrf_keyhashes: BTreeMap<Hash28, Hash32>,
    pub protocol_params_hash: Hash32,
    pub source_cardano_node_version: String,
    pub source_query_command: String,
    pub source_tip_hash: Hash32,
    pub source_tip_slot: SlotNo,
    pub fingerprint: Hash32,
    /// Oracle preimage for `protocol_params_hash` (PHASE4-N-F-G-A S2a), carried
    /// OUTSIDE the canonical CBOR fingerprint: the fingerprint already commits to
    /// `protocol_params_hash` (index 10), so this is that hash's preimage, not a
    /// new fingerprinted field — it does not alter the bundle fingerprint.
    /// Optional at structural import; required + hash-bound at the forge-install
    /// (`require_forge_current_pparams`).
    pub protocol_params_json: Option<String>,
}

impl LiveConsensusInputsCanonical {
    /// Venue epoch length in slots, derived from the cardano-cli-reported epoch
    /// window (`epoch_end_slot - epoch_start_slot + 1`). `None` when the window
    /// is degenerate (`end < start`) or overflows `u32` (no real venue epoch
    /// does), so callers fail closed rather than persist a defaulted length.
    /// WARMSTART-ERA-SCHEDULE-VENUE (DC-CINPUT-05).
    pub fn epoch_length_slots(&self) -> Option<u32> {
        let len = self
            .epoch_end_slot
            .0
            .checked_sub(self.epoch_start_slot.0)?
            .checked_add(1)?;
        u32::try_from(len).ok().filter(|&l| l > 0)
    }
}

/// Lift a validated raw bundle into the canonical form, computing
/// the canonical-CBOR fingerprint at lift-time. Deterministic in
/// the raw bundle.
pub fn canonical_from_raw(raw: LiveConsensusInputsRaw) -> LiveConsensusInputsCanonical {
    let encoded = encode_canonical_cbor(&raw);
    let fingerprint = blake2b_256(&encoded);
    LiveConsensusInputsCanonical {
        network_magic: raw.network_magic,
        genesis_hash: raw.genesis_hash,
        era: raw.era,
        epoch_no: raw.epoch_no,
        epoch_start_slot: raw.epoch_start_slot,
        epoch_end_slot: raw.epoch_end_slot,
        active_slots_coeff: raw.active_slots_coeff,
        epoch_nonce: raw.epoch_nonce,
        pool_distribution: raw.pool_distribution,
        pool_vrf_keyhashes: raw.pool_vrf_keyhashes,
        protocol_params_hash: raw.protocol_params_hash,
        source_cardano_node_version: raw.source_cardano_node_version,
        source_query_command: raw.source_query_command,
        source_tip_hash: raw.source_tip_hash,
        source_tip_slot: raw.source_tip_slot,
        fingerprint,
        protocol_params_json: raw.protocol_params_json,
    }
}

/// SOLE Canonical-returning authority — CN-CONS-IN-01 (full form).
/// File variant: reads the operator bundle from disk, runs the
/// C1a importer, lifts to canonical form.
pub fn import_live_consensus_inputs(
    path: &Path,
) -> Result<LiveConsensusInputsCanonical, LiveConsensusInputsImportError> {
    let bytes = fs::read(path).map_err(|e| LiveConsensusInputsImportError::Io(e.kind()))?;
    import_live_consensus_inputs_from_bytes(&bytes)
}

/// In-memory sibling of [`import_live_consensus_inputs`].
pub fn import_live_consensus_inputs_from_bytes(
    bytes: &[u8],
) -> Result<LiveConsensusInputsCanonical, LiveConsensusInputsImportError> {
    let raw = import_live_consensus_inputs_raw_from_bytes(bytes)?;
    Ok(canonical_from_raw(raw))
}

/// Closed error for the forge-current-pparams install (PHASE4-N-F-G-A S2a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeCurrentPParamsError {
    /// The bundle carries no `protocol_params_json` preimage. Forbidden on the
    /// forge-current-pparams install path: fail closed (no default substitution).
    PreimageAbsent,
    /// `blake2b_256(protocol_params_json) != protocol_params_hash`. The carried
    /// preimage does not match the fingerprinted oracle commitment. Fail closed.
    BindMismatch,
    /// The preimage hash-bound correctly but did not parse into a canonical
    /// `ProtocolParameters`.
    Parse(ProtocolParamsParseError),
}

impl LiveConsensusInputsCanonical {
    /// Produce the **current** `ProtocolParameters` for a forge-fidelity recovered
    /// ledger, sourced from the oracle preimage carried in this bundle.
    ///
    /// Mandatory + fail-closed (PHASE4-N-F-G-A S2a, CE-G-A-2a):
    /// 1. `protocol_params_json` MUST be present (else `PreimageAbsent`) — the
    ///    forge-current-pparams install never substitutes a default.
    /// 2. `blake2b_256(protocol_params_json)` MUST equal the fingerprinted
    ///    `protocol_params_hash` (else `BindMismatch`) — bound to the already-
    ///    committed oracle hash; no second commitment is invented.
    /// 3. The preimage parses into the modeled `ProtocolParameters`
    ///    (`network_id` derived from this bundle's `network_magic`).
    pub fn require_forge_current_pparams(
        &self,
    ) -> Result<ProtocolParameters, ForgeCurrentPParamsError> {
        let preimage = self
            .protocol_params_json
            .as_ref()
            .ok_or(ForgeCurrentPParamsError::PreimageAbsent)?;
        let digest = blake2b_256(preimage.as_bytes());
        if digest != self.protocol_params_hash {
            return Err(ForgeCurrentPParamsError::BindMismatch);
        }
        parse_protocol_parameters_json(preimage.as_bytes(), self.network_magic)
            .map_err(ForgeCurrentPParamsError::Parse)
    }
}

/// Canonical CBOR encoding of the 15 raw fields (the fingerprint
/// itself is NOT folded back into the input). Encoded as a fixed
/// 15-entry CBOR map keyed by declared-order index. BTreeMap
/// values produce canonical key-ordering automatically.
///
/// Field index assignment (frozen — adding a field requires a
/// new index AND bumping the canonical-form discriminator if a
/// future cluster reshapes the bundle):
///   0  network_magic                 uint
///   1  genesis_hash                  bytes(32)
///   2  era                           uint (CardanoEra as_u8)
///   3  epoch_no                      uint
///   4  epoch_start_slot              uint
///   5  epoch_end_slot                uint
///   6  active_slots_coeff            array [numer, denom]
///   7  epoch_nonce                   bytes(32)
///   8  pool_distribution             map { bytes(28) -> uint }
///   9  pool_vrf_keyhashes            map { bytes(28) -> bytes(32) }
///   10 protocol_params_hash          bytes(32)
///   11 source_cardano_node_version   text
///   12 source_query_command          text
///   13 source_tip_hash               bytes(32)
///   14 source_tip_slot               uint
fn encode_canonical_cbor(raw: &LiveConsensusInputsRaw) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    write_map_header(&mut buf, ContainerEncoding::Definite(15, canonical_width(15)));

    write_uint_canonical(&mut buf, 0);
    write_uint_canonical(&mut buf, raw.network_magic as u64);

    write_uint_canonical(&mut buf, 1);
    write_bytes_canonical(&mut buf, &raw.genesis_hash.0);

    write_uint_canonical(&mut buf, 2);
    write_uint_canonical(&mut buf, raw.era.as_u8() as u64);

    write_uint_canonical(&mut buf, 3);
    write_uint_canonical(&mut buf, raw.epoch_no.0);

    write_uint_canonical(&mut buf, 4);
    write_uint_canonical(&mut buf, raw.epoch_start_slot.0);

    write_uint_canonical(&mut buf, 5);
    write_uint_canonical(&mut buf, raw.epoch_end_slot.0);

    write_uint_canonical(&mut buf, 6);
    write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    write_uint_canonical(&mut buf, raw.active_slots_coeff.numer as u64);
    write_uint_canonical(&mut buf, raw.active_slots_coeff.denom as u64);

    write_uint_canonical(&mut buf, 7);
    write_bytes_canonical(&mut buf, &raw.epoch_nonce.0 .0);

    write_uint_canonical(&mut buf, 8);
    let n_pd = raw.pool_distribution.len() as u64;
    write_map_header(&mut buf, ContainerEncoding::Definite(n_pd, canonical_width(n_pd)));
    for (k, v) in &raw.pool_distribution {
        write_bytes_canonical(&mut buf, &k.0);
        write_uint_canonical(&mut buf, v.active_stake);
    }

    write_uint_canonical(&mut buf, 9);
    let n_vrf = raw.pool_vrf_keyhashes.len() as u64;
    write_map_header(&mut buf, ContainerEncoding::Definite(n_vrf, canonical_width(n_vrf)));
    for (k, v) in &raw.pool_vrf_keyhashes {
        write_bytes_canonical(&mut buf, &k.0);
        write_bytes_canonical(&mut buf, &v.0);
    }

    write_uint_canonical(&mut buf, 10);
    write_bytes_canonical(&mut buf, &raw.protocol_params_hash.0);

    write_uint_canonical(&mut buf, 11);
    write_text_canonical(&mut buf, &raw.source_cardano_node_version);

    write_uint_canonical(&mut buf, 12);
    write_text_canonical(&mut buf, &raw.source_query_command);

    write_uint_canonical(&mut buf, 13);
    write_bytes_canonical(&mut buf, &raw.source_tip_hash.0);

    write_uint_canonical(&mut buf, 14);
    write_uint_canonical(&mut buf, raw.source_tip_slot.0);

    buf
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"{
        "network_magic": 1,
        "genesis_hash_hex": "00000000000000000000000000000000000000000000000000000000000000aa",
        "era": "conway",
        "epoch_no": 200,
        "epoch_start_slot": 86400000,
        "epoch_end_slot": 86832000,
        "active_slots_coeff": {"numer": 1, "denom": 20},
        "epoch_nonce_hex": "00000000000000000000000000000000000000000000000000000000000000bb",
        "pool_distribution": {
            "00000000000000000000000000000000000000000000000000000001": {"active_stake": 123},
            "00000000000000000000000000000000000000000000000000000002": {"active_stake": 456}
        },
        "pool_vrf_keyhashes": {
            "00000000000000000000000000000000000000000000000000000001": "00000000000000000000000000000000000000000000000000000000000000cc",
            "00000000000000000000000000000000000000000000000000000002": "00000000000000000000000000000000000000000000000000000000000000dd"
        },
        "protocol_params_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ee",
        "source_cardano_node_version": "cardano-node 11.0.1",
        "source_query_command": "cardano-cli conway query stake-distribution --testnet-magic 1",
        "source_tip_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ff",
        "source_tip_slot": 86400500
    }"#;

    fn replace(json: &str, needle: &str, repl: &str) -> String {
        let mut s = json.to_string();
        let pos = s.find(needle).expect("needle in MINIMAL");
        s.replace_range(pos..pos + needle.len(), repl);
        s
    }

    #[test]
    fn import_round_trip_yields_canonical_form_with_fingerprint() {
        let c = import_live_consensus_inputs_from_bytes(MINIMAL.as_bytes()).expect("import ok");
        assert_eq!(c.network_magic, 1);
        assert_eq!(c.era, CardanoEra::Conway);
        assert_eq!(c.pool_distribution.len(), 2);
        // Fingerprint is non-zero (the all-zero hash would be a
        // strong smell that nothing was encoded).
        assert_ne!(c.fingerprint, Hash32([0u8; 32]));
    }

    #[test]
    fn fingerprint_is_deterministic_across_repeated_imports() {
        let a = import_live_consensus_inputs_from_bytes(MINIMAL.as_bytes()).unwrap();
        let b = import_live_consensus_inputs_from_bytes(MINIMAL.as_bytes()).unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
    }

    #[test]
    fn fingerprint_changes_when_any_canonical_input_changes() {
        let base = import_live_consensus_inputs_from_bytes(MINIMAL.as_bytes()).unwrap();
        let perturbed_inputs: Vec<(&str, &str, &str)> = vec![
            ("network_magic", "\"network_magic\": 1,", "\"network_magic\": 2,"),
            ("epoch_no", "\"epoch_no\": 200,", "\"epoch_no\": 201,"),
            (
                "epoch_start_slot",
                "\"epoch_start_slot\": 86400000,",
                "\"epoch_start_slot\": 86400001,",
            ),
            (
                "epoch_end_slot",
                "\"epoch_end_slot\": 86832000,",
                "\"epoch_end_slot\": 86832001,",
            ),
            (
                "asc_numer",
                "\"active_slots_coeff\": {\"numer\": 1, \"denom\": 20}",
                "\"active_slots_coeff\": {\"numer\": 2, \"denom\": 20}",
            ),
            (
                "asc_denom",
                "\"active_slots_coeff\": {\"numer\": 1, \"denom\": 20}",
                "\"active_slots_coeff\": {\"numer\": 1, \"denom\": 21}",
            ),
            (
                "epoch_nonce",
                "\"epoch_nonce_hex\": \"00000000000000000000000000000000000000000000000000000000000000bb\"",
                "\"epoch_nonce_hex\": \"00000000000000000000000000000000000000000000000000000000000000b1\"",
            ),
            (
                "genesis_hash",
                "\"genesis_hash_hex\": \"00000000000000000000000000000000000000000000000000000000000000aa\"",
                "\"genesis_hash_hex\": \"00000000000000000000000000000000000000000000000000000000000000a1\"",
            ),
            (
                "protocol_params_hash",
                "\"protocol_params_hash_hex\": \"00000000000000000000000000000000000000000000000000000000000000ee\"",
                "\"protocol_params_hash_hex\": \"00000000000000000000000000000000000000000000000000000000000000e1\"",
            ),
            (
                "source_tip_hash",
                "\"source_tip_hash_hex\": \"00000000000000000000000000000000000000000000000000000000000000ff\"",
                "\"source_tip_hash_hex\": \"00000000000000000000000000000000000000000000000000000000000000f1\"",
            ),
            ("source_tip_slot", "\"source_tip_slot\": 86400500", "\"source_tip_slot\": 86400600"),
            (
                "source_cardano_node_version",
                "\"source_cardano_node_version\": \"cardano-node 11.0.1\"",
                "\"source_cardano_node_version\": \"cardano-node 11.0.2\"",
            ),
            (
                "source_query_command",
                "\"source_query_command\": \"cardano-cli conway query stake-distribution --testnet-magic 1\"",
                "\"source_query_command\": \"cardano-cli conway query stake-distribution --testnet-magic 2\"",
            ),
            (
                "pool_distribution_stake",
                "\"00000000000000000000000000000000000000000000000000000001\": {\"active_stake\": 123}",
                "\"00000000000000000000000000000000000000000000000000000001\": {\"active_stake\": 124}",
            ),
            (
                "pool_vrf_value",
                "\"00000000000000000000000000000000000000000000000000000001\": \"00000000000000000000000000000000000000000000000000000000000000cc\"",
                "\"00000000000000000000000000000000000000000000000000000001\": \"00000000000000000000000000000000000000000000000000000000000000c1\"",
            ),
        ];
        for (label, needle, replacement) in perturbed_inputs {
            let bad = replace(MINIMAL, needle, replacement);
            let p = import_live_consensus_inputs_from_bytes(bad.as_bytes()).expect("import ok");
            assert_ne!(
                p.fingerprint, base.fingerprint,
                "perturbing {label} did not change the fingerprint"
            );
        }
    }

    #[test]
    fn canonical_field_count_is_fifteen() {
        // Encode header must be a map(15). A drift here means the
        // fingerprint surface silently grew/shrank.
        let c = import_live_consensus_inputs_from_bytes(MINIMAL.as_bytes()).unwrap();
        // Reconstruct the raw bytes that produced the fingerprint
        // (encode_canonical_cbor is a pure function of the raw
        // form) and check the leading byte.
        let raw = LiveConsensusInputsRaw {
            network_magic: c.network_magic,
            genesis_hash: c.genesis_hash.clone(),
            era: c.era,
            epoch_no: c.epoch_no,
            epoch_start_slot: c.epoch_start_slot,
            epoch_end_slot: c.epoch_end_slot,
            active_slots_coeff: c.active_slots_coeff,
            epoch_nonce: c.epoch_nonce.clone(),
            pool_distribution: c.pool_distribution.clone(),
            pool_vrf_keyhashes: c.pool_vrf_keyhashes.clone(),
            protocol_params_hash: c.protocol_params_hash.clone(),
            source_cardano_node_version: c.source_cardano_node_version.clone(),
            source_query_command: c.source_query_command.clone(),
            source_tip_hash: c.source_tip_hash.clone(),
            source_tip_slot: c.source_tip_slot,
            protocol_params_json: c.protocol_params_json.clone(),
        };
        let bytes = encode_canonical_cbor(&raw);
        // CBOR map(15) = major type 5 (0xa_) with argument 15
        // → leading byte 0xaf.
        assert_eq!(bytes[0], 0xaf);
    }

    #[test]
    fn fingerprint_is_blake2b_256_of_encode_canonical_cbor() {
        // Behavioural lock: the fingerprint MUST equal
        // blake2b_256(encode_canonical_cbor(raw)). A drift here
        // means the encoding rule diverged from the hashing rule.
        let bytes = MINIMAL.as_bytes();
        let raw = import_live_consensus_inputs_raw_from_bytes(bytes).unwrap();
        let expected = blake2b_256(&encode_canonical_cbor(&raw));
        let canonical = canonical_from_raw(raw);
        assert_eq!(canonical.fingerprint, expected);
    }

    // ---- S2a: require_forge_current_pparams (bind + parse) ----

    /// A minimal-but-valid cardano-cli protocol-parameters JSON with a non-default
    /// major (9) + rational floats, to prove the bind + exact parse on a value the
    /// `ProtocolParameters::default()` (major 2) would not produce.
    const CLI_PP_MAJOR_9: &str = r#"{"collateralPercentage":150,"maxBlockBodySize":90112,"maxBlockHeaderSize":1100,"maxTxExecutionUnits":{"memory":14000000,"steps":10000000000},"maxTxSize":16384,"minPoolCost":170000000,"monetaryExpansion":3.0e-3,"poolPledgeInfluence":0.3,"poolRetireMaxEpoch":18,"protocolVersion":{"major":9,"minor":0},"stakeAddressDeposit":2000000,"stakePoolDeposit":500000000,"stakePoolTargetNum":500,"treasuryCut":0.2,"txFeeFixed":155381,"txFeePerByte":44}"#;

    fn bound_bundle(pp_json: &str) -> LiveConsensusInputsCanonical {
        let mut stake = BTreeMap::new();
        stake.insert(Hash28([0x01; 28]), 1_000u64);
        let mut vrfs = BTreeMap::new();
        vrfs.insert(Hash28([0x01; 28]), Hash32([0x07; 32]));
        let mut b = crate::seed_consensus_merge::test_canonical_inputs(EpochNo(0), stake, vrfs);
        b.protocol_params_hash = blake2b_256(pp_json.as_bytes());
        b.protocol_params_json = Some(pp_json.to_string());
        b
    }

    #[test]
    fn require_forge_current_pparams_binds_and_parses() {
        let pp = bound_bundle(CLI_PP_MAJOR_9)
            .require_forge_current_pparams()
            .expect("preimage present + binds + parses");
        assert_eq!(pp.protocol_major, 9);
        assert_eq!(pp.protocol_minor, 0);
        assert_eq!(pp.monetary_expansion, ProtocolParameters::default().monetary_expansion);
        assert_eq!(pp.treasury_growth, ProtocolParameters::default().treasury_growth);
    }

    #[test]
    fn require_forge_current_pparams_absent_preimage_fails_closed() {
        let mut b = bound_bundle(CLI_PP_MAJOR_9);
        b.protocol_params_json = None;
        assert_eq!(
            b.require_forge_current_pparams().unwrap_err(),
            ForgeCurrentPParamsError::PreimageAbsent
        );
    }

    #[test]
    fn require_forge_current_pparams_tampered_preimage_fails_bind() {
        let mut b = bound_bundle(CLI_PP_MAJOR_9);
        // Keep the committed hash; tamper the preimage => no longer hash-binds.
        b.protocol_params_json = Some(CLI_PP_MAJOR_9.replace("\"major\":9", "\"major\":10"));
        assert_eq!(
            b.require_forge_current_pparams().unwrap_err(),
            ForgeCurrentPParamsError::BindMismatch
        );
    }
}
