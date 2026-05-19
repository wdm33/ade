// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Per-protocol agency marker for tx-submission2.
//
// Locked decision §7 #7: each mini-protocol owns its own agency enum.
// `TxSubmission2Agency` is deliberately NOT interchangeable with
// `ChainSyncAgency`, `BlockFetchAgency`, `HandshakeAgency`, or any
// other per-protocol agency. No From/Into conversion is provided; the
// type system rejects cross-protocol agency mixing at the compile
// boundary.

/// Which party currently holds agency in the tx-submission2 exchange.
///
/// Per the Ouroboros tx-submission2 spec the agency direction is
/// inverted from chain-sync: the Server (responder) drives the
/// conversation by issuing `RequestTxIds` / `RequestTxs` / `Done`, and
/// the Client (initiator) replies with `ReplyTxIds` / `ReplyTxs`. The
/// labels here follow the spec exactly.
///
///   - Client holds agency in `Init` (sends MsgInit handshake), and in
///     `TxIdsBlocking` / `TxIdsNonBlocking` / `TxsRequested` (replies
///     to outstanding server requests).
///   - Server holds agency in `Idle` (originates requests and Done).
///   - Nobody holds agency in `Done` — the protocol has terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxSubmission2Agency {
    Client,
    Server,
    Neither,
}
