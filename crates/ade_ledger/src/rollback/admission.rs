// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE rollback-ADMISSION guard (LIVE-LEDGER-EPOCH-TRANSITION S5).
//!
//! Recovery rematerializes the epoch accumulator + reduced checkpoint by
//! `reset_to_bootstrap` + re-fold of a canonical prefix. Before that re-fold, the
//! rollback target must be ADMISSIBLE — otherwise recovery could rematerialize
//! authoritative state from an inadmissible prefix (a split lineage, a rollback
//! deeper than the immutable point, or below the sealed anchor). This is the last
//! recovery safety rail before S4 promotes the accumulator to leadership authority;
//! it is NOT fork-choice (it admits or rejects one target, it does not select among
//! competing forks).
//!
//! `admit_rollback` is pure + total + fail-closed: an inadmissible target is a typed
//! [`RollbackAdmissionError`], never a silent rematerialization. The k-bound is a
//! BLOCK count (cardano `SecurityParam` k — the immutable/volatile split: candidates
//! never fork before the immutable tip = tip − k). The lineage check reads the
//! canonical chain's hash at the target slot through an injected reader (the reader's
//! IMPLEMENTATION is RED shell; the decision here is BLUE).

use ade_types::{BlockNo, Hash32, SlotNo};

/// A rollback endpoint: its slot, its block number (height, for the k-bound), and
/// its block hash (for the lineage check). `block_no` is the height cardano's
/// `SecurityParam` k bounds; `slot`/`hash` pin the canonical lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackPoint {
    pub slot: SlotNo,
    pub block_no: BlockNo,
    pub hash: Hash32,
}

/// Closed admission-error sum — every inadmissible rollback prefix is exactly one of
/// these. No `String`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackAdmissionError {
    /// The target is BELOW the sealed bootstrap anchor: recovery cannot rematerialize
    /// beneath the seed baseline (`reset_to_bootstrap` restores the anchor, and there
    /// is no canonical prefix before it).
    BeforeBootstrapAnchor {
        target_block_no: BlockNo,
        anchor_block_no: BlockNo,
    },
    /// The rollback depth (tip block − target block) exceeds `SecurityParam` k: the
    /// target is before the immutable point (tip − k). cardano's `ExceededRollback`.
    ExceededRollback {
        depth: u64,
        k: u64,
        tip_block_no: BlockNo,
        target_block_no: BlockNo,
    },
    /// The canonical chain's block hash at the target slot differs from the target
    /// hash: a same-height / divergent-lineage prefix. Never admitted (a height-only
    /// check would fold a new suffix onto a stale prefix — a split-lineage accumulator).
    LineageMismatch {
        slot: SlotNo,
        canonical: Hash32,
        target: Hash32,
    },
    /// The target slot carries no block on the canonical chain — it is not a selected
    /// point at all.
    TargetNotOnCanonicalChain {
        slot: SlotNo,
    },
}

/// BLUE, pure, total. Admit a rollback of the derived stores (epoch accumulator +
/// reduced checkpoint) from `tip` back to `target`, bounded by the bootstrap `anchor`
/// and `SecurityParam` `k`, and lineage-checked against the canonical chain via
/// `canonical_hash_at` (returns the canonical chain's block hash at a slot, or `None`
/// if that slot has no canonical block). MUST be called BEFORE any
/// `reset_to_bootstrap` + re-fold; on `Err` the caller fails closed and does NOT
/// rematerialize. Order: anchor floor, then k ceiling, then lineage — the cheapest
/// structural rejects first, the reader-backed check last.
pub fn admit_rollback<F>(
    tip: &RollbackPoint,
    target: &RollbackPoint,
    anchor: &RollbackPoint,
    k: u64,
    canonical_hash_at: F,
) -> Result<(), RollbackAdmissionError>
where
    F: Fn(SlotNo) -> Option<Hash32>,
{
    // 1. Not below the sealed bootstrap anchor.
    if target.block_no.0 < anchor.block_no.0 {
        return Err(RollbackAdmissionError::BeforeBootstrapAnchor {
            target_block_no: target.block_no,
            anchor_block_no: anchor.block_no,
        });
    }
    // 2. Within k blocks of the tip (target at/after the immutable point tip − k).
    //    saturating: a target ahead of the tip yields depth 0 (never "too deep"); a
    //    non-rollback (target == tip) is depth 0.
    let depth = tip.block_no.0.saturating_sub(target.block_no.0);
    if depth > k {
        return Err(RollbackAdmissionError::ExceededRollback {
            depth,
            k,
            tip_block_no: tip.block_no,
            target_block_no: target.block_no,
        });
    }
    // 3. Same-lineage: the canonical chain's hash at the target slot IS the target hash.
    match canonical_hash_at(target.slot) {
        Some(h) if h == target.hash => Ok(()),
        Some(canonical) => Err(RollbackAdmissionError::LineageMismatch {
            slot: target.slot,
            canonical,
            target: target.hash.clone(),
        }),
        None => Err(RollbackAdmissionError::TargetNotOnCanonicalChain { slot: target.slot }),
    }
}

/// What recovery must do to reconcile the persisted accumulator to the durable canonical chain — the BLUE
/// decision the shell executes (S5 step 2b). NEVER "the slot is close enough": either the accumulator is
/// certified to the exact canonical point (forward-fold), or it is reset + re-materialized from canonical
/// blocks, or recovery fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// The accumulator is lineage-certified AND at/behind the durable tip on the SAME lineage — fold forward
    /// only; no reset.
    ForwardFold,
    /// Reset to bootstrap + re-fold the canonical prefix — either an uncertified accumulator (no anchor), or
    /// a certified accumulator that over-advanced past an admissibly-shortened durable chain. The shell then
    /// re-folds from canonical blocks (fail-closed on a missing / non-contiguous span) and re-writes a fresh
    /// `LastAdvancedPoint` on success.
    ResetAndRefold,
}

/// BLUE, pure, total. Decide how recovery reconciles the persisted accumulator to the durable canonical
/// chain, given its lineage anchor (`None` = uncertified), the canonical point actually at the anchor's slot
/// (`durable_at_anchor`, resolved from the ChainDB by the shell), the durable `tip`, the bootstrap `anchor`
/// (seed), and `SecurityParam` `k`. Replaces the height-only reset:
/// - no anchor -> `ResetAndRefold` (uncertified; re-materialize from canonical);
/// - anchor present but the durable chain no longer carries it at that slot (absent / different hash) ->
///   fail closed (`LineageMismatch` / `TargetNotOnCanonicalChain`): a certified store contradicted by the
///   canonical chain is NOT silently reset;
/// - anchor present + still canonical + tip at/ahead -> `ForwardFold`;
/// - anchor present + still canonical + tip BEHIND (durable chain shortened) -> `admit_rollback` the bounded
///   rollback to the tip; admitted -> `ResetAndRefold`, else fail closed (`ExceededRollback` /
///   `BeforeBootstrapAnchor`).
pub fn reconcile_recovery<F>(
    anchor: Option<&RollbackPoint>,
    durable_at_anchor: Option<&RollbackPoint>,
    tip: &RollbackPoint,
    seed: &RollbackPoint,
    k: u64,
    canonical_hash_at: F,
) -> Result<RecoveryAction, RollbackAdmissionError>
where
    F: Fn(SlotNo) -> Option<Hash32>,
{
    let Some(anchor) = anchor else {
        // Uncertified (legacy pre-anchor store, or the transitional state after reset_to_bootstrap): recovery
        // does NOT trust height — reset + re-fold from canonical blocks.
        return Ok(RecoveryAction::ResetAndRefold);
    };
    // The accumulator's committed point MUST still be the canonical block at that slot.
    match durable_at_anchor {
        Some(dp)
            if dp.slot == anchor.slot
                && dp.block_no == anchor.block_no
                && dp.hash == anchor.hash =>
        {
            if tip.block_no.0 >= anchor.block_no.0 {
                // Certified, on-lineage, at/behind the tip -> fold forward (no rollback).
                Ok(RecoveryAction::ForwardFold)
            } else {
                // The durable chain shortened below the accumulator -> admit the bounded rollback to the tip.
                admit_rollback(anchor, tip, seed, k, canonical_hash_at)?;
                Ok(RecoveryAction::ResetAndRefold)
            }
        }
        // Present but contradicted by the canonical chain -> fail closed (never a silent reset of a
        // committed store).
        Some(dp) => Err(RollbackAdmissionError::LineageMismatch {
            slot: anchor.slot,
            canonical: dp.hash.clone(),
            target: anchor.hash.clone(),
        }),
        None => Err(RollbackAdmissionError::TargetNotOnCanonicalChain { slot: anchor.slot }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn pt(slot: u64, block: u64, hash: u8) -> RollbackPoint {
        RollbackPoint {
            slot: SlotNo(slot),
            block_no: BlockNo(block),
            hash: Hash32([hash; 32]),
        }
    }

    // The canonical chain oracle for the tests: block `b` (fill byte) sits at slot `b*10`.
    fn canonical(slot: SlotNo) -> Option<Hash32> {
        if slot.0 % 10 == 0 && slot.0 > 0 {
            Some(Hash32([(slot.0 / 10) as u8; 32]))
        } else {
            None
        }
    }

    const K: u64 = 5;
    // anchor = block 1 @ slot 10 hash 0x01.
    fn anchor() -> RollbackPoint {
        pt(10, 1, 0x01)
    }

    #[test]
    fn admissible_same_lineage_within_k_is_ok() {
        // tip block 8 @ slot 80; target block 4 @ slot 40 hash 0x04 (depth 4 <= k=5, on-lineage).
        let tip = pt(80, 8, 0x08);
        let target = pt(40, 4, 0x04);
        assert_eq!(admit_rollback(&tip, &target, &anchor(), K, canonical), Ok(()));
    }

    #[test]
    fn depth_exactly_k_is_admissible() {
        // tip block 8, target block 3: depth 5 == k -> admissible (immutable point is inclusive).
        let tip = pt(80, 8, 0x08);
        let target = pt(30, 3, 0x03);
        assert_eq!(admit_rollback(&tip, &target, &anchor(), K, canonical), Ok(()));
    }

    #[test]
    fn rollback_before_bootstrap_anchor_is_typed() {
        // target block 0 < anchor block 1.
        let tip = pt(80, 8, 0x08);
        let target = pt(0, 0, 0x00);
        assert_eq!(
            admit_rollback(&tip, &target, &anchor(), K, canonical),
            Err(RollbackAdmissionError::BeforeBootstrapAnchor {
                target_block_no: BlockNo(0),
                anchor_block_no: BlockNo(1),
            })
        );
    }

    #[test]
    fn rollback_beyond_k_is_typed_exceeded() {
        // tip block 8, target block 2: depth 6 > k=5 -> ExceededRollback (before immutable point).
        let tip = pt(80, 8, 0x08);
        let target = pt(20, 2, 0x02);
        assert_eq!(
            admit_rollback(&tip, &target, &anchor(), K, canonical),
            Err(RollbackAdmissionError::ExceededRollback {
                depth: 6,
                k: 5,
                tip_block_no: BlockNo(8),
                target_block_no: BlockNo(2),
            })
        );
    }

    #[test]
    fn same_height_wrong_hash_is_typed_lineage_mismatch() {
        // target block 4 @ slot 40 but hash 0xEE != canonical 0x04 -> divergent lineage.
        let tip = pt(80, 8, 0x08);
        let target = pt(40, 4, 0xEE);
        assert_eq!(
            admit_rollback(&tip, &target, &anchor(), K, canonical),
            Err(RollbackAdmissionError::LineageMismatch {
                slot: SlotNo(40),
                canonical: Hash32([0x04; 32]),
                target: Hash32([0xEE; 32]),
            })
        );
    }

    #[test]
    fn target_slot_not_on_canonical_chain_is_typed() {
        // slot 44 has no canonical block (44 % 10 != 0).
        let tip = pt(80, 8, 0x08);
        let target = pt(44, 4, 0x04);
        assert_eq!(
            admit_rollback(&tip, &target, &anchor(), K, canonical),
            Err(RollbackAdmissionError::TargetNotOnCanonicalChain { slot: SlotNo(44) })
        );
    }

    #[test]
    fn anchor_floor_is_checked_before_k_ceiling() {
        // A target both below the anchor AND beyond k: the anchor floor wins (cheapest reject).
        let tip = pt(80, 8, 0x08);
        let target = pt(0, 0, 0x00);
        assert!(matches!(
            admit_rollback(&tip, &target, &anchor(), K, canonical),
            Err(RollbackAdmissionError::BeforeBootstrapAnchor { .. })
        ));
    }

    #[test]
    fn non_rollback_target_equal_tip_is_admissible() {
        // target == tip (depth 0) on-lineage -> Ok (recovery to the current tip is a no-op rollback).
        let tip = pt(80, 8, 0x08);
        assert_eq!(admit_rollback(&tip, &tip.clone(), &anchor(), K, canonical), Ok(()));
    }

    // ----- reconcile_recovery: the recovery decision (S5 2b) -----

    #[test]
    fn reconcile_absent_anchor_resets_and_refolds() {
        // Uncertified -> reset + re-fold from canonical (never trust height).
        let tip = pt(80, 8, 0x08);
        assert_eq!(
            reconcile_recovery(None, None, &tip, &anchor(), K, canonical),
            Ok(RecoveryAction::ResetAndRefold)
        );
    }

    #[test]
    fn reconcile_certified_on_lineage_tip_ahead_forward_folds() {
        // anchor block 4; durable-at-anchor matches; tip block 8 (ahead) -> forward-fold, no reset.
        let anc = pt(40, 4, 0x04);
        let tip = pt(80, 8, 0x08);
        assert_eq!(
            reconcile_recovery(Some(&anc), Some(&anc), &tip, &anchor(), K, canonical),
            Ok(RecoveryAction::ForwardFold)
        );
    }

    #[test]
    fn reconcile_certified_tip_behind_within_k_resets_and_refolds() {
        // anchor block 8; tip block 4 (behind by 4 <= k=5), on-lineage -> admissible bounded rollback.
        let anc = pt(80, 8, 0x08);
        let dtip = pt(40, 4, 0x04);
        assert_eq!(
            reconcile_recovery(Some(&anc), Some(&anc), &dtip, &anchor(), K, canonical),
            Ok(RecoveryAction::ResetAndRefold)
        );
    }

    #[test]
    fn reconcile_certified_tip_behind_beyond_k_fails_closed() {
        // anchor block 8; tip block 2 (behind by 6 > k=5) -> ExceededRollback.
        let anc = pt(80, 8, 0x08);
        let dtip = pt(20, 2, 0x02);
        assert!(matches!(
            reconcile_recovery(Some(&anc), Some(&anc), &dtip, &anchor(), K, canonical),
            Err(RollbackAdmissionError::ExceededRollback { .. })
        ));
    }

    #[test]
    fn reconcile_anchor_contradicted_by_canonical_fails_closed() {
        // The committed anchor (block 4 @ slot 40 hash 0x04) is NOT the durable block at slot 40 (hash 0xEE)
        // -> LineageMismatch (a certified store contradicted by canonical is never silently reset).
        let anc = pt(40, 4, 0x04);
        let durable = pt(40, 4, 0xEE);
        let tip = pt(80, 8, 0x08);
        assert!(matches!(
            reconcile_recovery(Some(&anc), Some(&durable), &tip, &anchor(), K, canonical),
            Err(RollbackAdmissionError::LineageMismatch { .. })
        ));
    }

    #[test]
    fn reconcile_anchor_absent_from_canonical_chain_fails_closed() {
        // The anchor's slot no longer carries any durable block -> TargetNotOnCanonicalChain.
        let anc = pt(40, 4, 0x04);
        let tip = pt(80, 8, 0x08);
        assert!(matches!(
            reconcile_recovery(Some(&anc), None, &tip, &anchor(), K, canonical),
            Err(RollbackAdmissionError::TargetNotOnCanonicalChain { .. })
        ));
    }
}
