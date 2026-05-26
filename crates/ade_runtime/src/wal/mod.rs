// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN file-backed Ade-native WAL (PHASE4-N-M-A S3).

pub mod file_wal_store;

pub use file_wal_store::FileWalStore;
