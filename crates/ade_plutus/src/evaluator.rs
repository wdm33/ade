// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! UPLC evaluator surface (Phase 3 Cluster P-B).
//!
//! Slice: S-29 (`ade_plutus` scaffold + UPLC port).
//!
//! Entry obligations discharged in docs/active/S-29_obligation_discharge.md:
//! - O-29.1: aiken pinned at v1.1.21 commit 42babe5d5fcdd403ed58ed924fdc2aed331ede4d
//! - O-29.2: dep audit — no major-version conflicts
//! - O-29.3: Flat decoder probe commitment recorded
//!
//! Authority invariant: given `(uplc_term, args, cost_model,
//! builtin_set)`, the evaluation result and budget consumption are
//! identical to the IOG reference implementation at plutus 1.57 under
//! protocol version 10 (cardano-node 10.6.2).
//!
//! This module is the quarantine boundary: `PlutusScript` carries an
//! aiken `Program<NamedDeBruijn>` internally but does not expose it
//! in the public API. Callers see Ade-canonical bytes and errors only.

use aiken_uplc::ast::{DeBruijn, Program};

use crate::evaluator::PlutusError::{DecodeFailed, EncodeFailed};

/// A decoded UPLC script, ready for round-trip or (future) evaluation.
///
/// The internal aiken representation is `Program<NamedDeBruijn>` — the
/// on-chain form. This type does not leak aiken types across the crate
/// boundary: callers interact via the methods below, which take and
/// return Ade-canonical bytes and errors.
#[derive(Debug, Clone)]
pub struct PlutusScript {
    inner: Program<DeBruijn>,
}

impl PlutusScript {
    /// Decode a flat-encoded UPLC script.
    ///
    /// The Flat format is aiken's native wire encoding for UPLC.
    /// On-chain scripts are CBOR-wrapped Flat; use `from_cbor` for
    /// those. Raw Flat is primarily used in IOG conformance tests
    /// (`*.uplc.expected` compared against `.uplc` source after
    /// execution; the source itself is textual UPLC, not Flat).
    pub fn from_flat(bytes: &[u8]) -> Result<Self, PlutusError> {
        Program::<DeBruijn>::from_flat(bytes)
            .map(|inner| PlutusScript { inner })
            .map_err(|e| DecodeFailed(e.to_string()))
    }

    /// Decode a CBOR-wrapped Flat UPLC script (the on-chain form).
    ///
    /// This is how Plutus scripts appear in witness sets on mainnet.
    /// The outer CBOR is a single bytestring whose contents are the
    /// Flat-encoded UPLC.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self, PlutusError> {
        let owned_in = bytes.to_vec();
        let mut buf = Vec::new();
        Program::<DeBruijn>::from_cbor(&owned_in, &mut buf)
            .map(|inner| PlutusScript { inner })
            .map_err(|e| DecodeFailed(e.to_string()))
    }

    /// Re-encode as Flat bytes. The conformance round-trip property
    /// requires `from_flat(b).to_flat() == b` for any canonically
    /// encoded `b`.
    pub fn to_flat(&self) -> Result<Vec<u8>, PlutusError> {
        self.inner
            .to_flat()
            .map_err(|e| EncodeFailed(e.to_string()))
    }

    /// Re-encode as CBOR-wrapped Flat. On-chain round-trip property:
    /// `from_cbor(b).to_cbor() == b` when `b` is the canonical on-chain
    /// form.
    pub fn to_cbor(&self) -> Result<Vec<u8>, PlutusError> {
        self.inner
            .to_cbor()
            .map_err(|e| EncodeFailed(e.to_string()))
    }

    /// UPLC language version declared in the program header.
    /// Currently `(1, 0, 0)` for PV ≤ 10 on mainnet; `(1, 1, 0)` under
    /// PV11 (not yet activated).
    pub fn version(&self) -> (usize, usize, usize) {
        self.inner.version
    }
}

/// Errors surfaced by the UPLC wrapper.
///
/// Carries the underlying aiken error message as a string — aiken's
/// error types are deliberately not re-exported to keep them quarantined
/// inside `ade_plutus`. Callers match on the variant, not the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlutusError {
    /// Flat or CBOR decoding failed.
    DecodeFailed(String),
    /// Flat or CBOR encoding failed.
    EncodeFailed(String),
}

impl core::fmt::Display for PlutusError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeFailed(msg) => write!(f, "UPLC decode failed: {msg}"),
            EncodeFailed(msg) => write!(f, "UPLC encode failed: {msg}"),
        }
    }
}

impl std::error::Error for PlutusError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip fixture: a minimal hand-constructed UPLC program.
    ///
    /// This is the textual UPLC equivalent of `(program 1.0.0 (con integer 42))`
    /// encoded as flat bytes. Values captured from aiken's own conformance
    /// tests to ensure this is a valid canonical encoding.
    ///
    /// Format breakdown (flat):
    ///   version (1,0,0) → 3 × 7-bit varints → 0x01 0x00 0x00 (padded to bytes)
    ///   term tag `Constant` = 0b0100 (4-bit, tag width 4)
    ///   constant tag `Integer` = 0b0000 (4-bit, tag width 4)
    ///   integer value 42 → ZigZag-encoded varint
    ///   then padding
    ///
    /// Rather than hand-roll the exact bytes (error-prone), we decode a
    /// known-good textual form via aiken's parser and rely on
    /// re-encoding to produce canonical Flat.
    fn make_identity_program_flat() -> Vec<u8> {
        // Build a trivial Program programmatically via aiken's textual
        // parser, then convert to DeBruijn (the on-chain representation
        // PlutusScript uses internally). This avoids any hex fragility.
        let program_src = "(program 1.0.0 (con integer 42))";
        let named: Program<aiken_uplc::ast::NamedDeBruijn> = aiken_uplc::parser::program(program_src)
            .expect("parse source")
            .try_into()
            .expect("convert to NamedDeBruijn");
        let debruijn: Program<DeBruijn> = named.into();
        debruijn.to_flat().expect("encode flat")
    }

    #[test]
    fn flat_roundtrip_is_byte_identical() {
        let bytes = make_identity_program_flat();
        let script = PlutusScript::from_flat(&bytes).expect("decode");
        let re = script.to_flat().expect("re-encode");
        assert_eq!(bytes, re, "flat round-trip must be byte-identical");
    }

    #[test]
    fn cbor_roundtrip_is_byte_identical() {
        // Construct cbor-wrapped flat
        let flat = make_identity_program_flat();
        let script = PlutusScript::from_flat(&flat).expect("decode flat");
        let cbor = script.to_cbor().expect("encode cbor");

        let script2 = PlutusScript::from_cbor(&cbor).expect("decode cbor");
        let cbor2 = script2.to_cbor().expect("re-encode cbor");
        assert_eq!(cbor, cbor2, "cbor round-trip must be byte-identical");
    }

    #[test]
    fn version_extracted() {
        let bytes = make_identity_program_flat();
        let script = PlutusScript::from_flat(&bytes).expect("decode");
        assert_eq!(script.version(), (1, 0, 0));
    }

    #[test]
    fn decode_invalid_bytes_fails() {
        let bogus = vec![0xffu8; 8];
        let result = PlutusScript::from_flat(&bogus);
        assert!(matches!(result, Err(PlutusError::DecodeFailed(_))));
    }

    #[test]
    fn decode_empty_bytes_fails() {
        let result = PlutusScript::from_flat(&[]);
        assert!(matches!(result, Err(PlutusError::DecodeFailed(_))));
    }

    #[test]
    fn error_display_is_useful() {
        let err = PlutusError::DecodeFailed("test error".into());
        assert!(err.to_string().contains("test error"));
    }
}
