# Invariant Slice — PHASE4-N-G S6

## Slice Header

**Slice Name:** RED per-peer N2N server orchestrator (pure session driver)
**Cluster:** PHASE4-N-G
**Status:** In Progress
**CEs addressed:** CE-N-G-6
**Registry effects on merge:** `DC-PROTO-06.strengthened_in += "PHASE4-N-G"` (version threaded through server-role surface); confirms `CN-PROTO-06` end-to-end. No new flip beyond what S1/S3 already enforce.
**Dependencies:** N-G-S1..S5

---

## Intent

Compose the GREEN adapter (S5) + BLUE reducers (S3/S4) into a
per-peer session driver that decodes inbound mini-protocol frames,
calls the reducers, and produces outbound encoded frames. The
driver is **pure** — actual socket I/O is a thin tokio wrapper in
S7's evidence binary; this slice's surface is the deterministic
RED-style state machine.

Multi-peer determinism (OQ-4 resolved): per-peer state is
independent; cross-peer coordination is only through the single
`ServedChainSnapshot`. Two synthetic peers driven against one
orchestrator preserve per-session transcripts.

Key-boundary preserved: `ade_runtime::network::n2n_server` MUST NOT
import from `ade_runtime::producer::signing`. CI grep gate enforces.

---

## The change

### 1. New module `crates/ade_runtime/src/network/n2n_server.rs` (RED)

```rust
use ade_network::{
    block_fetch::server::{
        producer_block_fetch_serve, BlockFetchServerStep,
        ProducerBlockFetchServerError, ProducerBlockFetchServerState,
    },
    chain_sync::server::{
        producer_chain_sync_advance_tip, producer_chain_sync_serve,
        ProducerChainSyncServerState, ProducerServerError, ServerStep,
    },
    codec::block_fetch::{
        decode_block_fetch_message, encode_block_fetch_message, BlockFetchMessage,
    },
    codec::chain_sync::{
        decode_chain_sync_message, encode_chain_sync_message, ChainSyncMessage,
    },
    codec::version::{BlockFetchVersion, ChainSyncVersion},
};

use crate::producer::served_chain_lookups::ServedChainLookups;
use ade_ledger::producer::ServedChainSnapshot;

/// Per-peer N2N server state for a single connected peer. Holds both
/// mini-protocol reducer states; cross-peer coordination is only via
/// the shared `&ServedChainSnapshot`.
pub struct PerPeerN2nServerState {
    pub chain_sync: ProducerChainSyncServerState,
    pub block_fetch: ProducerBlockFetchServerState,
    pub chain_sync_version: ChainSyncVersion,
    pub block_fetch_version: BlockFetchVersion,
}

pub enum DispatchError {
    ChainSyncDecode(ade_codec::CodecError),
    BlockFetchDecode(ade_codec::CodecError),
    ChainSync(ProducerServerError),
    BlockFetch(ProducerBlockFetchServerError),
}

/// Process one inbound chain-sync frame. Decodes via the existing
/// PHASE4-N-A codec, runs the reducer, encodes any outgoing reply.
pub fn dispatch_chain_sync_frame(
    state: PerPeerN2nServerState,
    frame: &[u8],
    snap: &ServedChainSnapshot,
) -> Result<(PerPeerN2nServerState, Option<Vec<u8>>, bool /* session_done */), DispatchError>;

/// Same for block-fetch (returns Vec because RequestRange yields
/// multiple replies).
pub fn dispatch_block_fetch_frame(
    state: PerPeerN2nServerState,
    frame: &[u8],
    snap: &ServedChainSnapshot,
) -> Result<(PerPeerN2nServerState, Vec<Vec<u8>>, bool), DispatchError>;

/// Poll for a deferred chain-sync RollForward after broadcast
/// admission. The orchestrator calls this once per peer after each
/// `drain_and_admit`.
pub fn poll_chain_sync_advance(
    state: PerPeerN2nServerState,
    snap: &ServedChainSnapshot,
) -> Result<(PerPeerN2nServerState, Option<Vec<u8>>), DispatchError>;
```

### 2. CI gate `ci/ci_check_n2n_server_no_signing_dep.sh`

Greps `crates/ade_runtime/src/network/n2n_server.rs` for any import
of `producer::signing`, `signing::*`, `VrfSigningKey`, `KesSecret`,
`SigningError`. Fails on match.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/src/network/n2n_server.rs` (tests):
- `dispatch_chain_sync_frame_request_next_yields_outgoing_frame`
- `dispatch_chain_sync_frame_threads_negotiated_version_to_reducer`
- `dispatch_block_fetch_frame_request_range_yields_batch_frames`
- `dispatch_chain_sync_frame_rejects_undecodable_input`
- `poll_chain_sync_advance_idle_yields_none`

In `crates/ade_runtime/tests/n2n_server_two_peer_determinism.rs` (integration):
- `two_synthetic_peers_preserve_per_session_transcripts` — drive two
  peers in parallel against one orchestrator; assert each peer's
  outgoing frame sequence equals running that peer alone.

CI:
- `ci/ci_check_n2n_server_no_signing_dep.sh` (new).

---

## §14 Hard Prohibitions

- No import of `ade_runtime::producer::signing` (or anything inside)
  from `n2n_server.rs`. Enforced by the new CI gate.
- No raw socket I/O in this module — that's S7's evidence-binary
  scope. The driver returns owned `Vec<u8>` frames the caller can
  hand to a socket.
- No construction of outgoing frames except via
  `ServerReply::into_message` + the existing codec encoder.
- No HashMap iteration for per-peer state (BTreeMap keyed by peer if
  needed — but a single-peer driver suffices for this slice; the
  multi-peer test uses two independent `PerPeerN2nServerState`
  values).

---

## §15 Explicit Non-Goals

- Actual tokio socket integration — S7's binary.
- Handshake negotiation wiring — S7 plugs the negotiated version
  from existing N-A handshake into `PerPeerN2nServerState`.
- Connection lifecycle, back-pressure, timeouts — orchestrator
  scope.
- Mechanical cross-impl adapter — S7.

---

## Replay obligations

Per-peer dispatch functions are deterministic; multi-peer
independence is the new property tested here. No new replay corpus.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
