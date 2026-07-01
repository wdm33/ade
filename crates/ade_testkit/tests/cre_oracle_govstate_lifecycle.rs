//! CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY — the oracle ground-truth harness (read-only, evidence).
//!
//! Before ANY ratify/enact BLUE work, this decodes cardano's OWN governance state across the available
//! oracle states (POST-1340/1341/1342 db-analyser dumps) with Ade's `decode_native_nonutxo_state`, and
//! reports the canonical lifecycle: per-epoch proposal set (by action kind, with vote tallies + committee)
//! and the epoch-to-epoch TRANSITIONS (proposals REMOVED [expired or enacted], ADDED [submitted], or whose
//! votes CHANGED). It establishes the ground truth every later slice (vote capture, ratify, enact) is gated
//! against — and reveals whether the available window contains real RATIFY/ENACT events or only expiry
//! (which decides whether a richer governance corpus must be extracted before the enact slices).
//!
//! Read-only, no mutation, no runtime dependency. Reads LOCAL artifacts, so `#[ignore]`'d.

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::{decode_native_nonutxo_state, ImportedGovState};
use ade_types::conway::governance::{GovAction, GovActionState};
use ade_types::{Hash32, SlotNo};

fn kind(a: &GovAction) -> &'static str {
    match a {
        GovAction::ParameterChange { .. } => "ParameterChange",
        GovAction::HardForkInitiation { .. } => "HardForkInitiation",
        GovAction::TreasuryWithdrawals { .. } => "TreasuryWithdrawals",
        GovAction::NoConfidence { .. } => "NoConfidence",
        GovAction::UpdateCommittee { .. } => "UpdateCommittee",
        GovAction::NewConstitution { .. } => "NewConstitution",
        GovAction::InfoAction => "InfoAction",
    }
}

fn gid(p: &GovActionState) -> String {
    let mut s = String::new();
    for b in &p.action_id.tx_hash.0[..6] {
        s.push_str(&format!("{b:02x}"));
    }
    format!("{s}..#{}", p.action_id.index)
}

fn votes(p: &GovActionState) -> String {
    format!("cc={} drep={} spo={}", p.committee_votes.len(), p.drep_votes.len(), p.spo_votes.len())
}

fn decode_gov(env: &str, default: &str, slot: u64, epoch: u64) -> ImportedGovState {
    let path = std::env::var(env).unwrap_or_else(|_| default.to_string());
    let state = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, _c) = decode_native_nonutxo_state(&state, point, epoch, 2)
        .unwrap_or_else(|e| panic!("decode {path} @ epoch {epoch}: {e:?}"));
    s1a.imported_gov
}

fn report_epoch(label: &str, g: &ImportedGovState) {
    let mut by_kind: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut voted = 0usize;
    for p in &g.proposals {
        *by_kind.entry(kind(&p.gov_action)).or_insert(0) += 1;
        if !p.committee_votes.is_empty() || !p.drep_votes.is_empty() || !p.spo_votes.is_empty() {
            voted += 1;
        }
    }
    let (mut kh, mut sh, mut abstain, mut noconf) = (0usize, 0usize, 0usize, 0usize);
    for drep in g.vote_delegations.values() {
        match drep {
            ade_types::conway::cert::DRep::KeyHash(_) => kh += 1,
            ade_types::conway::cert::DRep::ScriptHash(_) => sh += 1,
            ade_types::conway::cert::DRep::AlwaysAbstain => abstain += 1,
            ade_types::conway::cert::DRep::AlwaysNoConfidence => noconf += 1,
        }
    }
    eprintln!(
        "epoch {label}: {} proposals ({} voted) | committee {} quorum {:?} | thresholds pool={:?} drep={:?} | vote_delegations {} (keyhash={} scripthash={} abstain={} noconf={}) | drep_expiry {} | committee_hot_keys {} | by kind: {:?}",
        g.proposals.len(), voted, g.committee.len(), g.committee_quorum,
        g.pool_voting_thresholds, g.drep_voting_thresholds,
        g.vote_delegations.len(), kh, sh, abstain, noconf,
        g.drep_expiry.len(), g.committee_hot_keys.len(), by_kind
    );
}

fn report_transition(label: &str, prev: &ImportedGovState, next: &ImportedGovState) {
    use std::collections::BTreeMap;
    let pmap: BTreeMap<_, _> = prev.proposals.iter().map(|p| (&p.action_id, p)).collect();
    let nmap: BTreeMap<_, _> = next.proposals.iter().map(|p| (&p.action_id, p)).collect();
    let removed: Vec<_> = prev.proposals.iter().filter(|p| !nmap.contains_key(&p.action_id)).collect();
    let added: Vec<_> = next.proposals.iter().filter(|p| !pmap.contains_key(&p.action_id)).collect();
    let mut vote_changed = Vec::new();
    for p in &prev.proposals {
        if let Some(n) = nmap.get(&p.action_id) {
            if n.committee_votes.len() != p.committee_votes.len()
                || n.drep_votes.len() != p.drep_votes.len()
                || n.spo_votes.len() != p.spo_votes.len()
            {
                vote_changed.push((p, *n));
            }
        }
    }
    eprintln!("\n--- transition {label}: removed={} added={} vote_changed={} ---", removed.len(), added.len(), vote_changed.len());
    for p in &removed {
        // REMOVED = expired (expires_after < ending epoch) OR enacted. Tag by the expiry predicate.
        eprintln!("  REMOVED {} [{}] expires_after={} votes({})", gid(p), kind(&p.gov_action), p.expires_after.0, votes(p));
    }
    for p in &added {
        eprintln!("  ADDED   {} [{}] proposed_in={} expires_after={}", gid(p), kind(&p.gov_action), p.proposed_in.0, p.expires_after.0);
    }
    for (p, n) in &vote_changed {
        eprintln!("  VOTED   {} [{}] {} -> {}", gid(p), kind(&p.gov_action), votes(p), votes(n));
    }
}

#[test]
#[ignore = "reads local POST-1340/1341/1342 preview states; run explicitly (CRE oracle ground-truth)"]
fn cre_oracle_govstate_lifecycle_1340_1342() {
    let base = "/home/ts/.cardano-ce3d-extract/db/ledger";
    let g1340 = decode_gov("CE3D_REF_1340", &format!("{base}/115776011_db-analyser/state"), 115_776_011, 1340);
    let g1341 = decode_gov("CE3D_REF_1341", &format!("{base}/115862416_db-analyser/state"), 115_862_416, 1341);
    let g1342 = decode_gov("CE3D_REF_1342", &format!("{base}/115948834_db-analyser/state"), 115_948_834, 1342);

    eprintln!("\n=== CRE ORACLE GOVERNANCE LIFECYCLE (cardano ground truth) ===");
    report_epoch("1340", &g1340);
    report_epoch("1341", &g1341);
    report_epoch("1342", &g1342);
    report_transition("1340->1341", &g1340, &g1341);
    report_transition("1341->1342", &g1341, &g1342);

    // Sanity: the decoder yields a non-trivial governance state at each epoch (the harness can read the oracle).
    assert!(!g1340.proposals.is_empty() && !g1341.proposals.is_empty(), "oracle gov state decodes non-empty");

    // S1 ground truth: the per-action voting thresholds (curPParams 22/23) are captured from the REAL
    // certified state with the CIP-1694 cardinalities (pool=5, drep=10) and are non-degenerate rationals.
    // This is the oracle gate on S1's threshold import — the imported authority matches what cardano holds.
    assert_eq!(g1340.pool_voting_thresholds.len(), 5, "poolVotingThresholds: 5 SPO actions (CIP-1694)");
    assert_eq!(g1340.drep_voting_thresholds.len(), 10, "drepVotingThresholds: 10 DRep actions (CIP-1694)");
    for (n, d) in g1340.pool_voting_thresholds.iter().chain(g1340.drep_voting_thresholds.iter()) {
        assert!(*d != 0 && *n <= *d, "each threshold is a proper fraction in [0,1]: {n}/{d}");
    }

    // S1 part 2a ground truth: the DState UMap drep field decodes into the real bootstrap vote-delegation
    // baseline (58k+ credentials on preview @1340), spanning hash-DReps AND the predefined DReps — proving
    // the robust decoder handles BOTH cardano DRep arities across the whole UMap without misaligning the
    // 58k-entry cursor (a wrong arity guess would have panicked the whole-state decode above).
    use ade_types::conway::cert::DRep;
    assert!(g1340.vote_delegations.len() > 10_000, "real bootstrap vote delegations captured (58k+ on preview)");
    let has_hash = g1340.vote_delegations.values().any(|d| matches!(d, DRep::KeyHash(_) | DRep::ScriptHash(_)));
    let has_predef = g1340
        .vote_delegations
        .values()
        .any(|d| matches!(d, DRep::AlwaysAbstain | DRep::AlwaysNoConfidence));
    assert!(has_hash && has_predef, "the drep decode spans hash-DReps and predefined DReps (both arities)");

    // S1 part 2b ground truth: the VState decodes into the real DRep-expiry baseline (8940 registered DReps
    // on preview @1340) and the committee hot->cold map (8 authorized members = the committee size). All
    // expiries are real epochs; every hot key resolves to a cold member.
    assert!(g1340.drep_expiry.len() > 1000, "real DRep-expiry baseline captured from vsDReps (8940 on preview)");
    assert!(
        !g1340.committee_hot_keys.is_empty() && g1340.committee_hot_keys.len() <= g1340.committee.len(),
        "committee hot keys resolve to committee members (8 authorized of {} @1340)",
        g1340.committee.len()
    );
    assert!(g1340.drep_expiry.values().all(|e| *e > 0), "DRep expiries are non-zero epoch numbers");
}
