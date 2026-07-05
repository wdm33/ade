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
        ("mark", &es.snapshots.mark.0),
        ("set", &es.snapshots.set.0),
        ("go", &es.snapshots.go.0),
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
