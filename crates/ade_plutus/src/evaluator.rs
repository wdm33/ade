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
use aiken_uplc::machine::cost_model::ExBudget;

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
    /// Parse textual UPLC source (the `(program 1.0.0 …)` form found
    /// in IOG's conformance `*.uplc` test inputs).
    ///
    /// Useful for conformance tests; on-chain scripts use flat/CBOR
    /// encoding, never textual.
    pub fn parse_textual(source: &str) -> Result<Self, PlutusError> {
        let named = aiken_uplc::parser::program(source)
            .map_err(|e| DecodeFailed(format!("parse: {e}")))?;
        let named: Program<aiken_uplc::ast::NamedDeBruijn> = named
            .try_into()
            .map_err(|e| DecodeFailed(format!("convert to NamedDeBruijn: {e:?}")))?;
        let debruijn: Program<DeBruijn> = named.into();
        Ok(PlutusScript { inner: debruijn })
    }

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

/// Compare two textual UPLC program strings for alpha-equivalence.
///
/// Parses both through aiken's textual parser, converts to DeBruijn
/// form (which strips variable names to indices), and compares
/// structurally. This matches the "same program up to bound-variable
/// renaming" property — which is what IOG's conformance tests
/// actually intend when they produce pretty-printed output.
///
/// Each input is first run through `sanitize_for_aiken_parser` to
/// convert IOG's `name-scope` variable notation (e.g. `x-0`, `j_1-0`)
/// into aiken-parser-compatible identifiers (underscore-joined).
/// DeBruijn normalization then strips names entirely, so this
/// rewriting preserves alpha-equivalence.
///
/// Returns `Ok(true)` on structural match, `Ok(false)` on mismatch,
/// and `Err(_)` if either side fails to parse (in which case the
/// caller typically falls back to string comparison).
pub fn programs_alpha_equivalent(a: &str, b: &str) -> Result<bool, PlutusError> {
    let sa = sanitize_for_aiken_parser(a);
    let sb = sanitize_for_aiken_parser(b);
    let pa = PlutusScript::parse_textual(&sa)?;
    let pb = PlutusScript::parse_textual(&sb)?;
    // Compare in DeBruijn form: bound-variable names have been
    // normalized to indices, so alpha-equivalent programs are `==`.
    Ok(pa.inner == pb.inner)
}

/// Rewrite IOG-style `name-scope` variable identifiers (e.g. `x-0`,
/// `j_1-0`) into aiken-parser-compatible identifiers by converting
/// the internal `-` to `_`. Preserves negative-integer `-` tokens
/// (those are preceded by whitespace or `(`).
///
/// Rule: replace every `-` whose preceding character is alphanumeric
/// or `_` with `_`. This rewrite is lossless for alpha-equivalence
/// since DeBruijn normalization discards names entirely.
fn sanitize_for_aiken_parser(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev: char = ' ';
    for c in s.chars() {
        if c == '-' && (prev.is_alphanumeric() || prev == '_') {
            out.push('_');
        } else {
            out.push(c);
        }
        prev = c;
    }
    out
}

/// Plutus language version, mirroring aiken's `Language` enum.
///
/// The numeric tag matches the on-chain `cost_models` map key
/// (Babbage+): `0 = V1`, `1 = V2`, `2 = V3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlutusLanguage {
    V1,
    V2,
    V3,
}

impl PlutusLanguage {
    fn to_aiken(self) -> pallas_primitives::conway::Language {
        use pallas_primitives::conway::Language as L;
        match self {
            PlutusLanguage::V1 => L::PlutusV1,
            PlutusLanguage::V2 => L::PlutusV2,
            PlutusLanguage::V3 => L::PlutusV3,
        }
    }
}

/// Canonical output of a Plutus evaluation.
///
/// Mirrors the subset of aiken's `EvalResult` that Ade commits to
/// matching byte-identically against the IOG conformance suite. Aiken
/// types are NOT leaked — `result_text` is the textual UPLC (as in
/// `*.uplc.expected` files) and `cost` is an i64 pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalOutput {
    /// Whether evaluation produced an error term (`Term::Error` or
    /// `Err(_)`). Separate from "the result was `false`".
    pub errored: bool,
    /// CPU steps consumed.
    pub cpu: i64,
    /// Memory units consumed.
    pub mem: i64,
    /// Textual UPLC of the result term. When `errored` is true, the
    /// textual form is the aiken-produced error representation (e.g.
    /// `(error)` or a builtin-specific error term).
    pub result_text: String,
    /// Any trace messages the program emitted via `trace`.
    pub logs: Vec<String>,
}

impl PlutusScript {
    /// Evaluate this script under the given Plutus language version,
    /// using aiken's default cost model for that version and a budget
    /// of `(i64::MAX, i64::MAX)` so that resource-limit checks happen
    /// only at the CEK runtime level, not at the initial-budget level.
    ///
    /// Use `eval_with_budget` when enforcing a real budget cap.
    ///
    /// Consumes the script — aiken's `eval_version` takes `self` by
    /// value because CEK execution threads the term.
    pub fn eval_default(self, version: PlutusLanguage) -> EvalOutput {
        self.eval_with_budget(
            version,
            ExBudget {
                mem: i64::MAX,
                cpu: i64::MAX,
            },
        )
    }

    /// Evaluate with explicit cost-model coefficients (from parsed
    /// pparams) and a CPU/memory budget cap.
    ///
    /// `costs` is the positional coefficient vector produced by
    /// `crate::cost_model::decode_cost_models` and looked up via
    /// `CostModels::get(version)`. Aiken's `eval_as` consumes this
    /// array directly without needing a typed struct adapter.
    ///
    /// This is the primary evaluation path for on-chain scripts: the
    /// ledger passes protocol-parameter-derived cost coefficients to
    /// each script execution.
    pub fn eval_with_costs(
        self,
        version: PlutusLanguage,
        costs: &[i64],
        initial: ExBudget,
    ) -> EvalOutput {
        let named: Program<aiken_uplc::ast::NamedDeBruijn> = self.inner.into();
        let aiken_version = version.to_aiken();
        let result = named.eval_as(&aiken_version, costs, Some(&initial));

        let consumed = result.cost();
        let errored = matches!(
            result.result(),
            Err(_) | Ok(aiken_uplc::ast::Term::Error)
        );
        let result_text = match result.result() {
            Ok(term) => format!("{term}"),
            Err(e) => format!("{e}"),
        };
        let logs = result.logs();

        EvalOutput {
            errored,
            cpu: consumed.cpu,
            mem: consumed.mem,
            result_text,
            logs,
        }
    }

    /// Evaluate with an explicit CPU/mem budget cap.
    ///
    /// Exhausting the budget produces an errored `EvalOutput` with
    /// `errored = true`. `cpu` and `mem` in the output are the
    /// quantities actually consumed (initial − remaining).
    pub fn eval_with_budget(self, version: PlutusLanguage, initial: ExBudget) -> EvalOutput {
        // aiken's eval uses NamedDeBruijn internally. Convert from our
        // DeBruijn-backed program first; aiken provides the From impl.
        let named: Program<aiken_uplc::ast::NamedDeBruijn> = self.inner.into();
        let aiken_version = version.to_aiken();
        let result = named.eval_version(initial, &aiken_version);

        let consumed = result.cost();
        let errored = matches!(
            result.result(),
            Err(_) | Ok(aiken_uplc::ast::Term::Error)
        );
        let result_text = match result.result() {
            Ok(term) => format!("{term}"),
            Err(e) => format!("{e}"),
        };
        let logs = result.logs();

        EvalOutput {
            errored,
            cpu: consumed.cpu,
            mem: consumed.mem,
            result_text,
            logs,
        }
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

    #[test]
    fn eval_identity_program_returns_constant() {
        // (program 1.0.0 (con integer 42)) — trivially evaluates to 42
        let bytes = make_identity_program_flat();
        let script = PlutusScript::from_flat(&bytes).expect("decode");
        let out = script.eval_default(PlutusLanguage::V1);
        assert!(!out.errored, "eval errored: {:?}", out.result_text);
        assert!(
            out.result_text.contains("42"),
            "expected '42' in result, got: {}",
            out.result_text
        );
        // Simple const evaluation consumes a non-zero budget.
        assert!(out.cpu > 0);
        assert!(out.mem > 0);
    }

    #[test]
    fn eval_consumes_budget_deterministically() {
        let bytes1 = make_identity_program_flat();
        let bytes2 = make_identity_program_flat();
        let out1 = PlutusScript::from_flat(&bytes1).unwrap().eval_default(PlutusLanguage::V1);
        let out2 = PlutusScript::from_flat(&bytes2).unwrap().eval_default(PlutusLanguage::V1);
        assert_eq!(out1.cpu, out2.cpu);
        assert_eq!(out1.mem, out2.mem);
        assert_eq!(out1.result_text, out2.result_text);
    }
}
