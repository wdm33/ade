// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Native cardano-node V2 (UTXO-HD `utxohd-mem`) LedgerDB `state` decoder
//! (MITHRIL-VERIFIED-ANCHOR-IMPORT, Stage 1).
//!
//! Decodes the cardano-node NewEpochState CBOR (the non-UTxO ledger state in a V2 snapshot `state`
//! file) into Ade's canonical `CertState` + the pool distribution + the Praos nonces, for the native
//! Mithril bootstrap. The raw cardano-node CBOR is RED/diagnostic INPUT; the decoded `CertState`
//! (canonical via `encode_cert_state`) is the authority. Stage 1 is NON-EMITTING: it returns a
//! structured probe report (the decoded summary + canonical-bytes commitment), never a
//! LedgerState/UTxO/admission artifact (the UTxO lives in the `tables` file — Stage 2).
//!
//! Hard boundaries:
//! - the Mithril-certified point is AUTHORITATIVE; the decoded epoch is cross-checked against it.
//! - the HardFork telescope navigation is EXPLICIT + era-tagged (past eras carry an end bound; the
//!   current era carries the live state) — require the current era == Conway; never "take the latest".
//! - a real VRF is REQUIRED for every active pool; a zero VRF is TERMINAL.
//! - `PoolDistr` and the decoded pools cross-check on pool identity + VRF (terminal mismatch, even
//!   for a zero-stake pool).
//! - deterministic: same bytes + same manifest point -> byte-identical canonical CertState + report.

use std::collections::BTreeMap;

use ade_codec::cbor::{
    read_any_int, read_array_header, read_bytes, read_map_header, read_tag, skip_item,
    ContainerEncoding,
};
use ade_crypto::blake2b::blake2b_256;
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{EpochNo, Hash28, Hash32};

use crate::consensus_input_extract::{Nonce, PraosNonces};
use crate::delegation::{CertState, DelegationState, PoolParams, PoolState};
use crate::snapshot::cert_state::{decode_cert_state, encode_cert_state};

/// The Conway era index in the HardFork telescope (Byron=0 … Conway=6).
const CONWAY_TELESCOPE_INDEX: usize = 6;
/// cardano-ledger `Credential` tag: 1 = KeyHash, 0 = ScriptHash.
const CRED_KEYHASH: u64 = 1;
const CRED_SCRIPTHASH: u64 = 0;
/// CBOR tag for a rational number (`margin`).
const TAG_RATIONAL: u64 = 30;

/// Why a V2 LedgerDB `state` decode fails. Every variant is TERMINAL (fail-closed before any use).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerDbStateError {
    /// Structurally malformed CBOR (wrong major type / truncated / unexpected shape).
    MalformedCbor(String),
    /// The current HardFork era is not supported (only Conway).
    UnsupportedEra { current_index: usize },
    /// An active pool's VRF is zero — a faithful decode requires the real VRF.
    ZeroVrf(PoolId),
    /// `PoolDistr` and the decoded pools disagree on a pool's VRF (terminal even at zero stake).
    PoolDistrVrfMismatch(PoolId),
    /// The decoded epoch does not match the Mithril-certified (authoritative) point's epoch.
    EpochMismatch { manifest_epoch: u64, decoded_epoch: u64 },
    /// The decoded CertState does not survive a canonical encode/decode round-trip.
    RoundTripMismatch,
}

impl From<ade_codec::CodecError> for LedgerDbStateError {
    fn from(e: ade_codec::CodecError) -> Self {
        LedgerDbStateError::MalformedCbor(format!("{e:?}"))
    }
}

/// The structured Stage-1 probe report (non-emitting). Carries the decoded authority candidate
/// summary + the Ade canonical-bytes commitment — never raw cardano-node CBOR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerDbStateProbe {
    /// The decoded current era index (Conway = 6).
    pub era_index: usize,
    /// The decoded epoch (cross-checked against the manifest-certified point's epoch).
    pub epoch: u64,
    /// `blake2b_256` of the canonical `encode_cert_state(cert_state)` — the Ade commitment.
    pub cert_state_commitment: Hash32,
    pub active_pool_count: usize,
    pub vrf_count: usize,
    pub future_pool_count: usize,
    pub retiring_count: usize,
    pub registration_count: usize,
    pub delegation_count: usize,
    pub reward_count: usize,
    pub pool_distr_count: usize,
    /// The PraosState epoch nonce (eta0) — leadership evidence that nonce extraction succeeded.
    pub praos_epoch_nonce: Hash32,
}

type R<T> = Result<T, LedgerDbStateError>;

fn malformed(detail: impl Into<String>) -> LedgerDbStateError {
    LedgerDbStateError::MalformedCbor(detail.into())
}

/// Read an array header and require an exact arity.
fn expect_array(d: &[u8], o: &mut usize, n: u64, what: &str) -> R<()> {
    match read_array_header(d, o)? {
        ContainerEncoding::Definite(c, _) if c == n => Ok(()),
        ContainerEncoding::Definite(c, _) => Err(malformed(format!("{what}: array arity {c} != {n}"))),
        ContainerEncoding::Indefinite => Err(malformed(format!("{what}: indefinite array"))),
    }
}

/// Read an array header, returning the arity (definite only).
fn array_len(d: &[u8], o: &mut usize, what: &str) -> R<u64> {
    match read_array_header(d, o)? {
        ContainerEncoding::Definite(c, _) => Ok(c),
        ContainerEncoding::Indefinite => Err(malformed(format!("{what}: indefinite array"))),
    }
}

/// Iterate a CBOR map (definite OR indefinite — cardano-node encodes ledger maps as indefinite
/// `bf … ff`), invoking `f` once per entry. `f` must advance the offset past exactly one key + value.
fn map_each<F>(d: &[u8], o: &mut usize, what: &str, mut f: F) -> R<()>
where
    F: FnMut(&mut usize) -> R<()>,
{
    match read_map_header(d, o)? {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                f(o)?;
            }
        }
        ContainerEncoding::Indefinite => loop {
            if *o >= d.len() {
                return Err(malformed(format!("{what}: unterminated indefinite map")));
            }
            if d[*o] == 0xff {
                *o += 1; // consume the break byte
                break;
            }
            f(o)?;
        },
    }
    Ok(())
}

fn read_u64(d: &[u8], o: &mut usize, what: &str) -> R<u64> {
    let (v, neg, _) = read_any_int(d, o)?;
    if neg {
        return Err(malformed(format!("{what}: unexpected negative int")));
    }
    Ok(v)
}

fn read_fixed_bytes(d: &[u8], o: &mut usize, n: usize, what: &str) -> R<Vec<u8>> {
    let (b, _) = read_bytes(d, o)?;
    if b.len() != n {
        return Err(malformed(format!("{what}: byte len {} != {n}", b.len())));
    }
    Ok(b)
}

fn hash28(b: Vec<u8>) -> Hash28 {
    let mut a = [0u8; 28];
    a.copy_from_slice(&b);
    Hash28(a)
}
fn hash32(b: Vec<u8>) -> Hash32 {
    let mut a = [0u8; 32];
    a.copy_from_slice(&b);
    Hash32(a)
}

/// Decode a `StakeCredential` = array(2)[credType, hash28].
fn read_credential(d: &[u8], o: &mut usize) -> R<StakeCredential> {
    expect_array(d, o, 2, "credential")?;
    let tag = read_u64(d, o, "credential.tag")?;
    let h = hash28(read_fixed_bytes(d, o, 28, "credential.hash")?);
    match tag {
        CRED_KEYHASH => Ok(StakeCredential::KeyHash(h)),
        CRED_SCRIPTHASH => Ok(StakeCredential::ScriptHash(h)),
        other => Err(malformed(format!("credential tag {other}"))),
    }
}

/// Decode one `PoolParams` value (the map VALUE; the pool id is the map key). The on-wire array is
/// `[vrf(32), pledge, cost, margin(tag30 [n,d]), rewardAcct[net,hash28], owners(set), relays,
/// metadata, …]`; Ade keeps the leadership-relevant prefix.
fn read_pool_params(d: &[u8], o: &mut usize, pool_id: PoolId) -> R<PoolParams> {
    let n = array_len(d, o, "poolparams")?;
    if n < 6 {
        return Err(malformed(format!("poolparams arity {n} < 6")));
    }
    let vrf = hash32(read_fixed_bytes(d, o, 32, "poolparams.vrf")?);
    if vrf.0 == [0u8; 32] {
        return Err(LedgerDbStateError::ZeroVrf(pool_id));
    }
    let pledge = Coin(read_u64(d, o, "poolparams.pledge")?);
    let cost = Coin(read_u64(d, o, "poolparams.cost")?);
    // margin: CBOR rational tag(30) [num, den].
    let (mt, _) = read_tag(d, o)?;
    if mt != TAG_RATIONAL {
        return Err(malformed(format!("poolparams.margin tag {mt} != 30")));
    }
    expect_array(d, o, 2, "poolparams.margin")?;
    let mnum = read_u64(d, o, "margin.num")?;
    let mden = read_u64(d, o, "margin.den")?;
    // reward account: array(2)[networkByte, credHash28] -> 29-byte stake address.
    expect_array(d, o, 2, "poolparams.rewardAcct")?;
    let net = read_u64(d, o, "rewardAcct.net")?;
    let rcred = read_fixed_bytes(d, o, 28, "rewardAcct.hash")?;
    let mut reward_account = Vec::with_capacity(29);
    reward_account.push(0xe0u8 | ((net as u8) & 0x0f)); // staking key-hash header | network
    reward_account.extend_from_slice(&rcred);
    // owners: a set (CBOR tag 258) of key hashes, or a plain array.
    let owners = read_hash28_set(d, o, "poolparams.owners")?;
    // remaining fields (relays, metadata, deposit, …) are not leadership-relevant: skip them.
    for _ in 6..n {
        skip_item(d, o)?;
    }
    Ok(PoolParams {
        pool_id,
        vrf_hash: vrf,
        pledge,
        cost,
        margin: (mnum, mden),
        reward_account,
        owners,
    })
}

/// Read a set/array of 28-byte hashes (owners). Tolerates the CBOR set tag (258).
fn read_hash28_set(d: &[u8], o: &mut usize, what: &str) -> R<Vec<Hash28>> {
    if ade_codec::cbor::peek_major(d, *o)? == 6 {
        let _ = read_tag(d, o)?; // set tag (258)
    }
    let n = array_len(d, o, what)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        out.push(hash28(read_fixed_bytes(d, o, 28, what)?));
    }
    Ok(out)
}

/// Navigate the HardFork telescope to the CURRENT era, EXPLICITLY: past eras encode a second field
/// that is an end bound (`array(3)`); the current era encodes the live state (`array(2)`). The
/// current era index = the count of past eras; require it == Conway.
fn navigate_to_current_era(d: &[u8], o: &mut usize) -> R<()> {
    let eras = array_len(d, o, "telescope")?;
    for idx in 0..eras as usize {
        expect_array(d, o, 2, "telescope.era")?; // [start_bound, secondField]
        skip_item(d, o)?; // start_bound
        // peek the second field: array(3) => end bound (past); array(2) => the live state (current).
        let second = array_len(d, o, "telescope.era.second")?;
        if second == 3 {
            for _ in 0..3 {
                skip_item(d, o)?; // skip the end bound's fields
            }
            continue; // a past era
        }
        if second == 2 {
            // CURRENT era at `idx`. The offset is now at the live state array's first element.
            if idx != CONWAY_TELESCOPE_INDEX {
                return Err(LedgerDbStateError::UnsupportedEra { current_index: idx });
            }
            return Ok(());
        }
        return Err(malformed(format!("telescope.era second arity {second}")));
    }
    Err(malformed("telescope: no current era"))
}

/// The byte range of the `headerState` (the second field of the ExtLedgerState). Its chainDepState
/// carries the five Praos nonces; scoping the nonce scan here avoids the VRF/hash false positives a
/// whole-state `82 01 5820` scan hits elsewhere in the ledger state.
fn headerstate_slice(d: &[u8]) -> R<&[u8]> {
    let o = &mut 0usize;
    expect_array(d, o, 2, "top")?;
    let _ = read_u64(d, o, "version")?;
    expect_array(d, o, 2, "extLedgerState")?;
    skip_item(d, o)?; // the whole ledgerState (the HardFork telescope)
    let start = *o;
    skip_item(d, o)?; // the headerState
    Ok(&d[start..*o])
}

/// Extract the five PraosState nonces from the headerState bytes: the TRAILING five contiguous
/// `Nonce` wrappers (`82 01 5820 <32B>`), in record order [evolving, candidate, epoch, lab,
/// lastEpochBlock]. The PraosState is the final structure of the snapshot, so its five nonce fields
/// are the last five `Nonce`s and end at the headerState's tail; a preceding HardFork tick nonce is
/// the sixth pattern, which a naive whole-scan can't tell apart. Fail-closed: require >= 5 patterns
/// and the trailing five contiguous (36 bytes apart).
fn extract_praos_nonces_v2(hs: &[u8]) -> R<PraosNonces> {
    const PREFIX: [u8; 4] = [0x82, 0x01, 0x58, 0x20];
    let mut offs: Vec<usize> = Vec::new();
    let mut i = 0usize;
    while i + 36 <= hs.len() {
        if hs[i..i + 4] == PREFIX {
            offs.push(i);
            i += 36;
        } else {
            i += 1;
        }
    }
    if offs.len() < 5 {
        return Err(malformed(format!("praos nonces: found {} < 5", offs.len())));
    }
    let tail = &offs[offs.len() - 5..];
    for w in tail.windows(2) {
        if w[1] - w[0] != 36 {
            return Err(malformed("praos nonces: trailing five not contiguous"));
        }
    }
    let nonce_at = |o: usize| -> Nonce {
        let mut b = [0u8; 32];
        b.copy_from_slice(&hs[o + 4..o + 36]);
        Nonce(b)
    };
    Ok(PraosNonces {
        evolving: nonce_at(tail[0]),
        candidate: nonce_at(tail[1]),
        epoch: nonce_at(tail[2]),
        lab: nonce_at(tail[3]),
        last_epoch_block: nonce_at(tail[4]),
    })
}

/// Decode the V2 LedgerDB `state` CBOR into the Stage-1 probe. `manifest_epoch` is the epoch implied
/// by the verified Mithril-certified point (the authority); the decoded epoch is cross-checked.
pub fn probe_ledgerdb_state(state_cbor: &[u8], manifest_epoch: u64) -> R<LedgerDbStateProbe> {
    let d = state_cbor;
    let o = &mut 0usize;
    // top: array(2)[version, [telescope, headerState]]
    expect_array(d, o, 2, "top")?;
    let _version = read_u64(d, o, "version")?;
    expect_array(d, o, 2, "extLedgerState")?; // [telescope, headerState]
    // telescope -> current era state (the offset lands at the live state array's element 0).
    navigate_to_current_era(d, o)?;
    // current era live state = array(2)[tag(int), array(N)[?, NewEpochState, …]]
    skip_item(d, o)?; // state[0] (an era/serialisation tag int)
    let inner_n = array_len(d, o, "eraState.inner")?;
    if inner_n < 2 {
        return Err(malformed(format!("eraState.inner arity {inner_n} < 2")));
    }
    skip_item(d, o)?; // inner[0]
    // NewEpochState = array(7)[epoch, blocksPrev, blocksCur, EpochState, rewardUpdate, [PoolDistr,…], stashed]
    expect_array(d, o, 7, "NewEpochState")?;
    let epoch = read_u64(d, o, "nes.epoch")?;
    if epoch != manifest_epoch {
        return Err(LedgerDbStateError::EpochMismatch {
            manifest_epoch,
            decoded_epoch: epoch,
        });
    }
    skip_item(d, o)?; // blocksPrev
    skip_item(d, o)?; // blocksCur
    // EpochState = array(4)[accountState, LedgerState, snapshots, nonMyopic]
    expect_array(d, o, 4, "EpochState")?;
    skip_item(d, o)?; // accountState
    // LedgerState = array(2)[CertState, UTxOState]
    expect_array(d, o, 2, "LedgerState")?;
    let (pool, delegation) = read_cert_state(d, o)?;
    skip_item(d, o)?; // UTxOState (the empty UTxO + the incremental stake distr; Stage 2 reads `tables`)
    skip_item(d, o)?; // EpochState.snapshots
    skip_item(d, o)?; // EpochState.nonMyopic
    skip_item(d, o)?; // nes.rewardUpdate
    // nes[5] = [PoolDistr, totalActiveStake] -> the cross-check
    let pd_n = array_len(d, o, "nes.poolDistrWrapper")?;
    let pool_distr = read_pool_distr(d, o)?;
    for _ in 1..pd_n {
        skip_item(d, o)?;
    }
    // cross-check PoolDistr <-> pools on VRF.
    for (pid, (_, vrf)) in &pool_distr {
        if let Some(pp) = pool.pools.get(pid) {
            if &pp.vrf_hash != vrf {
                return Err(LedgerDbStateError::PoolDistrVrfMismatch(pid.clone()));
            }
        }
    }
    // nonces: the five PraosState nonces (trailing record fields), scoped to the headerState.
    let hs = headerstate_slice(state_cbor)?;
    let nonces: PraosNonces = extract_praos_nonces_v2(hs)?;
    let cert_state = CertState { delegation, pool };
    // canonical round-trip self-check: the decoded CertState must encode + decode back identically.
    let encoded = encode_cert_state(&cert_state);
    let redecoded = decode_cert_state(&encoded)
        .map_err(|e| malformed(format!("cert_state round-trip: {e:?}")))?;
    if redecoded != cert_state {
        return Err(LedgerDbStateError::RoundTripMismatch);
    }
    let vrf_count = cert_state
        .pool
        .pools
        .values()
        .filter(|p| p.vrf_hash.0 != [0u8; 32])
        .count();
    let probe = LedgerDbStateProbe {
        era_index: CONWAY_TELESCOPE_INDEX,
        epoch,
        cert_state_commitment: blake2b_256(&encoded),
        active_pool_count: cert_state.pool.pools.len(),
        vrf_count,
        future_pool_count: cert_state.pool.future_pools.len(),
        retiring_count: cert_state.pool.retiring.len(),
        registration_count: cert_state.delegation.registrations.len(),
        delegation_count: cert_state.delegation.delegations.len(),
        reward_count: cert_state.delegation.rewards.len(),
        pool_distr_count: pool_distr.len(),
        praos_epoch_nonce: Hash32(nonces.epoch.0),
    };
    Ok(probe)
}

/// Decode the CertState (LedgerState[0]) = array(3)[VState, PState, DState] -> (PoolState, DelegationState).
fn read_cert_state(d: &[u8], o: &mut usize) -> R<(PoolState, DelegationState)> {
    expect_array(d, o, 3, "CertState")?;
    skip_item(d, o)?; // VState
    // PState = array(4)[?, psStakePoolParams(28B->PoolParams), psFutureStakePoolParams, psRetiring]
    expect_array(d, o, 4, "PState")?;
    skip_item(d, o)?; // PState[0] (32-byte-key map; not the active pool params)
    let pools = read_pool_map(d, o)?;
    let future_pools = read_pool_map(d, o)?;
    let retiring = read_retiring(d, o)?;
    let pool = PoolState {
        pools,
        future_pools,
        retiring,
    };
    let delegation = read_dstate(d, o)?;
    Ok((pool, delegation))
}

/// Decode a `Map PoolId PoolParams` (28-byte pool-id key -> PoolParams array).
fn read_pool_map(d: &[u8], o: &mut usize) -> R<BTreeMap<PoolId, PoolParams>> {
    let mut out = BTreeMap::new();
    map_each(d, o, "poolMap", |o| {
        let pid = PoolId(hash28(read_fixed_bytes(d, o, 28, "poolMap.key")?));
        let pp = read_pool_params(d, o, pid.clone())?;
        out.insert(pid, pp);
        Ok(())
    })?;
    Ok(out)
}

/// Decode `psRetiring` = `Map PoolId EpochNo`.
fn read_retiring(d: &[u8], o: &mut usize) -> R<BTreeMap<PoolId, EpochNo>> {
    let mut out = BTreeMap::new();
    map_each(d, o, "retiring", |o| {
        let pid = PoolId(hash28(read_fixed_bytes(d, o, 28, "retiring.key")?));
        let e = EpochNo(read_u64(d, o, "retiring.epoch")?);
        out.insert(pid, e);
        Ok(())
    })?;
    Ok(out)
}

/// Decode the DState (CertState[2]) = array(4)[UMap, futureGenDelegs, genDelegs, iRewards].
/// The UMap = `Map StakeCredential [reward, deposit, pool?, drep?]`. Every entry is a registration;
/// entries with a pool are delegations; rewards = cred -> reward.
fn read_dstate(d: &[u8], o: &mut usize) -> R<DelegationState> {
    expect_array(d, o, 4, "DState")?;
    let mut ds = DelegationState::new();
    map_each(d, o, "umap", |o| {
        let cred = read_credential(d, o)?;
        // value: array(4)[reward, deposit, pool?(28B|null), drep?]
        let vn = array_len(d, o, "umap.value")?;
        if vn < 3 {
            return Err(malformed(format!("umap.value arity {vn} < 3")));
        }
        let reward = Coin(read_u64(d, o, "umap.reward")?);
        let deposit = Coin(read_u64(d, o, "umap.deposit")?);
        // pool?: a 28-byte hash, or null (CBOR simple 22) / an empty container.
        let maj = ade_codec::cbor::peek_major(d, *o)?;
        if maj == 2 {
            // bytes -> delegated pool
            let pid = PoolId(hash28(read_fixed_bytes(d, o, 28, "umap.pool")?));
            ds.delegations.insert(cred.clone(), pid);
        } else {
            skip_item(d, o)?; // null / absent delegation
        }
        for _ in 3..vn {
            skip_item(d, o)?; // drep, etc.
        }
        ds.registrations.insert(cred.clone(), deposit);
        ds.rewards.insert(cred, reward);
        Ok(())
    })?;
    // futureGenDelegs, genDelegs, iRewards.
    skip_item(d, o)?;
    skip_item(d, o)?;
    skip_item(d, o)?;
    Ok(ds)
}

/// Decode the PoolDistr = `Map PoolId [stakeFraction, stake, vrf]`. Returns pool -> (stake, vrf).
fn read_pool_distr(d: &[u8], o: &mut usize) -> R<BTreeMap<PoolId, (u64, Hash32)>> {
    let mut out = BTreeMap::new();
    map_each(d, o, "poolDistr", |o| {
        let pid = PoolId(hash28(read_fixed_bytes(d, o, 28, "poolDistr.key")?));
        let vn = array_len(d, o, "poolDistr.value")?;
        if vn < 3 {
            return Err(malformed(format!("poolDistr.value arity {vn} < 3")));
        }
        skip_item(d, o)?; // stake fraction (rational)
        let stake = read_u64(d, o, "poolDistr.stake")?;
        let vrf = hash32(read_fixed_bytes(d, o, 32, "poolDistr.vrf")?);
        for _ in 3..vn {
            skip_item(d, o)?;
        }
        out.insert(pid, (stake, vrf));
        Ok(())
    })?;
    Ok(out)
}
