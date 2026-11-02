//! FROZEN-RATIFICATION CENSUS (SLICE LV-15b-0 §7 step 1) — read-only, evidence-only, `#[ignore]`d.
//!
//! Dumps, from a COPY of a store stopped just after an epoch boundary, every input Ade has for that
//! epoch's frozen DRep and SPO voting distributions, plus today's `derive_drep_voting_stake` result.
//! A separate reader compares them with the cardano-node snapshot's `psDRepDistr` / `psPoolDistr`
//! and splits the difference by cause.
//!
//! ⚠ Both stores open READ-WRITE. Point this at COPIES, never at a live store.
//!
//! ```text
//! ADE_CENSUS_ACC_REDB=/copy/epoch-accumulator.redb ADE_CENSUS_CP_REDB=/copy/reduced-utxo.redb \
//!   cargo test -p ade_runtime --test frozen_ratification_census -- --ignored --nocapture > dump.txt
//! ```

use ade_runtime::chaindb::{EpochAccumulatorStore, ReducedUtxoCheckpoint};
use ade_types::conway::cert::DRep;
use ade_types::shelley::cert::StakeCredential;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn cred(c: &StakeCredential) -> String {
    match c {
        StakeCredential::KeyHash(h) => format!("key:{}", hex(&h.0)),
        StakeCredential::ScriptHash(h) => format!("script:{}", hex(&h.0)),
    }
}

fn drep(d: &DRep) -> String {
    match d {
        DRep::KeyHash(h) => format!("key:{}", hex(&h.0)),
        DRep::ScriptHash(h) => format!("script:{}", hex(&h.0)),
        DRep::AlwaysAbstain => "abstain".into(),
        DRep::AlwaysNoConfidence => "noconfidence".into(),
    }
}

#[test]
#[ignore = "evidence-only; requires ADE_CENSUS_ACC_REDB and ADE_CENSUS_CP_REDB pointing at COPIES of a real store"]
fn dump_frozen_ratification_inputs() {
    let acc_path = std::env::var("ADE_CENSUS_ACC_REDB").expect("ADE_CENSUS_ACC_REDB");
    let cp_path = std::env::var("ADE_CENSUS_CP_REDB").expect("ADE_CENSUS_CP_REDB");
    let store = EpochAccumulatorStore::open(std::path::Path::new(&acc_path)).expect("open accumulator copy");
    let (slot, acc) = store.load_current().expect("load").expect("sealed");
    let cp = ReducedUtxoCheckpoint::open(std::path::Path::new(&cp_path)).expect("open checkpoint copy");
    println!(
        "ACC applied_through_slot={} epoch={} CP last_advanced_slot={:?}",
        slot.0,
        acc.epoch_state.epoch.0,
        cp.last_advanced_slot().expect("cursor").map(|s| s.0)
    );
    let gov = acc.gov_state.as_ref().expect("Conway governance state");
    let snaps = acc.epoch_state.snapshots.as_authoritative().expect("authoritative snapshots");
    let deleg = &acc.cert_state.delegation;

    for (c, d) in &gov.vote_delegations {
        println!("VOTEDELEG {} {}", cred(c), drep(d));
    }
    for (c, e) in &gov.drep_expiry {
        println!("DREPREG {} {}", cred(c), e);
    }
    for c in deleg.registrations.keys() {
        println!("REG {}", cred(c));
    }
    for (c, coin) in &deleg.rewards {
        println!("REWARD {} {}", cred(c), coin.0);
    }
    for (c, p) in &deleg.delegations {
        println!("POOLDELEG {} {}", cred(c), hex(&p.0 .0));
    }
    for (c, (p, coin)) in &snaps.mark.0.delegations {
        println!("MARKCRED {} {} {}", cred(c), hex(&p.0 .0), coin.0);
    }
    for (p, coin) in &snaps.mark.0.pool_stakes {
        println!("MARKPOOL {} {}", hex(&p.0 .0), coin.0);
    }
    for (p, coin) in &snaps.go.0.pool_stakes {
        println!("GOPOOL {} {}", hex(&p.0 .0), coin.0);
    }
    for (c, coin) in cp.sum_base_credential_stake().expect("base stake") {
        println!("BASE {} {}", cred(&c), coin.0);
    }
    for p in &gov.proposals {
        println!(
            "PROPOSAL {}#{} {} {} cc={} drep={} spo={}",
            hex(&p.action_id.tx_hash.0),
            p.action_id.index,
            p.deposit.0,
            hex(&p.return_addr),
            p.committee_votes.len(),
            p.drep_votes.len(),
            p.spo_votes.len()
        );
    }
    for (d, s) in ade_ledger::governance::derive_drep_voting_stake(&gov.vote_delegations, &snaps.mark.0) {
        println!("ADEDREP {} {}", drep(&d), s);
    }
}
