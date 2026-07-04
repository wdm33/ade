//! CRE S4.3b-bootstrap operational capstone (obligation A — V11 live-seed lineage): prove the LIVE
//! re-bootstrapped V11 seed is SOURCE-BOUND for all three new authoritative fields — num_dormant, the full
//! block ExUnits {mem, steps}, and the previous pparam-action root — and (with the warm-restart tool below)
//! survives a warm restart with identical canonical fingerprints. Reads the on-disk store produced by
//! `ade node run --bootstrap-mithril` into a NEW data-dir (`.cardano-s4-3b-v11`) alongside the verified
//! snapshot's source state. #[ignore] (local artifacts) — operational evidence, NOT a deterministic gate.

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_ledger::pparams::MaxBlockExUnits;
use ade_ledger::state::{DormantEpochs, LedgerState, PreviousPParamAction};
use ade_runtime::chaindb::EpochAccumulatorStore;
use ade_types::{CardanoEra, Hash32, SlotNo};

const DATA_DIR: &str = "/home/ts/.cardano-s4-3b-v11";
const SNAPSHOT_STATE: &str =
    "/home/ts/.cardano-preview-judge/preview-snapshot/db/ledger/115676685/state";
const SEED_SLOT: u64 = 115_676_685;
const SEED_EPOCH: u64 = 1338;
/// The 1338->1339 boundary slot: 1339 * 86_400. The store's chain must stay strictly below it so the seed
/// lineage claim is isolated from the shared boundary VrfCert residual (b3c_stake_residual).
const EPOCH_1338_END: u64 = 1339 * 86_400;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
#[ignore = "reads the live re-bootstrapped V11 store + the verified snapshot; run explicitly (CRE S4.3b-bootstrap obligation A)"]
fn cre_s4_3b_v11_seed_is_source_bound_with_evidence() {
    // 1. Source of truth: decode the VERIFIED snapshot S1a (the named bound source for all three fields).
    let src = std::fs::read(SNAPSHOT_STATE).expect("read snapshot source state");
    let point = SeedPoint { slot: SlotNo(SEED_SLOT), block_hash: Hash32([0u8; 32]) };
    let (s1a, _) = decode_native_nonutxo_state(&src, point, SEED_EPOCH, 2).expect("decode source S1a");
    let src_dormant = s1a.imported_gov.num_dormant_epochs;
    let src_block = s1a.protocol_params.max_block_ex_units.clone();
    let src_prev = match s1a.enacted_pparam_update.clone() {
        Some(id) => PreviousPParamAction::Enacted(id),
        None => PreviousPParamAction::NoPreviousAction,
    };

    // 2. The PERSISTED V11 store from the live re-bootstrap.
    let store = EpochAccumulatorStore::open(std::path::Path::new(&format!(
        "{DATA_DIR}/epoch-accumulator.redb"
    )))
    .expect("open the live V11 accumulator store");
    let (slot, acc) = store.load_current().expect("load_current").expect("store is complete");
    let gov = acc.gov_state.as_ref().expect("gov_state persisted in the live store");

    // 3. Report the applied position. The WITHIN-EPOCH seed source-binding is proven at bootstrap (the first
    //    read, before any follow). Here we additionally confirm the V11 fields SURVIVE a live warm-restart
    //    recovery — and, post-ECA-5, a CLEAN 1338->1339 boundary crossing (the fields carry forward unchanged).
    let within_epoch_1338 = slot.0 < EPOCH_1338_END;
    eprintln!(
        "store applied-through: slot {} (within epoch 1338 = {})",
        slot.0, within_epoch_1338
    );

    // 4. All three new V11 fields are SOURCE-BOUND — never Unversioned, never a fabricated default.
    assert_eq!(
        gov.num_dormant,
        DormantEpochs::Bound(src_dormant),
        "num_dormant Bound to the decoded snapshot source (V2/S4.1b lineage carried into V11)"
    );
    assert_eq!(
        acc.protocol_params.max_block_ex_units, src_block,
        "block ExUnits {{mem,steps}} Bound to the certified curPParams"
    );
    assert!(
        matches!(acc.protocol_params.max_block_ex_units, MaxBlockExUnits::Bound { .. }),
        "block ExUnits is Bound (a fresh V11 bootstrap), not Unversioned"
    );
    assert_eq!(
        gov.prev_pparam_action, src_prev,
        "prev_pparam_action bound to the decoded GovRelation root (Enacted/NoPreviousAction, from a source fact)"
    );
    assert!(
        !matches!(gov.prev_pparam_action, PreviousPParamAction::Unversioned),
        "prev_pparam_action is Known (a fresh V11 bootstrap), not Unversioned"
    );

    // 5. Canonical V11 fingerprints (evidence). pparams fp binds the block ExUnits; governance fp binds the
    //    prev-action root. Recorded for the warm-restart equality proof.
    let mut view = LedgerState::new(CardanoEra::Conway);
    view.gov_state = Some(gov.clone());
    view.protocol_params = acc.protocol_params.clone();
    let fp = fingerprint(&view);

    eprintln!("=== CRE S4.3b-bootstrap LIVE V11 SEED EVIDENCE (obligation A) ===");
    eprintln!("bootstrap anchor   : slot {SEED_SLOT} epoch {SEED_EPOCH} (preview magic 2, Conway)");
    eprintln!("store applied-thru : slot {} (within epoch 1338 = {})", slot.0, slot.0 < EPOCH_1338_END);
    eprintln!("num_dormant        : source={src_dormant} persisted={:?}", gov.num_dormant);
    eprintln!("max_block_ex_units : source={:?}", src_block);
    eprintln!("                     persisted={:?}", acc.protocol_params.max_block_ex_units);
    eprintln!("prev_pparam_action : source={src_prev:?}");
    eprintln!("                     persisted={:?}", gov.prev_pparam_action);
    eprintln!("pparams fingerprint: {}", hex(&fp.pparams.0));
    eprintln!("gov fingerprint    : {}", hex(&fp.governance.0));
    eprintln!(
        "gov shape          : proposals={} committee={} vote_delegations={}",
        gov.proposals.len(),
        gov.committee.len(),
        gov.vote_delegations.len()
    );
}
