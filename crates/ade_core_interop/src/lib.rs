// RED — Live cardano-node interop driver. This crate carries no
// authoritative decisions; every interesting state-transition lives in
// BLUE `ade_core::consensus::*` and GREEN
// `ade_runtime::consensus::chain_selector`. The library surface here
// is a thin readiness probe used by the closure-gate test to assert
// the binary builds and the orchestrator can be wired up.
//
// The full live tip-agreement loop (subscribe to chain-sync, feed
// arriving headers into `process_stream_input`, assert peer tip
// equality for a sustained window) is the manual operator pass per
// the slice doc; the manual run captures evidence at
// `docs/clusters/PHASE4-N-B/CE-N-B-6_<date>.log`.

#![deny(unsafe_code)]

pub mod follow;
pub mod tx_submission;

use ade_core::consensus::candidate::{ChainSelectorState, TiebreakerView};
use ade_core::consensus::events::{Point, SecurityParam};
use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_runtime::consensus::chain_selector::OrchestratorState;
use ade_types::{BlockNo, Hash28, Hash32, SlotNo};

/// Build a fresh orchestrator state at genesis with the supplied
/// security-parameter k. Used by the `#[ignore]` interop test as a
/// readiness probe — the orchestrator can be constructed without
/// touching the network.
pub fn fresh_orchestrator(k: u64, initial_nonce: Nonce) -> OrchestratorState {
    let selector = ChainSelectorState {
        current_tip: Point {
            slot: SlotNo(0),
            hash: Hash32([0u8; 32]),
        },
        current_tip_block_no: BlockNo(0),
        current_tiebreaker: TiebreakerView {
            slot: SlotNo(0),
            issuer_hash: Hash28([0u8; 28]),
            op_cert_counter: 0,
            leader_vrf_output_first_8: [0u8; 8],
        },
        immutable_tip: Point {
            slot: SlotNo(0),
            hash: Hash32([0u8; 32]),
        },
        immutable_tip_block_no: BlockNo(0),
        security_param: SecurityParam(k),
    };
    OrchestratorState::new(PraosChainDepState::genesis(initial_nonce), selector)
}
