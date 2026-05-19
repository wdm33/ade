// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Per-protocol agency marker for keep-alive.
//
// Locked decision §7 #7: each mini-protocol owns its own agency enum.
// `KeepAliveAgency` is deliberately NOT interchangeable with
// `ChainSyncAgency`, `BlockFetchAgency`, `TxSubmission2Agency`,
// `PeerSharingAgency`, or any other per-protocol agency. No From/Into
// conversion is provided; the type system rejects cross-protocol
// agency mixing at the compile boundary.

/// Which party currently holds agency in the keep-alive exchange.
///
/// Per the Ouroboros keep-alive spec:
///   - Client holds agency in `ClientIdle` (originates KeepAlive /
///     Done).
///   - Server holds agency in `ServerHasAgency` (replies with
///     ResponseKeepAlive).
///   - Nobody holds agency in `Done` — the protocol has terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAliveAgency {
    Client,
    Server,
    Neither,
}
