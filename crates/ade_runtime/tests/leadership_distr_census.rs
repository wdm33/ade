//! LEADERSHIP-DISTR CENSUS — read-only, evidence-only, `#[ignore]`d by default.
//!
//! Dumps the sealed `FrozenLeadershipPoolDistr` (cardano `nesPd`) an existing store holds for a
//! given leadership epoch, so Ade's σ operands can be compared against an oracle WITHOUT a re-run
//! and WITHOUT a fresh bootstrap. Opened through the crate's own `EpochAccumulatorStore` API — no
//! new tool, no reimplementation of the codec.
//!
//! ⚠ `EpochAccumulatorStore::open` is READ-WRITE. Point this at a COPY, never at a live store.
//!
//! ```text
//! ADE_CENSUS_ACC_REDB=/path/to/copy/epoch-accumulator.redb \
//! ADE_CENSUS_EPOCH=306 \
//!   cargo test -p ade_runtime --test leadership_distr_census -- --ignored --nocapture
//! ```
//!
//! Opened by: CE-B12-10's blocker (LeaderValueAboveThreshold at epoch 306 = seed+2, the FIRST
//! natively-frozen leadership epoch — seed+1 is imported verbatim by `from_mark_pool_distr`).

use ade_runtime::chaindb::EpochAccumulatorStore;
use ade_types::EpochNo;

#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_ACC_REDB pointing at a COPY of a real store"]
fn dump_frozen_leadership_for_epoch() {
    let path = match std::env::var("ADE_CENSUS_ACC_REDB") {
        Ok(p) => p,
        Err(_) => panic!("set ADE_CENSUS_ACC_REDB to a COPY of an epoch-accumulator.redb"),
    };
    let epoch: u64 = std::env::var("ADE_CENSUS_EPOCH")
        .expect("set ADE_CENSUS_EPOCH")
        .parse()
        .expect("ADE_CENSUS_EPOCH must be a u64");

    let store = EpochAccumulatorStore::open(std::path::Path::new(&path)).expect("open store copy");

    for (label, got) in [
        ("CURRENT", store.frozen_leadership_for_epoch(EpochNo(epoch))),
        (
            "BOOTSTRAP",
            store.bootstrap_frozen_leadership_for_epoch(EpochNo(epoch)),
        ),
    ] {
        let d = match got {
            Ok(Some(d)) => d,
            Ok(None) => {
                println!("[{label}] epoch {epoch}: ABSENT");
                continue;
            }
            Err(e) => {
                println!("[{label}] epoch {epoch}: ERROR {e:?}");
                continue;
            }
        };

        // The denominator is POOL-INDEPENDENT, which is what makes this census decisive without
        // knowing which header failed: `to_pool_distr_view` sums exactly these entries.
        let total: u128 = d.pools.values().map(|e| e.active_stake as u128).sum();
        let zero_stake = d.pools.values().filter(|e| e.active_stake == 0).count();

        println!("=== [{label}] frozen leadership, target epoch {epoch} ===");
        println!(
            "  target_leadership_epoch      {}",
            d.target_leadership_epoch.0
        );
        println!("  source_slot                  {}", d.source_slot.0);
        println!("  source_hash                  {}", hex(&d.source_hash.0));
        println!(
            "  source_checkpoint_commitment {}",
            hex(&d.source_checkpoint_commitment.0)
        );
        println!("  pools                        {}", d.pools.len());
        println!("  of which zero-stake          {zero_stake}");
        println!("  TOTAL ACTIVE STAKE           {total}   <- the σ denominator");
        println!(
            "  canonical_hash               {}",
            hex(&ade_ledger::frozen_leadership::canonical_hash(&d).0)
        );

        // Largest pools, so a single wrongly-included/excluded pool is visible by inspection.
        let mut by_stake: Vec<_> = d.pools.iter().collect();
        by_stake.sort_by(|a, b| {
            b.1.active_stake
                .cmp(&a.1.active_stake)
                .then(a.0 .0.cmp(&b.0 .0))
        });
        println!("  top 12 pools by stake (pool_keyhash  stake  share):");
        for (pid, e) in by_stake.iter().take(12) {
            let share = if total == 0 {
                0.0
            } else {
                (e.active_stake as f64) / (total as f64) * 100.0
            };
            println!(
                "    {}  {:>18}  {:>8.5}%",
                hex(&pid.0),
                e.active_stake,
                share
            );
        }

        // THE CONTROL for "the denominator is inflated": if a pool's NUMERATOR moved by the same
        // proportion across epochs, sigma is unchanged and an inflated denominator explains nothing.
        if let Ok(ph) = std::env::var("ADE_CENSUS_POOL") {
            let ph = ph.replace('_', "");
            let want: Vec<u8> = (0..ph.len() / 2)
                .map(|i| u8::from_str_radix(&ph[i * 2..i * 2 + 2], 16).expect("hex"))
                .collect();
            match d.pools.iter().find(|(k, _)| k.0[..] == want[..]) {
                Some((_, e)) => println!(
                    "  PROBE POOL stake             {}   share {:.8}%",
                    e.active_stake,
                    (e.active_stake as f64) / (total as f64) * 100.0
                ),
                None => println!("  PROBE POOL                   ABSENT from this distribution"),
            }
        }

        // A pool at ~0.106% is the σ implied by the observed threshold (see the CE-B12-10 evidence);
        // list that band so the failing issuer is identifiable without re-running the node.
        println!("  pools with share in [0.09%, 0.13%] (the band implied by the rejected header):");
        for (pid, e) in &d.pools {
            if total == 0 {
                break;
            }
            let share = (e.active_stake as f64) / (total as f64) * 100.0;
            if (0.09..=0.13).contains(&share) {
                println!(
                    "    {}  {:>18}  {:>8.5}%",
                    hex(&pid.0),
                    e.active_stake,
                    share
                );
            }
        }
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Identify the issuer of a rejected header from its `LeaderValueAboveThreshold` operands ALONE,
/// with no error widening and no node re-run: for every pool in the sealed distribution, ask the
/// PRODUCTION `check_leader_claim` what threshold it would produce, and report byte-equality with
/// the observed one. The threshold is a function of (asc, sigma) only, so this is exact.
///
/// The VRF output is padded with 0xFF below the observed high 8 bytes so the claim is guaranteed to
/// FAIL and hand back its threshold; the padding cannot affect the threshold itself.
///
/// ```text
/// ADE_CENSUS_ACC_REDB=<copy> ADE_CENSUS_EPOCH=306 \
/// ADE_CENSUS_THRESHOLD=0003_93da_7d18_2b1a \
///   cargo test -p ade_runtime --test leadership_distr_census \
///     identify_issuer -- --ignored --nocapture
/// ```
#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_ACC_REDB pointing at a COPY of a real store"]
fn identify_issuer_from_threshold_bytes() {
    use ade_core::consensus::{check_leader_claim, ActiveSlotsCoeff, StakeFraction};
    use ade_crypto::vrf::VrfOutput;

    let path = std::env::var("ADE_CENSUS_ACC_REDB").expect("ADE_CENSUS_ACC_REDB");
    let epoch: u64 = std::env::var("ADE_CENSUS_EPOCH")
        .expect("ADE_CENSUS_EPOCH")
        .parse()
        .unwrap();
    let want_hex = std::env::var("ADE_CENSUS_THRESHOLD")
        .expect("ADE_CENSUS_THRESHOLD (16 hex chars, underscores allowed)")
        .replace('_', "");
    let mut want = [0u8; 8];
    for i in 0..8 {
        want[i] = u8::from_str_radix(&want_hex[i * 2..i * 2 + 2], 16).expect("hex");
    }

    let store = EpochAccumulatorStore::open(std::path::Path::new(&path)).expect("open store copy");
    let d = store
        .frozen_leadership_for_epoch(EpochNo(epoch))
        .expect("read")
        .expect("present");
    let total: u64 = d.pools.values().map(|e| e.active_stake).sum();

    // preprod activeSlotsCoeff = 0.05
    let asc = ActiveSlotsCoeff {
        numer: 1,
        denom: 20,
    };
    let mut out = [0xFFu8; 64];
    out[0..8].copy_from_slice(&want);
    // Bump the high word so the claim cannot accidentally succeed at exact equality.
    let bumped = u64::from_be_bytes(want).saturating_add(1);
    out[0..8].copy_from_slice(&bumped.to_be_bytes());
    let output = VrfOutput(out);

    println!("target threshold bytes  {want:?}");
    println!("denominator (sum)       {total}");
    let mut hits = 0;
    for (pid, e) in &d.pools {
        if e.active_stake == 0 {
            continue;
        }
        let sigma = StakeFraction {
            numer: e.active_stake,
            denom: total,
        };
        if let Err(ade_core::consensus::VrfCertError::LeaderValueAboveThreshold {
            threshold, ..
        }) = check_leader_claim(&output, sigma, asc)
        {
            if threshold == want {
                hits += 1;
                println!(
                    "MATCH  pool {}  stake {}  sigma {}/{}",
                    hex(&pid.0),
                    e.active_stake,
                    e.active_stake,
                    total
                );
            }
        }
    }
    println!("matches: {hits}");
}

/// Turn the gap into TARGET NUMBERS the oracle can be checked against: given the observed
/// `value`, bisect on the PRODUCTION `check_leader_claim` for
///   (a) the smallest pool stake that would have been accepted at Ade's denominator, and
///   (b) the largest denominator that would have been accepted at Ade's numerator.
/// Whichever of those the oracle's `nesPd` actually holds names the defective operand.
///
/// ```text
/// ADE_CENSUS_ACC_REDB=<copy> ADE_CENSUS_EPOCH=306 \
/// ADE_CENSUS_VALUE=0003942465_34dddb ADE_CENSUS_POOL=0ed90e52…  \
///   cargo test -p ade_runtime --test leadership_distr_census required -- --ignored --nocapture
/// ```
#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_ACC_REDB pointing at a COPY of a real store"]
fn required_sigma_operands_for_acceptance() {
    use ade_core::consensus::{check_leader_claim, ActiveSlotsCoeff, StakeFraction};
    use ade_crypto::vrf::VrfOutput;

    let path = std::env::var("ADE_CENSUS_ACC_REDB").expect("ADE_CENSUS_ACC_REDB");
    let epoch: u64 = std::env::var("ADE_CENSUS_EPOCH")
        .expect("ADE_CENSUS_EPOCH")
        .parse()
        .unwrap();
    let value_hex = std::env::var("ADE_CENSUS_VALUE")
        .expect("ADE_CENSUS_VALUE")
        .replace('_', "");
    let pool_hex = std::env::var("ADE_CENSUS_POOL")
        .expect("ADE_CENSUS_POOL")
        .replace('_', "");

    let mut vhi = [0u8; 8];
    for i in 0..8 {
        vhi[i] = u8::from_str_radix(&value_hex[i * 2..i * 2 + 2], 16).expect("hex");
    }
    // The real VRF output's low 56 bytes are unknown. Using zeros makes `p` the SMALLEST value
    // consistent with the observed high 8 bytes, so every bound below is the most GENEROUS one --
    // i.e. a lower bound on the stake actually required. Stated, not hidden.
    let mut out = [0u8; 64];
    out[0..8].copy_from_slice(&vhi);
    let output = VrfOutput(out);
    let asc = ActiveSlotsCoeff {
        numer: 1,
        denom: 20,
    };

    let store = EpochAccumulatorStore::open(std::path::Path::new(&path)).expect("open store copy");
    let d = store
        .frozen_leadership_for_epoch(EpochNo(epoch))
        .expect("read")
        .expect("present");
    let total: u64 = d.pools.values().map(|e| e.active_stake).sum();
    let pid_bytes: Vec<u8> = (0..pool_hex.len() / 2)
        .map(|i| u8::from_str_radix(&pool_hex[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect();
    let ade_stake = d
        .pools
        .iter()
        .find(|(k, _)| k.0[..] == pid_bytes[..])
        .map(|(_, e)| e.active_stake)
        .expect("pool present in the sealed distribution");

    let accepts = |numer: u64, denom: u64| {
        check_leader_claim(&output, StakeFraction { numer, denom }, asc).is_ok()
    };

    println!("ade_stake (numerator)   {ade_stake}");
    println!("ade_total (denominator) {total}");
    println!(
        "accepts at Ade's own operands? {}",
        accepts(ade_stake, total)
    );

    // (a) smallest accepting numerator at Ade's denominator
    let (mut lo, mut hi) = (ade_stake, total);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if accepts(mid, total) {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    println!("REQUIRED pool stake at Ade's denominator  >= {lo}");
    println!(
        "  delta vs Ade                            +{} ({:.6}%)",
        lo - ade_stake,
        (lo - ade_stake) as f64 / ade_stake as f64 * 100.0
    );

    // (b) largest accepting denominator at Ade's numerator
    let (mut lo2, mut hi2) = (ade_stake, total);
    while lo2 < hi2 {
        let mid = lo2 + (hi2 - lo2).div_ceil(2);
        if accepts(ade_stake, mid) {
            lo2 = mid
        } else {
            hi2 = mid - 1
        }
    }
    println!("REQUIRED total stake at Ade's numerator    <= {lo2}");
    println!(
        "  delta vs Ade                            -{} ({:.6}%)",
        total - lo2,
        (total - lo2) as f64 / total as f64 * 100.0
    );
}
