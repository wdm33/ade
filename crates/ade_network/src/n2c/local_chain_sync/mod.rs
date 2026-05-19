// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalChainSync mini-protocol state machine (BLUE) — S-A8.
//
// Pure local-chain-sync transition that consumes/produces the codec
// types defined in S-A2 (`LocalChainSyncMessage`, `Point`, `Tip`) and
// emits `LocalChainSyncEvent` values. The state graph mirrors N2N
// chain-sync; the sole difference is that RollForward carries a full
// `block_bytes: Vec<u8>` (no separate block-fetch on the N2C surface)
// rather than just header bytes. Block decoding remains opaque at this
// layer (DC-PROTO-06).
//
// Per-protocol agency type per locked §7 #7: `LocalChainSyncAgency`
// is non-interchangeable with the N2N `ChainSyncAgency` or any other
// per-protocol agency. The selected version is threaded as an explicit
// input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::LocalChainSyncAgency;
pub use event::{LocalChainSyncEvent, Point, Tip};
pub use state::{LocalChainSyncError, LocalChainSyncOutput, LocalChainSyncState};
pub use transition::local_chain_sync_transition;
