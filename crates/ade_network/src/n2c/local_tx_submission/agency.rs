// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Per-protocol agency marker for N2C LocalTxSubmission.
//
// Locked decision §7 #7: each mini-protocol owns its own agency enum.
// `LocalTxSubmissionAgency` is deliberately NOT interchangeable with
// any other per-protocol agency. No From/Into conversion is provided;
// the type system rejects cross-protocol agency mixing at the compile
// boundary.

/// Which party currently holds agency in the LocalTxSubmission
/// exchange.
///
/// Per the Ouroboros local-tx-submission spec:
///   - Client holds agency in `Idle` (originates SubmitTx / Done).
///   - Server holds agency in `Busy` (delivers AcceptTx / RejectTx).
///   - Nobody holds agency in `Done` — the protocol has terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTxSubmissionAgency {
    Client,
    Server,
    Neither,
}
