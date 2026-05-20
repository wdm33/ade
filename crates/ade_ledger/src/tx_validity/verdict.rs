// Core Contract:
// - Closed taxonomies; no owned String, no #[non_exhaustive], no Box<dyn>.
// - The reject `class` is the canonical/replay comparison surface; the full
//   structured `error` is debug-only and not part of the canonical bytes.
//
// Parallels `block_validity::verdict`: a closed verdict whose `Valid` carries
// the applied state + tx id and whose `Invalid` carries a coarse,
// oracle-comparable `class` PLUS the full structured reason. `class()` is the
// total mapping from reason to class (DC-TXV-01/02/04, mirroring DC-VAL-04).

use ade_types::Hash32;

use crate::block_validity::verdict::FieldError;
use crate::error::LedgerError;
use crate::state::LedgerState;
use crate::tx_validity::witness::WitnessClosureError;

/// The single-tx verdict. `Valid` carries the tx id (from preserved body
/// bytes) and the applied `LedgerState`; `Invalid` carries a coarse,
/// oracle-comparable `class` PLUS the full structured error for debugging.
// `Eq` is omitted because the embedded `LedgerError` is `PartialEq`-only
// upstream; this is a structural fact of `LedgerError`, not an open surface.
// `Valid` carries the applied `LedgerState` per the slice §9 contract, so the
// size asymmetry with `Invalid` is intentional — boxing the state would hide
// the authoritative output behind an indirection for no determinism benefit.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum TxValidityVerdict {
    Valid {
        tx_id: Hash32,
        applied: LedgerState,
    },
    Invalid {
        class: TxRejectClass,
        error: TxValidityError,
    },
}

/// Coarse, closed reject class — the canonical/replay comparison surface and
/// what the reference oracle exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxRejectClass {
    Phase1Invalid,
    WitnessInvalid,
    MissingRequiredSigner,
    Phase2Invalid,
    MalformedField,
}

/// Full structured reject reason. Closed; no owned String, no Box<dyn>.
// `Eq` is omitted because the embedded `LedgerError` is `PartialEq`-only
// upstream.
#[derive(Debug, Clone, PartialEq)]
pub enum TxValidityError {
    /// The tx (era envelope / body / witness set) failed to decode.
    Decode(LedgerError),
    /// Required-signer coverage over the preserved body hash failed
    /// fail-closed (from B2-S1 `verify_required_witnesses`).
    Witness(WitnessClosureError),
    /// A phase-1 state-backed check (structural / value / fee / network)
    /// rejected the tx.
    Phase1(LedgerError),
    /// Phase-2 (Plutus) evaluation rejected the tx.
    Phase2(LedgerError),
    /// A fixed-size field had the wrong length (fail-closed).
    MalformedField(FieldError),
}

impl TxValidityError {
    /// Total mapping from the full reason to the coarse comparison class.
    pub fn class(&self) -> TxRejectClass {
        match self {
            // A decode failure is a malformed-field-class rejection: the tx
            // could not be parsed into a body/witness set at all.
            TxValidityError::Decode(_) => TxRejectClass::MalformedField,
            TxValidityError::Witness(e) => match e {
                WitnessClosureError::MissingRequiredSigner { .. } => {
                    TxRejectClass::MissingRequiredSigner
                }
                WitnessClosureError::InvalidWitnessSignature { .. }
                | WitnessClosureError::MalformedWitnessField { .. } => {
                    TxRejectClass::WitnessInvalid
                }
            },
            TxValidityError::Phase1(_) => TxRejectClass::Phase1Invalid,
            TxValidityError::Phase2(_) => TxRejectClass::Phase2Invalid,
            TxValidityError::MalformedField(_) => TxRejectClass::MalformedField,
        }
    }
}
