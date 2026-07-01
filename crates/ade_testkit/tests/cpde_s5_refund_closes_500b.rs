//! CONWAY-PROPOSAL-DEPOSIT-EXPIRY S5 — direct proof the S4 refund closes the -500B CE-3d gap.
//!
//! The full `ce3d_boundary_differential` re-run needs the CE-3c SEED accumulator re-bootstrapped with the
//! current code (S1 governance import), because the existing seed predates S1 and carries NO tracked
//! proposals — so the refund has nothing to refund and the -500B persists. This test proves the closure
//! WITHOUT that heavy re-bootstrap: it decodes the REAL certified POST-1340 governance state (the same 50
//! proposals S1 imports), runs the PUBLIC S4 planner `governance::plan_deposit_refunds` over them at the
//! 1340->1341 boundary with Ade's current committee-only authority, and asserts the planned refunds are
//! EXACTLY the -500B: +400,000 ADA to acct1 (00ceb134..) and +100,000 ADA to acct2 (00f53256..).
//!
//! Reads a LOCAL extraction artifact, so it is `#[ignore]`'d; run explicitly.

use std::collections::BTreeMap;

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::governance::plan_deposit_refunds;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_ledger::rational::Rational;
use ade_types::shelley::cert::StakeCredential;
use ade_types::{Hash28, Hash32, SlotNo};

fn h28(hex: &str) -> Hash28 {
    let b: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut h = [0u8; 28];
    h.copy_from_slice(&b);
    Hash28(h)
}

#[test]
#[ignore = "reads the local POST-1340 preview state; run explicitly (S5 -500B closure proof)"]
fn cpde_s5_planner_refunds_close_the_500b_on_real_proposals() {
    let path = std::env::var("CE3D_REF_1340").unwrap_or_else(|_| {
        "/home/ts/.cardano-ce3d-extract/db/ledger/115776011_db-analyser/state".to_string()
    });
    let state = std::fs::read(&path).expect("POST-1340 state");
    let point = SeedPoint { slot: SlotNo(115_776_011), block_hash: Hash32([0u8; 32]) };
    let (s1a, _commit) =
        decode_native_nonutxo_state(&state, point, 1340, 2).expect("decode POST-1340 native state");
    let g = &s1a.imported_gov;
    assert_eq!(g.proposals.len(), 50, "the certified POST-1340 set");

    // Ade's CURRENT boundary authority: committee + quorum imported; DRep/SPO thresholds + stake empty
    // (the S4.0 census proved the committee gate resolves the whole set).
    let (qn, qd) = g.committee_quorum.expect("imported quorum");
    let quorum = Rational::new(qn as i128, qd as i128).expect("non-zero quorum");
    let empty_drep = BTreeMap::new();
    let empty_pool = BTreeMap::new();
    let empty_hot = BTreeMap::new();
    let empty_drep_expiry = BTreeMap::new();

    // Plan the refunds at the 1340 -> 1341 boundary (new_epoch = 1341, ending_epoch = 1340). The thresholds
    // are EMPTY here, matching the live boundary: CRE S1 imports + commitment-binds the voting thresholds
    // but does NOT thread them into the live gate (the SPO gate has no active-stake guard, so threading
    // would activate SPO ratification — that is the CRE ratify slice, S4). So the committee gate is the
    // binding authority for the CPDE -500B closure, exactly as on the live boundary.
    let plan = plan_deposit_refunds(
        &g.proposals,
        &empty_drep,
        &empty_pool,
        &g.committee,
        &quorum,
        &[],
        &[],
        1341,
        &empty_hot,
        &empty_drep_expiry,
        &ade_ledger::state::DormantEpochs::Unversioned,
    )
    .expect("the whole set is provably-safe -> a clean plan (no PotentiallyRatifiable)");

    // Exactly the five expiring TreasuryWithdrawals refund.
    assert_eq!(plan.removed.len(), 5, "the five expiring proposals refund");

    // The two real CE-3d return accounts (key-hash reward-account credentials).
    let acct1 = StakeCredential::KeyHash(h28("ceb13422f661e2ecb6cdffedb71aea95053d66cd527cc7ed55d976b4"));
    let acct2 = StakeCredential::KeyHash(h28("f53256bcaa4c5e36a48b3863069cc6e0e8a6ec7a4eff702ac662a4cb"));
    let (mut sum1, mut sum2) = (0u128, 0u128);
    for e in &plan.removed {
        let (cred, deposit) = e.credit.as_ref().expect("a 100k-ADA deposit");
        if *cred == acct1 {
            sum1 += deposit.0 as u128;
        } else if *cred == acct2 {
            sum2 += deposit.0 as u128;
        } else {
            panic!("unexpected refund return account: {cred:?}");
        }
    }

    // The exact -500B: +400,000 ADA to acct1 (4 proposals), +100,000 ADA to acct2 (1 proposal).
    assert_eq!(sum1, 400_000_000_000u128, "acct1 (00ceb134..) refunded +400,000 ADA");
    assert_eq!(sum2, 100_000_000_000u128, "acct2 (00f53256..) refunded +100,000 ADA");
    assert_eq!(
        sum1 + sum2,
        500_000_000_000u128,
        "the planned refunds total exactly the -500B CE-3d reward gap",
    );

    eprintln!(
        "S5 PROOF: the S4 planner refunds the real CE-3d proposals -> acct1 +{} ADA, acct2 +{} ADA \
         (total {} = the -500B gap). The full differential needs the seed re-bootstrapped with S1.",
        sum1 / 1_000_000,
        sum2 / 1_000_000,
        (sum1 + sum2) / 1_000_000,
    );
}
