// RVBP B1 — the reduced dispatch fires on the REAL live follower entry point.
//
// The security review (B1) found the reduced-boundary routing lived only on `apply_block_with_accounting`,
// which has ZERO production callers — the live follower (`block_validity` -> `apply_block_with_verdicts` ->
// `apply_shelley_era_block_{with_verdicts,classified}`) still ran the FULL boundary and fabricated a
// reward-only stub mark. This test drives the ACTUAL follower entry point across a Conway epoch boundary and
// proves the reduced dispatch fires there: a track_utxo=false crossing produces NO authority
// (snapshots/cert/gov = ReducedUnavailable), and a within-epoch apply keeps the authoritative seed (so the
// flip is boundary-gated, not unconditional). Uses a REAL Conway block (not synthetic CBOR), per the
// project's real-interop discipline.

use ade_ledger::epoch::EpochStakeSnapshots;
use ade_ledger::rules::apply_block_with_verdicts;
use ade_ledger::state::{slot_to_epoch, LedgerState};
use ade_types::{CardanoEra, EpochNo, SlotNo};

// The REAL Conway block captured from the live preprod peer (public chain data), reused from the ade_node
// admission fixture (the same fixture `reduced_advance.rs` proves its reduced projection against).
const RAW_CONWAY_BLOCK: &[u8] =
    include_bytes!("../../ade_node/tests/fixtures/raw_era_block_conway.cbor");

/// The inner Conway block bytes (envelope-unwrapped) + the block's slot.
fn inner_conway_block() -> (Vec<u8>, u64) {
    let env = ade_codec::cbor::envelope::decode_block_envelope(RAW_CONWAY_BLOCK).expect("envelope");
    assert_eq!(env.era, CardanoEra::Conway, "fixture is a Conway block");
    let inner = RAW_CONWAY_BLOCK[env.block_start..env.block_end].to_vec();
    let decoded = ade_codec::conway::decode_conway_block(&inner).expect("decode conway block");
    let slot = decoded.decoded().header.body.slot;
    (inner, slot)
}

/// THE B1 gate: the live follower path (`apply_block_with_verdicts`) crosses a Conway boundary in the REDUCED
/// plane — the post-state carries NO fabricated authority. If the reduced dispatch were still only on the
/// zero-caller helper, this crossing would run the full boundary and the snapshots/cert/gov would be
/// Authoritative (a fabricated stub mark) — this test would fail.
#[test]
fn live_follower_path_crosses_reduced_at_conway_boundary() {
    let (inner, slot) = inner_conway_block();
    let block_epoch = slot_to_epoch(SlotNo(slot)).expect("the fixture slot maps to an epoch");
    assert!(block_epoch.0 >= 1, "the block epoch must allow a one-epoch-behind seed");

    // A reduced follower: track_utxo=false Conway, positioned ONE epoch behind the block so the crossing fires.
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = false;
    state.epoch_state.epoch = EpochNo(block_epoch.0 - 1);
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));
    // The seed cert/gov are AUTHORITATIVE before the crossing (bootstrap seed) — we prove the boundary flips them.
    assert!(
        state.cert_state.as_authoritative().is_some(),
        "the seed cert is authoritative before crossing"
    );
    assert!(
        state.gov_state.as_authoritative().is_some(),
        "the seed gov is authoritative before crossing"
    );

    let result = apply_block_with_verdicts(&state, CardanoEra::Conway, &inner)
        .expect("the live follower path applies the boundary-crossing block");

    // The reduced dispatch fired on the REAL follower path — no fabricated authority survives the crossing.
    assert!(
        matches!(
            result.new_state.epoch_state.snapshots,
            EpochStakeSnapshots::ReducedUnavailable
        ),
        "reduced crossing produces NO mark/set/go on the live path (snapshots = ReducedUnavailable)"
    );
    assert!(
        result.new_state.cert_state.is_reduced(),
        "reduced crossing makes cert UNAVAILABLE BY TYPE on the live path (never a carried full CertState)"
    );
    assert!(
        result.new_state.gov_state.is_reduced(),
        "reduced crossing makes gov UNAVAILABLE BY TYPE on the live path (never a carried full ConwayGovState)"
    );
    assert_eq!(
        result.new_state.epoch_state.epoch, block_epoch,
        "the reduced boundary advanced the epoch to the block's epoch"
    );
}

/// Produce the post-crossing reduced continuation state via the REAL follower path (shared by the gate #3/#4
/// tests below). A track_utxo=false Conway follower one epoch behind the fixture block, crossed forward.
fn crossed_reduced_state() -> LedgerState {
    let (inner, slot) = inner_conway_block();
    let block_epoch = slot_to_epoch(SlotNo(slot)).expect("the fixture slot maps to an epoch");
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = false;
    state.epoch_state.epoch = EpochNo(block_epoch.0 - 1);
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));
    apply_block_with_verdicts(&state, CardanoEra::Conway, &inner)
        .expect("the live follower path applies the boundary-crossing block")
        .new_state
}

/// User gate #3: the reduced continuation is NOT serializable as a normal full-authority snapshot — the encoder
/// fails closed with `ReducedStateNotSerializable` rather than persist a fabricated cert/gov (so nothing a
/// reduced follower produced across the boundary can be rehydrated or fingerprinted as authority).
#[test]
fn reduced_continuation_is_not_serializable_as_authority() {
    let reduced = crossed_reduced_state();
    let r = ade_ledger::snapshot::encode_ledger_state(&reduced);
    assert!(
        matches!(
            r,
            Err(ade_ledger::snapshot::SnapshotEncodeError::ReducedStateNotSerializable)
        ),
        "a reduced continuation must fail closed at encode, never serialize a fabricated authority snapshot"
    );
}

/// User gate #4: a direct full-cert / full-governance access after the reduced crossing fails closed with
/// `FullBoundaryStateRequired` — never a fabricated or empty stand-in.
#[test]
fn reduced_continuation_full_access_fails_closed() {
    let reduced = crossed_reduced_state();
    let slot = reduced.epoch_state.slot;
    assert!(
        matches!(
            reduced.cert_state.require_full(slot),
            Err(ade_ledger::governance::GovernanceTerminal::FullBoundaryStateRequired { .. })
        ),
        "a direct cert require_full after a reduced crossing must fail closed with FullBoundaryStateRequired"
    );
    assert!(
        matches!(
            reduced.gov_state.require_full(slot),
            Err(ade_ledger::governance::GovernanceTerminal::FullBoundaryStateRequired { .. })
        ),
        "a direct gov require_full after a reduced crossing must fail closed with FullBoundaryStateRequired"
    );
}

/// The reduced flip is BOUNDARY-GATED, not unconditional: a within-epoch (no-boundary) apply on the same live
/// path keeps the authoritative seed cert/gov. This isolates the flip to the epoch crossing — a reduced
/// follower does not spuriously drop its seed authority on every block.
#[test]
fn live_follower_within_epoch_keeps_authoritative_seed() {
    let (inner, slot) = inner_conway_block();
    let block_epoch = slot_to_epoch(SlotNo(slot)).expect("the fixture slot maps to an epoch");

    // SAME epoch as the block -> detect_epoch_transition returns None -> no boundary.
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = false;
    state.epoch_state.epoch = block_epoch;
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));

    let result = apply_block_with_verdicts(&state, CardanoEra::Conway, &inner)
        .expect("the live follower path applies the within-epoch block");

    assert!(
        result.new_state.cert_state.as_authoritative().is_some(),
        "within-epoch reduced follower carries the authoritative seed cert forward (no spurious reduced flip)"
    );
    assert!(
        result.new_state.gov_state.as_authoritative().is_some(),
        "within-epoch reduced follower carries the authoritative seed gov forward"
    );
}
