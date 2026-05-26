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

/// Per-UTxO record as cardano-cli emits it. The shape consumed
/// by the seed importer. cardano-cli emits additional fields
/// (`datum`, `inlineDatum`, `inlineDatumhash`) that this struct
/// deliberately does NOT declare:
///
/// - `datum` and `inlineDatum` carry the PARSED form of the
///   datum (Plutus-data as JSON). Plutus integers can exceed
///   `f64` precision (preprod has datum trees with literals
///   like `1.79…e308`), and `serde_json` cannot deserialize
///   such numbers into any numeric type. We consume only the
///   raw CBOR hex via `inline_datum_raw` and the on-chain
///   `datumhash`. Per serde's default behavior these unknown
///   keys are silently skipped without parsing their values.
///
///   **Hard contract — PHASE4-N-M-A1.1.** These fields are
///   ignored ONLY because the authoritative seed-import path
///   does not consume them. If a future slice ever needs
///   `datum` / `inlineDatum` semantics (e.g. for ledger
///   replay that requires datum content, or a CLI/UX surface
///   that displays Plutus data), it MUST be added through a
///   non-floating, lossless Plutus-data decoder consuming the
///   `inline_datum_raw` (CBOR) bytes — NEVER by re-adding
///   `Option<serde_json::Value>` here. Reintroducing the
///   JSON-parsed shape would silently accept f64-truncated
///   Plutus integers and convert a strict tolerance into a
///   hidden semantic bypass.
///
/// - `inlineDatumhash` is operator-side metadata; the canonical
///   datum hash is recomputed from `inline_datum_raw` bytes.
///
/// Declared fields with `#[serde(default)]` accept `null` values
/// (cardano-cli writes `null` rather than omitting fields).
/// `value` is required.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RawUtxoEntry {
    pub address: String,
    #[serde(default)]
    pub datumhash: Option<String>,
    #[serde(default)]
    #[serde(rename = "inlineDatumRaw")]
    pub inline_datum_raw: Option<String>,
    #[serde(default)]
    #[serde(rename = "referenceScript")]
    pub reference_script: Option<RawReferenceScript>,
    pub value: RawValue,
}

/// `referenceScript` field shape as cardano-cli emits it
/// (cardano-node 11.0.x). PHASE4-N-M-A1.1.
///
/// JSON:
/// ```text
/// "referenceScript": {
///   "script": { "cborHex": "<hex>", "description": "<...>", "type": "<...>" },
///   "scriptLanguage": "<...>"
/// }
/// ```
///
/// `scriptLanguage` and `description` are accepted-and-ignored
/// metadata fields. The canonical bytes are derived from `script.type`
/// + `script.cborHex` alone (see `importer::encode_script_ref`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RawReferenceScript {
    pub script: RawScriptEnvelope,
    #[serde(default)]
    #[serde(rename = "scriptLanguage")]
    pub script_language: Option<String>,
}

/// Inner `script` object: `cborHex` is the canonical CBOR payload
/// (CBOR-encoded `bytes(plutus_binary)` for Plutus variants;
/// CBOR-encoded `native_script` array for SimpleScript). `type`
/// is the closed-vocabulary script variant name.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RawScriptEnvelope {
    #[serde(rename = "cborHex")]
    pub cbor_hex: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub ty: String,
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
    /// Amounts are deserialized via the lossy
    /// [`amount_from_number`] helper because cardano-cli emits
    /// f64 literals (e.g. `1.49999999999999e19`) when the value
    /// exceeds the JSON-precise integer range.
    Assets(BTreeMap<String, AssetAmount>),
}

/// Asset amount as serialized by cardano-cli. Accepts both
/// integer and f64 literals; f64 is parsed via `as u64` saturate
/// (cardano-cli emits f64 only for amounts above `u64::MAX as f64`
/// precision, so the conversion is lossy ONLY at the JSON
/// boundary, not in any BLUE / authority path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetAmount(pub u64);

impl<'de> serde::Deserialize<'de> for AssetAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let v = serde_json::Value::deserialize(deserializer)?;
        match v {
            serde_json::Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    Ok(AssetAmount(u))
                } else if let Some(f) = n.as_f64() {
                    if !f.is_finite() || f < 0.0 {
                        return Err(D::Error::custom(
                            "asset amount must be finite and non-negative",
                        ));
                    }
                    // Saturate at u64::MAX. Any amount this large
                    // is beyond Cardano's protocol-meaningful
                    // range; cardano-cli emits f64 because the
                    // JSON spec doesn't promise integer precision
                    // past 2^53.
                    Ok(AssetAmount(f.min(u64::MAX as f64) as u64))
                } else {
                    Err(D::Error::custom("asset amount is not representable as u64"))
                }
            }
            other => Err(D::Error::custom(format!(
                "asset amount must be a JSON number, got {}",
                match other {
                    serde_json::Value::Null => "null",
                    serde_json::Value::Bool(_) => "bool",
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Object(_) => "object",
                    serde_json::Value::Number(_) => unreachable!(),
                }
            ))),
        }
    }
}

impl AssetAmount {
    pub fn get(self) -> u64 {
        self.0
    }
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
        assert!(first.datumhash.is_none());
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
        // The struct silently skips the parsed-form `inlineDatum`
        // (per PHASE4-N-M-A1.1 §3); we only consume the raw hex.
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

    const REFERENCE_SCRIPT_ENTRY: &str = r#"{
        "0000000000000000000000000000000000000000000000000000000000000004#0": {
            "address": "addr_test1wp97ley0p7xqksmh6tq3c6v8depl9jpfvnkk68d29fwznmcmlpuqk",
            "inlineDatum": null,
            "inlineDatumRaw": null,
            "referenceScript": {
                "script": {
                    "cborHex": "590a5b0100003323232323232",
                    "description": "",
                    "type": "PlutusScriptV2"
                },
                "scriptLanguage": "PlutusScriptLanguage PlutusScriptV2"
            },
            "value": { "lovelace": 12345678 }
        }
    }"#;

    #[test]
    fn parse_utxo_seed_json_reference_script_entry() {
        let parsed = parse_utxo_seed_json(REFERENCE_SCRIPT_ENTRY.as_bytes()).expect("parse");
        let only = parsed.values().next().expect("entry");
        let rs = only.reference_script.as_ref().expect("ref script present");
        assert_eq!(rs.script.ty, "PlutusScriptV2");
        assert_eq!(rs.script.cbor_hex, "590a5b0100003323232323232");
        assert_eq!(
            rs.script_language.as_deref(),
            Some("PlutusScriptLanguage PlutusScriptV2")
        );
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
