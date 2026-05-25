// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN receive-side glue (PHASE4-N-H).
//!
//! Composes the BLUE pieces from `ade_ledger::receive` into a
//! pipeline the RED orchestrator (S4) can drive:
//!   - [`events_to_state`] lifts N-A `ForkChoiceSignal` +
//!     `BatchDeliveryEvent` into the unified `ReceiveEvent` stream.
//!   - [`in_memory_chain_write`] wires `ChainDbWrite` to
//!     `ade_runtime::chaindb::ChainDb`.

pub mod events_to_state;
pub mod in_memory_chain_write;

pub use events_to_state::{lift_block_fetch_event, lift_chain_sync_signal};
pub use in_memory_chain_write::ChainDbWriter;
