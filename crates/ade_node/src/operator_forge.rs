// Core Contract:
// - RED shell: opens/reads operator key files; owns no authoritative state
// - Reuses the existing RED loaders; never reimplements key parsing
// - Key custody RED-confined to ProducerShell; no private byte ever escapes
// - Structured, secret-free errors; fail closed

//! RED operator-forge ingress (PHASE4-N-F-F S2).
//!
//! The single named `--mode node` operator-material ingress site. It consumes a
//! presence-validated [`ForgePaths`] (from the GREEN S1 classifier) and builds a
//! [`ProducerShell`] — the RED key-custody holder — by **reusing** the existing
//! cold / VRF / KES / opcert loaders. It reimplements no parser: the cardano-cli
//! cold/VRF text-envelope loaders and the KES-any-format / opcert-JSON helpers
//! are the same ones `produce_mode` uses (the latter two widened to
//! `pub(crate)`). KES-period-vs-opcert freshness is enforced inside
//! [`ProducerShell::init`] (carried, CN-PROD-02 / the cluster's I5), not here.
//!
//! Key custody is RED-confined: `load_operator_producer_shell` returns the shell
//! by value and the module exposes no private-key bytes — no byte accessor, no
//! serialization, no logging. Every failure is a structured, secret-free
//! [`OperatorForgeError`] (the inner `KeyLoadError` carries no path bytes per
//! OP-OPS-04; `ShellInitError` carries only detail strings + period numbers).
//!
//! This lands the RED-custody-loading half of `CN-NODE-03` (registry,
//! `declared`). It is RED and lands tested-but-unwired — nothing in the binary
//! path calls it until S3 assembles the `ForgeActivation` and flips `Some`/`None`.

use crate::forge_intent::ForgePaths;
use crate::produce_mode::{load_kes_skey_any_format, parse_simple_opcert_json};
use ade_runtime::producer::keys::{
    load_cold_signing_key_skey, load_vrf_signing_key_skey, KeyLoadError,
};
use ade_runtime::producer::producer_shell::{ProducerShell, ShellInitError};

/// Closed, secret-free error for node-path operator-material ingress. Each
/// variant wraps a structured loader / parser / init failure; none carries a
/// path string or key bytes (the inner `KeyLoadError` is path-byte-free per
/// OP-OPS-04, `ShellInitError` carries only detail strings + period numbers).
#[derive(Debug)]
pub enum OperatorForgeError {
    ColdKeyLoad(KeyLoadError),
    VrfKeyLoad(KeyLoadError),
    KesKeyLoad(KeyLoadError),
    OpcertParse(&'static str),
    ShellInit(ShellInitError),
}

impl std::fmt::Display for OperatorForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorForgeError::ColdKeyLoad(e) => {
                write!(f, "operator cold signing key load failed: {e:?}")
            }
            OperatorForgeError::VrfKeyLoad(e) => {
                write!(f, "operator VRF signing key load failed: {e:?}")
            }
            OperatorForgeError::KesKeyLoad(e) => {
                write!(f, "operator KES signing key load failed: {e:?}")
            }
            OperatorForgeError::OpcertParse(detail) => {
                write!(f, "operator opcert parse failed: {detail}")
            }
            OperatorForgeError::ShellInit(e) => {
                write!(f, "operator producer shell init failed: {e:?}")
            }
        }
    }
}

impl std::error::Error for OperatorForgeError {}

/// Load the complete operator key set into a [`ProducerShell`] (RED custody).
///
/// Loads cold → VRF → KES → opcert via the reused loaders, maps each failure to
/// the structured [`OperatorForgeError`], then `ProducerShell::init` (which
/// enforces the opcert shape + the KES-period-vs-opcert freshness bound). The
/// returned shell is the sole custody holder; this function exposes no private
/// byte. `paths` is already presence-validated (S1) — every field is present.
pub fn load_operator_producer_shell(
    paths: &ForgePaths,
) -> Result<ProducerShell, OperatorForgeError> {
    let cold = load_cold_signing_key_skey(&paths.cold).map_err(OperatorForgeError::ColdKeyLoad)?;
    let vrf = load_vrf_signing_key_skey(&paths.vrf).map_err(OperatorForgeError::VrfKeyLoad)?;
    let kes = load_kes_skey_any_format(&paths.kes).map_err(OperatorForgeError::KesKeyLoad)?;
    let opcert =
        parse_simple_opcert_json(&paths.opcert).map_err(OperatorForgeError::OpcertParse)?;
    ProducerShell::init(kes, vrf, cold, opcert).map_err(OperatorForgeError::ShellInit)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    fn hex_encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    fn write_cardano_cli_envelope(path: &Path, ty: &str, payload: &[u8]) {
        let cbor_hex = format!("58{:02x}{}", payload.len(), hex_encode(payload));
        let json = format!(
            "{{\"type\":\"{ty}\",\"description\":\"N-F-F S2 test fixture\",\"cborHex\":\"{cbor_hex}\"}}"
        );
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }

    /// Write a complete real-format operator key set (ade-native KES envelope,
    /// cardano-cli VRF/cold text-envelopes, opcert JSON whose hot_vkey is the
    /// KES vkey from the same seed). `opcert_kes_period` lets a test force a
    /// KES-period-vs-opcert mismatch. Returns the five paths as `ForgePaths`.
    fn write_operator_material(dir: &Path, opcert_kes_period: u64) -> ForgePaths {
        let kes_seed = [0x42u8; 32];
        let kes_path = dir.join("kes.ade.skey");
        ade_runtime::producer::keys::write_ade_kes_envelope(&kes_path, &kes_seed, 0).unwrap();

        let vrf_seed = [0x07u8; 32];
        let (vrf_sk_bytes, _) = cardano_crypto::vrf::VrfDraft03::keypair_from_seed(&vrf_seed);
        let vrf_path = dir.join("vrf.skey");
        write_cardano_cli_envelope(&vrf_path, "VrfSigningKey_PraosVRF", &vrf_sk_bytes);

        let cold_seed = [0x33u8; 32];
        let cold_path = dir.join("cold.skey");
        write_cardano_cli_envelope(&cold_path, "StakePoolSigningKey_ed25519", &cold_seed);

        use ade_crypto::kes_sum::KesAlgorithm;
        let kes_sk_raw =
            ade_crypto::kes_sum::Sum6Kes::gen_key_kes_from_seed_bytes(&kes_seed).unwrap();
        let kes_vkey = ade_crypto::kes_sum::Sum6Kes::derive_verification_key(&kes_sk_raw);
        let opcert_path = dir.join("opcert.json");
        let opcert_json = format!(
            r#"{{"hot_vkey_hex": "{}", "sequence_number": 0, "kes_period": {}, "sigma_hex": "{}"}}"#,
            hex_encode(&kes_vkey),
            opcert_kes_period,
            "0".repeat(128),
        );
        std::fs::write(&opcert_path, opcert_json).unwrap();

        ForgePaths {
            cold: cold_path,
            kes: kes_path,
            vrf: vrf_path,
            opcert: opcert_path,
            genesis: dir.join("genesis.json"), // unused in S2 (genesis parse is S3)
        }
    }

    #[test]
    fn load_operator_producer_shell_builds_shell_from_complete_material() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_operator_material(dir.path(), 0);
        let shell = load_operator_producer_shell(&paths).expect("complete material builds shell");
        // Public surface only — never private bytes.
        assert_eq!(shell.opcert().sequence_number, 0);
        assert_eq!(shell.opcert().kes_period, 0);
        // Deterministic public keys: a second load yields the same cold vkey.
        let shell2 = load_operator_producer_shell(&paths).expect("re-load");
        assert_eq!(shell.cold_vk().0, shell2.cold_vk().0);
        let _ = shell.vrf_verification_key();
        let _ = shell.public_metadata();
    }

    #[test]
    fn load_operator_producer_shell_missing_cold_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let mut paths = write_operator_material(dir.path(), 0);
        paths.cold = PathBuf::from("/nonexistent/cold.skey");
        let err = load_operator_producer_shell(&paths).expect_err("missing cold fails closed");
        assert!(
            matches!(err, OperatorForgeError::ColdKeyLoad(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn load_operator_producer_shell_missing_vrf_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let mut paths = write_operator_material(dir.path(), 0);
        paths.vrf = PathBuf::from("/nonexistent/vrf.skey");
        let err = load_operator_producer_shell(&paths).expect_err("missing vrf fails closed");
        assert!(
            matches!(err, OperatorForgeError::VrfKeyLoad(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn load_operator_producer_shell_bad_opcert_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_operator_material(dir.path(), 0);
        std::fs::write(&paths.opcert, b"not json").unwrap();
        let err = load_operator_producer_shell(&paths).expect_err("bad opcert fails closed");
        assert!(
            matches!(err, OperatorForgeError::OpcertParse(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn load_operator_producer_shell_kes_period_past_opcert_fails_closed() {
        // KES envelope is at period 0; an opcert anchored at period 5 puts the
        // current KES period below the opcert start => ShellInit fails closed
        // (the carried CN-PROD-02 / I5 freshness bound at init).
        let dir = tempfile::tempdir().unwrap();
        let paths = write_operator_material(dir.path(), 5);
        let err =
            load_operator_producer_shell(&paths).expect_err("kes/opcert mismatch fails closed");
        assert!(
            matches!(err, OperatorForgeError::ShellInit(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn operator_forge_error_carries_no_path_or_key_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let mut paths = write_operator_material(dir.path(), 0);
        let marker = "/super/secret/operator/cold-PATHMARKER.skey";
        paths.cold = PathBuf::from(marker);
        let err = load_operator_producer_shell(&paths).expect_err("missing cold fails closed");
        let dbg = format!("{err:?}");
        let disp = format!("{err}");
        assert!(!dbg.contains("PATHMARKER"), "Debug leaked a path: {dbg}");
        assert!(
            !disp.contains("PATHMARKER"),
            "Display leaked a path: {disp}"
        );
    }
}
