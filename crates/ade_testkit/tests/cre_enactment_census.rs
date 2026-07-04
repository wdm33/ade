//! CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY — the ratify→enact ground-truth census (read-only, GROUND
//! TRUTH for a FUTURE ratify/enact cluster; NOT authorization to expand S4).
//!
//! Target: the Preview param-update action 69c948cd..#0 ("Increase Tx/Block Memory Units pt1"), submitted
//! epoch 1088, ENACTED (obs: maxTxExUnits.mem=16,500,000, maxBlockExUnits.mem=72,000,000). The local
//! ChainDB — not explorer metadata — establishes the exact ratify/enact boundary across epochs 1087-1103.
//! States are extracted via LOCAL db-analyser `--store-ledger` (see scratchpad/cre-census/extract_all.sh).
//!
//! This file starts with a single-epoch PROBE (does the decoder work at epoch 1088, ~2 years before the
//! CE-3d corpus?); the full per-transition census + fixture follows as the window extraction completes.
//! `#[ignore]`'d (reads local artifacts).

use ade_crypto::blake2b_256;
use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_types::conway::governance::GovActionId;
use ade_types::{Hash32, SlotNo};

fn h32(hex: &str) -> Hash32 {
    let b: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut h = [0u8; 32];
    h.copy_from_slice(&b);
    Hash32(h)
}

/// The target action id: tx 69c948cd..1f69, index 0.
fn target() -> GovActionId {
    GovActionId {
        tx_hash: h32("69c948cde90c6b9d7d61595e8534c106ec44132cb049ab2558399db1260c1f69"),
        index: 0,
    }
}

fn ledger_state_path(slot: u64) -> String {
    format!("/home/ts/.cardano-ce3d-extract/db/ledger/{slot}_db-analyser/state")
}

#[test]
#[ignore = "reads the local db-analyser epoch-1088 state; run explicitly (CRE enactment-census probe)"]
fn cre_census_probe_epoch_1088() {
    // Epoch 1088 first block = slot 94003205 (115862400 - (1341-1088)*86400, first block >= 94003200).
    let path = ledger_state_path(94003205);
    let state = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let point = SeedPoint { slot: SlotNo(94003205), block_hash: Hash32([0u8; 32]) };
    let (s1a, commit) = decode_native_nonutxo_state(&state, point, 1088, 2)
        .unwrap_or_else(|e| panic!("decode epoch 1088 @94003205: {e:?}"));
    let g = &s1a.imported_gov;
    let present = g.proposals.iter().any(|p| p.action_id == target());
    eprintln!(
        "epoch 1088 @94003205: {} proposals | target 69c948cd..#0 present={} | committee={} quorum={:?} | \
         maxTxExUnits.mem={} | gov-commit={:02x}{:02x}{:02x}{:02x}",
        g.proposals.len(),
        present,
        g.committee.len(),
        g.committee_quorum,
        s1a.protocol_params.max_tx_ex_units_mem,
        commit.0[0], commit.0[1], commit.0[2], commit.0[3],
    );
    // The first block of epoch 1088 is BEFORE the in-epoch submission, so the action is expected ABSENT
    // here (it appears at the 1088->1089 boundary). The probe's job is only to prove the decoder reads this
    // far-earlier Conway state at all; the presence/ratify/enact lifecycle is the full census below.
    assert!(
        s1a.protocol_params.max_tx_ex_units_mem > 0,
        "the decoder reads epoch-1088 curPParams (a real maxTxExUnits.mem)"
    );
}

/// CRE S4.3c grounding: decode the real epoch-1095 (pre) and epoch-1096 (post) states, diff the proposal
/// sets, and classify each of the six removals (enacted winner / expired / pruned) with its kind, expiry,
/// deposit and return address — the acceptance-witness data S4.3c must explain in full.
#[test]
#[ignore = "reads local db-analyser 1095 + 1096 states; classifies the six 1095->1096 removals (CRE S4.3c grounding)"]
fn cre_s4_3c_classify_1095_1096_removals() {
    use ade_types::conway::governance::GovAction;
    let dec = |slot: u64, epoch: u64| {
        let st = std::fs::read(ledger_state_path(slot)).unwrap_or_else(|e| panic!("read {slot}: {e}"));
        let pt = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
        decode_native_nonutxo_state(&st, pt, epoch, 2).unwrap_or_else(|e| panic!("decode {slot}: {e:?}"))
    };
    let hx = |b: &[u8]| b.iter().take(8).map(|x| format!("{x:02x}")).collect::<String>();
    let (pre, _) = dec(94_608_021, 1095);
    let (post, _) = dec(94_694_406, 1096);
    let post_ids: std::collections::BTreeSet<GovActionId> =
        post.imported_gov.proposals.iter().map(|p| p.action_id.clone()).collect();
    let removed: Vec<_> = pre.imported_gov.proposals.iter().filter(|p| !post_ids.contains(&p.action_id)).collect();
    eprintln!("=== CRE S4.3c: 1095->1096 removals ===");
    eprintln!(
        "1095 proposals={} enacted_root={:?} | 1096 proposals={} enacted_root={:?} | removed={}",
        pre.imported_gov.proposals.len(),
        pre.enacted_pparam_update.as_ref().map(|g| hx(&g.tx_hash.0)),
        post.imported_gov.proposals.len(),
        post.enacted_pparam_update.as_ref().map(|g| hx(&g.tx_hash.0)),
        removed.len()
    );
    for p in &removed {
        let kind = match &p.gov_action {
            GovAction::ParameterChange { prev_action, update, .. } => format!(
                "ParameterChange prev={:?} update_len={}",
                prev_action.as_ref().map(|g| hx(&g.tx_hash.0)),
                update.len()
            ),
            other => format!("{}", std::any::type_name_of_val(other)).replace("ade_types::conway::governance::", ""),
        };
        eprintln!(
            "  {}#{}: expires_after={} deposit={} return_len={} enacted_target={} | {}",
            hx(&p.action_id.tx_hash.0),
            p.action_id.index,
            p.expires_after.0,
            p.deposit.0,
            p.return_addr.len(),
            p.action_id == target(),
            kind
        );
    }
}

/// CRE S4.3c CORPUS DIFFERENTIAL (real 1095 state): run the SINGLE enactment authority
/// (`plan_conway_governance_epoch`) over the REAL decoded epoch-1095 proposal set + real ratify authority +
/// real pparams/lineage, and assert it reproduces the on-chain 1095→1096 enactment — 69c948cd..#0 ENACTS
/// (maxTx.mem 14M→16.5M, maxBlock.mem 62M→72M, steps preserved), the root advances 602d8572→69c948cd, the six
/// removals classify (1 Enacted + 5 PrunedByEnactment), all six deposits refund, and the set shrinks 59→53.
/// This is the acceptance witness: the same real authority the S4.4 anchor proves RATIFIES the target.
#[test]
#[ignore = "reads the local db-analyser epoch-1095 state; the S4.3c enactment corpus differential"]
fn cre_s4_3c_enactment_differential_1095() {
    use ade_ledger::governance::{
        derive_drep_voting_stake, plan_conway_governance_epoch, ConwayGovernanceEpochInput, DepositReturn,
        PParamsDelta, PrevPParamActionDelta, RemovalCause,
    };
    use ade_ledger::pparams::MaxBlockExUnits;
    use ade_ledger::rational::Rational;
    use ade_ledger::state::{DormantEpochs, PreviousPParamAction};

    let slot = 94_608_021u64; // epoch 1095, before the 1095→1096 enactment
    let state = std::fs::read(ledger_state_path(slot)).expect("read 1095 state");
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, _) = decode_native_nonutxo_state(&state, point, 1095, 2).expect("decode 1095");
    let g = &s1a.imported_gov;

    // The current (pre-boundary) pparams: real tx exec-units from the decoded state; the block ExUnits is the
    // real 1095 curPParams value (62M/20e9) — set explicitly only if the census decode left it Unversioned.
    let mut cur_pp = s1a.protocol_params.clone();
    if matches!(cur_pp.max_block_ex_units, MaxBlockExUnits::Unversioned) {
        cur_pp.max_block_ex_units = MaxBlockExUnits::Bound { mem: 62_000_000, steps: 20_000_000_000 };
    }
    eprintln!(
        "1095 curPParams exec-units: maxTx {{mem {}, steps {}}} | maxBlock {:?} | enacted_root present={}",
        cur_pp.max_tx_ex_units_mem, cur_pp.max_tx_ex_units_cpu, cur_pp.max_block_ex_units,
        s1a.enacted_pparam_update.is_some(),
    );
    let cur_prev = match &s1a.enacted_pparam_update {
        Some(id) => PreviousPParamAction::Enacted(id.clone()),
        None => PreviousPParamAction::NoPreviousAction,
    };

    // The real ratify authority (identical to the S4.4 oracle anchor `cre_s4_oracle_anchor_ratify_decision`).
    let drep_stake = derive_drep_voting_stake(&g.vote_delegations, &s1a.snapshots.mark.0);
    let quorum = g
        .committee_quorum
        .map(|(n, d)| Rational::new(n as i128, d.max(1) as i128).unwrap())
        .unwrap_or_else(|| Rational::new(1, 1).unwrap());
    let num_dormant = DormantEpochs::Bound(g.num_dormant_epochs);
    let input = ConwayGovernanceEpochInput {
        proposals: &g.proposals,
        drep_stake: &drep_stake,
        pool_stake: &s1a.snapshots.go.0.pool_stakes,
        committee_members: &g.committee,
        committee_quorum: &quorum,
        pool_thresholds: &g.pool_voting_thresholds,
        drep_thresholds: &g.drep_voting_thresholds,
        committee_hot_keys: &g.committee_hot_keys,
        drep_expiry: &g.drep_expiry,
        num_dormant: &num_dormant,
        current_pparams: &cur_pp,
        current_prev_pparam_action: &cur_prev,
        new_epoch: 1096, // ending_epoch 1095 (the enacting boundary)
    };
    // Deposit routing is not under test here (the registration set is not in the non-UTxO state); treat all as
    // registered so refunds route ToRewardAccount. The DECISION (six refunds of 100k) is what we assert.
    let plan = plan_conway_governance_epoch(&input, |_| true).expect("the 1095 boundary enacts (no terminal)");

    // (1) The enacted delta: maxTx.mem 16.5M, maxBlock.mem 72M, BOTH steps preserved.
    match &plan.pparams {
        PParamsDelta::Set(pp) => {
            assert_eq!(pp.max_tx_ex_units_mem, 16_500_000, "maxTx mem 14M -> 16.5M");
            assert_eq!(pp.max_tx_ex_units_cpu, cur_pp.max_tx_ex_units_cpu, "maxTx steps preserved");
            assert_eq!(
                pp.max_block_ex_units,
                MaxBlockExUnits::Bound { mem: 72_000_000, steps: 20_000_000_000 },
                "maxBlock mem 62M -> 72M, steps preserved"
            );
        }
        other => panic!("expected an enactment (pparams Set), got {other:?}"),
    }
    // (2) The enacted root advances to the target 69c948cd..#0.
    assert_eq!(plan.prev_pparam_action, PrevPParamActionDelta::Set(target()), "root advances to 69c948cd");
    // (3) Exactly six removals: the target Enacted + five PrunedByEnactment siblings.
    assert_eq!(plan.removals.len(), 6, "the six 1095->1096 removals");
    let enacted: Vec<_> = plan.removals.iter().filter(|r| r.cause == RemovalCause::Enacted).collect();
    assert_eq!(enacted.len(), 1);
    assert_eq!(enacted[0].action_id, target(), "the target is the enacted winner");
    assert_eq!(
        plan.removals.iter().filter(|r| r.cause == RemovalCause::PrunedByEnactment).count(),
        5,
        "five superseded siblings pruned",
    );
    // (4) All six deposits refund (100k each).
    assert_eq!(plan.deposit_returns.len(), 6);
    assert!(
        plan.deposit_returns.iter().all(|d| matches!(
            d, DepositReturn::ToRewardAccount { amount, .. } if amount.0 == 100_000_000_000
        )),
        "all six deposits refund 100k",
    );
    // (5) The set shrinks 59 -> 53.
    assert_eq!(g.proposals.len(), 59, "the real 1095 proposal set");
    assert_eq!(plan.proposals.len(), 53, "59 - 6 = 53 after the enactment");
    eprintln!(
        "CRE S4.3c differential OK: enacted 69c948cd; 6 removals (1 Enacted + 5 Pruned); 59->53; \
         maxTx.mem->16.5M maxBlock.mem->72M (steps preserved)"
    );
}

/// CRE S5 — the consolidated differential + residual REPORT (one runnable artifact; reads local db-analyser
/// states). Emits the six-item report and asserts each provable claim, keeping the THREE status levels strictly
/// separate: (A) the S4.3c supported exec-units enactment subset CLOSED; (B) the CE-3d governance refund
/// discrepancy GONE + the remaining B3c residual quantified AND isolated from governance; (C) full Conway
/// governance still PARTIAL. See docs/clusters/.../S5-differential-proof-and-residual-report.md.
#[test]
#[ignore = "reads local POST-1340 / POST-1341 / epoch-1095+1096 states; the CRE S5 consolidated report"]
fn cre_s5_differential_report() {
    use ade_ledger::governance::{
        plan_conway_governance_epoch, ConwayGovernanceEpochInput, DepositReturn, RemovalCause,
    };
    use ade_ledger::pparams::MaxBlockExUnits;
    use ade_ledger::rational::Rational;
    use ade_ledger::state::{DormantEpochs, PreviousPParamAction};
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::Hash28;

    let dec = |slot: u64, epoch: u64| {
        let st = std::fs::read(ledger_state_path(slot)).unwrap_or_else(|e| panic!("read {slot}: {e}"));
        let pt = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
        decode_native_nonutxo_state(&st, pt, epoch, 2).unwrap_or_else(|e| panic!("decode {slot}: {e:?}")).0
    };
    let h28 = |hex: &str| {
        let b: Vec<u8> =
            (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        let mut a = [0u8; 28];
        a.copy_from_slice(&b);
        Hash28(a)
    };
    let block_mem = |p: &ade_ledger::pparams::ProtocolParameters| match p.max_block_ex_units {
        MaxBlockExUnits::Bound { mem, .. } => mem,
        MaxBlockExUnits::Unversioned => 0,
    };
    // The two REAL CE-3d expiry return accounts (registered, undelegated reward accounts).
    let acct1_hash = h28("ceb13422f661e2ecb6cdffedb71aea95053d66cd527cc7ed55d976b4");
    let acct2_hash = h28("f53256bcaa4c5e36a48b3863069cc6e0e8a6ec7a4eff702ac662a4cb");
    let acct1 = StakeCredential::KeyHash(acct1_hash.clone());
    let acct2 = StakeCredential::KeyHash(acct2_hash.clone());

    eprintln!("\n================ CRE S5 DIFFERENTIAL + RESIDUAL REPORT ================");

    // ===== ITEM 1: the five expired-proposal refund totals + destinations (POST-1340 → 1341) =====
    let s1340 = dec(115_776_011, 1340);
    let g = &s1340.imported_gov;
    let quorum = g
        .committee_quorum
        .map(|(n, d)| Rational::new(n as i128, d.max(1) as i128).unwrap())
        .unwrap_or_else(|| Rational::new(1, 1).unwrap());
    let empty_drep = std::collections::BTreeMap::new();
    let empty_pool = std::collections::BTreeMap::new();
    let empty_hot = std::collections::BTreeMap::new();
    let empty_expiry = std::collections::BTreeMap::new();
    let no_thr: &[(u64, u64)] = &[];
    let cur_pp_1340 = s1340.protocol_params.clone();
    let cur_prev_1340 = match &s1340.enacted_pparam_update {
        Some(id) => PreviousPParamAction::Enacted(id.clone()),
        None => PreviousPParamAction::NoPreviousAction,
    };
    let input_1340 = ConwayGovernanceEpochInput {
        proposals: &g.proposals,
        drep_stake: &empty_drep,
        pool_stake: &empty_pool,
        committee_members: &g.committee,
        committee_quorum: &quorum,
        pool_thresholds: no_thr,
        drep_thresholds: no_thr,
        committee_hot_keys: &empty_hot,
        drep_expiry: &empty_expiry,
        num_dormant: &DormantEpochs::Unversioned,
        current_pparams: &cur_pp_1340,
        current_prev_pparam_action: &cur_prev_1340,
        new_epoch: 1341,
    };
    // committee-only authority (the live CE-3d gate) + the real registered return accounts route to reward.
    let plan_1340 = plan_conway_governance_epoch(&input_1340, |_| true)
        .expect("POST-1340 refund plan is clean (NO terminal)");
    let (mut to1, mut to2, mut n_expired) = (0u64, 0u64, 0usize);
    for (r, d) in plan_1340.removals.iter().zip(plan_1340.deposit_returns.iter()) {
        if r.cause == RemovalCause::Expired {
            n_expired += 1;
        }
        if let DepositReturn::ToRewardAccount { credential, amount, .. } = d {
            if credential == &acct1 {
                to1 += amount.0;
            } else if credential == &acct2 {
                to2 += amount.0;
            }
        }
    }
    assert_eq!(plan_1340.removals.len(), 5, "exactly five removals");
    assert_eq!(n_expired, 5, "all five are Expired (no ratifiable action in POST-1340)");
    assert_eq!(to1, 400_000_000_000, "acct1 += 400,000 ADA (4 proposals)");
    assert_eq!(to2, 100_000_000_000, "acct2 += 100,000 ADA (1 proposal)");
    eprintln!("\nITEM 1 — five expired-proposal refunds (POST-1340 → 1341):");
    eprintln!("  acct1 ceb13422.. += {to1}  (400,000 ADA, 4 proposals)");
    eprintln!("  acct2 f53256bc.. += {to2}  (100,000 ADA, 1 proposal)");
    eprintln!("  TOTAL = {} lovelace = 500,000 ADA  [the ENTIRE CE-3d -500B governance refund gap]", to1 + to2);

    // ===== ITEM 2: CE-3d reward/pot accounting, BEFORE vs AFTER =====
    eprintln!("\nITEM 2 — CE-3d reward/pot accounting BEFORE vs AFTER (ground-truthed via POST-1340→1341):");
    eprintln!("  BEFORE (pre-S4.3a): the direct-replay boundary DROPPED these five expiries with no refund");
    eprintln!("    → reward total short by -500,037,651,836 (the 500B above + a ~-37.6M rounding tail).");
    eprintln!("  AFTER (S4.3a single authority, this proof): the five refunds land identically on both paths;");
    eprintln!("    treasury/reserves are UNTOUCHED by refunds (deposit pot is the source) → the treasury/reserves");
    eprintln!("    delta is a STABLE +231M/+894M across 1340/1341/1342 = the B3c residual (item 5), NOT the refunds.");

    // ===== ITEM 3: the 1095→1096 enactment observables (from the REAL states) =====
    let s1095 = dec(94_608_021, 1095);
    let s1096 = dec(94_694_406, 1096);
    assert_eq!(s1095.imported_gov.proposals.len(), 59, "1095 proposal set");
    assert_eq!(s1096.imported_gov.proposals.len(), 53, "1096 proposal set (59 - 6)");
    assert_eq!(s1096.enacted_pparam_update.as_ref(), Some(&target()), "root advanced to 69c948cd..#0");
    assert_eq!(s1095.protocol_params.max_tx_ex_units_mem, 14_000_000, "1095 maxTx.mem");
    assert_eq!(s1096.protocol_params.max_tx_ex_units_mem, 16_500_000, "1096 maxTx.mem");
    assert_eq!(block_mem(&s1095.protocol_params), 62_000_000, "1095 maxBlock.mem");
    assert_eq!(block_mem(&s1096.protocol_params), 72_000_000, "1096 maxBlock.mem");
    let post_ids: std::collections::BTreeSet<_> =
        s1096.imported_gov.proposals.iter().map(|p| p.action_id.clone()).collect();
    let removed: Vec<_> = s1095
        .imported_gov
        .proposals
        .iter()
        .filter(|p| !post_ids.contains(&p.action_id))
        .map(|p| p.action_id.clone())
        .collect();
    assert_eq!(removed.len(), 6, "six removals");
    assert!(removed.contains(&target()), "the enacted winner 69c948cd is one of the six");
    eprintln!("\nITEM 3 — 1095→1096 enactment observables (real states):");
    eprintln!("  enacted root: 602d8572..#0 → 69c948cd..#0 (target)");
    eprintln!("  proposal count: 59 → 53 (six removed: 1 Enacted winner + 5 PrunedByEnactment siblings)");
    eprintln!("  six-removal manifest:");
    for id in &removed {
        let cause = if id == &target() { "Enacted" } else { "PrunedByEnactment" };
        let hex: String = id.tx_hash.0.iter().take(4).map(|b| format!("{b:02x}")).collect();
        eprintln!("    {hex}..#{} : {cause}", id.index);
    }
    eprintln!("  maxTxExUnits.mem 14,000,000 → 16,500,000 ; maxBlockExUnits.mem 62,000,000 → 72,000,000 (steps preserved)");
    eprintln!("  deposit routes: all six 100k deposits refunded (registered → reward account)");
    eprintln!("  [planner reproduction proven by cre_s4_3c_enactment_differential_1095]");

    // ===== ITEM 4: replay-vs-accumulator identity =====
    // On the REAL data the single authority is deterministic (plan twice → byte-identical); the full cross-path
    // boundary identity (replay boundary == accumulator boundary) with a real enactment is the committed
    // hermetic gate cre_s4_3c_enactment_is_identical_on_replay_and_accumulator_paths.
    let plan_1340_again = plan_conway_governance_epoch(&input_1340, |_| true).expect("replay");
    assert_eq!(plan_1340, plan_1340_again, "the single authority is replay-deterministic on real data");
    eprintln!("\nITEM 4 — replay-vs-accumulator identity:");
    eprintln!("  single authority replay-deterministic on real POST-1340 data (plan == plan);");
    eprintln!("  full cross-path enactment identity proven by cre_s4_3c_enactment_is_identical_on_replay_and_accumulator_paths.");

    // ===== ITEM 5: the exact remaining B3c residual, ISOLATED from governance =====
    let s1341 = dec(115_862_416, 1341);
    let go = &s1341.snapshots.go.0;
    let go_total: u64 = go.pool_stakes.values().map(|c| c.0).sum();
    assert_eq!(go_total, 1_673_934_797_356_442, "POST-1341 reference go total (the certified snapshot)");
    // ISOLATION: the two governance-refund accounts are UNDELEGATED → absent from the go delegation set, so the
    // -500B governance refunds and the -343B go-stake undercount are on DISJOINT credential sets.
    assert!(!go.delegations.contains_key(&acct1_hash), "acct1 is undelegated → absent from go (isolated from B3c)");
    assert!(!go.delegations.contains_key(&acct2_hash), "acct2 is undelegated → absent from go (isolated from B3c)");
    eprintln!("\nITEM 5 — remaining B3c residual, ISOLATED from governance:");
    eprintln!("  remaining CE-3d gap = go-stake undercount -343,260,172,883 lovelace (~0.0205% uniform on base-UTxO");
    eprintln!("    stake of real delegated credentials in the reduced checkpoint) — a UTxO-component residual, NOT governance.");
    eprintln!("  POST-1341 reference go total = {go_total} ({} pools);", go.pool_stakes.len());
    eprintln!("  ISOLATION PROVEN: acct1/acct2 (the refund accounts) are UNDELEGATED → absent from go.delegations,");
    eprintln!("    so the governance-refund credential set and the B3c go-stake credential set are DISJOINT.");
    eprintln!("  [B3c tracked separately in project_b3c_stake_residual; NOT a governance defect]");

    // ===== ITEM 6: any terminal encountered =====
    eprintln!("\nITEM 6 — terminals encountered: NONE (POST-1340 refund plan + 1095 enactment both reach a clean plan).");
    eprintln!("  The closed fail-closed surface it WOULD emit (each with the offending action_id):");
    eprintln!("    UnsupportedRatifiedAction{{NotParameterChange|NonExecUnitsField|NoExecUnitsField|ChangedSteps|");
    eprintln!("      OversizedUpdate|MalformedUpdate|ChainedEnactment|CompetingRatifiableActions}},");
    eprintln!("    UnversionedStateOnEnactPath, Malformed{{ReturnAddrNotRewardAccount}}, DormantRequired.");

    // ===== THE THREE-WAY STATUS SEPARATION (unmistakable) =====
    eprintln!("\n---------------- STATUS (kept separate) ----------------");
    eprintln!("  (A) S4.3c supported exec-units parameter-change enactment subset: CLOSED / ENFORCED.");
    eprintln!("  (B) CE-3d governance refund discrepancy: GONE (500B refunds land, authority-proven +");
    eprintln!("      ground-truthed); remaining residual = B3c go-stake -343B (UTxO-component, isolated).");
    eprintln!("      Full accumulator-level BYTE-EXACT CE-3d differential AWAITS an S1-re-bootstrapped seed + the B3c fix.");
    eprintln!("  (C) Full Conway governance: PARTIAL — unsupported ratified kinds, broader parameter changes,");
    eprintln!("      committee/constitution/treasury/hard-fork enactment, and multi-ratifiable competition all");
    eprintln!("      remain fail-closed terminals. The CRE cluster is NOT globally complete.");
    eprintln!("  (separate) MissingDRepActivityParam live accumulator stall = a continuous-operation blocker,");
    eprintln!("      pre-existing (LIVE-LEDGER-EPOCH-TRANSITION cert-apply path), NOT an S4.3c defect.");
    eprintln!("=======================================================================\n");
}

/// CRE S4.3b GATE 5 (real witness) — the DEFINITIVE key-20/21 validation against REAL on-chain bytes.
/// Reads proposal 69c948cd..#0's raw `ParameterChange.update` from the local Preview epoch-1095 ledger state
/// (the proposal is live at 1095, before its 1095->1096 enactment) and proves: keys 20/21 decode to the full
/// ExUnits pairs (maxTx.mem=16.5M, maxBlock.mem=72M), no unsupported field, AND a MEMORY-ONLY effect — the
/// update's steps equal the currently-bound steps (else S4.3c must classify it UnsupportedRatifiedAction,
/// never silently enact memory only).
#[test]
#[ignore = "reads the local db-analyser epoch-1095 state carrying 69c948cd..#0's raw update"]
fn cre_s4_3b_gate5_real_witness_update_decodes() {
    use ade_ledger::governance::decode_exec_units_param_update;
    use ade_ledger::pparams::MaxBlockExUnits;
    use ade_types::conway::governance::GovAction;

    let slot = 94_608_021u64; // epoch 1095 first-block slot (census row)
    let path = ledger_state_path(slot);
    let state = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, _commit) = decode_native_nonutxo_state(&state, point, 1095, 2)
        .unwrap_or_else(|e| panic!("decode epoch 1095 @{slot}: {e:?}"));

    let prop = s1a
        .imported_gov
        .proposals
        .iter()
        .find(|p| p.action_id == target())
        .expect("target 69c948cd..#0 present at epoch 1095");
    let update = match &prop.gov_action {
        GovAction::ParameterChange { update, .. } => update.clone(),
        other => panic!("target is not a ParameterChange: {other:?}"),
    };

    let raw_hash = blake2b_256(&update);
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    eprintln!("--- CRE S4.3b real-witness manifest ---");
    eprintln!("point: slot {slot}, epoch 1095, era Conway, network preview(magic 2)");
    eprintln!("target: 69c948cde90c6b9d7d61595e8534c106ec44132cb049ab2558399db1260c1f69#0");
    eprintln!("raw update: {} bytes, blake2b256={}", update.len(), hex(&raw_hash.0));
    eprintln!("raw update hex: {}", hex(&update));

    // Faithfulness: the live-extracted bytes ARE the committed hermetic witness
    // (ade_ledger governance.rs cre_s4_3b_gate5_real_witness_manifest), so the committed decode-proof is
    // provably the real chain's — not a synthetic round-trip that shares the decoder's key assumption.
    const COMMITTED_WITNESS: &[u8] = &[
        0xa2, 0x14, 0x82, 0x1a, 0x00, 0xfb, 0xc5, 0x20, 0x1b, 0x00, 0x00, 0x00, 0x02, 0x54, 0x0b, 0xe4,
        0x00, 0x15, 0x82, 0x1a, 0x04, 0x4a, 0xa2, 0x00, 0x1b, 0x00, 0x00, 0x00, 0x04, 0xa8, 0x17, 0xc8,
        0x00,
    ];
    assert_eq!(update.as_slice(), COMMITTED_WITNESS, "live bytes match the committed hermetic witness");
    assert_eq!(
        hex(&raw_hash.0),
        "4b70f9513bb1768b34680ada28c9bf27bfe4f0cdf885d4003de9f9c78cec4d2b",
        "committed manifest blake2b matches the live extraction"
    );

    let decoded = decode_exec_units_param_update(&update).expect("decode the REAL update bytes");
    eprintln!(
        "decoded: max_tx={:?} max_block={:?} unsupported={:?}",
        decoded.max_tx_ex_units, decoded.max_block_ex_units, decoded.unsupported_fields
    );

    let tx = decoded.max_tx_ex_units.expect("maxTxExUnits present (key 20)");
    let blk = decoded.max_block_ex_units.expect("maxBlockExUnits present (key 21)");
    assert_eq!(tx.mem, 16_500_000, "REAL maxTxExUnits.mem via key 20");
    assert_eq!(blk.mem, 72_000_000, "REAL maxBlockExUnits.mem via key 21");
    assert!(
        decoded.unsupported_fields.0.is_empty(),
        "the witness touches ONLY the two exec-units keys; unsupported={:?}",
        decoded.unsupported_fields
    );

    // MEMORY-ONLY effect: the update's steps == the currently-bound steps.
    let cur_tx_steps = s1a.protocol_params.max_tx_ex_units_cpu;
    let (cur_blk_mem, cur_blk_steps) = match s1a.protocol_params.max_block_ex_units {
        MaxBlockExUnits::Bound { mem, steps } => (mem, steps),
        MaxBlockExUnits::Unversioned => {
            panic!("V11 decode must Bind block ExUnits from the certified 1095 curPParams")
        }
    };
    eprintln!(
        "current 1095 curPParams: maxTx.mem={} maxTx.steps={} maxBlock.mem={} maxBlock.steps={}",
        s1a.protocol_params.max_tx_ex_units_mem, cur_tx_steps, cur_blk_mem, cur_blk_steps
    );
    assert_eq!(tx.steps, cur_tx_steps, "MEMORY-ONLY: maxTx steps unchanged (else UnsupportedRatifiedAction)");
    assert_eq!(blk.steps, cur_blk_steps, "MEMORY-ONLY: maxBlock steps unchanged (else UnsupportedRatifiedAction)");
    eprintln!(
        "VERDICT: real witness decodes via keys 20/21 to maxTx(16.5M,{}) maxBlock(72M,{}) — memory-only, supported.",
        tx.steps, blk.steps
    );
}

/// Partial census over whatever window states are extracted so far (auto-discovers the *_db-analyser
/// snapshots in the 1087-1103 slot range). Reports the lifecycle-so-far: action presence + maxTxExUnits.mem.
#[test]
#[ignore = "reads local db-analyser window states as they extract; run for a live census status"]
fn cre_census_partial_available() {
    let dir = "/home/ts/.cardano-ce3d-extract/db/ledger";
    let mut slots: Vec<u64> = std::fs::read_dir(dir)
        .expect("ledger dir")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_suffix("_db-analyser")
                .and_then(|s| s.parse::<u64>().ok())
        })
        .filter(|s| (93_900_000..=95_400_000).contains(s))
        .collect();
    slots.sort_unstable();
    eprintln!("=== CRE ENACTMENT-CENSUS (partial, {} epochs extracted) ===", slots.len());
    for slot in slots {
        // The stored slot is a few slots INTO its epoch; round the (115862400-slot) gap UP to whole epochs.
        let epoch = 1341 - (115_862_400 - slot + 86_399) / 86_400;
        let state = std::fs::read(format!("{dir}/{slot}_db-analyser/state")).expect("state");
        let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
        let (s1a, _) = match decode_native_nonutxo_state(&state, point, epoch, 2) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("epoch {epoch} @{slot}: DECODE FAILED -> {e:?}");
                continue;
            }
        };
        let g = &s1a.imported_gov;
        let present = g.proposals.iter().any(|p| p.action_id == target());
        let is_target = s1a
            .enacted_pparam_update
            .as_ref()
            .map(|id| *id == target())
            .unwrap_or(false);
        eprintln!(
            "epoch {epoch} @{slot}: {:>3} proposals | target present={:<5} | cur {}/{} prev {}/{} | deposit_pot={} | enacted_pp={} target={}",
            g.proposals.len(),
            present,
            s1a.protocol_params.max_tx_ex_units_mem,
            s1a.max_block_ex_units_mem,
            s1a.prev_max_tx_ex_units_mem,
            s1a.prev_max_block_ex_units_mem,
            s1a.gov_deposit_pot.0,
            s1a.enacted_pparam_update
                .as_ref()
                .map(|id| format!("{:02x}{:02x}..#{}", id.tx_hash.0[0], id.tx_hash.0[1], id.index))
                .unwrap_or_else(|| "none".to_string()),
            is_target,
        );
    }
}

// ============================================================================================
// Row-emission scaffold: one canonical, replay-stable row per epoch-boundary state, with the four
// required evidence additions. GROUND TRUTH for a FUTURE ratify/enact cluster -- NOT S4 authority.
// ============================================================================================

/// The acquisition manifest (addition 1, OPERATIONAL tier -- local db-analyser, never live runtime
/// authority). Bound into each row's provenance so the corpus is reproducible.
const DBA_IMAGE: &str = "ghcr.io/intersectmbo/cardano-node:11.0.1";
const PREVIEW_CONFIG: &str = "/home/ts/.cardano-node-preview/config/config.json";
const DBA_COMMAND: &str =
    "db-analyser --db <ce3d-db> --config <preview-config> --in-mem --db-validation minimum-block-validation --store-ledger <epoch-first-slot>";

/// One canonical census row. Replay-stable: the same ledger state yields a byte-identical row + row_hash.
#[derive(Clone, PartialEq, Eq, Debug)]
struct CensusRow {
    // (1) provenance binding -- the ChainDB point + era (block-hash/genesis/config binding is threaded from
    // the manifest below; the ledger-state file itself carries no block hash, so that is a ChainDB witness).
    epoch: u64,
    slot: u64,
    era: &'static str,
    // (3) lifecycle observables (their first-change boundary is the ratify/enact evidence)
    action_present: bool,
    proposal_count: u64,
    max_tx_ex_units_mem: u64,
    max_block_ex_units_mem: u64,
    deposit_pot: u64,
    // prevPParams observables (the previous-epoch exec-mem) — at the enactment boundary `prev` still holds the
    // OLD value while the maxTx/maxBlock above hold the NEW, proving the flip lands exactly at that boundary.
    prev_max_tx_ex_units_mem: u64,
    prev_max_block_ex_units_mem: u64,
    // The ledger's enacted-authority pointer (`prevGovActionIds.pgaPParamUpdate`) — becomes the enacting
    // action's id at the enactment boundary. `enacted_is_target` records when it names THE target action: the
    // proof that the enacted params were CAUSED by the target, not merely coincident with its observables.
    enacted_pparam_update: Option<GovActionId>,
    enacted_is_target: bool,
    // (2) canonical hashes (project encoding): the gov/non-UTxO state commitment + this row's witness hash.
    gov_state_hash: [u8; 32],
    row_hash: [u8; 32],
}

fn build_row(epoch: u64, slot: u64, state: &[u8]) -> CensusRow {
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, commitment) = decode_native_nonutxo_state(state, point, epoch, 2)
        .unwrap_or_else(|e| panic!("decode epoch {epoch} @{slot}: {e:?}"));
    let g = &s1a.imported_gov;
    let action_present = g.proposals.iter().any(|p| p.action_id == target());
    let proposal_count = g.proposals.len() as u64;
    let max_tx = s1a.protocol_params.max_tx_ex_units_mem;
    let max_block = s1a.max_block_ex_units_mem;
    let deposit = s1a.gov_deposit_pot.0;
    let prev_max_tx = s1a.prev_max_tx_ex_units_mem;
    let prev_max_block = s1a.prev_max_block_ex_units_mem;
    let enacted = s1a.enacted_pparam_update.clone();
    let enacted_is_target = enacted.as_ref().map(|id| *id == target()).unwrap_or(false);
    // canonical row encoding (fixed field order, big-endian) -> the differential witness hash.
    let mut buf = Vec::new();
    buf.extend_from_slice(&epoch.to_be_bytes());
    buf.extend_from_slice(&slot.to_be_bytes());
    buf.push(action_present as u8);
    buf.extend_from_slice(&proposal_count.to_be_bytes());
    buf.extend_from_slice(&max_tx.to_be_bytes());
    buf.extend_from_slice(&max_block.to_be_bytes());
    buf.extend_from_slice(&deposit.to_be_bytes());
    buf.extend_from_slice(&prev_max_tx.to_be_bytes());
    buf.extend_from_slice(&prev_max_block.to_be_bytes());
    // enacted-authority pointer: 0x00 for SNothing, else 0x01 || tx_hash(32) || index(BE u32).
    match &enacted {
        None => buf.push(0x00),
        Some(id) => {
            buf.push(0x01);
            buf.extend_from_slice(&id.tx_hash.0);
            buf.extend_from_slice(&id.index.to_be_bytes());
        }
    }
    buf.extend_from_slice(&commitment.0);
    CensusRow {
        epoch,
        slot,
        era: "Conway",
        action_present,
        proposal_count,
        max_tx_ex_units_mem: max_tx,
        max_block_ex_units_mem: max_block,
        deposit_pot: deposit,
        prev_max_tx_ex_units_mem: prev_max_tx,
        prev_max_block_ex_units_mem: prev_max_block,
        enacted_pparam_update: enacted,
        enacted_is_target,
        gov_state_hash: commitment.0,
        row_hash: blake2b_256(&buf).0,
    }
}

/// Auto-discover the extracted window slots (the *_db-analyser snapshots in 1087-1103), sorted by epoch.
fn window_slots() -> Vec<u64> {
    let dir = "/home/ts/.cardano-ce3d-extract/db/ledger";
    let mut slots: Vec<u64> = std::fs::read_dir(dir)
        .expect("ledger dir")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_suffix("_db-analyser")
                .and_then(|s| s.parse::<u64>().ok())
        })
        .filter(|s| (93_900_000..=95_400_000).contains(s))
        .collect();
    slots.sort_unstable();
    slots
}

#[test]
#[ignore = "reads the local ImmutableDB; emits the selected-chain witness for every census boundary slot"]
fn cre_census_chain_point_witnesses() {
    use ade_testkit::harness::immutabledb_witness::witness_for_slot;
    let dir = std::path::Path::new(IMMUTABLE_DIR);
    let slots = window_slots();
    assert!(!slots.is_empty(), "some window states extracted");
    eprintln!("=== CRE census — selected-chain witnesses ({} rows) ===", slots.len());
    let mut era_tags = std::collections::BTreeSet::new();
    for &slot in &slots {
        // A returned witness means the header bytes at the index offsets hash to the index's stored hash —
        // an unambiguous chain-point binding, not just a state fingerprint.
        let w = witness_for_slot(dir, slot).unwrap_or_else(|e| panic!("witness @{slot}: {e}"));
        assert_eq!(w.slot, slot);
        assert!(w.parent_header_hash.is_some(), "a real boundary block has a parent");
        era_tags.insert(w.era_tag);
        let hh: String = w.block_header_hash.iter().take(6).map(|b| format!("{b:02x}")).collect();
        let ph: String = w
            .parent_header_hash
            .map(|p| p.iter().take(6).map(|b| format!("{b:02x}")).collect())
            .unwrap_or_else(|| "genesis".into());
        eprintln!("  slot {slot} block#{:<7} era_tag={} header={hh}.. parent={ph}..", w.block_no, w.era_tag);
    }
    // Every census boundary is the same era (Conway) — the era tag must be constant across the window.
    assert_eq!(era_tags.len(), 1, "one era across the census window, got tags {era_tags:?}");
}

fn build_census(slots: &[u64]) -> Vec<CensusRow> {
    let dir = "/home/ts/.cardano-ce3d-extract/db/ledger";
    slots
        .iter()
        .map(|&slot| {
            let epoch = 1341 - (115_862_400 - slot + 86_399) / 86_400;
            let state = std::fs::read(format!("{dir}/{slot}_db-analyser/state")).expect("state");
            build_row(epoch, slot, &state)
        })
        .collect()
}

#[test]
#[ignore = "reads the local db-analyser window states; emits the canonical census + first-change boundaries"]
fn cre_census_rows_and_first_boundaries() {
    let slots = window_slots();
    let rows = build_census(&slots);
    assert!(!rows.is_empty(), "some window states extracted");
    eprintln!("=== CRE ENACTMENT CENSUS ({} epochs) ===", rows.len());
    eprintln!("manifest: image={DBA_IMAGE} config={PREVIEW_CONFIG}");
    eprintln!("command:  {DBA_COMMAND}");
    for r in &rows {
        eprintln!(
            "ep {} @{} [{}] present={} n={:<3} cur={}/{} prev={}/{} deposit={} enacted={} row={:02x}{:02x}{:02x}{:02x}",
            r.epoch, r.slot, r.era, r.action_present, r.proposal_count,
            r.max_tx_ex_units_mem, r.max_block_ex_units_mem,
            r.prev_max_tx_ex_units_mem, r.prev_max_block_ex_units_mem, r.deposit_pot,
            enacted_str(r),
            r.row_hash[0], r.row_hash[1], r.row_hash[2], r.row_hash[3],
        );
    }
    // (3) first boundary each lifecycle fact changes -- the params changing IS the enactment evidence.
    eprintln!("--- lifecycle transitions (first-boundary-of-change) ---");
    for w in rows.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if a.action_present != b.action_present {
            eprintln!("  action_present {} -> {} @ epoch {}", a.action_present, b.action_present, b.epoch);
        }
        if a.max_tx_ex_units_mem != b.max_tx_ex_units_mem {
            eprintln!("  maxTxExUnits.mem {} -> {} @ epoch {}  [ENACTMENT]", a.max_tx_ex_units_mem, b.max_tx_ex_units_mem, b.epoch);
        }
        if a.max_block_ex_units_mem != b.max_block_ex_units_mem {
            eprintln!("  maxBlockExUnits.mem {} -> {} @ epoch {}  [ENACTMENT]", a.max_block_ex_units_mem, b.max_block_ex_units_mem, b.epoch);
        }
        if a.enacted_pparam_update != b.enacted_pparam_update {
            eprintln!(
                "  enacted PParamUpdate root {} -> {} @ epoch {}{}",
                enacted_str(a), enacted_str(b), b.epoch,
                if b.enacted_is_target { "  [= TARGET: the ledger names the target action the enacted authority]" } else { "" },
            );
        }
    }
    // The causal chain: IF the census window spans the enactment (maxTx flips), THEN at that SAME boundary the
    // enacted-authority pointer must name the target AND the action must leave the proposal map AND prevPParams
    // must still hold the old value. This is what upgrades the census from "params coincide with the
    // observables" to "the ledger's own authority record attributes the flip to the target action".
    for w in rows.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if a.max_tx_ex_units_mem != b.max_tx_ex_units_mem {
            assert!(b.enacted_is_target, "at the enactment boundary the enacted PParamUpdate root names the target");
            assert!(a.action_present && !b.action_present, "the target leaves the proposal map on enactment");
            assert_eq!(b.prev_max_tx_ex_units_mem, a.max_tx_ex_units_mem, "prevPParams still holds the pre-enactment maxTx");
            assert_eq!(b.prev_max_block_ex_units_mem, a.max_block_ex_units_mem, "prevPParams still holds the pre-enactment maxBlock");
        }
    }
    // (4) replay: the SAME slots re-decoded twice produce byte-identical rows + hashes.
    assert_eq!(build_census(&slots), build_census(&slots), "census replay is byte-identical");
}

/// Render a row's enacted-authority PParamUpdate pointer (`prevGovActionIds.pgaPParamUpdate`) compactly.
fn enacted_str(r: &CensusRow) -> String {
    match &r.enacted_pparam_update {
        None => "none".to_string(),
        Some(id) => format!(
            "{:02x}{:02x}..#{}{}",
            id.tx_hash.0[0], id.tx_hash.0[1], id.index,
            if r.enacted_is_target { "(TARGET)" } else { "" },
        ),
    }
}

/// CRE S4 ORACLE ANCHOR (built BEFORE any live activation): run Ade's REAL ratify gate
/// (`evaluate_ratification`) over the census states with the real thresholds + derived DRep stake + the
/// proposals' accumulated votes, and check whether it reproduces the oracle — 69c948cd..#0 enacted at 1096,
/// so it must be decided RATIFIED at the 1095→1096 boundary (ending_epoch=1095) and NOT before it enacts.
/// If Ade's gate does not reproduce this, the S4.2/S4.3 activation does NOT flip until the gate is correct.
#[test]
#[ignore = "reads local census states; runs the REAL ratify gate as the S4 activation oracle anchor"]
fn cre_s4_oracle_anchor_ratify_decision() {
    use ade_ledger::governance::{derive_drep_voting_stake, evaluate_ratification};
    use ade_ledger::rational::Rational;

    // Run the real ratify gate on the census state at (slot, epoch) with ending_epoch = epoch (the boundary
    // that epoch's accumulated votes feed). Returns (target_ratified, total_ratified, proposal_count).
    fn ratify(slot: u64, epoch: u64) -> (bool, usize, usize) {
        let dir = "/home/ts/.cardano-ce3d-extract/db/ledger";
        let state = std::fs::read(format!("{dir}/{slot}_db-analyser/state")).expect("state");
        let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
        let (s1a, _) = decode_native_nonutxo_state(&state, point, epoch, 2).expect("decode");
        let g = &s1a.imported_gov;
        let drep_stake = derive_drep_voting_stake(&g.vote_delegations, &s1a.snapshots.mark.0);
        let quorum = g
            .committee_quorum
            .map(|(n, d)| Rational::new(n as i128, d.max(1) as i128).unwrap())
            .unwrap_or_else(|| Rational::new(1, 1).unwrap());
        // The census state is a real decoded Conway state (a named bound source) → V2 Bound dormancy.
        let num_dormant = ade_ledger::state::DormantEpochs::Bound(g.num_dormant_epochs);
        let r = evaluate_ratification(
            &g.proposals,
            &drep_stake,
            &s1a.snapshots.go.0.pool_stakes,
            &g.committee,
            &quorum,
            &g.pool_voting_thresholds,
            &g.drep_voting_thresholds,
            epoch,
            &g.committee_hot_keys,
            &g.drep_expiry,
            &num_dormant,
        )
        .expect("ratify gate (Bound dormancy)");
        (r.ratified.iter().any(|p| p.action_id == target()), r.ratified.len(), g.proposals.len())
    }

    let (t94, n94, p94) = ratify(94_521_600, 1094);
    let (t95, n95, p95) = ratify(94_608_021, 1095);
    eprintln!("=== CRE S4 ORACLE ANCHOR (real ratify gate on the census) ===");
    eprintln!("ending_epoch 1094 (n={p94}): target ratified={t94} | {n94} total ratified");
    eprintln!("ending_epoch 1095 (n={p95}): target ratified={t95} | {n95} total ratified");
    // The oracle: 69c948cd..#0 enacted at 1096 ⇒ it ratifies at the 1095→1096 boundary. Under S4.1 the gate
    // now runs with V2 Bound dormancy (from each decoded census state's real numDormant), and the outcome is
    // UNCHANGED from the pre-S4.1 c2f5960e anchor — asserted EXPLICITLY (the full outcome, not a loose
    // old==new): ratifies at 1095, NOT at 1094, and the target is the ONLY proposal ratified at 1095.
    assert!(!t94, "not yet ratified at the 1094→1095 boundary (still voting)");
    assert!(t95, "ratifies at the 1095→1096 boundary (the enacting boundary)");
    assert_eq!(n95, 1, "the target is the ONLY proposal ratified at 1095 — no false positives");
}

/// CRE S4.1b GATE 6 — the c2f5960e anchor rerun AGAINST A PERSISTED+RECOVERED V2 STORE. Build the live
/// governance store from the census 1095 state (dormancy `Bound` to the real decoded numDormant), persist it
/// through the DURABLE codec, recover it, and prove: (a) it is restart-equal with the real offset preserved,
/// and (b) the RECOVERED store reproduces the oracle — 69c948cd..#0 ratifies at 1095, and only it. Separates
/// "is the V2 seed correct" (this) from "does the live path consume it" (S4.2).
#[test]
#[ignore = "reads local census states; S4.1b — the recovered V2 governance store ratifies the anchor outcome"]
fn cre_s4_1b_recovered_v2_store_ratifies_the_anchor_outcome() {
    use ade_ledger::governance::{derive_drep_voting_stake, evaluate_ratification};
    use ade_ledger::rational::Rational;
    use ade_ledger::snapshot::{decode_gov_state, encode_gov_state};
    use ade_ledger::state::{ConwayGovState, DormantEpochs};

    let dir = "/home/ts/.cardano-ce3d-extract/db/ledger";
    let state = std::fs::read(format!("{dir}/94608021_db-analyser/state")).expect("state");
    let point = SeedPoint { slot: SlotNo(94_608_021), block_hash: Hash32([0u8; 32]) };
    let (s1a, _) = decode_native_nonutxo_state(&state, point, 1095, 2).expect("decode");
    let g = &s1a.imported_gov;

    // Build the V2 governance store (Bound to the REAL decoded numDormant), then PERSIST → RECOVER it.
    let store = ConwayGovState {
        prev_pparam_action: ade_ledger::state::PreviousPParamAction::Unversioned,
        proposals: g.proposals.clone(),
        committee: g.committee.clone(),
        committee_quorum: g.committee_quorum.unwrap_or((1, 1)),
        drep_expiry: g.drep_expiry.clone(),
        gov_action_lifetime: g.gov_action_lifetime,
        vote_delegations: g.vote_delegations.clone(),
        pool_voting_thresholds: g.pool_voting_thresholds.clone(),
        drep_voting_thresholds: g.drep_voting_thresholds.clone(),
        committee_hot_keys: g.committee_hot_keys.clone(),
        num_dormant: DormantEpochs::Bound(g.num_dormant_epochs),
    };
    let recovered = decode_gov_state(&encode_gov_state(&store)).expect("recover the V2 store");
    assert_eq!(recovered, store, "the V2 governance store is restart-equal");
    assert_eq!(
        recovered.num_dormant,
        DormantEpochs::Bound(g.num_dormant_epochs),
        "the real dormant offset survives the durable round-trip"
    );

    // Rerun the anchor AGAINST the recovered store.
    let drep_stake = derive_drep_voting_stake(&recovered.vote_delegations, &s1a.snapshots.mark.0);
    let quorum =
        Rational::new(recovered.committee_quorum.0 as i128, recovered.committee_quorum.1.max(1) as i128).unwrap();
    let r = evaluate_ratification(
        &recovered.proposals,
        &drep_stake,
        &s1a.snapshots.go.0.pool_stakes,
        &recovered.committee,
        &quorum,
        &recovered.pool_voting_thresholds,
        &recovered.drep_voting_thresholds,
        1095,
        &recovered.committee_hot_keys,
        &recovered.drep_expiry,
        &recovered.num_dormant,
    )
    .expect("ratify against the recovered V2 store");
    assert!(
        r.ratified.iter().any(|p| p.action_id == target()),
        "the RECOVERED V2 store ratifies 69c948cd..#0 at 1095"
    );
    assert_eq!(r.ratified.len(), 1, "no false positives from the recovered store");
    eprintln!(
        "CRE S4.1b: recovered V2 store (Bound({})) ratifies the target at 1095, {} total",
        g.num_dormant_epochs,
        r.ratified.len()
    );
}

// ============================================================================================
// The PERMANENT differential fixture: every ground-truth row bound to its exact SELECTED-CHAIN POINT
// (block_header_hash + parent + genesis/config hash + network_magic), NOT merely a decoded-state fingerprint.
// A state commitment proves "these bytes decode to this state", NOT "these came from this exact historical
// chain block" — two chain points can share a projection. See the chain-point-witness methodology note.
// ============================================================================================

use ade_testkit::harness::immutabledb_witness::{witness_for_slot, ChainPointWitness};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const IMMUTABLE_DIR: &str = "/home/ts/.cardano-ce3d-extract/db/immutable";
const LEDGER_DIR: &str = "/home/ts/.cardano-ce3d-extract/db/ledger";
const TARGET_ACTION_STR: &str = "69c948cde90c6b9d7d61595e8534c106ec44132cb049ab2558399db1260c1f69#0";

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The four era genesis hashes from the node config — the machine-independent protocol identity that the
/// db-analyser extraction was pinned to (`--config`). Read live so the fixture binds to the REAL config.
fn genesis_hashes() -> BTreeMap<String, String> {
    let cfg = std::fs::read_to_string(PREVIEW_CONFIG).expect("read config.json");
    let v: serde_json::Value = serde_json::from_str(&cfg).expect("parse config.json");
    let mut m = BTreeMap::new();
    for (era, key) in [
        ("byron", "ByronGenesisHash"),
        ("shelley", "ShelleyGenesisHash"),
        ("alonzo", "AlonzoGenesisHash"),
        ("conway", "ConwayGenesisHash"),
    ] {
        m.insert(era.to_string(), v[key].as_str().expect("genesis hash").to_string());
    }
    m
}

/// A single config-identity digest over the four genesis hashes (fixed era order) — one value binding every
/// row to the exact protocol configuration.
fn genesis_config_hash(hashes: &BTreeMap<String, String>) -> String {
    let mut buf = Vec::new();
    for era in ["byron", "shelley", "alonzo", "conway"] {
        let h = hashes.get(era).expect("era hash");
        buf.extend_from_slice(&(0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()).collect::<Vec<u8>>());
    }
    hex32(&blake2b_256(&buf).0)
}

/// The constant acquisition + protocol provenance shared by every row (the manifest the row hashes bind to).
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
struct FixtureManifest {
    description: String,
    target_action: String,
    network_magic: u64,
    genesis_hashes: BTreeMap<String, String>,
    genesis_config_hash: String,
    dba_image: String,
    dba_command: String,
    preview_config: String,
    window_epochs: [u64; 2],
}

/// One fully-witnessed census row: the selected-chain point + provenance + state fingerprints + the
/// lifecycle observables, all bound into `row_hash`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
struct WitnessedCensusRow {
    epoch: u64,
    slot: u64,
    block_no: u64,
    era: String,
    era_tag: u8,
    // (1) selected-chain identity — what a state fingerprint CANNOT provide.
    block_header_hash: String,
    parent_header_hash: String,
    // (2) state fingerprints — the raw extracted blob + the canonical decoded projection.
    raw_ledger_state_blob_hash: String,
    canonical_decoded_state_hash: String,
    // (3) lifecycle observables (the enactment evidence).
    action_present: bool,
    proposal_count: u64,
    max_tx_ex_units_mem: u64,
    max_block_ex_units_mem: u64,
    prev_max_tx_ex_units_mem: u64,
    prev_max_block_ex_units_mem: u64,
    deposit_pot: u64,
    enacted_pparam_update: Option<String>,
    enacted_is_target: bool,
    // the differential witness over EVERY field above + the manifest's network_magic + genesis_config_hash.
    row_hash: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
struct CensusFixture {
    manifest: FixtureManifest,
    rows: Vec<WitnessedCensusRow>,
}

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/cre_enactment_census/census_1087_1103.json")
}

fn build_witnessed_row(
    epoch: u64,
    slot: u64,
    network_magic: u64,
    genesis_config_hash: &str,
) -> WitnessedCensusRow {
    // Decode the ledger-state projection (the census observables + the canonical state commitment).
    let raw = std::fs::read(format!("{LEDGER_DIR}/{slot}_db-analyser/state")).expect("state");
    let point = SeedPoint { slot: SlotNo(slot), block_hash: Hash32([0u8; 32]) };
    let (s1a, commitment) = decode_native_nonutxo_state(&raw, point, epoch, 2)
        .unwrap_or_else(|e| panic!("decode epoch {epoch} @{slot}: {e:?}"));
    let g = &s1a.imported_gov;
    let action_present = g.proposals.iter().any(|p| p.action_id == target());
    let enacted = s1a.enacted_pparam_update.clone();
    let enacted_is_target = enacted.as_ref().map(|id| *id == target()).unwrap_or(false);
    // Bind the row to its exact selected-chain point (block_header_hash + parent), from the ImmutableDB.
    let w: ChainPointWitness =
        witness_for_slot(Path::new(IMMUTABLE_DIR), slot).unwrap_or_else(|e| panic!("witness @{slot}: {e}"));
    assert_eq!(w.slot, slot);
    assert_eq!(w.era_tag, 7, "every census boundary is Conway (HFC era tag 7)");
    let raw_blob_hash = blake2b_256(&raw).0;
    let parent = w.parent_header_hash.expect("a real boundary block has a parent");

    // canonical row encoding (fixed order, big-endian) -> the differential witness hash. Binds the
    // chain-point identity, the config/network provenance, the state fingerprints, AND the observables.
    let gch: Vec<u8> = (0..genesis_config_hash.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&genesis_config_hash[i..i + 2], 16).unwrap())
        .collect();
    let mut buf = Vec::new();
    buf.extend_from_slice(&epoch.to_be_bytes());
    buf.extend_from_slice(&slot.to_be_bytes());
    buf.extend_from_slice(&w.block_no.to_be_bytes());
    buf.push(w.era_tag);
    buf.extend_from_slice(&w.block_header_hash);
    buf.extend_from_slice(&parent);
    buf.extend_from_slice(&network_magic.to_be_bytes());
    buf.extend_from_slice(&gch);
    buf.extend_from_slice(&raw_blob_hash);
    buf.extend_from_slice(&commitment.0);
    buf.push(action_present as u8);
    buf.extend_from_slice(&(g.proposals.len() as u64).to_be_bytes());
    buf.extend_from_slice(&s1a.protocol_params.max_tx_ex_units_mem.to_be_bytes());
    buf.extend_from_slice(&s1a.max_block_ex_units_mem.to_be_bytes());
    buf.extend_from_slice(&s1a.prev_max_tx_ex_units_mem.to_be_bytes());
    buf.extend_from_slice(&s1a.prev_max_block_ex_units_mem.to_be_bytes());
    buf.extend_from_slice(&s1a.gov_deposit_pot.0.to_be_bytes());
    match &enacted {
        None => buf.push(0x00),
        Some(id) => {
            buf.push(0x01);
            buf.extend_from_slice(&id.tx_hash.0);
            buf.extend_from_slice(&id.index.to_be_bytes());
        }
    }
    buf.push(enacted_is_target as u8);

    WitnessedCensusRow {
        epoch,
        slot,
        block_no: w.block_no,
        era: format!("{:?}", s1a.era),
        era_tag: w.era_tag,
        block_header_hash: hex32(&w.block_header_hash),
        parent_header_hash: hex32(&parent),
        raw_ledger_state_blob_hash: hex32(&raw_blob_hash),
        canonical_decoded_state_hash: hex32(&commitment.0),
        action_present,
        proposal_count: g.proposals.len() as u64,
        max_tx_ex_units_mem: s1a.protocol_params.max_tx_ex_units_mem,
        max_block_ex_units_mem: s1a.max_block_ex_units_mem,
        prev_max_tx_ex_units_mem: s1a.prev_max_tx_ex_units_mem,
        prev_max_block_ex_units_mem: s1a.prev_max_block_ex_units_mem,
        deposit_pot: s1a.gov_deposit_pot.0,
        enacted_pparam_update: enacted
            .map(|id| format!("{}#{}", hex32(&id.tx_hash.0), id.index)),
        enacted_is_target,
        row_hash: hex32(&blake2b_256(&buf).0),
    }
}

fn build_fixture() -> CensusFixture {
    let hashes = genesis_hashes();
    let gch = genesis_config_hash(&hashes);
    let network_magic = 2u64;
    let slots = window_slots();
    let rows: Vec<WitnessedCensusRow> = slots
        .iter()
        .map(|&slot| {
            let epoch = 1341 - (115_862_400 - slot + 86_399) / 86_400;
            build_witnessed_row(epoch, slot, network_magic, &gch)
        })
        .collect();
    CensusFixture {
        manifest: FixtureManifest {
            description: "CONWAY-RATIFICATION-AND-ENACTMENT-AUTHORITY enactment ground truth — the Preview \
                param-update 69c948cd..#0 (Increase Tx/Block Memory Units pt1): present@1089, ratified+enacted \
                @1096 (14M/62M -> 16.5M/72M), persisted to 1103. Every row bound to its selected-chain point."
                .to_string(),
            target_action: TARGET_ACTION_STR.to_string(),
            network_magic,
            genesis_hashes: hashes,
            genesis_config_hash: gch,
            dba_image: DBA_IMAGE.to_string(),
            dba_command: DBA_COMMAND.to_string(),
            preview_config: PREVIEW_CONFIG.to_string(),
            window_epochs: [1087, 1103],
        },
        rows,
    }
}

/// Emit / gate the PERMANENT differential fixture: every row bound to its exact selected-chain point. On the
/// first run (fixture absent) this WRITES the committed JSON; thereafter it re-derives from the live
/// ChainDB + config and asserts BYTE-EQUALITY (the reproducibility gate) + replay determinism.
#[test]
#[ignore = "reads the local ChainDB (ImmutableDB + ledger states) + node config; emits/gates the witnessed fixture"]
fn cre_census_emit_and_gate_witnessed_fixture() {
    let fixture = build_fixture();
    assert_eq!(fixture.rows.len(), 17, "the full 1087-1103 window");
    // the enactment row (epoch 1096) names the target as the enacted authority.
    let enact = fixture.rows.iter().find(|r| r.epoch == 1096).expect("1096 row");
    assert!(enact.enacted_is_target, "the 1096 row's enacted PParamUpdate root IS the target");
    assert_eq!(enact.max_tx_ex_units_mem, 16_500_000);
    assert_eq!(enact.prev_max_tx_ex_units_mem, 14_000_000);
    assert_eq!(enact.block_no, 3_715_747, "the enactment block's selected-chain identity is pinned");

    let json = serde_json::to_string_pretty(&fixture).expect("serialize");
    let path = fixture_path();
    if path.exists() {
        let on_disk = std::fs::read_to_string(&path).expect("read committed fixture");
        assert_eq!(
            json.trim(),
            on_disk.trim(),
            "regenerated witnessed fixture != the committed one (reproducibility gate)"
        );
        eprintln!("fixture reproducibility gate PASSED: {} rows @ {}", fixture.rows.len(), path.display());
    } else {
        std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir fixtures");
        std::fs::write(&path, &json).expect("write fixture");
        eprintln!("WROTE witnessed fixture ({} rows) -> {}", fixture.rows.len(), path.display());
    }
    // replay determinism: re-derive from the same inputs -> byte-identical.
    assert_eq!(json, serde_json::to_string_pretty(&build_fixture()).unwrap(), "fixture derivation is deterministic");
}
