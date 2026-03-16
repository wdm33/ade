// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

/// Structured codec error with byte offset information.
///
/// All variants carry offsets for diagnostic localization. No `String` fields —
/// only `&'static str` for determinism and equality comparability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// An unknown HFC era tag was encountered.
    UnknownEraTag { tag: u8 },

    /// Unexpected end of input.
    UnexpectedEof { offset: usize, needed: usize },

    /// Structurally invalid CBOR.
    InvalidCborStructure { offset: usize, detail: &'static str },

    /// Trailing bytes after a complete CBOR item.
    TrailingBytes { consumed: usize, total: usize },

    /// Unexpected CBOR major type.
    UnexpectedCborType {
        offset: usize,
        expected: &'static str,
        actual: u8,
    },

    /// Invalid length or count.
    InvalidLength { offset: usize, detail: &'static str },
}

impl core::fmt::Display for CodecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CodecError::UnknownEraTag { tag } => {
                write!(f, "unknown era tag: {tag}")
            }
            CodecError::UnexpectedEof { offset, needed } => {
                write!(
                    f,
                    "unexpected EOF at offset {offset}, needed {needed} bytes"
                )
            }
            CodecError::InvalidCborStructure { offset, detail } => {
                write!(f, "invalid CBOR structure at offset {offset}: {detail}")
            }
            CodecError::TrailingBytes { consumed, total } => {
                write!(f, "trailing bytes: consumed {consumed} of {total} bytes")
            }
            CodecError::UnexpectedCborType {
                offset,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "unexpected CBOR type at offset {offset}: expected {expected}, got major type {actual}"
                )
            }
            CodecError::InvalidLength { offset, detail } => {
                write!(f, "invalid length at offset {offset}: {detail}")
            }
        }
    }
}

impl std::error::Error for CodecError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_unknown_era_tag() {
        let e = CodecError::UnknownEraTag { tag: 8 };
        assert_eq!(format!("{e}"), "unknown era tag: 8");
    }

    #[test]
    fn error_display_unexpected_eof() {
        let e = CodecError::UnexpectedEof {
            offset: 10,
            needed: 4,
        };
        assert_eq!(
            format!("{e}"),
            "unexpected EOF at offset 10, needed 4 bytes"
        );
    }

    #[test]
    fn error_display_trailing_bytes() {
        let e = CodecError::TrailingBytes {
            consumed: 5,
            total: 10,
        };
        assert_eq!(format!("{e}"), "trailing bytes: consumed 5 of 10 bytes");
    }

    #[test]
    fn error_equality() {
        let a = CodecError::UnknownEraTag { tag: 3 };
        let b = CodecError::UnknownEraTag { tag: 3 };
        let c = CodecError::UnknownEraTag { tag: 4 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn error_is_std_error() {
        let e = CodecError::InvalidCborStructure {
            offset: 0,
            detail: "test",
        };
        let _: &dyn std::error::Error = &e;
    }
}
