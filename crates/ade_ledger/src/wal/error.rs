// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE closed WAL error sum (PHASE4-N-M-A S3).

use ade_codec::CodecError;
use ade_types::Hash32;
use std::io;

/// Closed error sum for `WalStore` operations + entry codec +
/// chain verification.
#[derive(Debug)]
pub enum WalError {
    /// Underlying IO failure on the storage backend.
    Io(io::ErrorKind),
    /// CBOR primitive read/write failed during entry codec.
    Decode(CodecError),
    /// Entry structurally malformed (wrong array length, unknown
    /// tag, short hash, ...).
    Structural { reason: &'static str },
    /// `verify_chain` walked the WAL and found an entry whose
    /// `prior_fp` did not match the previous entry's `post_fp`
    /// (or the anchor's `initial_ledger_fingerprint` for the
    /// first entry). DC-WAL-02. Authority-fatal.
    ChainBreak {
        entry_index: u64,
        expected_prior_fp: Hash32,
        actual_prior_fp: Hash32,
    },
    /// A WAL entry referenced a slot for which the block-bytes
    /// map (passed to `replay_from_anchor`) had no entry.
    BlockBytesMissing { block_hash: Hash32 },
    /// Stored entry's CRC did not match its bytes.
    CorruptCrc { file: String },
}
