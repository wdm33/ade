// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![deny(unsafe_code)]

pub mod admission;
pub mod bootstrap;
pub mod bootstrap_anchor;
pub mod chaindb;
pub mod clock;
pub mod consensus;
pub mod consensus_inputs;
pub mod forward_sync;
pub mod genesis_bootstrap;
pub mod mithril_bootstrap;
pub mod mithril_import;
pub mod network;
pub mod orchestrator;
pub mod producer;
pub mod receive;
pub mod recovered_anchor;
pub mod recovery;
pub mod rollback;
pub mod seed_consensus_merge;
pub mod seed_consensus_provenance;
pub mod seed_epoch_lineage;
pub mod seed_import;
pub mod wal;
