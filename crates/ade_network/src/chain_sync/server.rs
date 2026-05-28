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

use ade_types::{CardanoEra, Hash32, SlotNo};

use crate::chain_sync::agency::ChainSyncAgency;
use crate::chain_sync::state::{ChainSyncOutput, ChainSyncState};
use crate::chain_sync::transition::chain_sync_transition;
use crate::codec::chain_sync::{compose_rollforward_header, ChainSyncMessage, Point, Tip};
use crate::codec::version::ChainSyncVersion;

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

// =====================================================================
// PHASE4-N-G S3 — Producer-side chain-sync server reducers
// =====================================================================
//
// `producer_chain_sync_serve` and `producer_chain_sync_advance_tip`
// are pure, total, deterministic transitions composing the BLUE
// `chain_sync_transition` (PHASE4-N-A) for grammar validation with
// the producer-side decision logic (whose state lives in
// `ProducerChainSyncServerState`).
//
// The reducers read the served chain through a trait-bound seam
// (`ServedHeaderLookup`) to avoid a hard dependency on `ade_ledger`
// from this BLUE protocol crate; the production impl lives in
// `ade_runtime` (S5 GREEN adapter) and the test impl in this file's
// `tests` module wraps `ade_ledger::producer::ServedChainSnapshot`
// over the Conway-576 corpus.
//
// Deterministic-resolution discipline (`DC-PROTO-08`): every
// server-agency state returns a legal `RollForward` / `RollBackward`
// / `AwaitReply` or a structured close-or-error. No ambiguous wait.

/// Header projection emitted by `ServedHeaderLookup::next_after`.
/// `header_bytes` is the canonical header sub-slice per
/// `accepted_block_header_bytes` (`DC-CONS-18`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderProjection {
    pub slot: SlotNo,
    pub hash: Hash32,
    pub block_no: u64,
    /// The era of the block this header projects from. Drives the
    /// ChainSync consensus era index in the RollForward header wrap
    /// (CN-WIRE-08); `header_bytes` stays the BARE era-specific header.
    pub era: CardanoEra,
    pub header_bytes: Vec<u8>,
}

/// Read-side trait over the producer's served chain. Implemented in
/// `ade_runtime` by the GREEN adapter wrapping `ServedChainSnapshot`;
/// implemented in this file's tests by a corpus-backed mock.
pub trait ServedHeaderLookup {
    /// Smallest `(slot, hash) > cursor` in the served chain, with
    /// the canonical header projection of that block's bytes.
    fn next_after(&self, cursor: Option<(SlotNo, Hash32)>) -> Option<HeaderProjection>;

    /// First point in `points` that matches a served-chain key.
    /// Returns `Point::Block { slot, hash }` for a match. `Origin` is
    /// considered intersected only if the served chain is empty AND
    /// `points` includes `Origin` — but under narrow scope we don't
    /// claim Origin; tests pin this behavior.
    fn intersect(&self, points: &[Point]) -> Option<(SlotNo, Hash32)>;

    /// Current served-chain head `(slot, hash, block_no)` if any.
    fn tip(&self) -> Option<(SlotNo, Hash32, u64)>;
}

/// Producer-side chain-sync server state.
///
/// `state` delegates protocol-grammar state to the PHASE4-N-A BLUE
/// state machine. `last_announced` is the producer-side cursor into
/// the served chain — the largest `(slot, hash)` we have told this
/// session's client about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducerChainSyncServerState {
    pub state: ChainSyncState,
    pub last_announced: Option<(SlotNo, Hash32)>,
}

impl ProducerChainSyncServerState {
    pub fn new() -> Self {
        Self {
            state: ChainSyncState::Idle,
            last_announced: None,
        }
    }
}

/// Closed producer-side server error sum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerServerError {
    /// The peer message violated the chain-sync grammar at the
    /// underlying BLUE state machine. Carries the upstream typed
    /// error verbatim.
    Grammar(crate::chain_sync::state::ChainSyncError),
}

/// One step of the producer-side chain-sync server pump in response
/// to a client message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerStep {
    /// Client said `Done`; session is over.
    Done,
    /// Server has produced a legal reply for the orchestrator to
    /// encode and send. State has already advanced.
    Reply(ServerReply),
}

/// Process one client-originated chain-sync message. Pure, total,
/// deterministic.
pub fn producer_chain_sync_serve(
    mut state: ProducerChainSyncServerState,
    in_msg: ChainSyncMessage,
    served: &dyn ServedHeaderLookup,
    version: ChainSyncVersion,
) -> Result<(ProducerChainSyncServerState, ServerStep), ProducerServerError> {
    // Validate grammar + advance the BLUE state machine. The
    // sender-agency for any input the producer-side server pump
    // accepts is Client (the peer).
    let (new_state, _output) =
        chain_sync_transition(state.state, ChainSyncAgency::Client, version, in_msg.clone())
            .map_err(ProducerServerError::Grammar)?;
    state.state = new_state;

    match in_msg {
        ChainSyncMessage::RequestNext => {
            // state is now CanAwait. Decide RollForward vs AwaitReply
            // by checking the served chain past our cursor.
            if let Some(proj) = served.next_after(state.last_announced.clone()) {
                let tip = tip_from_lookup(served);
                state.last_announced = Some((proj.slot, proj.hash.clone()));
                state.state = ChainSyncState::Idle;
                // CN-WIRE-08: wrap the bare header projection into the
                // ChainSync wire shape `[era_idx, tag24(header_cbor)]` via
                // the single tag-24 authority — no bare header on the wire.
                let header = compose_rollforward_header(proj.era, &proj.header_bytes);
                Ok((state, ServerStep::Reply(ServerReply::roll_forward(header, tip))))
            } else {
                // No fresh block. Park in MustReply; the orchestrator
                // will later call advance_tip when a block arrives.
                state.state = ChainSyncState::MustReply;
                Ok((state, ServerStep::Reply(ServerReply::await_reply())))
            }
        }
        ChainSyncMessage::FindIntersect { points } => {
            // state is now Intersect. Resolve immediately.
            let tip = tip_from_lookup(served);
            match served.intersect(&points) {
                Some((slot, hash)) => {
                    state.state = ChainSyncState::Idle;
                    Ok((
                        state,
                        ServerStep::Reply(ServerReply::intersect_found(
                            Point::Block { slot, hash },
                            tip,
                        )),
                    ))
                }
                None => {
                    state.state = ChainSyncState::Idle;
                    Ok((state, ServerStep::Reply(ServerReply::intersect_not_found(tip))))
                }
            }
        }
        ChainSyncMessage::Done => Ok((state, ServerStep::Done)),
        // Server-originated messages from a Client sender are grammar
        // violations the underlying state machine already rejected;
        // we should never reach here. The match is exhaustive for
        // safety.
        ChainSyncMessage::RollForward { .. }
        | ChainSyncMessage::RollBackward { .. }
        | ChainSyncMessage::AwaitReply
        | ChainSyncMessage::IntersectFound { .. }
        | ChainSyncMessage::IntersectNotFound { .. } => {
            // The grammar layer above already rejects this; we keep a
            // total match for completeness. If somehow reached, treat
            // as grammar violation.
            Err(ProducerServerError::Grammar(
                crate::chain_sync::state::ChainSyncError::IllegalTransition {
                    state: state.state,
                    message_tag: "server-originated message arrived as Client",
                    agency: "Client",
                },
            ))
        }
    }
}

/// Poll for a deferred RollForward. Returns `Some(reply)` when the
/// per-session state is in a server-agency wait AND the served chain
/// has a block past the cursor; `None` otherwise.
pub fn producer_chain_sync_advance_tip(
    mut state: ProducerChainSyncServerState,
    served: &dyn ServedHeaderLookup,
) -> Result<(ProducerChainSyncServerState, Option<ServerReply>), ProducerServerError> {
    match state.state {
        ChainSyncState::CanAwait | ChainSyncState::MustReply => {
            if let Some(proj) = served.next_after(state.last_announced.clone()) {
                let tip = tip_from_lookup(served);
                state.last_announced = Some((proj.slot, proj.hash.clone()));
                state.state = ChainSyncState::Idle;
                let header = compose_rollforward_header(proj.era, &proj.header_bytes);
                Ok((state, Some(ServerReply::roll_forward(header, tip))))
            } else {
                Ok((state, None))
            }
        }
        ChainSyncState::Idle | ChainSyncState::Intersect | ChainSyncState::Done => {
            Ok((state, None))
        }
    }
}

fn tip_from_lookup(served: &dyn ServedHeaderLookup) -> Tip {
    match served.tip() {
        Some((slot, hash, block_no)) => Tip {
            point: Point::Block { slot, hash },
            block_no,
        },
        None => Tip {
            point: Point::Origin,
            block_no: 0,
        },
    }
}

// Silence unused-import warnings in non-test builds; the import is
// used by the test mock and the reducer above.
#[allow(unused_imports)]
use ChainSyncOutput as _ChainSyncOutputUsed;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::codec::chain_sync::{decode_chain_sync_message, encode_chain_sync_message};

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

    // -----------------------------------------------------------------
    // PHASE4-N-G S3: reducer tests with corpus-backed ServedHeaderLookup
    // -----------------------------------------------------------------

    use std::collections::BTreeMap;

    use ade_codec::cbor::envelope::decode_block_envelope;
    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::vrf_cert::ActiveSlotsCoeff;
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary, Nonce};
    use ade_core::consensus::praos_state::PraosChainDepState;
    use ade_ledger::block_validity::{accepted_block_header_bytes, decode_block};
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::producer::{self_accept, ServedChainSnapshot};
    use ade_ledger::state::LedgerState;
    use ade_testkit::validity::ConwayValidityCorpus;
    use ade_types::{CardanoEra, EpochNo, Hash28, Hash32 as AdeHash32, SlotNo as AdeSlotNo};

    const EPOCH_576: EpochNo = EpochNo(576);
    const EPOCH_577_START: u64 = 163_900_800;
    const MAINNET_EPOCH_LENGTH: u64 = 432_000;

    fn schedule() -> EraSchedule {
        let start_576 = EPOCH_577_START - MAINNET_EPOCH_LENGTH;
        let eras = vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: AdeSlotNo(start_576),
            start_epoch: EPOCH_576,
            slot_length_ms: 1_000,
            epoch_length_slots: MAINNET_EPOCH_LENGTH as u32,
            safe_zone_slots: MAINNET_EPOCH_LENGTH as u32,
        }];
        EraSchedule::new(BootstrapAnchorHash(AdeHash32([0u8; 32])), 0, eras)
            .expect("schedule")
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
                    vrf_keyhash: AdeHash32(p.vrf_keyhash),
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
            let env = decode_block_envelope(&c.blocks[i]).expect("envelope decodes");
            env.block_end - env.block_start
        });
        let block_bytes: Vec<Vec<u8>> = idxs.into_iter().take(n).map(|i| c.blocks[i].clone()).collect();
        let schedule = schedule();
        let ledger = {
            let mut l = LedgerState::new(CardanoEra::Conway);
            l.epoch_state.epoch = EPOCH_576;
            l
        };
        let chain_dep = {
            let mut s = PraosChainDepState::empty();
            s.epoch_nonce = Nonce(AdeHash32(c.epoch_nonce));
            s.evolving_nonce = Nonce(AdeHash32(c.epoch_nonce));
            s
        };
        let mut snap = ServedChainSnapshot::new();
        for b in &block_bytes {
            let accepted = self_accept(b, &ledger, &chain_dep, &schedule, &view)
                .expect("corpus block self-accepts");
            snap = ade_ledger::producer::served_chain_admit(snap, accepted)
                .expect("admit accepts validator-cleared block");
        }
        (snap, block_bytes)
    }

    /// Test impl of `ServedHeaderLookup` over the real
    /// `ServedChainSnapshot` + canonical header projection. This is
    /// exactly the shape the GREEN adapter in ade_runtime will take
    /// (S5).
    struct SnapshotLookup<'a> {
        snap: &'a ServedChainSnapshot,
    }

    impl<'a> ServedHeaderLookup for SnapshotLookup<'a> {
        fn next_after(
            &self,
            cursor: Option<(SlotNo, Hash32)>,
        ) -> Option<HeaderProjection> {
            let mut iter: Vec<(SlotNo, Hash32, &[u8])> = self
                .snap
                .iter()
                .map(|(s, h, b)| (s, h.clone(), b))
                .collect();
            iter.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
            let cursor = cursor;
            let next = iter.into_iter().find(|(s, h, _)| match &cursor {
                Some((c_s, c_h)) => (*s, h) > (*c_s, c_h),
                None => true,
            })?;
            let decoded = decode_block(next.2).expect("snapshot block decodes");
            let proj = self_proj_header_via_canonical(next.2);
            Some(HeaderProjection {
                slot: next.0,
                hash: next.1,
                block_no: decoded.header_input.block_no.0,
                era: decoded.era,
                header_bytes: proj,
            })
        }
        fn intersect(&self, points: &[Point]) -> Option<(SlotNo, Hash32)> {
            for p in points {
                if let Point::Block { slot, hash } = p {
                    if self.snap.block_bytes(*slot, hash).is_some() {
                        return Some((*slot, hash.clone()));
                    }
                }
            }
            None
        }
        fn tip(&self) -> Option<(SlotNo, Hash32, u64)> {
            let mut last: Option<(SlotNo, Hash32, &[u8])> = None;
            for (s, h, b) in self.snap.iter() {
                last = Some((s, h.clone(), b));
            }
            let (s, h, b) = last?;
            let decoded = decode_block(b).expect("decode");
            Some((s, h, decoded.header_input.block_no.0))
        }
    }

    fn self_proj_header_via_canonical(block_bytes: &[u8]) -> Vec<u8> {
        // Mirror the AcceptedBlock path: we have raw bytes here, so
        // construct AcceptedBlock via the corpus path and project.
        let (c, view) = corpus_view();
        let schedule = schedule();
        let ledger = {
            let mut l = LedgerState::new(CardanoEra::Conway);
            l.epoch_state.epoch = EPOCH_576;
            l
        };
        let chain_dep = {
            let mut s = PraosChainDepState::empty();
            s.epoch_nonce = Nonce(AdeHash32(c.epoch_nonce));
            s.evolving_nonce = Nonce(AdeHash32(c.epoch_nonce));
            s
        };
        let accepted = self_accept(block_bytes, &ledger, &chain_dep, &schedule, &view)
            .expect("self_accept");
        accepted_block_header_bytes(&accepted)
            .expect("header projects")
            .to_vec()
    }

    fn empty_snapshot_lookup() -> ServedChainSnapshot {
        ServedChainSnapshot::new()
    }

    fn v() -> ChainSyncVersion {
        ChainSyncVersion::new(9)
    }

    #[test]
    fn producer_chain_sync_serve_request_next_idle_yields_roll_forward_when_served_has_block() {
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState::new();
        let (state2, step) =
            producer_chain_sync_serve(state, ChainSyncMessage::RequestNext, &look, v()).unwrap();
        assert_eq!(state2.state, ChainSyncState::Idle);
        assert!(state2.last_announced.is_some(), "cursor must advance");
        match step {
            ServerStep::Reply(reply) => match reply.into_message() {
                ChainSyncMessage::RollForward { header, .. } => {
                    assert!(!header.is_empty(), "RollForward must carry non-empty header bytes");
                }
                other => panic!("expected RollForward, got {other:?}"),
            },
            other => panic!("expected Reply, got {other:?}"),
        }
    }

    #[test]
    fn producer_chain_sync_serve_request_next_idle_yields_await_reply_when_served_empty() {
        let snap = empty_snapshot_lookup();
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState::new();
        let (state2, step) =
            producer_chain_sync_serve(state, ChainSyncMessage::RequestNext, &look, v()).unwrap();
        assert_eq!(state2.state, ChainSyncState::MustReply);
        assert!(state2.last_announced.is_none(), "cursor unchanged with no block");
        match step {
            ServerStep::Reply(reply) => match reply.into_message() {
                ChainSyncMessage::AwaitReply => {}
                other => panic!("expected AwaitReply, got {other:?}"),
            },
            other => panic!("expected Reply, got {other:?}"),
        }
    }

    #[test]
    fn producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found() {
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        // Known point: pick the only block we admitted.
        let (slot, hash, _bytes) = snap.iter().next().expect("non-empty");
        let known = Point::Block { slot, hash: hash.clone() };
        let state = ProducerChainSyncServerState::new();
        let (state2, step) = producer_chain_sync_serve(
            state,
            ChainSyncMessage::FindIntersect { points: vec![known.clone()] },
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, ChainSyncState::Idle);
        match step {
            ServerStep::Reply(reply) => match reply.into_message() {
                ChainSyncMessage::IntersectFound { point, .. } => {
                    assert_eq!(point, known);
                }
                other => panic!("expected IntersectFound, got {other:?}"),
            },
            other => panic!("expected Reply, got {other:?}"),
        }
    }

    #[test]
    fn producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found() {
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        let unknown = Point::Block {
            slot: SlotNo(1),
            hash: Hash32([0x00; 32]),
        };
        let state = ProducerChainSyncServerState::new();
        let (state2, step) = producer_chain_sync_serve(
            state,
            ChainSyncMessage::FindIntersect { points: vec![unknown] },
            &look,
            v(),
        )
        .unwrap();
        assert_eq!(state2.state, ChainSyncState::Idle);
        match step {
            ServerStep::Reply(reply) => match reply.into_message() {
                ChainSyncMessage::IntersectNotFound { .. } => {}
                other => panic!("expected IntersectNotFound, got {other:?}"),
            },
            other => panic!("expected Reply, got {other:?}"),
        }
    }

    #[test]
    fn producer_chain_sync_serve_done_terminates_session() {
        let snap = empty_snapshot_lookup();
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState::new();
        let (state2, step) =
            producer_chain_sync_serve(state, ChainSyncMessage::Done, &look, v()).unwrap();
        assert_eq!(state2.state, ChainSyncState::Done);
        assert_eq!(step, ServerStep::Done);
    }

    #[test]
    fn producer_chain_sync_serve_rejects_illegal_grammar_pair() {
        // Server message (RollForward) arriving from Client agency is
        // an immediate grammar reject at the BLUE state-machine layer.
        let snap = empty_snapshot_lookup();
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState::new();
        let err = producer_chain_sync_serve(
            state,
            ChainSyncMessage::RollForward { header: vec![0x40], tip: tip_sample() },
            &look,
            v(),
        )
        .expect_err("must reject grammar violation");
        match err {
            ProducerServerError::Grammar(_) => {}
        }
    }

    #[test]
    fn producer_chain_sync_advance_tip_idle_yields_none() {
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState::new(); // Idle
        let (state2, reply) = producer_chain_sync_advance_tip(state, &look).unwrap();
        assert_eq!(state2.state, ChainSyncState::Idle);
        assert!(reply.is_none(), "advance_tip from Idle yields None");
    }

    #[test]
    fn producer_chain_sync_advance_tip_can_await_yields_roll_forward_when_block_available() {
        // Drive the state machine into CanAwait without using serve
        // (which would auto-consume the block). Directly construct
        // the state.
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState {
            state: ChainSyncState::CanAwait,
            last_announced: None,
        };
        let (state2, reply) = producer_chain_sync_advance_tip(state, &look).unwrap();
        assert_eq!(state2.state, ChainSyncState::Idle);
        assert!(state2.last_announced.is_some());
        match reply.expect("Some RollForward").into_message() {
            ChainSyncMessage::RollForward { .. } => {}
            other => panic!("expected RollForward, got {other:?}"),
        }
    }

    #[test]
    fn producer_chain_sync_advance_tip_must_reply_yields_roll_forward_when_block_available() {
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState {
            state: ChainSyncState::MustReply,
            last_announced: None,
        };
        let (state2, reply) = producer_chain_sync_advance_tip(state, &look).unwrap();
        assert_eq!(state2.state, ChainSyncState::Idle);
        assert!(reply.is_some());
    }

    #[test]
    fn producer_chain_sync_advance_tip_can_await_yields_none_when_cursor_at_head() {
        let (snap, _blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        // Set cursor to the only admitted block; advance_tip should
        // find nothing further.
        let (slot, hash, _) = snap.iter().next().expect("non-empty");
        let state = ProducerChainSyncServerState {
            state: ChainSyncState::CanAwait,
            last_announced: Some((slot, hash.clone())),
        };
        let (state2, reply) = producer_chain_sync_advance_tip(state, &look).unwrap();
        assert_eq!(state2.state, ChainSyncState::CanAwait);
        assert!(reply.is_none(), "cursor at head -> no advance");
    }

    #[test]
    fn producer_chain_sync_serve_roll_forward_header_equals_accepted_block_header_bytes() {
        // DC-CONS-18 surface, strengthened by CN-WIRE-08 (PHASE4-N-X):
        // the RollForward bytes the reducer emits are the ChainSync wire
        // wrap `[era_idx, tag24(header_cbor)]` of the canonical header
        // projection. The inner, once decomposed, MUST equal
        // accepted_block_header_bytes byte-for-byte; the bare header is
        // never served, and the era index is the CONSENSUS index (Conway
        // = 6, i.e. storage discriminant 7 minus one).
        use crate::codec::chain_sync::decompose_rollforward_header;

        let (snap, blocks) = build_served_with_first_n_lightest(1);
        let look = SnapshotLookup { snap: &snap };
        let state = ProducerChainSyncServerState::new();
        let (_state2, step) =
            producer_chain_sync_serve(state, ChainSyncMessage::RequestNext, &look, v()).unwrap();
        let header_in_reply = match step {
            ServerStep::Reply(reply) => match reply.into_message() {
                ChainSyncMessage::RollForward { header, .. } => header,
                other => panic!("expected RollForward, got {other:?}"),
            },
            other => panic!("expected Reply, got {other:?}"),
        };
        let bare = self_proj_header_via_canonical(&blocks[0]);
        assert_ne!(header_in_reply, bare, "must NOT serve the bare header");
        let (era_idx, inner) =
            decompose_rollforward_header(&header_in_reply).expect("wire header decomposes");
        // The served corpus block is Conway (storage 7) → consensus 6.
        assert_eq!(era_idx, 6, "Conway ChainSync header era index must be 6");
        assert_eq!(inner, &bare[..], "decomposed header must equal accepted_block_header_bytes");
    }

    #[test]
    fn producer_chain_sync_serve_replays_byte_identical_over_corpus() {
        // Replay-equivalence: the same canonical inputs produce a
        // byte-identical outgoing frame sequence across two runs.
        let (snap, _blocks) = build_served_with_first_n_lightest(2);
        let look = SnapshotLookup { snap: &snap };
        let inputs = vec![
            ChainSyncMessage::RequestNext,
            ChainSyncMessage::RequestNext,
        ];
        let run = |inputs: &[ChainSyncMessage]| -> Vec<Vec<u8>> {
            let mut s = ProducerChainSyncServerState::new();
            let mut frames = Vec::new();
            for m in inputs {
                let (s2, step) =
                    producer_chain_sync_serve(s, m.clone(), &look, v()).unwrap();
                s = s2;
                match step {
                    ServerStep::Reply(reply) => {
                        frames.push(encode_chain_sync_message(&reply.into_message()));
                    }
                    ServerStep::Done => break,
                }
            }
            frames
        };
        let a = run(&inputs);
        let b = run(&inputs);
        assert_eq!(a, b, "replay must be byte-identical");
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
