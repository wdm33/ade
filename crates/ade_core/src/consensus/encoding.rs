// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::{BlockNo, EpochNo, Hash28, Hash32, SlotNo};
use minicbor::data::Type;
use minicbor::{Decoder, Encoder};

use crate::consensus::errors::{
    HFCError, HeaderValidationError, NonceEvolutionError, OpCertCounterError, VrfCertError,
};
use crate::consensus::events::{
    BlockDistance, ChainEvent, ChainSelectionReject, Point, SecurityParam,
};
use crate::consensus::praos_state::{Nonce, OpCertCounterMap, PraosChainDepState};

// =============================================================================
// DecodeError
// =============================================================================

/// CLOSED decode error. No `Box<dyn>`, no `String`, no `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Cbor(&'static str),
    UnknownDiscriminant {
        for_enum: &'static str,
        found: u32,
    },
    FieldCountMismatch {
        expected: u32,
        actual: u32,
    },
    InvalidLength {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl From<minicbor::decode::Error> for DecodeError {
    fn from(_e: minicbor::decode::Error) -> Self {
        DecodeError::Cbor("minicbor decode failure")
    }
}

// `Vec<u8>` writes are infallible. The void error never occurs for an
// in-memory encoder, but minicbor still surfaces it as `Result`.
type CborWriteError = minicbor::encode::Error<core::convert::Infallible>;

fn enc_err(_e: CborWriteError) -> DecodeError {
    DecodeError::Cbor("minicbor encode failure")
}

// =============================================================================
// Primitive helpers
// =============================================================================

fn encode_nonce(enc: &mut Encoder<&mut Vec<u8>>, n: &Nonce) -> Result<(), DecodeError> {
    enc.bytes(n.as_bytes()).map_err(enc_err)?;
    Ok(())
}

fn decode_nonce(dec: &mut Decoder<'_>) -> Result<Nonce, DecodeError> {
    let bs = dec.bytes()?;
    if bs.len() != 32 {
        return Err(DecodeError::InvalidLength {
            field: "Nonce",
            expected: 32,
            actual: bs.len(),
        });
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(bs);
    Ok(Nonce(Hash32(buf)))
}

fn encode_hash32(enc: &mut Encoder<&mut Vec<u8>>, h: &Hash32) -> Result<(), DecodeError> {
    enc.bytes(&h.0).map_err(enc_err)?;
    Ok(())
}

fn decode_hash32(dec: &mut Decoder<'_>) -> Result<Hash32, DecodeError> {
    let bs = dec.bytes()?;
    if bs.len() != 32 {
        return Err(DecodeError::InvalidLength {
            field: "Hash32",
            expected: 32,
            actual: bs.len(),
        });
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(bs);
    Ok(Hash32(buf))
}

fn encode_hash28(enc: &mut Encoder<&mut Vec<u8>>, h: &Hash28) -> Result<(), DecodeError> {
    enc.bytes(&h.0).map_err(enc_err)?;
    Ok(())
}

fn decode_hash28(dec: &mut Decoder<'_>) -> Result<Hash28, DecodeError> {
    let bs = dec.bytes()?;
    if bs.len() != 28 {
        return Err(DecodeError::InvalidLength {
            field: "Hash28",
            expected: 28,
            actual: bs.len(),
        });
    }
    let mut buf = [0u8; 28];
    buf.copy_from_slice(bs);
    Ok(Hash28(buf))
}

fn encode_opt_u64(enc: &mut Encoder<&mut Vec<u8>>, v: Option<u64>) -> Result<(), DecodeError> {
    match v {
        Some(x) => {
            enc.u64(x).map_err(enc_err)?;
        }
        None => {
            enc.null().map_err(enc_err)?;
        }
    }
    Ok(())
}

fn decode_opt_u64(dec: &mut Decoder<'_>) -> Result<Option<u64>, DecodeError> {
    match dec.datatype()? {
        Type::Null => {
            dec.null()?;
            Ok(None)
        }
        _ => Ok(Some(dec.u64()?)),
    }
}

fn expect_array_len(dec: &mut Decoder<'_>, expected: u32) -> Result<(), DecodeError> {
    let len = dec
        .array()?
        .ok_or(DecodeError::Cbor("expected definite-length array"))?;
    if len != u64::from(expected) {
        let actual = if len > u64::from(u32::MAX) {
            u32::MAX
        } else {
            len as u32
        };
        return Err(DecodeError::FieldCountMismatch { expected, actual });
    }
    Ok(())
}

// =============================================================================
// Point / BlockDistance / SecurityParam
// =============================================================================

fn encode_point(enc: &mut Encoder<&mut Vec<u8>>, p: &Point) -> Result<(), DecodeError> {
    enc.array(2).map_err(enc_err)?;
    enc.u64(p.slot.0).map_err(enc_err)?;
    encode_hash32(enc, &p.hash)?;
    Ok(())
}

fn decode_point(dec: &mut Decoder<'_>) -> Result<Point, DecodeError> {
    expect_array_len(dec, 2)?;
    let slot = SlotNo(dec.u64()?);
    let hash = decode_hash32(dec)?;
    Ok(Point { slot, hash })
}

// =============================================================================
// OpCertCounterMap
// =============================================================================

fn encode_op_cert_counters(
    enc: &mut Encoder<&mut Vec<u8>>,
    m: &OpCertCounterMap,
) -> Result<(), DecodeError> {
    enc.array(m.len() as u64).map_err(enc_err)?;
    for ((pool, kes_period), counter) in m.iter() {
        enc.array(3).map_err(enc_err)?;
        encode_hash28(enc, pool)?;
        enc.u64(*kes_period).map_err(enc_err)?;
        enc.u64(*counter).map_err(enc_err)?;
    }
    Ok(())
}

fn decode_op_cert_counters(dec: &mut Decoder<'_>) -> Result<OpCertCounterMap, DecodeError> {
    let len = dec
        .array()?
        .ok_or(DecodeError::Cbor("expected definite-length array"))?;
    let mut map = OpCertCounterMap::new();
    for _ in 0..len {
        expect_array_len(dec, 3)?;
        let pool = decode_hash28(dec)?;
        let kes_period = dec.u64()?;
        let counter = dec.u64()?;
        map.insert_unchecked(pool, kes_period, counter);
    }
    Ok(map)
}

// =============================================================================
// PraosChainDepState
// =============================================================================

const CHAIN_DEP_STATE_FIELDS: u32 = 9;

pub fn encode_chain_dep_state(s: &PraosChainDepState) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    if encode_chain_dep_state_into(&mut buf, s).is_err() {
        // Encoder never returns Err for a Vec<u8> writer (Infallible).
        // Keep the function's signature simple for callers; if the
        // Infallible invariant is ever violated, the buffer is empty
        // and decode will fail loudly.
        buf.clear();
    }
    buf
}

fn encode_chain_dep_state_into(
    buf: &mut Vec<u8>,
    s: &PraosChainDepState,
) -> Result<(), DecodeError> {
    let mut enc = Encoder::new(buf);
    enc.array(u64::from(CHAIN_DEP_STATE_FIELDS))
        .map_err(enc_err)?;
    encode_nonce(&mut enc, &s.evolving_nonce)?;
    encode_nonce(&mut enc, &s.candidate_nonce)?;
    encode_nonce(&mut enc, &s.epoch_nonce)?;
    encode_nonce(&mut enc, &s.previous_epoch_nonce)?;
    encode_nonce(&mut enc, &s.lab_nonce)?;
    encode_opt_u64(&mut enc, s.last_epoch_block.map(|e| e.0))?;
    encode_opt_u64(&mut enc, s.last_slot.map(|s| s.0))?;
    encode_opt_u64(&mut enc, s.last_block_no.map(|b| b.0))?;
    encode_op_cert_counters(&mut enc, &s.op_cert_counters)?;
    Ok(())
}

pub fn decode_chain_dep_state(bytes: &[u8]) -> Result<PraosChainDepState, DecodeError> {
    let mut dec = Decoder::new(bytes);
    expect_array_len(&mut dec, CHAIN_DEP_STATE_FIELDS)?;
    let evolving_nonce = decode_nonce(&mut dec)?;
    let candidate_nonce = decode_nonce(&mut dec)?;
    let epoch_nonce = decode_nonce(&mut dec)?;
    let previous_epoch_nonce = decode_nonce(&mut dec)?;
    let lab_nonce = decode_nonce(&mut dec)?;
    let last_epoch_block = decode_opt_u64(&mut dec)?.map(EpochNo);
    let last_slot = decode_opt_u64(&mut dec)?.map(SlotNo);
    let last_block_no = decode_opt_u64(&mut dec)?.map(BlockNo);
    let op_cert_counters = decode_op_cert_counters(&mut dec)?;
    Ok(PraosChainDepState {
        evolving_nonce,
        candidate_nonce,
        epoch_nonce,
        previous_epoch_nonce,
        lab_nonce,
        last_epoch_block,
        last_slot,
        last_block_no,
        op_cert_counters,
    })
}

// =============================================================================
// HFCError
// =============================================================================

const HFC_EMPTY_ERA_LIST: u32 = 0;
const HFC_NON_MONOTONIC_ERAS: u32 = 1;
const HFC_ZERO_SLOT_LENGTH: u32 = 2;
const HFC_ZERO_EPOCH_LENGTH: u32 = 3;
const HFC_SLOT_BEFORE_SYSTEM_START: u32 = 4;
const HFC_SLOT_AFTER_LAST_ERA: u32 = 5;

fn encode_hfc_error(enc: &mut Encoder<&mut Vec<u8>>, e: &HFCError) -> Result<(), DecodeError> {
    enc.array(2).map_err(enc_err)?;
    match e {
        HFCError::EmptyEraList => {
            enc.u32(HFC_EMPTY_ERA_LIST).map_err(enc_err)?;
            enc.array(0).map_err(enc_err)?;
        }
        HFCError::NonMonotonicEras {
            prev_start,
            next_start,
        } => {
            enc.u32(HFC_NON_MONOTONIC_ERAS).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(prev_start.0).map_err(enc_err)?;
            enc.u64(next_start.0).map_err(enc_err)?;
        }
        HFCError::ZeroSlotLength { era_index } => {
            enc.u32(HFC_ZERO_SLOT_LENGTH).map_err(enc_err)?;
            enc.array(1).map_err(enc_err)?;
            enc.u8(*era_index).map_err(enc_err)?;
        }
        HFCError::ZeroEpochLength { era_index } => {
            enc.u32(HFC_ZERO_EPOCH_LENGTH).map_err(enc_err)?;
            enc.array(1).map_err(enc_err)?;
            enc.u8(*era_index).map_err(enc_err)?;
        }
        HFCError::SlotBeforeSystemStart {
            slot,
            first_era_start,
        } => {
            enc.u32(HFC_SLOT_BEFORE_SYSTEM_START).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(slot.0).map_err(enc_err)?;
            enc.u64(first_era_start.0).map_err(enc_err)?;
        }
        HFCError::SlotAfterLastEra { slot, last_era_end } => {
            enc.u32(HFC_SLOT_AFTER_LAST_ERA).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(slot.0).map_err(enc_err)?;
            enc.u64(last_era_end.0).map_err(enc_err)?;
        }
    }
    Ok(())
}

fn decode_hfc_error(dec: &mut Decoder<'_>) -> Result<HFCError, DecodeError> {
    expect_array_len(dec, 2)?;
    let disc = dec.u32()?;
    match disc {
        HFC_EMPTY_ERA_LIST => {
            expect_array_len(dec, 0)?;
            Ok(HFCError::EmptyEraList)
        }
        HFC_NON_MONOTONIC_ERAS => {
            expect_array_len(dec, 2)?;
            let prev_start = SlotNo(dec.u64()?);
            let next_start = SlotNo(dec.u64()?);
            Ok(HFCError::NonMonotonicEras {
                prev_start,
                next_start,
            })
        }
        HFC_ZERO_SLOT_LENGTH => {
            expect_array_len(dec, 1)?;
            let era_index = dec.u8()?;
            Ok(HFCError::ZeroSlotLength { era_index })
        }
        HFC_ZERO_EPOCH_LENGTH => {
            expect_array_len(dec, 1)?;
            let era_index = dec.u8()?;
            Ok(HFCError::ZeroEpochLength { era_index })
        }
        HFC_SLOT_BEFORE_SYSTEM_START => {
            expect_array_len(dec, 2)?;
            let slot = SlotNo(dec.u64()?);
            let first_era_start = SlotNo(dec.u64()?);
            Ok(HFCError::SlotBeforeSystemStart {
                slot,
                first_era_start,
            })
        }
        HFC_SLOT_AFTER_LAST_ERA => {
            expect_array_len(dec, 2)?;
            let slot = SlotNo(dec.u64()?);
            let last_era_end = SlotNo(dec.u64()?);
            Ok(HFCError::SlotAfterLastEra { slot, last_era_end })
        }
        other => Err(DecodeError::UnknownDiscriminant {
            for_enum: "HFCError",
            found: other,
        }),
    }
}

// =============================================================================
// VrfCertError
// =============================================================================

const VRF_MALFORMED_KEY: u32 = 0;
const VRF_MALFORMED_PROOF: u32 = 1;
const VRF_VERIFICATION_FAILED: u32 = 2;
const VRF_LEADER_VALUE_ABOVE_THRESHOLD: u32 = 3;

fn encode_vrf_cert_error(
    enc: &mut Encoder<&mut Vec<u8>>,
    e: &VrfCertError,
) -> Result<(), DecodeError> {
    enc.array(2).map_err(enc_err)?;
    match e {
        VrfCertError::MalformedKey => {
            enc.u32(VRF_MALFORMED_KEY).map_err(enc_err)?;
            enc.array(0).map_err(enc_err)?;
        }
        VrfCertError::MalformedProof => {
            enc.u32(VRF_MALFORMED_PROOF).map_err(enc_err)?;
            enc.array(0).map_err(enc_err)?;
        }
        VrfCertError::VerificationFailed => {
            enc.u32(VRF_VERIFICATION_FAILED).map_err(enc_err)?;
            enc.array(0).map_err(enc_err)?;
        }
        VrfCertError::LeaderValueAboveThreshold { value, threshold } => {
            enc.u32(VRF_LEADER_VALUE_ABOVE_THRESHOLD).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.bytes(value).map_err(enc_err)?;
            enc.bytes(threshold).map_err(enc_err)?;
        }
    }
    Ok(())
}

fn decode_vrf_cert_error(dec: &mut Decoder<'_>) -> Result<VrfCertError, DecodeError> {
    expect_array_len(dec, 2)?;
    let disc = dec.u32()?;
    match disc {
        VRF_MALFORMED_KEY => {
            expect_array_len(dec, 0)?;
            Ok(VrfCertError::MalformedKey)
        }
        VRF_MALFORMED_PROOF => {
            expect_array_len(dec, 0)?;
            Ok(VrfCertError::MalformedProof)
        }
        VRF_VERIFICATION_FAILED => {
            expect_array_len(dec, 0)?;
            Ok(VrfCertError::VerificationFailed)
        }
        VRF_LEADER_VALUE_ABOVE_THRESHOLD => {
            expect_array_len(dec, 2)?;
            let v_bytes = dec.bytes()?;
            if v_bytes.len() != 8 {
                return Err(DecodeError::InvalidLength {
                    field: "VrfCertError::value",
                    expected: 8,
                    actual: v_bytes.len(),
                });
            }
            let mut value = [0u8; 8];
            value.copy_from_slice(v_bytes);
            let t_bytes = dec.bytes()?;
            if t_bytes.len() != 8 {
                return Err(DecodeError::InvalidLength {
                    field: "VrfCertError::threshold",
                    expected: 8,
                    actual: t_bytes.len(),
                });
            }
            let mut threshold = [0u8; 8];
            threshold.copy_from_slice(t_bytes);
            Ok(VrfCertError::LeaderValueAboveThreshold { value, threshold })
        }
        other => Err(DecodeError::UnknownDiscriminant {
            for_enum: "VrfCertError",
            found: other,
        }),
    }
}

// =============================================================================
// OpCertCounterError
// =============================================================================

const OP_CERT_REGRESSION: u32 = 0;

fn encode_op_cert_error(
    enc: &mut Encoder<&mut Vec<u8>>,
    e: &OpCertCounterError,
) -> Result<(), DecodeError> {
    enc.array(2).map_err(enc_err)?;
    match e {
        OpCertCounterError::Regression {
            existing,
            attempted,
        } => {
            enc.u32(OP_CERT_REGRESSION).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(*existing).map_err(enc_err)?;
            enc.u64(*attempted).map_err(enc_err)?;
        }
    }
    Ok(())
}

fn decode_op_cert_error(dec: &mut Decoder<'_>) -> Result<OpCertCounterError, DecodeError> {
    expect_array_len(dec, 2)?;
    let disc = dec.u32()?;
    match disc {
        OP_CERT_REGRESSION => {
            expect_array_len(dec, 2)?;
            let existing = dec.u64()?;
            let attempted = dec.u64()?;
            Ok(OpCertCounterError::Regression {
                existing,
                attempted,
            })
        }
        other => Err(DecodeError::UnknownDiscriminant {
            for_enum: "OpCertCounterError",
            found: other,
        }),
    }
}

// =============================================================================
// NonceEvolutionError
// =============================================================================

const NONCE_SLOT_BEFORE_LAST: u32 = 0;
const NONCE_UNINITIALISED_EPOCH_NONCE: u32 = 1;

fn encode_nonce_evolution_error(
    enc: &mut Encoder<&mut Vec<u8>>,
    e: &NonceEvolutionError,
) -> Result<(), DecodeError> {
    enc.array(2).map_err(enc_err)?;
    match e {
        NonceEvolutionError::SlotBeforeLast { last, attempted } => {
            enc.u32(NONCE_SLOT_BEFORE_LAST).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(last.0).map_err(enc_err)?;
            enc.u64(attempted.0).map_err(enc_err)?;
        }
        NonceEvolutionError::UninitialisedEpochNonce => {
            enc.u32(NONCE_UNINITIALISED_EPOCH_NONCE).map_err(enc_err)?;
            enc.array(0).map_err(enc_err)?;
        }
    }
    Ok(())
}

fn decode_nonce_evolution_error(dec: &mut Decoder<'_>) -> Result<NonceEvolutionError, DecodeError> {
    expect_array_len(dec, 2)?;
    let disc = dec.u32()?;
    match disc {
        NONCE_SLOT_BEFORE_LAST => {
            expect_array_len(dec, 2)?;
            let last = SlotNo(dec.u64()?);
            let attempted = SlotNo(dec.u64()?);
            Ok(NonceEvolutionError::SlotBeforeLast { last, attempted })
        }
        NONCE_UNINITIALISED_EPOCH_NONCE => {
            expect_array_len(dec, 0)?;
            Ok(NonceEvolutionError::UninitialisedEpochNonce)
        }
        other => Err(DecodeError::UnknownDiscriminant {
            for_enum: "NonceEvolutionError",
            found: other,
        }),
    }
}

// =============================================================================
// HeaderValidationError
// =============================================================================

const HVE_VRF_CERT: u32 = 0;
const HVE_OP_CERT_COUNTER: u32 = 1;
const HVE_NONCE: u32 = 2;
const HVE_SLOT_BEFORE_LAST_APPLIED: u32 = 3;
const HVE_BLOCK_NO_OUT_OF_ORDER: u32 = 4;
const HVE_BODY_HASH_MISMATCH: u32 = 5;
const HVE_ERA_MISMATCH: u32 = 6;
const HVE_HFC: u32 = 7;

fn encode_header_validation_error(
    enc: &mut Encoder<&mut Vec<u8>>,
    e: &HeaderValidationError,
) -> Result<(), DecodeError> {
    enc.array(2).map_err(enc_err)?;
    match e {
        HeaderValidationError::VrfCert(v) => {
            enc.u32(HVE_VRF_CERT).map_err(enc_err)?;
            encode_vrf_cert_error(enc, v)?;
        }
        HeaderValidationError::OpCertCounter(o) => {
            enc.u32(HVE_OP_CERT_COUNTER).map_err(enc_err)?;
            encode_op_cert_error(enc, o)?;
        }
        HeaderValidationError::Nonce(n) => {
            enc.u32(HVE_NONCE).map_err(enc_err)?;
            encode_nonce_evolution_error(enc, n)?;
        }
        HeaderValidationError::SlotBeforeLastApplied { last, attempted } => {
            enc.u32(HVE_SLOT_BEFORE_LAST_APPLIED).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(last.0).map_err(enc_err)?;
            enc.u64(attempted.0).map_err(enc_err)?;
        }
        HeaderValidationError::BlockNoOutOfOrder { last, attempted } => {
            enc.u32(HVE_BLOCK_NO_OUT_OF_ORDER).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(last.0).map_err(enc_err)?;
            enc.u64(attempted.0).map_err(enc_err)?;
        }
        HeaderValidationError::BodyHashMismatch { expected, actual } => {
            enc.u32(HVE_BODY_HASH_MISMATCH).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            encode_hash32(enc, expected)?;
            encode_hash32(enc, actual)?;
        }
        HeaderValidationError::EraMismatch {
            schedule_era,
            header_era,
        } => {
            enc.u32(HVE_ERA_MISMATCH).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u8(*schedule_era).map_err(enc_err)?;
            enc.u8(*header_era).map_err(enc_err)?;
        }
        HeaderValidationError::HFC(h) => {
            enc.u32(HVE_HFC).map_err(enc_err)?;
            encode_hfc_error(enc, h)?;
        }
    }
    Ok(())
}

fn decode_header_validation_error(
    dec: &mut Decoder<'_>,
) -> Result<HeaderValidationError, DecodeError> {
    expect_array_len(dec, 2)?;
    let disc = dec.u32()?;
    match disc {
        HVE_VRF_CERT => Ok(HeaderValidationError::VrfCert(decode_vrf_cert_error(dec)?)),
        HVE_OP_CERT_COUNTER => Ok(HeaderValidationError::OpCertCounter(decode_op_cert_error(
            dec,
        )?)),
        HVE_NONCE => Ok(HeaderValidationError::Nonce(decode_nonce_evolution_error(
            dec,
        )?)),
        HVE_SLOT_BEFORE_LAST_APPLIED => {
            expect_array_len(dec, 2)?;
            let last = SlotNo(dec.u64()?);
            let attempted = SlotNo(dec.u64()?);
            Ok(HeaderValidationError::SlotBeforeLastApplied { last, attempted })
        }
        HVE_BLOCK_NO_OUT_OF_ORDER => {
            expect_array_len(dec, 2)?;
            let last = BlockNo(dec.u64()?);
            let attempted = BlockNo(dec.u64()?);
            Ok(HeaderValidationError::BlockNoOutOfOrder { last, attempted })
        }
        HVE_BODY_HASH_MISMATCH => {
            expect_array_len(dec, 2)?;
            let expected = decode_hash32(dec)?;
            let actual = decode_hash32(dec)?;
            Ok(HeaderValidationError::BodyHashMismatch { expected, actual })
        }
        HVE_ERA_MISMATCH => {
            expect_array_len(dec, 2)?;
            let schedule_era = dec.u8()?;
            let header_era = dec.u8()?;
            Ok(HeaderValidationError::EraMismatch {
                schedule_era,
                header_era,
            })
        }
        HVE_HFC => Ok(HeaderValidationError::HFC(decode_hfc_error(dec)?)),
        other => Err(DecodeError::UnknownDiscriminant {
            for_enum: "HeaderValidationError",
            found: other,
        }),
    }
}

// =============================================================================
// ChainSelectionReject
// =============================================================================

const CSR_FORK_BEFORE_IMMUTABLE_TIP: u32 = 0;
const CSR_EXCEEDED_ROLLBACK: u32 = 1;
const CSR_HEADER_INVALID: u32 = 2;
const CSR_TIEBREAKER_LOSS_KEEP_CURRENT: u32 = 3;

fn encode_chain_selection_reject(
    enc: &mut Encoder<&mut Vec<u8>>,
    r: &ChainSelectionReject,
) -> Result<(), DecodeError> {
    enc.array(2).map_err(enc_err)?;
    match r {
        ChainSelectionReject::ForkBeforeImmutableTip {
            immutable_tip,
            candidate_intersection,
            rollback_depth,
            security_param,
        } => {
            enc.u32(CSR_FORK_BEFORE_IMMUTABLE_TIP).map_err(enc_err)?;
            enc.array(4).map_err(enc_err)?;
            encode_point(enc, immutable_tip)?;
            encode_point(enc, candidate_intersection)?;
            enc.u64(rollback_depth.0).map_err(enc_err)?;
            enc.u64(security_param.0).map_err(enc_err)?;
        }
        ChainSelectionReject::ExceededRollback { requested, max } => {
            enc.u32(CSR_EXCEEDED_ROLLBACK).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            enc.u64(requested.0).map_err(enc_err)?;
            enc.u64(max.0).map_err(enc_err)?;
        }
        ChainSelectionReject::HeaderInvalid { at_point, reason } => {
            enc.u32(CSR_HEADER_INVALID).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            encode_point(enc, at_point)?;
            encode_header_validation_error(enc, reason)?;
        }
        ChainSelectionReject::TiebreakerLossKeepCurrent {
            current_tip,
            candidate_tip,
        } => {
            enc.u32(CSR_TIEBREAKER_LOSS_KEEP_CURRENT).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            encode_point(enc, current_tip)?;
            encode_point(enc, candidate_tip)?;
        }
    }
    Ok(())
}

fn decode_chain_selection_reject(dec: &mut Decoder<'_>) -> Result<ChainSelectionReject, DecodeError> {
    expect_array_len(dec, 2)?;
    let disc = dec.u32()?;
    match disc {
        CSR_FORK_BEFORE_IMMUTABLE_TIP => {
            expect_array_len(dec, 4)?;
            let immutable_tip = decode_point(dec)?;
            let candidate_intersection = decode_point(dec)?;
            let rollback_depth = BlockDistance(dec.u64()?);
            let security_param = SecurityParam(dec.u64()?);
            Ok(ChainSelectionReject::ForkBeforeImmutableTip {
                immutable_tip,
                candidate_intersection,
                rollback_depth,
                security_param,
            })
        }
        CSR_EXCEEDED_ROLLBACK => {
            expect_array_len(dec, 2)?;
            let requested = BlockDistance(dec.u64()?);
            let max = SecurityParam(dec.u64()?);
            Ok(ChainSelectionReject::ExceededRollback { requested, max })
        }
        CSR_HEADER_INVALID => {
            expect_array_len(dec, 2)?;
            let at_point = decode_point(dec)?;
            let reason = decode_header_validation_error(dec)?;
            Ok(ChainSelectionReject::HeaderInvalid { at_point, reason })
        }
        CSR_TIEBREAKER_LOSS_KEEP_CURRENT => {
            expect_array_len(dec, 2)?;
            let current_tip = decode_point(dec)?;
            let candidate_tip = decode_point(dec)?;
            Ok(ChainSelectionReject::TiebreakerLossKeepCurrent {
                current_tip,
                candidate_tip,
            })
        }
        other => Err(DecodeError::UnknownDiscriminant {
            for_enum: "ChainSelectionReject",
            found: other,
        }),
    }
}

// =============================================================================
// ChainEvent
// =============================================================================

const CE_CHAIN_EXTENDED: u32 = 0;
const CE_ROLLED_BACK: u32 = 1;
const CE_ROLLED_FORWARD: u32 = 2;
const CE_CHAIN_SELECTED: u32 = 3;
const CE_REJECTED: u32 = 4;

pub fn encode_chain_event(e: &ChainEvent) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    if encode_chain_event_into(&mut buf, e).is_err() {
        buf.clear();
    }
    buf
}

fn encode_chain_event_into(buf: &mut Vec<u8>, e: &ChainEvent) -> Result<(), DecodeError> {
    let mut enc = Encoder::new(buf);
    enc.array(2).map_err(enc_err)?;
    match e {
        ChainEvent::ChainExtended { new_tip, block_no } => {
            enc.u32(CE_CHAIN_EXTENDED).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            encode_point(&mut enc, new_tip)?;
            enc.u64(block_no.0).map_err(enc_err)?;
        }
        ChainEvent::RolledBack { to_point, depth } => {
            enc.u32(CE_ROLLED_BACK).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            encode_point(&mut enc, to_point)?;
            enc.u64(depth.0).map_err(enc_err)?;
        }
        ChainEvent::RolledForward { from, to } => {
            enc.u32(CE_ROLLED_FORWARD).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            encode_point(&mut enc, from)?;
            encode_point(&mut enc, to)?;
        }
        ChainEvent::ChainSelected {
            new_tip,
            replaced_tip,
        } => {
            enc.u32(CE_CHAIN_SELECTED).map_err(enc_err)?;
            enc.array(2).map_err(enc_err)?;
            encode_point(&mut enc, new_tip)?;
            match replaced_tip {
                Some(p) => {
                    enc.array(1).map_err(enc_err)?;
                    encode_point(&mut enc, p)?;
                }
                None => {
                    enc.array(0).map_err(enc_err)?;
                }
            }
        }
        ChainEvent::Rejected { reason } => {
            enc.u32(CE_REJECTED).map_err(enc_err)?;
            enc.array(1).map_err(enc_err)?;
            encode_chain_selection_reject(&mut enc, reason)?;
        }
    }
    Ok(())
}

pub fn decode_chain_event(bytes: &[u8]) -> Result<ChainEvent, DecodeError> {
    let mut dec = Decoder::new(bytes);
    expect_array_len(&mut dec, 2)?;
    let disc = dec.u32()?;
    match disc {
        CE_CHAIN_EXTENDED => {
            expect_array_len(&mut dec, 2)?;
            let new_tip = decode_point(&mut dec)?;
            let block_no = BlockNo(dec.u64()?);
            Ok(ChainEvent::ChainExtended { new_tip, block_no })
        }
        CE_ROLLED_BACK => {
            expect_array_len(&mut dec, 2)?;
            let to_point = decode_point(&mut dec)?;
            let depth = BlockDistance(dec.u64()?);
            Ok(ChainEvent::RolledBack { to_point, depth })
        }
        CE_ROLLED_FORWARD => {
            expect_array_len(&mut dec, 2)?;
            let from = decode_point(&mut dec)?;
            let to = decode_point(&mut dec)?;
            Ok(ChainEvent::RolledForward { from, to })
        }
        CE_CHAIN_SELECTED => {
            expect_array_len(&mut dec, 2)?;
            let new_tip = decode_point(&mut dec)?;
            let opt_len = dec
                .array()?
                .ok_or(DecodeError::Cbor("expected definite-length array"))?;
            let replaced_tip = match opt_len {
                0 => None,
                1 => Some(decode_point(&mut dec)?),
                _ => {
                    return Err(DecodeError::FieldCountMismatch {
                        expected: 1,
                        actual: if opt_len > u64::from(u32::MAX) {
                            u32::MAX
                        } else {
                            opt_len as u32
                        },
                    });
                }
            };
            Ok(ChainEvent::ChainSelected {
                new_tip,
                replaced_tip,
            })
        }
        CE_REJECTED => {
            expect_array_len(&mut dec, 1)?;
            let reason = decode_chain_selection_reject(&mut dec)?;
            Ok(ChainEvent::Rejected { reason })
        }
        other => Err(DecodeError::UnknownDiscriminant {
            for_enum: "ChainEvent",
            found: other,
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn op_cert_counter_map_iteration_is_deterministic() {
        let mut a = OpCertCounterMap::new();
        a.upsert_strict(Hash28([1u8; 28]), 5, 1).unwrap();
        a.upsert_strict(Hash28([2u8; 28]), 3, 2).unwrap();
        a.upsert_strict(Hash28([1u8; 28]), 7, 3).unwrap();
        a.upsert_strict(Hash28([3u8; 28]), 1, 4).unwrap();

        let mut b = OpCertCounterMap::new();
        b.upsert_strict(Hash28([3u8; 28]), 1, 4).unwrap();
        b.upsert_strict(Hash28([1u8; 28]), 7, 3).unwrap();
        b.upsert_strict(Hash28([2u8; 28]), 3, 2).unwrap();
        b.upsert_strict(Hash28([1u8; 28]), 5, 1).unwrap();

        let a_keys: Vec<(Hash28, u64)> = a.iter().map(|((p, k), _)| (p.clone(), *k)).collect();
        let b_keys: Vec<(Hash28, u64)> = b.iter().map(|((p, k), _)| (p.clone(), *k)).collect();
        assert_eq!(a_keys, b_keys);

        let expected = vec![
            (Hash28([1u8; 28]), 5),
            (Hash28([1u8; 28]), 7),
            (Hash28([2u8; 28]), 3),
            (Hash28([3u8; 28]), 1),
        ];
        assert_eq!(a_keys, expected);
    }
}
