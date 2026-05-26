// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE WAL closed-sum entries (PHASE4-N-M-A S3).
//!
//! Single variant in this sub-cluster: `AdmitBlock`. Future
//! entries (`RollBackward`, `CaptureSnapshot`) are additive.
//!
//! Each entry carries fingerprint deltas: `prior_fp` MUST equal
//! the previous entry's `post_fp` (or the anchor's
//! `initial_ledger_fingerprint` for the first entry). DC-WAL-02.

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_uint, write_array_header,
    write_bytes_canonical, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_types::{Hash32, SlotNo};

use super::error::WalError;

/// Wire tag for `WalEntry::AdmitBlock`. Future variants
/// allocate distinct tags additively (1 = RollBackward,
/// 2 = CaptureSnapshot, etc.).
pub const TAG_ADMIT_BLOCK: u64 = 0;

/// Closed sum: every authority-affecting forward step recorded
/// in the WAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalEntry {
    /// One admit pass through CN-CONS-08 produced a new
    /// canonical ledger state. `prior_fp` is the chain link
    /// back to the previous step (or the anchor for the first
    /// entry).
    AdmitBlock {
        prior_fp: Hash32,
        block_hash: Hash32,
        slot: SlotNo,
        verdict: BlockVerdictTag,
        post_fp: Hash32,
    },
}

/// Closed tag for the block-validity verdict. Mirrors the BLUE
/// `BlockValidityVerdict` discriminant. Compact: no payload
/// needed for replay-equivalence — the post_fp encodes the
/// effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockVerdictTag {
    Valid,
    Invalid,
}

impl BlockVerdictTag {
    pub fn wire_code(self) -> u64 {
        match self {
            Self::Valid => 0,
            Self::Invalid => 1,
        }
    }
    pub fn from_wire_code(code: u64) -> Option<Self> {
        match code {
            0 => Some(Self::Valid),
            1 => Some(Self::Invalid),
            _ => None,
        }
    }
}

impl WalEntry {
    pub fn prior_fp(&self) -> Hash32 {
        match self {
            Self::AdmitBlock { prior_fp, .. } => prior_fp.clone(),
        }
    }

    pub fn post_fp(&self) -> Hash32 {
        match self {
            Self::AdmitBlock { post_fp, .. } => post_fp.clone(),
        }
    }
}

/// Canonical CBOR encode for a single entry. Wire shape:
/// ```text
/// array(2) [ uint TAG, payload ]
///
/// AdmitBlock payload (TAG=0):
///   array(5) [
///     bytes(32) prior_fp,
///     bytes(32) block_hash,
///     uint slot,
///     uint verdict_code,
///     bytes(32) post_fp,
///   ]
/// ```
pub fn encode_wal_entry(entry: &WalEntry) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    match entry {
        WalEntry::AdmitBlock {
            prior_fp,
            block_hash,
            slot,
            verdict,
            post_fp,
        } => {
            write_uint_canonical(&mut buf, TAG_ADMIT_BLOCK);
            write_array_header(&mut buf, ContainerEncoding::Definite(5, canonical_width(5)));
            write_bytes_canonical(&mut buf, &prior_fp.0);
            write_bytes_canonical(&mut buf, &block_hash.0);
            write_uint_canonical(&mut buf, slot.0);
            write_uint_canonical(&mut buf, verdict.wire_code());
            write_bytes_canonical(&mut buf, &post_fp.0);
        }
    }
    buf
}

/// Canonical CBOR decode for a single entry.
pub fn decode_wal_entry(bytes: &[u8]) -> Result<(WalEntry, usize), WalError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, 2, "entry wrapper")?;
    let (tag, _w): (u64, IntWidth) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
    match tag {
        TAG_ADMIT_BLOCK => {
            expect_definite_array(bytes, &mut o, 5, "AdmitBlock payload")?;
            let prior_fp = read_hash32(bytes, &mut o)?;
            let block_hash = read_hash32(bytes, &mut o)?;
            let (slot, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let (verdict_code, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let verdict = BlockVerdictTag::from_wire_code(verdict_code)
                .ok_or(WalError::Structural { reason: "unknown verdict code" })?;
            let post_fp = read_hash32(bytes, &mut o)?;
            Ok((
                WalEntry::AdmitBlock {
                    prior_fp,
                    block_hash,
                    slot: SlotNo(slot),
                    verdict,
                    post_fp,
                },
                o,
            ))
        }
        _ => Err(WalError::Structural { reason: "unknown wal entry tag" }),
    }
}

fn expect_definite_array(
    bytes: &[u8],
    offset: &mut usize,
    expected_len: u64,
    label: &'static str,
) -> Result<(), WalError> {
    let enc = read_array_header(bytes, offset).map_err(WalError::Decode)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        ContainerEncoding::Definite(_, _) => Err(WalError::Structural {
            reason: match label {
                "entry wrapper" => "entry wrapper had wrong array length",
                "AdmitBlock payload" => "AdmitBlock payload had wrong array length",
                _ => "unknown array shape",
            },
        }),
        ContainerEncoding::Indefinite => Err(WalError::Structural {
            reason: "indefinite-length array not allowed in WAL",
        }),
    }
}

fn read_hash32(bytes: &[u8], offset: &mut usize) -> Result<Hash32, WalError> {
    let (h, _w) = read_bytes(bytes, offset).map_err(WalError::Decode)?;
    if h.len() != 32 {
        return Err(WalError::Structural {
            reason: "expected 32-byte hash",
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&h);
    Ok(Hash32(arr))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample() -> WalEntry {
        WalEntry::AdmitBlock {
            prior_fp: Hash32([0x55; 32]),
            block_hash: Hash32([0x66; 32]),
            slot: SlotNo(23013664),
            verdict: BlockVerdictTag::Valid,
            post_fp: Hash32([0x77; 32]),
        }
    }

    #[test]
    fn wal_entry_admit_block_round_trips_canonical_cbor() {
        let e = sample();
        let bytes = encode_wal_entry(&e);
        let (decoded, consumed) = decode_wal_entry(&bytes).expect("decode");
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, e);
    }

    #[test]
    fn wal_entry_encode_two_runs_byte_identical() {
        assert_eq!(encode_wal_entry(&sample()), encode_wal_entry(&sample()));
    }

    #[test]
    fn wal_entry_decode_rejects_unknown_tag() {
        // Hand-craft array(2)[uint 99, ...].
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        write_uint_canonical(&mut buf, 99);
        // Filler so the (unread) payload doesn't underflow.
        write_uint_canonical(&mut buf, 0);
        let res = decode_wal_entry(&buf);
        match res {
            Err(WalError::Structural { reason }) if reason.contains("unknown wal entry tag") => {}
            other => panic!("expected unknown tag, got {other:?}"),
        }
    }

    #[test]
    fn wal_entry_decode_rejects_unknown_verdict() {
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        write_uint_canonical(&mut buf, TAG_ADMIT_BLOCK);
        write_array_header(&mut buf, ContainerEncoding::Definite(5, canonical_width(5)));
        write_bytes_canonical(&mut buf, &[0u8; 32]);
        write_bytes_canonical(&mut buf, &[0u8; 32]);
        write_uint_canonical(&mut buf, 0);
        write_uint_canonical(&mut buf, 99); // unknown verdict code
        write_bytes_canonical(&mut buf, &[0u8; 32]);
        let res = decode_wal_entry(&buf);
        match res {
            Err(WalError::Structural { reason }) if reason.contains("verdict") => {}
            other => panic!("expected unknown verdict, got {other:?}"),
        }
    }

    #[test]
    fn block_verdict_tag_round_trips_wire_code() {
        for v in [BlockVerdictTag::Valid, BlockVerdictTag::Invalid] {
            assert_eq!(BlockVerdictTag::from_wire_code(v.wire_code()), Some(v));
        }
        assert_eq!(BlockVerdictTag::from_wire_code(99), None);
    }

    #[test]
    fn wal_entry_match_is_exhaustive() {
        let e = sample();
        match &e {
            WalEntry::AdmitBlock {
                prior_fp,
                block_hash,
                slot,
                verdict,
                post_fp,
            } => {
                assert_eq!(prior_fp.0[0], 0x55);
                assert_eq!(block_hash.0[0], 0x66);
                assert_eq!(slot.0, 23013664);
                assert!(matches!(verdict, BlockVerdictTag::Valid));
                assert_eq!(post_fp.0[0], 0x77);
            }
        }
    }
}
