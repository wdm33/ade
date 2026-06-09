// PHASE4-N-AI AI-S2 — shared detector (DC-NODE-23) + venue-split resolver
// (DC-NODE-24). Pure, total, venue-blind detector; venue-gated resolver.
// CE-AI-2.

use ade_ledger::receive::events::TipPoint;
use ade_node::node_sync::{
    classify_receive, resolve_disposition, CandidateSummary, ReceiveClass, ReceiveDisposition,
    VenueRole,
};
use ade_types::shelley::block::PrevHash;
use ade_types::{BlockNo, Hash32, SlotNo};

fn tip(block_no: u64, hash: u8) -> TipPoint {
    TipPoint {
        slot: SlotNo(block_no),
        hash: Hash32([hash; 32]),
        block_no,
    }
}

fn cand(block_no: u64, hash: u8, prev_hash: PrevHash) -> CandidateSummary {
    CandidateSummary {
        slot: SlotNo(block_no),
        block_no: BlockNo(block_no),
        hash: Hash32([hash; 32]),
        prev_hash,
    }
}

fn blk(h: u8) -> PrevHash {
    PrevHash::Block(Hash32([h; 32]))
}

// ---------- detector (DC-NODE-23) ----------

#[test]
fn classify_already_have_when_in_spine() {
    // in_spine overrides everything else (a known echo is never competing).
    let d = tip(10, 0xAA);
    // Even a candidate that is NOT a fresh extension:
    let c = cand(99, 0xCC, blk(0x99));
    assert_eq!(classify_receive(d, &c, true), ReceiveClass::AlreadyHave);
}

#[test]
fn classify_linear_extend_on_exact_parent_and_block_no() {
    let d = tip(10, 0xAA);
    let c = cand(11, 0xBB, blk(0xAA)); // prev == tip.hash, block_no == 11
    assert_eq!(classify_receive(d, &c, false), ReceiveClass::LinearExtend);
}

#[test]
fn classify_competing_on_nonmatching_parent() {
    let d = tip(10, 0xAA);
    let c = cand(11, 0xBB, blk(0x99)); // right height, wrong parent
    assert_eq!(classify_receive(d, &c, false), ReceiveClass::Competing);
}

#[test]
fn classify_competing_on_wrong_block_no() {
    let d = tip(10, 0xAA);
    let c = cand(12, 0xBB, blk(0xAA)); // right parent, wrong height (not +1)
    assert_eq!(classify_receive(d, &c, false), ReceiveClass::Competing);
}

#[test]
fn classify_competing_on_genesis_prev_hash() {
    let d = tip(10, 0xAA);
    let c = cand(11, 0xBB, PrevHash::Genesis); // genesis parent never extends a non-genesis tip
    assert_eq!(classify_receive(d, &c, false), ReceiveClass::Competing);
}

// ---------- resolver (DC-NODE-24) ----------

#[test]
fn resolve_singleproducer_competing_refuses() {
    assert_eq!(
        resolve_disposition(ReceiveClass::Competing, VenueRole::SingleProducer),
        ReceiveDisposition::RefuseSingleProducer
    );
}

#[test]
fn resolve_participant_competing_needs_fork_choice() {
    assert_eq!(
        resolve_disposition(ReceiveClass::Competing, VenueRole::Participant),
        ReceiveDisposition::NeedsForkChoice
    );
}

#[test]
fn resolve_participant_already_have_and_linear_extend_do_not_call_fork_choice() {
    // Participant does NOT mean "send everything to fork-choice": only Competing
    // becomes NeedsForkChoice; the fast path passes through.
    assert_eq!(
        resolve_disposition(ReceiveClass::AlreadyHave, VenueRole::Participant),
        ReceiveDisposition::AlreadyHave
    );
    assert_eq!(
        resolve_disposition(ReceiveClass::LinearExtend, VenueRole::Participant),
        ReceiveDisposition::LinearExtend
    );
}

#[test]
fn resolve_unknown_venue_fails_closed() {
    // Unknown -> RefuseSingleProducer as a fail-closed disposition, NOT an
    // inferred SingleProducer venue (OQ-5).
    assert_eq!(
        resolve_disposition(ReceiveClass::Competing, VenueRole::Unknown),
        ReceiveDisposition::RefuseSingleProducer
    );
}

#[test]
fn resolve_passthrough_already_have_and_linear_extend() {
    for venue in [
        VenueRole::Unknown,
        VenueRole::SingleProducer,
        VenueRole::Participant,
    ] {
        assert_eq!(
            resolve_disposition(ReceiveClass::AlreadyHave, venue),
            ReceiveDisposition::AlreadyHave
        );
        assert_eq!(
            resolve_disposition(ReceiveClass::LinearExtend, venue),
            ReceiveDisposition::LinearExtend
        );
    }
}
