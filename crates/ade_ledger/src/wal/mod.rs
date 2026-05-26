// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE Ade-native WAL — closed `WalEntry` sum + append-only
//! `WalStore` trait + fingerprint-chain replay (PHASE4-N-M-A S3).
//!
//! Doctrine: per memory [[feedback-oracle-seed-then-ade-owns]],
//! the WAL is part of Ade's runtime authority — every
//! forward-step from a `BootstrapAnchor` is recorded as a closed
//! `WalEntry` whose fingerprint chain is verifiable. CN-WAL-01
//! enforces the single append authority; DC-WAL-01 enforces
//! append-only by trait surface; DC-WAL-02 enforces the
//! fingerprint chain; DC-WAL-03 (proven at A4) is the
//! replay-equivalence runtime contract.

pub mod error;
pub mod event;
pub mod replay;
pub mod store_trait;

pub use error::WalError;
pub use event::{
    decode_wal_entry, encode_wal_entry, BlockVerdictTag, WalEntry, TAG_ADMIT_BLOCK,
};
pub use replay::replay_from_anchor;
pub use store_trait::WalStore;
