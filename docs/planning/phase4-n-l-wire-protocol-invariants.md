# PHASE4-N-L — Wire-protocol (mux session driver + handshake) — invariants sketch

## Framing

PHASE4-N-L wires the existing BLUE mini-protocol state machines and
mux frame codec into a real Ouroboros-N2N session over TCP. The
cluster closes the gap that PHASE4-N-K explicitly deferred: the
mux session driver above `ade_network::mux::MuxTransport` that
turns raw socket bytes into `PeerInboundFrame`s on the orchestrator
inbox.

**PHASE4-N-L introduces no BLUE authority. The session driver is
GREEN: deterministic composition of existing BLUE codecs +
handshake + per-protocol state machines over canonical byte
input. The tokio I/O layer, the TCP dialer, and the wall-clock for
keep-alive are RED.**

That split mirrors PHASE4-N-K's discipline: the session reducer
is replay-equivalent under a deterministic byte stream; only the
I/O wrapper is nondeterministic.

After this cluster, `ade_node` can FOLLOW a real cardano-node
peer end-to-end. Block production over a real peer (`CN-CONS-06`
live half) and the live-evidence obligations (`RO-LIVE-01`,
`RO-LIVE-02`) become operator-action work — running `ade_node`
against a private peer and capturing the log.

Predecessor anchors (HEAD `d62c2bc`): PHASE4-N-A (mini-protocol
state machines), PHASE4-N-G (server response paths),
PHASE4-N-H (receive admit), PHASE4-N-K (orchestrator + binary).

## 1. What must always be true

- **I-1 Handshake-before-traffic.** No mini-protocol frame
  reaches the orchestrator inbox until the handshake state
  machine has emitted `N2nHandshakeOutput::Accepted` (or
  `N2cHandshakeOutput::Accepted` for n2c). Type-state: the
  session driver's `Connected` state is a different type from
  `Handshaking`, and only `Connected` can deliver frames.
- **I-2 Closed mini-protocol id registry.** The set of accepted
  `MiniProtocolId` values is a closed sum at the dispatch site.
  Unknown ids are peer-fatal (`PeerHaltReason::UnknownMiniProtocolId`),
  never silently dropped, never silently accepted.
- **I-3 Per-mini-protocol ordering preserved.** Within a single
  `(peer, mini_protocol_id)` pair, bytes the peer sent in order
  reach the per-protocol state machine in the same order. No
  reordering inside the session driver.
- **I-4 Per-peer isolation at the wire layer.** A mux-frame
  decode error on peer A's socket halts only peer A's session.
  Sibling peer tasks and the producer continue. Mirrors
  PHASE4-N-K's DC-NODE-01 at the wire layer.
- **I-5 Mux frame timestamp is caller-supplied.** The existing
  `MuxFrame.header.timestamp: u32` field stays caller-supplied;
  the session driver MUST NOT call `Instant::now()` from a
  GREEN file. Timestamp injection sites are restricted to the
  RED encoder wrapper (one site) — enforced by CI grep.
- **I-6 Backpressure honored end-to-end.** The wire reader does
  not drain bytes faster than the per-protocol bounded queue
  can consume; the orchestrator inbox is bounded; queue overflow
  is fail-fast (`PeerHaltReason::BackpressureExceeded`) rather
  than silent drop.
- **I-7 Outbound frame ordering is deterministic.** Effects emitted
  by `orchestrator::core::step` that map to `SendToPeer { peer_id,
  bytes }` reach the socket in the order the orchestrator emitted
  them.

## 2. What must never be possible

- **¬P-1** Mini-protocol frame delivery before handshake completes.
- **¬P-2** Silent acceptance of an unknown `MiniProtocolId`. Closed
  registry; unknown id → peer-fatal.
- **¬P-3** Cross-peer state mutation at the wire layer. One peer's
  mux state cannot mutate another's; the wire layer holds no shared
  per-peer buffer.
- **¬P-4** Wall-clock reads from any GREEN session-driver file.
  Timestamps enter through the RED encoder wrapper, not the core.
- **¬P-5** Unbounded buffering. Every per-peer + per-protocol
  channel is bounded; overflow is fail-fast with a closed reason.
- **¬P-6** Parallel handshake state machines. The N2N handshake
  authority is the existing `handshake::n2n_transition` /
  `select_n2n_version`; the session driver composes these, never
  reimplements them.
- **¬P-7** Bypass of `MuxFrame::{encode,decode}`. Every outbound
  byte sequence the session writes to a socket is the output of
  `encode_frame`; every inbound demux happens through
  `decode_frame`.
- **¬P-8** TLS/auth deferral leakage. If TLS is deferred (decision
  in §7), the session driver must not pretend to authenticate;
  the peer-identity field stays a network-address record only,
  not a cryptographic identity.

## 3. What must remain identical across executions

- Replaying a recorded byte stream through the session driver
  (pure reducer over byte chunks → mux frames → orchestrator
  events) yields byte-identical outbound frames and a
  byte-identical sequence of `OrchestratorEvent`s.
- Handshake completion: same `(supported_versions, peer_proposal)`
  produces the same `Accepted { version, params }` deterministically.
- Mini-protocol dispatch ordering: same inbound byte sequence →
  same per-protocol message sequence.

## 4. What must be replay-equivalent

- The session driver's `SessionStep` reducer — `(state,
  ByteChunkIn) → Result<(state', Vec<SessionEffect>), SessionError>`
  — replayed against a frozen handshake transcript + frame corpus
  produces byte-identical effects.
- Outbound frame encoding determinism: given the same `(seq, payload)`
  the encoder produces the same bytes (already true by `encode_frame`'s
  purity, but the wrapper must not inject randomness).

## 5. State transitions in scope

All GREEN session core + RED tokio I/O. BLUE codecs unchanged.

```text
session_core::step(state, ByteChunkIn)
  -> Result<(state', Vec<SessionEffect>), SessionError>
  // composes mux::decode_frame + per-mini-protocol dispatch
  // (chain_sync / block_fetch / keep_alive / handshake)
  // per buffered chunk

handshake_driver::run(transport, our_version_table)
  -> Result<(NegotiatedN2n, Transport), HandshakeError>
  // RED wrapper around handshake::n2n_transition that owns the
  // socket for the handshake window only; hands the transport
  // to session_core on completion

mux_session::run(peer_id, transport, events_out)
  -> Result<(), SessionRunError>
  // RED tokio task: reads from transport, feeds session_core,
  // forwards SessionEffect::EmitOrchestratorEvent on events_out,
  // writes SessionEffect::SendBytes to transport
```

## 6. TCB color hypothesis

- **GREEN (new):**
  - `ade_network::session::core` — pure `SessionState` +
    `SessionStep::step` reducer.
  - `ade_network::session::event` — closed `ByteChunkIn` /
    `SessionEffect` / `SessionError` / `PeerHaltReason` extension.
  - `ade_network::session::demux` — closed mini-protocol id
    registry + dispatch table.
  - `ade_network::session::handshake_driver` (the pure half — drives
    `n2n_transition` over an opaque transport trait).
- **RED (new):**
  - `ade_network::mux::transport` (extended) — full duplex read +
    write loop, bounded queue management.
  - `ade_runtime::network::n2n_dialer` — outbound TCP +
    handshake driver wiring → orchestrator inbox.
  - `ade_runtime::network::mux_pump` — per-connection RED tokio
    task that owns the socket and drives `session_core::step`.
  - `ade_runtime::orchestrator::keep_alive_session` — RED
    periodic ping task driven by `Clock::next_tick`.
- **BLUE:** unchanged.

## 7. Decisions on framing questions

| # | Question | Recommendation |
|---|----------|----------------|
| 1 | TLS / authenticated transport | **Defer.** Cardano N2N is plain TCP by default on testnet; mainnet uses a curve25519 layer for some configurations. Defer the auth layer; declare "peer identity is network-address only" as a closed limit (¬P-8) and record an `open_obligation` on the new `RO-LIVE-03` (or strengthen `RO-LIVE-01/02`). |
| 2 | Crate hosting | Put the GREEN session core in `ade_network::session::core` (the existing empty placeholder). Put the RED tokio pumps in `ade_runtime::network` (mirrors PHASE4-N-G's `n2n_server` placement). |
| 3 | Outbound dialer scope | Ship N2N-client (outbound, follow-a-peer). N2C local protocols are out of scope — operator-tool surface for a later cluster. |
| 4 | Bounded queue sizing | Pin defaults (1024 inbound frames per peer, 256 outbound) as constants in the cluster doc; operator-tunable cadence is forbidden (mirrors `SnapshotCadence::DEFAULT` discipline). |
| 5 | Close RO-LIVE-01 / RO-LIVE-02 / CN-CONS-06? | **No.** This cluster makes the live pass MECHANICALLY runnable; the actual live-pass log capture is still operator-action work (separate one-slice cluster). |
| 6 | Keep-alive cadence | Use a fixed default (e.g. 60s) shipped as `KeepAliveCadence::DEFAULT`; operator-tunable forbidden in this cluster (Tier 5 future). |
| 7 | Peer-sharing protocol | **Defer.** Ship chain-sync + block-fetch + keep-alive + handshake only. Peer-sharing + tx-submission are future clusters (mempool integration depends on the latter). |
| 8 | Replay corpus | Recorded byte-chunk transcript from a single live peer-follow window — frozen under `corpus/n2n_session/`. Closes DC-SESS-03 replay equivalence. |
| 9 | Cold-key authentication | N/A for client-only session. Server-side opcert/KES wiring is the existing producer surface; this cluster doesn't touch it. |

## 8. Registry deltas (planned at /cluster-plan)

### New families
- **DC-NET** — derived constraints on the wire-session surface
  (handshake-before-traffic, closed mini-protocol id registry,
  per-mini-protocol ordering, backpressure discipline, clock-injection
  on keep-alive).
- **CN-NET** — single-authority closures on the wire-session
  surface (mux frame encode/decode, handshake transition, session
  reducer).

### New rules (all `*-01` style; status `declared` until slice-close flips)

- **CN-SESS-01** — Single mux frame authority. `mux::frame::{encode,decode}_frame`
  is the SOLE pub fn pair encoding/decoding `MuxFrame` to/from
  bytes in the workspace. (Mirrors CN-STORE-08.) **Likely already
  true at HEAD — slice S1 just adds the CI grep gate.**
- **CN-SESS-02** — Single handshake authority. `handshake::n2n_transition`
  is the SOLE pub fn driving the N2N handshake state machine.
  (Likely already true at HEAD — slice S1 adds the gate.)
- **CN-SESS-03** — Single session step authority. `session::core::step`
  is the SOLE pub fn reducing `(SessionState, ByteChunkIn) ->
  (SessionState, Vec<SessionEffect>)`. New.
- **DC-SESS-01** — Handshake-before-traffic. Type-state: a
  `MuxSession` in `Handshaking` cannot emit `EmitOrchestratorEvent::
  PeerFrame`; only `Connected` can. Compile-time guarantee +
  runtime test (`session_blocks_frames_before_handshake`).
- **DC-SESS-02** — Closed mini-protocol id registry. The dispatch
  table is a closed `match` over a closed
  `AcceptedMiniProtocol` enum; unknown ids return
  `SessionError::UnknownMiniProtocolId { id }` (peer-fatal).
- **DC-SESS-03** — Per-mini-protocol ordering + session replay
  equivalence. Replaying the same byte chunks yields byte-identical
  outbound frames + identical orchestrator event sequence.
  CI script: `ci_check_session_replay_purity.sh`.
- **DC-SESS-04** — Backpressure discipline. Bounded queues
  everywhere; overflow is fail-fast `PeerHaltReason::BackpressureExceeded`.
- **DC-SESS-05** — Wire layer clock injection. The session
  reducer + dispatch table contain no `SystemTime` / `Instant::now`
  / `tokio::time` reads. Keep-alive is driven by `Clock`
  (PHASE4-N-K's seam, reused). CI extends `ci_check_clock_seam.sh`.

### New live-evidence obligation
- **RO-LIVE-03** — Live tip-following pass: operator runs
  `ade_node --peer ADDR` against a private cardano-node, captures
  a 30-minute JSONL log of `(peer_tip, ade_tip, agreement_verdict)`
  per admitted block, attaches the log to the cluster doc.
  `open_obligation = "blocked_until_operator_peer_available"`
  at append; the operator-action follow-on cluster
  (`PHASE4-N-L-LIVE`) closes it.

### Existing-rule updates
- `RO-LIVE-01`, `RO-LIVE-02`, `CN-CONS-06` — note that this
  cluster MAKES the live pass runnable but DOES NOT close them.
  Cross-ref bump only.
- `DC-NODE-01` — `strengthened_in += "PHASE4-N-L"` (per-peer
  isolation now extends to the wire layer).
- `DC-NODE-03` — `strengthened_in += "PHASE4-N-L"` (clock-injection
  seam now covers keep-alive).

### Strengthenings recorded at cluster close
- `T-DET-01.strengthened_in += "PHASE4-N-L"` — replay equivalence
  now covers the byte-stream → orchestrator-event path.
- `CN-CONS-08.strengthened_in += "PHASE4-N-L"` — admit path is
  now driven by real socket bytes end-to-end.

## 9. Slice shape (proposed; refine at `/cluster-plan`)

| Slice | Scope | TCB |
|-------|-------|-----|
| S1 | CI gates pin existing CN-SESS-01 / CN-SESS-02 (mux frame + handshake single-authority). Closed `AcceptedMiniProtocol` enum + dispatch table (DC-SESS-02). | GREEN + CI |
| S2 | GREEN `session::core::{state, event, step}` — pure byte-chunk → effects reducer. Type-state Handshaking/Connected split (DC-SESS-01). | GREEN |
| S3 | GREEN `session::demux` — frame buffering, partial-frame accumulation, per-mini-protocol fanout. Pure. | GREEN |
| S4 | GREEN `session::handshake_driver` — pure handshake state-machine driver over an opaque `Transport` trait. CN-SESS-02 + DC-SESS-01 type-level. | GREEN |
| S5 | RED `ade_network::mux::transport` extended — full duplex async reader + writer with bounded queues (DC-SESS-04). | RED |
| S6 | RED `ade_runtime::network::mux_pump` — per-connection tokio task wiring `session::core::step` to a real `MuxTransport`, forwards `SessionEffect`s to the orchestrator inbox. | RED |
| S7 | RED `ade_runtime::network::n2n_dialer` — TCP outbound + handshake-driver call → orchestrator `PeerConnected` event. | RED |
| S8 | GREEN+RED `keep_alive_session` — Clock-driven ping pump (DC-SESS-05 reuses PHASE4-N-K's Clock seam). | RED |
| S9 | Replay-equivalence harness — recorded byte-chunk transcript at `corpus/n2n_session/`; closes DC-SESS-03. | test |

Dependencies: S2..S4 depend on S1; S5 depends on the closed mini-protocol enum from S1; S6 depends on S2/S3/S5; S7 depends on S4/S6; S8 depends on S6 + PHASE4-N-K Clock; S9 depends on S2/S3.

## 10. Honest-scope carry-forward

- **TLS / authenticated transport** — out of scope; declared as
  ¬P-8. A follow-on cluster (PHASE4-N-M auth?) lands the
  curve25519 wrapper when needed.
- **N2C local protocols** — out of scope; operator-tool surface
  for a later cluster.
- **Peer-sharing + tx-submission** — out of scope. Mempool
  integration (PHASE4-N-E) is a precondition for tx-submission's
  live half.
- **Live operator pass** — `RO-LIVE-03` is the new
  `blocked_until_operator_peer_available` obligation; closing
  it is the one-slice `PHASE4-N-L-LIVE` follow-on cluster
  (operator runs the binary, captures the log).

## 11. Why this is the right next cluster

PHASE4-N-K shipped the orchestrator + binary; the binary today
bootstraps and idles. PHASE4-N-L is the smallest cluster that
turns "binary that bootstraps" into "binary that follows a
real cardano-node peer over the wire" — the precondition for
every live-evidence obligation in the registry
(`RO-LIVE-01/02/03`, `CN-CONS-06`). Block production over a
real peer comes for free once the session driver lands
(producer broadcast already plugs into the same mux fabric
via the existing N-G server path).

No new BLUE authority. No new cryptography. Just the wire glue.
