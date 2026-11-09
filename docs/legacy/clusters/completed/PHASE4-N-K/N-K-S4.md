# Invariant Slice — PHASE4-N-K S4

## Slice Header

**Slice Name:** RED `orchestrator::peer_session` — per-peer
receive task + multi-peer dispatch.
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** CE-N-K-2 (DC-NODE-01).
**Registry effects on merge:** DC-NODE-01 → `enforced` with
`code_locus`, `tests`, `ci_script`, `strengthened_in =
["PHASE4-N-K"]`.
**Dependencies:** S2 (orchestrator core).

---

## Intent

A failing peer never poisons the orchestrator. Per-peer state is
held in `BTreeMap` keyed by `PeerId`; one peer's decode error,
validity reject, or socket drop removes only that peer's entry
and emits `PeerSessionHalted`. Other peers and the producer keep
running.

---

## Scope

- `crates/ade_runtime/src/orchestrator/peer_session.rs` — RED
  channel-driven task wrapper.
- `crates/ade_runtime/src/orchestrator/mod.rs` — re-export.

`PeerSession` is a thin tokio task that:
- Owns a `tokio::sync::mpsc::Receiver<PeerInboundFrame>` (frames
  from the socket layer).
- Forwards each frame to the orchestrator as a
  `PeerChainSyncFrame` / `PeerBlockFetchFrame` event.
- Returns deterministically when the socket closes or the
  orchestrator emits `PeerSessionHalted`.

Per-peer state isolation is enforced at the core level (S2):
`OrchestratorState::per_peer_receive` is keyed by `PeerId`. This
slice adds the RED adapter that makes the per-peer claim
observable under tokio.

---

## Execution Boundary

- **BLUE:** none.
- **GREEN:** unchanged.
- **RED:** `orchestrator::peer_session`.

---

## Invariants Preserved

- All from S2.
- N-H receive-side authority (CN-CONS-08 admit path) unchanged.

## Invariants Strengthened or Introduced

- DC-NODE-01 (this slice introduces).

---

## Design Summary

```rust
pub struct PeerInboundFrame {
    pub kind: PeerInboundFrameKind,
    pub bytes: Vec<u8>,
}

pub enum PeerInboundFrameKind {
    ChainSyncClient,        // from upstream peer (receive-side)
    BlockFetchClient,
    ChainSyncServer,        // from downstream peer (producer-served)
    BlockFetchServer,
}

pub struct PeerSession {
    pub peer_id: PeerId,
    pub inbound: mpsc::Receiver<PeerInboundFrame>,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
}

impl PeerSession {
    pub async fn run(mut self);
}
```

The task body is a `select!` over `inbound.recv()` and a
shutdown notification; on each inbound frame it constructs the
matching `OrchestratorEvent` and sends to `events_out`. Errors
on `events_out.send()` mean the orchestrator dropped — task
exits.

---

## Replay, Crash, and Epoch Validation

- **Tests:**
  - `peer_session_routes_chain_sync_to_orchestrator_event` —
    feed one frame, observe one event with the right `PeerId`.
  - `peer_session_isolation_holds_under_failure` — two peers,
    A and B; inject a malformed frame from A. Step the
    orchestrator until both peers' frames are processed; assert
    A is removed (`PeerSessionHalted`), B continues, and the
    global `ReceiveState` reflects only B's progress.
  - `peer_session_per_peer_state_does_not_cross` —
    `OrchestratorState::per_peer_receive` keys are distinct;
    a peer-A `RollForward` does not appear in peer B's pending
    header cache.

## §12 Mechanical Acceptance Criteria

- [ ] `peer_session_routes_chain_sync_to_orchestrator_event`
- [ ] `peer_session_isolation_holds_under_failure`
- [ ] `peer_session_per_peer_state_does_not_cross`
- [ ] `peer_session_disconnect_removes_per_peer_map_entry`
- [ ] `ci_check_peer_session_no_blue_import.sh` — the file
  imports only from `crate::orchestrator`, `tokio::sync::mpsc`,
  and standard library. No direct dependency on
  `ade_ledger::receive::*` or `ade_network::codec::*`
  (those flow through orchestrator events).

---

## §14 Hard Prohibitions

- No `unwrap()` / `expect()` / `panic!()` in non-test code.
- No direct call to `dispatch_chain_sync_inbound` /
  `dispatch_block_fetch_inbound` from this file — they're
  called by the orchestrator core when it processes the event.
- No shared mutable state between peer tasks (each peer task
  owns its `mpsc::Receiver`; no `Arc<Mutex<…>>` over per-peer
  fields).

## §15 Explicit Non-Goals

- No actual TCP socket layer (operator-action live-evidence work).
- No handshake driver (S6 covers the listening side; the
  client-side dialer is not in this cluster — operator pass).
- No mempool subscription.

---

## §16 Completion Checklist

- [ ] All §12 tests added and passing.
- [ ] CI gate passes.
- [ ] Registry DC-NODE-01 flipped to `enforced`.
