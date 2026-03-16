// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use ade_types::CardanoEra;

use crate::cbor::{self, ContainerEncoding};
use crate::error::CodecError;

/// Decoded HardForkCombinator block envelope.
///
/// Contains the era discriminant and the byte range of the inner
/// era-specific block within the original input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockEnvelope {
    /// The era tag from the outer array.
    pub era: CardanoEra,
    /// Start offset of the era-specific block body in the original data.
    pub block_start: usize,
    /// End offset (exclusive) of the era-specific block body.
    pub block_end: usize,
}

/// Decode the HardForkCombinator block envelope.
///
/// The envelope is a CBOR 2-element array: `[era_tag, era_block]`.
/// This is the top-level decode chokepoint — all external block bytes
/// enter the system through this function.
///
/// Returns a `BlockEnvelope` containing the era and the byte range
/// of the inner block within `data`.
pub fn decode_block_envelope(data: &[u8]) -> Result<BlockEnvelope, CodecError> {
    if data.is_empty() {
        return Err(CodecError::UnexpectedEof {
            offset: 0,
            needed: 1,
        });
    }

    let mut offset = 0;

    // Read outer array header — must be definite length 2
    let encoding = cbor::read_array_header(data, &mut offset)?;
    match encoding {
        ContainerEncoding::Definite(2, _) => {}
        ContainerEncoding::Definite(n, _) => {
            return Err(CodecError::InvalidCborStructure {
                offset: 0,
                detail: if n < 2 {
                    "block envelope array has fewer than 2 elements"
                } else {
                    "block envelope array has more than 2 elements"
                },
            });
        }
        ContainerEncoding::Indefinite => {
            return Err(CodecError::InvalidCborStructure {
                offset: 0,
                detail: "block envelope must be definite-length array",
            });
        }
    }

    // Read era tag — must be unsigned integer
    let (tag_value, _) = cbor::read_uint(data, &mut offset)?;

    // Convert to CardanoEra
    if tag_value > 255 {
        return Err(CodecError::UnknownEraTag { tag: 255 });
    }
    let era = CardanoEra::try_from(tag_value as u8)
        .map_err(|e| CodecError::UnknownEraTag { tag: e.0 })?;

    // Identify the byte span of the era-specific block
    let block_start = offset;
    let (_, block_end) = cbor::skip_item(data, &mut offset)?;

    // Verify no trailing bytes
    if offset != data.len() {
        return Err(CodecError::TrailingBytes {
            consumed: offset,
            total: data.len(),
        });
    }

    Ok(BlockEnvelope {
        era,
        block_start,
        block_end,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn decode_simple_envelope() {
        // [0, [1, 2, 3]]  =>  era=ByronEbb, body=[1,2,3]
        // 82 00 83 01 02 03
        let data = [0x82, 0x00, 0x83, 0x01, 0x02, 0x03];
        let env = decode_block_envelope(&data).unwrap();
        assert_eq!(env.era, CardanoEra::ByronEbb);
        assert_eq!(env.block_start, 2);
        assert_eq!(env.block_end, 6);
        assert_eq!(
            &data[env.block_start..env.block_end],
            &[0x83, 0x01, 0x02, 0x03]
        );
    }

    #[test]
    fn decode_shelley_era_tag() {
        // [2, h'cafe']  =>  era=Shelley
        // 82 02 42 ca fe
        let data = [0x82, 0x02, 0x42, 0xca, 0xfe];
        let env = decode_block_envelope(&data).unwrap();
        assert_eq!(env.era, CardanoEra::Shelley);
        assert_eq!(&data[env.block_start..env.block_end], &[0x42, 0xca, 0xfe]);
    }

    #[test]
    fn decode_conway_era_tag() {
        // [7, 42]
        // 82 07 18 2a
        let data = [0x82, 0x07, 0x18, 0x2a];
        let env = decode_block_envelope(&data).unwrap();
        assert_eq!(env.era, CardanoEra::Conway);
    }

    #[test]
    fn reject_unknown_era_tag_8() {
        // [8, 0]
        // 82 08 00
        let data = [0x82, 0x08, 0x00];
        let result = decode_block_envelope(&data);
        assert_eq!(result, Err(CodecError::UnknownEraTag { tag: 8 }));
    }

    #[test]
    fn reject_unknown_era_tag_255() {
        // [255, 0]
        // 82 18 ff 00
        let data = [0x82, 0x18, 0xff, 0x00];
        let result = decode_block_envelope(&data);
        assert_eq!(result, Err(CodecError::UnknownEraTag { tag: 255 }));
    }

    #[test]
    fn reject_empty_input() {
        let result = decode_block_envelope(&[]);
        assert_eq!(
            result,
            Err(CodecError::UnexpectedEof {
                offset: 0,
                needed: 1,
            })
        );
    }

    #[test]
    fn reject_truncated_input() {
        // Just the array header, no elements
        let data = [0x82];
        let result = decode_block_envelope(&data);
        assert!(result.is_err());
    }

    #[test]
    fn reject_trailing_bytes() {
        // [0, 0] followed by extra byte
        // 82 00 00 ff
        let data = [0x82, 0x00, 0x00, 0xff];
        let result = decode_block_envelope(&data);
        assert_eq!(
            result,
            Err(CodecError::TrailingBytes {
                consumed: 3,
                total: 4,
            })
        );
    }

    #[test]
    fn reject_wrong_array_length() {
        // [0] — only 1 element
        // 81 00
        let data = [0x81, 0x00];
        let result = decode_block_envelope(&data);
        assert!(matches!(
            result,
            Err(CodecError::InvalidCborStructure { .. })
        ));
    }

    #[test]
    fn reject_non_array_envelope() {
        // map(1) instead of array
        // a1 00 00
        let data = [0xa1, 0x00, 0x00];
        let result = decode_block_envelope(&data);
        assert!(matches!(
            result,
            Err(CodecError::UnexpectedCborType {
                expected: "array",
                ..
            })
        ));
    }

    #[test]
    fn reject_non_integer_era_tag() {
        // [h'00', 0]
        // 82 41 00 00
        let data = [0x82, 0x41, 0x00, 0x00];
        let result = decode_block_envelope(&data);
        assert!(matches!(
            result,
            Err(CodecError::UnexpectedCborType {
                expected: "unsigned integer",
                ..
            })
        ));
    }

    #[test]
    fn all_valid_era_tags() {
        for tag in 0..=7u8 {
            // [tag, 0]
            let data = [0x82, tag, 0x00];
            let env = decode_block_envelope(&data).unwrap();
            assert_eq!(env.era.as_u8(), tag);
        }
    }
}
