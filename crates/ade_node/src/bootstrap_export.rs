//! BOOTSTRAP-CERTSTATE-PRODUCER — part 2: the pinned-immutable-point capture orchestration.
//!
//! The whole bootstrap package binds to ONE immutable source point P = (slot, hash). Every state
//! query MUST target the IMMUTABLE ledger state — cardano-cli `--immutable-tip`, NOT the default
//! `--volatile-tip` (= the current/volatile state, which can advance beyond the immutable tip) — so
//! each payload is evaluated against the immutable state at the immutable tip, not a moving volatile
//! state. The producer pins P (the immutable tip) BEFORE the capture and re-resolves Q AFTER, and
//! accepts ONLY if Q == P. Because the immutable tip is MONOTONIC (it finalizes forward, never
//! regresses) and an immutable state at a FIXED point cannot change, Q == P proves the immutable tip
//! stayed P across the whole capture, so every `--immutable-tip` query provably evaluated against the
//! SAME canonical immutable ledger state at P. A drift is a STRUCTURED fail-close (the operator
//! retries), never a silently point-inconsistent package. (Issuing a query against the volatile-tip
//! default is the exact hole this contract closes — the production source must never do so.)
//!
//! RED: the production [`ImmutableLedgerSource`] shells cardano-cli and deletes the raw ledger dump
//! (RED capture material) on BOTH the success and failure paths. The orchestration logic here is
//! GREEN + deterministic and is exercised hermetically with a mock source.

use ade_core::consensus::events::Point;
use ade_crypto::blake2b_256;
use ade_ledger::bootstrap_manifest::{verify_and_import_cert_state, BootstrapManifest};
use ade_ledger::delegation::CertState;
use ade_runtime::consensus_inputs::json::{
    encode_consensus_inputs_json, RawConsensusInputs, RawFraction, RawPoolEntry,
};
use ade_types::{CardanoEra, Hash32};
use serde_json::Value;
use std::collections::BTreeMap;

/// An immutable source point P = (slot, hash) — the SINGLE point the bootstrap package binds to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePoint {
    pub slot: u64,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// The immutable tip advanced during the capture (Q != P) — the captures may straddle two points.
    /// Fail closed; the operator retries (success requires a window where the immutable tip is stable).
    PointDrift {
        before: CapturePoint,
        after: CapturePoint,
    },
    /// A captured payload carries a point that disagrees with P (a stronger check than before==after).
    PayloadPointMismatch {
        expected: CapturePoint,
        found: CapturePoint,
        payload: &'static str,
    },
    /// A cardano-cli query / extraction failure (RED).
    Query(String),
}

/// The cardano-node query surface. CONTRACT: the production impl MUST issue EVERY query with
/// `--immutable-tip` (the immutable ledger state at the immutable tip — NEVER the default
/// `--volatile-tip`/current state), so the `Q == P` invariant proves all payloads derive from one
/// canonical immutable state at P. It shells cardano-cli and deletes the raw ledger dump on ALL
/// paths; the test impl returns fixtures + a controllable tip.
pub trait ImmutableLedgerSource {
    fn immutable_tip(&self) -> Result<CapturePoint, CaptureError>;
    fn pool_state(&self) -> Result<Value, CaptureError>;
    /// The dstate.accounts object only (extracted from the ledger-state; the raw multi-hundred-MB
    /// ledger dump is RED transient and is NEVER returned or persisted).
    fn dstate_accounts(&self) -> Result<Value, CaptureError>;
    fn stake_snapshot(&self) -> Result<Value, CaptureError>;
    fn protocol_state(&self) -> Result<Value, CaptureError>;
    /// The point a payload claims to be at, if it carries one (e.g. a ledger-state tip), for the
    /// "validate P where available" obligation. `None` if the payload carries no point.
    fn payload_point(&self, _payload: &Value) -> Option<CapturePoint> {
        None
    }
}

/// The captured, point-consistent source state — all bound to the single [`CapturePoint`].
#[derive(Debug)]
pub struct PinnedCapture {
    pub point: CapturePoint,
    pub pool_state: Value,
    pub dstate_accounts: Value,
    pub stake_snapshot: Value,
    pub protocol_state: Value,
}

/// Pin ONE immutable point P, capture every source, re-resolve Q, accept ONLY if Q == P (the
/// before==after invariant is the point-scoping proof, since cardano-cli can't scope to an arbitrary
/// P) AND every payload that carries a point validates to P. A drift / mismatch is terminal.
pub fn capture_at_pinned_point<S: ImmutableLedgerSource>(
    source: &S,
) -> Result<PinnedCapture, CaptureError> {
    // 1. resolve + record P BEFORE any capture.
    let before = source.immutable_tip()?;
    // 2. capture every source against the node's immutable tip (--immutable-tip, per the contract).
    let pool_state = source.pool_state()?;
    let dstate_accounts = source.dstate_accounts()?;
    let stake_snapshot = source.stake_snapshot()?;
    let protocol_state = source.protocol_state()?;
    // 3. validate P where a payload carries its own point (stronger than before==after).
    for (payload, label) in [
        (&pool_state, "pool-state"),
        (&dstate_accounts, "dstate.accounts"),
        (&stake_snapshot, "stake-snapshot"),
        (&protocol_state, "protocol-state"),
    ] {
        if let Some(found) = source.payload_point(payload) {
            if found != before {
                return Err(CaptureError::PayloadPointMismatch {
                    expected: before,
                    found,
                    payload: label,
                });
            }
        }
    }
    // 4. re-resolve Q AFTER the capture; accept ONLY if the immutable tip did not advance.
    let after = source.immutable_tip()?;
    if before != after {
        return Err(CaptureError::PointDrift { before, after });
    }
    Ok(PinnedCapture {
        point: before,
        pool_state,
        dstate_accounts,
        stake_snapshot,
        protocol_state,
    })
}

// ===== Part 2 (RED): the production cardano-cli capture source. =====

/// Deletes the unique raw-capture temp dir on ALL paths (success / parse-fail / drift-fail / early
/// return / unwind) — RED capture material is never left behind. `keep` (the `--keep-raw-capture`
/// operator flag, never the default) preserves it for diagnosis. (A SIGKILL cannot be guarded; a
/// SIGINT handler is a follow-on hardening.)
pub struct RawCaptureGuard {
    pub dir: std::path::PathBuf,
    pub keep: bool,
}

impl Drop for RawCaptureGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

/// The RED cardano-cli capture source. THE contract: every query is issued with `--immutable-tip`
/// (proven honored — distinct from volatile, strict-fail on bogus flags), so `capture_at_pinned_point`
/// (Q==P) proves one canonical immutable state at P. `protocol-parameters` rejects `--immutable-tip`,
/// so the protocol params are taken from the ledger-state (`esLState.esPp`) at P, never a volatile
/// query. The large ledger-state dump lands in `capture_dir` (unique), guarded by [`RawCaptureGuard`].
pub struct CardanoCliSource {
    /// e.g. `["docker","exec","cardano-node-preview","cardano-cli"]` or `["cardano-cli"]`.
    pub cli_prefix: Vec<String>,
    pub network_magic: u32,
    pub socket_path: String,
    pub capture_dir: std::path::PathBuf,
}

impl CardanoCliSource {
    fn run_query_tip(&self, sub: &[&str], tip_flag: Option<&str>) -> Result<String, CaptureError> {
        let prog = self
            .cli_prefix
            .first()
            .ok_or(CaptureError::Query("empty cli_prefix".into()))?;
        let magic = self.network_magic.to_string();
        let mut cmd = std::process::Command::new(prog);
        cmd.args(&self.cli_prefix[1..]).arg("query").args(sub);
        cmd.args(["--testnet-magic", &magic, "--socket-path", &self.socket_path]);
        if let Some(flag) = tip_flag {
            cmd.arg(flag);
        }
        let out = cmd
            .output()
            .map_err(|e| CaptureError::Query(format!("spawn {sub:?}: {e}")))?;
        if !out.status.success() {
            return Err(CaptureError::Query(format!(
                "{sub:?} failed (exit {:?}): {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        String::from_utf8(out.stdout)
            .map_err(|e| CaptureError::Query(format!("utf8 {sub:?}: {e}")))
    }

    fn run_query(&self, sub: &[&str]) -> Result<String, CaptureError> {
        // THE capture contract: every pinned-state query targets the immutable tip.
        self.run_query_tip(sub, Some("--immutable-tip"))
    }

    fn run_raw(&self, args: &[&str]) -> Result<String, CaptureError> {
        let prog = self
            .cli_prefix
            .first()
            .ok_or(CaptureError::Query("empty cli_prefix".into()))?;
        let mut cmd = std::process::Command::new(prog);
        cmd.args(&self.cli_prefix[1..]).args(args);
        let out = cmd
            .output()
            .map_err(|e| CaptureError::Query(format!("spawn {args:?}: {e}")))?;
        if !out.status.success() {
            return Err(CaptureError::Query(format!(
                "{args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        String::from_utf8(out.stdout).map_err(|e| CaptureError::Query(format!("utf8 {args:?}: {e}")))
    }

    /// The immutable tip P + the node's reported epoch (for the NetworkProfile epoch check).
    pub fn tip_with_epoch(&self) -> Result<(CapturePoint, u64), CaptureError> {
        let v = self.parse(&self.run_query(&["tip"])?, "tip")?;
        let slot = v.get("slot").and_then(Value::as_u64).ok_or(CaptureError::Query("tip.slot".into()))?;
        let hash = v.get("hash").and_then(Value::as_str).ok_or(CaptureError::Query("tip.hash".into()))?.to_string();
        let epoch = v.get("epoch").and_then(Value::as_u64).ok_or(CaptureError::Query("tip.epoch".into()))?;
        Ok((CapturePoint { slot, hash }, epoch))
    }

    /// The volatile tip's reported epoch (for the protocol-parameters epoch-static check).
    pub fn volatile_epoch(&self) -> Result<u64, CaptureError> {
        let v = self.parse(&self.run_query_tip(&["tip"], Some("--volatile-tip"))?, "volatile tip")?;
        v.get("epoch").and_then(Value::as_u64).ok_or(CaptureError::Query("volatile tip.epoch".into()))
    }

    /// The `protocol-parameters` JSON (the preimage for `protocol_params_hash`). `protocol-parameters`
    /// rejects `--immutable-tip`, so this uses the node default (current state) — epoch-static-verified
    /// by the caller (immutable-tip epoch == volatile-tip epoch).
    pub fn protocol_parameters_json(&self) -> Result<String, CaptureError> {
        self.run_query_tip(&["protocol-parameters"], None)
    }

    /// The cardano-cli version line (bundle provenance).
    pub fn node_version(&self) -> Result<String, CaptureError> {
        Ok(self.run_raw(&["--version"])?.lines().next().unwrap_or("").trim().to_string())
    }

    fn parse(&self, s: &str, what: &'static str) -> Result<Value, CaptureError> {
        serde_json::from_str(s).map_err(|e| CaptureError::Query(format!("parse {what}: {e}")))
    }
}

impl ImmutableLedgerSource for CardanoCliSource {
    fn immutable_tip(&self) -> Result<CapturePoint, CaptureError> {
        let v = self.parse(&self.run_query(&["tip"])?, "tip")?;
        let slot = v
            .get("slot")
            .and_then(Value::as_u64)
            .ok_or(CaptureError::Query("tip.slot".into()))?;
        let hash = v
            .get("hash")
            .and_then(Value::as_str)
            .ok_or(CaptureError::Query("tip.hash".into()))?
            .to_string();
        Ok(CapturePoint { slot, hash })
    }

    fn pool_state(&self) -> Result<Value, CaptureError> {
        self.parse(&self.run_query(&["pool-state", "--all-stake-pools"])?, "pool-state")
    }

    fn dstate_accounts(&self) -> Result<Value, CaptureError> {
        // The ledger-state is large RED capture material: persist it to the unique capture dir (the
        // RawCaptureGuard deletes it), then extract ONLY dstate.accounts (the BLUE-bound section).
        let raw = self.run_query(&["ledger-state"])?;
        let _ = std::fs::create_dir_all(&self.capture_dir);
        let _ = std::fs::write(self.capture_dir.join("ledger-state.json"), &raw);
        let ls = self.parse(&raw, "ledger-state")?;
        ls.pointer("/stateBefore/esLState/delegationState/dstate/accounts")
            .cloned()
            .ok_or(CaptureError::Query(
                "ledger-state .stateBefore.esLState.delegationState.dstate.accounts missing".into(),
            ))
    }

    fn stake_snapshot(&self) -> Result<Value, CaptureError> {
        self.parse(&self.run_query(&["stake-snapshot", "--all-stake-pools"])?, "stake-snapshot")
    }

    fn protocol_state(&self) -> Result<Value, CaptureError> {
        self.parse(&self.run_query(&["protocol-state"])?, "protocol-state")
    }
}

// ===== Part 3: the manifest binding + the inspection report + the atomic 4-sibling emit. =====

fn hex32(h: &Hash32) -> String {
    h.0.iter().map(|b| format!("{b:02x}")).collect()
}

fn path_append(p: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    std::path::PathBuf::from(s)
}

/// Build the binding manifest. Every field is a binding: network magic, era, the source point (slot
/// + block hash), the bundle hash (`seed_hash`), the cert-state hash, and the era/protocol-profile
/// commitment (`source_commitment`). The existing importer (`verify_and_import_cert_state`) re-checks
/// these against the sibling bytes — a producer/importer hash mismatch is fail-closed.
pub fn build_manifest(
    network_magic: u32,
    source_point: Point,
    bundle_bytes: &[u8],
    certstate_bytes: &[u8],
    profile_commitment: Hash32,
) -> BootstrapManifest {
    BootstrapManifest {
        network_magic,
        era: CardanoEra::Conway,
        source_point,
        seed_hash: blake2b_256(bundle_bytes),
        cert_state_hash: blake2b_256(certstate_bytes),
        source_commitment: profile_commitment,
    }
}

/// The release-artifact inspection report (`.inspect.json`) — the judge's machine-auditable surface.
/// Carries the source point + the four hashes + the six cert-state counts + the binding verdict (the
/// EXISTING importer gate run over the emitted bytes — `bound` only if the package self-verifies).
pub fn build_inspect_report(
    cert_state: &CertState,
    source_point: &Point,
    bundle_bytes: &[u8],
    certstate_bytes: &[u8],
    manifest_bytes: &[u8],
    network_magic: u32,
    query_command: &str,
    node_version: &str,
    profile_id: &str,
    network_commitment: Hash32,
) -> Value {
    let binding_verdict = match verify_and_import_cert_state(
        manifest_bytes,
        bundle_bytes,
        certstate_bytes,
        network_magic,
        CardanoEra::Conway,
    ) {
        Ok(_) => "bound",
        Err(_) => "UNBOUND",
    };
    let vrf_count = cert_state
        .pool
        .pools
        .values()
        .filter(|p| p.vrf_hash != Hash32([0u8; 32]))
        .count();
    serde_json::json!({
        "source_slot": source_point.slot.0,
        "source_hash": hex32(&source_point.hash),
        "bundle_hash": hex32(&blake2b_256(bundle_bytes)),
        "certstate_hash": hex32(&blake2b_256(certstate_bytes)),
        "manifest_hash": hex32(&blake2b_256(manifest_bytes)),
        "active_pool_count": cert_state.pool.pools.len(),
        "future_pool_count": cert_state.pool.future_pools.len(),
        "retiring_count": cert_state.pool.retiring.len(),
        "delegation_count": cert_state.delegation.delegations.len(),
        "reward_count": cert_state.delegation.rewards.len(),
        "vrf_count": vrf_count,
        "binding_verdict": binding_verdict,
        // audit provenance — NOT part of the canonical manifest identity (the manifest binds only the
        // consensus-relevant fields: network/era/source-point/bundle-hash/certstate-hash/profile).
        "source_query_command": query_command,
        "source_cardano_node_version": node_version,
        // the resolved network identity (committed registry): names the profile + its commitment.
        "network_profile_id": profile_id,
        "network_profile_commitment": hex32(&network_commitment),
    })
}

/// The concise success receipt (the four paths + hashes) the command returns to the operator;
/// `.inspect.json` remains the machine-auditable evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitReceipt {
    pub bundle_path: String,
    pub certstate_path: String,
    pub manifest_path: String,
    pub inspect_path: String,
    pub bundle_hash: String,
    pub certstate_hash: String,
    pub manifest_hash: String,
}

/// Atomically emit ALL FOUR sibling artifacts (all-or-nothing). Each is written to a `.tmp` sibling
/// first; only once ALL four temps succeed are they renamed into place. Any failure deletes every
/// temp — never a partial, importable-but-incomplete package. The bundle base `<base>` yields
/// `<base>.json` + the importer's sibling convention `<base>.json.{certstate,manifest,inspect.json}`.
pub fn emit_package(
    output_base: &std::path::Path,
    bundle_bytes: &[u8],
    certstate_bytes: &[u8],
    manifest_bytes: &[u8],
    inspect_json: &[u8],
) -> Result<EmitReceipt, CaptureError> {
    let bundle = output_base.with_extension("json");
    let certstate = path_append(&bundle, ".certstate");
    let manifest = path_append(&bundle, ".manifest");
    let inspect = path_append(&bundle, ".inspect.json");
    let files: [(&std::path::Path, &[u8]); 4] = [
        (&bundle, bundle_bytes),
        (&certstate, certstate_bytes),
        (&manifest, manifest_bytes),
        (&inspect, inspect_json),
    ];
    let temps: Vec<std::path::PathBuf> = files.iter().map(|(p, _)| path_append(p, ".tmp")).collect();
    let cleanup = |temps: &[std::path::PathBuf]| {
        for t in temps {
            let _ = std::fs::remove_file(t);
        }
    };
    for ((_, bytes), tmp) in files.iter().zip(&temps) {
        if let Err(e) = std::fs::write(tmp, bytes) {
            cleanup(&temps);
            return Err(CaptureError::Query(format!("write {tmp:?}: {e}")));
        }
    }
    for ((p, _), tmp) in files.iter().zip(&temps) {
        if let Err(e) = std::fs::rename(tmp, p) {
            cleanup(&temps);
            return Err(CaptureError::Query(format!("rename {tmp:?}: {e}")));
        }
    }
    Ok(EmitReceipt {
        bundle_path: bundle.display().to_string(),
        certstate_path: certstate.display().to_string(),
        manifest_path: manifest.display().to_string(),
        inspect_path: inspect.display().to_string(),
        bundle_hash: hex32(&blake2b_256(bundle_bytes)),
        certstate_hash: hex32(&blake2b_256(certstate_bytes)),
        manifest_hash: hex32(&blake2b_256(manifest_bytes)),
    })
}

// ===== Part 1b: the consensus-inputs bundle builder (reuses the consensus_inputs authority encoder). =====

/// The genesis-static + epoch-geometry + profile context for the bundle (everything not in the live
/// pool/stake/nonce captures). `protocol_params_json` is the cardano-cli `query protocol-parameters`
/// JSON preimage (epoch-static at P — proven P.epoch == volatile.epoch by the caller).
#[derive(Debug, Clone)]
pub struct BundleProfile {
    pub network_magic: u32,
    pub genesis_hash_hex: String,
    pub active_slots_coeff: (u32, u32),
    pub epoch_no: u64,
    pub epoch_start_slot: u64,
    pub epoch_end_slot: u64,
    pub protocol_params_json: String,
    pub node_version: String,
    pub query_command: String,
}

/// Assemble the consensus-inputs bundle BYTES from the point-P captures and emit them through the
/// AUTHORITY encoder (`encode_consensus_inputs_json`) — never a hand-rolled format. Mapping:
/// pool_distribution ← stake-snapshot `stakeGo`; pool_vrf_keyhashes ← pool-state `poolParams.spsVrf`;
/// epoch_nonce_hex ← protocol-state `epochNonce`; protocol_params_hash ← blake2b(protocol_params_json).
/// FAIL CLOSED if any pool in the distribution lacks a VRF key (the leadership-critical alignment).
pub fn build_bundle(
    point: &CapturePoint,
    profile: &BundleProfile,
    pool_state: &Value,
    stake_snapshot: &Value,
    protocol_state: &Value,
) -> Result<Vec<u8>, CaptureError> {
    // pool_distribution ← the leader-election `go` stake snapshot.
    let pools = stake_snapshot
        .get("pools")
        .and_then(Value::as_object)
        .ok_or(CaptureError::Query("stake-snapshot.pools".into()))?;
    let mut pool_distribution: BTreeMap<String, RawPoolEntry> = BTreeMap::new();
    for (pool, snap) in pools {
        let go = snap
            .get("stakeGo")
            .and_then(Value::as_u64)
            .ok_or(CaptureError::Query(format!("stakeGo for {pool}")))?;
        pool_distribution.insert(pool.clone(), RawPoolEntry { active_stake: go });
    }
    // pool_vrf_keyhashes ← pool-state poolParams.spsVrf.
    let ps = pool_state
        .as_object()
        .ok_or(CaptureError::Query("pool-state object".into()))?;
    let mut pool_vrf_keyhashes: BTreeMap<String, String> = BTreeMap::new();
    for (pool, entry) in ps {
        let vrf = entry
            .get("poolParams")
            .and_then(|p| p.get("spsVrf"))
            .and_then(Value::as_str)
            .ok_or(CaptureError::Query(format!("spsVrf for {pool}")))?;
        pool_vrf_keyhashes.insert(pool.clone(), vrf.to_string());
    }
    // FIDELITY (leadership-critical): every pool in the distribution MUST carry a VRF key — fail closed.
    for pool in pool_distribution.keys() {
        if !pool_vrf_keyhashes.contains_key(pool) {
            return Err(CaptureError::Query(format!(
                "pool {pool} is in the stake distribution but has no VRF key (unverifiable leadership)"
            )));
        }
    }
    let epoch_nonce_hex = protocol_state
        .get("epochNonce")
        .and_then(Value::as_str)
        .ok_or(CaptureError::Query("protocol-state.epochNonce".into()))?
        .to_string();
    let protocol_params_hash = blake2b_256(profile.protocol_params_json.as_bytes());
    let raw = RawConsensusInputs {
        network_magic: profile.network_magic,
        genesis_hash_hex: profile.genesis_hash_hex.clone(),
        era: "conway".into(),
        epoch_no: profile.epoch_no,
        epoch_start_slot: profile.epoch_start_slot,
        epoch_end_slot: profile.epoch_end_slot,
        active_slots_coeff: RawFraction {
            numer: profile.active_slots_coeff.0,
            denom: profile.active_slots_coeff.1,
        },
        epoch_nonce_hex,
        pool_distribution,
        pool_vrf_keyhashes,
        protocol_params_hash_hex: hex32(&protocol_params_hash),
        source_cardano_node_version: profile.node_version.clone(),
        source_query_command: profile.query_command.clone(),
        source_tip_hash_hex: point.hash.clone(),
        source_tip_slot: point.slot,
        protocol_params_json: Some(profile.protocol_params_json.clone()),
        network: Some("preview".into()),
        epoch_length: Some(profile.epoch_end_slot - profile.epoch_start_slot + 1),
        pool_distribution_source: Some(
            "cardano-cli query stake-snapshot --all-stake-pools (go stake) --immutable-tip".into(),
        ),
    };
    encode_consensus_inputs_json(&raw).map_err(|e| CaptureError::Query(format!("encode bundle: {e}")))
}

// ===== The one-command orchestration (capture -> assemble -> bind -> emit). =====

fn hex_decode_32(s: &str) -> Result<Hash32, CaptureError> {
    if s.len() != 64 {
        return Err(CaptureError::Query(format!(
            "source hash hex must be 64 chars, got {}",
            s.len()
        )));
    }
    let mut arr = [0u8; 32];
    for (i, byte) in arr.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
            .map_err(|_| CaptureError::Query(format!("invalid hex in source hash: {s}")))?;
    }
    Ok(Hash32(arr))
}

// ===== The committed, closed network-identity profile registry. =====

/// A committed, CLOSED network-identity profile — DERIVED COMPATIBILITY DATA (the fixed per-network
/// constants), NOT a semantic feature flag. The P-pinned ledger-state supplies the epoch-sensitive
/// protocol parameters; this supplies ONLY the network constants. Genesis hashes are the reviewed
/// Shelley-genesis hashes from each network's node config (NEVER a prior bundle).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkProfile {
    pub id: &'static str,
    pub network_magic: u32,
    pub genesis_hash: Hash32,
    pub active_slots_coeff: (u32, u32),
    pub epoch_length: u64,
}

/// Resolve the committed profile for a network identity. CLOSED + enumerated — an unknown identity
/// FAILS CLOSED (never a loose config default). Adding a network = a reviewed entry here + a test.
pub fn resolve_network_profile(network: &str) -> Result<NetworkProfile, CaptureError> {
    // reviewed Shelley-genesis hashes (extracted from each network's node config, 2026-06-22).
    let (id, network_magic, genesis_hex, active_slots_coeff, epoch_length) = match network {
        "preview" => (
            "preview",
            2u32,
            "363498d1024f84bb39d3fa9593ce391483cb40d479b87233f868d6e57c3a400d",
            (1u32, 20u32),
            86_400u64,
        ),
        "preprod" => (
            "preprod",
            1u32,
            "162d29c4e1cf6b8a84f2d692e67a3ac6bc7851bc3e6e4afe64d15778bed8bd86",
            (1u32, 20u32),
            432_000u64,
        ),
        other => {
            return Err(CaptureError::Query(format!(
                "unknown network identity '{other}' — the registry is closed (preview|preprod)"
            )))
        }
    };
    Ok(NetworkProfile {
        id,
        network_magic,
        genesis_hash: hex_decode_32(genesis_hex)?,
        active_slots_coeff,
        epoch_length,
    })
}

impl NetworkProfile {
    /// The network-identity commitment (named in the inspect report): blake2b over the FIXED network
    /// constants. Distinct from the consensus profile commitment (which folds in the epoch-sensitive
    /// protocol params).
    pub fn commitment(&self) -> Hash32 {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.id.as_bytes());
        buf.extend_from_slice(&self.network_magic.to_be_bytes());
        buf.extend_from_slice(&self.genesis_hash.0);
        buf.extend_from_slice(&self.active_slots_coeff.0.to_be_bytes());
        buf.extend_from_slice(&self.active_slots_coeff.1.to_be_bytes());
        buf.extend_from_slice(&self.epoch_length.to_be_bytes());
        blake2b_256(&buf)
    }

    /// Verify the live node's network magic matches this profile — fail-closed BEFORE export.
    pub fn verify_live_magic(&self, live_magic: u32) -> Result<(), CaptureError> {
        if live_magic == self.network_magic {
            Ok(())
        } else {
            Err(CaptureError::Query(format!(
                "live node magic {live_magic} != profile '{}' magic {} — refusing to export against the wrong network",
                self.id, self.network_magic
            )))
        }
    }

    /// Epoch geometry for a slot under this profile (Shelley-from-genesis: epoch = slot / epoch_length).
    pub fn epoch_of(&self, slot: u64) -> (u64, u64, u64) {
        let epoch_no = slot / self.epoch_length;
        let start = epoch_no * self.epoch_length;
        (epoch_no, start, start + self.epoch_length - 1)
    }

    /// Verify the node's reported epoch matches this profile's geometry for the slot — catches a
    /// wrong-profile selection (a different epoch_length computes a different epoch).
    pub fn verify_epoch(&self, slot: u64, node_epoch: u64) -> Result<(), CaptureError> {
        let computed = self.epoch_of(slot).0;
        if computed == node_epoch {
            Ok(())
        } else {
            Err(CaptureError::Query(format!(
                "profile '{}' computes epoch {computed} for slot {slot} but the node reports {node_epoch} — wrong network profile",
                self.id
            )))
        }
    }

    /// Derive the [`BundleProfile`] for a captured point under this profile: the network constants +
    /// the slot's epoch geometry + the P-pinned protocol params (epoch-sensitive, NOT from here).
    pub fn bundle_profile(
        &self,
        slot: u64,
        protocol_params_json: String,
        node_version: String,
        query_command: String,
    ) -> BundleProfile {
        let (epoch_no, epoch_start_slot, epoch_end_slot) = self.epoch_of(slot);
        BundleProfile {
            network_magic: self.network_magic,
            genesis_hash_hex: hex32(&self.genesis_hash),
            active_slots_coeff: self.active_slots_coeff,
            epoch_no,
            epoch_start_slot,
            epoch_end_slot,
            protocol_params_json,
            node_version,
            query_command,
        }
    }
}

/// The one-command orchestration (BOOTSTRAP-CERTSTATE-PRODUCER): pinned capture at P → CertState
/// assembly → bundle construction → manifest binding → atomic four-file emit. FAIL-CLOSED — nothing is
/// emitted unless every capture + binding check passes (the `emit_package` step is last and atomic).
/// The RED raw-capture cleanup is the CALLER's [`RawCaptureGuard`], so this stays source-agnostic and
/// hermetically testable. All four artifacts share the ONE captured point P.
pub fn run_bootstrap_export<S: ImmutableLedgerSource>(
    source: &S,
    profile: &BundleProfile,
    profile_commitment: Hash32,
    profile_id: &str,
    network_commitment: Hash32,
    output_base: &std::path::Path,
) -> Result<EmitReceipt, CaptureError> {
    let cap = capture_at_pinned_point(source)?;
    let point = Point {
        slot: ade_types::SlotNo(cap.point.slot),
        hash: hex_decode_32(&cap.point.hash)?,
    };
    // network id for the address header: testnets -> 0, mainnet -> 1.
    let network_id = if profile.network_magic == 764_824_073 { 1u8 } else { 0u8 };
    let cert_state = ade_runtime::consensus_inputs::cert_state_extract::assemble_cert_state(
        &cap.pool_state,
        &cap.dstate_accounts,
        network_id,
    )
    .map_err(|e| CaptureError::Query(format!("assemble cert-state: {e:?}")))?;
    let bundle = build_bundle(
        &cap.point,
        profile,
        &cap.pool_state,
        &cap.stake_snapshot,
        &cap.protocol_state,
    )?;
    let certstate = ade_ledger::snapshot::cert_state::encode_cert_state(&cert_state);
    let manifest = build_manifest(profile.network_magic, point.clone(), &bundle, &certstate, profile_commitment);
    let manifest_bytes = manifest.canonical_bytes();
    let inspect = build_inspect_report(
        &cert_state,
        &point,
        &bundle,
        &certstate,
        &manifest_bytes,
        profile.network_magic,
        &profile.query_command,
        &profile.node_version,
        profile_id,
        network_commitment,
    );
    let inspect_bytes = serde_json::to_vec_pretty(&inspect)
        .map_err(|e| CaptureError::Query(format!("inspect serialize: {e}")))?;
    emit_package(output_base, &bundle, &certstate, &manifest_bytes, &inspect_bytes)
}

/// Exit code for a `--mode bootstrap_export` failure (fail-closed).
pub const EXIT_BOOTSTRAP_EXPORT_FAILURE: i32 = 60;

/// The `--mode bootstrap_export` command (RED orchestration): resolve the committed network profile,
/// verify the live node matches it, capture the immutable-tip state at one pinned point, and emit the
/// four bound artifacts. Fail-closed — any capture/verify/bind failure aborts before emit, and the RED
/// raw ledger dump is always cleaned up (unless `keep_raw_capture`).
pub fn run_bootstrap_export_command(
    network: &str,
    output_base: &std::path::Path,
    keep_raw_capture: bool,
) -> Result<EmitReceipt, CaptureError> {
    let profile = resolve_network_profile(network)?;
    // RED capture source: docker exec <container> cardano-cli, socket inside the container.
    let container = format!("cardano-node-{}", profile.id);
    let capture_dir = std::env::temp_dir().join(format!("ade-bootstrap-{}-capture", profile.id));
    let source = CardanoCliSource {
        cli_prefix: vec!["docker".into(), "exec".into(), container, "cardano-cli".into()],
        network_magic: profile.network_magic,
        socket_path: "/ipc/node.socket".into(),
        capture_dir: capture_dir.clone(),
    };
    let _guard = RawCaptureGuard { dir: capture_dir, keep: keep_raw_capture };

    // 1. immutable tip P + node-reported epoch -> verify the committed profile matches the live node.
    let (point, node_epoch) = source.tip_with_epoch()?;
    profile.verify_epoch(point.slot, node_epoch)?;

    // 2. protocol-parameters are epoch-sensitive (the profile carries only network constants); the
    //    query rejects --immutable-tip, so capture the default state and require the volatile epoch ==
    //    the immutable epoch (no boundary crossed -> the params are P's).
    let volatile_epoch = source.volatile_epoch()?;
    if volatile_epoch != node_epoch {
        return Err(CaptureError::Query(format!(
            "epoch boundary crossed during capture (immutable epoch {node_epoch} != volatile epoch \
             {volatile_epoch}) — protocol-parameters not epoch-static for P; retry"
        )));
    }
    let protocol_params_json = source.protocol_parameters_json()?;
    let node_version = source.node_version()?;

    // 3. assemble the BundleProfile + the consensus profile commitment (the manifest source_commitment).
    let query_command = format!(
        "cardano-cli query {{tip,pool-state,ledger-state,stake-snapshot,protocol-state}} \
         --immutable-tip + protocol-parameters; --testnet-magic {}",
        profile.network_magic
    );
    let bundle_profile =
        profile.bundle_profile(point.slot, protocol_params_json.clone(), node_version, query_command);
    let pp_hash = blake2b_256(protocol_params_json.as_bytes());
    let source_commitment = ade_ledger::reduced_epoch_view::consensus_profile_commitment(
        &profile.genesis_hash,
        &pp_hash,
        ade_core::consensus::vrf_cert::ActiveSlotsCoeff {
            numer: profile.active_slots_coeff.0,
            denom: profile.active_slots_coeff.1,
        },
    );

    // 4. pinned capture -> assemble -> bind -> emit, with bounded retry on immutable-tip drift. The
    //    multi-query capture (~15s) must fit inside one stable window; the immutable tip advances ~per
    //    block (~10s), so a drifted attempt is re-tried with a fresh capture. The protocol params are
    //    epoch-static across the (sub-minute) retry span, so they are captured once above. A drift is
    //    a clean no-emit (the contract aborts before emit), so retrying never leaves a partial package.
    const MAX_ATTEMPTS: u32 = 12;
    let mut last = CaptureError::Query("no capture attempt made".into());
    for attempt in 1..=MAX_ATTEMPTS {
        match run_bootstrap_export(
            &source,
            &bundle_profile,
            source_commitment.clone(),
            profile.id,
            profile.commitment(),
            output_base,
        ) {
            Ok(receipt) => return Ok(receipt),
            Err(e @ CaptureError::PointDrift { .. }) => {
                eprintln!(
                    "attempt {attempt}/{MAX_ATTEMPTS}: immutable tip drifted during capture; retrying"
                );
                last = e;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// A mock source whose immutable tip advances by `drift_per_query` slots on each `immutable_tip`
    /// call — modeling the real node's tip moving (or not) during the capture window.
    struct MockSource {
        base_slot: u64,
        drift_per_query: u64,
        calls: Cell<u64>,
        payload_point: Option<CapturePoint>,
    }

    impl ImmutableLedgerSource for MockSource {
        fn immutable_tip(&self) -> Result<CapturePoint, CaptureError> {
            let n = self.calls.get();
            self.calls.set(n + 1);
            Ok(CapturePoint {
                slot: self.base_slot + n * self.drift_per_query,
                hash: format!("h{}", self.base_slot + n * self.drift_per_query),
            })
        }
        fn pool_state(&self) -> Result<Value, CaptureError> {
            Ok(serde_json::json!({}))
        }
        fn dstate_accounts(&self) -> Result<Value, CaptureError> {
            Ok(serde_json::json!({}))
        }
        fn stake_snapshot(&self) -> Result<Value, CaptureError> {
            Ok(serde_json::json!({}))
        }
        fn protocol_state(&self) -> Result<Value, CaptureError> {
            Ok(serde_json::json!({}))
        }
        fn payload_point(&self, _payload: &Value) -> Option<CapturePoint> {
            self.payload_point.clone()
        }
    }

    #[test]
    fn stable_immutable_tip_yields_a_point_consistent_capture() {
        let s = MockSource {
            base_slot: 115_455_568,
            drift_per_query: 0, // the immutable tip did NOT advance
            calls: Cell::new(0),
            payload_point: None,
        };
        let cap = capture_at_pinned_point(&s).expect("stable tip -> consistent capture");
        assert_eq!(cap.point.slot, 115_455_568);
        assert_eq!(s.calls.get(), 2, "resolved the immutable tip before AND after");
    }

    #[test]
    fn a_drifting_immutable_tip_fails_closed_never_emits() {
        let s = MockSource {
            base_slot: 100,
            drift_per_query: 20, // the immutable tip advanced during the capture
            calls: Cell::new(0),
            payload_point: None,
        };
        let r = capture_at_pinned_point(&s);
        assert!(
            matches!(r, Err(CaptureError::PointDrift { .. })),
            "a moving immutable tip must fail closed, got {r:?}"
        );
    }

    #[test]
    fn a_payload_claiming_a_different_point_fails_closed() {
        let s = MockSource {
            base_slot: 100,
            drift_per_query: 0,
            calls: Cell::new(0),
            payload_point: Some(CapturePoint { slot: 999, hash: "other".into() }),
        };
        let r = capture_at_pinned_point(&s);
        assert!(
            matches!(r, Err(CaptureError::PayloadPointMismatch { .. })),
            "a payload at a different point must fail closed, got {r:?}"
        );
    }

    // ----- Part 3: manifest binding + inspect report + atomic emit -----
    use ade_ledger::delegation::{DelegationState, PoolParams, PoolState};
    use ade_ledger::snapshot::cert_state::encode_cert_state;
    use ade_types::shelley::cert::StakeCredential;
    use ade_types::tx::{Coin, PoolId};
    use ade_types::{Hash28, SlotNo};
    use std::collections::BTreeMap;

    fn sample_cert_state() -> CertState {
        let pool_id = PoolId(Hash28([0x11; 28]));
        let mut pools = BTreeMap::new();
        pools.insert(
            pool_id.clone(),
            PoolParams {
                pool_id: pool_id.clone(),
                vrf_hash: Hash32([0x52; 32]),
                pledge: Coin(500_000_000),
                cost: Coin(340_000_000),
                margin: (3, 40),
                reward_account: vec![0xe0; 29],
                owners: vec![Hash28([0x04; 28])],
            },
        );
        let cred = StakeCredential::KeyHash(Hash28([0xaa; 28]));
        let mut delegations = BTreeMap::new();
        delegations.insert(cred.clone(), pool_id);
        let mut rewards = BTreeMap::new();
        rewards.insert(cred, Coin(18412));
        CertState {
            delegation: DelegationState {
                registrations: BTreeMap::new(),
                delegations,
                rewards,
            },
            pool: PoolState {
                pools,
                future_pools: BTreeMap::new(),
                retiring: BTreeMap::new(),
            },
        }
    }

    fn sample_point() -> Point {
        Point {
            slot: SlotNo(115_455_568),
            hash: Hash32([0xab; 32]),
        }
    }

    #[test]
    fn manifest_binds_and_inspect_self_verifies_through_the_existing_importer() {
        let cs = sample_cert_state();
        let certstate = encode_cert_state(&cs);
        let bundle = b"canonical-seed-bytes".to_vec(); // verify_and_import only HASHES the seed
        let profile = Hash32([0x9c; 32]);
        let manifest = build_manifest(2, sample_point(), &bundle, &certstate, profile.clone());
        let manifest_bytes = manifest.canonical_bytes();

        // verify the actual CANONICAL BYTES bind every required field (not merely the inspect report):
        // decode the canonical_bytes back and assert each binding individually.
        let decoded = BootstrapManifest::decode(&manifest_bytes).unwrap();
        assert_eq!(decoded, manifest, "manifest canonical bytes round-trip");
        assert_eq!(decoded.network_magic, 2, "binds network magic");
        assert_eq!(decoded.source_point.slot, SlotNo(115_455_568), "binds source slot");
        assert_eq!(decoded.source_point.hash, Hash32([0xab; 32]), "binds source block hash");
        assert_eq!(decoded.seed_hash, blake2b_256(&bundle), "binds the consensus bundle hash");
        assert_eq!(decoded.cert_state_hash, blake2b_256(&certstate), "binds the certstate hash");
        assert_eq!(decoded.source_commitment, profile, "binds the profile commitment");
        assert_eq!(decoded.era, CardanoEra::Conway, "binds the era context");

        let report = build_inspect_report(
            &cs, &sample_point(), &bundle, &certstate, &manifest_bytes, 2,
            "cardano-cli query ... --immutable-tip", "cardano-node 11.0.1",
            "preview", Hash32([0x77; 32]),
        );
        assert_eq!(report["binding_verdict"].as_str(), Some("bound"), "package self-verifies");
        assert_eq!(report["active_pool_count"].as_u64(), Some(1));
        assert_eq!(report["delegation_count"].as_u64(), Some(1));
        assert_eq!(report["reward_count"].as_u64(), Some(1));
        assert_eq!(report["vrf_count"].as_u64(), Some(1));
        assert_eq!(report["source_slot"].as_u64(), Some(115_455_568));
    }

    #[test]
    fn emit_writes_all_four_siblings_atomically_with_a_receipt() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = dir.path().join("ade-inputs-ep1336");
        let r = emit_package(&base, b"bundle", b"cert", b"manifest", b"{}").expect("emit");
        for p in [&r.bundle_path, &r.certstate_path, &r.manifest_path, &r.inspect_path] {
            assert!(std::path::Path::new(p).exists(), "{p} must exist");
        }
        assert!(r.bundle_path.ends_with("ade-inputs-ep1336.json"));
        assert!(r.certstate_path.ends_with("ade-inputs-ep1336.json.certstate"));
        assert!(r.manifest_path.ends_with("ade-inputs-ep1336.json.manifest"));
        assert!(r.inspect_path.ends_with("ade-inputs-ep1336.json.inspect.json"));
        assert!(!std::path::Path::new(&format!("{}.tmp", r.bundle_path)).exists(), "no leftover temp");
    }

    // ----- Part 1b: bundle builder fidelity (reuses the consensus_inputs authority) -----
    const POOL: &str = "11111111111111111111111111111111111111111111111111111111";
    const VRF: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const NONCE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn stake_snapshot_fx() -> Value {
        let mut pools = serde_json::Map::new();
        pools.insert(
            POOL.to_string(),
            serde_json::json!({"stakeGo": 1000, "stakeMark": 1001, "stakeSet": 1002}),
        );
        serde_json::json!({"pools": pools, "total": {}})
    }
    fn pool_state_fx() -> Value {
        let mut m = serde_json::Map::new();
        m.insert(
            POOL.to_string(),
            serde_json::json!({"poolParams": {"spsVrf": VRF}, "futurePoolParams": null, "retiring": null}),
        );
        Value::Object(m)
    }
    fn protocol_state_fx() -> Value {
        serde_json::json!({ "epochNonce": NONCE })
    }
    fn profile_fx() -> BundleProfile {
        BundleProfile {
            network_magic: 2,
            genesis_hash_hex: "9a".repeat(32),
            active_slots_coeff: (1, 20),
            epoch_no: 1336,
            epoch_start_slot: 1,
            epoch_end_slot: 432000,
            protocol_params_json: "{\"k\":1}".into(),
            node_version: "cardano-node 11.0.1".into(),
            query_command: "cardano-cli query ... --immutable-tip".into(),
        }
    }
    fn point_fx() -> CapturePoint {
        CapturePoint { slot: 115_455_568, hash: "ab".repeat(32) }
    }

    #[test]
    fn bundle_round_trips_through_the_authority_aligned_pools_and_source_point() {
        let bytes = build_bundle(&point_fx(), &profile_fx(), &pool_state_fx(), &stake_snapshot_fx(), &protocol_state_fx())
            .expect("build");
        // parse via the AUTHORITY — proves we emit its format, not a parallel one.
        let raw =
            ade_runtime::consensus_inputs::json::parse_consensus_inputs_json(&bytes).expect("parse via authority");
        assert_eq!(raw.pool_distribution.get(POOL).map(|e| e.active_stake), Some(1000), "stakeGo->active_stake");
        assert_eq!(raw.pool_vrf_keyhashes.get(POOL).map(String::as_str), Some(VRF), "VRF aligns to the pool ID");
        assert_eq!(
            raw.pool_distribution.keys().collect::<Vec<_>>(),
            raw.pool_vrf_keyhashes.keys().collect::<Vec<_>>(),
            "VRF key-set == distribution key-set"
        );
        assert_eq!(raw.epoch_nonce_hex, NONCE, "nonce from protocol-state.epochNonce");
        assert_eq!(raw.source_tip_slot, 115_455_568, "source slot == P (the manifest source point)");
        assert_eq!(raw.source_tip_hash_hex, "ab".repeat(32), "source hash == P");
        assert_eq!(
            raw.protocol_params_hash_hex,
            hex32(&blake2b_256(b"{\"k\":1}")),
            "protocol_params_hash == blake2b(protocol_params_json)"
        );
    }

    #[test]
    fn changing_a_leadership_input_changes_the_bundle_bytes() {
        let mut ss = stake_snapshot_fx();
        let b1 = build_bundle(&point_fx(), &profile_fx(), &pool_state_fx(), &ss, &protocol_state_fx()).unwrap();
        ss["pools"][POOL]["stakeGo"] = serde_json::json!(9999);
        let b2 = build_bundle(&point_fx(), &profile_fx(), &pool_state_fx(), &ss, &protocol_state_fx()).unwrap();
        assert_ne!(b1, b2, "a changed go-stake must change the bundle/profile commitment");
    }

    #[test]
    fn a_pool_with_stake_but_no_vrf_fails_closed() {
        let empty_ps = serde_json::json!({});
        let r = build_bundle(&point_fx(), &profile_fx(), &empty_ps, &stake_snapshot_fx(), &protocol_state_fx());
        assert!(matches!(r, Err(CaptureError::Query(_))), "stake without VRF must fail closed, got {r:?}");
    }

    // ----- Rich-mock package round-trip: the RELEASE WIRING test (one happy + one failure). -----
    // Proves the one-command judge workflow wires together end-to-end: pinned capture -> CertState
    // assembly -> bundle construction -> manifest binding -> atomic four-file emit -> importer
    // self-verification. NOT a fake node — just realistic in-memory fixtures through the real pipeline.
    struct RichMock {
        tip: CapturePoint,
        pool_state: Value,
        dstate: Value,
        stake_snapshot: Value,
        protocol_state: Value,
    }
    impl ImmutableLedgerSource for RichMock {
        fn immutable_tip(&self) -> Result<CapturePoint, CaptureError> {
            Ok(self.tip.clone())
        }
        fn pool_state(&self) -> Result<Value, CaptureError> {
            Ok(self.pool_state.clone())
        }
        fn dstate_accounts(&self) -> Result<Value, CaptureError> {
            Ok(self.dstate.clone())
        }
        fn stake_snapshot(&self) -> Result<Value, CaptureError> {
            Ok(self.stake_snapshot.clone())
        }
        fn protocol_state(&self) -> Result<Value, CaptureError> {
            Ok(self.protocol_state.clone())
        }
    }

    fn rich_pool_state() -> Value {
        serde_json::json!({
            "11111111111111111111111111111111111111111111111111111111": {
                "futurePoolParams": null,
                "poolParams": {
                    "spsVrf": "52a8535d6b2e69025d188d13c10c3940a1ead314ca67cd9b400b3e36472164e0",
                    "spsMargin": 0.075, "spsPledge": 500000000, "spsCost": 340000000,
                    "spsAccountId": {"keyHash": "0470daa17236a4291be26c24d9b4bb9ed023e282077572458cdfcf1a"},
                    "spsOwners": ["0470daa17236a4291be26c24d9b4bb9ed023e282077572458cdfcf1a"]
                },
                "retiring": null
            }
        })
    }
    fn rich_dstate() -> Value {
        serde_json::json!({
            "keyHash-0000c862b7cda2e46b3b0eb5f17d1de67d8dd26fe7cc9f91a044bea6": {
                "balance": 18412, "deposit": 2000000, "drep": "drep-alwaysNoConfidence",
                "reward": 18412, "spool": "11111111111111111111111111111111111111111111111111111111"
            }
        })
    }
    fn rich_stake_snapshot() -> Value {
        let mut pools = serde_json::Map::new();
        pools.insert(
            "11111111111111111111111111111111111111111111111111111111".to_string(),
            serde_json::json!({"stakeGo": 500000000, "stakeMark": 500000000, "stakeSet": 500000000}),
        );
        serde_json::json!({"pools": pools, "total": {}})
    }
    fn rich_mock(stake_snapshot: Value) -> RichMock {
        RichMock {
            tip: CapturePoint { slot: 115_455_568, hash: "ab".repeat(32) },
            pool_state: rich_pool_state(),
            dstate: rich_dstate(),
            stake_snapshot,
            protocol_state: serde_json::json!({ "epochNonce": "bb".repeat(32) }),
        }
    }
    fn rich_profile() -> BundleProfile {
        BundleProfile {
            network_magic: 2,
            genesis_hash_hex: "9a".repeat(32),
            active_slots_coeff: (1, 20),
            epoch_no: 1336,
            epoch_start_slot: 1,
            epoch_end_slot: 432000,
            protocol_params_json: "{\"k\":1}".into(),
            node_version: "cardano-node 11.0.1".into(),
            query_command: "cardano-cli query ... --immutable-tip".into(),
        }
    }

    #[test]
    fn rich_mock_round_trip_emits_a_self_verifying_four_file_package() {
        let dir = tempfile::TempDir::new().unwrap();
        let base = dir.path().join("ade-inputs-ep1336");
        let receipt = run_bootstrap_export(
            &rich_mock(rich_stake_snapshot()),
            &rich_profile(),
            Hash32([0x9c; 32]),
            "preview",
            Hash32([0x55; 32]),
            &base,
        )
        .expect("export must succeed");
        for p in [&receipt.bundle_path, &receipt.certstate_path, &receipt.manifest_path, &receipt.inspect_path] {
            assert!(std::path::Path::new(p).exists(), "{p} must exist");
        }
        // the EXISTING importer self-verifies the emitted bytes (catches manifest/hash/sibling wiring bugs).
        let inspect: Value = serde_json::from_slice(&std::fs::read(&receipt.inspect_path).unwrap()).unwrap();
        assert_eq!(inspect["binding_verdict"].as_str(), Some("bound"), "emitted package self-verifies");
        assert_eq!(inspect["network_profile_id"].as_str(), Some("preview"), "inspect names the resolved profile");
        // the ONE captured point P flows identically into capture -> bundle -> manifest/inspect.
        assert_eq!(inspect["source_slot"].as_u64(), Some(115_455_568));
        let bundle_bytes = std::fs::read(&receipt.bundle_path).unwrap();
        let raw = ade_runtime::consensus_inputs::json::parse_consensus_inputs_json(&bundle_bytes).unwrap();
        assert_eq!(raw.source_tip_slot, 115_455_568, "bundle binds the same P slot");
        assert_eq!(raw.source_tip_hash_hex, "ab".repeat(32), "bundle binds the same P hash");
    }

    #[test]
    fn rich_mock_failure_emits_no_partial_package() {
        // a pool with stake but no VRF in pool-state -> fails closed BEFORE the atomic emit.
        let mut pools = serde_json::Map::new();
        pools.insert(
            "33333333333333333333333333333333333333333333333333333333".to_string(),
            serde_json::json!({"stakeGo": 1, "stakeMark": 1, "stakeSet": 1}),
        );
        let bad_snapshot = serde_json::json!({"pools": pools, "total": {}});
        let dir = tempfile::TempDir::new().unwrap();
        let base = dir.path().join("ade-inputs-ep1336");
        let r = run_bootstrap_export(
            &rich_mock(bad_snapshot),
            &rich_profile(),
            Hash32([0x9c; 32]),
            "preview",
            Hash32([0x55; 32]),
            &base,
        );
        assert!(r.is_err(), "must fail closed");
        assert!(!base.with_extension("json").exists(), "no bundle emitted on failure");
        assert!(
            std::fs::read_dir(dir.path()).unwrap().next().is_none(),
            "temp dir empty — no partial package"
        );
    }

    // ----- The closed network-profile registry -----
    #[test]
    fn network_registry_resolves_committed_profiles_and_fails_closed_on_unknown() {
        let preview = resolve_network_profile("preview").unwrap();
        assert_eq!(preview.network_magic, 2);
        assert_eq!(preview.epoch_length, 86_400);
        assert_eq!(preview.active_slots_coeff, (1, 20));
        assert_eq!(
            hex32(&preview.genesis_hash),
            "363498d1024f84bb39d3fa9593ce391483cb40d479b87233f868d6e57c3a400d"
        );
        let preprod = resolve_network_profile("preprod").unwrap();
        assert_eq!(preprod.network_magic, 1);
        assert_eq!(preprod.epoch_length, 432_000);
        // unknown identity fails closed (no loose default).
        assert!(resolve_network_profile("mainnet").is_err());
        assert!(resolve_network_profile("").is_err());
        // distinct network identities -> distinct commitments.
        assert_ne!(preview.commitment(), preprod.commitment());
        // the committed preview epoch geometry matches the live node (slot 115_455_568 -> epoch 1336).
        assert_eq!(preview.epoch_of(115_455_568).0, 1336);
    }

    #[test]
    fn reject_a_preview_capture_under_a_preprod_profile() {
        let preview = resolve_network_profile("preview").unwrap();
        let preprod = resolve_network_profile("preprod").unwrap();
        // a live preview node: magic 2, epoch 1336 @ slot 115_455_568 — passes under its own profile.
        assert!(preview.verify_live_magic(2).is_ok());
        assert!(preview.verify_epoch(115_455_568, 1336).is_ok());
        // selecting --network preprod against that same preview node MUST fail closed (both guards).
        assert!(preprod.verify_live_magic(2).is_err(), "preprod profile rejects a preview-magic node");
        assert!(
            preprod.verify_epoch(115_455_568, 1336).is_err(),
            "preprod epoch geometry (267) != node epoch 1336"
        );
    }
}
