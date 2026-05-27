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

use ade_types::{Hash32, SlotNo};

use crate::block_fetch::agency::BlockFetchAgency;
use crate::block_fetch::state::{BlockFetchError, BlockFetchState};
use crate::block_fetch::transition::block_fetch_transition;
use crate::codec::block_fetch::{BlockFetchMessage, Point};
use crate::codec::version::BlockFetchVersion;

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

// =====================================================================
// PHASE4-N-G S4 — Producer-side block-fetch server reducer
// =====================================================================

/// Read-side trait over the producer's served chain for block-fetch.
/// Production impl lives in `ade_runtime` (S5 GREEN adapter); test
/// impl wraps `ade_ledger::producer::ServedChainSnapshot`.
pub trait ServedRangeLookup {
    /// Inclusive range of `(slot, hash, bytes)` in BTreeMap order.
    /// `bytes` is the underlying `AcceptedBlock.as_bytes()` cloned
    /// into an owned `Vec<u8>` for ownership transfer into the
    /// `ServerReply::block(bytes)` constructor.
    fn range_bytes(
        &self,
        from: (SlotNo, Hash32),
        to: (SlotNo, Hash32),
    ) -> Vec<(SlotNo, Hash32, Vec<u8>)>;
}

/// Producer-side block-fetch server state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducerBlockFetchServerState {
    pub state: BlockFetchState,
}

impl ProducerBlockFetchServerState {
    pub fn new() -> Self {
        Self {
            state: BlockFetchState::Idle,
        }
    }
}

/// Closed producer-side block-fetch server error sum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerBlockFetchServerError {
    Grammar(BlockFetchError),
}

/// One step of the producer-side block-fetch server pump in response
/// to a client message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockFetchServerStep {
    /// Client said `ClientDone`; session over.
    Done,
    /// Sequence of server replies to emit in order.
    Replies(Vec<ServerReply>),
}

/// Process one client-originated block-fetch message. Pure, total,
/// deterministic. The reducer never re-encodes block bytes; every
/// `Block { bytes }` payload it constructs is sourced verbatim from
/// `served.range_bytes` (which itself returns
/// `AcceptedBlock.as_bytes()` slices).
pub fn producer_block_fetch_serve(
    mut state: ProducerBlockFetchServerState,
    in_msg: BlockFetchMessage,
    served: &dyn ServedRangeLookup,
    version: BlockFetchVersion,
) -> Result<
    (ProducerBlockFetchServerState, BlockFetchServerStep),
    ProducerBlockFetchServerError,
> {
    let (new_state, _output) =
        block_fetch_transition(state.state, BlockFetchAgency::Client, version, in_msg.clone())
            .map_err(ProducerBlockFetchServerError::Grammar)?;
    state.state = new_state;

    match in_msg {
        BlockFetchMessage::RequestRange(range) => {
            // Narrow scope: Origin endpoints → NoBlocks. We do not
            // serve the genesis chain.
            let from = match range.from {
                Point::Origin => {
                    state.state = BlockFetchState::Idle;
                    return Ok((state, BlockFetchServerStep::Replies(vec![ServerReply::no_blocks()])));
                }
                Point::Block { slot, hash } => (slot, hash),
            };
            let to = match range.to {
                Point::Origin => {
                    state.state = BlockFetchState::Idle;
                    return Ok((state, BlockFetchServerStep::Replies(vec![ServerReply::no_blocks()])));
                }
                Point::Block { slot, hash } => (slot, hash),
            };
            let entries = served.range_bytes(from.clone(), to.clone());
            if entries.is_empty() {
                state.state = BlockFetchState::Idle;
                return Ok((state, BlockFetchServerStep::Replies(vec![ServerReply::no_blocks()])));
            }
            // PHASE4-N-R-B B4 / CN-SNAPSHOT-02 / N4 fix: enforce
            // protocol-defined block-fetch failure semantics for
            // partial-overlap RequestRange. Both endpoints MUST be
            // present in the served snapshot; if either endpoint is
            // missing, reply NoBlocks (matches Haskell
            // ouroboros-network reference for unknown / partially-
            // unavailable ranges). The previous implementation
            // returned "whatever was in the BTreeMap range" — a
            // permissive ad-hoc partial response that diverged from
            // the canonical cardano-node behavior.
            let first_key = (entries.first().map(|(s, h, _)| (*s, h.clone())))
                .expect("entries is non-empty");
            let last_key = (entries.last().map(|(s, h, _)| (*s, h.clone())))
                .expect("entries is non-empty");
            if first_key != from || last_key != to {
                state.state = BlockFetchState::Idle;
                return Ok((state, BlockFetchServerStep::Replies(vec![ServerReply::no_blocks()])));
            }
            // Non-empty range with both endpoints present:
            // [StartBatch, Block(b)*, BatchDone].
            // After the orchestrator transmits all of these, the BLUE
            // state machine walks Busy -> Streaming -> ... -> Idle;
            // we set the final state directly since the reducer's
            // contract is "what state will the per-session machine be
            // in once these frames are sent."
            let mut replies = Vec::with_capacity(entries.len() + 2);
            replies.push(ServerReply::start_batch());
            for (_slot, _hash, bytes) in entries {
                replies.push(ServerReply::block(bytes));
            }
            replies.push(ServerReply::batch_done());
            state.state = BlockFetchState::Idle;
            Ok((state, BlockFetchServerStep::Replies(replies)))
        }
        BlockFetchMessage::ClientDone => Ok((state, BlockFetchServerStep::Done)),
        // Server-originated messages from Client sender are grammar
        // violations — already rejected by the BLUE transition above.
        BlockFetchMessage::StartBatch
        | BlockFetchMessage::NoBlocks
        | BlockFetchMessage::Block { .. }
        | BlockFetchMessage::BatchDone => Err(ProducerBlockFetchServerError::Grammar(
            crate::block_fetch::state::BlockFetchError::IllegalTransition {
                state: state.state,
                message_tag: "server-originated message arrived as Client",
                agency: BlockFetchAgency::Client,
            },
        )),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::block_fetch::{decode_block_fetch_message, encode_block_fetch_message, Range};

    fn block_bytes_sample() -> Vec<u8> {
        // Block body is carried as a single opaque CBOR item per the
        // block-fetch codec contract. A `bytes(6)` value is the simplest
        // single-item shape that round-trips byte-identically.
        vec![0x46, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]
    }

    // -----------------------------------------------------------------
    // PHASE4-N-G S4: reducer tests with corpus-backed ServedRangeLookup
    // -----------------------------------------------------------------

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::producer::{self_accept, ServedChainSnapshot};
    use ade_ledger::state::LedgerState;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28};

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        let eras = vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: SlotNo(start_576),
            start_epoch: EPOCH_576,
            slot_length_ms: 1_000,
            epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
            safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
        }];
        EraSchedule::new(BootstrapAnchorHash(Hash32([0u8; 32])), 0, eras).expect("schedule")
    }

    fn corpus_view() -> (ConwayValidityCorpus, PoolDistrView) {
        let c = ConwayValidityCorpus::load().expect("corpus loads");
        let total = c.pd_total_active_stake;
        let asc = ActiveSlotsCoeff {
            numer: c.asc.numer as u32,
            denom: c.asc.denom as u32,
        };
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        for (pool_id, p) in &c.pools {
            let scale = total / p.sigma.denom;
            let active_stake = p.sigma.numer * scale;
            pools.insert(
                Hash28(*pool_id),
                PoolEntry {
                    active_stake,
                    vrf_keyhash: Hash32(p.vrf_keyhash),
                },
            );
        }
        let view = PoolDistrView::new(EPOCH_576, total, asc, pools);
        (c, view)
    }

    fn build_served_with_first_n_lightest(n: usize) -> (ServedChainSnapshot, Vec<Vec<u8>>) {
        let (c, view) = corpus_view();
        let mut idxs: Vec<usize> = (0..c.blocks.len()).collect();
        idxs.sort_by_key(|&i| {
            let env = decode_block_envelope(&c.blocks[i]).expect("env");
            env.block_end - env.block_start
        });
        let block_bytes: Vec<Vec<u8>> =
            idxs.into_iter().take(n).map(|i| c.blocks[i].clone()).collect();
        let schedule = schedule();
        let ledger = {
            let mut l = LedgerState::new(CardanoEra::Conway);
            l.epoch_state.epoch = EPOCH_576;
            l
        };
        let chain_dep = {
            let mut s = PraosChainDepState::empty();
            s.epoch_nonce = Nonce(Hash32(c.epoch_nonce));
            s.evolving_nonce = Nonce(Hash32(c.epoch_nonce));
            s
        };
        let mut snap = ServedChainSnapshot::new();
        for b in &block_bytes {
            let accepted = self_accept(b, &ledger, &chain_dep, &schedule, &view)
                .expect("self_accept");
            snap = ade_ledger::producer::served_chain_admit(snap, accepted).expect("admit");
        }
        (snap, block_bytes)
    }

    struct SnapshotRangeLookup<'a> {
        snap: &'a ServedChainSnapshot,
    }

    impl<'a> ServedRangeLookup for SnapshotRangeLookup<'a> {
        fn range_bytes(
            &self,
            from: (SlotNo, Hash32),
            to: (SlotNo, Hash32),
        ) -> Vec<(SlotNo, Hash32, Vec<u8>)> {
            self.snap
                .range_bytes(from, to)
                .map(|(slot, hash, bytes)| (slot, hash.clone(), bytes.to_vec()))
                .collect()
        }
    }

    fn v() -> BlockFetchVersion {
        BlockFetchVersion::new(9)
    }

    fn block_point(slot: u64, hash: [u8; 32]) -> Point {
        Point::Block {
            slot: SlotNo(slot),
            hash: Hash32(hash),
        }
    }

    #[test]
    fn producer_block_fetch_serve_request_range_in_chain_yields_start_batch_blocks_batch_done() {
        let (snap, _blocks) = build_served_with_first_n_lightest(2);
        let look = SnapshotRangeLookup { snap: &snap };
        // Compute (from, to) covering all admitted blocks.
        let mut keys: Vec<(SlotNo, Hash32)> = snap
            .iter()
            .map(|(s, h, _)| (s, h.clone()))
            .collect();
        keys.sort();
        let from = keys.first().expect("non-empty").clone();
        let to = keys.last().expect("non-empty").clone();
        let range = Range {
            from: Point::Block { slot: from.0, hash: from.1.clone() },
            to: Point::Block { slot: to.0, hash: to.1.clone() },
        };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, BlockFetchState::Idle);
        match step {
            BlockFetchServerStep::Replies(replies) => {
                // Shape: [StartBatch, Block(_)..N, BatchDone]
                assert!(replies.len() >= 3, "expected StartBatch + Block(_)* + BatchDone, got {} replies", replies.len());
                let msgs: Vec<BlockFetchMessage> =
                    replies.into_iter().map(|r| r.into_message()).collect();
                assert!(matches!(msgs.first(), Some(BlockFetchMessage::StartBatch)));
                assert!(matches!(msgs.last(), Some(BlockFetchMessage::BatchDone)));
                let middle = &msgs[1..msgs.len() - 1];
                for m in middle {
                    assert!(matches!(m, BlockFetchMessage::Block { .. }));
                }
            }
            other => panic!("expected Replies, got {other:?}"),
        }
    }

    #[test]
    fn producer_block_fetch_serve_request_range_empty_in_chain_yields_no_blocks() {
        // Range bounds inside admitted slot extent but with hashes
        // that don't exist → no entries returned by range_bytes →
        // NoBlocks. Use a range from (slot=u64::MAX, hash=0xFF..)
        // to (slot=u64::MAX, hash=0xFF..) — empty in the snapshot.
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotRangeLookup { snap: &snap };
        let high = (SlotNo(u64::MAX), Hash32([0xFF; 32]));
        let range = Range {
            from: Point::Block { slot: high.0, hash: high.1.clone() },
            to: Point::Block { slot: high.0, hash: high.1.clone() },
        };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, BlockFetchState::Idle);
        match step {
            BlockFetchServerStep::Replies(replies) => {
                let msgs: Vec<_> = replies.into_iter().map(|r| r.into_message()).collect();
                assert_eq!(msgs, vec![BlockFetchMessage::NoBlocks]);
            }
            other => panic!("expected Replies, got {other:?}"),
        }
    }

    #[test]
    fn producer_block_fetch_serve_request_range_outside_chain_yields_no_blocks() {
        // Range bounds outside any admitted block — same NoBlocks
        // result. Distinct from "empty range within chain" by
        // intention.
        let snap = ServedChainSnapshot::new();
        let look = SnapshotRangeLookup { snap: &snap };
        let range = Range {
            from: block_point(100, [0xAA; 32]),
            to: block_point(200, [0xBB; 32]),
        };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, BlockFetchState::Idle);
        match step {
            BlockFetchServerStep::Replies(replies) => {
                let msgs: Vec<_> = replies.into_iter().map(|r| r.into_message()).collect();
                assert_eq!(msgs, vec![BlockFetchMessage::NoBlocks]);
            }
            other => panic!("expected Replies, got {other:?}"),
        }
    }

    #[test]
    fn producer_block_fetch_serve_block_bytes_equal_accepted_block_as_bytes() {
        let (snap, blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotRangeLookup { snap: &snap };
        let (slot, hash, _) = snap.iter().next().expect("non-empty");
        let range = Range {
            from: Point::Block { slot, hash: hash.clone() },
            to: Point::Block { slot, hash: hash.clone() },
        };
        let state = ProducerBlockFetchServerState::new();
        let (_state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        let msgs: Vec<BlockFetchMessage> = match step {
            BlockFetchServerStep::Replies(r) => r.into_iter().map(|x| x.into_message()).collect(),
            other => panic!("expected Replies, got {other:?}"),
        };
        let block_msg = msgs
            .iter()
            .find_map(|m| match m {
                BlockFetchMessage::Block { bytes } => Some(bytes.clone()),
                _ => None,
            })
            .expect("Block in replies");
        // DC-CONS-17 surface: served bytes equal admitted
        // AcceptedBlock.as_bytes() which equals the corpus block.
        assert_eq!(block_msg, blocks[0]);
    }

    #[test]
    fn producer_block_fetch_serve_request_range_with_origin_endpoint_yields_no_blocks() {
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotRangeLookup { snap: &snap };
        let range = Range {
            from: Point::Origin,
            to: block_point(999_999_999, [0xFF; 32]),
        };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, BlockFetchState::Idle);
        match step {
            BlockFetchServerStep::Replies(r) => {
                let msgs: Vec<_> = r.into_iter().map(|x| x.into_message()).collect();
                assert_eq!(msgs, vec![BlockFetchMessage::NoBlocks]);
            }
            other => panic!("expected Replies, got {other:?}"),
        }
    }

    #[test]
    fn producer_block_fetch_serve_client_done_terminates_session() {
        let snap = ServedChainSnapshot::new();
        let look = SnapshotRangeLookup { snap: &snap };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) =
            producer_block_fetch_serve(state, BlockFetchMessage::ClientDone, &look, v()).unwrap();
        assert_eq!(state2.state, BlockFetchState::Done);
        assert_eq!(step, BlockFetchServerStep::Done);
    }

    #[test]
    fn producer_block_fetch_serve_rejects_illegal_grammar_pair() {
        // Server message (StartBatch) from Client agency → grammar
        // reject at the BLUE state machine.
        let snap = ServedChainSnapshot::new();
        let look = SnapshotRangeLookup { snap: &snap };
        let state = ProducerBlockFetchServerState::new();
        let err = producer_block_fetch_serve(
            state,
            BlockFetchMessage::StartBatch,
            &look,
            v(),
        )
        .expect_err("must reject");
        match err {
            ProducerBlockFetchServerError::Grammar(_) => {}
        }
    }

    #[test]
    fn producer_block_fetch_serve_replays_byte_identical_over_corpus() {
        let (snap, _blocks) = build_served_with_first_n_lightest(2);
        let look = SnapshotRangeLookup { snap: &snap };
        let mut keys: Vec<(SlotNo, Hash32)> = snap.iter().map(|(s, h, _)| (s, h.clone())).collect();
        keys.sort();
        let from = keys.first().expect("non-empty").clone();
        let to = keys.last().expect("non-empty").clone();
        let range = Range {
            from: Point::Block { slot: from.0, hash: from.1.clone() },
            to: Point::Block { slot: to.0, hash: to.1.clone() },
        };
        let inputs = vec![BlockFetchMessage::RequestRange(range)];
        let run = |inputs: &[BlockFetchMessage]| -> Vec<Vec<u8>> {
            let mut state = ProducerBlockFetchServerState::new();
            let mut frames = Vec::new();
            for m in inputs {
                let (s2, step) =
                    producer_block_fetch_serve(state, m.clone(), &look, v()).unwrap();
                state = s2;
                match step {
                    BlockFetchServerStep::Replies(replies) => {
                        for r in replies {
                            frames.push(encode_block_fetch_message(&r.into_message()));
                        }
                    }
                    BlockFetchServerStep::Done => break,
                }
            }
            frames
        };
        let a = run(&inputs);
        let b = run(&inputs);
        assert_eq!(a, b, "replay must be byte-identical");
    }

    // ==========================================================
    // PHASE4-N-R-B B4 / OQ8 / CN-SNAPSHOT-02 / N4
    // ==========================================================
    //
    // Partial-overlap RequestRange MUST return NoBlocks per the
    // Cardano block-fetch protocol's failure semantics. Both
    // endpoints + every block between MUST be present.

    #[test]
    fn n_r_b_partial_overlap_from_endpoint_not_in_snapshot_yields_no_blocks() {
        let (snap, _blocks) = build_served_with_first_n_lightest(2);
        let look = SnapshotRangeLookup { snap: &snap };
        // Real `to` from the snapshot; `from` is a fabricated key
        // not present (slot=0, all-zero hash).
        let mut keys: Vec<(SlotNo, Hash32)> = snap
            .iter()
            .map(|(s, h, _)| (s, h.clone()))
            .collect();
        keys.sort();
        let to_key = keys.last().expect("non-empty").clone();
        let range = Range {
            from: block_point(0, [0u8; 32]),
            to: Point::Block {
                slot: to_key.0,
                hash: to_key.1,
            },
        };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, BlockFetchState::Idle);
        match step {
            BlockFetchServerStep::Replies(replies) => {
                let msgs: Vec<BlockFetchMessage> =
                    replies.into_iter().map(|r| r.into_message()).collect();
                assert_eq!(msgs, vec![BlockFetchMessage::NoBlocks]);
            }
            BlockFetchServerStep::Done => panic!("expected NoBlocks reply"),
        }
    }

    #[test]
    fn n_r_b_partial_overlap_to_endpoint_not_in_snapshot_yields_no_blocks() {
        let (snap, _blocks) = build_served_with_first_n_lightest(2);
        let look = SnapshotRangeLookup { snap: &snap };
        let mut keys: Vec<(SlotNo, Hash32)> = snap
            .iter()
            .map(|(s, h, _)| (s, h.clone()))
            .collect();
        keys.sort();
        let from_key = keys.first().expect("non-empty").clone();
        // Real `from`; `to` is a fabricated key (very high slot).
        let range = Range {
            from: Point::Block {
                slot: from_key.0,
                hash: from_key.1,
            },
            to: block_point(u64::MAX - 1, [0xFFu8; 32]),
        };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, BlockFetchState::Idle);
        match step {
            BlockFetchServerStep::Replies(replies) => {
                let msgs: Vec<BlockFetchMessage> =
                    replies.into_iter().map(|r| r.into_message()).collect();
                assert_eq!(msgs, vec![BlockFetchMessage::NoBlocks]);
            }
            BlockFetchServerStep::Done => panic!("expected NoBlocks reply"),
        }
    }

    #[test]
    fn n_r_b_partial_overlap_both_endpoints_fabricated_yields_no_blocks() {
        let (snap, _blocks) = build_served_with_first_n_lightest(2);
        let look = SnapshotRangeLookup { snap: &snap };
        // Both endpoints fabricated and bracketing the real entries:
        // BTreeMap range_bytes may still return non-empty if the
        // fabricated range straddles real keys; the endpoint-presence
        // check rejects the partial overlap.
        let mut keys: Vec<(SlotNo, Hash32)> = snap
            .iter()
            .map(|(s, h, _)| (s, h.clone()))
            .collect();
        keys.sort();
        let real_min_slot = keys.first().expect("non-empty").0;
        let real_max_slot = keys.last().expect("non-empty").0;
        let range = Range {
            from: block_point(real_min_slot.0.saturating_sub(1), [0u8; 32]),
            to: block_point(real_max_slot.0 + 1, [0xFFu8; 32]),
        };
        let state = ProducerBlockFetchServerState::new();
        let (state2, step) = producer_block_fetch_serve(
            state,
            BlockFetchMessage::RequestRange(range),
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, BlockFetchState::Idle);
        match step {
            BlockFetchServerStep::Replies(replies) => {
                let msgs: Vec<BlockFetchMessage> =
                    replies.into_iter().map(|r| r.into_message()).collect();
                assert_eq!(msgs, vec![BlockFetchMessage::NoBlocks]);
            }
            BlockFetchServerStep::Done => panic!("expected NoBlocks reply"),
        }
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
