// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN cardano-cli JSON UTxO deserializer (PHASE4-N-M-A S1).
//!
//! Typed serde structs for the cardano-cli 11.0.0
//! `query utxo --whole-utxo` JSON shape. Pure structural
//! deserialization; no canonicalization happens here (that's
//! `importer::import_cardano_cli_json_utxo`'s job).

use serde::Deserialize;
use std::collections::BTreeMap;

/// Top-level: a map of "tx_hash_hex#index" → per-UTxO record.
pub type RawUtxoMap = BTreeMap<String, RawUtxoEntry>;

/// Per-UTxO record as cardano-cli emits it.
///
/// All fields are `Option<...>` so the deserializer accepts
/// `null` values (the cli writes `null` rather than omitting
/// fields). `value` is required.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RawUtxoEntry {
    pub address: String,
    #[serde(default)]
    pub datum: Option<serde_json::Value>,
    #[serde(default)]
    pub datumhash: Option<String>,
    #[serde(default)]
    #[serde(rename = "inlineDatum")]
    pub inline_datum: Option<serde_json::Value>,
    #[serde(default)]
    #[serde(rename = "inlineDatumRaw")]
    pub inline_datum_raw: Option<String>,
    #[serde(default)]
    #[serde(rename = "inlineDatumhash")]
    pub inline_datum_hash: Option<String>,
    #[serde(default)]
    #[serde(rename = "referenceScript")]
    pub reference_script: Option<serde_json::Value>,
    pub value: RawValue,
}

/// `value` field: `{"lovelace": N, "<policy_hex>": {"<asset_name_hex>": N}}`.
/// We model it as a flat BTreeMap from currency-symbol string
/// (`"lovelace"` for ada, otherwise the 28-byte policy hex) to
/// either a bare uint (lovelace) or a nested asset map.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RawValueEntry {
    /// Lovelace amount: a bare unsigned int.
    Lovelace(u64),
    /// Multi-asset map: policy hex → { asset name hex → amount }.
    Assets(BTreeMap<String, u64>),
}

pub type RawValue = BTreeMap<String, RawValueEntry>;

/// Parse the cardano-cli JSON UTxO dump from raw bytes.
///
/// This is the SOLE pub fn that converts JSON bytes into the
/// typed intermediate `RawUtxoMap`. The downstream
/// `importer::import_cardano_cli_json_utxo` consumes that.
pub fn parse_utxo_seed_json(bytes: &[u8]) -> Result<RawUtxoMap, serde_json::Error> {
    serde_json::from_slice(bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    const MINIMAL_TWO_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000001#0": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "datum": null,
            "datumhash": null,
            "inlineDatum": null,
            "inlineDatumRaw": null,
            "referenceScript": null,
            "value": { "lovelace": 1000000 }
        },
        "0000000000000000000000000000000000000000000000000000000000000002#3": {
            "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
            "datum": null,
            "datumhash": null,
            "inlineDatum": null,
            "inlineDatumRaw": null,
            "referenceScript": null,
            "value": { "lovelace": 2000000 }
        }
    }"#;

    const INLINE_DATUM_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000003#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "datum": null,
            "inlineDatum": { "bytes": "7f5055adc0fddd13ee66d565d1a2ae552be4a9fcdd6835613fbb872f" },
            "inlineDatumRaw": "581c7f5055adc0fddd13ee66d565d1a2ae552be4a9fcdd6835613fbb872f",
            "inlineDatumhash": "9ec2ff07ca1ea368165397aa52c636d9d96a6c944666bb595437cd25218e6080",
            "referenceScript": null,
            "value": { "lovelace": 10000000 }
        }
    }"#;

    #[test]
    fn parse_utxo_seed_json_minimal_two_entry() {
        let parsed = parse_utxo_seed_json(MINIMAL_TWO_ENTRY.as_bytes()).expect("parse");
        assert_eq!(parsed.len(), 2);
        let first = parsed
            .get("0000000000000000000000000000000000000000000000000000000000000001#0")
            .expect("first entry");
        assert_eq!(
            first.address,
            "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u"
        );
        assert!(first.datum.is_none());
        assert_eq!(first.value.len(), 1);
        match first.value.get("lovelace").expect("lovelace") {
            RawValueEntry::Lovelace(n) => assert_eq!(*n, 1_000_000),
            other => panic!("expected Lovelace, got {other:?}"),
        }
    }

    #[test]
    fn parse_utxo_seed_json_inline_datum_entry() {
        let parsed = parse_utxo_seed_json(INLINE_DATUM_ENTRY.as_bytes()).expect("parse");
        let only = parsed.values().next().expect("entry");
        assert!(only.inline_datum.is_some());
        assert_eq!(
            only.inline_datum_raw.as_deref(),
            Some("581c7f5055adc0fddd13ee66d565d1a2ae552be4a9fcdd6835613fbb872f")
        );
    }

    #[test]
    fn parse_utxo_seed_json_rejects_garbage() {
        let result = parse_utxo_seed_json(b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_utxo_seed_json_btree_order_independent_of_source_order() {
        // Same entries in two different textual orders → same map
        // (BTreeMap orders by key).
        let order_a = r#"{
            "0000000000000000000000000000000000000000000000000000000000000002#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 200 }
            },
            "0000000000000000000000000000000000000000000000000000000000000001#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 100 }
            }
        }"#;
        let order_b = r#"{
            "0000000000000000000000000000000000000000000000000000000000000001#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 100 }
            },
            "0000000000000000000000000000000000000000000000000000000000000002#0": {
                "address": "addr_test1vq0ast4z2dypfrl9kg2c0garrcy085w78dls8xsx954x34cmgvp2u",
                "value": { "lovelace": 200 }
            }
        }"#;
        let a = parse_utxo_seed_json(order_a.as_bytes()).expect("a");
        let b = parse_utxo_seed_json(order_b.as_bytes()).expect("b");
        // BTreeMap equality is structural — order-independent.
        assert_eq!(a, b);
        // Iteration order is canonical (smallest key first).
        let keys: Vec<&String> = a.keys().collect();
        assert_eq!(
            keys[0],
            "0000000000000000000000000000000000000000000000000000000000000001#0"
        );
    }
}
