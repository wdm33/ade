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
use crate::produce_mode::load_kes_skey_any_format;
use ade_runtime::producer::coordinator::GenesisAnchor;
use ade_runtime::producer::genesis_parser::{parse_shelley_genesis, GenesisParseError};
use ade_runtime::producer::keys::{
    load_cold_signing_key_skey, load_vrf_signing_key_skey, KeyLoadError,
};
use ade_runtime::producer::opcert_envelope::{parse_opcert_envelope, OpCertParseError};
use ade_runtime::producer::producer_shell::{ProducerShell, ShellInitError};
use ade_types::{Hash28, SlotNo};

/// Closed, secret-free error for node-path operator-material ingress. Each
/// variant wraps a structured loader / parser / init failure; none carries a
/// path string or key bytes (the inner `KeyLoadError` is path-byte-free per
/// OP-OPS-04, `ShellInitError` carries only detail strings + period numbers).
#[derive(Debug)]
pub enum OperatorForgeError {
    ColdKeyLoad(KeyLoadError),
    VrfKeyLoad(KeyLoadError),
    KesKeyLoad(KeyLoadError),
    OpcertParse(OpCertParseError),
    ShellInit(ShellInitError),
    GenesisParse(GenesisParseError),
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
            OperatorForgeError::OpcertParse(e) => {
                write!(f, "operator opcert parse failed: {e:?}")
            }
            OperatorForgeError::ShellInit(e) => {
                write!(f, "operator producer shell init failed: {e:?}")
            }
            OperatorForgeError::GenesisParse(e) => {
                write!(f, "operator genesis anchor parse failed: {e:?}")
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
    // Real cardano-cli `NodeOperationalCertificate` envelope ingress on the node
    // path (PHASE4-N-F-G-A S2) — retires `parse_simple_opcert_json`. The opcert
    // metadata/counter (hot_vkey, sequence_number, kes_period, sigma) is the real
    // parser's output; the cold VK in the envelope is discarded (the shell's cold
    // key is the custody authority).
    let opcert_bytes = std::fs::read(&paths.opcert)
        .map_err(|_| OperatorForgeError::OpcertParse(OpCertParseError::JsonShape))?;
    let opcert = parse_opcert_envelope(&opcert_bytes)
        .map_err(OperatorForgeError::OpcertParse)?
        .opcert;
    ProducerShell::init(kes, vrf, cold, opcert).map_err(OperatorForgeError::ShellInit)
}

/// The operator-material-backed forge inputs the `--mode node` arm needs to
/// build a `ForgeActivation` (PHASE4-N-F-F S3) — everything EXCEPT the recovered
/// `BootstrapState` (the lifecycle's single recovered state, borrowed by the
/// caller) and the injected clock (RED-owned by the caller).
///
/// `genesis` is returned so the caller can build the `CoordinatorState` (the
/// genesis-anchor host for the reused `kes_period_for_slot`); this module does
/// NOT build a `CoordinatorState` (kept caller-side so custody/no-leak stays
/// trivially gated).
pub struct OperatorForgeMaterial {
    pub shell: ProducerShell,
    pub genesis: GenesisAnchor,
    pub pool_id: Hash28,
    pub anchor_millis: u64,
    pub start_slot: SlotNo,
    pub slot_length_ms: u32,
}

/// Slot at which KES period 0 begins — the genesis KES-period origin. KES periods
/// are counted from the genesis slot (0) at `slotsPerKESPeriod` cadence, so this is
/// `0` for a from-genesis network. `parse_shelley_genesis` requires it as an
/// argument (the real `shelley-genesis.json` does NOT carry it); the node path
/// supplies the proven genesis origin, never a fabricated chain fact
/// (PHASE4-N-F-G-A S2 PO-3). `ProducerShell::init`'s CN-PROD-02 freshness bound
/// mechanically enforces consistency with the operator opcert's `kes_period`.
const GENESIS_KES_PERIOD_ORIGIN_SLOT: u64 = 0;

/// Build the operator-material-backed forge inputs from a presence-validated
/// [`ForgePaths`].
///
/// **Mithril boundary (load-bearing).** This is operator signing-material
/// ingress, NOT bootstrap. It does NOT call Mithril, does NOT create a second
/// bootstrap path, and does NOT re-derive initial state. The real
/// `parse_shelley_genesis` (PHASE4-N-F-G-A S2) extracts ONLY the clock/KES/network
/// constants (`networkMagic`, `systemStart`→`slot_zero_time_unix_ms`,
/// `slotLength`→`slot_length_ms`, the KES fields) for the activation — it is NOT a
/// starting-state source and NOT a new semantic genesis authority. The forge base
/// is the single recovered `BootstrapState` the lifecycle's FirstRun (Mithril) /
/// WarmStart (WAL) arm already produced; the caller borrows it.
///
/// `pool_id` is derived in this ONE named place from the operator cold key
/// (`blake2b_224(cold_vk)`) — never fabricated. `protocol_version` + `pparams` are
/// **no longer produced here**: they are sourced from the recovered ledger's
/// current `protocol_params` (installed by S2a) at `ForgeActivation` construction,
/// so the forge reads a truthful current protocol version, not a fabricated default.
pub fn build_operator_forge_material(
    paths: &ForgePaths,
) -> Result<OperatorForgeMaterial, OperatorForgeError> {
    let shell = load_operator_producer_shell(paths)?;
    // Real cardano-cli `shelley-genesis.json` ingress on the node path (S2) —
    // retires `parse_simple_genesis_json`; clock/KES/network constants ONLY.
    let genesis_bytes = std::fs::read(&paths.genesis)
        .map_err(|_| OperatorForgeError::GenesisParse(GenesisParseError::JsonShape))?;
    let genesis = parse_shelley_genesis(&genesis_bytes, GENESIS_KES_PERIOD_ORIGIN_SLOT)
        .map_err(OperatorForgeError::GenesisParse)?;
    // pool_id from the operator cold verification key — the one named derivation.
    let pool_id = Hash28(ade_crypto::blake2b_224(&shell.cold_vk().0).0);
    // Clock-seam anchors (DC-NODE-03 / DC-NODE-05): slot_zero_time IS slot 0's
    // wall-clock time, so the conversion anchor is (slot_zero_time_unix_ms,
    // start_slot = 0, slot_length_ms). Read the Copy fields before moving genesis.
    let anchor_millis = genesis.slot_zero_time_unix_ms;
    let slot_length_ms = u32::try_from(genesis.slot_length_ms)
        .unwrap_or(u32::MAX)
        .max(1);
    Ok(OperatorForgeMaterial {
        shell,
        genesis,
        pool_id,
        anchor_millis,
        start_slot: SlotNo(0),
        slot_length_ms,
    })
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

    /// Minimal canonical CBOR uint (covers the small seq / kes_period values the
    /// tests use).
    fn cbor_uint(n: u64) -> Vec<u8> {
        if n < 24 {
            vec![n as u8]
        } else if n < 256 {
            vec![0x18, n as u8]
        } else {
            vec![0x19, (n >> 8) as u8, n as u8]
        }
    }

    /// Write a REAL cardano-cli `NodeOperationalCertificate` text envelope: the
    /// `cborHex` is CBOR `array(2)` of `[array(4)[hot_vkey(bytes32), seq(uint),
    /// kes_period(uint), sigma(bytes64)], cold_vk(bytes32)]` (the exact shape
    /// `parse_opcert_envelope` locks). The cold VK is discarded by the node path.
    fn write_real_opcert_envelope(
        path: &Path,
        hot_vkey: &[u8],
        seq: u64,
        kes_period: u64,
        sigma: &[u8],
        cold_vk: &[u8],
    ) {
        let mut cbor = vec![0x82, 0x84]; // array(2) [ array(4) ...
        cbor.extend_from_slice(&[0x58, 0x20]); // bytes(32)
        cbor.extend_from_slice(hot_vkey);
        cbor.extend_from_slice(&cbor_uint(seq));
        cbor.extend_from_slice(&cbor_uint(kes_period));
        cbor.extend_from_slice(&[0x58, 0x40]); // bytes(64)
        cbor.extend_from_slice(sigma);
        cbor.extend_from_slice(&[0x58, 0x20]); // bytes(32) cold_vk
        cbor.extend_from_slice(cold_vk);
        let json = format!(
            "{{\"type\":\"NodeOperationalCertificate\",\"description\":\"\",\"cborHex\":\"{}\"}}",
            hex_encode(&cbor)
        );
        std::fs::write(path, json).unwrap();
    }

    /// Write a complete real-format operator key set (ade-native KES envelope,
    /// cardano-cli VRF/cold text-envelopes, a REAL `NodeOperationalCertificate`
    /// envelope whose hot_vkey is the KES vkey from the same seed, and a REAL
    /// `shelley-genesis.json`). `opcert_kes_period` lets a test force a
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
        write_real_opcert_envelope(
            &opcert_path,
            &kes_vkey,
            0,
            opcert_kes_period,
            &[0u8; 64],
            &[0u8; 32],
        );

        // REAL shelley-genesis.json (clock/KES/network constants only; S2). The
        // node path supplies kes_anchor_slot as the genesis origin (not in genesis);
        // systemStart 2022-06-01T00:00:00Z = 1_654_041_600_000 ms; slotLength 1s.
        let genesis_path = dir.join("genesis.json");
        std::fs::write(
            &genesis_path,
            br#"{
                "networkMagic": 1,
                "systemStart": "2022-06-01T00:00:00Z",
                "slotLength": 1,
                "slotsPerKESPeriod": 129600,
                "maxKESEvolutions": 63
            }"#,
        )
        .unwrap();

        ForgePaths {
            cold: cold_path,
            kes: kes_path,
            vrf: vrf_path,
            opcert: opcert_path,
            genesis: genesis_path,
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

    // ---- S3: build_operator_forge_material -------------------------------

    #[test]
    fn build_operator_forge_material_from_complete_material() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_operator_material(dir.path(), 0);
        let mat = build_operator_forge_material(&paths).expect("complete material builds");
        // pool_id derived from the operator cold vkey — the one named place.
        let expected_pool = Hash28(ade_crypto::blake2b_224(&mat.shell.cold_vk().0).0);
        assert_eq!(mat.pool_id, expected_pool);
        // Clock-seam anchors come from the REAL shelley-genesis (clock/KES/network
        // only; protocol_version + pparams are NOT here — they come from the
        // recovered ledger's current protocol_params, installed by S2a).
        assert_eq!(mat.anchor_millis, 1_654_041_600_000);
        assert_eq!(mat.start_slot, SlotNo(0));
        assert_eq!(mat.slot_length_ms, 1000);
        // PO-3: kes_anchor_slot is the proven genesis KES-period origin (0), not a
        // fabricated value nor an inherited simple-JSON field.
        assert_eq!(mat.genesis.kes_anchor_slot, 0);
    }

    #[test]
    fn build_operator_forge_material_bad_genesis_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_operator_material(dir.path(), 0);
        std::fs::write(&paths.genesis, b"not json").unwrap();
        // `OperatorForgeMaterial` is deliberately not `Debug` (it holds the
        // custody `ProducerShell`), so match the result rather than `expect_err`.
        let r = build_operator_forge_material(&paths);
        assert!(
            matches!(r, Err(OperatorForgeError::GenesisParse(_))),
            "bad genesis must fail closed"
        );
    }

    #[test]
    fn build_operator_forge_material_pool_id_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let paths = write_operator_material(dir.path(), 0);
        let a = build_operator_forge_material(&paths).expect("build a");
        let b = build_operator_forge_material(&paths).expect("build b");
        assert_eq!(a.pool_id, b.pool_id);
    }
}
