// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `ade_node` binary entry point (PHASE4-N-L-LIVE S2).
//!
//! Replaces the PHASE4-N-K honest-scope stub. Dispatches on
//! `cli.mode`:
//!
//! - `WireOnly` (default) — runs `wire_only::run_wire_only`:
//!   spawn one task per `--peer`, each dials TCP, completes
//!   the N2N handshake (`CN-SESS-02`), issues one chain-sync
//!   `FindIntersect(Origin)`, reads the peer tip, exits. The
//!   JSONL log under `--log PATH` (default
//!   `./wire_smoke.jsonl`) records the closed-vocabulary event
//!   stream (`LiveLogEvent`).
//!
//! - `Admission` — fails closed via
//!   `wire_only::run_admission_unavailable`. The ledger-seed
//!   prerequisite (genesis-JSON → initial-LedgerState or
//!   cardano-node ledger-snapshot importer) is the
//!   `PHASE4-N-M-LEDGER-SEED` cluster's deliverable. Until that
//!   lands, admission mode emits a single
//!   `NodeShutdown { reason: LedgerSeedUnavailable }` event and
//!   exits with `EXIT_GENERIC_STARTUP`.
//!
//! Doctrine: per
//! `~/.claude/projects/.../memory/feedback-shell-must-not-overstate-semantic-truth.md`,
//! the wire-only mode MUST NOT emit `agreement_verdict` /
//! `admitted_block` / `ledger_applied` / `projection_updated`.
//! The closed `LiveLogEvent` enum makes those event names
//! unrepresentable; the CI gate
//! `ci/ci_check_wire_only_event_vocabulary_closed.sh` enforces
//! the same at the file-tree level.

#![deny(unsafe_code)]

use std::fs::File;
use std::process::ExitCode;

use ade_node::{Cli, CliError, LiveLogWriter, Mode};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;

#[tokio::main]
async fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    let cli = match Cli::parse_from(argv) {
        Ok(c) => c,
        Err(e) => {
            print_cli_error(&e);
            return ExitCode::from(ade_node::EXIT_GENERIC_STARTUP as u8);
        }
    };

    let log_file = match File::create(&cli.log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "ade_node: cannot create log file {}: {:?}",
                cli.log_path.display(),
                e.kind()
            );
            return ExitCode::from(ade_node::EXIT_GENERIC_STARTUP as u8);
        }
    };
    let writer = LiveLogWriter::new(log_file);

    // Signal handler — flips the watch flag on SIGINT / SIGTERM
    // so the per-peer tasks can see the shutdown intent.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
        let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
        let _ = shutdown_tx.send(true);
    });

    match cli.mode {
        Mode::WireOnly => ade_node::run_wire_only(&cli, writer, shutdown_rx).await,
        Mode::Admission => ade_node::wire_only::run_admission_unavailable(writer).await,
    }
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
        CliError::UnknownMode(m) => {
            eprintln!(
                "ade_node: --mode {} is not a known mode (expected wire_only | admission)",
                m
            );
        }
        CliError::InvalidTipReadTimeout(s) => {
            eprintln!(
                "ade_node: --tip-read-timeout-secs {} is not a valid u32",
                s
            );
        }
    }
}
