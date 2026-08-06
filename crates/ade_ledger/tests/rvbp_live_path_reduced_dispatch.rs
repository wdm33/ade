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
use ade_ledger::state::{mainnet_shelley_schedule, LedgerState};
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
    let block_epoch = mainnet_shelley_schedule()
        .locate(SlotNo(slot))
        .expect("the fixture slot maps to an epoch")
        .epoch;
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

    let result = apply_block_with_verdicts(&state, CardanoEra::Conway, &inner, &mainnet_shelley_schedule())
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
    let block_epoch = mainnet_shelley_schedule()
        .locate(SlotNo(slot))
        .expect("the fixture slot maps to an epoch")
        .epoch;
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = false;
    state.epoch_state.epoch = EpochNo(block_epoch.0 - 1);
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));
    apply_block_with_verdicts(&state, CardanoEra::Conway, &inner, &mainnet_shelley_schedule())
        .expect("the live follower path applies the boundary-crossing block")
        .new_state
}

/// User gate #3: nothing a reduced follower produced across the boundary may be rehydrated or fingerprinted
/// AS AUTHORITY.
///
/// SLICE-P2 changed the MECHANISM, not the property. This gate previously asserted that encoding a reduced
/// continuation ERRORS (`ReducedStateNotSerializable`). Refusing to write bytes secures the property bluntly
/// — and also made a `track_utxo=false` follower permanently un-snapshottable after its first epoch
/// boundary, which was the preprod LIVE-2 blocker (a reduced follower is exactly what
/// `apply_reduced_epoch_boundary` produces, by design).
///
/// The property is now carried by the TYPE, the way `EpochStakeSnapshots` already carried it: a reduced
/// projection encodes to the `array(0)` marker and decodes back to `ReducedUnavailable`. Encoding alone
/// would be a relaxation, so this asserts the full guarantee — it encodes, it comes back REDUCED for cert
/// AND gov, and the authoritative forms are the only route to `Authoritative`.
#[test]
fn reduced_continuation_is_not_serializable_as_authority() {
    let reduced = crossed_reduced_state();
    assert!(reduced.cert_state.is_reduced() && reduced.gov_state.is_reduced());

    let bytes = ade_ledger::snapshot::encode_ledger_state(&reduced)
        .expect("SLICE-P2: a reduced continuation is now representable, so it must encode");
    let back = ade_ledger::snapshot::decode_ledger_state(&bytes)
        .expect("a reduced snapshot must decode");

    // THE guarantee: reduced in, reduced out. Never promoted to authority.
    assert!(
        back.cert_state.is_reduced(),
        "a reduced cert projection must NEVER rehydrate as Authoritative"
    );
    assert!(
        back.gov_state.is_reduced(),
        "a reduced gov projection must NEVER rehydrate as Authoritative"
    );
    assert!(back.cert_state.as_authoritative().is_none());
    assert!(back.gov_state.as_authoritative().is_none());
    // Still fail-closed for any reader that wants full authority.
    assert!(back.cert_state.require_full(back.epoch_state.slot).is_err());
    assert!(back.gov_state.require_full(back.epoch_state.slot).is_err());
    // Deterministic: re-encoding the decoded state reproduces the same bytes.
    assert_eq!(
        ade_ledger::snapshot::encode_ledger_state(&back).unwrap(),
        bytes,
        "reduced snapshot round-trip must be byte-identical"
    );
}

/// SLICE-P2 (CE-P2-2/CE-P2-3), the other direction: an AUTHORITATIVE state is untouched by the change.
/// Its bytes must be exactly what they were before the reduced marker existed — otherwise every existing
/// durable snapshot is invalidated — and it must never decode as reduced.
#[test]
fn authoritative_state_encoding_is_unchanged_and_never_decodes_reduced() {
    let mut full = LedgerState::new(CardanoEra::Conway);
    full.epoch_state.epoch = EpochNo(305);
    assert!(!full.cert_state.is_reduced() && !full.gov_state.is_reduced());

    let bytes = ade_ledger::snapshot::encode_ledger_state(&full).expect("a full state encodes");

    // THE byte-identity pin. Captured from the PRE-SLICE-P2 encoder for this exact state, so it proves
    // compatibility against the old code rather than against the new code's own round-trip. If adding the
    // reduced marker had shifted one byte of an authoritative encoding, every durable snapshot in
    // existence would silently fail to decode; this catches that at compile-test time instead of at a
    // customer's warm-start.
    const PRE_P2_FULL_CONWAY_305: &str = "89071b009fdf42f6e48000f441a04786a0a0a0a0a0a05387190131008382a0\
a082a0a082a0a00000a000584f9818182c1a00025ef51a0001000019400019044c1a001e84801a1dcd650012189682030a8203\
1903e882010502001a000f42401a1443fd0082010118961a00d59f801b00000002540be40001f60000f6f6";
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        PRE_P2_FULL_CONWAY_305.replace('\n', ""),
        "SLICE-P2 must not move a single byte of an AUTHORITATIVE encoding -- existing durable \
         snapshots decode with these exact bytes"
    );

    let back = ade_ledger::snapshot::decode_ledger_state(&bytes).expect("and decodes");
    assert!(
        !back.cert_state.is_reduced(),
        "an authoritative cert state must never decode as reduced"
    );
    assert!(
        !back.gov_state.is_reduced(),
        "an authoritative gov state must never decode as reduced"
    );
    assert_eq!(back, full, "full-authority round-trip is unchanged");

    // The marker must be ABSENT from an authoritative encoding: cert is written as a bstr (major type 2)
    // and gov as null (0xF6) here, so a bare 0x80 array(0) head must not appear where the reduced marker
    // would sit. Proven structurally by the reduced encoding differing from the full one.
    let reduced_bytes = {
        let mut r = full.clone();
        r.cert_state = ade_ledger::state::CertStateProjection::ReducedUnavailable;
        r.gov_state = ade_ledger::state::GovStateProjection::ReducedUnavailable;
        ade_ledger::snapshot::encode_ledger_state(&r).expect("reduced encodes")
    };
    assert_ne!(
        reduced_bytes, bytes,
        "reduced and authoritative encodings must be distinguishable"
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
    let block_epoch = mainnet_shelley_schedule()
        .locate(SlotNo(slot))
        .expect("the fixture slot maps to an epoch")
        .epoch;

    // SAME epoch as the block -> detect_epoch_transition returns None -> no boundary.
    let mut state = LedgerState::new(CardanoEra::Conway);
    state.track_utxo = false;
    state.epoch_state.epoch = block_epoch;
    state.epoch_state.slot = SlotNo(slot.saturating_sub(1));

    let result = apply_block_with_verdicts(&state, CardanoEra::Conway, &inner, &mainnet_shelley_schedule())
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
