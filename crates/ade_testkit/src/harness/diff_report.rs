use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// A single field-level divergence between expected and actual values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Divergence {
    /// Dot-separated path to the divergent field.
    pub path: String,
    /// Expected value from reference oracle.
    pub expected: serde_json::Value,
    /// Actual value produced by project code.
    pub actual: serde_json::Value,
}

impl fmt::Display for Divergence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: expected {}, got {}",
            self.path, self.expected, self.actual
        )
    }
}

/// Deterministically-ordered collection of field-level divergences.
///
/// Uses `BTreeMap` for reproducible iteration order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffReport {
    pub divergences: BTreeMap<String, Divergence>,
}

impl DiffReport {
    /// Returns true if no divergences were found.
    pub fn is_empty(&self) -> bool {
        self.divergences.is_empty()
    }

    /// Returns the number of divergent fields.
    pub fn divergence_count(&self) -> usize {
        self.divergences.len()
    }
}

impl fmt::Display for DiffReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.divergences.is_empty() {
            write!(f, "no divergences")
        } else {
            writeln!(f, "{} divergence(s):", self.divergences.len())?;
            for (key, div) in &self.divergences {
                writeln!(f, "  [{key}] {div}")?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_report() {
        let report = DiffReport {
            divergences: BTreeMap::new(),
        };
        assert!(report.is_empty());
        assert_eq!(report.divergence_count(), 0);
    }

    #[test]
    fn report_with_divergences() {
        let mut divergences = BTreeMap::new();
        divergences.insert(
            "slot".to_string(),
            Divergence {
                path: "slot".to_string(),
                expected: json!(42),
                actual: json!(43),
            },
        );
        let report = DiffReport { divergences };
        assert!(!report.is_empty());
        assert_eq!(report.divergence_count(), 1);
    }

    #[test]
    fn report_display_empty() {
        let report = DiffReport {
            divergences: BTreeMap::new(),
        };
        assert_eq!(format!("{report}"), "no divergences");
    }

    #[test]
    fn divergence_display() {
        let div = Divergence {
            path: "header.slot".to_string(),
            expected: json!(100),
            actual: json!(200),
        };
        let s = format!("{div}");
        assert!(s.contains("header.slot"));
        assert!(s.contains("100"));
        assert!(s.contains("200"));
    }

    #[test]
    fn report_roundtrip_json() {
        let mut divergences = BTreeMap::new();
        divergences.insert(
            "field_a".to_string(),
            Divergence {
                path: "field_a".to_string(),
                expected: json!("x"),
                actual: json!("y"),
            },
        );
        let report = DiffReport { divergences };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: DiffReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, parsed);
    }
}
