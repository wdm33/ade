//! Real-bytes `tables` → authoritative `UTxOState` materialization for the S1c converter
//! (MITHRIL-VERIFIED-ANCHOR-INTEGRATION / S1c). Materializes a SAMPLE of the real Mithril preprod
//! `tables` into Ade's ledger `UTxOState`: every sampled entry must materialize (no opaque fallback),
//! the `fingerprint_utxo_v2` must be DETERMINISTIC (same tables → same commitment), the manifest
//! binding must hold (and reject a wrong point), and the sample must carry both datum/script
//! (AlonzoPlus) and pure-payment (ShelleyMary) entries. RED: reads the restored snapshot. Skips when
//! absent.

use ade_ledger::fingerprint::fingerprint_utxo_v2;
use ade_ledger::mithril_utxo_materialize::{
    bind_utxo_to_manifest, materialize_tables_to_utxo, verify_utxo_binding,
};
use ade_ledger::utxo::TxOut;
use ade_types::Hash32;

fn tables_bytes() -> Option<Vec<u8>> {
    let path = std::env::var("ADE_TABLES").unwrap_or_else(|_| {
        "/home/ts/Code/rust/ade/.mithril-scratch/restore-ancillary/db/ledger/126400064/tables"
            .to_string()
    });
    match std::fs::read(&path) {
        Ok(d) => Some(d),
        Err(_) => {
            eprintln!("SKIP: no tables corpus at {path}");
            None
        }
    }
}

#[test]
fn materialize_real_preprod_tables_to_utxo_state() {
    let data = match tables_bytes() {
        Some(d) => d,
        None => return,
    };
    // era index 6 (Conway) is what the Stage-1 `state` decode of the SAME snapshot reports (PO#2);
    // a sample cap keeps the test fast (the whole-file materialization is the production call).
    let sample = Some(300_000usize);
    let utxo = materialize_tables_to_utxo(&data, 6, sample).expect("materialize tables -> UTxOState");
    assert!(utxo.len() > 1000, "expected a real sample");

    // Every sampled entry materialized (no error) into one of the ledger TxOut variants; count the
    // AlonzoPlus (datum/script byte-preserved) vs ShelleyMary/Byron (pure-payment) split.
    let mut alonzo_plus = 0usize;
    let mut shelley_mary = 0usize;
    let mut byron = 0usize;
    for (_tx_in, tx_out) in &utxo.utxos {
        match tx_out {
            TxOut::AlonzoPlus { .. } => alonzo_plus += 1,
            TxOut::ShelleyMary { .. } => shelley_mary += 1,
            TxOut::Byron { .. } => byron += 1,
        }
    }
    eprintln!(
        "materialized {} outputs | AlonzoPlus {} | ShelleyMary {} | Byron {}",
        utxo.len(),
        alonzo_plus,
        shelley_mary,
        byron
    );
    assert!(
        alonzo_plus > 0,
        "the sample must contain datum/script (AlonzoPlus) outputs (tags 1/3/4/5)"
    );
    assert!(
        shelley_mary > 0,
        "the sample must contain pure-payment (ShelleyMary) outputs (tags 0/2)"
    );

    // DETERMINISTIC: same tables + same era + same cap -> identical authoritative fingerprint.
    let utxo2 = materialize_tables_to_utxo(&data, 6, sample).expect("re-materialize");
    let fp1 = fingerprint_utxo_v2(&utxo);
    let fp2 = fingerprint_utxo_v2(&utxo2);
    assert_eq!(fp1, fp2, "deterministic UTxO fingerprint over the real sample");

    // The binding holds for the one manifest point and is TERMINAL for a wrong point.
    let point = Hash32([0x42; 32]);
    let stage1 = Hash32([0x11; 32]);
    let stage2 = Hash32([0x22; 32]);
    let record = bind_utxo_to_manifest(&point, &stage1, &stage2, &utxo);
    assert!(verify_utxo_binding(&record, &point, &stage1, &stage2, &utxo).is_ok());
    assert!(
        verify_utxo_binding(&record, &Hash32([0x43; 32]), &stage1, &stage2, &utxo).is_err(),
        "a wrong manifest point is a terminal binding mismatch"
    );
}
