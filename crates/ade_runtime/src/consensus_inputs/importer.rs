// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN cardano-cli operator-bundle importer (PHASE4-N-M-C S1a).
//!
//! Converts a structurally-parsed [`RawConsensusInputs`] into the
//! typed-validated [`LiveConsensusInputsRaw`]. Validation here is
//! exhaustive — every field is checked, every hash width is
//! verified, every `Option` field is rejected if absent. There is
//! no partial-import fallback (¬P-C4).
//!
//! Rules:
//!   - CN-CONS-IN-01 — sole authority `import_live_consensus_inputs_raw_from_bytes`
//!     converts JSON bytes → [`LiveConsensusInputsRaw`]. C1b
//!     extends this into the canonical-form authority that returns
//!     `LiveConsensusInputsCanonical`.
//!   - DC-CONS-IN-01 — closed [`LiveConsensusInputsImportError`]
//!     sum; no `Option` field defaults; missing fields are fatal
//!     at import.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use ade_core::consensus::praos_state::Nonce;
use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

use super::json::{parse_consensus_inputs_json, RawConsensusInputs, RawPoolEntry};

/// Per-pool entry on the typed-validated form. The fingerprint
/// added in C1b extends this; for C1a we only carry `active_stake`
/// (paired with the VRF keyhash via the `pool_vrf_keyhashes` map
/// on the parent struct).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolEntry {
    pub active_stake: u64,
}

/// Typed-validated form of an operator consensus-inputs bundle.
///
/// All fields are required; every hash field has been checked to
/// be a valid 28- or 32-byte hex string. Pool-distribution and
/// pool-VRF-keyhash maps share an identical key-set. The
/// canonical-form fingerprint is added in C1b
/// (`LiveConsensusInputsCanonical`); this struct carries the
/// same field shape minus that fingerprint so C1b can layer on
/// the canonical CBOR encoding without re-running validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveConsensusInputsRaw {
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
    /// Oracle preimage for `protocol_params_hash` (PHASE4-N-F-G-A S2a). Optional
    /// at structural import; required + hash-bound at the forge-install site (see
    /// `canonical::LiveConsensusInputsCanonical::require_forge_current_pparams`).
    pub protocol_params_json: Option<String>,
}

/// Closed import-error sum (DC-CONS-IN-01).
#[derive(Debug)]
pub enum LiveConsensusInputsImportError {
    /// IO failure reading the bundle file.
    Io(io::ErrorKind),
    /// JSON parse failure (shape didn't match
    /// `RawConsensusInputs` — missing required field, unknown
    /// field, or type mismatch).
    Json(serde_json::Error),
    /// A field had a structurally-valid JSON value but a value
    /// that the importer refuses (e.g. ASC denominator of 0).
    BadField { field: &'static str },
    /// A field that the typed form makes mandatory was missing.
    /// (At C1a serde+`deny_unknown_fields` already catches most
    /// of these; this variant covers internal-consistency
    /// missing-coverage such as a pool appearing in one map but
    /// not the other.)
    MissingField { field: &'static str },
    /// A hex-encoded hash field had wrong length or non-hex
    /// characters.
    BadHashHex { field: &'static str },
    /// `epoch_end_slot < epoch_start_slot`, or the source tip
    /// slot is outside `[start, end]` and the operator didn't
    /// flag the bundle as cross-epoch (not allowed at C — see
    /// ¬P-C2 / DC-ADMIT-11).
    BadEpochWindow { epoch_start: u64, epoch_end: u64 },
    /// A pool-distribution invariant was violated (e.g. the
    /// distribution and VRF-keyhash maps disagree on the
    /// key-set).
    BadPoolDistribution { detail: &'static str },
    /// The bundle declares an era this cluster does not support.
    EraNotSupported { era: String },
}

impl From<serde_json::Error> for LiveConsensusInputsImportError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl core::fmt::Display for LiveConsensusInputsImportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(k) => write!(f, "io: {:?}", k),
            Self::Json(e) => write!(f, "json: {}", e),
            Self::BadField { field } => write!(f, "bad field: {}", field),
            Self::MissingField { field } => write!(f, "missing field: {}", field),
            Self::BadHashHex { field } => write!(f, "bad hash hex: {}", field),
            Self::BadEpochWindow {
                epoch_start,
                epoch_end,
            } => write!(
                f,
                "bad epoch window: start={} end={}",
                epoch_start, epoch_end
            ),
            Self::BadPoolDistribution { detail } => {
                write!(f, "bad pool distribution: {}", detail)
            }
            Self::EraNotSupported { era } => write!(f, "era not supported: {}", era),
        }
    }
}

impl std::error::Error for LiveConsensusInputsImportError {}

/// SOLE pub fn (file variant) — read the operator bundle from
/// disk and import it. CN-CONS-IN-01 (C1a half).
pub fn import_live_consensus_inputs_raw(
    path: &Path,
) -> Result<LiveConsensusInputsRaw, LiveConsensusInputsImportError> {
    let bytes = fs::read(path).map_err(|e| LiveConsensusInputsImportError::Io(e.kind()))?;
    import_live_consensus_inputs_raw_from_bytes(&bytes)
}

/// In-memory variant. Same single-authority guarantee; the file
/// variant is a one-line wrapper.
pub fn import_live_consensus_inputs_raw_from_bytes(
    bytes: &[u8],
) -> Result<LiveConsensusInputsRaw, LiveConsensusInputsImportError> {
    let raw = parse_consensus_inputs_json(bytes)?;
    validate_and_lift(raw)
}

fn validate_and_lift(
    raw: RawConsensusInputs,
) -> Result<LiveConsensusInputsRaw, LiveConsensusInputsImportError> {
    let era = parse_era(&raw.era)?;
    if !matches!(era, CardanoEra::Conway) {
        return Err(LiveConsensusInputsImportError::EraNotSupported {
            era: raw.era.clone(),
        });
    }

    if raw.epoch_end_slot < raw.epoch_start_slot {
        return Err(LiveConsensusInputsImportError::BadEpochWindow {
            epoch_start: raw.epoch_start_slot,
            epoch_end: raw.epoch_end_slot,
        });
    }

    // The source tip MUST lie inside the declared epoch window —
    // an operator who extracted at a tip in epoch E+1 should bump
    // `epoch_no` rather than carry stale window bounds.
    if raw.source_tip_slot < raw.epoch_start_slot || raw.source_tip_slot > raw.epoch_end_slot {
        return Err(LiveConsensusInputsImportError::BadEpochWindow {
            epoch_start: raw.epoch_start_slot,
            epoch_end: raw.epoch_end_slot,
        });
    }

    if raw.active_slots_coeff.denom == 0 {
        return Err(LiveConsensusInputsImportError::BadField {
            field: "active_slots_coeff.denom",
        });
    }
    let asc = ActiveSlotsCoeff {
        numer: raw.active_slots_coeff.numer,
        denom: raw.active_slots_coeff.denom,
    };

    let genesis_hash = parse_hash32(&raw.genesis_hash_hex, "genesis_hash_hex")?;
    let epoch_nonce_hash = parse_hash32(&raw.epoch_nonce_hex, "epoch_nonce_hex")?;
    let protocol_params_hash =
        parse_hash32(&raw.protocol_params_hash_hex, "protocol_params_hash_hex")?;
    let source_tip_hash = parse_hash32(&raw.source_tip_hash_hex, "source_tip_hash_hex")?;
    let epoch_nonce = Nonce(epoch_nonce_hash);

    let pool_distribution = lift_pool_distribution(&raw.pool_distribution)?;
    let pool_vrf_keyhashes = lift_pool_vrf_keyhashes(&raw.pool_vrf_keyhashes)?;

    // Key-set parity: every pool in the distribution must have a
    // VRF keyhash entry, and vice versa.
    if pool_distribution.len() != pool_vrf_keyhashes.len() {
        return Err(LiveConsensusInputsImportError::BadPoolDistribution {
            detail: "pool_distribution and pool_vrf_keyhashes size mismatch",
        });
    }
    for k in pool_distribution.keys() {
        if !pool_vrf_keyhashes.contains_key(k) {
            return Err(LiveConsensusInputsImportError::BadPoolDistribution {
                detail: "pool in pool_distribution missing from pool_vrf_keyhashes",
            });
        }
    }

    Ok(LiveConsensusInputsRaw {
        network_magic: raw.network_magic,
        genesis_hash,
        era,
        epoch_no: EpochNo(raw.epoch_no),
        epoch_start_slot: SlotNo(raw.epoch_start_slot),
        epoch_end_slot: SlotNo(raw.epoch_end_slot),
        active_slots_coeff: asc,
        epoch_nonce,
        pool_distribution,
        pool_vrf_keyhashes,
        protocol_params_hash,
        source_cardano_node_version: raw.source_cardano_node_version,
        source_query_command: raw.source_query_command,
        source_tip_hash,
        source_tip_slot: SlotNo(raw.source_tip_slot),
        protocol_params_json: raw.protocol_params_json,
    })
}

fn parse_era(s: &str) -> Result<CardanoEra, LiveConsensusInputsImportError> {
    match s {
        "byron_ebb" => Ok(CardanoEra::ByronEbb),
        "byron_regular" => Ok(CardanoEra::ByronRegular),
        "shelley" => Ok(CardanoEra::Shelley),
        "allegra" => Ok(CardanoEra::Allegra),
        "mary" => Ok(CardanoEra::Mary),
        "alonzo" => Ok(CardanoEra::Alonzo),
        "babbage" => Ok(CardanoEra::Babbage),
        "conway" => Ok(CardanoEra::Conway),
        other => Err(LiveConsensusInputsImportError::EraNotSupported {
            era: other.to_string(),
        }),
    }
}

fn parse_hash32(
    hex: &str,
    field: &'static str,
) -> Result<Hash32, LiveConsensusInputsImportError> {
    if hex.len() != 64 {
        return Err(LiveConsensusInputsImportError::BadHashHex { field });
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let pair = &hex[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(pair, 16)
            .map_err(|_| LiveConsensusInputsImportError::BadHashHex { field })?;
    }
    Ok(Hash32(bytes))
}

fn parse_hash28(
    hex: &str,
    field: &'static str,
) -> Result<Hash28, LiveConsensusInputsImportError> {
    if hex.len() != 56 {
        return Err(LiveConsensusInputsImportError::BadHashHex { field });
    }
    let mut bytes = [0u8; 28];
    for i in 0..28 {
        let pair = &hex[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(pair, 16)
            .map_err(|_| LiveConsensusInputsImportError::BadHashHex { field })?;
    }
    Ok(Hash28(bytes))
}

fn lift_pool_distribution(
    raw: &BTreeMap<String, RawPoolEntry>,
) -> Result<BTreeMap<Hash28, PoolEntry>, LiveConsensusInputsImportError> {
    let mut out: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    for (k, v) in raw {
        let id = parse_hash28(k, "pool_distribution.pool_id")?;
        out.insert(
            id,
            PoolEntry {
                active_stake: v.active_stake,
            },
        );
    }
    Ok(out)
}

fn lift_pool_vrf_keyhashes(
    raw: &BTreeMap<String, String>,
) -> Result<BTreeMap<Hash28, Hash32>, LiveConsensusInputsImportError> {
    let mut out: BTreeMap<Hash28, Hash32> = BTreeMap::new();
    for (k, v) in raw {
        let id = parse_hash28(k, "pool_vrf_keyhashes.pool_id")?;
        let vrf = parse_hash32(v, "pool_vrf_keyhashes.vrf_keyhash")?;
        out.insert(id, vrf);
    }
    Ok(out)
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
            "00000000000000000000000000000000000000000000000000000001": {"active_stake": 123}
        },
        "pool_vrf_keyhashes": {
            "00000000000000000000000000000000000000000000000000000001": "00000000000000000000000000000000000000000000000000000000000000cc"
        },
        "protocol_params_hash_hex": "00000000000000000000000000000000000000000000000000000000000000dd",
        "source_cardano_node_version": "cardano-node 11.0.1",
        "source_query_command": "cardano-cli conway query stake-distribution --testnet-magic 1",
        "source_tip_hash_hex": "00000000000000000000000000000000000000000000000000000000000000ee",
        "source_tip_slot": 86400500
    }"#;

    fn replace(json: &str, needle: &str, repl: &str) -> String {
        let mut s = json.to_string();
        let pos = s.find(needle).expect("needle in MINIMAL");
        s.replace_range(pos..pos + needle.len(), repl);
        s
    }

    #[test]
    fn minimal_round_trip_imports_to_typed() {
        let out = import_live_consensus_inputs_raw_from_bytes(MINIMAL.as_bytes())
            .expect("import ok");
        assert_eq!(out.network_magic, 1);
        assert_eq!(out.era, CardanoEra::Conway);
        assert_eq!(out.epoch_no, EpochNo(200));
        assert_eq!(out.epoch_start_slot, SlotNo(86_400_000));
        assert_eq!(out.epoch_end_slot, SlotNo(86_832_000));
        assert_eq!(out.active_slots_coeff.numer, 1);
        assert_eq!(out.active_slots_coeff.denom, 20);
        assert_eq!(out.pool_distribution.len(), 1);
        assert_eq!(out.pool_vrf_keyhashes.len(), 1);
        assert_eq!(out.source_tip_slot, SlotNo(86_400_500));
    }

    #[test]
    fn unsupported_era_fails_fast() {
        let bad = replace(MINIMAL, "\"era\": \"conway\"", "\"era\": \"babbage\"");
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::EraNotSupported { .. }
        ));
    }

    #[test]
    fn empty_era_string_fails_fast() {
        let bad = replace(MINIMAL, "\"era\": \"conway\"", "\"era\": \"\"");
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::EraNotSupported { .. }
        ));
    }

    #[test]
    fn epoch_end_before_start_is_bad_window() {
        let bad = replace(MINIMAL, "\"epoch_end_slot\": 86832000,", "\"epoch_end_slot\": 1,");
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::BadEpochWindow { .. }
        ));
    }

    #[test]
    fn tip_outside_window_is_bad_window() {
        let bad = replace(MINIMAL, "\"source_tip_slot\": 86400500", "\"source_tip_slot\": 1");
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::BadEpochWindow { .. }
        ));
    }

    #[test]
    fn zero_asc_denom_is_bad_field() {
        let bad = replace(MINIMAL, "{\"numer\": 1, \"denom\": 20}", "{\"numer\": 1, \"denom\": 0}");
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::BadField { field: "active_slots_coeff.denom" }
        ));
    }

    #[test]
    fn short_genesis_hash_is_bad_hash_hex() {
        // 30 chars instead of 64.
        let bad = replace(
            MINIMAL,
            "\"genesis_hash_hex\": \"00000000000000000000000000000000000000000000000000000000000000aa\"",
            "\"genesis_hash_hex\": \"deadbeef\"",
        );
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::BadHashHex { field: "genesis_hash_hex" }
        ));
    }

    #[test]
    fn non_hex_in_hash_is_bad_hash_hex() {
        // Replace a single nibble with a non-hex character.
        let bad = replace(
            MINIMAL,
            "\"epoch_nonce_hex\": \"00000000000000000000000000000000000000000000000000000000000000bb\"",
            "\"epoch_nonce_hex\": \"zz000000000000000000000000000000000000000000000000000000000000bb\"",
        );
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::BadHashHex { field: "epoch_nonce_hex" }
        ));
    }

    #[test]
    fn pool_in_distribution_missing_from_vrf_map_is_bad_pool() {
        // Drop the lone pool from the VRF map but keep it in the
        // distribution.
        let bad = replace(
            MINIMAL,
            "\"pool_vrf_keyhashes\": {\n            \"00000000000000000000000000000000000000000000000000000001\": \"00000000000000000000000000000000000000000000000000000000000000cc\"\n        },",
            "\"pool_vrf_keyhashes\": {},",
        );
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::BadPoolDistribution { .. }
        ));
    }

    #[test]
    fn pool_id_wrong_width_is_bad_hash_hex() {
        // 28-byte pool id should be 56 hex chars; supply 50.
        let bad = replace(
            MINIMAL,
            "\"00000000000000000000000000000000000000000000000000000001\": {\"active_stake\": 123}",
            "\"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdead\": {\"active_stake\": 123}",
        );
        let err = import_live_consensus_inputs_raw_from_bytes(bad.as_bytes()).unwrap_err();
        assert!(matches!(
            err,
            LiveConsensusInputsImportError::BadHashHex { .. }
        ));
    }

    #[test]
    fn bad_json_surface_is_json_variant() {
        let err = import_live_consensus_inputs_raw_from_bytes(b"{not json").unwrap_err();
        assert!(matches!(err, LiveConsensusInputsImportError::Json(_)));
    }

    #[test]
    fn import_is_deterministic_across_repeated_calls() {
        let a = import_live_consensus_inputs_raw_from_bytes(MINIMAL.as_bytes()).unwrap();
        let b = import_live_consensus_inputs_raw_from_bytes(MINIMAL.as_bytes()).unwrap();
        assert_eq!(a, b);
    }
}
