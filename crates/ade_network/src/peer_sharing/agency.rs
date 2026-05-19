// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Per-protocol agency marker for peer-sharing.
//
// Locked decision §7 #7: each mini-protocol owns its own agency enum.
// `PeerSharingAgency` is deliberately NOT interchangeable with
// `ChainSyncAgency`, `BlockFetchAgency`, `TxSubmission2Agency`,
// `KeepAliveAgency`, or any other per-protocol agency. No From/Into
// conversion is provided; the type system rejects cross-protocol
// agency mixing at the compile boundary.

/// Which party currently holds agency in the peer-sharing exchange.
///
/// Per the Ouroboros peer-sharing spec:
///   - Client holds agency in `Idle` (originates ShareRequest /
///     Done).
///   - Server holds agency in `Busy` (replies with SharePeers).
///   - Nobody holds agency in `Done` — the protocol has terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerSharingAgency {
    Client,
    Server,
    Neither,
}
