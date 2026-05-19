// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data
//
// Pure block-fetch transition function.
//
// Shape (per slice §9):
//   fn block_fetch_transition(state, agency, version, msg)
//       -> Result<(new_state, output), error>
//
// No async, no I/O, no ambient state. The selected version is an
// explicit input — never read from a session global (DC-PROTO-06).
//
// State graph (only these tuples are legal; everything else is
// IllegalTransition):
//
//   Idle      + Client + RequestRange(range) -> Busy      + Reply(RequestRange(range))
//   Idle      + Client + ClientDone          -> Done      + Done
//   Busy      + Server + StartBatch          -> Streaming + Event(BatchStarted)
//   Busy      + Server + NoBlocks            -> Idle      + Event(NoBlocks)
//   Streaming + Server + Block { bytes }     -> Streaming + Event(BlockDelivered { block_bytes: bytes })
//   Streaming + Server + BatchDone           -> Idle      + Event(BatchCompleted)
//
// Range validation: in the Idle+Client+RequestRange branch, an inverted
// concrete range (`from` block slot > `to` block slot) is rejected as
// `MalformedMessage`. Origin endpoints are pseudo-points (genesis) — any
// combination involving Origin is legal at the state machine; the
// server returns NoBlocks for empty ranges.

use crate::block_fetch::agency::BlockFetchAgency;
use crate::block_fetch::event::BatchDeliveryEvent;
use crate::block_fetch::state::{BlockFetchError, BlockFetchOutput, BlockFetchState};
use crate::codec::block_fetch::{BlockFetchMessage, Point};
use crate::codec::version::BlockFetchVersion;

/// Highest block-fetch mini-protocol version this state machine accepts.
///
/// Block-fetch has shipped a single closed grammar (6 messages, no
/// version-gated variants) for every cardano-node 11.0.1 (10.6.2 forward-compatible) supported
/// version. We pin the upper bound at `MAX_BLOCK_FETCH_VERSION` so a
/// future spec extension cannot silently transit messages whose
/// semantics this state machine has not been updated for — the
/// `InvalidForVersion` error surfaces the mismatch at the protocol
/// boundary instead of letting an unknown future variant through.
const MAX_BLOCK_FETCH_VERSION: u16 = 100;

/// Pure block-fetch transition.
///
/// `agency` is the agency the *sender* held when it produced `msg`.
/// Client-originated messages (RequestRange / ClientDone) are paired
/// with `BlockFetchAgency::Client`; server-originated replies are
/// paired with `Server`. Any other pairing returns `IllegalTransition`.
pub fn block_fetch_transition(
    state: BlockFetchState,
    agency: BlockFetchAgency,
    version: BlockFetchVersion,
    msg: BlockFetchMessage,
) -> Result<(BlockFetchState, BlockFetchOutput), BlockFetchError> {
    if version.get() > MAX_BLOCK_FETCH_VERSION {
        return Err(BlockFetchError::InvalidForVersion {
            version,
            message_tag: message_tag(&msg),
        });
    }
    match (state, agency, msg) {
        (
            BlockFetchState::Idle,
            BlockFetchAgency::Client,
            BlockFetchMessage::RequestRange(range),
        ) => {
            if let (
                Point::Block { slot: s_from, .. },
                Point::Block { slot: s_to, .. },
            ) = (&range.from, &range.to)
            {
                if s_from.0 > s_to.0 {
                    return Err(BlockFetchError::MalformedMessage {
                        reason: "BlockFetch range is inverted (from > to)",
                    });
                }
            }
            Ok((
                BlockFetchState::Busy,
                BlockFetchOutput::Reply(BlockFetchMessage::RequestRange(range)),
            ))
        }
        (BlockFetchState::Idle, BlockFetchAgency::Client, BlockFetchMessage::ClientDone) => {
            Ok((BlockFetchState::Done, BlockFetchOutput::Done))
        }
        (BlockFetchState::Busy, BlockFetchAgency::Server, BlockFetchMessage::StartBatch) => Ok((
            BlockFetchState::Streaming,
            BlockFetchOutput::Event(BatchDeliveryEvent::BatchStarted),
        )),
        (BlockFetchState::Busy, BlockFetchAgency::Server, BlockFetchMessage::NoBlocks) => Ok((
            BlockFetchState::Idle,
            BlockFetchOutput::Event(BatchDeliveryEvent::NoBlocks),
        )),
        (
            BlockFetchState::Streaming,
            BlockFetchAgency::Server,
            BlockFetchMessage::Block { bytes },
        ) => Ok((
            BlockFetchState::Streaming,
            BlockFetchOutput::Event(BatchDeliveryEvent::BlockDelivered { block_bytes: bytes }),
        )),
        (BlockFetchState::Streaming, BlockFetchAgency::Server, BlockFetchMessage::BatchDone) => {
            Ok((
                BlockFetchState::Idle,
                BlockFetchOutput::Event(BatchDeliveryEvent::BatchCompleted),
            ))
        }
        (state, agency, msg) => Err(BlockFetchError::IllegalTransition {
            state,
            message_tag: message_tag(&msg),
            agency,
        }),
    }
}

fn message_tag(msg: &BlockFetchMessage) -> &'static str {
    match msg {
        BlockFetchMessage::RequestRange(_) => "RequestRange",
        BlockFetchMessage::ClientDone => "ClientDone",
        BlockFetchMessage::StartBatch => "StartBatch",
        BlockFetchMessage::NoBlocks => "NoBlocks",
        BlockFetchMessage::Block { .. } => "Block",
        BlockFetchMessage::BatchDone => "BatchDone",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::block_fetch::Range;
    use ade_types::{Hash32, SlotNo};

    fn version() -> BlockFetchVersion {
        BlockFetchVersion::new(9)
    }

    fn block_point(slot: u64, seed: u8) -> Point {
        Point::Block {
            slot: SlotNo(slot),
            hash: Hash32([seed; 32]),
        }
    }

    fn sample_range() -> Range {
        Range {
            from: block_point(100, 0x11),
            to: block_point(200, 0x22),
        }
    }

    fn sample_block_bytes() -> Vec<u8> {
        vec![0x82, 0x01, 0x02, 0x03, 0x04, 0xFF, 0xAA, 0xBB, 0xCC, 0xDD]
    }

    #[test]
    fn idle_request_range_yields_busy_then_start_batch() {
        // Two-step drive: Idle -> Busy via client RequestRange,
        // then Busy -> Streaming via server StartBatch + Event(BatchStarted).
        let range = sample_range();
        let (st1, out1) = block_fetch_transition(
            BlockFetchState::Idle,
            BlockFetchAgency::Client,
            version(),
            BlockFetchMessage::RequestRange(range.clone()),
        )
        .expect("idle+request_range");
        assert_eq!(st1, BlockFetchState::Busy);
        match out1 {
            BlockFetchOutput::Reply(BlockFetchMessage::RequestRange(r)) => {
                assert_eq!(r, range);
            }
            other => panic!("expected Reply(RequestRange), got {other:?}"),
        }

        let (st2, out2) = block_fetch_transition(
            st1,
            BlockFetchAgency::Server,
            version(),
            BlockFetchMessage::StartBatch,
        )
        .expect("busy+start_batch");
        assert_eq!(st2, BlockFetchState::Streaming);
        match out2 {
            BlockFetchOutput::Event(BatchDeliveryEvent::BatchStarted) => {}
            other => panic!("expected Event(BatchStarted), got {other:?}"),
        }
    }

    #[test]
    fn busy_no_blocks_returns_to_idle_with_event() {
        let (st, out) = block_fetch_transition(
            BlockFetchState::Busy,
            BlockFetchAgency::Server,
            version(),
            BlockFetchMessage::NoBlocks,
        )
        .expect("busy+no_blocks");
        assert_eq!(st, BlockFetchState::Idle);
        match out {
            BlockFetchOutput::Event(BatchDeliveryEvent::NoBlocks) => {}
            other => panic!("expected Event(NoBlocks), got {other:?}"),
        }
    }

    #[test]
    fn streaming_block_delivers_bytes_byte_identical() {
        let bytes_in = sample_block_bytes();
        let (st, out) = block_fetch_transition(
            BlockFetchState::Streaming,
            BlockFetchAgency::Server,
            version(),
            BlockFetchMessage::Block {
                bytes: bytes_in.clone(),
            },
        )
        .expect("streaming+block");
        assert_eq!(st, BlockFetchState::Streaming);
        match out {
            BlockFetchOutput::Event(BatchDeliveryEvent::BlockDelivered { block_bytes }) => {
                assert_eq!(block_bytes, bytes_in);
            }
            other => panic!("expected Event(BlockDelivered), got {other:?}"),
        }
    }

    #[test]
    fn streaming_batch_done_returns_to_idle() {
        let (st, out) = block_fetch_transition(
            BlockFetchState::Streaming,
            BlockFetchAgency::Server,
            version(),
            BlockFetchMessage::BatchDone,
        )
        .expect("streaming+batch_done");
        assert_eq!(st, BlockFetchState::Idle);
        match out {
            BlockFetchOutput::Event(BatchDeliveryEvent::BatchCompleted) => {}
            other => panic!("expected Event(BatchCompleted), got {other:?}"),
        }
    }

    #[test]
    fn multi_block_streaming_preserves_order() {
        // Drive five consecutive Block messages and assert five
        // BlockDelivered events arrive in the same order with
        // byte-identical payloads.
        let payloads: Vec<Vec<u8>> = (0u8..5)
            .map(|i| {
                let mut v = sample_block_bytes();
                v.push(i);
                v.push(i.wrapping_add(0xA0));
                v
            })
            .collect();

        let mut state = BlockFetchState::Streaming;
        let mut delivered: Vec<Vec<u8>> = Vec::new();
        for p in &payloads {
            let (next, out) = block_fetch_transition(
                state,
                BlockFetchAgency::Server,
                version(),
                BlockFetchMessage::Block { bytes: p.clone() },
            )
            .expect("streaming+block");
            state = next;
            match out {
                BlockFetchOutput::Event(BatchDeliveryEvent::BlockDelivered { block_bytes }) => {
                    delivered.push(block_bytes);
                }
                other => panic!("expected Event(BlockDelivered), got {other:?}"),
            }
        }
        assert_eq!(state, BlockFetchState::Streaming);
        assert_eq!(delivered, payloads);
    }

    #[test]
    fn client_done_terminates_session() {
        let (st, out) = block_fetch_transition(
            BlockFetchState::Idle,
            BlockFetchAgency::Client,
            version(),
            BlockFetchMessage::ClientDone,
        )
        .expect("idle+client_done");
        assert_eq!(st, BlockFetchState::Done);
        match out {
            BlockFetchOutput::Done => {}
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn illegal_message_in_idle_returns_error() {
        // Server-only message arriving while the state machine is
        // Idle is grammar-illegal.
        let err = block_fetch_transition(
            BlockFetchState::Idle,
            BlockFetchAgency::Server,
            version(),
            BlockFetchMessage::StartBatch,
        )
        .expect_err("must reject");
        match err {
            BlockFetchError::IllegalTransition {
                state,
                message_tag,
                agency,
            } => {
                assert_eq!(state, BlockFetchState::Idle);
                assert_eq!(message_tag, "StartBatch");
                assert_eq!(agency, BlockFetchAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn wrong_agency_returns_error() {
        // RequestRange is a client-originated message; passing Server
        // agency for it is grammar-illegal.
        let err = block_fetch_transition(
            BlockFetchState::Idle,
            BlockFetchAgency::Server,
            version(),
            BlockFetchMessage::RequestRange(sample_range()),
        )
        .expect_err("must reject");
        match err {
            BlockFetchError::IllegalTransition {
                message_tag,
                agency,
                ..
            } => {
                assert_eq!(message_tag, "RequestRange");
                assert_eq!(agency, BlockFetchAgency::Server);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn version_gating_rejects_out_of_version_message() {
        // The block-fetch wire grammar across cardano-node 11.0.1 (10.6.2 forward-compatible) has
        // shipped a single closed message set for every supported
        // version, so there is no real per-variant version gating yet
        // (the codec already accepts all 6 variants on the wire). The
        // state machine still has to expose the InvalidForVersion
        // error path because the type signature commits to it; the
        // pinned guard rejects future versions above
        // MAX_BLOCK_FETCH_VERSION = 100. When IOG ships a block-fetch
        // grammar extension, the gate moves to per-variant checks and
        // this test gets per-variant siblings.
        let bogus_version = BlockFetchVersion::new(MAX_BLOCK_FETCH_VERSION + 1);
        let err = block_fetch_transition(
            BlockFetchState::Idle,
            BlockFetchAgency::Client,
            bogus_version,
            BlockFetchMessage::RequestRange(sample_range()),
        )
        .expect_err("must reject");
        match err {
            BlockFetchError::InvalidForVersion {
                version,
                message_tag,
            } => {
                assert_eq!(version, bogus_version);
                assert_eq!(message_tag, "RequestRange");
            }
            other => panic!("expected InvalidForVersion, got {other:?}"),
        }
    }

    #[test]
    fn inverted_range_returns_malformed() {
        // RequestRange with from.slot > to.slot is a grammar violation:
        // the protocol requires a forward-oriented inclusive range.
        // The codec accepts an inverted range at the byte layer; the
        // state machine is the enforcement point for the grammar
        // invariant. Covers the BlockFetchError::MalformedMessage path.
        let inverted = Range {
            from: block_point(500, 0x11),
            to: block_point(100, 0x22),
        };
        let err = block_fetch_transition(
            BlockFetchState::Idle,
            BlockFetchAgency::Client,
            version(),
            BlockFetchMessage::RequestRange(inverted),
        )
        .expect_err("must reject");
        match err {
            BlockFetchError::MalformedMessage { reason } => {
                assert_eq!(reason, "BlockFetch range is inverted (from > to)");
            }
            other => panic!("expected MalformedMessage, got {other:?}"),
        }
    }

    #[test]
    fn block_delivered_event_carries_bytes_verbatim() {
        // Explicit byte-identity test: every byte in [0..=255] passed
        // in as Block.bytes must arrive verbatim in
        // BlockDelivered.block_bytes — no transformation, no padding,
        // no truncation. Distinct from the small-payload roundtrip in
        // streaming_block_delivers_bytes_byte_identical.
        let bytes_in: Vec<u8> = (0u8..=255).collect();
        let (_, out) = block_fetch_transition(
            BlockFetchState::Streaming,
            BlockFetchAgency::Server,
            version(),
            BlockFetchMessage::Block {
                bytes: bytes_in.clone(),
            },
        )
        .expect("streaming+block");
        match out {
            BlockFetchOutput::Event(BatchDeliveryEvent::BlockDelivered { block_bytes }) => {
                assert_eq!(block_bytes.len(), bytes_in.len());
                assert_eq!(block_bytes, bytes_in);
            }
            other => panic!("expected Event(BlockDelivered), got {other:?}"),
        }
    }
}
