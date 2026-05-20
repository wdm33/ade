// Core Contract:
// - Canonical CBOR for the verdict's replay/comparison surface only.
// - The full TxValidityError detail is NOT encoded here: only the coarse,
//   oracle-aligned comparison surface is byte-stable.
// - Mirrors block_validity::encoding: minicbor, definite-length arrays,
//   `[discriminant, ...payload]`, closed SurfaceDecodeError.

use ade_types::Hash32;
use minicbor::{Decoder, Encoder};

use super::verdict::{TxRejectClass, TxValidityVerdict};

/// The per-tx replay/comparison surface: `Valid` carries the `tx_id`; `Invalid`
/// carries only the coarse reject `class`. The full error detail is debug-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxVerdictSurface {
    Valid { tx_id: Hash32 },
    Invalid { class: TxRejectClass },
}

/// CLOSED decode error. No `Box<dyn>`, no owned `String`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxSurfaceDecodeError {
    Cbor(&'static str),
    UnknownDiscriminant { for_enum: &'static str, found: u32 },
    FieldCount { expected: u32, actual: u32 },
}

impl From<minicbor::decode::Error> for TxSurfaceDecodeError {
    fn from(_e: minicbor::decode::Error) -> Self {
        TxSurfaceDecodeError::Cbor("minicbor decode failure")
    }
}

// `Vec<u8>` writes are infallible; minicbor still surfaces a `Result`.
type CborWriteError = minicbor::encode::Error<core::convert::Infallible>;

fn enc_err(_e: CborWriteError) -> TxSurfaceDecodeError {
    TxSurfaceDecodeError::Cbor("minicbor encode failure")
}

const SURFACE_VALID: u32 = 0;
const SURFACE_INVALID: u32 = 1;

const CLASS_PHASE1_INVALID: u32 = 0;
const CLASS_WITNESS_INVALID: u32 = 1;
const CLASS_MISSING_REQUIRED_SIGNER: u32 = 2;
const CLASS_PHASE2_INVALID: u32 = 3;
const CLASS_MALFORMED_FIELD: u32 = 4;

fn class_discriminant(c: TxRejectClass) -> u32 {
    match c {
        TxRejectClass::Phase1Invalid => CLASS_PHASE1_INVALID,
        TxRejectClass::WitnessInvalid => CLASS_WITNESS_INVALID,
        TxRejectClass::MissingRequiredSigner => CLASS_MISSING_REQUIRED_SIGNER,
        TxRejectClass::Phase2Invalid => CLASS_PHASE2_INVALID,
        TxRejectClass::MalformedField => CLASS_MALFORMED_FIELD,
    }
}

fn class_from_discriminant(d: u32) -> Result<TxRejectClass, TxSurfaceDecodeError> {
    match d {
        CLASS_PHASE1_INVALID => Ok(TxRejectClass::Phase1Invalid),
        CLASS_WITNESS_INVALID => Ok(TxRejectClass::WitnessInvalid),
        CLASS_MISSING_REQUIRED_SIGNER => Ok(TxRejectClass::MissingRequiredSigner),
        CLASS_PHASE2_INVALID => Ok(TxRejectClass::Phase2Invalid),
        CLASS_MALFORMED_FIELD => Ok(TxRejectClass::MalformedField),
        other => Err(TxSurfaceDecodeError::UnknownDiscriminant {
            for_enum: "TxRejectClass",
            found: other,
        }),
    }
}

fn decode_hash32(dec: &mut Decoder<'_>) -> Result<Hash32, TxSurfaceDecodeError> {
    let bs = dec.bytes()?;
    if bs.len() != 32 {
        return Err(TxSurfaceDecodeError::Cbor("Hash32 length"));
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(bs);
    Ok(Hash32(buf))
}

/// Encode the per-tx replay/comparison surface:
/// `Valid -> [0, tx_id]`; `Invalid -> [1, reject_class_discriminant]`.
/// The full error detail is debug-only and NOT part of the canonical bytes.
pub fn encode_tx_verdict_surface(v: &TxValidityVerdict) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    if encode_tx_verdict_surface_into(&mut buf, v).is_err() {
        // Encoder never returns Err for a Vec<u8> writer (Infallible). If that
        // invariant is ever violated, the buffer is cleared and decode fails loudly.
        buf.clear();
    }
    buf
}

fn encode_tx_verdict_surface_into(
    buf: &mut Vec<u8>,
    v: &TxValidityVerdict,
) -> Result<(), TxSurfaceDecodeError> {
    let mut enc = Encoder::new(buf);
    match v {
        TxValidityVerdict::Valid { tx_id, .. } => {
            enc.array(2).map_err(enc_err)?;
            enc.u32(SURFACE_VALID).map_err(enc_err)?;
            enc.bytes(&tx_id.0).map_err(enc_err)?;
        }
        TxValidityVerdict::Invalid { class, .. } => {
            enc.array(2).map_err(enc_err)?;
            enc.u32(SURFACE_INVALID).map_err(enc_err)?;
            enc.u32(class_discriminant(*class)).map_err(enc_err)?;
        }
    }
    Ok(())
}

pub fn decode_tx_verdict_surface(bytes: &[u8]) -> Result<TxVerdictSurface, TxSurfaceDecodeError> {
    let mut dec = Decoder::new(bytes);
    let len = dec
        .array()?
        .ok_or(TxSurfaceDecodeError::Cbor("expected definite-length array"))?;
    let outer = if len > u64::from(u32::MAX) {
        u32::MAX
    } else {
        len as u32
    };
    if outer != 2 {
        return Err(TxSurfaceDecodeError::FieldCount {
            expected: 2,
            actual: outer,
        });
    }
    let disc = dec.u32()?;
    match disc {
        SURFACE_VALID => {
            let tx_id = decode_hash32(&mut dec)?;
            Ok(TxVerdictSurface::Valid { tx_id })
        }
        SURFACE_INVALID => {
            let class = class_from_discriminant(dec.u32()?)?;
            Ok(TxVerdictSurface::Invalid { class })
        }
        other => Err(TxSurfaceDecodeError::UnknownDiscriminant {
            for_enum: "TxVerdictSurface",
            found: other,
        }),
    }
}
