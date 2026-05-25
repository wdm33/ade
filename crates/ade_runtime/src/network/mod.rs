// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED network shell for the producer-side server pump (PHASE4-N-G).
//!
//! Hosts the per-peer N2N session driver (`n2n_server`) that composes
//! the GREEN adapter (`producer::broadcast_to_served`) + BLUE
//! reducers (`ade_network::{chain_sync,block_fetch}::server`) into a
//! state machine the orchestrator/binary layers drive against a real
//! socket.
//!
//! Key-boundary doctrine: this module MUST NOT import from
//! `crate::producer::signing`. Enforced by
//! `ci/ci_check_n2n_server_no_signing_dep.sh`.

pub mod n2n_server;
