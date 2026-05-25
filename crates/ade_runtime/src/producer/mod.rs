// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Producer-side RED scaffolding (PHASE4-N-C).
//!
//! Private-key custody lives only inside this module tree. BLUE crates
//! (`ade_core`, `ade_codec`, `ade_types`, `ade_ledger`, `ade_crypto`)
//! consume signed artifacts (`VrfProof`, `KesSignature`, `OpCert` bytes)
//! across the RED -> BLUE boundary; the secrets themselves never cross.
//!
//! See `docs/clusters/PHASE4-N-C/cluster.md` and
//! `docs/clusters/PHASE4-N-C/N-C-S1.md` for the full key-boundary contract.
//! Mechanically enforced by `ci/ci_check_private_key_custody.sh`.

pub mod broadcast;
pub mod broadcast_to_served;
pub mod keys;
pub mod scheduler;
pub mod served_chain_lookups;
pub mod signing;
pub mod tick_assembler;
