// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// BLUE producer-side chain-sync server-role surface (PHASE4-N-G S1).
//
// Closed type wrapper: only Server-agency-legal variants of
// `ChainSyncMessage` are constructible via `ServerReply`. The inner
// enum field is private; no constructor exists for `RequestNext`,
// `FindIntersect`, or `Done`, so attempting to build one is a compile
// error. The only path from `ServerReply` to wire bytes is via
// `into_message()` followed by the existing
// `crate::codec::chain_sync::encode_chain_sync_message`.
//
// Per CN-PROTO-06: client-originated messages from the server-role
// pump are unrepresentable in the public API; misuse is a compile
// error.

use crate::codec::chain_sync::{ChainSyncMessage, Point, Tip};

/// Closed wrapper for server-agency-legal chain-sync replies.
///
/// The wire `ChainSyncMessage` enum carries both client- and
/// server-originated variants; this type carries only the server
/// subset and is the only value the producer-side orchestrator may
/// encode for the chain-sync mini-protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerReply(ServerVariant);

/// Private inner enum — the closure is at the type level: no public
/// projection exists, and the wrapper's only constructors below cover
/// the server variants.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ServerVariant {
    RollForward { header: Vec<u8>, tip: Tip },
    RollBackward { point: Point, tip: Tip },
    AwaitReply,
    IntersectFound { point: Point, tip: Tip },
    IntersectNotFound { tip: Tip },
}

impl ServerReply {
    /// Server `RollForward { header, tip }`. The header bytes must be
    /// the canonical header projection of an `AcceptedBlock` per
    /// `ade_ledger::block_validity::accepted_block_header_bytes`
    /// (`DC-CONS-18`); this constructor enforces the call-site
    /// discipline at the type-level by accepting `Vec<u8>` only — the
    /// reducer that calls it is responsible for the projection.
    pub fn roll_forward(header: Vec<u8>, tip: Tip) -> Self {
        Self(ServerVariant::RollForward { header, tip })
    }

    /// Server `RollBackward { point, tip }`.
    pub fn roll_backward(point: Point, tip: Tip) -> Self {
        Self(ServerVariant::RollBackward { point, tip })
    }

    /// Server `AwaitReply` — moves the per-session state machine into
    /// `MustReply`. The reducer must subsequently emit a `RollForward`
    /// / `RollBackward` per `DC-PROTO-08`.
    pub fn await_reply() -> Self {
        Self(ServerVariant::AwaitReply)
    }

    /// Server `IntersectFound { point, tip }`.
    pub fn intersect_found(point: Point, tip: Tip) -> Self {
        Self(ServerVariant::IntersectFound { point, tip })
    }

    /// Server `IntersectNotFound { tip }`.
    pub fn intersect_not_found(tip: Tip) -> Self {
        Self(ServerVariant::IntersectNotFound { tip })
    }

    /// Project to the wire `ChainSyncMessage` for codec encoding.
    /// The output is guaranteed by construction to be a server-agency
    /// variant only.
    pub fn into_message(self) -> ChainSyncMessage {
        match self.0 {
            ServerVariant::RollForward { header, tip } => {
                ChainSyncMessage::RollForward { header, tip }
            }
            ServerVariant::RollBackward { point, tip } => {
                ChainSyncMessage::RollBackward { point, tip }
            }
            ServerVariant::AwaitReply => ChainSyncMessage::AwaitReply,
            ServerVariant::IntersectFound { point, tip } => {
                ChainSyncMessage::IntersectFound { point, tip }
            }
            ServerVariant::IntersectNotFound { tip } => {
                ChainSyncMessage::IntersectNotFound { tip }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::chain_sync::{decode_chain_sync_message, encode_chain_sync_message};
    use ade_types::{Hash32, SlotNo};

    fn tip_sample() -> Tip {
        Tip {
            point: Point::Block {
                slot: SlotNo(1234),
                hash: Hash32([0xAA; 32]),
            },
            block_no: 5678,
        }
    }

    fn point_sample() -> Point {
        Point::Block {
            slot: SlotNo(99),
            hash: Hash32([0xBB; 32]),
        }
    }

    fn header_sample() -> Vec<u8> {
        // Header is carried as a single opaque CBOR item per the
        // chain-sync codec contract. A `bytes(6)` value is the simplest
        // single-item shape that round-trips byte-identically.
        vec![0x46, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]
    }

    #[test]
    fn chain_sync_server_reply_round_trips_through_codec() {
        // Every server-agency variant we expose must round-trip
        // byte-identically through the existing codec, otherwise the
        // closed wrapper has drifted from the wire grammar.
        let replies = vec![
            ServerReply::roll_forward(header_sample(), tip_sample()),
            ServerReply::roll_backward(point_sample(), tip_sample()),
            ServerReply::await_reply(),
            ServerReply::intersect_found(point_sample(), tip_sample()),
            ServerReply::intersect_not_found(tip_sample()),
        ];
        for r in replies {
            let msg = r.clone().into_message();
            let bytes = encode_chain_sync_message(&msg);
            let decoded = decode_chain_sync_message(&bytes)
                .expect("server reply round-trips through codec");
            assert_eq!(msg, decoded, "round-trip equality on {msg:?}");
        }
    }

    #[test]
    fn chain_sync_server_reply_into_message_only_yields_server_variants() {
        // Exhaustive match: every reply we can construct projects to
        // exactly one of the five server-agency variants. The match
        // arms below ARE the closure proof — adding a client variant
        // here would not compile because no constructor exists.
        let replies = vec![
            ServerReply::roll_forward(header_sample(), tip_sample()),
            ServerReply::roll_backward(point_sample(), tip_sample()),
            ServerReply::await_reply(),
            ServerReply::intersect_found(point_sample(), tip_sample()),
            ServerReply::intersect_not_found(tip_sample()),
        ];
        for r in replies {
            match r.into_message() {
                ChainSyncMessage::RollForward { .. } => {}
                ChainSyncMessage::RollBackward { .. } => {}
                ChainSyncMessage::AwaitReply => {}
                ChainSyncMessage::IntersectFound { .. } => {}
                ChainSyncMessage::IntersectNotFound { .. } => {}
                ChainSyncMessage::RequestNext
                | ChainSyncMessage::FindIntersect { .. }
                | ChainSyncMessage::Done => {
                    panic!("ServerReply must not project to a client-agency variant")
                }
            }
        }
    }
}
