// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-F-G-M S2 (CN-WIRE-11): Ade's serve-side ChainSync server must (A) DECODE a real cardano-node
//! follower's `MsgFindIntersect` -- whose points list is a CBOR indefinite-length array -- and (B) REPLY
//! `IntersectFound[Origin]`. Pinned to captured real-cardano-node fixtures (NOT an Ade<->Ade round-trip,
//! which passes against Ade's own definite-length encoding and so cannot catch this).
//!
//! Live failure (instrumented C1 rerun, reproduced across 9 follower reconnects): the follower sent
//! `82 04 9f 80 80 ff` and Ade's decoder rejected it ("indefinite-length array not allowed") -> dispatch
//! dropped the frame -> no reply -> the follower timed out at SingIntersect.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use ade_network::chain_sync::server::{
    producer_chain_sync_serve, HeaderProjection, ProducerChainSyncServerState, ServedHeaderLookup,
    ServerStep,
};
use ade_network::codec::chain_sync::{
    decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage, Point,
};
use ade_network::codec::version::ChainSyncVersion;
use ade_types::{Hash32, SlotNo};

fn corpus_chain_sync_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("corpus/network/n2n/chain_sync")
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(corpus_chain_sync_dir().join(name))
        .unwrap_or_else(|e| panic!("chain_sync fixture {name} present: {e:?}"))
}

/// A minimal served chain whose tip is a self-accepted block 0 and which has NO matching block point (so
/// only `Origin` can intersect). Mirrors the genesis-rehearsal scenario without pulling in ade_ledger.
struct GenesisTipLookup;
impl ServedHeaderLookup for GenesisTipLookup {
    fn next_after(&self, _cursor: Option<(SlotNo, Hash32)>) -> Option<HeaderProjection> {
        None
    }
    fn intersect(&self, _points: &[Point]) -> Option<(SlotNo, Hash32)> {
        None // no block points on this served chain; only Origin intersects
    }
    fn tip(&self) -> Option<(SlotNo, Hash32, u64)> {
        Some((SlotNo(0), Hash32([0x11; 32]), 0)) // Ade's own served block-0 tip
    }
}

/// CE-G-M-1 (A): the real-node `MsgFindIntersect` (indefinite-length points list) decodes.
#[test]
fn real_cardano_node_findintersect_indefinite_points_list_decodes() {
    let bytes = fixture("c1privnet_follower_findintersect_recv.cbor");
    assert_eq!(
        bytes,
        vec![0x82, 0x04, 0x9f, 0x80, 0x80, 0xff],
        "fixture is the captured real-node FindIntersect request (de-muxed payload)"
    );
    match decode_chain_sync_message(&bytes)
        .expect("real cardano-node FindIntersect must decode (CN-WIRE-11)")
    {
        ChainSyncMessage::FindIntersect { points } => {
            assert_eq!(
                points,
                vec![Point::Origin, Point::Origin],
                "indefinite points list = [Origin, Origin]"
            );
        }
        other => panic!("expected FindIntersect, got {other:?}"),
    }
}

/// CE-G-M-1 (B): fed the decoded real-node request, Ade's serve reducer replies `IntersectFound[Origin]`,
/// matching the captured c1 `IntersectFound` grammar (tag 5 + Origin point + 2-element tip). The tip itself
/// is node-specific (Ade serves block 0), so the pin is the GRAMMAR, not the dynamic tip's byte-identity.
#[test]
fn real_cardano_node_findintersect_yields_intersect_found_origin() {
    let req = fixture("c1privnet_follower_findintersect_recv.cbor");
    let msg = decode_chain_sync_message(&req).expect("decodes");
    let state = ProducerChainSyncServerState::new();
    let (_s2, step) =
        producer_chain_sync_serve(state, msg, &GenesisTipLookup, ChainSyncVersion::new(15)).unwrap();
    let reply = match step {
        ServerStep::Reply(r) => r.into_message(),
        other => panic!("expected Reply, got {other:?}"),
    };
    match &reply {
        ChainSyncMessage::IntersectFound { point, tip } => {
            assert_eq!(*point, Point::Origin, "Origin is the universal common ancestor");
            assert!(
                matches!(tip.point, Point::Block { .. }),
                "Ade serves block 0 -> its own tip point is a Block (node-specific)"
            );
        }
        other => panic!("expected IntersectFound[Origin], got {other:?}"),
    }
    // Reply grammar pin vs the captured c1 IntersectFound fixture: tag 5 + Origin point (0x80). The c1
    // fixture payload is bytes [8..] (it includes an 8-byte mux header); ours is header-less. The tip
    // differs (node-specific), so we pin the [83 05 80 ...] grammar prefix only.
    let reply_bytes = encode_chain_sync_message(&reply);
    assert_eq!(
        &reply_bytes[0..3],
        &[0x83, 0x05, 0x80],
        "Ade reply is IntersectFound[Origin, ...] grammar"
    );
    let c1_reply = fixture("c1privnet_origin_intersect_recv.cbor");
    assert_eq!(
        &c1_reply[8..11],
        &[0x83, 0x05, 0x80],
        "captured c1 IntersectFound reply fixture has the SAME grammar"
    );
}
