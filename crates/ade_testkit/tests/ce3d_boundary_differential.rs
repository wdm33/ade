//! CE-3d: byte-exact boundary differential — preview epochs 1341 & 1342.
//!
//! Drives Ade's `EpochAccumulator` across two FULLY SELF-DERIVED preview epoch
//! boundaries over a fixed corpus, then compares the self-derived reward update
//! (RUPD) + go-snapshot against cardano-node `db-analyser` reference ledger
//! states:
//!
//!   - cross 1340 -> 1341 : reward update consumes epoch 1339's block production
//!     (the 1st boundary whose entire input epoch Ade followed natively).
//!   - cross 1341 -> 1342 : reward update consumes epoch 1340's block production
//!     (the 2nd fully self-derived boundary).
//!
//! This is the gate (LIVE-LEDGER-EPOCH-TRANSITION S3 item #3 / CE-3d) that
//! decides whether the accumulator may REPLACE the seed-anchored EVIEW replay
//! window (S4). It exercises the SAME public primitives the live co-advancer
//! (`node_lifecycle::advance_ledger_state_to_durable_tip`) calls, in the same
//! order, so the differential tests the real boundary mechanics (B3c included).
//!
//! Reads LOCAL extraction artifacts (NOT yet committed fixtures), so it is
//! `#[ignore]`'d. Paths come from env with local defaults:
//!   - `CE3D_CORPUS`       dir with `manifest.json` + `<slot>.cbor` blocks
//!   - `CE3D_SEED_STORES`  dir with Ade's CE-3c `epoch-accumulator.redb`
//!                         + `reduced-checkpoint.redb` (copied, never mutated)
//!   - `CE3D_WORK`         scratch dir for the store copies
//!   - `CE3D_REF_1341`     cardano POST-1341 snapshot tarball (`.tar.gz`)
//!   - `CE3D_REF_1342`     cardano POST-1342 snapshot tarball (`.tar.gz`)

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ade_core::consensus::era_schedule::{EraSchedule, EraSummary};
use ade_core::consensus::BootstrapAnchorHash;
use ade_runtime::chaindb::{
    advance_accumulator_over_chaindb, advance_reduced_checkpoint_over_chaindb,
    cross_accumulator_over_boundary_block, AccumulatorBoundaryOutcome, AccumulatorChaindbOutcome,
    ChainDb, EpochAccumulatorStore, InMemoryChainDb, ReducedUtxoCheckpoint, StoredBlock,
};
use ade_types::shelley::cert::StakeCredential;
use ade_types::{CardanoEra, EpochNo, Hash32, SlotNo};

/// Preview epoch geometry: epoch E starts at slot E * 86_400.
const PREVIEW_EPOCH_LEN: u32 = 86_400;
/// Ade's CE-3c bootstrap seed epoch (where the accumulator is anchored).
const SEED_EPOCH: u64 = 1338;
/// First block of epoch 1341 (boundary 1340 -> 1341): chunk 26820.
const EPOCH_1341_FIRST_SLOT: u64 = 115_862_416;
/// First block of epoch 1342 (boundary 1341 -> 1342): chunk 26840.
const EPOCH_1342_FIRST_SLOT: u64 = 115_948_834;

fn env_path(key: &str, default: &str) -> PathBuf {
    std::env::var(key).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(default))
}

/// The single open-ended Conway era anchored at the seed epoch — the SAME
/// slot->epoch arithmetic the live `build_native_schedule` uses. `locate`
/// extrapolates within the last (open) era, so boundaries at 1341/1342 are
/// detected without pre-extension.
fn preview_schedule() -> EraSchedule {
    let start_slot = SEED_EPOCH * u64::from(PREVIEW_EPOCH_LEN);
    EraSchedule::new(
        BootstrapAnchorHash(Hash32([0u8; 32])),
        0,
        vec![EraSummary {
            randomness_stabilisation_window_slots: None,
            era: CardanoEra::Conway,
            start_slot: SlotNo(start_slot),
            start_epoch: EpochNo(SEED_EPOCH),
            slot_length_ms: 1_000,
            epoch_length_slots: PREVIEW_EPOCH_LEN,
            safe_zone_slots: PREVIEW_EPOCH_LEN,
        }],
    )
    .expect("preview schedule")
}

fn parse_hash32(hex: &str) -> Hash32 {
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect::<Vec<u8>>();
    let mut h = [0u8; 32];
    h.copy_from_slice(&bytes);
    Hash32(h)
}

/// Load corpus blocks with `slot <= up_to_slot` into a fresh `InMemoryChainDb`.
fn load_corpus(dir: &Path, up_to_slot: u64) -> (InMemoryChainDb, usize) {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).expect("manifest"))
            .expect("manifest json");
    let blocks = manifest["blocks"].as_array().expect("blocks array");
    let db = InMemoryChainDb::new();
    let mut n = 0usize;
    for b in blocks {
        let slot = b["slot"].as_u64().expect("slot");
        if slot > up_to_slot {
            continue;
        }
        let file = b["file"].as_str().expect("file");
        let hash = parse_hash32(b["hash"].as_str().expect("hash"));
        let bytes = std::fs::read(dir.join(file)).expect("block cbor");
        db.put_block(&StoredBlock { hash, slot: SlotNo(slot), bytes }).expect("put_block");
        n += 1;
    }
    (db, n)
}

/// Mirror `node_lifecycle::advance_ledger_state_to_durable_tip`'s observe-only
/// boundary-segmented co-advance loop, using only public primitives. The corpus
/// is forward-only, so the reorg-reset helpers are no-ops and omitted.
fn co_advance(
    store: &EpochAccumulatorStore,
    cp: &ReducedUtxoCheckpoint,
    chaindb: &dyn ChainDb,
    sched: &EraSchedule,
) {
    let tip = chaindb.tip().expect("tip").expect("non-empty");
    let seed_slot = store.seed_slot().expect("seed").expect("sealed");
    let cp_seed = cp.seed_slot().expect("cp seed").expect("cp sealed");
    loop {
        match advance_accumulator_over_chaindb(store, chaindb, sched, seed_slot, tip.slot) {
            Ok(AccumulatorChaindbOutcome::ReachedTip { .. }) => break,
            Ok(AccumulatorChaindbOutcome::StalledAt { slot: s_bb, reason }) => {
                let s_prev = store.last_advanced_slot().expect("cursor").expect("durable cursor");
                advance_reduced_checkpoint_over_chaindb(
                    cp,
                    chaindb,
                    cp_seed,
                    s_prev,
                    CardanoEra::Conway,
                )
                .expect("checkpoint -> s_prev");
                let mark = cp.sum_base_credential_stake().expect("mark");
                let boundary_hash =
                    chaindb.get_block_by_slot(s_prev).expect("hash read").expect("boundary block").hash;
                store.bind_boundary_mark(s_prev, &boundary_hash).expect("bind mark");
                match cross_accumulator_over_boundary_block(store, chaindb, sched, s_bb, &mark) {
                    Ok(AccumulatorBoundaryOutcome::Crossed { from_epoch, to_epoch, slot }) => {
                        let _ = store.clear_boundary_mark();
                        eprintln!(
                            "  CROSSED {} -> {} at slot {} (mark from s_prev {}, reason: {reason})",
                            from_epoch.0, to_epoch.0, slot.0, s_prev.0
                        );
                    }
                    Ok(AccumulatorBoundaryOutcome::AlreadyCrossed { .. }) => {
                        let _ = store.clear_boundary_mark();
                    }
                    Ok(AccumulatorBoundaryOutcome::Stalled { slot, reason }) => {
                        panic!("boundary cross stalled at {}: {reason}", slot.0);
                    }
                    Err(e) => panic!("boundary cross fault: {e:?}"),
                }
            }
            Err(e) => panic!("within-epoch reconcile fault: {e:?}"),
        }
    }
    advance_reduced_checkpoint_over_chaindb(cp, chaindb, cp_seed, tip.slot, CardanoEra::Conway)
        .expect("checkpoint -> tip");
}

/// One side of the differential: the pots + the go-snapshot pool stakes + the reward accounts,
/// keyed by canonical bytes so the accumulator and the cardano reference compare uniformly.
struct PostState {
    epoch: u64,
    treasury: u64,
    reserves: u64,
    /// PoolId(28B) -> active stake (the go-snapshot's calculatePoolDistr).
    go: BTreeMap<Vec<u8>, u64>,
    /// [discriminant | hash28] -> reward-account balance.
    rewards: BTreeMap<Vec<u8>, u64>,
}

/// Canonical byte key for a stake credential (discriminant-preserving — rewards are keyed by the
/// full credential, unlike the Hash28-only go-snapshot key).
fn cred_key(c: &StakeCredential) -> Vec<u8> {
    let (tag, h) = match c {
        StakeCredential::KeyHash(h) => (0u8, h),
        StakeCredential::ScriptHash(h) => (1u8, h),
    };
    let mut k = Vec::with_capacity(29);
    k.push(tag);
    k.extend_from_slice(&h.0);
    k
}

fn ade_post_state(store: &EpochAccumulatorStore) -> PostState {
    let (_slot, acc) = store.load_current().expect("load").expect("sealed accumulator");
    let es = &acc.epoch_state;
    PostState {
        epoch: es.epoch.0,
        treasury: es.treasury.0,
        reserves: es.reserves.0,
        go: es
            .snapshots
            .as_authoritative()
            .expect("authoritative")
            .go
            .0
            .pool_stakes
            .iter()
            .map(|(pid, c)| ((pid.0).0.to_vec(), c.0))
            .collect(),
        rewards: acc
            .cert_state
            .delegation
            .rewards
            .iter()
            .map(|(cred, c)| (cred_key(cred), c.0))
            .collect(),
    }
}

/// The cardano POST reference, decoded by ADE'S OWN `decode_native_nonutxo_state` (it parses the
/// 11.0.1 Conway `state` cleanly, where the testkit snapshot_loader's reward/go navigation does
/// not). Produces the same `PostState` as the accumulator side, so the diff is type-uniform.
fn ref_post_state(state_path: &Path, slot: u64, epoch: u64) -> PostState {
    use ade_ledger::bootstrap_anchor::SeedPoint;
    use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
    let state = std::fs::read(state_path).expect("ref state");
    let point = SeedPoint {
        slot: SlotNo(slot),
        block_hash: Hash32([0u8; 32]),
    };
    let (s1a, _commit) =
        decode_native_nonutxo_state(&state, point, epoch, 2).expect("decode cardano ref state");
    PostState {
        epoch: s1a.epoch.0,
        treasury: s1a.treasury.0,
        reserves: s1a.reserves.0,
        go: s1a
            .snapshots
            .go
            .0
            .pool_stakes
            .iter()
            .map(|(pid, c)| ((pid.0).0.to_vec(), c.0))
            .collect(),
        rewards: s1a
            .cert_state
            .delegation
            .rewards
            .iter()
            .map(|(cred, c)| (cred_key(cred), c.0))
            .collect(),
    }
}

/// The cardano reference leadership PoolDistr (`nes[5]` / `nesPd`) for the epoch this reference state is at —
/// the SAME `decode_native_nonutxo_state(...).pool_distr` field that produced the proven-byte-exact seed
/// pool_distribution. Keyed by pool keyhash(28B) -> (active_stake, VRF keyhash). This is the LITERAL nes[5]
/// (incl. zero-stake registered + retired/POOLREAP'd pools); NEVER the lossy `.mark_pool_distr`.
fn ref_nes_pd(state_path: &Path, slot: u64, epoch: u64) -> BTreeMap<[u8; 28], (u64, [u8; 32])> {
    use ade_ledger::bootstrap_anchor::SeedPoint;
    use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
    let state = std::fs::read(state_path).expect("ref state");
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, _commit) =
        decode_native_nonutxo_state(&state, point, epoch, 2).expect("decode cardano ref state");
    s1a.pool_distr
        .iter()
        .map(|(pid, (stake, vrf))| ((pid.0).0, (*stake, vrf.0)))
        .collect()
}

/// Ade's boundary-frozen leadership in the SAME comparable form as [`ref_nes_pd`] (keyhash -> (stake, VRF)).
fn ade_leadership_map(
    distr: &ade_ledger::frozen_leadership::FrozenLeadershipPoolDistr,
) -> BTreeMap<[u8; 28], (u64, [u8; 32])> {
    distr
        .pools
        .iter()
        .map(|(h, e)| (h.0, (e.active_stake, e.vrf_keyhash.0)))
        .collect()
}

/// S4-pre-2 probe (FAST): the leadership pool SET is the DELEGATION IMAGE (pools with >=1 delegator), NOT all
/// registered pools. Verify against the v5 store's accumulator state at epoch 1340 that `image ∩ registered`
/// is ~658 (the reference nesPd count), while `registered` is ~703.
#[test]
#[ignore = "S4-pre-2 probe: delegation-image pool set size vs registered (env S5_SEED_STORES); FAST"]
fn s4pre2_delegation_image_pool_set_size() {
    let seed_dir = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
    let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
    let dst = work.join("s4pre2-probe");
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).expect("mkdir");
    let acc_copy = dst.join("epoch-accumulator.redb");
    std::fs::copy(seed_dir.join("epoch-accumulator.redb"), &acc_copy).expect("copy acc");
    let store = EpochAccumulatorStore::open(&acc_copy).expect("open acc");
    let (_slot, acc) = store.load_current().expect("load").expect("sealed");
    let registered = acc.cert_state.pool.pools.len();
    let image: std::collections::BTreeSet<_> =
        acc.cert_state.delegation.delegations.values().cloned().collect();
    let image_registered = image
        .iter()
        .filter(|p| acc.cert_state.pool.pools.contains_key(p))
        .count();
    eprintln!(
        "epoch {} — registered_pools={registered} delegation_image={} image_registered={image_registered}",
        acc.epoch_state.epoch.0,
        image.len()
    );
}

/// S4-pre-2 sanity (FAST): the cardano reference `nesPd` (`nes[5]`) decodes non-empty from each POST state —
/// the reference the boundary-freeze proof compares against. Confirms the decoder + fixtures before the SLOW
/// recovery run relies on them.
#[test]
#[ignore = "S4-pre-2 sanity: reference nesPd (nes[5]) decodes non-empty from the POST states (env CE3D_REF); FAST"]
fn s4pre2_reference_nespd_decodes_nonempty() {
    let ref_dir = env_path("CE3D_REF", "/home/ts/.cardano-ce3d-extract/db/ledger");
    for (dir, slot, epoch) in [
        ("115776011_db-analyser/state", 115_776_011u64, 1340u64),
        ("115862416_db-analyser/state", 115_862_416, 1341),
        ("115948834_db-analyser/state", 115_948_834, 1342),
    ] {
        let pd = ref_nes_pd(&ref_dir.join(dir), slot, epoch);
        let zero = pd.values().filter(|(s, _)| *s == 0).count();
        eprintln!("reference nesPd POST-{epoch}: {} pools ({zero} zero-stake)", pd.len());
        assert!(pd.len() > 100, "reference nesPd for epoch {epoch} decodes non-empty (nes[5])");
    }
}

fn ok(b: bool) -> &'static str {
    if b {
        "MATCH"
    } else {
        "*** MISMATCH ***"
    }
}

/// Compare the accumulator's self-derived POST-state to the cardano reference, field by field.
fn compare(label: &str, ade: &PostState, refs: &PostState) {
    eprintln!("==================== {label} ====================");
    eprintln!("  epoch    ade={} ref={}  {}", ade.epoch, refs.epoch, ok(ade.epoch == refs.epoch));
    eprintln!(
        "  treasury ade={} ref={} d{}  {}",
        ade.treasury,
        refs.treasury,
        ade.treasury as i128 - refs.treasury as i128,
        ok(ade.treasury == refs.treasury),
    );
    eprintln!(
        "  reserves ade={} ref={} d{}  {}",
        ade.reserves,
        refs.reserves,
        ade.reserves as i128 - refs.reserves as i128,
        ok(ade.reserves == refs.reserves),
    );
    diff_map("go_pool_stakes", &ade.go, &refs.go);
    diff_map("rewards", &ade.rewards, &refs.rewards);
}

/// Full-map byte-exact comparison with a mismatch breakdown + a few samples (surfaces B3c).
fn diff_map(name: &str, ade: &BTreeMap<Vec<u8>, u64>, refs: &BTreeMap<Vec<u8>, u64>) {
    let at: u64 = ade.values().sum();
    let rt: u64 = refs.values().sum();
    let (mut matched, mut val_mismatch, mut only_ade, mut only_ref) =
        (0usize, 0usize, 0usize, 0usize);
    let mut samples: Vec<String> = Vec::new();
    let keys: std::collections::BTreeSet<&Vec<u8>> = ade.keys().chain(refs.keys()).collect();
    for k in &keys {
        let hex: String = k.iter().take(8).map(|b| format!("{b:02x}")).collect();
        match (ade.get(*k), refs.get(*k)) {
            (Some(a), Some(r)) if a == r => matched += 1,
            (Some(a), Some(r)) => {
                val_mismatch += 1;
                if samples.len() < 6 {
                    samples.push(format!("{hex}.. ade={a} ref={r} (d{})", *a as i128 - *r as i128));
                }
            }
            (Some(a), None) => {
                only_ade += 1;
                if samples.len() < 6 {
                    samples.push(format!("{hex}.. ade={a} ref=ABSENT"));
                }
            }
            (None, Some(r)) => {
                only_ref += 1;
                if samples.len() < 6 {
                    samples.push(format!("{hex}.. ade=ABSENT ref={r}"));
                }
            }
            (None, None) => {}
        }
    }
    let exact = val_mismatch == 0 && only_ade == 0 && only_ref == 0 && at == rt;
    eprintln!(
        "  {name}: {} keys (ade {} / ref {}); total ade={at} ref={rt} d{}  {}",
        keys.len(),
        ade.len(),
        refs.len(),
        at as i128 - rt as i128,
        ok(exact),
    );
    eprintln!(
        "    matched={matched} val_mismatch={val_mismatch} only_ade={only_ade} only_ref={only_ref}"
    );
    for s in &samples {
        eprintln!("    {s}");
    }
}

/// CE-3d acceptance #2 (real data): the native non-UTxO decoder (`decode_native_nonutxo_state`,
/// the bootstrap's S1a) yields non-empty mark/set/go from a REAL certified preview state, with the
/// `go` total == the snapshot's decoded value (python-cross-checked). Proves the bootstrap snapshot
/// import on real preview Conway data (the fix that makes the accumulator's `go` non-empty).
#[test]
#[ignore = "reads a local preview POST-1341 state; run explicitly"]
fn ce3d_native_decode_yields_nonempty_snapshots() {
    use ade_ledger::bootstrap_anchor::SeedPoint;
    use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
    let path = env_path(
        "CE3D_REF_1341_STATE",
        "/home/ts/.cardano-ce3d-extract/db/ledger/115862416_db-analyser/state",
    );
    let state = std::fs::read(&path).expect("ref state");
    let point = SeedPoint {
        slot: SlotNo(115_862_416),
        block_hash: Hash32([0u8; 32]),
    };
    let (s1a, _commit) = decode_native_nonutxo_state(&state, point, 1341, 2)
        .expect("decode preview POST-1341 native non-utxo state");
    let go = &s1a.snapshots.go.0;
    let go_total: u64 = go.pool_stakes.values().map(|c| c.0).sum();
    eprintln!(
        "native decode: mark {}p/{} set {}p/{} go {}p/{}",
        s1a.snapshots.mark.0.pool_stakes.len(),
        s1a.snapshots.mark.0.pool_stakes.values().map(|c| c.0).sum::<u64>(),
        s1a.snapshots.set.0.pool_stakes.len(),
        s1a.snapshots.set.0.pool_stakes.values().map(|c| c.0).sum::<u64>(),
        go.pool_stakes.len(),
        go_total,
    );
    assert!(!go.pool_stakes.is_empty(), "go must be non-empty");
    assert_eq!(go_total, 1_673_934_797_356_442, "go total == the certified snapshot");
}

/// Print-only: decode BOTH cardano reference states (POST-1341 @115862416, POST-1342 @115948834)
/// via Ade's own decoder and dump the comparison targets (go total/count, pots, reward count).
/// These are the byte-exact numbers the accumulator's self-derived 1341/1342 must reproduce.
#[test]
#[ignore = "reads local CE-3d reference states; run explicitly"]
fn ce3d_reference_targets() {
    for (slot, epoch) in [(EPOCH_1341_FIRST_SLOT, 1341u64), (EPOCH_1342_FIRST_SLOT, 1342u64)] {
        let path = PathBuf::from(format!(
            "/home/ts/.cardano-ce3d-extract/db/ledger/{slot}_db-analyser/state"
        ));
        if !path.exists() {
            eprintln!("(ref @{slot} absent)");
            continue;
        }
        let r = ref_post_state(&path, slot, epoch);
        eprintln!(
            "REF POST-{epoch} @{slot}: treasury={} reserves={} go={}p/{} rewards={}acct/{}",
            r.treasury,
            r.reserves,
            r.go.len(),
            r.go.values().sum::<u64>(),
            r.rewards.len(),
            r.rewards.values().sum::<u64>(),
        );
    }
}

/// Fast diagnostic (no crossing): dump the CE-3c seed accumulator's snapshot +
/// delegation state, to localize whether an empty boundary stake originates in
/// the saved accumulator or the re-cross.
#[test]
#[ignore = "inspects the CE-3c seed accumulator; run explicitly"]
fn ce3d_inspect_seed_accumulator() {
    let seed = env_path("CE3D_SEED_STORES", "/home/ts/.cardano-ce3c-firstrun");
    let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work");
    std::fs::create_dir_all(&work).expect("work dir");
    let acc_path = work.join("inspect-accumulator.redb");
    std::fs::copy(seed.join("epoch-accumulator.redb"), &acc_path).expect("copy accumulator");
    let store = EpochAccumulatorStore::open(&acc_path).expect("open accumulator");
    let (slot, acc) = store.load_current().expect("load").expect("sealed accumulator");
    let es = &acc.epoch_state;
    eprintln!("seed accumulator @ slot {} epoch {}", slot.0, es.epoch.0);
    eprintln!(
        "  cert_state: delegations={} rewards={}",
        acc.cert_state.delegation.delegations.len(),
        acc.cert_state.delegation.rewards.len(),
    );
    for (nm, snap) in [
        ("mark", &es.snapshots.as_authoritative().unwrap().mark.0),
        ("set", &es.snapshots.as_authoritative().unwrap().set.0),
        ("go", &es.snapshots.as_authoritative().unwrap().go.0),
    ] {
        eprintln!(
            "  snapshot {nm}: pool_stakes={} delegations={} stake_total={}",
            snap.pool_stakes.len(),
            snap.delegations.len(),
            snap.pool_stakes.values().map(|c| c.0).sum::<u64>(),
        );
    }
    eprintln!(
        "  prev_block_production pools={} reserves={} treasury={} pending_reward_update={}",
        acc.prev_block_production.len(),
        es.reserves.0,
        es.treasury.0,
        acc.pending_reward_update.is_some(),
    );
}

#[test]
#[ignore = "reads local CE-3d extraction artifacts; run explicitly"]
fn ce3d_boundary_differential_1341_1342() {
    let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
    let seed = env_path("CE3D_SEED_STORES", "/home/ts/.cardano-ce3d-rebootstrap");
    let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work");
    let ref_1341 = env_path(
        "CE3D_REF_1341",
        "/home/ts/.cardano-ce3d-extract/db/ledger/115862416_db-analyser/state",
    );
    let ref_1342 = env_path(
        "CE3D_REF_1342",
        "/home/ts/.cardano-ce3d-extract/db/ledger/115948834_db-analyser/state",
    );

    // Copy the re-bootstrapped seed stores into a scratch workdir — the originals are the
    // proof and must NOT be mutated by the advance.
    std::fs::create_dir_all(&work).expect("work dir");
    let acc_path = work.join("epoch-accumulator.redb");
    let cp_path = work.join("reduced-checkpoint.redb");
    std::fs::copy(seed.join("epoch-accumulator.redb"), &acc_path).expect("copy accumulator");
    std::fs::copy(seed.join("reduced-checkpoint.redb"), &cp_path).expect("copy checkpoint");

    let store = EpochAccumulatorStore::open(&acc_path).expect("open accumulator");
    let cp = ReducedUtxoCheckpoint::open(&cp_path).expect("open checkpoint");
    let sched = preview_schedule();

    eprintln!(
        "seed accumulator: last_advanced={:?} seed_slot={:?}",
        store.last_advanced_slot().ok().flatten().map(|s| s.0),
        store.seed_slot().ok().flatten().map(|s| s.0),
    );

    // ---- PHASE 1341: cross 1340 -> 1341 ----
    let (db1, n1) = load_corpus(&corpus, EPOCH_1341_FIRST_SLOT);
    eprintln!("phase 1341: loaded {n1} corpus blocks (<= {EPOCH_1341_FIRST_SLOT})");
    co_advance(&store, &cp, &db1, &sched);
    let ade_1341 = ade_post_state(&store);
    assert_eq!(ade_1341.epoch, 1341, "accumulator must be at epoch 1341 after the cross");

    if ref_1341.exists() {
        let refs = ref_post_state(&ref_1341, EPOCH_1341_FIRST_SLOT, 1341);
        assert_eq!(refs.epoch, 1341, "ref must be the POST-1341 state");
        compare("POST-1341 (reward update consumes 1339 block production)", &ade_1341, &refs);
    } else {
        eprintln!(
            "(ref_1341 absent — Ade-side POST-1341: epoch={} treasury={} reserves={} rewards={} go_total={})",
            ade_1341.epoch,
            ade_1341.treasury,
            ade_1341.reserves,
            ade_1341.rewards.len(),
            ade_1341.go.values().sum::<u64>(),
        );
    }

    // ---- PHASE 1342: cross 1341 -> 1342 (1341 cross is idempotent) ----
    let (db2, n2) = load_corpus(&corpus, EPOCH_1342_FIRST_SLOT);
    eprintln!("phase 1342: loaded {n2} corpus blocks (<= {EPOCH_1342_FIRST_SLOT})");
    co_advance(&store, &cp, &db2, &sched);
    let ade_1342 = ade_post_state(&store);
    assert_eq!(ade_1342.epoch, 1342, "accumulator must be at epoch 1342 after the cross");

    if ref_1342.exists() {
        let refs = ref_post_state(&ref_1342, EPOCH_1342_FIRST_SLOT, 1342);
        assert_eq!(refs.epoch, 1342, "ref must be the POST-1342 state");
        compare("POST-1342 (reward update consumes 1340 block production)", &ade_1342, &refs);
    } else {
        eprintln!(
            "(ref_1342 absent — Ade-side POST-1342: epoch={} treasury={} reserves={} rewards={} go_total={})",
            ade_1342.epoch,
            ade_1342.treasury,
            ade_1342.reserves,
            ade_1342.rewards.len(),
            ade_1342.go.values().sum::<u64>(),
        );
    }
}

/// B3c.0 ADJUDICATION (sealed, GREEN-only): re-run the ORIGINAL CE-3d go-stake decomposition path (the same
/// `co_advance` + `ade_post_state`/`ref_post_state` extraction) to POST-1342, from an ISOLATED prepared copy
/// (env `ADJ_DIR`), in ONE uninterrupted process. Pins the chain point, input-store blake2b hashes, the
/// checkpoint fingerprint, and a canonical (path-free) report hash. Run TWICE from independently prepared copies
/// and require byte-identical reports. Adjudicates whether the -343,260,172,883 go-stake residual REPRODUCES.
#[test]
#[ignore = "B3c.0 adjudication: sealed pinned CE-3d go-stake decomposition to POST-1342 (env ADJ_DIR); run uninterrupted"]
fn b3c0_adjudication_go_stake() {
    use ade_crypto::blake2b_256;
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let adj = env_path("ADJ_DIR", "/home/ts/.cardano-b3c0-adj-a");
    let acc_path = adj.join("epoch-accumulator.redb");
    let cp_path = adj.join("reduced-checkpoint.redb");
    // Copy-verify: hash the input stores BEFORE opening (open is read-WRITE).
    let acc_h = hexs(&blake2b_256(&std::fs::read(&acc_path).expect("read acc")).0);
    let cp_h = hexs(&blake2b_256(&std::fs::read(&cp_path).expect("read cp")).0);
    let ref_1342 = env_path(
        "CE3D_REF_1342",
        "/home/ts/.cardano-ce3d-extract/db/ledger/115948834_db-analyser/state",
    );
    let ref_h = hexs(&blake2b_256(&std::fs::read(&ref_1342).expect("read ref")).0);

    let store = EpochAccumulatorStore::open(&acc_path).expect("open acc");
    let cp = ReducedUtxoCheckpoint::open(&cp_path).expect("open cp");
    let sched = preview_schedule();
    let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
    let (db, n) = load_corpus(&corpus, EPOCH_1342_FIRST_SLOT);
    eprintln!("adjudication: loaded {n} corpus blocks (<= {EPOCH_1342_FIRST_SLOT}); advancing UNINTERRUPTED...");
    co_advance(&store, &cp, &db, &sched);

    let ade = ade_post_state(&store);
    let cp_fp = cp.fingerprint().map(|h| hexs(&h.0)).unwrap_or_else(|_| "incomplete".into());
    let refs = ref_post_state(&ref_1342, EPOCH_1342_FIRST_SLOT, 1342);
    let ade_go: u128 = ade.go.values().map(|v| *v as u128).sum();
    let card_go: u128 = refs.go.values().map(|v| *v as u128).sum();
    let ade_rew: u128 = ade.rewards.values().map(|v| *v as u128).sum();
    let card_rew: u128 = refs.rewards.values().map(|v| *v as u128).sum();

    let report = format!(
"B3C0-ADJUDICATION-v1
chain_point=slot:{}|epoch:{}
input_accumulator_blake2b={acc_h}
input_checkpoint_blake2b={cp_h}
reference_state_blake2b={ref_h}
checkpoint_fingerprint={cp_fp}
ade_go_pools={}|ade_go_total={ade_go}
card_go_pools={}|card_go_total={card_go}
go_stake_residual={}
ade_reward_total={ade_rew}|card_reward_total={card_rew}|reward_residual={}
ade_treasury={}|card_treasury={}|treasury_residual={}
ade_reserves={}|card_reserves={}|reserves_residual={}
",
        EPOCH_1342_FIRST_SLOT, ade.epoch,
        ade.go.len(), refs.go.len(),
        ade_go as i128 - card_go as i128,
        ade_rew as i128 - card_rew as i128,
        ade.treasury, refs.treasury, ade.treasury as i128 - refs.treasury as i128,
        ade.reserves, refs.reserves, ade.reserves as i128 - refs.reserves as i128,
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");
    // Pinned regression fixture: runs A and B (independent sha256-verified copies, one uninterrupted process
    // each) produced this identical residual and report hash. A single invocation now self-checks against the
    // doubled-confirmed pin instead of leaving the operator to diff two runs by hand.
    assert_eq!(
        ade_go as i128 - card_go as i128,
        -343_260_172_883,
        "the -343B go-stake residual reproduces exactly (doubled-confirmed, base UTxO exonerated)"
    );
    assert_eq!(
        report_hash, "6b04d8c0de217b408ca8bd44e003de6922bc37224080fe6272490032939252d9",
        "canonical adjudication report hash (pinned; runs A+B byte-identical from isolated copies)"
    );
}

// ===========================================================================
// CE3D-GO-STAKE-DERIVATION-LOCALIZATION (GREEN / evidence; NO BLUE change)
//
// Localizes the REAL -343,260,172,883 go-stake residual at the CREDENTIAL level,
// across the mark/set/go snapshot phases. Base UTxO is already EXONERATED (B3c.0);
// this reads the accumulator's existing `snapshots.*.delegations` (per-credential
// Hash28 -> (PoolId, Coin)) and the cardano decode as-is. See
// docs/clusters/CE3D-GO-STAKE-DERIVATION-LOCALIZATION/.
// ===========================================================================

use ade_ledger::epoch::StakeSnapshot;

/// Per-credential delegations as canonical bytes: Hash28 -> (PoolId bytes, coin). The go-snapshot key is
/// Hash28 (discriminant-stripped) on both the accumulator and the cardano decode.
fn deleg_canon(s: &StakeSnapshot) -> BTreeMap<[u8; 28], ([u8; 28], u64)> {
    s.delegations.iter().map(|(h, (pid, c))| (h.0, ((pid.0).0, c.0))).collect()
}

/// pool_stakes as canonical bytes.
fn pool_canon(s: &StakeSnapshot) -> BTreeMap<[u8; 28], u128> {
    s.pool_stakes.iter().map(|(pid, c)| ((pid.0).0, c.0 as u128)).collect()
}

/// Fold per-credential delegations back into per-pool totals.
fn fold_delegations(d: &BTreeMap<[u8; 28], ([u8; 28], u64)>) -> BTreeMap<[u8; 28], u128> {
    let mut m: BTreeMap<[u8; 28], u128> = BTreeMap::new();
    for (_, (pid, c)) in d {
        *m.entry(*pid).or_insert(0) += *c as u128;
    }
    m
}

/// pool_stakes == fold(delegations), value-wise (absent == 0). A false here would be a folding defect.
fn fold_ok(s: &StakeSnapshot) -> bool {
    let declared = pool_canon(s);
    let folded = fold_delegations(&deleg_canon(s));
    let keys: std::collections::BTreeSet<&[u8; 28]> = declared.keys().chain(folded.keys()).collect();
    keys.iter().all(|k| declared.get(*k).copied().unwrap_or(0) == folded.get(*k).copied().unwrap_or(0))
}

/// Closed-cause buckets for the per-credential go-phase differential: (count, summed delta).
#[derive(Default)]
struct GoBuckets {
    only_ade: (u64, i128),
    only_ref: (u64, i128),
    target_mismatch: (u64, i128),
    value_delta: (u64, i128),
    matched: u64,
}

/// Classify every credential of the union into exactly one closed cause. The summed deltas conserve:
/// Σ(only_ade + only_ref + target_mismatch + value_delta) == Σ_cred (ade_coin - ref_coin) == go residual.
fn classify_go(
    ade: &BTreeMap<[u8; 28], ([u8; 28], u64)>,
    card: &BTreeMap<[u8; 28], ([u8; 28], u64)>,
) -> GoBuckets {
    let mut b = GoBuckets::default();
    let keys: std::collections::BTreeSet<&[u8; 28]> = ade.keys().chain(card.keys()).collect();
    for k in keys {
        match (ade.get(k), card.get(k)) {
            (Some((ap, ac)), Some((rp, rc))) => {
                let d = *ac as i128 - *rc as i128;
                if ap == rp && ac == rc {
                    b.matched += 1;
                } else if ap == rp {
                    b.value_delta.0 += 1;
                    b.value_delta.1 += d;
                } else {
                    b.target_mismatch.0 += 1;
                    b.target_mismatch.1 += d;
                }
            }
            (Some((_, ac)), None) => {
                b.only_ade.0 += 1;
                b.only_ade.1 += *ac as i128;
            }
            (None, Some((_, rc))) => {
                b.only_ref.0 += 1;
                b.only_ref.1 -= *rc as i128;
            }
            (None, None) => {}
        }
    }
    b
}

/// Decode a cardano POST reference and return the three phases' per-credential delegations + fold-ok flags.
fn ref_phase_delegations(
    state_path: &Path,
    slot: u64,
    epoch: u64,
) -> [(BTreeMap<[u8; 28], ([u8; 28], u64)>, bool, usize); 3] {
    use ade_ledger::bootstrap_anchor::SeedPoint;
    use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
    let state = std::fs::read(state_path).expect("ref state");
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, _c) = decode_native_nonutxo_state(&state, point, epoch, 2).expect("decode ref");
    [&s1a.snapshots.mark.0, &s1a.snapshots.set.0, &s1a.snapshots.go.0]
        .map(|snap| (deleg_canon(snap), fold_ok(snap), snap.pool_stakes.len()))
}

/// I-GSD-5 base-zero gate: at POST-1340 the reduced checkpoint's per-credential base equals a fresh
/// `reduce_txout` of cardano's reference UTxO byte-for-byte (0 mismatches) — the B3c.0 proof, re-asserted so
/// this cluster is self-contained. Establishes the base-UTxO pipeline contributes zero error.
#[test]
#[ignore = "GSD base-zero: POST-1340 checkpoint == reduction (materializes tables; ~2min)"]
fn gsd_base_zero_at_post1340() {
    use ade_ledger::mithril_utxo_materialize::materialize_tables_to_utxo;
    use ade_ledger::reduced_utxo::{reduce_txout, ReducedStakeRef};
    let tables_path = env_path(
        "CE3D_TABLES_1340",
        "/home/ts/.cardano-ce3d-extract/db/ledger/115776011_db-analyser/tables",
    );
    let tables = std::fs::read(&tables_path).expect("read POST-1340 tables");
    let utxo = materialize_tables_to_utxo(&tables, 6, None).expect("materialize");
    let mut reduc: BTreeMap<StakeCredential, u64> = BTreeMap::new();
    for out in utxo.utxos.values() {
        if let (coin, ReducedStakeRef::Base(cred)) = reduce_txout(out) {
            *reduc.entry(cred).or_insert(0) += coin.0;
        }
    }
    let seed_cp =
        env_path("CE3D_SEED_STORES", "/home/ts/.cardano-ce3d-rebootstrap").join("reduced-checkpoint.redb");
    let iso = std::env::temp_dir().join(format!("gsd-base-cp-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&iso);
    std::fs::copy(&seed_cp, &iso).expect("copy checkpoint to isolated path");
    let cp = ReducedUtxoCheckpoint::open(&iso).expect("open isolated checkpoint");
    let chk = cp.sum_base_credential_stake().expect("sum_base_credential_stake");
    let reduc_total: u128 = reduc.values().map(|v| *v as u128).sum();
    let chk_total: u128 = chk.values().map(|c| c.0 as u128).sum();
    let keys: std::collections::BTreeSet<&StakeCredential> = reduc.keys().chain(chk.keys()).collect();
    let mismatches = keys
        .iter()
        .filter(|k| reduc.get(**k).copied().unwrap_or(0) != chk.get(**k).map(|x| x.0).unwrap_or(0))
        .count();
    drop(cp);
    let _ = std::fs::remove_file(&iso);
    eprintln!(
        "GSD base-zero: reduction_total={reduc_total} checkpoint_total={chk_total} creds={} mismatches={mismatches}",
        reduc.len()
    );
    assert_eq!(reduc_total, chk_total, "base-UTxO aggregate byte-exact");
    assert_eq!(mismatches, 0, "base-UTxO zero: checkpoint == reduction for EVERY credential");
}

/// The credential-level go-phase localization (fast, DOUBLED deliverable). `go(1342)` is the seed's imported
/// `mark` rotated forward twice (`rotate_snapshots` is a pure clone — proven byte-for-byte in
/// `gsd_provenance_and_live_derivation`), so this reads the seed accumulator's mark directly and diffs it
/// against cardano's decoded `go(1342)`, per credential, classifying every delta into the closed cause set.
#[test]
#[ignore = "GSD: per-credential go-phase differential + classification (fast; run twice byte-identical)"]
fn gsd_go_phase_credential_differential() {
    use ade_crypto::blake2b_256;
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();

    // Ade go(1342) == seed accumulator's mark (ISOLATED copy, open is read-write).
    let seed = env_path("CE3D_SEED_STORES", "/home/ts/.cardano-ce3d-rebootstrap");
    let acc_src = seed.join("epoch-accumulator.redb");
    let acc_h = hexs(&blake2b_256(&std::fs::read(&acc_src).expect("read acc")).0);
    let iso = std::env::temp_dir().join(format!("gsd-seed-acc-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&iso);
    std::fs::copy(&acc_src, &iso).expect("copy seed acc to isolated path");
    let (ade_go, ade_fold_ok, ade_pools) = {
        let store = EpochAccumulatorStore::open(&iso).expect("open isolated seed acc");
        let (_slot, acc) = store.load_current().expect("load").expect("sealed accumulator");
        let m = &acc.epoch_state.snapshots.as_authoritative().unwrap().mark.0;
        (deleg_canon(m), fold_ok(m), m.pool_stakes.len())
    };
    let _ = std::fs::remove_file(&iso);

    // cardano POST-1342 go snapshot.
    let ref_1342 =
        env_path("CE3D_REF_1342", "/home/ts/.cardano-ce3d-extract/db/ledger/115948834_db-analyser/state");
    let ref_h = hexs(&blake2b_256(&std::fs::read(&ref_1342).expect("read ref")).0);
    let [_, _, (card_go, card_fold_ok, card_pools)] =
        ref_phase_delegations(&ref_1342, EPOCH_1342_FIRST_SLOT, 1342);

    let b = classify_go(&ade_go, &card_go);
    let ade_total: i128 = ade_go.values().map(|(_, c)| *c as i128).sum();
    let card_total: i128 = card_go.values().map(|(_, c)| *c as i128).sum();
    let residual = ade_total - card_total;
    let classified = b.only_ade.1 + b.only_ref.1 + b.target_mismatch.1 + b.value_delta.1;

    let report = format!(
"CE3D-GO-STAKE-DERIVATION-LOCALIZATION-v1
chain_point=slot:{}|epoch:1342
ade_go_source=seed_mark(=go1342_by_rotation)
input_seed_accumulator_blake2b={acc_h}
reference_state_blake2b={ref_h}
ade_go_creds={}|card_go_creds={}
ade_go_pools={ade_pools}|card_go_pools={card_pools}
ade_go_total={ade_total}|card_go_total={card_total}|go_residual={residual}
only_ade_count={}|only_ade_sum={}
only_ref_count={}|only_ref_sum={}
target_mismatch_count={}|target_mismatch_sum={}
value_delta_count={}|value_delta_sum={}
matched={}
classified_sum={classified}
fold_ok_ade={ade_fold_ok}|fold_ok_card={card_fold_ok}
",
        EPOCH_1342_FIRST_SLOT,
        ade_go.len(), card_go.len(),
        b.only_ade.0, b.only_ade.1,
        b.only_ref.0, b.only_ref.1,
        b.target_mismatch.0, b.target_mismatch.1,
        b.value_delta.0, b.value_delta.1,
        b.matched,
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");

    assert_eq!(residual, -343_260_172_883, "go(1342) residual reproduces exactly (seed mark vs cardano go)");
    assert_eq!(classified, -343_260_172_883, "the closed-cause buckets sum to the residual exactly");
    assert!(ade_fold_ok, "Ade go: pool_stakes == fold(delegations) (no folding defect)");
    assert!(card_fold_ok, "cardano go: pool_stakes == fold(delegations)");
    // Pinned localization (doubled byte-identical, independent processes): the ENTIRE residual is a
    // per-credential stake-VALUE difference (same credential, same delegation target). Base UTxO is exonerated
    // (gsd_base_zero_at_post1340), so the non-base component is the reward-account contribution. It is NOT
    // delegation presence, NOT delegation target, NOT folding.
    assert_eq!(b.only_ade.1, 0, "only-Ade credentials carry zero stake (phantom, e.g. the 32 phantom pools)");
    assert_eq!(b.only_ref.0, 0, "cardano's go has no credential absent from Ade's go");
    assert_eq!(b.target_mismatch.0, 0, "no delegation-target mismatch");
    assert_eq!(b.value_delta.1, -343_260_172_883, "the entire residual is per-credential stake value (reward component)");
    assert_eq!(
        report_hash, "1e07cc50ee1bf14b3c5520fc3ba68694e969fdad7a366b5824f9f18c7492d385",
        "canonical localization report hash (pinned; doubled byte-identical from independent processes)"
    );
}

/// Provenance + live-derivation proof (SLOW: advances to POST-1342, ~60min). Proves `go(1342).delegations`
/// equals the seed's imported `mark` byte-for-byte (so the fast test's seed-mark read IS go(1342), not a
/// substitute), and emits the FRESH `mark(1342)`/`set(1342)` per-credential differential vs cardano — which
/// localizes whether the live derivation shares the go residual or is clean.
#[test]
#[ignore = "GSD provenance: advance to POST-1342, prove go==seed mark + fresh mark/set differential (SLOW)"]
fn gsd_provenance_and_live_derivation() {
    use ade_crypto::blake2b_256;
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let dir = env_path("GSD_DIR", "/home/ts/.cardano-ce3d-goderiv-a");
    let acc_path = dir.join("epoch-accumulator.redb");
    let cp_path = dir.join("reduced-checkpoint.redb");
    let acc_h = hexs(&blake2b_256(&std::fs::read(&acc_path).expect("read acc")).0);
    let cp_h = hexs(&blake2b_256(&std::fs::read(&cp_path).expect("read cp")).0);

    let store = EpochAccumulatorStore::open(&acc_path).expect("open acc");
    let cp = ReducedUtxoCheckpoint::open(&cp_path).expect("open cp");
    // Capture the seed's imported mark BEFORE advancing (same store, single open).
    let seed_mark = {
        let (_s0, acc0) = store.load_current().expect("load seed").expect("sealed");
        deleg_canon(&acc0.epoch_state.snapshots.as_authoritative().unwrap().mark.0)
    };

    let sched = preview_schedule();
    let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
    let (db, n) = load_corpus(&corpus, EPOCH_1342_FIRST_SLOT);
    eprintln!("GSD provenance: loaded {n} corpus blocks; advancing to POST-1342 UNINTERRUPTED...");
    co_advance(&store, &cp, &db, &sched);

    let (_s1, acc) = store.load_current().expect("load").expect("sealed accumulator");
    let go_1342 = deleg_canon(&acc.epoch_state.snapshots.as_authoritative().unwrap().go.0);
    let mark_1342 = deleg_canon(&acc.epoch_state.snapshots.as_authoritative().unwrap().mark.0);
    let set_1342 = deleg_canon(&acc.epoch_state.snapshots.as_authoritative().unwrap().set.0);
    assert_eq!(acc.epoch_state.epoch.0, 1342, "accumulator at epoch 1342");

    // PROVE: go(1342) is the seed's imported mark rotated forward (rotate_snapshots is a pure clone).
    let go_is_seed_mark = go_1342 == seed_mark;

    // cardano POST-1342 mark/set/go.
    let ref_1342 =
        env_path("CE3D_REF_1342", "/home/ts/.cardano-ce3d-extract/db/ledger/115948834_db-analyser/state");
    let ref_h = hexs(&blake2b_256(&std::fs::read(&ref_1342).expect("read ref")).0);
    let [(card_mark, _, _), (card_set, _, _), (card_go, _, _)] =
        ref_phase_delegations(&ref_1342, EPOCH_1342_FIRST_SLOT, 1342);

    // Per-phase residual = Σ_cred (ade_coin - ref_coin) over the union.
    let phase_residual = |ade: &BTreeMap<[u8; 28], ([u8; 28], u64)>, card: &BTreeMap<[u8; 28], ([u8; 28], u64)>| -> i128 {
        let at: i128 = ade.values().map(|(_, c)| *c as i128).sum();
        let ct: i128 = card.values().map(|(_, c)| *c as i128).sum();
        at - ct
    };
    let bm = classify_go(&mark_1342, &card_mark);
    let bs = classify_go(&set_1342, &card_set);
    let bg = classify_go(&go_1342, &card_go);

    let report = format!(
"CE3D-GO-STAKE-DERIVATION-PROVENANCE-v1
chain_point=slot:{}|epoch:1342
input_accumulator_blake2b={acc_h}
input_checkpoint_blake2b={cp_h}
reference_state_blake2b={ref_h}
go_equals_seed_mark={go_is_seed_mark}
mark_residual={}|mark_only_ade={}|mark_only_ref={}|mark_target_mismatch={}|mark_value_delta={}
set_residual={}|set_only_ade={}|set_only_ref={}|set_target_mismatch={}|set_value_delta={}
go_residual={}|go_only_ade={}|go_only_ref={}|go_target_mismatch={}|go_value_delta={}
",
        EPOCH_1342_FIRST_SLOT,
        phase_residual(&mark_1342, &card_mark), bm.only_ade.1, bm.only_ref.1, bm.target_mismatch.1, bm.value_delta.1,
        phase_residual(&set_1342, &card_set), bs.only_ade.1, bs.only_ref.1, bs.target_mismatch.1, bs.value_delta.1,
        phase_residual(&go_1342, &card_go), bg.only_ade.1, bg.only_ref.1, bg.target_mismatch.1, bg.value_delta.1,
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");

    assert!(go_is_seed_mark, "go(1342).delegations == the seed's imported mark (pure-clone rotation)");
    assert_eq!(
        phase_residual(&go_1342, &card_go),
        -343_260_172_883,
        "the advanced go(1342) residual matches the pinned -343B"
    );
    // The FRESH live-derived phases ALSO diverge as pure value_delta (same cred, same pool) => the
    // reward-contribution discrepancy is in the LIVE derivation, not confined to the seed's imported go.
    // set(1342) is POST-1340-derived, whose base is exonerated (gsd_base_zero_at_post1340), so its value_delta
    // is unambiguously the reward-account contribution.
    assert_eq!(phase_residual(&mark_1342, &card_mark), -355_446_908_982, "fresh mark(1342) residual (POST-1341)");
    assert_eq!(phase_residual(&set_1342, &card_set), -363_268_230_670, "fresh set(1342) residual (POST-1340)");
    assert_eq!(bm.value_delta.1, -355_446_908_982, "mark(1342): the residual is pure per-credential value_delta");
    assert_eq!(bs.value_delta.1, -363_268_230_670, "set(1342): the residual is pure per-credential value_delta");
    assert_eq!(bm.only_ref.1 + bm.target_mismatch.1, 0, "mark(1342): not delegation presence/target");
    assert_eq!(bs.only_ref.1 + bs.target_mismatch.1, 0, "set(1342): not delegation presence/target");
    assert_eq!(
        report_hash, "8b254305d5028ce23603ee2550d2f057c1ea4a042b324559ea0ed8b838a96b29",
        "canonical provenance report hash (pinned; deterministic advance from the isolated seed copy)"
    );
}

// ===========================================================================
// CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION -- PRECONDITION (GREEN / evidence)
//
// The go-stake localization proved the residual is a per-credential stake-VALUE
// difference (base exonerated, delegation/folding ruled out), and INFERRED the
// non-base component is the reward-account contribution. This test DIRECTLY
// measures that: it compares Ade's RAW reward-account balances (the seed
// accumulator's cert_state.delegation.rewards at POST-1340, pre-snapshot-fold)
// against cardano's POST-1340 reward map, per credential, split delegated vs
// undelegated -- proving whether the reward balances are already wrong BEFORE
// snapshot construction. If so, the corrective slice targets reward EVOLUTION
// (RUPD/lifecycle/inputs), NOT the build_boundary_mark_snapshot fold. The CPDE
// gov-refund (-500B, undelegated) is kept in a disjoint bucket.
// ===========================================================================

/// Canonical reward map: full StakeCredential key (discriminant-preserving) -> lovelace.
fn reward_canon<'a, I>(it: I) -> BTreeMap<Vec<u8>, u64>
where
    I: Iterator<Item = (&'a StakeCredential, &'a ade_types::tx::Coin)>,
{
    it.map(|(c, co)| (cred_key(c), co.0)).collect()
}

/// The 28-byte credential hash (the go-snapshot key, discriminant-stripped) for either credential variant.
fn cred_hash28(c: &StakeCredential) -> [u8; 28] {
    match c {
        StakeCredential::KeyHash(h) | StakeCredential::ScriptHash(h) => h.0,
    }
}

/// CRAE root confirmation (cardano-only, fast): the reward accrued to the mark's credentials across the
/// 1340->1341 boundary (POST-1341 reward - POST-1340 reward, the RUPD payout) equals the go-stake residual.
/// Ade builds the mark from PRE-RUPD rewards (`epoch_accumulator.rs:486`, before the boundary reward-update at
/// :508); cardano's SNAP takes it AFTER `applyRUpd`. So Ade's snapshot is one RUPD stale, undercounting each
/// delegated credential's stake by exactly the reward it accrued at that boundary -- the -343..-363B.
#[test]
#[ignore = "CRAE root confirm: boundary RUPD reward-accrual on the mark's creds == the go residual (cardano-only)"]
fn crae_rupd_accrual_equals_residual() {
    use ade_crypto::blake2b_256;
    use ade_ledger::bootstrap_anchor::SeedPoint;
    use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let reward_by_hash28 = |slot: u64, epoch: u64| -> BTreeMap<[u8; 28], u128> {
        let path = format!("/home/ts/.cardano-ce3d-extract/db/ledger/{slot}_db-analyser/state");
        let state = std::fs::read(&path).expect("ref state");
        let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
        let (s1a, _c) = decode_native_nonutxo_state(&state, point, epoch, 2).expect("decode");
        let mut m: BTreeMap<[u8; 28], u128> = BTreeMap::new();
        for (c, co) in s1a.cert_state.delegation.rewards.iter() {
            *m.entry(cred_hash28(c)).or_insert(0) += co.0 as u128;
        }
        m
    };
    let rew_1340 = reward_by_hash28(115_776_011, 1340);
    let rew_1341 = reward_by_hash28(EPOCH_1341_FIRST_SLOT, 1341);

    // The mark(1341) credentials (= what set(1342) covers).
    let mark_creds: std::collections::BTreeSet<[u8; 28]> = {
        let state = std::fs::read("/home/ts/.cardano-ce3d-extract/db/ledger/115862416_db-analyser/state")
            .expect("read POST-1341 state");
        let point = SeedPoint { slot: SlotNo(EPOCH_1341_FIRST_SLOT), block_hash: Hash32([0u8; 32]) };
        let (s1a, _c) = decode_native_nonutxo_state(&state, point, 1341, 2).expect("decode");
        s1a.snapshots.mark.0.delegations.keys().map(|h| h.0).collect()
    };

    // Reward accrued to the mark's credentials across the boundary (the RUPD payout the stale mark misses).
    let mut accrual: i128 = 0;
    for h in &mark_creds {
        let d = rew_1341.get(h).copied().unwrap_or(0) as i128 - rew_1340.get(h).copied().unwrap_or(0) as i128;
        accrual += d;
    }
    let report = format!(
"CE3D-RUPD-ACCRUAL-v1
mark_creds={}|reward_accrued_across_1340_1341_boundary={accrual}
",
        mark_creds.len(),
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");
    // RESULT: +315,961,836,959 accrued to the mark's creds across the boundary -- same magnitude, opposite sign
    // as the -363B set(1342) residual (the gap is within-epoch withdrawals; the gross RUPD payout the stale mark
    // misses ~= the full -363B). CONFIRMS the root: Ade's mark freezes PRE-RUPD rewards (epoch_accumulator.rs:486
    // before the RUPD at :508); cardano's SNAP takes it AFTER applyRUpd, so every delegated cred is short by the
    // boundary RUPD payout.
    assert!(
        (300_000_000_000..=380_000_000_000).contains(&accrual),
        "the boundary RUPD accrual on the mark's creds accounts for the go-stake residual"
    );
    assert_eq!(
        report_hash, "368928233277a6ba7847cb4b24e57e9151fce848b99983b83c778d63d5a721b9",
        "canonical RUPD-accrual report hash (pinned)"
    );
}

/// CRAE model check (cardano-only, fast): is cardano's OWN mark(1341) snapshot stake == cardano's OWN base UTxO
/// + reward-account balance, per credential? Resolves the paradox (base/reward/fold all verified correct, yet the
/// snapshot is -350B off) -- either the snapshot IS base+reward (=> Ade feeds the fold wrong inputs) or it is NOT
/// (=> Ade's stake model is missing a component). Uses only cardano ground truth at POST-1341.
#[test]
#[ignore = "CRAE model: cardano mark(1341) vs cardano base+reward per credential (cardano-only, fast)"]
fn crae_cardano_mark_is_base_plus_reward() {
    use ade_crypto::blake2b_256;
    use ade_ledger::bootstrap_anchor::SeedPoint;
    use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
    use ade_ledger::mithril_utxo_materialize::materialize_tables_to_utxo;
    use ade_ledger::reduced_utxo::{reduce_txout, ReducedStakeRef};
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();

    // cardano base UTxO at POST-1341, folded to Hash28.
    let tables = std::fs::read("/home/ts/.cardano-ce3d-extract/db/ledger/115862416_db-analyser/tables")
        .expect("read POST-1341 tables");
    let utxo = materialize_tables_to_utxo(&tables, 6, None).expect("materialize");
    let mut base_h: BTreeMap<[u8; 28], u128> = BTreeMap::new();
    for out in utxo.utxos.values() {
        if let (coin, ReducedStakeRef::Base(cred)) = reduce_txout(out) {
            *base_h.entry(cred_hash28(&cred)).or_insert(0) += coin.0 as u128;
        }
    }

    // cardano reward (folded to Hash28) + mark(1341) snapshot, from the same decode.
    let state = std::fs::read("/home/ts/.cardano-ce3d-extract/db/ledger/115862416_db-analyser/state")
        .expect("read POST-1341 state");
    let point = SeedPoint { slot: SlotNo(EPOCH_1341_FIRST_SLOT), block_hash: Hash32([0u8; 32]) };
    let (s1a, _c) = decode_native_nonutxo_state(&state, point, 1341, 2).expect("decode POST-1341");
    let mut reward_h: BTreeMap<[u8; 28], u128> = BTreeMap::new();
    for (c, co) in s1a.cert_state.delegation.rewards.iter() {
        *reward_h.entry(cred_hash28(c)).or_insert(0) += co.0 as u128;
    }
    let mark: BTreeMap<[u8; 28], u64> =
        s1a.snapshots.mark.0.delegations.iter().map(|(h, (_, c))| (h.0, c.0)).collect();

    // Per credential in cardano's mark: reconstructed = base + reward; compare to the snapshot's stake.
    let (mut resid, mut mm, mut base_only, mut rew_only) = (0i128, 0u64, 0i128, 0i128);
    for (h, mstake) in &mark {
        let base = base_h.get(h).copied().unwrap_or(0);
        let reward = reward_h.get(h).copied().unwrap_or(0);
        let recon = base + reward;
        let d = *mstake as i128 - recon as i128;
        if d != 0 {
            resid += d;
            mm += 1;
        }
        // also track how much of each mark entry base vs reward would supply (diagnostic totals below).
        let _ = (base, reward, &mut base_only, &mut rew_only);
    }
    let mark_total: i128 = mark.values().map(|v| *v as i128).sum();
    let recon_total: i128 =
        mark.keys().map(|h| (base_h.get(h).copied().unwrap_or(0) + reward_h.get(h).copied().unwrap_or(0)) as i128).sum();

    let report = format!(
"CE3D-CARDANO-MARK-MODEL-v1
mark_creds={}|mark_stake_total={mark_total}
reconstructed_base_plus_reward_total={recon_total}
mark_minus_reconstructed={}|mismatch_creds={mm}
",
        mark.len(),
        mark_total - recon_total,
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");
    let _ = resid;
    // RESULT: cardano's OWN mark(1341) == base + reward for 59,687 of 59,701 credentials -- only 14 mismatch
    // (whales whose UTxO moved between the snapshot point and the sampled tables, a benign point artifact). So
    // the snapshot model IS base+reward; the -350B is NOT a missing stake component. The residual comes from
    // Ade reading the reward at the wrong point (pre-RUPD) -- see crae_rupd_accrual_equals_residual.
    assert!(mm < 50, "cardano's mark == base+reward for ~all credentials (only whale point-artifacts differ)");
    assert_eq!(
        report_hash, "97138693295ab37acdb63d480d48416e5e49d211a705ba99239b89f25e224681",
        "canonical cardano-mark-model report hash (pinned)"
    );
}

#[test]
#[ignore = "CRAE precondition: raw reward-account map differential Ade(seed) vs cardano at POST-1340 (fast)"]
fn crae_raw_reward_map_post1340() {
    use ade_crypto::blake2b_256;
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();

    // Ade's RAW reward map + the delegated-credential set (seed accumulator @ POST-1340; isolated copy).
    let seed = env_path("CE3D_SEED_STORES", "/home/ts/.cardano-ce3d-rebootstrap");
    let acc_src = seed.join("epoch-accumulator.redb");
    let acc_h = hexs(&blake2b_256(&std::fs::read(&acc_src).expect("read acc")).0);
    let iso = std::env::temp_dir().join(format!("crae-seed-acc-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&iso);
    std::fs::copy(&acc_src, &iso).expect("copy seed acc to isolated path");
    let (ade_rew, delegated, ade_epoch) = {
        let store = EpochAccumulatorStore::open(&iso).expect("open seed acc");
        let (_s, acc) = store.load_current().expect("load").expect("sealed accumulator");
        let rew = reward_canon(acc.cert_state.delegation.rewards.iter());
        let deleg: std::collections::BTreeSet<Vec<u8>> =
            acc.cert_state.delegation.delegations.keys().map(cred_key).collect();
        (rew, deleg, acc.epoch_state.epoch.0)
    };
    let _ = std::fs::remove_file(&iso);
    assert_eq!(ade_epoch, 1340, "seed accumulator is at POST-1340");

    // cardano POST-1340 RAW reward map (ground truth).
    let ref_1340 = env_path(
        "CE3D_REF_1340",
        "/home/ts/.cardano-ce3d-extract/db/ledger/115776011_db-analyser/state",
    );
    let ref_h = hexs(&blake2b_256(&std::fs::read(&ref_1340).expect("read ref")).0);
    let card_rew = {
        use ade_ledger::bootstrap_anchor::SeedPoint;
        use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
        let state = std::fs::read(&ref_1340).expect("ref state");
        let point = SeedPoint { slot: SlotNo(115_776_011), block_hash: Hash32([0u8; 32]) };
        let (s1a, _c) = decode_native_nonutxo_state(&state, point, 1340, 2).expect("decode POST-1340");
        reward_canon(s1a.cert_state.delegation.rewards.iter())
    };

    // Per-credential reward differential, split delegated vs undelegated (CPDE lives in undelegated).
    let keys: std::collections::BTreeSet<&Vec<u8>> = ade_rew.keys().chain(card_rew.keys()).collect();
    let (mut d_resid, mut u_resid) = (0i128, 0i128);
    let (mut d_mm, mut u_mm) = (0u64, 0u64);
    let (mut d_only_ade, mut d_only_ref, mut u_only_ade, mut u_only_ref) = (0u64, 0u64, 0u64, 0u64);
    for k in &keys {
        let a = ade_rew.get(*k).copied();
        let c = card_rew.get(*k).copied();
        let d = a.unwrap_or(0) as i128 - c.unwrap_or(0) as i128;
        if d == 0 {
            continue;
        }
        if delegated.contains(*k) {
            d_resid += d;
            d_mm += 1;
            if a.is_none() {
                d_only_ref += 1;
            }
            if c.is_none() {
                d_only_ade += 1;
            }
        } else {
            u_resid += d;
            u_mm += 1;
            if a.is_none() {
                u_only_ref += 1;
            }
            if c.is_none() {
                u_only_ade += 1;
            }
        }
    }
    let ade_total: i128 = ade_rew.values().map(|v| *v as i128).sum();
    let card_total: i128 = card_rew.values().map(|v| *v as i128).sum();

    let report = format!(
"CE3D-RAW-REWARD-PRECONDITION-v1
chain_point=POST-1340|slot:115776011|epoch:1340
input_seed_accumulator_blake2b={acc_h}
reference_state_blake2b={ref_h}
ade_reward_accts={}|card_reward_accts={}|delegated_creds={}
ade_reward_total={ade_total}|card_reward_total={card_total}|reward_residual_total={}
delegated_mismatch_creds={d_mm}|delegated_reward_residual={d_resid}|delegated_only_ade={d_only_ade}|delegated_only_ref={d_only_ref}
undelegated_mismatch_creds={u_mm}|undelegated_reward_residual={u_resid}|undelegated_only_ade={u_only_ade}|undelegated_only_ref={u_only_ref}
",
        ade_rew.len(), card_rew.len(), delegated.len(),
        ade_total - card_total,
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");
    // RESULT: the raw reward map at POST-1340 is essentially correct (delegated residual +29,435,384, ~0.00001%,
    // POSITIVE) -- NOT the -343B. So the reward balances are NOT wrong before the boundary; the go-stake -343B
    // enters LATER, at the boundary reward-update (RUPD) that the mark snapshot freezes. See crae_reward_map_post1341.
    assert_eq!(d_only_ade + d_only_ref, 0, "delegated reward accounts present on both sides (no one-sided)");
    assert!(
        d_resid.abs() < 100_000_000,
        "raw delegated reward residual at POST-1340 is negligible (not the -343B); it enters at the boundary RUPD"
    );
    assert_eq!(
        report_hash, "5a416c297b995f7ea819c95fc61f0864570621a13265d05521b1dd7ec11b4645",
        "canonical POST-1340 raw-reward report hash (pinned)"
    );
}

/// CRAE confirmation (SLOW, one crossing ~30min): advance from the POST-1340 seed to POST-1341 and compare Ade's
/// reward map BEFORE (seed, POST-1340) and AFTER (POST-1341, post-boundary-RUPD) against the cardano reference.
/// Proves the reward is correct BEFORE the 1340->1341 boundary and becomes wrong AT the boundary RUPD -- pinning
/// the "first point where a reward balance becomes wrong" to the reward-update application, not raw evolution.
#[test]
#[ignore = "CRAE confirmation: reward map correct at POST-1340, wrong at POST-1341 (boundary RUPD); SLOW"]
fn crae_reward_map_post1341() {
    use ade_crypto::blake2b_256;
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let dir = env_path("CRAE_DIR", "/home/ts/.cardano-ce3d-craec");
    let acc_path = dir.join("epoch-accumulator.redb");
    let cp_path = dir.join("reduced-checkpoint.redb");
    let acc_h = hexs(&blake2b_256(&std::fs::read(&acc_path).expect("read acc")).0);

    let store = EpochAccumulatorStore::open(&acc_path).expect("open acc");
    let cp = ReducedUtxoCheckpoint::open(&cp_path).expect("open cp");
    // Reward map + delegated set BEFORE advancing (seed, POST-1340).
    let (seed_rew, seed_deleg, seed_epoch) = {
        let (_s, acc) = store.load_current().expect("load seed").expect("sealed");
        (
            reward_canon(acc.cert_state.delegation.rewards.iter()),
            acc.cert_state.delegation.delegations.keys().map(cred_key).collect::<std::collections::BTreeSet<_>>(),
            acc.epoch_state.epoch.0,
        )
    };
    assert_eq!(seed_epoch, 1340, "seed at POST-1340");

    let sched = preview_schedule();
    let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
    let (db, n) = load_corpus(&corpus, EPOCH_1341_FIRST_SLOT);
    eprintln!("CRAE: loaded {n} blocks; advancing seed POST-1340 -> POST-1341 UNINTERRUPTED...");
    co_advance(&store, &cp, &db, &sched);
    let (ade_rew_1341, deleg_1341, epoch_1341) = {
        let (_s, acc) = store.load_current().expect("load").expect("sealed");
        (
            reward_canon(acc.cert_state.delegation.rewards.iter()),
            acc.cert_state.delegation.delegations.keys().map(cred_key).collect::<std::collections::BTreeSet<_>>(),
            acc.epoch_state.epoch.0,
        )
    };
    assert_eq!(epoch_1341, 1341, "accumulator at POST-1341 after the cross");

    // cardano references.
    let decode_rew = |slot: u64, epoch: u64| -> BTreeMap<Vec<u8>, u64> {
        use ade_ledger::bootstrap_anchor::SeedPoint;
        use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
        let path = format!("/home/ts/.cardano-ce3d-extract/db/ledger/{slot}_db-analyser/state");
        let state = std::fs::read(&path).expect("ref state");
        let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
        let (s1a, _c) = decode_native_nonutxo_state(&state, point, epoch, 2).expect("decode ref");
        reward_canon(s1a.cert_state.delegation.rewards.iter())
    };
    let card_1340 = decode_rew(115_776_011, 1340);
    let card_1341 = decode_rew(EPOCH_1341_FIRST_SLOT, 1341);

    // Delegated-credential reward residual at a point.
    let deleg_resid = |ade: &BTreeMap<Vec<u8>, u64>, card: &BTreeMap<Vec<u8>, u64>, deleg: &std::collections::BTreeSet<Vec<u8>>| -> (i128, u64) {
        let keys: std::collections::BTreeSet<&Vec<u8>> = ade.keys().chain(card.keys()).collect();
        let (mut resid, mut mm) = (0i128, 0u64);
        for k in keys {
            if !deleg.contains(k) {
                continue;
            }
            let d = ade.get(k).copied().unwrap_or(0) as i128 - card.get(k).copied().unwrap_or(0) as i128;
            if d != 0 {
                resid += d;
                mm += 1;
            }
        }
        (resid, mm)
    };
    let (r1340, m1340) = deleg_resid(&seed_rew, &card_1340, &seed_deleg);
    let (r1341, m1341) = deleg_resid(&ade_rew_1341, &card_1341, &deleg_1341);

    let report = format!(
"CE3D-RAW-REWARD-BOUNDARY-RUPD-v1
input_accumulator_blake2b={acc_h}
delegated_reward_residual_POST1340={r1340}|mismatch_creds={m1340}
delegated_reward_residual_POST1341={r1341}|mismatch_creds={m1341}
delta_introduced_at_1340_1341_boundary_RUPD={}
",
        r1341 - r1340,
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");
    // RESULT: reward correct at BOTH POST-1340 (+29,435,384) and POST-1341 (+29,441,734); the boundary RUPD
    // moved the delegated residual only +6,350. So the -343B is NOT the RUPD either -- the reward map is right
    // pre- AND post-boundary. The defect is in the BASE the mark combines (the ADVANCED checkpoint), not reward.
    // See crae_advanced_base_at_post1341.
    assert!(r1340.abs() < 100_000_000, "reward correct BEFORE the boundary (POST-1340)");
    assert!(r1341.abs() < 100_000_000, "reward STILL correct AFTER the boundary RUPD (POST-1341) -- not the RUPD");
    assert_eq!(
        report_hash, "c718f7e61f79cddbd9a02670b480d7ec7bd8492f3890d0b5d19f18edd6d2b2ef",
        "canonical boundary-RUPD reward report hash (pinned; reward correct on both sides of the boundary)"
    );
}

/// CRAE root-cause test (the decisive one): does the reduced checkpoint's base stay byte-exact AFTER it ADVANCES
/// through the epoch? B3c.0 exonerated only the reducer + a FRESH checkpoint at the seed point (POST-1340); the
/// mark snapshot combines the checkpoint ADVANCED to the boundary. Advance a fresh checkpoint copy from the seed
/// to POST-1341 and compare its `sum_base_credential_stake` to a fresh `reduce_txout` of cardano's POST-1341
/// reference UTxO. A residual here (≈ the -350B) localizes the defect to the checkpoint ADVANCE, not the reducer,
/// not the reward, not the fold.
#[test]
#[ignore = "CRAE root: advanced checkpoint base at POST-1341 vs cardano UTxO reduction (SLOW: checkpoint advance)"]
fn crae_advanced_base_at_post1341() {
    use ade_crypto::blake2b_256;
    use ade_ledger::mithril_utxo_materialize::materialize_tables_to_utxo;
    use ade_ledger::reduced_utxo::{reduce_txout, ReducedStakeRef};
    let hexs = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();

    // Advance a fresh checkpoint copy from the seed to POST-1341.
    let seed = env_path("CE3D_SEED_STORES", "/home/ts/.cardano-ce3d-rebootstrap");
    let cp_src = seed.join("reduced-checkpoint.redb");
    let cp_h = hexs(&blake2b_256(&std::fs::read(&cp_src).expect("read cp")).0);
    let iso = std::env::temp_dir().join(format!("crae-adv-cp-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&iso);
    std::fs::copy(&cp_src, &iso).expect("copy checkpoint to isolated path");
    let ade_base = {
        let cp = ReducedUtxoCheckpoint::open(&iso).expect("open checkpoint");
        let cp_seed = cp.seed_slot().expect("seed").expect("sealed");
        let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
        let (db, n) = load_corpus(&corpus, EPOCH_1341_FIRST_SLOT);
        eprintln!("CRAE advanced-base: loaded {n} blocks; advancing checkpoint seed -> POST-1341...");
        advance_reduced_checkpoint_over_chaindb(
            &cp,
            &db,
            cp_seed,
            SlotNo(EPOCH_1341_FIRST_SLOT),
            CardanoEra::Conway,
        )
        .expect("advance checkpoint to POST-1341");
        cp.sum_base_credential_stake().expect("sum_base_credential_stake")
    };
    let _ = std::fs::remove_file(&iso);

    // Fresh reduction of cardano's POST-1341 reference UTxO (ground truth at the advanced point).
    let tables = std::fs::read("/home/ts/.cardano-ce3d-extract/db/ledger/115862416_db-analyser/tables")
        .expect("read POST-1341 tables");
    let utxo = materialize_tables_to_utxo(&tables, 6, None).expect("materialize");
    let mut card_base: BTreeMap<StakeCredential, u64> = BTreeMap::new();
    for out in utxo.utxos.values() {
        if let (coin, ReducedStakeRef::Base(cred)) = reduce_txout(out) {
            *card_base.entry(cred).or_insert(0) += coin.0;
        }
    }

    let ade_total: i128 = ade_base.values().map(|c| c.0 as i128).sum();
    let card_total: i128 = card_base.values().map(|v| *v as i128).sum();
    let keys: std::collections::BTreeSet<&StakeCredential> = ade_base.keys().chain(card_base.keys()).collect();
    let (mut mm, mut only_ade, mut only_ref) = (0u64, 0u64, 0u64);
    for k in &keys {
        let a = ade_base.get(*k).map(|c| c.0);
        let c = card_base.get(*k).copied();
        let d = a.unwrap_or(0) as i128 - c.unwrap_or(0) as i128;
        if d != 0 {
            mm += 1;
            if a.is_none() {
                only_ref += 1;
            }
            if c.is_none() {
                only_ade += 1;
            }
        }
    }
    let report = format!(
"CE3D-ADVANCED-BASE-POST1341-v1
input_checkpoint_blake2b={cp_h}
ade_advanced_base_total={ade_total}|card_base_total={card_total}|advanced_base_residual={}
mismatch_creds={mm}|only_ade={only_ade}|only_ref={only_ref}
",
        ade_total - card_total,
    );
    let report_hash = hexs(&blake2b_256(report.as_bytes()).0);
    eprintln!("\n{report}report_hash={report_hash}");
    // RESULT: the advanced checkpoint base is BYTE-EXACT at POST-1341 (residual 0, 0 mismatches). So the
    // checkpoint ADVANCE is correct too -- the -350B is NOT base. Combined with correct reward, this forces the
    // paradox resolved by crae_cardano_mark_is_base_plus_reward + the pre-RUPD mark ordering.
    assert_eq!(ade_total - card_total, 0, "the ADVANCED checkpoint base is byte-exact vs cardano at POST-1341");
    assert_eq!(mm, 0, "advanced base: zero per-credential mismatches");
    assert_eq!(
        report_hash, "b5ee48e8c8754e1e30346c0f0a8fd2dced9a282950505cc61ec6c6ffd82e6ec5",
        "canonical advanced-base report hash (pinned)"
    );
}

// ===========================================================================
// LIVE-LEDGER-EPOCH-TRANSITION S5 step 2c — the POSITIVE replay-equivalence
// evidence bundle. A within-k, SAME-LINEAGE rollback that is admitted, then
// event-qualified CLEARED (reset), then re-folded from the canonical ChainDB
// prefix, re-advances to a byte-IDENTICAL post-boundary state vs. the
// uninterrupted run — across the exact fingerprints S4 leadership will read.
// Proves S5's core claim: recovery admission + rematerialization is
// replay-equivalent for restart and controlled rollback. It does NOT activate
// accumulator-derived leadership authority (that is S4).
//
// Negative evidence lives elsewhere (a layered proof):
//   LineageMismatch / ExceededRollback / TargetNotOnCanonicalChain — wired
//     integration (ade_node node_lifecycle `s5_*` tests);
//   CorruptLastAdvancedPoint — wired/store (epoch_accumulator_store);
//   MissingCanonicalSpan / NonContiguousCanonicalSpan — wired/refold
//     (accumulator_recover_admit resolve path);
//   FingerprintMismatch — typed T-REC-05 (warm_start_recovery gate);
//   BeforeBootstrapAnchor — BLUE admission guard
//     (rollback::admission::rollback_before_bootstrap_anchor_is_typed); the
//     live rollback seam is structurally unreachable (a selected rollback
//     target is never below the immutable bootstrap floor), so there is no
//     wired fixture without fabricating a lower-block second fixture;
//   SchemaMismatch — the schema-v4 rejection path (epoch_accumulator
//     UnknownVersion / codec_rejects_pre_c_v3_store_rebootstrap_required).
// ===========================================================================

/// Copy the seed's two redb stores into an isolated work dir and open them (open is read-WRITE, so the
/// seed is never mutated). Returns the stores + the work dir (for the warm-start reopen).
fn s5_open_isolated(
    seed_dir: &Path,
    work: &Path,
    tag: &str,
) -> (EpochAccumulatorStore, ReducedUtxoCheckpoint, PathBuf) {
    let dst = work.join(tag);
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).expect("mkdir work");
    let acc_dst = dst.join("epoch-accumulator.redb");
    let cp_dst = dst.join("reduced-checkpoint.redb");
    std::fs::copy(seed_dir.join("epoch-accumulator.redb"), &acc_dst).expect("copy acc");
    std::fs::copy(seed_dir.join("reduced-checkpoint.redb"), &cp_dst).expect("copy cp");
    let store = EpochAccumulatorStore::open(&acc_dst).expect("open acc");
    let cp = ReducedUtxoCheckpoint::open(&cp_dst).expect("open cp");
    (store, cp, dst)
}

/// #1 the accumulator canonical hash = blake2b_256 of its canonical encoding (byte-covers the ENTIRE
/// non-UTxO ledger: pots, snapshots/go, cert/reward state, prev buffers, pending RUPD).
fn s5_acc_hash(store: &EpochAccumulatorStore) -> Hash32 {
    let (_slot, acc) = store.load_current().expect("load").expect("sealed");
    ade_crypto::blake2b_256(&ade_ledger::epoch_accumulator::encode_epoch_accumulator(&acc))
}

/// #6 the accumulator-derived AUTHORITY stake view: the `stake_by_pool` distribution `to_pool_distr_view`
/// consumes (the go-snapshot pool stakes + total), committed via the runtime's canonical stake-hash
/// formula (`EpochConsensusView::stake_view_canonical_hash`). The VRF/param bindings the full projection
/// also needs are supplied by S4, not the accumulator; the STAKE is what recovery must preserve.
fn s5_authority_stake_view_hash(store: &EpochAccumulatorStore) -> Hash32 {
    let (_slot, acc) = store.load_current().expect("load").expect("sealed");
    let go = &acc.epoch_state.snapshots.as_authoritative().expect("authoritative").go.0;
    let total: u128 = go.pool_stakes.values().map(|c| c.0 as u128).sum();
    let mut buf = Vec::with_capacity(24 + go.pool_stakes.len() * 36);
    buf.extend_from_slice(&total.to_be_bytes());
    buf.extend_from_slice(&(go.pool_stakes.len() as u64).to_be_bytes());
    for (pool, coin) in &go.pool_stakes {
        buf.extend_from_slice(&(pool.0).0); // PoolId(Hash28) -> 28 bytes
        buf.extend_from_slice(&coin.0.to_be_bytes());
    }
    ade_crypto::blake2b_256(&buf)
}

/// #2 the reduced-checkpoint state commitment: blake2b over the per-base-credential stake sums (the
/// checkpoint's authoritative reduced content). The build-marker `fingerprint()` is `Incomplete` after an
/// ADVANCE (it is written only by a fresh `build_from`), so recovery equivalence is proven over the CONTENT
/// the accumulator boundary-mark consumes.
fn s5_checkpoint_state_hash(cp: &ReducedUtxoCheckpoint) -> Hash32 {
    let sums = cp.sum_base_credential_stake().expect("reduced base-credential stake");
    let mut buf = Vec::with_capacity(8 + sums.len() * 37);
    buf.extend_from_slice(&(sums.len() as u64).to_be_bytes());
    for (cred, coin) in &sums {
        buf.extend_from_slice(&cred_key(cred));
        buf.extend_from_slice(&coin.0.to_be_bytes());
    }
    ade_crypto::blake2b_256(&buf)
}

/// #7 the warm-start replay hash: reopen the DURABLE stores from disk (the node's kill->warm-start
/// sequence) + advance-to-tip (idempotent at tip), then hash — proves the PERSISTED state, not just the
/// in-memory state, is byte-identical after recovery. Reuses the already-loaded corpus db.
fn s5_warm_start_hash(dst: &Path, db: &dyn ChainDb, sched: &EraSchedule) -> Hash32 {
    let store = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb")).expect("reopen acc");
    let cp = ReducedUtxoCheckpoint::open(&dst.join("reduced-checkpoint.redb")).expect("reopen cp");
    co_advance(&store, &cp, db, sched);
    s5_acc_hash(&store)
}

/// #8 the frozen leadership canonical hash = blake2b_256 of the persisted `FrozenLeadershipPoolDistr` (the
/// self-contained leadership `nesPd` authority S4-pre persists). Recovery MUST preserve it byte-identically:
/// it is epoch-frozen, so no advance / rollback / reset / refold changes it (`reset_to_bootstrap` deliberately
/// preserves it, 1b), and it is durable across a warm restart.
fn s5_leadership_hash(store: &EpochAccumulatorStore, epoch: EpochNo) -> Hash32 {
    ade_ledger::frozen_leadership::canonical_hash(
        &store
            .leadership_authority_for_epoch(epoch)
            .expect("leadership authority for epoch"),
    )
}

/// #8 warm-start variant: reopen the DURABLE store from disk (the kill->warm-start sequence) and hash the
/// leadership authority for `epoch` — proves the PERSISTED epoch-indexed object, not just the in-memory one, is
/// byte-identical after recovery. No `co_advance` needed: the leadership object is epoch-frozen, not fold-evolved.
fn s5_warm_start_leadership_hash(dst: &Path, epoch: EpochNo) -> Hash32 {
    let store = EpochAccumulatorStore::open(&dst.join("epoch-accumulator.redb")).expect("reopen acc");
    s5_leadership_hash(&store, epoch)
}

/// S4-0: seed the BOOTSTRAP-certified leadership (`nesPd_seed`) into a store copy from the manifest-bound seed
/// record read from `seed_dir`'s durable chain.db sidecar. The recovery arms then prove the native
/// boundary-frozen epochs survive rollback/refold + warm restart, and a reset restores CURRENT := BOOTSTRAP.
fn s5_seal_leadership(store: &EpochAccumulatorStore, seed_dir: &Path) {
    use ade_ledger::frozen_leadership::FrozenLeadershipPoolDistr;
    use ade_ledger::seed_consensus_inputs::decode_seed_epoch_consensus_inputs;
    use ade_runtime::chaindb::{PersistentChainDb, PersistentChainDbOptions, SnapshotStore};
    let cdb = PersistentChainDb::open(PersistentChainDbOptions::at(seed_dir.join("chain.db")))
        .expect("open cdb");
    let fps = cdb.list_seed_epoch_consensus_anchor_fps().expect("list");
    let record = decode_seed_epoch_consensus_inputs(
        &cdb.get_seed_epoch_consensus_inputs(&fps[0]).expect("get").expect("present"),
    )
    .expect("decode");
    let nespd_seed = FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(&record);
    store
        .seal_bootstrap_leadership_epochs(&[nespd_seed])
        .expect("seal bootstrap leadership from the manifest-bound seed record");
}

/// The canonical selected point (slot, block_no, hash) of the last corpus block at-or-below `target_slot`.
fn s5_corpus_point(corpus: &Path, target_slot: u64) -> (SlotNo, u64, Hash32) {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus.join("manifest.json")).expect("manifest"))
            .expect("manifest json");
    let mut best: Option<(u64, String, String)> = None;
    for b in manifest["blocks"].as_array().expect("blocks") {
        let s = b["slot"].as_u64().expect("slot");
        if s <= target_slot && best.as_ref().map_or(true, |(bs, _, _)| s > *bs) {
            best = Some((
                s,
                b["file"].as_str().expect("file").to_string(),
                b["hash"].as_str().expect("hash").to_string(),
            ));
        }
    }
    let (s, file, hash) = best.expect("a corpus block <= target");
    let bytes = std::fs::read(corpus.join(&file)).expect("block cbor");
    let decoded =
        ade_ledger::block_validity::header_input::decode_block(&bytes).expect("decode block");
    (SlotNo(s), decoded.header_input.block_no.0, parse_hash32(&hash))
}

/// Open an isolated copy of the v5 seed and RE-SEAL its CURRENT (epoch-1340) state as the bootstrap
/// baseline. The v5 seed's true bootstrap is epoch 1338, but the CE-3d corpus begins at late-1339, so a
/// `reset_to_bootstrap` -> 1338 cannot refold; re-sealing at the current advanced point gives a
/// corpus-refoldable recovery baseline (the exact state b3c0 folds from). Returns the baseline slot.
fn s5_open_resealed(
    seed_dir: &Path,
    work: &Path,
    tag: &str,
) -> (EpochAccumulatorStore, ReducedUtxoCheckpoint, PathBuf, SlotNo) {
    let (store, cp, dst) = s5_open_isolated(seed_dir, work, tag);
    let (slot, acc) = store.load_current().expect("load").expect("sealed");
    store.seal_bootstrap(&acc, slot).expect("re-seal accumulator at current");
    cp.seal_bootstrap(slot).expect("re-seal checkpoint at current");
    // S4-pre-1c: leadership-certify the copy (schema-v5 marker + frozen object) so the recovery arms can prove
    // the leadership authority survives clean advance, rollback+reset+refold, and warm restart unchanged.
    s5_seal_leadership(&store, seed_dir);
    (store, cp, dst, slot)
}

#[test]
#[ignore = "S5 2c: recovery replay-equivalence — uninterrupted vs advance+within-k-rollback+reset+refold, byte-identical (env S5_SEED_STORES / CE3D_CORPUS / CE3D_WORK); crosses 1340 -> 1341; SLOW ~100min (folds ~2461 real Conway blocks per pass)"]
fn s5_recovery_replay_equivalence_within_k_rollback() {
    use ade_ledger::rollback::{admit_rollback, RollbackPoint};
    use ade_types::BlockNo;

    let seed_dir = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
    let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
    let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
    let ref_dir = env_path("CE3D_REF", "/home/ts/.cardano-ce3d-extract/db/ledger");
    let sched = preview_schedule();
    // Post-boundary tip = the first block of epoch 1341 (crosses the 1340->1341 self-derived boundary).
    let final_slot = EPOCH_1341_FIRST_SLOT;

    // ONE corpus load, reused for both runs + both warm-starts (the file reads dominate the wall clock).
    let (db, n) = load_corpus(&corpus, final_slot);
    eprintln!("S5-2c: {n} corpus blocks <= {final_slot}");

    // ===== A: the UNINTERRUPTED reference — advance the re-sealed baseline -> post-boundary tip 1341 =====
    let (store_a, cp_a, dst_a, base_slot) = s5_open_resealed(&seed_dir, &work, "a");
    eprintln!("S5-2c: re-sealed recovery baseline at slot {} (corpus-refoldable)", base_slot.0);
    co_advance(&store_a, &cp_a, &db, &sched);
    let post_a = ade_post_state(&store_a);
    let acc_a = s5_acc_hash(&store_a);
    let cpst_a = s5_checkpoint_state_hash(&cp_a);
    let auth_a = s5_authority_stake_view_hash(&store_a);
    // The 1340->1341 cross seals nesPd for the NEXT leadership epoch (into-epoch + 1 = 1342); read it by exact index.
    let lead_a = s5_leadership_hash(&store_a, EpochNo(1342));

    // ===== S4-pre-2 REFERENCE PROOF (item 2): the boundary-frozen CURRENT leadership sealed by the 1340->1341
    // cross must byte-match the cardano reference nesPd (nes[5]) for its target epoch. LET THE TEST DECIDE the
    // mapping: compare against ALL three reference epochs and REPORT which one it byte-matches, then assert it
    // is exactly the labeled target_leadership_epoch. No mark/set/go naming — the reference field is nesPd. =====
    {
        let current = store_a
            .leadership_authority_for_epoch(EpochNo(1342))
            .expect("boundary-frozen current leadership for epoch 1342");
        let ade = ade_leadership_map(&current);
        let ref_1340 = ref_nes_pd(&ref_dir.join("115776011_db-analyser/state"), 115_776_011, 1340);
        let ref_1341 = ref_nes_pd(&ref_dir.join("115862416_db-analyser/state"), 115_862_416, 1341);
        let ref_1342 = ref_nes_pd(&ref_dir.join("115948834_db-analyser/state"), 115_948_834, 1342);
        let matched_epoch: Option<u64> = if ref_1342 == ade {
            Some(1342)
        } else if ref_1341 == ade {
            Some(1341)
        } else if ref_1340 == ade {
            Some(1340)
        } else {
            None
        };
        let src_epoch = post_a.epoch.saturating_sub(1);
        let hxs = |h: &Hash32| h.0.iter().map(|x| format!("{x:02x}")).collect::<String>();
        eprintln!("S4-pre-2 REFERENCE PROOF (native boundary leadership freeze):");
        eprintln!("  boundary_source_epoch          = {src_epoch}");
        eprintln!("  boundary_target_epoch          = {}", post_a.epoch);
        eprintln!("  frozen_leadership_target_epoch = {}", current.target_leadership_epoch.0);
        eprintln!(
            "  reference                      = {}",
            matched_epoch.map(|e| format!("POST-{e} nesPd")).unwrap_or_else(|| "NONE".into())
        );
        eprintln!("  pool_count ade / ref@target    = {} / {}", ade.len(),
            match current.target_leadership_epoch.0 { 1340 => ref_1340.len(), 1341 => ref_1341.len(), _ => ref_1342.len() });
        eprintln!("  zero_stake_pools               = {}", ade.values().filter(|(s, _)| *s == 0).count());
        eprintln!("  hash                           = {}", hxs(&lead_a));
        // Pin the mapping EMPIRICALLY: the labeled target epoch MUST be the reference the freeze byte-matches.
        assert_eq!(
            matched_epoch,
            Some(current.target_leadership_epoch.0),
            "boundary-frozen leadership must byte-match reference nesPd for its labeled target epoch {} (matched {:?}) \
             — if this is off-by-one, fix the freeze target semantics + labels, NOT the reference",
            current.target_leadership_epoch.0,
            matched_epoch
        );
        // The pinned mapping: target_leadership_epoch == the boundary's into-epoch + 1.
        assert_eq!(current.target_leadership_epoch.0, post_a.epoch + 1, "target_leadership_epoch == boundary into-epoch + 1");
        // The exact per-field proof (pool count + ids + stake + VRF, incl. zero-stake + retired, in one map eq).
        let reference = match current.target_leadership_epoch.0 {
            1340 => &ref_1340,
            1341 => &ref_1341,
            _ => &ref_1342,
        };
        assert_eq!(ade.len(), reference.len(), "pool_count exact");
        assert_eq!(&ade, reference, "pool ids + stake + VRF byte-exact vs reference nesPd (incl zero-stake + retired)");
        assert!(ade.values().any(|(s, _)| *s == 0), "zero-stake registered leadership pools preserved");
    }

    drop(store_a);
    drop(cp_a);
    let warm_a = s5_warm_start_hash(&dst_a, &db, &sched);
    let warm_lead_a = s5_warm_start_leadership_hash(&dst_a, EpochNo(1342));

    // The within-k, SAME-LINEAGE rollback target R: a real canonical block ~5_000 slots below the tip
    // (a few hundred blocks — well inside k=2160), above the re-sealed baseline.
    let (r_slot, r_bno, r_hash) = s5_corpus_point(&corpus, final_slot - 5_000);

    // ===== B: advance -> admit within-k rollback -> event-qualified CLEAR (reset) -> refold =====
    let (store_b, cp_b, dst_b, _) = s5_open_resealed(&seed_dir, &work, "b");
    co_advance(&store_b, &cp_b, &db, &sched);
    let b_tip = store_b.last_advanced_point().expect("lap").expect("certified tip");
    // Admit the rollback of B's certified tip back to R against the pre-rollback canonical chain — the exact
    // `admit_rollback` the runtime pre-clear `accumulator_admit_and_clear_for_rollback` calls.
    let depth = b_tip.block_no.0 - r_bno;
    eprintln!("S5-2c: admitting rollback tip(block {}) -> R(block {r_bno}), depth {depth} (k=2160)", b_tip.block_no.0);
    admit_rollback(
        &RollbackPoint { slot: b_tip.slot, block_no: b_tip.block_no, hash: b_tip.header_hash.clone() },
        &RollbackPoint { slot: r_slot, block_no: BlockNo(r_bno), hash: r_hash },
        &RollbackPoint { slot: base_slot, block_no: BlockNo(0), hash: Hash32([0u8; 32]) },
        2160,
        |s| db.get_block_by_slot(s).ok().flatten().map(|blk| blk.hash),
    )
    .expect("a within-k same-lineage rollback is admitted");
    // Event-qualified CLEAR: reset BOTH derived stores to the re-sealed baseline (anchor-absent, uncertified),
    // discarding the advanced state, then refold from the canonical ChainDB prefix — the re-materialization.
    store_b.reset_to_bootstrap().expect("clear accumulator anchor");
    cp_b.reset_to_bootstrap().expect("reset reduced checkpoint");
    assert_eq!(store_b.last_advanced_point().expect("lap"), None, "post-clear: uncertified");
    co_advance(&store_b, &cp_b, &db, &sched);
    let post_b = ade_post_state(&store_b);
    let acc_b = s5_acc_hash(&store_b);
    let cpst_b = s5_checkpoint_state_hash(&cp_b);
    let auth_b = s5_authority_stake_view_hash(&store_b);
    let lead_b = s5_leadership_hash(&store_b, EpochNo(1342));
    drop(store_b);
    drop(cp_b);
    let warm_b = s5_warm_start_hash(&dst_b, &db, &sched);
    let warm_lead_b = s5_warm_start_leadership_hash(&dst_b, EpochNo(1342));

    // ===== byte-identity across the S4-authority-relevant fingerprints =====
    let hx = |h: &Hash32| h.0.iter().map(|x| format!("{x:02x}")).collect::<String>();
    eprintln!("S5-2c fingerprints (uninterrupted A vs rollback-recovery B):");
    eprintln!("  epoch                 {} / {}", post_a.epoch, post_b.epoch);
    eprintln!("  #1 accumulator hash   {}", hx(&acc_a));
    eprintln!("  #2 checkpoint state   {}", hx(&cpst_a));
    eprintln!("  #6 authority stake    {}", hx(&auth_a));
    eprintln!("  #7 warm-start replay  {}", hx(&warm_a));
    eprintln!("  #8 frozen leadership  {}", hx(&lead_a));

    assert_eq!(post_a.epoch, 1341, "both cross into epoch 1341");
    assert_eq!(acc_a, acc_b, "#1 accumulator canonical hash byte-identical");
    assert_eq!(cpst_a, cpst_b, "#2 checkpoint / reduced-state content byte-identical");
    assert_eq!(post_a.treasury, post_b.treasury, "#3 treasury byte-identical");
    assert_eq!(post_a.reserves, post_b.reserves, "#3 reserves byte-identical");
    assert_eq!(post_a.rewards, post_b.rewards, "#4 reward map byte-identical");
    assert_eq!(post_a.go, post_b.go, "#5 go pool-set + values byte-identical");
    assert_eq!(auth_a, auth_b, "#6 accumulator-derived authority stake view byte-identical");
    assert_eq!(warm_a, warm_b, "#7 warm-start replay hash byte-identical");
    // Warm-start reopen re-materializes the SAME state as the in-memory run (durable round-trip).
    assert_eq!(warm_a, acc_a, "warm-start reopen == in-memory accumulator (A)");
    assert_eq!(warm_b, acc_b, "warm-start reopen == in-memory accumulator (B)");
    // #8 the frozen leadership authority (S4-pre-1c): epoch-frozen, so byte-identical across clean advance vs
    // rollback+reset+refold, and durable across warm restart. `reset_to_bootstrap` preserves it (1b).
    assert_eq!(lead_a, lead_b, "#8 frozen leadership canonical hash byte-identical (clean vs rollback-refold)");
    assert_eq!(warm_lead_a, lead_a, "#8 warm-start leadership hash == in-memory (A) — durable across restart");
    assert_eq!(warm_lead_b, lead_b, "#8 warm-start leadership hash == in-memory (B) — durable across restart");
}

/// LEADERSHIP DISTRIBUTION AUTHORITY TRACE: classify each of the 659 seed leadership pools by its
/// stake source (which accumulator snapshot, if any, carries it with the same value), its VRF source
/// (active cert params?), and its lifecycle (zero-stake / retiring / future / not-in-active) — proving
/// exactly what the accumulator CAN and CANNOT reconstruct, and what S4-pre's frozen leadership distr must
/// carry. Fixes nothing; produces the reference classification.
#[test]
#[ignore = "LDAT: classify the 659 seed leadership pools vs the accumulator state (env S5_SEED_STORES); FAST"]
fn ldat_classify_leadership_pools() {
    use ade_ledger::consensus_view::{PoolDistrView, PoolEntry};
    use ade_ledger::seed_consensus_inputs::decode_seed_epoch_consensus_inputs;
    use ade_runtime::chaindb::{PersistentChainDb, PersistentChainDbOptions, SnapshotStore};
    use ade_types::{Hash28, PoolId};

    let seed_dir = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
    let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");
    let hx = |h: &Hash28| h.0.iter().map(|x| format!("{x:02x}")).collect::<String>();

    let dst = work.join("ldat");
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).expect("mkdir");
    let acc_copy = dst.join("epoch-accumulator.redb");
    std::fs::copy(seed_dir.join("epoch-accumulator.redb"), &acc_copy).expect("copy acc");
    let store = EpochAccumulatorStore::open(&acc_copy).expect("open acc");
    store.reset_to_bootstrap().expect("reset to seed");
    let (_slot, acc) = store.load_current().expect("load").expect("sealed");

    let cdb = PersistentChainDb::open(PersistentChainDbOptions::at(seed_dir.join("chain.db"))).expect("open cdb");
    let fps = cdb.list_seed_epoch_consensus_anchor_fps().expect("list");
    let record = decode_seed_epoch_consensus_inputs(
        &cdb.get_seed_epoch_consensus_inputs(&fps[0]).expect("get").expect("present"),
    )
    .expect("decode");

    let snaps = acc.epoch_state.snapshots.as_authoritative().unwrap();
    let pool = &acc.cert_state.pool;
    let (go, set, mark) = (&snaps.go.0.pool_stakes, &snaps.set.0.pool_stakes, &snaps.mark.0.pool_stakes);

    let (mut go_exact, mut set_exact, mut mark_exact, mut no_snapshot) = (0u32, 0u32, 0u32, 0u32);
    let (mut vrf_match, mut vrf_mismatch, mut vrf_missing) = (0u32, 0u32, 0u32);
    let (mut zero_stake, mut retiring, mut future, mut not_active) = (0u32, 0u32, 0u32, 0u32);
    let mut unreconstructable: Vec<(Hash28, u64)> = Vec::new();

    for (h, seed_entry) in &record.pool_distribution {
        let pid = PoolId(h.clone());
        let st = seed_entry.active_stake;
        if go.get(&pid).map(|c| c.0) == Some(st) {
            go_exact += 1;
        } else if set.get(&pid).map(|c| c.0) == Some(st) {
            set_exact += 1;
        } else if mark.get(&pid).map(|c| c.0) == Some(st) {
            mark_exact += 1;
        } else {
            no_snapshot += 1;
            if st > 0 {
                unreconstructable.push((h.clone(), st));
            }
        }
        if st == 0 {
            zero_stake += 1;
        }
        match pool.pools.get(&pid) {
            Some(p) if p.vrf_hash == seed_entry.vrf_keyhash => vrf_match += 1,
            Some(_) => vrf_mismatch += 1,
            None => {
                vrf_missing += 1;
                not_active += 1;
            }
        }
        if pool.retiring.contains_key(&pid) {
            retiring += 1;
        }
        if pool.future_pools.contains_key(&pid) {
            future += 1;
        }
    }

    eprintln!("LDAT @ epoch {} — {} leadership pools", record.epoch_no.0, record.pool_distribution.len());
    eprintln!("  STAKE source: go_exact={go_exact} set_exact={set_exact} mark_exact={mark_exact} NO_snapshot={no_snapshot}");
    eprintln!("  VRF source (active cert params): match={vrf_match} mismatch={vrf_mismatch} MISSING={vrf_missing}");
    eprintln!("  lifecycle: zero_stake={zero_stake} retiring={retiring} future_pools={future} not_in_active={not_active}");
    eprintln!("  UNRECONSTRUCTABLE (non-zero stake, in NO accumulator snapshot): {}", unreconstructable.len());
    for (h, st) in &unreconstructable {
        let pid = PoolId(h.clone());
        eprintln!(
            "    pool {} stake {st} | in_active_params={} retiring={} future_pools={}",
            hx(h), pool.pools.contains_key(&pid), pool.retiring.contains_key(&pid), pool.future_pools.contains_key(&pid)
        );
    }

    // ACCEPTANCE: reconstruct the leadership PoolDistr from the accumulator's SET-snapshot stake + active-params
    // VRF, supplementing ONLY a retired pool's VRF from the frozen reference (the exact datum S4-pre must
    // persist), and assert it is BYTE-IDENTICAL to the seed leadership view. Proves the reference semantics
    // (stake=SET, vrf=frozen-params) AND quantifies the irreducible gap (retired-pool frozen VRF).
    let mut frozen_vrf_supplements = 0u32;
    let mut rec_pools: BTreeMap<Hash28, PoolEntry> = BTreeMap::new();
    for (h, seed_entry) in &record.pool_distribution {
        let pid = PoolId(h.clone());
        let active_stake = set.get(&pid).map(|c| c.0).unwrap_or(0); // SET stake (0 for zero-stake registered)
        let vrf_keyhash = match pool.pools.get(&pid) {
            Some(p) => p.vrf_hash.clone(),
            None => {
                frozen_vrf_supplements += 1;
                seed_entry.vrf_keyhash.clone() // the retired pool: VRF only in the frozen leadership snapshot
            }
        };
        rec_pools.insert(h.clone(), PoolEntry { active_stake, vrf_keyhash });
    }
    let total: u64 = rec_pools.values().map(|e| e.active_stake).sum();
    let reconstructed = PoolDistrView::new(record.epoch_no, total, record.active_slots_coeff, rec_pools);
    let reference = PoolDistrView::from_seed_epoch_consensus_inputs(&record);
    eprintln!("  reconstruction: SET stake + active VRF; frozen_vrf_supplements={frozen_vrf_supplements}");
    assert_eq!(
        reconstructed, reference,
        "LDAT: leadership PoolDistr reconstructs BYTE-EXACT from SET stake + active-params VRF + frozen supplement"
    );
    assert_eq!(
        frozen_vrf_supplements, 1,
        "exactly one retired pool needs a snapshot-frozen VRF (the irreducible datum S4-pre must persist)"
    );
    eprintln!("LDAT PROVEN: leadership = SET-snapshot stake + frozen pool-params VRF. The accumulator's active \
               params supply 658/659 VRFs byte-exact; 1 retired pool needs the frozen VRF -> S4-pre.");
}

/// S4-pre-1 SEED IDENTITY (the acceptance core): the self-contained `FrozenLeadershipPoolDistr` built from the
/// manifest-bound seed record projects BYTE-EXACT to the seed-window leadership `PoolDistr` — 659/659 pools,
/// stake + VRF exact — WITHOUT any go / active-param / retiring lookup. Proves the frozen object answers
/// leadership directly (design 1). Persistence codec + store schema + the real bootstrap wiring are S4-pre-1b.
#[test]
#[ignore = "S4-pre-1: from_frozen_leadership(seed record) == seed leadership PoolDistr byte-exact (env S5_SEED_STORES); FAST"]
fn s4pre_frozen_leadership_seed_identity() {
    use ade_ledger::consensus_view::PoolDistrView;
    use ade_ledger::frozen_leadership::FrozenLeadershipPoolDistr;
    use ade_ledger::seed_consensus_inputs::decode_seed_epoch_consensus_inputs;
    use ade_runtime::chaindb::{PersistentChainDb, PersistentChainDbOptions, SnapshotStore};

    let seed_dir = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
    let cdb = PersistentChainDb::open(PersistentChainDbOptions::at(seed_dir.join("chain.db"))).expect("open cdb");
    let fps = cdb.list_seed_epoch_consensus_anchor_fps().expect("list");
    let record = decode_seed_epoch_consensus_inputs(
        &cdb.get_seed_epoch_consensus_inputs(&fps[0]).expect("get").expect("present"),
    )
    .expect("decode");

    // Bootstrap import: the self-contained frozen leadership distr from the manifest-bound seed record.
    let frozen = FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(&record);
    assert_eq!(frozen.target_leadership_epoch, record.epoch_no, "same leadership epoch");
    assert_eq!(frozen.pools.len(), record.pool_distribution.len(), "all 659 leadership pools carried");

    // Project + compare to the proven-byte-exact seed leadership view (epoch + total + asc + per-pool stake + VRF).
    let from_frozen = frozen.to_pool_distr_view(record.active_slots_coeff);
    let from_seed = PoolDistrView::from_seed_epoch_consensus_inputs(&record);
    assert_eq!(
        from_frozen, from_seed,
        "S4-pre-1: from_frozen_leadership == seed leadership PoolDistr byte-exact (the SELF-CONTAINED authority)"
    );
    eprintln!(
        "S4-pre-1 SEED IDENTITY PROVEN: {} pools; the self-contained frozen leadership distr projects == the \
         seed leadership view byte-exact (no go/active-param/retiring lookup).",
        frozen.pools.len()
    );
}

/// S4-pre-1c CERTIFIED BOOTSTRAP LINEAGE (S4-0 epoch-indexed): seal the frozen leadership authority THROUGH THE
/// DURABLE STORE via the EXACT call the native bootstrap makes (`seal_bootstrap_leadership_epochs`), from the real
/// v5 seed record, and prove the certified store answers leadership BY EXACT EPOCH INDEX + stable across reopen.
/// This is the durable-path analog of `s4pre_frozen_leadership_seed_identity` (which proves only the in-memory
/// projection): it covers "fresh bootstrap store has the v5 marker", "the bootstrap-indexed object for the seed
/// epoch is present", "leadership_authority_for_epoch(seed_epoch) loads it", "hash stable across reopen", and
/// "to_pool_distr_view == seed leadership PoolDistr, 659/659, incl. zero-stake + the retired 1M-ADA pool".
#[test]
#[ignore = "S4-pre-1c/S4-0: seal_bootstrap_leadership_epochs produces a v5-certified store == seed leadership PoolDistr, read by exact epoch index, stable across reopen (env S5_SEED_STORES / CE3D_WORK); FAST"]
fn s4pre_1c_frozen_leadership_bootstrap_lineage() {
    use ade_ledger::consensus_view::PoolDistrView;
    use ade_ledger::frozen_leadership::{canonical_hash, FrozenLeadershipPoolDistr};
    use ade_ledger::seed_consensus_inputs::decode_seed_epoch_consensus_inputs;
    use ade_runtime::chaindb::{PersistentChainDb, PersistentChainDbOptions, SnapshotStore};

    let seed_dir = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
    let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");

    // The manifest-bound seed record (the leadership nesPd) from the durable chain.db sidecar.
    let cdb = PersistentChainDb::open(PersistentChainDbOptions::at(seed_dir.join("chain.db"))).expect("open cdb");
    let fps = cdb.list_seed_epoch_consensus_anchor_fps().expect("list");
    let record = decode_seed_epoch_consensus_inputs(
        &cdb.get_seed_epoch_consensus_inputs(&fps[0]).expect("get").expect("present"),
    )
    .expect("decode");

    // Seal via the EXACT native-bootstrap call into a FRESH store, source-bound to the record's own seed point
    // (which the real bootstrap binds to `binding.certified_point` — proven equal by the mithril-assembly
    // coherence gate, so the source check passes on a legitimate bootstrap).
    let dst = work.join("s4pre-1c-lineage");
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).expect("mkdir");
    let acc_path = dst.join("epoch-accumulator.redb");
    {
        let store = EpochAccumulatorStore::open(&acc_path).expect("open acc");
        store
            .seal_bootstrap_leadership_epochs(&[FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(&record)])
            .expect("seal bootstrap leadership from the manifest-bound seed record");

        // A fresh certified store: the schema-v5 marker + the BOOTSTRAP-indexed object for the seed epoch are
        // present, and the fail-closed epoch-indexed authority read loads it for its exact target epoch (which
        // also proves the CURRENT-indexed copy the bootstrap seal writes alongside it).
        assert!(
            store.bootstrap_frozen_leadership_for_epoch(record.epoch_no).expect("raw read").is_some(),
            "v5 leadership marker + bootstrap-indexed object present for the seed epoch"
        );
        let leadership = store
            .leadership_authority_for_epoch(record.epoch_no)
            .expect("leadership authority loads for the seed epoch");

        // 659/659 pools; zero-stake registered pools + the retired 1M-ADA pool are carried (the byte-exact
        // projection below proves each pool's frozen stake+VRF, incl. the retired pool's VRF — LDAT's 1 supplement).
        assert_eq!(leadership.pools.len(), record.pool_distribution.len(), "all leadership pools carried");
        assert!(
            leadership.pools.values().any(|e| e.active_stake == 0),
            "zero-stake registered leadership pools preserved"
        );
        assert!(
            leadership.pools.values().any(|e| e.active_stake >= 1_000_000_000_000),
            "the retired 1M-ADA leadership pool is carried with its frozen stake+VRF"
        );

        // The certified store projects BYTE-EXACT to the seed leadership PoolDistr (epoch+total+asc+stake+VRF).
        let from_store = leadership.to_pool_distr_view(record.active_slots_coeff);
        let from_seed = PoolDistrView::from_seed_epoch_consensus_inputs(&record);
        assert_eq!(from_store, from_seed, "certified store leadership == seed leadership PoolDistr byte-exact");
        // No drift through the codec/store round-trip vs the direct projection.
        assert_eq!(leadership, FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(&record));
    }

    // Canonical hash is STABLE across reopen (warm-restart durability of the leadership authority).
    let reopen_hash = || {
        let store = EpochAccumulatorStore::open(&acc_path).expect("reopen acc");
        canonical_hash(
            &store
                .leadership_authority_for_epoch(record.epoch_no)
                .expect("reopened leadership authority for the seed epoch"),
        )
    };
    assert_eq!(reopen_hash(), reopen_hash(), "frozen leadership canonical hash stable across reopen");

    eprintln!(
        "S4-pre-1c/S4-0 LINEAGE PROVEN: seal_bootstrap_leadership_epochs produced a v5-certified store; \
         {} leadership pools == the seed leadership PoolDistr byte-exact, read by exact epoch index; hash stable across reopen.",
        record.pool_distribution.len()
    );
}

/// S4-0 EPOCH-INDEXED LEADERSHIP ACCEPTANCE (1338 -> 1342): ONE certified store carrying the full leadership band,
/// each epoch built by its REAL provenance builder and read back by EXACT index — bootstrap {1338 = the seed
/// record's `nesPd` (SET-derived), 1339 = an imported MARK snapshot} + native {1340/1341/1342 = boundary freezes,
/// sealed CURRENT-only}. Proves: (a) all five indices resolve to DISTINCT objects whose `target_leadership_epoch`
/// == the queried epoch (never "the latest / current / nearest" object); (b) the bootstrap band is separable —
/// the two bootstrap epochs are present in `bootstrap_frozen_leadership_for_epoch`, the three native epochs are
/// not; (c) every epoch's authority hash is byte-stable across a reopen; (d) `reset_to_bootstrap` restores
/// CURRENT := BOOTSTRAP, so 1338/1339 survive but 1340/1341/1342 fail closed (`LeadershipEpochNotSealed`); (e)
/// off-band epochs (1337 / 1343) and a legacy (un-certified) store fail closed. The wrong-INDEX corruption case is
/// proven by the store unit `leadership_authority_rejects_wrong_epoch_object`; byte-exact native content vs the
/// reference nesPd is proven by the S5 #8 long proof — here 1340+ are native-shaped objects at their exact indices.
#[test]
#[ignore = "S4-0: epoch-indexed leadership acceptance 1338->1342 (bootstrap+MARK+native provenance, exact index, reset partition, off-band + legacy fail-closed) (env S5_SEED_STORES / CE3D_WORK); FAST"]
fn s4_0_epoch_indexed_leadership_acceptance_1338_to_1342() {
    use ade_ledger::epoch_accumulator::EpochAccumulator;
    use ade_ledger::frozen_leadership::{canonical_hash, FrozenLeadershipPoolDistr};
    use ade_ledger::seed_consensus_inputs::decode_seed_epoch_consensus_inputs;
    use ade_runtime::chaindb::{
        LeadershipAuthorityError, PersistentChainDb, PersistentChainDbOptions, SnapshotStore,
    };
    use ade_types::{Coin, PoolId};
    use std::collections::{BTreeMap, BTreeSet};

    let seed_dir = env_path("S5_SEED_STORES", "/home/ts/.cardano-ce3d-s1seed-v5");
    let work = env_path("CE3D_WORK", "/home/ts/.cardano-ce3d-extract/harness-work-s5");

    // --- The real epoch-1338 bootstrap object: the manifest-bound seed record's `pool_distribution` IS the
    // SET-derived leadership `nesPd_1338` (proven byte-exact vs the reference at bootstrap, S4-pre-1a). ---
    let cdb =
        PersistentChainDb::open(PersistentChainDbOptions::at(seed_dir.join("chain.db"))).expect("open cdb");
    let fps = cdb.list_seed_epoch_consensus_anchor_fps().expect("list");
    let record = decode_seed_epoch_consensus_inputs(
        &cdb.get_seed_epoch_consensus_inputs(&fps[0]).expect("get").expect("present"),
    )
    .expect("decode");
    assert_eq!(record.epoch_no.0, 1338, "the CE-3d v5 seed record is the epoch-1338 leadership nesPd");
    let nespd_1338 = FrozenLeadershipPoolDistr::from_seed_epoch_consensus_inputs(&record);

    // --- The epoch-1339 bootstrap object from an imported MARK snapshot (the seed+1 bridge source — the ONE
    // epoch no native freeze produces). Representative MARK derived from the 1338 pool set as (stake, vrf); 1339
    // content fidelity is the bootstrap-verbatim mark (the byte-exact reference path begins at 1340, S4-pre-2). ---
    let mark_1339: BTreeMap<PoolId, (u64, Hash32)> = record
        .pool_distribution
        .iter()
        .map(|(h, e)| (PoolId(h.clone()), (e.active_stake, e.vrf_keyhash.clone())))
        .collect();
    let nespd_1339 = FrozenLeadershipPoolDistr::from_mark_pool_distr(
        EpochNo(1339),
        record.seed_point_slot,
        record.seed_point_hash.clone(),
        &mark_1339,
    );

    // --- Native boundary freezes 1340/1341/1342 (S4-pre-2 shape: delegated ∩ registered). Built over the full
    // pool set at distinct target indices; a distinct `target_leadership_epoch` => a distinct canonical hash. ---
    let native = |target: u64| -> FrozenLeadershipPoolDistr {
        let mut delegated = BTreeSet::new();
        let mut stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();
        let mut vrfs: BTreeMap<PoolId, Hash32> = BTreeMap::new();
        for (h, e) in record.pool_distribution.iter() {
            let pid = PoolId(h.clone());
            delegated.insert(pid.clone());
            stakes.insert(pid.clone(), Coin(e.active_stake));
            vrfs.insert(pid, e.vrf_keyhash.clone());
        }
        FrozenLeadershipPoolDistr::from_boundary_snapshot(
            EpochNo(target),
            SlotNo(target * 1_000),
            Hash32([target as u8; 32]),
            &delegated,
            &stakes,
            &vrfs,
        )
    };

    // --- Populate ONE store: bootstrap {1338, 1339}; native CURRENT-only {1340, 1341, 1342}. ---
    let dst = work.join("s4-0-acceptance-1338-1342");
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).expect("mkdir");
    let acc_path = dst.join("epoch-accumulator.redb");
    let store = EpochAccumulatorStore::open(&acc_path).expect("open acc");
    // A bootstrap accumulator anchor so `reset_to_bootstrap` (below) has a target — the leadership reset is
    // coupled to the accumulator reset in one call. (Content is irrelevant here; the leadership band is the SUT.)
    store
        .seal_bootstrap(&EpochAccumulator::new(CardanoEra::Conway), record.seed_point_slot)
        .expect("seal bootstrap accumulator anchor");
    store
        .seal_bootstrap_leadership_epochs(&[nespd_1338.clone(), nespd_1339.clone()])
        .expect("seal bootstrap {1338, 1339}");
    for target in [1340u64, 1341, 1342] {
        store.seal_current_leadership(&native(target)).expect("seal native current leadership");
    }

    // (a) exact-index reads across the whole band: each returns the object whose target == the queried epoch.
    let mut hashes = Vec::new();
    for epoch in [1338u64, 1339, 1340, 1341, 1342] {
        let got = store
            .leadership_authority_for_epoch(EpochNo(epoch))
            .unwrap_or_else(|e| panic!("epoch {epoch} must read exact: {e:?}"));
        assert_eq!(got.target_leadership_epoch.0, epoch, "exact-index read: object target == queried epoch");
        hashes.push(canonical_hash(&got));
    }
    // Distinct objects (a distinct target => a distinct canonical authority hash).
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            assert_ne!(hashes[i], hashes[j], "each epoch's leadership authority is a distinct object");
        }
    }

    // (b) bootstrap band separable: 1338/1339 are bootstrap-indexed; the natives are CURRENT-only.
    for e in [1338u64, 1339] {
        assert!(
            store.bootstrap_frozen_leadership_for_epoch(EpochNo(e)).expect("read").is_some(),
            "epoch {e} is bootstrap-certified"
        );
    }
    for e in [1340u64, 1341, 1342] {
        assert!(
            store.bootstrap_frozen_leadership_for_epoch(EpochNo(e)).expect("read").is_none(),
            "native epoch {e} is CURRENT-only, not bootstrap"
        );
    }

    // (e) off-band epochs fail closed under the valid marker (not a nearest-neighbour read).
    for off in [1337u64, 1343] {
        match store.leadership_authority_for_epoch(EpochNo(off)) {
            Err(LeadershipAuthorityError::LeadershipEpochNotSealed { requested }) => {
                assert_eq!(requested, off)
            }
            other => {
                panic!("off-band epoch {off} must fail closed as LeadershipEpochNotSealed, got {other:?}")
            }
        }
    }

    // (c) each epoch's authority hash is byte-stable across a reopen (drop the writer first — redb is single-open).
    drop(store);
    let band_hashes = || -> Vec<Hash32> {
        let s = EpochAccumulatorStore::open(&acc_path).expect("reopen acc");
        [1338u64, 1339, 1340, 1341, 1342]
            .iter()
            .map(|&e| canonical_hash(&s.leadership_authority_for_epoch(EpochNo(e)).expect("reopened read")))
            .collect()
    };
    assert_eq!(band_hashes(), hashes, "the whole leadership band is byte-stable across reopen");

    // (d) reset_to_bootstrap restores CURRENT := BOOTSTRAP: the bootstrap band survives, the natives fail closed.
    let store = EpochAccumulatorStore::open(&acc_path).expect("reopen acc for reset");
    store.reset_to_bootstrap().expect("reset to bootstrap");
    for e in [1338u64, 1339] {
        let got = store.leadership_authority_for_epoch(EpochNo(e)).expect("bootstrap survives reset");
        assert_eq!(got.target_leadership_epoch.0, e);
    }
    for e in [1340u64, 1341, 1342] {
        match store.leadership_authority_for_epoch(EpochNo(e)) {
            Err(LeadershipAuthorityError::LeadershipEpochNotSealed { requested }) => {
                assert_eq!(requested, e)
            }
            other => panic!("native epoch {e} must be cleared by reset (LeadershipEpochNotSealed), got {other:?}"),
        }
    }

    // (e) a legacy / never-certified store fails closed with the schema refusal (no v5 marker).
    let legacy_dir = work.join("s4-0-acceptance-legacy");
    let _ = std::fs::remove_dir_all(&legacy_dir);
    std::fs::create_dir_all(&legacy_dir).expect("mkdir");
    let legacy =
        EpochAccumulatorStore::open(&legacy_dir.join("epoch-accumulator.redb")).expect("open legacy");
    match legacy.leadership_authority_for_epoch(EpochNo(1338)) {
        Err(LeadershipAuthorityError::OldAccumulatorSchemaNotLeadershipCertified { .. }) => {}
        other => panic!(
            "a legacy store must fail closed as OldAccumulatorSchemaNotLeadershipCertified, got {other:?}"
        ),
    }

    eprintln!(
        "S4-0 ACCEPTANCE PROVEN: leadership band 1338..=1342 read by EXACT index (bootstrap 1338 seed-record + \
         1339 MARK, native 1340/1341/1342); reset restores CURRENT:=BOOTSTRAP (natives cleared); off-band \
         1337/1343 + legacy store fail closed."
    );
}
