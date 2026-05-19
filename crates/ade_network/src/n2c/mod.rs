// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C mini-protocol state machines (BLUE) — S-A8.
//
// Four pure transition state machines for the Ouroboros N2C
// (node-to-client) mini-protocols beyond handshake:
// LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor.
// N2C handshake state machine lives in `ade_network::handshake` (S-A3).
//
// The state machines own the closed wire grammar; ledger-semantic
// interpretation of LSQ Query/Result payloads and mempool-semantic
// interpretation of LocalTxMonitor Query/Reply payloads live in
// future clusters (N-F). Opaque Vec<u8> at this layer.

pub mod local_chain_sync;
pub mod local_state_query;
pub mod local_tx_monitor;
pub mod local_tx_submission;
