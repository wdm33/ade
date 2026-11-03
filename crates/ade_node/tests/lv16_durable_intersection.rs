// SLICE LV-16 — a recovered node finds its common ancestor with the peer
// (DC-NODE-48 / T-REC-07). Over real contiguous Conway blocks: the offer is
// read from the validated durable ancestry; a peer that no longer has the
// durable tip intersects at its parent; the rollback that follows leaves the
// same durable chain and ledger state as an execution that never adopted the
// tip.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::events::SecurityParam;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::block_validity::decode_block;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::receive::ReceiveState;
use ade_ledger::rollback::intersection::{
    classify_intersect_reply, intersection_offsets, CandidateSource, ChainPoint, IntersectReply,
    IntersectionBound, IntersectionOutcome,
};
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{WalEntry, WalError, WalStore};
use ade_network::chain_sync::server::ServedHeaderLookup;
use ade_network::codec::chain_sync::Point as WirePoint;
use ade_node::node_lifecycle::{
    durable_intersection_offer, run_participant_sync, IntersectionAncestryReadError,
    IntersectionOfferError,
};
use ade_node::node_sync::{NodeBlockSource, NodeSyncItem};
use ade_runtime::admission::IntersectOffer;
use ade_runtime::chaindb::{ChainDb, ChainTip, InMemoryChainDb, StoredBlock};
use ade_runtime::forward_sync::ForwardSyncState;
use ade_runtime::network::served_chain_projection::ChainDbServedSource;
use ade_runtime::rollback::{PersistentSnapshotCache, SnapshotCadence};
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

const K: u64 = 2160;

fn conway_blocks(n: usize) -> Vec<StoredBlock> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/contiguous/conway");
    let mut names: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("corpus/contiguous/conway")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "cbor"))
        .collect();
    names.sort();
    names
        .into_iter()
        .take(n)
        .map(|p| {
            let bytes = std::fs::read(&p).unwrap();
            let d = decode_block(&bytes).expect("corpus block decodes");
            StoredBlock {
                hash: d.block_hash,
                slot: d.header_input.slot,
                bytes,
            }
        })
        .collect()
}

fn db_of(blocks: &[StoredBlock]) -> InMemoryChainDb {
    let db = InMemoryChainDb::new();
    for b in blocks {
        db.put_block(b).unwrap();
    }
    db
}

fn tip_of(b: &StoredBlock) -> ChainTip {
    ChainTip {
        hash: b.hash.clone(),
        slot: b.slot,
    }
}

fn candidates(offer: &IntersectOffer) -> &ade_ledger::rollback::intersection::IntersectionCandidates {
    match offer {
        IntersectOffer::Candidates(c) => c,
        IntersectOffer::Origin => panic!("expected candidates"),
    }
}

/// The peer's answer, produced by Ade's own served-chain intersect over the peer's
/// chain: the first offered point that is a stored block there.
fn peer_reply(peer: &InMemoryChainDb, offer: &IntersectOffer) -> IntersectReply {
    match ChainDbServedSource::new(peer).intersect(&offer.points()) {
        Some((slot, hash)) => IntersectReply::Found(ChainPoint { slot, hash }),
        None => IntersectReply::NotFound,
    }
}

#[test]
fn lv16_corpus_blocks_are_a_linked_chain() {
    let blocks = conway_blocks(400);
    for w in blocks.windows(2) {
        let child = decode_block(&w[1].bytes).unwrap();
        assert_eq!(
            child.prev_hash,
            ade_types::shelley::block::PrevHash::Block(w[0].hash.clone()),
            "the fixture must be contiguous for the ancestry walk to mean anything"
        );
    }
}

#[test]
fn lv16_offer_is_read_from_the_validated_durable_ancestry() {
    let blocks = conway_blocks(300);
    let db = db_of(&blocks);
    let tip = blocks.last().unwrap();

    // Short history: the oldest stored block is the deepest candidate.
    let offer = durable_intersection_offer(&db, Some(&tip_of(tip)), K).unwrap();
    let c = candidates(&offer);
    assert_eq!(c.bound(), IntersectionBound::RetainedHistory);
    let depths: Vec<u64> = intersection_offsets(K, 299);
    assert_eq!(c.candidates().len(), depths.len());
    for (cand, d) in c.candidates().iter().zip(depths.iter()) {
        assert_eq!(cand.source, CandidateSource::Durable { depth: *d });
        let expect = &blocks[blocks.len() - 1 - *d as usize];
        assert_eq!(cand.point.slot, expect.slot);
        assert_eq!(cand.point.hash, expect.hash);
    }
    assert_eq!(c.deepest().point.hash, blocks[0].hash);

    // With k smaller than the history, the deepest candidate is exactly k back.
    let offer = durable_intersection_offer(&db, Some(&tip_of(tip)), 50).unwrap();
    let c = candidates(&offer);
    assert_eq!(c.bound(), IntersectionBound::SecurityParam);
    assert_eq!(c.deepest().point.hash, blocks[blocks.len() - 1 - 50].hash);
    assert_eq!(c.deepest().source, CandidateSource::Durable { depth: 50 });
}

#[test]
fn lv16_a_block_whose_hash_is_not_its_lookup_hash_is_refused() {
    let blocks = conway_blocks(20);
    let db = db_of(&blocks[..19]);
    // A tip whose index entry points at bytes of a different block.
    let wrong = Hash32([0x5A; 32]);
    db.put_block(&StoredBlock {
        hash: wrong.clone(),
        slot: blocks[19].slot,
        bytes: blocks[18].bytes.clone(),
    })
    .unwrap();
    let r = durable_intersection_offer(
        &db,
        Some(&ChainTip {
            hash: wrong,
            slot: blocks[19].slot,
        }),
        K,
    );
    assert!(
        matches!(
            r,
            Err(IntersectionOfferError::Read(IntersectionAncestryReadError::HashMismatch { .. }))
        ),
        "the hash index is not trusted on its own: {r:?}"
    );
}

#[test]
fn lv16_a_tip_that_is_not_stored_is_offered_alone() {
    let blocks = conway_blocks(5);
    let db = db_of(&blocks[1..]);
    let anchor = tip_of(&blocks[0]);
    let offer = durable_intersection_offer(&InMemoryChainDb::new(), Some(&anchor), K).unwrap();
    assert_eq!(candidates(&offer).bound(), IntersectionBound::NoStoredAncestry);
    assert_eq!(
        offer.points(),
        vec![WirePoint::Block {
            slot: anchor.slot,
            hash: anchor.hash.clone()
        }]
    );
    // With stored blocks above it, the anchor is never offered (DC-NODE-48 (c)).
    let offer = durable_intersection_offer(&db, Some(&tip_of(&blocks[4])), K).unwrap();
    assert!(offer.points().iter().all(|p| match p {
        WirePoint::Block { hash, .. } => *hash != anchor.hash,
        WirePoint::Origin => false,
    }));
}

/// The killed store: Ade's durable tip is a block the peer no longer has; the peer
/// has its parent.
#[test]
fn lv16_killed_store_shape_intersects_at_the_parent() {
    let blocks = conway_blocks(300);
    let m = 250;
    let ade = db_of(&blocks[..=m + 1]);
    let peer = db_of(&blocks[..=m]);
    let offer = durable_intersection_offer(&ade, Some(&tip_of(&blocks[m + 1])), K).unwrap();
    match classify_intersect_reply(candidates(&offer), &peer_reply(&peer, &offer)).unwrap() {
        IntersectionOutcome::Found(found) => {
            assert_eq!(found.point.hash, blocks[m].hash);
            assert_eq!(found.source, CandidateSource::Durable { depth: 1 });
        }
        other => panic!("expected the parent, got {other:?}"),
    }

    // The pre-slice offer (the tip alone) finds nothing on the same peer.
    let tip_only = IntersectOffer::Candidates(
        ade_ledger::rollback::intersection::IntersectionCandidates::without_ancestry(ChainPoint {
            slot: blocks[m + 1].slot,
            hash: blocks[m + 1].hash.clone(),
        }),
    );
    assert_eq!(peer_reply(&peer, &tip_only), IntersectReply::NotFound);
}

#[test]
fn lv16_a_deeper_fork_intersects_at_the_newest_offered_common_point() {
    let blocks = conway_blocks(300);
    let ade = db_of(&blocks);
    let tip = blocks.len() - 1;
    // The peer forked 17 blocks below Ade's tip: offsets 13 and 21 bracket it.
    let peer = db_of(&blocks[..=tip - 17]);
    let offer = durable_intersection_offer(&ade, Some(&tip_of(&blocks[tip])), K).unwrap();
    match classify_intersect_reply(candidates(&offer), &peer_reply(&peer, &offer)).unwrap() {
        IntersectionOutcome::Found(found) => {
            assert_eq!(found.source, CandidateSource::Durable { depth: 21 });
            assert_eq!(found.point.hash, blocks[tip - 21].hash);
        }
        other => panic!("expected depth 21, got {other:?}"),
    }
}

#[test]
fn lv16_a_peer_sharing_no_offered_point_is_no_common_point() {
    let blocks = conway_blocks(300);
    let ade = db_of(&blocks[100..]);
    let peer = db_of(&blocks[..100]);
    let offer = durable_intersection_offer(&ade, Some(&tip_of(&blocks[299])), K).unwrap();
    assert_eq!(
        classify_intersect_reply(candidates(&offer), &peer_reply(&peer, &offer)),
        Ok(IntersectionOutcome::NoCommonPoint {
            bound: IntersectionBound::RetainedHistory,
            deepest: ChainPoint {
                slot: blocks[100].slot,
                hash: blocks[100].hash.clone()
            },
        })
    );
}

// ---------- the rollback after the intersection ----------

#[derive(Default)]
struct VecWal {
    entries: Vec<WalEntry>,
}
impl WalStore for VecWal {
    fn append(&mut self, entry: WalEntry) -> Result<(), WalError> {
        self.entries.push(entry);
        Ok(())
    }
    fn read_all(&self) -> Result<Vec<WalEntry>, WalError> {
        Ok(self.entries.clone())
    }
}

fn schedule() -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Conway,
            start_slot: SlotNo(0),
            start_epoch: EpochNo(0),
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("schedule")
}

fn chain_dep() -> PraosChainDepState {
    PraosChainDepState::genesis(Nonce(Hash32([0xCD; 32])))
}

#[tokio::test]
async fn lv16_rollback_to_the_found_parent_matches_a_run_that_never_saw_the_tip() {
    let blocks = conway_blocks(300);
    let m = 250;
    let parent = &blocks[m];

    // Ade: durable chain through the orphaned tip, ledger snapshot at the parent.
    let ade = db_of(&blocks[..=m + 1]);
    PersistentSnapshotCache::new(&ade)
        .capture(parent.slot, &LedgerState::new(CardanoEra::Conway), &chain_dep())
        .unwrap();
    // Reference: the same history without the tip.
    let reference = db_of(&blocks[..=m]);
    PersistentSnapshotCache::new(&reference)
        .capture(parent.slot, &LedgerState::new(CardanoEra::Conway), &chain_dep())
        .unwrap();

    // The intersection the peer (which never had the tip) gives.
    let offer = durable_intersection_offer(&ade, Some(&tip_of(&blocks[m + 1])), K).unwrap();
    let found = match classify_intersect_reply(candidates(&offer), &peer_reply(&reference, &offer))
        .unwrap()
    {
        IntersectionOutcome::Found(f) => f.point,
        other => panic!("{other:?}"),
    };

    // The peer's first reply after it: RollBackward(found).
    let mut fwd = ForwardSyncState::new(
        ReceiveState::new(LedgerState::new(CardanoEra::Conway), chain_dep()),
        Hash32([0xAB; 32]),
        SnapshotCadence::DEFAULT,
    );
    let mut wal = VecWal::default();
    let mut pending = false;
    let mut src = NodeBlockSource::in_memory_items(vec![NodeSyncItem::RollBack {
        peer: "peer-1".to_string(),
        point: WirePoint::Block {
            slot: found.slot,
            hash: found.hash.clone(),
        },
    }]);
    run_participant_sync(
        &mut src,
        &mut fwd,
        &ade,
        &mut wal,
        &schedule(),
        &ade_testkit::consensus::ledger_view_stub::LedgerViewStub::new(),
        &mut pending,
        SecurityParam(K),
        &mut None,
        &mut None,
        &BTreeMap::new(),
        None,
        &mut None,
        None,
        None,
    )
    .await
    .expect("the rollback to the found parent applies");

    // Tip, WAL and every stored block agree with the reference.
    assert_eq!(ade.tip().unwrap(), reference.tip().unwrap());
    assert!(wal
        .read_all()
        .unwrap()
        .iter()
        .any(|e| matches!(e, WalEntry::RollBack { .. })));
    for b in &blocks[..=m] {
        assert_eq!(
            ade.get_block_by_slot(b.slot).unwrap(),
            reference.get_block_by_slot(b.slot).unwrap()
        );
    }
    assert!(ade.get_block_by_slot(blocks[m + 1].slot).unwrap().is_none());
    // The ledger state is the reference's state at the same point.
    assert_eq!(
        fingerprint(&fwd.receive.ledger).combined,
        fingerprint(&LedgerState::new(CardanoEra::Conway)).combined
    );
    assert!(!pending);

    // After the rollback the next offer is the reference's own: same tip, same points.
    let after = durable_intersection_offer(&ade, ade.tip().unwrap().as_ref(), K).unwrap();
    let reference_offer =
        durable_intersection_offer(&reference, reference.tip().unwrap().as_ref(), K).unwrap();
    assert_eq!(after, reference_offer);
}
