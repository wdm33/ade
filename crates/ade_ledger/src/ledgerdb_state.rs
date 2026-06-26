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
    peek_major, read_any_int, read_array_header, read_bytes, read_map_header, read_tag, skip_item,
    ContainerEncoding,
};
use ade_crypto::blake2b::blake2b_256;
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

use crate::bootstrap_anchor::SeedPoint;
use crate::consensus_input_extract::{Nonce, PraosNonces};
use crate::delegation::{CertState, DelegationState, PoolParams, PoolState};
use crate::pparams::{MinUtxoRule, ProtocolParameters};
use crate::rational::Rational;
use crate::snapshot::cert_state::{decode_cert_state, encode_cert_state};

/// The Conway era index in the HardFork telescope (Byron=0 … Conway=6).
const CONWAY_TELESCOPE_INDEX: usize = 6;
// The stake-credential CBOR discriminant (0=KeyHash, 1=ScriptHash) lives in `crate::cred` — the
// SINGLE source of truth shared by every native-state decoder; `read_credential` routes through it.
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
    crate::cred::stake_credential_from_ledger_tag(tag, h)
        .ok_or_else(|| malformed(format!("credential tag {tag}")))
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

/// The chain tip (slot + header hash) the snapshot's `headerState` AnnTip carries -- the point Ade
/// must ChainSync-follow from after a native Mithril bootstrap, and the native source for a
/// manifest's `certified_point` (replacing any out-of-band frozen-node extraction).
///
/// `headerState` = `array(2)[WithOrigin(AnnTip), chainDepState]`. The `WithOrigin "At"` wrapper is
/// `array(1)[ [eraIndex, [slotNo, headerHash(32B), blockNo]] ]` (the HardFork era discriminator wraps
/// the inner AnnTip). `Origin` (`array(0)`) means a genesis-tip snapshot with no block -> fail-closed.
pub fn decode_ledgerdb_tip(state_cbor: &[u8]) -> R<(SlotNo, Hash32)> {
    let hs = headerstate_slice(state_cbor)?;
    let o = &mut 0usize;
    expect_array(hs, o, 2, "headerState")?; // [tip, chainDepState]
    match array_len(hs, o, "headerState.tip(WithOrigin)")? {
        0 => Err(malformed(
            "headerState tip is Origin: snapshot has no block to follow from",
        )),
        1 => {
            expect_array(hs, o, 2, "tip.hardforkAnnTip")?; // [eraIndex, annTip]
            skip_item(hs, o)?; // eraIndex (HardFork era discriminator; not needed for the point)
            expect_array(hs, o, 3, "tip.annTip")?; // [slotNo, headerHash, blockNo]
            let slot = read_u64(hs, o, "tip.slotNo")?;
            let hash = hash32(read_fixed_bytes(hs, o, 32, "tip.headerHash")?);
            Ok((SlotNo(slot), hash))
        }
        n => Err(malformed(format!(
            "headerState tip WithOrigin arity {n} (expected 0 = Origin | 1 = At)"
        ))),
    }
}

#[cfg(test)]
mod tip_tests {
    use super::*;

    #[test]
    fn read_credential_uses_canonical_ledger_tags() {
        // cardano-ledger Credential CBOR: tag 0 = KeyHash, tag 1 = ScriptHash. A flip mislabels every
        // delegator (key<->script) so the delegation map no longer joins the UTxO and the cross-epoch
        // leader schedule collapses (the seed+2 stake came out ~10% of real). This is the SINGLE
        // stake-credential decoder on the native-bootstrap path; pin its convention.
        let mut key_cred = vec![0x82u8, 0x00, 0x58, 0x1c]; // array(2)[tag=0, bytes(28)]
        key_cred.extend_from_slice(&[0xaa; 28]);
        let mut o = 0;
        assert_eq!(
            read_credential(&key_cred, &mut o).expect("decode"),
            StakeCredential::KeyHash(Hash28([0xaa; 28])),
            "tag 0 must decode to KeyHash"
        );
        let mut script_cred = vec![0x82u8, 0x01, 0x58, 0x1c]; // array(2)[tag=1, bytes(28)]
        script_cred.extend_from_slice(&[0xbb; 28]);
        let mut o = 0;
        assert_eq!(
            read_credential(&script_cred, &mut o).expect("decode"),
            StakeCredential::ScriptHash(Hash28([0xbb; 28])),
            "tag 1 must decode to ScriptHash"
        );
    }

    #[test]
    fn decode_ledgerdb_tip_reads_anntip_slot_and_hash() {
        // [version, [ledgerState=a(0), headerState]] where
        // headerState = [ At[ [era6, [slot=100, hash, blockNo=50]] ], chainDep=a(0) ].
        let mut s: Vec<u8> = vec![0x82, 0x01, 0x82, 0x80];
        s.push(0x82); // headerState a(2)
        s.push(0x81); // WithOrigin At: a(1)
        s.push(0x82); // hardfork annTip a(2)
        s.push(0x06); // eraIndex = 6 (Conway)
        s.push(0x83); // annTip a(3)
        s.extend_from_slice(&[0x18, 0x64]); // slotNo = 100
        s.extend_from_slice(&[0x58, 0x20]); // bytes(32)
        s.extend_from_slice(&[0xAB; 32]); // headerHash
        s.extend_from_slice(&[0x18, 0x32]); // blockNo = 50
        s.push(0x80); // chainDepState a(0)
        let (slot, hash) = decode_ledgerdb_tip(&s).expect("tip parse");
        assert_eq!(slot, SlotNo(100));
        assert_eq!(hash, Hash32([0xAB; 32]));
    }

    #[test]
    fn decode_ledgerdb_tip_origin_is_fail_closed() {
        // headerState = [ Origin=a(0), chainDep=a(0) ]
        let s: Vec<u8> = vec![0x82, 0x01, 0x82, 0x80, 0x82, 0x80, 0x80];
        assert!(decode_ledgerdb_tip(&s).is_err());
    }
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
    if offs.len() < 6 {
        return Err(malformed(format!("praos nonces: found {} < 6", offs.len())));
    }
    let tail = &offs[offs.len() - 6..];
    for w in tail.windows(2) {
        if w[1] - w[0] != 36 {
            return Err(malformed("praos nonces: trailing six not contiguous"));
        }
    }
    let nonce_at = |o: usize| -> Nonce {
        let mut b = [0u8; 32];
        b.copy_from_slice(&hs[o + 4..o + 36]);
        Nonce(b)
    };
    // The Praos chain-dep state (ouroboros-consensus PraosState) serializes SIX contiguous nonce
    // wrappers in record order [evolving, candidate, epoch, previousEpoch, lab, lastEpochBlock]
    // (Ouroboros/Consensus/Protocol/Praos.hs:272-283/313). epochNonce (the leader-VRF eta0) is
    // tail[2] and lastEpochBlockNonce is tail[5] (both cross-checked by value), and the CANDIDATE is
    // tail[1] -- PROVEN by value: eta0(N+1) = blake2b(tail[1] || tail[5]) reproduces the live node's
    // epoch nonce for the next epoch (ECA-5). The EVOLVING nonce is tail[0] -- the prior 5-nonce scan
    // dropped it (it took the last FIVE = [candidate, epoch, previousEpoch, lab, lastEpochBlock] and
    // mis-read previousEpoch as evolving), feeding a wrong candidate into eta0(seed+2). DC-EPOCH-16:
    // the boundary-2 live gate caught it -- boundary 1 used the seeded candidate, never the evolving.
    Ok(PraosNonces {
        evolving: nonce_at(tail[0]),
        candidate: nonce_at(tail[1]),
        epoch: nonce_at(tail[2]),
        lab: nonce_at(tail[4]),
        last_epoch_block: nonce_at(tail[5]),
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

// ===========================================================================
// S1a — native non-UTxO snapshot decoder
// (MITHRIL-VERIFIED-ANCHOR-INTEGRATION). DERIVED, BLUE, deterministic.
//
// Extends the Stage-1 navigation to decode EVERY snapshot-present non-UTxO field
// natively from the V2 LedgerDB `state` file — the FULL CertState, all five Praos
// nonces, the pool distribution with VRF bindings, the Conway current protocol
// parameters (NewEpochState.esLState.lsUTxOState.utxosGovState.curPParams), the
// reserves/treasury pots (esAccountState), and the previous-epoch block production
// (nesBprev) — and binds them all into one commitment. NO cardano-cli, NO JSON
// consensus-input bundle, NO operator seed: the function takes only `state_cbor`,
// the manifest-certified `point`, and `manifest_epoch`.
//
// Empirical grounding (real preprod snapshot, cardano-node 11.0.1 / cardano-ledger
// -conway 1.22.1.0, slot 126400064, epoch 296):
//   NewEpochState = array(7); nesEL=[0] (epoch 296); nesBprev=[1] (INDEFINITE map
//   KeyHash(28) -> Natural, 66 pools); nesBcur=[2]; EpochState=[3]=array(4);
//   esAccountState=EpochState[0]=array(2)[treasury, reserves] (cardano-ledger order,
//   treasury FIRST: 1_890_267_427_632_547 / 13_051_749_596_873_397); LedgerState=
//   EpochState[1]=array(2)[CertState, UTxOState]; UTxOState=array(6)[utxo(empty),
//   deposited, fees, govState=array(7), incrStake, donation]; the Conway curPParams
//   = utxosGovState[3] = array(31) with the field layout recorded in
//   `read_conway_pparams`. (`d` and `extraEntropy` are removed in Conway — the
//   array has NO decentralization field; protocolVersion sits at index 12.)
// ===========================================================================

/// The Conway on-wire current-PParams array arity (cardano-ledger-conway 1.22.1.0).
/// `d`/`extraEntropy` removed vs Shelley; governance fields appended.
const CONWAY_PPARAMS_FIELDS: u64 = 31;
/// `ConwayGovState` (cardano-ledger 1.22) is a 7-field array; `curPParams` is field 3.
const CONWAY_GOV_STATE_FIELDS: u64 = 7;
const CONWAY_GOV_STATE_CURPPARAMS_INDEX: usize = 3;
/// `UTxOState` (Conway) = `[utxo, deposited, fees, govState, incrStake, donation]`.
const UTXO_STATE_GOVSTATE_INDEX: usize = 3;
/// CBOR tag for a rational number (`a0`, `rho`, `tau`, `minFeeRefScriptCostPerByte`).
const TAG_RATIONAL_PP: u64 = 30;
/// The Cardano mainnet network magic. The internal network id is derived from the
/// manifest magic: mainnet magic -> `network_id = 1`; any other (testnet) magic -> 0.
const MAINNET_NETWORK_MAGIC: u32 = 764_824_073;

/// Derive the internal Cardano network id from a network magic. Mainnet magic maps to
/// `1`; every other (testnet) magic maps to `0`. Cardano binds authority-bearing state to
/// the intended network, so the imported state's network id is the manifest's, not a
/// placeholder.
fn network_id_from_magic(network_magic: u32) -> u8 {
    if network_magic == MAINNET_NETWORK_MAGIC {
        1
    } else {
        0
    }
}

/// Why a native non-UTxO snapshot decode fails. Every variant is TERMINAL
/// (fail-closed before any field is used; never a default / partial emission).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeNonUtxoError {
    /// Structurally malformed CBOR (wrong major type / truncated / unexpected shape).
    MalformedCbor(String),
    /// The current HardFork era is not Conway (telescope index != 6).
    UnsupportedEra { current_index: usize },
    /// The decoded NES epoch does not match the manifest-certified epoch.
    EpochMismatch { manifest_epoch: u64, decoded_epoch: u64 },
    /// An active pool's VRF is zero — a faithful decode requires the real VRF.
    ZeroVrf(PoolId),
    /// `PoolDistr` and the decoded pools disagree on a pool's VRF (terminal even at zero stake).
    PoolDistrVrfMismatch(PoolId),
    /// `esAccountState` is absent / malformed where the snapshot must carry it.
    AccountStateMissing(String),
    /// `nesBprev` (previous-epoch block production) is absent / malformed.
    BlockProductionMissing(String),
    /// A `nesBprev` pool id is not among the CertState pools (block-producer coherence).
    BlockProductionUnknownPool(PoolId),
    /// The Conway `GovState` / `curPParams` is absent / malformed where expected.
    ProtocolParamsMissing(String),
    /// The decoded CertState does not survive a canonical encode/decode round-trip.
    RoundTripMismatch,
}

/// DIAGNOSTIC ONLY — never a bootstrap verdict. The distribution of pool reward-account
/// network nibbles observed in the snapshot. The manifest network magic is the SOLE network
/// authority (see `NativeSnapshotNonUtxoState::network_id`); reward-account nibbles are
/// OPERATOR-controlled ledger data (real preprod snapshots carry a MIX of net-0 and net-1
/// pool reward accounts), so this is recorded as canonical probe evidence and NEVER accepts
/// or rejects the snapshot. A `Uniform` nibble disagreeing with the derived `network_id` is
/// recorded, not rejected — operator metadata is not network authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewardNibbleObservation {
    /// No pool reward accounts to observe.
    None,
    /// Every pool reward account carries the same network nibble (the value).
    Uniform(u8),
    /// Pool reward accounts carry a mix of network nibbles (the real preprod case).
    Mixed,
}

/// Observe (DIAGNOSTIC, never a verdict) the pool reward-account network-nibble distribution.
fn observe_reward_account_nibbles(cert_state: &CertState) -> RewardNibbleObservation {
    let mut nibbles = cert_state
        .pool
        .pools
        .values()
        .filter_map(|pp| pp.reward_account.first().map(|h| h & 0x0f));
    match nibbles.next() {
        None => RewardNibbleObservation::None,
        Some(first) if nibbles.all(|n| n == first) => RewardNibbleObservation::Uniform(first),
        Some(_) => RewardNibbleObservation::Mixed,
    }
}

impl From<ade_codec::CodecError> for NativeNonUtxoError {
    fn from(e: ade_codec::CodecError) -> Self {
        NativeNonUtxoError::MalformedCbor(format!("{e:?}"))
    }
}

type Rn<T> = Result<T, NativeNonUtxoError>;

fn nn_malformed(detail: impl Into<String>) -> NativeNonUtxoError {
    NativeNonUtxoError::MalformedCbor(detail.into())
}

/// The COMPLETE authoritative non-UTxO ledger state decoded natively from a V2 LedgerDB
/// `state` file, bound to the manifest-certified point. Every field is decoded from the
/// snapshot (no silent default for any snapshot-present field) and bound into the
/// commitment. BLUE: an emitted structure + a commitment; NOT assembled into
/// `LedgerState`/`PraosChainDepState` and NOT persisted (that is S1b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSnapshotNonUtxoState {
    /// The telescope era — EXPLICIT Conway (telescope index 6), never inferred.
    pub era: CardanoEra,
    /// The internal Cardano network id derived from the manifest network magic
    /// (mainnet -> 1, testnet -> 0). Binds the imported state to the intended
    /// network; the same value is set on `protocol_params.network_id`.
    pub network_id: u8,
    /// The NewEpochState `nesEL` epoch, cross-checked `== manifest_epoch`.
    pub epoch: EpochNo,
    /// The manifest-certified point (bound through, not decoded from the state file).
    pub point: SeedPoint,
    /// The FULL `CertState` (`nesEs.esLState.lsCertState`) — the struct, not a hash.
    pub cert_state: CertState,
    /// All five Praos nonces (evolving, candidate, epoch/eta0, lab, previous-epoch).
    pub praos_nonces: PraosNonces,
    /// `nesPd` (PoolDistr): pool -> (active stake, VRF) — the VRF bindings.
    pub pool_distr: BTreeMap<PoolId, (u64, Hash32)>,
    /// ECA-5 (DC-EPOCH-15): the MARK stake snapshot's PoolDistr — the seed+1 (next-epoch) leadership,
    /// the BOOTSTRAP BRIDGE authority. `calculatePoolDistr(ssStakeMark)`: pool -> (active stake, VRF).
    pub mark_pool_distr: BTreeMap<PoolId, (u64, Hash32)>,
    /// The Conway current protocol parameters (`utxosGovState.curPParams`).
    pub protocol_params: ProtocolParameters,
    /// `esAccountState.reserves`.
    pub reserves: Coin,
    /// `esAccountState.treasury`.
    pub treasury: Coin,
    /// `nesBprev`: blocks each pool made in the previous epoch.
    pub block_production: BTreeMap<PoolId, u64>,
    /// DIAGNOSTIC evidence only (never a verdict): the pool reward-account network-nibble
    /// distribution. `network_id` above (manifest-derived) is the sole network authority.
    pub reward_nibble_observation: RewardNibbleObservation,
}

/// ECA-5 piece (a): decode the MARK stake snapshot (`ssStakeMark`, the seed+1 leadership) and compute
/// its PoolDistr. In this ledger version `SnapShot = array(2)[ssStake: map(StakeCredential -> [Coin,
/// PoolId]), ssPoolParams: map(PoolId -> PoolParams)]` -- stake + delegation are COMBINED in ssStake.
/// Active stake per pool is `calculatePoolDistr`: sum the coin of every credential delegated to the
/// pool. The per-pool VRF is taken from the durable cert-state registrations (`cert_pool_vrf`) because
/// this version's ssPoolParams is not vrf-first like the cert-state encoding; that coherence is the
/// mark<->nesPd VRF cross-check. Pools with zero stake (or no cert-state registration) are dropped.
/// Derived from the MARK snapshot + the durable cert-state ALONE -- never nesPd / window-replay / an oracle.
fn read_mark_snapshot_pool_distr(
    d: &[u8],
    o: &mut usize,
    cert_pool_vrf: &BTreeMap<PoolId, Hash32>,
) -> R<BTreeMap<PoolId, (u64, Hash32)>> {
    let sn = array_len(d, o, "mark.SnapShot")?;
    // This ledger version's SnapShot = array(2)[ssStake, ssPoolParams] (stake + delegation COMBINED).
    if sn < 2 {
        return Err(malformed(format!("mark.SnapShot arity {sn} < 2")));
    }
    // map0 = ssStake: map(StakeCredential -> [Coin, PoolId]) -- stake + delegation in one entry.
    // `calculatePoolDistr`: sum each delegated credential's coin into its pool.
    let mut pool_stake: BTreeMap<PoolId, u64> = BTreeMap::new();
    map_each(d, o, "mark.stake", |o| {
        let _cred = read_credential(d, o)?; // key: array(2)[tag, hash28] (skipped; pool is in the value)
        expect_array(d, o, 2, "mark.stake.val")?; // value: [coin, pool]
        let coin = read_u64(d, o, "mark.stake.coin")?;
        let pool = PoolId(hash28(read_fixed_bytes(d, o, 28, "mark.stake.pool")?));
        let e = pool_stake.entry(pool).or_insert(0u64);
        *e = e.saturating_add(coin);
        Ok(())
    })?;
    // map1 = ssPoolParams: this ledger version encodes it differently (NOT vrf-first like the cert-state
    // pool registrations), so the VRF is taken from the durable cert-state registrations -- which decode
    // correctly and whose coherence with this PoolDistr is the mark<->nesPd VRF cross-check. Skip map1.
    for _ in 1..sn {
        skip_item(d, o)?;
    }
    // build pool -> (active_stake, vrf). A staked pool with no cert-state registration (retired between the
    // mark snapshot and the seed point) cannot lead in seed+1 -> omit it (never fabricate a VRF).
    let mut out: BTreeMap<PoolId, (u64, Hash32)> = BTreeMap::new();
    for (pool, st) in pool_stake {
        if st == 0 {
            continue;
        }
        if let Some(vrf) = cert_pool_vrf.get(&pool) {
            out.insert(pool, (st, vrf.clone()));
        }
    }
    Ok(out)
}


/// `nn`-error wrapper for [`read_mark_snapshot_pool_distr`] (mirrors `read_pool_distr_nn`).
fn read_mark_snapshot_pool_distr_nn(
    d: &[u8],
    o: &mut usize,
    cert_pool_vrf: &BTreeMap<PoolId, Hash32>,
) -> Rn<BTreeMap<PoolId, (u64, Hash32)>> {
    read_mark_snapshot_pool_distr(d, o, cert_pool_vrf).map_err(|e| match e {
        LedgerDbStateError::ZeroVrf(p) => NativeNonUtxoError::ZeroVrf(p),
        LedgerDbStateError::MalformedCbor(s) => NativeNonUtxoError::MalformedCbor(s),
        other => nn_malformed(format!("mark snapshot: {other:?}")),
    })
}

/// Decode the V2 LedgerDB `state` CBOR into the COMPLETE native non-UTxO ledger state +
/// a commitment over all emitted fields. `point` is the manifest-certified slot+hash;
/// `manifest_epoch` is the manifest-certified epoch (the authority — the decoded NES
/// epoch is cross-checked against it); `manifest_network_magic` is the manifest-certified
/// network magic, from which the internal network id is derived (mainnet -> 1, else 0) and
/// bound onto every authority-bearing field. Fail-closed: any missing / malformed expected
/// field, a non-Conway era, an epoch mismatch, a zero / mismatched VRF, or a block producer
/// outside the CertState pool set is TERMINAL (never defaulted or partial). Network identity
/// is the manifest magic ALONE; reward-account nibbles are diagnostic evidence only and never
/// accept or reject the snapshot.
pub fn decode_native_nonutxo_state(
    state_cbor: &[u8],
    point: SeedPoint,
    manifest_epoch: u64,
    manifest_network_magic: u32,
) -> Rn<(NativeSnapshotNonUtxoState, Hash32)> {
    // Derive the internal network id from the manifest magic (the authority). Every
    // network-bound field below is bound to / checked against THIS id.
    let network_id = network_id_from_magic(manifest_network_magic);
    let d = state_cbor;
    let o = &mut 0usize;
    // top: array(2)[version, [telescope, headerState]]
    nn_expect_array(d, o, 2, "top")?;
    let _version = nn_read_u64(d, o, "version")?;
    nn_expect_array(d, o, 2, "extLedgerState")?; // [telescope, headerState]
    // telescope -> current era state; REQUIRE the current era == Conway (index 6).
    nn_navigate_to_conway(d, o)?;
    // current era live state = array(2)[tag(int), array(N)[?, NewEpochState, …]]
    skip_item(d, o)?; // state[0] (an era/serialisation tag int)
    let inner_n = nn_array_len(d, o, "eraState.inner")?;
    if inner_n < 2 {
        return Err(nn_malformed(format!("eraState.inner arity {inner_n} < 2")));
    }
    skip_item(d, o)?; // inner[0]
    // NewEpochState = array(7)[epoch, nesBprev, nesBcur, EpochState, rewardUpdate, [PoolDistr,…], stashed]
    nn_expect_array(d, o, 7, "NewEpochState")?;
    let epoch = nn_read_u64(d, o, "nes.epoch")?;
    if epoch != manifest_epoch {
        return Err(NativeNonUtxoError::EpochMismatch {
            manifest_epoch,
            decoded_epoch: epoch,
        });
    }
    // nes[1] = nesBprev (previous-epoch block production) — decoded, not skipped.
    let block_production = read_block_production(d, o)?;
    skip_item(d, o)?; // nes[2] nesBcur (current-epoch blocks; not authoritative for this slice)
    // EpochState = array(4)[esAccountState, LedgerState, snapshots, nonMyopic]
    nn_expect_array(d, o, 4, "EpochState")?;
    // esAccountState = array(2)[treasury, reserves] (cardano-ledger order: treasury FIRST).
    let (treasury, reserves) = read_account_state(d, o)?;
    // LedgerState = array(2)[CertState, UTxOState]
    nn_expect_array(d, o, 2, "LedgerState")?;
    let (pool, delegation) = nn_read_cert_state(d, o)?;
    // UTxOState = array(6)[utxo, deposited, fees, govState, incrStake, donation]
    let protocol_params = read_conway_pparams_from_utxo_state(d, o, network_id)?;
    // EpochState.snapshots = SnapShots = array(4)[ssStakeMark, ssStakeSet, ssStakeGo, ssFee] (this ledger
    // version caches NO mark PoolDistr). ECA-5 piece (a): decode ssStakeMark -> the seed+1 leadership
    // bridge (calculatePoolDistr); skip set/go/fee.
    // SnapShots = array(4)[ssStakeMark, ssStakeSet, ssStakeGo, ssFee]; ssStakeMark -> the seed+1 bridge.
    // The seed+1 leadership VRFs come from the durable cert-state pool registrations.
    let cert_pool_vrf: BTreeMap<PoolId, Hash32> = pool
        .pools
        .iter()
        .map(|(p, pp)| (p.clone(), pp.vrf_hash.clone()))
        .collect();
    nn_expect_array(d, o, 4, "EpochState.snapshots")?;
    let mark_pool_distr = read_mark_snapshot_pool_distr_nn(d, o, &cert_pool_vrf)?;
    skip_item(d, o)?; // ssStakeSet
    skip_item(d, o)?; // ssStakeGo
    skip_item(d, o)?; // ssFee
    skip_item(d, o)?; // EpochState.nonMyopic
    skip_item(d, o)?; // nes.rewardUpdate
    // nes[5] = [PoolDistr, totalActiveStake]
    let pd_n = nn_array_len(d, o, "nes.poolDistrWrapper")?;
    let pool_distr = read_pool_distr_nn(d, o)?;
    for _ in 1..pd_n {
        skip_item(d, o)?;
    }

    let cert_state = CertState { delegation, pool };

    // Coherence: every PoolDistr pool's VRF must match the CertState pool's VRF
    // (terminal mismatch, even at zero stake — DC-faithful).
    for (pid, (_, vrf)) in &pool_distr {
        if let Some(pp) = cert_state.pool.pools.get(pid) {
            if &pp.vrf_hash != vrf {
                return Err(NativeNonUtxoError::PoolDistrVrfMismatch(pid.clone()));
            }
        }
    }
    // ECA-5 cross-check: every pool present in BOTH the MARK PoolDistr and nesPd must share the SAME VRF
    // (a pool's VRF is stable across the one-epoch gap unless it re-registered; a mismatch means the MARK
    // decode drifted). TERMINAL on mismatch — the bridge authority is only as trustworthy as this.
    for (pid, (_, mark_vrf)) in &mark_pool_distr {
        if let Some((_, nes_vrf)) = pool_distr.get(pid) {
            if mark_vrf != nes_vrf {
                return Err(NativeNonUtxoError::PoolDistrVrfMismatch(pid.clone()));
            }
        }
    }
    // Coherence: every block producer in the previous epoch must be a known CertState pool.
    for pid in block_production.keys() {
        if !cert_state.pool.pools.contains_key(pid) {
            return Err(NativeNonUtxoError::BlockProductionUnknownPool(pid.clone()));
        }
    }
    // Network identity is bound from the manifest network magic (`network_id`, above) — the
    // SOLE authority. The V2 LedgerDB `state` file carries no network-authoritative identity
    // field of its own; the only network-bound datum is each pool's reward-account
    // stake-address header nibble (`0xe0 | net`), which is OPERATOR-controlled ledger data
    // (real preprod snapshots carry a MIX of net-0 and net-1 pool reward accounts). It is
    // recorded as DIAGNOSTIC evidence only and NEVER accepts or rejects the snapshot — a
    // heuristic on operator metadata, unanimous or otherwise, is not a network authority.
    let reward_nibble_observation = observe_reward_account_nibbles(&cert_state);

    // The five Praos nonces (trailing PraosState record fields), scoped to the headerState.
    let hs = nn_headerstate_slice(state_cbor)?;
    let praos_nonces = nn_extract_praos_nonces(hs)?;

    // Canonical CertState round-trip self-check (encode + decode back identically).
    let encoded_cert = encode_cert_state(&cert_state);
    let redecoded = decode_cert_state(&encoded_cert)
        .map_err(|e| nn_malformed(format!("cert_state round-trip: {e:?}")))?;
    if redecoded != cert_state {
        return Err(NativeNonUtxoError::RoundTripMismatch);
    }

    let state = NativeSnapshotNonUtxoState {
        era: CardanoEra::Conway,
        network_id,
        epoch: EpochNo(epoch),
        point,
        cert_state,
        praos_nonces,
        pool_distr,
        mark_pool_distr,
        protocol_params,
        reserves,
        treasury,
        block_production,
        reward_nibble_observation,
    };
    let commitment = commit_native_nonutxo_state(&state);
    Ok((state, commitment))
}

/// Navigate the HardFork telescope to the current era and REQUIRE it == Conway. The
/// non-Conway terminal carries `NativeNonUtxoError::UnsupportedEra` (never a silent
/// fallback to the latest element).
fn nn_navigate_to_conway(d: &[u8], o: &mut usize) -> Rn<()> {
    navigate_to_current_era(d, o).map_err(|e| match e {
        LedgerDbStateError::UnsupportedEra { current_index } => {
            NativeNonUtxoError::UnsupportedEra { current_index }
        }
        other => nn_malformed(format!("telescope: {other:?}")),
    })
}

/// Decode `nesBprev` = `Map PoolId Natural` (the on-disk encoding is an INDEFINITE map
/// of 28-byte pool-id key -> block count). Returns pool -> blocks.
fn read_block_production(d: &[u8], o: &mut usize) -> Rn<BTreeMap<PoolId, u64>> {
    let mut out = BTreeMap::new();
    // A non-map here is a structural error (the field must be present + a map).
    if peek_major(d, *o).map_err(NativeNonUtxoError::from)? != 5 {
        return Err(NativeNonUtxoError::BlockProductionMissing(format!(
            "nesBprev: expected a map, got major {}",
            peek_major(d, *o).unwrap_or(0xff)
        )));
    }
    nn_map_each(d, o, "nesBprev", |o| {
        let pid = PoolId(nn_hash28(nn_read_fixed_bytes(d, o, 28, "nesBprev.key")?));
        let blocks = nn_read_u64(d, o, "nesBprev.count")?;
        out.insert(pid, blocks);
        Ok(())
    })?;
    Ok(out)
}

/// Decode `esAccountState` = `array(2)[treasury, reserves]` (cardano-ledger AccountState
/// `EncCBOR`: treasury FIRST, reserves second). Returns `(treasury, reserves)`.
fn read_account_state(d: &[u8], o: &mut usize) -> Rn<(Coin, Coin)> {
    if peek_major(d, *o).map_err(NativeNonUtxoError::from)? != 4 {
        return Err(NativeNonUtxoError::AccountStateMissing(
            "esAccountState: not an array".into(),
        ));
    }
    let n = nn_array_len(d, o, "esAccountState")?;
    if n != 2 {
        return Err(NativeNonUtxoError::AccountStateMissing(format!(
            "esAccountState arity {n} != 2"
        )));
    }
    let treasury = Coin(nn_read_u64(d, o, "esAccountState.treasury")?);
    let reserves = Coin(nn_read_u64(d, o, "esAccountState.reserves")?);
    Ok((treasury, reserves))
}

/// Navigate `LedgerState[1]` = `UTxOState` and decode the Conway current protocol
/// parameters from `utxosGovState[curPParams]`. Consumes the whole UTxOState item (the
/// caller's offset lands just past it). The UTxO map / deposited / fees / incrStake /
/// donation are NOT this slice's authority (the UTxO lives in `tables`); only the
/// embedded current PParams is decoded.
fn read_conway_pparams_from_utxo_state(
    d: &[u8],
    o: &mut usize,
    network_id: u8,
) -> Rn<ProtocolParameters> {
    // UTxOState = array(6); descend to [3] = govState.
    if peek_major(d, *o).map_err(NativeNonUtxoError::from)? != 4 {
        return Err(NativeNonUtxoError::ProtocolParamsMissing(
            "UTxOState: not an array".into(),
        ));
    }
    let un = nn_array_len(d, o, "UTxOState")?;
    if (un as usize) <= UTXO_STATE_GOVSTATE_INDEX {
        return Err(NativeNonUtxoError::ProtocolParamsMissing(format!(
            "UTxOState arity {un} has no govState"
        )));
    }
    for _ in 0..UTXO_STATE_GOVSTATE_INDEX {
        skip_item(d, o)?; // utxo, deposited, fees
    }
    // utxosGovState = array(7); curPParams is field 3.
    let gn = nn_array_len(d, o, "utxosGovState")?;
    if gn != CONWAY_GOV_STATE_FIELDS {
        return Err(NativeNonUtxoError::ProtocolParamsMissing(format!(
            "ConwayGovState arity {gn} != {CONWAY_GOV_STATE_FIELDS}"
        )));
    }
    for _ in 0..CONWAY_GOV_STATE_CURPPARAMS_INDEX {
        skip_item(d, o)?; // Proposals, committee, constitution
    }
    let pp = read_conway_pparams(d, o, network_id)?;
    // skip the remaining govState fields (prevPParams, futurePParams, drepPulser).
    for _ in (CONWAY_GOV_STATE_CURPPARAMS_INDEX + 1)..(gn as usize) {
        skip_item(d, o)?;
    }
    // skip the remaining UTxOState fields (incrStake, donation, …).
    for _ in (UTXO_STATE_GOVSTATE_INDEX + 1)..(un as usize) {
        skip_item(d, o)?;
    }
    Ok(pp)
}

/// Decode the Conway on-wire `curPParams` = `array(31)`. Field order (cardano-ledger
/// -conway 1.22.1.0, verified field-by-field against the real preprod snapshot):
///
/// ```text
///  0 minFeeA(Coin)          1 minFeeB(Coin)           2 maxBBSize(u32)
///  3 maxTxSize(u32)         4 maxBHSize(u32)          5 keyDeposit(Coin)
///  6 poolDeposit(Coin)      7 eMax(EpochInterval)     8 nOpt(u16)
///  9 a0(tag30 rational)    10 rho(tag30 rational)    11 tau(tag30 rational)
/// 12 protVer([major,minor]) 13 minPoolCost(Coin)     14 coinsPerUTxOByte(Coin)
/// 15 costModels(map)        16 prices([tag30,tag30])  17 maxTxExUnits([mem,steps])
/// 18 maxBlockExUnits(...)   19 maxValSize(u32)        20 collateralPercentage(u16)
/// 21 maxCollateralInputs    22 poolVotingThresholds   23 drepVotingThresholds
/// 24 committeeMinSize       25 committeeMaxTermLength  26 govActionLifetime
/// 27 govActionDeposit(Coin) 28 dRepDeposit(Coin)      29 dRepActivity
/// 30 minFeeRefScriptCostPerByte(tag30 rational)
/// ```
///
/// Conway REMOVED `d` and `extraEntropy` (present in Shelley PParams) — there is no
/// decentralization field. `decentralization` is PROVEN-ABSENT from the Conway PParams
/// array and carries its Conway constant (`d = 0`, fully decentralized). `network_id` is
/// NOT a Conway protocol parameter either; it is supplied by the caller, derived from the
/// manifest network magic (the authority), and bound onto the shared `ProtocolParameters`.
fn read_conway_pparams(d: &[u8], o: &mut usize, network_id: u8) -> Rn<ProtocolParameters> {
    if peek_major(d, *o).map_err(NativeNonUtxoError::from)? != 4 {
        return Err(NativeNonUtxoError::ProtocolParamsMissing(
            "curPParams: not an array".into(),
        ));
    }
    let n = nn_array_len(d, o, "curPParams")?;
    if n != CONWAY_PPARAMS_FIELDS {
        return Err(NativeNonUtxoError::ProtocolParamsMissing(format!(
            "curPParams arity {n} != {CONWAY_PPARAMS_FIELDS} (require Conway)"
        )));
    }
    let min_fee_a = Coin(nn_read_u64(d, o, "pp.minFeeA")?);
    let min_fee_b = Coin(nn_read_u64(d, o, "pp.minFeeB")?);
    let max_block_body_size = nn_read_u32(d, o, "pp.maxBBSize")?;
    let max_tx_size = nn_read_u32(d, o, "pp.maxTxSize")?;
    let max_block_header_size = nn_read_u32(d, o, "pp.maxBHSize")?;
    let key_deposit = Coin(nn_read_u64(d, o, "pp.keyDeposit")?);
    let pool_deposit = Coin(nn_read_u64(d, o, "pp.poolDeposit")?);
    let e_max = nn_read_u32(d, o, "pp.eMax")?;
    let n_opt = nn_read_u32(d, o, "pp.nOpt")?;
    let pool_influence = read_pp_rational(d, o, "pp.a0")?;
    let monetary_expansion = read_pp_rational(d, o, "pp.rho")?;
    let treasury_growth = read_pp_rational(d, o, "pp.tau")?;
    let (protocol_major, protocol_minor) = read_protocol_version(d, o)?;
    let min_pool_cost = Coin(nn_read_u64(d, o, "pp.minPoolCost")?);
    // coinsPerUTxOByte is Conway's PER-BYTE replacement for the Shelley `minUTxOValue`.
    // It is preserved faithfully as `MinUtxoRule::PerByte` and NEVER remapped onto the
    // absolute-floor `LegacyAbsoluteMin` (which would let the min-UTxO validator treat a
    // per-byte coefficient as an absolute floor and admit outputs under a false minimum).
    let min_utxo_rule = MinUtxoRule::PerByte(Coin(nn_read_u64(d, o, "pp.coinsPerUTxOByte")?));
    let cost_models_cbor = Some(read_raw_item(d, o, "pp.costModels")?);
    skip_item(d, o)?; // [16] prices (not on the shared params)
    let (max_tx_ex_units_mem, max_tx_ex_units_cpu) = read_ex_units(d, o)?; // [17] maxTxExUnits
    skip_item(d, o)?; // [18] maxBlockExUnits
    skip_item(d, o)?; // [19] maxValSize
    let collateral_percent = nn_read_u64(d, o, "pp.collateralPercentage")?.min(u16::MAX as u64) as u16; // [20]
    // [21..=30]: maxCollateralInputs, poolVotingThresholds, drepVotingThresholds,
    // committeeMinSize, committeeMaxTermLength, govActionLifetime, govActionDeposit,
    // dRepDeposit, dRepActivity, minFeeRefScriptCostPerByte — governance / Conway-only
    // params that have no home on the shared `ProtocolParameters`; consumed but not mapped.
    for _ in 21..CONWAY_PPARAMS_FIELDS {
        skip_item(d, o)?;
    }
    Ok(ProtocolParameters {
        min_fee_a,
        min_fee_b,
        max_block_body_size,
        max_tx_size,
        max_block_header_size,
        key_deposit,
        pool_deposit,
        e_max,
        n_opt,
        pool_influence,
        monetary_expansion,
        treasury_growth,
        protocol_major,
        protocol_minor,
        min_utxo_rule,
        min_pool_cost,
        // Conway removed `d`: fully decentralized (proven-absent → 0, not silently defaulted).
        decentralization: Rational::zero(),
        collateral_percent,
        max_tx_ex_units_mem,
        max_tx_ex_units_cpu,
        // networkId is not a Conway protocol parameter (absent from the PParams array); it is
        // bound from the manifest network magic (the authority), not decoded from the array.
        network_id,
        cost_models_cbor,
    })
}

/// Read a Conway PParams rational field (`tag(30) array(2)[num, den]`).
fn read_pp_rational(d: &[u8], o: &mut usize, what: &str) -> Rn<Rational> {
    let (t, _) = read_tag(d, o).map_err(NativeNonUtxoError::from)?;
    if t != TAG_RATIONAL_PP {
        return Err(NativeNonUtxoError::ProtocolParamsMissing(format!(
            "{what}: tag {t} != 30"
        )));
    }
    nn_expect_array(d, o, 2, what)?;
    let num = nn_read_u64(d, o, what)?;
    let den = nn_read_u64(d, o, what)?;
    Rational::new(num as i128, den as i128).ok_or_else(|| {
        NativeNonUtxoError::ProtocolParamsMissing(format!("{what}: zero denominator"))
    })
}

/// Read the Conway `protVer` = `array(2)[major, minor]`.
fn read_protocol_version(d: &[u8], o: &mut usize) -> Rn<(u32, u32)> {
    nn_expect_array(d, o, 2, "pp.protVer")?;
    let major = nn_read_u32(d, o, "pp.protVer.major")?;
    let minor = nn_read_u32(d, o, "pp.protVer.minor")?;
    Ok((major, minor))
}

/// Read an `ExUnits` = `array(2)[mem, steps]`, returning `(mem, steps)`.
fn read_ex_units(d: &[u8], o: &mut usize) -> Rn<(u64, u64)> {
    nn_expect_array(d, o, 2, "pp.exUnits")?;
    let mem = nn_read_u64(d, o, "pp.exUnits.mem")?;
    let steps = nn_read_u64(d, o, "pp.exUnits.steps")?;
    Ok((mem, steps))
}

/// Capture one CBOR item verbatim (used to preserve the `costModels` map bytes exactly,
/// as aiken's `eval_phase_two_raw` consumes them unchanged).
fn read_raw_item(d: &[u8], o: &mut usize, _what: &str) -> Rn<Vec<u8>> {
    let (start, end) = skip_item(d, o).map_err(NativeNonUtxoError::from)?;
    Ok(d[start..end].to_vec())
}

/// Canonical deterministic encoding of the COMPLETE non-UTxO state, hashed into the
/// commitment. Every emitted field is bound: the era tag, the epoch, the point
/// (slot+hash), the canonical CertState bytes, all five nonces, the pool distribution
/// (pool ++ stake ++ vrf), the protocol params (via the canonical `encode_pparams`), the
/// pots, and the block production (pool ++ blocks). BTreeMap iteration is ascending +
/// deterministic; fixed big-endian widths => an identical state serializes identically.
fn commit_native_nonutxo_state(s: &NativeSnapshotNonUtxoState) -> Hash32 {
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(b"ade-native-nonutxo-state-commitment-v2");
    v.push(s.era.as_u8());
    // The manifest-derived network id (a network identity perturbation flips the commitment).
    v.push(s.network_id);
    v.extend_from_slice(&s.epoch.0.to_be_bytes());
    v.extend_from_slice(&s.point.slot.0.to_be_bytes());
    v.extend_from_slice(&s.point.block_hash.0);
    // FULL CertState via the single canonical encoder.
    let cert_bytes = encode_cert_state(&s.cert_state);
    v.extend_from_slice(&(cert_bytes.len() as u64).to_be_bytes());
    v.extend_from_slice(&cert_bytes);
    // all five nonces (record order).
    for nonce in [
        &s.praos_nonces.evolving,
        &s.praos_nonces.candidate,
        &s.praos_nonces.epoch,
        &s.praos_nonces.lab,
        &s.praos_nonces.last_epoch_block,
    ] {
        v.extend_from_slice(&nonce.0);
    }
    // pool distribution.
    v.extend_from_slice(&(s.pool_distr.len() as u64).to_be_bytes());
    for (pid, (stake, vrf)) in &s.pool_distr {
        v.extend_from_slice(&pid.0 .0);
        v.extend_from_slice(&stake.to_be_bytes());
        v.extend_from_slice(&vrf.0);
    }
    // protocol params via the single canonical encoder.
    let pp_bytes = crate::snapshot::gov_state::encode_pparams(&s.protocol_params);
    v.extend_from_slice(&(pp_bytes.len() as u64).to_be_bytes());
    v.extend_from_slice(&pp_bytes);
    // The min-UTxO rule KIND, bound explicitly: `encode_pparams` serializes only the coin
    // payload (era-faithful byte-identity for the persistence path), so the rule kind would
    // not otherwise be bound. A `PerByte(c)` and a `LegacyAbsoluteMin(c)` with the same coin
    // must commit differently — the imported parameter's semantics are part of the field.
    let min_utxo_rule_kind: u8 = match s.protocol_params.min_utxo_rule {
        crate::pparams::MinUtxoRule::LegacyAbsoluteMin(_) => 0,
        crate::pparams::MinUtxoRule::PerByte(_) => 1,
    };
    v.push(min_utxo_rule_kind);
    // pots.
    v.extend_from_slice(&s.reserves.0.to_be_bytes());
    v.extend_from_slice(&s.treasury.0.to_be_bytes());
    // block production.
    v.extend_from_slice(&(s.block_production.len() as u64).to_be_bytes());
    for (pid, blocks) in &s.block_production {
        v.extend_from_slice(&pid.0 .0);
        v.extend_from_slice(&blocks.to_be_bytes());
    }
    // reward-account nibble observation (DIAGNOSTIC evidence — committed for determinism, NOT a
    // verdict): None=0, Uniform(n)=[1,n], Mixed=2.
    match &s.reward_nibble_observation {
        RewardNibbleObservation::None => v.push(0),
        RewardNibbleObservation::Uniform(n) => {
            v.push(1);
            v.push(*n);
        }
        RewardNibbleObservation::Mixed => v.push(2),
    }
    blake2b_256(&v)
}

// ---- S1a-local CBOR helpers (NativeNonUtxoError-typed mirrors of the Stage-1 helpers,
// so the new decoder's errors are its own closed enum) ----

fn nn_expect_array(d: &[u8], o: &mut usize, n: u64, what: &str) -> Rn<()> {
    match read_array_header(d, o).map_err(NativeNonUtxoError::from)? {
        ContainerEncoding::Definite(c, _) if c == n => Ok(()),
        ContainerEncoding::Definite(c, _) => {
            Err(nn_malformed(format!("{what}: array arity {c} != {n}")))
        }
        ContainerEncoding::Indefinite => Err(nn_malformed(format!("{what}: indefinite array"))),
    }
}

fn nn_array_len(d: &[u8], o: &mut usize, what: &str) -> Rn<u64> {
    match read_array_header(d, o).map_err(NativeNonUtxoError::from)? {
        ContainerEncoding::Definite(c, _) => Ok(c),
        ContainerEncoding::Indefinite => Err(nn_malformed(format!("{what}: indefinite array"))),
    }
}

fn nn_map_each<F>(d: &[u8], o: &mut usize, what: &str, mut f: F) -> Rn<()>
where
    F: FnMut(&mut usize) -> Rn<()>,
{
    match read_map_header(d, o).map_err(NativeNonUtxoError::from)? {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                f(o)?;
            }
        }
        ContainerEncoding::Indefinite => loop {
            if *o >= d.len() {
                return Err(nn_malformed(format!("{what}: unterminated indefinite map")));
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

fn nn_read_u64(d: &[u8], o: &mut usize, what: &str) -> Rn<u64> {
    let (v, neg, _) = read_any_int(d, o).map_err(NativeNonUtxoError::from)?;
    if neg {
        return Err(nn_malformed(format!("{what}: unexpected negative int")));
    }
    Ok(v)
}

fn nn_read_u32(d: &[u8], o: &mut usize, what: &str) -> Rn<u32> {
    let v = nn_read_u64(d, o, what)?;
    if v > u32::MAX as u64 {
        return Err(nn_malformed(format!("{what}: {v} exceeds u32")));
    }
    Ok(v as u32)
}

fn nn_read_fixed_bytes(d: &[u8], o: &mut usize, n: usize, what: &str) -> Rn<Vec<u8>> {
    let (b, _) = read_bytes(d, o).map_err(NativeNonUtxoError::from)?;
    if b.len() != n {
        return Err(nn_malformed(format!("{what}: byte len {} != {n}", b.len())));
    }
    Ok(b)
}

fn nn_hash28(b: Vec<u8>) -> Hash28 {
    let mut a = [0u8; 28];
    a.copy_from_slice(&b);
    Hash28(a)
}

/// CertState decode, re-using the Stage-1 navigation but lifting its errors into the
/// native enum (zero VRF / round-trip surface as the native variants).
fn nn_read_cert_state(d: &[u8], o: &mut usize) -> Rn<(PoolState, DelegationState)> {
    read_cert_state(d, o).map_err(|e| match e {
        LedgerDbStateError::ZeroVrf(p) => NativeNonUtxoError::ZeroVrf(p),
        LedgerDbStateError::MalformedCbor(s) => NativeNonUtxoError::MalformedCbor(s),
        other => nn_malformed(format!("cert_state: {other:?}")),
    })
}

/// PoolDistr decode, lifting Stage-1 errors into the native enum.
fn read_pool_distr_nn(d: &[u8], o: &mut usize) -> Rn<BTreeMap<PoolId, (u64, Hash32)>> {
    read_pool_distr(d, o).map_err(|e| match e {
        LedgerDbStateError::ZeroVrf(p) => NativeNonUtxoError::ZeroVrf(p),
        LedgerDbStateError::MalformedCbor(s) => NativeNonUtxoError::MalformedCbor(s),
        other => nn_malformed(format!("pool_distr: {other:?}")),
    })
}

/// Header-state slice + nonce extraction, lifting Stage-1 errors into the native enum.
fn nn_headerstate_slice(d: &[u8]) -> Rn<&[u8]> {
    headerstate_slice(d).map_err(|e| nn_malformed(format!("headerState: {e:?}")))
}

fn nn_extract_praos_nonces(hs: &[u8]) -> Rn<PraosNonces> {
    extract_praos_nonces_v2(hs).map_err(|e| nn_malformed(format!("praos nonces: {e:?}")))
}
