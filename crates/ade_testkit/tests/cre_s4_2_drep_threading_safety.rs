//! CRE S4.2 safety gate: threading the DRep/committee authority into the live gate must NOT turn any CE-3d
//! proposal into a potentially-ratifiable HALT or a wrongful refund. Re-runs the CE-3d 1340->1341 census with
//! the REAL imported DRep authority (thresholds + derived stake + expiry + hot keys, + the Bound num_dormant)
//! and EMPTY SPO (S4.2 leaves the non-monotone SPO gate inert until S4.3), and requires
//! `potentially_ratifiable == 0` — the SAME result the empty-threshold `cpde_s4_0` census proves. If it holds,
//! threading the DRep authority is a safe flip (no proposal that was provably-unratifiable becomes ratifiable,
//! and none becomes a wrongful refund). #[ignore] (local artifacts).

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::governance::{derive_drep_voting_stake, proposal_ratification_observation};
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_ledger::rational::Rational;
use ade_ledger::state::DormantEpochs;
use ade_types::{Hash32, SlotNo};
use std::collections::BTreeMap;

const POST_1340_SLOT: u64 = 115_776_011;
const POST_1340_STATE: &str = "/home/ts/.cardano-ce3d-extract/db/ledger/115776011_db-analyser/state";
// The live node's own bootstrap source (S4.1b): the verified Preview snapshot @ epoch 1338.
const PREVIEW_SEED_SLOT: u64 = 115_676_685;
const PREVIEW_SEED_STATE: &str = "/home/ts/.cardano-preview-judge/preview-snapshot/db/ledger/115676685/state";

/// Run the observe-only census with the REAL DRep authority + EMPTY SPO and return the potentially-ratifiable
/// count. `ending_epoch` is the boundary being evaluated.
fn potentially_ratifiable_with_drep(state_path: &str, slot: u64, epoch: u64, ending_epoch: u64) -> (usize, usize) {
    let state = std::fs::read(state_path).unwrap_or_else(|e| panic!("read {state_path}: {e}"));
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, _) = decode_native_nonutxo_state(&state, point, epoch, 2).expect("decode state");
    let g = &s1a.imported_gov;

    // The REAL imported DRep/committee authority that S4.2 threads; SPO stays EMPTY (deferred to S4.3).
    let drep_stake = derive_drep_voting_stake(&g.vote_delegations, &s1a.snapshots.mark.0);
    let (qn, qd) = g.committee_quorum.expect("imported quorum");
    let quorum = Rational::new(qn as i128, qd as i128).expect("non-zero quorum");
    let num_dormant = DormantEpochs::Bound(g.num_dormant_epochs);
    let empty_pool_stake = BTreeMap::new();
    let empty_pool_thresholds: &[(u64, u64)] = &[]; // SPO gate inert under S4.2

    let mut potentially_ratifiable = 0usize;
    let mut drep_active = 0usize;
    for p in &g.proposals {
        let obs = proposal_ratification_observation(
            p,
            &drep_stake,
            &empty_pool_stake,
            &g.committee,
            &quorum,
            empty_pool_thresholds,      // SPO threshold EMPTY (S4.2)
            &g.drep_voting_thresholds,  // REAL DRep thresholds
            ending_epoch,
            &g.committee_hot_keys,      // REAL committee hot keys
            &g.drep_expiry,             // REAL drep_expiry
            &num_dormant,               // Bound (S4.1)
        )
        .expect("observe (Bound dormancy)");
        if obs.potentially_ratifiable {
            potentially_ratifiable += 1;
        }
        if obs.drep_inputs_present {
            drep_active += 1;
        }
    }
    eprintln!(
        "  state epoch {epoch} (boundary ->{}) : {} proposals | potentially_ratifiable={} | drep_inputs_present={}",
        ending_epoch + 1,
        g.proposals.len(),
        potentially_ratifiable,
        drep_active,
    );
    (potentially_ratifiable, drep_active)
}

/// S4.2 SAFETY GATE: with the real DRep/committee authority threaded live and SPO empty, NO proposal becomes
/// potentially-ratifiable — for BOTH the CE-3d POST-1340 corpus AND the live node's own bootstrap source (the
/// Preview snapshot @ epoch 1338). So threading the DRep gate neither introduces a boundary HALT nor a
/// wrongful refund on either the oracle corpus or the store S4.1b actually produced.
#[test]
#[ignore = "reads local POST-1340 + preview-snapshot states; CRE S4.2 DRep-threading safety gate"]
fn cre_s4_2_threading_drep_introduces_no_halt_or_wrongful_refund() {
    eprintln!("=== CRE S4.2 DREP-THREADING SAFETY (real DRep authority, empty SPO) ===");
    // CE-3d oracle corpus: the 1340->1341 boundary.
    let (pr_ce3d, active_ce3d) =
        potentially_ratifiable_with_drep(POST_1340_STATE, POST_1340_SLOT, 1340, 1340);
    assert!(active_ce3d > 0, "the DRep gate is actually active on the CE-3d set (not silently skipped)");
    assert_eq!(pr_ce3d, 0, "CE-3d: threading DRep keeps the whole set provably-unratifiable");

    // The live node's own seed: the Preview snapshot @ epoch 1338, evaluated at its 1338->1339 boundary.
    let (pr_seed, active_seed) =
        potentially_ratifiable_with_drep(PREVIEW_SEED_STATE, PREVIEW_SEED_SLOT, 1338, 1338);
    assert!(active_seed > 0, "the DRep gate is genuinely active on the live seed too (not silently skipped)");
    assert_eq!(
        pr_seed, 0,
        "the live V2 seed @1338 stays provably-unratifiable under DRep threading — the node does NOT halt at \
         the 1338->1339 boundary (S4.2 introduces no continuity blocker; S4.3 handles genuine ratification)"
    );
}

/// S4.2 refund-set differential (the mechanical version of the safety argument): plan the CE-3d 1340->1341
/// boundary refunds with the CURRENT live authority (empty DRep + empty committee_hot_keys) and with the S4.2
/// live authority (threaded DRep + committee_hot_keys), SPO empty in BOTH, and assert the two `RefundPlan`s are
/// BYTE-IDENTICAL. This converts "potentially_ratifiable==0 ⇒ same refund set" from an argument into a check.
#[test]
#[ignore = "reads the local POST-1340 state; CRE S4.2 refund-set differential"]
fn cre_s4_2_refund_plan_is_identical_before_and_after_drep_threading() {
    use ade_ledger::governance::plan_deposit_refunds;
    let state = std::fs::read(POST_1340_STATE).unwrap_or_else(|e| panic!("read: {e}"));
    let point = SeedPoint { slot: SlotNo(POST_1340_SLOT), block_hash: Hash32([0u8; 32]) };
    let (s1a, _) = decode_native_nonutxo_state(&state, point, 1340, 2).expect("decode");
    let g = &s1a.imported_gov;
    let (qn, qd) = g.committee_quorum.expect("quorum");
    let quorum = Rational::new(qn as i128, qd as i128).expect("quorum");
    let dormant = DormantEpochs::Bound(g.num_dormant_epochs);
    let empty_pool_stake = BTreeMap::new();
    let no_spo: &[(u64, u64)] = &[];

    // (a) CURRENT live boundary: DRep authority + committee_hot_keys EMPTY (committee + quorum seeded).
    let plan_before = plan_deposit_refunds(
        &g.proposals, &BTreeMap::new(), &empty_pool_stake, &g.committee, &quorum, no_spo, &[], 1341,
        &BTreeMap::new(), &BTreeMap::new(), &dormant,
    )
    .expect("pre-S4.2 plan is clean");

    // (b) S4.2 live boundary: real DRep authority + committee_hot_keys threaded; SPO still empty.
    let drep_stake = derive_drep_voting_stake(&g.vote_delegations, &s1a.snapshots.mark.0);
    let plan_after = plan_deposit_refunds(
        &g.proposals, &drep_stake, &empty_pool_stake, &g.committee, &quorum, no_spo,
        &g.drep_voting_thresholds, 1341, &g.committee_hot_keys, &g.drep_expiry, &dormant,
    )
    .expect("S4.2 plan is clean (no PotentiallyRatifiable)");

    assert_eq!(
        plan_before, plan_after,
        "threading the DRep/committee authority produces a BYTE-IDENTICAL refund plan — S4.2 is a provable \
         no-op on the CPDE refund set"
    );
    eprintln!("CRE S4.2 differential: refund plan identical ({} removed) before/after DRep threading", plan_after.removed.len());
}
