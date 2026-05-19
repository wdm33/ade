// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Module barrel — GREEN. Authority lives in:
//   - frame      (BLUE) pure Ouroboros mux frame encode/decode
//   - transport  (RED)  tokio-based socket I/O

pub mod frame;
pub mod transport;
