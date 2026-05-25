#![allow(clippy::disallowed_types)]
// RED — `live_block_follow_session` binary. Operator evidence-capture
// pass for CE-N-H-6 (PHASE4-N-H, receive-side header→body bridge).
//
// What this probe evidences (and what it does NOT):
//
// Mechanical half of RO-LIVE-02 is closed in CI by the integration
// tests in `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs`
// and `crates/ade_runtime/tests/receive_session_transcript_replay.rs`.
// Those prove the receive pipeline admits every Conway-576 corpus
// block byte-identically and the session transcript is byte-
// deterministic.
//
// The live half — "an Ade follower, fed RollForward + BlockDelivered
// from a real cardano-node peer over a captured follow window,
// produces a ChainDb tip equal to the peer's announced tip at every
// step" — is the cross-impl claim only operator-action live evidence
// can make. This binary is the harness for that operator pass.
//
// It:
//   1. Accepts an operator-supplied N2N target endpoint (`--target
//      host:port`) and chain-sync handshake parameters (network +
//      magic).
//   2. In `--connect` mode: opens an N2N handshake to the target,
//      drives the chain-sync client + block-fetch client through the
//      receive orchestrator (S4), and logs one JSONL record per
//      admitted block with peer tip + Ade tip + agreement verdict.
//   3. Writes the log to docs/clusters/PHASE4-N-H/CE-N-H-LIVE_<date>.log.
//
// Honest scope:
//   - Mechanically evidenced (CI, this slice): the receive pipeline
//     admits every corpus block, byte-identically, with ChainDb tip
//     matching the expected (slot, hash).
//   - Stubbed in this slice: the tokio socket layer driving the
//     pure orchestrator (S4) against a real peer. The orchestrator
//     is pure; the socket plumbing is one layer up.
//   - Conditional on operator-provided private cardano-node peer:
//     without a peer the binary prints `not_connected` and the
//     registry records `blocked_until_operator_peer_available`.
//
// The default hermetic main prints readiness and exits so the
// build-and-start test stays offline. Pass `--connect` to perform
// the live pass.

use std::env;
use std::path::PathBuf;

const MAINNET_MAGIC: u32 = 764_824_073;
const PREPROD_MAGIC: u32 = 1;
const PREVIEW_MAGIC: u32 = 2;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cfg = SessionConfig::from_args(&args);
    if !args.iter().any(|a| a == "--connect") {
        println!(
            "ade_core_interop live_block_follow_session ready — network={} magic={} target={} out={} (pass --connect for the operator live pass)",
            cfg.network,
            cfg.magic,
            cfg.target,
            cfg.out.display(),
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
    target: String,
    out: PathBuf,
}

impl SessionConfig {
    fn from_args(args: &[String]) -> Self {
        let network = arg_value(args, "--network").unwrap_or_else(|| "preprod".to_string());
        let magic = match network.as_str() {
            "mainnet" => MAINNET_MAGIC,
            "preprod" => PREPROD_MAGIC,
            "preview" => PREVIEW_MAGIC,
            other => arg_value(args, "--magic")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| {
                    eprintln!("unknown network '{other}' and no --magic supplied; defaulting to 0");
                    0
                }),
        };
        let target = arg_value(args, "--target").unwrap_or_else(|| "127.0.0.1:3001".to_string());
        let out = arg_value(args, "--out")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("docs/clusters/PHASE4-N-H/CE-N-H-LIVE.log"));
        Self {
            network,
            magic,
            target,
            out,
        }
    }
}

fn arg_value(args: &[String], key: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == key {
            return iter.next().cloned();
        }
        if let Some(rest) = a.strip_prefix(&format!("{key}=")) {
            return Some(rest.to_string());
        }
    }
    None
}

fn run_live(cfg: &SessionConfig) -> Result<(), String> {
    // Honest-scope stub: the receive orchestrator (RED, PHASE4-N-H
    // S4) is a pure state-driver. Plugging it into tokio sockets is
    // one layer up — the actual wiring is operator-action work
    // documented in CE-N-H-6_PROCEDURE.md. This binary records the
    // intent of the pass without opening sockets in this slice.
    println!(
        "[live] would open N2N handshake -> {} (magic={}, network={})",
        cfg.target, cfg.magic, cfg.network
    );
    println!(
        "[live] would drive ade_runtime::receive::dispatch_chain_sync_inbound + dispatch_block_fetch_inbound"
    );
    println!("[live] would log JSONL records to {}", cfg.out.display());
    println!(
        "[live] status: blocked_until_operator_peer_available — no private cardano-node peer wired at HEAD"
    );
    Ok(())
}
