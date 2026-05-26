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
//! `stake_snapshot` = array(2)[delegations_map, pool_stakes_map] where
//! delegations_map = map(N) Hash28 -> array(2)[PoolId, Coin]
//! pool_stakes_map = map(M) PoolId -> Coin.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    read_any_int, read_array_header, read_bytes, read_map_header, write_array_header,
    write_bytes_canonical, write_map_header, write_uint_canonical, ContainerEncoding, IntWidth,
};
use ade_types::tx::{Coin, PoolId};
use ade_types::{EpochNo, Hash28, SlotNo};

use crate::epoch::{
    GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotState, StakeSnapshot,
};
use crate::state::EpochState;

use super::error::{SnapshotDecodeError, StructuralReason};

const FIELDS: u64 = 7;

pub fn encode_epoch_state(state: &EpochState) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(&mut buf, ContainerEncoding::Definite(FIELDS, IntWidth::Inline));
    write_uint_canonical(&mut buf, state.epoch.0);
    write_uint_canonical(&mut buf, state.slot.0);
    write_snapshot_state(&mut buf, &state.snapshots);
    write_uint_canonical(&mut buf, state.reserves.0);
    write_uint_canonical(&mut buf, state.treasury.0);
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(state.block_production.len() as u64, IntWidth::Inline),
    );
    for (pool, count) in &state.block_production {
        write_bytes_canonical(&mut buf, &pool.0 .0);
        write_uint_canonical(&mut buf, *count);
    }
    write_uint_canonical(&mut buf, state.epoch_fees.0);
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
    Ok(EpochState {
        epoch: EpochNo(epoch),
        slot: SlotNo(slot),
        snapshots,
        reserves: Coin(reserves),
        treasury: Coin(treasury),
        block_production,
        epoch_fees: Coin(fees),
    })
}

fn write_snapshot_state(buf: &mut Vec<u8>, s: &SnapshotState) {
    write_array_header(buf, ContainerEncoding::Definite(3, IntWidth::Inline));
    write_stake_snapshot(buf, &s.mark.0);
    write_stake_snapshot(buf, &s.set.0);
    write_stake_snapshot(buf, &s.go.0);
}

fn read_snapshot_state(bytes: &[u8], o: &mut usize) -> Result<SnapshotState, SnapshotDecodeError> {
    expect_array(bytes, o, 3)?;
    let mark = read_stake_snapshot(bytes, o)?;
    let set = read_stake_snapshot(bytes, o)?;
    let go = read_stake_snapshot(bytes, o)?;
    Ok(SnapshotState {
        mark: MarkSnapshot(mark),
        set: SetSnapshot(set),
        go: GoSnapshot(go),
    })
}

fn write_stake_snapshot(buf: &mut Vec<u8>, s: &StakeSnapshot) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    // delegations: map Hash28 -> array(2)[PoolId, Coin]
    write_map_header(
        buf,
        ContainerEncoding::Definite(s.delegations.len() as u64, IntWidth::Inline),
    );
    for (cred, (pool, coin)) in &s.delegations {
        write_bytes_canonical(buf, &cred.0);
        write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
        write_bytes_canonical(buf, &pool.0 .0);
        write_uint_canonical(buf, coin.0);
    }
    // pool_stakes: map PoolId -> Coin
    write_map_header(
        buf,
        ContainerEncoding::Definite(s.pool_stakes.len() as u64, IntWidth::Inline),
    );
    for (pool, coin) in &s.pool_stakes {
        write_bytes_canonical(buf, &pool.0 .0);
        write_uint_canonical(buf, coin.0);
    }
}

fn read_stake_snapshot(
    bytes: &[u8],
    o: &mut usize,
) -> Result<StakeSnapshot, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let delegations_n = match read_map_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut delegations: BTreeMap<Hash28, (PoolId, Coin)> = BTreeMap::new();
    for _ in 0..delegations_n {
        let (cred_bytes, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        if cred_bytes.len() != 28 {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::Hash28LengthMismatch,
            });
        }
        let mut cred = [0u8; 28];
        cred.copy_from_slice(&cred_bytes);
        expect_array(bytes, o, 2)?;
        let pool = read_pool_id(bytes, o)?;
        let (coin, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        delegations.insert(Hash28(cred), (pool, Coin(coin)));
    }
    let pool_stakes_n = match read_map_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut pool_stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();
    for _ in 0..pool_stakes_n {
        let pool = read_pool_id(bytes, o)?;
        let (coin, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        pool_stakes.insert(pool, Coin(coin));
    }
    Ok(StakeSnapshot {
        delegations,
        pool_stakes,
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

    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }

    fn make_stake_snapshot(seed: u8) -> StakeSnapshot {
        let mut s = StakeSnapshot::new();
        s.delegations.insert(Hash28([seed; 28]), (pool(seed + 1), Coin(1_000)));
        s.delegations.insert(Hash28([seed + 1; 28]), (pool(seed + 2), Coin(2_000)));
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
        e.snapshots.mark = MarkSnapshot(make_stake_snapshot(0x10));
        e.snapshots.set = SetSnapshot(make_stake_snapshot(0x20));
        e.snapshots.go = GoSnapshot(make_stake_snapshot(0x30));
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
    fn epoch_state_encode_deterministic_across_runs() {
        let e = make_state();
        let a = encode_epoch_state(&e);
        let b = encode_epoch_state(&e);
        assert_eq!(a, b);
    }
}
