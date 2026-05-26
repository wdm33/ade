# Invariant Slice — PHASE4-N-L S2

**Slice Name:** GREEN `session::{core, event, state}` — pure session reducer with type-state.
**Cluster:** PHASE4-N-L
**Status:** In Progress
**CEs addressed:** CE-N-L-3 (CN-SESS-03 + DC-SESS-01).
**Registry effects on merge:**
- CN-SESS-03 → `enforced` (`ci/ci_check_session_core_closure.sh`).
- DC-SESS-01 → `enforced` (type-state + runtime test).
**Dependencies:** S1.

## Intent

Make session evolution a single pure reducer. Type-state forbids frame delivery before handshake; the closed `SessionEffect` sum forbids effects outside the declared surface.

## Scope

- `crates/ade_network/src/session/state.rs` — `SessionState` (Handshaking + Connected branches).
- `crates/ade_network/src/session/event.rs` — `ByteChunkIn`, `SessionEffect`, `SessionError`, `PeerHaltReason` (extended).
- `crates/ade_network/src/session/core.rs` — `step(state, ByteChunkIn) -> Result<(state', Vec<SessionEffect>), SessionError>`.
- `ci/ci_check_session_core_closure.sh` — single pub `step`; no tokio/SystemTime/Instant in `session/*`.

## Design

```rust
pub enum SessionState {
    Handshaking(HandshakeProgress),
    Connected(ConnectedState),
}

pub struct HandshakeProgress {
    inner: handshake::state::HandshakeState,
    buffer: FrameBuffer, // pre-handshake bytes accumulator
}

pub struct ConnectedState {
    negotiated_version: u16,
    buffer: FrameBuffer,
    next_outbound_seq: u64,
}

pub enum ByteChunkIn {
    Inbound(Vec<u8>),
    OutboundFrame { mini_protocol: AcceptedMiniProtocol, payload: Vec<u8>, mode: MuxMode, timestamp: u32 },
    HandshakeStart { our_versions: VersionTable, role: HandshakeRole },
}

pub enum SessionEffect {
    SendBytes(Vec<u8>),
    EmitOrchestratorEvent(OrchestratorEvent),
    HandshakeComplete { version: u16 },
    PeerHalted(PeerHaltReason),
}

pub enum SessionError {
    UnknownMiniProtocolId { id: u16 },
    PreHandshakeMiniProtocolFrame { id: u16 },
    HandshakeFrameRejected,
    Mux(MuxError),
    Handshake(HandshakeError),
}

pub fn step(
    state: &mut SessionState,
    event: ByteChunkIn,
) -> Result<Vec<SessionEffect>, SessionError>;
```

Routing:
- `Handshaking + Inbound(bytes)` → accumulate; on full frame, decode via `decode_frame` → must be id=0 (handshake); else `PreHandshakeMiniProtocolFrame`. Run `n2n_transition`. On Accepted, transition to `Connected` and emit `HandshakeComplete`.
- `Connected + Inbound(bytes)` → accumulate; for each complete frame, demux by id; unknown → `UnknownMiniProtocolId`; known → emit `OrchestratorEvent::Peer{ChainSync|BlockFetch|...}Frame`.
- `OutboundFrame` → encode via `encode_frame` → emit `SendBytes`.
- `HandshakeStart` → encode proposal via handshake codec → emit `SendBytes`.

## §12 Mechanical Acceptance Criteria

- [ ] `session_blocks_frames_before_handshake` — Handshaking state + a chain-sync-id frame → `PreHandshakeMiniProtocolFrame` error.
- [ ] `session_step_two_runs_byte_identical` — same byte-chunk sequence → identical effects across two runs.
- [ ] `session_unknown_mini_protocol_id_is_peer_fatal` — unknown id → `SessionError::UnknownMiniProtocolId`.
- [ ] `session_handshake_completion_transitions_state` — running `n2n_transition` to Accepted moves SessionState into Connected.
- [ ] `session_outbound_frame_encodes_via_encode_frame` — `OutboundFrame` event → bytes that decode back via `decode_frame`.
- [ ] `ci/ci_check_session_core_closure.sh` passes.

## §14 Hard Prohibitions

- No tokio / SystemTime / Instant / rand in `session/*`.
- No HashMap / HashSet (BTreeMap if needed).
- No bypass of `mux::frame::{encode,decode}_frame` for any byte emission.
- No reimplementation of `n2n_transition`.
- No second pub `step` function in `session/*`.

## §15 Non-Goals

- No real socket I/O. No outbound dialer. No keep-alive scheduling.
- No actual chain-sync / block-fetch decode in this slice — frames are forwarded to the orchestrator as `PeerChainSyncFrame`/`PeerBlockFetchFrame` events, which the orchestrator already routes to the BLUE codecs.
