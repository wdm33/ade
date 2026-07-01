// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE combined-snapshot framing (PHASE4-N-J S7).
//!
//! Wraps `(LedgerState, PraosChainDepState)` as a single
//! byte payload with:
//!
//! * a versioned schema tag (`SCHEMA_VERSION == 2`; v2 = the embedded cert state is the
//!   6-field encoding with future_pools, ECA-0a) — unknown versions (incl. an old v1
//!   persistent-cache blob) reject before any payload work (DC-STORE-09);
//! * the source state's combined `LedgerFingerprint.combined` hash
//!   embedded inline — verified after decode (DC-STORE-08).
//!
//! Wire shape:
//! ```text
//! array(4) [
//!   uint      version,             // == 1
//!   bytes(32) source_fingerprint,  // combined ledger fingerprint hash
//!   bytes     ledger_state_bytes,  // S6 encode_ledger_state output
//!   bytes     chain_dep_bytes,     // S1 encode_chain_dep output
//! ]
//! ```
//!
//! Single-authority discipline (CN-STORE-08): this module's
//! `encode_snapshot` / `decode_snapshot` pair is the SOLE public
//! authority for `(LedgerState, PraosChainDepState) <-> bytes`
//! conversion in the workspace.

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bytes, write_array_header,
    write_bytes_canonical, write_uint_canonical, ContainerEncoding,
};
use ade_core::consensus::praos_state::PraosChainDepState;
use ade_types::{CardanoEra, Hash32};

use crate::fingerprint::fingerprint;
use crate::state::LedgerState;

use super::chain_dep::{decode_chain_dep, encode_chain_dep};
use super::error::{SnapshotDecodeError, SnapshotEncodeError, StructuralReason};
use super::ledger::{decode_ledger_state, encode_ledger_state};

// v2 (ECA-0a): the embedded LedgerState's cert state is the 6-field encoding (adds future_pools).
// The bump makes an old v1 persistent-cache blob reject as UnknownVersion (clean) rather than as a
// structural ArrayLengthMismatch from the embedded cert-state decode.
pub const SCHEMA_VERSION: u32 = 2;

const FIELDS: u64 = 4;

pub fn encode_snapshot(
    ledger: &LedgerState,
    chain_dep: &PraosChainDepState,
) -> Result<Vec<u8>, SnapshotEncodeError> {
    if (ledger.era as u8) < (CardanoEra::Conway as u8) {
        return Err(SnapshotEncodeError::EraNotSupported { era: ledger.era });
    }
    let ledger_bytes = encode_ledger_state(ledger)?;
    let chain_dep_bytes = encode_chain_dep(chain_dep)?;
    let fp = fingerprint(ledger).combined;
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS, canonical_width(FIELDS)),
    );
    write_uint_canonical(&mut buf, SCHEMA_VERSION as u64);
    write_bytes_canonical(&mut buf, &fp.0);
    write_bytes_canonical(&mut buf, &ledger_bytes);
    write_bytes_canonical(&mut buf, &chain_dep_bytes);
    Ok(buf)
}

pub fn decode_snapshot(
    bytes: &[u8],
) -> Result<(LedgerState, PraosChainDepState), SnapshotDecodeError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, FIELDS)?;
    // Version tag: read + verify BEFORE any payload work (DC-STORE-09).
    let version_u = read_u64(bytes, &mut o)?;
    if version_u > u32::MAX as u64 {
        return Err(SnapshotDecodeError::UnknownVersion {
            expected: SCHEMA_VERSION,
            found: u32::MAX,
        });
    }
    let version = version_u as u32;
    if version != SCHEMA_VERSION {
        return Err(SnapshotDecodeError::UnknownVersion {
            expected: SCHEMA_VERSION,
            found: version,
        });
    }
    let (fp_bytes, _) = read_bytes(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    if fp_bytes.len() != 32 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash32LengthMismatch,
        });
    }
    let mut fp_arr = [0u8; 32];
    fp_arr.copy_from_slice(&fp_bytes);
    let expected_fp = Hash32(fp_arr);

    let (ls_bytes, _) = read_bytes(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let ledger = decode_ledger_state(&ls_bytes)?;
    let (cd_bytes, _) = read_bytes(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let chain_dep = decode_chain_dep(&cd_bytes)?;

    // Fingerprint cross-check (DC-STORE-08).
    let actual_fp = fingerprint(&ledger).combined;
    if actual_fp != expected_fp {
        return Err(SnapshotDecodeError::FingerprintMismatch {
            expected: expected_fp,
            actual: actual_fp,
        });
    }
    Ok((ledger, chain_dep))
}

fn read_u64(bytes: &[u8], o: &mut usize) -> Result<u64, SnapshotDecodeError> {
    let (v, _is_neg, _w) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    Ok(v)
}

fn expect_array(
    bytes: &[u8],
    o: &mut usize,
    expected_len: u64,
) -> Result<(), SnapshotDecodeError> {
    let enc = read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    match enc {
        ContainerEncoding::Definite(n, _) if n == expected_len => Ok(()),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use ade_core::consensus::praos_state::Nonce;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId};
    use ade_types::{BlockNo, EpochNo, Hash28, SlotNo};

    use crate::pparams::ConwayOnlyDepositParams;
    use crate::state::ConwayGovState;

    fn sample_ledger() -> LedgerState {
        let mut s = LedgerState::new(CardanoEra::Conway);
        s.max_lovelace_supply = 45_000_000_000_000_000;
        s.epoch_state.epoch = EpochNo(580);
        s.epoch_state.slot = SlotNo(164_000_000);
        s.epoch_state.reserves = Coin(13_888_022_852_926_644);
        s.cert_state
            .delegation
            .registrations
            .insert(StakeCredential::KeyHash(Hash28([0xAB; 28])), Coin(2_000_000));
        s.cert_state.delegation.delegations.insert(
            StakeCredential::KeyHash(Hash28([0xAB; 28])),
            PoolId(Hash28([0xCD; 28])),
        );
        s.gov_state = Some(ConwayGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: (2, 3),
            drep_expiry: BTreeMap::new(),
            gov_action_lifetime: 6,
            vote_delegations: BTreeMap::new(),
            pool_voting_thresholds: vec![(1, 2)],
            drep_voting_thresholds: vec![(67, 100)],
            committee_hot_keys: BTreeMap::new(),
            num_dormant: crate::state::DormantEpochs::Unversioned,
        });
        s.conway_deposit_params = Some(ConwayOnlyDepositParams {
            drep_deposit: Coin(500_000_000),
            gov_action_deposit: Coin(100_000_000_000),
            drep_activity: 20,
        });
        s
    }

    fn sample_chain_dep() -> PraosChainDepState {
        let mut cd = PraosChainDepState::genesis(Nonce(ade_types::Hash32([0xAB; 32])));
        cd.evolving_nonce = Nonce(ade_types::Hash32([0xAB; 32]));
        cd.last_epoch_block = Some(EpochNo(580));
        cd.last_slot = Some(SlotNo(164_000_000));
        cd.last_block_no = Some(BlockNo(123));
        let _ = cd.op_cert_counters.upsert_strict(Hash28([0xEE; 28]), 5, 9);
        cd
    }

    #[test]
    fn snapshot_round_trip() {
        let l = sample_ledger();
        let cd = sample_chain_dep();
        let bytes = encode_snapshot(&l, &cd).expect("encode");
        let (l2, cd2) = decode_snapshot(&bytes).expect("decode");
        assert_eq!(l2, l);
        assert_eq!(cd2, cd);
    }

    #[test]
    fn snapshot_encode_deterministic_across_runs() {
        let l = sample_ledger();
        let cd = sample_chain_dep();
        let a = encode_snapshot(&l, &cd).expect("encode a");
        let b = encode_snapshot(&l, &cd).expect("encode b");
        assert_eq!(a, b);
    }

    #[test]
    fn decode_rejects_unknown_version() {
        let l = sample_ledger();
        let cd = sample_chain_dep();
        let mut bytes = encode_snapshot(&l, &cd).expect("encode");
        // Array header is 0x84 (definite array(4) inline); version is the next
        // byte. Patch SCHEMA_VERSION=2 (0x02) → an unknown 3 (0x03).
        assert_eq!(bytes[0], 0x84);
        assert_eq!(bytes[1], 0x02);
        bytes[1] = 0x03;
        match decode_snapshot(&bytes) {
            Err(SnapshotDecodeError::UnknownVersion { expected, found }) => {
                assert_eq!(expected, SCHEMA_VERSION);
                assert_eq!(found, 3);
            }
            other => panic!("expected UnknownVersion, got {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_fingerprint_mismatch() {
        let l = sample_ledger();
        let cd = sample_chain_dep();
        let mut bytes = encode_snapshot(&l, &cd).expect("encode");
        // Locate the fingerprint bstr header: position [2] is byte-array
        // major(2) | len-inline / one-byte indicator. 32-byte bstr =
        // 0x58 0x20, so bytes 4..36 are the 32 fingerprint bytes (after the
        // 1-byte array header + 1-byte version + 2-byte bstr header).
        assert_eq!(bytes[2], 0x58); // major(2) | AI_ONE_BYTE
        assert_eq!(bytes[3], 0x20); // length 32
        bytes[4] ^= 0xFF; // corrupt one byte
        match decode_snapshot(&bytes) {
            Err(SnapshotDecodeError::FingerprintMismatch { .. }) => {}
            other => panic!("expected FingerprintMismatch, got {other:?}"),
        }
    }

    #[test]
    fn encode_pre_conway_era_rejected() {
        let l = LedgerState::new(CardanoEra::Babbage);
        let cd = sample_chain_dep();
        match encode_snapshot(&l, &cd) {
            Err(SnapshotEncodeError::EraNotSupported { era }) => {
                assert_eq!(era, CardanoEra::Babbage);
            }
            other => panic!("expected EraNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn round_trip_via_fingerprint_combined() {
        let l = sample_ledger();
        let cd = sample_chain_dep();
        let bytes = encode_snapshot(&l, &cd).expect("encode");
        let (l2, _) = decode_snapshot(&bytes).expect("decode");
        assert_eq!(fingerprint(&l2).combined, fingerprint(&l).combined);
    }
}
