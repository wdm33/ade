// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE `CertState` snapshot encoder/decoder (PHASE4-N-J S3).
//!
//! Wire shape (6-item array):
//! ```text
//! array(6) [
//!   map(N1) StakeCredential -> Coin,                      // registrations
//!   map(N2) StakeCredential -> PoolId (bytes(28)),        // delegations
//!   map(N3) StakeCredential -> Coin,                      // rewards
//!   map(N4) PoolId -> PoolParams,                         // pools
//!   map(N5) PoolId -> uint epoch,                         // retiring
//!   map(N6) PoolId -> PoolParams,                         // future_pools (staged re-regs, ECA-0a)
//! ]
//! ```
//!
//! StakeCredential is `array(2)[variant(0=Key,1=Script), bytes(28)]`.
//! PoolParams is `array(7)[pool_id, vrf_hash, pledge, cost,
//! array(2)[margin_num, margin_den], reward_account, owners[]]`.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    canonical_width, read_any_int, read_array_header, read_bytes, read_map_header,
    write_array_header, write_bytes_canonical, write_map_header, write_uint_canonical,
    ContainerEncoding, IntWidth,
};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{EpochNo, Hash28, Hash32};

use crate::delegation::{CertState, DelegationState, PoolParams, PoolState};

use super::error::{SnapshotDecodeError, StructuralReason};

const FIELDS: u64 = 6;

pub fn encode_cert_state(state: &CertState) -> Vec<u8> {
    let mut buf = Vec::new();
    write_array_header(
        &mut buf,
        ContainerEncoding::Definite(FIELDS, IntWidth::Inline),
    );
    // 1. registrations
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.delegation.registrations.len() as u64,
            canonical_width(state.delegation.registrations.len() as u64),
        ),
    );
    for (cred, coin) in &state.delegation.registrations {
        write_stake_credential(&mut buf, cred);
        write_uint_canonical(&mut buf, coin.0);
    }
    // 2. delegations
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.delegation.delegations.len() as u64,
            canonical_width(state.delegation.delegations.len() as u64),
        ),
    );
    for (cred, pool) in &state.delegation.delegations {
        write_stake_credential(&mut buf, cred);
        write_pool_id(&mut buf, pool);
    }
    // 3. rewards
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.delegation.rewards.len() as u64,
            canonical_width(state.delegation.rewards.len() as u64),
        ),
    );
    for (cred, coin) in &state.delegation.rewards {
        write_stake_credential(&mut buf, cred);
        write_uint_canonical(&mut buf, coin.0);
    }
    // 4. pools
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.pool.pools.len() as u64,
            canonical_width(state.pool.pools.len() as u64),
        ),
    );
    for (pool, params) in &state.pool.pools {
        write_pool_id(&mut buf, pool);
        write_pool_params(&mut buf, params);
    }
    // 5. retiring
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.pool.retiring.len() as u64,
            canonical_width(state.pool.retiring.len() as u64),
        ),
    );
    for (pool, epoch) in &state.pool.retiring {
        write_pool_id(&mut buf, pool);
        write_uint_canonical(&mut buf, epoch.0);
    }
    // 6. future_pools (staged re-registrations, adopted at the next epoch boundary; ECA-0a)
    write_map_header(
        &mut buf,
        ContainerEncoding::Definite(
            state.pool.future_pools.len() as u64,
            canonical_width(state.pool.future_pools.len() as u64),
        ),
    );
    for (pool, params) in &state.pool.future_pools {
        write_pool_id(&mut buf, pool);
        write_pool_params(&mut buf, params);
    }
    buf
}

pub fn decode_cert_state(bytes: &[u8]) -> Result<CertState, SnapshotDecodeError> {
    let mut o = 0usize;
    expect_array(bytes, &mut o, FIELDS)?;

    let registrations = decode_map(bytes, &mut o, |b, off| {
        let cred = read_stake_credential(b, off)?;
        let (coin, _, _) = read_any_int(b, off).map_err(SnapshotDecodeError::Cbor)?;
        Ok((cred, Coin(coin)))
    })?;
    let delegations = decode_map(bytes, &mut o, |b, off| {
        let cred = read_stake_credential(b, off)?;
        let pool = read_pool_id(b, off)?;
        Ok((cred, pool))
    })?;
    let rewards = decode_map(bytes, &mut o, |b, off| {
        let cred = read_stake_credential(b, off)?;
        let (coin, _, _) = read_any_int(b, off).map_err(SnapshotDecodeError::Cbor)?;
        Ok((cred, Coin(coin)))
    })?;
    let pools = decode_map(bytes, &mut o, |b, off| {
        let pool = read_pool_id(b, off)?;
        let params = read_pool_params(b, off)?;
        Ok((pool, params))
    })?;
    let retiring = decode_map(bytes, &mut o, |b, off| {
        let pool = read_pool_id(b, off)?;
        let (epoch, _, _) = read_any_int(b, off).map_err(SnapshotDecodeError::Cbor)?;
        Ok((pool, EpochNo(epoch)))
    })?;
    let future_pools = decode_map(bytes, &mut o, |b, off| {
        let pool = read_pool_id(b, off)?;
        let params = read_pool_params(b, off)?;
        Ok((pool, params))
    })?;

    Ok(CertState {
        delegation: DelegationState {
            registrations,
            delegations,
            rewards,
        },
        pool: PoolState { pools, future_pools, retiring },
    })
}

fn decode_map<K: Ord, V, F>(
    bytes: &[u8],
    o: &mut usize,
    mut read_entry: F,
) -> Result<BTreeMap<K, V>, SnapshotDecodeError>
where
    F: FnMut(&[u8], &mut usize) -> Result<(K, V), SnapshotDecodeError>,
{
    let n = match read_map_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    let mut m = BTreeMap::new();
    for _ in 0..n {
        let (k, v) = read_entry(bytes, o)?;
        m.insert(k, v);
    }
    Ok(m)
}

fn write_stake_credential(buf: &mut Vec<u8>, cred: &StakeCredential) {
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    let (variant, hash): (u64, &Hash28) = match cred {
        StakeCredential::KeyHash(h) => (0, h),
        StakeCredential::ScriptHash(h) => (1, h),
    };
    write_uint_canonical(buf, variant);
    write_bytes_canonical(buf, &hash.0);
}

fn read_stake_credential(
    bytes: &[u8],
    o: &mut usize,
) -> Result<StakeCredential, SnapshotDecodeError> {
    expect_array(bytes, o, 2)?;
    let (variant, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let (h, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if h.len() != 28 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash28LengthMismatch,
        });
    }
    let mut arr = [0u8; 28];
    arr.copy_from_slice(&h);
    match variant {
        0 => Ok(StakeCredential::KeyHash(Hash28(arr))),
        1 => Ok(StakeCredential::ScriptHash(Hash28(arr))),
        _ => Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::EraTagOutOfRange,
        }),
    }
}

fn write_pool_id(buf: &mut Vec<u8>, pool: &PoolId) {
    write_bytes_canonical(buf, &pool.0 .0);
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

fn write_pool_params(buf: &mut Vec<u8>, p: &PoolParams) {
    write_array_header(buf, ContainerEncoding::Definite(7, IntWidth::Inline));
    write_pool_id(buf, &p.pool_id);
    write_bytes_canonical(buf, &p.vrf_hash.0);
    write_uint_canonical(buf, p.pledge.0);
    write_uint_canonical(buf, p.cost.0);
    // margin: array(2)[num, den]
    write_array_header(buf, ContainerEncoding::Definite(2, IntWidth::Inline));
    write_uint_canonical(buf, p.margin.0);
    write_uint_canonical(buf, p.margin.1);
    write_bytes_canonical(buf, &p.reward_account);
    // owners: array(N) of bytes(28)
    write_array_header(
        buf,
        ContainerEncoding::Definite(
            p.owners.len() as u64,
            canonical_width(p.owners.len() as u64),
        ),
    );
    for owner in &p.owners {
        write_bytes_canonical(buf, &owner.0);
    }
}

fn read_pool_params(bytes: &[u8], o: &mut usize) -> Result<PoolParams, SnapshotDecodeError> {
    expect_array(bytes, o, 7)?;
    let pool_id = read_pool_id(bytes, o)?;
    let (vrf, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    if vrf.len() != 32 {
        return Err(SnapshotDecodeError::Structural {
            reason: StructuralReason::Hash32LengthMismatch,
        });
    }
    let mut vrf_arr = [0u8; 32];
    vrf_arr.copy_from_slice(&vrf);
    let (pledge, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let (cost, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    expect_array(bytes, o, 2)?;
    let (m_num, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let (m_den, _, _) = read_any_int(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let (reward_account, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
    let owners_n = match read_array_header(bytes, o).map_err(SnapshotDecodeError::Cbor)? {
        ContainerEncoding::Definite(n, _) => n,
        _ => {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::ArrayLengthMismatch,
            })
        }
    };
    // Do NOT pre-allocate on the attacker-declared `owners_n`: a huge count would trigger an OOM
    // pre-allocation before the per-element EOF check. Grow as we read (bounded by the input length).
    let mut owners = Vec::new();
    for _ in 0..owners_n {
        let (b, _) = read_bytes(bytes, o).map_err(SnapshotDecodeError::Cbor)?;
        if b.len() != 28 {
            return Err(SnapshotDecodeError::Structural {
                reason: StructuralReason::Hash28LengthMismatch,
            });
        }
        let mut arr = [0u8; 28];
        arr.copy_from_slice(&b);
        owners.push(Hash28(arr));
    }
    Ok(PoolParams {
        pool_id,
        vrf_hash: Hash32(vrf_arr),
        pledge: Coin(pledge),
        cost: Coin(cost),
        margin: (m_num, m_den),
        reward_account,
        owners,
    })
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

    fn cred_key(b: u8) -> StakeCredential {
        StakeCredential::KeyHash(Hash28([b; 28]))
    }

    fn cred_script(b: u8) -> StakeCredential {
        StakeCredential::ScriptHash(Hash28([b; 28]))
    }

    fn pool(b: u8) -> PoolId {
        PoolId(Hash28([b; 28]))
    }

    fn params(b: u8) -> PoolParams {
        PoolParams {
            pool_id: pool(b),
            vrf_hash: Hash32([b; 32]),
            pledge: Coin(1_000_000_000),
            cost: Coin(340_000_000),
            margin: (3, 100),
            reward_account: vec![b; 29],
            owners: vec![Hash28([b ^ 0x01; 28]), Hash28([b ^ 0x02; 28])],
        }
    }

    fn make_state() -> CertState {
        let mut s = CertState::new();
        s.delegation
            .registrations
            .insert(cred_key(0x10), Coin(2_000_000));
        s.delegation
            .registrations
            .insert(cred_script(0x11), Coin(2_000_000));
        s.delegation.delegations.insert(cred_key(0x10), pool(0x20));
        s.delegation.rewards.insert(cred_key(0x10), Coin(500_000));
        s.pool.pools.insert(pool(0x20), params(0x20));
        s.pool.pools.insert(pool(0x21), params(0x21));
        s.pool.retiring.insert(pool(0x21), EpochNo(600));
        // A staged re-registration of pool 0x20 (different params) — exercises field 6.
        s.pool.future_pools.insert(pool(0x20), params(0x99));
        s
    }

    #[test]
    fn cert_state_round_trip_empty() {
        let s = CertState::new();
        let bytes = encode_cert_state(&s);
        let decoded = decode_cert_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn cert_state_round_trip_populated() {
        let s = make_state();
        let bytes = encode_cert_state(&s);
        let decoded = decode_cert_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }

    #[test]
    fn cert_state_encode_deterministic_across_runs() {
        let s = make_state();
        let a = encode_cert_state(&s);
        let b = encode_cert_state(&s);
        assert_eq!(a, b);
    }

    #[test]
    fn cert_state_pool_params_round_trip_with_empty_owners() {
        let mut p = params(0xAA);
        p.owners = vec![];
        let mut s = CertState::new();
        s.pool.pools.insert(pool(0xAA), p);
        let bytes = encode_cert_state(&s);
        let decoded = decode_cert_state(&bytes).expect("decode");
        assert_eq!(decoded, s);
    }
}
