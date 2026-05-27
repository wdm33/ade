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
    /// PHASE4-N-O: one-shot Ade-native KES key generation. Emits an
    /// `ade.kes.seed.v1` envelope at `--out-file PATH`. Does not
    /// require `--genesis-path` or any peer/admission flag.
    KeyGenKes,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "wire_only" => Some(Self::WireOnly),
            "admission" => Some(Self::Admission),
            "key_gen_kes" => Some(Self::KeyGenKes),
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
    // Admission-mode-only flags. Each is required when
    // --mode admission is set (validated via
    // [`Cli::extract_admission_cli`]). They remain `Option` on
    // `Cli` itself so wire-only mode does not consume them.
    pub json_seed_path: Option<PathBuf>,
    pub seed_point_slot: Option<u64>,
    pub seed_block_hash_hex: Option<String>,
    pub wal_dir: Option<PathBuf>,
    pub snapshot_dir: Option<PathBuf>,
    pub network_magic: Option<u32>,
    pub genesis_hash_hex: Option<String>,
    /// Path to the cardano-cli operator consensus-inputs JSON
    /// bundle imported by the admission runner (PHASE4-N-M-C
    /// CN-CONS-IN-01). Required when `--mode admission` is set.
    pub consensus_inputs_path: Option<PathBuf>,
    // -------------------------------------------------------------------
    // PHASE4-N-O — KeyGenKes-mode flags. Each is parsed unconditionally
    // and validated only when `--mode key_gen_kes` is set (via
    // [`Cli::extract_key_gen_kes_cli`]).
    // -------------------------------------------------------------------
    /// Target path for the emitted `ade.kes.seed.v1` envelope. Required
    /// when `--mode key_gen_kes` is set.
    pub out_file: Option<PathBuf>,
    /// KES period index to encode into the envelope (default 0).
    pub period_idx: Option<u32>,
    /// Optional test seam: 32 bytes of seed material read from this
    /// file instead of `/dev/urandom`. Honest-scope: production use
    /// MUST omit this flag.
    pub seed_file: Option<PathBuf>,
}

/// Closed admission-mode CLI bundle (B5).
///
/// Extracted from `Cli` via [`Cli::extract_admission_cli`] which
/// validates every required admission flag is present. No `Option`
/// fields; no `Default` impl; no `#[non_exhaustive]` — construction
/// failure is a CLI parse error, not a runtime error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionCli {
    pub json_seed_path: PathBuf,
    pub seed_point_slot: u64,
    pub seed_block_hash_hex: String,
    pub peer_addrs: Vec<String>,
    pub wal_dir: PathBuf,
    pub snapshot_dir: PathBuf,
    pub log_path: PathBuf,
    pub network_magic: u32,
    pub genesis_hash_hex: String,
    /// Operator consensus-inputs JSON bundle path
    /// (PHASE4-N-M-C). Imported via `import_live_consensus_inputs`
    /// before the runner starts.
    pub consensus_inputs_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    MissingGenesisPath,
    UnknownFlag(String),
    FlagMissingValue(String),
    UnknownMode(String),
    InvalidTipReadTimeout(String),
    InvalidSeedPointSlot(String),
    InvalidNetworkMagic(String),
    AdmissionMissingFlag(&'static str),
    AdmissionEmptyPeerList,
    // PHASE4-N-O — KeyGenKes-mode errors.
    KeyGenMissingOutFile,
    InvalidPeriodIdx(String),
}

/// Closed PHASE4-N-O `key_gen_kes` CLI bundle. Constructed via
/// [`Cli::extract_key_gen_kes_cli`] which validates that the
/// load-bearing flags are present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyGenKesCli {
    pub out_file: PathBuf,
    pub period_idx: u32,
    pub seed_file: Option<PathBuf>,
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
        let mut json_seed_path: Option<PathBuf> = None;
        let mut seed_point_slot: Option<u64> = None;
        let mut seed_block_hash_hex: Option<String> = None;
        let mut wal_dir: Option<PathBuf> = None;
        let mut snapshot_dir: Option<PathBuf> = None;
        let mut network_magic: Option<u32> = None;
        let mut genesis_hash_hex: Option<String> = None;
        let mut consensus_inputs_path: Option<PathBuf> = None;
        let mut out_file: Option<PathBuf> = None;
        let mut period_idx: Option<u32> = None;
        let mut seed_file: Option<PathBuf> = None;

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
                "--json-seed" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--json-seed".to_string())
                    })?;
                    json_seed_path = Some(PathBuf::from(v));
                }
                "--seed-point-slot" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--seed-point-slot".to_string())
                    })?;
                    seed_point_slot = Some(
                        v.parse::<u64>()
                            .map_err(|_| CliError::InvalidSeedPointSlot(v))?,
                    );
                }
                "--seed-block-hash" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--seed-block-hash".to_string())
                    })?;
                    seed_block_hash_hex = Some(v);
                }
                "--wal-dir" => {
                    let v = iter
                        .next()
                        .ok_or_else(|| CliError::FlagMissingValue("--wal-dir".to_string()))?;
                    wal_dir = Some(PathBuf::from(v));
                }
                "--snapshot-dir" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--snapshot-dir".to_string())
                    })?;
                    snapshot_dir = Some(PathBuf::from(v));
                }
                "--network-magic" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--network-magic".to_string())
                    })?;
                    network_magic = Some(
                        v.parse::<u32>()
                            .map_err(|_| CliError::InvalidNetworkMagic(v))?,
                    );
                }
                "--genesis-hash" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--genesis-hash".to_string())
                    })?;
                    genesis_hash_hex = Some(v);
                }
                "--consensus-inputs-path" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--consensus-inputs-path".to_string())
                    })?;
                    consensus_inputs_path = Some(PathBuf::from(v));
                }
                "--out-file" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--out-file".to_string())
                    })?;
                    out_file = Some(PathBuf::from(v));
                }
                "--period-idx" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--period-idx".to_string())
                    })?;
                    period_idx = Some(
                        v.parse::<u32>()
                            .map_err(|_| CliError::InvalidPeriodIdx(v))?,
                    );
                }
                "--seed-file" => {
                    let v = iter.next().ok_or_else(|| {
                        CliError::FlagMissingValue("--seed-file".to_string())
                    })?;
                    seed_file = Some(PathBuf::from(v));
                }
                other => return Err(CliError::UnknownFlag(other.to_string())),
            }
        }

        // genesis-path is required in admission and wire-only modes.
        // KeyGenKes is a one-shot operator command with no chain
        // context; --genesis-path is not relevant. We synthesize a
        // sentinel placeholder so the field stays non-optional for
        // downstream consumers.
        let genesis_path = if mode == Mode::KeyGenKes {
            genesis_path.unwrap_or_else(PathBuf::new)
        } else {
            genesis_path.ok_or(CliError::MissingGenesisPath)?
        };
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
            json_seed_path,
            seed_point_slot,
            seed_block_hash_hex,
            wal_dir,
            snapshot_dir,
            network_magic,
            genesis_hash_hex,
            consensus_inputs_path,
            out_file,
            period_idx,
            seed_file,
        })
    }

    /// Validate `key_gen_kes`-mode requirements + return a closed
    /// [`KeyGenKesCli`]. Missing `--out-file` surfaces as
    /// `CliError::KeyGenMissingOutFile`.
    pub fn extract_key_gen_kes_cli(&self) -> Result<KeyGenKesCli, CliError> {
        let out_file = self
            .out_file
            .clone()
            .ok_or(CliError::KeyGenMissingOutFile)?;
        Ok(KeyGenKesCli {
            out_file,
            period_idx: self.period_idx.unwrap_or(0),
            seed_file: self.seed_file.clone(),
        })
    }

    /// Validate that every required admission-mode flag is present
    /// + return a closed [`AdmissionCli`] bundle. Missing flags
    /// surface as `CliError::AdmissionMissingFlag(<flag-name>)`.
    pub fn extract_admission_cli(&self) -> Result<AdmissionCli, CliError> {
        let json_seed_path = self
            .json_seed_path
            .clone()
            .ok_or(CliError::AdmissionMissingFlag("--json-seed"))?;
        let seed_point_slot = self
            .seed_point_slot
            .ok_or(CliError::AdmissionMissingFlag("--seed-point-slot"))?;
        let seed_block_hash_hex = self
            .seed_block_hash_hex
            .clone()
            .ok_or(CliError::AdmissionMissingFlag("--seed-block-hash"))?;
        let wal_dir = self
            .wal_dir
            .clone()
            .ok_or(CliError::AdmissionMissingFlag("--wal-dir"))?;
        let snapshot_dir = self
            .snapshot_dir
            .clone()
            .ok_or(CliError::AdmissionMissingFlag("--snapshot-dir"))?;
        let network_magic = self
            .network_magic
            .ok_or(CliError::AdmissionMissingFlag("--network-magic"))?;
        let genesis_hash_hex = self
            .genesis_hash_hex
            .clone()
            .ok_or(CliError::AdmissionMissingFlag("--genesis-hash"))?;
        let consensus_inputs_path = self
            .consensus_inputs_path
            .clone()
            .ok_or(CliError::AdmissionMissingFlag("--consensus-inputs-path"))?;
        if self.peer_addrs.is_empty() {
            return Err(CliError::AdmissionEmptyPeerList);
        }
        Ok(AdmissionCli {
            json_seed_path,
            seed_point_slot,
            seed_block_hash_hex,
            peer_addrs: self.peer_addrs.clone(),
            wal_dir,
            snapshot_dir,
            log_path: self.log_path.clone(),
            network_magic,
            genesis_hash_hex,
            consensus_inputs_path,
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

    fn parse_admission(extra: &[&str]) -> Result<AdmissionCli, CliError> {
        let mut args = vec![
            "--genesis-path".to_string(),
            "/g".to_string(),
            "--mode".to_string(),
            "admission".to_string(),
            "--json-seed".to_string(),
            "/seed.json".to_string(),
            "--seed-point-slot".to_string(),
            "12345".to_string(),
            "--seed-block-hash".to_string(),
            "aa".repeat(32),
            "--wal-dir".to_string(),
            "/wal".to_string(),
            "--snapshot-dir".to_string(),
            "/snap".to_string(),
            "--network-magic".to_string(),
            "1".to_string(),
            "--genesis-hash".to_string(),
            "bb".repeat(32),
            "--peer".to_string(),
            "127.0.0.1:3001".to_string(),
            "--consensus-inputs-path".to_string(),
            "/cinputs.json".to_string(),
        ];
        for s in extra {
            args.push(s.to_string());
        }
        let cli = parse(&args.iter().map(String::as_str).collect::<Vec<_>>())?;
        cli.extract_admission_cli()
    }

    #[test]
    fn admission_cli_parses_full_arg_set() {
        let acli = parse_admission(&[]).expect("parse");
        assert_eq!(acli.seed_point_slot, 12345);
        assert_eq!(acli.network_magic, 1);
        assert_eq!(acli.peer_addrs, vec!["127.0.0.1:3001"]);
    }

    #[test]
    fn admission_cli_rejects_missing_required_flag() {
        // Drop --json-seed.
        let args = vec![
            "--genesis-path",
            "/g",
            "--mode",
            "admission",
            // (no --json-seed)
            "--seed-point-slot",
            "1",
            "--seed-block-hash",
            "aa",
            "--wal-dir",
            "/w",
            "--snapshot-dir",
            "/s",
            "--network-magic",
            "1",
            "--genesis-hash",
            "bb",
            "--peer",
            "1.1.1.1:1",
        ];
        let cli = parse(&args).expect("base parse");
        let err = cli.extract_admission_cli().expect_err("must reject");
        assert_eq!(err, CliError::AdmissionMissingFlag("--json-seed"));
    }

    #[test]
    fn admission_cli_rejects_unknown_flag() {
        let err = parse(&["--genesis-path", "/g", "--mode", "admission", "--bogus"])
            .expect_err("must reject");
        assert_eq!(err, CliError::UnknownFlag("--bogus".to_string()));
    }

    #[test]
    fn admission_cli_parses_repeatable_peer_flag() {
        let acli =
            parse_admission(&["--peer", "2.2.2.2:3001", "--peer", "3.3.3.3:3001"]).expect("ok");
        assert_eq!(
            acli.peer_addrs,
            vec![
                "127.0.0.1:3001".to_string(),
                "2.2.2.2:3001".to_string(),
                "3.3.3.3:3001".to_string(),
            ]
        );
    }

    #[test]
    fn admission_cli_rejects_empty_peer_list() {
        let args = vec![
            "--genesis-path",
            "/g",
            "--mode",
            "admission",
            "--json-seed",
            "/s.json",
            "--seed-point-slot",
            "1",
            "--seed-block-hash",
            "aa",
            "--wal-dir",
            "/w",
            "--snapshot-dir",
            "/s",
            "--network-magic",
            "1",
            "--genesis-hash",
            "bb",
            "--consensus-inputs-path",
            "/cinputs.json",
            // no --peer at all
        ];
        let cli = parse(&args).expect("base parse");
        let err = cli.extract_admission_cli().expect_err("must reject");
        assert_eq!(err, CliError::AdmissionEmptyPeerList);
    }

    #[test]
    fn admission_cli_rejects_invalid_seed_point_slot() {
        let err = parse(&[
            "--genesis-path",
            "/g",
            "--mode",
            "admission",
            "--seed-point-slot",
            "not-a-number",
        ])
        .expect_err("must reject");
        assert_eq!(
            err,
            CliError::InvalidSeedPointSlot("not-a-number".to_string())
        );
    }

    // =====================================================================
    // PHASE4-N-O — key_gen_kes mode
    // =====================================================================

    #[test]
    fn cli_parses_key_gen_kes_mode_with_out_file() {
        let cli = parse(&[
            "--mode",
            "key_gen_kes",
            "--out-file",
            "/tmp/kes.ade.skey",
        ])
        .expect("parse");
        assert_eq!(cli.mode, Mode::KeyGenKes);
        let kgc = cli.extract_key_gen_kes_cli().expect("extract");
        assert_eq!(kgc.out_file, PathBuf::from("/tmp/kes.ade.skey"));
        assert_eq!(kgc.period_idx, 0);
        assert!(kgc.seed_file.is_none());
    }

    #[test]
    fn cli_parses_key_gen_kes_with_period_idx() {
        let cli = parse(&[
            "--mode",
            "key_gen_kes",
            "--out-file",
            "/tmp/kes.ade.skey",
            "--period-idx",
            "17",
        ])
        .expect("parse");
        let kgc = cli.extract_key_gen_kes_cli().expect("extract");
        assert_eq!(kgc.period_idx, 17);
    }

    #[test]
    fn cli_parses_key_gen_kes_with_seed_file() {
        let cli = parse(&[
            "--mode",
            "key_gen_kes",
            "--out-file",
            "/tmp/kes.ade.skey",
            "--seed-file",
            "/tmp/seed.bin",
        ])
        .expect("parse");
        let kgc = cli.extract_key_gen_kes_cli().expect("extract");
        assert_eq!(kgc.seed_file, Some(PathBuf::from("/tmp/seed.bin")));
    }

    #[test]
    fn cli_rejects_key_gen_kes_without_out_file() {
        let cli = parse(&["--mode", "key_gen_kes"]).expect("base parse");
        let err = cli.extract_key_gen_kes_cli().expect_err("must reject");
        assert_eq!(err, CliError::KeyGenMissingOutFile);
    }

    #[test]
    fn cli_rejects_key_gen_kes_with_bad_period_idx() {
        let err = parse(&[
            "--mode",
            "key_gen_kes",
            "--out-file",
            "/tmp/kes.ade.skey",
            "--period-idx",
            "not-a-number",
        ])
        .expect_err("must reject");
        assert_eq!(
            err,
            CliError::InvalidPeriodIdx("not-a-number".to_string())
        );
    }

    #[test]
    fn cli_key_gen_kes_does_not_require_genesis_path() {
        // key_gen_kes is a one-shot operator command; --genesis-path is
        // not relevant. CLI parse must succeed without it.
        let _cli = parse(&[
            "--mode",
            "key_gen_kes",
            "--out-file",
            "/tmp/kes.ade.skey",
        ])
        .expect("parse without --genesis-path");
    }
}
