// Core Contract:
// - Closed taxonomies; no owned String, no #[non_exhaustive], no Box<dyn>.
// - The reject `class` is the canonical/replay comparison surface; the full
//   structured `error` is debug-only and not part of the canonical bytes.

use ade_core::consensus::errors::HeaderValidationError;
use ade_core::consensus::events::Point;
use ade_types::{BlockNo, Hash28, Hash32};

use crate::error::LedgerError;
use crate::rules::BlockVerdict;

/// The verdict. `Valid` carries the body stats; `Invalid` carries a coarse,
/// oracle-comparable `class` PLUS the full structured error for debugging.
// `Eq` is omitted because `Body(LedgerError)` is `PartialEq`-only upstream;
// this is a structural fact of `LedgerError`, not an open surface.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockValidityVerdict {
    Valid {
        tip: Point,
        block_no: BlockNo,
        body: BlockVerdict,
    },
    Invalid {
        class: BlockRejectClass,
        error: BlockValidityError,
    },
}

/// Coarse, closed reject class — the canonical/replay comparison surface and
/// what the reference oracle exposes. CBOR-round-trippable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockRejectClass {
    HeaderInvalid,
    BodyInvalid,
    BodyHashMismatch,
    MalformedField,
    MissingConsensusInput,
}

/// Full structured reject reason. Closed; no owned String, no Box<dyn>.
// `Eq` is omitted because `Body(LedgerError)` is `PartialEq`-only upstream.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockValidityError {
    Header(HeaderValidationError),
    Body(LedgerError),
    BodyHashMismatch { header: Hash32, actual: Hash32 },
    MalformedField(FieldError),
    MissingConsensusInput(MissingInput),
    /// The header's `block_number`/`prev_hash` pair is position-illegal:
    /// a genesis-successor (`block_number 0`) without `PrevHash::Genesis`,
    /// or a non-genesis block (`> 0`) with `PrevHash::Genesis`. Fail-fast
    /// at decode, ahead of the header authority (CN-WIRE-09 position
    /// clause, CE-G-J-3). Coarse class: `HeaderInvalid`.
    HeaderPositionInvalid {
        block_number: u64,
        prev_is_genesis: bool,
    },
}

impl BlockValidityError {
    /// Total mapping from the full reason to the coarse comparison class.
    pub fn class(&self) -> BlockRejectClass {
        match self {
            BlockValidityError::Header(_) => BlockRejectClass::HeaderInvalid,
            BlockValidityError::Body(_) => BlockRejectClass::BodyInvalid,
            BlockValidityError::BodyHashMismatch { .. } => BlockRejectClass::BodyHashMismatch,
            BlockValidityError::MalformedField(_) => BlockRejectClass::MalformedField,
            BlockValidityError::MissingConsensusInput(_) => {
                BlockRejectClass::MissingConsensusInput
            }
            BlockValidityError::HeaderPositionInvalid { .. } => BlockRejectClass::HeaderInvalid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldError {
    pub field: FieldKind,
    pub expected: usize,
    pub actual: usize,
}

/// Closed set of fixed-size fields whose length is checked fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    VkeyWitness,
    BootstrapKey,
    Ed25519Signature,
    VrfVkey,
    VrfProof,
    KesVkey,
    KesSignature,
    OpCertSignature,
    BlockBodyHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingInput {
    EpochNonce,
    SetSnapshot,
    PoolVrfKeyhash(Hash28),
    ActiveSlotsCoeff,
}
