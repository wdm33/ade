// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! GREEN cardano-cli JSON UTxO seed importer (PHASE4-N-M-A S1).
//!
//! Single authority for converting a cardano-cli
//! `query utxo --whole-utxo --out-file utxo.json` dump into Ade
//! canonical `(UTxOState, UtxoFingerprint)`. CN-SEED-01.
//!
//! Doctrine: the cardano-cli output is a **bootstrap input
//! artifact at a named point P** — not a runtime authority. After
//! import, Ade owns the runtime representation. See memory
//! [[feedback-oracle-seed-then-ade-owns]].
//!
//! Honest scope:
//! - Conway-era post-Mary outputs supported (Babbage map form).
//! - Lovelace-only + multi-asset values supported.
//! - Inline datum (`inlineDatumRaw`) + datum hash supported.
//! - Reference scripts → fail-fast `UnsupportedTxOutFeature`
//!   (Phase-2 of this slice; documented as known-narrow scope).
//! - Byron / Shelley / Allegra / Mary legacy outputs →
//!   fail-fast (out of scope; preprod has none from the current
//!   tip).

pub mod json;
pub mod importer;

pub use importer::{import_cardano_cli_json_utxo, JsonSeedError, UtxoFingerprint};
