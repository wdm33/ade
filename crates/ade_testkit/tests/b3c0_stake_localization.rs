//! B3c.0 — base-UTxO stake localization EVIDENCE (GREEN; NO BLUE change).
//!
//! CONCLUSION (proven clean): the base-UTxO pipeline (`reduce_txout` -> the durable `ReducedUtxoCheckpoint` ->
//! `sum_base_credential_stake`) is BYTE-EXACT correct. The −343,260,172,883 CE-3d go-stake residual is REAL (it
//! reproduces cleanly — see `docs/clusters/B3C-STAKE-RESIDUAL/B3c.0-adjudication-result.md`) but is NOT a
//! base-UTxO undercount — it is a go-stake DERIVATION discrepancy (a later evidence slice).
//!
//! EVIDENCE-HARNESS RULE (learned the hard way): `ReducedUtxoCheckpoint::open` is redb `Database::create`
//! (read-WRITE, can write on open). Analysis MUST run from an ISOLATED stable copy with EXCLUSIVE single-process
//! ownership — never a shared or mid-advance file, never concurrent opens. Violating this fabricated a fake
//! "75M / ~14-whale / −2.1e12 base-UTxO drift"; the clean single-process test below refutes it. Local artifacts
//! only (`#[ignore]`).

use std::collections::BTreeMap;

use ade_ledger::mithril_utxo_materialize::materialize_tables_to_utxo;
use ade_ledger::reduced_utxo::{reduce_txout, ReducedStakeRef};
use ade_runtime::chaindb::ReducedUtxoCheckpoint;
use ade_types::shelley::cert::StakeCredential;

/// The re-bootstrap seed reduced checkpoint (production, advanced to POST-1340) — copied to an ISOLATED path
/// before opening (never opened in place: open is read-write).
const SEED_CP: &str = "/home/ts/.cardano-ce3d-rebootstrap/reduced-checkpoint.redb";
/// The POST-1340 reference UTxO lives in the `tables` file (`state` is the non-UTxO ledger state only).
const TABLES_1340: &str = "/home/ts/.cardano-ce3d-extract/db/ledger/115776011_db-analyser/tables";
/// Conway era index for `materialize_tables_to_utxo`.
const CONWAY: usize = 6;

/// DEFINITIVE: the durable reduced checkpoint == a fresh `reduce_txout` of the same-point reference UTxO,
/// byte-for-byte per FULL `StakeCredential`. One process, an ISOLATED fresh copy opened ONCE (the evidence-harness
/// rule). Proves the base-UTxO pipeline is exact — so the −343B is NOT a base-UTxO undercount.
#[test]
#[ignore = "isolated fresh copy + one open; proves the base-UTxO checkpoint is byte-exact (B3c.0 evidence)"]
fn b3c0_clean_checkpoint_vs_reduction() {
    // Truth: fresh reduction of the POST-1340 reference UTxO, keyed by full StakeCredential.
    let tables = std::fs::read(TABLES_1340).expect("read tables");
    let utxo = materialize_tables_to_utxo(&tables, CONWAY, None).expect("materialize");
    let mut reduc: BTreeMap<StakeCredential, u64> = BTreeMap::new();
    for out in utxo.utxos.values() {
        if let (coin, ReducedStakeRef::Base(cred)) = reduce_txout(out) {
            *reduc.entry(cred).or_insert(0) += coin.0;
        }
    }
    // Production checkpoint: ISOLATED fresh copy, opened ONCE (never the shared seed directly).
    let iso = std::env::temp_dir().join(format!("b3c0-evidence-cp-{}.redb", std::process::id()));
    let _ = std::fs::remove_file(&iso);
    std::fs::copy(SEED_CP, &iso).expect("copy seed checkpoint to an isolated path");
    let cp = ReducedUtxoCheckpoint::open(&iso).expect("open isolated checkpoint");
    let chk = cp.sum_base_credential_stake().expect("sum_base_credential_stake");

    let reduc_total: u128 = reduc.values().map(|v| *v as u128).sum();
    let chk_total: u128 = chk.values().map(|c| c.0 as u128).sum();
    let keys: std::collections::BTreeSet<&StakeCredential> = reduc.keys().chain(chk.keys()).collect();
    let mismatches = keys
        .iter()
        .filter(|k| reduc.get(**k).copied().unwrap_or(0) != chk.get(**k).map(|x| x.0).unwrap_or(0))
        .count();
    eprintln!(
        "B3c.0 base-UTxO: reduction_total={reduc_total} checkpoint_total={chk_total} creds={} mismatches={mismatches}",
        reduc.len()
    );
    assert_eq!(reduc_total, chk_total, "the base-UTxO aggregate is byte-exact");
    assert_eq!(mismatches, 0, "the durable checkpoint == the fresh reduction for EVERY credential");
    let _ = std::fs::remove_file(&iso);
}

/// The reference UTxO by RAW address type: `base` total is byte-exact with the checkpoint/reduction, `POINTER`
/// stake is negligible (refutes the pointer hypothesis for the −343B), and enterprise/byron carry no stake
/// (correctly NonContributing).
#[test]
#[ignore = "materializes the POST-1340 reference UTxO (tables); B3c.0 address-type distribution"]
fn b3c0_utxo_class_totals() {
    use ade_codec::address::decode_address;
    use ade_types::address::Address;
    let tables = std::fs::read(TABLES_1340).expect("read tables");
    let utxo = materialize_tables_to_utxo(&tables, CONWAY, None).expect("materialize UTxO");
    let mut cls: BTreeMap<&str, (u64, u128)> = BTreeMap::new();
    for out in utxo.utxos.values() {
        let coin = out.coin().0 as u128;
        let c = match decode_address(out.address_bytes()) {
            Ok(Address::Base(_)) => "base",
            Ok(Address::Pointer(_)) => "POINTER",
            Ok(Address::Enterprise(_)) => "enterprise",
            Ok(Address::Byron(_)) => "byron",
            Ok(Address::Reward(_)) => "reward",
            Err(_) => "undecodable",
        };
        let e = cls.entry(c).or_insert((0, 0));
        e.0 += 1;
        e.1 += coin;
    }
    eprintln!("B3c.0 reference UTxO ({} entries) by RAW address type:", utxo.utxos.len());
    for (c, (n, sum)) in &cls {
        eprintln!("  {c:12} count={n:>10} coin_sum={sum}");
    }
    let pointer = cls.get("POINTER").map(|(_, s)| *s).unwrap_or(0);
    assert!(pointer < 100_000_000_000, "pointer stake is negligible — not the −343B");
}
