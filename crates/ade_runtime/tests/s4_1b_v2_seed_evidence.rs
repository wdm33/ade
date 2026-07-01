//! CRE S4.1b operational capstone: prove the LIVE re-bootstrapped V2 governance seed is source-bound and
//! survives a warm restart. Reads the on-disk store produced by `ade node run --bootstrap-mithril` into a
//! NEW data-dir (`.cardano-s4-1b-v2`) alongside the verified snapshot's source state. #[ignore] (local
//! artifacts). This is operational evidence — it does NOT participate in the deterministic test suite.

use ade_ledger::bootstrap_anchor::SeedPoint;
use ade_ledger::fingerprint::fingerprint;
use ade_ledger::ledgerdb_state::decode_native_nonutxo_state;
use ade_ledger::state::{DormantEpochs, LedgerState};
use ade_runtime::chaindb::EpochAccumulatorStore;
use ade_types::{CardanoEra, Hash32, SlotNo};

const DATA_DIR: &str = "/home/ts/.cardano-s4-1b-v2";
const SNAPSHOT_STATE: &str =
    "/home/ts/.cardano-preview-judge/preview-snapshot/db/ledger/115676685/state";
const SEED_SLOT: u64 = 115_676_685;
const SEED_EPOCH: u64 = 1338;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
#[ignore = "reads the live re-bootstrapped V2 store + the verified snapshot; run explicitly (CRE S4.1b capstone)"]
fn cre_s4_1b_live_v2_seed_is_source_bound_with_evidence() {
    // 1. Decode the VERIFIED snapshot's source S1a → the real numDormantEpochs (the named bound source).
    let src = std::fs::read(SNAPSHOT_STATE).expect("read snapshot source state");
    let point = SeedPoint { slot: SlotNo(SEED_SLOT), block_hash: Hash32([0u8; 32]) };
    let (s1a, _) = decode_native_nonutxo_state(&src, point, SEED_EPOCH, 2).expect("decode source S1a");
    let source_dormant = s1a.imported_gov.num_dormant_epochs;

    // 2. Read the PERSISTED V2 store from the live re-bootstrap → gov_state + the governance fingerprint.
    let store = EpochAccumulatorStore::open(std::path::Path::new(&format!(
        "{DATA_DIR}/epoch-accumulator.redb"
    )))
    .expect("open the live accumulator store");
    let (slot, acc) = store.load_current().expect("load_current").expect("store is complete");
    let gov = acc.gov_state.as_ref().expect("gov_state persisted in the live store");

    // 3. The persisted seed is V2, Bound DIRECTLY to the decoded source — NOT Unversioned, NOT a fabricated
    //    default. (On preview the real value is 0, so this is a GENUINE Bound(0), not a laundered one — the
    //    non-zero binding is proven hermetically by `s4_1b_assembled_seed_binds...`.)
    assert_eq!(
        gov.num_dormant,
        DormantEpochs::Bound(source_dormant),
        "the live V2 store's dormancy is Bound to the decoded snapshot source"
    );

    // 4. The V2 governance fingerprint (evidence). Distinct from the V1 layout by construction.
    let mut view = LedgerState::new(CardanoEra::Conway);
    view.gov_state = Some(gov.clone());
    let gov_fp = fingerprint(&view).governance;

    eprintln!("=== CRE S4.1b LIVE V2 SEED EVIDENCE ===");
    eprintln!("bootstrap anchor       : slot {SEED_SLOT}  epoch {SEED_EPOCH}");
    eprintln!("network / era          : preview (magic 2) / Conway");
    eprintln!(
        "num_dormant            : source={source_dormant}  persisted={:?}  (source-bound)",
        gov.num_dormant
    );
    eprintln!("store applied-through  : slot {}", slot.0);
    eprintln!("V2 governance fp       : {}", hex(&gov_fp.0));
    eprintln!(
        "gov shape              : proposals={} committee={} vote_delegations={}",
        gov.proposals.len(),
        gov.committee.len(),
        gov.vote_delegations.len()
    );
}
