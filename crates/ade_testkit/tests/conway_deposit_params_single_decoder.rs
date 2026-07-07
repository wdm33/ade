//! One-decoder proof (CONWAY-DEPOSIT-PARAMS bootstrap authority): the harness snapshot parser
//! (`ade_testkit::harness::snapshot_loader::parse_conway_gov_params`) and the single canonical BLUE decoder
//! (`ade_ledger::ledgerdb_state::decode_native_nonutxo_state`) read the Conway-only deposit params
//! (`govActionDeposit`[27] / `dRepDeposit`[28] / `dRepActivity`[29]) from the SAME curPParams positions and
//! produce IDENTICAL values on the SAME synthetic snapshot bytes. This is the mechanical guard against a
//! "close enough" parallel parser drifting from the authority decoder (IDD: one canonical decoder).
//!
//! Hermetic — the synthetic V2 `state` is built in-process (pure, no file I/O), mirroring the fixture in
//! `crates/ade_ledger/tests/ledgerdb_nonutxo_hermetic.rs` so both decoders navigate a real-shaped Conway
//! NewEpochState.

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_testkit::harness::snapshot_loader::parse_conway_gov_params;
use ade_types::{Hash32, SlotNo};

/// A testnet network magic (preprod) — derives `network_id = 0`, matching the fixture pool reward-account
/// network nibble (0).
const TESTNET_MAGIC: u32 = 1;

// The distinctive curPParams deposit values the fixture encodes at 27/28/29 (NOT 20/None defaults).
const FIXTURE_GOV_ACTION_DEPOSIT: u64 = 1_000_000_000;
const FIXTURE_DREP_DEPOSIT: u64 = 500_000_000;
const FIXTURE_DREP_ACTIVITY: u64 = 20;

// ---- minimal CBOR byte builders (same shape as the ledgerdb hermetic fixture) ----
fn hdr(major: u8, n: u64) -> Vec<u8> {
    let mt = major << 5;
    if n < 24 {
        vec![mt | n as u8]
    } else if n < 256 {
        vec![mt | 24, n as u8]
    } else if n < 65536 {
        vec![mt | 25, (n >> 8) as u8, n as u8]
    } else if n <= u32::MAX as u64 {
        let mut v = vec![mt | 26];
        v.extend_from_slice(&(n as u32).to_be_bytes());
        v
    } else {
        let mut v = vec![mt | 27];
        v.extend_from_slice(&n.to_be_bytes());
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
fn rational(num: u64, den: u64) -> Vec<u8> {
    concat(&[tag(30), arr(2), uint(num), uint(den)])
}

const POOL_ID: [u8; 28] = [0x11; 28];

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
        uint(0), // reward-account network byte (testnet)
        bytes(&[0xaa; 28]),
        arr(0),
    ])
}

/// A full Conway curPParams array(31) with the distinctive deposit values at 27/28/29.
fn conway_pparams() -> Vec<u8> {
    concat(&[
        arr(31),
        uint(44),
        uint(155_381),
        uint(90_112),
        uint(16_384),
        uint(1_100),
        uint(2_000_000),
        uint(500_000_000),
        uint(18),
        uint(500),
        rational(3, 10),
        rational(3, 1000),
        rational(1, 5),
        concat(&[arr(2), uint(11), uint(0)]),
        uint(170_000_000),
        uint(4_310),
        concat(&[map(0)]),
        concat(&[arr(2), rational(577, 10_000), rational(721, 10_000_000)]),
        concat(&[arr(2), uint(16_500_000), uint(10_000_000_000)]),
        concat(&[arr(2), uint(72_000_000), uint(20_000_000_000)]),
        uint(5_000),
        uint(150),
        uint(3),
        arr(0),
        arr(0),
        uint(3),
        uint(146),
        uint(6),
        uint(FIXTURE_GOV_ACTION_DEPOSIT), // 27 govActionDeposit
        uint(FIXTURE_DREP_DEPOSIT),       // 28 dRepDeposit
        uint(FIXTURE_DREP_ACTIVITY),      // 29 dRepActivity
        rational(15, 1),                  // 30 minFeeRefScriptCostPerByte
    ])
}

fn build_state() -> Vec<u8> {
    let vrf = [0x55u8; 32];
    let pstate = concat(&[
        arr(4),
        map(0),
        concat(&[map(1), bytes(&POOL_ID), pool_params(vrf)]),
        map(0),
        concat(&[map(1), bytes(&POOL_ID), uint(1337)]),
    ]);
    let umap_entry_key = concat(&[arr(2), uint(1), bytes(&[0x22; 28])]);
    let umap_entry_val = concat(&[arr(4), uint(500), uint(2_000_000), bytes(&POOL_ID), vec![NULL]]);
    let dstate = concat(&[
        arr(4),
        concat(&[map(1), umap_entry_key, umap_entry_val]),
        map(0),
        map(0),
        arr(0),
    ]);
    let drep_cred = concat(&[arr(2), uint(0), bytes(&[0xD1; 28])]);
    let drep_state = concat(&[arr(4), uint(1350), arr(0), uint(500_000_000), arr(0)]);
    let cold_cred = concat(&[arr(2), uint(0), bytes(&[0xC0; 28])]);
    let hot_cred = concat(&[arr(2), uint(0), bytes(&[0xC1; 28])]);
    let committee_auth = concat(&[arr(2), uint(0), hot_cred]);
    let vstate = concat(&[
        arr(3),
        concat(&[map(1), drep_cred, drep_state]),
        concat(&[map(1), cold_cred, committee_auth]),
        uint(0),
    ]);
    let cert = concat(&[arr(3), vstate, pstate, dstate]);

    let pparams = conway_pparams();
    let gov_state = concat(&[
        arr(7),
        concat(&[arr(2), concat(&[arr(4), arr(0), arr(0), arr(0), arr(0)]), arr(0)]),
        arr(0),
        arr(0),
        pparams.clone(),
        pparams,
        arr(0),
        arr(0),
    ]);
    let utxo_state = concat(&[
        arr(6),
        map(0),
        uint(1_159_004_000_000),
        uint(6_669_569_234),
        gov_state,
        map(0),
        uint(0),
    ]);

    let ls = concat(&[arr(2), cert, utxo_state]);
    let acct = concat(&[arr(2), uint(1_890_267_427_632_547), uint(13_051_749_596_873_397)]);
    let mark = concat(&[
        arr(2),
        concat(&[
            map(1),
            concat(&[arr(2), uint(0), bytes(&[0x33; 28])]),
            concat(&[arr(2), uint(1_000_000), bytes(&POOL_ID)]),
        ]),
        map(0),
    ]);
    let empty_snap = concat(&[arr(2), map(0), map(0)]);
    let snaps = concat(&[arr(4), mark, empty_snap.clone(), empty_snap, uint(0)]);
    let es = concat(&[arr(4), acct, ls, snaps, arr(0)]);

    let pd = concat(&[
        map(1),
        bytes(&POOL_ID),
        concat(&[arr(3), uint(0), uint(100), bytes(&vrf)]),
    ]);
    let pdw = concat(&[arr(2), pd, uint(0)]);

    let mut bprev = map(1);
    bprev.extend(bytes(&POOL_ID));
    bprev.extend(uint(50));

    let nes = concat(&[
        arr(7),
        uint(296),
        bprev,
        map(0),
        es,
        arr(0),
        pdw,
        vec![NULL],
    ]);
    let inner2 = concat(&[arr(2), concat(&[arr(1), uint(0)]), nes]);
    let era_state = concat(&[arr(2), uint(2), inner2]);
    // telescope: 6 past eras + the current (era index 6 = Conway)
    let mut tele = arr(7);
    for _ in 0..6 {
        tele.extend(concat(&[arr(2), bound(), bound()]));
    }
    tele.extend(concat(&[arr(2), bound(), era_state]));
    let mut ns = arr(6);
    for kk in 0..6u8 {
        ns.extend(nonce(kk + 1));
    }
    let hs = concat(&[arr(2), uint(0), ns]);
    concat(&[arr(2), uint(1), concat(&[arr(2), tele, hs])])
}

fn point() -> SeedPoint {
    SeedPoint {
        slot: SlotNo(126_400_064),
        block_hash: Hash32([0xab; 32]),
    }
}

/// The BLUE authority decoder and the harness parser read IDENTICAL deposit params from the SAME bytes.
#[test]
fn harness_and_blue_decoder_agree_on_conway_deposit_params() {
    let state = build_state();

    // BLUE authority decoder (ade_ledger) — the single canonical curPParams decoder.
    let (s1a, _commit) =
        decode_native_nonutxo_state(&state, point(), 296, TESTNET_MAGIC).expect("BLUE decode");
    let blue = &s1a.conway_deposit_params;

    // Harness parser (ade_testkit) — reads the SAME curPParams positions 27/28/29.
    let harness = parse_conway_gov_params(&state).expect("harness parse");

    assert_eq!(
        blue.gov_action_deposit.0, harness.gov_action_deposit,
        "govActionDeposit (curPParams idx 27) must be IDENTICAL across the two decoders"
    );
    assert_eq!(
        blue.drep_deposit.0, harness.drep_deposit,
        "dRepDeposit (curPParams idx 28) must be IDENTICAL across the two decoders"
    );
    assert_eq!(
        blue.drep_activity, harness.drep_activity,
        "dRepActivity (curPParams idx 29) must be IDENTICAL across the two decoders"
    );

    // And both match the fixture's own curPParams (not a coincidental shared default).
    assert_eq!(blue.gov_action_deposit.0, FIXTURE_GOV_ACTION_DEPOSIT);
    assert_eq!(blue.drep_deposit.0, FIXTURE_DREP_DEPOSIT);
    assert_eq!(blue.drep_activity, FIXTURE_DREP_ACTIVITY);
}
