// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

pub mod envelope;

use crate::error::CodecError;

// CBOR major type constants
pub const MAJOR_UNSIGNED: u8 = 0;
pub const MAJOR_NEGATIVE: u8 = 1;
pub const MAJOR_BYTES: u8 = 2;
pub const MAJOR_TEXT: u8 = 3;
pub const MAJOR_ARRAY: u8 = 4;
pub const MAJOR_MAP: u8 = 5;
pub const MAJOR_TAG: u8 = 6;
pub const MAJOR_SIMPLE: u8 = 7;

// Additional info thresholds
const AI_ONE_BYTE: u8 = 24;
const AI_TWO_BYTES: u8 = 25;
const AI_FOUR_BYTES: u8 = 26;
const AI_EIGHT_BYTES: u8 = 27;
const AI_INDEFINITE: u8 = 31;

const BREAK_BYTE: u8 = 0xff;

/// Encoding width of a CBOR integer/length argument.
///
/// Records how a value was encoded on the wire, enabling exact
/// round-trip re-encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntWidth {
    /// Value 0-23, encoded inline in the initial byte.
    Inline,
    /// Additional info 24, 1 byte follows.
    I8,
    /// Additional info 25, 2 bytes follow.
    I16,
    /// Additional info 26, 4 bytes follow.
    I32,
    /// Additional info 27, 8 bytes follow.
    I64,
}

/// Container (array/map) length encoding metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerEncoding {
    /// Definite-length with element count and encoding width.
    Definite(u64, IntWidth),
    /// Indefinite-length (terminated by break byte).
    Indefinite,
}

// ---------------------------------------------------------------------------
// Reading primitives
// ---------------------------------------------------------------------------

fn peek_byte(data: &[u8], offset: usize) -> Result<u8, CodecError> {
    data.get(offset)
        .copied()
        .ok_or(CodecError::UnexpectedEof { offset, needed: 1 })
}

fn read_byte(data: &[u8], offset: &mut usize) -> Result<u8, CodecError> {
    let b = peek_byte(data, *offset)?;
    *offset += 1;
    Ok(b)
}

/// Decode a CBOR initial byte into (major_type, additional_info).
fn decode_initial(data: &[u8], offset: &mut usize) -> Result<(u8, u8), CodecError> {
    let b = read_byte(data, offset)?;
    Ok((b >> 5, b & 0x1f))
}

/// Decode the argument value from additional info (0..=27).
///
/// The caller must ensure `ai` is in range 0..=27. Values 28..=30 are
/// reserved; 31 is indefinite (handled by callers before calling this).
fn decode_argument(
    data: &[u8],
    offset: &mut usize,
    ai: u8,
    start_offset: usize,
) -> Result<(u64, IntWidth), CodecError> {
    match ai {
        0..=23 => Ok((u64::from(ai), IntWidth::Inline)),
        AI_ONE_BYTE => {
            let b = read_byte(data, offset)?;
            Ok((u64::from(b), IntWidth::I8))
        }
        AI_TWO_BYTES => {
            if *offset + 2 > data.len() {
                return Err(CodecError::UnexpectedEof {
                    offset: *offset,
                    needed: 2,
                });
            }
            let val = u16::from_be_bytes([data[*offset], data[*offset + 1]]);
            *offset += 2;
            Ok((u64::from(val), IntWidth::I16))
        }
        AI_FOUR_BYTES => {
            if *offset + 4 > data.len() {
                return Err(CodecError::UnexpectedEof {
                    offset: *offset,
                    needed: 4,
                });
            }
            let val = u32::from_be_bytes([
                data[*offset],
                data[*offset + 1],
                data[*offset + 2],
                data[*offset + 3],
            ]);
            *offset += 4;
            Ok((u64::from(val), IntWidth::I32))
        }
        AI_EIGHT_BYTES => {
            if *offset + 8 > data.len() {
                return Err(CodecError::UnexpectedEof {
                    offset: *offset,
                    needed: 8,
                });
            }
            let val = u64::from_be_bytes([
                data[*offset],
                data[*offset + 1],
                data[*offset + 2],
                data[*offset + 3],
                data[*offset + 4],
                data[*offset + 5],
                data[*offset + 6],
                data[*offset + 7],
            ]);
            *offset += 8;
            Ok((val, IntWidth::I64))
        }
        _ => Err(CodecError::InvalidCborStructure {
            offset: start_offset,
            detail: "reserved additional info value (28-30)",
        }),
    }
}

/// Peek at the CBOR major type at the given offset without advancing.
pub fn peek_major(data: &[u8], offset: usize) -> Result<u8, CodecError> {
    let b = peek_byte(data, offset)?;
    Ok(b >> 5)
}

/// Read a CBOR unsigned integer (major type 0).
/// Returns (value, encoding width).
pub fn read_uint(data: &[u8], offset: &mut usize) -> Result<(u64, IntWidth), CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;
    if major != MAJOR_UNSIGNED {
        return Err(CodecError::UnexpectedCborType {
            offset: start,
            expected: "unsigned integer",
            actual: major,
        });
    }
    decode_argument(data, offset, ai, start)
}

/// Read a CBOR byte string (major type 2), returning the raw bytes and length width.
///
/// Only supports definite-length byte strings. Indefinite-length byte strings
/// are not used in Cardano block encoding.
pub fn read_bytes(data: &[u8], offset: &mut usize) -> Result<(Vec<u8>, IntWidth), CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;
    if major != MAJOR_BYTES {
        return Err(CodecError::UnexpectedCborType {
            offset: start,
            expected: "byte string",
            actual: major,
        });
    }
    if ai == AI_INDEFINITE {
        return Err(CodecError::InvalidCborStructure {
            offset: start,
            detail: "indefinite-length byte string not supported",
        });
    }
    let (len, width) = decode_argument(data, offset, ai, start)?;
    let len = len as usize;
    if *offset + len > data.len() {
        return Err(CodecError::UnexpectedEof {
            offset: *offset,
            needed: len,
        });
    }
    let bytes = data[*offset..*offset + len].to_vec();
    *offset += len;
    Ok((bytes, width))
}

/// Read a CBOR array header (major type 4).
pub fn read_array_header(data: &[u8], offset: &mut usize) -> Result<ContainerEncoding, CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;
    if major != MAJOR_ARRAY {
        return Err(CodecError::UnexpectedCborType {
            offset: start,
            expected: "array",
            actual: major,
        });
    }
    if ai == AI_INDEFINITE {
        return Ok(ContainerEncoding::Indefinite);
    }
    let (count, width) = decode_argument(data, offset, ai, start)?;
    Ok(ContainerEncoding::Definite(count, width))
}

/// Read a CBOR map header (major type 5).
pub fn read_map_header(data: &[u8], offset: &mut usize) -> Result<ContainerEncoding, CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;
    if major != MAJOR_MAP {
        return Err(CodecError::UnexpectedCborType {
            offset: start,
            expected: "map",
            actual: major,
        });
    }
    if ai == AI_INDEFINITE {
        return Ok(ContainerEncoding::Indefinite);
    }
    let (count, width) = decode_argument(data, offset, ai, start)?;
    Ok(ContainerEncoding::Definite(count, width))
}

/// Read a CBOR tag (major type 6). Returns (tag_value, encoding width).
pub fn read_tag(data: &[u8], offset: &mut usize) -> Result<(u64, IntWidth), CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;
    if major != MAJOR_TAG {
        return Err(CodecError::UnexpectedCborType {
            offset: start,
            expected: "tag",
            actual: major,
        });
    }
    decode_argument(data, offset, ai, start)
}

/// Read any CBOR integer (major type 0 or 1).
/// Returns (value, is_negative, encoding width).
///
/// For major type 0: value is the unsigned integer.
/// For major type 1: the actual value is -(1 + returned_value).
pub fn read_any_int(data: &[u8], offset: &mut usize) -> Result<(u64, bool, IntWidth), CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;
    match major {
        MAJOR_UNSIGNED => {
            let (val, width) = decode_argument(data, offset, ai, start)?;
            Ok((val, false, width))
        }
        MAJOR_NEGATIVE => {
            let (val, width) = decode_argument(data, offset, ai, start)?;
            Ok((val, true, width))
        }
        _ => Err(CodecError::UnexpectedCborType {
            offset: start,
            expected: "integer",
            actual: major,
        }),
    }
}

/// Skip over an entire CBOR item, advancing offset past it.
/// Returns the byte range `[start, end)` of the skipped item.
pub fn skip_item(data: &[u8], offset: &mut usize) -> Result<(usize, usize), CodecError> {
    let start = *offset;
    let (major, ai) = decode_initial(data, offset)?;

    match major {
        MAJOR_UNSIGNED | MAJOR_NEGATIVE => {
            if ai < AI_ONE_BYTE {
                // value inline, nothing more to read
            } else {
                let _ = decode_argument(data, offset, ai, start)?;
            }
        }
        MAJOR_BYTES | MAJOR_TEXT => {
            if ai == AI_INDEFINITE {
                loop {
                    let b = peek_byte(data, *offset)?;
                    if b == BREAK_BYTE {
                        *offset += 1;
                        break;
                    }
                    let _ = skip_item(data, offset)?;
                }
            } else {
                let (len, _) = decode_argument(data, offset, ai, start)?;
                let len = len as usize;
                if *offset + len > data.len() {
                    return Err(CodecError::UnexpectedEof {
                        offset: *offset,
                        needed: len,
                    });
                }
                *offset += len;
            }
        }
        MAJOR_ARRAY => {
            if ai == AI_INDEFINITE {
                loop {
                    let b = peek_byte(data, *offset)?;
                    if b == BREAK_BYTE {
                        *offset += 1;
                        break;
                    }
                    let _ = skip_item(data, offset)?;
                }
            } else {
                let (count, _) = decode_argument(data, offset, ai, start)?;
                for _ in 0..count {
                    let _ = skip_item(data, offset)?;
                }
            }
        }
        MAJOR_MAP => {
            if ai == AI_INDEFINITE {
                loop {
                    let b = peek_byte(data, *offset)?;
                    if b == BREAK_BYTE {
                        *offset += 1;
                        break;
                    }
                    let _ = skip_item(data, offset)?;
                    let _ = skip_item(data, offset)?;
                }
            } else {
                let (count, _) = decode_argument(data, offset, ai, start)?;
                for _ in 0..count {
                    let _ = skip_item(data, offset)?;
                    let _ = skip_item(data, offset)?;
                }
            }
        }
        MAJOR_TAG => {
            let _ = decode_argument(data, offset, ai, start)?;
            let _ = skip_item(data, offset)?;
        }
        // Major type 7: simple values and floats
        _ => {
            match ai {
                0..=23 => { /* simple value, no additional bytes */ }
                AI_ONE_BYTE => {
                    let _ = read_byte(data, offset)?;
                }
                AI_TWO_BYTES => {
                    if *offset + 2 > data.len() {
                        return Err(CodecError::UnexpectedEof {
                            offset: *offset,
                            needed: 2,
                        });
                    }
                    *offset += 2;
                }
                AI_FOUR_BYTES => {
                    if *offset + 4 > data.len() {
                        return Err(CodecError::UnexpectedEof {
                            offset: *offset,
                            needed: 4,
                        });
                    }
                    *offset += 4;
                }
                AI_EIGHT_BYTES => {
                    if *offset + 8 > data.len() {
                        return Err(CodecError::UnexpectedEof {
                            offset: *offset,
                            needed: 8,
                        });
                    }
                    *offset += 8;
                }
                AI_INDEFINITE => {
                    // break code — should only appear inside indefinite containers
                    // which handle it themselves. Seeing it here means malformed input.
                    return Err(CodecError::InvalidCborStructure {
                        offset: start,
                        detail: "unexpected break code outside indefinite container",
                    });
                }
                _ => {
                    return Err(CodecError::InvalidCborStructure {
                        offset: start,
                        detail: "reserved additional info in simple/float",
                    });
                }
            }
        }
    }

    Ok((start, *offset))
}

// ---------------------------------------------------------------------------
// Writing primitives
// ---------------------------------------------------------------------------

/// Compute the canonical (minimal) encoding width for a value.
pub fn canonical_width(value: u64) -> IntWidth {
    if value <= 23 {
        IntWidth::Inline
    } else if value <= 0xff {
        IntWidth::I8
    } else if value <= 0xffff {
        IntWidth::I16
    } else if value <= 0xffff_ffff {
        IntWidth::I32
    } else {
        IntWidth::I64
    }
}

/// Write a CBOR initial byte + argument with the given major type and width.
pub fn write_argument(buf: &mut Vec<u8>, major: u8, value: u64, width: IntWidth) {
    let m = major << 5;
    match width {
        IntWidth::Inline => buf.push(m | (value as u8)),
        IntWidth::I8 => {
            buf.push(m | AI_ONE_BYTE);
            buf.push(value as u8);
        }
        IntWidth::I16 => {
            buf.push(m | AI_TWO_BYTES);
            buf.extend_from_slice(&(value as u16).to_be_bytes());
        }
        IntWidth::I32 => {
            buf.push(m | AI_FOUR_BYTES);
            buf.extend_from_slice(&(value as u32).to_be_bytes());
        }
        IntWidth::I64 => {
            buf.push(m | AI_EIGHT_BYTES);
            buf.extend_from_slice(&value.to_be_bytes());
        }
    }
}

/// Write a CBOR unsigned integer with canonical (minimal) width.
pub fn write_uint_canonical(buf: &mut Vec<u8>, value: u64) {
    write_argument(buf, MAJOR_UNSIGNED, value, canonical_width(value));
}

/// Write a CBOR unsigned integer with a specific width.
pub fn write_uint(buf: &mut Vec<u8>, value: u64, width: IntWidth) {
    write_argument(buf, MAJOR_UNSIGNED, value, width);
}

/// Write a CBOR byte string with canonical length encoding.
pub fn write_bytes_canonical(buf: &mut Vec<u8>, bytes: &[u8]) {
    let len = bytes.len() as u64;
    write_argument(buf, MAJOR_BYTES, len, canonical_width(len));
    buf.extend_from_slice(bytes);
}

/// Write a CBOR byte string with a specific length encoding width.
pub fn write_bytes(buf: &mut Vec<u8>, bytes: &[u8], width: IntWidth) {
    write_argument(buf, MAJOR_BYTES, bytes.len() as u64, width);
    buf.extend_from_slice(bytes);
}

/// Write a CBOR array header.
pub fn write_array_header(buf: &mut Vec<u8>, encoding: ContainerEncoding) {
    match encoding {
        ContainerEncoding::Definite(count, width) => {
            write_argument(buf, MAJOR_ARRAY, count, width);
        }
        ContainerEncoding::Indefinite => {
            buf.push((MAJOR_ARRAY << 5) | AI_INDEFINITE);
        }
    }
}

/// Write a CBOR map header.
pub fn write_map_header(buf: &mut Vec<u8>, encoding: ContainerEncoding) {
    match encoding {
        ContainerEncoding::Definite(count, width) => {
            write_argument(buf, MAJOR_MAP, count, width);
        }
        ContainerEncoding::Indefinite => {
            buf.push((MAJOR_MAP << 5) | AI_INDEFINITE);
        }
    }
}

/// Write a CBOR tag with specific encoding width.
pub fn write_tag(buf: &mut Vec<u8>, value: u64, width: IntWidth) {
    write_argument(buf, MAJOR_TAG, value, width);
}

/// Write the CBOR break code (0xff) for ending indefinite containers.
pub fn write_break(buf: &mut Vec<u8>) {
    buf.push(BREAK_BYTE);
}

/// Check if we're at a break byte (end of indefinite container).
pub fn is_break(data: &[u8], offset: usize) -> Result<bool, CodecError> {
    let b = peek_byte(data, offset)?;
    Ok(b == BREAK_BYTE)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn read_uint_inline_values() {
        for v in 0..=23u8 {
            let data = [v];
            let mut offset = 0;
            let (val, width) = read_uint(&data, &mut offset).unwrap();
            assert_eq!(val, u64::from(v));
            assert_eq!(width, IntWidth::Inline);
            assert_eq!(offset, 1);
        }
    }

    #[test]
    fn read_uint_one_byte() {
        let data = [0x18, 0x64]; // value 100
        let mut offset = 0;
        let (val, width) = read_uint(&data, &mut offset).unwrap();
        assert_eq!(val, 100);
        assert_eq!(width, IntWidth::I8);
        assert_eq!(offset, 2);
    }

    #[test]
    fn read_uint_two_bytes() {
        let data = [0x19, 0x01, 0x00]; // value 256
        let mut offset = 0;
        let (val, width) = read_uint(&data, &mut offset).unwrap();
        assert_eq!(val, 256);
        assert_eq!(width, IntWidth::I16);
    }

    #[test]
    fn read_uint_four_bytes() {
        let data = [0x1a, 0x00, 0x01, 0x00, 0x00]; // value 65536
        let mut offset = 0;
        let (val, width) = read_uint(&data, &mut offset).unwrap();
        assert_eq!(val, 65536);
        assert_eq!(width, IntWidth::I32);
    }

    #[test]
    fn read_uint_eight_bytes() {
        let data = [0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
        let mut offset = 0;
        let (val, width) = read_uint(&data, &mut offset).unwrap();
        assert_eq!(val, 0x1_0000_0000);
        assert_eq!(width, IntWidth::I64);
    }

    #[test]
    fn read_uint_wrong_type_returns_error() {
        let data = [0x41, 0x00]; // byte string of length 1
        let mut offset = 0;
        let result = read_uint(&data, &mut offset);
        assert_eq!(
            result,
            Err(CodecError::UnexpectedCborType {
                offset: 0,
                expected: "unsigned integer",
                actual: MAJOR_BYTES,
            })
        );
    }

    #[test]
    fn read_bytes_simple() {
        let data = [0x44, 0xde, 0xad, 0xbe, 0xef]; // h'deadbeef'
        let mut offset = 0;
        let (bytes, width) = read_bytes(&data, &mut offset).unwrap();
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(width, IntWidth::Inline);
        assert_eq!(offset, 5);
    }

    #[test]
    fn read_array_header_definite() {
        let data = [0x82]; // array(2)
        let mut offset = 0;
        let enc = read_array_header(&data, &mut offset).unwrap();
        assert_eq!(enc, ContainerEncoding::Definite(2, IntWidth::Inline));
    }

    #[test]
    fn read_array_header_indefinite() {
        let data = [0x9f]; // array(*)
        let mut offset = 0;
        let enc = read_array_header(&data, &mut offset).unwrap();
        assert_eq!(enc, ContainerEncoding::Indefinite);
    }

    #[test]
    fn read_map_header_definite() {
        let data = [0xa3]; // map(3)
        let mut offset = 0;
        let enc = read_map_header(&data, &mut offset).unwrap();
        assert_eq!(enc, ContainerEncoding::Definite(3, IntWidth::Inline));
    }

    #[test]
    fn read_tag_value() {
        let data = [0xd8, 0x18]; // tag(24)
        let mut offset = 0;
        let (val, width) = read_tag(&data, &mut offset).unwrap();
        assert_eq!(val, 24);
        assert_eq!(width, IntWidth::I8);
    }

    #[test]
    fn skip_item_unsigned() {
        let data = [0x18, 0xff, 0x00]; // uint(255), then extra byte
        let mut offset = 0;
        let (start, end) = skip_item(&data, &mut offset).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 2);
    }

    #[test]
    fn skip_item_array() {
        // array(2) [ uint(1), uint(2) ]
        let data = [0x82, 0x01, 0x02];
        let mut offset = 0;
        let (start, end) = skip_item(&data, &mut offset).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 3);
    }

    #[test]
    fn skip_item_nested() {
        // array(2) [ array(1) [ uint(0) ], uint(3) ]
        let data = [0x82, 0x81, 0x00, 0x03];
        let mut offset = 0;
        let (_, end) = skip_item(&data, &mut offset).unwrap();
        assert_eq!(end, 4);
    }

    #[test]
    fn skip_item_map() {
        // map(1) { uint(1): uint(2) }
        let data = [0xa1, 0x01, 0x02];
        let mut offset = 0;
        let (_, end) = skip_item(&data, &mut offset).unwrap();
        assert_eq!(end, 3);
    }

    #[test]
    fn skip_item_tagged() {
        // tag(24) byte_string(1) [0x00]
        let data = [0xd8, 0x18, 0x41, 0x00];
        let mut offset = 0;
        let (_, end) = skip_item(&data, &mut offset).unwrap();
        assert_eq!(end, 4);
    }

    #[test]
    fn write_read_uint_round_trip() {
        for (value, width) in [
            (0u64, IntWidth::Inline),
            (23, IntWidth::Inline),
            (24, IntWidth::I8),
            (255, IntWidth::I8),
            (256, IntWidth::I16),
            (65535, IntWidth::I16),
            (65536, IntWidth::I32),
            (0xffff_ffff, IntWidth::I32),
            (0x1_0000_0000, IntWidth::I64),
        ] {
            let mut buf = Vec::new();
            write_uint(&mut buf, value, width);
            let mut offset = 0;
            let (read_val, read_width) = read_uint(&buf, &mut offset).unwrap();
            assert_eq!(read_val, value, "value mismatch for {value}");
            assert_eq!(read_width, width, "width mismatch for {value}");
            assert_eq!(offset, buf.len(), "offset mismatch for {value}");
        }
    }

    #[test]
    fn write_read_non_canonical_widths() {
        // Write value 5 with I8 width (non-canonical, canonical would be Inline)
        let mut buf = Vec::new();
        write_uint(&mut buf, 5, IntWidth::I8);
        assert_eq!(buf, [0x18, 0x05]);

        let mut offset = 0;
        let (val, width) = read_uint(&buf, &mut offset).unwrap();
        assert_eq!(val, 5);
        assert_eq!(width, IntWidth::I8);
    }

    #[test]
    fn canonical_width_boundaries() {
        assert_eq!(canonical_width(0), IntWidth::Inline);
        assert_eq!(canonical_width(23), IntWidth::Inline);
        assert_eq!(canonical_width(24), IntWidth::I8);
        assert_eq!(canonical_width(255), IntWidth::I8);
        assert_eq!(canonical_width(256), IntWidth::I16);
        assert_eq!(canonical_width(65535), IntWidth::I16);
        assert_eq!(canonical_width(65536), IntWidth::I32);
        assert_eq!(canonical_width(0xffff_ffff), IntWidth::I32);
        assert_eq!(canonical_width(0x1_0000_0000), IntWidth::I64);
    }

    #[test]
    fn empty_input_returns_eof() {
        let data: &[u8] = &[];
        let mut offset = 0;
        let result = read_uint(data, &mut offset);
        assert_eq!(
            result,
            Err(CodecError::UnexpectedEof {
                offset: 0,
                needed: 1,
            })
        );
    }

    #[test]
    fn truncated_two_byte_uint_returns_eof() {
        let data = [0x19, 0x01]; // 2-byte uint, but only 1 byte of payload
        let mut offset = 0;
        let result = read_uint(&data, &mut offset);
        assert_eq!(
            result,
            Err(CodecError::UnexpectedEof {
                offset: 1,
                needed: 2,
            })
        );
    }

    #[test]
    fn write_bytes_canonical_round_trip() {
        let original = vec![0xca, 0xfe, 0xba, 0xbe];
        let mut buf = Vec::new();
        write_bytes_canonical(&mut buf, &original);
        let mut offset = 0;
        let (decoded, _) = read_bytes(&buf, &mut offset).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn peek_major_does_not_advance() {
        let data = [0x82, 0x01, 0x02]; // array(2)
        let major = peek_major(&data, 0).unwrap();
        assert_eq!(major, MAJOR_ARRAY);
        // offset not advanced, can read again
        let major2 = peek_major(&data, 0).unwrap();
        assert_eq!(major2, MAJOR_ARRAY);
    }

    #[test]
    fn is_break_detection() {
        assert!(is_break(&[0xff], 0).unwrap());
        assert!(!is_break(&[0x00], 0).unwrap());
    }
}

// ---------------------------------------------------------------------------
// HFC Bound Encoding
// ---------------------------------------------------------------------------

/// Encode a CBOR null (simple value 22 = 0xf6).
pub fn write_null(buf: &mut Vec<u8>) {
    buf.push(0xf6);
}

/// Encode a positive bignum (tag 2 + byte string) using canonical widths.
///
/// Used for relative_time in HFC bounds (picoseconds from genesis).
/// Strips leading zero bytes to produce minimal encoding.
pub fn write_positive_bignum(buf: &mut Vec<u8>, value: &[u8]) {
    write_tag(buf, 2, canonical_width(2));
    let start = value.iter().position(|&b| b != 0).unwrap_or(value.len().saturating_sub(1));
    write_bytes_canonical(buf, &value[start..]);
}

/// Encode an HFC era Bound: array(3) [relative_time_pico, slot, epoch].
///
/// relative_time is encoded as uint(0) when zero, or positive bignum (tag 2)
/// for non-zero values. This matches the Haskell `NominalDiffTime` encoding.
/// slot and epoch are plain uints with canonical width.
pub fn write_hfc_bound(buf: &mut Vec<u8>, epoch: u64, slot: u64, relative_time_pico: u128) {
    write_array_header(buf, ContainerEncoding::Definite(3, canonical_width(3)));
    if relative_time_pico == 0 {
        write_uint_canonical(buf, 0);
    } else {
        let pico_bytes = relative_time_pico.to_be_bytes();
        write_positive_bignum(buf, &pico_bytes);
    }
    write_uint_canonical(buf, slot);
    write_uint_canonical(buf, epoch);
}

/// Encode an HFC Past entry: array(2) [start_bound, end_bound].
pub fn write_hfc_past(
    buf: &mut Vec<u8>,
    start_epoch: u64, start_slot: u64, start_time_pico: u128,
    end_epoch: u64, end_slot: u64, end_time_pico: u128,
) {
    write_array_header(buf, ContainerEncoding::Definite(2, canonical_width(2)));
    write_hfc_bound(buf, start_epoch, start_slot, start_time_pico);
    write_hfc_bound(buf, end_epoch, end_slot, end_time_pico);
}
