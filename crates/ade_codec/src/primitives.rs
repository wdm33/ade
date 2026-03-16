// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use crate::cbor;
use crate::error::CodecError;
use crate::traits::{AdeDecode, AdeEncode, CodecContext};

// ---------------------------------------------------------------------------
// AdeEncode/AdeDecode for ade_types primitive newtypes
// ---------------------------------------------------------------------------

impl AdeEncode for ade_types::SlotNo {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_uint_canonical(buf, self.0);
        Ok(())
    }
}

impl AdeDecode for ade_types::SlotNo {
    fn ade_decode(
        data: &[u8],
        offset: &mut usize,
        _ctx: &CodecContext,
    ) -> Result<Self, CodecError> {
        let (val, _) = cbor::read_uint(data, offset)?;
        Ok(ade_types::SlotNo(val))
    }
}

impl AdeEncode for ade_types::BlockNo {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_uint_canonical(buf, self.0);
        Ok(())
    }
}

impl AdeDecode for ade_types::BlockNo {
    fn ade_decode(
        data: &[u8],
        offset: &mut usize,
        _ctx: &CodecContext,
    ) -> Result<Self, CodecError> {
        let (val, _) = cbor::read_uint(data, offset)?;
        Ok(ade_types::BlockNo(val))
    }
}

impl AdeEncode for ade_types::EpochNo {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_uint_canonical(buf, self.0);
        Ok(())
    }
}

impl AdeDecode for ade_types::EpochNo {
    fn ade_decode(
        data: &[u8],
        offset: &mut usize,
        _ctx: &CodecContext,
    ) -> Result<Self, CodecError> {
        let (val, _) = cbor::read_uint(data, offset)?;
        Ok(ade_types::EpochNo(val))
    }
}

impl AdeEncode for ade_types::Hash28 {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_bytes_canonical(buf, &self.0);
        Ok(())
    }
}

impl AdeDecode for ade_types::Hash28 {
    fn ade_decode(
        data: &[u8],
        offset: &mut usize,
        _ctx: &CodecContext,
    ) -> Result<Self, CodecError> {
        let (bytes, _) = cbor::read_bytes(data, offset)?;
        if bytes.len() != 28 {
            return Err(CodecError::InvalidLength {
                offset: *offset - bytes.len(),
                detail: "Hash28 must be exactly 28 bytes",
            });
        }
        let mut arr = [0u8; 28];
        arr.copy_from_slice(&bytes);
        Ok(ade_types::Hash28(arr))
    }
}

impl AdeEncode for ade_types::Hash32 {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        cbor::write_bytes_canonical(buf, &self.0);
        Ok(())
    }
}

impl AdeDecode for ade_types::Hash32 {
    fn ade_decode(
        data: &[u8],
        offset: &mut usize,
        _ctx: &CodecContext,
    ) -> Result<Self, CodecError> {
        let (bytes, _) = cbor::read_bytes(data, offset)?;
        if bytes.len() != 32 {
            return Err(CodecError::InvalidLength {
                offset: *offset - bytes.len(),
                detail: "Hash32 must be exactly 32 bytes",
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(ade_types::Hash32(arr))
    }
}

// ---------------------------------------------------------------------------
// AdeEncode/AdeDecode for RawCbor — identity codec
// ---------------------------------------------------------------------------

impl AdeEncode for crate::preserved::RawCbor {
    fn ade_encode(&self, buf: &mut Vec<u8>, _ctx: &CodecContext) -> Result<(), CodecError> {
        buf.extend_from_slice(&self.0);
        Ok(())
    }
}

impl AdeDecode for crate::preserved::RawCbor {
    fn ade_decode(
        data: &[u8],
        offset: &mut usize,
        _ctx: &CodecContext,
    ) -> Result<Self, CodecError> {
        let (start, end) = cbor::skip_item(data, offset)?;
        Ok(crate::preserved::RawCbor(data[start..end].to_vec()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ade_types::*;

    fn ctx() -> CodecContext {
        CodecContext {
            era: CardanoEra::ByronEbb,
        }
    }

    #[test]
    fn slot_no_round_trip() {
        for val in [0u64, 1, 23, 24, 255, 256, 65535, 65536, 1_000_000] {
            let slot = SlotNo(val);
            let mut buf = Vec::new();
            slot.ade_encode(&mut buf, &ctx()).unwrap();
            let mut offset = 0;
            let decoded = SlotNo::ade_decode(&buf, &mut offset, &ctx()).unwrap();
            assert_eq!(decoded, slot);
            assert_eq!(offset, buf.len());
        }
    }

    #[test]
    fn block_no_round_trip() {
        let bn = BlockNo(4802149);
        let mut buf = Vec::new();
        bn.ade_encode(&mut buf, &ctx()).unwrap();
        let mut offset = 0;
        let decoded = BlockNo::ade_decode(&buf, &mut offset, &ctx()).unwrap();
        assert_eq!(decoded, bn);
    }

    #[test]
    fn epoch_no_round_trip() {
        let en = EpochNo(618);
        let mut buf = Vec::new();
        en.ade_encode(&mut buf, &ctx()).unwrap();
        let mut offset = 0;
        let decoded = EpochNo::ade_decode(&buf, &mut offset, &ctx()).unwrap();
        assert_eq!(decoded, en);
    }

    #[test]
    fn hash32_round_trip() {
        let h = Hash32([0xab; 32]);
        let mut buf = Vec::new();
        h.ade_encode(&mut buf, &ctx()).unwrap();
        let mut offset = 0;
        let decoded = Hash32::ade_decode(&buf, &mut offset, &ctx()).unwrap();
        assert_eq!(decoded, h);
    }

    #[test]
    fn hash28_round_trip() {
        let h = Hash28([0xcd; 28]);
        let mut buf = Vec::new();
        h.ade_encode(&mut buf, &ctx()).unwrap();
        let mut offset = 0;
        let decoded = Hash28::ade_decode(&buf, &mut offset, &ctx()).unwrap();
        assert_eq!(decoded, h);
    }

    #[test]
    fn hash32_wrong_length_rejected() {
        // Encode a 16-byte string where 32 is expected
        let mut buf = Vec::new();
        cbor::write_bytes_canonical(&mut buf, &[0u8; 16]);
        let mut offset = 0;
        let result = Hash32::ade_decode(&buf, &mut offset, &ctx());
        assert!(matches!(result, Err(CodecError::InvalidLength { .. })));
    }

    #[test]
    fn hash28_wrong_length_rejected() {
        let mut buf = Vec::new();
        cbor::write_bytes_canonical(&mut buf, &[0u8; 32]);
        let mut offset = 0;
        let result = Hash28::ade_decode(&buf, &mut offset, &ctx());
        assert!(matches!(result, Err(CodecError::InvalidLength { .. })));
    }

    #[test]
    fn raw_cbor_identity_round_trip() {
        use crate::preserved::RawCbor;
        // A CBOR array [1, 2, 3]
        let original = vec![0x83, 0x01, 0x02, 0x03];
        let raw = RawCbor(original.clone());

        // Encode — should reproduce exact bytes
        let mut buf = Vec::new();
        raw.ade_encode(&mut buf, &ctx()).unwrap();
        assert_eq!(buf, original);

        // Decode — should capture same span
        let mut offset = 0;
        let decoded = RawCbor::ade_decode(&original, &mut offset, &ctx()).unwrap();
        assert_eq!(decoded.as_bytes(), &original[..]);
        assert_eq!(offset, original.len());
    }
}
