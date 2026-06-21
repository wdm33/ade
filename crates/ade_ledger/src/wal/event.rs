// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE WAL closed-sum entries (PHASE4-N-M-A S3; extended
//! PHASE4-N-F-A A3a).
//!
//! Two semantic classes of entry coexist in the same WAL:
//!
//! - `AdmitBlock` — a **block/ledger transition**. Carries
//!   fingerprint deltas: `prior_fp` MUST equal the previous
//!   `AdmitBlock`'s `post_fp` (or the anchor's
//!   `initial_ledger_fingerprint` for the first one). DC-WAL-02.
//! - `SeedEpochConsensusInputsImported` — a **bootstrap
//!   provenance event** (PHASE4-N-F-A A3a). It records that the
//!   seed-epoch consensus-input sidecar was imported and bound to
//!   the anchor; it does **not** participate in the
//!   `prior_fp`/`post_fp` block-transition chain.
//!
//! Tags are append-only. `1` and `2` remain **reserved** for the
//! future `RollBackward` / `CaptureSnapshot` entries and are not
//! reused for a different semantic class.

use ade_codec::cbor::{
    canonical_width, read_array_header, read_bytes, read_uint, write_array_header,
    write_bytes_canonical, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_types::{BlockNo, CardanoEra, EpochNo, Hash32, SlotNo};
use ade_core::consensus::events::Point;
use crate::reduced_snapshot::SnapshotPhase;

use super::error::WalError;

/// Wire tag for `WalEntry::AdmitBlock`.
pub const TAG_ADMIT_BLOCK: u64 = 0;
/// Wire tag for `WalEntry::RollBack` (PHASE4-N-AI). Tag 1 is RollBack,
/// fulfilling the previously reserved RollBackward slot. Tag 2
/// (CaptureSnapshot) remains RESERVED; never reused.
pub const TAG_ROLLBACK: u64 = 1;
/// Wire tag for `WalEntry::SeedEpochConsensusInputsImported`
/// (PHASE4-N-F-A A3a). Append-only: the next free tag after the
/// reserved 1/2.
pub const TAG_SEED_EPOCH_CONSENSUS_INPUTS_IMPORTED: u64 = 3;
/// Wire tag for `WalEntry::EpochConsensusViewActivated` (EPOCH-CONSENSUS-VIEW S3f-4a).
/// Append-only: the next free tag after 0/1/3 (2 reserved).
pub const TAG_EPOCH_CONSENSUS_VIEW_ACTIVATED: u64 = 4;

/// Closed sum: every authority-affecting forward step recorded
/// in the WAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalEntry {
    /// One admit pass through CN-CONS-08 produced a new
    /// canonical ledger state. `prior_fp` is the chain link
    /// back to the previous step (or the anchor for the first
    /// entry).
    AdmitBlock {
        prior_fp: Hash32,
        block_hash: Hash32,
        slot: SlotNo,
        verdict: BlockVerdictTag,
        post_fp: Hash32,
    },
    /// PHASE4-N-F-A A3a — bootstrap provenance event: the
    /// seed-epoch consensus-input sidecar (keyed by `anchor_fp`)
    /// was imported during verified bootstrap. `sidecar_hash` is
    /// `blake2b_256` of the exact A1 canonical sidecar bytes.
    /// This entry does **not** carry `prior_fp`/`post_fp` — it is
    /// not a block transition and never advances the fingerprint
    /// chain. Exactly one per store/anchor (replay fails closed
    /// on a duplicate or an `anchor_fp` that mismatches the
    /// replay anchor).
    SeedEpochConsensusInputsImported {
        anchor_fp: Hash32,
        sidecar_hash: Hash32,
        epoch_no: EpochNo,
    },
    /// PHASE4-N-AI AI-S1 (OQ-1 mechanism A): a DURABLE MARKER that a
    /// rollback to `to_point` occurred. It records that a rollback
    /// happened; it does NOT decide or materialize one. Replay
    /// re-anchors the fingerprint chain to `to_point`'s in-chain
    /// `post_fp` (fp-only); the recovery/materialize layer re-invokes
    /// the EXISTING `materialize_rolled_back_state` + lockstep
    /// authority. `prior_tip` / `selected_tip` / `reason` are
    /// audit/reconciliation fields ONLY — replay never sets the durable
    /// tip from `selected_tip`. Tag 1 (the reserved RollBackward slot);
    /// append-only (CN-WAL-01); re-anchors (does not extend) the
    /// AdmitBlock fingerprint chain.
    RollBack {
        to_point: RollbackPoint,
        reason: RollbackReason,
        prior_tip: RollbackPoint,
        selected_tip: RollbackPoint,
    },
    /// EPOCH-CONSENSUS-VIEW S3f-4a (DC-EPOCH-04 / DC-EPOCH-06): the durable proof that
    /// THIS exact `EpochConsensusView` became authoritative for `target_epoch` at THIS
    /// exact selected-chain transition. Records the ENTIRE activation identity (not just
    /// hash + point) so a replay can prove idempotence (byte-identical record for the same
    /// target epoch) or detect a structured conflict (any differing binding/hash). Like
    /// `SeedEpochConsensusInputsImported` it carries no `prior_fp`/`post_fp` — it is not a
    /// block transition and never advances the fingerprint chain.
    EpochConsensusViewActivated {
        target_epoch: EpochNo,
        network_magic: u32,
        era: CardanoEra,
        transition_point: Point,
        source_checkpoint_commitment: Hash32,
        snapshot_phase: SnapshotPhase,
        nonce_commitment: Hash32,
        stake_view_canonical_hash: Hash32,
        view_canonical_hash: Hash32,
    },
}

/// Wire code for a `SnapshotPhase` in the WAL (0=Mark, 1=Set, 2=Go). An unknown code fails
/// closed on decode.
fn snapshot_phase_wire(p: SnapshotPhase) -> u64 {
    match p {
        SnapshotPhase::Mark => 0,
        SnapshotPhase::Set => 1,
        SnapshotPhase::Go => 2,
    }
}
fn snapshot_phase_from_wire(code: u64) -> Option<SnapshotPhase> {
    match code {
        0 => Some(SnapshotPhase::Mark),
        1 => Some(SnapshotPhase::Set),
        2 => Some(SnapshotPhase::Go),
        _ => None,
    }
}

/// Whether replaying `new` after `existing` is idempotent or a structured conflict, for an
/// activation of the SAME target epoch (DC-EPOCH-04: at most one canonically bound view may
/// activate per target epoch). `None` when the pair is not a same-target-epoch activation
/// pair (the caller does not apply this rule). Byte-identical (structural equality of the
/// whole record) ⇒ idempotent; any differing binding/hash ⇒ conflict (fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationReplayOutcome {
    Idempotent,
    Conflict,
}
pub fn activation_replay_outcome(existing: &WalEntry, new: &WalEntry) -> Option<ActivationReplayOutcome> {
    match (existing, new) {
        (
            WalEntry::EpochConsensusViewActivated { target_epoch: e1, .. },
            WalEntry::EpochConsensusViewActivated { target_epoch: e2, .. },
        ) if e1 == e2 => Some(if existing == new {
            ActivationReplayOutcome::Idempotent
        } else {
            ActivationReplayOutcome::Conflict
        }),
        _ => None,
    }
}

/// A chain point recorded in a `WalEntry::RollBack` — slot + hash +
/// block_no. Carries `block_no` so an auditor can compute rollback
/// depth without a separate lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackPoint {
    pub slot: SlotNo,
    pub hash: Hash32,
    pub block_no: BlockNo,
}

/// Closed reason a `WalEntry::RollBack` was recorded. uint wire code;
/// an unknown code fails closed on decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackReason {
    /// A peer-origin chain won Praos fork-choice (DC-CONS-03).
    ForkChoiceWin,
    /// The followed peer's chain-sync delivered a RollBackward.
    PeerRollBackward,
}

impl RollbackReason {
    pub fn wire_code(self) -> u64 {
        match self {
            Self::ForkChoiceWin => 0,
            Self::PeerRollBackward => 1,
        }
    }
    pub fn from_wire_code(code: u64) -> Option<Self> {
        match code {
            0 => Some(Self::ForkChoiceWin),
            1 => Some(Self::PeerRollBackward),
            _ => None,
        }
    }
}

/// Closed tag for the block-validity verdict. Mirrors the BLUE
/// `BlockValidityVerdict` discriminant. Compact: no payload
/// needed for replay-equivalence — the post_fp encodes the
/// effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockVerdictTag {
    Valid,
    Invalid,
}

impl BlockVerdictTag {
    pub fn wire_code(self) -> u64 {
        match self {
            Self::Valid => 0,
            Self::Invalid => 1,
        }
    }
    pub fn from_wire_code(code: u64) -> Option<Self> {
        match code {
            0 => Some(Self::Valid),
            1 => Some(Self::Invalid),
            _ => None,
        }
    }
}

// NOTE (PHASE4-N-F-A A3a): there are deliberately **no**
// `prior_fp()`/`post_fp()` accessors. With two semantic classes
// of entry, only `AdmitBlock` participates in the
// fingerprint chain. Both chain walks (`replay_from_anchor`,
// `WalStore::verify_chain`) `match` on the variant explicitly, so
// a future third variant is a compile error in every walk rather
// than being silently treated as a (missing) chain link.

/// Canonical CBOR encode for a single entry. Wire shape:
/// ```text
/// array(2) [ uint TAG, payload ]
///
/// AdmitBlock payload (TAG=0):
///   array(5) [
///     bytes(32) prior_fp,
///     bytes(32) block_hash,
///     uint slot,
///     uint verdict_code,
///     bytes(32) post_fp,
///   ]
///
/// SeedEpochConsensusInputsImported payload (TAG=3):
///   array(3) [
///     bytes(32) anchor_fp,
///     bytes(32) sidecar_hash,
///     uint epoch_no,
///   ]
/// ```
pub fn encode_wal_entry(entry: &WalEntry) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
    match entry {
        WalEntry::AdmitBlock {
            prior_fp,
            block_hash,
            slot,
            verdict,
            post_fp,
        } => {
            write_uint_canonical(&mut buf, TAG_ADMIT_BLOCK);
            write_array_header(&mut buf, ContainerEncoding::Definite(5, canonical_width(5)));
            write_bytes_canonical(&mut buf, &prior_fp.0);
            write_bytes_canonical(&mut buf, &block_hash.0);
            write_uint_canonical(&mut buf, slot.0);
            write_uint_canonical(&mut buf, verdict.wire_code());
            write_bytes_canonical(&mut buf, &post_fp.0);
        }
        WalEntry::SeedEpochConsensusInputsImported {
            anchor_fp,
            sidecar_hash,
            epoch_no,
        } => {
            write_uint_canonical(&mut buf, TAG_SEED_EPOCH_CONSENSUS_INPUTS_IMPORTED);
            write_array_header(&mut buf, ContainerEncoding::Definite(3, canonical_width(3)));
            write_bytes_canonical(&mut buf, &anchor_fp.0);
            write_bytes_canonical(&mut buf, &sidecar_hash.0);
            write_uint_canonical(&mut buf, epoch_no.0);
        }
        WalEntry::RollBack {
            to_point,
            reason,
            prior_tip,
            selected_tip,
        } => {
            write_uint_canonical(&mut buf, TAG_ROLLBACK);
            write_array_header(&mut buf, ContainerEncoding::Definite(10, canonical_width(10)));
            write_rollback_point(&mut buf, to_point);
            write_uint_canonical(&mut buf, reason.wire_code());
            write_rollback_point(&mut buf, prior_tip);
            write_rollback_point(&mut buf, selected_tip);
        }
        WalEntry::EpochConsensusViewActivated {
            target_epoch,
            network_magic,
            era,
            transition_point,
            source_checkpoint_commitment,
            snapshot_phase,
            nonce_commitment,
            stake_view_canonical_hash,
            view_canonical_hash,
        } => {
            write_uint_canonical(&mut buf, TAG_EPOCH_CONSENSUS_VIEW_ACTIVATED);
            write_array_header(&mut buf, ContainerEncoding::Definite(10, canonical_width(10)));
            write_uint_canonical(&mut buf, target_epoch.0);
            write_uint_canonical(&mut buf, *network_magic as u64);
            write_uint_canonical(&mut buf, *era as u64);
            write_uint_canonical(&mut buf, transition_point.slot.0);
            write_bytes_canonical(&mut buf, &transition_point.hash.0);
            write_bytes_canonical(&mut buf, &source_checkpoint_commitment.0);
            write_uint_canonical(&mut buf, snapshot_phase_wire(*snapshot_phase));
            write_bytes_canonical(&mut buf, &nonce_commitment.0);
            write_bytes_canonical(&mut buf, &stake_view_canonical_hash.0);
            write_bytes_canonical(&mut buf, &view_canonical_hash.0);
        }
    }
    buf
}

fn write_rollback_point(buf: &mut Vec<u8>, p: &RollbackPoint) {
    write_uint_canonical(buf, p.slot.0);
    write_bytes_canonical(buf, &p.hash.0);
    write_uint_canonical(buf, p.block_no.0);
}

/// Canonical CBOR decode for a single entry.
pub fn decode_wal_entry(bytes: &[u8]) -> Result<(WalEntry, usize), WalError> {
    let mut o = 0usize;
    expect_definite_array(bytes, &mut o, 2, "entry wrapper")?;
    let (tag, _w): (u64, IntWidth) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
    match tag {
        TAG_ADMIT_BLOCK => {
            expect_definite_array(bytes, &mut o, 5, "AdmitBlock payload")?;
            let prior_fp = read_hash32(bytes, &mut o)?;
            let block_hash = read_hash32(bytes, &mut o)?;
            let (slot, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let (verdict_code, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let verdict =
                BlockVerdictTag::from_wire_code(verdict_code).ok_or(WalError::Structural {
                    reason: "unknown verdict code",
                })?;
            let post_fp = read_hash32(bytes, &mut o)?;
            Ok((
                WalEntry::AdmitBlock {
                    prior_fp,
                    block_hash,
                    slot: SlotNo(slot),
                    verdict,
                    post_fp,
                },
                o,
            ))
        }
        TAG_SEED_EPOCH_CONSENSUS_INPUTS_IMPORTED => {
            expect_definite_array(bytes, &mut o, 3, "SeedEpochConsensusInputsImported payload")?;
            let anchor_fp = read_hash32(bytes, &mut o)?;
            let sidecar_hash = read_hash32(bytes, &mut o)?;
            let (epoch_no, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            Ok((
                WalEntry::SeedEpochConsensusInputsImported {
                    anchor_fp,
                    sidecar_hash,
                    epoch_no: EpochNo(epoch_no),
                },
                o,
            ))
        }
        TAG_ROLLBACK => {
            expect_definite_array(bytes, &mut o, 10, "RollBack payload")?;
            let to_point = read_rollback_point(bytes, &mut o)?;
            let (reason_code, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let reason =
                RollbackReason::from_wire_code(reason_code).ok_or(WalError::Structural {
                    reason: "unknown rollback reason code",
                })?;
            let prior_tip = read_rollback_point(bytes, &mut o)?;
            let selected_tip = read_rollback_point(bytes, &mut o)?;
            Ok((
                WalEntry::RollBack {
                    to_point,
                    reason,
                    prior_tip,
                    selected_tip,
                },
                o,
            ))
        }
        TAG_EPOCH_CONSENSUS_VIEW_ACTIVATED => {
            expect_definite_array(bytes, &mut o, 10, "EpochConsensusViewActivated payload")?;
            let (target_epoch, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let (network_magic, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            if network_magic > u32::MAX as u64 {
                return Err(WalError::Structural { reason: "network_magic out of u32 range" });
            }
            let (era_code, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let era = CardanoEra::ALL
                .into_iter()
                .find(|e| *e as u64 == era_code)
                .ok_or(WalError::Structural { reason: "unknown era code" })?;
            let (slot, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let transition_hash = read_hash32(bytes, &mut o)?;
            let source_checkpoint_commitment = read_hash32(bytes, &mut o)?;
            let (phase_code, _w) = read_uint(bytes, &mut o).map_err(WalError::Decode)?;
            let snapshot_phase = snapshot_phase_from_wire(phase_code)
                .ok_or(WalError::Structural { reason: "unknown snapshot phase code" })?;
            let nonce_commitment = read_hash32(bytes, &mut o)?;
            let stake_view_canonical_hash = read_hash32(bytes, &mut o)?;
            let view_canonical_hash = read_hash32(bytes, &mut o)?;
            Ok((
                WalEntry::EpochConsensusViewActivated {
                    target_epoch: EpochNo(target_epoch),
                    network_magic: network_magic as u32,
                    era,
                    transition_point: Point { slot: SlotNo(slot), hash: transition_hash },
                    source_checkpoint_commitment,
                    snapshot_phase,
                    nonce_commitment,
                    stake_view_canonical_hash,
                    view_canonical_hash,
                },
                o,
            ))
        }
        _ => Err(WalError::Structural {
            reason: "unknown wal entry tag",
        }),
    }
}

fn expect_definite_array(
    bytes: &[u8],
    offset: &mut usize,
    expected_len: u64,
    label: &'static str,
) -> Result<(), WalError> {
    let enc = read_array_header(bytes, offset).map_err(WalError::Decode)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        ContainerEncoding::Definite(_, _) => Err(WalError::Structural {
            reason: match label {
                "entry wrapper" => "entry wrapper had wrong array length",
                "AdmitBlock payload" => "AdmitBlock payload had wrong array length",
                _ => "unknown array shape",
            },
        }),
        ContainerEncoding::Indefinite => Err(WalError::Structural {
            reason: "indefinite-length array not allowed in WAL",
        }),
    }
}

fn read_hash32(bytes: &[u8], offset: &mut usize) -> Result<Hash32, WalError> {
    let (h, _w) = read_bytes(bytes, offset).map_err(WalError::Decode)?;
    if h.len() != 32 {
        return Err(WalError::Structural {
            reason: "expected 32-byte hash",
        });
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&h);
    Ok(Hash32(arr))
}

fn read_rollback_point(bytes: &[u8], offset: &mut usize) -> Result<RollbackPoint, WalError> {
    let (slot, _w) = read_uint(bytes, offset).map_err(WalError::Decode)?;
    let hash = read_hash32(bytes, offset)?;
    let (block_no, _w) = read_uint(bytes, offset).map_err(WalError::Decode)?;
    Ok(RollbackPoint {
        slot: SlotNo(slot),
        hash,
        block_no: BlockNo(block_no),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn sample() -> WalEntry {
        WalEntry::AdmitBlock {
            prior_fp: Hash32([0x55; 32]),
            block_hash: Hash32([0x66; 32]),
            slot: SlotNo(23013664),
            verdict: BlockVerdictTag::Valid,
            post_fp: Hash32([0x77; 32]),
        }
    }

    fn sample_provenance() -> WalEntry {
        WalEntry::SeedEpochConsensusInputsImported {
            anchor_fp: Hash32([0x11; 32]),
            sidecar_hash: Hash32([0x22; 32]),
            epoch_no: EpochNo(576),
        }
    }

    #[test]
    fn wal_entry_admit_block_round_trips_canonical_cbor() {
        let e = sample();
        let bytes = encode_wal_entry(&e);
        let (decoded, consumed) = decode_wal_entry(&bytes).expect("decode");
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, e);
    }

    #[test]
    fn wal_entry_encode_two_runs_byte_identical() {
        assert_eq!(encode_wal_entry(&sample()), encode_wal_entry(&sample()));
    }

    #[test]
    fn wal_entry_decode_rejects_unknown_tag() {
        // Hand-craft array(2)[uint 99, ...].
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        write_uint_canonical(&mut buf, 99);
        // Filler so the (unread) payload doesn't underflow.
        write_uint_canonical(&mut buf, 0);
        let res = decode_wal_entry(&buf);
        match res {
            Err(WalError::Structural { reason }) if reason.contains("unknown wal entry tag") => {}
            other => panic!("expected unknown tag, got {other:?}"),
        }
    }

    #[test]
    fn wal_entry_decode_rejects_unknown_verdict() {
        let mut buf = Vec::new();
        write_array_header(&mut buf, ContainerEncoding::Definite(2, canonical_width(2)));
        write_uint_canonical(&mut buf, TAG_ADMIT_BLOCK);
        write_array_header(&mut buf, ContainerEncoding::Definite(5, canonical_width(5)));
        write_bytes_canonical(&mut buf, &[0u8; 32]);
        write_bytes_canonical(&mut buf, &[0u8; 32]);
        write_uint_canonical(&mut buf, 0);
        write_uint_canonical(&mut buf, 99); // unknown verdict code
        write_bytes_canonical(&mut buf, &[0u8; 32]);
        let res = decode_wal_entry(&buf);
        match res {
            Err(WalError::Structural { reason }) if reason.contains("verdict") => {}
            other => panic!("expected unknown verdict, got {other:?}"),
        }
    }

    #[test]
    fn block_verdict_tag_round_trips_wire_code() {
        for v in [BlockVerdictTag::Valid, BlockVerdictTag::Invalid] {
            assert_eq!(BlockVerdictTag::from_wire_code(v.wire_code()), Some(v));
        }
        assert_eq!(BlockVerdictTag::from_wire_code(99), None);
    }

    #[test]
    fn wal_entry_match_is_exhaustive() {
        let e = sample();
        match &e {
            WalEntry::AdmitBlock {
                prior_fp,
                block_hash,
                slot,
                verdict,
                post_fp,
            } => {
                assert_eq!(prior_fp.0[0], 0x55);
                assert_eq!(block_hash.0[0], 0x66);
                assert_eq!(slot.0, 23013664);
                assert!(matches!(verdict, BlockVerdictTag::Valid));
                assert_eq!(post_fp.0[0], 0x77);
            }
            WalEntry::SeedEpochConsensusInputsImported {
                anchor_fp,
                sidecar_hash,
                epoch_no,
            } => {
                assert_eq!(anchor_fp.0[0], 0x11);
                assert_eq!(sidecar_hash.0[0], 0x22);
                assert_eq!(epoch_no.0, 576);
            }
            WalEntry::RollBack { .. } => unreachable!("sample() is an AdmitBlock"),
            WalEntry::EpochConsensusViewActivated { .. } => {
                unreachable!("sample() is an AdmitBlock")
            }
        }
    }

    #[test]
    fn wal_seed_cinput_entry_round_trips_byte_identical() {
        let e = sample_provenance();
        let bytes = encode_wal_entry(&e);
        let (decoded, consumed) = decode_wal_entry(&bytes).expect("decode");
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, e);
        // Two encodes are byte-identical (determinism).
        assert_eq!(encode_wal_entry(&e), encode_wal_entry(&sample_provenance()));
    }

    #[test]
    fn wal_seed_cinput_entry_uses_tag_three() {
        // The provenance entry must serialize under the append-only
        // tag 3, leaving the reserved 1/2 untouched.
        let bytes = encode_wal_entry(&sample_provenance());
        let mut o = 0usize;
        expect_definite_array(&bytes, &mut o, 2, "entry wrapper").expect("wrapper");
        let (tag, _w): (u64, IntWidth) = read_uint(&bytes, &mut o).expect("tag");
        assert_eq!(tag, TAG_SEED_EPOCH_CONSENSUS_INPUTS_IMPORTED);
        assert_eq!(tag, 3);
    }

    fn activation_with(epoch: u64, view_hash: u8) -> WalEntry {
        WalEntry::EpochConsensusViewActivated {
            target_epoch: EpochNo(epoch),
            network_magic: 2,
            era: CardanoEra::Conway,
            transition_point: Point { slot: SlotNo(115_000_000), hash: Hash32([0xa1; 32]) },
            source_checkpoint_commitment: Hash32([0xb2; 32]),
            snapshot_phase: SnapshotPhase::Set,
            nonce_commitment: Hash32([0xc3; 32]),
            stake_view_canonical_hash: Hash32([0xd4; 32]),
            view_canonical_hash: Hash32([view_hash; 32]),
        }
    }

    #[test]
    fn wal_epoch_view_activated_round_trips_byte_identical() {
        let e = activation_with(577, 0xe5);
        let bytes = encode_wal_entry(&e);
        let (decoded, consumed) = decode_wal_entry(&bytes).expect("decode");
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, e);
        // determinism: two encodes byte-identical.
        assert_eq!(encode_wal_entry(&e), encode_wal_entry(&activation_with(577, 0xe5)));
    }

    #[test]
    fn wal_epoch_view_activated_uses_tag_four() {
        let bytes = encode_wal_entry(&activation_with(577, 0xe5));
        let mut o = 0usize;
        expect_definite_array(&bytes, &mut o, 2, "entry wrapper").expect("wrapper");
        let (tag, _w): (u64, IntWidth) = read_uint(&bytes, &mut o).expect("tag");
        assert_eq!(tag, TAG_EPOCH_CONSENSUS_VIEW_ACTIVATED);
        assert_eq!(tag, 4);
    }

    #[test]
    fn activation_replay_idempotent_vs_conflict() {
        // DC-EPOCH-04: same target epoch + byte-identical record -> idempotent.
        assert_eq!(
            activation_replay_outcome(&activation_with(577, 0xe5), &activation_with(577, 0xe5)),
            Some(ActivationReplayOutcome::Idempotent)
        );
        // same target epoch + ANY differing binding/hash -> structured conflict (fail closed).
        assert_eq!(
            activation_replay_outcome(&activation_with(577, 0xe5), &activation_with(577, 0xff)),
            Some(ActivationReplayOutcome::Conflict)
        );
        // a DIFFERENT target epoch is not an idempotence/conflict pair.
        assert_eq!(
            activation_replay_outcome(&activation_with(577, 0xe5), &activation_with(578, 0xe5)),
            None
        );
        // a non-activation pair is not subject to the rule.
        assert_eq!(activation_replay_outcome(&sample(), &activation_with(577, 0xe5)), None);
    }
}
