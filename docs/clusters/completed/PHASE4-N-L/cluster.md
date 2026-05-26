# Cluster PHASE4-N-L — Wire protocol (mux session driver + handshake)

> **Status:** Planning artifact (non-normative). Introduces
> `CN-SESS-01/02/03` + `DC-SESS-01..05` + `RO-LIVE-03` as enforced
> (RO-LIVE-03 stays `blocked_until_operator_peer_available`).
> Strengthens `T-DET-01`, `CN-CONS-08`, `DC-NODE-01`, `DC-NODE-03`.

## Primary invariant

> The `ade_node` binary follows a real cardano-node peer
> end-to-end over the wire by composing the existing BLUE
> authorities — `mux::frame::{encode,decode}_frame` (CN-SESS-01),
> `handshake::n2n_transition` (CN-SESS-02), and the per-mini-protocol
> state machines from PHASE4-N-A — through a single GREEN session
> reducer (`session::core::step`, CN-SESS-03). Handshake completes
> before any frame is delivered to the orchestrator (DC-SESS-01);
> the mini-protocol id space is a closed registry (DC-SESS-02);
> ordering is preserved per (peer, mini_protocol_id) (DC-SESS-03);
> per-peer queues are bounded with fail-fast overflow (DC-SESS-04);
> no wall-clock or rand reaches the GREEN session core (DC-SESS-05).

## Scope

- **GREEN (new):**
  - `ade_network::session::core` — pure session reducer
    (`SessionState` type-state + `step`).
  - `ade_network::session::event` — closed `ByteChunkIn` /
    `SessionEffect` / `SessionError` sums + `AcceptedMiniProtocol`
    closed enum.
  - `ade_network::session::demux` — partial-frame buffer +
    per-protocol fanout.
  - `ade_network::session::handshake_driver` — pure driver over
    opaque `Transport` trait.
- **RED (new):**
  - `ade_network::mux::transport` (extended) — full-duplex
    bounded-queue reader + writer.
  - `ade_runtime::network::mux_pump` — per-connection tokio task.
  - `ade_runtime::network::n2n_dialer` — outbound TCP +
    handshake-driver call → orchestrator `PeerConnected`.
  - `ade_runtime::orchestrator::keep_alive_session` — Clock-driven
    ping pump.
- **BLUE:** unchanged.

Out-of-scope (declared): TLS / authenticated transport, N2C local
protocols, peer-sharing, tx-submission, mempool integration.

## Grounding (verified at HEAD `d62c2bc`)

- **`ade_network::mux::frame::{encode_frame, decode_frame}`** —
  BLUE 8-byte-header codec; pure; `MuxError` is a closed sum.
- **`ade_network::handshake::{n2n_transition,
  select_n2n_version}`** — BLUE handshake state machine + version
  selection; `HandshakeError` is a closed sum.
- **`ade_network::{chain_sync, block_fetch, keep_alive}`** —
  BLUE per-protocol reducers (codec + state machine + agency).
- **`ade_runtime::clock::Clock`** — PHASE4-N-K seam reused for
  keep-alive cadence.
- **`ade_runtime::orchestrator::event::{OrchestratorEvent,
  OrchestratorEffect}`** — PHASE4-N-K closed event vocabulary;
  new `PeerHaltReason` variants added in S1.

## Slice index

| Slice | Scope | TCB |
|-------|-------|-----|
| S1 | CI gates for CN-SESS-01 (mux frame) + CN-SESS-02 (handshake) + closed `AcceptedMiniProtocol` enum (DC-SESS-02). | CI + GREEN |
| S2 | GREEN `session::{core, event, state}` — pure reducer with `Handshaking`/`Connected` type-state. CN-SESS-03 + DC-SESS-01. | GREEN |
| S3 | GREEN `session::demux` — partial-frame buffer, per-protocol fanout. | GREEN |
| S4 | GREEN `session::handshake_driver` — pure driver over `Transport` trait. | GREEN |
| S5 | RED `ade_network::mux::transport` extended — full-duplex bounded-queue async loop. DC-SESS-04. | RED |
| S6 | RED `ade_runtime::network::mux_pump` — per-connection tokio task. | RED |
| S7 | RED `ade_runtime::network::n2n_dialer` — outbound TCP + handshake driver. | RED |
| S8 | RED `keep_alive_session` — Clock-driven ping pump. DC-SESS-05. | RED |
| S9 | Replay-equivalence harness — recorded byte-chunk transcript. DC-SESS-03. | GREEN + test |

Dependencies: S2..S4 depend on S1; S5 depends on S1; S6 depends on S2/S3/S5; S7 depends on S4/S6; S8 depends on S6 + PHASE4-N-K Clock; S9 depends on S2/S3.

## Exit criteria (CI-verifiable)

- [ ] **CE-N-L-1 (CN-SESS-01)** — `ci/ci_check_mux_frame_closure.sh`
  asserts a single pub `encode_frame`/`decode_frame` pair in the
  workspace.
- [ ] **CE-N-L-2 (CN-SESS-02)** — `ci/ci_check_handshake_closure.sh`
  asserts a single pub `n2n_transition` and a single pub
  `n2c_transition` across the workspace.
- [ ] **CE-N-L-3 (CN-SESS-03 + DC-SESS-01)** — `ci/ci_check_session_core_closure.sh`
  asserts `session::core::step` is the only pub reducer in
  `session/`, and the `Handshaking`/`Connected` type-state split
  is structurally present.
- [ ] **CE-N-L-4 (DC-SESS-02)** —
  `ci/ci_check_mini_protocol_id_registry_closed.sh` asserts the
  `AcceptedMiniProtocol` enum is closed and the dispatch table
  is a `match` over it (no wildcard accept).
- [ ] **CE-N-L-5 (DC-SESS-03)** —
  `session_replay_equivalence_holds` proves byte-identical
  effects across two replays of the same byte-chunk corpus.
- [ ] **CE-N-L-6 (DC-SESS-04)** —
  `ci/ci_check_session_no_unbounded.sh` asserts no
  `mpsc::unbounded_channel` in session / mux_pump / n2n_dialer
  files; integration test asserts overflow emits
  `PeerHaltReason::BackpressureExceeded`.
- [ ] **CE-N-L-7 (DC-SESS-05)** — `ci/ci_check_clock_seam.sh`
  (extended) asserts the GREEN session core contains no
  `SystemTime` / `Instant` / `tokio::time` reads; keep-alive
  cadence routes through `ade_runtime::clock::Clock`.

> No human review may substitute for these checks.

## TCB color map (FC/IS partition)

- **BLUE:** unchanged.
- **GREEN:**
  - `crates/ade_network/src/session/core.rs`
  - `crates/ade_network/src/session/event.rs`
  - `crates/ade_network/src/session/state.rs`
  - `crates/ade_network/src/session/demux.rs`
  - `crates/ade_network/src/session/handshake_driver.rs`
  - `crates/ade_network/src/session/mod.rs`
- **RED:**
  - `crates/ade_network/src/mux/transport.rs` (extended)
  - `crates/ade_runtime/src/network/mux_pump.rs`
  - `crates/ade_runtime/src/network/n2n_dialer.rs`
  - `crates/ade_runtime/src/orchestrator/keep_alive_session.rs`

Rules:
- No RED behavior in any GREEN session file.
- No BLUE reimplementation in the session core; only composition.

## Forbidden during this cluster

- No `mpsc::unbounded_channel` / `unbounded`-named queue
  constructors in any session, mux_pump, n2n_dialer, or
  keep_alive_session file.
- No `SystemTime::now()` / `Instant::now()` / `tokio::time::*` in
  any GREEN session file. Keep-alive cadence MUST route through
  `ade_runtime::clock::Clock`.
- No bypass of `mux::frame::{encode,decode}_frame` for outbound
  or inbound bytes.
- No parallel handshake reducer; the session driver calls
  `handshake::n2n_transition`.
- No silent acceptance of unknown `MiniProtocolId` — the dispatch
  table is a closed `match` on `AcceptedMiniProtocol`.
- No TLS / cryptographic peer-identity logic in this cluster
  (declared ¬P-8; future cluster).
- No new mini-protocol implementations (the BLUE state machines
  ship at HEAD already).

## Replay obligations introduced

- New canonical replay surface: byte-chunk → orchestrator-event
  stream (DC-SESS-03). Corpus at `corpus/n2n_session/`.
- `T-DET-01.strengthened_in += "PHASE4-N-L"` (byte-stream →
  event-stream determinism).
- `CN-CONS-08.strengthened_in += "PHASE4-N-L"` (admit path now
  driven by real socket bytes end-to-end through the production
  binary).
- `DC-NODE-01.strengthened_in += "PHASE4-N-L"` (per-peer
  isolation extended to the wire layer).
- `DC-NODE-03.strengthened_in += "PHASE4-N-L"` (clock-injection
  seam now covers keep-alive).

## Open obligations carried after closure

- `RO-LIVE-03` —
  `open_obligation = "blocked_until_operator_peer_available"`.
  The mechanical wire layer is ready; the live operator pass is
  a separate one-slice follow-on cluster (`PHASE4-N-L-LIVE`).
- `RO-LIVE-01`, `RO-LIVE-02`, `CN-CONS-06` carry through
  unchanged — same operator-action work, now actually runnable.

## Authority reminder

This document is a planning aid only. Correctness rules live in
`docs/ade-invariant-registry.toml`. If there is ever a conflict:

> **Normative documents + CI enforcement win.**
