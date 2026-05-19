// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Per-protocol agency marker for the N2N and N2C handshakes.
//
// Locked decision §7 #7: each mini-protocol owns its own agency enum.
// `HandshakeAgency` is deliberately NOT interchangeable with any other
// per-protocol agency (e.g. `ChainSyncAgency`). The type system refuses
// to mix them at the compile boundary.

/// Which party currently holds agency in the handshake exchange.
///
/// The handshake is a single round-trip: the client always proposes,
/// then the server replies (accept or refuse). After the reply, no
/// party holds agency — the protocol has terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeAgency {
    /// Client must send (Idle state, before ProposeVersions).
    ClientHasAgency,
    /// Server must reply (after the client's ProposeVersions).
    ServerHasAgency,
    /// Terminal — protocol has resolved to Done or Refused.
    NobodyHasAgency,
}
