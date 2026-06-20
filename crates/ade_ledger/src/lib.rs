// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::float_arithmetic)]

pub mod alonzo;
pub mod babbage;
pub mod block_body_hash;
pub mod block_validity;
pub mod bootstrap_anchor;
pub mod byron;
pub mod cert_classify;
pub mod consensus_input_extract;
pub mod consensus_view;
pub mod conway;
pub mod delegation;
pub mod epoch;
pub mod error;
pub mod fingerprint;
pub mod genesis_source;
pub mod gov_cert;
pub mod governance;
pub mod hfc;
pub mod late_era_validation;
pub mod mary;
pub mod mempool;
pub mod phase;
pub mod plutus_eval;
pub mod pointer_resolve;
pub mod pparams;
pub mod pre_resolve;
pub mod producer;
pub mod rational;
pub mod receive;
pub mod recovered_anchor_point;
pub mod reduced_advance;
pub mod reduced_utxo;
pub mod rollback;
pub mod rules;
pub mod scripts;
pub mod seed_consensus_inputs;
pub mod shelley;
pub mod snapshot;
pub mod stake_ref;
pub mod state;
pub mod tx_validity;
pub mod utxo;
pub mod utxo_overlay;
pub mod value;
pub mod wal;
pub mod witness;
