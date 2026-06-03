// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-V S3 — first in-process forge → self_accept drive.
//!
//! `run_real_forge` is driven by a **consistent eligible-leader** tick:
//! the operator's pool is registered in `pool_distr_view` with the exact
//! recipes Conway header validation binds — `pool_id =
//! blake2b_224(cold_vk)` (the issuer-pool derivation at
//! `header_input.rs`), `vrf_keyhash = blake2b_256(vrf_vk)` (the VRF
//! keyhash binding at `header_validate.rs` step 5) — at positive stake
//! with ASC 1/1, and the eta0 nonce is shared between the forge VRF input
//! and validation. S2's envelope wrap means the forged bytes now decode,
//! so the tick reaches `self_accept`'s header validation.
//!
//! ## PHASE4-N-W S2: producer Praos VRF now matches the validator
//!
//! With the producer-side leader-eligibility VRF migrated to Praos
//! (CN-FORGE-04), the aligned tick now reaches `ForgeSucceeded`. The
//! producer builds its leader VRF proof over the **Praos** input
//! `praos_vrf_input(slot, eta0) = blake2b256(slot_be8 ‖ eta0_32)` (sourced
//! from `LeaderScheduleAnswer.expected_vrf_input`, the single
//! `leader_vrf_input` authority) and evaluates eligibility from
//! `praos_leader_value(output)`. `validate_and_apply_header` then verifies
//! the single combined proof over the **same** `praos_vrf_input` via
//! `verify_praos_vrf` (`header_validate.rs` `HeaderVrf::Praos` branch), so
//! `self_accept` accepts the forged Conway block — producer self-accept ≡
//! receive-path verification (R2). This is the first in-process forge →
//! self-accept success.
//!
//! Before N-W the producer built the **TPraos** role-tagged input
//! `slot ‖ eta0 ‖ 0x4C`, which the Praos validator rejected at the VRF
//! proof step; N-W removed that producer/validator asymmetry.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_core::consensus::vrf_cert::{leader_vrf_input, ActiveSlotsCoeff};
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_crypto::vrf::VrfVerificationKey;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::state::LedgerState;
use ade_runtime::producer::coordinator::CoordinatorEvent;
use ade_runtime::producer::producer_log::ForgeFailureReason;
use ade_runtime::producer::producer_shell::ProducerShell;
use ade_types::shelley::block::{OperationalCert, PrevHash, ProtocolVersion};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

use ade_network::block_fetch::server::{
    producer_block_fetch_serve, BlockFetchServerStep, ProducerBlockFetchServerState,
};
use ade_network::codec::block_fetch::{decompose_blockfetch_block, BlockFetchMessage, Point, Range};
use ade_network::codec::version::BlockFetchVersion;
use ade_node::produce_mode::{run_real_forge, ForgeRequestContext};
use ade_runtime::producer::self_accepted_handoff::SelfAcceptedHandoff;
use ade_runtime::producer::served_chain_handle::{ServedChainHandle, ServedChainView};
use ade_runtime::producer::served_chain_lookups::ServedChainLookups;

// =========================================================================
// Synthetic-corpus helpers (mirror forge_handler_variants::synth_shell)
// =========================================================================

fn synth_shell(cold_seed: u8, vrf_seed: u8, kes_seed: u8) -> ProducerShell {
    use ade_runtime::producer::signing::{ColdSigningKey, VrfSigningKey};
    use cardano_crypto::vrf::VrfDraft03;

    let cold_bytes = [cold_seed; 32];
    let cold = ColdSigningKey::from_bytes_zeroizing(&cold_bytes).unwrap();

    let (vrf_sk_bytes, vrf_vk_bytes) = VrfDraft03::keypair_from_seed(&[vrf_seed; 32]);
    let vrf = VrfSigningKey::from_bytes_zeroizing(&vrf_sk_bytes).unwrap();

    let kes_seed_bytes = [kes_seed; 32];
    let kes = ade_runtime::producer::signing::KesSecret::from_seed_at_period(&kes_seed_bytes, 0)
        .unwrap();

    use ade_crypto::kes_sum::{KesAlgorithm, Sum6Kes};
    let kes_sk_raw = Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed_bytes).unwrap();
    let hot_vkey = Sum6Kes::derive_verification_key(&kes_sk_raw);

    use ed25519_dalek::{Signer, SigningKey as DalekSk};
    let cold_dalek = DalekSk::from_bytes(&cold_bytes);
    let mut signable = Vec::with_capacity(48);
    signable.extend_from_slice(&hot_vkey);
    signable.extend_from_slice(&0u64.to_be_bytes());
    signable.extend_from_slice(&0u64.to_be_bytes());
    let sigma = cold_dalek.sign(&signable);

    let opcert = OperationalCert {
        hot_vkey: hot_vkey.to_vec(),
        sequence_number: 0,
        kes_period: 0,
        sigma: sigma.to_bytes().to_vec(),
    };
    let _vrf_vk = VrfVerificationKey(vrf_vk_bytes);

    ProducerShell::init(kes, vrf, cold, opcert).expect("shell init")
}

fn era_schedule(epoch: EpochNo) -> EraSchedule {
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            era: CardanoEra::Conway,
            start_slot: ade_types::SlotNo(0),
            start_epoch: epoch,
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("era schedule")
}

/// Build the consistent eligible-leader `ForgeRequestContext` plus its
/// owned backing values. The returned tuple keeps every borrowed input
/// alive for the duration of the `run_real_forge` call.
struct EligibleFixture {
    eta0_holder: PraosChainDepState,
    vrf_vk: VrfVerificationKey,
    leader_answer: LeaderScheduleAnswer,
    pparams: ProtocolParameters,
    base_state: LedgerState,
    era_schedule: EraSchedule,
    pool_distr_view: PoolDistrView,
}

impl EligibleFixture {
    fn build(shell: &ProducerShell, slot: u64, epoch: EpochNo) -> Self {
        let eta0 = Nonce(Hash32([0xCD; 32]));
        let vrf_vk = shell.vrf_verification_key();
        let cold_vk = shell.cold_vk();

        // pool_id = blake2b_224(cold_vk): the issuer pool the header
        // validator derives from the issuer vkey (header_input.rs).
        let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&cold_vk.0);
        // vrf_keyhash = blake2b_256(vrf_vk): the recipe header validation
        // binds at step 5 (header_validate.rs VRF keyhash binding).
        let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&vrf_vk.0);

        // total = 1 + ASC 1/1 ⇒ always eligible regardless of VRF output.
        let total: u64 = 1;
        let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
        pools.insert(
            pool_id.clone(),
            PoolEntry {
                active_stake: total,
                vrf_keyhash,
            },
        );
        let pool_distr_view =
            PoolDistrView::new(epoch, total, ActiveSlotsCoeff { numer: 1, denom: 1 }, pools);

        let leader_answer = LeaderScheduleAnswer {
            slot: ade_types::SlotNo(slot),
            pool: pool_id,
            epoch,
            expected_vrf_input: leader_vrf_input(
                CardanoEra::Conway,
                ade_types::SlotNo(slot),
                &eta0,
            ),
            stake_fraction: (1, 1),
            asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
        };

        // The same nonce feeds both the forge VRF input and validation.
        let mut eta0_holder = PraosChainDepState::empty();
        eta0_holder.epoch_nonce = eta0;

        let mut base_state = LedgerState::new(CardanoEra::Conway);
        base_state.epoch_state.epoch = epoch;

        EligibleFixture {
            eta0_holder,
            vrf_vk,
            leader_answer,
            pparams: ProtocolParameters::default(),
            base_state,
            era_schedule: era_schedule(epoch),
            pool_distr_view,
        }
    }

    fn ctx(&self) -> ForgeRequestContext<'_> {
        ForgeRequestContext {
            eta0: &self.eta0_holder.epoch_nonce,
            vrf_vk: &self.vrf_vk,
            leader_schedule_answer: &self.leader_answer,
            pparams: &self.pparams,
            base_state: &self.base_state,
            chain_dep_state: &self.eta0_holder,
            era_schedule: &self.era_schedule,
            pool_distr_view: &self.pool_distr_view,
            block_number: BlockNo(0),
            prev_hash: PrevHash::Genesis,
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
            prev_opcert_counter: None,
        }
    }
}

// =========================================================================
// CE-V-6 — OQ4 honest fallback: ForgeFailed with the next blocker pinned
// =========================================================================

/// PHASE4-N-W S2: the producer-side Praos VRF construction now matches the
/// validator (CN-FORGE-04). The consistent eligible-leader tick forges a
/// Conway/Praos block whose single combined VRF certificate the SAME
/// receive-path `verify_praos_vrf` accepts, so `self_accept` reaches
/// `ForgeSucceeded` — the first in-process forge → self-accept success
/// (CE-W-3, CE-W-4, CE-W-5; closes the deferred CN-FORGE-01 "ForgeSucceeded
/// reachable" strengthening).
#[test]
fn forge_to_self_accept_succeeds() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();

    let (event, _handoff) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    match event {
        CoordinatorEvent::ForgeSucceeded { slot: s, .. } => {
            assert_eq!(s, slot, "ForgeSucceeded must preserve the slot");
        }
        CoordinatorEvent::ForgeFailed { slot: _, reason } => {
            panic!(
                "expected ForgeSucceeded — the producer Praos VRF now matches the \
                 validator's verify_praos_vrf; got ForgeFailed {{ {:?} }}. If a \
                 deeper producer/validator asymmetry has surfaced, pin it as a \
                 follow-on cluster (N-X) per the CE-W-5 contingency.",
                reason
            );
        }
        CoordinatorEvent::ForgeNotLeader { .. } => {
            panic!("expected the eligible path; got ForgeNotLeader (setup bug)");
        }
        other => panic!("expected ForgeSucceeded, got {:?}", other),
    }
}

// =========================================================================
// PHASE4-N-F-G-B S1 — self-accepted-artifact handoff surfacing + carrier
// =========================================================================

/// PHASE4-N-F-G-B S1: the forge surfaces the BLUE self-accepted `AcceptedBlock`
/// token (the new sibling return component) EXACTLY on the self-accept success
/// path — `Some` alongside `ForgeSucceeded`, and its bytes are the forged block
/// verbatim. (The `None`-on-failure half is proven by the not-leader / failed
/// tests in `forge_handler_variants.rs`.) The closed `CoordinatorEvent` surface
/// is unchanged: the token rides the return tuple, not a `ForgeSucceeded` field.
#[test]
fn forge_surfaces_accepted_block_only_on_self_accept() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();

    let (event, handoff) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    let artifact_bytes = match &event {
        CoordinatorEvent::ForgeSucceeded { artifact, .. } => artifact.bytes.clone(),
        other => panic!("expected ForgeSucceeded (eligible fixture), got {other:?}"),
    };
    let surfaced = handoff.expect("ForgeSucceeded must surface a self-accepted token");
    assert_eq!(
        surfaced.as_bytes(),
        artifact_bytes.as_slice(),
        "the surfaced AcceptedBlock must be the forged block verbatim — the \
         ORIGINAL self-accept token, never re-derived from artifact.bytes"
    );
}

/// PHASE4-N-F-G-B S1: a `SelfAcceptedHandoff` is constructible end-to-end from a
/// real forge whose self-accept passes — and ONLY from that BLUE `AcceptedBlock`.
/// We take the token surfaced by `run_real_forge`, wrap it via the sole
/// constructor, and confirm the carrier holds the forged block verbatim (and
/// `into_accepted` yields the same BLUE token back for S2's `push_atomic`).
/// There is no raw-bytes / artifact / event constructor — that type-level fence
/// is asserted in `ade_runtime`'s carrier tests.
#[test]
fn handoff_carrier_constructs_only_from_self_accepted_forge() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();

    let (event, handoff) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    let forged_bytes = match event {
        CoordinatorEvent::ForgeSucceeded { artifact, .. } => artifact.bytes,
        other => panic!("expected ForgeSucceeded (eligible fixture), got {other:?}"),
    };
    let accepted = handoff.expect("eligible forge self-accepts and surfaces a token");

    // Wrap via the SOLE constructor — it takes the BLUE `AcceptedBlock`.
    let carrier = SelfAcceptedHandoff::from_self_accepted(accepted);
    assert_eq!(
        carrier.accepted().as_bytes(),
        forged_bytes.as_slice(),
        "the carrier must hold the original self-accepted block verbatim"
    );
    // The consuming accessor yields the same BLUE token back (S2 push_atomic input).
    let back = carrier.into_accepted();
    assert_eq!(back.as_bytes(), forged_bytes.as_slice());
}

// =========================================================================
// PHASE4-N-F-G-B S2 — sibling served-chain admit (handoff → push_atomic)
// =========================================================================

/// PHASE4-N-F-G-B S2: the node-spine sibling admit feeds the served chain ONLY
/// by consuming a self-accepted handoff via `into_accepted()` → `push_atomic`.
/// Forge a real handoff (the EligibleFixture self-accepts), admit it, and
/// confirm it is present in the served snapshot — admitted via `push_atomic`.
#[test]
fn sibling_serve_admits_via_push_atomic_only() {
    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, 1, EpochNo(0));
    let ctx = fixture.ctx();
    let (_event, accepted) = run_real_forge(1, /* kes_period = */ 0, &ctx, &mut shell);
    let accepted = accepted.expect("eligible forge surfaces a self-accepted token");
    // Wrap the surfaced BLUE token in the S1 carrier (as forge_one_from_recovered
    // does on the node spine), then admit ONLY via into_accepted() -> push_atomic.
    let handoff = SelfAcceptedHandoff::from_self_accepted(accepted);

    let (handle, view) = ServedChainHandle::new();
    handle
        .push_atomic(handoff.into_accepted())
        .expect("served-chain admit accepts the self-accepted block");
    assert_eq!(
        view.borrow().len(),
        1,
        "the self-accepted handoff is admitted to the served snapshot via push_atomic"
    );
}

/// PHASE4-N-F-G-B S2: the same self-accepted handoff admits to a byte-identical
/// served snapshot (`push_atomic` is deterministic in the admitted block).
#[test]
fn serve_sibling_admission_replay_byte_identical() {
    let mut shell1 = synth_shell(0x77, 0x88, 0x99);
    let f1 = EligibleFixture::build(&shell1, 1, EpochNo(0));
    let c1 = f1.ctx();
    let (_e1, a1) = run_real_forge(1, 0, &c1, &mut shell1);

    let mut shell2 = synth_shell(0x77, 0x88, 0x99);
    let f2 = EligibleFixture::build(&shell2, 1, EpochNo(0));
    let c2 = f2.ctx();
    let (_e2, a2) = run_real_forge(1, 0, &c2, &mut shell2);

    let h1 = SelfAcceptedHandoff::from_self_accepted(a1.expect("a1 self-accepted"));
    let h2 = SelfAcceptedHandoff::from_self_accepted(a2.expect("a2 self-accepted"));

    let (ha, va) = ServedChainHandle::new();
    ha.push_atomic(h1.into_accepted()).expect("admit a");
    let (hb, vb) = ServedChainHandle::new();
    hb.push_atomic(h2.into_accepted()).expect("admit b");
    assert_eq!(
        va.borrow().fingerprint(),
        vb.borrow().fingerprint(),
        "identical self-accepted handoffs admit to a byte-identical served snapshot"
    );
}

/// PHASE4-N-F-G-B S2: the node-spine admit obtains the `AcceptedBlock` fed to
/// `push_atomic` ONLY by consuming a `SelfAcceptedHandoff` via `into_accepted()`
/// — there is no other handoff → `AcceptedBlock` path (the S1 carrier fence).
/// The closure type-checks only because `into_accepted()` yields exactly the
/// `AcceptedBlock` that `push_atomic` accepts — a compile-time pin of the sole
/// admit path (no raw-bytes / direct-`AcceptedBlock` feed exists).
#[test]
fn serve_sibling_push_atomic_fed_only_by_into_accepted() {
    let _admit =
        |handle: &ServedChainHandle, h: SelfAcceptedHandoff| handle.push_atomic(h.into_accepted());
}

// =========================================================================
// CE-W-6 (PHASE4-N-W S1) — TPraos producer-forge fail-closed
// =========================================================================

/// A producer-forge request whose era schedule locates a non-Praos era
/// (Shelley = TPraos) must fail closed with the structured
/// `ForgeFailureReason::UnsupportedProducerEra` — the sketch's
/// `UnsupportedEra::ProducerForge` policy (I6 / N5). The guard fires before
/// any VRF/KES key use. TPraos *validation* is unaffected (this slice does
/// not touch `vrf_input` / `VrfRole`).
#[test]
fn tpraos_producer_forge_fails_closed_with_unsupported_era() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);

    // A Shelley (TPraos) era schedule located at the forge slot.
    let shelley_schedule = EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            era: CardanoEra::Shelley,
            start_slot: ade_types::SlotNo(0),
            start_epoch: epoch,
            slot_length_ms: 1_000,
            epoch_length_slots: 432_000,
            safe_zone_slots: 129_600,
        }],
    )
    .expect("shelley era schedule");

    let ctx = ForgeRequestContext {
        eta0: &fixture.eta0_holder.epoch_nonce,
        vrf_vk: &fixture.vrf_vk,
        leader_schedule_answer: &fixture.leader_answer,
        pparams: &fixture.pparams,
        base_state: &fixture.base_state,
        chain_dep_state: &fixture.eta0_holder,
        era_schedule: &shelley_schedule,
        pool_distr_view: &fixture.pool_distr_view,
        block_number: BlockNo(0),
        prev_hash: PrevHash::Genesis,
        protocol_version: ProtocolVersion { major: 9, minor: 0 },
        prev_opcert_counter: None,
    };

    let (event, handoff) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    // PHASE4-N-F-G-B S1: a TPraos fail-closed (UnsupportedProducerEra) outcome
    // surfaces no self-accepted handoff.
    assert!(
        handoff.is_none(),
        "UnsupportedProducerEra fail-closed must surface no handoff"
    );
    match event {
        CoordinatorEvent::ForgeFailed { slot: s, reason } => {
            assert_eq!(s, slot, "ForgeFailed must preserve the slot");
            assert_eq!(
                reason,
                ForgeFailureReason::UnsupportedProducerEra,
                "a non-Praos (Shelley) producer-forge request must fail closed \
                 with UnsupportedProducerEra, not attempt a forge",
            );
        }
        other => panic!("expected ForgeFailed {{ UnsupportedProducerEra }}, got {:?}", other),
    }
}

// =========================================================================
// PHASE4-N-F-G-C S1 — live-feed forge → serve → in-process block-fetch (CE-G-C-1)
// =========================================================================

/// Serve a single `(slot, hash)` over a node-spine `ServedChainView` via the
/// reused block-fetch reducer + `ServedChainLookups`, returning the `MsgBlock`
/// wire payload (tag-24 wrapped). Hermetic: drives `producer_block_fetch_serve`
/// directly — no real listener / socket / peer (that is the operator-gated leg).
/// Mirrors `produce_loopback::serve_block_fetch_payload`; replicated here
/// because each `tests/*.rs` is its own crate. It composes the PUBLIC serve API,
/// not a reimplementation of any wire authority.
fn serve_block_fetch_payload(view: &ServedChainView, slot: SlotNo, hash: Hash32) -> Vec<u8> {
    let snap = view.borrow();
    let look = ServedChainLookups { snap: &snap };
    let range = Range {
        from: Point::Block {
            slot,
            hash: hash.clone(),
        },
        to: Point::Block { slot, hash },
    };
    let (_state, step) = producer_block_fetch_serve(
        ProducerBlockFetchServerState::new(),
        BlockFetchMessage::RequestRange(range),
        &look,
        BlockFetchVersion::new(9),
    )
    .expect("serve");
    match step {
        BlockFetchServerStep::Replies(replies) => replies
            .into_iter()
            .find_map(|r| match r.into_message() {
                BlockFetchMessage::Block { bytes } => Some(bytes),
                _ => None,
            })
            .expect("a Block reply for the served range"),
        other => panic!("expected Replies, got {other:?}"),
    }
}

/// PHASE4-N-F-G-C S1 (CE-G-C-1): the FORGE-derived self-accepted block is served
/// byte-identically over block-fetch. A consistent eligible-leader tick forges a
/// real Conway/Praos block (`ForgeSucceeded`); the surfaced BLUE self-accepted
/// token is wrapped in the S1 `SelfAcceptedHandoff` (as the node spine does) and
/// admitted ONLY via `into_accepted()` → the single `ServedChainHandle::push_atomic`;
/// an in-process block-fetch loopback over the served view returns a `MsgBlock`
/// whose tag-24-unwrapped payload is the FORGED block bytes byte-for-byte.
///
/// This closes the serve leg of CE-G-C-1's `… → forge → self-accept →
/// sibling-serve → in-process block-fetch …` chain for a FORGE-derived block
/// (the G-B S3 `block_fetch_payload_is_self_accepted_bytes` proof used a
/// corpus-derived block; this uses the real forge output). The feed → ForgeTick
/// reachability leg is `node_sync::tests::live_wire_pump_feed_reaches_forge_tick`;
/// the live dial/pump are proven in `ade_runtime` wire_pump tests. NO peer
/// acceptance is claimed — that is operator-gated (RO-LIVE-01/06).
#[test]
fn live_feed_forge_serve_loopback_returns_forged_block() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();

    let (event, handoff_token) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    let forged_bytes = match event {
        CoordinatorEvent::ForgeSucceeded { slot: s, artifact } => {
            assert_eq!(s, slot, "ForgeSucceeded preserves the slot");
            artifact.bytes
        }
        other => panic!("expected ForgeSucceeded, got {:?}", other),
    };
    let accepted = handoff_token.expect("ForgeSucceeded surfaces the BLUE self-accepted token");

    // Node-spine admit: ONLY via the S1 carrier's into_accepted() → push_atomic.
    let handoff = SelfAcceptedHandoff::from_self_accepted(accepted);
    let (handle, view) = ServedChainHandle::new();
    let tip = handle
        .push_atomic(handoff.into_accepted())
        .expect("node-spine admit via into_accepted()");

    // In-process block-fetch over the served view → MsgBlock (tag-24).
    let payload = serve_block_fetch_payload(&view, tip.slot, tip.hash.clone());
    let inner = decompose_blockfetch_block(&payload).expect("served payload is tag24-wrapped");
    assert_eq!(
        inner,
        &forged_bytes[..],
        "served block-fetch payload (tag24-unwrapped) is the FORGED self-accept bytes"
    );
}
