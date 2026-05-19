// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2C LocalTxMonitor mini-protocol state machine (BLUE) — S-A8b.
//
// Pure local-tx-monitor transition that consumes the codec types from
// `codec::local_tx_monitor` (closed 12-message wire grammar matching
// cardano-node 11.0.1) and emits `LocalTxMonitorEvent` values.
//
// Per-protocol agency type per locked §7 #7: `LocalTxMonitorAgency`
// is non-interchangeable with any other per-protocol agency. The
// selected version is threaded as an explicit input (DC-PROTO-06).

pub mod agency;
pub mod event;
pub mod state;
pub mod transition;

pub use agency::LocalTxMonitorAgency;
pub use event::{LocalTxMonitorEvent, MempoolMeasures, MempoolSizeAndCapacity};
pub use state::{BusyKind, LocalTxMonitorError, LocalTxMonitorOutput, LocalTxMonitorState};
pub use transition::local_tx_monitor_transition;
