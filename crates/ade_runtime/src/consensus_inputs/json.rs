// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN cardano-cli operator-bundle JSON deserializer (PHASE4-N-M-C S1a).
//!
//! Pure structural deserialization of the operator's consensus-inputs
//! JSON envelope into typed serde structs. Validation (hash widths,
//! era recognition, epoch-window consistency, pool-distribution
//! invariants) lives in `importer.rs`; this module only ensures the
//! JSON parses as the declared shape.
//!
//! The envelope is operator-produced — they run cardano-cli
//! commands and assemble the result. The expected shape is
//! fixed; per ¬P-C4 there is no partial-import fallback.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The full operator bundle. All fields are mandatory and there is
/// no `#[serde(default)]`: a missing field is a JSON deserialization
/// error before the typed-import layer is even reached.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawConsensusInputs {
    /// Cardano network magic (e.g. `1` for preprod).
    pub network_magic: u32,
    /// Hex-encoded 32-byte genesis hash (Shelley genesis).
    pub genesis_hash_hex: String,
    /// Lowercase era name; importer rejects anything other than
    /// `"conway"` for the C cluster.
    pub era: String,
    /// Epoch number this bundle pertains to.
    pub epoch_no: u64,
    /// First slot of the epoch (inclusive).
    pub epoch_start_slot: u64,
    /// Last slot of the epoch (inclusive).
    pub epoch_end_slot: u64,
    /// Active-slots-coefficient as a fraction.
    pub active_slots_coeff: RawFraction,
    /// Hex-encoded 32-byte epoch nonce.
    pub epoch_nonce_hex: String,
    /// Pool distribution: hex-encoded 28-byte pool id → entry.
    /// `BTreeMap` ordering is mirrored into the typed form.
    pub pool_distribution: BTreeMap<String, RawPoolEntry>,
    /// Pool VRF key hashes: hex-encoded 28-byte pool id → hex
    /// 32-byte VRF key hash. Must cover the same key-set as
    /// `pool_distribution`.
    pub pool_vrf_keyhashes: BTreeMap<String, String>,
    /// Hex-encoded 32-byte protocol-parameters hash (operator
    /// records this from cardano-cli's protocol-state output).
    pub protocol_params_hash_hex: String,
    /// Source cardano-node version string (`cardano-node --version`).
    pub source_cardano_node_version: String,
    /// Exact cardano-cli query command(s) that produced the
    /// bundle.
    pub source_query_command: String,
    /// Hex-encoded 32-byte tip block hash at extract time.
    pub source_tip_hash_hex: String,
    /// Tip slot at extract time.
    pub source_tip_slot: u64,
    /// The exact cardano-cli `query protocol-parameters` JSON string (the
    /// canonical dump the builder hashes) — the oracle **preimage** for the
    /// already-fingerprinted `protocol_params_hash`. It is NOT a new canonical
    /// bundle field and does not alter the bundle fingerprint (PHASE4-N-F-G-A
    /// S2a). Structurally optional: historical / import-only bundles (pre-S2a
    /// evidence) may omit it; it is **mandatory and hash-bound at the node
    /// forge-current-pparams install site** (`require_forge_current_pparams`).
    #[serde(default)]
    pub protocol_params_json: Option<String>,
    /// Venue tag (C2-VENUE-PARAM): `"preview"` | `"preprod"`. Provenance only —
    /// `#[serde(default)]` so pre-venue-param bundles still parse, and it does
    /// NOT enter the canonical fingerprint (like `protocol_params_json`).
    #[serde(default)]
    pub network: Option<String>,
    /// Venue epoch length in slots (provenance; equals
    /// `epoch_end_slot - epoch_start_slot + 1`).
    #[serde(default)]
    pub epoch_length: Option<u64>,
    /// Provenance: the cardano-cli query the `pool_distribution` active_stake
    /// was sourced from (the leader-election `go` snapshot).
    #[serde(default)]
    pub pool_distribution_source: Option<String>,
}

/// Rational fraction `numer / denom` for ASC. `denom` must be
/// non-zero (enforced in `importer.rs`).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawFraction {
    pub numer: u32,
    pub denom: u32,
}

/// Per-pool entry. Active stake in lovelace.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawPoolEntry {
    pub active_stake: u64,
}

/// SOLE pub fn converting JSON bytes into the structural
/// intermediate. Downstream `importer::import_live_consensus_inputs_raw_from_bytes`
/// consumes the result.
pub fn parse_consensus_inputs_json(bytes: &[u8]) -> Result<RawConsensusInputs, serde_json::Error> {
    serde_json::from_slice(bytes)
}

/// The encoder counterpart to [`parse_consensus_inputs_json`] — the SOLE pub fn producing the
/// operator-bundle JSON bytes from the typed shape. `bootstrap-export` (BOOTSTRAP-CERTSTATE-PRODUCER)
/// assembles a `RawConsensusInputs` and calls THIS to emit `<base>.json`; it never hand-rolls the
/// format, so the emitted bundle is parse-identical to the import authority (the round-trip test
/// pins `parse(encode(raw)) == raw`).
pub fn encode_consensus_inputs_json(raw: &RawConsensusInputs) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec_pretty(raw)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MINIMAL_BUNDLE: &str = r#"{
        "network_magic": 2,
        "genesis_hash_hex": "9a",
        "era": "conway",
        "epoch_no": 1336,
        "epoch_start_slot": 1,
        "epoch_end_slot": 432000,
        "active_slots_coeff": {"numer": 1, "denom": 20},
        "epoch_nonce_hex": "bb",
        "pool_distribution": {"11": {"active_stake": 1000}},
        "pool_vrf_keyhashes": {"11": "22"},
        "protocol_params_hash_hex": "dd",
        "source_cardano_node_version": "cardano-node 11.0.1",
        "source_query_command": "cardano-cli query ...",
        "source_tip_hash_hex": "ab",
        "source_tip_slot": 115455568,
        "protocol_params_json": "{\"k\":1}",
        "network": "preview"
    }"#;

    #[test]
    fn encode_round_trips_through_the_parser_authority() {
        // bootstrap-export reuses THIS authority to emit <base>.json: encode(parse(json)) must
        // re-parse to the identical typed shape (never a parallel JSON format).
        let raw = parse_consensus_inputs_json(MINIMAL_BUNDLE.as_bytes()).unwrap();
        let bytes = encode_consensus_inputs_json(&raw).unwrap();
        let raw2 = parse_consensus_inputs_json(&bytes).unwrap();
        assert_eq!(raw, raw2, "encode(parse(json)) re-parses to the SAME RawConsensusInputs");
    }

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

    #[test]
    fn minimal_round_trip_parses() {
        let raw = parse_consensus_inputs_json(MINIMAL.as_bytes()).expect("parse ok");
        assert_eq!(raw.network_magic, 1);
        assert_eq!(raw.era, "conway");
        assert_eq!(raw.epoch_no, 200);
        assert_eq!(raw.active_slots_coeff, RawFraction { numer: 1, denom: 20 });
        assert_eq!(raw.pool_distribution.len(), 1);
        assert_eq!(raw.pool_vrf_keyhashes.len(), 1);
    }

    #[test]
    fn missing_required_field_is_error() {
        // Drop the `epoch_no` field; deserialization must fail
        // because there is no `#[serde(default)]`.
        let bad = MINIMAL.replace(r#""epoch_no": 200,"#, "");
        let err = parse_consensus_inputs_json(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("epoch_no"));
    }

    #[test]
    fn unknown_field_is_rejected() {
        // `deny_unknown_fields` rejects operator-introduced typos
        // before they silently take a default path.
        let bad = MINIMAL.replace(
            r#""source_tip_slot": 86400500"#,
            r#""source_tip_slot": 86400500, "extra_field": 1"#,
        );
        let err = parse_consensus_inputs_json(bad.as_bytes()).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("extra_field"));
    }

    #[test]
    fn venue_tagged_bundle_parses_and_records_network() {
        // C2-VENUE-PARAM: a venue-tagged bundle (network / epoch_length /
        // pool_distribution_source) must be ACCEPTED by deny_unknown_fields,
        // and the venue tag recorded as provenance.
        let tagged = MINIMAL.replace(
            r#""source_tip_slot": 86400500"#,
            concat!(
                "\"source_tip_slot\": 86400500,\n",
                "        \"network\": \"preview\",\n",
                "        \"epoch_length\": 86400,\n",
                "        \"pool_distribution_source\": \"cardano-cli query stake-snapshot --all-stake-pools (leader-election go stake)\""
            ),
        );
        let raw = parse_consensus_inputs_json(tagged.as_bytes()).expect("venue-tagged parse ok");
        assert_eq!(raw.network.as_deref(), Some("preview"));
        assert_eq!(raw.epoch_length, Some(86400));
        assert!(raw.pool_distribution_source.is_some());
    }

    #[test]
    fn pre_venue_param_bundle_still_parses() {
        // Backward-compat: a bundle with no venue tag still parses; the venue
        // fields default to None (no fingerprint impact).
        let raw = parse_consensus_inputs_json(MINIMAL.as_bytes()).expect("parse ok");
        assert_eq!(raw.network, None);
        assert_eq!(raw.epoch_length, None);
        assert_eq!(raw.pool_distribution_source, None);
    }
}
