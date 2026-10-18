//! SEED-BOUNDARY REWARD CENSUS — read-only, evidence-only, `#[ignore]`d by default (SLICE LV-2, E1).
//!
//! Question: does Ade's seed boundary, crossed exactly as sealed — the `Native` seed reward authority
//! (LV-2), the seed state's go snapshot, `nesBprev` and the snapshot's `ssFee` — reproduce cardano's
//! Complete reward update for that boundary? Nothing is emulated: the sealed seed is crossed by the
//! production transition. It needs a store sealed by an LV-2 binary; an older store's reward record
//! is refused by the decoder, as it should be.
//!
//! The sealed bootstrap seed is read DIRECTLY from the store's meta table and decoded with the
//! production `decode_epoch_accumulator`. `EpochAccumulatorStore::open` refuses a store whose
//! semantics marker predates this binary — correct for production — but the SEED blob is imported
//! state that no boundary rule has touched, so it stays valid evidence across such bumps. The read is
//! read-only; point it at a COPY regardless, never at a live store.
//!
//! Optional assertions (LV-3, CE-LV3-4):
//!   ADE_CENSUS_EXPECT_RESERVES_DRAW=<lovelace>   the native reserves draw must equal this
//!   ADE_CENSUS_EXPECT_UNPAID_POOL=<hex28>        no credential delegated to this pool is paid
//!
//! ```text
//! ADE_CENSUS_ACC_REDB=/path/to/copy/epoch-accumulator.redb \
//! ADE_CENSUS_FEE_SS=23699540134 \
//! ADE_CENSUS_RS=/path/to/rupd-rs-by-credential.txt \      # lines: <tag 0|1> <hex28> <lovelace>
//!   cargo test --release -p ade_runtime --test seed_boundary_reward_census -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;

use ade_ledger::bootstrap_reward_update::SeedRewardAuthority;
use ade_ledger::epoch_accumulator::{
    cross_epoch_boundary, decode_epoch_accumulator, SelectedBlockCtx,
};
use ade_types::shelley::cert::StakeCredential;
use ade_types::{EpochNo, SlotNo};

fn key(c: &StakeCredential) -> String {
    let (t, h) = match c {
        StakeCredential::KeyHash(h) => (0, h),
        StakeCredential::ScriptHash(h) => (1, h),
    };
    format!("{t} {}", h.0.iter().map(|b| format!("{b:02x}")).collect::<String>())
}

#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_ACC_REDB pointing at a COPY of a real store"]
fn native_seed_boundary_reward_vs_cardano_rs() {
    let path = std::env::var("ADE_CENSUS_ACC_REDB").expect("ADE_CENSUS_ACC_REDB");
    let fee_ss: u64 = std::env::var("ADE_CENSUS_FEE_SS").expect("ADE_CENSUS_FEE_SS").parse().unwrap();
    let rs_path = std::env::var("ADE_CENSUS_RS").expect("ADE_CENSUS_RS");

    let mut reference: BTreeMap<String, u64> = BTreeMap::new();
    for line in std::fs::read_to_string(&rs_path).expect("read rs").lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        reference.insert(format!("{} {}", f[0], f[1]), f[2].parse().unwrap());
    }

    // The table and key names are duplicated from the store on purpose; a rename surfaces here as a
    // missing table/key rather than as a silent zero.
    const META_TABLE: redb::TableDefinition<&str, &[u8]> =
        redb::TableDefinition::new("epoch_acc_meta");
    let db = redb::Database::open(&path).expect("open redb copy");
    let acc = {
        let txn = db.begin_read().expect("read txn");
        let meta = txn.open_table(META_TABLE).expect("epoch_acc_meta table");
        let blob = meta
            .get("bootstrap_blob")
            .expect("read bootstrap_blob")
            .expect("the store holds a sealed bootstrap seed");
        decode_epoch_accumulator(blob.value()).expect("decode the sealed bootstrap seed")
    };
    let slot = acc.epoch_state.slot;
    let seed_epoch = acc.epoch_state.epoch;
    println!(
        "SEED slot={} epoch={} reserves={} treasury={} epoch_fees={} prev_fees={} bprev_pools={} bprev_blocks={}",
        slot.0,
        seed_epoch.0,
        acc.epoch_state.reserves.0,
        acc.epoch_state.treasury.0,
        acc.epoch_state.epoch_fees.0,
        acc.prev_epoch_fees.0,
        acc.prev_block_production.len(),
        acc.prev_block_production.values().sum::<u64>(),
    );

    // The sealed authority must be Native and carry the reference's ssFee. It is crossed AS SEALED.
    match &acc.pending_reward_update {
        Some(SeedRewardAuthority::Native(native)) => {
            println!("SEED authority=NATIVE fee_ss={}", native.fee_ss.0);
            assert_eq!(native.fee_ss.0, fee_ss, "the sealed ssFee must equal the reference's feeSS");
        }
        Some(SeedRewardAuthority::Imported(r)) => panic!(
            "this census is for a seed whose reward update was not yet computed; the sealed authority is \
             IMPORTED ({} credentials)",
            r.reward_delta.len()
        ),
        None => panic!("the sealed seed carries no seed reward authority"),
    }
    let before = acc.cert_state.delegation.rewards.clone();
    let go_deleg: BTreeMap<String, String> = acc
        .epoch_state
        .snapshots
        .as_authoritative()
        .expect("authoritative snapshots")
        .go
        .0
        .delegations
        .iter()
        .map(|(c, (pid, _))| {
            let hx = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
            (key(c), hx(&(pid.0).0))
        })
        .collect();
    let mut by_pool: BTreeMap<String, (usize, i128)> = BTreeMap::new();
    let (res0, tre0) = (acc.epoch_state.reserves.0, acc.epoch_state.treasury.0);

    let target = EpochNo(seed_epoch.0 + 1);
    let ctx = SelectedBlockCtx {
        era: acc.era,
        block_epoch: target,
        block_slot: SlotNo(slot.0 + 1),
        issuer_pool: acc.prev_block_production.keys().next().expect("a producer").clone(),
        boundary_mark: Some(BTreeMap::new()),
        active_slots_per_epoch: 21_600,
    };
    let after = cross_epoch_boundary(acc, target, &ctx).expect("cross the seed boundary natively");

    let mut ade: BTreeMap<String, i128> = BTreeMap::new();
    for (c, v) in &after.cert_state.delegation.rewards {
        let d = v.0 as i128 - before.get(c).map(|x| x.0).unwrap_or(0) as i128;
        if d != 0 {
            ade.insert(key(c), d);
        }
    }
    for (c, v) in &before {
        if !after.cert_state.delegation.rewards.contains_key(c) && v.0 != 0 {
            ade.insert(key(c), -(v.0 as i128));
        }
    }

    let (mut exact, mut differ, mut only_ref, mut only_ade) = (0usize, 0usize, 0usize, 0usize);
    let (mut sum_ref, mut sum_ade, mut sum_abs) = (0i128, 0i128, 0i128);
    for (k, r) in &reference {
        sum_ref += *r as i128;
        match ade.get(k) {
            Some(a) if *a == *r as i128 => exact += 1,
            Some(a) => {
                differ += 1;
                sum_abs += (*a - *r as i128).abs();
                if differ <= 40 {
                    println!("DIFFER {k} ade={a} cardano={r} d={}", *a - *r as i128);
                }
            }
            None => {
                only_ref += 1;
                sum_abs += *r as i128;
                if only_ref <= 40 {
                    println!("ONLY-CARDANO {k} {r}");
                }
            }
        }
    }
    for (k, a) in &ade {
        sum_ade += *a;
        if !reference.contains_key(k) {
            only_ade += 1;
            sum_abs += a.abs();
            let pool = go_deleg.get(k).cloned().unwrap_or_else(|| "NOT-IN-GO".into());
            let e = by_pool.entry(pool).or_insert((0, 0));
            e.0 += 1;
            e.1 += *a;
            if *a >= 400_000_000 {
                println!("ONLY-ADE {k} {a}");
            }
        }
    }
    for (pool, (n, sum)) in &by_pool {
        println!("ONLY-ADE-BY-GO-POOL {pool} creds={n} sum={sum}");
    }
    for k in ["4245236ab8056760efceebbff57e8cab220182be3e36439e520a6454", "7a507e54fcae6f3b497ddf679e29d170dad54905e19bbcfb7d398756", "d2cfce07b09d41fde7e4e1e4469d5f3e7b563c9870469dd93cb8ee5f"] {
        println!("DIFFER-GO-POOL {k} -> {:?}", go_deleg.get(k));
    }
    println!(
        "RESULT cardano_creds={} ade_creds={} exact={exact} differ={differ} only_cardano={only_ref} only_ade={only_ade}",
        reference.len(),
        ade.len()
    );
    println!("RESULT sum_cardano={sum_ref} sum_ade={sum_ade} sum_abs_diff={sum_abs}");
    println!(
        "RESULT reserves_draw={} treasury_gain={} prev_fees_after={} epoch_fees_after={}",
        res0 as i128 - after.epoch_state.reserves.0 as i128,
        after.epoch_state.treasury.0 as i128 - tre0 as i128,
        after.prev_epoch_fees.0,
        after.epoch_state.epoch_fees.0
    );

    if let Ok(expected) = std::env::var("ADE_CENSUS_EXPECT_RESERVES_DRAW") {
        let expected: i128 = expected.parse().expect("ADE_CENSUS_EXPECT_RESERVES_DRAW");
        assert_eq!(
            res0 as i128 - after.epoch_state.reserves.0 as i128,
            expected,
            "the native reserves draw must equal the reference's deltaR"
        );
    }
    if let Ok(pool) = std::env::var("ADE_CENSUS_EXPECT_UNPAID_POOL") {
        let paid: Vec<&String> = ade
            .iter()
            .filter(|(k, a)| **a > 0 && go_deleg.get(&k[2..]) == Some(&pool))
            .map(|(k, _)| k)
            .collect();
        assert!(
            go_deleg.values().any(|p| *p == pool),
            "the named pool has no delegators in the go snapshot — the assertion would be vacuous"
        );
        assert!(paid.is_empty(), "{} credentials of pool {pool} were paid", paid.len());
    }
}

/// LV-2 E4 (far end): the fee pots of the store's CURRENT accumulator, read directly. On a store
/// that has crossed exactly the seed boundary, `prev_epoch_fees` is what Ade rotated out of the seed
/// epoch — to be compared with the reference's `feeSS` for the following epoch.
#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_ACC_REDB pointing at a COPY of a real store"]
fn current_accumulator_fee_pots() {
    let path = std::env::var("ADE_CENSUS_ACC_REDB").expect("ADE_CENSUS_ACC_REDB");
    const META_TABLE: redb::TableDefinition<&str, &[u8]> =
        redb::TableDefinition::new("epoch_acc_meta");
    let db = redb::Database::open(&path).expect("open redb copy");
    let txn = db.begin_read().expect("read txn");
    let meta = txn.open_table(META_TABLE).expect("epoch_acc_meta table");
    for key in ["bootstrap_blob", "current_blob"] {
        let blob = meta.get(key).expect("read").expect("blob present");
        let acc = decode_epoch_accumulator(blob.value()).expect("decode");
        println!(
            "FEES {key} slot={} epoch={} epoch_fees={} prev_epoch_fees={} bcur_blocks={} bprev_blocks={} reserves={} treasury={}",
            acc.epoch_state.slot.0,
            acc.epoch_state.epoch.0,
            acc.epoch_state.epoch_fees.0,
            acc.prev_epoch_fees.0,
            acc.epoch_state.block_production.values().sum::<u64>(),
            acc.prev_block_production.values().sum::<u64>(),
            acc.epoch_state.reserves.0,
            acc.epoch_state.treasury.0,
        );
    }
}

/// Per-credential view of one pool in the store's CURRENT accumulator: every credential the MARK
/// snapshot holds for `ADE_CENSUS_MARK_POOL`, with its mark stake and its current reward balance.
/// For a credential-by-credential comparison against a reference `pstakeMark.activeStake`.
#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_ACC_REDB pointing at a COPY of a real store"]
fn mark_snapshot_credentials_for_pool() {
    let path = std::env::var("ADE_CENSUS_ACC_REDB").expect("ADE_CENSUS_ACC_REDB");
    let want = std::env::var("ADE_CENSUS_MARK_POOL").expect("ADE_CENSUS_MARK_POOL");
    const META_TABLE: redb::TableDefinition<&str, &[u8]> =
        redb::TableDefinition::new("epoch_acc_meta");
    let db = redb::Database::open(&path).expect("open redb copy");
    let txn = db.begin_read().expect("read txn");
    let meta = txn.open_table(META_TABLE).expect("epoch_acc_meta table");
    let blob = meta.get("current_blob").expect("read").expect("current blob");
    let acc = decode_epoch_accumulator(blob.value()).expect("decode");
    let hx = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    println!(
        "ACC slot={} epoch={} reserves={} treasury={}",
        acc.epoch_state.slot.0, acc.epoch_state.epoch.0, acc.epoch_state.reserves.0, acc.epoch_state.treasury.0
    );
    let mark = &acc.epoch_state.snapshots.as_authoritative().expect("authoritative").mark.0;
    let mut n = 0usize;
    let mut sum = 0u128;
    for (cred, (pid, coin)) in &mark.delegations {
        if hx(&(pid.0).0) != want {
            continue;
        }
        let h = cred.hash();
        let key = acc.cert_state.delegation.rewards.get(&StakeCredential::KeyHash(h.clone()));
        let script = acc.cert_state.delegation.rewards.get(&StakeCredential::ScriptHash(h.clone()));
        println!(
            "MARK {} {} reward_key={:?} reward_script={:?}",
            hx(&h.0),
            coin.0,
            key.map(|c| c.0),
            script.map(|c| c.0)
        );
        n += 1;
        sum += coin.0 as u128;
    }
    println!("MARKSUM pool={want} creds={n} stake={sum}");
}
