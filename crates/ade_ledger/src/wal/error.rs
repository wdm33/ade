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
    /// PHASE4-N-F-A A3a: replay encountered a second
    /// `SeedEpochConsensusInputsImported` entry. Exactly one
    /// provenance entry is allowed per store/anchor; a duplicate
    /// is authority-fatal (fail closed).
    DuplicateProvenance,
    /// PHASE4-N-F-A A3a: a `SeedEpochConsensusInputsImported`
    /// entry's `anchor_fp` did not match the replay anchor's
    /// `initial_ledger_fingerprint`. The sidecar provenance is
    /// bound to a different anchor; fail closed.
    ProvenanceAnchorMismatch { expected: Hash32, actual: Hash32 },
    /// PHASE4-N-AI AI-S1: a `WalEntry::RollBack` named a `to_point`
    /// that is not an effective in-chain `AdmitBlock` point (by slot,
    /// then by hash via the replay re-anchor lookup). Rollback-to-anchor
    /// (rolling back the entire admitted chain) is out of scope for
    /// AI-S1 and also fails here. Authority-fatal (fail closed).
    RollbackTargetNotInChain { entry_index: u64, to_slot: u64 },
}
