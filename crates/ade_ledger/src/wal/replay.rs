// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE WAL replay reducer (PHASE4-N-M-A S3 + S4).
//!
//! Pure replay of `(BootstrapAnchor + WAL entries 1..N)` against
//! the initial ledger + per-entry block bytes. The output is a
//! `Hash32` fingerprint that MUST equal `WAL[N].post_fp` (the
//! mechanical proof of DC-WAL-03 replay-equivalence).
//!
//! This slice ships only the fingerprint-chain replay (no actual
//! `block_validity` invocation per entry — that's sub-cluster B's
//! integration point). The replay's authority claim here is:
//! "given a chain of `(prior_fp, post_fp)` deltas anchored at the
//! initial ledger fingerprint, the final fingerprint equals the
//! WAL tail's `post_fp`." The block_bytes input is reserved for
//! the future sub-cluster B replay path that will call
//! `block_validity` per entry.

use std::collections::BTreeMap;

use ade_types::Hash32;

use super::error::WalError;
use super::event::WalEntry;

/// Pure replay reducer over an in-memory WAL entry vector +
/// per-block bytes map. Returns the final ledger fingerprint
/// (which MUST equal `entries.last().post_fp()` if the chain is
/// consistent — `verify_chain` is called inline).
///
/// `block_bytes` is keyed by `block_hash` from each
/// `WalEntry::AdmitBlock`. Empty map is acceptable IFF the WAL
/// is empty.
pub fn replay_from_anchor(
    anchor_initial_ledger_fp: &Hash32,
    entries: &[WalEntry],
    block_bytes: &BTreeMap<Hash32, Vec<u8>>,
) -> Result<Hash32, WalError> {
    // 1. Verify chain integrity.
    let mut prev_post_fp = anchor_initial_ledger_fp.clone();
    for (index, entry) in entries.iter().enumerate() {
        let prior = entry.prior_fp();
        if prior != prev_post_fp {
            return Err(WalError::ChainBreak {
                entry_index: index as u64,
                expected_prior_fp: prev_post_fp,
                actual_prior_fp: prior,
            });
        }
        // 2. Confirm the block bytes are available for this entry.
        match entry {
            WalEntry::AdmitBlock { block_hash, .. } => {
                if !block_bytes.contains_key(block_hash) {
                    return Err(WalError::BlockBytesMissing {
                        block_hash: block_hash.clone(),
                    });
                }
            }
        }
        prev_post_fp = entry.post_fp();
    }

    Ok(prev_post_fp)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::wal::event::BlockVerdictTag;
    use ade_types::SlotNo;

    fn mk_entry(
        prior_fp: u8,
        post_fp: u8,
        block_hash_byte: u8,
        slot: u64,
    ) -> WalEntry {
        WalEntry::AdmitBlock {
            prior_fp: Hash32([prior_fp; 32]),
            block_hash: Hash32([block_hash_byte; 32]),
            slot: SlotNo(slot),
            verdict: BlockVerdictTag::Valid,
            post_fp: Hash32([post_fp; 32]),
        }
    }

    fn block_bytes_map(block_hashes: &[u8]) -> BTreeMap<Hash32, Vec<u8>> {
        let mut m = BTreeMap::new();
        for b in block_hashes {
            m.insert(Hash32([*b; 32]), vec![*b]);
        }
        m
    }

    #[test]
    fn replay_from_anchor_empty_wal_returns_anchor_fp() {
        let anchor_fp = Hash32([0x42; 32]);
        let entries: Vec<WalEntry> = Vec::new();
        let bb = BTreeMap::new();
        let result = replay_from_anchor(&anchor_fp, &entries, &bb).expect("ok");
        assert_eq!(result, anchor_fp);
    }

    #[test]
    fn replay_from_anchor_three_entry_chain_ok() {
        let anchor_fp = Hash32([0x01; 32]);
        let entries = vec![
            mk_entry(0x01, 0x02, 0xA1, 100),
            mk_entry(0x02, 0x03, 0xA2, 101),
            mk_entry(0x03, 0x04, 0xA3, 102),
        ];
        let bb = block_bytes_map(&[0xA1, 0xA2, 0xA3]);
        let result = replay_from_anchor(&anchor_fp, &entries, &bb).expect("ok");
        assert_eq!(result, Hash32([0x04; 32]));
    }

    #[test]
    fn replay_from_anchor_catches_chain_break() {
        let anchor_fp = Hash32([0x01; 32]);
        let entries = vec![
            mk_entry(0x01, 0x02, 0xA1, 100),
            // Bad: prior_fp 0x99 instead of 0x02.
            mk_entry(0x99, 0x03, 0xA2, 101),
        ];
        let bb = block_bytes_map(&[0xA1, 0xA2]);
        let err = replay_from_anchor(&anchor_fp, &entries, &bb).expect_err("must fail");
        match err {
            WalError::ChainBreak { entry_index: 1, .. } => {}
            other => panic!("expected ChainBreak@1, got {other:?}"),
        }
    }

    #[test]
    fn replay_from_anchor_catches_missing_block_bytes() {
        let anchor_fp = Hash32([0x01; 32]);
        let entries = vec![
            mk_entry(0x01, 0x02, 0xA1, 100),
            mk_entry(0x02, 0x03, 0xA2, 101),
        ];
        // Only the first block's bytes are present.
        let bb = block_bytes_map(&[0xA1]);
        let err = replay_from_anchor(&anchor_fp, &entries, &bb).expect_err("must fail");
        match err {
            WalError::BlockBytesMissing { block_hash } => {
                assert_eq!(block_hash, Hash32([0xA2; 32]));
            }
            other => panic!("expected BlockBytesMissing, got {other:?}"),
        }
    }

    #[test]
    fn replay_from_anchor_two_runs_byte_identical() {
        let anchor_fp = Hash32([0xAA; 32]);
        let entries = vec![
            mk_entry(0xAA, 0xAB, 0xB1, 200),
            mk_entry(0xAB, 0xAC, 0xB2, 201),
        ];
        let bb = block_bytes_map(&[0xB1, 0xB2]);
        let a = replay_from_anchor(&anchor_fp, &entries, &bb).expect("a");
        let b = replay_from_anchor(&anchor_fp, &entries, &bb).expect("b");
        assert_eq!(a, b);
        assert_eq!(a, Hash32([0xAC; 32]));
    }
}
