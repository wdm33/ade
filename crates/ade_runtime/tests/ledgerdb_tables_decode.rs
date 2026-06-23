//! Real-bytes whole-tables decode + deterministic commitment for the native MemPack TxOut decoder
//! (MITHRIL-VERIFIED-ANCHOR-IMPORT, Stage 2). Runs `decode_tables_commitment` over the real Mithril
//! preprod `tables`: every TxOut must decode (no opaque fallback), the map must be canonically
//! sorted, the commitment must be deterministic, and the era binding (PO#2) must refuse a non-Conway
//! state era. RED: reads the restored snapshot. Skips when absent.

use ade_ledger::ledgerdb_tables::decode_tables_commitment;

#[test]
fn decode_real_preprod_tables_commitment() {
    let path = std::env::var("ADE_TABLES").unwrap_or_else(|_| {
        "/home/ts/Code/rust/ade/.mithril-scratch/restore-ancillary/db/ledger/126400064/tables"
            .to_string()
    });
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("SKIP: no tables corpus at {path}");
            return;
        }
    };
    // era index 6 (Conway) is what the Stage-1 `state` decode of the SAME snapshot reports (PO#2);
    // a sample cap keeps the test fast (the whole-file commitment is the production call).
    let s = decode_tables_commitment(&data, 6, Some(300_000)).expect("decode tables");
    eprintln!(
        "decoded {} TxOuts | tags {:?} | commitment {:02x?}..",
        s.count,
        s.tag_counts,
        &s.commitment.0[..6]
    );
    assert!(s.count > 1000, "expected a real sample");
    assert!(
        s.tag_counts.iter().filter(|&&c| c > 0).count() >= 3,
        "multiple tag forms present (incl. tag-2/3 Addr28Extra)"
    );
    // deterministic: same bytes + same era => identical whole-tables commitment.
    let s2 = decode_tables_commitment(&data, 6, Some(300_000)).expect("re-decode");
    assert_eq!(
        s.commitment, s2.commitment,
        "deterministic whole-tables commitment"
    );
    // PO#2: a non-Conway state era refuses to interpret the tables as this MemPack layout.
    assert!(
        decode_tables_commitment(&data, 5, Some(1)).is_err(),
        "non-Conway state era is terminal"
    );
}
