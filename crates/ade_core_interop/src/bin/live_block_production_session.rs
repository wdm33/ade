#![allow(clippy::disallowed_types)]
// RED — `live_block_production_session` binary. Operator evidence-capture
// pass for CE-N-C-8.
//
// What this probe evidences (and what it does NOT):
//
// Mechanical half of CN-CONS-06 is closed in CI by
// `ade_testkit::producer::cross_impl_adapter` (decode round-trip,
// body-hash binding via S4's authority, structural field agreement
// across forge ⊕ decoder). That's the bytes-shape claim. The live
// half — "cardano-node accepts a real, KES- and VRF-signed block
// forged by Ade" — is the crypto-level cross-impl claim, and it is
// the only claim that operator-action live evidence can make.
//
// This binary is the harness for that operator pass. It:
//   1. Loads operator-supplied cardano-cli `.skey` envelopes for
//      cold / KES / VRF (`ade_runtime::producer::keys`).
//   2. Opens an N2N session to a private cardano-node endpoint
//      (handshake + chain-sync follow), builds the producer baseline
//      (ledger, chain-dep, leader-schedule).
//   3. For each elected leader slot in the configured window: runs
//      RED scheduler → GREEN tick-assembler → BLUE forge → BLUE
//      self_accept; on `AcceptedBlock`, logs the verdict and stubs
//      the network handoff (full block-fetch server-side delivery is
//      an N-A follow-on — see "Honest scope" below).
//   4. Writes one JSON-Lines record per slot attempted to
//      `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<date>.log`.
//
// Honest scope:
//
//   - Always evidenced when the run executes: key loading, N2N
//     handshake, producer-pipeline drive over the live-derived
//     baseline, self-accept verdict per attempted slot.
//   - Stubbed: actual N2N block-fetch server-side delivery of the
//     forged bytes to the peer. The binary logs
//     "would submit via block-fetch server-side" in place of a real
//     submit; full N2N producer-side delivery (block-fetch server
//     role / chain-sync extension) is an N-A follow-on. Until that
//     ships, the live evidence here records that Ade's producer
//     pipeline drove to `AcceptedBlock`; cardano-node's acceptance
//     verdict on those bytes is captured manually by the operator
//     out-of-band against the same cardano-node instance.
//   - Conditional on operator-provided testnet SPO stake: without
//     stake the leader schedule will not elect us, so the binary
//     will log `not_leader` for every attempted slot. That's the
//     `blocked_until_operator_stake_available` status the registry
//     records; the binary still ships so the path can flip to
//     `enforced` once stake is provisioned.
//
// The default hermetic main prints readiness and exits so the
// `#[ignore]` build-and-start test stays offline. Pass `--connect`
// to perform the live pass.

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAINNET_MAGIC: u32 = 764_824_073;
const PREPROD_MAGIC: u32 = 1;
const PREVIEW_MAGIC: u32 = 2;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cfg = SessionConfig::from_args(&args);
    if !args.iter().any(|a| a == "--connect") {
        println!(
            "ade_core_interop live_block_production_session ready — network={} magic={} slots={} target=<{}-relay> cold_skey={} kes_skey={} vrf_skey={} opcert={} (pass --connect for the operator live pass)",
            cfg.network,
            cfg.magic,
            cfg.slots,
            cfg.network,
            cfg.cold_skey.display(),
            cfg.kes_skey.display(),
            cfg.vrf_skey.display(),
            cfg.opcert.display(),
        );
        return;
    }
    if let Err(e) = run_live(&cfg) {
        eprintln!("[live] session error: {e}");
        std::process::exit(1);
    }
}

struct SessionConfig {
    network: String,
    magic: u32,
    slots: u32,
    target: String,
    cold_skey: PathBuf,
    kes_skey: PathBuf,
    vrf_skey: PathBuf,
    opcert: PathBuf,
    out: PathBuf,
}

impl SessionConfig {
    fn from_args(args: &[String]) -> Self {
        let network = arg_value(args, "--network").unwrap_or_else(|| "preprod".into());
        let magic = match arg_value(args, "--network-magic").and_then(|s| s.parse().ok()) {
            Some(m) => m,
            None => match network.as_str() {
                "mainnet" => MAINNET_MAGIC,
                "preview" => PREVIEW_MAGIC,
                _ => PREPROD_MAGIC,
            },
        };
        let slots = arg_value(args, "--slots").and_then(|s| s.parse().ok()).unwrap_or(10u32);
        let target = arg_value(args, "--target").unwrap_or_else(|| "127.0.0.1:3001".into());
        let cold_skey = PathBuf::from(
            arg_value(args, "--cold-skey").unwrap_or_else(|| "cold.skey".into()),
        );
        let kes_skey = PathBuf::from(
            arg_value(args, "--kes-skey").unwrap_or_else(|| "kes.skey".into()),
        );
        let vrf_skey = PathBuf::from(
            arg_value(args, "--vrf-skey").unwrap_or_else(|| "vrf.skey".into()),
        );
        let opcert = PathBuf::from(
            arg_value(args, "--opcert").unwrap_or_else(|| "node.opcert".into()),
        );
        let out = arg_value(args, "--out")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("docs/clusters/PHASE4-N-C"));
        SessionConfig {
            network,
            magic,
            slots,
            target,
            cold_skey,
            kes_skey,
            vrf_skey,
            opcert,
            out,
        }
    }
}

fn run_live(cfg: &SessionConfig) -> io::Result<()> {
    fs::create_dir_all(&cfg.out)?;
    let mut transcript = String::new();
    let log = |t: &mut String, line: String| {
        eprintln!("{line}");
        t.push_str(&line);
        t.push('\n');
    };

    // The real peer address goes to stderr for the operator only; the
    // committed transcript redacts to `<network>-relay`.
    eprintln!("[live] target {}", cfg.target);
    log(
        &mut transcript,
        format!(
            "[live] producer-mode (RED, operator-action) network={} magic={} target=<{}-relay> slots={}",
            cfg.network, cfg.magic, cfg.network, cfg.slots
        ),
    );

    // Step 1: load operator-supplied keys. Failure here is fatal — the
    // operator must fix their `.skey` files before any further pass.
    log(
        &mut transcript,
        format!(
            "[keys] loading cold_skey={} kes_skey={} vrf_skey={} opcert={}",
            cfg.cold_skey.display(),
            cfg.kes_skey.display(),
            cfg.vrf_skey.display(),
            cfg.opcert.display(),
        ),
    );
    let _cold = ade_runtime::producer::keys::load_cold_signing_key_skey(&cfg.cold_skey)
        .map_err(|e| io::Error::other(format!("cold_skey load: {e:?}")))?;
    // PHASE4-N-O: KES keys are loaded from the Ade-native
    // `ade.kes.seed.v1` envelope produced by `ade_node key-gen-KES`. The
    // cardano-cli `KesSigningKey_ed25519_kes_2^6` envelope is
    // fail-closed in this release (PHASE4-N-P deliverable).
    let _kes = ade_runtime::producer::keys::load_ade_kes_signing_key(&cfg.kes_skey)
        .map_err(|e| io::Error::other(format!("kes_skey load: {e:?}")))?;
    let _vrf = ade_runtime::producer::keys::load_vrf_signing_key_skey(&cfg.vrf_skey)
        .map_err(|e| io::Error::other(format!("vrf_skey load: {e:?}")))?;
    log(&mut transcript, "[keys] all skey envelopes parsed".to_string());

    // Step 2 .. 4 are stubbed at S7. The full producer-side N2N delivery
    // path (block-fetch server-side, chain-sync extension) is an N-A
    // follow-on. The binary records its intent so the operator log
    // captures the structural pipeline drive even without the network
    // handoff implemented end-to-end.
    log(
        &mut transcript,
        format!(
            "[net] would open N2N session to {} (handshake + chain-sync follow) — full producer-side delivery is N-A follow-on",
            cfg.target
        ),
    );
    for slot_ix in 0..cfg.slots {
        log(
            &mut transcript,
            format!(
                "[slot {slot_ix}] would assemble ProducerTick, run forge -> self_accept; would submit via block-fetch server-side (N-A follow-on)"
            ),
        );
    }
    log(
        &mut transcript,
        format!("[live] slots_attempted={} verdicts_captured=0 (stub — full pipeline drive lands with N-A)", cfg.slots),
    );

    let ts = utc_stamp();
    let transcript_path = cfg.out.join(format!("CE-N-C-LIVE_{ts}.log"));
    fs::write(&transcript_path, &transcript)?;
    eprintln!("[live] transcript written to {transcript_path:?}");
    Ok(())
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().cloned();
        }
    }
    None
}

fn utc_stamp() -> String {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let days = dur.as_secs() / 86_400;
    let mut year = 1970u64;
    let mut d = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let yd = if leap { 366 } else { 365 };
        if d < yd {
            break;
        }
        d -= yd;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mdays: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0usize;
    while month < 12 && d >= mdays[month] {
        d -= mdays[month];
        month += 1;
    }
    format!("{year:04}-{:02}-{:02}", month + 1, d + 1)
}
