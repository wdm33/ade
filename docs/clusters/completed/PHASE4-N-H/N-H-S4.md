# Invariant Slice — PHASE4-N-H S4

## Slice Header

**Slice Name:** RED per-peer N2N receive orchestrator (pure session driver)
**Cluster:** PHASE4-N-H
**Status:** In Progress
**CEs addressed:** CE-N-H-4
**Registry effects on merge:** `DC-PROTO-06.strengthened_in += "PHASE4-N-H"` (version threaded through receive-role surface).
**Dependencies:** N-H-S1..S3

---

## Intent

Mirror `ade_runtime::network::n2n_server` for the receive side:
per-peer session driver decodes inbound chain-sync + block-fetch
client-role frames, lifts via the GREEN adapter, calls
`receive_apply`. Pure (no socket I/O); the socket layer is operator-
action territory in S6.

Multi-peer determinism: per-peer receive state is independent; the
single shared `ChainDb` is the cross-peer coordination point. Two
peers receiving the same block both admit successfully (idempotent
`put_block` at byte-identity).

Key-boundary: receive orchestrator must NOT import from
`producer::signing`, `producer::broadcast`, or `producer::scheduler`.

---

## The change

### 1. New `crates/ade_runtime/src/receive/orchestrator.rs`

```rust
pub struct PerPeerReceiveState {
    pub receive_state: ReceiveState,
    pub chain_sync_version: ChainSyncVersion,
    pub block_fetch_version: BlockFetchVersion,
}

pub enum ReceiveDispatchError {
    ChainSyncDecode(CodecError),
    BlockFetchDecode(CodecError),
    Receive(ReceiveError),
    /// Frame decoded but the corresponding event was not state-
    /// changing (e.g. BatchStarted). Orchestrator treats this as a
    /// no-op; surfaced as a distinct error so the caller can log.
    /// Optional refinement; for now we just return Ok(NoOp).
    Filtered,
}

pub fn dispatch_chain_sync_inbound<W: ChainDbWrite>(
    state: &mut PerPeerReceiveState,
    frame: &[u8],
    chain_write: &mut W,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Option<ReceiveEffect>, ReceiveDispatchError>;

pub fn dispatch_block_fetch_inbound<W: ChainDbWrite>(...)
    -> Result<Option<ReceiveEffect>, ReceiveDispatchError>;
```

### 2. CI gate `ci/ci_check_receive_orchestrator_no_producer_dep.sh`

Forbids any import of `producer::signing` / `producer::broadcast` /
`producer::scheduler` from `ade_runtime::receive::*`.

### 3. Multi-peer integration test
`crates/ade_runtime/tests/receive_two_peer_independence.rs`

Two peers, shared `InMemoryChainDb`. Both receive the same corpus
block via RollForward + BlockDelivered. Assert both peers see
`Admitted` (the second peer's `put_block` is idempotent on
byte-identity). Per-peer transcripts equal their solo runs.

---

## §12 Mechanical Acceptance Criteria (named tests)

In `crates/ade_runtime/src/receive/orchestrator.rs`:
- `dispatch_chain_sync_inbound_decodes_then_caches` (RollForward).
- `dispatch_block_fetch_inbound_decodes_then_admits` (BlockDelivered).
- `dispatch_chain_sync_inbound_threads_negotiated_version` — version
  is read from per-peer state, not a global.
- `dispatch_rejects_undecodable_input` — decode failure → typed
  error.
- `dispatch_filters_non_state_changing_events` — Intersected /
  BatchStarted yield `Ok(None)`.

In `crates/ade_runtime/tests/receive_two_peer_independence.rs`:
- `two_peers_admit_same_block_into_shared_chaindb`.
- `two_peers_per_session_transcripts_match_solo_runs`.

CI: `ci/ci_check_receive_orchestrator_no_producer_dep.sh` (new).

---

## §14 Hard Prohibitions

- No imports from `producer::signing`, `producer::broadcast`,
  `producer::scheduler` anywhere under `ade_runtime::receive::*`.
- No `tokio` sockets in the orchestrator — pure state-driver. The
  socket layer is S6's binary.
- No `unwrap`/`expect`/`panic!` in production code.

---

## §15 Explicit Non-Goals

- Actual tokio socket wiring — S6's evidence-binary scope.
- Handshake negotiation wiring — S6 plugs the negotiated version
  from N-A handshake into `PerPeerReceiveState`.
- Fork choice / multi-peer chain selection — future cluster.

---

## Replay obligations

The orchestrator is RED; replay-equivalence is proven at the BLUE
reducer + GREEN adapter layer (S3). The multi-peer test pins the
independence property explicitly.

---

## Authority reminder

If this slice conflicts with the project's normative specifications
or the invariant registry, those win.
