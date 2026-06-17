// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! PHASE4-N-AE Slice A — recover -> serve continuity + forge-on-followed-tip gate.
//!
//! Grounded by the 2026-06-06 code trace:
//!   - `seed_to_snapshot` (admission) persists a ledger SNAPSHOT at the recovered
//!     anchor slot via `PersistentSnapshotCache::capture` — NO servable
//!     `StoredBlock`. So after recover `ChainDb::tip()` is `None` and the anchor
//!     is not a peer-intersectable point.
//!   - Before AE.A the relay-loop forge read `selected_tip` from `ChainDb::tip()`
//!     and FELL BACK to the recovered anchor when that was `None`
//!     (`node_lifecycle.rs:1098-1103`), then set the forged successor's parent =
//!     `selected_tip.hash`. So a forge firing on a `NoWorkReady` gap (before the
//!     follow durably stored the peer's tip) built its successor on the
//!     snapshot-only anchor → the served chain was not peer-adoptable.
//!   - The follow path itself (`run_node_sync` → `pump_block` → `put_block`) DOES
//!     store followed blocks as servable `StoredBlock`s.
//!
//! ## AE.A (this slice) — green
//!
//! AE.A removes the `recovered.tip` forge-base fallback and gates the forge on
//! `durable_servable_tip == followed_peer_tip` (the closed GREEN classifier
//! `forge_followed_tip_admission`, fail-closed to a typed
//! `ForgeRefused::NotCaughtUp`). The forged-on-followed-tip tests prove the
//! parent byte-equality (DC-CONS-24), the served-chain continuity (DC-NODE-14
//! followed-tip clause), and replay determinism (T-REC-05). These are
//! un-ignored.
//!
//! ## AE.B (recovered-anchor intersectability) — still red-on-demand
//!
//! The recovered-anchor clause of DC-NODE-14 (make a recovered anchor
//! peer-intersectable for the recover-at/near-tip case with zero followed
//! blocks) is AE.B. Those two fixtures stay `#[ignore]`d; run them on demand:
//!
//! ```text
//! cargo test -p ade_node --test phase4_n_ae_recover_serve_continuity_diag \
//!   -- --ignored --nocapture
//! ```

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use ade_core::consensus::praos_state::PraosChainDepState;
use ade_ledger::pparams::ProtocolParameters;
use ade_ledger::receive::events::TipPoint;
use ade_ledger::utxo::UTxOState;
use ade_network::block_fetch::server::ServedRangeLookup;
use ade_network::chain_sync::server::ServedHeaderLookup;
use ade_network::codec::chain_sync::Point;
use ade_node::admission::seed_to_snapshot;
use ade_node::node_sync::{
    forge_followed_tip_admission, ForgeFollowedTipAdmission, ForgeRefused, NotCaughtUpReason,
};
use ade_runtime::chaindb::{ChainDb, InMemoryChainDb, StoredBlock};
use ade_runtime::network::ChainDbServedSource;
use ade_types::{Hash32, SlotNo};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Recover at a non-Origin Conway tip exactly as the admission path does:
/// `seed_to_snapshot` captures a ledger snapshot at `slot` — and ONLY a
/// snapshot, never a servable `StoredBlock`. Returns the store (which is both a
/// `ChainDb` and a `SnapshotStore`, so the same instance backs recover + serve).
fn recover_at_anchor(slot: SlotNo) -> InMemoryChainDb {
    let db = InMemoryChainDb::new();
    seed_to_snapshot(
        UTxOState::new(),
        PraosChainDepState::empty(),
        slot,
        &db,
        ProtocolParameters::default(),
    )
    .expect("seed_to_snapshot recovers a non-Origin anchor (snapshot only)");
    db
}

fn hash_hex(h: &Hash32) -> String {
    h.0.iter().map(|b| format!("{b:02x}")).collect()
}

fn field(label: &str, value: String) {
    eprintln!("    {label:<42} = {value}");
}

// The recovered non-Origin tip (RO-LIVE-05 shape: the real preprod Conway tip
// Ade recovers at). The hash stands for the recovered tip's block hash — in the
// real flow it is the `--seed-block-hash` CLI arg / BootstrapAnchor, NOT derived
// from `seed_to_snapshot` (which is keyed by slot only).
const ANCHOR_SLOT: SlotNo = SlotNo(124_140_368);
fn anchor_hash() -> Hash32 {
    Hash32([0xD1; 32])
}

// ===========================================================================
// AE.A — Gap 2a: the forge base no longer falls back to the snapshot anchor.
// ===========================================================================

#[test]
fn forge_base_falls_back_to_snapshot_anchor() {
    // AE.A adjusted (was RED): the forge base is the DURABLE servable tip, never
    // the snapshot-only recovered anchor. After `seed_to_snapshot` the durable
    // servable tip (the serve projection's `tip()`) is `None` — there is NO
    // StoredBlock — so the admissibility gate sees `NoDurableServableTip` and
    // refuses; it does NOT silently substitute the recovered anchor as a base.
    let db = recover_at_anchor(ANCHOR_SLOT);
    let anchor = anchor_hash();

    // The forge base IS the durable servable tip (DC-NODE-15): the serve
    // projection's tip — exactly the (slot, hash, block_no) a peer would see.
    let durable_servable_tip: Option<TipPoint> =
        ChainDbServedSource::new(&db)
            .tip()
            .map(|(slot, hash, block_no)| TipPoint {
                slot,
                hash,
                block_no,
            });

    // The recovered anchor a peer would follow to (the would-be old fallback).
    let recovered_anchor_tip = TipPoint {
        slot: ANCHOR_SLOT,
        hash: anchor.clone(),
        block_no: 7,
    };

    eprintln!("[AE.A] at forge decision after recover (no fallback):");
    field("durable_servable_tip", format!("{durable_servable_tip:?}"));
    field(
        "recovered_anchor_tip (NOT a forge base)",
        format!(
            "slot={} hash={}",
            recovered_anchor_tip.slot.0,
            hash_hex(&recovered_anchor_tip.hash)
        ),
    );

    // The durable servable tip after a snapshot-only recover is None — the
    // recovered anchor is NEVER materialized as a servable forge base.
    assert!(
        durable_servable_tip.is_none(),
        "after a snapshot-only recover the durable servable tip is None — the recovered \
         anchor is NEVER a forge base (the removed node_lifecycle.rs:1102 fallback)"
    );

    // The admissibility gate refuses with the named NoDurableServableTip reason —
    // it never silently treats the absent durable tip as the recovered anchor.
    let verdict = forge_followed_tip_admission(durable_servable_tip, Some(recovered_anchor_tip));
    assert_eq!(
        verdict,
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::NoDurableServableTip,
        },
        "with no durable servable tip the forge is refused (NoDurableServableTip), never \
         based on the recovered anchor"
    );
}

// ===========================================================================
// AE.A — the forge is refused (typed) when not caught up to the followed tip.
// ===========================================================================

#[test]
fn forge_refused_not_caught_up() {
    // DC-NODE-15: `durable_servable_tip != followed_peer_tip` ⇒ the typed
    // `ForgeFollowedTipAdmission::NotCaughtUp` with the correct named reason; the
    // forge does not fire. All three reasons are diagnostic-distinct (never a
    // fake tip, never a silently-collapsed equality failure). The structured
    // `ForgeRefused::NotCaughtUp` carries both tips + the reason.
    let durable = TipPoint {
        slot: SlotNo(100),
        hash: Hash32([0xA0; 32]),
        block_no: 10,
    };
    let peer_ahead = TipPoint {
        slot: SlotNo(101),
        hash: Hash32([0xB0; 32]),
        block_no: 11,
    };

    // (1) Both present, hashes differ ⇒ TipMismatch.
    assert_eq!(
        forge_followed_tip_admission(Some(durable.clone()), Some(peer_ahead.clone())),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::TipMismatch,
        },
        "durable tip behind the peer (different hash) ⇒ TipMismatch (no forge)"
    );

    // (1b) Same hash but different block_no ⇒ still TipMismatch (block_no is
    // compared, never hash-only).
    let same_hash_diff_no = TipPoint {
        slot: durable.slot,
        hash: durable.hash.clone(),
        block_no: durable.block_no + 1,
    };
    assert_eq!(
        forge_followed_tip_admission(Some(durable.clone()), Some(same_hash_diff_no)),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::TipMismatch,
        },
        "same hash but mismatched block_no ⇒ TipMismatch (block_no IS compared)"
    );

    // (2) No durable servable tip yet (the follow stored nothing), peer present
    // ⇒ NoDurableServableTip.
    assert_eq!(
        forge_followed_tip_admission(None, Some(peer_ahead.clone())),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::NoDurableServableTip,
        },
        "no durable servable tip ⇒ NoDurableServableTip (no forge, no fake tip)"
    );

    // (3) No followed peer tip observed yet ⇒ NoFollowedPeerTip.
    assert_eq!(
        forge_followed_tip_admission(Some(durable.clone()), None),
        ForgeFollowedTipAdmission::NotCaughtUp {
            reason: NotCaughtUpReason::NoFollowedPeerTip,
        },
        "no followed peer tip ⇒ NoFollowedPeerTip (no forge)"
    );

    // The structured typed refusal carries BOTH tips + the reason — never a
    // log-string-only path (the shape the ForgeTick arm records).
    let refused = ForgeRefused::NotCaughtUp {
        local_servable_tip: Some(durable.clone()),
        followed_peer_tip: Some(peer_ahead.clone()),
        reason: NotCaughtUpReason::TipMismatch,
    };
    match refused {
        ForgeRefused::NotCaughtUp {
            local_servable_tip,
            followed_peer_tip,
            reason,
        } => {
            assert_eq!(local_servable_tip, Some(durable));
            assert_eq!(followed_peer_tip, Some(peer_ahead));
            assert_eq!(reason, NotCaughtUpReason::TipMismatch);
        }
        ForgeRefused::SingleProducerFenceViolation { .. } => {
            panic!("expected NotCaughtUp, got SingleProducerFenceViolation")
        }
        ForgeRefused::ReselectionPending => {
            panic!("expected NotCaughtUp, got ReselectionPending")
        }
    }
}

// ===========================================================================
// RED — AE.B (Gap 2b): the recovered anchor is not a peer-intersectable serve
// point. Stays #[ignore]d until PHASE4-N-AE.B (recovered-anchor clause).
// ===========================================================================

#[test]
fn recovered_anchor_is_not_peer_intersectable() {
    // PHASE4-N-AE.B (Option B — the invariant's FAIL-CLOSED side): a recover-ONLY
    // store (zero followed/forged blocks ⇒ NO servable successor) has no
    // projectable forge parent. The invariant is "advertise a parent as an
    // intersection point ONLY IF Ade can prove it is the parent of a real
    // servable successor" — with no successor there is nothing to prove, so
    // intersect(anchor) == None (the peer falls back to Origin; there is nothing
    // to adopt yet). The POSITIVE case (a forged successor ⇒ the parent IS
    // intersectable) is `followed_tip::forged_successor_on_recovered_anchor_is_not_peer_adoptable`
    // + the live-style follow→serve test. This guards the "no synthetic / no
    // magic anchor" boundary: the anchor is NEVER materialized as a StoredBlock.
    let db = recover_at_anchor(ANCHOR_SLOT);
    let served = ChainDbServedSource::new(&db);
    let anchor = anchor_hash();

    let durable_tip = served.tip();
    let gbh = ChainDb::get_block_by_hash(&db, &anchor).expect("get_block_by_hash");
    let isect = served.intersect(&[Point::Block {
        slot: ANCHOR_SLOT,
        hash: anchor.clone(),
    }]);

    eprintln!("[AE.B diag] after recover-ONLY (seed_to_snapshot @ non-Origin anchor, no successor):");
    field("recovered_tip_slot", ANCHOR_SLOT.0.to_string());
    field("recovered_tip_hash", hash_hex(&anchor));
    field("durable_servable_tip", format!("{durable_tip:?}"));
    field(
        "get_block_by_hash(recovered_tip_hash)",
        format!("{:?}", gbh.as_ref().map(|b| b.slot)),
    );
    field("intersect(recovered_tip)", format!("{isect:?}"));

    // Option B fail-closed: no servable successor ⇒ no projection.
    assert_eq!(
        isect, None,
        "recover-ONLY (no servable successor) must NOT project the anchor — the \
         projection is proof-gated on a real servable successor (the earliest \
         StoredBlock's prev_hash). Positive case: forged_successor_..._is_not_peer_adoptable."
    );
    assert!(
        gbh.is_none(),
        "the recovered anchor is NEVER materialized as a StoredBlock (no synthetic bytes)"
    );
}

// ===========================================================================
// AE.A — forge-on-followed-tip: parent byte-equality + serve continuity +
// replay determinism (the durable-tip forge harness).
// ===========================================================================

mod followed_tip {
    use super::*;

    use std::collections::BTreeMap;

    use ade_core::consensus::era_schedule::EraSchedule;
    use ade_core::consensus::leader_schedule::LeaderScheduleAnswer;
    use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
    use ade_core::consensus::vrf_cert::{leader_vrf_input, ActiveSlotsCoeff};
    use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
    use ade_crypto::vrf::VrfVerificationKey;
    use ade_ledger::block_validity::decode_block;
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::receive::ReceiveState;
    use ade_ledger::state::LedgerState;
    use ade_node::node_sync::admit_forged_block_durably;
    use ade_node::produce_mode::{run_real_forge, ForgeRequestContext};
    use ade_runtime::chaindb::{PersistentChainDb, PersistentChainDbOptions};
    use ade_runtime::forward_sync::ForwardSyncState;
    use ade_runtime::producer::coordinator::CoordinatorEvent;
    use ade_runtime::producer::producer_shell::ProducerShell;
    use ade_runtime::producer::self_accepted_handoff::SelfAcceptedHandoff;
    use ade_runtime::rollback::SnapshotCadence;
    use ade_runtime::wal::FileWalStore;
    use ade_types::shelley::block::{OperationalCert, PrevHash, ProtocolVersion};
    use ade_types::{BlockNo, CardanoEra, EpochNo, Hash28};
    use tempfile::TempDir;

    // Mirrors forge_succeeds::synth_shell — a deterministic operator shell whose
    // pool wins under ASC 1/1 (always-eligible) leadership. (Each tests/*.rs is
    // its own crate, so the helper is replicated rather than imported.)
    fn synth_shell(cold_seed: u8, vrf_seed: u8, kes_seed: u8) -> ProducerShell {
        use ade_runtime::producer::signing::{ColdSigningKey, VrfSigningKey};
        use cardano_crypto::vrf::VrfDraft03;

        let cold_bytes = [cold_seed; 32];
        let cold = ColdSigningKey::from_bytes_zeroizing(&cold_bytes).unwrap();

        let (vrf_sk_bytes, _vrf_vk_bytes) = VrfDraft03::keypair_from_seed(&[vrf_seed; 32]);
        let vrf = VrfSigningKey::from_bytes_zeroizing(&vrf_sk_bytes).unwrap();

        let kes_seed_bytes = [kes_seed; 32];
        let kes =
            ade_runtime::producer::signing::KesSecret::from_seed_at_period(&kes_seed_bytes, 0)
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

        ProducerShell::init(kes, vrf, cold, opcert, 0).expect("shell init")
    }

    /// The always-eligible leadership context for `slot` in `epoch` (pool_id =
    /// blake2b_224(cold_vk), vrf_keyhash = blake2b_256(vrf_vk), total = 1, ASC
    /// 1/1, shared eta0) — mirrors forge_succeeds::EligibleFixture.
    struct Eligible {
        chain_dep: PraosChainDepState,
        vrf_vk: VrfVerificationKey,
        answer: LeaderScheduleAnswer,
        pparams: ProtocolParameters,
        base_state: LedgerState,
        era_schedule: EraSchedule,
        pool_distr_view: PoolDistrView,
    }

    impl Eligible {
        fn build(shell: &ProducerShell, slot: u64, epoch: EpochNo) -> Self {
            let eta0 = Nonce(Hash32([0xCD; 32]));
            let vrf_vk = shell.vrf_verification_key();
            let cold_vk = shell.cold_vk();
            let pool_id: Hash28 = ade_crypto::blake2b::blake2b_224(&cold_vk.0);
            let vrf_keyhash: Hash32 = ade_crypto::blake2b::blake2b_256(&vrf_vk.0);

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

            let answer = LeaderScheduleAnswer {
                slot: SlotNo(slot),
                pool: pool_id,
                epoch,
                expected_vrf_input: leader_vrf_input(CardanoEra::Conway, SlotNo(slot), &eta0),
                stake_fraction: (1, 1),
                asc: ActiveSlotsCoeff { numer: 1, denom: 1 },
            };

            let mut chain_dep = PraosChainDepState::empty();
            chain_dep.epoch_nonce = eta0;

            let mut base_state = LedgerState::new(CardanoEra::Conway);
            base_state.epoch_state.epoch = epoch;

            let era_schedule = EraSchedule::new(
                BootstrapAnchorHash(Hash32([0u8; 32])),
                0,
                vec![EraSummary {
                    era: CardanoEra::Conway,
                    start_slot: SlotNo(0),
                    start_epoch: epoch,
                    slot_length_ms: 1_000,
                    epoch_length_slots: 432_000,
                    safe_zone_slots: 129_600,
                }],
            )
            .expect("era schedule");

            Eligible {
                chain_dep,
                vrf_vk,
                answer,
                pparams: ProtocolParameters::default(),
                base_state,
                era_schedule,
                pool_distr_view,
            }
        }
    }

    /// A fresh durable store (real PersistentChainDb + FileWalStore) plus a
    /// ForwardSyncState whose base matches the Eligible forge base (genesis
    /// Conway @ epoch 0 + chain_dep eta0 = 0xCD), so `pump_block`'s extend-only
    /// `block_validity` accepts the forged genesis-successor block 0. Mirrors
    /// forge_succeeds::durable_store.
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

    /// Forge a genesis-successor block 0 to self-accept; return the handoff +
    /// the eligible fixture (whose era_schedule + pool_distr_view the durable
    /// admit reuses for header validation).
    fn forge_block0(shell: &mut ProducerShell, slot: u64) -> (SelfAcceptedHandoff, Eligible) {
        let fx = Eligible::build(shell, slot, EpochNo(0));
        let ctx = ForgeRequestContext {
            eta0: &fx.chain_dep.epoch_nonce,
            vrf_vk: &fx.vrf_vk,
            leader_schedule_answer: &fx.answer,
            pparams: &fx.pparams,
            base_state: &fx.base_state,
            chain_dep_state: &fx.chain_dep,
            era_schedule: &fx.era_schedule,
            pool_distr_view: &fx.pool_distr_view,
            block_number: BlockNo(0),
            prev_hash: PrevHash::Genesis,
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
            prev_opcert_counter: None,
        };
        let (event, handoff) = run_real_forge(slot, 0, &ctx, shell);
        let accepted = match (event, handoff) {
            (CoordinatorEvent::ForgeSucceeded { .. }, Some(a)) => a,
            (ev, _) => panic!("setup: expected block-0 ForgeSucceeded, got {ev:?}"),
        };
        (SelfAcceptedHandoff::from_self_accepted(accepted), fx)
    }

    /// The followed-peer-tip the wire stream would observe for the durable tip:
    /// exactly the (slot, hash, block_no) a peer sees via the serve projection.
    fn served_tip_point(chaindb: &PersistentChainDb) -> TipPoint {
        let (slot, hash, block_no) = ChainDbServedSource::new(chaindb)
            .tip()
            .expect("a durable servable tip is present");
        TipPoint {
            slot,
            hash,
            block_no,
        }
    }

    /// The canonical `prev_hash` of a forged block — read from the inner Conway
    /// header body (the `prev_hash` is NOT surfaced by `HeaderInput`, so the
    /// parent identity is read from the preserved CBOR, never inferred from the
    /// block number).
    fn forged_prev_hash(block_bytes: &[u8]) -> PrevHash {
        let decoded = decode_block(block_bytes).expect("forged block decodes");
        let inner = &block_bytes[decoded.inner_start..decoded.inner_end];
        let preserved =
            ade_codec::conway::decode_conway_block(inner).expect("inner conway block decodes");
        preserved.decoded().header.body.prev_hash.clone()
    }

    #[test]
    fn forge_on_followed_tip_proceeds_with_parent_byte_equal() {
        // DC-NODE-15 + DC-CONS-24: with the durable servable tip == the followed
        // peer tip T, the gate is CaughtUp and the forge proceeds; the forged
        // successor's prev_hash byte-equals T and its block_no == T.block_no + 1.
        let mut shell = synth_shell(0x21, 0x22, 0x23);
        // The follow durably stored the peer tip T (here: forge block 0 + admit
        // via the SAME pump_block chokepoint the follow uses — a servable
        // StoredBlock). T is the durable servable tip.
        let (h0, fx0) = forge_block0(&mut shell, 1);
        let (_dir, chaindb, mut wal, mut state) = durable_store();
        let t = admit_forged_block_durably(
            &h0,
            &mut state,
            &chaindb,
            &mut wal,
            &fx0.era_schedule,
            &fx0.pool_distr_view,
        )
        .expect("durable admit of the followed tip T")
        .expect("T advanced the durable tip");

        // The followed peer tip == the durable servable tip (a caught-up node).
        let followed_peer_tip = served_tip_point(&chaindb);
        let durable_servable_tip = served_tip_point(&chaindb);
        assert_eq!(
            forge_followed_tip_admission(
                Some(durable_servable_tip.clone()),
                Some(followed_peer_tip.clone())
            ),
            ForgeFollowedTipAdmission::CaughtUp,
            "durable servable tip == followed peer tip ⇒ CaughtUp (forge admissible)"
        );

        // Forge the successor T+1 ON the durable tip T (the CaughtUp forge base).
        let fx1 = Eligible::build(&shell, 2, EpochNo(0));
        let ctx1 = ForgeRequestContext {
            eta0: &state.receive.chain_dep.epoch_nonce,
            vrf_vk: &fx1.vrf_vk,
            leader_schedule_answer: &fx1.answer,
            pparams: &fx1.pparams,
            base_state: &state.receive.ledger,
            chain_dep_state: &state.receive.chain_dep,
            era_schedule: &fx1.era_schedule,
            pool_distr_view: &fx1.pool_distr_view,
            block_number: BlockNo(followed_peer_tip.block_no + 1),
            prev_hash: PrevHash::Block(t.hash.clone()),
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
            prev_opcert_counter: None,
        };
        let (ev1, hd1) = run_real_forge(2, 0, &ctx1, &mut shell);
        let accepted1 = match (ev1, hd1) {
            (CoordinatorEvent::ForgeSucceeded { .. }, Some(a)) => a,
            (ev, _) => panic!("expected T+1 ForgeSucceeded on the followed tip, got {ev:?}"),
        };
        let forged = decode_block(accepted1.as_bytes()).expect("forged successor decodes");

        // DC-CONS-24: the forged parent byte-equals the followed peer tip hash —
        // parent identity is the canonical hash, never inferred from block_no.
        match forged_prev_hash(accepted1.as_bytes()) {
            PrevHash::Block(parent) => assert_eq!(
                parent, followed_peer_tip.hash,
                "forged prev_hash byte-equals the followed peer tip hash (DC-CONS-24)"
            ),
            PrevHash::Genesis => panic!("the successor on a followed tip must not be Genesis"),
        }
        assert_eq!(
            forged.header_input.block_no.0,
            followed_peer_tip.block_no + 1,
            "forged block_no == followed_peer_tip.block_no + 1 (DC-CONS-24)"
        );
    }

    #[test]
    fn served_chain_intersects_at_followed_tip_and_rolls_to_forged() {
        // DC-NODE-14 (followed-tip lineage clause): after the follow stored the
        // peer tip T as a servable StoredBlock and the gated forge admitted T+1
        // on it, a peer at T can FindIntersect there and roll forward onto Ade's
        // forged successor: intersect([T]) == Some(T) and next_after(T) == T+1.
        let mut shell = synth_shell(0x31, 0x32, 0x33);
        let (h0, fx0) = forge_block0(&mut shell, 1);
        let (_dir, chaindb, mut wal, mut state) = durable_store();
        let t = admit_forged_block_durably(
            &h0,
            &mut state,
            &chaindb,
            &mut wal,
            &fx0.era_schedule,
            &fx0.pool_distr_view,
        )
        .expect("durable admit of the followed tip T")
        .expect("T advanced the durable tip");

        // Gate is CaughtUp (durable == followed), then forge + admit T+1.
        let followed_peer_tip = served_tip_point(&chaindb);
        assert_eq!(
            forge_followed_tip_admission(
                Some(served_tip_point(&chaindb)),
                Some(followed_peer_tip.clone())
            ),
            ForgeFollowedTipAdmission::CaughtUp,
        );
        let fx1 = Eligible::build(&shell, 2, EpochNo(0));
        let ctx1 = ForgeRequestContext {
            eta0: &state.receive.chain_dep.epoch_nonce,
            vrf_vk: &fx1.vrf_vk,
            leader_schedule_answer: &fx1.answer,
            pparams: &fx1.pparams,
            base_state: &state.receive.ledger,
            chain_dep_state: &state.receive.chain_dep,
            era_schedule: &fx1.era_schedule,
            pool_distr_view: &fx1.pool_distr_view,
            block_number: BlockNo(followed_peer_tip.block_no + 1),
            prev_hash: PrevHash::Block(t.hash.clone()),
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
            prev_opcert_counter: None,
        };
        let (ev1, hd1) = run_real_forge(2, 0, &ctx1, &mut shell);
        let h1 = match (ev1, hd1) {
            (CoordinatorEvent::ForgeSucceeded { .. }, Some(a)) => {
                SelfAcceptedHandoff::from_self_accepted(a)
            }
            (ev, _) => panic!("expected T+1 ForgeSucceeded, got {ev:?}"),
        };
        let forged_tip = admit_forged_block_durably(
            &h1,
            &mut state,
            &chaindb,
            &mut wal,
            &fx1.era_schedule,
            &fx1.pool_distr_view,
        )
        .expect("durable admit of the forged successor T+1")
        .expect("T+1 advanced the durable tip");

        // The served chain intersects at the followed tip T.
        let served = ChainDbServedSource::new(&chaindb);
        let isect = served.intersect(&[Point::Block {
            slot: t.slot,
            hash: t.hash.clone(),
        }]);
        assert_eq!(
            isect,
            Some((t.slot, t.hash.clone())),
            "intersect([T]) == Some(T) — the peer can FindIntersect at the followed tip"
        );

        // next_after(T) projects the forged successor T+1 (the peer rolls forward
        // onto Ade's forged block).
        let next = served
            .next_after(Some((t.slot, t.hash.clone())))
            .expect("next_after(T) projects the forged successor");
        assert_eq!(
            next.slot, forged_tip.slot,
            "next_after(T).slot == forged T+1 slot"
        );
        assert_eq!(
            next.hash, forged_tip.hash,
            "next_after(T).hash == forged T+1 hash"
        );
        assert_eq!(
            next.block_no,
            followed_peer_tip.block_no + 1,
            "next_after(T).block_no == T.block_no + 1 (the forged successor)"
        );
    }

    #[test]
    fn recover_follow_forge_two_runs_byte_identical() {
        // T-REC-05 (strengthened): the served chain + forged successor are a
        // deterministic function of (recovered/followed base, forged successor).
        // Two independent runs over the same inputs produce a byte-identical
        // served tip, intersect point, and forged successor bytes.
        fn one_run() -> (TipPoint, Vec<u8>) {
            let mut shell = synth_shell(0x41, 0x42, 0x43);
            let (h0, fx0) = forge_block0(&mut shell, 1);
            let (_dir, chaindb, mut wal, mut state) = durable_store();
            let t = admit_forged_block_durably(
                &h0,
                &mut state,
                &chaindb,
                &mut wal,
                &fx0.era_schedule,
                &fx0.pool_distr_view,
            )
            .expect("admit T")
            .expect("T tip");
            let followed_peer_tip = served_tip_point(&chaindb);
            assert_eq!(
                forge_followed_tip_admission(
                    Some(served_tip_point(&chaindb)),
                    Some(followed_peer_tip.clone())
                ),
                ForgeFollowedTipAdmission::CaughtUp,
            );
            let fx1 = Eligible::build(&shell, 2, EpochNo(0));
            let ctx1 = ForgeRequestContext {
                eta0: &state.receive.chain_dep.epoch_nonce,
                vrf_vk: &fx1.vrf_vk,
                leader_schedule_answer: &fx1.answer,
                pparams: &fx1.pparams,
                base_state: &state.receive.ledger,
                chain_dep_state: &state.receive.chain_dep,
                era_schedule: &fx1.era_schedule,
                pool_distr_view: &fx1.pool_distr_view,
                block_number: BlockNo(followed_peer_tip.block_no + 1),
                prev_hash: PrevHash::Block(t.hash.clone()),
                protocol_version: ProtocolVersion { major: 9, minor: 0 },
                prev_opcert_counter: None,
            };
            let (ev1, hd1) = run_real_forge(2, 0, &ctx1, &mut shell);
            let h1 = match (ev1, hd1) {
                (CoordinatorEvent::ForgeSucceeded { .. }, Some(a)) => {
                    SelfAcceptedHandoff::from_self_accepted(a)
                }
                (ev, _) => panic!("expected T+1 ForgeSucceeded, got {ev:?}"),
            };
            let forged_bytes = h1.accepted().as_bytes().to_vec();
            admit_forged_block_durably(
                &h1,
                &mut state,
                &chaindb,
                &mut wal,
                &fx1.era_schedule,
                &fx1.pool_distr_view,
            )
            .expect("admit T+1")
            .expect("T+1 tip");
            (served_tip_point(&chaindb), forged_bytes)
        }

        let (tip_a, forged_a) = one_run();
        let (tip_b, forged_b) = one_run();
        assert_eq!(
            tip_a, tip_b,
            "the served tip is byte-identical across runs (T-REC-05)"
        );
        assert_eq!(
            forged_a, forged_b,
            "the forged successor bytes are byte-identical across runs (T-REC-05)"
        );
    }

    // =======================================================================
    // RED — AE.B (Gap 2, end-to-end): a forged successor on the recovered anchor
    // is not peer-adoptable. Stays #[ignore]d until PHASE4-N-AE.B.
    // =======================================================================

    #[test]
    // Named historically (was RED, demonstrating non-adoptability). AE.B Option B
    // FLIPS it green: the forged successor on the recovered anchor IS now
    // peer-adoptable via the proof-gated FindIntersect-only projection.
    fn forged_successor_on_recovered_anchor_is_not_peer_adoptable() {
        // A modest non-Origin anchor (epoch 0, so the always-eligible leadership
        // fixture locates cleanly). The bug does not depend on the slot magnitude.
        let anchor_slot = SlotNo(50);
        let anchor = Hash32([0xD1; 32]);
        let anchor_block_no = 7u64;
        let successor_slot = 51u64;
        let epoch = EpochNo(0);

        let db = recover_at_anchor(anchor_slot);

        // Forge the successor ON the recovered anchor — exactly the pre-AE.A
        // node_lifecycle.rs:1102 fallback base: prev_hash = Block(anchor),
        // block_no = anchor + 1.
        let mut shell = synth_shell(0x77, 0x88, 0x99);
        let lead = Eligible::build(&shell, successor_slot, epoch);
        let ctx = ForgeRequestContext {
            eta0: &lead.chain_dep.epoch_nonce,
            vrf_vk: &lead.vrf_vk,
            leader_schedule_answer: &lead.answer,
            pparams: &lead.pparams,
            base_state: &lead.base_state,
            chain_dep_state: &lead.chain_dep,
            era_schedule: &lead.era_schedule,
            pool_distr_view: &lead.pool_distr_view,
            block_number: BlockNo(anchor_block_no + 1),
            prev_hash: PrevHash::Block(anchor.clone()),
            protocol_version: ProtocolVersion { major: 9, minor: 0 },
            prev_opcert_counter: None,
        };

        let (event, _handoff) = run_real_forge(successor_slot, 0, &ctx, &mut shell);
        let artifact = match event {
            CoordinatorEvent::ForgeSucceeded { artifact, .. } => artifact,
            other => panic!(
                "diag setup: expected a ForgeSucceeded successor on the recovered anchor, got {other:?}"
            ),
        };
        let decoded = decode_block(&artifact.bytes).expect("the forged successor decodes");
        let forged_block_no = decoded.header_input.block_no.0;
        let forged_hash = Hash32(artifact.hash);
        let forged_parent = anchor.clone();

        // Admit the forged successor as a SERVABLE StoredBlock (the real flow
        // admits it durably via admit_forged_block_durably -> pump_block ->
        // put_block; here put_block directly populates the served store the
        // projection reads — this fixture exercises the SERVE projection).
        db.put_block(&StoredBlock {
            slot: SlotNo(successor_slot),
            hash: forged_hash.clone(),
            bytes: artifact.bytes.clone(),
        })
        .expect("the forged successor is a servable StoredBlock");

        let served = ChainDbServedSource::new(&db);
        let parent_servable =
            ChainDb::get_block_by_hash(&db, &forged_parent).expect("get_block_by_hash");
        let isect = served.intersect(&[Point::Block {
            slot: anchor_slot,
            hash: forged_parent.clone(),
        }]);
        let next = served.next_after(Some((anchor_slot, forged_parent.clone())));
        let parent_bytes = ServedRangeLookup::range_bytes(
            &served,
            (anchor_slot, forged_parent.clone()),
            (anchor_slot, forged_parent.clone()),
        );

        eprintln!("[AE.B diag] forged successor on the recovered anchor + durable admit:");
        field("recovered/forged_parent_hash", hash_hex(&forged_parent));
        field("forged_parent_block_no (anchor)", anchor_block_no.to_string());
        field("forged_block_no", forged_block_no.to_string());
        field("forged_block_hash", hash_hex(&forged_hash));
        field(
            "get_block_by_hash(forged_parent)",
            format!("{:?}", parent_servable.as_ref().map(|b| b.slot)),
        );
        field("intersect(forged_parent)", format!("{isect:?}"));
        field(
            "next_after(forged_parent)",
            format!("{:?}", next.as_ref().map(|h| hash_hex(&h.hash))),
        );

        assert_eq!(
            forged_block_no,
            anchor_block_no + 1,
            "the forge built the successor (anchor_block_no + 1) on the recovered tip"
        );
        // AE.B Option B (DC-NODE-14 anchor clause): the forged parent (the recovered
        // anchor) IS peer-intersectable via the proof-gated FindIntersect-only
        // projection (the earliest servable StoredBlock's prev_hash == the anchor).
        assert_eq!(
            isect,
            Some((anchor_slot, forged_parent.clone())),
            "the forged successor's parent (recovered anchor) is FindIntersect-able (proof-gated)"
        );
        // ...and a relay that intersects there rolls forward onto the forged successor.
        let next = next.expect("next_after(forged_parent) projects the forged successor");
        assert_eq!(
            next.hash, forged_hash,
            "next_after(parent) == the forged successor hash"
        );
        assert_eq!(
            next.slot,
            SlotNo(successor_slot),
            "next_after(parent) slot == the forged successor slot"
        );
        // Hard boundary (no synthetic bytes): the projected parent is NEVER a
        // StoredBlock and BlockFetch of it refuses structurally (serves no bytes).
        assert!(
            parent_servable.is_none(),
            "the recovered anchor is never materialized as a StoredBlock (no synthetic bytes)"
        );
        assert!(
            parent_bytes.is_empty(),
            "BlockFetch of the projected parent refuses structurally (no bytes served)"
        );
    }
}
