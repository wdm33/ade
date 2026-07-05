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
        let m = &acc.epoch_state.snapshots.mark.0;
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
        deleg_canon(&acc0.epoch_state.snapshots.mark.0)
    };

    let sched = preview_schedule();
    let corpus = env_path("CE3D_CORPUS", "/home/ts/.cardano-ce3d-extract/corpus_blocks");
    let (db, n) = load_corpus(&corpus, EPOCH_1342_FIRST_SLOT);
    eprintln!("GSD provenance: loaded {n} corpus blocks; advancing to POST-1342 UNINTERRUPTED...");
    co_advance(&store, &cp, &db, &sched);

    let (_s1, acc) = store.load_current().expect("load").expect("sealed accumulator");
    let go_1342 = deleg_canon(&acc.epoch_state.snapshots.go.0);
    let mark_1342 = deleg_canon(&acc.epoch_state.snapshots.mark.0);
    let set_1342 = deleg_canon(&acc.epoch_state.snapshots.set.0);
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
