// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// GREEN (Tier-5) mempool eviction/ordering. `order` is a deterministic
// projection over the ALREADY-ADMITTED tx ids. It MUST NOT call `tx_validity`,
// MUST NOT touch the accumulating state, and MUST NOT change any admit verdict —
// it only reorders/trims what the Tier-1 gate already admitted. The validity
// verdict belongs entirely to `admit`; policy is below it.

use ade_types::Hash32;

use super::admit::MempoolState;

/// A closed, deterministic ordering policy over the admitted tx ids. No
/// timing-dependent collapse (DC-MEM-02): the policy is a pure function of the
/// admitted set and the chosen variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderPolicy {
    /// Admission order — the order ids were admitted in (the natural order).
    ArrivalOrder,
    /// Ascending by tx id bytes — a deterministic re-projection that proves the
    /// policy can reorder without consulting validity.
    TxIdAscending,
}

/// Deterministically order the admitted tx ids under `policy`. Reads ONLY the
/// admitted-id list — never the accumulating state, never `tx_validity`. The
/// returned order is a permutation of `mempool.accepted()`; no id is added or
/// dropped, so it cannot change which txs are admitted.
pub fn order(mempool: &MempoolState, policy: OrderPolicy) -> Vec<Hash32> {
    let mut ids = mempool.accepted().to_vec();
    match policy {
        OrderPolicy::ArrivalOrder => {}
        OrderPolicy::TxIdAscending => ids.sort_by_key(|h| h.0),
    }
    ids
}
