//! CONWAY-PROPOSAL-DEPOSIT-EXPIRY S4.0 — the RATIFICATION CENSUS (read-only, evidence-only).
//!
//! Before any S4 boundary refund MUTATION exists, this census proves — over the WHOLE tracked governance
//! set at the exact CE-3d target boundary — whether Ade's CURRENT (committee-only) ratification authority
//! is sufficient: i.e. whether every tracked proposal falls in an explicitly SAFE terminal category
//! (`PresentGateFailed` = provably unratifiable via a present failed gate, or `InfoActionNeverEnacts`), and
//! NONE is `PotentiallyRatifiable` (would need un-imported DRep/SPO threshold + stake authority) or
//! `Malformed`. If the census is clean, the narrow S4 committee-gate evaluator is admissible.
//!
//! It is GREEN/test authority: it MUTATES nothing and adds NO runtime dependency. It exercises the REAL
//! `governance::check_ratification` path via the observe-only `proposal_ratification_observation` (it does
//! not restate ratification logic), and produces a canonical, GovActionId-sorted report that is RETAINED
//! as committed evidence (the golden at `docs/clusters/.../cpde-s4-0-ratification-census.txt`) — bound to
//! the exact state by its GovActionIds. Regenerate with `REGEN_CPDE_S4_0_GOLDEN=1`.
//!
//! Target: the 1340 -> 1341 boundary. The ratification check runs as of the ENDING epoch (1340); a
//! proposal expires when `expires_after < ending_epoch` (1340). Reads a LOCAL extraction artifact, so it
//! is `#[ignore]`'d; the extraction is EVIDENCE INPUT to the census only, never live authority.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::governance::proposal_ratification_observation;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_ledger::rational::Rational;
use ade_types::conway::governance::{GovAction, GovActionState};
use ade_types::{Hash32, SlotNo};

/// The epoch that is ENDING at the target 1340 -> 1341 boundary (ratification evaluates as of this epoch;
/// `expires_after < ENDING_EPOCH` is the Conway removal predicate).
const ENDING_EPOCH: u64 = 1340;
/// The certified POST-1340 ledger-state point slot.
const POST_1340_SLOT: u64 = 115_776_011;
/// The retained evidence golden (committed; bound to the exact state by its GovActionIds).
const GOLDEN_REL: &str = "docs/clusters/CONWAY-PROPOSAL-DEPOSIT-EXPIRY/cpde-s4-0-ratification-census.txt";

fn action_kind(a: &GovAction) -> &'static str {
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

fn gov_action_id_hex(p: &GovActionState) -> String {
    let mut s = String::with_capacity(72);
    for b in &p.action_id.tx_hash.0 {
        let _ = write!(s, "{b:02x}");
    }
    let _ = write!(s, "#{}", p.action_id.index);
    s
}

#[test]
#[ignore = "reads the local POST-1340 preview state; run explicitly (S4.0 census gate)"]
fn cpde_s4_0_ratification_census_committee_authority_resolves_whole_set() {
    let path = std::env::var("CE3D_REF_1340").unwrap_or_else(|_| {
        "/home/ts/.cardano-ce3d-extract/db/ledger/115776011_db-analyser/state".to_string()
    });
    let state = std::fs::read(&path).expect("POST-1340 state");
    let point = SeedPoint { slot: SlotNo(POST_1340_SLOT), block_hash: Hash32([0u8; 32]) };
    let (s1a, _commit) =
        decode_native_nonutxo_state(&state, point, 1340, 2).expect("decode POST-1340 native state");
    let g = &s1a.imported_gov;

    // Ade's CURRENT governance authority at the boundary: committee + quorum are IMPORTED; the DRep/SPO
    // voting thresholds, the DRep stake distribution, drep_expiry, and committee_hot_keys are NOT (empty)
    // — exactly what the S4 evaluator would see today.
    let (qn, qd) = g.committee_quorum.expect("imported committee quorum");
    let quorum = Rational::new(qn as i128, qd as i128).expect("non-zero quorum denominator");
    let empty_drep_stake = BTreeMap::new();
    let empty_pool_stake = BTreeMap::new();
    let empty_hot_keys = BTreeMap::new();
    let empty_drep_expiry = BTreeMap::new();
    let pool_thresholds: &[(u64, u64)] = &[];
    let drep_thresholds: &[(u64, u64)] = &[];

    // Canonical order: GovActionId.
    let mut proposals: Vec<&GovActionState> = g.proposals.iter().collect();
    proposals.sort_by(|a, b| a.action_id.cmp(&b.action_id));

    let mut report = String::new();
    let _ = writeln!(report, "# CPDE-S4.0 RATIFICATION CENSUS (observe-only; the real check_ratification path)");
    let _ = writeln!(report, "# state: certified POST-1340 preview ledger state, point slot {POST_1340_SLOT}");
    let _ = writeln!(report, "# boundary: 1340 -> 1341 (ratification evaluated as of ending_epoch {ENDING_EPOCH})");
    let _ = writeln!(
        report,
        "# authority: committee {} members / quorum {qn}/{qd} (IMPORTED); \
         drep_thresholds=[] pool_thresholds=[] drep_stake=[] (NOT imported)",
        g.committee.len()
    );
    let _ = writeln!(
        report,
        "# fields: gov_action_id | kind | proposed_in | expires_after | expiring | \
         committee_active/size | committee_yes | drep_inputs | spo_inputs | observation"
    );

    let mut present_gate_failed = 0usize;
    let mut info_never = 0usize;
    let mut potentially_ratifiable = 0usize;
    let mut malformed = 0usize;
    let mut refunds = 0usize;

    for p in &proposals {
        let obs = proposal_ratification_observation(
            p,
            &empty_drep_stake,
            &empty_pool_stake,
            &g.committee,
            &quorum,
            pool_thresholds,
            drep_thresholds,
            ENDING_EPOCH,
            &empty_hot_keys,
            &empty_drep_expiry,
        );
        let expiring = p.expires_after.0 < ENDING_EPOCH;
        let malformed_repr = p.return_addr.len() != 29;
        let observation = if malformed_repr {
            malformed += 1;
            "Malformed"
        } else if obs.is_info_action {
            info_never += 1;
            "InfoActionNeverEnacts"
        } else if obs.potentially_ratifiable {
            potentially_ratifiable += 1;
            "PotentiallyRatifiable"
        } else {
            present_gate_failed += 1;
            if expiring {
                refunds += 1;
            }
            "PresentGateFailed"
        };
        let _ = writeln!(
            report,
            "{} | {} | {} | {} | {} | {}/{} | {} | {} | {} | {}",
            gov_action_id_hex(p),
            action_kind(&p.gov_action),
            p.proposed_in.0,
            p.expires_after.0,
            if expiring { "Y" } else { "N" },
            obs.committee_active_members,
            obs.committee_size,
            obs.committee_yes,
            if obs.drep_inputs_present { "Y" } else { "N" },
            if obs.spo_inputs_present { "Y" } else { "N" },
            observation,
        );
    }

    let clean = potentially_ratifiable == 0 && malformed == 0;
    let _ = writeln!(
        report,
        "# summary: PresentGateFailed={present_gate_failed} InfoActionNeverEnacts={info_never} \
         PotentiallyRatifiable={potentially_ratifiable} Malformed={malformed} | expiring_refunds={refunds}"
    );
    let _ = writeln!(
        report,
        "# verdict: {}",
        if clean {
            "CENSUS CLEAN — the committee-only authority resolves every tracked proposal; narrow S4 admissible"
        } else {
            "S4 BLOCKED — a tracked proposal is not in a safe terminal category; close the import gap first"
        }
    );

    // Retain the canonical report as committed evidence (golden), bound to the exact state by its
    // GovActionIds. `REGEN_CPDE_S4_0_GOLDEN=1` rewrites it; otherwise the run must REPRODUCE it byte-for-byte.
    let golden_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(GOLDEN_REL);
    if std::env::var("REGEN_CPDE_S4_0_GOLDEN").is_ok() {
        std::fs::write(&golden_path, &report).expect("write census golden");
        eprintln!("REGENERATED census golden: {}", golden_path.display());
    } else {
        let golden = std::fs::read_to_string(&golden_path).unwrap_or_else(|e| {
            panic!(
                "missing committed census golden {} ({e}); regenerate with REGEN_CPDE_S4_0_GOLDEN=1",
                golden_path.display()
            )
        });
        assert_eq!(
            report, golden,
            "the census report drifted from the committed golden evidence (the certified state or the \
             ratification authority changed) — re-review before regenerating"
        );
    }

    eprintln!("\n{report}");

    // Decisive whole-set assertion: NO tracked proposal is PotentiallyRatifiable or Malformed.
    assert!(
        clean,
        "S4 BLOCKED: {potentially_ratifiable} potentially-ratifiable + {malformed} malformed tracked \
         proposal(s) — the committee-only authority is INSUFFICIENT for the whole set; close the threshold \
         + DRep-stake import gap before any S4 mutation code"
    );
    // The committee must be ACTIVE at the boundary for the committee gate to PROVE anything (flag #2).
    let active_committee = g.committee.values().filter(|e| **e >= ENDING_EPOCH).count();
    assert!(
        active_committee > 0,
        "the constitutional committee has NO members active at epoch {ENDING_EPOCH} — the committee gate \
         would SKIP and the negative proof would NOT be established"
    );
    // Exactly the five expiring TreasuryWithdrawals are refund-eligible (the CE-3d -500B accounts).
    assert_eq!(refunds, 5, "expected exactly 5 expiring + provably-unratifiable refunds");
}
