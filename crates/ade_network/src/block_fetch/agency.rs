// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Per-protocol agency marker for block-fetch.
//
// Locked decision §7 #7: each mini-protocol owns its own agency enum.
// `BlockFetchAgency` is deliberately NOT interchangeable with
// `ChainSyncAgency`, `HandshakeAgency`, or any other per-protocol
// agency. No From/Into conversion is provided; the type system rejects
// cross-protocol agency mixing at the compile boundary.

/// Which party currently holds agency in the block-fetch exchange.
///
/// Per the Ouroboros block-fetch spec:
///   - Client holds agency in `Idle` (originates RequestRange /
///     ClientDone).
///   - Server holds agency in `Busy`, `Streaming` (delivers
///     StartBatch / NoBlocks / Block / BatchDone).
///   - Nobody holds agency in `Done` — the protocol has terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFetchAgency {
    Client,
    Server,
    Neither,
}
