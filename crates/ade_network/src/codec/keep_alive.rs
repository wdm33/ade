// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2N KeepAlive mini-protocol message codec (BLUE).
//
// Wire shape:
//   keepAliveMessage =
//       [0, cookie(u16)]   ; MsgKeepAlive
//     / [1, cookie(u16)]   ; MsgResponseKeepAlive
//     / [2]                ; MsgDone

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_u16, decode_u64, encode_array_header, encode_u64, require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::KeepAlive;

/// 16-bit nonce echoed by the responder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeepAliveCookie(pub u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeepAliveMessage {
    KeepAlive(KeepAliveCookie),
    ResponseKeepAlive(KeepAliveCookie),
    Done,
}

pub fn encode_keep_alive_message(msg: &KeepAliveMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        KeepAliveMessage::KeepAlive(c) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 0);
            encode_u64(&mut buf, c.0 as u64);
        }
        KeepAliveMessage::ResponseKeepAlive(c) => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 1);
            encode_u64(&mut buf, c.0 as u64);
        }
        KeepAliveMessage::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 2);
        }
    }
    buf
}

pub fn decode_keep_alive_message(bytes: &[u8]) -> Result<KeepAliveMessage, CodecError> {
    if bytes.is_empty() {
        return Err(CodecError::Truncated { needed: 1, got: 0 });
    }
    let mut offset = 0usize;
    let arr_len = decode_array_header(PROTOCOL, bytes, &mut offset)?;
    if arr_len < 1 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "empty outer array",
        });
    }
    let tag = decode_u64(PROTOCOL, bytes, &mut offset)?;
    let msg = match (tag, arr_len) {
        (0, 2) => {
            let c = decode_u16(PROTOCOL, bytes, &mut offset, "cookie")?;
            KeepAliveMessage::KeepAlive(KeepAliveCookie(c))
        }
        (1, 2) => {
            let c = decode_u16(PROTOCOL, bytes, &mut offset, "cookie")?;
            KeepAliveMessage::ResponseKeepAlive(KeepAliveCookie(c))
        }
        (2, 1) => KeepAliveMessage::Done,
        (other, _) => return Err(CodecError::UnknownTag { protocol: PROTOCOL, tag: other }),
    };
    require_consumed(PROTOCOL, bytes, offset)?;
    Ok(msg)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample_messages() -> Vec<KeepAliveMessage> {
        vec![
            KeepAliveMessage::KeepAlive(KeepAliveCookie(0)),
            KeepAliveMessage::KeepAlive(KeepAliveCookie(65535)),
            KeepAliveMessage::ResponseKeepAlive(KeepAliveCookie(1234)),
            KeepAliveMessage::Done,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_keep_alive_message(&msg);
            let decoded = decode_keep_alive_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_keep_alive_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_keep_alive_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::KeepAlive, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_keep_alive_message(&KeepAliveMessage::KeepAlive(KeepAliveCookie(0xBEEF)));
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_keep_alive_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
