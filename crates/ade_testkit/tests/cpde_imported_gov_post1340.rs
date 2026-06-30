//! CONWAY-PROPOSAL-DEPOSIT-EXPIRY S1 — the POST-1340 real-data import proof.
//!
//! Decodes the certified preview POST-1340 (`115776011`) `db-analyser` ledger state with Ade's own
//! `decode_native_nonutxo_state` and asserts the imported Conway governance state byte-exactly:
//!   - 50 live proposals; committee = 8 members / quorum 2/3;
//!   - the FIVE expiring (`expires_after=1339`) `TreasuryWithdrawals` proposals — deposit 100k ADA,
//!     returning to acct1 ×4 / acct2 ×1, the one contested proposal carrying 2 DRep + 0 committee votes
//!     (so its committee gate provably fails ⇒ refund).
//!
//! Reads a LOCAL extraction artifact (not a committed fixture), so it is `#[ignore]`'d; run explicitly.

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_types::conway::governance::GovAction;
use ade_types::{Hash32, SlotNo};

fn h28(hex: &str) -> Vec<u8> {
    (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect()
}

#[test]
#[ignore = "reads the local POST-1340 preview state; run explicitly"]
fn cpde_imported_gov_post_1340_byte_exact() {
    let path = std::env::var("CE3D_REF_1340").unwrap_or_else(|_| {
        "/home/ts/.cardano-ce3d-extract/db/ledger/115776011_db-analyser/state".to_string()
    });
    let state = std::fs::read(&path).expect("POST-1340 state");
    let point = SeedPoint { slot: SlotNo(115_776_011), block_hash: Hash32([0u8; 32]) };
    let (s1a, _commit) =
        decode_native_nonutxo_state(&state, point, 1340, 2).expect("decode POST-1340 native state");
    let g = &s1a.imported_gov;

    assert_eq!(g.proposals.len(), 50, "POST-1340 live cgsProposals count");
    assert_eq!(g.committee.len(), 8, "constitutional committee members");
    assert_eq!(g.committee_quorum, Some((2, 3)), "committee quorum 2/3");

    let acct1 = h28("ceb13422f661e2ecb6cdffedb71aea95053d66cd527cc7ed55d976b4");
    let acct2 = h28("f53256bcaa4c5e36a48b3863069cc6e0e8a6ec7a4eff702ac662a4cb");
    let ret = |p: &ade_types::conway::governance::GovActionState| p.return_addr[p.return_addr.len() - 28..].to_vec();

    let targets: Vec<_> = g.proposals.iter().filter(|p| p.expires_after.0 == 1339).collect();
    assert_eq!(targets.len(), 5, "five proposals expire at the 1340->1341 boundary");
    for p in &targets {
        assert!(matches!(p.gov_action, GovAction::TreasuryWithdrawals { .. }), "TreasuryWithdrawals");
        assert_eq!(p.deposit.0, 100_000_000_000, "deposit = 100k ADA");
        assert_eq!(p.proposed_in.0, 1309, "proposed_in");
        let r = ret(p);
        assert!(r == acct1 || r == acct2, "returns to acct1 or acct2");
    }
    assert_eq!(targets.iter().filter(|p| ret(p) == acct1).count(), 4, "4 -> acct1 (+400k)");
    assert_eq!(targets.iter().filter(|p| ret(p) == acct2).count(), 1, "1 -> acct2 (+100k)");

    // The one contested proposal: 2 DRep votes, 0 committee votes ⇒ committee gate provably fails.
    let contested: Vec<_> = targets.iter().filter(|p| !p.drep_votes.is_empty()).collect();
    assert_eq!(contested.len(), 1, "exactly one contested proposal");
    assert_eq!(contested[0].drep_votes.len(), 2, "2 DRep votes");
    assert!(contested[0].committee_votes.is_empty(), "0 committee votes ⇒ committee gate fails");
}
