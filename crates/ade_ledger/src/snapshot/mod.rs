// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE canonical snapshot encoder/decoder (PHASE4-N-J).
//!
//! Closes PHASE4-N-I's deferred DC-CONS-21 open_obligation by
//! shipping canonical, deterministic, version-tagged + fingerprint-
//! embedded byte encoders for `(LedgerState, PraosChainDepState)`.
//! Restart-safe rollback becomes possible.
//!
//! Scope: Conway-only encoder. Pre-Conway encode/decode →
//! `EraNotSupported` structurally.
//!
//! Single-authority discipline (CN-STORE-08): the `encode_*` +
//! `decode_*` pairs in this module are the SOLE pub fn pairs in
//! the project encoding/decoding `LedgerState` or
//! `PraosChainDepState` to/from bytes.

pub mod cert_state;
pub mod chain_dep;
pub mod epoch_state;
pub mod error;
pub mod framing;
pub mod gov_state;
pub mod ledger;
pub mod utxo_state;

pub use cert_state::{decode_cert_state, encode_cert_state};
pub use chain_dep::{decode_chain_dep, encode_chain_dep};
pub use epoch_state::{decode_epoch_state, encode_epoch_state};
pub use error::{SnapshotDecodeError, SnapshotEncodeError, StructuralReason};
pub use framing::{decode_snapshot, encode_snapshot, SCHEMA_VERSION};
pub use gov_state::{
    decode_conway_deposit_params, decode_gov_state, decode_pparams, encode_conway_deposit_params,
    encode_gov_state, encode_pparams,
};
pub use ledger::{decode_ledger_state, encode_ledger_state};
pub use utxo_state::{
    decode_tx_out_canonical, decode_utxo_state, encode_tx_out_canonical, encode_utxo_state,
};
