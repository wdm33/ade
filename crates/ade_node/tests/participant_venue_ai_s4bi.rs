// PHASE4-N-AI AI-S4b-i — Participant venue declaration (inert). Proves the
// venue is an explicit closed declaration (Unknown/absent fails closed; both
// flags fail closed) and that declaring Participant is INERT before AI-S4b-ii
// wires the live routing (Participant reaches the same live fallback as
// Unknown for every existing venue consumer).

use ade_ledger::receive::events::TipPoint;
use ade_node::cli::{Cli, CliError};
use ade_node::node_sync::{
    single_producer_forge_decision, venue_policy, warm_start_forge_mode, ForgeMode, ForgeRefused,
    SingleProducerFenceReason, SingleProducerForgeDecision, VenueRole,
};
use ade_types::{Hash32, SlotNo};

fn base(extra: &[&str]) -> Vec<String> {
    let mut v = vec![
        "ade_node".to_string(),
        "--mode".to_string(),
        "node".to_string(),
        "--genesis-path".to_string(),
        "/g".to_string(),
    ];
    v.extend(extra.iter().map(|s| s.to_string()));
    v
}

fn tp(slot: u64, b: u8) -> TipPoint {
    TipPoint {
        slot: SlotNo(slot),
        hash: Hash32([b; 32]),
        block_no: slot,
    }
}

// ---------- CLI: explicit, closed, fail-closed ----------

#[test]
fn cli_participant_venue_flag_recognized() {
    let cli = Cli::parse_from(base(&["--participant-venue"])).expect("parse");
    assert!(cli.participant_venue);
    assert!(!cli.single_producer_venue);
}

#[test]
fn cli_both_venues_fails_closed() {
    let err = Cli::parse_from(base(&["--single-producer-venue", "--participant-venue"]))
        .expect_err("a venue cannot be both -- must fail closed");
    assert_eq!(err, CliError::ConflictingVenue);
}

#[test]
fn cli_no_venue_flag_is_unknown() {
    // No venue flag -> neither role declared (no default inference). The forge
    // activation's venue_role then stays its `Unknown` default.
    let cli = Cli::parse_from(base(&[])).expect("parse");
    assert!(!cli.participant_venue);
    assert!(!cli.single_producer_venue);
}

// ---------- Inertness: Participant reaches the same live fallback as Unknown ----------

#[test]
fn participant_venue_is_inert_before_live_wiring() {
    let tip = tp(10, 1);
    let extend = ForgeMode::SingleProducerExtendOwnDurableSpine {
        adopted_root: tip.clone(),
        current_tip: tip.clone(),
    };

    // venue_policy: Participant == Unknown (byte-equal; no venue echoed).
    for mode in [ForgeMode::InitialCatchupRequired, extend.clone()] {
        assert_eq!(
            venue_policy(VenueRole::Participant, &mode),
            venue_policy(VenueRole::Unknown, &mode),
        );
    }

    // warm_start_forge_mode: Participant == Unknown (both InitialCatchupRequired
    // since neither is SingleProducer).
    assert_eq!(
        warm_start_forge_mode(VenueRole::Participant, Some(&tip), Some(5)),
        warm_start_forge_mode(VenueRole::Unknown, Some(&tip), Some(5)),
    );

    // single_producer_forge_decision, initial gate: byte-equal (no venue echoed).
    assert_eq!(
        single_producer_forge_decision(
            &ForgeMode::InitialCatchupRequired,
            Some(tip.clone()),
            Some(tip.clone()),
            Some(tip.clone()),
            VenueRole::Participant,
            false,
            false,
        ),
        single_producer_forge_decision(
            &ForgeMode::InitialCatchupRequired,
            Some(tip.clone()),
            Some(tip.clone()),
            Some(tip.clone()),
            VenueRole::Unknown,
            false,
            false,
        ),
    );

    // Extend mode: both REFUSE with the same reason -- the decision is identical
    // (neither activates the extend); the observed venue is echoed for
    // diagnostics only, so the carriers differ in that field but not the verdict.
    let refuses_not_declared = |v: VenueRole| {
        matches!(
            single_producer_forge_decision(
                &extend,
                Some(tip.clone()),
                Some(tip.clone()),
                Some(tip.clone()),
                v,
                false,
                false,
            ),
            SingleProducerForgeDecision::Refuse(ForgeRefused::SingleProducerFenceViolation {
                reason: SingleProducerFenceReason::VenueNotDeclaredSingleProducer,
                ..
            })
        )
    };
    assert!(refuses_not_declared(VenueRole::Participant));
    assert!(refuses_not_declared(VenueRole::Unknown));
}
