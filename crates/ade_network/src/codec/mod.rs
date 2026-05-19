// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Mini-protocol message codec (BLUE) — S-A2.
//
// Each protocol module exposes:
//   pub fn encode_<protocol>_message(msg: &<Protocol>Message) -> Vec<u8>;
//   pub fn decode_<protocol>_message(bytes: &[u8]) -> Result<<Protocol>Message, CodecError>;
//
// Closed-enum-per-protocol: no `#[non_exhaustive]`, no `dyn` dispatch,
// no generic `Codec<P>` trait. The codec models the *closed wire
// grammar* only; semantic interpretation (state machines, ledger
// queries) lives in the slices that follow S-A2.

pub mod block_fetch;
pub mod chain_sync;
pub mod error;
pub mod handshake;
pub mod keep_alive;
pub mod local_chain_sync;
pub mod local_state_query;
pub mod local_tx_monitor;
pub mod local_tx_submission;
pub mod n2c_handshake;
pub mod peer_sharing;
pub mod primitives;
pub mod tx_submission;
pub mod version;

pub use error::{CodecError, ProtocolKind};
pub use version::{
    BlockFetchVersion, ChainSyncVersion, KeepAliveVersion, LocalChainSyncVersion,
    LocalStateQueryVersion, LocalTxMonitorVersion, LocalTxSubmissionVersion, N2CVersion,
    N2NVersion, PeerSharingVersion, TxSubmission2Version,
};
