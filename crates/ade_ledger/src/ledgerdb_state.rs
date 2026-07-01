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
use ade_types::conway::cert::DRep;
use ade_types::conway::governance::{GovAction, GovActionId, GovActionState, Vote};
use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, PoolId};
use ade_types::{CardanoEra, EpochNo, Hash28, Hash32, SlotNo};

use crate::bootstrap_anchor::SeedPoint;
use crate::consensus_input_extract::{Nonce, PraosNonces};
use crate::delegation::{CertState, DelegationState, PoolParams, PoolState};
use crate::epoch::{GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotState, StakeSnapshot};
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

    /// CE-3d acceptance #1 + #3 (hermetic): a SnapShot's combined `ssStake` (cred -> [coin, pool])
    /// decodes to the full `StakeSnapshot` (delegations + `calculatePoolDistr` pool_stakes), and the
    /// result round-trips through the accumulator's `epoch_state` codec byte-identically (restart).
    #[test]
    fn read_stake_snapshot_full_decodes_combined_stake_and_delegation() {
        // SnapShot = array(2)[ ssStake: map(cred -> [coin, pool]), ssPoolParams: map(0) ].
        // aa,bb -> pool11 (aggregate to 3000); cc -> pool22 (500). ssPoolParams skipped.
        let mut s: Vec<u8> = vec![0x82, 0xa3]; // SnapShot a(2) ; ssStake map(3)
        let mut entry = |cred: u8, hi: u8, lo: u8, pool: u8| {
            s.extend_from_slice(&[0x82, 0x00, 0x58, 0x1c]); // key cred = a(2)[tag0, bytes(28)]
            s.extend_from_slice(&[cred; 28]);
            s.extend_from_slice(&[0x82, 0x19, hi, lo, 0x58, 0x1c]); // val a(2)[coin u16, bytes(28) pool]
            s.extend_from_slice(&[pool; 28]);
        };
        entry(0xaa, 0x03, 0xe8, 0x11); // 1000 -> pool11
        entry(0xbb, 0x07, 0xd0, 0x11); // 2000 -> pool11
        entry(0xcc, 0x01, 0xf4, 0x22); // 500  -> pool22
        s.push(0xa0); // ssPoolParams map(0) -- skipped

        let mut o = 0usize;
        let snap = read_stake_snapshot_full(&s, &mut o, "test").expect("decode");
        assert_eq!(o, s.len(), "consumes the whole SnapShot (ssStake + skipped ssPoolParams)");
        assert_eq!(snap.delegations.len(), 3);
        assert_eq!(
            snap.delegations.get(&Hash28([0xaa; 28])),
            Some(&(PoolId(Hash28([0x11; 28])), Coin(1000)))
        );
        assert_eq!(
            snap.delegations.get(&Hash28([0xcc; 28])),
            Some(&(PoolId(Hash28([0x22; 28])), Coin(500)))
        );
        assert_eq!(snap.pool_stakes.get(&PoolId(Hash28([0x11; 28]))), Some(&Coin(3000)));
        assert_eq!(snap.pool_stakes.get(&PoolId(Hash28([0x22; 28]))), Some(&Coin(500)));

        // Acceptance #3: the snapshots survive the accumulator's encode/decode byte-identically.
        use crate::epoch::{GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotState};
        use crate::snapshot::epoch_state::{decode_epoch_state, encode_epoch_state};
        use crate::state::EpochState;
        let mut es = EpochState::new();
        es.snapshots = SnapshotState {
            mark: MarkSnapshot(snap.clone()),
            set: SetSnapshot(snap.clone()),
            go: GoSnapshot(snap.clone()),
        };
        let rt = decode_epoch_state(&encode_epoch_state(&es)).expect("round-trip");
        assert_eq!(rt.snapshots.go.0, snap, "go survives encode/decode byte-identically");
        assert_eq!(rt.snapshots.mark.0, snap);
        assert_eq!(rt.snapshots.set.0, snap);
    }

    /// CE-3d fail-closed: a structurally short SnapShot is TERMINAL (never empty-substituted).
    #[test]
    fn read_stake_snapshot_full_rejects_short_snapshot() {
        let s: Vec<u8> = vec![0x81, 0xa0]; // SnapShot a(1)[ map(0) ] -- missing ssPoolParams
        let mut o = 0usize;
        assert!(read_stake_snapshot_full(&s, &mut o, "test").is_err());
    }

    // ===== CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1 — gov-state decoder fail-closed proofs =====

    /// An unknown `Vote` value is TERMINAL (never silently defaulted).
    #[test]
    fn nn_read_vote_rejects_unknown_value() {
        let s = vec![0x07u8]; // uint 7 — not 0/1/2
        let mut o = 0usize;
        assert!(matches!(
            nn_read_vote(&s, &mut o),
            Err(NativeNonUtxoError::UnsupportedGovernanceState(_))
        ));
    }

    /// `SJust` committee decodes member→expiry + the UnitInterval quorum; `null` ⇒ no committee.
    #[test]
    fn nn_read_committee_decodes_sjust_and_snothing() {
        // SJust: array(1)[ array(2)[ map(1){ KeyHash(0xC0) => 1007 }, tag30[2,3] ] ]
        let mut s = vec![0x81, 0x82, 0xa1, 0x82, 0x00, 0x58, 0x1c];
        s.extend_from_slice(&[0xC0; 28]);
        s.extend_from_slice(&[0x19, 0x03, 0xef]); // 1007
        s.extend_from_slice(&[0xd8, 0x1e, 0x82, 0x02, 0x03]); // tag(30) [2,3]
        let mut o = 0usize;
        let (members, quorum) = nn_read_committee(&s, &mut o).expect("committee SJust");
        assert_eq!(o, s.len(), "consumes the whole committee");
        assert_eq!(members.get(&StakeCredential::KeyHash(Hash28([0xC0; 28]))), Some(&1007u64));
        assert_eq!(quorum, Some((2, 3)));

        let n = vec![0xf6u8]; // null -> SNothing
        let mut o2 = 0usize;
        let (m2, q2) = nn_read_committee(&n, &mut o2).expect("committee SNothing");
        assert!(m2.is_empty() && q2.is_none(), "absent committee is empty + no quorum");
    }

    /// An unknown `GovAction` variant inside a proposal is TERMINAL — never silently skipped.
    #[test]
    fn nn_read_gov_action_state_rejects_unknown_action() {
        // array(7)[ gasId, a0, a0, a0, procedure[deposit 0, return[], gov_action(tag 99), anchor null] ]
        let mut s = vec![0x87, 0x82, 0x58, 0x20];
        s.extend_from_slice(&[0xAB; 32]); // gasId tx_hash
        s.push(0x00); // gasId index
        s.extend_from_slice(&[0xa0, 0xa0, 0xa0]); // empty cc/drep/spo votes
        // procedure = array(4)[deposit 0, return_addr bytes(0), gov_action array(1)[uint 99], anchor null]
        s.extend_from_slice(&[0x84, 0x00, 0x40, 0x81, 0x18, 0x63, 0xf6]);
        let mut o = 0usize;
        assert!(
            nn_read_gov_action_state(&s, &mut o).is_err(),
            "unknown gov-action variant must halt, not be coerced to a default",
        );
    }

    /// A truncated `Proposals` OMap element is TERMINAL (never a partial / empty proposal set).
    #[test]
    fn nn_read_proposals_rejects_truncated() {
        // array(2)[ GovRelation=array(4)[SNothing;4], OMap=indef-array[ array(7) header then EOF ] ]
        let s = vec![0x82, 0x84, 0x80, 0x80, 0x80, 0x80, 0x9f, 0x87];
        let mut o = 0usize;
        assert!(nn_read_proposals(&s, &mut o).is_err(), "truncated proposal must halt");
    }

    /// The enacted PParamUpdate root (`GovRelation` element 0 = `prevGovActionIds.pgaPParamUpdate`) is decoded
    /// from the head of `Proposals`: `SJust id` → `Some(id)`, exactly the id in element 0 (not 1..3).
    #[test]
    fn nn_read_proposals_decodes_enacted_pparam_update_root() {
        let mut s = vec![0x82]; // Proposals = array(2)
        s.push(0x84); // GovRelation = array(4)
        // [0] PParamUpdate = SJust(GovActionId(tx 0x11..; index 0)) = array(1)[ array(2)[bytes32, 0] ]
        s.extend_from_slice(&[0x81, 0x82, 0x58, 0x20]);
        s.extend_from_slice(&[0x11; 32]);
        s.push(0x00);
        // [1] HardFork = SJust(other id) — must NOT be mistaken for the pparam-update root
        s.extend_from_slice(&[0x81, 0x82, 0x58, 0x20]);
        s.extend_from_slice(&[0x22; 32]);
        s.push(0x00);
        s.extend_from_slice(&[0x80, 0x80]); // [2] Committee = SNothing, [3] Constitution = SNothing
        s.extend_from_slice(&[0x80]); // OMap = array(0) — no live proposals
        let mut o = 0usize;
        let (props, enacted) = nn_read_proposals(&s, &mut o).expect("decode");
        assert!(props.is_empty());
        let id = enacted.expect("SJust PParamUpdate root");
        assert_eq!(id.tx_hash, Hash32([0x11; 32]), "the PParamUpdate root, not the HardFork root");
        assert_eq!(id.index, 0);
        assert_eq!(o, s.len(), "the whole Proposals is consumed");
    }

    /// `SNothing` PParamUpdate root (genesis: no param-update ever enacted) → `None`; a StrictMaybe arity ≠
    /// {0,1} is TERMINAL, never coerced.
    #[test]
    fn nn_read_strict_maybe_gov_action_id_none_and_terminal() {
        let mut o = 0usize;
        assert_eq!(nn_read_strict_maybe_gov_action_id(&[0x80], &mut o, "t").unwrap(), None);
        let mut o = 0usize;
        // array(2) is neither SNothing nor SJust → terminal.
        assert!(nn_read_strict_maybe_gov_action_id(&[0x82, 0x00, 0x00], &mut o, "t").is_err());
    }

    /// The v6 commitment is deterministic AND binds the imported gov state: an empty proposal set
    /// commits differently from a populated one (absent ≠ empty), and mutating any bound field flips it.
    #[test]
    fn v6_commitment_is_deterministic_and_binds_gov() {
        use crate::consensus_input_extract::{Nonce, PraosNonces};
        use crate::delegation::CertState;
        use crate::epoch::SnapshotState;
        use ade_types::conway::governance::{GovAction, GovActionId, GovActionState};
        let mk = |gov: ImportedGovState| NativeSnapshotNonUtxoState {
            era: CardanoEra::Conway,
            network_id: 0,
            epoch: EpochNo(1340),
            point: SeedPoint { slot: SlotNo(1), block_hash: Hash32([0; 32]) },
            cert_state: CertState::new(),
            praos_nonces: PraosNonces {
                evolving: Nonce([0; 32]),
                candidate: Nonce([0; 32]),
                epoch: Nonce([0; 32]),
                lab: Nonce([0; 32]),
                last_epoch_block: Nonce([0; 32]),
            },
            pool_distr: BTreeMap::new(),
            mark_pool_distr: BTreeMap::new(),
            snapshots: SnapshotState::default(),
            protocol_params: ProtocolParameters::default(),
            reserves: Coin(0),
            treasury: Coin(0),
            block_production: BTreeMap::new(),
            current_block_production: BTreeMap::new(),
            reward_deltas: BTreeMap::new(),
            rupd_delta_treasury: Coin(0),
            rupd_delta_reserves: Coin(0),
            epoch_fees: Coin(0),
            imported_gov: gov,
            reward_nibble_observation: RewardNibbleObservation::None,
            max_block_ex_units_mem: 0,
            gov_deposit_pot: Coin(0),
            prev_max_tx_ex_units_mem: 0,
            prev_max_block_ex_units_mem: 0,
            enacted_pparam_update: None,
        };
        let empty = ImportedGovState {
            proposals: Vec::new(),
            committee: BTreeMap::new(),
            committee_quorum: None,
            gov_action_lifetime: 6,
            pool_voting_thresholds: Vec::new(),
            drep_voting_thresholds: Vec::new(),
            vote_delegations: BTreeMap::new(),
            drep_expiry: BTreeMap::new(),
            committee_hot_keys: BTreeMap::new(),
            num_dormant_epochs: 0,
        };
        let one = ImportedGovState {
            proposals: vec![GovActionState {
                action_id: GovActionId { tx_hash: Hash32([0xAB; 32]), index: 0 },
                committee_votes: Vec::new(),
                drep_votes: Vec::new(),
                spo_votes: Vec::new(),
                deposit: Coin(100_000_000_000),
                return_addr: vec![0xE0; 29],
                gov_action: GovAction::TreasuryWithdrawals {
                    withdrawals: vec![(vec![0xE0; 29], Coin(1))],
                    policy_hash: None,
                },
                proposed_in: EpochNo(1309),
                expires_after: EpochNo(1339),
            }],
            committee: BTreeMap::new(),
            committee_quorum: Some((2, 3)),
            gov_action_lifetime: 6,
            pool_voting_thresholds: vec![(51, 100)],
            drep_voting_thresholds: vec![(51, 100)],
            vote_delegations: BTreeMap::from([(
                StakeCredential::KeyHash(Hash28([0x11; 28])),
                DRep::KeyHash(Hash28([0x22; 28])),
            )]),
            drep_expiry: BTreeMap::from([(StakeCredential::KeyHash(Hash28([0x44; 28])), 1350)]),
            committee_hot_keys: BTreeMap::from([(
                StakeCredential::KeyHash(Hash28([0x55; 28])),
                StakeCredential::ScriptHash(Hash28([0x66; 28])),
            )]),
            num_dormant_epochs: 3,
        };
        // deterministic
        assert_eq!(
            commit_native_nonutxo_state(&mk(empty.clone())),
            commit_native_nonutxo_state(&mk(empty.clone())),
        );
        // absent (empty) ≠ populated: the proposal set is bound
        assert_ne!(
            commit_native_nonutxo_state(&mk(empty)),
            commit_native_nonutxo_state(&mk(one.clone())),
            "v6 binds the proposal set (empty ≠ populated)",
        );
        // a single bound field flip changes the commitment
        let mut mutated = one.clone();
        mutated.proposals[0].deposit = Coin(999);
        assert_ne!(
            commit_native_nonutxo_state(&mk(one.clone())),
            commit_native_nonutxo_state(&mk(mutated)),
            "v6 binds the proposal deposit",
        );
        // the imported govActionLifetime (live-proposal expiry authority) is bound: a tampered lifetime
        // flips the commitment, so it cannot be silently substituted (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3).
        let mut diff_lifetime = one.clone();
        diff_lifetime.gov_action_lifetime = 7;
        assert_ne!(
            commit_native_nonutxo_state(&mk(one.clone())),
            commit_native_nonutxo_state(&mk(diff_lifetime)),
            "v9 binds the imported gov_action_lifetime",
        );
        // the imported voting thresholds (curPParams 22/23) are bound: a tampered SPO/DRep threshold flips
        // the commitment (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S1).
        let mut diff_thresholds = one.clone();
        diff_thresholds.drep_voting_thresholds = vec![(67, 100)];
        assert_ne!(
            commit_native_nonutxo_state(&mk(one.clone())),
            commit_native_nonutxo_state(&mk(diff_thresholds)),
            "v9 binds the imported drep_voting_thresholds",
        );
        // the imported bootstrap vote delegations (DState UMap drep field) are bound: a tampered delegation
        // flips the commitment (CRE S1 part 2a).
        let mut diff_vd = one.clone();
        diff_vd
            .vote_delegations
            .insert(StakeCredential::KeyHash(Hash28([0x33; 28])), DRep::AlwaysNoConfidence);
        assert_ne!(
            commit_native_nonutxo_state(&mk(one.clone())),
            commit_native_nonutxo_state(&mk(diff_vd)),
            "v10 binds the imported vote_delegations",
        );
        // the imported VState maps (drep_expiry + committee hot->cold) are bound (CRE S1 part 2b).
        let mut diff_expiry = one.clone();
        diff_expiry.drep_expiry.insert(StakeCredential::KeyHash(Hash28([0x77; 28])), 9999);
        assert_ne!(
            commit_native_nonutxo_state(&mk(one.clone())),
            commit_native_nonutxo_state(&mk(diff_expiry)),
            "v10 binds the imported drep_expiry",
        );
        let mut diff_hot = one.clone();
        diff_hot.committee_hot_keys.insert(
            StakeCredential::KeyHash(Hash28([0x88; 28])),
            StakeCredential::KeyHash(Hash28([0x99; 28])),
        );
        assert_ne!(
            commit_native_nonutxo_state(&mk(one)),
            commit_native_nonutxo_state(&mk(diff_hot)),
            "v10 binds the imported committee_hot_keys",
        );
    }

    #[test]
    fn read_vstate_is_fail_closed_on_malformed_governance() {
        // cred = array(2)[0(keyhash), bytes28]
        let cred = |b: u8| {
            let mut v = vec![0x82, 0x00, 0x58, 0x1c];
            v.extend_from_slice(&[b; 28]);
            v
        };
        let run = |bytes: &[u8]| {
            let mut o = 0usize;
            read_vstate(bytes, &mut o)
        };
        // 1. empty DRepState array(0) -> TERMINAL
        let mut b = vec![0x83, 0xa1];
        b.extend(cred(0xD1));
        b.extend([0x80, 0xa0, 0x00]);
        assert!(run(&b).is_err(), "empty DRepState is terminal");
        // 2. empty committee-auth array(0) -> TERMINAL
        let mut b = vec![0x83, 0xa0, 0xa1];
        b.extend(cred(0xC0));
        b.extend([0x80, 0x00]);
        assert!(run(&b).is_err(), "empty committee auth is terminal");
        // 3. out-of-range committee variant (2) -> TERMINAL
        let mut b = vec![0x83, 0xa0, 0xa1];
        b.extend(cred(0xC0));
        b.extend([0x82, 0x02, 0xf6, 0x00]);
        assert!(run(&b).is_err(), "committee variant 2 is terminal");
        // 4. arity-1 MemberAuthorized (variant 0, no hot cred) -> TERMINAL (the n<2 guard)
        let mut b = vec![0x83, 0xa0, 0xa1];
        b.extend(cred(0xC0));
        b.extend([0x81, 0x00, 0x00]);
        assert!(run(&b).is_err(), "arity-1 MemberAuthorized is terminal");
        // sanity: a well-formed minimal VState decodes and consumes EXACTLY its bytes.
        let mut b = vec![0x83, 0xa1];
        b.extend(cred(0xD1));
        b.extend([0x84, 0x01, 0x80, 0x00, 0x80]); // DRepState[expiry=1, anchor(arr0), deposit=0, delegs(arr0)]
        b.push(0xa1);
        b.extend(cred(0xC0));
        b.extend([0x82, 0x00]); // auth = [0, hot]
        b.extend(cred(0xC1));
        b.push(0x05); // numDormant = 5
        let mut o = 0usize;
        let (de, chk, nd) = read_vstate(&b, &mut o).expect("well-formed VState decodes");
        assert_eq!((de.len(), chk.len(), nd), (1, 1, 5));
        assert_eq!(o, b.len(), "read_vstate consumes exactly the VState bytes");
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
    let (pool, delegation, _vote_delegations) = read_cert_state(d, o)?;
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

/// Decode the CertState (LedgerState[0]) = array(3)[VState, PState, DState] -> (PoolState,
/// DelegationState, BootstrapGovImport). The VState yields the DRep-expiry + committee-hot-key import (CRE
/// S1 part 2b); the DState UMap yields the DRep vote-delegation baseline (part 2a).
fn read_cert_state(d: &[u8], o: &mut usize) -> R<(PoolState, DelegationState, BootstrapGovImport)> {
    expect_array(d, o, 3, "CertState")?;
    let (drep_expiry, committee_hot_keys, num_dormant_epochs) = read_vstate(d, o)?; // VState
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
    let (delegation, vote_delegations) = read_dstate(d, o)?;
    let gov_import = BootstrapGovImport {
        vote_delegations,
        drep_expiry,
        committee_hot_keys,
        num_dormant_epochs,
    };
    Ok((pool, delegation, gov_import))
}

/// The bootstrap governance inputs threaded out of the CertState decode into the gov import (CRE S1
/// part 2): the DRep vote-delegation baseline (DState UMap) plus the DRep expiry and committee hot-key
/// maps (VState). All three are IMPORTED (commitment-bound) but kept OUT of the live gate until S4.
struct BootstrapGovImport {
    vote_delegations: BTreeMap<StakeCredential, DRep>,
    drep_expiry: BTreeMap<StakeCredential, u64>,
    committee_hot_keys: BTreeMap<StakeCredential, StakeCredential>,
    num_dormant_epochs: u64,
}

/// Decode the VState (CertState[0]) = `array(3)[vsDReps, vsCommitteeState, vsNumDormantEpochs]`:
/// - `vsDReps` = `map { DRepCred => DRepState array(4)[expiry, anchor, deposit, delegs] }` → `drep_expiry`
///   (field [0]).
/// - `vsCommitteeState` = `map { ColdCred => array(2)[variant, payload] }` where variant 0 =
///   MemberAuthorized(hotCred) and variant 1 = MemberResigned(anchor?) → `committee_hot_keys[hot] = cold`
///   (inverted; the ratification committee gate looks up by the voter's HOT credential).
/// FAIL-CLOSED: an out-of-range committee variant / empty DRepState is TERMINAL.
fn read_vstate(
    d: &[u8],
    o: &mut usize,
) -> R<(BTreeMap<StakeCredential, u64>, BTreeMap<StakeCredential, StakeCredential>, u64)> {
    expect_array(d, o, 3, "VState")?;
    let mut drep_expiry: BTreeMap<StakeCredential, u64> = BTreeMap::new();
    map_each(d, o, "vsDReps", |o| {
        let cred = read_credential(d, o)?;
        let n = array_len(d, o, "DRepState")?;
        if n == 0 {
            return Err(malformed("DRepState: empty array".to_string()));
        }
        let expiry = read_u64(d, o, "DRepState.expiry")?;
        for _ in 1..n {
            skip_item(d, o)?;
        }
        drep_expiry.insert(cred, expiry);
        Ok(())
    })?;
    let mut committee_hot_keys: BTreeMap<StakeCredential, StakeCredential> = BTreeMap::new();
    map_each(d, o, "vsCommitteeState", |o| {
        let cold = read_credential(d, o)?;
        let n = array_len(d, o, "CommitteeAuthorization")?;
        if n == 0 {
            return Err(malformed("CommitteeAuthorization: empty array".to_string()));
        }
        match read_u64(d, o, "CommitteeAuthorization.variant")? {
            0 => {
                // MemberAuthorized(hotCred): the hot credential is element [1]. Reject an arity-1 auth BY
                // CONSTRUCTION (fail-closed, symmetric to read_native_drep's guard) so the unconditional
                // read_credential below cannot over-consume the following item's bytes on a malformed
                // `[0]` (unreachable on honest cardano — always encodeListLen 2 — but terminal by design;
                // the committee map is definite, so a silent misalignment would cascade through the decode).
                if n < 2 {
                    return Err(malformed(format!(
                        "CommitteeAuthorization: MemberAuthorized with arity {n} < 2"
                    )));
                }
                // index the hot credential -> cold (the gate resolves hot->cold).
                let hot = read_credential(d, o)?;
                committee_hot_keys.insert(hot, cold);
                for _ in 2..n {
                    skip_item(d, o)?;
                }
            }
            1 => {
                // MemberResigned(anchor?): no hot key; consume the anchor payload.
                for _ in 1..n {
                    skip_item(d, o)?;
                }
            }
            v => {
                return Err(malformed(format!(
                    "CommitteeAuthorization: variant {v} out of range"
                )))
            }
        }
        Ok(())
    })?;
    // vsNumDormantEpochs — captured (NOT discarded): cardano's active-DRep test is
    // `drepExpiry + numDormantEpochs >= currentEpoch`, so the S4 activation needs this offset to reproduce
    // the ratification denominator; discarding it here would force a VState re-decode at S4.
    let num_dormant_epochs = read_u64(d, o, "vsNumDormantEpochs")?;
    Ok((drep_expiry, committee_hot_keys, num_dormant_epochs))
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
fn read_dstate(d: &[u8], o: &mut usize) -> R<(DelegationState, BTreeMap<StakeCredential, DRep>)> {
    expect_array(d, o, 4, "DState")?;
    let mut ds = DelegationState::new();
    let mut vote_delegations: BTreeMap<StakeCredential, DRep> = BTreeMap::new();
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
        // drep?: null (0xF6 = no vote delegation) or a DRep. Captured as the CRE S1 bootstrap-baseline
        // vote_delegations import (read-only here; kept OUT of the live gate until the S4 activation). Any
        // fields past the drep are skipped, so the UMap cursor stays aligned exactly as the old skip did.
        if vn >= 4 {
            if ade_codec::cbor::peek_major(d, *o)? == 7 {
                skip_item(d, o)?; // null / simple -> no vote delegation
            } else {
                vote_delegations.insert(cred.clone(), read_native_drep(d, o)?);
            }
            for _ in 4..vn {
                skip_item(d, o)?;
            }
        }
        ds.registrations.insert(cred.clone(), deposit);
        ds.rewards.insert(cred, reward);
        Ok(())
    })?;
    // futureGenDelegs, genDelegs, iRewards.
    skip_item(d, o)?;
    skip_item(d, o)?;
    skip_item(d, o)?;
    Ok((ds, vote_delegations))
}

/// Decode a cardano-ledger `DRep` from the DState UMap drep field. Robust to the arity variance:
/// `[0, keyhash28]` / `[1, scripthash28]` carry a hash; `[2]` / `[3]` are the predefined DReps
/// (AlwaysAbstain / AlwaysNoConfidence) and may be encoded arity-1 or arity-2-with-null — any trailing
/// element is consumed so the UMap cursor stays byte-aligned regardless of which encoding the snapshot uses.
fn read_native_drep(d: &[u8], o: &mut usize) -> R<DRep> {
    let n = array_len(d, o, "umap.drep")?;
    if n == 0 {
        return Err(malformed("umap.drep: empty array".to_string()));
    }
    let variant = read_u64(d, o, "umap.drep.variant")?;
    // A hash DRep (variant 0/1) MUST carry its 28-byte hash as the array's 2nd element. Reject an arity-1
    // hash variant BY CONSTRUCTION (fail-closed, symmetric to the n == 0 guard) so the unconditional hash
    // read below can never over-consume the following item's bytes on a malformed `[0]`/`[1]` (unreachable
    // on real cardano — DRepKeyHash/DRepScriptHash are fixed at encodeListLen 2 — but terminal by design).
    if variant <= 1 && n < 2 {
        return Err(malformed(format!(
            "umap.drep: hash variant {variant} with arity {n} < 2"
        )));
    }
    let drep = match variant {
        0 => DRep::KeyHash(hash28(read_fixed_bytes(d, o, 28, "umap.drep.keyhash")?)),
        1 => DRep::ScriptHash(hash28(read_fixed_bytes(d, o, 28, "umap.drep.scripthash")?)),
        2 => DRep::AlwaysAbstain,
        3 => DRep::AlwaysNoConfidence,
        v => return Err(malformed(format!("umap.drep: variant {v} out of range"))),
    };
    // Consume any trailing payload element (a normalized-encoding null for the predefined DReps).
    let consumed: u64 = if variant <= 1 { 2 } else { 1 };
    for _ in consumed..n {
        skip_item(d, o)?;
    }
    Ok(drep)
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
/// curPParams index of `govActionLifetime` (epochs a proposal lives before expiry) — imported for the
/// CONWAY-PROPOSAL-DEPOSIT-EXPIRY live-proposal expiry authority (S3).
const CONWAY_PP_GOV_ACTION_LIFETIME_INDEX: u64 = 26;
/// curPParams index of `poolVotingThresholds` (per-action SPO thresholds) — CONWAY-RATIFICATION-AND-
/// ENACTMENT-AUTHORITY S1.
const CONWAY_PP_POOL_VOTING_THRESHOLDS_INDEX: u64 = 22;
/// curPParams index of `drepVotingThresholds` (per-action DRep thresholds) — CRE S1.
const CONWAY_PP_DREP_VOTING_THRESHOLDS_INDEX: u64 = 23;
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
    /// A governance proposal / committee carries a representation Ade does not model (e.g. an unknown
    /// `GovAction` variant or committee shape). TERMINAL for the deposit-expiry authority path — NEVER
    /// coerced to an empty/default set (an absent set is not an empty set). (CONWAY-PROPOSAL-DEPOSIT-EXPIRY.)
    UnsupportedGovernanceState(String),
    /// A governance proposal / committee is structurally malformed where the gov-state must carry it.
    MalformedGovernanceState(String),
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
    /// LIVE-LEDGER-EPOCH-TRANSITION (CE-3d): the FULL mark/set/go stake snapshots decoded from
    /// `esSnapshots` — the reward + leadership stake authority the `EpochAccumulator` must seed (NOT
    /// the cold-start empty default). Without these, the accumulator's `go` is empty for the first ~3
    /// boundaries after bootstrap → zero member rewards. Decoded from the certified snapshot ALONE.
    pub snapshots: SnapshotState,
    /// The Conway current protocol parameters (`utxosGovState.curPParams`).
    pub protocol_params: ProtocolParameters,
    /// `esAccountState.reserves`.
    pub reserves: Coin,
    /// `esAccountState.treasury`.
    pub treasury: Coin,
    /// `nesBprev`: blocks each pool made in the previous epoch.
    pub block_production: BTreeMap<PoolId, u64>,
    /// `nesBcur`: blocks each pool made in the CURRENT (seed) epoch so far, as of the snapshot point.
    /// Seeds the accumulator's `epoch_state.block_production` so the seed→seed+1 boundary counts the
    /// whole seed epoch (the bootstrap follow replays only from anchor+1, never the early-epoch blocks).
    pub current_block_production: BTreeMap<PoolId, u64>,
    /// `nesRu` Complete reward update's `rs`, aggregated per credential = the reward deltas the real
    /// chain applies at the NEXT epoch boundary (the seed-window-end RUPD that authority(N+2) needs).
    /// Empty when SNothing or mid-pulse.
    pub reward_deltas: BTreeMap<StakeCredential, Coin>,
    /// `nesRu` Complete reward update's `deltaT` — the treasury INCREASE the seed-boundary RUPD applies.
    /// Pairs with `reward_deltas` (the rs) to form the COMPLETE cardano reward at the seed boundary; the
    /// `EpochAccumulator` adds this to treasury so the pots are byte-exact (it cannot re-derive the
    /// pre-seed fees natively). Zero when the nesRu is SNothing.
    pub rupd_delta_treasury: Coin,
    /// `nesRu` Complete reward update's `deltaR` MAGNITUDE — the reserves DECREASE the seed-boundary
    /// RUPD applies (subtracted from reserves). Zero when the nesRu is SNothing.
    pub rupd_delta_reserves: Coin,
    /// The certified epoch fee pot (UTxOState index 2) at the snapshot point — epoch (seed)'s fees
    /// accumulated so far. Seeds `epoch_state.epoch_fees` so the first bootstrap-adjacent native boundary
    /// (seed+1 -> seed+2, paying epoch (seed)'s reward) consumes the FULL epoch fees (this pre-seed
    /// snapshot pot + the followed tail), not just the followed tail. Manifest-bound certified state.
    pub epoch_fees: Coin,
    /// The bootstrap-imported Conway governance state (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1) — the live
    /// `Proposals` set + the constitutional committee — bound into the v6 commitment. An ABSENT gov
    /// state is never an empty one: a pre-v6 store fails closed on load (re-bootstrap required).
    pub imported_gov: ImportedGovState,
    /// DIAGNOSTIC evidence only (never a verdict): the pool reward-account network-nibble
    /// distribution. `network_id` above (manifest-derived) is the sole network authority.
    pub reward_nibble_observation: RewardNibbleObservation,
    /// `curPParams.maxBlockExUnits.mem` (index 18) — decoded for the CRE enactment-census differential (the
    /// param-update ground truth changes BOTH maxTx and maxBlock exec-mem). Not otherwise carried on the
    /// shared `ProtocolParameters` (which holds only the tx exec-units).
    pub max_block_ex_units_mem: u64,
    /// `UTxOState.deposited` (index 1) — the total deposit pot (key/pool/DRep/gov deposits). Decoded for the
    /// CRE enactment census (an enacted or expired gov action refunds its deposit, moving this pot).
    pub gov_deposit_pot: Coin,
    /// `prevPParams.maxTxExUnits.mem` (ConwayGovState field 4, index 20) — the protocol params as of the
    /// PREVIOUS epoch. Decoded for the CRE enactment census: at the enactment boundary `prev` still holds the
    /// old value while `cur` holds the new, proving the param flip lands exactly at THIS boundary.
    pub prev_max_tx_ex_units_mem: u64,
    /// `prevPParams.maxBlockExUnits.mem` (ConwayGovState field 4, index 18) — see `prev_max_tx_ex_units_mem`.
    pub prev_max_block_ex_units_mem: u64,
    /// The ledger's own enacted-authority pointer for the PParamUpdate purpose — the root of the `GovRelation`
    /// at the head of `Proposals` (gov-state field 0), i.e. `prevGovActionIds.pgaPParamUpdate`. `None` at
    /// genesis (no PParamUpdate ever enacted). Decoded for the CRE census: when an action is enacted this
    /// pointer becomes that action's id, so the fixture PROVES the enacted params were caused by the target
    /// action (not merely coincident with its observables). Never a live-gate input.
    pub enacted_pparam_update: Option<GovActionId>,
}

/// Decode a cardano `SnapShot = array(2)[ssStake: map(StakeCredential -> [Coin, PoolId]),
/// ssPoolParams: map(PoolId -> PoolParams)]` (stake + delegation COMBINED in ssStake, this ledger
/// version) into Ade's `StakeSnapshot`: `delegations` = credential-hash -> (pool, coin); `pool_stakes`
/// = the `calculatePoolDistr` per-pool sum. `ssPoolParams` is SKIPPED — pool params live in the durable
/// cert state, and this version's ssPoolParams is not vrf-first; the per-pool VRF comes from the
/// cert-state registrations (the mark<->nesPd VRF cross-check, see [`derive_mark_pool_distr`]). Faithful
/// to the certified snapshot ALONE — never the UTxO checkpoint, a live capture, nesPd, window-replay,
/// or an oracle. A malformed / short SnapShot is TERMINAL (no empty-substitution).
fn read_stake_snapshot_full(d: &[u8], o: &mut usize, label: &'static str) -> R<StakeSnapshot> {
    let sn = array_len(d, o, label)?;
    // SnapShot = array(2)[ssStake, ssPoolParams] (stake + delegation COMBINED).
    if sn < 2 {
        return Err(malformed(format!("{label}.SnapShot arity {sn} < 2")));
    }
    // map0 = ssStake: map(StakeCredential -> [Coin, PoolId]) -- stake + delegation in one entry.
    let mut delegations: BTreeMap<Hash28, (PoolId, Coin)> = BTreeMap::new();
    let mut pool_stakes: BTreeMap<PoolId, u64> = BTreeMap::new();
    map_each(d, o, label, |o| {
        let cred = read_credential(d, o)?; // key: array(2)[tag, hash28]
        expect_array(d, o, 2, "snap.stake.val")?; // value: [coin, pool]
        let coin = read_u64(d, o, "snap.stake.coin")?;
        let pool = PoolId(hash28(read_fixed_bytes(d, o, 28, "snap.stake.pool")?));
        // StakeSnapshot is Hash28-keyed (discriminant-erased), matching how the boundary fold reads it.
        delegations.insert(cred.hash().clone(), (pool.clone(), Coin(coin)));
        let e = pool_stakes.entry(pool).or_insert(0u64);
        *e = e.saturating_add(coin);
        Ok(())
    })?;
    // map1 = ssPoolParams (+ any trailing): skipped (see the fn doc).
    for _ in 1..sn {
        skip_item(d, o)?;
    }
    Ok(StakeSnapshot {
        delegations,
        pool_stakes: pool_stakes.into_iter().map(|(p, c)| (p, Coin(c))).collect(),
    })
}

/// `nn`-error wrapper for [`read_stake_snapshot_full`].
fn read_stake_snapshot_full_nn(d: &[u8], o: &mut usize, label: &'static str) -> Rn<StakeSnapshot> {
    read_stake_snapshot_full(d, o, label).map_err(|e| match e {
        LedgerDbStateError::ZeroVrf(p) => NativeNonUtxoError::ZeroVrf(p),
        LedgerDbStateError::MalformedCbor(s) => NativeNonUtxoError::MalformedCbor(s),
        other => nn_malformed(format!("{label} snapshot: {other:?}")),
    })
}

/// ECA-5 (DC-EPOCH-15): `calculatePoolDistr(ssStakeMark)` — pool -> (active stake, VRF) — derived from
/// the FULL decoded MARK snapshot's `pool_stakes` + the durable cert-state VRF registrations
/// (`cert_pool_vrf`). A staked pool with no cert-state registration (retired between the mark snapshot
/// and the seed point) cannot lead in seed+1 -> omitted (never fabricate a VRF). Byte-identical to the
/// pre-CE-3d `read_mark_snapshot_pool_distr` (same per-pool sum, same zero-stake + missing-VRF drops).
fn derive_mark_pool_distr(
    mark: &StakeSnapshot,
    cert_pool_vrf: &BTreeMap<PoolId, Hash32>,
) -> BTreeMap<PoolId, (u64, Hash32)> {
    let mut out: BTreeMap<PoolId, (u64, Hash32)> = BTreeMap::new();
    for (pool, st) in &mark.pool_stakes {
        if st.0 == 0 {
            continue;
        }
        if let Some(vrf) = cert_pool_vrf.get(pool) {
            out.insert(pool.clone(), (st.0, vrf.clone()));
        }
    }
    out
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
/// Sum the amounts of one credential's `Set Reward` value in the `rs` map. The set is optionally
/// CBOR-tag-258 wrapped, then `array(M)`; each `Reward = array(3)[type(word), pool(hash28), amount]`.
/// Conway aggregates ALL of a credential's rewards (member + leader), so we sum every amount.
fn read_reward_set_amount_sum(d: &[u8], o: &mut usize) -> R<u64> {
    if peek_major(d, *o).map_err(|e| malformed(format!("rewardSet.peek: {e:?}")))? == 6 {
        let _ = read_tag(d, o).map_err(|e| malformed(format!("rewardSet.tag: {e:?}")))?; // set tag 258
    }
    let m = array_len(d, o, "rewardSet")?;
    let mut sum = 0u64;
    for _ in 0..m {
        expect_array(d, o, 3, "Reward")?;
        let _rtype = read_u64(d, o, "Reward.type")?;
        let _pool = read_fixed_bytes(d, o, 28, "Reward.pool")?;
        let amount = read_u64(d, o, "Reward.amount")?;
        sum = sum.saturating_add(amount);
    }
    Ok(sum)
}

/// Decode `nesRu` (`StrictMaybe PulsingRewUpdate`) -> the per-credential reward deltas the real chain
/// applies at the NEXT epoch boundary. Only a Complete (already-pulsed) `RewardUpdate` yields deltas;
/// `SNothing` or a mid-pulse `Pulsing` update yields an empty map (the boundary then carries the
/// snapshot rewards unchanged). `rs = map(Credential -> Set Reward)`; credentials route through the
/// SAME `read_credential` as the dstate so the keys JOIN the existing reward map. The full item is
/// consumed regardless (so the caller's decode stays aligned).
/// Decode the snapshot's Complete `nesRu` into (rs per-credential map, deltaT, delta_reserves).
/// `deltaT` is the treasury increase; `delta_reserves` is the reserves-decrease MAGNITUDE. Both are
/// the cardano-computed pots for the seed→seed+1 boundary (they fold in epoch (seed-1)'s fees, which
/// the native accumulator cannot reconstruct from post-bootstrap blocks). The seed-boundary apply
/// adds deltaT to treasury and subtracts delta_reserves from reserves so the pots are byte-exact.
fn read_reward_update_deltas(d: &[u8], o: &mut usize) -> R<(BTreeMap<StakeCredential, Coin>, Coin, Coin)> {
    let mut deltas: BTreeMap<StakeCredential, Coin> = BTreeMap::new();
    // StrictMaybe: array(0) = SNothing, array(1) = SJust PulsingRewUpdate.
    let sm = array_len(d, o, "nesRu.strictMaybe")?;
    if sm == 0 {
        return Ok((deltas, Coin(0), Coin(0)));
    }
    // SJust PulsingRewUpdate = array[tag, ...]: tag 0 = Pulsing(+RewardSnapShot+Pulser), 1 = Complete(+RewardUpdate).
    let _pru = array_len(d, o, "nesRu.pulsing")?;
    let tag = read_u64(d, o, "nesRu.tag")?;
    if tag != 1 {
        // M4 (B3c / DC-EPOCH-18): a mid-pulse (Pulsing tag 0) PulsingRewUpdate means the seed+2 reward
        // distribution is NOT yet computed in the snapshot. FAIL CLOSED -- bootstrap must not proceed
        // with zero boundary rewards (a silent undercount of the seed+2 stake). Only SNothing (handled
        // above, sm == 0) is a legit empty delta.
        return Err(malformed(format!(
            "nesRu is mid-pulse (Pulsing tag {tag}, not Complete): seed+2 reward distribution not yet computed"
        )));
    }
    // Complete: the remaining field is RewardUpdate = array(5)[deltaT, -deltaR, rs, -deltaF, nonMyopic].
    expect_array(d, o, 5, "RewardUpdate")?;
    // deltaT (treasury increase) and the deltaR MAGNITUDE (reserves decrease). DeltaCoin values; we
    // take the read magnitude — for a normal reward update deltaT is a treasury gain and deltaR is a
    // reserves draw, both applied as positive pots at the seed boundary.
    let (delta_treasury, _, _) =
        read_any_int(d, o).map_err(|e| malformed(format!("RewardUpdate.deltaT: {e:?}")))?;
    let (delta_reserves, _, _) =
        read_any_int(d, o).map_err(|e| malformed(format!("RewardUpdate.deltaR: {e:?}")))?;
    map_each(d, o, "RewardUpdate.rs", |o| {
        let cred = read_credential(d, o)?;
        let sum = read_reward_set_amount_sum(d, o)?;
        if sum > 0 {
            let e = deltas.entry(cred).or_insert(Coin(0));
            e.0 = e.0.saturating_add(sum);
        }
        Ok(())
    })?;
    skip_item(d, o).map_err(|e| malformed(format!("RewardUpdate.deltaF: {e:?}")))?;
    skip_item(d, o).map_err(|e| malformed(format!("RewardUpdate.nonMyopic: {e:?}")))?;
    Ok((deltas, Coin(delta_treasury), Coin(delta_reserves)))
}

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
    // nes[2] = nesBcur (CURRENT-epoch block production so far, as of the snapshot point). Seeds the
    // accumulator's `epoch_state.block_production` so the seed→seed+1 boundary counts the WHOLE seed
    // epoch — including the early-epoch blocks the bootstrap follow never replays (it starts at
    // anchor+1). Without it the first self-derived reward update under-counts blocks → low eta.
    let current_block_production = read_block_production(d, o)?;
    // EpochState = array(4)[esAccountState, LedgerState, snapshots, nonMyopic]
    nn_expect_array(d, o, 4, "EpochState")?;
    // esAccountState = array(2)[treasury, reserves] (cardano-ledger order: treasury FIRST).
    let (treasury, reserves) = read_account_state(d, o)?;
    // LedgerState = array(2)[CertState, UTxOState]
    nn_expect_array(d, o, 2, "LedgerState")?;
    let (pool, delegation, gov_import) = nn_read_cert_state(d, o)?;
    // UTxOState = array(6)[utxo, deposited, fees, govState, incrStake, donation]
    let (
        protocol_params,
        epoch_fees,
        mut imported_gov,
        gov_deposit_pot,
        max_block_ex_units_mem,
        prev_max_tx_ex_units_mem,
        prev_max_block_ex_units_mem,
        enacted_pparam_update,
    ) = read_conway_pparams_from_utxo_state(d, o, network_id)?;
    // Thread the CertState bootstrap governance import (CRE S1 part 2): the DState-UMap DRep vote
    // delegations (2a) + the VState DRep-expiry and committee-hot-key maps (2b). They live ONLY in
    // ImportedGovState (commitment-bound); NOT in the live cert `delegation` or the live ConwayGovState gate
    // — the S4 activation threads them.
    imported_gov.vote_delegations = gov_import.vote_delegations;
    imported_gov.drep_expiry = gov_import.drep_expiry;
    imported_gov.committee_hot_keys = gov_import.committee_hot_keys;
    imported_gov.num_dormant_epochs = gov_import.num_dormant_epochs;
    // EpochState.snapshots = SnapShots = array(4)[ssStakeMark, ssStakeSet, ssStakeGo, ssFee] (this ledger
    // version caches NO mark PoolDistr). CE-3d: decode the FULL mark/set/go stake snapshots (the reward +
    // leadership stake authority the EpochAccumulator seeds) — NOT just the ECA-5 mark PoolDistr, and
    // never a cold-start empty default (which leaves the accumulator's `go` empty for ~3 boundaries ->
    // zero member rewards). The seed+1 leadership VRFs come from the durable cert-state pool registrations.
    let cert_pool_vrf: BTreeMap<PoolId, Hash32> = pool
        .pools
        .iter()
        .map(|(p, pp)| (p.clone(), pp.vrf_hash.clone()))
        .collect();
    nn_expect_array(d, o, 4, "EpochState.snapshots")?;
    let mark_snapshot = read_stake_snapshot_full_nn(d, o, "ssStakeMark")?;
    let set_snapshot = read_stake_snapshot_full_nn(d, o, "ssStakeSet")?;
    let go_snapshot = read_stake_snapshot_full_nn(d, o, "ssStakeGo")?;
    skip_item(d, o)?; // ssFee
    // ECA-5 mark PoolDistr: derived from the FULL mark snapshot (byte-identical to the prior dedicated read).
    let mark_pool_distr = derive_mark_pool_distr(&mark_snapshot, &cert_pool_vrf);
    skip_item(d, o)?; // EpochState.nonMyopic
    // nes.rewardUpdate (nesRu): decode the Complete RUPD's per-credential reward deltas — the
    // seed-window-end reward distribution that authority(N+2)'s stake needs. R-error -> Rn.
    let (reward_deltas, rupd_delta_treasury, rupd_delta_reserves) = read_reward_update_deltas(d, o)
        .map_err(|e| nn_malformed(format!("nesRu rewardUpdate: {e:?}")))?;
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
    // Coherence: a block producer is normally a known CertState pool. A producer NOT in the active set is a
    // pool that RETIRED after producing — removed from the active pool params at the epoch boundary, but its
    // block count remains valid for the epoch it produced in (cardano still credits a retired pool's blocks).
    // This is a legitimate ledger state (surfaced by the epoch-1090 Preview census; the CE-3d 1340 corpus
    // never exercised a retired producer). TOLERATE an unknown producer rather than fail closed. Defense in
    // depth: bound its block count by an impossible-per-epoch ceiling so a GROSS decode misalignment (a
    // fabricated garbage count) is still TERMINAL; a subtle misalignment corrupts the cursor and is caught
    // by the downstream epoch-state / snapshot / gov-state decode that follows.
    const IMPOSSIBLE_EPOCH_BLOCKS: u64 = 1_000_000;
    for (pid, blocks) in block_production.iter().chain(current_block_production.iter()) {
        if !cert_state.pool.pools.contains_key(pid) && *blocks > IMPOSSIBLE_EPOCH_BLOCKS {
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
        snapshots: SnapshotState {
            mark: MarkSnapshot(mark_snapshot),
            set: SetSnapshot(set_snapshot),
            go: GoSnapshot(go_snapshot),
        },
        protocol_params,
        reserves,
        treasury,
        block_production,
        current_block_production,
        reward_deltas,
        rupd_delta_treasury,
        rupd_delta_reserves,
        epoch_fees,
        imported_gov,
        reward_nibble_observation,
        max_block_ex_units_mem,
        gov_deposit_pot,
        prev_max_tx_ex_units_mem,
        prev_max_block_ex_units_mem,
        enacted_pparam_update,
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
/// parameters from `utxosGovState[curPParams]` AND the certified epoch fee pot (index 2).
/// Consumes the whole UTxOState item (the caller's offset lands just past it). The UTxO
/// map (index 0) lives in `tables` and `deposited` (index 1) is not this slice's authority;
/// `fees` (index 2) IS — it is the certified historical fee pot the first bootstrap-adjacent
/// native boundary consumes (epoch (seed)'s pre-seed fees), so the 1339->1340-style reward
/// does not under-draw. Also imports the gov-state Proposals (index 0) + Committee (index 1) for
/// CONWAY-PROPOSAL-DEPOSIT-EXPIRY. Returns `(curPParams, fees, importedGovState)`.
fn read_conway_pparams_from_utxo_state(
    d: &[u8],
    o: &mut usize,
    network_id: u8,
) -> Rn<(ProtocolParameters, Coin, ImportedGovState, Coin, u64, u64, u64, Option<GovActionId>)> {
    // UTxOState = array(6)[utxo, deposited, fees, govState, incrStake, donation]; descend to [3] = govState.
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
    // [0]=utxo (in `tables`), [1]=deposited — not this slice's authority; [2]=fees IS the certified
    // epoch fee pot. Read it, landing at [3]=govState (UTXO_STATE_GOVSTATE_INDEX).
    skip_item(d, o)?; // [0] utxo
    let deposited = Coin(nn_read_u64(d, o, "UTxOState.deposited")?); // [1] deposit pot (for the CRE census)
    let fees = Coin(nn_read_u64(d, o, "UTxOState.fees")?); // [2] fees
    // utxosGovState = array(7); curPParams is field 3.
    let gn = nn_array_len(d, o, "utxosGovState")?;
    if gn != CONWAY_GOV_STATE_FIELDS {
        return Err(NativeNonUtxoError::ProtocolParamsMissing(format!(
            "ConwayGovState arity {gn} != {CONWAY_GOV_STATE_FIELDS}"
        )));
    }
    let (proposals, enacted_pparam_update) = nn_read_proposals(d, o)?; // [0] (+ the enacted PParamUpdate root)
    let (committee, committee_quorum) = nn_read_committee(d, o)?; // [1]
    skip_item(d, o)?; // [2] constitution
    let (pp, gov_action_lifetime, pool_voting_thresholds, drep_voting_thresholds, max_block_ex_units_mem) =
        read_conway_pparams(d, o, network_id)?; // [3] curPParams
    // [4] prevPParams — the PREVIOUS-epoch params. At an enactment boundary `prev` still holds the OLD value
    // while `cur` holds the NEW, proving the flip lands exactly at THIS boundary (CRE enactment census).
    let (prev_pp, _, _, _, prev_max_block_ex_units_mem) = read_conway_pparams(d, o, network_id)?;
    let prev_max_tx_ex_units_mem = prev_pp.max_tx_ex_units_mem;
    // skip the remaining govState fields (futurePParams [5], drepPulser [6]).
    for _ in (CONWAY_GOV_STATE_CURPPARAMS_INDEX + 2)..(gn as usize) {
        skip_item(d, o)?;
    }
    // skip the remaining UTxOState fields (incrStake, donation, …).
    for _ in (UTXO_STATE_GOVSTATE_INDEX + 1)..(un as usize) {
        skip_item(d, o)?;
    }
    Ok((
        pp,
        fees,
        ImportedGovState {
            proposals,
            committee,
            committee_quorum,
            gov_action_lifetime,
            pool_voting_thresholds,
            drep_voting_thresholds,
            // Placeholders — vote_delegations (DState UMap) + drep_expiry/committee_hot_keys (VState) are
            // decoded upstream in the CertState and threaded in by decode_native_nonutxo_state; the gov-state
            // pass has no access to them here.
            vote_delegations: BTreeMap::new(),
            drep_expiry: BTreeMap::new(),
            committee_hot_keys: BTreeMap::new(),
            num_dormant_epochs: 0,
        },
        deposited,
        max_block_ex_units_mem,
        prev_max_tx_ex_units_mem,
        prev_max_block_ex_units_mem,
        enacted_pparam_update,
    ))
}

/// The bootstrap-imported Conway governance state (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1): the live
/// `Proposals` set (identity-bound `GovActionState`s incl their canonical vote maps) and the
/// constitutional `Committee` (active member → term-expiry epoch) + its quorum. Carried in
/// [`NativeSnapshotNonUtxoState`] and bound into the v6 commitment so the deposit-expiry authority path
/// has canonical, manifest-bound governance inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedGovState {
    /// The live `cgsProposals` OMap values — the authoritative tracked proposal set.
    pub proposals: Vec<GovActionState>,
    /// Active constitutional-committee cold credential → term-expiry epoch.
    pub committee: std::collections::BTreeMap<StakeCredential, u64>,
    /// Committee approval quorum (UnitInterval numerator, denominator); `None` when no committee.
    pub committee_quorum: Option<(u64, u64)>,
    /// `govActionLifetime` (curPParams index 26) — the number of epochs a newly submitted proposal lives
    /// before it expires. The current, era-correct protocol value read from the certified, manifest-bound
    /// snapshot's `curPParams` (NOT a default): it is the persisted timing authority for a LIVE-submitted
    /// proposal's `expires_after` (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3). Bound into the commitment so a
    /// tampered lifetime is caught; `0` is impossible on any real network and is rejected at capture.
    pub gov_action_lifetime: u64,
    /// `poolVotingThresholds` (curPParams index 22) — the per-action SPO ratification thresholds
    /// (CIP-1694 order: motionNoConfidence, committeeNormal, committeeNoConfidence, hardForkInitiation,
    /// ppSecurityGroup), each a UnitInterval (numerator, denominator). Imported from the certified
    /// curPParams (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S1) and commitment-bound here as
    /// tamper-evidence, but DELIBERATELY NOT threaded into the live `ConwayGovState` gate at import: the SPO
    /// gate has no active-stake guard, so feeding it the thresholds would ACTIVATE SPO ratification on the
    /// authoritative boundary. The activation is the CRE ratify slice (S4), with oracle verification.
    pub pool_voting_thresholds: Vec<(u64, u64)>,
    /// `drepVotingThresholds` (curPParams index 23) — the per-action DRep ratification thresholds (CIP-1694
    /// order: motionNoConfidence, committeeNormal, committeeNoConfidence, updateConstitution,
    /// hardForkInitiation, ppNetworkGroup, ppEconomicGroup, ppTechnicalGroup, ppGovernanceGroup,
    /// treasuryWithdrawal), each a UnitInterval. Imported + commitment-bound at S1; like the SPO thresholds,
    /// NOT threaded into the live gate until the CRE ratify activation (S4) — the DRep gate IS active-stake-
    /// guarded (`total_drep_active_stake > 0`), but both thresholds activate together for a single
    /// oracle-verified semantic boundary, never piecemeal at import.
    pub drep_voting_thresholds: Vec<(u64, u64)>,
    /// The bootstrap-baseline DRep vote delegations (`credential -> DRep`), captured from the DState UMap's
    /// per-credential drep field (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S1 part 2). This is the
    /// starting delegation graph the within-epoch vote-delegation certs (gov_cert.rs) then evolve. Imported
    /// + commitment-bound here, but — like the thresholds — NOT threaded into the live `ConwayGovState`
    /// until the CRE ratify activation (S4): a bootstrap baseline without the live cert deltas is a partial
    /// DRep-stake distribution, and activating the DRep gate on a partial distribution could under-count.
    pub vote_delegations: std::collections::BTreeMap<StakeCredential, ade_types::conway::cert::DRep>,
    /// The bootstrap-baseline DRep expiry epochs (`DRepCred -> expiry`), captured from the VState `vsDReps`
    /// map's `DRepState[0]` (CRE S1 part 2b). Required for a SAFE DRep-gate activation: the active-DRep
    /// filter uses it to exclude expired DReps from the ratification denominator — without it the denominator
    /// inflates and a live gate could falsely reject. Imported + commitment-bound; NOT threaded to the live
    /// gate until S4.
    pub drep_expiry: std::collections::BTreeMap<StakeCredential, u64>,
    /// The bootstrap-baseline committee hot->cold key map (`hot -> cold`), captured from the VState
    /// `vsCommitteeState` (variant-0 MemberAuthorized), inverted so the committee ratification gate can
    /// resolve a voter's HOT credential to its active COLD member (CRE S1 part 2b). Imported +
    /// commitment-bound; NOT threaded to the live gate until S4.
    pub committee_hot_keys: std::collections::BTreeMap<StakeCredential, StakeCredential>,
    /// `vsNumDormantEpochs` — the count of consecutive proposal-free epochs. cardano's active-DRep test is
    /// `drepExpiry + numDormantEpochs >= currentEpoch`, so S4 needs this offset to reproduce the DRep
    /// ratification denominator exactly. Imported + commitment-bound; applied at the S4 activation, not here.
    pub num_dormant_epochs: u64,
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
type ConwayPParamsGov = (ProtocolParameters, u64, Vec<(u64, u64)>, Vec<(u64, u64)>, u64);

fn read_conway_pparams(d: &[u8], o: &mut usize, network_id: u8) -> Rn<ConwayPParamsGov> {
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
    let (max_block_ex_units_mem, _) = read_ex_units(d, o)?; // [18] maxBlockExUnits (mem for the CRE census)
    skip_item(d, o)?; // [19] maxValSize
    let collateral_percent = nn_read_u64(d, o, "pp.collateralPercentage")?.min(u16::MAX as u64) as u16; // [20]
    // [21..=30]: maxCollateralInputs, poolVotingThresholds, drepVotingThresholds,
    // committeeMinSize, committeeMaxTermLength, govActionLifetime, govActionDeposit,
    // dRepDeposit, dRepActivity, minFeeRefScriptCostPerByte. govActionLifetime (index 26) IS captured
    // (the live-proposal expiry authority, CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3); the rest have no home on
    // the shared `ProtocolParameters` and are consumed but not mapped.
    let mut gov_action_lifetime: u64 = 0;
    let mut pool_voting_thresholds: Vec<(u64, u64)> = Vec::new();
    let mut drep_voting_thresholds: Vec<(u64, u64)> = Vec::new();
    for idx in 21..CONWAY_PPARAMS_FIELDS {
        if idx == CONWAY_PP_POOL_VOTING_THRESHOLDS_INDEX {
            pool_voting_thresholds = nn_read_voting_thresholds(d, o, "pp.poolVotingThresholds")?;
        } else if idx == CONWAY_PP_DREP_VOTING_THRESHOLDS_INDEX {
            drep_voting_thresholds = nn_read_voting_thresholds(d, o, "pp.drepVotingThresholds")?;
        } else if idx == CONWAY_PP_GOV_ACTION_LIFETIME_INDEX {
            gov_action_lifetime = nn_read_u64(d, o, "pp.govActionLifetime")?;
        } else {
            skip_item(d, o)?;
        }
    }
    Ok((ProtocolParameters {
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
    }, gov_action_lifetime, pool_voting_thresholds, drep_voting_thresholds, max_block_ex_units_mem))
}

/// Read a `[* UnitInterval]` voting-threshold vector (a per-action ratification-threshold list). Each
/// UnitInterval is a curPParams rational `tag(30) array(2)[num, den]`, mirroring `read_pp_rational` but
/// collecting `(num, den)` pairs in CIP-1694 action order.
fn nn_read_voting_thresholds(d: &[u8], o: &mut usize, what: &str) -> Rn<Vec<(u64, u64)>> {
    let n = nn_array_len(d, o, what)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let (t, _) = read_tag(d, o).map_err(NativeNonUtxoError::from)?;
        if t != TAG_RATIONAL_PP {
            return Err(NativeNonUtxoError::ProtocolParamsMissing(format!(
                "{what}: UnitInterval tag {t} != 30"
            )));
        }
        nn_expect_array(d, o, 2, what)?;
        let num = nn_read_u64(d, o, what)?;
        let den = nn_read_u64(d, o, what)?;
        // Fail-closed at capture (IDD §8), mirroring read_pp_rational: a voting-threshold UnitInterval must
        // be a proper fraction in [0,1]. A zero denominator or num > den is structurally invalid governance
        // state and must not enter the commitment as a silently-degenerate rational.
        if den == 0 || num > den {
            return Err(NativeNonUtxoError::ProtocolParamsMissing(format!(
                "{what}: invalid UnitInterval {num}/{den} (require 0 < den, num <= den)"
            )));
        }
        out.push((num, den));
    }
    Ok(out)
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
/// The closed `GovAction` sum's 1-byte discriminant (CIP-1694 tag order), for the v6 commitment.
fn gov_action_kind_tag(a: &GovAction) -> u8 {
    match a {
        GovAction::ParameterChange { .. } => 0,
        GovAction::HardForkInitiation { .. } => 1,
        GovAction::TreasuryWithdrawals { .. } => 2,
        GovAction::NoConfidence { .. } => 3,
        GovAction::UpdateCommittee { .. } => 4,
        GovAction::NewConstitution { .. } => 5,
        GovAction::InfoAction => 6,
    }
}

fn commit_native_nonutxo_state(s: &NativeSnapshotNonUtxoState) -> Hash32 {
    let mut v: Vec<u8> = Vec::new();
    // v5: binds the certified epoch fee pot (`epoch_fees`) alongside the v4 seed-boundary RUPD pots
    // (`rupd_delta_treasury`/`rupd_delta_reserves`), the v3 `current_block_production` (nesBcur) and the
    // v1 `block_production` (nesBprev).
    v.extend_from_slice(b"ade-native-nonutxo-state-commitment-v10");
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
    // block production (nesBprev).
    v.extend_from_slice(&(s.block_production.len() as u64).to_be_bytes());
    for (pid, blocks) in &s.block_production {
        v.extend_from_slice(&pid.0 .0);
        v.extend_from_slice(&blocks.to_be_bytes());
    }
    // current block production (nesBcur) — bound symmetrically with nesBprev (v3).
    v.extend_from_slice(&(s.current_block_production.len() as u64).to_be_bytes());
    for (pid, blocks) in &s.current_block_production {
        v.extend_from_slice(&pid.0 .0);
        v.extend_from_slice(&blocks.to_be_bytes());
    }
    // v4: the seed-boundary RUPD pots — the treasury increase + reserves-decrease magnitude.
    v.extend_from_slice(&s.rupd_delta_treasury.0.to_be_bytes());
    v.extend_from_slice(&s.rupd_delta_reserves.0.to_be_bytes());
    // v5: the certified epoch fee pot (seed epoch's pre-seed fees) — authority for the first
    // bootstrap-adjacent native boundary's reward; a fee-pot perturbation flips the commitment.
    v.extend_from_slice(&s.epoch_fees.0.to_be_bytes());
    // v6: the imported Conway gov state (CONWAY-PROPOSAL-DEPOSIT-EXPIRY) — the live proposal set
    // (identity-bound by GovActionId, with deposit/return-addr/expiry/vote maps) + the constitutional
    // committee. A tampered proposal/deposit/return-addr/expiry/vote flips the commitment. The DECODE
    // order is canonical (cardano OMap / BTreeMap), so this serialization is deterministic.
    let commit_cred = |v: &mut Vec<u8>, c: &StakeCredential| {
        let (tag, h) = match c {
            StakeCredential::KeyHash(h) => (0u8, h),
            StakeCredential::ScriptHash(h) => (1u8, h),
        };
        v.push(tag);
        v.extend_from_slice(&h.0);
    };
    let vote_byte = |vt: &Vote| -> u8 {
        match vt {
            Vote::No => 0,
            Vote::Yes => 1,
            Vote::Abstain => 2,
        }
    };
    let g = &s.imported_gov;
    // v7: the imported `govActionLifetime` (curPParams) — the timing authority for a LIVE proposal's
    // expiry (CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3). Binding it here is FRESH-BOOTSTRAP tamper-evidence: a
    // tampered/substituted lifetime in the certified snapshot flips this commitment, caught at bootstrap.
    // (It is NOT a warm-start load gate — a pre-S3 durable store recovers `gov_action_lifetime = 0` via
    // the unchanged accumulator codec and fail-closes at the runtime CAPTURE guard `GovActionLifetime
    // Unproven`, which fires exactly where the value is consumed; the lifetime is used nowhere else.)
    v.extend_from_slice(&g.gov_action_lifetime.to_be_bytes());
    v.extend_from_slice(&(g.proposals.len() as u64).to_be_bytes());
    for p in &g.proposals {
        v.extend_from_slice(&p.action_id.tx_hash.0);
        v.extend_from_slice(&p.action_id.index.to_be_bytes());
        // action-kind discriminant (the closed GovAction sum, by declaration order).
        v.push(gov_action_kind_tag(&p.gov_action));
        v.extend_from_slice(&p.deposit.0.to_be_bytes());
        v.extend_from_slice(&(p.return_addr.len() as u64).to_be_bytes());
        v.extend_from_slice(&p.return_addr);
        v.extend_from_slice(&p.proposed_in.0.to_be_bytes());
        v.extend_from_slice(&p.expires_after.0.to_be_bytes());
        for votes in [&p.committee_votes, &p.drep_votes] {
            v.extend_from_slice(&(votes.len() as u64).to_be_bytes());
            for (cred, vt) in votes {
                commit_cred(&mut v, cred);
                v.push(vote_byte(vt));
            }
        }
        v.extend_from_slice(&(p.spo_votes.len() as u64).to_be_bytes());
        for (h, vt) in &p.spo_votes {
            v.extend_from_slice(&h.0);
            v.push(vote_byte(vt));
        }
    }
    v.extend_from_slice(&(g.committee.len() as u64).to_be_bytes());
    for (cred, exp) in &g.committee {
        commit_cred(&mut v, cred);
        v.extend_from_slice(&exp.to_be_bytes());
    }
    match g.committee_quorum {
        Some((n, d)) => {
            v.push(1);
            v.extend_from_slice(&n.to_be_bytes());
            v.extend_from_slice(&d.to_be_bytes());
        }
        None => v.push(0),
    }
    // v8: the imported per-action SPO/DRep voting thresholds (curPParams 22/23) — the ratification-gate
    // authority (CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY S1). Bound as fresh-bootstrap tamper-evidence,
    // same posture as the lifetime above. The thresholds are captured here for tamper-evidence but are NOT
    // threaded into the live ConwayGovState gate at import (the SPO gate has no active-stake guard, so that
    // would activate SPO ratification on the authoritative boundary); the ratify semantic activates
    // deliberately in the CRE ratify slice (S4). A pre-S1 store recovers EMPTY vectors via the unchanged
    // accumulator codec, identical to the not-yet-activated live gate.
    for thresholds in [&g.pool_voting_thresholds, &g.drep_voting_thresholds] {
        v.extend_from_slice(&(thresholds.len() as u64).to_be_bytes());
        for (num, den) in thresholds {
            v.extend_from_slice(&num.to_be_bytes());
            v.extend_from_slice(&den.to_be_bytes());
        }
    }
    // v9: the bootstrap-baseline DRep vote delegations (DState UMap drep field, CRE S1 part 2a). Bound as
    // fresh-bootstrap tamper-evidence; NOT threaded into the live gate until the S4 activation (same posture
    // as the thresholds). The BTreeMap gives a deterministic key order; the DRep is bound by discriminant +
    // its 28-byte hash (predefined DReps carry no hash).
    v.extend_from_slice(&(g.vote_delegations.len() as u64).to_be_bytes());
    for (cred, drep) in &g.vote_delegations {
        commit_cred(&mut v, cred);
        match drep {
            DRep::KeyHash(h) => {
                v.push(0);
                v.extend_from_slice(&h.0);
            }
            DRep::ScriptHash(h) => {
                v.push(1);
                v.extend_from_slice(&h.0);
            }
            DRep::AlwaysAbstain => v.push(2),
            DRep::AlwaysNoConfidence => v.push(3),
        }
    }
    // v10: the bootstrap-baseline DRep expiry + committee hot->cold maps (VState, CRE S1 part 2b). Bound as
    // fresh-bootstrap tamper-evidence; NOT threaded into the live gate until S4. Both are BTreeMaps
    // (deterministic key order).
    v.extend_from_slice(&(g.drep_expiry.len() as u64).to_be_bytes());
    for (cred, expiry) in &g.drep_expiry {
        commit_cred(&mut v, cred);
        v.extend_from_slice(&expiry.to_be_bytes());
    }
    v.extend_from_slice(&(g.committee_hot_keys.len() as u64).to_be_bytes());
    for (hot, cold) in &g.committee_hot_keys {
        commit_cred(&mut v, hot);
        commit_cred(&mut v, cold);
    }
    v.extend_from_slice(&g.num_dormant_epochs.to_be_bytes());
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
fn nn_read_cert_state(
    d: &[u8],
    o: &mut usize,
) -> Rn<(PoolState, DelegationState, BootstrapGovImport)> {
    read_cert_state(d, o).map_err(|e| match e {
        LedgerDbStateError::ZeroVrf(p) => NativeNonUtxoError::ZeroVrf(p),
        LedgerDbStateError::MalformedCbor(s) => NativeNonUtxoError::MalformedCbor(s),
        other => nn_malformed(format!("cert_state: {other:?}")),
    })
}

// ===========================================================================
// CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1 — bootstrap import of the gov-state Proposals + Committee.
// All decoders are FAIL-CLOSED: an unknown GovAction variant / vote value / committee shape is
// TERMINAL (Unsupported/Malformed), never coerced to an empty/default. Identity is the GovActionId.
// ===========================================================================

/// Lift the `R<>` credential decoder into the native enum as a governance-malformed terminal.
fn nn_read_credential(d: &[u8], o: &mut usize) -> Rn<StakeCredential> {
    read_credential(d, o).map_err(|e| match e {
        LedgerDbStateError::MalformedCbor(s) => NativeNonUtxoError::MalformedGovernanceState(s),
        other => NativeNonUtxoError::MalformedGovernanceState(format!("credential: {other:?}")),
    })
}

/// `GovActionId` = `array(2)[tx_hash(bytes32), index(uint)]`.
fn nn_read_gov_action_id(d: &[u8], o: &mut usize) -> Rn<GovActionId> {
    nn_expect_array(d, o, 2, "GovActionId")?;
    let (txid, _) = read_bytes(d, o)?;
    if txid.len() != 32 {
        return Err(NativeNonUtxoError::MalformedGovernanceState(format!(
            "GovActionId.tx_hash len {} != 32",
            txid.len()
        )));
    }
    let mut h = [0u8; 32];
    h.copy_from_slice(&txid);
    let (idx, _) = ade_codec::cbor::read_uint(d, o)?;
    Ok(GovActionId {
        tx_hash: Hash32(h),
        index: u32::try_from(idx)
            .map_err(|_| NativeNonUtxoError::MalformedGovernanceState("GovActionId.index > u32".into()))?,
    })
}

/// A Conway `Vote` (0=No, 1=Yes, 2=Abstain). Unknown ⇒ terminal (no silent default).
fn nn_read_vote(d: &[u8], o: &mut usize) -> Rn<Vote> {
    let (v, _) = ade_codec::cbor::read_uint(d, o)?;
    match v {
        0 => Ok(Vote::No),
        1 => Ok(Vote::Yes),
        2 => Ok(Vote::Abstain),
        other => Err(NativeNonUtxoError::UnsupportedGovernanceState(format!(
            "vote value {other}"
        ))),
    }
}

/// A vote map keyed by a discriminated `StakeCredential` (committee-hot or DRep credentials).
fn nn_read_cred_vote_map(d: &[u8], o: &mut usize) -> Rn<Vec<(StakeCredential, Vote)>> {
    let enc = read_map_header(d, o)?;
    let mut out = Vec::new();
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                let cred = nn_read_credential(d, o)?;
                out.push((cred, nn_read_vote(d, o)?));
            }
        }
        ContainerEncoding::Indefinite => {
            while !ade_codec::cbor::is_break(d, *o)? {
                let cred = nn_read_credential(d, o)?;
                out.push((cred, nn_read_vote(d, o)?));
            }
            *o += 1; // consume break
        }
    }
    Ok(out)
}

/// The SPO vote map keyed by a pool `KeyHash` (bytes28).
fn nn_read_spo_vote_map(d: &[u8], o: &mut usize) -> Rn<Vec<(Hash28, Vote)>> {
    let enc = read_map_header(d, o)?;
    let mut out = Vec::new();
    let one = |d: &[u8], o: &mut usize| -> Rn<(Hash28, Vote)> {
        let (k, _) = read_bytes(d, o)?;
        if k.len() != 28 {
            return Err(NativeNonUtxoError::MalformedGovernanceState(format!(
                "spo vote key len {} != 28",
                k.len()
            )));
        }
        Ok((nn_hash28(k), nn_read_vote(d, o)?))
    };
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                out.push(one(d, o)?);
            }
        }
        ContainerEncoding::Indefinite => {
            while !ade_codec::cbor::is_break(d, *o)? {
                out.push(one(d, o)?);
            }
            *o += 1; // consume break
        }
    }
    Ok(out)
}

/// One `GovActionState` = `array(7)[gasId, ccVotes, drepVotes, spoVotes, procedure, proposed_in,
/// expires_after]`. The `procedure` (= `array(4)[deposit, return_addr, gov_action, anchor]`) reuses the
/// SAME closed `gov_action` grammar as the tx-body path (`ade_codec::conway::governance`) — an unknown
/// gov-action variant fails closed identically (no silent skip).
fn nn_read_gov_action_state(d: &[u8], o: &mut usize) -> Rn<GovActionState> {
    nn_expect_array(d, o, 7, "GovActionState")?;
    let action_id = nn_read_gov_action_id(d, o)?;
    let committee_votes = nn_read_cred_vote_map(d, o)?;
    let drep_votes = nn_read_cred_vote_map(d, o)?;
    let spo_votes = nn_read_spo_vote_map(d, o)?;
    let proc = ade_codec::conway::governance::decode_proposal_procedure(d, o)?;
    let proposed_in = EpochNo(nn_read_u64(d, o, "GovActionState.proposed_in")?);
    let expires_after = EpochNo(nn_read_u64(d, o, "GovActionState.expires_after")?);
    Ok(GovActionState {
        action_id,
        committee_votes,
        drep_votes,
        spo_votes,
        deposit: proc.deposit,
        return_addr: proc.return_addr,
        gov_action: proc.gov_action,
        proposed_in,
        expires_after,
    })
}

/// The Conway live `Proposals` (gov-state index 0) = `array(2)[GovRelation, OMap]`. The OMap is a (usually
/// indefinite) array of `GovActionState` — the authoritative proposal set. The `GovRelation` is the ledger's
/// enacted-authority roots (`prevGovActionIds`), whose PParamUpdate slot is decoded for the CRE enactment
/// census (returned alongside the proposals); the other purposes are not this decode's authority.
fn nn_read_proposals(d: &[u8], o: &mut usize) -> Rn<(Vec<GovActionState>, Option<GovActionId>)> {
    nn_expect_array(d, o, 2, "Proposals")?;
    // [0] GovRelation = array(4)[ StrictMaybe GovActionId ; 4 ] — the enacted-authority root per purpose, in
    // cardano-ledger order (PParamUpdate, HardFork, Committee, Constitution). cardano StrictMaybe encodes
    // SNothing = array(0), SJust x = array(1)[x]. Element 0 (PParamUpdate) IS `prevGovActionIds.pgaPParamUpdate`:
    // at an enactment boundary it becomes the enacting action's id, which the CRE census records as the proof
    // that the enacted params were CAUSED by that action (not merely coincident with its observables). Elements
    // 1..3 are not this census's authority (skipped). NEVER a live-gate input — read for the census only.
    nn_expect_array(d, o, 4, "Proposals.GovRelation")?;
    let enacted_pparam_update = nn_read_strict_maybe_gov_action_id(d, o, "GovRelation.pparamUpdate")?;
    for _ in 1..4 {
        skip_item(d, o)?; // HardFork, Committee, Constitution enacted roots — not this census's authority
    }
    let enc = read_array_header(d, o)?;
    let mut out = Vec::new();
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                out.push(nn_read_gov_action_state(d, o)?);
            }
        }
        ContainerEncoding::Indefinite => {
            while !ade_codec::cbor::is_break(d, *o)? {
                out.push(nn_read_gov_action_state(d, o)?);
            }
            *o += 1; // consume break
        }
    }
    Ok((out, enacted_pparam_update))
}

/// A cardano `StrictMaybe GovActionId`: `SNothing = array(0)`, `SJust id = array(1)[GovActionId]`. Any other
/// arity is TERMINAL (never coerced to `None`). Used to read the enacted-authority roots at the head of the
/// Conway `Proposals` (the `GovRelation` of `prevGovActionIds`).
fn nn_read_strict_maybe_gov_action_id(d: &[u8], o: &mut usize, what: &str) -> Rn<Option<GovActionId>> {
    match nn_array_len(d, o, what)? {
        0 => Ok(None),
        1 => Ok(Some(nn_read_gov_action_id(d, o)?)),
        n => Err(NativeNonUtxoError::MalformedGovernanceState(format!(
            "{what}: StrictMaybe arity {n} not in {{0,1}}"
        ))),
    }
}

/// A CBOR `UnitInterval` = `tag(30) array(2)[num, den]` → `(num, den)`.
fn nn_read_unit_interval(d: &[u8], o: &mut usize, what: &str) -> Rn<(u64, u64)> {
    let (t, _) = read_tag(d, o)?;
    if t != 30 {
        return Err(NativeNonUtxoError::MalformedGovernanceState(format!(
            "{what}: UnitInterval tag {t} != 30"
        )));
    }
    nn_expect_array(d, o, 2, what)?;
    let num = nn_read_u64(d, o, "UnitInterval.num")?;
    let den = nn_read_u64(d, o, "UnitInterval.den")?;
    Ok((num, den))
}

/// The constitutional `Committee` (gov-state index 1) = `StrictMaybe Committee`. `SJust` =
/// `array(1)[array(2)[map{cold_cred ⇒ term_expiry_epoch}, quorum:UnitInterval]]`; `SNothing` =
/// `array(0)` or `null`. Returns the active-member→expiry map + the quorum (None when no committee).
fn nn_read_committee(
    d: &[u8],
    o: &mut usize,
) -> Rn<(std::collections::BTreeMap<StakeCredential, u64>, Option<(u64, u64)>)> {
    use std::collections::BTreeMap;
    // SNothing as a CBOR simple/null (major 7).
    if peek_major(d, *o)? == 7 {
        skip_item(d, o)?;
        return Ok((BTreeMap::new(), None));
    }
    let enc = read_array_header(d, o)?;
    let n = match enc {
        ContainerEncoding::Definite(n, _) => n,
        ContainerEncoding::Indefinite => {
            return Err(NativeNonUtxoError::MalformedGovernanceState(
                "committee StrictMaybe indefinite array".into(),
            ))
        }
    };
    if n == 0 {
        return Ok((BTreeMap::new(), None)); // SNothing as array(0)
    }
    if n != 1 {
        return Err(NativeNonUtxoError::UnsupportedGovernanceState(format!(
            "committee StrictMaybe arity {n} (expected 0 or 1)"
        )));
    }
    nn_expect_array(d, o, 2, "Committee")?;
    let mut members = BTreeMap::new();
    let menc = read_map_header(d, o)?;
    let one = |d: &[u8], o: &mut usize, members: &mut BTreeMap<StakeCredential, u64>| -> Rn<()> {
        let cred = nn_read_credential(d, o)?;
        let epoch = nn_read_u64(d, o, "committee.member.expiry")?;
        members.insert(cred, epoch);
        Ok(())
    };
    match menc {
        ContainerEncoding::Definite(mn, _) => {
            for _ in 0..mn {
                one(d, o, &mut members)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !ade_codec::cbor::is_break(d, *o)? {
                one(d, o, &mut members)?;
            }
            *o += 1; // consume break
        }
    }
    let quorum = nn_read_unit_interval(d, o, "committee.quorum")?;
    Ok((members, Some(quorum)))
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
