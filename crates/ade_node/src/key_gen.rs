// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! RED `key-gen-KES` mode (PHASE4-N-O S1).
//!
//! One-shot operator command. Sources 32 bytes of entropy from
//! `/dev/urandom` (or from a `--seed-file PATH` test seam),
//! materializes them into an `ade.kes.seed.v1` envelope via
//! [`ade_runtime::producer::keys::write_ade_kes_envelope`], reloads the
//! envelope as a self-check, derives the Sum6KES root verification-key
//! fingerprint, and prints **exactly four lines** to stdout — verbatim
//! from the user-provided operator spec:
//!
//! ```text
//! Generated Ade KES key: <out_file>
//! Format: ade.kes.seed.v1
//! Role: kes_hot_signing_key
//! Public verification key fingerprint: <hex>
//! ```
//!
//! Private-key material (the 32-byte seed) NEVER appears in stdout,
//! stderr, JSONL logs, or structured errors. The local seed buffer is
//! best-effort zeroized after the envelope is written. KesSecret itself
//! is dropped at end of function scope, which the `cardano-crypto`
//! crate zeroizes on drop.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use ade_runtime::producer::keys::{
    load_ade_kes_signing_key, write_ade_kes_envelope, KeyLoadError,
};

use crate::cli::KeyGenKesCli;

/// Exit code for any RED-side failure inside `key-gen-KES`. Reuses the
/// generic-startup exit code from the existing `ade_node` surface.
pub const EXIT_KEY_GEN_FAILURE: i32 = crate::node::EXIT_GENERIC_STARTUP;

/// Run the Ade-native KES key-gen mode.
pub async fn run_key_gen_kes(cli: KeyGenKesCli) -> ExitCode {
    let mut seed = match read_seed(&cli) {
        Ok(s) => s,
        Err(detail) => {
            eprintln!("ade_node key-gen-KES: {}", detail);
            return ExitCode::from(EXIT_KEY_GEN_FAILURE as u8);
        }
    };

    if let Err(err) = write_ade_kes_envelope(&cli.out_file, &seed, cli.period_idx) {
        eprintln!("ade_node key-gen-KES: write failed: {}", classify_err(&err));
        zeroize_seed(&mut seed);
        return ExitCode::from(EXIT_KEY_GEN_FAILURE as u8);
    }

    // Self-check: reload + derive the verification-key fingerprint.
    let kes_sk = match load_ade_kes_signing_key(&cli.out_file) {
        Ok(sk) => sk,
        Err(err) => {
            eprintln!(
                "ade_node key-gen-KES: round-trip self-check failed: {}",
                classify_err(&err)
            );
            zeroize_seed(&mut seed);
            return ExitCode::from(EXIT_KEY_GEN_FAILURE as u8);
        }
    };
    let vk_fp = kes_sk.verification_key_fingerprint();
    drop(kes_sk);
    zeroize_seed(&mut seed);

    println!("Generated Ade KES key: {}", cli.out_file.display());
    println!("Format: ade.kes.seed.v1");
    println!("Role: kes_hot_signing_key");
    println!("Public verification key fingerprint: {}", vk_fp);

    ExitCode::SUCCESS
}

// -------------------------------------------------------------------------
// Entropy source
// -------------------------------------------------------------------------

/// Read exactly 32 bytes from `--seed-file` (when supplied) or
/// `/dev/urandom`. Static `&'static str` detail strings only — never
/// the path / never the bytes themselves.
fn read_seed(cli: &KeyGenKesCli) -> Result<[u8; 32], &'static str> {
    let mut buf = [0u8; 32];
    match &cli.seed_file {
        Some(path) => read_exact_32(path, &mut buf).map_err(|e| match e {
            ReadExactError::IoOpen => "cannot open --seed-file",
            ReadExactError::IoRead => "cannot read from --seed-file",
            ReadExactError::ShortRead => "--seed-file does not contain 32 bytes",
        }),
        None => read_exact_32(&PathBuf::from("/dev/urandom"), &mut buf).map_err(|e| match e {
            ReadExactError::IoOpen => "cannot open /dev/urandom",
            ReadExactError::IoRead => "cannot read from /dev/urandom",
            ReadExactError::ShortRead => "/dev/urandom returned < 32 bytes",
        }),
    }?;
    Ok(buf)
}

enum ReadExactError {
    IoOpen,
    IoRead,
    ShortRead,
}

fn read_exact_32(path: &std::path::Path, out: &mut [u8; 32]) -> Result<(), ReadExactError> {
    let mut f = std::fs::File::open(path).map_err(|_| ReadExactError::IoOpen)?;
    let mut total = 0;
    while total < 32 {
        let n = f
            .read(&mut out[total..])
            .map_err(|_| ReadExactError::IoRead)?;
        if n == 0 {
            return Err(ReadExactError::ShortRead);
        }
        total += n;
    }
    Ok(())
}

fn zeroize_seed(seed: &mut [u8; 32]) {
    for b in seed.iter_mut() {
        *b = 0;
    }
    core::hint::black_box(seed);
}

// -------------------------------------------------------------------------
// Closed error classification — &'static str only. No path / no bytes.
// -------------------------------------------------------------------------

fn classify_err(err: &KeyLoadError) -> &'static str {
    match err {
        KeyLoadError::Io(_) => "filesystem error",
        KeyLoadError::MalformedEnvelope { .. } => "malformed envelope",
        KeyLoadError::UnexpectedType { .. } => "unexpected envelope type",
        KeyLoadError::CborHexDecode { .. } => "cbor-hex decode error",
        KeyLoadError::Crypto(_) => "crypto error",
        KeyLoadError::UnsupportedExpandedKesKeyFormat => "unsupported expanded KES skey format",
        KeyLoadError::AdeEnvelope(_) => "Ade envelope error",
        KeyLoadError::KesParse(_) => "cardano-cli expanded KES skey parse error",
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fixed_seed_file(dir: &std::path::Path, seed: [u8; 32]) -> PathBuf {
        let path = dir.join("seed.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&seed).unwrap();
        path
    }

    #[tokio::test]
    async fn run_key_gen_kes_writes_loadable_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let seed = [0x42u8; 32];
        let seed_path = fixed_seed_file(dir.path(), seed);
        let out_path = dir.path().join("kes.ade.skey");

        let cli = KeyGenKesCli {
            out_file: out_path.clone(),
            period_idx: 0,
            seed_file: Some(seed_path),
        };
        let exit = run_key_gen_kes(cli).await;
        assert_eq!(exit, ExitCode::SUCCESS);

        // The written envelope is loadable.
        let kes_sk = load_ade_kes_signing_key(&out_path).unwrap();
        assert_eq!(kes_sk.current_period().0, 0);
    }

    #[tokio::test]
    async fn run_key_gen_kes_writes_at_loaded_period() {
        let dir = tempfile::tempdir().unwrap();
        let seed = [0x07u8; 32];
        let seed_path = fixed_seed_file(dir.path(), seed);
        let out_path = dir.path().join("kes.ade.skey");

        let cli = KeyGenKesCli {
            out_file: out_path.clone(),
            period_idx: 12,
            seed_file: Some(seed_path),
        };
        let exit = run_key_gen_kes(cli).await;
        assert_eq!(exit, ExitCode::SUCCESS);

        let kes_sk = load_ade_kes_signing_key(&out_path).unwrap();
        assert_eq!(kes_sk.current_period().0, 12);
    }

    #[tokio::test]
    async fn run_key_gen_kes_rejects_short_seed_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("seed.bin");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&[0x11u8; 16])
            .unwrap();

        let cli = KeyGenKesCli {
            out_file: dir.path().join("kes.ade.skey"),
            period_idx: 0,
            seed_file: Some(path),
        };
        let exit = run_key_gen_kes(cli).await;
        assert_eq!(exit, ExitCode::from(EXIT_KEY_GEN_FAILURE as u8));
    }
}
