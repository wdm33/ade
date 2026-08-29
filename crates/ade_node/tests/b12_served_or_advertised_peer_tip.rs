//! SLICE B12 (DC-NODE-47) — the followed-peer-tip signal reports the STRONGEST
//! evidence that the peer possesses a block, not only the weakest.
//!
//! DC-NODE-15's predicate (`forge_followed_tip_admission`) is NOT touched by this
//! slice and is not re-proven here; `phase4_n_ae_recover_serve_continuity_diag.rs`
//! owns it. What is proven here is the OPERAND: an advertisement is testimony, a
//! served-and-durably-admitted block is a demonstration, and the signal must
//! report whichever is stronger.
//!
//! The tuples below are the REAL ones. The frontier case is lifted verbatim from
//! `docs/evidence/run-stores/preprod-live2c/b12-census-classified.txt` — the run
//! that refused 762/762 — so the fix is measured against the failure rather than
//! against an invented example.

use ade_ledger::receive::events::TipPoint;
use ade_node::node_sync::{
    forge_followed_tip_admission, participant_forge_decision, single_producer_forge_decision,
    ForgeFollowedTipAdmission, ForgeMode, NodeBlockSource, NotCaughtUpReason,
    ParticipantForgeDecision, SingleProducerForgeDecision, VenueRole,
};
use ade_types::{Hash32, SlotNo};

fn h(byte: u8) -> Hash32 {
    Hash32([byte; 32])
}

fn hex32(s: &str) -> Hash32 {
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("hex");
    }
    Hash32(out)
}

// ---------------------------------------------------------------------------
// The census tuple, verbatim. One admitted ForgeTick out of 762 identical ones.
//
//   local_slot=130603493  local_block=5036049  local_hash=cc388271…
//   peer_slot=130603474   peer_block=5036048   peer_hash=ea08d817…
//   local_minus_peer=1    local_parent_is_peer_tip=yes
//   verdict=tip_mismatch  proceed_to_forge=false
// ---------------------------------------------------------------------------

fn census_local_tip() -> TipPoint {
    TipPoint {
        slot: SlotNo(130_603_493),
        hash: hex32("cc388271f12d6f9d00bc4857ce6853ec5411f3c6a221ed8b226dc752bd808baf"),
        block_no: 5_036_049,
    }
}

fn census_peer_advertised() -> TipPoint {
    TipPoint {
        slot: SlotNo(130_603_474),
        hash: hex32("ea08d8178a897e44dcccf98c08bcb4ada3c1b64f7b37bab396ed2b333300e832"),
        block_no: 5_036_048,
    }
}

/// A source carrying an explicit advertised tip, as the wire pump's `TipUpdate`
/// would leave it. Drives the SAME public methods production drives — there is no
/// test-only path into the served fact.
fn source_with(advertised: Option<TipPoint>) -> NodeBlockSource {
    NodeBlockSource::in_memory_with_followed_tip(Vec::new(), advertised)
}

// ===========================================================================
// CE-B12-1 — the exact live failure, now resolving.
// ===========================================================================

#[test]
fn the_census_frontier_tuple_resolves_caught_up_once_service_is_evidence() {
    let local = census_local_tip();
    let advertised = census_peer_advertised();

    // Pre-fix: the signal reports only what the peer SAID. This is the 762/762 line.
    let mut source = source_with(Some(advertised.clone()));
    assert_eq!(
        forge_followed_tip_admission(Some(local.clone()), source.followed_peer_tip_signal().tip()),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::TipMismatch,
        },
        "the advertisement alone reproduces the live refusal: local_minus_peer == +1"
    );
    assert_eq!(
        local.block_no - advertised.block_no,
        1,
        "non-vacuity: this IS the +1 tuple the census classified, not a rephrased one"
    );

    // The peer SERVED us that successor and Ade durably admitted it. It provably has it.
    source.record_served_tip(local.clone());

    assert_eq!(
        source.followed_peer_tip_signal().tip(),
        Some(local.clone()),
        "service is the stronger evidence and must dominate a one-block-stale advertisement"
    );
    assert_eq!(
        forge_followed_tip_admission(Some(local.clone()), source.followed_peer_tip_signal().tip()),
        ForgeFollowedTipAdmission::CaughtUp,
        "DC-NODE-15's predicate is unchanged; its operand became truthful"
    );

    // The advertisement half is still separately readable, and still says what it said.
    assert_eq!(
        source.followed_peer_tip_signal().advertised(),
        Some(advertised),
        "the two evidences stay separable — evidence consumers read this half (§4.6)"
    );
}

// ===========================================================================
// CE-B12-2 — THE CATCH-UP CONTROL. Non-vacuity for the entire slice.
//
// The danger the 2026-08-09 supersession named is a gate that stops enforcing
// catch-up. It cannot: during catch-up the chain-sync `tip` field carries the
// peer's REAL HEAD, so the advertisement LEADS and dominates.
//
// Measured, not assumed — the census counted `peer_announcements = 9798` against
// `peer_advances = 13` over that run's ~9,800-block catch-up. The advertisement
// sat still at the peer's own head while service climbed underneath it.
// ===========================================================================

#[test]
fn a_catch_up_gap_keeps_the_advertisement_dominant_and_the_gate_refusing() {
    // The shape the counters describe: ~9,800 blocks of service to climb, the
    // advertisement parked at the peer's head the whole way.
    let peer_head = TipPoint {
        slot: SlotNo(130_603_493),
        hash: h(0xEE),
        block_no: 5_036_049,
    };

    let mut source = source_with(Some(peer_head.clone()));

    for climbed in [1u64, 500, 4_000, 9_797, 9_799] {
        let local = TipPoint {
            slot: SlotNo(130_603_493 - (9_800 - climbed) * 20),
            hash: h(0x11),
            block_no: 5_036_049 - (9_800 - climbed),
        };
        source.record_served_tip(local.clone());

        assert_eq!(
            source.followed_peer_tip_signal().tip(),
            Some(peer_head.clone()),
            "at {climbed} blocks climbed the ADVERTISEMENT must still dominate — \
             service is below the peer's head"
        );
        assert_eq!(
            forge_followed_tip_admission(
                Some(local.clone()),
                source.followed_peer_tip_signal().tip()
            ),
            ForgeFollowedTipAdmission::NotCaughtUp {
                reason: NotCaughtUpReason::TipMismatch,
            },
            "a node {} blocks behind must NOT be admissible to forge",
            9_800 - climbed
        );
    }

    // And only when service reaches the advertised head does the gate open.
    source.record_served_tip(peer_head.clone());
    assert_eq!(
        forge_followed_tip_admission(
            Some(peer_head.clone()),
            source.followed_peer_tip_signal().tip()
        ),
        ForgeFollowedTipAdmission::CaughtUp,
        "arriving AT the peer's advertised head is what makes the forge admissible"
    );
}

// ===========================================================================
// CE-B12-3 — a self-forged tip is not service. This is what stops the served
// fact from licensing a second block on Ade's own unserved spine.
// ===========================================================================

#[test]
fn a_self_forged_tip_is_not_served_evidence_and_the_gate_refuses() {
    let peer_block = TipPoint {
        slot: SlotNo(1_000),
        hash: h(0xB0),
        block_no: 500,
    };
    let ade_own_block = TipPoint {
        slot: SlotNo(1_020),
        hash: h(0x0A),
        block_no: 501,
    };

    let mut source = source_with(Some(peer_block.clone()));
    // The peer served block 500 and Ade admitted it.
    source.record_served_tip(peer_block.clone());
    // Ade then forged 501 itself. NOTHING records that as served — the peer never
    // delivered it, so the peer has no demonstrated possession of it.
    assert_eq!(
        source.followed_peer_tip_signal().tip(),
        Some(peer_block),
        "Ade's own forged block must not appear as peer-served evidence"
    );
    assert_eq!(
        forge_followed_tip_admission(Some(ade_own_block), source.followed_peer_tip_signal().tip()),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::TipMismatch,
        },
        "with a self-forged tip the gate still refuses — the peer has not been shown to hold it"
    );
}

// ===========================================================================
// CE-B12-4 — a rollback clears the served fact.
// ===========================================================================

#[test]
fn a_rollback_clears_the_served_fact_and_the_signal_falls_back_to_the_advertisement() {
    let advertised = TipPoint {
        slot: SlotNo(900),
        hash: h(0xB0),
        block_no: 450,
    };
    let served = TipPoint {
        slot: SlotNo(1_000),
        hash: h(0xC0),
        block_no: 451,
    };

    let mut source = source_with(Some(advertised.clone()));
    source.record_served_tip(served.clone());
    assert_eq!(
        source.followed_peer_tip_signal().tip(),
        Some(served),
        "before the rollback, service dominates"
    );

    source.clear_served_tip();
    assert_eq!(
        source.followed_peer_tip_signal().served(),
        None,
        "the rollback clears the served fact outright"
    );
    assert_eq!(
        source.followed_peer_tip_signal().tip(),
        Some(advertised),
        "after a rollback the signal falls back to the advertisement — it must never keep \
         naming a block that may no longer be on the selected chain"
    );
}

// ===========================================================================
// CE-B12-5 — service is recorded at the ADMIT, so a source that has admitted
// nothing has no served evidence to offer.
// ===========================================================================

#[test]
fn a_source_that_has_admitted_nothing_offers_no_served_evidence() {
    let advertised = TipPoint {
        slot: SlotNo(900),
        hash: h(0xB0),
        block_no: 450,
    };
    let source = source_with(Some(advertised.clone()));
    assert_eq!(
        source.followed_peer_tip_signal().served(),
        None,
        "receipt is not possession evidence Ade may act on; only a successful durable admit is"
    );
    assert_eq!(
        source.followed_peer_tip_signal().tip(),
        Some(advertised),
        "with no served fact the signal is exactly the pre-slice signal"
    );

    let empty = source_with(None);
    assert_eq!(
        empty.followed_peer_tip_signal().tip(),
        None,
        "neither evidence present ⇒ None ⇒ NoFollowedPeerTip, unchanged"
    );
}

// ===========================================================================
// CE-B12-9 — the tie-break is CONSERVATIVE. Same height, different hash, is a
// fork the AO owns; the advertisement wins and the gate refuses.
// ===========================================================================

#[test]
fn a_tie_at_the_same_height_resolves_to_the_advertisement_and_refuses() {
    let advertised = TipPoint {
        slot: SlotNo(1_000),
        hash: h(0xB0),
        block_no: 500,
    };
    let served_fork = TipPoint {
        slot: SlotNo(1_000),
        hash: h(0xC0), // same height, DIFFERENT block
        block_no: 500,
    };

    let mut source = source_with(Some(advertised.clone()));
    source.record_served_tip(served_fork.clone());

    assert_eq!(
        source.followed_peer_tip_signal().tip(),
        Some(advertised),
        "STRICTLY greater, not >=: at equal block_no the advertisement wins"
    );
    assert_eq!(
        forge_followed_tip_admission(Some(served_fork), source.followed_peer_tip_signal().tip()),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::TipMismatch,
        },
        "the tips then disagree and TipMismatch is PRESERVED as the diagnostic — the tie must \
         not collapse to NoFollowedPeerTip and lose which way they differed"
    );
}

// ===========================================================================
// CE-B12-8 — the `--participant-venue` discriminator was never available.
//
// The LIVE-2c handoff records "cheapest discriminator first": declare the venue
// and see whether the Participant latch fires. It cannot. All three venue routes
// consult the SAME DC-NODE-15 predicate for the initial catch-up, on operands
// bound once before the venue branch. This turns that code read into a test.
// ===========================================================================

#[test]
fn all_three_venue_routes_refuse_on_the_pre_fix_census_tuple() {
    let local = census_local_tip();
    let advertised = census_peer_advertised();

    // The pre-fix signal: advertisement only.
    let signal = Some(advertised.clone());

    // (1) Unknown — the default route calls the gate directly.
    assert_eq!(
        forge_followed_tip_admission(Some(local.clone()), signal.clone()),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::TipMismatch,
        },
        "Unknown venue: the gate refuses"
    );

    // (2) Participant — defers to the SAME gate until the first caught-up instant,
    //     so the latch cannot fire while the gate refuses.
    let decision = participant_forge_decision(
        &ForgeMode::InitialCatchupRequired,
        Some(local.clone()),
        signal.clone(),
        VenueRole::Participant,
        false,
        false,
        false,
    );
    assert!(
        matches!(decision, ParticipantForgeDecision::UseInitialCatchupGate),
        "Participant defers to the initial catch-up gate — it does not route around it"
    );
    assert_eq!(
        forge_followed_tip_admission(Some(local.clone()), signal.clone()),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::TipMismatch,
        },
        "…and that gate refuses, so ParticipantExtendOnSelectedHead is never latched"
    );

    // (3) SingleProducer — same deferral, same gate.
    let decision = single_producer_forge_decision(
        &ForgeMode::InitialCatchupRequired,
        Some(local.clone()),
        signal.clone(),
        signal.clone(),
        VenueRole::SingleProducer,
        false,
        false,
    );
    assert!(
        matches!(decision, SingleProducerForgeDecision::UseInitialCatchupGate),
        "SingleProducer defers to the same initial catch-up gate"
    );

    // And the fix clears all three at once, because there is only ever one operand.
    let mut source = source_with(Some(advertised));
    source.record_served_tip(local.clone());
    assert_eq!(
        forge_followed_tip_admission(Some(local), source.followed_peer_tip_signal().tip()),
        ForgeFollowedTipAdmission::CaughtUp,
        "one signal, one meaning: the truthful operand opens every route that consults it"
    );
}
