# PHASE4-N-L — Closure record

> Companion to `cluster.md`. Records the mechanical evidence
> shipped, the registry deltas applied, and the carry-forward
> obligations after closure.

## Registry deltas applied

| Rule | Change | Notes |
|------|--------|-------|
| `CN-SESS-01` | `declared → enforced` | Single mux frame authority — sole pub `encode_frame`/`decode_frame` pair in `ade_network::mux::frame`. `ci/ci_check_mux_frame_closure.sh`. |
| `CN-SESS-02` | `declared → enforced` | Single handshake authority — sole pub `n2n_transition` / `n2c_transition` in `ade_network::handshake::transition`. `ci/ci_check_handshake_closure.sh`. |
| `CN-SESS-03` | `declared → enforced` | Single session reducer authority — sole pub `step` in `ade_network::session::core`. `ci/ci_check_session_core_closure.sh`. |
| `DC-SESS-01` | `declared → enforced` | Handshake-before-traffic — `SessionState::Handshaking` cannot deliver mini-protocol frames; type-state + runtime test. `ci/ci_check_session_core_closure.sh`. |
| `DC-SESS-02` | `declared → enforced` | Closed mini-protocol id registry — `AcceptedMiniProtocol::from_id` closes with `_ => None`; dispatch site has no wildcard accept. `ci/ci_check_mini_protocol_id_registry_closed.sh`. |
| `DC-SESS-03` | `declared → enforced` | Session replay equivalence — two-run byte-identity proven via `tests/session_replay_equivalence.rs`. |
| `DC-SESS-04` | `declared → enforced` | Backpressure discipline — bounded mpsc + `TransportError::BackpressureExceeded`. `ci/ci_check_session_no_unbounded.sh`. |
| `DC-SESS-05` | `declared → enforced` | Wire-layer clock injection — session core wall-clock-free; keep-alive routes via `Clock`. `ci/ci_check_clock_seam.sh` (extended). |
| `RO-LIVE-03` | `declared` (kept) | `open_obligation = "blocked_until_operator_peer_available"`. Mechanical wire layer ready; live operator pass is a separate one-slice follow-on cluster (`PHASE4-N-L-LIVE`). |

## Family choice — ID collision note

`CN-NET-*` and `DC-NET-01` were already in use (operator-topology
rules from the classification table; `DC-NET-01` = three-tier peer
management). PHASE4-N-L's session-layer rules therefore use
`CN-SESS-*` and `DC-SESS-*` to avoid ID collision while remaining
append-only. The cluster doc, slice docs, and closure record all
reference the `*-SESS-*` IDs.

## Existing rules strengthened (`strengthened_in += "PHASE4-N-L"`)

To be applied at close (the rule deltas above already populate
`strengthened_in = ["PHASE4-N-L"]` for the newly-enforced rules):

- `T-DET-01` — byte-stream → orchestrator-event determinism now
  proven end-to-end via the session reducer.
- `CN-CONS-08` — admit path now driven by real socket bytes
  end-to-end (via mux_pump → orchestrator).
- `DC-NODE-01` — per-peer isolation now extends to the wire layer
  (each pump task owns its own `MuxTransportHandle` +
  `SessionState`; no shared mutable state).
- `DC-NODE-03` — clock-injection seam now covers keep-alive
  (KeepAliveSession is driven by `Clock`).

## Mechanical artifacts shipped

### New GREEN files (5)
- `crates/ade_network/src/session/event.rs` —
  `AcceptedMiniProtocol` closed registry, `ByteChunkIn`,
  `SessionEffect`, `SessionError`, `HandshakeRole`.
- `crates/ade_network/src/session/state.rs` — `SessionState`
  (Handshaking/Connected type-state).
- `crates/ade_network/src/session/demux.rs` — `FrameBuffer`
  partial-frame accumulator.
- `crates/ade_network/src/session/core.rs` — `step` reducer
  (CN-SESS-03).
- `crates/ade_network/src/session/handshake_driver.rs` —
  `Transport` trait + `run_n2n_handshake_initiator` /
  `run_n2n_handshake_responder`.

### New RED files (4 + 1 extended)
- `crates/ade_network/src/mux/transport.rs` (extended) —
  `spawn_duplex` + `MuxTransportHandle` + `TransportError`
  closed sum + `DuplexCapacity::DEFAULT`.
- `crates/ade_runtime/src/network/mux_pump.rs` — per-connection
  tokio task bridging `MuxTransportHandle` to the session reducer.
- `crates/ade_runtime/src/network/n2n_dialer.rs` — outbound TCP
  + handshake driver + `MuxPump` spawn + `PeerConnected` event.
- `crates/ade_runtime/src/orchestrator/keep_alive_session.rs` —
  Clock-driven ping pump.

### New integration test
- `crates/ade_network/tests/session_replay_equivalence.rs` —
  two-run byte-identity proof (DC-SESS-03).

### New `OrchestratorEvent` variant
- `OrchestratorEvent::OutboundKeepAlive { peer_id }` — emitted by
  `KeepAliveSession`; orchestrator core records it (no immediate
  effect — session-layer keep-alive frame encoding is a future
  cluster).

### New CI scripts (5)
- `ci/ci_check_mux_frame_closure.sh` — CN-SESS-01.
- `ci/ci_check_handshake_closure.sh` — CN-SESS-02.
- `ci/ci_check_session_core_closure.sh` — CN-SESS-03 + DC-SESS-01 + DC-SESS-05 (session-side).
- `ci/ci_check_mini_protocol_id_registry_closed.sh` — DC-SESS-02.
- `ci/ci_check_session_no_unbounded.sh` — DC-SESS-04.

### Existing CI script extended
- `ci/ci_check_clock_seam.sh` — now also covers `ade_network::session/`
  for the wire-side wall-clock-free guarantee (DC-SESS-05).

## Cargo dep change

`ade_network/Cargo.toml` gained `sync` + `rt-multi-thread` features
on its existing tokio dep (for `mpsc` bounded queues + `tokio::spawn`
in `mux::transport`). The session core files MUST NOT import
tokio — `ci/ci_check_session_core_closure.sh` enforces.

## Honest-scope carry-forwards

- **TLS / authenticated transport** — out of scope (declared ¬P-8
  in the invariants sketch). Cardano N2N is plain TCP on testnets;
  the curve25519 auth layer is a future cluster.
- **N2C local protocols** — out of scope.
- **Peer-sharing + tx-submission** — out of scope. Mempool
  integration (PHASE4-N-E) is a precondition for tx-submission's
  live half.
- **Live operator pass (`RO-LIVE-03`)** — the mechanical wire
  layer is ready. Running `ade_node` against a private cardano-node
  peer + capturing the JSONL log is the follow-on cluster's
  deliverable. The binary CLI now accepts
  `--peer ADDR --listen ADDR` for that purpose.

## Test summary (touched crates)

- `cargo test -p ade_network --lib session:: mux::` →
  **28 passed, 0 failed**.
- `cargo test -p ade_network --test session_replay_equivalence` →
  **2 passed, 0 failed**.
- `cargo test -p ade_runtime --lib network::mux_pump network::n2n_dialer orchestrator::keep_alive_session` →
  **7 passed, 0 failed**.
- Pre-existing N-K integration tests still green
  (`shutdown_resume_identity`, `authority_fatal_decode`,
  `orchestrator_peer_isolation`, `orchestrator_replay_equivalence`).
