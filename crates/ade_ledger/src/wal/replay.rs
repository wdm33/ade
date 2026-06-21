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

use ade_types::{EpochNo, Hash32};

use super::error::WalError;
use super::event::WalEntry;

/// Provenance recovered from WAL for bootstrap-associated sidecar state.
/// This is not a `BootstrapAnchor`; it proves which sidecar hash belongs to
/// the replay anchor for the seed epoch.
///
/// Produced by `replay_from_anchor` when the WAL contains a
/// `SeedEpochConsensusInputsImported` entry (PHASE4-N-F-A A3a).
/// Warm-start (A3b) consumes this view to locate + verify the
/// persisted sidecar; A3a only reconstructs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredBootstrapProvenance {
    pub anchor_fp: Hash32,
    pub sidecar_hash: Hash32,
    pub epoch_no: EpochNo,
}

/// Result of a pure WAL replay (PHASE4-N-F-A A3a). Carries the
/// block-transition fingerprint tail (the AdmitBlock-chain
/// authority claim, unchanged from the single-variant era), plus
/// any reconstructed bootstrap-sidecar provenance and a count of
/// block transitions actually replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayOutcome {
    /// The final `post_fp` of the `AdmitBlock` chain (or the
    /// anchor fp if no `AdmitBlock` entries were present). MUST
    /// equal the last `AdmitBlock`'s `post_fp`.
    pub tail_fp: Hash32,
    /// The reconstructed seed-epoch sidecar provenance, if the
    /// WAL recorded an import. `None` when the WAL has no
    /// provenance entry (e.g. a fresh cold-start before any
    /// import).
    pub provenance: Option<RecoveredBootstrapProvenance>,
    /// Number of `AdmitBlock` entries replayed. A WAL may contain
    /// a provenance entry and zero block transitions, so
    /// "entries non-empty" no longer implies "a block was
    /// admitted" — callers that need the latter MUST check this.
    pub admit_count: u64,
}

/// Pure replay reducer over an in-memory WAL entry vector +
/// per-block bytes map. Returns a [`ReplayOutcome`]: the
/// `AdmitBlock`-chain fingerprint tail (which MUST equal the last
/// `AdmitBlock`'s `post_fp` if the chain is consistent) plus any
/// reconstructed [`RecoveredBootstrapProvenance`].
///
/// Two semantic classes are folded here and kept distinct:
///
/// - `AdmitBlock` — a block/ledger transition. Its `prior_fp`
///   MUST equal the previous `AdmitBlock`'s `post_fp` (or the
///   anchor fp for the first one); its block bytes MUST be
///   present in `block_bytes`. This logic is unchanged from the
///   single-variant WAL.
/// - `SeedEpochConsensusInputsImported` — a bootstrap provenance
///   event. It does **not** participate in the
///   `prior_fp`/`post_fp` continuity and requires no block bytes.
///   It updates the recovered provenance view. Exactly one is
///   allowed: a duplicate is `DuplicateProvenance`; an
///   `anchor_fp` that mismatches `anchor_initial_ledger_fp` is
///   `ProvenanceAnchorMismatch`. Both fail closed.
///
/// `block_bytes` is keyed by `block_hash` from each
/// `WalEntry::AdmitBlock`. Empty map is acceptable IFF the WAL
/// has no `AdmitBlock` entries.
pub fn replay_from_anchor(
    anchor_initial_ledger_fp: &Hash32,
    entries: &[WalEntry],
    block_bytes: &BTreeMap<Hash32, Vec<u8>>,
) -> Result<ReplayOutcome, WalError> {
    // Pre-pass (PHASE4-N-AI AI-S1): determine which AdmitBlock entries
    // a later RollBack supersedes, and validate each RollBack target is
    // an active in-chain point (by slot). fp-ONLY — no materialize here;
    // the recovery/materialize layer re-invokes
    // `materialize_rolled_back_state` (AI-S3 / the Layer-2 hermetic test).
    let superseded = compute_superseded(entries)?;

    let mut prev_post_fp = anchor_initial_ledger_fp.clone();
    let mut provenance: Option<RecoveredBootstrapProvenance> = None;
    let mut admit_count: u64 = 0;
    // Effective AdmitBlock point (slot, hash) -> post_fp, for the
    // RollBack re-anchor lookup.
    let mut point_fp: BTreeMap<(u64, [u8; 32]), Hash32> = BTreeMap::new();

    for (index, entry) in entries.iter().enumerate() {
        match entry {
            WalEntry::AdmitBlock {
                prior_fp,
                block_hash,
                slot,
                post_fp,
                ..
            } => {
                // Superseded by a later RollBack: abandoned. Its bytes
                // are NOT required and it does not advance the fp chain.
                if superseded[index] {
                    continue;
                }
                // Block-transition chain link (unchanged authority).
                if *prior_fp != prev_post_fp {
                    return Err(WalError::ChainBreak {
                        entry_index: index as u64,
                        expected_prior_fp: prev_post_fp,
                        actual_prior_fp: prior_fp.clone(),
                    });
                }
                if !block_bytes.contains_key(block_hash) {
                    return Err(WalError::BlockBytesMissing {
                        block_hash: block_hash.clone(),
                    });
                }
                point_fp.insert((slot.0, block_hash.0), post_fp.clone());
                prev_post_fp = post_fp.clone();
                admit_count += 1;
            }
            WalEntry::RollBack { to_point, .. } => {
                // Re-anchor the fp chain to the EXISTING in-chain
                // `post_fp` at `to_point` (an already-verified
                // fingerprint, NOT a recorded rollback fp). fp-ONLY:
                // this does NOT materialize state. `selected_tip` /
                // `prior_tip` are audit fields and are NOT consulted for
                // the re-anchor (no durable tip is set from metadata).
                let key = (to_point.slot.0, to_point.hash.0);
                match point_fp.get(&key) {
                    Some(fp) => prev_post_fp = fp.clone(),
                    None => {
                        return Err(WalError::RollbackTargetNotInChain {
                            entry_index: index as u64,
                            to_slot: to_point.slot.0,
                        })
                    }
                }
            }
            WalEntry::SeedEpochConsensusInputsImported {
                anchor_fp,
                sidecar_hash,
                epoch_no,
            } => {
                // Provenance event: NOT part of the fingerprint
                // chain. `prev_post_fp` is left untouched so an
                // `AdmitBlock` after this entry still links to the
                // previous `AdmitBlock`'s `post_fp`.
                if provenance.is_some() {
                    return Err(WalError::DuplicateProvenance);
                }
                if *anchor_fp != *anchor_initial_ledger_fp {
                    return Err(WalError::ProvenanceAnchorMismatch {
                        expected: anchor_initial_ledger_fp.clone(),
                        actual: anchor_fp.clone(),
                    });
                }
                provenance = Some(RecoveredBootstrapProvenance {
                    anchor_fp: anchor_fp.clone(),
                    sidecar_hash: sidecar_hash.clone(),
                    epoch_no: *epoch_no,
                });
            }
            WalEntry::EpochConsensusViewActivated { .. } => {
                // EPOCH-CONSENSUS-VIEW S3f-4a: a non-transition activation record (no
                // prior_fp/post_fp), so it does NOT advance the fingerprint chain. The
                // replay APPLICATION of the idempotence/conflict rule
                // (`activation_replay_outcome`) + republishing the active view is wired in
                // S3f-4c; here the entry is a chain no-op.
            }
        }
    }

    Ok(ReplayOutcome {
        tail_fp: prev_post_fp,
        provenance,
        admit_count,
    })
}

/// Pre-pass (PHASE4-N-AI AI-S1): for each `RollBack`, mark the
/// `AdmitBlock` entries it supersedes (the abandoned branch above
/// `to_point`) and validate the target is an active in-chain point by
/// slot. Pure, deterministic; no materialize. The authoritative
/// (slot, hash) re-anchor check happens in the main walk via `point_fp`.
///
/// Rollback-to-anchor (an empty active stack at the target) is out of
/// scope for AI-S1 and fails closed (`RollbackTargetNotInChain`).
pub(crate) fn compute_superseded(entries: &[WalEntry]) -> Result<Vec<bool>, WalError> {
    let mut superseded = vec![false; entries.len()];
    // Stack of active (effective) AdmitBlock (entry_index, slot).
    let mut active: Vec<(usize, u64)> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        match entry {
            WalEntry::AdmitBlock { slot, .. } => active.push((i, slot.0)),
            WalEntry::RollBack { to_point, .. } => {
                while let Some(&(idx, s)) = active.last() {
                    if s > to_point.slot.0 {
                        superseded[idx] = true;
                        active.pop();
                    } else {
                        break;
                    }
                }
                match active.last() {
                    Some(&(_, s)) if s == to_point.slot.0 => {}
                    _ => {
                        return Err(WalError::RollbackTargetNotInChain {
                            entry_index: i as u64,
                            to_slot: to_point.slot.0,
                        })
                    }
                }
            }
            WalEntry::SeedEpochConsensusInputsImported { .. } => {}
            WalEntry::EpochConsensusViewActivated { .. } => {}
        }
    }
    Ok(superseded)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::wal::event::BlockVerdictTag;
    use ade_types::SlotNo;

    fn mk_entry(prior_fp: u8, post_fp: u8, block_hash_byte: u8, slot: u64) -> WalEntry {
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

    fn mk_provenance(anchor_fp: u8, sidecar_hash: u8, epoch: u64) -> WalEntry {
        WalEntry::SeedEpochConsensusInputsImported {
            anchor_fp: Hash32([anchor_fp; 32]),
            sidecar_hash: Hash32([sidecar_hash; 32]),
            epoch_no: EpochNo(epoch),
        }
    }

    #[test]
    fn replay_from_anchor_empty_wal_returns_anchor_fp() {
        let anchor_fp = Hash32([0x42; 32]);
        let entries: Vec<WalEntry> = Vec::new();
        let bb = BTreeMap::new();
        let result = replay_from_anchor(&anchor_fp, &entries, &bb).expect("ok");
        assert_eq!(result.tail_fp, anchor_fp);
        assert_eq!(result.provenance, None);
        assert_eq!(result.admit_count, 0);
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
        assert_eq!(result.tail_fp, Hash32([0x04; 32]));
        assert_eq!(result.admit_count, 3);
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
        assert_eq!(a.tail_fp, Hash32([0xAC; 32]));
    }

    // --- PHASE4-N-F-A A3a: provenance entry replay ---

    #[test]
    fn replay_yields_bootstrap_provenance_view() {
        // A WAL with a single provenance entry (no block
        // transitions): replay returns the typed view, the
        // fingerprint tail stays at the anchor (no chain
        // movement), and admit_count is 0.
        let anchor_fp = Hash32([0x01; 32]);
        let entries = vec![mk_provenance(0x01, 0xEE, 576)];
        let bb = BTreeMap::new();
        let out = replay_from_anchor(&anchor_fp, &entries, &bb).expect("ok");
        assert_eq!(out.tail_fp, anchor_fp);
        assert_eq!(out.admit_count, 0);
        assert_eq!(
            out.provenance,
            Some(RecoveredBootstrapProvenance {
                anchor_fp: Hash32([0x01; 32]),
                sidecar_hash: Hash32([0xEE; 32]),
                epoch_no: EpochNo(576),
            })
        );
    }

    #[test]
    fn admit_block_chain_unaffected_by_provenance_entry() {
        // A provenance entry interleaved between AdmitBlock
        // entries must NOT break the prior_fp/post_fp continuity:
        // the AdmitBlock after it still links to the previous
        // AdmitBlock's post_fp (0x02), not to anything the
        // provenance entry carries.
        let anchor_fp = Hash32([0x01; 32]);
        let entries = vec![
            mk_entry(0x01, 0x02, 0xA1, 100),
            mk_provenance(0x01, 0xEE, 576),
            mk_entry(0x02, 0x03, 0xA2, 101),
        ];
        let bb = block_bytes_map(&[0xA1, 0xA2]);
        let out = replay_from_anchor(&anchor_fp, &entries, &bb).expect("ok");
        assert_eq!(out.tail_fp, Hash32([0x03; 32]));
        assert_eq!(out.admit_count, 2);
        assert!(out.provenance.is_some());
    }

    #[test]
    fn replay_rejects_duplicate_provenance_entry() {
        let anchor_fp = Hash32([0x01; 32]);
        let entries = vec![
            mk_provenance(0x01, 0xEE, 576),
            mk_provenance(0x01, 0xEF, 576),
        ];
        let bb = BTreeMap::new();
        let err = replay_from_anchor(&anchor_fp, &entries, &bb).expect_err("must fail");
        assert!(matches!(err, WalError::DuplicateProvenance), "got {err:?}");
    }

    #[test]
    fn replay_rejects_anchor_mismatched_provenance_entry() {
        let anchor_fp = Hash32([0x01; 32]);
        // anchor_fp on the entry (0x99) != replay anchor (0x01).
        let entries = vec![mk_provenance(0x99, 0xEE, 576)];
        let bb = BTreeMap::new();
        let err = replay_from_anchor(&anchor_fp, &entries, &bb).expect_err("must fail");
        match err {
            WalError::ProvenanceAnchorMismatch { expected, actual } => {
                assert_eq!(expected, Hash32([0x01; 32]));
                assert_eq!(actual, Hash32([0x99; 32]));
            }
            other => panic!("expected ProvenanceAnchorMismatch, got {other:?}"),
        }
    }
}
