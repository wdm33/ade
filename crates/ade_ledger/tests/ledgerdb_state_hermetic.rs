//! Hermetic fail-closed + round-trip + determinism tests for the native V2 LedgerDB `state` decoder
//! (MITHRIL-VERIFIED-ANCHOR-IMPORT, Stage 1). Builds minimal synthetic V2 `state` CBOR in-process
//! (pure — no file I/O), so the fail-closed boundaries are committable CI tests. The happy fixture
//! carries the required at-least-one retirement, delegation, reward, and a real-VRF pool.

use ade_ledger::ledgerdb_state::{probe_ledgerdb_state, LedgerDbStateError};

// ---- minimal CBOR byte builders ----
fn hdr(major: u8, n: u64) -> Vec<u8> {
    let mt = major << 5;
    if n < 24 {
        vec![mt | n as u8]
    } else if n < 256 {
        vec![mt | 24, n as u8]
    } else if n < 65536 {
        vec![mt | 25, (n >> 8) as u8, n as u8]
    } else {
        let mut v = vec![mt | 26];
        v.extend_from_slice(&(n as u32).to_be_bytes());
        v
    }
}
fn arr(n: u64) -> Vec<u8> {
    hdr(4, n)
}
fn map(n: u64) -> Vec<u8> {
    hdr(5, n)
}
fn uint(n: u64) -> Vec<u8> {
    hdr(0, n)
}
fn bytes(b: &[u8]) -> Vec<u8> {
    let mut v = hdr(2, b.len() as u64);
    v.extend_from_slice(b);
    v
}
fn tag(t: u64) -> Vec<u8> {
    hdr(6, t)
}
const NULL: u8 = 0xf6;

fn concat(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}
fn bound() -> Vec<u8> {
    concat(&[arr(3), uint(0), uint(0), uint(0)])
}
fn nonce(b: u8) -> Vec<u8> {
    concat(&[arr(2), uint(1), bytes(&[b; 32])])
}

/// PoolParams value (array 6): vrf, pledge, cost, margin(tag30 [n,d]), rewardAcct[net,hash28], owners.
fn pool_params(vrf: [u8; 32]) -> Vec<u8> {
    concat(&[
        arr(6),
        bytes(&vrf),
        uint(1000),
        uint(340),
        tag(30),
        arr(2),
        uint(1),
        uint(10),
        arr(2),
        uint(0),
        bytes(&[0xaa; 28]),
        arr(0), // owners (empty)
    ])
}

const POOL_ID: [u8; 28] = [0x11; 28];

/// Build a minimal synthetic V2 `state` CBOR. `current_era_index` controls the telescope's current
/// era (6 = Conway). `pool_vrf` / `pool_distr_vrf` let the fail-closed tests force ZeroVrf / mismatch.
fn build_state(current_era_index: u64, pool_vrf: [u8; 32], pool_distr_vrf: [u8; 32]) -> Vec<u8> {
    // PState = [map32B(empty), pools(1), future(empty), retiring(1)]
    let pstate = concat(&[
        arr(4),
        map(0),
        concat(&[map(1), bytes(&POOL_ID), pool_params(pool_vrf)]),
        map(0),
        concat(&[map(1), bytes(&POOL_ID), uint(1337)]),
    ]);
    // DState = [umap(1), futureGenDelegs, genDelegs, iRewards]; umap entry = cred -> [reward, deposit, pool, null]
    let umap_entry_key = concat(&[arr(2), uint(1), bytes(&[0x22; 28])]);
    let umap_entry_val = concat(&[arr(4), uint(500), uint(2_000_000), bytes(&POOL_ID), vec![NULL]]);
    let dstate = concat(&[
        arr(4),
        concat(&[map(1), umap_entry_key, umap_entry_val]),
        map(0),
        map(0),
        arr(0),
    ]);
    // CertState = [VState, PState, DState]. VState = array(3)[vsDReps, vsCommitteeState, vsNumDormantEpochs]
    // — a minimal empty VState (the CRE S1 `read_vstate` tightening requires array(3), not the old arr(0)
    // stub; this probe asserts only PState/DState counts, so empty dreps/committee is fine).
    let vstate = concat(&[arr(3), map(0), map(0), uint(0)]);
    let cert = concat(&[arr(3), vstate, pstate, dstate]);
    // LedgerState = [CertState, UTxOState(dummy)]
    let ls = concat(&[arr(2), cert, arr(0)]);
    // EpochState = [acct, LedgerState, snaps, nonmyopic]
    let es = concat(&[arr(4), arr(0), ls, arr(0), arr(0)]);
    // PoolDistr wrapper = [poolDistr_map(1), totalActiveStake]
    let pd = concat(&[
        map(1),
        bytes(&POOL_ID),
        concat(&[arr(3), uint(0), uint(100), bytes(&pool_distr_vrf)]),
    ]);
    let pdw = concat(&[arr(2), pd, uint(0)]);
    // NES = [epoch, blocksPrev, blocksCur, EpochState, rewardUpdate, poolDistrWrapper, stashed]
    let nes = concat(&[
        arr(7),
        uint(1336),
        map(0),
        map(0),
        es,
        arr(0),
        pdw,
        vec![NULL],
    ]);
    // era live state = [tag(int), [dummy(array1), NES]]
    let inner2 = concat(&[arr(2), concat(&[arr(1), uint(0)]), nes]);
    let era_state = concat(&[arr(2), uint(2), inner2]);
    // telescope: current_era_index past eras [bound,bound] + current [bound, era_state]
    let mut tele = arr(current_era_index + 1);
    for _ in 0..current_era_index {
        tele.extend(concat(&[arr(2), bound(), bound()]));
    }
    tele.extend(concat(&[arr(2), bound(), era_state]));
    // headerState = [dummy, array(6) of the trailing PraosState nonces in record
    // order [evolving, candidate, epoch, previousEpoch, lab, lastEpochBlock]]
    let mut ns = arr(6);
    for k in 0..6u8 {
        ns.extend(nonce(k + 1));
    }
    let hs = concat(&[arr(2), uint(0), ns]);
    // ExtLedgerState = [telescope, headerState]; top = [version, ExtLedgerState]
    concat(&[arr(2), uint(1), concat(&[arr(2), tele, hs])])
}

#[test]
fn happy_minimal_state_decodes_with_required_elements() {
    let vrf = [0x55u8; 32];
    let st = build_state(6, vrf, vrf);
    let p = probe_ledgerdb_state(&st, 1336).expect("decode");
    assert_eq!(p.era_index, 6);
    assert_eq!(p.epoch, 1336);
    assert_eq!(p.active_pool_count, 1);
    assert_eq!(p.vrf_count, 1, "the pool carries a real VRF");
    assert_eq!(p.retiring_count, 1, ">= one retirement");
    assert_eq!(p.delegation_count, 1, ">= one delegation");
    assert_eq!(p.reward_count, 1, ">= one reward");
    assert_eq!(p.registration_count, 1);
    assert_eq!(p.pool_distr_count, 1);
}

#[test]
fn determinism_same_bytes_same_commitment() {
    let vrf = [0x55u8; 32];
    let st = build_state(6, vrf, vrf);
    let a = probe_ledgerdb_state(&st, 1336).unwrap();
    let b = probe_ledgerdb_state(&st, 1336).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.cert_state_commitment, b.cert_state_commitment);
}

#[test]
fn zero_vrf_is_terminal() {
    let st = build_state(6, [0u8; 32], [0u8; 32]);
    assert!(matches!(
        probe_ledgerdb_state(&st, 1336),
        Err(LedgerDbStateError::ZeroVrf(_))
    ));
}

#[test]
fn wrong_era_is_terminal_no_fallback_to_latest() {
    let vrf = [0x55u8; 32];
    let st = build_state(5, vrf, vrf); // current era = Babbage (5), not Conway
    assert!(matches!(
        probe_ledgerdb_state(&st, 1336),
        Err(LedgerDbStateError::UnsupportedEra { current_index: 5 })
    ));
}

#[test]
fn pool_distr_vrf_mismatch_is_terminal() {
    let st = build_state(6, [0x55u8; 32], [0x66u8; 32]);
    assert!(matches!(
        probe_ledgerdb_state(&st, 1336),
        Err(LedgerDbStateError::PoolDistrVrfMismatch(_))
    ));
}

#[test]
fn epoch_mismatch_is_terminal() {
    let vrf = [0x55u8; 32];
    let st = build_state(6, vrf, vrf);
    assert!(matches!(
        probe_ledgerdb_state(&st, 9999),
        Err(LedgerDbStateError::EpochMismatch {
            decoded_epoch: 1336,
            manifest_epoch: 9999
        })
    ));
}

#[test]
fn malformed_cbor_is_terminal() {
    assert!(matches!(
        probe_ledgerdb_state(&[0x00, 0x01, 0x02], 1336),
        Err(LedgerDbStateError::MalformedCbor(_))
    ));
}
