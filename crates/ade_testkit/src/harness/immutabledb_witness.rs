//! CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY — the selected-chain WITNESS reader (GREEN, operational
//! evidence tooling).
//!
//! A decoded-state fingerprint proves only *"these bytes decode to this state"*, NOT *"these came from this
//! exact selected-chain block at this exact boundary"* — two chain points can share a projection, so a state
//! commitment cannot distinguish a wrong branch / wrong snapshot point / extraction mismatch. This narrow
//! read-only reader of the cardano-node ImmutableDB (`.secondary` index + `.chunk`) emits the canonical
//! chain point for a slot — (block_no, block_header_hash, parent_header_hash, era_tag) — so each ground-truth
//! census row is bound to an unambiguous Cardano chain identity.
//!
//! Reuses `ade_crypto::block_header_hash` (the hash is recomputed from the real header bytes and
//! cross-checked against the index's stored hash). NO BLUE ledger semantics, no runtime dependency, no
//! governance scope. See the `feedback_chain_point_witness_not_state_fingerprint` methodology note.
//!
//! SCOPE: this reader targets the Praos window (Shelley-onward, NO Byron EBBs). It treats every
//! secondary-index `blockOrEBB` as an absolute slot and reads the block-wrapper era tag as a single byte
//! (valid because every Cardano HFC era index is < 24). The blake2b cross-check would REJECT (not silently
//! misread) a byte-misalignment, but a Byron-EBB chunk is outside this reader's contract — extend it before
//! reuse there. The CRE census window is entirely Conway (HFC era tag 7), asserted per row.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// A cardano-node ImmutableDB secondary-index entry is 56 bytes, big-endian:
/// `blockOffset:u64, headerOffset:u16, headerSize:u16, checksum:u32, headerHash:[u8;32], blockOrEBB:u64`
/// (`blockOrEBB` = the absolute slot for a regular block). Verified empirically against preview chunk 0
/// (entry 0 = slot 0, entry 1 = slot 20).
const SECONDARY_ENTRY_LEN: usize = 56;

/// The canonical selected-chain witness for one boundary slot — the chain-identity binding a decoded-state
/// fingerprint cannot provide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainPointWitness {
    pub slot: u64,
    pub block_no: u64,
    pub block_header_hash: [u8; 32],
    /// The block's parent, from the header's `prevHash` field. `None` only for a genesis-successor block
    /// (CBOR null); every real boundary block in this census carries `Some`.
    pub parent_header_hash: Option<[u8; 32]>,
    /// The HFC era index from the block wrapper `[eraTag, block]` (provenance; the census cross-checks it is
    /// constant across the window and consistent with the decoded-state era).
    pub era_tag: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessError {
    Io(String),
    /// No block is stored at exactly this slot (the census snapshot slot must be a real chain block).
    NotFound { slot: u64 },
    Malformed(String),
    /// The header bytes at the index offsets do not hash to the index's stored header hash — a corrupt read
    /// or a wrong offset. TERMINAL (never a coerced witness).
    HashMismatch { slot: u64 },
}

impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WitnessError::Io(s) => write!(f, "io: {s}"),
            WitnessError::NotFound { slot } => write!(f, "no block at slot {slot}"),
            WitnessError::Malformed(s) => write!(f, "malformed: {s}"),
            WitnessError::HashMismatch { slot } => write!(f, "header-hash mismatch at slot {slot}"),
        }
    }
}
impl std::error::Error for WitnessError {}

struct SecEntry {
    block_offset: u64,
    header_offset: u16,
    header_size: u16,
    header_hash: [u8; 32],
    slot: u64,
}

fn be_u16(b: &[u8]) -> u16 {
    u16::from_be_bytes([b[0], b[1]])
}
fn be_u64(b: &[u8]) -> u64 {
    u64::from_be_bytes(b[0..8].try_into().unwrap())
}

fn parse_secondary(bytes: &[u8]) -> Result<Vec<SecEntry>, WitnessError> {
    if bytes.len() % SECONDARY_ENTRY_LEN != 0 {
        return Err(WitnessError::Malformed(format!(
            "secondary length {} not a multiple of {SECONDARY_ENTRY_LEN}",
            bytes.len()
        )));
    }
    let mut out = Vec::with_capacity(bytes.len() / SECONDARY_ENTRY_LEN);
    for c in bytes.chunks_exact(SECONDARY_ENTRY_LEN) {
        let mut hh = [0u8; 32];
        hh.copy_from_slice(&c[16..48]);
        out.push(SecEntry {
            block_offset: be_u64(&c[0..8]),
            header_offset: be_u16(&c[8..10]),
            header_size: be_u16(&c[10..12]),
            header_hash: hh,
            slot: be_u64(&c[48..56]),
        });
    }
    Ok(out)
}

fn read_secondary(dir: &Path, chunk: u32) -> Result<Vec<SecEntry>, WitnessError> {
    let p = dir.join(format!("{chunk:05}.secondary"));
    let bytes = std::fs::read(&p).map_err(|e| WitnessError::Io(format!("{}: {e}", p.display())))?;
    parse_secondary(&bytes)
}

/// The highest chunk index present (from the `.secondary` file names).
fn max_chunk(dir: &Path) -> Result<u32, WitnessError> {
    let mut mx = 0u32;
    let mut any = false;
    for e in std::fs::read_dir(dir).map_err(|e| WitnessError::Io(e.to_string()))? {
        let e = e.map_err(|e| WitnessError::Io(e.to_string()))?;
        if let Some(name) = e.file_name().to_str() {
            if let Some(stem) = name.strip_suffix(".secondary") {
                if let Ok(n) = stem.parse::<u32>() {
                    mx = mx.max(n);
                    any = true;
                }
            }
        }
    }
    if !any {
        return Err(WitnessError::Malformed("no .secondary files in immutable dir".into()));
    }
    Ok(mx)
}

/// Binary-search the chunk whose slot range contains `slot`. Assumes the chunks along the search path are
/// non-empty (true for the dense Conway range this census covers); an empty chunk in the path is TERMINAL
/// rather than silently skipped.
fn locate_chunk(dir: &Path, slot: u64, max: u32) -> Result<u32, WitnessError> {
    let (mut lo, mut hi) = (0u32, max);
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let sec = read_secondary(dir, mid)?;
        if sec.is_empty() {
            return Err(WitnessError::Malformed(format!(
                "empty chunk {mid} on the search path for slot {slot}"
            )));
        }
        let first = sec.first().unwrap().slot;
        let last = sec.last().unwrap().slot;
        if slot < first {
            if mid == 0 {
                break;
            }
            hi = mid - 1;
        } else if slot > last {
            lo = mid + 1;
        } else {
            return Ok(mid);
        }
    }
    Err(WitnessError::NotFound { slot })
}

/// Read one canonical CBOR unsigned integer (major type 0) at `o`; returns `(value, next_offset)`.
fn cbor_uint(b: &[u8], o: usize) -> Result<(u64, usize), WitnessError> {
    let ib = *b.get(o).ok_or_else(|| WitnessError::Malformed("uint: eof".into()))?;
    if ib >> 5 != 0 {
        return Err(WitnessError::Malformed(format!("uint: major type {} != 0", ib >> 5)));
    }
    let ai = ib & 0x1f;
    let need = |n: usize| b.get(o + 1..o + 1 + n).ok_or_else(|| WitnessError::Malformed("uint: eof".into()));
    match ai {
        0..=23 => Ok((ai as u64, o + 1)),
        24 => Ok((need(1)?[0] as u64, o + 2)),
        25 => {
            let s = need(2)?;
            Ok((u16::from_be_bytes([s[0], s[1]]) as u64, o + 3))
        }
        26 => {
            let s = need(4)?;
            Ok((u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as u64, o + 5))
        }
        27 => {
            let s = need(8)?;
            Ok((u64::from_be_bytes(s.try_into().unwrap()), o + 9))
        }
        _ => Err(WitnessError::Malformed(format!("uint: reserved additional info {ai}"))),
    }
}

/// Parse the block header's leading fields: `array(2)[ header_body=array(15|10)[ block_no, slot, prevHash,
/// .. ], signature ]`. Returns `(block_no, parent_header_hash)`; `prevHash` is CBOR `null` (Genesis) or
/// `bytes(32)` (the parent). Only the first three body fields are read — enough for the chain-point witness.
fn parse_block_no_and_parent(header: &[u8]) -> Result<(u64, Option<[u8; 32]>), WitnessError> {
    if header.first() != Some(&0x82) {
        return Err(WitnessError::Malformed("header is not array(2)".into()));
    }
    let bb = *header.get(1).ok_or_else(|| WitnessError::Malformed("header_body: eof".into()))?;
    if bb != 0x8f && bb != 0x8a {
        return Err(WitnessError::Malformed(format!(
            "header_body is not array(15|10): {bb:02x}"
        )));
    }
    let (block_no, o) = cbor_uint(header, 2)?;
    let (_slot, o) = cbor_uint(header, o)?;
    let pb = *header.get(o).ok_or_else(|| WitnessError::Malformed("prevHash: eof".into()))?;
    let parent = if pb == 0xf6 {
        None
    } else if pb == 0x58 && header.get(o + 1) == Some(&0x20) {
        let mut h = [0u8; 32];
        h.copy_from_slice(
            header
                .get(o + 2..o + 34)
                .ok_or_else(|| WitnessError::Malformed("prevHash: bytes32 eof".into()))?,
        );
        Some(h)
    } else {
        return Err(WitnessError::Malformed(format!(
            "prevHash: expected null or bytes(32), got {pb:02x}"
        )));
    };
    Ok((block_no, parent))
}

/// Emit the canonical selected-chain witness for the block stored at exactly `slot` in the ImmutableDB at
/// `immutable_dir` (the `.../db/immutable` directory). Fail-closed: a missing block, an unparsable index, or
/// a header that does not hash to the index's stored hash is TERMINAL.
pub fn witness_for_slot(immutable_dir: &Path, slot: u64) -> Result<ChainPointWitness, WitnessError> {
    let max = max_chunk(immutable_dir)?;
    let chunk = locate_chunk(immutable_dir, slot, max)?;
    let sec = read_secondary(immutable_dir, chunk)?;
    let entry = sec
        .iter()
        .find(|e| e.slot == slot)
        .ok_or(WitnessError::NotFound { slot })?;

    let chunk_path = immutable_dir.join(format!("{chunk:05}.chunk"));
    let mut f = File::open(&chunk_path).map_err(|e| WitnessError::Io(format!("{}: {e}", chunk_path.display())))?;
    // The header sits at blockOffset + headerOffset for headerSize bytes.
    f.seek(SeekFrom::Start(entry.block_offset + entry.header_offset as u64))
        .map_err(|e| WitnessError::Io(e.to_string()))?;
    let mut header = vec![0u8; entry.header_size as usize];
    f.read_exact(&mut header).map_err(|e| WitnessError::Io(e.to_string()))?;
    // Cross-check: the header bytes MUST hash (blake2b-256, Ade's own primitive) to the index's stored hash.
    if ade_crypto::block_header_hash(&header).0 != entry.header_hash {
        return Err(WitnessError::HashMismatch { slot });
    }

    // The era tag is the block wrapper's first element: `[eraTag, block]` → byte blockOffset+1 (a CBOR uint
    // < 24 for every Cardano era, so a single byte).
    f.seek(SeekFrom::Start(entry.block_offset))
        .map_err(|e| WitnessError::Io(e.to_string()))?;
    let mut wrap = [0u8; 2];
    f.read_exact(&mut wrap).map_err(|e| WitnessError::Io(e.to_string()))?;
    if wrap[0] != 0x82 {
        return Err(WitnessError::Malformed(format!(
            "block wrapper is not array(2): {:02x}",
            wrap[0]
        )));
    }
    let era_tag = wrap[1];

    let (block_no, parent_header_hash) = parse_block_no_and_parent(&header)?;
    Ok(ChainPointWitness {
        slot,
        block_no,
        block_header_hash: entry.header_hash,
        parent_header_hash,
        era_tag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_secondary_rejects_ragged_length() {
        assert!(matches!(parse_secondary(&[0u8; 55]), Err(WitnessError::Malformed(_))));
        assert!(parse_secondary(&[0u8; 112]).is_ok());
    }

    #[test]
    fn cbor_uint_widths() {
        assert_eq!(cbor_uint(&[0x0a], 0).unwrap(), (10, 1));
        assert_eq!(cbor_uint(&[0x18, 0xff], 0).unwrap(), (255, 2));
        assert_eq!(cbor_uint(&[0x19, 0x01, 0x00], 0).unwrap(), (256, 3));
        assert_eq!(cbor_uint(&[0x1a, 0x00, 0x0f, 0x42, 0x40], 0).unwrap(), (1_000_000, 5));
        assert!(cbor_uint(&[0x40], 0).is_err(), "major type 2 is not a uint");
    }

    #[test]
    fn parse_header_block_no_and_parent() {
        // array(2)[ array(15)[ block_no=0x0186a0(100000), slot=0x14, prevHash=bytes32(0x11..), .. ], sig ]
        let mut h = vec![0x82, 0x8f, 0x1a, 0x00, 0x01, 0x86, 0xa0, 0x14, 0x58, 0x20];
        h.extend_from_slice(&[0x11; 32]);
        let (bn, parent) = parse_block_no_and_parent(&h).unwrap();
        assert_eq!(bn, 100_000);
        assert_eq!(parent, Some([0x11; 32]));
        // genesis-successor: prevHash = null
        let g = vec![0x82, 0x8a, 0x00, 0x00, 0xf6];
        assert_eq!(parse_block_no_and_parent(&g).unwrap(), (0, None));
    }
}
