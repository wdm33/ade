// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED CLI parser for `ade_node` (PHASE4-N-K S7).
//!
//! Mandatory: `--genesis-path PATH` (cold-start bundle directory).
//! Optional: `--network NAME` (metadata only), `--chain-db PATH`,
//! `--snapshot-store PATH`, `--listen ADDR`, `--peer ADDR` (repeatable).
//!
//! Parsing is intentionally argv-style (no external clap dep) so
//! the binary stays minimal. Errors map to deterministic
//! `CliError` values; the binary translates each into an exit
//! code via `main.rs`.

use std::path::PathBuf;

/// Closed mode discriminator. `WireOnly` is the only mode this
/// cluster (PHASE4-N-L-LIVE) ships. `Admission` is the
/// placeholder for `RO-LIVE-05` / `PHASE4-N-M-LEDGER-SEED`; the
/// binary fail-closed-exits when invoked in admission mode
/// without a ledger seed prerequisite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    WireOnly,
    Admission,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "wire_only" => Some(Self::WireOnly),
            "admission" => Some(Self::Admission),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub genesis_path: PathBuf,
    pub network: String,
    pub chain_db_path: Option<PathBuf>,
    pub snapshot_store_path: Option<PathBuf>,
    pub listen_addr: Option<String>,
    pub peer_addrs: Vec<String>,
    pub mode: Mode,
    pub log_path: PathBuf,
    pub tip_read_timeout_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    MissingGenesisPath,
    UnknownFlag(String),
    FlagMissingValue(String),
    UnknownMode(String),
    InvalidTipReadTimeout(String),
}

impl Cli {
    pub fn parse_from<I, S>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let argv: Vec<String> = args.into_iter().map(Into::into).collect();
        // Skip argv[0] (program name) if present.
        let mut iter = argv.into_iter().skip(1).peekable();
        let mut genesis_path: Option<PathBuf> = None;
        let mut network = String::from("preprod");
        let mut chain_db_path: Option<PathBuf> = None;
        let mut snapshot_store_path: Option<PathBuf> = None;
        let mut listen_addr: Option<String> = None;
        let mut peer_addrs: Vec<String> = Vec::new();
        let mut mode = Mode::WireOnly;
        let mut log_path = PathBuf::from("./wire_smoke.jsonl");
        let mut tip_read_timeout_secs: u32 = 30;

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--genesis-path" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--genesis-path".to_string())
                    })?;
                    genesis_path = Some(PathBuf::from(v));
                }
                "--network" => {
                    network = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--network".to_string())
                    })?;
                }
                "--chain-db" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--chain-db".to_string())
                    })?;
                    chain_db_path = Some(PathBuf::from(v));
                }
                "--snapshot-store" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--snapshot-store".to_string())
                    })?;
                    snapshot_store_path = Some(PathBuf::from(v));
                }
                "--listen" => {
                    let v = iter
                        .next()
                        .ok_or_else(|| CliError::FlagMissingValue("--listen".to_string()))?;
                    listen_addr = Some(v);
                }
                "--peer" => {
                    let v = iter
                        .next()
                        .ok_or_else(|| CliError::FlagMissingValue("--peer".to_string()))?;
                    peer_addrs.push(v);
                }
                "--mode" => {
                    let v = iter
                        .next()
                        .ok_or_else(|| CliError::FlagMissingValue("--mode".to_string()))?;
                    mode = Mode::parse(&v).ok_or(CliError::UnknownMode(v))?;
                }
                "--log" => {
                    let v = iter
                        .next()
                        .ok_or_else(|| CliError::FlagMissingValue("--log".to_string()))?;
                    log_path = PathBuf::from(v);
                }
                "--tip-read-timeout-secs" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--tip-read-timeout-secs".to_string())
                    })?;
                    tip_read_timeout_secs = v
                        .parse::<u32>()
                        .map_err(|_| CliError::InvalidTipReadTimeout(v))?;
                }
                other => return Err(CliError::UnknownFlag(other.to_string())),
            }
        }

        // genesis-path is required in admission mode; in wire-only
        // mode it remains required at CLI parse time (the operator
        // explicitly opts into "no admission" via --mode wire_only,
        // but the flag itself is still parsed for the future
        // admission cluster). Honest-scope: the wire-only path
        // does not actually consume genesis_path.
        let genesis_path = genesis_path.ok_or(CliError::MissingGenesisPath)?;
        Ok(Self {
            genesis_path,
            network,
            chain_db_path,
            snapshot_store_path,
            listen_addr,
            peer_addrs,
            mode,
            log_path,
            tip_read_timeout_secs,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, CliError> {
        let mut argv = vec!["ade_node".to_string()];
        argv.extend(args.iter().map(|s| s.to_string()));
        Cli::parse_from(argv)
    }

    #[test]
    fn cli_requires_genesis_path() {
        let err = parse(&[]).expect_err("must require genesis-path");
        assert_eq!(err, CliError::MissingGenesisPath);
    }

    #[test]
    fn cli_accepts_minimal_args() {
        let cli = parse(&["--genesis-path", "/etc/ade/genesis"]).expect("parse");
        assert_eq!(cli.genesis_path, PathBuf::from("/etc/ade/genesis"));
        assert_eq!(cli.network, "preprod");
        assert!(cli.chain_db_path.is_none());
        assert!(cli.peer_addrs.is_empty());
    }

    #[test]
    fn cli_accepts_full_args() {
        let cli = parse(&[
            "--genesis-path",
            "/g",
            "--network",
            "mainnet",
            "--chain-db",
            "/data/chain.db",
            "--snapshot-store",
            "/data/snap.db",
            "--listen",
            "0.0.0.0:3001",
            "--peer",
            "1.1.1.1:3001",
            "--peer",
            "2.2.2.2:3001",
        ])
        .expect("parse");
        assert_eq!(cli.network, "mainnet");
        assert_eq!(cli.listen_addr.as_deref(), Some("0.0.0.0:3001"));
        assert_eq!(cli.peer_addrs, vec!["1.1.1.1:3001", "2.2.2.2:3001"]);
    }

    #[test]
    fn cli_rejects_unknown_flag() {
        let err = parse(&["--genesis-path", "/g", "--bogus"]).expect_err("must reject");
        assert_eq!(err, CliError::UnknownFlag("--bogus".to_string()));
    }

    #[test]
    fn cli_rejects_flag_missing_value() {
        let err = parse(&["--genesis-path"]).expect_err("must reject");
        assert_eq!(
            err,
            CliError::FlagMissingValue("--genesis-path".to_string())
        );
    }
}
