// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::fmt;

/// Failure modes the chain database may surface.
///
/// Not-found is NOT an error — `get_block_*` returns `Ok(None)` for
/// absent records. `ChainDbError` is reserved for situations where the
/// db cannot fulfill the request: I/O failure, integrity violation,
/// schema incompatibility, or invalid caller request.
#[derive(Debug)]
pub enum ChainDbError {
    /// Storage layer I/O failure (disk full, permissions, etc.).
    /// Transient; retry may succeed.
    Io(std::io::Error),

    /// Stored data failed integrity check (checksum mismatch,
    /// truncated record, version tag invalid). Storage is corrupted;
    /// retry will not recover.
    Corruption(String),

    /// Storage was opened with a schema version this binary doesn't
    /// understand. Caller chooses the migration path.
    SchemaMismatch { expected: u32, found: u32 },

    /// Operation invalid for the current state — e.g., rolling back
    /// to a slot beyond the tip, or putting a block whose claimed
    /// slot conflicts with an existing block at that slot.
    InvalidOperation(String),
}

impl fmt::Display for ChainDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainDbError::Io(e) => write!(f, "chaindb I/O: {e}"),
            ChainDbError::Corruption(detail) => {
                write!(f, "chaindb corruption: {detail}")
            }
            ChainDbError::SchemaMismatch { expected, found } => write!(
                f,
                "chaindb schema mismatch: expected v{expected}, found v{found}",
            ),
            ChainDbError::InvalidOperation(detail) => {
                write!(f, "chaindb invalid operation: {detail}")
            }
        }
    }
}

impl std::error::Error for ChainDbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChainDbError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ChainDbError {
    fn from(e: std::io::Error) -> Self {
        ChainDbError::Io(e)
    }
}
