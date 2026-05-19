// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Per-protocol agency marker for chain-sync.
//
// Locked decision §7 #7: each mini-protocol owns its own agency enum.
// `ChainSyncAgency` is deliberately NOT interchangeable with
// `HandshakeAgency` or any other per-protocol agency. No From/Into
// conversion is provided; the type system rejects cross-protocol
// agency mixing at the compile boundary.

/// Which party currently holds agency in the chain-sync exchange.
///
/// Per the Ouroboros chain-sync spec:
///   - Client holds agency in `Idle` (originates RequestNext /
///     FindIntersect / ClientDone).
///   - Server holds agency in `CanAwait`, `MustReply`, `Intersect`
///     (delivers RollForward / RollBackward / AwaitReply /
///     IntersectFound / IntersectNotFound).
///   - Nobody holds agency in `Done` — the protocol has terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainSyncAgency {
    Client,
    Server,
    Neither,
}
