//! Local Preview corpus acceptance for the native V2 LedgerDB `state` decoder
//! (MITHRIL-VERIFIED-ANCHOR-IMPORT, Stage 1). The corpus is a real cardano-node Preview snapshot
//! `state` file (NOT committed — the node rotates it); point the env var / default path at one.
//! Skips cleanly when the corpus is absent (CI without a live node). RED: reads a local file.

use ade_ledger::ledgerdb_state::probe_ledgerdb_state;

#[test]
fn decode_local_preview_corpus() {
    let path = std::env::var("ADE_LEDGERDB_CORPUS")
        .unwrap_or_else(|_| "/tmp/ade-snap-state.cbor".to_string());
    let state = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("SKIP: no V2 LedgerDB corpus at {path}");
            return;
        }
    };
    let slot: u64 = std::fs::read_to_string("/tmp/ade-snap-slot.txt")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    // Preview is Shelley-from-genesis: epoch = slot / epoch_length (86400).
    let expected_epoch = slot / 86400;
    eprintln!(
        "corpus {} bytes, slot {slot}, expected_epoch {expected_epoch}",
        state.len()
    );
    match probe_ledgerdb_state(&state, expected_epoch) {
        Ok(probe) => {
            eprintln!("PROBE OK: {probe:#?}");
            assert_eq!(probe.era_index, 6, "current era must be Conway");
            assert_eq!(
                probe.vrf_count, probe.active_pool_count,
                "every active pool must carry a real VRF"
            );
            assert!(probe.active_pool_count > 100, "expected hundreds of pools");
            assert!(
                probe.registration_count > 1000,
                "expected thousands of registrations"
            );
            // determinism: same bytes + same epoch -> byte-identical canonical commitment.
            let probe2 = probe_ledgerdb_state(&state, expected_epoch).expect("re-probe");
            assert_eq!(
                probe.cert_state_commitment, probe2.cert_state_commitment,
                "deterministic canonical CertState commitment"
            );
        }
        Err(e) => panic!("PROBE FAILED: {e:?}"),
    }
}
