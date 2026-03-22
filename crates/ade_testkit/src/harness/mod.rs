pub mod adapters;
pub mod address_extractor;
pub mod block_diff;
pub mod diff_report;
pub mod era_mapping;
pub mod genesis_loader;
pub mod ledger_diff;
pub mod oracle_manifest;
pub mod protocol_diff;
pub mod provenance;
pub mod regression_corpus;
pub mod shelley_loader;
pub mod snapshot_loader;
pub mod transcript;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Cardano eras in chronological order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Era {
    Byron,
    Shelley,
    Allegra,
    Mary,
    Alonzo,
    Babbage,
    Conway,
}

impl Era {
    /// Returns all eras in chronological order.
    pub fn all() -> &'static [Era] {
        &[
            Era::Byron,
            Era::Shelley,
            Era::Allegra,
            Era::Mary,
            Era::Alonzo,
            Era::Babbage,
            Era::Conway,
        ]
    }

    /// Returns the era name as a lowercase string slice.
    pub fn as_str(&self) -> &'static str {
        match self {
            Era::Byron => "byron",
            Era::Shelley => "shelley",
            Era::Allegra => "allegra",
            Era::Mary => "mary",
            Era::Alonzo => "alonzo",
            Era::Babbage => "babbage",
            Era::Conway => "conway",
        }
    }
}

impl fmt::Display for Era {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured error type for harness operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessError {
    /// Operation not yet implemented (stub placeholder).
    NotYetImplemented(String),
    /// Error during block/transaction decoding.
    DecodingError(String),
    /// Validation constraint violation.
    ValidationError(String),
    /// I/O operation failed.
    IoError(String),
    /// Parse error (TOML, JSON, etc.).
    ParseError(String),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessError::NotYetImplemented(msg) => write!(f, "not yet implemented: {msg}"),
            HarnessError::DecodingError(msg) => write!(f, "decoding error: {msg}"),
            HarnessError::ValidationError(msg) => write!(f, "validation error: {msg}"),
            HarnessError::IoError(msg) => write!(f, "I/O error: {msg}"),
            HarnessError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for HarnessError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_all_returns_seven_eras() {
        assert_eq!(Era::all().len(), 7);
    }

    #[test]
    fn era_ordering_is_chronological() {
        let eras = Era::all();
        for window in eras.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn era_display_matches_as_str() {
        for era in Era::all() {
            assert_eq!(format!("{era}"), era.as_str());
        }
    }

    #[test]
    fn era_roundtrip_json() {
        for era in Era::all() {
            let json = serde_json::to_string(era).unwrap();
            let parsed: Era = serde_json::from_str(&json).unwrap();
            assert_eq!(*era, parsed);
        }
    }

    #[test]
    fn harness_error_display() {
        let err = HarnessError::NotYetImplemented("test".to_string());
        assert_eq!(format!("{err}"), "not yet implemented: test");
    }

    #[test]
    fn harness_error_is_std_error() {
        let err = HarnessError::DecodingError("bad bytes".to_string());
        let _: &dyn std::error::Error = &err;
    }
}
