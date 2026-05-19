// RED — `live_consensus_session` binary. Manual evidence-capture pass
// for CE-N-B-6.
//
// Scaffold: builds the orchestrator at genesis and prints "ready" so a
// human operator running the binary against a pinned-Docker
// cardano-node 10.6.2 can confirm the wiring before they extend the
// session driver to subscribe to chain-sync via `ade_network` and feed
// arriving headers into `process_stream_input`.
//
// The closure-gate test (`tests/live_consensus_session.rs`, gated
// behind `#[ignore]`) invokes this binary and asserts it starts. The
// full live tip-agreement validation is the operator's task per the
// slice doc.

use ade_core::consensus::praos_state::Nonce;
use ade_core_interop::fresh_orchestrator;
use ade_types::Hash32;

fn main() {
    // k = 2160 mainnet, initial nonce = all-zeros placeholder (the
    // real session driver will derive this from genesis JSON via
    // `ade_runtime::consensus::genesis_parser`).
    let state = fresh_orchestrator(2160, Nonce(Hash32([0u8; 32])));
    println!(
        "ade_core_interop live_consensus_session ready — current_tip_block_no = {:?}, k = {:?}",
        state.selector.current_tip_block_no, state.selector.security_param
    );
}
