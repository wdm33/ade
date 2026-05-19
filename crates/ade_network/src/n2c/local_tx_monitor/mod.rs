// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalTxMonitor mini-protocol state machine (BLUE) — S-A8.
//
// Pure local-tx-monitor transition that consumes the codec types
// defined in S-A2 (`LocalTxMonitorMessage`, `LocalTxMonitorQuery`,
// `LocalTxMonitorReply`) and emits `LocalTxMonitorEvent` values. The
// state machine owns the closed wire grammar of the LocalTxMonitor
// protocol but NOT the mempool-semantic interpretation of
// query/reply payloads — those are opaque `Vec<u8>` at this layer
// (DC-PROTO-06).
//
// Per-protocol agency type per locked §7 #7: `LocalTxMonitorAgency`
// is non-interchangeable with any other per-protocol agency. The
// selected version is threaded as an explicit input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::LocalTxMonitorAgency;
pub use event::{LocalTxMonitorEvent, LocalTxMonitorQuery, LocalTxMonitorReply};
pub use state::{LocalTxMonitorError, LocalTxMonitorOutput, LocalTxMonitorState};
pub use transition::local_tx_monitor_transition;
