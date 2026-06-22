//! Verified-Mithril-snapshot acceptance for the V2 LedgerDB `state` decoder
//! (MITHRIL-VERIFIED-ANCHOR-IMPORT, Stage 1, gate 5). Decodes the Mithril ANCILLARY ledger state
//! (the certified fast-bootstrap snapshot, fetched with `--include-ancillary` + the ancillary
//! verification key) and confirms the SAME structural verdict as the local corpus. The binding epoch
//! is the state's own NES epoch (internal to the certified snapshot), NOT the filename slot — the
//! caller binds that to the certificate beacon. RED: reads the restored Mithril snapshot; skips when
//! absent.

use ade_ledger::ledgerdb_state::{probe_ledgerdb_state, LedgerDbStateError};

#[test]
fn decode_verified_mithril_ledger_state() {
    let dir = std::env::var("ADE_MITHRIL_LEDGER_DIR").unwrap_or_else(|_| {
        "/home/ts/Code/rust/ade/.mithril-scratch/restore-ancillary/db/ledger".to_string()
    });
    let mut slots: Vec<u64> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .flatten()
            .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse::<u64>().ok()))
            .collect(),
        Err(_) => {
            eprintln!("SKIP: no Mithril restore at {dir}");
            return;
        }
    };
    if slots.is_empty() {
        eprintln!("SKIP: no ledger snapshot in {dir}");
        return;
    }
    slots.sort();
    let slot = *slots.last().unwrap();
    let state = std::fs::read(format!("{dir}/{slot}/state")).expect("read mithril state");
    eprintln!("Mithril ledger snapshot slot {slot}, state {} bytes", state.len());
    // Discover the snapshot's own NES epoch (the binding authority — a wrong manifest epoch returns
    // the decoded one fail-closed; the filename slot is NOT used as the authority).
    let nes_epoch = match probe_ledgerdb_state(&state, u64::MAX) {
        Err(LedgerDbStateError::EpochMismatch { decoded_epoch, .. }) => decoded_epoch,
        other => panic!("epoch discovery unexpected: {other:?}"),
    };
    eprintln!("decoded NES epoch (binding authority, not the filename slot {slot}): {nes_epoch}");
    let p = probe_ledgerdb_state(&state, nes_epoch).expect("decode mithril state");
    eprintln!("MITHRIL PROBE OK: {p:#?}");
    assert_eq!(p.era_index, 6, "current era is Conway");
    assert_eq!(
        p.vrf_count, p.active_pool_count,
        "every active pool carries a real VRF"
    );
    assert!(p.active_pool_count > 50, "expected real pools");
    assert!(p.registration_count > 500, "expected real registrations");
    // determinism: same bytes + same epoch -> byte-identical canonical commitment.
    let p2 = probe_ledgerdb_state(&state, nes_epoch).unwrap();
    assert_eq!(
        p.cert_state_commitment, p2.cert_state_commitment,
        "deterministic canonical CertState commitment"
    );
}
