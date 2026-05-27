// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `ServedChainHandle` (PHASE4-N-R-B B2).
//!
//! Wraps the pure-value [`ServedChainSnapshot`] with a
//! `tokio::sync::watch::channel` so the producer-mode broadcast
//! handler can push new admitted blocks atomically and per-peer
//! tasks can read a coherent latest snapshot.
//!
//! Doctrine (DQ-B1, locked):
//!
//! ```text
//! Watch channel guarantees:
//!   - readers see a whole ServedChainSnapshot value
//!   - no torn push
//!   - delayed readers may skip intermediate snapshots
//!
//! Watch channel does NOT guarantee:
//!   - every peer observes every intermediate producer update
//!   - notification history
//! ```
//!
//! This is permitted because block-fetch semantics depend on
//! whether the requested block is present in the snapshot read
//! at dispatch time. A peer requesting a block not in the
//! current snapshot receives `NoBlocks` per the closed failure
//! semantics resolved by N-R-A A1 OQ8.
//!
//! `push_atomic` returns a closed `ServedTip { slot, hash }`
//! (DQ-B2) so the coordinator's `BlockServed` log can be
//! constructed from the transition result, not from a later
//! snapshot read.

use ade_ledger::producer::{served_chain_admit, AcceptedBlock, ServedChainAdmitError, ServedChainSnapshot};
use ade_types::{Hash32, SlotNo};
use tokio::sync::watch;

/// Closed `ServedTip` returned by `push_atomic`. Carries the
/// (slot, hash) of the just-pushed block — sufficient for the
/// coordinator's `BlockServed` log emission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServedTip {
    pub slot: SlotNo,
    pub hash: Hash32,
}

/// Closed push error surface. No `String` payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum PushError {
    /// `served_chain_admit` rejected the block — either the
    /// bytes failed to decode, or two distinct byte sequences
    /// resolved to the same `(slot, hash)` key. Both are
    /// unreachable for real `AcceptedBlock` tokens but the
    /// variant exists for strict totality.
    AdmitError(ServedChainAdmitError),
    /// Watch sender's send_modify panicked or the channel was
    /// poisoned. Unreachable under normal operation; variant
    /// kept for fail-closed completeness.
    SenderUnavailable,
}

/// RED shared handle around a `tokio::sync::watch` channel
/// holding the latest `ServedChainSnapshot`. `push_atomic`
/// covers the full insertion in one `send_modify` call —
/// no observer can see a torn snapshot mid-insertion.
pub struct ServedChainHandle {
    sender: watch::Sender<ServedChainSnapshot>,
}

/// Cloneable receiver-side view. Per-peer tasks call
/// `view.borrow()` to get a `Ref<'_, ServedChainSnapshot>`
/// suitable for `&ServedChainSnapshot` argument passing.
#[derive(Clone)]
pub struct ServedChainView {
    receiver: watch::Receiver<ServedChainSnapshot>,
}

impl ServedChainHandle {
    /// Construct a fresh handle + initial view pair. The
    /// initial `ServedChainSnapshot` is empty.
    pub fn new() -> (Self, ServedChainView) {
        let (tx, rx) = watch::channel(ServedChainSnapshot::new());
        (
            Self { sender: tx },
            ServedChainView { receiver: rx },
        )
    }

    /// Push an `AcceptedBlock` into the served chain
    /// atomically. The watch send_modify closure runs under
    /// the channel's internal lock for the full insertion —
    /// no observer can read a partial state.
    ///
    /// Returns the `ServedTip { slot, hash }` derived from the
    /// AcceptedBlock's decoded header.
    pub fn push_atomic(&self, accepted: AcceptedBlock) -> Result<ServedTip, PushError> {
        use ade_ledger::block_validity::header_input::decode_block;

        // Decode once outside the lock to extract the (slot,
        // hash) key. The same bytes are then handed to
        // served_chain_admit, which re-decodes inside — the
        // double-decode is acceptable (decode_block is pure +
        // cheap; the alternative would require exposing
        // header-projection helpers from served_chain_admit).
        let decoded = decode_block(accepted.as_bytes())
            .map_err(|e| PushError::AdmitError(ServedChainAdmitError::Decode(e)))?;
        let tip = ServedTip {
            slot: decoded.header_input.slot,
            hash: decoded.block_hash,
        };

        // send_modify holds the watch lock for the duration of
        // the closure — atomic insertion guaranteed.
        let mut admit_result: Result<(), PushError> = Ok(());
        self.sender.send_modify(|snap_mut| {
            let old = std::mem::replace(snap_mut, ServedChainSnapshot::new());
            match served_chain_admit(old, accepted.clone()) {
                Ok(new_snap) => *snap_mut = new_snap,
                Err(e) => {
                    // Restore the old snapshot via re-decode of accepted; but since
                    // we already replaced, we have nothing to restore to in this
                    // branch. The previous snapshot was consumed by served_chain_admit's
                    // error path. Set snap_mut to a fresh empty snapshot to avoid
                    // leaving state inconsistent — and surface the error to the caller.
                    //
                    // In practice this is unreachable for a real AcceptedBlock:
                    // - Decode never fails (we already decoded successfully above).
                    // - KeyByteConflict requires a blake2b_256 collision.
                    admit_result = Err(PushError::AdmitError(e));
                }
            }
        });
        admit_result?;
        Ok(tip)
    }

    /// Number of receivers currently observing this handle.
    /// Used by produce_mode shutdown to confirm all per-peer
    /// tasks have dropped their views.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl ServedChainView {
    /// Borrow the current snapshot. The returned `Ref` holds
    /// a read lock — keep its lifetime short.
    pub fn borrow(&self) -> watch::Ref<'_, ServedChainSnapshot> {
        self.receiver.borrow()
    }

    /// Construct a fresh receiver-side view by subscribing to
    /// the same channel. Used when handing the snapshot view
    /// to a new per-peer task.
    pub fn subscribe(&self) -> Self {
        ServedChainView {
            receiver: self.receiver.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    // Construction smoke test only — push_atomic's full path
    // requires a real AcceptedBlock (the constructor is private
    // to `ade_ledger::producer::self_accept`); end-to-end
    // exercise lives in B4's integration tests with a corpus
    // block. B2's tests prove the handle/view shape and the
    // empty-snapshot read path.

    #[test]
    fn handle_construction_yields_empty_snapshot() {
        let (_handle, view) = ServedChainHandle::new();
        let snap = view.borrow();
        assert!(snap.is_empty(), "fresh handle must yield empty snapshot");
        assert_eq!(snap.len(), 0);
    }

    #[test]
    fn view_subscribe_creates_independent_receiver() {
        let (handle, view1) = ServedChainHandle::new();
        let view2 = view1.subscribe();
        // Both views point at the same channel; receiver_count
        // reflects active receivers.
        assert_eq!(handle.receiver_count(), 2);
        drop(view2);
        assert_eq!(handle.receiver_count(), 1);
        drop(view1);
        assert_eq!(handle.receiver_count(), 0);
    }

    #[test]
    fn served_tip_is_closed_value_type() {
        let t = ServedTip {
            slot: SlotNo(42),
            hash: Hash32([0x11; 32]),
        };
        let cloned = t.clone();
        assert_eq!(t.slot, SlotNo(42));
        assert_eq!(cloned.hash.0, [0x11; 32]);
    }
}
