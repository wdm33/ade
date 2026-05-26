# Invariant Slice — PHASE4-N-L S6

**Slice Name:** RED `ade_runtime::network::mux_pump` — per-connection tokio task.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** part of CE-N-L-3 / CE-N-L-5 / CE-N-L-6.
**Dependencies:** S2, S3, S5.

## Intent

Bridge a `MuxTransportHandle` (S5) to the pure session reducer (S2). One tokio task per peer:
- Pulls inbound byte chunks from `transport.inbound`.
- Feeds each chunk to `session::core::step`.
- Routes `SessionEffect::SendBytes` to `transport.outbound`.
- Routes `SessionEffect::EmitOrchestratorEvent` to the orchestrator inbox channel.
- On `SessionError` or backpressure → emit `OrchestratorEvent::PeerDisconnected` + exit.

## Scope

- `crates/ade_runtime/src/network/mux_pump.rs` — new RED file.
- `crates/ade_runtime/src/network/mod.rs` — re-export.

```rust
pub struct MuxPump {
    pub peer_id: PeerId,
    pub transport: MuxTransportHandle,
    pub session_state: SessionState,
    pub events_out: mpsc::Sender<OrchestratorEvent>,
}

impl MuxPump {
    pub async fn run(mut self);
}
```

The pump owns the `SessionState`; the orchestrator core (in `ade_runtime`) holds its own per-peer dispatch state separately.

## §12 Mechanical Acceptance Criteria

- [ ] `mux_pump_round_trips_one_chain_sync_frame_over_loopback` — bind two ends, hand peer 1 a pump driving a fake handshake-Connected SessionState, write one encoded chain-sync `RequestNext` from peer 2 → observe `PeerChainSyncFrame` on orchestrator inbox.
- [ ] `mux_pump_halts_peer_on_session_error` — feed garbage bytes → `OrchestratorEvent::PeerDisconnected`.
- [ ] `mux_pump_overflow_emits_peer_halt` — saturate orchestrator inbox; pump emits `PeerDisconnected` rather than blocking.

## §14 Hard Prohibitions

- No direct call to `ade_ledger::receive::*` (route via orchestrator events).
- No HashMap.
- No `mpsc::unbounded_channel`.

## §15 Non-Goals

- No handshake driving (S4 + S7).
- No peer pooling.
