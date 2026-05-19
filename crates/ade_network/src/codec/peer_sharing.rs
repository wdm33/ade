// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// N2N PeerSharing mini-protocol message codec (BLUE).
//
// Wire shape:
//   peerSharingMessage =
//       [0, amount(u8)]            ; MsgShareRequest
//     / [1, [peerAddress*]]        ; MsgSharePeers
//     / [2]                        ; MsgDone
//
//   peerAddress =
//       [0, ipv4(u32), port(u16)]
//     / [1, ipv6(bytes16), port(u16), flowinfo(u32), scope(u32)]

use crate::codec::error::{CodecError, ProtocolKind};
use crate::codec::primitives::{
    decode_array_header, decode_bytes, decode_u16, decode_u32, decode_u64, encode_array_header,
    encode_bytes, encode_u64, require_consumed,
};

const PROTOCOL: ProtocolKind = ProtocolKind::PeerSharing;

/// Closed peer address taxonomy (IPv4 / IPv6 with port).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerAddress {
    V4 { addr: u32, port: u16 },
    V6 { addr: [u8; 16], port: u16, flowinfo: u32, scope: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerSharingMessage {
    ShareRequest { amount: u8 },
    SharePeers { peers: Vec<PeerAddress> },
    Done,
}

fn encode_peer_address(buf: &mut Vec<u8>, p: &PeerAddress) {
    match p {
        PeerAddress::V4 { addr, port } => {
            encode_array_header(buf, 3);
            encode_u64(buf, 0);
            encode_u64(buf, *addr as u64);
            encode_u64(buf, *port as u64);
        }
        PeerAddress::V6 { addr, port, flowinfo, scope } => {
            encode_array_header(buf, 5);
            encode_u64(buf, 1);
            encode_bytes(buf, addr);
            encode_u64(buf, *port as u64);
            encode_u64(buf, *flowinfo as u64);
            encode_u64(buf, *scope as u64);
        }
    }
}

fn decode_peer_address(data: &[u8], offset: &mut usize) -> Result<PeerAddress, CodecError> {
    let arr_len = decode_array_header(PROTOCOL, data, offset)?;
    if arr_len < 1 {
        return Err(CodecError::InvalidProtocolMessage {
            protocol: PROTOCOL,
            reason: "empty peer address array",
        });
    }
    let tag = decode_u64(PROTOCOL, data, offset)?;
    match (tag, arr_len) {
        (0, 3) => {
            let addr = decode_u32(PROTOCOL, data, offset, "ipv4 address")?;
            let port = decode_u16(PROTOCOL, data, offset, "port")?;
            Ok(PeerAddress::V4 { addr, port })
        }
        (1, 5) => {
            let raw = decode_bytes(PROTOCOL, data, offset)?;
            if raw.len() != 16 {
                return Err(CodecError::InvalidProtocolMessage {
                    protocol: PROTOCOL,
                    reason: "ipv6 address not 16 bytes",
                });
            }
            let mut a = [0u8; 16];
            a.copy_from_slice(&raw);
            let port = decode_u16(PROTOCOL, data, offset, "port")?;
            let flowinfo = decode_u32(PROTOCOL, data, offset, "flowinfo")?;
            let scope = decode_u32(PROTOCOL, data, offset, "scope")?;
            Ok(PeerAddress::V6 { addr: a, port, flowinfo, scope })
        }
        (other, _) => Err(CodecError::UnknownTag { protocol: PROTOCOL, tag: other }),
    }
}

pub fn encode_peer_sharing_message(msg: &PeerSharingMessage) -> Vec<u8> {
    let mut buf = Vec::new();
    match msg {
        PeerSharingMessage::ShareRequest { amount } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 0);
            encode_u64(&mut buf, *amount as u64);
        }
        PeerSharingMessage::SharePeers { peers } => {
            encode_array_header(&mut buf, 2);
            encode_u64(&mut buf, 1);
            encode_array_header(&mut buf, peers.len() as u64);
            for p in peers {
                encode_peer_address(&mut buf, p);
            }
        }
        PeerSharingMessage::Done => {
            encode_array_header(&mut buf, 1);
            encode_u64(&mut buf, 2);
        }
    }
    buf
}

pub fn decode_peer_sharing_message(bytes: &[u8]) -> Result<PeerSharingMessage, CodecError> {
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
            let amount_v = decode_u64(PROTOCOL, bytes, &mut offset)?;
            if amount_v > u8::MAX as u64 {
                return Err(CodecError::InvalidIntegerRange {
                    protocol: PROTOCOL,
                    field: "amount",
                    value: amount_v,
                });
            }
            PeerSharingMessage::ShareRequest { amount: amount_v as u8 }
        }
        (1, 2) => {
            let n = decode_array_header(PROTOCOL, bytes, &mut offset)?;
            let mut peers = Vec::with_capacity(n as usize);
            for _ in 0..n {
                peers.push(decode_peer_address(bytes, &mut offset)?);
            }
            PeerSharingMessage::SharePeers { peers }
        }
        (2, 1) => PeerSharingMessage::Done,
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

    fn sample_messages() -> Vec<PeerSharingMessage> {
        vec![
            PeerSharingMessage::ShareRequest { amount: 0 },
            PeerSharingMessage::ShareRequest { amount: 200 },
            PeerSharingMessage::SharePeers {
                peers: vec![
                    PeerAddress::V4 { addr: 0xC0A80001, port: 3001 },
                    PeerAddress::V6 {
                        addr: [0x20, 0x01, 0x0D, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01],
                        port: 3001,
                        flowinfo: 0,
                        scope: 0,
                    },
                ],
            },
            PeerSharingMessage::SharePeers { peers: vec![] },
            PeerSharingMessage::Done,
        ]
    }

    #[test]
    fn roundtrip_every_variant() {
        for msg in sample_messages() {
            let bytes = encode_peer_sharing_message(&msg);
            let decoded = decode_peer_sharing_message(&bytes).expect("decode");
            assert_eq!(decoded, msg);
            assert_eq!(encode_peer_sharing_message(&decoded), bytes);
        }
    }

    #[test]
    fn decode_rejects_unknown_tag() {
        let bytes = vec![0x81, 0x18, 0x63];
        match decode_peer_sharing_message(&bytes) {
            Err(CodecError::UnknownTag { protocol: ProtocolKind::PeerSharing, tag: 99 }) => {}
            other => panic!("expected UnknownTag, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_truncated_input() {
        let full = encode_peer_sharing_message(&PeerSharingMessage::SharePeers {
            peers: vec![PeerAddress::V4 { addr: 0xC0A80001, port: 3001 }],
        });
        for n in 0..full.len() {
            let slice = &full[..n];
            let err = decode_peer_sharing_message(slice).expect_err("must reject truncated");
            match err {
                CodecError::Truncated { .. }
                | CodecError::MalformedCbor { .. }
                | CodecError::InvalidProtocolMessage { .. } => {}
                other => panic!("expected truncation-class error, got {other:?}"),
            }
        }
    }
}
