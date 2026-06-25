// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `PraosChainDepState` snapshot encoder/decoder (PHASE4-N-J S1).
//!
//! Wire shape (canonical CBOR):
//! ```text
//! array(10) [   (legacy array(9) accepted on decode -> last_epoch_block_nonce = None)
//!   bytes(32)  evolving_nonce,
//!   bytes(32)  candidate_nonce,
//!   bytes(32)  epoch_nonce,
//!   bytes(32)  previous_epoch_nonce,
//!   bytes(32)  lab_nonce,
//!   null | uint  last_epoch_block,
//!   null | uint  last_slot,
//!   null | uint  last_block_no,
//!   array(N) [ array(3)[hash28, kes_period, counter], ... ]  op_cert_counters,
//!   null | bytes(32)  last_epoch_block_nonce
//! ]
//! ```
//!
//! All containers definite-length; BTreeMap iteration only.

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bytes, write_array_header,
    write_bytes_canonical, write_null, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_codec::CodecError;
use ade_core::consensus::praos_state::{Nonce, OpCertCounterMap, PraosChainDepState};
use ade_types::{BlockNo, EpochNo, Hash28, Hash32, SlotNo};

use super::error::{SnapshotDecodeError, SnapshotEncodeError, StructuralReason};

/// Current canonical arity: the 9 legacy fields + `last_epoch_block_nonce`
/// (always written on encode). Decode also accepts the legacy arity.
const FIELDS: u64 = 10;
/// Pre-DC-EPOCH-16 arity. A legacy store decodes with
/// `last_epoch_block_nonce = None` (explicit unset) — preserving its
/// already-promised within-epoch operation, but barred from the rolling
/// cross-boundary combine until re-seeded.
const FIELDS_LEGACY: u64 = 9;

pub fn encode_chain_dep(cd: &PraosChainDepState) -> Result<Vec<u8>, SnapshotEncodeError> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS, IntWidth::Inline),
    );
    write_nonce(&mut buf, &cd.evolving_nonce);
    write_nonce(&mut buf, &cd.candidate_nonce);
    write_nonce(&mut buf, &cd.epoch_nonce);
    write_nonce(&mut buf, &cd.previous_epoch_nonce);
    write_nonce(&mut buf, &cd.lab_nonce);
    write_opt_u64(&mut buf, cd.last_epoch_block.map(|e| e.0));
    write_opt_u64(&mut buf, cd.last_slot.map(|s| s.0));
    write_opt_u64(&mut buf, cd.last_block_no.map(|b| b.0));
    write_op_cert_counters(&mut buf, &cd.op_cert_counters);
    write_opt_nonce(&mut buf, &cd.last_epoch_block_nonce);
    Ok(buf)
}

pub fn decode_chain_dep(bytes: &[u8]) -> Result<PraosChainDepState, SnapshotDecodeError> {
    let mut o = 0usize;
    let arity = expect_array_9_or_10(bytes, &mut o)?;
    let evolving_nonce = read_nonce(bytes, &mut o)?;
    let candidate_nonce = read_nonce(bytes, &mut o)?;
    let epoch_nonce = read_nonce(bytes, &mut o)?;
    let previous_epoch_nonce = read_nonce(bytes, &mut o)?;
    let lab_nonce = read_nonce(bytes, &mut o)?;
    let last_epoch_block = read_opt_u64(bytes, &mut o)?.map(EpochNo);
    let last_slot = read_opt_u64(bytes, &mut o)?.map(SlotNo);
    let last_block_no = read_opt_u64(bytes, &mut o)?.map(BlockNo);
    let op_cert_counters = read_op_cert_counters(bytes, &mut o)?;
    // Legacy array(9) stores predate DC-EPOCH-16 -> explicit unset operand.
    let last_epoch_block_nonce = if arity == FIELDS {
        read_opt_nonce(bytes, &mut o)?
    } else {
        None
    };
    Ok(PraosChainDepState {
        evolving_nonce,
        candidate_nonce,
        epoch_nonce,
        previous_epoch_nonce,
        lab_nonce,
        last_epoch_block,
        last_epoch_block_nonce,
        last_slot,
        last_block_no,
        op_cert_counters,
    })
}

// ---------------------------------------------------------------------------
// Field helpers
// ---------------------------------------------------------------------------

fn write_nonce(buf: &mut Vec<u8>, n: &Nonce) {
    write_bytes_canonical(buf, &n.0 .0);
}

fn read_nonce(bytes: &[u8], o: &mut usize) -> Result<Nonce, SnapshotDecodeError> {
    let (b, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if b.len() != 32 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::NonceLengthMismatch,
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&b);
    Ok(Nonce(Hash32(arr)))
}

fn write_opt_u64(buf: &mut Vec<u8>, v: Option<u64>) {
    match v {
        Some(x) => write_uint_canonical(buf, x),
        None => write_null(buf),
    }
}

fn read_opt_u64(bytes: &[u8], o: &mut usize) -> Result<Option<u64>, SnapshotDecodeError> {
    // Peek at the byte to distinguish null (0xF6) from a uint.
    if *o >= bytes.len() {
        return Err(SnapshotDecodeError::Cbor(CodecError::UnexpectedEof {
            offset: *o,
            needed: 1,
        }));
    }
    if bytes[*o] == 0xF6 {
        *o += 1;
        return Ok(None);
    }
    let (v, _is_neg, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    Ok(Some(v))
}

fn write_op_cert_counters(buf: &mut Vec<u8>, m: &OpCertCounterMap) {
    write_array_header(
        buf,
        ContainerEncoding::Definite(m.len() as u64, canonical_width(m.len() as u64)),
    );
    for ((pool, kes_period), counter) in m.iter() {
        // Inner: array(3) [bytes(28) pool, uint kes_period, uint counter].
        write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
        write_bytes_canonical(buf, &pool.0);
        write_uint_canonical(buf, *kes_period);
        write_uint_canonical(buf, *counter);
    }
}

fn read_op_cert_counters(
    bytes: &[u8],
    o: &mut usize,
) -> Result<OpCertCounterMap, SnapshotDecodeError> {
    let enc = read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let n = match enc {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut map = OpCertCounterMap::new();
    for _ in 0..n {
        expect_array(bytes, o, 3)?;
        let (pool_bytes, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        if pool_bytes.len() != 28 {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::Hash28LengthMismatch,
            });
        }
        let mut pool = [0u8; 28];
        pool.copy_from_slice(&pool_bytes);
        let (kes_period, _isn, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        let (counter, _isn2, _w2) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        // upsert_strict is fine for fresh decode (each key unique).
        let _ = map.upsert_strict(Hash28(pool), kes_period, counter);
    }
    Ok(map)
}

fn expect_array(bytes: &[u8], o: &mut usize, expected_len: u64) -> Result<(), SnapshotDecodeError> {
    let enc = read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

/// Accept exactly the current `FIELDS` arity or the legacy `FIELDS_LEGACY`
/// arity, returning which one was read. Any other arity is structural.
fn expect_array_9_or_10(bytes: &[u8], o: &mut usize) -> Result<u64, SnapshotDecodeError> {
    let enc = read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == FIELDS || n == FIELDS_LEGACY => Ok(n),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

fn write_opt_nonce(buf: &mut Vec<u8>, v: &Option<Nonce>) {
    match v {
        Some(n) => write_bytes_canonical(buf, &n.0 .0),
        None => write_null(buf),
    }
}

fn read_opt_nonce(bytes: &[u8], o: &mut usize) -> Result<Option<Nonce>, SnapshotDecodeError> {
    if *o >= bytes.len() {
        return Err(SnapshotDecodeError::Cbor(CodecError::UnexpectedEof {
            offset: *o,
            needed: 1,
        }));
    }
    if bytes[*o] == 0xF6 {
        *o += 1;
        return Ok(None);
    }
    let (b, _w) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if b.len() != 32 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::NonceLengthMismatch,
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&b);
    Ok(Some(Nonce(Hash32(arr))))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample_empty() -> PraosChainDepState {
        PraosChainDepState::empty()
    }

    fn sample_full() -> PraosChainDepState {
        let mut s = PraosChainDepState::genesis(Nonce(Hash32([0xAB; 32])));
        s.evolving_nonce = Nonce(Hash32([0x01; 32]));
        s.candidate_nonce = Nonce(Hash32([0x02; 32]));
        s.epoch_nonce = Nonce(Hash32([0x03; 32]));
        s.previous_epoch_nonce = Nonce(Hash32([0x04; 32]));
        s.lab_nonce = Nonce(Hash32([0x05; 32]));
        s.last_epoch_block = Some(EpochNo(576));
        s.last_slot = Some(SlotNo(163_900_801));
        s.last_block_no = Some(BlockNo(9876));
        let _ = s.op_cert_counters.upsert_strict(Hash28([0xAA; 28]), 5, 100);
        let _ = s.op_cert_counters.upsert_strict(Hash28([0xAA; 28]), 6, 101);
        let _ = s.op_cert_counters.upsert_strict(Hash28([0xBB; 28]), 5, 50);
        s
    }

    #[test]
    fn chain_dep_round_trip_with_operand() {
        let mut s = sample_full();
        s.last_epoch_block_nonce = Some(Nonce(Hash32([0x06; 32])));
        let bytes = encode_chain_dep(&s).expect("encode");
        // Always writes the array(10) form.
        assert_eq!(bytes[0], 0x8a, "encode must write array(10)");
        let decoded = decode_chain_dep(&bytes).expect("decode");
        assert_eq!(decoded, s);
        assert_eq!(
            decoded.last_epoch_block_nonce,
            Some(Nonce(Hash32([0x06; 32])))
        );
    }

    #[test]
    fn chain_dep_legacy_array9_decodes_to_none() {
        // A pre-DC-EPOCH-16 store: the canonical legacy array(9) form, with no
        // last_epoch_block_nonce field. It must decode with the operand explicitly
        // absent (None) — preserving within-epoch operation, barred from the
        // rolling cross-boundary combine.
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(9, IntWidth::Inline));
        for b in [0x01u8, 0x02, 0x03, 0x04, 0x05] {
            write_bytes_canonical(&mut buf, &[b; 32]); // 5 nonces
        }
        write_null(&mut buf); // last_epoch_block = None
        write_null(&mut buf); // last_slot = None
        write_null(&mut buf); // last_block_no = None
        write_array_header(&mut buf, ContainerEncoding::Definite(0, IntWidth::Inline)); // op_cert_counters
        let decoded = decode_chain_dep(&buf).expect("legacy array(9) decodes");
        assert_eq!(decoded.last_epoch_block_nonce, None);
        assert_eq!(decoded.evolving_nonce, Nonce(Hash32([0x01; 32])));
        assert_eq!(decoded.lab_nonce, Nonce(Hash32([0x05; 32])));
    }

    #[test]
    fn chain_dep_round_trip_empty() {
        let s = sample_empty();
        let bytes = encode_chain_dep(&s).expect("encode");
        let decoded = decode_chain_dep(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn chain_dep_round_trip_full() {
        let s = sample_full();
        let bytes = encode_chain_dep(&s).expect("encode");
        let decoded = decode_chain_dep(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn chain_dep_encode_deterministic_across_runs() {
        let s = sample_full();
        let a = encode_chain_dep(&s).expect("encode a");
        let b = encode_chain_dep(&s).expect("encode b");
        assert_eq!(a, b, "encoder must be byte-identical across runs");
    }

    #[test]
    fn chain_dep_decode_rejects_truncated() {
        let s = sample_empty();
        let bytes = encode_chain_dep(&s).expect("encode");
        // Cut off the last byte.
        let truncated = &bytes[..bytes.len() - 1];
        let err = decode_chain_dep(truncated).expect_err("must reject truncated");
        match err {
            SnapshotDecodeError::Cbor(_) | SnapshotDecodeError::Structural { .. } => {}
            other => panic!("expected Cbor or Structural, got {other:?}"),
        }
    }

    #[test]
    fn chain_dep_decode_rejects_wrong_array_length() {
        // Build an array(8) instead of array(9).
        let mut bytes = Vec::new();
        write_array_header(&mut bytes, ContainerEncoding::Definite(8, IntWidth::Inline));
        for _ in 0..8 {
            write_null(&mut bytes);
        }
        let err = decode_chain_dep(&bytes).expect_err("must reject");
        match err {
            SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            } => {}
            other => panic!("expected ArrayLengthMismatch, got {other:?}"),
        }
    }
}
