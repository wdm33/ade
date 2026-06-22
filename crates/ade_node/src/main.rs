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

// MEM-OPT-OPS S1 (OP-MEM-02 / DC-MEM-06): process-wide allocator. mimalloc
// returns freed pages to the OS, so the transient seed-import peak no longer
// pins RSS the way the glibc arena does. RED runtime only — allocation
// addresses never enter a fingerprint (gate: ci_check_alloc_determinism_neutral.sh).
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
        Mode::Admission => {
            // Drop the wire-only writer immediately — admission mode
            // opens its own AdmissionLogWriter (bidirectional vocabulary
            // isolation per DC-ADMIT-04).
            drop(writer);
            let acli = match cli.extract_admission_cli() {
                Ok(a) => a,
                Err(e) => {
                    print_cli_error(&e);
                    return ExitCode::from(ade_node::EXIT_GENERIC_STARTUP as u8);
                }
            };
            ade_node::admission::dispatch_admission(acli, shutdown_rx).await
        }
        Mode::KeyGenKes => {
            // key-gen-KES is a one-shot command. The wire-only writer
            // opened at startup is not used (key-gen emits its closed
            // four-line CLI vocabulary directly to stdout); we drop it
            // so no empty JSONL file lingers.
            drop(writer);
            let _ = std::fs::remove_file(&cli.log_path);
            let kgc = match cli.extract_key_gen_kes_cli() {
                Ok(k) => k,
                Err(e) => {
                    print_cli_error(&e);
                    return ExitCode::from(ade_node::EXIT_GENERIC_STARTUP as u8);
                }
            };
            ade_node::run_key_gen_kes(kgc).await
        }
        Mode::Produce => {
            // Produce mode opens its own JSONL evidence writer
            // (ProducerLogEvent vocabulary); drop the wire-only
            // writer to avoid a stray empty file.
            drop(writer);
            let _ = std::fs::remove_file(&cli.log_path);
            let pcli = match cli.extract_produce_cli() {
                Ok(p) => p,
                Err(e) => {
                    print_cli_error(&e);
                    return ExitCode::from(ade_node::EXIT_GENERIC_STARTUP as u8);
                }
            };
            ade_node::produce_mode::run_produce_mode(pcli, shutdown_rx).await
        }
        Mode::Node => {
            // PHASE4-N-F-C: the real Ade node lifecycle owner. It opens
            // its own persistent stores; drop the wire-only writer so no
            // stray empty JSONL lingers.
            drop(writer);
            let _ = std::fs::remove_file(&cli.log_path);
            ade_node::run_node_lifecycle(cli, shutdown_rx).await
        }
        Mode::BootstrapExport => {
            // One-shot producer command; drop the wire-only writer + its stray log.
            drop(writer);
            let _ = std::fs::remove_file(&cli.log_path);
            let output_base = match cli.output_base.clone() {
                Some(p) => p,
                None => {
                    eprintln!("--mode bootstrap_export requires --output <base>");
                    return ExitCode::from(ade_node::EXIT_GENERIC_STARTUP as u8);
                }
            };
            match ade_node::bootstrap_export::run_bootstrap_export_command(
                &cli.network,
                &output_base,
                cli.keep_raw_capture,
            ) {
                Ok(r) => {
                    println!("bootstrap-export OK (network {}):", cli.network);
                    println!("  bundle:    {}  ({})", r.bundle_path, r.bundle_hash);
                    println!("  certstate: {}  ({})", r.certstate_path, r.certstate_hash);
                    println!("  manifest:  {}  ({})", r.manifest_path, r.manifest_hash);
                    println!("  inspect:   {}", r.inspect_path);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("bootstrap-export FAILED: {e:?}");
                    ExitCode::from(ade_node::bootstrap_export::EXIT_BOOTSTRAP_EXPORT_FAILURE as u8)
                }
            }
        }
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
                "ade_node: --mode {} is not a known mode (expected wire_only | admission | key_gen_kes | produce | node)",
                m
            );
        }
        CliError::InvalidTipReadTimeout(s) => {
            eprintln!(
                "ade_node: --tip-read-timeout-secs {} is not a valid u32",
                s
            );
        }
        CliError::InvalidSeedPointSlot(s) => {
            eprintln!("ade_node: --seed-point-slot {} is not a valid u64", s);
        }
        CliError::InvalidNetworkMagic(s) => {
            eprintln!("ade_node: --network-magic {} is not a valid u32", s);
        }
        CliError::AdmissionMissingFlag(name) => {
            eprintln!("ade_node: --mode admission requires {}", name);
        }
        CliError::AdmissionEmptyPeerList => {
            eprintln!("ade_node: --mode admission requires at least one --peer");
        }
        CliError::KeyGenMissingOutFile => {
            eprintln!("ade_node: --mode key_gen_kes requires --out-file");
        }
        CliError::InvalidPeriodIdx(s) => {
            eprintln!("ade_node: --period-idx {} is not a valid u32", s);
        }
        CliError::ProduceMissingFlag(name) => {
            eprintln!("ade_node: --mode produce requires {}", name);
        }
        CliError::InvalidMaxSlots(s) => {
            eprintln!("ade_node: --max-slots {} is not a valid u64", s);
        }
        CliError::ConflictingVenue => {
            eprintln!(
                "ade_node: --single-producer-venue and --participant-venue are mutually exclusive"
            );
        }
    }
}
