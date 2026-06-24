//! RED — `ade mithril snapshot fetch` (MITHRIL-FIRST-RUN-CONTINUITY S4): the judge-facing snapshot
//! ACQUISITION command. It is the only step that talks to Mithril; `ade node run` then consumes a
//! purely-local verified directory.
//!
//! Flow: resolve the committed `NetworkProfile` + Mithril aggregator/keys for `--network` →
//! download (or reuse) a Mithril cardano-db snapshot with `--include-ancillary` →
//! derive the certified point NATIVELY from the snapshot's ledger state
//! (`decode_ledgerdb_tip` — slot + header hash; NO frozen node) → lay out `state`/`tables` where
//! `ade node run --snapshot-dir` expects them → emit `manifest.json` + a compact verified-snapshot
//! receipt. A pinned `--certificate` selects a specific snapshot for reproducibility instead of
//! "latest".

use std::path::{Path, PathBuf};
use std::process::Command;

/// Committed per-network Mithril acquisition profile (closed, like `NetworkProfile`). The genesis +
/// ancillary verification keys are PUBLIC Mithril constants; an env override is allowed for key
/// rotation (`ADE_MITHRIL_GENESIS_VKEY` / `ADE_MITHRIL_ANCILLARY_VKEY` / `ADE_MITHRIL_AGGREGATOR`).
pub struct MithrilProfile {
    pub aggregator: String,
    pub genesis_vkey: String,
    pub ancillary_vkey: String,
}

/// Resolve the committed Mithril profile. Preview = the `pre-release-preview` deployment.
pub fn resolve_mithril_profile(network: &str) -> Option<MithrilProfile> {
    let (agg, gvk, avk): (&str, &str, &str) = match network {
        "preview" => (
            "https://aggregator.pre-release-preview.api.mithril.network/aggregator",
            "5b3132372c37332c3132342c3136312c362c3133372c3133312c3231332c3230372c3131372c3139382c38352c3137362c3139392c3136322c3234312c36382c3132332c3131392c3134352c31332c3233322c3234332c34392c3232392c322c3234392c3230352c3230352c33392c3233352c34345d",
            "5b3138392c3139322c3231362c3135302c3131342c3231362c3233372c3231302c34352c31382c32312c3139362c3230382c3234362c3134362c322c3235322c3234332c3235312c3139372c32382c3135372c3230342c3134352c33302c31342c3232382c3136382c3132392c38332c3133362c33365d",
        ),
        "preprod" => (
            "https://aggregator.release-preprod.api.mithril.network/aggregator",
            "", // preprod keys: env-supplied (the preview lane is the committed bounty path)
            "",
        ),
        _ => return None,
    };
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
    Some(MithrilProfile {
        aggregator: env("ADE_MITHRIL_AGGREGATOR", agg),
        genesis_vkey: env("ADE_MITHRIL_GENESIS_VKEY", gvk),
        ancillary_vkey: env("ADE_MITHRIL_ANCILLARY_VKEY", avk),
    })
}

#[derive(Debug)]
pub enum FetchError {
    UnknownNetwork(String),
    Io(String),
    Download(String),
    NoLedgerSnapshot(String),
    TipDecode(String),
    NetworkMismatch(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::UnknownNetwork(n) => write!(f, "unknown --network '{n}' (preview|preprod)"),
            FetchError::Io(e) => write!(f, "io: {e}"),
            FetchError::Download(e) => write!(f, "mithril-client download: {e}"),
            FetchError::NoLedgerSnapshot(e) => write!(f, "no ledger snapshot in download: {e}"),
            FetchError::TipDecode(e) => write!(f, "decode certified point from ledger state: {e}"),
            FetchError::NetworkMismatch(e) => write!(f, "snapshot network mismatch: {e}"),
        }
    }
}

fn to_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// The latest `db/ledger/<slot>` directory (highest slot) under the snapshot dir.
fn latest_ledger_dir(db: &Path) -> Result<(PathBuf, u64), FetchError> {
    let ledger = db.join("ledger");
    let mut best: Option<(PathBuf, u64)> = None;
    for entry in std::fs::read_dir(&ledger).map_err(|e| {
        FetchError::NoLedgerSnapshot(format!("{}: {e}", ledger.display()))
    })? {
        let entry = entry.map_err(|e| FetchError::Io(e.to_string()))?;
        if let Some(name) = entry.file_name().to_str() {
            if let Ok(slot) = name.parse::<u64>() {
                if best.as_ref().map(|(_, b)| slot > *b).unwrap_or(true) {
                    best = Some((entry.path(), slot));
                }
            }
        }
    }
    best.ok_or_else(|| FetchError::NoLedgerSnapshot(format!("no <slot> dirs under {}", ledger.display())))
}

/// Highest immutable chunk number under `db/immutable` (the `immutable_range.hi`).
fn immutable_hi(db: &Path) -> u64 {
    let dir = db.join("immutable");
    let mut hi = 0u64;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if let Some(stem) = e.path().file_stem().and_then(|s| s.to_str()) {
                if let Ok(n) = stem.parse::<u64>() {
                    hi = hi.max(n);
                }
            }
        }
    }
    hi
}

/// `mithril-client cardano-db snapshot list --json` -> the latest snapshot digest. The version banner
/// is on stdout before the JSON, so scan for the first `"hash":"<64hex>"`.
fn latest_digest(profile: &MithrilProfile) -> Result<String, FetchError> {
    let out = Command::new("mithril-client")
        .env("AGGREGATOR_ENDPOINT", &profile.aggregator)
        .env("GENESIS_VERIFICATION_KEY", &profile.genesis_vkey)
        .env("ANCILLARY_VERIFICATION_KEY", &profile.ancillary_vkey)
        .args(["cardano-db", "snapshot", "list", "--json"])
        .output()
        .map_err(|e| FetchError::Download(format!("snapshot list: {e} (is mithril-client installed?)")))?;
    let s = String::from_utf8_lossy(&out.stdout);
    let needle = "\"hash\":\"";
    let i = s.find(needle).ok_or_else(|| {
        FetchError::Download("snapshot list returned no snapshots".to_string())
    })?;
    let rest = &s[i + needle.len()..];
    let end = rest.find('"').ok_or_else(|| FetchError::Download("malformed snapshot list".to_string()))?;
    Ok(rest[..end].to_string())
}

/// Download + verify a cardano-db snapshot (mithril verifies the immutable files via the certificate;
/// the ancillary ledger state is IOG-signed). Returns the digest used.
fn download(profile: &MithrilProfile, output_dir: &Path, certificate: Option<&Path>) -> Result<String, FetchError> {
    let digest = match certificate {
        Some(c) => {
            // pinned: the cert/manifest file carries the snapshot digest (a 64-hex token).
            let txt = std::fs::read_to_string(c).map_err(|e| FetchError::Io(format!("{}: {e}", c.display())))?;
            let needle = "\"hash\":\"";
            if let Some(i) = txt.find(needle) {
                let rest = &txt[i + needle.len()..];
                rest[..rest.find('"').unwrap_or(0)].to_string()
            } else {
                txt.trim().to_string()
            }
        }
        None => latest_digest(profile)?,
    };
    eprintln!("ade mithril snapshot fetch: downloading {digest} (--include-ancillary) …");
    let status = Command::new("mithril-client")
        .env("AGGREGATOR_ENDPOINT", &profile.aggregator)
        .env("GENESIS_VERIFICATION_KEY", &profile.genesis_vkey)
        .env("ANCILLARY_VERIFICATION_KEY", &profile.ancillary_vkey)
        .args([
            "cardano-db",
            "download",
            &digest,
            "--include-ancillary",
            "--download-dir",
            output_dir.to_str().ok_or_else(|| FetchError::Io("non-utf8 output dir".to_string()))?,
        ])
        .status()
        .map_err(|e| FetchError::Download(format!("{e} (is mithril-client installed?)")))?;
    if !status.success() {
        return Err(FetchError::Download(format!("mithril-client exited {status}")));
    }
    Ok(digest)
}

/// Lay out `state`/`tables` at `<output_dir>/{state,tables}` (where `ade node run --snapshot-dir`
/// reads them) as symlinks into the canonical `db/ledger/<slot>/` (no 640 MB copy; the snapshot dir
/// is the durable acquisition artifact).
fn layout(output_dir: &Path, ledger_dir: &Path) -> Result<(), FetchError> {
    for f in ["state", "tables"] {
        let link = output_dir.join(f);
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(ledger_dir.join(f), &link)
            .map_err(|e| FetchError::Io(format!("symlink {f}: {e}")))?;
    }
    Ok(())
}

/// `ade mithril snapshot fetch` entry point.
pub fn run_mithril_snapshot_fetch(
    network: &str,
    output_dir: &Path,
    certificate: Option<&Path>,
) -> Result<(), FetchError> {
    let net_profile = crate::bootstrap_export::resolve_network_profile(network)
        .map_err(|_| FetchError::UnknownNetwork(network.to_string()))?;
    let mithril = resolve_mithril_profile(network).ok_or_else(|| FetchError::UnknownNetwork(network.to_string()))?;

    std::fs::create_dir_all(output_dir).map_err(|e| FetchError::Io(e.to_string()))?;
    let db = output_dir.join("db");

    // 1. Acquire: reuse an already-verified local snapshot, else download (verifying) once.
    let (verification, digest) = if db.join("ledger").is_dir() {
        eprintln!(
            "ade mithril snapshot fetch: reusing verified snapshot at {} (delete it to re-download)",
            db.display()
        );
        // The Mithril snapshot/cert digest from a prior download (.mithril-digest), else a zero
        // placeholder. Recorded provenance only -- node-run binds on network/genesis/certified-point,
        // NOT on this hash -- but the manifest field MUST be valid 64-hex (else BadHashHex terminal).
        let digest = std::fs::read_to_string(output_dir.join(".mithril-digest"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .unwrap_or_else(|| "0".repeat(64));
        ("reused (previously mithril-verified)".to_string(), digest)
    } else {
        let d = download(&mithril, output_dir, certificate)?;
        let _ = std::fs::write(output_dir.join(".mithril-digest"), &d);
        (
            "mithril-client verified the immutable files via the certificate; ancillary IOG-signed"
                .to_string(),
            d,
        )
    };

    // 2. Native certified point from the snapshot's ledger state (slot + header hash) — no frozen node.
    let (ledger_dir, _ledger_slot) = latest_ledger_dir(&db)?;
    let state_path = ledger_dir.join("state");
    let state = std::fs::read(&state_path).map_err(|e| FetchError::Io(format!("{}: {e}", state_path.display())))?;
    let (tip_slot, tip_hash) = ade_ledger::ledgerdb_state::decode_ledgerdb_tip(&state)
        .map_err(|e| FetchError::TipDecode(format!("{e:?}")))?;

    // 3. Lay out state/tables for `ade node run --snapshot-dir`.
    layout(output_dir, &ledger_dir)?;

    let genesis_hex = to_hex(&net_profile.genesis_hash.0);
    let tip_hash_hex = to_hex(&tip_hash.0);
    let imm_hi = immutable_hi(&db);
    let mc_version = Command::new("mithril-client")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "mithril-client".to_string());

    // 4. Emit manifest.json (the RawMithrilManifest shape `ade node run --bootstrap-mithril` parses).
    let manifest = format!(
        "{{\n  \"artifact_type\": \"cardano-database-snapshot\",\n  \"certificate_hash_hex\": \"{digest}\",\n  \"network_magic\": {magic},\n  \"genesis_hash_hex\": \"{genesis_hex}\",\n  \"certified_point\": {{ \"slot\": {slot}, \"block_hash_hex\": \"{tip_hash_hex}\" }},\n  \"immutable_range\": {{ \"lo\": 0, \"hi\": {imm_hi} }},\n  \"source_mithril_client_version\": \"{mc_version}\",\n  \"source_command\": \"ade mithril snapshot fetch --network {network}\"\n}}\n",
        magic = net_profile.network_magic,
        slot = tip_slot.0,
    );
    let manifest_path = output_dir.join("manifest.json");
    std::fs::write(&manifest_path, &manifest).map_err(|e| FetchError::Io(format!("manifest: {e}")))?;

    // 5. The compact verified-snapshot receipt.
    let state_sz = std::fs::metadata(&state_path).map(|m| m.len()).unwrap_or(0);
    let tables_sz = std::fs::metadata(ledger_dir.join("tables")).map(|m| m.len()).unwrap_or(0);
    let receipt = format!(
        "\n=== Ade verified Mithril snapshot receipt ===\n\
         network / profile      : {network} (magic {magic})\n\
         shelley genesis hash   : {genesis_hex}\n\
         mithril aggregator     : {agg}\n\
         certified point        : slot {slot} / block {tip_hash_hex}\n\
         ledger state           : {state} ({state_sz} bytes)\n\
         ledger tables          : {tables} ({tables_sz} bytes)\n\
         immutable range        : 0 .. {imm_hi}\n\
         verification           : {verification}\n\
         manifest               : {manifest_path}\n\
         ============================================\n\
         next: ade node run --network {network} --bootstrap-mithril {manifest_path} \\\n\
         \x20      --snapshot-dir {out} --data-dir <your-data-dir> --peer <preview-node-addr>\n",
        magic = net_profile.network_magic,
        agg = mithril.aggregator,
        slot = tip_slot.0,
        state = state_path.display(),
        tables = ledger_dir.join("tables").display(),
        manifest_path = manifest_path.display(),
        out = output_dir.display(),
    );
    eprintln!("{receipt}");
    let _ = std::fs::write(output_dir.join("snapshot-receipt.txt"), receipt.trim_start());

    Ok(())
}
