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
use ade_core::consensus::errors::{HeaderValidationError, VrfCertError};
use ade_core::consensus::validate_and_apply_header;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_crypto::vrf::VrfVerificationKey;
use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::seed_consensus_inputs::SeedEpochConsensusInputs;
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

// PHASE4-N-U S1 — durable-forge-admit (DC-NODE-12) test surface.
use ade_ledger::receive::ReceiveState;
use ade_ledger::wal::{WalEntry, WalStore};
use ade_node::node_sync::admit_forged_block_durably;
use ade_runtime::chaindb::{ChainDb, PersistentChainDb, PersistentChainDbOptions};
use ade_runtime::forward_sync::ForwardSyncState;
use ade_runtime::rollback::SnapshotCadence;
use ade_runtime::wal::FileWalStore;
use tempfile::TempDir;

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
// PHASE4-N-F-G-J S5 — genesis-successor rehearsal mechanics (hermetic).
//
// The C1 genesis rehearsal correlates a real Haskell follower's accept of an
// Ade-forged GENESIS-SUCCESSOR block (block 0 + PrevHash::Genesis) into a
// non-promotable PrivateRehearsalManifest. These hermetic tests prove the
// harness wiring + bind it to the genesis-successor; the live correlate runs
// only under the operator-gated node_c1_genesis_rehearsal_live arm. The
// synthetic peer event here is correlate MECHANICS only — never written under
// the rehearsal home, never an acceptance claim.
// =========================================================================

#[test]
fn genesis_rehearsal_manifest_binds_block_zero_genesis() {
    use ade_node::ba02_evidence::{correlate, AdeForgeRecord, PeerAcceptEvent};
    use ade_node::rehearsal_evidence::{
        PrivateRehearsalManifest, RehearsalEnvelope, RehearsalVenue,
    };

    let epoch = EpochNo(0);
    let slot = 1u64;
    let magic = 42u32;

    // EligibleFixture forges a self-accepting genesis-successor (block 0 +
    // Genesis) — the SAME fixture forge_to_self_accept_succeeds proves.
    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();
    let (event, _handoff) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    let artifact = match event {
        CoordinatorEvent::ForgeSucceeded { artifact, .. } => artifact,
        other => panic!("expected ForgeSucceeded genesis block, got {other:?}"),
    };

    // The forged block IS the genesis-successor. decode_block runs the S3
    // check_header_position rule, so a block_number 0 that decodes successfully
    // MUST carry PrevHash::Genesis (else it would be rejected).
    let decoded = ade_ledger::block_validity::decode_block(&artifact.bytes)
        .expect("forged genesis block decodes + passes check_header_position");
    assert_eq!(
        decoded.header_input.block_no.0, 0,
        "genesis-successor is block 0 (Genesis prev guaranteed by check_header_position)"
    );

    // Correlate a follower's served-block accept of THIS genesis block's hash.
    let ade = AdeForgeRecord::from_forge_artifact(&artifact, magic);
    let events = vec![PeerAcceptEvent::PeerServedBlock {
        block_hash: Hash32(artifact.hash),
        slot: Some(artifact.slot),
        peer: "127.0.0.1:3010".to_string(),
    }];
    let outcome = correlate(&ade, &events);
    let envelope = RehearsalEnvelope {
        venue: RehearsalVenue::PrivateTestnetC1,
        peer_log_file: "phase4-n-f-g-j-genesis-rehearsal-test-peer.log".to_string(),
        peer_log_file_sha256: "ee".repeat(32),
    };
    let manifest = PrivateRehearsalManifest::from_correlate_outcome(&outcome, envelope)
        .expect("a follower accept of the genesis block correlates to a rehearsal manifest");

    let toml = manifest.to_canonical_toml();
    assert!(toml.contains("is_rehearsal = true"), "non-promotable marker");
    assert!(toml.contains("not_bounty_evidence = true"), "non-promotable marker");
    let hex: String = artifact.hash.iter().map(|b| format!("{b:02x}")).collect();
    assert!(
        toml.contains(&format!("matched_block_hash_hex = \"{hex}\"")),
        "manifest binds the exact forged genesis-block hash"
    );
}

#[test]
fn genesis_rehearsal_no_evidence_writes_nothing() {
    use ade_node::ba02_evidence::{correlate, AdeForgeRecord};
    use ade_node::rehearsal_evidence::{
        PrivateRehearsalManifest, RehearsalEnvelope, RehearsalVenue,
    };

    // No follower accept => NoEvidence => from_correlate_outcome => None. There
    // is no synthetic-manifest path for the genesis rehearsal.
    let ade = AdeForgeRecord {
        forged_block_hash: Hash32([0x11; 32]),
        slot: 1,
        network_magic: 42,
    };
    let outcome = correlate(&ade, &[]);
    let envelope = RehearsalEnvelope {
        venue: RehearsalVenue::PrivateTestnetC1,
        peer_log_file: "phase4-n-f-g-j-genesis-rehearsal-none.log".to_string(),
        peer_log_file_sha256: "ee".repeat(32),
    };
    assert!(
        PrivateRehearsalManifest::from_correlate_outcome(&outcome, envelope).is_none(),
        "NoEvidence must yield no genesis rehearsal manifest"
    );
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

// =========================================================================
// PHASE4-N-F-G-O S1 — feed-side BlockFetch tag-24 unwrap before decode (CN-WIRE-12)
// =========================================================================

/// PHASE4-N-F-G-O (CN-WIRE-12): the FEED/receive-side mirror of the serve wrap.
/// The genesis-successor block 0 (PrevHash::Genesis), served tag-24-wrapped over
/// block-fetch (the captured-shape `d8 18 …` wire payload the C1 follower echoed
/// back — the exact bytes Ade's feed crashed on before this cluster), is stripped
/// by the SINGLE `ade_codec` unwrap authority (`decompose_blockfetch_block` = the
/// exact call the wire pump now makes on the receive path) back to the bare
/// `[era, block]`, which `decode_block` accepts as Ade's block 0. This pins the
/// captured wrapped payload → unwrap → decode → block 0 / Genesis chain (G-O §12
/// acceptance #1 + #2) WITHOUT a brittle 830-byte literal: the forged genesis
/// block IS the payload the follower echoes, and the serve composer produces the
/// identical tag-24 shape.
#[test]
fn feed_unwrap_decodes_genesis_successor_block_zero() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();

    // Forge the genesis-successor block 0 (PrevHash::Genesis) and admit it to the
    // served view via the sole node-spine path (handoff → into_accepted → push_atomic).
    let (event, handoff_token) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    let forged_bytes = match event {
        CoordinatorEvent::ForgeSucceeded { artifact, .. } => artifact.bytes,
        other => panic!("expected ForgeSucceeded genesis block, got {other:?}"),
    };
    let accepted = handoff_token.expect("ForgeSucceeded surfaces the BLUE self-accepted token");
    let handoff = SelfAcceptedHandoff::from_self_accepted(accepted);
    let (handle, view) = ServedChainHandle::new();
    let tip = handle
        .push_atomic(handoff.into_accepted())
        .expect("node-spine admit via into_accepted()");

    // Serve it tag-24-wrapped (the captured-shape wire payload: `d8 18 …`).
    let wire_payload = serve_block_fetch_payload(&view, tip.slot, tip.hash.clone());
    assert_eq!(
        &wire_payload[0..2],
        &[0xd8, 0x18],
        "the served BlockFetch payload is tag-24-wrapped (the captured C1 shape)"
    );

    // FEED/receive-side unwrap via the SINGLE authority — the exact call the wire
    // pump's receive path makes → bare [era, block].
    let bare = decompose_blockfetch_block(&wire_payload)
        .expect("the captured-shape tag-24 payload unwraps via the single authority");
    assert_eq!(
        bare,
        &forged_bytes[..],
        "the unwrapped bare bytes are the forged genesis block verbatim"
    );

    // The bare bytes decode as Ade's block 0; decode_block runs the S3
    // check_header_position rule, so a block_no 0 that decodes MUST carry
    // PrevHash::Genesis (else it would be rejected) — the feed no longer crashes.
    let decoded = ade_ledger::block_validity::decode_block(bare)
        .expect("the unwrapped bare bytes decode as a block (the feed no longer crashes)");
    assert_eq!(
        decoded.header_input.block_no.0, 0,
        "the feed decodes Ade's genesis-successor block 0 (PrevHash::Genesis \
         guaranteed by check_header_position)"
    );
}

// =========================================================================
// PHASE4-N-F-G-P S1 — feed header-validation view from the recovered surface (DC-CINPUT-04)
// =========================================================================

/// PHASE4-N-F-G-P (DC-CINPUT-04): the feed/receive header validator MUST use the
/// recovered consensus surface — the SAME projection the forge uses
/// (`PoolDistrView::from_seed_epoch_consensus_inputs`) — so Step 5 (VRF-keyhash
/// binding) + Step 7 (leader threshold) find the producer's stake. The live C1
/// feed failed `Header(VrfCert(VerificationFailed))` because the wiring fed an
/// EMPTY placeholder view (`pool_active_stake = None`). This pins both halves:
/// (1) the recovered-surface view validates the forged genesis-successor header
/// through Step 5 + Step 7; (2) the empty placeholder view fails closed exactly as
/// the live feed did — locking the regression. (Same recovered record drives the
/// forge leadership view, so forge + feed share one consensus surface.)
#[test]
fn feed_header_validates_against_recovered_surface_not_empty_view() {
    let epoch = EpochNo(0);
    let slot = 1u64;

    let mut shell = synth_shell(0x77, 0x88, 0x99);
    let fixture = EligibleFixture::build(&shell, slot, epoch);
    let ctx = fixture.ctx();
    let (event, _handoff) = run_real_forge(slot, /* kes_period = */ 0, &ctx, &mut shell);
    let forged_bytes = match event {
        CoordinatorEvent::ForgeSucceeded { artifact, .. } => artifact.bytes,
        other => panic!("expected ForgeSucceeded genesis block, got {other:?}"),
    };
    let decoded =
        ade_ledger::block_validity::decode_block(&forged_bytes).expect("forged genesis block decodes");

    // Build the recovered SeedEpochConsensusInputs over the SAME consensus surface
    // the fixture forged under (pool_id = blake2b_224(cold_vk), vrf_keyhash =
    // blake2b_256(vrf_vk), total = 1, ASC 1/1 — the EligibleFixture recipe).
    let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&shell.cold_vk().0);
    let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&fixture.vrf_vk.0);
    let mut pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    pools.insert(
        pool_id,
        PoolEntry {
            active_stake: 1,
            vrf_keyhash,
        },
    );
    let record = SeedEpochConsensusInputs {
        anchor_fp: Hash32([0u8; 32]),
        epoch_no: epoch,
        epoch_start_slot: SlotNo(epoch.0 * 432_000),
        epoch_length_slots: 432_000,
        epoch_nonce: fixture.eta0_holder.epoch_nonce.clone(),
        active_slots_coeff: ActiveSlotsCoeff { numer: 1, denom: 1 },
        total_active_stake: 1,
        pool_distribution: pools,
    };

    // (1) The recovered-surface feed view validates the header (Step 5 + Step 7).
    let recovered_view = PoolDistrView::from_seed_epoch_consensus_inputs(&record);
    validate_and_apply_header(
        &fixture.eta0_holder,
        &decoded.header_input,
        &recovered_view,
        &fixture.era_schedule,
    )
    .expect(
        "the recovered consensus surface validates the genesis-successor header \
         through Step 5 + Step 7 (DC-CINPUT-04)",
    );

    // (2) The EMPTY placeholder view fails closed exactly as the live C1 feed did
    // (pool_active_stake = None ⇒ Step 7 VrfCert(VerificationFailed)) — the
    // pre-G-P bug this slice removes from the live feed wiring.
    let empty_view =
        PoolDistrView::new(epoch, 0, ActiveSlotsCoeff { numer: 0, denom: 1 }, BTreeMap::new());
    let err = validate_and_apply_header(
        &fixture.eta0_holder,
        &decoded.header_input,
        &empty_view,
        &fixture.era_schedule,
    )
    .expect_err("the empty placeholder view must fail closed (the pre-G-P bug)");
    assert!(
        matches!(
            err,
            HeaderValidationError::VrfCert(VrfCertError::VerificationFailed)
        ),
        "empty view fails at the Step 7 leader threshold with VrfCert(VerificationFailed), got {err:?}"
    );
}

// =========================================================================
// PHASE4-N-F-G-R S1 — stable served block 0 (DC-NODE-11)
// =========================================================================
// PHASE4-N-U S3 (DC-NODE-13): serve_gate_keeps_first_block_zero_skips_reforge is
// RETIRED with the monotone serve gate. The durable chain is extend-only
// (DC-CONS-23) — a re-mint block 0 fails closed at admit — so it holds exactly
// one block 0 by construction, and the serve PROJECTION serves that stable chain
// without a gate. DC-NODE-11's stability is now proven by serve-as-projection:
// tests/node_spine_serve_loopback.rs (served_view_projects_durable_chain,
// follower_fetches_coherent_history_incl_ingested_predecessor,
// served_view_retires_accumulator) + ci/ci_check_served_chain_projection.sh.

// =========================================================================
// PHASE4-N-U S1 — own-forged durable admit through the pump (DC-NODE-12,
// DC-CONS-23, DC-WAL-04 chaining). A self-accepted forged block becomes
// durable ONLY through admit_forged_block_durably -> pump_block (the SAME
// chokepoint received blocks use): extend-only validate -> StoreBlockBytes ->
// AppendWal -> AdvanceTip. The forge advances no durable tip directly.
// =========================================================================

/// Forge a genesis-successor block 0 (block 0 + PrevHash::Genesis) to
/// self-accept; return the typed handoff, the self-accepted bytes (for the
/// I-10 byte-identity check), and the eligible fixture (whose era_schedule +
/// pool_distr_view the durable admit reuses for header validation).
fn forge_block0_handoff(
    shell: &mut ProducerShell,
    slot: u64,
) -> (SelfAcceptedHandoff, Vec<u8>, EligibleFixture) {
    let fixture = EligibleFixture::build(shell, slot, EpochNo(0));
    let (event, handoff) = run_real_forge(slot, 0, &fixture.ctx(), shell);
    let accepted = match (event, handoff) {
        (CoordinatorEvent::ForgeSucceeded { .. }, Some(a)) => a,
        (ev, _) => panic!(
            "PHASE4-N-U S1 setup: expected block-0 ForgeSucceeded with a self-accepted token, \
             got {ev:?}"
        ),
    };
    let forged_bytes = accepted.as_bytes().to_vec();
    (
        SelfAcceptedHandoff::from_self_accepted(accepted),
        forged_bytes,
        fixture,
    )
}

/// A fresh durable store (real PersistentChainDb + FileWalStore) plus a
/// ForwardSyncState whose base matches the forge base (genesis Conway @ epoch 0
/// + chain_dep eta0 = 0xCD, the EligibleFixture constants), so pump_block's
/// extend-only block_validity accepts the forged genesis-successor block 0.
/// The anchor/prior_fp seed is Hash32([0xA0; 32]) — the first forged AdmitBlock
/// chains from it (DC-WAL-04).
fn durable_store() -> (TempDir, PersistentChainDb, FileWalStore, ForwardSyncState) {
    let dir = TempDir::new().expect("tempdir");
    let chaindb =
        PersistentChainDb::open(PersistentChainDbOptions::at(dir.path().join("chain.db")))
            .expect("chaindb");
    let wal = FileWalStore::open(dir.path().join("wal")).expect("wal");
    let mut ledger = LedgerState::new(CardanoEra::Conway);
    ledger.epoch_state.epoch = EpochNo(0);
    let mut chain_dep = PraosChainDepState::empty();
    chain_dep.epoch_nonce = Nonce(Hash32([0xCD; 32]));
    let state = ForwardSyncState::new(
        ReceiveState::new(ledger, chain_dep),
        Hash32([0xA0; 32]),
        SnapshotCadence::DEFAULT,
    );
    (dir, chaindb, wal, state)
}

#[test]
fn forge_tick_durable_admit_advances_tip() {
    // DC-NODE-12 / CE-1: a self-accepted forged block reaches the durable tip
    // ONLY through admit_forged_block_durably -> pump_block (durable-before-tip).
    let mut shell = synth_shell(0x11, 0x22, 0x33);
    let (h, _bytes, fixture) = forge_block0_handoff(&mut shell, 1);
    let (_dir, chaindb, mut wal, mut state) = durable_store();

    let tip = admit_forged_block_durably(
        &h,
        &mut state,
        &chaindb,
        &mut wal,
        &fixture.era_schedule,
        &fixture.pool_distr_view,
    )
    .expect("durable admit ok")
    .expect("the forged block advanced the durable tip");

    let chain_tip = ChainDb::tip(&chaindb).expect("tip").expect("non-empty");
    assert_eq!(chain_tip.slot, tip.slot, "ChainDb tip slot == admitted tip");
    assert_eq!(chain_tip.hash, tip.hash, "ChainDb tip hash == admitted tip");
    let entries = wal.read_all().expect("read_all");
    assert!(
        entries
            .iter()
            .any(|e| matches!(e, WalEntry::AdmitBlock { slot, .. } if *slot == tip.slot)),
        "the forged block must be WAL-admitted at the durable tip slot (durable-before-tip)"
    );
}

#[test]
fn forge_successor_builds_block_1_from_durable_tip() {
    // DC-NODE-12 / CE-1: after admitting block 0, state.receive + the durable
    // ChainDb advance together, so the next forge builds block 1 on the durable
    // tip — a real growing chain.
    let mut shell = synth_shell(0xBB, 0xCC, 0xDD);
    let (h0, _b0, fx0) = forge_block0_handoff(&mut shell, 1);
    let (_dir, chaindb, mut wal, mut state) = durable_store();

    let tip0 = admit_forged_block_durably(
        &h0,
        &mut state,
        &chaindb,
        &mut wal,
        &fx0.era_schedule,
        &fx0.pool_distr_view,
    )
    .expect("admit block 0")
    .expect("tip 0");
    assert_eq!(
        state.receive.chain_dep.last_block_no,
        Some(BlockNo(0)),
        "the evolved spine's last_block_no is 0 after admitting block 0"
    );

    // Forge block 1 on the DURABLE tip (block 0), against the EVOLVED base.
    let fx1 = EligibleFixture::build(&shell, 2, EpochNo(0));
    let ctx1 = ForgeRequestContext {
        eta0: &state.receive.chain_dep.epoch_nonce,
        vrf_vk: &fx1.vrf_vk,
        leader_schedule_answer: &fx1.leader_answer,
        pparams: &fx1.pparams,
        base_state: &state.receive.ledger,
        chain_dep_state: &state.receive.chain_dep,
        era_schedule: &fx1.era_schedule,
        pool_distr_view: &fx1.pool_distr_view,
        block_number: BlockNo(1),
        prev_hash: PrevHash::Block(tip0.hash.clone()),
        protocol_version: ProtocolVersion { major: 9, minor: 0 },
        prev_opcert_counter: None,
    };
    let (ev1, hd1) = run_real_forge(2, 0, &ctx1, &mut shell);
    let accepted1 = match (ev1, hd1) {
        (CoordinatorEvent::ForgeSucceeded { .. }, Some(a)) => a,
        (ev, _) => panic!("expected block-1 ForgeSucceeded on the durable tip, got {ev:?}"),
    };
    let h1 = SelfAcceptedHandoff::from_self_accepted(accepted1);

    let tip1 = admit_forged_block_durably(
        &h1,
        &mut state,
        &chaindb,
        &mut wal,
        &fx1.era_schedule,
        &fx1.pool_distr_view,
    )
    .expect("admit block 1")
    .expect("tip 1");

    assert_ne!(tip1.hash, tip0.hash, "block 1 has a distinct hash from block 0");
    assert_eq!(
        state.receive.chain_dep.last_block_no,
        Some(BlockNo(1)),
        "after admitting block 1 the durable spine's last_block_no is 1 — a real growing chain"
    );
    let chain_tip = ChainDb::tip(&chaindb).expect("tip").expect("non-empty");
    assert_eq!(chain_tip.hash, tip1.hash, "the durable ChainDb tip is block 1");
    let admits = wal
        .read_all()
        .expect("read_all")
        .into_iter()
        .filter(|e| matches!(e, WalEntry::AdmitBlock { .. }))
        .count();
    assert_eq!(
        admits, 2,
        "the WAL holds two forged AdmitBlock entries (block 0 + block 1)"
    );
}

#[test]
fn forged_admit_bytes_byte_identical_to_self_accept() {
    // I-10 / CE-2: the durably-stored bytes are byte-identical to the
    // self-accepted bytes — no re-encode between self_accept and durable admit.
    let mut shell = synth_shell(0x44, 0x55, 0x66);
    let (h, forged_bytes, fixture) = forge_block0_handoff(&mut shell, 1);
    let (_dir, chaindb, mut wal, mut state) = durable_store();

    let tip = admit_forged_block_durably(
        &h,
        &mut state,
        &chaindb,
        &mut wal,
        &fixture.era_schedule,
        &fixture.pool_distr_view,
    )
    .expect("ok")
    .expect("tip");

    let stored = ChainDb::get_block_by_hash(&chaindb, &tip.hash)
        .expect("get")
        .expect("block present durably");
    assert_eq!(
        stored.bytes, forged_bytes,
        "the durably-stored bytes must be byte-identical to the self-accepted bytes (I-10)"
    );
}

#[test]
fn stale_tip_forge_fails_closed() {
    // DC-CONS-23 / CE-3: a stale-tip forge (a re-mint block 0 against a chain
    // already at block 0) fails closed at the extend-only admit; the durable
    // tip is unchanged. No own-block override; no admit-time fork-choice.
    let mut shell = synth_shell(0x88, 0x99, 0xAA);
    let (h0, _b0, fx0) = forge_block0_handoff(&mut shell, 1);
    let (_dir, chaindb, mut wal, mut state) = durable_store();
    let tip0 = admit_forged_block_durably(
        &h0,
        &mut state,
        &chaindb,
        &mut wal,
        &fx0.era_schedule,
        &fx0.pool_distr_view,
    )
    .expect("admit block 0")
    .expect("tip 0");

    // A second genesis-successor block 0 (PrevHash::Genesis) does not extend a
    // chain already at block 0 — the extend-only admit rejects it.
    let (h_stale, _bs, fx_stale) = forge_block0_handoff(&mut shell, 3);
    let res = admit_forged_block_durably(
        &h_stale,
        &mut state,
        &chaindb,
        &mut wal,
        &fx_stale.era_schedule,
        &fx_stale.pool_distr_view,
    );
    assert!(
        res.is_err(),
        "a stale-tip re-mint (block 0 on a chain already at block 0) must fail closed (DC-CONS-23)"
    );
    let chain_tip = ChainDb::tip(&chaindb).expect("tip").expect("non-empty");
    assert_eq!(
        chain_tip.hash, tip0.hash,
        "the durable tip is unchanged after the failed stale-tip admit"
    );
    assert_eq!(chain_tip.slot, tip0.slot);
}

#[test]
fn forged_admit_wal_prior_fp_chains() {
    // DC-WAL-04 (chaining) / CE-4: the forged AdmitBlock's prior_fp chains from
    // the anchor initial_ledger_fingerprint, and the WAL verifies from it.
    let mut shell = synth_shell(0x77, 0x12, 0x34);
    let (h, _bytes, fixture) = forge_block0_handoff(&mut shell, 1);
    let (_dir, chaindb, mut wal, mut state) = durable_store();
    admit_forged_block_durably(
        &h,
        &mut state,
        &chaindb,
        &mut wal,
        &fixture.era_schedule,
        &fixture.pool_distr_view,
    )
    .expect("ok")
    .expect("tip");

    let (prior_fp, post_fp) = wal
        .read_all()
        .expect("read_all")
        .into_iter()
        .find_map(|e| match e {
            WalEntry::AdmitBlock {
                prior_fp, post_fp, ..
            } => Some((prior_fp, post_fp)),
            _ => None,
        })
        .expect("a forged AdmitBlock entry");
    assert_eq!(
        prior_fp,
        Hash32([0xA0; 32]),
        "the first forged AdmitBlock.prior_fp == the anchor initial_ledger_fingerprint"
    );
    assert_ne!(post_fp, Hash32([0u8; 32]), "post_fp is a real ledger fingerprint");
    wal.verify_chain(&Hash32([0xA0; 32]))
        .expect("the forged WAL chain verifies from the anchor");
}
