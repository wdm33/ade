use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::diff_report::{DiffReport, Divergence};
use super::{Era, HarnessError};

/// Project-owned comparison container for block fields.
///
/// This is a generic comparison type, NOT a domain type. It holds
/// key-value pairs extracted from a block for differential comparison
/// against reference oracle data. Uses `BTreeMap` for deterministic ordering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockFields {
    pub era: Era,
    pub fields: BTreeMap<String, serde_json::Value>,
}

/// Trait for decoding raw CBOR bytes into comparable block fields.
///
/// Implementations will be era-specific. The harness uses this trait
/// to obtain project-side block representations for comparison against
/// reference oracle data.
pub trait BlockDecoder {
    fn decode_block(&self, era: Era, cbor: &[u8]) -> Result<BlockFields, HarnessError>;
}

/// Stub decoder that returns `NotYetImplemented` for all eras.
///
/// Placeholder until real CBOR decoding is implemented in later phases.
pub struct StubBlockDecoder;

impl BlockDecoder for StubBlockDecoder {
    fn decode_block(&self, era: Era, _cbor: &[u8]) -> Result<BlockFields, HarnessError> {
        Err(HarnessError::NotYetImplemented(format!(
            "block decoding for {era} not yet implemented"
        )))
    }
}

/// Compare two `BlockFields` and produce a `DiffReport`.
///
/// Performs field-level comparison with deterministic ordering.
/// Reports missing fields, extra fields, and value mismatches.
pub fn diff_block_fields(expected: &BlockFields, actual: &BlockFields) -> DiffReport {
    let mut divergences = BTreeMap::new();

    if expected.era != actual.era {
        divergences.insert(
            "era".to_string(),
            Divergence {
                path: "era".to_string(),
                expected: serde_json::to_value(expected.era).unwrap_or_default(),
                actual: serde_json::to_value(actual.era).unwrap_or_default(),
            },
        );
    }

    for (key, expected_val) in &expected.fields {
        match actual.fields.get(key) {
            Some(actual_val) if actual_val != expected_val => {
                divergences.insert(
                    key.clone(),
                    Divergence {
                        path: key.clone(),
                        expected: expected_val.clone(),
                        actual: actual_val.clone(),
                    },
                );
            }
            None => {
                divergences.insert(
                    key.clone(),
                    Divergence {
                        path: key.clone(),
                        expected: expected_val.clone(),
                        actual: serde_json::Value::Null,
                    },
                );
            }
            _ => {}
        }
    }

    for (key, actual_val) in &actual.fields {
        if !expected.fields.contains_key(key) {
            divergences.insert(
                key.clone(),
                Divergence {
                    path: key.clone(),
                    expected: serde_json::Value::Null,
                    actual: actual_val.clone(),
                },
            );
        }
    }

    DiffReport { divergences }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_block_fields(era: Era, pairs: &[(&str, serde_json::Value)]) -> BlockFields {
        let mut fields = BTreeMap::new();
        for (k, v) in pairs {
            fields.insert(k.to_string(), v.clone());
        }
        BlockFields { era, fields }
    }

    #[test]
    fn self_comparison_zero_divergences_byron() {
        let fields = make_block_fields(Era::Byron, &[("slot", json!(0)), ("hash", json!("abc"))]);
        let report = diff_block_fields(&fields, &fields);
        assert!(report.is_empty(), "expected zero divergences: {report}");
    }

    #[test]
    fn self_comparison_zero_divergences_shelley() {
        let fields = make_block_fields(
            Era::Shelley,
            &[("slot", json!(4492800)), ("block_number", json!(4490511))],
        );
        let report = diff_block_fields(&fields, &fields);
        assert!(report.is_empty());
    }

    #[test]
    fn self_comparison_zero_divergences_allegra() {
        let fields = make_block_fields(
            Era::Allegra,
            &[("slot", json!(16588800)), ("size", json!(4096))],
        );
        let report = diff_block_fields(&fields, &fields);
        assert!(report.is_empty());
    }

    #[test]
    fn self_comparison_zero_divergences_mary() {
        let fields = make_block_fields(
            Era::Mary,
            &[("slot", json!(23068800)), ("tx_count", json!(12))],
        );
        let report = diff_block_fields(&fields, &fields);
        assert!(report.is_empty());
    }

    #[test]
    fn self_comparison_zero_divergences_alonzo() {
        let fields = make_block_fields(
            Era::Alonzo,
            &[("slot", json!(39916800)), ("script_count", json!(3))],
        );
        let report = diff_block_fields(&fields, &fields);
        assert!(report.is_empty());
    }

    #[test]
    fn self_comparison_zero_divergences_babbage() {
        let fields = make_block_fields(
            Era::Babbage,
            &[
                ("slot", json!(72316896)),
                ("protocol_version", json!({"major": 8, "minor": 0})),
            ],
        );
        let report = diff_block_fields(&fields, &fields);
        assert!(report.is_empty());
    }

    #[test]
    fn self_comparison_zero_divergences_conway() {
        let fields = make_block_fields(
            Era::Conway,
            &[
                ("slot", json!(107913600)),
                ("governance_actions", json!([])),
            ],
        );
        let report = diff_block_fields(&fields, &fields);
        assert!(report.is_empty());
    }

    #[test]
    fn detects_value_mismatch() {
        let expected = make_block_fields(Era::Byron, &[("slot", json!(0))]);
        let actual = make_block_fields(Era::Byron, &[("slot", json!(1))]);
        let report = diff_block_fields(&expected, &actual);
        assert_eq!(report.divergence_count(), 1);
        assert!(report.divergences.contains_key("slot"));
    }

    #[test]
    fn detects_missing_field() {
        let expected =
            make_block_fields(Era::Shelley, &[("slot", json!(100)), ("hash", json!("x"))]);
        let actual = make_block_fields(Era::Shelley, &[("slot", json!(100))]);
        let report = diff_block_fields(&expected, &actual);
        assert_eq!(report.divergence_count(), 1);
        assert!(report.divergences.contains_key("hash"));
        assert_eq!(report.divergences["hash"].actual, serde_json::Value::Null);
    }

    #[test]
    fn detects_extra_field() {
        let expected = make_block_fields(Era::Mary, &[("slot", json!(100))]);
        let actual = make_block_fields(Era::Mary, &[("slot", json!(100)), ("extra", json!(42))]);
        let report = diff_block_fields(&expected, &actual);
        assert_eq!(report.divergence_count(), 1);
        assert!(report.divergences.contains_key("extra"));
        assert_eq!(
            report.divergences["extra"].expected,
            serde_json::Value::Null
        );
    }

    #[test]
    fn detects_era_mismatch() {
        let expected = make_block_fields(Era::Byron, &[]);
        let actual = make_block_fields(Era::Shelley, &[]);
        let report = diff_block_fields(&expected, &actual);
        assert!(report.divergences.contains_key("era"));
    }

    #[test]
    fn stub_decoder_returns_not_yet_implemented() {
        let decoder = StubBlockDecoder;
        for era in Era::all() {
            let result = decoder.decode_block(*era, &[]);
            match result {
                Err(HarnessError::NotYetImplemented(msg)) => {
                    assert!(msg.contains(&era.to_string()));
                }
                other => panic!("expected NotYetImplemented for {era}, got {other:?}"),
            }
        }
    }

    #[test]
    fn block_fields_roundtrip_json() {
        let fields = make_block_fields(
            Era::Conway,
            &[
                ("slot", json!(12345)),
                ("hash", json!("deadbeef")),
                ("nested", json!({"a": 1, "b": [2, 3]})),
            ],
        );
        let json = serde_json::to_string(&fields).unwrap();
        let parsed: BlockFields = serde_json::from_str(&json).unwrap();
        assert_eq!(fields, parsed);
    }

    #[test]
    fn diff_report_deterministic_key_order() {
        let expected =
            make_block_fields(Era::Byron, &[("z_field", json!(1)), ("a_field", json!(2))]);
        let actual = make_block_fields(Era::Byron, &[("z_field", json!(9)), ("a_field", json!(8))]);
        let report = diff_block_fields(&expected, &actual);
        let keys: Vec<&String> = report.divergences.keys().collect();
        assert_eq!(keys, vec!["a_field", "z_field"]);
    }
}
