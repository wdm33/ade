//! EPOCH-CONSENSUS-VIEW S3f-4d-1 (DC-EPOCH-08 substrate) — the activation SOURCE WINDOW and
//! the explicit source→target epoch mapping.
//!
//! The window that produces an activation candidate is NOT generically "epoch N". The
//! Cardano Mark→Set snapshot lag means the completed epoch whose blocks the window drives
//! over (`source_epoch`) is distinct from the epoch whose leadership reads the activated
//! view (`target_epoch`). This module names those roles and validates the window so a wrong
//! / incomplete / out-of-lineage range fails closed BEFORE it can produce a candidate.
//!
//! Source = `checkpoint commitment + a canonical, durable ChainDB range`. The range MUST be
//! pinned to the selected ChainDB lineage and the source epoch, and be complete (contiguous
//! by `prev_hash`), ordered (strictly increasing slot), and bounded (within the window).
//! NO peer/network read, wall-clock, or async side channel may influence it.

use ade_ledger::reduced_snapshot::SnapshotPhase;
use ade_types::{EpochNo, Hash32, SlotNo};

/// PROOF OBLIGATION (slice-entry, NOT a footnote): the exact Mark→Set leadership lag in
/// epochs. Leadership for `target_epoch` reads the Set snapshot, which is the Mark taken at
/// the boundary that produced the stake at the end of `source_epoch`. This single named
/// constant is the ONLY place the lag is encoded (never an inline `source + k`), so the
/// off-by-one is auditable in one place. Its value (2) is PINNED by the Cardano
/// snapshot-timing proof and CONFIRMED by the live leadership-schedule proof — until both
/// pass, the live flip (S3f-4d-3) stays gated.
pub const LEADERSHIP_SNAPSHOT_LAG_EPOCHS: u64 = 2;

/// The EXPLICIT `source_epoch` → `target_epoch` mapping (the Mark→Set lag). Leadership reads
/// the Set phase; deriving a target for any other phase is a caller error.
pub fn target_epoch_for_source(source_epoch: EpochNo, snapshot_phase: SnapshotPhase) -> Option<EpochNo> {
    if snapshot_phase != SnapshotPhase::Set {
        return None;
    }
    Some(EpochNo(source_epoch.0.saturating_add(LEADERSHIP_SNAPSHOT_LAG_EPOCHS)))
}

/// The named-role window that produces an activation candidate. Every role is explicit; the
/// `target_epoch` is derived by [`target_epoch_for_source`], never inline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationSourceWindow {
    /// The completed epoch whose admitted blocks the window drives over.
    pub source_epoch: EpochNo,
    /// The bounded, ordered durable ChainDB range (inclusive).
    pub source_window_start: SlotNo,
    pub source_window_end: SlotNo,
    /// The phase this window produces (Mark, which becomes Set for leadership).
    pub snapshot_phase: SnapshotPhase,
    /// The epoch whose LEADERSHIP reads the activated view (== target_epoch_for_source).
    pub target_epoch: EpochNo,
    /// The durable tip immediately BEFORE the window — the completeness anchor the first
    /// block's `prev_hash` must equal.
    pub source_window_anchor: Hash32,
    /// The durable tip OF the window — the selected-chain lineage pin the last block's hash
    /// must equal.
    pub lineage_pin: Hash32,
}

/// One block of the source window, in selected-chain order (slot + its hash + its parent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceWindowBlock {
    pub slot: SlotNo,
    pub hash: Hash32,
    pub prev_hash: Hash32,
}

/// Why a claimed source window is rejected (fail closed — never produces a candidate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceWindowError {
    /// The window has no blocks.
    Empty,
    /// A block's slot is outside `[source_window_start, source_window_end]`.
    OutOfWindow { slot: SlotNo },
    /// The blocks are not strictly increasing by slot.
    NotOrdered,
    /// A slot appears more than once.
    Duplicate { slot: SlotNo },
    /// The first block does not link to `source_window_anchor` (a gap at the window start).
    AnchorMismatch,
    /// A block's `prev_hash` does not link to the previous block (a gap / missing block).
    ChainGap { slot: SlotNo },
    /// The last block's hash is not the `lineage_pin` (the window is not pinned to the
    /// selected lineage tip).
    LineageMismatch,
    /// The window's `target_epoch` is not the explicit mapping of its `source_epoch`/phase.
    TargetEpochMismatch,
}

/// Validate a claimed source window against its [`ActivationSourceWindow`] bounds. The blocks
/// must be: non-empty; in `[start, end]`; strictly ordered by slot (no duplicates); a
/// CONTIGUOUS chain (`block[0].prev_hash == anchor`, `block[i].prev_hash == block[i-1].hash`)
/// so no block is missing; and the last block's hash == `lineage_pin`. The window's
/// `target_epoch` must equal the explicit source→target mapping. Any failure is fail-closed.
pub fn validate_source_window(
    window: &ActivationSourceWindow,
    blocks: &[SourceWindowBlock],
) -> Result<(), SourceWindowError> {
    // The window's target epoch MUST be the explicit mapping (no inline drift).
    match target_epoch_for_source(window.source_epoch, window.snapshot_phase) {
        Some(t) if t == window.target_epoch => {}
        _ => return Err(SourceWindowError::TargetEpochMismatch),
    }
    if blocks.is_empty() {
        return Err(SourceWindowError::Empty);
    }
    let mut prev: Option<&SourceWindowBlock> = None;
    for b in blocks {
        if b.slot.0 < window.source_window_start.0 || b.slot.0 > window.source_window_end.0 {
            return Err(SourceWindowError::OutOfWindow { slot: b.slot });
        }
        match prev {
            None => {
                // completeness anchor: the first block links to the pre-window tip.
                if b.prev_hash != window.source_window_anchor {
                    return Err(SourceWindowError::AnchorMismatch);
                }
            }
            Some(p) => {
                if b.slot.0 == p.slot.0 {
                    return Err(SourceWindowError::Duplicate { slot: b.slot });
                }
                if b.slot.0 < p.slot.0 {
                    return Err(SourceWindowError::NotOrdered);
                }
                // completeness: contiguous chain link (no missing block between p and b).
                if b.prev_hash != p.hash {
                    return Err(SourceWindowError::ChainGap { slot: b.slot });
                }
            }
        }
        prev = Some(b);
    }
    // pinned: the last block IS the selected-chain lineage tip.
    if blocks[blocks.len() - 1].hash != window.lineage_pin {
        return Err(SourceWindowError::LineageMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(b: u8) -> Hash32 {
        Hash32([b; 32])
    }

    fn window() -> ActivationSourceWindow {
        ActivationSourceWindow {
            source_epoch: EpochNo(575),
            source_window_start: SlotNo(100),
            source_window_end: SlotNo(200),
            snapshot_phase: SnapshotPhase::Set,
            target_epoch: EpochNo(577), // 575 + 2 (the explicit lag)
            source_window_anchor: h(0x00),
            lineage_pin: h(0x03),
        }
    }

    // a complete, ordered, bounded, pinned chain: anchor(00) <- 01 <- 02 <- 03(pin).
    fn good_blocks() -> Vec<SourceWindowBlock> {
        vec![
            SourceWindowBlock { slot: SlotNo(110), hash: h(0x01), prev_hash: h(0x00) },
            SourceWindowBlock { slot: SlotNo(150), hash: h(0x02), prev_hash: h(0x01) },
            SourceWindowBlock { slot: SlotNo(190), hash: h(0x03), prev_hash: h(0x02) },
        ]
    }

    #[test]
    fn target_epoch_is_the_explicit_lag() {
        assert_eq!(target_epoch_for_source(EpochNo(575), SnapshotPhase::Set), Some(EpochNo(577)));
        // any non-Set phase has no leadership target.
        assert_eq!(target_epoch_for_source(EpochNo(575), SnapshotPhase::Mark), None);
        assert_eq!(target_epoch_for_source(EpochNo(575), SnapshotPhase::Go), None);
    }

    #[test]
    fn valid_window_passes() {
        assert_eq!(validate_source_window(&window(), &good_blocks()), Ok(()));
    }

    #[test]
    fn empty_window_fails_closed() {
        assert_eq!(validate_source_window(&window(), &[]), Err(SourceWindowError::Empty));
    }

    #[test]
    fn out_of_window_block_fails_closed() {
        let mut b = good_blocks();
        b[1].slot = SlotNo(250); // past source_window_end
        assert_eq!(
            validate_source_window(&window(), &b),
            Err(SourceWindowError::OutOfWindow { slot: SlotNo(250) })
        );
    }

    #[test]
    fn unordered_and_duplicate_fail_closed() {
        // anchor + chain links OK up to the offending block, so the ORDERING check bites.
        let unordered = vec![
            SourceWindowBlock { slot: SlotNo(110), hash: h(0x01), prev_hash: h(0x00) },
            SourceWindowBlock { slot: SlotNo(105), hash: h(0x02), prev_hash: h(0x01) }, // slot < prev
        ];
        assert_eq!(
            validate_source_window(&window(), &unordered),
            Err(SourceWindowError::NotOrdered)
        );
        let duplicate = vec![
            SourceWindowBlock { slot: SlotNo(110), hash: h(0x01), prev_hash: h(0x00) },
            SourceWindowBlock { slot: SlotNo(110), hash: h(0x02), prev_hash: h(0x01) }, // same slot
        ];
        assert_eq!(
            validate_source_window(&window(), &duplicate),
            Err(SourceWindowError::Duplicate { slot: SlotNo(110) })
        );
    }

    #[test]
    fn missing_block_breaks_the_chain() {
        // drop the middle block -> 03.prev_hash(02) != 01.hash -> ChainGap.
        let b = vec![good_blocks()[0].clone(), good_blocks()[2].clone()];
        assert!(matches!(
            validate_source_window(&window(), &b),
            Err(SourceWindowError::ChainGap { .. })
        ));
    }

    #[test]
    fn anchor_and_lineage_pin_fail_closed() {
        let mut bad_anchor = good_blocks();
        bad_anchor[0].prev_hash = h(0xee); // does not link to the anchor
        assert_eq!(
            validate_source_window(&window(), &bad_anchor),
            Err(SourceWindowError::AnchorMismatch)
        );
        let mut bad_tip = good_blocks();
        bad_tip[2].hash = h(0xff); // last block != lineage_pin
        assert_eq!(
            validate_source_window(&window(), &bad_tip),
            Err(SourceWindowError::LineageMismatch)
        );
    }

    #[test]
    fn wrong_target_epoch_fails_closed() {
        let mut w = window();
        w.target_epoch = EpochNo(576); // not 575 + 2
        assert_eq!(
            validate_source_window(&w, &good_blocks()),
            Err(SourceWindowError::TargetEpochMismatch)
        );
    }
}
