// PHASE4-N-AI AI-S1 — rollback WAL durability foundation (CE-AI-1).
//
// Proves: WalEntry::RollBack canonical round-trip + fail-closed decode;
// rollback-aware fp replay recovers the SELECTED chain (never the
// abandoned branch) without requiring the abandoned block's bytes; the
// re-anchor target equals the fp materialized by the EXISTING
// materialize_rolled_back_state authority (hard line 4); verify_chain
// accepts a recorded rollback and rejects an out-of-chain target.

use std::collections::BTreeMap;

use ade_core::consensus::era_schedule::EraSchedule;
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_core::consensus::{BootstrapAnchorHash, EraSummary};
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::rollback::{
    materialize_rolled_back_state, BlockSource, SnapshotReader, TargetPoint,
};
use ade_ledger::state::LedgerState;
use ade_ledger::wal::{
    decode_wal_entry, encode_wal_entry, replay_from_anchor, BlockVerdictTag, RollbackPoint,
    RollbackReason, WalEntry, WalError, WalStore,
};
use ade_testkit::consensus::ledger_view_stub::LedgerViewStub;
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash32, SlotNo};

// ---------- helpers ----------

fn h(b: u8) -> Hash32 {
    Hash32([b; 32])
}

fn admit(prior: u8, post: u8, block_hash: u8, slot: u64) -> WalEntry {
    WalEntry::AdmitBlock {
        prior_fp: h(prior),
        block_hash: h(block_hash),
        slot: SlotNo(slot),
        verdict: BlockVerdictTag::Valid,
        post_fp: h(post),
    }
}

fn point(slot: u64, hash: u8, block_no: u64) -> RollbackPoint {
    RollbackPoint {
        slot: SlotNo(slot),
        hash: h(hash),
        block_no: BlockNo(block_no),
    }
}

fn bb(hashes: &[u8]) -> BTreeMap<Hash32, Vec<u8>> {
    let mut m = BTreeMap::new();
    for b in hashes {
        m.insert(h(*b), vec![*b]);
    }
    m
}

/// In-memory WalStore for exercising the default `verify_chain`.
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

/// a1 (fork point) -> a2 (abandoned) -> RollBack(to a1) -> b1 -> b2
/// (selected). a2's bytes are intentionally absent from the byte map.
fn rollback_wal() -> Vec<WalEntry> {
    vec![
        admit(0xA0, 0xB0, 0xA1, 10), // a1 @ slot 10, post=0xB0 (fork point)
        admit(0xB0, 0xAB, 0xA2, 11), // a2 @ slot 11 (abandoned)
        WalEntry::RollBack {
            to_point: point(10, 0xA1, 1),
            reason: RollbackReason::ForkChoiceWin,
            prior_tip: point(11, 0xA2, 2),
            selected_tip: point(12, 0xB2, 2),
        },
        admit(0xB0, 0xC1, 0xB1, 11), // b1 @ slot 11 (selected) prior=0xB0=a1.post
        admit(0xC1, 0xC2, 0xB2, 12), // b2 @ slot 12 (selected)
    ]
}

// ---------- 1. encoding (T-ENC-02 / T-ENC-03) ----------

#[test]
fn wal_rollback_entry_round_trips_canonical_cbor() {
    let e = WalEntry::RollBack {
        to_point: point(10, 0xA1, 1),
        reason: RollbackReason::ForkChoiceWin,
        prior_tip: point(11, 0xA2, 2),
        selected_tip: point(12, 0xB2, 2),
    };
    let bytes = encode_wal_entry(&e);
    let (decoded, consumed) = decode_wal_entry(&bytes).expect("decode");
    assert_eq!(consumed, bytes.len());
    assert_eq!(decoded, e);
    assert_eq!(encode_wal_entry(&decoded), bytes); // re-encode byte-identical
}

#[test]
fn wal_decode_rejects_unknown_tag() {
    // array(2)[ uint 5, array(0)[] ] — tag 5 is not a known WalEntry.
    let bytes = [0x82u8, 0x05, 0x80];
    let err = decode_wal_entry(&bytes).expect_err("must reject unknown tag");
    assert!(matches!(err, WalError::Structural { .. }), "got {err:?}");
}

#[test]
fn wal_decode_rejects_noncanonical_rollback() {
    // tag 1 (RollBack) with an INDEFINITE-length payload array — the
    // WAL forbids indefinite-length arrays.
    let bytes = [0x82u8, 0x01, 0x9f];
    let err = decode_wal_entry(&bytes).expect_err("must reject indefinite");
    assert!(matches!(err, WalError::Structural { .. }), "got {err:?}");
}

#[test]
fn wal_decode_rejects_malformed_rollback_payload() {
    let e = WalEntry::RollBack {
        to_point: point(10, 0xA1, 1),
        reason: RollbackReason::PeerRollBackward,
        prior_tip: point(11, 0xA2, 2),
        selected_tip: point(12, 0xB2, 2),
    };
    let mut bytes = encode_wal_entry(&e);
    bytes.truncate(bytes.len() - 5); // chop into the selected_tip
    let err = decode_wal_entry(&bytes).expect_err("must reject truncated");
    assert!(
        matches!(err, WalError::Decode(_) | WalError::Structural { .. }),
        "got {err:?}"
    );
}

#[test]
fn rollback_reason_wire_code_is_closed() {
    assert_eq!(
        RollbackReason::from_wire_code(0),
        Some(RollbackReason::ForkChoiceWin)
    );
    assert_eq!(
        RollbackReason::from_wire_code(1),
        Some(RollbackReason::PeerRollBackward)
    );
    assert_eq!(RollbackReason::from_wire_code(2), None);
    assert_eq!(RollbackReason::from_wire_code(99), None);
}

// ---------- 2. rollback-aware fp replay (CE-AI-1) ----------

#[test]
fn replay_with_rollback_recovers_selected_not_abandoned() {
    let anchor = h(0xA0);
    let entries = rollback_wal();
    let bbm = bb(&[0xA1, 0xB1, 0xB2]); // a2 (0xA2) deliberately omitted
    let out = replay_from_anchor(&anchor, &entries, &bbm).expect("replay ok");
    assert_eq!(out.tail_fp, h(0xC2), "tail must be the SELECTED b2 post_fp");
    assert_ne!(out.tail_fp, h(0xAB), "tail must NOT be the abandoned a2 post_fp");
    assert_eq!(out.admit_count, 3, "a1 + b1 + b2 effective; a2 superseded");
}

#[test]
fn replay_rollback_missing_abandoned_bytes_is_ok() {
    // The abandoned branch's bytes being absent does NOT cause
    // BlockBytesMissing (it is superseded).
    let anchor = h(0xA0);
    let entries = rollback_wal();
    let bbm = bb(&[0xA1, 0xB1, 0xB2]); // no 0xA2
    assert!(replay_from_anchor(&anchor, &entries, &bbm).is_ok());
}

#[test]
fn replay_with_rollback_two_runs_byte_identical() {
    let anchor = h(0xA0);
    let entries = rollback_wal();
    let bbm = bb(&[0xA1, 0xB1, 0xB2]);
    let a = replay_from_anchor(&anchor, &entries, &bbm).expect("a");
    let b = replay_from_anchor(&anchor, &entries, &bbm).expect("b");
    assert_eq!(a, b);
}

#[test]
fn replay_rollback_target_not_in_chain_fails_closed() {
    let anchor = h(0xA0);
    // RollBack to_point slot 10 present but WRONG hash (0x99) -> the
    // (slot, hash) re-anchor lookup misses -> fail closed.
    let entries = vec![
        admit(0xA0, 0xB0, 0xA1, 10),
        WalEntry::RollBack {
            to_point: point(10, 0x99, 1),
            reason: RollbackReason::ForkChoiceWin,
            prior_tip: point(10, 0xA1, 1),
            selected_tip: point(11, 0xB1, 2),
        },
        admit(0xB0, 0xC1, 0xB1, 11),
    ];
    let bbm = bb(&[0xA1, 0xB1]);
    let err = replay_from_anchor(&anchor, &entries, &bbm).expect_err("must fail closed");
    assert!(
        matches!(err, WalError::RollbackTargetNotInChain { .. }),
        "got {err:?}"
    );
}

#[test]
fn replay_linear_wal_unaffected_by_rollback_support() {
    // Regression: a WAL with no RollBack entries replays exactly as before.
    let anchor = h(0x01);
    let entries = vec![admit(0x01, 0x02, 0xA1, 100), admit(0x02, 0x03, 0xA2, 101)];
    let bbm = bb(&[0xA1, 0xA2]);
    let out = replay_from_anchor(&anchor, &entries, &bbm).expect("ok");
    assert_eq!(out.tail_fp, h(0x03));
    assert_eq!(out.admit_count, 2);
}

// ---------- 3. verify_chain (trait default, rollback-aware) ----------

#[test]
fn verify_chain_accepts_recorded_rollback() {
    let mut wal = VecWal::default();
    for e in rollback_wal() {
        wal.append(e).unwrap();
    }
    wal.verify_chain(&h(0xA0))
        .expect("verify_chain accepts a recorded rollback");
}

#[test]
fn verify_chain_rejects_rollback_target_not_in_chain() {
    let mut wal = VecWal::default();
    wal.append(admit(0xA0, 0xB0, 0xA1, 10)).unwrap();
    wal.append(WalEntry::RollBack {
        to_point: point(10, 0x99, 1), // wrong hash
        reason: RollbackReason::ForkChoiceWin,
        prior_tip: point(10, 0xA1, 1),
        selected_tip: point(11, 0xB1, 2),
    })
    .unwrap();
    let err = wal.verify_chain(&h(0xA0)).expect_err("must fail closed");
    assert!(
        matches!(err, WalError::RollbackTargetNotInChain { .. }),
        "got {err:?}"
    );
}

// ---------- 4. Layer 2: re-anchor fp == materialize_rolled_back_state fp ----------

struct OneSnapshotReader {
    slot: SlotNo,
    ledger: LedgerState,
    chain_dep: PraosChainDepState,
}
impl SnapshotReader for OneSnapshotReader {
    fn nearest_le(
        &self,
        target_slot: SlotNo,
    ) -> Option<(SlotNo, LedgerState, PraosChainDepState)> {
        if self.slot <= target_slot {
            Some((self.slot, self.ledger.clone(), self.chain_dep.clone()))
        } else {
            None
        }
    }
}

struct EmptySource;
impl BlockSource for EmptySource {
    fn blocks_in_range(&self, _f: SlotNo, _t: SlotNo) -> Vec<(SlotNo, Vec<u8>)> {
        Vec::new()
    }
}

fn one_era_schedule() -> EraSchedule {
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

#[test]
fn rollback_replay_reanchor_fp_equals_materialized_fp() {
    // The rolled-back-to ledger L at slot 10.
    let ledger = LedgerState::new(CardanoEra::Conway);
    let fp_l = fingerprint(&ledger).combined;

    // (a) materialize_rolled_back_state @ slot 10 (degenerate
    //     snapshot-at-target) re-invokes the EXISTING authority and
    //     yields a ledger whose fingerprint is fp_l.
    let reader = OneSnapshotReader {
        slot: SlotNo(10),
        ledger: ledger.clone(),
        chain_dep: PraosChainDepState::empty(),
    };
    let view = LedgerViewStub::new();
    let (got_ledger, _got_cd) = materialize_rolled_back_state(
        TargetPoint {
            slot: SlotNo(10),
            hash: h(0xA1),
        },
        &reader,
        &EmptySource,
        &one_era_schedule(),
        &view,
        None,
    )
    .expect("materialize ok");
    assert_eq!(fingerprint(&got_ledger).combined, fp_l);

    // (b) A WAL whose AdmitBlock at the fork point (slot 10) has
    //     post_fp = fp_l. The RollBack re-anchors the fp chain to that
    //     in-chain post_fp; the post-rollback b1 chains from fp_l.
    let anchor = h(0xA0);
    let entries = vec![
        WalEntry::AdmitBlock {
            prior_fp: anchor.clone(),
            block_hash: h(0xA1),
            slot: SlotNo(10),
            verdict: BlockVerdictTag::Valid,
            post_fp: fp_l.clone(), // == materialized fp
        },
        WalEntry::RollBack {
            to_point: point(10, 0xA1, 1),
            reason: RollbackReason::ForkChoiceWin,
            prior_tip: point(11, 0xA2, 2),
            selected_tip: point(11, 0xB1, 2),
        },
        WalEntry::AdmitBlock {
            prior_fp: fp_l, // chains from the rolled-back (materialized) fp
            block_hash: h(0xB1),
            slot: SlotNo(11),
            verdict: BlockVerdictTag::Valid,
            post_fp: h(0xC1),
        },
    ];
    let bbm = bb(&[0xA1, 0xB1]);
    let out = replay_from_anchor(&anchor, &entries, &bbm).expect("replay ok");
    // b1.prior_fp == fp_l matched without ChainBreak, proving the
    // re-anchor target IS the materialized fp — never a blindly-trusted
    // recorded rollback fp (the entry carries no fp).
    assert_eq!(out.tail_fp, h(0xC1));
    assert_eq!(out.admit_count, 2);
}
