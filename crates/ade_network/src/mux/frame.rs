// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Ouroboros mux frame — BLUE. Pure encode/decode over fixed 8-byte
// header + opaque payload. No I/O, no async, no time, no allocator
// dependence beyond Vec<u8> reservation.
//
// Header layout (8 bytes, big-endian):
//   bytes 0..4  : u32 timestamp (microseconds mod 2^32; caller-supplied)
//   bytes 4..6  : u16 = (mode_bit << 15) | (mini_protocol_id & 0x7FFF)
//                       mode_bit: 0 = Initiator, 1 = Responder
//   bytes 6..8  : u16 payload length (max 65535)
//
// The 1-bit mode partition forces mini_protocol_id into 15 bits.
// `MiniProtocolId::new` rejects values with the high bit set so the
// encode path cannot conflate mode and id.

/// Direction-of-control marker for an Ouroboros mux frame.
///
/// Encoded as the high bit of the second u16 in the header.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MuxMode {
    Initiator = 0,
    Responder = 1,
}

/// Ouroboros mini-protocol identifier (15-bit space).
///
/// The high bit (0x8000) is reserved for the mode flag in the wire
/// header; `new` rejects any value with that bit set.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct MiniProtocolId(u16);

impl MiniProtocolId {
    pub const MAX: u16 = 0x7FFF;

    pub const fn new(id: u16) -> Result<Self, MuxError> {
        if id > Self::MAX {
            Err(MuxError::InvalidMiniProtocolId { id })
        } else {
            Ok(Self(id))
        }
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MuxHeader {
    pub timestamp: u32,
    pub mode: MuxMode,
    pub mini_protocol_id: MiniProtocolId,
    pub length: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MuxFrame {
    pub header: MuxHeader,
    pub payload: Vec<u8>,
}

/// Structured frame-level failures. No String errors — every variant
/// is a closed enum so the session layer can branch deterministically.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MuxError {
    /// Input slice does not carry the full frame the header advertises.
    Truncated { needed: usize, got: usize },
    /// Payload exceeds the 16-bit length field (encode-time check).
    PayloadTooLarge { len: usize },
    /// Mini-protocol id has the mode bit set (15-bit overflow).
    InvalidMiniProtocolId { id: u16 },
}

pub const HEADER_LEN: usize = 8;
pub const MAX_PAYLOAD: usize = u16::MAX as usize;

const MODE_BIT: u16 = 0x8000;
const ID_MASK: u16 = 0x7FFF;

pub fn encode_frame(frame: &MuxFrame) -> Result<Vec<u8>, MuxError> {
    if frame.payload.len() > MAX_PAYLOAD {
        return Err(MuxError::PayloadTooLarge { len: frame.payload.len() });
    }
    let length = frame.header.length;
    if length as usize != frame.payload.len() {
        return Err(MuxError::Truncated {
            needed: length as usize,
            got: frame.payload.len(),
        });
    }
    let mode_bit: u16 = match frame.header.mode {
        MuxMode::Initiator => 0,
        MuxMode::Responder => MODE_BIT,
    };
    let id_word: u16 = mode_bit | (frame.header.mini_protocol_id.get() & ID_MASK);

    let mut out = Vec::with_capacity(HEADER_LEN + frame.payload.len());
    out.extend_from_slice(&frame.header.timestamp.to_be_bytes());
    out.extend_from_slice(&id_word.to_be_bytes());
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&frame.payload);
    Ok(out)
}

pub fn decode_frame(bytes: &[u8]) -> Result<(MuxFrame, &[u8]), MuxError> {
    if bytes.len() < HEADER_LEN {
        return Err(MuxError::Truncated {
            needed: HEADER_LEN,
            got: bytes.len(),
        });
    }
    let mut ts_buf = [0u8; 4];
    ts_buf.copy_from_slice(&bytes[0..4]);
    let timestamp = u32::from_be_bytes(ts_buf);

    let mut id_buf = [0u8; 2];
    id_buf.copy_from_slice(&bytes[4..6]);
    let id_word = u16::from_be_bytes(id_buf);
    let mode = if id_word & MODE_BIT != 0 {
        MuxMode::Responder
    } else {
        MuxMode::Initiator
    };
    let mini_protocol_id = MiniProtocolId(id_word & ID_MASK);

    let mut len_buf = [0u8; 2];
    len_buf.copy_from_slice(&bytes[6..8]);
    let length = u16::from_be_bytes(len_buf);

    let payload_start = HEADER_LEN;
    let payload_end = payload_start
        .checked_add(length as usize)
        .ok_or(MuxError::PayloadTooLarge { len: length as usize })?;
    if bytes.len() < payload_end {
        return Err(MuxError::Truncated {
            needed: payload_end,
            got: bytes.len(),
        });
    }

    let payload = bytes[payload_start..payload_end].to_vec();
    let remaining = &bytes[payload_end..];
    let frame = MuxFrame {
        header: MuxHeader {
            timestamp,
            mode,
            mini_protocol_id,
            length,
        },
        payload,
    };
    Ok((frame, remaining))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn make_frame(timestamp: u32, mode: MuxMode, id: u16, payload: Vec<u8>) -> MuxFrame {
        let length = payload.len() as u16;
        MuxFrame {
            header: MuxHeader {
                timestamp,
                mode,
                mini_protocol_id: MiniProtocolId::new(id).expect("id <= 0x7FFF"),
                length,
            },
            payload,
        }
    }

    #[test]
    fn frame_roundtrip_byte_identical() {
        // Cover every documented N2N + N2C mini-protocol id we know.
        // N2N: handshake=0, chain-sync=2, block-fetch=3, tx-submission2=4,
        //      keep-alive=8, peer-sharing=10.
        // N2C: handshake=0, local-chain-sync=5, local-tx-submission=6,
        //      local-state-query=7, local-tx-monitor=9.
        let ids: [u16; 11] = [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0x7FFF];
        let modes = [MuxMode::Initiator, MuxMode::Responder];
        let payload_sizes: [usize; 6] = [0, 1, 255, 256, 1024, 65535];

        for &id in &ids {
            for &mode in &modes {
                for &size in &payload_sizes {
                    let payload: Vec<u8> = (0..size).map(|i| (i as u8) ^ (id as u8)).collect();
                    let timestamp: u32 = 0xDEAD_BEEF ^ (id as u32) ^ (size as u32);
                    let frame = make_frame(timestamp, mode, id, payload);
                    let bytes = encode_frame(&frame).expect("encode");
                    assert_eq!(bytes.len(), HEADER_LEN + frame.payload.len());
                    let (decoded, rest) = decode_frame(&bytes).expect("decode");
                    assert!(rest.is_empty(), "no trailing bytes for single frame");
                    assert_eq!(decoded, frame, "round-trip identity");
                    let re_encoded = encode_frame(&decoded).expect("re-encode");
                    assert_eq!(re_encoded, bytes, "byte-identical re-encode");
                }
            }
        }
    }

    #[test]
    fn frame_decode_rejects_short_input() {
        for n in 0..HEADER_LEN {
            let bytes = vec![0u8; n];
            let err = decode_frame(&bytes).expect_err("must reject < 8 bytes");
            match err {
                MuxError::Truncated { needed, got } => {
                    assert_eq!(needed, HEADER_LEN);
                    assert_eq!(got, n);
                }
                other => panic!("expected Truncated, got {:?}", other),
            }
        }
    }

    #[test]
    fn frame_decode_rejects_oversized_payload() {
        // Header claims length=10 but only 3 payload bytes follow.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(&0u16.to_be_bytes());
        bytes.extend_from_slice(&10u16.to_be_bytes());
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]);

        let err = decode_frame(&bytes).expect_err("must reject when payload short");
        match err {
            MuxError::Truncated { needed, got } => {
                assert_eq!(needed, HEADER_LEN + 10);
                assert_eq!(got, HEADER_LEN + 3);
            }
            other => panic!("expected Truncated, got {:?}", other),
        }
    }

    #[test]
    fn frame_decode_returns_remaining_bytes() {
        let a = make_frame(1, MuxMode::Initiator, 2, vec![0x11, 0x22, 0x33]);
        let b = make_frame(2, MuxMode::Responder, 3, vec![0x44, 0x55]);
        let c = make_frame(3, MuxMode::Initiator, 8, vec![]);

        let mut concatenated = encode_frame(&a).expect("encode a");
        concatenated.extend_from_slice(&encode_frame(&b).expect("encode b"));
        concatenated.extend_from_slice(&encode_frame(&c).expect("encode c"));

        let (got_a, rest1) = decode_frame(&concatenated).expect("decode a");
        assert_eq!(got_a, a);
        let (got_b, rest2) = decode_frame(rest1).expect("decode b");
        assert_eq!(got_b, b);
        let (got_c, rest3) = decode_frame(rest2).expect("decode c");
        assert_eq!(got_c, c);
        assert!(rest3.is_empty(), "exact concatenation drains");
    }
}
