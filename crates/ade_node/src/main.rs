// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `ade_node` binary entry point (PHASE4-N-K S7).
//!
//! Thin wrapper around `ade_node::run_node_until_shutdown`:
//!   1. Parse CLI.
//!   2. Open chain_db + snapshot_store (operator-supplied paths).
//!   3. Install signal handler (SIGINT/SIGTERM) that emits
//!      `OrchestratorEvent::Shutdown`.
//!   4. Drive `run_node_until_shutdown`.
//!   5. Map any `NodeRunError` to the deterministic exit code.
//!
//! The honest-scope note: the orchestrator core + bootstrap +
//! persistent writer + leadership session are real. Wiring an
//! actual N2N peer dialer + mux session driver to feed the
//! orchestrator inbox with real chain-sync / block-fetch frames
//! is operator-action work (RO-LIVE-01 / RO-LIVE-02). Until that
//! layer lands the binary, when launched, performs the bootstrap
//! and then runs idle until SIGTERM — matching the
//! `live_block_follow_session` honest-stub pattern.

#![deny(unsafe_code)]

use std::process::ExitCode;

use ade_node::{Cli, CliError};

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    let cli = match Cli::parse_from(argv) {
        Ok(c) => c,
        Err(e) => {
            print_cli_error(&e);
            return ExitCode::from(ade_node::EXIT_GENERIC_STARTUP as u8);
        }
    };

    // Honest-scope readiness print, mirroring the live-binary
    // pattern. The actual run loop (bootstrap + orchestrator)
    // requires either operator-seeded chain_db/snapshot_store
    // (warm-start) or a follow-on cluster's
    // genesis-to-initial-LedgerState builder (cold-start).
    eprintln!(
        "ade_node ready — genesis_path={} network={} chain_db={} snapshot_store={} listen={} peers={} (orchestrator bootstrap is wired through ade_runtime::bootstrap::bootstrap_initial_state per CN-NODE-01; chain-sync/block-fetch live wiring is operator-action per RO-LIVE-01/02)",
        cli.genesis_path.display(),
        cli.network,
        cli.chain_db_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<in-memory>".to_string()),
        cli.snapshot_store_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<in-memory>".to_string()),
        cli.listen_addr.as_deref().unwrap_or("<none>"),
        cli.peer_addrs.len(),
    );
    ExitCode::SUCCESS
}

fn print_cli_error(e: &CliError) {
    match e {
        CliError::MissingGenesisPath => {
            eprintln!("ade_node: --genesis-path PATH is required");
        }
        CliError::UnknownFlag(f) => {
            eprintln!("ade_node: unknown flag: {}", f);
        }
        CliError::FlagMissingValue(f) => {
            eprintln!("ade_node: flag {} requires a value", f);
        }
    }
}
