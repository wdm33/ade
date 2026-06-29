//! Hermetic tests for the native non-UTxO snapshot decoder
//! (MITHRIL-VERIFIED-ANCHOR-INTEGRATION, S1a). Builds minimal synthetic V2 `state` CBOR in-process
//! (pure — no file I/O) carrying a Conway NewEpochState with: a real-VRF pool + PoolDistr (VRF
//! cross-checked), an esAccountState (treasury/reserves), a full Conway curPParams array(31), and a
//! nesBprev block-production map. Proves: determinism + commitment equality; commitment binds EVERY
//! emitted field (perturbation); and the fail-closed negatives (non-Conway, epoch mismatch, zero VRF,
//! VRF mismatch, missing/malformed PParams/accountState/blockProduction, unknown-pool producer).

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::{decode_native_nonutxo_state, NativeNonUtxoError};
use ade_ledger::pparams::MinUtxoRule;
use ade_types::tx::Coin;
use ade_types::{Hash32, SlotNo};

/// The mainnet network magic — derives `network_id = 1`.
const MAINNET_MAGIC: u32 = 764_824_073;
/// A testnet network magic (preprod) — derives `network_id = 0`, matching the fixture
/// reward-account network nibble (0).
const TESTNET_MAGIC: u32 = 1;

// ---- minimal CBOR byte builders (same shape as the Stage-1 hermetic fixture) ----
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

/// PoolParams value (array 6): vrf, pledge, cost, margin(tag30 [n,d]), rewardAcct[net,hash28], owners.
/// `reward_net` is the reward-account network byte (the snapshot's network-bound field).
fn pool_params(vrf: [u8; 32], reward_net: u8) -> Vec<u8> {
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
        uint(reward_net as u64),
        bytes(&[0xaa; 28]),
        arr(0), // owners (empty)
    ])
}

const POOL_ID: [u8; 28] = [0x11; 28];
const OTHER_POOL_ID: [u8; 28] = [0x99; 28];

/// A full Conway curPParams array(31), with the field layout the decoder expects. The mapped fields
/// carry recognizable sentinel values so the perturbation tests can target them by position.
fn conway_pparams() -> Vec<u8> {
    conway_pparams_with_min_fee_a(44)
}

/// As [`conway_pparams`] but with a caller-chosen `minFeeA` (so the commitment-binding test can
/// perturb the protocol params cleanly, without hand-computed byte offsets).
fn conway_pparams_with_min_fee_a(min_fee_a: u64) -> Vec<u8> {
    concat(&[
        arr(31),
        uint(min_fee_a),  // 0 minFeeA
        uint(155_381),    // 1 minFeeB
        uint(90_112),     // 2 maxBBSize
        uint(16_384),     // 3 maxTxSize
        uint(1_100),      // 4 maxBHSize
        uint(2_000_000),  // 5 keyDeposit
        uint(500_000_000),// 6 poolDeposit
        uint(18),         // 7 eMax
        uint(500),        // 8 nOpt
        rational(3, 10),  // 9 a0
        rational(3, 1000),// 10 rho
        rational(1, 5),   // 11 tau
        concat(&[arr(2), uint(11), uint(0)]), // 12 protVer [major, minor]
        uint(170_000_000),// 13 minPoolCost
        uint(4_310),      // 14 coinsPerUTxOByte
        concat(&[map(0)]),// 15 costModels (empty map; raw-preserved)
        concat(&[arr(2), rational(577, 10_000), rational(721, 10_000_000)]), // 16 prices
        concat(&[arr(2), uint(16_500_000), uint(10_000_000_000)]), // 17 maxTxExUnits
        concat(&[arr(2), uint(72_000_000), uint(20_000_000_000)]), // 18 maxBlockExUnits
        uint(5_000),      // 19 maxValSize
        uint(150),        // 20 collateralPercentage
        uint(3),          // 21 maxCollateralInputs
        arr(0),           // 22 poolVotingThresholds (placeholder)
        arr(0),           // 23 drepVotingThresholds (placeholder)
        uint(3),          // 24 committeeMinSize
        uint(146),        // 25 committeeMaxTermLength
        uint(6),          // 26 govActionLifetime
        uint(1_000_000_000), // 27 govActionDeposit
        uint(500_000_000),// 28 dRepDeposit
        uint(20),         // 29 dRepActivity
        rational(15, 1),  // 30 minFeeRefScriptCostPerByte
    ])
}

/// Knobs for the fail-closed tests.
#[derive(Clone)]
struct Knobs {
    current_era_index: u64,
    pool_vrf: [u8; 32],
    pool_distr_vrf: [u8; 32],
    treasury: u64,
    reserves: u64,
    /// nesBprev entries (pool id -> blocks).
    block_production: Vec<([u8; 28], u64)>,
    /// The pool reward-account network byte (the snapshot's network-bound field; the
    /// `network_id` decoded from it is cross-checked against the manifest-derived id).
    reward_net: u8,
    /// Override the curPParams item entirely (for malformed/missing-PParams tests).
    pparams_override: Option<Vec<u8>>,
    /// Override the esAccountState item entirely.
    account_state_override: Option<Vec<u8>>,
    /// Override the govState arity by dropping it to fewer fields (for missing-PParams).
    gov_state_short: bool,
}

impl Default for Knobs {
    fn default() -> Self {
        let vrf = [0x55u8; 32];
        Knobs {
            current_era_index: 6,
            pool_vrf: vrf,
            pool_distr_vrf: vrf,
            treasury: 1_890_267_427_632_547,
            reserves: 13_051_749_596_873_397,
            block_production: vec![([0x11; 28], 50)],
            reward_net: 0, // testnet (matches the default TESTNET_MAGIC -> network_id 0)
            pparams_override: None,
            account_state_override: None,
            gov_state_short: false,
        }
    }
}

fn build_state(k: &Knobs) -> Vec<u8> {
    // PState = [map32B(empty), pools(1), future(empty), retiring(1)]
    let pstate = concat(&[
        arr(4),
        map(0),
        concat(&[map(1), bytes(&POOL_ID), pool_params(k.pool_vrf, k.reward_net)]),
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
    // CertState = [VState(empty), PState, DState]
    let cert = concat(&[arr(3), arr(0), pstate, dstate]);

    // UTxOState = [utxo(empty), deposited, fees, govState, incrStake, donation]
    let pparams = k.pparams_override.clone().unwrap_or_else(conway_pparams);
    let gov_state = if k.gov_state_short {
        // a 2-field govState (no curPParams at index 3) -> ProtocolParamsMissing
        concat(&[arr(2), arr(0), arr(0)])
    } else {
        // ConwayGovState = array(7)[Proposals, committee, constitution, curPParams, prev, future, drepPulser]
        concat(&[
            arr(7),
            arr(0),       // Proposals (placeholder)
            arr(0),       // committee
            arr(0),       // constitution
            pparams,      // curPParams
            arr(0),       // prevPParams
            arr(0),       // futurePParams
            arr(0),       // drepPulser
        ])
    };
    let utxo_state = concat(&[
        arr(6),
        map(0),                // utxo (empty — UTxO lives in `tables`)
        uint(1_159_004_000_000), // deposited
        uint(6_669_569_234),   // fees
        gov_state,
        map(0),                // incrStake (placeholder)
        uint(0),               // donation
    ]);

    // LedgerState = [CertState, UTxOState]
    let ls = concat(&[arr(2), cert, utxo_state]);
    // esAccountState = array(2)[treasury, reserves]
    let acct = k
        .account_state_override
        .clone()
        .unwrap_or_else(|| concat(&[arr(2), uint(k.treasury), uint(k.reserves)]));
    // EpochState = [acct, LedgerState, snaps, nonmyopic]
    // SnapShots = array(4)[ssStakeMark, ssStakeSet, ssStakeGo, ssFee]; each StakeSnapshot =
    // array(2)[ssStake: map(StakeCredential -> [Coin, PoolId]), ssPoolParams]. The decode reads the
    // FULL mark/set/go ssStake (here the mark carries one credential delegating COIN to POOL_ID; set/go
    // are valid-but-empty) and derives the seed+1 mark PoolDistr from the decoded mark.
    let mark = concat(&[
        arr(2),
        concat(&[
            map(1),
            concat(&[arr(2), uint(0), bytes(&[0x33; 28])]),
            concat(&[arr(2), uint(1_000_000), bytes(&POOL_ID)]),
        ]),
        map(0), // ssPoolParams (the VRF comes from the cert-state registration)
    ]);
    // ssStakeSet / ssStakeGo: valid-but-empty StakeSnapshots = array(2)[map(0) ssStake, map(0) ssPoolParams].
    let empty_snap = concat(&[arr(2), map(0), map(0)]);
    let snaps = concat(&[arr(4), mark, empty_snap.clone(), empty_snap, uint(0)]);
    let es = concat(&[arr(4), acct, ls, snaps, arr(0)]);

    // PoolDistr wrapper = [poolDistr_map(1), totalActiveStake]
    let pd = concat(&[
        map(1),
        bytes(&POOL_ID),
        concat(&[arr(3), uint(0), uint(100), bytes(&k.pool_distr_vrf)]),
    ]);
    let pdw = concat(&[arr(2), pd, uint(0)]);

    // nesBprev = map(pool -> blocks)
    let mut bprev = map(k.block_production.len() as u64);
    for (pid, blocks) in &k.block_production {
        bprev.extend(bytes(pid));
        bprev.extend(uint(*blocks));
    }

    // NES = [epoch, nesBprev, nesBcur, EpochState, rewardUpdate, poolDistrWrapper, stashed]
    let nes = concat(&[
        arr(7),
        uint(296),
        bprev,
        map(0), // nesBcur
        es,
        arr(0), // rewardUpdate
        pdw,
        vec![NULL],
    ]);
    // era live state = [tag(int), [dummy(array1), NES]]
    let inner2 = concat(&[arr(2), concat(&[arr(1), uint(0)]), nes]);
    let era_state = concat(&[arr(2), uint(2), inner2]);
    // telescope: current_era_index past eras [bound,bound] + current [bound, era_state]
    let mut tele = arr(k.current_era_index + 1);
    for _ in 0..k.current_era_index {
        tele.extend(concat(&[arr(2), bound(), bound()]));
    }
    tele.extend(concat(&[arr(2), bound(), era_state]));
    // headerState = [dummy, array(6) of the trailing PraosState nonces in record
    // order [evolving, candidate, epoch, previousEpoch, lab, lastEpochBlock]]
    let mut ns = arr(6);
    for kk in 0..6u8 {
        ns.extend(nonce(kk + 1));
    }
    let hs = concat(&[arr(2), uint(0), ns]);
    // ExtLedgerState = [telescope, headerState]; top = [version, ExtLedgerState]
    concat(&[arr(2), uint(1), concat(&[arr(2), tele, hs])])
}

fn point() -> SeedPoint {
    SeedPoint {
        slot: SlotNo(126_400_064),
        block_hash: Hash32([0xab; 32]),
    }
}

#[test]
fn happy_minimal_state_decodes_all_fields() {
    let st = build_state(&Knobs::default());
    let (s, _c) = decode_native_nonutxo_state(&st, point(), 296, TESTNET_MAGIC).expect("decode");
    assert_eq!(s.era, ade_types::CardanoEra::Conway);
    assert_eq!(s.epoch.0, 296);
    assert_eq!(s.point, point());
    assert_eq!(s.cert_state.pool.pools.len(), 1, "the FULL CertState carries the pool");
    assert_eq!(s.cert_state.delegation.delegations.len(), 1);
    assert_eq!(s.pool_distr.len(), 1);
    // Record order [evolving, candidate, epoch, previousEpoch, lab, lastEpochBlock] (tail[0..6]).
    // evolving=tail[0]; candidate=tail[1] + epoch=tail[2] + lastEpochBlock=tail[5] are value-proven
    // against the live node: eta0(N+1) = blake2b(candidate || lastEpochBlock), and the self-evolved
    // eta0(seed+2) matches the node's epochNonce (DC-EPOCH-16 live gate). previousEpoch (tail[3]) is
    // skipped (write-only bookkeeping).
    assert_eq!(s.praos_nonces.evolving.0, [1u8; 32]);
    assert_eq!(s.praos_nonces.candidate.0, [2u8; 32]);
    assert_eq!(s.praos_nonces.epoch.0, [3u8; 32]);
    assert_eq!(s.praos_nonces.lab.0, [5u8; 32]);
    assert_eq!(s.praos_nonces.last_epoch_block.0, [6u8; 32]);
    // protocol params decoded natively (not defaulted).
    assert_eq!(s.protocol_params.min_fee_a, Coin(44));
    assert_eq!(s.protocol_params.min_fee_b, Coin(155_381));
    assert_eq!(s.protocol_params.max_tx_size, 16_384);
    assert_eq!(s.protocol_params.key_deposit, Coin(2_000_000));
    assert_eq!(s.protocol_params.pool_deposit, Coin(500_000_000));
    assert_eq!(s.protocol_params.protocol_major, 11);
    assert_eq!(s.protocol_params.min_pool_cost, Coin(170_000_000));
    assert_eq!(s.protocol_params.collateral_percent, 150);
    assert_eq!(s.protocol_params.max_tx_ex_units_mem, 16_500_000);
    assert_eq!(s.protocol_params.max_tx_ex_units_cpu, 10_000_000_000);
    // coinsPerUTxOByte preserved faithfully as a PER-BYTE rule (NOT an absolute min).
    assert_eq!(
        s.protocol_params.min_utxo_rule,
        MinUtxoRule::PerByte(Coin(4_310))
    );
    // network id derived from the manifest magic (testnet -> 0), bound on the state and
    // on the protocol params.
    assert_eq!(s.network_id, 0);
    assert_eq!(s.protocol_params.network_id, 0);
    // pots.
    assert_eq!(s.treasury, Coin(1_890_267_427_632_547));
    assert_eq!(s.reserves, Coin(13_051_749_596_873_397));
    // block production (subset of CertState pools).
    assert_eq!(s.block_production.len(), 1);
    assert_eq!(s.block_production[&ade_types::tx::PoolId(ade_types::Hash28(POOL_ID))], 50);
}

#[test]
fn determinism_same_bytes_same_commitment() {
    let st = build_state(&Knobs::default());
    let (a, ca) = decode_native_nonutxo_state(&st, point(), 296, TESTNET_MAGIC).unwrap();
    let (b, cb) = decode_native_nonutxo_state(&st, point(), 296, TESTNET_MAGIC).unwrap();
    assert_eq!(a, b);
    assert_eq!(ca, cb, "deterministic commitment");
}

/// The commitment binds EVERY emitted field: perturb a field in the input (or the bound-through
/// point) and the commitment must change.
#[test]
fn commitment_binds_every_field() {
    let base = build_state(&Knobs::default());
    let (_s0, c0) = decode_native_nonutxo_state(&base, point(), 296, TESTNET_MAGIC).unwrap();

    // point (bound through, not decoded) — a different slot must change the commitment.
    let p2 = SeedPoint {
        slot: SlotNo(999),
        block_hash: point().block_hash,
    };
    let (_s, c) = decode_native_nonutxo_state(&base, p2, 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "point.slot is bound");
    let p3 = SeedPoint {
        slot: point().slot,
        block_hash: Hash32([0xcd; 32]),
    };
    let (_s, c) = decode_native_nonutxo_state(&base, p3, 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "point.block_hash is bound");

    // reserves.
    let mut k = Knobs::default();
    k.reserves += 1;
    let (_s, c) = decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "reserves is bound");

    // treasury.
    let mut k = Knobs::default();
    k.treasury += 1;
    let (_s, c) = decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "treasury is bound");

    // block production count.
    let mut k = Knobs::default();
    k.block_production = vec![([0x11; 28], 51)];
    let (_s, c) = decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "block_production is bound");

    // pool distribution VRF (also kept consistent with the pool's VRF so it stays valid).
    let mut k = Knobs::default();
    k.pool_vrf = [0x77; 32];
    k.pool_distr_vrf = [0x77; 32];
    let (_s, c) = decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "pool VRF (cert_state + pool_distr) is bound");

    // protocol params (minFeeA): a different value must change the commitment.
    let mut k = Knobs::default();
    k.pparams_override = Some(conway_pparams_with_min_fee_a(45));
    let (_s, c) = decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "protocol_params is bound");

    // network id (derived from the manifest magic): a mainnet manifest yields network_id 1
    // (with a matching mainnet reward-account net byte so the decode stays valid).
    let mut k = Knobs::default();
    k.reward_net = 1;
    let (_s, c) = decode_native_nonutxo_state(&build_state(&k), point(), 296, MAINNET_MAGIC).unwrap();
    assert_ne!(c, c0, "network_id is bound");

    // a nonce (perturbing the input bytes by rebuilding with a different trailing nonce is
    // covered implicitly; here we confirm a nonce difference via a direct byte edit on a copy).
    // The fifth nonce body sits at the very tail (last 32 bytes); flip one byte.
    let mut alt = base.clone();
    let n = alt.len();
    alt[n - 1] ^= 0x01;
    let (_s, c) = decode_native_nonutxo_state(&alt, point(), 296, TESTNET_MAGIC).unwrap();
    assert_ne!(c, c0, "praos nonces are bound");
}

#[test]
fn no_cli_or_seed_inputs_only_manifest_args() {
    // Compile-time witness: the entry point takes only the manifest-authoritative inputs
    // (state_cbor, point, manifest_epoch, manifest_network_magic) — no CLI / JSON bundle / seed.
    let st = build_state(&Knobs::default());
    let _ = decode_native_nonutxo_state(&st, point(), 296, TESTNET_MAGIC).unwrap();
}

/// The network id is DERIVED from the manifest network magic, not a placeholder: the
/// mainnet magic yields `network_id = 1`; a testnet magic yields `0`. It is bound on the
/// state AND on the protocol params (the same derived value).
#[test]
fn network_id_derived_from_manifest_magic() {
    // testnet magic -> network_id 0 (fixture reward-account net byte is 0).
    let st0 = build_state(&Knobs::default());
    let (s0, _c0) = decode_native_nonutxo_state(&st0, point(), 296, TESTNET_MAGIC).unwrap();
    assert_eq!(s0.network_id, 0, "testnet magic derives network_id 0");
    assert_eq!(s0.protocol_params.network_id, 0, "pp.network_id == derived");

    // mainnet magic -> network_id 1 (with a matching mainnet reward-account net byte).
    let mut k = Knobs::default();
    k.reward_net = 1;
    let st1 = build_state(&k);
    let (s1, _c1) = decode_native_nonutxo_state(&st1, point(), 296, MAINNET_MAGIC).unwrap();
    assert_eq!(s1.network_id, 1, "mainnet magic derives network_id 1");
    assert_eq!(s1.protocol_params.network_id, 1, "pp.network_id == derived");
}

/// Conway `coinsPerUTxOByte` (4310 in the fixture) is preserved faithfully as a PER-BYTE
/// rule — NOT remapped onto the absolute-floor `LegacyAbsoluteMin`.
#[test]
fn conway_pparams_decode_yields_per_byte_min_utxo_rule() {
    let st = build_state(&Knobs::default());
    let (s, _c) = decode_native_nonutxo_state(&st, point(), 296, TESTNET_MAGIC).unwrap();
    assert_eq!(
        s.protocol_params.min_utxo_rule,
        MinUtxoRule::PerByte(Coin(4_310)),
        "coinsPerUTxOByte must be PerByte, not an absolute floor"
    );
    assert!(
        !matches!(
            s.protocol_params.min_utxo_rule,
            MinUtxoRule::LegacyAbsoluteMin(_)
        ),
        "the Conway per-byte rule must NOT populate LegacyAbsoluteMin"
    );
}

// ---- fail-closed negatives ----

#[test]
fn wrong_era_is_terminal() {
    let mut k = Knobs::default();
    k.current_era_index = 5; // Babbage
    assert!(matches!(
        decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::UnsupportedEra { current_index: 5 })
    ));
}

#[test]
fn epoch_mismatch_is_terminal() {
    let st = build_state(&Knobs::default());
    assert!(matches!(
        decode_native_nonutxo_state(&st, point(), 9999, TESTNET_MAGIC),
        Err(NativeNonUtxoError::EpochMismatch {
            decoded_epoch: 296,
            manifest_epoch: 9999
        })
    ));
}

#[test]
fn zero_vrf_is_terminal() {
    let mut k = Knobs::default();
    k.pool_vrf = [0u8; 32];
    k.pool_distr_vrf = [0u8; 32];
    assert!(matches!(
        decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::ZeroVrf(_))
    ));
}

#[test]
fn pool_distr_vrf_mismatch_is_terminal() {
    let mut k = Knobs::default();
    k.pool_vrf = [0x55; 32];
    k.pool_distr_vrf = [0x66; 32];
    assert!(matches!(
        decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::PoolDistrVrfMismatch(_))
    ));
}

#[test]
fn missing_pparams_is_terminal_no_default() {
    let mut k = Knobs::default();
    k.gov_state_short = true; // govState has no curPParams
    assert!(matches!(
        decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::ProtocolParamsMissing(_))
    ));
}

#[test]
fn malformed_pparams_arity_is_terminal() {
    let mut k = Knobs::default();
    // a 5-field PParams array (not Conway's 31) -> ProtocolParamsMissing
    k.pparams_override = Some(concat(&[arr(5), uint(1), uint(2), uint(3), uint(4), uint(5)]));
    assert!(matches!(
        decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::ProtocolParamsMissing(_))
    ));
}

#[test]
fn malformed_account_state_is_terminal() {
    let mut k = Knobs::default();
    // a 3-field accountState (not array(2)) -> AccountStateMissing
    k.account_state_override = Some(concat(&[arr(3), uint(1), uint(2), uint(3)]));
    assert!(matches!(
        decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::AccountStateMissing(_))
    ));
}

#[test]
fn block_production_unknown_pool_is_terminal() {
    let mut k = Knobs::default();
    // a producer not in the CertState pool set
    k.block_production = vec![(OTHER_POOL_ID, 5)];
    assert!(matches!(
        decode_native_nonutxo_state(&build_state(&k), point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::BlockProductionUnknownPool(_))
    ));
}

#[test]
fn malformed_cbor_is_terminal() {
    assert!(matches!(
        decode_native_nonutxo_state(&[0x00, 0x01, 0x02], point(), 296, TESTNET_MAGIC),
        Err(NativeNonUtxoError::MalformedCbor(_))
    ));
}

/// A pool reward-account net byte (testnet 0) disagreeing with the manifest-derived id
/// (mainnet magic -> 1) is DIAGNOSTIC ONLY — the decode SUCCEEDS (network_id is bound from the
/// manifest authority, not the operator-controlled nibble) and records the disagreement as a
/// `Uniform` observation. The manifest is the sole network authority; the nibble never rejects.
#[test]
fn reward_nibble_disagreement_is_diagnostic_not_terminal() {
    // default fixture reward_net is 0 (testnet); pass the mainnet magic (derives network_id 1).
    let st = build_state(&Knobs::default());
    let (s, _c) = decode_native_nonutxo_state(&st, point(), 296, MAINNET_MAGIC)
        .expect("a reward-nibble disagreement must NOT reject — the manifest is the authority");
    assert_eq!(s.network_id, 1, "network_id is bound from the manifest magic, not the nibble");
    assert_eq!(
        s.reward_nibble_observation,
        ade_ledger::ledgerdb_state::RewardNibbleObservation::Uniform(0),
        "the disagreeing nibble is recorded as diagnostic evidence, never a verdict"
    );
}
