// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `EpochState` snapshot encoder/decoder (PHASE4-N-J S4).
//!
//! Wire shape:
//! ```text
//! array(7) [
//!   uint  epoch,
//!   uint  slot,
//!   snapshot_state (array(3) of stake_snapshot),
//!   uint  reserves,
//!   uint  treasury,
//!   map(N) PoolId -> uint  block_production,
//!   uint  epoch_fees,
//! ]
//! ```
//! `stake_snapshot` = array(3)[delegations_map, pool_stakes_map, pool_params_map] where
//! delegations_map = map(N) credential(array(2)[tag, hash28]) -> array(2)[PoolId, Coin]
//! pool_stakes_map = map(M) PoolId -> Coin
//! pool_params_map = map(K) PoolId -> array(5)[pledge, cost, array(2)[m_num, m_den],
//!                                             credential reward_account, array(J) owner hash28]
//! (LV-4: whole credentials, frozen pool parameters; the array(2) form of earlier stores is refused).

use std::collections::{BTreeMap, BTreeSet};

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bytes, read_map_header,
    write_array_header, write_bytes_canonical, write_map_header, write_uint_canonical,
    ContainerEncoding, IntWidth,
};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{EpochNo, Hash28, SlotNo};

use super::cert_state::{read_stake_credential, write_stake_credential};
use crate::epoch::{
    GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotPoolParams, SnapshotState, StakeSnapshot,
};
use crate::state::EpochState;

use super::error::{SnapshotDecodeError, StructuralReason};

const FIELDS: u64 = 8;

pub fn encode_epoch_state(state: &EpochState) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS, IntWidth::Inline),
    );
    write_uint_canonical(&mut buf, state.epoch.0);
    write_uint_canonical(&mut buf, state.slot.0);
    write_snapshot_state(&mut buf, &state.snapshots);
    write_uint_canonical(&mut buf, state.reserves.0);
    write_uint_canonical(&mut buf, state.treasury.0);
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.block_production.len() as u64,
            canonical_width(state.block_production.len() as u64),
        ),
    );
    for (pool, count) in &state.block_production {
        write_bytes_canonical(&mut buf, &pool.0 .0);
        write_uint_canonical(&mut buf, *count);
    }
    write_uint_canonical(&mut buf, state.epoch_fees.0);
    write_uint_canonical(&mut buf, state.donation_pot.0);
    buf
}

pub fn decode_epoch_state(bytes: &[u8]) -> Result<EpochState, SnapshotDecodeError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, FIELDS)?;
    let (epoch, _, _) = read_any_int(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let (slot, _, _) = read_any_int(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let snapshots = read_snapshot_state(bytes, &mut o)?;
    let (reserves, _, _) = read_any_int(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let (treasury, _, _) = read_any_int(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let bp_n = match read_map_header(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut block_production: BTreeMap<PoolId, u64> = BTreeMap::new();
    for _ in 0..bp_n {
        let pool = read_pool_id(bytes, &mut o)?;
        let (count, _, _) = read_any_int(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
        block_production.insert(pool, count);
    }
    let (fees, _, _) = read_any_int(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    let (donation_pot, _, _) = read_any_int(bytes, &mut o).map_err(SnapshotDecodeError::Cbor)?;
    Ok(EpochState {
        epoch: EpochNo(epoch),
        slot: SlotNo(slot),
        snapshots,
        reserves: Coin(reserves),
        treasury: Coin(treasury),
        block_production,
        epoch_fees: Coin(fees),
        donation_pot: Coin(donation_pot),
    })
}

fn write_snapshot_state(buf: &mut Vec<u8>, s: &crate::epoch::EpochStakeSnapshots) {
    match s {
        // Authoritative: array(3) + mark/set/go — the EXISTING format, byte-identical (RVBP gate 7).
        crate::epoch::EpochStakeSnapshots::Authoritative(a) => {
            write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
            write_stake_snapshot(buf, &a.mark.0);
            write_stake_snapshot(buf, &a.set.0);
            write_stake_snapshot(buf, &a.go.0);
        }
        // A reduced projection: array(0) — a DISTINCT, non-authoritative encoding. A reduced-plane ledger is
        // never serialized as an authoritative accumulator snapshot, and decode restores it as
        // `ReducedUnavailable`, never fabricated mark/set/go (RVBP gates 1/2/3).
        crate::epoch::EpochStakeSnapshots::ReducedUnavailable => {
            write_array_header(buf, ContainerEncoding::Definite(0, IntWidth::Inline));
        }
    }
}

fn read_snapshot_state(
    bytes: &[u8],
    o: &mut usize,
) -> Result<crate::epoch::EpochStakeSnapshots, SnapshotDecodeError> {
    let n = match read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    match n {
        3 => {
            let mark = read_stake_snapshot(bytes, o)?;
            let set = read_stake_snapshot(bytes, o)?;
            let go = read_stake_snapshot(bytes, o)?;
            Ok(crate::epoch::EpochStakeSnapshots::Authoritative(SnapshotState {
                mark: MarkSnapshot(mark),
                set: SetSnapshot(set),
                go: GoSnapshot(go),
            }))
        }
        0 => Ok(crate::epoch::EpochStakeSnapshots::ReducedUnavailable),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

fn write_stake_snapshot(buf: &mut Vec<u8>, s: &StakeSnapshot) {
    write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
    write_map_header(
        buf,
        ContainerEncoding::Definite(
            s.delegations.len() as u64,
            canonical_width(s.delegations.len() as u64),
        ),
    );
    for (cred, (pool, coin)) in &s.delegations {
        write_stake_credential(buf, cred);
        write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_bytes_canonical(buf, &pool.0 .0);
        write_uint_canonical(buf, coin.0);
    }
    write_map_header(
        buf,
        ContainerEncoding::Definite(
            s.pool_stakes.len() as u64,
            canonical_width(s.pool_stakes.len() as u64),
        ),
    );
    for (pool, coin) in &s.pool_stakes {
        write_bytes_canonical(buf, &pool.0 .0);
        write_uint_canonical(buf, coin.0);
    }
    write_map_header(
        buf,
        ContainerEncoding::Definite(
            s.pool_params.len() as u64,
            canonical_width(s.pool_params.len() as u64),
        ),
    );
    for (pool, pp) in &s.pool_params {
        write_bytes_canonical(buf, &pool.0 .0);
        write_array_header(buf, ContainerEncoding::Definite(5, IntWidth::Inline));
        write_uint_canonical(buf, pp.pledge.0);
        write_uint_canonical(buf, pp.cost.0);
        write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_uint_canonical(buf, pp.margin.0);
        write_uint_canonical(buf, pp.margin.1);
        write_stake_credential(buf, &pp.reward_account);
        write_array_header(
            buf,
            ContainerEncoding::Definite(
                pp.self_delegated_owners.len() as u64,
                canonical_width(pp.self_delegated_owners.len() as u64),
            ),
        );
        for owner in &pp.self_delegated_owners {
            write_bytes_canonical(buf, &owner.0);
        }
    }
}

fn read_definite_map_len(bytes: &[u8], o: &mut usize) -> Result<u64, SnapshotDecodeError> {
    match read_map_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => Ok(n),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::ArrayLengthMismatch,
        }),
    }
}

fn read_hash28(bytes: &[u8], o: &mut usize) -> Result<Hash28, SnapshotDecodeError> {
    let (b, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if b.len() != 28 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash28LengthMismatch,
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&b);
    Ok(Hash28(arr))
}

fn read_snapshot_pool_params(
    bytes: &[u8],
    o: &mut usize,
) -> Result<SnapshotPoolParams, SnapshotDecodeError> {
    expect_array(bytes, o, 5)?;
    let (pledge, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let (cost, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    expect_array(bytes, o, 2)?;
    let (m_num, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let (m_den, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let reward_account = read_stake_credential(bytes, o)?;
    let owners_n = match read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut self_delegated_owners = BTreeSet::new();
    for _ in 0..owners_n {
        self_delegated_owners.insert(read_hash28(bytes, o)?);
    }
    Ok(SnapshotPoolParams {
        pledge: Coin(pledge),
        cost: Coin(cost),
        margin: (m_num, m_den),
        reward_account,
        self_delegated_owners,
    })
}

fn read_stake_snapshot(bytes: &[u8], o: &mut usize) -> Result<StakeSnapshot, SnapshotDecodeError> {
    expect_array(bytes, o, 3)?;
    let delegations_n = read_definite_map_len(bytes, o)?;
    let mut delegations: BTreeMap<StakeCredential, (PoolId, Coin)> = BTreeMap::new();
    for _ in 0..delegations_n {
        let cred = read_stake_credential(bytes, o)?;
        expect_array(bytes, o, 2)?;
        let pool = read_pool_id(bytes, o)?;
        let (coin, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        delegations.insert(cred, (pool, Coin(coin)));
    }
    let pool_stakes_n = read_definite_map_len(bytes, o)?;
    let mut pool_stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();
    for _ in 0..pool_stakes_n {
        let pool = read_pool_id(bytes, o)?;
        let (coin, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        pool_stakes.insert(pool, Coin(coin));
    }
    let pool_params_n = read_definite_map_len(bytes, o)?;
    let mut pool_params: BTreeMap<PoolId, SnapshotPoolParams> = BTreeMap::new();
    for _ in 0..pool_params_n {
        let pool = read_pool_id(bytes, o)?;
        pool_params.insert(pool, read_snapshot_pool_params(bytes, o)?);
    }
    Ok(StakeSnapshot {
        delegations,
        pool_stakes,
        pool_params,
    })
}

fn read_pool_id(bytes: &[u8], o: &mut usize) -> Result<PoolId, SnapshotDecodeError> {
    let (b, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if b.len() != 28 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::PoolIdLengthMismatch,
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&b);
    Ok(PoolId(Hash28(arr)))
}

fn expect_array(bytes: &[u8], o: &mut usize, expected_len: u64) -> Result<(), SnapshotDecodeError> {
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

    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }

    fn make_stake_snapshot(seed: u8) -> StakeSnapshot {
        let mut s = StakeSnapshot::new();
        s.delegations
            .insert(StakeCredential::KeyHash(Hash28([seed; 28])), (pool(seed + 1), Coin(1_000)));
        s.delegations
            .insert(StakeCredential::KeyHash(Hash28([seed + 1; 28])), (pool(seed + 2), Coin(2_000)));
        s.pool_stakes.insert(pool(seed + 1), Coin(1_000));
        s.pool_stakes.insert(pool(seed + 2), Coin(2_000));
        s
    }

    fn make_state() -> EpochState {
        let mut e = EpochState::new();
        e.epoch = EpochNo(576);
        e.slot = SlotNo(163_900_801);
        e.reserves = Coin(13_888_022_852_926_644);
        e.treasury = Coin(1_434_657_232_801_879);
        e.epoch_fees = Coin(8_321_001_400);
        let snaps = e.snapshots.as_authoritative_mut().expect("authoritative");
        snaps.mark = MarkSnapshot(make_stake_snapshot(0x10));
        snaps.set = SetSnapshot(make_stake_snapshot(0x20));
        snaps.go = GoSnapshot(make_stake_snapshot(0x30));
        e.block_production.insert(pool(0x40), 100);
        e.block_production.insert(pool(0x41), 50);
        e
    }

    #[test]
    fn epoch_state_round_trip_empty() {
        let e = EpochState::new();
        let bytes = encode_epoch_state(&e);
        let decoded = decode_epoch_state(&bytes).expect("decode");
        assert_eq!(decoded, e);
    }

    #[test]
    fn epoch_state_round_trip_populated() {
        let e = make_state();
        let bytes = encode_epoch_state(&e);
        let decoded = decode_epoch_state(&bytes).expect("decode");
        assert_eq!(decoded, e);
    }

    #[test]
    fn reduced_snapshots_encode_distinctly_and_round_trip() {
        // A reduced projection encodes as CBOR array(0) and decodes back to ReducedUnavailable — never a
        // fabricated mark/set/go (RVBP N-RVB-1, gate 2).
        let mut rbuf = Vec::new();
        write_snapshot_state(&mut rbuf, &crate::epoch::EpochStakeSnapshots::ReducedUnavailable);
        assert_eq!(rbuf, vec![0x80], "reduced snapshots are exactly CBOR array(0)");
        let mut o = 0usize;
        let decoded = read_snapshot_state(&rbuf, &mut o).expect("decode reduced");
        assert!(decoded.is_reduced(), "reduced round-trips to reduced, never authoritative");
        assert_eq!(o, rbuf.len(), "all reduced bytes consumed");

        // Authoritative encodes as the LEGACY array(3) + mark/set/go — byte-identical to the pre-enum format
        // (no discriminant prefix), so existing snapshots decode unchanged (gate 7). Its bytes can never be
        // confused with a reduced projection (gate 2).
        let auth = make_state().snapshots.clone();
        let mut abuf = Vec::new();
        write_snapshot_state(&mut abuf, &auth);
        assert_eq!(abuf[0], 0x83, "authoritative snapshots start with CBOR array(3) — the legacy header");
        assert_ne!(abuf, rbuf, "authoritative and reduced snapshot encodings are structurally distinct");
        let mut o2 = 0usize;
        let rt = read_snapshot_state(&abuf, &mut o2).expect("decode authoritative");
        assert_eq!(rt, auth, "authoritative snapshots round-trip byte-for-byte");
    }

    #[test]
    fn epoch_state_encode_deterministic_across_runs() {
        let e = make_state();
        let a = encode_epoch_state(&e);
        let b = encode_epoch_state(&e);
        assert_eq!(a, b);
    }
}
