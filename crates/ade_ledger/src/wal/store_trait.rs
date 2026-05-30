// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `WalStore` trait — append-only by type (PHASE4-N-M-A S3).
//!
//! CN-WAL-01: `append` is the SOLE mutation method on any
//! `WalStore` impl. There is no `truncate` / `rewrite` / `replace`
//! / `delete` / `clear`. DC-WAL-01 makes the append-only property
//! a compile-time property of the trait surface, not a runtime
//! convention.

use ade_types::Hash32;

use super::error::WalError;
use super::event::WalEntry;

/// Append-only WAL store. Trait is intentionally narrow: three
/// methods, none of which mutate or remove prior entries.
///
/// - `append(entry)` writes one entry; durable after return.
/// - `read_all()` returns every entry in append order.
/// - `verify_chain(anchor_fp)` walks the entries and asserts
///   each `prior_fp` matches the previous `post_fp` (or
///   `anchor_fp` for the first entry).
///
/// **Adding a method named `truncate`, `rewrite`, `replace`,
/// `delete`, or `clear` is a CI failure** (see
/// `ci/ci_check_wal_append_only.sh`).
pub trait WalStore: Send + Sync {
    /// Append one entry. Implementations MUST durably flush
    /// before returning Ok (or fail loud).
    fn append(&mut self, entry: WalEntry) -> Result<(), WalError>;

    /// Return every entry in append order. Implementations may
    /// stream lazily but the resulting iterator MUST yield
    /// entries in the order they were appended.
    fn read_all(&self) -> Result<Vec<WalEntry>, WalError>;

    /// Walk the WAL and verify the `AdmitBlock` fingerprint chain.
    /// `anchor_fp` is the `BootstrapAnchor::initial_ledger_fingerprint`
    /// (or whatever was the last `post_fp` if the WAL was
    /// resumed from a snapshot). Returns Ok iff every `AdmitBlock`
    /// entry's `prior_fp` equals the previous `AdmitBlock`'s
    /// `post_fp` (or `anchor_fp` for the first one).
    ///
    /// `SeedEpochConsensusInputsImported` entries (PHASE4-N-F-A
    /// A3a) are **transparent** to this walk: they are bootstrap
    /// provenance events, not block transitions, so they neither
    /// advance nor break the fingerprint chain. The explicit
    /// `match` keeps that distinction a compile-time property — a
    /// future entry variant forces this walk to be revisited.
    fn verify_chain(&self, anchor_fp: &Hash32) -> Result<(), WalError> {
        let entries = self.read_all()?;
        let mut prev_post_fp: Hash32 = anchor_fp.clone();
        for (index, entry) in entries.iter().enumerate() {
            match entry {
                WalEntry::AdmitBlock {
                    prior_fp, post_fp, ..
                } => {
                    if *prior_fp != prev_post_fp {
                        return Err(WalError::ChainBreak {
                            entry_index: index as u64,
                            expected_prior_fp: prev_post_fp,
                            actual_prior_fp: prior_fp.clone(),
                        });
                    }
                    prev_post_fp = post_fp.clone();
                }
                WalEntry::SeedEpochConsensusInputsImported { .. } => {
                    // Not part of the block-transition chain.
                }
            }
        }
        Ok(())
    }
}
