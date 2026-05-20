// Core Contract:
// - Canonical CBOR for the verdict's replay/comparison surface only.
// - The full LedgerError / HeaderValidationError detail is NOT encoded here:
//   only the coarse, oracle-aligned comparison surface is byte-stable.
// - Mirrors ade_core::consensus::encoding: minicbor, definite-length arrays,
//   `[discriminant, ...payload]`, closed SurfaceDecodeError.

use ade_core::consensus::events::Point;
use ade_types::{BlockNo, Hash32, SlotNo};
use minicbor::{Decoder, Encoder};

use super::verdict::{BlockRejectClass, BlockValidityVerdict};

/// The replay/comparison surface: `Valid` carries `(tip, block_no)`; `Invalid`
/// carries only the coarse reject `class`. The full error detail is debug-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerdictSurface {
    Valid { tip: Point, block_no: BlockNo },
    Invalid { class: BlockRejectClass },
}

/// CLOSED decode error. No `Box<dyn>`, no owned `String`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceDecodeError {
    Cbor(&'static str),
    UnknownDiscriminant { for_enum: &'static str, found: u32 },
    FieldCount { expected: u32, actual: u32 },
}

impl From<minicbor::decode::Error> for SurfaceDecodeError {
    fn from(_e: minicbor::decode::Error) -> Self {
        SurfaceDecodeError::Cbor("minicbor decode failure")
    }
}

// `Vec<u8>` writes are infallible; minicbor still surfaces a `Result`.
type CborWriteError = minicbor::encode::Error<core::convert::Infallible>;

fn enc_err(_e: CborWriteError) -> SurfaceDecodeError {
    SurfaceDecodeError::Cbor("minicbor encode failure")
}

const SURFACE_VALID: u32 = 0;
const SURFACE_INVALID: u32 = 1;

const CLASS_HEADER_INVALID: u32 = 0;
const CLASS_BODY_INVALID: u32 = 1;
const CLASS_BODY_HASH_MISMATCH: u32 = 2;
const CLASS_MALFORMED_FIELD: u32 = 3;
const CLASS_MISSING_CONSENSUS_INPUT: u32 = 4;

fn class_discriminant(c: BlockRejectClass) -> u32 {
    match c {
        BlockRejectClass::HeaderInvalid => CLASS_HEADER_INVALID,
        BlockRejectClass::BodyInvalid => CLASS_BODY_INVALID,
        BlockRejectClass::BodyHashMismatch => CLASS_BODY_HASH_MISMATCH,
        BlockRejectClass::MalformedField => CLASS_MALFORMED_FIELD,
        BlockRejectClass::MissingConsensusInput => CLASS_MISSING_CONSENSUS_INPUT,
    }
}

fn class_from_discriminant(d: u32) -> Result<BlockRejectClass, SurfaceDecodeError> {
    match d {
        CLASS_HEADER_INVALID => Ok(BlockRejectClass::HeaderInvalid),
        CLASS_BODY_INVALID => Ok(BlockRejectClass::BodyInvalid),
        CLASS_BODY_HASH_MISMATCH => Ok(BlockRejectClass::BodyHashMismatch),
        CLASS_MALFORMED_FIELD => Ok(BlockRejectClass::MalformedField),
        CLASS_MISSING_CONSENSUS_INPUT => Ok(BlockRejectClass::MissingConsensusInput),
        other => Err(SurfaceDecodeError::UnknownDiscriminant {
            for_enum: "BlockRejectClass",
            found: other,
        }),
    }
}

fn expect_array_len(dec: &mut Decoder<'_>, expected: u32) -> Result<(), SurfaceDecodeError> {
    let len = dec
        .array()?
        .ok_or(SurfaceDecodeError::Cbor("expected definite-length array"))?;
    if len != u64::from(expected) {
        let actual = if len > u64::from(u32::MAX) {
            u32::MAX
        } else {
            len as u32
        };
        return Err(SurfaceDecodeError::FieldCount { expected, actual });
    }
    Ok(())
}

fn encode_hash32(enc: &mut Encoder<&mut Vec<u8>>, h: &Hash32) -> Result<(), SurfaceDecodeError> {
    enc.bytes(&h.0).map_err(enc_err)?;
    Ok(())
}

fn decode_hash32(dec: &mut Decoder<'_>) -> Result<Hash32, SurfaceDecodeError> {
    let bs = dec.bytes()?;
    if bs.len() != 32 {
        return Err(SurfaceDecodeError::Cbor("Hash32 length"));
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(bs);
    Ok(Hash32(buf))
}

fn encode_point(enc: &mut Encoder<&mut Vec<u8>>, p: &Point) -> Result<(), SurfaceDecodeError> {
    enc.array(2).map_err(enc_err)?;
    enc.u64(p.slot.0).map_err(enc_err)?;
    encode_hash32(enc, &p.hash)?;
    Ok(())
}

fn decode_point(dec: &mut Decoder<'_>) -> Result<Point, SurfaceDecodeError> {
    expect_array_len(dec, 2)?;
    let slot = SlotNo(dec.u64()?);
    let hash = decode_hash32(dec)?;
    Ok(Point { slot, hash })
}

/// Encode the replay/comparison surface:
/// `Valid -> [0, tip, block_no]`; `Invalid -> [1, reject_class_discriminant]`.
/// The full error detail is debug-only and NOT part of the canonical bytes.
pub fn encode_verdict_surface(v: &BlockValidityVerdict) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    if encode_verdict_surface_into(&mut buf, v).is_err() {
        // Encoder never returns Err for a Vec<u8> writer (Infallible). If that
        // invariant is ever violated, the buffer is cleared and decode fails loudly.
        buf.clear();
    }
    buf
}

fn encode_verdict_surface_into(
    buf: &mut Vec<u8>,
    v: &BlockValidityVerdict,
) -> Result<(), SurfaceDecodeError> {
    let mut enc = Encoder::new(buf);
    match v {
        BlockValidityVerdict::Valid { tip, block_no, .. } => {
            enc.array(3).map_err(enc_err)?;
            enc.u32(SURFACE_VALID).map_err(enc_err)?;
            encode_point(&mut enc, tip)?;
            enc.u64(block_no.0).map_err(enc_err)?;
        }
        BlockValidityVerdict::Invalid { class, .. } => {
            enc.array(2).map_err(enc_err)?;
            enc.u32(SURFACE_INVALID).map_err(enc_err)?;
            enc.u32(class_discriminant(*class)).map_err(enc_err)?;
        }
    }
    Ok(())
}

pub fn decode_verdict_surface(bytes: &[u8]) -> Result<VerdictSurface, SurfaceDecodeError> {
    let mut dec = Decoder::new(bytes);
    let len = dec
        .array()?
        .ok_or(SurfaceDecodeError::Cbor("expected definite-length array"))?;
    let outer = if len > u64::from(u32::MAX) {
        u32::MAX
    } else {
        len as u32
    };
    let disc = dec.u32()?;
    match disc {
        SURFACE_VALID => {
            if outer != 3 {
                return Err(SurfaceDecodeError::FieldCount {
                    expected: 3,
                    actual: outer,
                });
            }
            let tip = decode_point(&mut dec)?;
            let block_no = BlockNo(dec.u64()?);
            Ok(VerdictSurface::Valid { tip, block_no })
        }
        SURFACE_INVALID => {
            if outer != 2 {
                return Err(SurfaceDecodeError::FieldCount {
                    expected: 2,
                    actual: outer,
                });
            }
            let class = class_from_discriminant(dec.u32()?)?;
            Ok(VerdictSurface::Invalid { class })
        }
        other => Err(SurfaceDecodeError::UnknownDiscriminant {
            for_enum: "VerdictSurface",
            found: other,
        }),
    }
}
