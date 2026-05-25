// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// BLUE producer-side block-fetch server-role surface (PHASE4-N-G S1).
//
// Closed type wrapper: only Server-agency-legal variants of
// `BlockFetchMessage` are constructible via `ServerReply`. The inner
// enum field is private; no constructor exists for `RequestRange` or
// `ClientDone`, so attempting to build one is a compile error. The
// only path from `ServerReply` to wire bytes is via `into_message()`
// followed by `crate::codec::block_fetch::encode_block_fetch_message`.
//
// Per CN-PROTO-06: client-originated messages from the server-role
// pump are unrepresentable in the public API; misuse is a compile
// error.

use crate::codec::block_fetch::BlockFetchMessage;

/// Closed wrapper for server-agency-legal block-fetch replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerReply(ServerVariant);

#[derive(Debug, Clone, PartialEq, Eq)]
enum ServerVariant {
    StartBatch,
    NoBlocks,
    Block { bytes: Vec<u8> },
    BatchDone,
}

impl ServerReply {
    /// Server `StartBatch` — opens the streaming sub-protocol.
    pub fn start_batch() -> Self {
        Self(ServerVariant::StartBatch)
    }

    /// Server `NoBlocks` — empties the requested range.
    pub fn no_blocks() -> Self {
        Self(ServerVariant::NoBlocks)
    }

    /// Server `Block { bytes }`. The bytes MUST be sourced from a
    /// `ServedChainSnapshot` slice (S2) — which is itself sourced
    /// from `AcceptedBlock::as_bytes()` (`CN-CONS-07`) — per the
    /// `DC-CONS-17` invariant. The reducer that calls this constructor
    /// is the enforcement point; the wrapper itself only enforces the
    /// agency closure.
    pub fn block(bytes: Vec<u8>) -> Self {
        Self(ServerVariant::Block { bytes })
    }

    /// Server `BatchDone` — closes the streaming sub-protocol.
    pub fn batch_done() -> Self {
        Self(ServerVariant::BatchDone)
    }

    /// Project to the wire `BlockFetchMessage` for codec encoding.
    /// The output is guaranteed by construction to be a server-agency
    /// variant only.
    pub fn into_message(self) -> BlockFetchMessage {
        match self.0 {
            ServerVariant::StartBatch => BlockFetchMessage::StartBatch,
            ServerVariant::NoBlocks => BlockFetchMessage::NoBlocks,
            ServerVariant::Block { bytes } => BlockFetchMessage::Block { bytes },
            ServerVariant::BatchDone => BlockFetchMessage::BatchDone,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::block_fetch::{decode_block_fetch_message, encode_block_fetch_message};

    fn block_bytes_sample() -> Vec<u8> {
        // Block body is carried as a single opaque CBOR item per the
        // block-fetch codec contract. A `bytes(6)` value is the simplest
        // single-item shape that round-trips byte-identically.
        vec![0x46, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]
    }

    #[test]
    fn block_fetch_server_reply_round_trips_through_codec() {
        let replies = vec![
            ServerReply::start_batch(),
            ServerReply::no_blocks(),
            ServerReply::block(block_bytes_sample()),
            ServerReply::batch_done(),
        ];
        for r in replies {
            let msg = r.clone().into_message();
            let bytes = encode_block_fetch_message(&msg);
            let decoded = decode_block_fetch_message(&bytes)
                .expect("server reply round-trips through codec");
            assert_eq!(msg, decoded, "round-trip equality on {msg:?}");
        }
    }

    #[test]
    fn block_fetch_server_reply_into_message_only_yields_server_variants() {
        let replies = vec![
            ServerReply::start_batch(),
            ServerReply::no_blocks(),
            ServerReply::block(block_bytes_sample()),
            ServerReply::batch_done(),
        ];
        for r in replies {
            match r.into_message() {
                BlockFetchMessage::StartBatch => {}
                BlockFetchMessage::NoBlocks => {}
                BlockFetchMessage::Block { .. } => {}
                BlockFetchMessage::BatchDone => {}
                BlockFetchMessage::RequestRange(_) | BlockFetchMessage::ClientDone => {
                    panic!("ServerReply must not project to a client-agency variant")
                }
            }
        }
    }
}
