# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **66 CI checks** at HEAD (`d62c2bc`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml` — **223 entries**) for
> rule IDs; reads the Phase 4 cluster plan
> (`docs/active/phase_4_cluster_plan.md`), the closed N-D / N-A / N-B /
> N-E / N-C / N-G / N-H / N-I / N-J / N-K / B1 / B2 / B3 / B4 / B5
> cluster docs, the OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY /
> ENACTMENT-COMMITTEE-WRITEBACK / PROPOSAL-PROCEDURES-DECODE cluster
> docs, and the **just-closed PHASE4-N-L cluster doc + CLOSURE
> record** (`docs/clusters/completed/PHASE4-N-L/{cluster,CLOSURE}.md`).
>
> **This is the PHASE4-N-L FULL CLOSE refresh (HEAD `d62c2bc`).** The
> previous SEAMS (HEAD `1946573`) pinned the PHASE4-N-K full-close
> state and surfaced "live Ouroboros mux + handshake driver above
> `MuxTransport`" as the highest-priority operator-action follow-on
> (`RO-LIVE-01`/`RO-LIVE-02`). **PHASE4-N-L mechanically closes the
> wire-layer half of that candidate** by shipping the session reducer
> + handshake driver + mux pump + n2n dialer + keep-alive session,
> and demotes the live-pass half to a single one-slice follow-on
> cluster tracked on `RO-LIVE-03` (renamed from -01/-02; both
> previous ids subsumed by the new live-pass obligation).
>
> **THE KEY FULL-CLOSE DELTAS.** PHASE4-N-L introduces **five new
> seams** — all non-BLUE — that bind the existing closed-grammar
> wire codecs into a running session over a real TCP socket:
>
> 1. **Mux session reducer seam (CN-SESS-03).**
>    `ade_network::session::core::step` is the **SOLE pub fn reducing
>    `(SessionState, ByteChunkIn) -> Result<Vec<SessionEffect>,
>    SessionError>`** in the workspace. `ByteChunkIn`, `SessionEffect`,
>    `SessionError` are closed sum types — no runtime extension; no
>    `#[non_exhaustive]`. Enforced by
>    `ci/ci_check_session_core_closure.sh`.
> 2. **Mini-protocol id registry seam (DC-SESS-02).**
>    `ade_network::session::event::AcceptedMiniProtocol` is a closed
>    enum with `from_id` that closes with `_ => None`. Adding a
>    mini-protocol is a SINGLE-variant addition + a SINGLE match-arm
>    addition at the dispatch site. Enforced by
>    `ci/ci_check_mini_protocol_id_registry_closed.sh`.
> 3. **Transport seam (CN-SESS-02).**
>    `ade_network::session::handshake_driver::Transport` is a **sync**
>    trait. Test impl: in-memory `Pipe` pair. Production impl:
>    `BlockingTransport` bridging from the tokio duplex mpsc channels
>    (runs inside `tokio::task::spawn_blocking`). The trait keeps the
>    handshake driver async-free.
> 4. **`MuxTransportHandle` full-duplex seam (DC-SESS-04).**
>    `ade_network::mux::transport::MuxTransportHandle` carries a
>    bounded `mpsc::Receiver<Vec<u8>>` (inbound) + a bounded
>    `mpsc::Sender<Vec<u8>>` (outbound) + reader/writer
>    `JoinHandle`s. Overflow → `TransportError::BackpressureExceeded`
>    (fail-fast — no silent drop). `DuplexCapacity::DEFAULT { 1024,
>    256, 16384 }` is pinned at compile time.
> 5. **Keep-alive cadence seam (DC-SESS-05).**
>    `KeepAliveCadence::DEFAULT { interval_ms: 60_000 }` is pinned at
>    compile time; operator-tunable cadence is **forbidden** in this
>    cluster (mirrors `SnapshotCadence::DEFAULT` discipline). The
>    session is driven by `ade_runtime::clock::Clock` — reuses and
>    extends the PHASE4-N-K Clock seam end-to-end into the wire
>    layer.
>
> **A new live-evidence seam (operator-action) is also opened:**
> `ade_node --peer ADDR --listen ADDR` now functionally connects to
> a real cardano-node peer. This **closes the operator-facing "binary
> that idles" gap from N-K** and opens **`RO-LIVE-03`** (live
> tip-following pass) as the new
> `blocked_until_operator_peer_available` obligation, subsuming the
> N-K-flagged `RO-LIVE-01`/`RO-LIVE-02`.
>
> **No new BLUE seams.** The existing closed-grammar wire codecs
> (mux frame, handshake, chain-sync, block-fetch, keep-alive) are
> reused unchanged. The session reducer composes them; it does not
> reinterpret them.
>
> Counts at this refresh: **+5 CI scripts** (61 → 66:
> `ci_check_mux_frame_closure.sh`, `ci_check_handshake_closure.sh`,
> `ci_check_session_core_closure.sh`,
> `ci_check_mini_protocol_id_registry_closed.sh`,
> `ci_check_session_no_unbounded.sh`) + **1 CI script extended**
> (`ci_check_clock_seam.sh` now covers `ade_network::session/`);
> **+9 registry rules** flipped or introduced (`CN-SESS-01`,
> `CN-SESS-02`, `CN-SESS-03`, `DC-SESS-01..05` all `enforced`;
> `RO-LIVE-03` declared with
> `open_obligation = "blocked_until_operator_peer_available"`);
> **4 carried rules strengthened** (`T-DET-01`, `CN-CONS-08`,
> `DC-NODE-01`, `DC-NODE-03` each gain
> `strengthened_in += PHASE4-N-L`); **+5 new GREEN files**
> (`session/{event, state, demux, core, handshake_driver}.rs`);
> **+1 RED file extended + 3 new RED files**
> (`mux/transport.rs` extended; `ade_runtime::network::{mux_pump,
> n2n_dialer}.rs`; `ade_runtime::orchestrator::keep_alive_session.rs`);
> **+1 new integration test file**
> (`session_replay_equivalence.rs`); **0 new operator-action probe
> binaries**; **0 new BLUE chokepoints**. Total invariant registry:
> **223 entries** (214 → 223 — the `RO-LIVE-01`/`RO-LIVE-02`
> placeholders are subsumed by `RO-LIVE-03`'s named live-pass
> obligation rather than retired; deprecation requires major
> version bump per IDD discipline).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-L is fully closed at this HEAD.** The Ade workspace can
now drive a real Ouroboros session: `ade_node` opens a TCP socket
against a real cardano-node peer, the n2n_dialer drives the
handshake to a negotiated version, the mux_pump bridges the duplex
`MuxTransportHandle` into the closed-sum `OrchestratorEvent`
stream, the session reducer demuxes mini-protocol frames, and the
keep-alive session emits clock-driven liveness pings. The cluster
**does not** add new BLUE; it composes the BLUE shipped by N-A..N-K
and the closed-grammar wire codecs.

**PHASE4-N-K remains fully closed** (carried). **PHASE4-N-J remains
fully closed** (carried). **PHASE4-N-I remains fully closed**
(carried). **PHASE4-N-H remains fully closed** (carried).
**PHASE4-N-G remains fully closed** (carried). **PHASE4-N-C remains
fully closed** (carried). **PHASE4-N-E remains fully closed**
(carried). **PROPOSAL-PROCEDURES-DECODE remains fully closed**
(carried). **PHASE4-B3..B5, OQ5 / COMMITTEE / DREP /
ENACTMENT-COMMITTEE-WRITEBACK** all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there remain **eight** fully-wired *external*
> ingress surfaces (block bytes, Plutus script bytes, snapshot bytes,
> Ouroboros mux frames, genesis JSON bundles, chain-selector stream
> inputs, the N-E wire-level mempool ingress, and the N-H receive-side
> N2N peer ingress). **PHASE4-N-L adds one *new external-surface
> driver* — the live N2N TCP socket attached via `--peer ADDR`** —
> but does NOT add a new wire-format ingress; the bytes still reduce
> through `mux::frame::decode_frame` (carried, unchanged) into
> closed mini-protocol message enums.
>
> **N-L formalises THE WIRE-LAYER SESSION SEAM** between raw socket
> bytes and the closed-sum `OrchestratorEvent` stream consumed by
> the GREEN orchestrator core. The seam is the closed sum trio
> `ByteChunkIn` / `SessionEffect` / `SessionError` plus the closed
> reducer `session::core::step`. The mux_pump translates RED-side
> tokio async into `ByteChunkIn`; the reducer demuxes
> mini-protocol frames deterministically; the result is dispatched
> back through the existing N-K `OrchestratorEvent` stream. Replay
> equivalence under a recorded `ByteChunkIn` corpus is the binding
> contract (DC-SESS-03).

### Surface: Live N2N TCP peer ingress (NEW in PHASE4-N-L — CN-SESS-01..03 + DC-SESS-01..05)

```
Surface: a TCP socket connected to a real cardano-node peer
         (initiator: --peer ADDR; responder: --listen ADDR)
Reduces to: ByteChunkIn { mini_protocol: AcceptedMiniProtocol, bytes: Vec<u8> }
         (after Handshaking → Connected transition)
         then: closed mini-protocol message via existing decode_* chokepoint
         then: existing OrchestratorEvent::PeerRx* variant (carried from N-K)
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED ade_runtime::network::n2n_dialer — open TCP socket; spawn
     reader + writer tasks into a bounded duplex
     (DuplexCapacity::DEFAULT { 1024, 256, 16384 }) returning a
     MuxTransportHandle.
  2. RED handshake_driver (via BlockingTransport bridge over the
     duplex mpsc channels, in tokio::task::spawn_blocking) — drives
     the closed-grammar N2N handshake_transition to a negotiated
     version. Reuses ade_network::handshake unchanged.
  3. On handshake success → emit OrchestratorEvent::PeerConnected
     {peer_id, role, negotiated_version}.
  4. RED ade_runtime::network::mux_pump — per-connection tokio
     task. Reads inbound Vec<u8> chunks from MuxTransportHandle.rx
     and feeds them as ByteChunkIn into session::core::step.
  5. GREEN session::core::step — demuxes via FrameBuffer; routes
     by AcceptedMiniProtocol; emits SessionEffect::*. Pure; sync;
     wall-clock-free.
  6. RED mux_pump dispatches SessionEffect to OrchestratorEvent
     translation + writes outbound bytes via the bounded outbound
     mpsc::Sender. Overflow → TransportError::BackpressureExceeded
     (fail-fast; closes the peer's task; sibling peers continue).
  7. RED keep_alive_session — Clock-driven (Clock::tick_stream;
     DC-NODE-03 reused) cadence pump emitting
     OrchestratorEvent::OutboundKeepAlive {peer_id} at
     KeepAliveCadence::DEFAULT { interval_ms: 60_000 }.
Cross-surface state sharing: per-peer state is fully independent
  — each peer owns its own MuxTransportHandle, FrameBuffer,
  SessionState, and orchestrator per-peer entry. A peer-session
  failure halts that peer's pump task only; sibling peers + the
  producer + the server pump continue (DC-NODE-01 strengthened).
```

**Rule (NEW in N-L).** The live-wire surface has a SINGLE session
reducer — `session::core::step` — and a SINGLE handshake authority —
`handshake::n2n_transition` / `n2c_transition` — and a SINGLE mux
frame authority — `mux::frame::{encode,decode}_frame`. Wire-layer
extension attaches by:

- Adding a new `AcceptedMiniProtocol` variant (closed enum;
  `from_id` closes with `_ => None`) + a matching `from_id` arm +
  a matching dispatch arm in `session::core::step` (closed-sum
  extension; CI-defended by
  `ci_check_mini_protocol_id_registry_closed.sh`).
- Adding a new `ByteChunkIn` / `SessionEffect` / `SessionError`
  variant + matching reducer arm (closed-sum extension; CI-defended
  by `ci_check_session_core_closure.sh`).
- Adding a new `Transport` impl alongside `Pipe` /
  `BlockingTransport` (closed registry-tracked addition; the
  handshake driver remains sync).
- Adding a new `KeepAliveCadence` instance is **forbidden in this
  cluster** — `DEFAULT { interval_ms: 60_000 }` is pinned at
  compile time per DC-SESS-05.

— **not** by adding a parallel `pub fn` reducing `(SessionState,
ByteChunkIn)` anywhere outside `session::core`, **not** by adding
a parallel `encode_frame`/`decode_frame` pair, **not** by allowing
mini-protocol frames to be delivered while `SessionState ==
Handshaking` (DC-SESS-01 type-state gate), **not** by introducing
unbounded mpsc anywhere in the session/mux path (DC-SESS-04;
CI-defended by `ci_check_session_no_unbounded.sh`), **not** by
reading the wall clock from any session-layer or keep-alive file
(DC-SESS-05; CI-defended by `ci_check_clock_seam.sh` extension).

### Surface: Process-boundary node entry (carried from N-K; **now drives a real session in N-L**)

Carried structurally. **N-L effect:** the `ade_node` binary now
accepts `--peer ADDR` and `--listen ADDR`; when supplied, the
production runner spawns `n2n_dialer` (initiator) and/or the
server pump's listener (responder) which spawn `mux_pump` tasks
per connected peer. The bootstrap + orchestrator core + persistent
writer chokepoints are unchanged. Authority-fatal exit codes
(`EXIT_AUTHORITY_FATAL_IO = 10`, `EXIT_AUTHORITY_FATAL_DECODE = 12`,
`EXIT_GENERIC_STARTUP = 1`) are unchanged.

### Surface: Receive-side N2N peer ingress (carried from N-H + N-I + N-J + N-K; **N-L wires real bytes end-to-end**)

Carried. **N-L effect:** the receive-side admit path
(`admit_via_block_validity`) is now driven by **real socket
bytes** end-to-end via the new session + mux-pump pipeline.
`CN-CONS-08.strengthened_in += PHASE4-N-L`. Per-peer task
isolation now extends to the wire layer — each pump task owns its
own `MuxTransportHandle` + `SessionState`; no shared mutable state.
`DC-NODE-01.strengthened_in += PHASE4-N-L`.

### Surfaces carried unchanged from prior revisions

- **Producer-side chain-sync server-role ingress** (N-G): carried.
- **Producer-side block-fetch server-role ingress** (N-G): carried.
- **Forge-block transition** (N-C): carried.
- **Self-accept broadcast gate** (N-C): carried.
- **Scheduler input ingress** (N-C): carried. Now driven by
  `leadership_session` via `Clock::tick_stream()` (carried from
  N-K).
- **Mempool ingress** (Tier-1 wire-level — N-E): carried.
- **Conway tx-body `proposal_procedures` sub-grammar** (PP): carried.
- **Single-tx validity** (B2): carried.
- **Mempool admission** (Tier-1 gate — B2): carried.
- **Full block validity** (B1): carried.
- **Persistent ledger snapshot encoding** (N-J): carried.
  `PersistentSnapshotWriter` from N-K is the production
  cadence-driven caller; unchanged.
- **Block bytes, Plutus script bytes, Snapshot bytes, Consensus-input
  extraction, Ouroboros mux frames, Genesis JSON bundles,
  Chain-selector stream inputs**: all carried.

### Receive-side rollback authority (carried from N-I + N-J + N-K)

The BLUE chokepoint set
(`materialize_rolled_back_state`, `commit_rollback`,
`RollbackContext`, `ChainDbWrite::rollback_to_slot`) is structurally
unchanged. **N-L effect:** the receive-side rollback path is now
exercised end-to-end against real peer bytes once a live N2N
session is established. `T-DET-01.strengthened_in += PHASE4-N-L`
— byte-stream → orchestrator-event determinism now proven
end-to-end via the session reducer.

### Candidates — surfaces not yet wired

- **N-L SUBSUMED the prior revision's `RO-LIVE-01`/`RO-LIVE-02`
  "live mux + handshake driver above `MuxTransport`" candidate
  pair** — the mechanical wire layer is now shipped. The
  operator-action live-pass half is **the new
  `RO-LIVE-03`** (`blocked_until_operator_peer_available`) — a
  separate one-slice follow-on cluster (`PHASE4-N-L-LIVE`).
- **CANDIDATE (NEW at N-L close): TLS / authenticated transport.**
  Declared `¬P-8` in the N-L invariants sketch. Cardano N2N is plain
  TCP on testnets; the curve25519 auth layer is a future cluster.
  Out of N-L scope.
- **CANDIDATE (NEW at N-L close): N2C local protocols.** The session
  reducer is N2N-scoped; the N2C local-chain-sync / local-tx-submission
  / local-state-query family are tracked under
  `CE-NODE-N2C-LTX` (carried) and a future N2C session cluster.
- **CANDIDATE (NEW at N-L close): peer-sharing + tx-submission live
  half.** Tx-submission's live half requires mempool integration
  (PHASE4-N-E) as a precondition; tracked.
- **CANDIDATE (carried from N-K): snapshot schema migration v1 → v2
  tooling** (`DC-STORE-09.open_obligation`).
- **CANDIDATE (carried from N-K): metrics + observability surface.**
  The orchestrator core consumes `OrchestratorEvent` and produces
  `OrchestratorEffect`; a future cluster threads a closed
  `MetricEffect` arm + a RED Prometheus exporter through the
  runner. Tier-5; no BLUE invariants change.
- **CANDIDATE (carried): snapshot eviction policy.**
- **CANDIDATE (carried, now further enabled by N-L live wire):
  multi-peer fork choice.**
- **CANDIDATE (carried): pre-Conway snapshot encoder.**
- **CE-N-H-6 live-evidence — still
  `blocked_until_operator_peer_available`** (carried; **subsumed by
  `RO-LIVE-03`** in scope — the live pass capturing `RO-LIVE-03`
  evidence will also capture CE-N-H-6 evidence).
- **CE-N-G-8 / CE-N-C-8 live-evidence — still
  `blocked_until_operator_*_available`** (carried).
- **PROPOSAL-PROCEDURES-DECODE remains closed** (carried).
- **PHASE4-N-E remains closed** (carried).

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-L** *(FULLY CLOSED at this HEAD — mechanical close; live operator pass tracked on `RO-LIVE-03`)* | **Live N2N TCP peer ingress: socket bytes → session reducer → orchestrator** | `ByteChunkIn` (mux pump → session reducer); `SessionEffect` (session reducer → mux pump); `OrchestratorEvent::PeerRx*` (mux pump → orchestrator core) | **DONE:** `ade_network::session::core::step` (CN-SESS-03 — SOLE session reducer `pub fn`); `ade_network::session::event::{AcceptedMiniProtocol, ByteChunkIn, SessionEffect, SessionError, HandshakeRole}` (closed sums); `ade_network::session::state::SessionState` (closed type-state); `ade_network::session::demux::FrameBuffer`; `ade_network::session::handshake_driver::Transport` (sync trait — CN-SESS-02); `ade_network::mux::transport::{MuxTransportHandle, TransportError, DuplexCapacity::DEFAULT}` (DC-SESS-04 — bounded fail-fast); `ade_runtime::network::{mux_pump, n2n_dialer}`; `ade_runtime::orchestrator::keep_alive_session` (DC-SESS-05 — `KeepAliveCadence::DEFAULT { interval_ms: 60_000 }`). 5 new CI scripts + 1 extended. 8 registry rules flipped to `enforced`. 4 carried rules `strengthened_in += PHASE4-N-L`. | **wired & closed in PHASE4-N-L (mechanical half; live operator pass is `RO-LIVE-03` — `blocked_until_operator_peer_available`)** |
| **NEW CANDIDATE — `RO-LIVE-03` live operator pass** *(NEW obligation declared at N-L close; subsumes N-K-flagged `RO-LIVE-01`/`RO-LIVE-02`)* | **Real cardano-node peer drives `ade_node` via N2N socket; capture JSONL evidence log** | Live `OrchestratorEvent::PeerRx*` stream fed by `mux_pump` over real bytes | `ade_node --peer ADDR --listen ADDR` + the `RO-LIVE-03` procedure doc. Tracked on `RO-LIVE-03.open_obligation = "blocked_until_operator_peer_available"`. One-slice follow-on cluster `PHASE4-N-L-LIVE`. | **candidate (operator-action follow-on; `blocked_until_operator_peer_available`)** |
| **NEW CANDIDATE — TLS / authenticated transport (curve25519 auth layer)** *(declared `¬P-8` in N-L invariants sketch; flagged by N-L close)* | **Authenticated N2N session** | A new `Transport` impl alongside `Pipe`/`BlockingTransport` plus a session-level auth gate | Future cluster; out of N-L scope. | **candidate (next-cluster seam)** |
| **NEW CANDIDATE — N2C local protocols session driver** *(flagged by N-L close)* | **Local-chain-sync / local-tx-submission / local-state-query over a Unix domain socket** | Similar shape to N2N — a `local_*::transition` family + an N2C-scoped `AcceptedMiniProtocol` parallel | Future cluster; out of N-L scope. Tracked alongside `CE-NODE-N2C-LTX`. | **candidate (next-cluster seam)** |
| **NEW CANDIDATE — Peer-sharing + tx-submission live half** *(flagged by N-L close)* | **Live tx submission + peer-sharing protocols over an established N-L session** | Existing closed mini-protocol surfaces (peer-sharing, tx-submission) become real over a live session | Future cluster; tx-submission's live half requires mempool integration (PHASE4-N-E) as precondition. | **candidate (next-cluster seam)** |
| **NEW CANDIDATE — Snapshot schema migration v1 → v2 tooling** *(carried from N-K — `DC-STORE-09` `open_obligation`)* | Carried. | Carried. | **candidate (carried from N-K)** |
| **NEW CANDIDATE — Metrics + observability surface** *(carried from N-K)* | Carried. | Carried. | **candidate (carried from N-K)** |
| **CANDIDATE — Snapshot eviction policy** *(carried)* | Carried. | Carried. | **candidate (carried)** |
| **CANDIDATE — Multi-peer fork choice (Praos longest-chain across competing peers)** *(carried; now further enabled by N-L live wire layer)* | Carried. | Carried. | **candidate (carried)** |
| **CANDIDATE — Pre-Conway snapshot encoder** *(carried)* | Carried. | Carried. | **candidate (carried)** |
| **CE-N-H-6 (cross-cluster obligation carried — subsumed by `RO-LIVE-03` in capture scope)** | **Live N2N follow-mode admission** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-G-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade serving)** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-C-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade forging)** | Carried. | Carried. | **carried (`blocked_until_operator_stake_available`)** |
| **N-C+ (declared non-goal in N-C cluster doc; OQ-4 lock)** | Carried. | Carried. | Carried. | candidate (declared non-goal) |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | Carried. | Carried. | Carried. | **deferred cross-cluster obligation** |
| **PP OQ-1..OQ-4 (separable seams)** | Carried. | Carried. | Carried. | candidate (carried) |
| B+ (full tx UTxO scope) | Carried. | Carried. | Carried. | candidate |
| B+ (Conway body witness depth) | Carried. | Carried. | Carried. | candidate (B2-carried) |
| B+ (pre-Conway tx) | Carried. | Carried. | Carried. | candidate |
| B1+ (pre-Babbage block) | Carried. | Carried. | Carried. | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum | Single dispatch fn over opaque-bytes payloads | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn over opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver (now mechanically closed; live pass is `RO-LIVE-03`) | `StreamInput` translated from `ChainSyncMessage` + `BlockFetchMessage` | Composition layer in `ade_core_interop` | **subsumed by N-L mechanical close** |

### Operator-action evidence (live-wire artifacts — not BLUE seams)

The Ade workspace closes Tier-1 wire-level seams in two halves: a
mechanical / GREEN half (code + harness + CI gates that the workspace
itself can certify on every push) and a **live-wire operator-action
half** (a real peer / client at the other end of a real socket
producing bytes Ade has never seen).

**At this HEAD two live-evidence logs remain committed**, four
cross-cluster obligations remain `blocked_until_operator_*_available`
(CE-N-H-6, CE-N-G-8, CE-N-C-8, **and the NEW `RO-LIVE-03`**), and
one cross-cluster obligation is carried from N-E. **N-L closes the
mechanical half of the live-wire story** by shipping the session
reducer + handshake driver + mux pump + n2n dialer + keep-alive
session and exposing them via `ade_node --peer ADDR --listen ADDR`.
`RO-LIVE-03` is the operator-action follow-on (one-slice cluster
`PHASE4-N-L-LIVE`).

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried) | Outbound-client probe against a real preprod N2N relay | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS | RED operator action (deferred) |
| `docs/clusters/completed/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | (pending) `CE-N-C-LIVE_<date>.log` | **`blocked_until_operator_stake_available`** (carried) | Cardano-node accepts an Ade-forged block as the next chain head | RED operator action |
| `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` | (pending) `CE-N-G-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | A real cardano-node peer issuing `RequestRange` accepts Ade-served bytes | RED operator action |
| `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` | (pending) `CE-N-H-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried; subsumed by `RO-LIVE-03` capture scope) | Ade follower fed RollForward + BlockDelivered from a real cardano-node peer produces a matching ChainDb tip | RED operator action |
| **(NEW in N-L)** `docs/clusters/completed/PHASE4-N-L/RO-LIVE-03_PROCEDURE.md` *(to be written by `PHASE4-N-L-LIVE`)* | (pending) `RO-LIVE-03_<date>.log` | **`blocked_until_operator_peer_available`** | `ade_node --peer ADDR` connects to a real cardano-node peer, drives N2N handshake to a negotiated version, follows tip end-to-end via the session reducer | RED operator action |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
At this HEAD there are still **five** such binaries (no N-L
addition):

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 | captured |
| `live_block_production_session` (PHASE4-N-C S7) | N-C S7 | CE-N-C-8 | blocked_until_operator_stake_available |
| `live_block_fetch_session` (PHASE4-N-G S7) | N-G S7 | CE-N-G-8 | blocked_until_operator_peer_available |
| `live_block_follow_session` (PHASE4-N-H S6) | N-H S6 | CE-N-H-6 | blocked_until_operator_peer_available |

**Pattern carried.** Hermetic default + `--connect <peer>` live pass.
**N-L has no new entry in this family** — the live operator pass
runs the `ade_node` production binary directly with
`--peer ADDR --listen ADDR`, not a separate probe binary.

**These are evidence-log patterns, not BLUE seams.**

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are now
**`RO-LIVE-03`** (the live operator pass — the next post-N-L
cluster to write), **CE-N-C-8** (live cardano-node forge
acceptance), **CE-N-G-8** (live cardano-node block-fetch
acceptance), **CE-N-H-6** (live cardano-node follow-mode admission —
subsumed by `RO-LIVE-03` capture scope), **multi-peer fork choice**
(now further enabled by N-L live wire), **CE-NODE-N2C-LTX**, and
the four **PROPOSAL-PROCEDURES-DECODE open obligations**.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **twenty-one** authoritative domains. **PHASE4-N-L added one
new compositional domain — wire-layer session authority** — at the
GREEN+RED level. No new BLUE chokepoint is introduced; the session
reducer composes the closed mini-protocol codecs + handshake + mux
frame chokepoints shipped earlier. Prior cluster narratives are
preserved unchanged.

### Wire-layer session authority (NEW in PHASE4-N-L)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **GREEN session reducer chokepoint** | `ade_network::session::core::step` | GREEN | The **SOLE `pub fn` reducing `(SessionState, ByteChunkIn) -> Result<Vec<SessionEffect>, SessionError>`** (CN-SESS-03 — CI-defended). Pure; sync; wall-clock-free. Demuxes mini-protocol frames via `FrameBuffer`; dispatches by `AcceptedMiniProtocol`; emits closed `SessionEffect`s. Type-state gates handshake-before-traffic (DC-SESS-01) — `SessionState::Handshaking` cannot deliver mini-protocol frames. |
| **GREEN closed session event vocabulary** | `ade_network::session::event::{AcceptedMiniProtocol, ByteChunkIn, SessionEffect, SessionError, HandshakeRole}` | GREEN | Closed sum types. `AcceptedMiniProtocol::from_id` closes with `_ => None` (DC-SESS-02 — CI-defended). `ByteChunkIn`/`SessionEffect`/`SessionError` carry no `String`; no `#[non_exhaustive]`; no plug-in registry. |
| **GREEN closed type-state** | `ade_network::session::state::SessionState` | GREEN | Closed enum `{ Handshaking, Connected }`. Type-state encodes the handshake-before-traffic gate; mini-protocol frames are rejected at `Handshaking` (DC-SESS-01). |
| **GREEN frame demuxer** | `ade_network::session::demux::FrameBuffer` | GREEN | Partial-frame accumulator. Buffers inbound bytes until a complete `mux::frame` boundary is available, then yields a `(mini_protocol_id, payload_bytes)` pair into `step`. Pure; deterministic. |
| **GREEN handshake driver** | `ade_network::session::handshake_driver::{Transport, run_n2n_handshake_initiator, run_n2n_handshake_responder}` | GREEN | Drives the closed-grammar N2N handshake to a negotiated version. `Transport` is a **sync** trait (CN-SESS-02); test impl: in-memory `Pipe` pair; production impl: `BlockingTransport` bridging the tokio duplex mpsc channels via `tokio::task::spawn_blocking`. Reuses `ade_network::handshake::transition` unchanged. |
| **RED `MuxTransportHandle` + `spawn_duplex`** | `ade_network::mux::transport::{MuxTransportHandle, TransportError, DuplexCapacity}` | RED | Carries bounded `mpsc::Receiver<Vec<u8>>` (inbound) + bounded `mpsc::Sender<Vec<u8>>` (outbound) + reader/writer `JoinHandle`s. Overflow → `TransportError::BackpressureExceeded` (fail-fast — no silent drop). `DuplexCapacity::DEFAULT { 1024, 256, 16384 }` pinned at compile time (DC-SESS-04). |
| **RED mux pump** | `ade_runtime::network::mux_pump` | RED | Per-connection tokio task. Reads inbound chunks from `MuxTransportHandle.rx` → calls `session::core::step` → dispatches `SessionEffect`s + writes outbound bytes via the bounded outbound `mpsc::Sender`. Translates RED-side asynchrony into the GREEN session reducer's events; never mutates orchestrator state directly. |
| **RED n2n dialer** | `ade_runtime::network::n2n_dialer` | RED | Opens outbound TCP, calls `spawn_duplex`, drives the handshake driver via `BlockingTransport`, emits `OrchestratorEvent::PeerConnected` on success, then hands off to `mux_pump`. |
| **RED keep-alive session** | `ade_runtime::orchestrator::keep_alive_session` | RED | Clock-driven cadence pump emitting `OrchestratorEvent::OutboundKeepAlive { peer_id }` at `KeepAliveCadence::DEFAULT { interval_ms: 60_000 }` (DC-SESS-05). Reuses the N-K `Clock` seam end-to-end into the wire layer. |
| **CI gates (5 new + 1 extended)** | `ci/ci_check_{mux_frame_closure,handshake_closure,session_core_closure,mini_protocol_id_registry_closed,session_no_unbounded}.sh` + `ci_check_clock_seam.sh` (extended) | CI | (1) `mux_frame_closure` — sole pub `encode_frame`/`decode_frame` pair in `ade_network::mux::frame` (CN-SESS-01). (2) `handshake_closure` — sole pub `n2n_transition`/`n2c_transition` in `ade_network::handshake::transition` (CN-SESS-02). (3) `session_core_closure` — sole pub `step` in `ade_network::session::core`; no `tokio::*` imports in session core files; type-state gate present (CN-SESS-03 + DC-SESS-01 + DC-SESS-05 session-side). (4) `mini_protocol_id_registry_closed` — `AcceptedMiniProtocol::from_id` closes with `_ => None`; no wildcard accept at dispatch (DC-SESS-02). (5) `session_no_unbounded` — no `mpsc::unbounded_*`, no `Vec` growth without bound in mux/session paths (DC-SESS-04). (Extended) `clock_seam` — now also covers `ade_network::session/` for wire-side wall-clock-free guarantee (DC-SESS-05). Total CI count: 61 → 66. |

**Rule.** This domain has:
- **One GREEN session reducer** (`session::core::step` — CN-SESS-03
  single-authority).
- **One GREEN closed event/effect vocabulary** (`ByteChunkIn` /
  `SessionEffect` / `SessionError` / `AcceptedMiniProtocol` —
  trait-less, data-only).
- **One closed type-state** (`SessionState` — handshake-before-traffic
  type-state gate; DC-SESS-01).
- **One GREEN handshake driver** with a sync `Transport` trait + two
  impls (`Pipe`, `BlockingTransport`).
- **One RED `MuxTransportHandle`** with pinned `DuplexCapacity::DEFAULT`
  and fail-fast `TransportError::BackpressureExceeded` (DC-SESS-04).
- **One RED mux pump** + **one RED n2n dialer** + **one RED keep-alive
  session**.
- **Five new CI gates + one extended** defending the above.

**THE KEY SEAMS:**

1. **`session::core::step` is the SOLE session reducer `pub fn`**
   (CN-SESS-03). CI-defended via workspace-wide grep. Pure; sync;
   wall-clock-free.
2. **`AcceptedMiniProtocol::from_id` closes with `_ => None`**
   (DC-SESS-02). Adding a mini-protocol is a single-variant addition
   + a single match-arm addition at the dispatch site. CI-defended.
3. **`Transport` is a sync trait** (CN-SESS-02). Test impl: in-memory
   `Pipe` pair. Production impl: `BlockingTransport` bridging the
   tokio duplex mpsc channels (runs inside
   `tokio::task::spawn_blocking`). Keeps the handshake driver
   async-free.
4. **`MuxTransportHandle` is bounded both ways** (DC-SESS-04).
   `DuplexCapacity::DEFAULT { 1024, 256, 16384 }` pinned at compile
   time. Overflow → `TransportError::BackpressureExceeded`
   (fail-fast — no silent drop). CI-defended by
   `ci_check_session_no_unbounded.sh`.
5. **`KeepAliveCadence::DEFAULT { interval_ms: 60_000 }` is pinned at
   compile time** (DC-SESS-05). Operator-tunable cadence is forbidden
   in this cluster (mirrors `SnapshotCadence::DEFAULT` discipline).
   The keep-alive session is driven by the N-K `Clock` seam.
6. **Type-state gates handshake-before-traffic** (DC-SESS-01).
   `SessionState::Handshaking` cannot deliver mini-protocol frames;
   `Connected` is the only state in which the dispatch arms accept
   payloads.
7. **Replay equivalence holds under recorded `ByteChunkIn`**
   (DC-SESS-03). Two replays produce byte-identical
   `Vec<SessionEffect>` sequences. Proven by
   `tests/session_replay_equivalence.rs`.
8. **Per-peer task isolation extends to the wire layer**
   (DC-NODE-01 strengthening). Each pump task owns its own
   `MuxTransportHandle` + `FrameBuffer` + `SessionState` + per-peer
   orchestrator entry; failure halts only that task.

**New work** that adds a wire-layer feature attaches by:
- Adding an `AcceptedMiniProtocol` variant + `from_id` arm +
  dispatch arm in `session::core::step` (closed-sum extension).
- Adding a `ByteChunkIn` / `SessionEffect` / `SessionError` variant
  + matching reducer arm (closed-sum extension).
- Adding a new `Transport` impl alongside `Pipe` / `BlockingTransport`
  (deliberate registry-tracked closed addition; the handshake driver
  remains sync).
- Adding a new RED runner file under `ade_runtime::network::*` (e.g.
  a future N2C dialer; or a future TLS dialer) following the
  `n2n_dialer` shape.

— **not** by adding a parallel `pub fn` reducing `(SessionState,
ByteChunkIn)` outside `session::core`, **not** by adding a parallel
`encode_frame`/`decode_frame` pair, **not** by delivering
mini-protocol frames in `SessionState::Handshaking`, **not** by
introducing unbounded mpsc, **not** by reading the wall clock from
any session-layer or keep-alive file, **not** by adding an
operator-tunable keep-alive cadence in this cluster.

**Declared non-goals carried from the cluster doc:**
- TLS / authenticated transport — out of scope (declared `¬P-8` in
  invariants sketch). Future cluster.
- N2C local protocols — out of scope. Future cluster (alongside
  `CE-NODE-N2C-LTX`).
- Peer-sharing + tx-submission live half — out of scope.
  Tx-submission's live half requires mempool integration as
  precondition.
- Live operator pass (`RO-LIVE-03`) — the mechanical wire layer is
  ready; running `ade_node` against a private cardano-node peer +
  capturing the JSONL log is the follow-on cluster's deliverable.

### Node orchestration authority (carried from PHASE4-N-K)

Carried. **N-L usage:** the orchestrator core's `OrchestratorEvent`
stream now receives real peer events end-to-end from the
`mux_pump` + `n2n_dialer` + `keep_alive_session` RED runners. The
new `OrchestratorEvent::OutboundKeepAlive { peer_id }` variant is
added by N-L; the orchestrator core records it (no immediate
effect — session-layer keep-alive frame encoding is a future
cluster).

### Persistent ledger snapshot encoding authority (carried from PHASE4-N-J + N-K)

Carried unchanged.

### Receive-side rollback authority (carried from N-I + N-J + N-K)

Carried. **N-L note:** `T-DET-01.strengthened_in += PHASE4-N-L` —
byte-stream → orchestrator-event determinism now proven end-to-end
via the session reducer.

### Receive-side admission authority (carried from PHASE4-N-H + N-K)

Carried. **N-L note:** the admit path is now driven by **real
socket bytes** end-to-end via the session reducer + mux pump.
`CN-CONS-08.strengthened_in += PHASE4-N-L`.

### Producer-side server response authority (carried from N-G)

Carried.

### Block production authority (carried from N-C + N-K)

Carried.

### Mempool ingress (carried from N-E)

Carried.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged. **N-L-specific strengthening:** `T-DET-01`
(replay equivalence) now extends through the session reducer
end-to-end; `CN-CONS-08` (receive admit-path closure) now driven
by real socket bytes; `DC-NODE-01` (per-peer isolation) now extends
to the wire layer; `DC-NODE-03` (clock-injection seam) now covers
keep-alive end-to-end.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. N-L added new edges within `ade_network` (a BLUE-with-per-file-RED-carve-out
  crate): the GREEN session files import the BLUE mux-frame + handshake
  + mini-protocol codecs but never the RED `mux::transport` or
  `bin::capture_*` siblings; the RED `mux::transport` is the only
  tokio-importing file in `ade_network`. New edge:
  `ade_runtime::network::{mux_pump, n2n_dialer}` + `ade_runtime::orchestrator::keep_alive_session`
  depend on `ade_network::{session, mux::transport, handshake}`
  (GREEN/RED → GREEN/RED within `ade_network`, never reaching into
  RED siblings inappropriately). Same direction; allowed.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE. N-L
  added no new BLUE; the session core + event + state + demux +
  handshake_driver are GREEN and explicitly non-async (no `tokio::*`,
  no `async fn`).
- **`ci_check_mux_frame_closure.sh`** *(N-L — CN-SESS-01 enforcement)* —
  asserts the sole pub `encode_frame`/`decode_frame` pair in
  `ade_network::mux::frame`.
- **`ci_check_handshake_closure.sh`** *(N-L — CN-SESS-02 enforcement)* —
  asserts the sole pub `n2n_transition`/`n2c_transition` in
  `ade_network::handshake::transition`.
- **`ci_check_session_core_closure.sh`** *(N-L — CN-SESS-03 +
  DC-SESS-01 + DC-SESS-05 session-side enforcement)* — asserts sole
  pub `step` in `ade_network::session::core`; no `tokio::*` imports
  in session core files; type-state gate present.
- **`ci_check_mini_protocol_id_registry_closed.sh`** *(N-L —
  DC-SESS-02 enforcement)* — asserts `AcceptedMiniProtocol::from_id`
  closes with `_ => None`; no wildcard accept at dispatch site.
- **`ci_check_session_no_unbounded.sh`** *(N-L — DC-SESS-04
  enforcement)* — asserts no `mpsc::unbounded_*` and no unbounded
  `Vec` growth in mux/session paths.
- **`ci_check_clock_seam.sh`** *(extended in N-L — DC-SESS-05
  enforcement)* — now also covers `ade_network::session/` to enforce
  the wire-side wall-clock-free guarantee.
- *N-K carried CI gates:* `ci_check_bootstrap_closure.sh`,
  `ci_check_orchestrator_core_purity.sh`,
  `ci_check_persistent_writer_no_parallel_cadence.sh`,
  `ci_check_peer_session_isolation.sh`,
  `ci_check_node_binary_uses_single_bootstrap.sh`.
- *N-J carried CI gates:* `ci_check_snapshot_encoder_closure.sh`.
- *N-I carried CI gates:* `ci_check_rollback_materialize_closure.sh`,
  `ci_check_snapshot_cadence_purity.sh`.
- *N-H carried CI gates:* `ci_check_admitted_block_closure.sh`,
  `ci_check_receive_reducer_closure.sh`,
  `ci_check_receive_replay_purity.sh`,
  `ci_check_receive_orchestrator_no_producer_dep.sh`,
  `ci_check_receive_paths_corpus_present.sh`.
- *N-G carried CI gates:* `ci_check_no_parallel_header_splitter.sh`,
  `ci_check_served_chain_closure.sh`,
  `ci_check_chain_sync_server_closure.sh`,
  `ci_check_block_fetch_server_closure.sh`,
  `ci_check_broadcast_to_served_purity.sh`,
  `ci_check_n2n_server_no_signing_dep.sh`,
  `ci_check_server_paths_corpus_present.sh`.
- *N-C carried CI gates:* `ci_check_private_key_custody.sh`,
  `ci_check_opcert_closed.sh`, `ci_check_forge_purity.sh`,
  `ci_check_no_producer_body_encoder.sh`,
  `ci_check_self_accept_gate.sh`, `ci_check_scheduler_closure.sh`,
  `ci_check_producer_corpus_present.sh`.
- `ci_check_constitution_coverage.sh` — carried.
- `ci_check_proposal_procedures_closed.sh` *(PP — DC-LEDGER-11)* — carried.
- `ci_check_mempool_ingress_closure.sh` /
  `ci_check_mempool_ingress_replay.sh` *(N-E)* — carried.
- `ci_check_credential_discriminant_closed.sh` *(OQ5 / COMMITTEE /
  DREP / ENACTMENT)* — carried.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5)* — carried.
- `ci_check_deposit_param_authority.sh` *(B3)* — carried.
- `ci_check_conway_cert_classification_closed.sh` *(B3F)* — carried.
- `ci_check_no_chaindb_in_consensus_blue.sh` /
  `ci_check_no_float_in_consensus.sh` /
  `ci_check_no_density_in_fork_choice.sh` /
  `ci_check_consensus_closed_enums.sh` — carried.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh` —
  carried.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PHASE4-N-L
added six closed surfaces** at the wire-layer session — the
`AcceptedMiniProtocol` closed enum + `from_id` registry; the
`ByteChunkIn` / `SessionEffect` / `SessionError` closed sum trio;
the `SessionState` closed type-state; the `Transport` sync trait
with closed impl set (`Pipe`, `BlockingTransport`); the
`MuxTransportHandle` + `TransportError` + `DuplexCapacity::DEFAULT`
closed bounded-duplex surface; and the `KeepAliveCadence::DEFAULT`
compile-time-pinned cadence. Plus **five new CI gates + one
extended** (CI count 61 → 66) and **eight newly-flipped + one
newly-declared + four strengthened** registry rules (registry
total 214 → 223).

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. |
| `StakeCredential` *(OQ5)* | `ade_types::shelley::cert` | 2 variants | DC-LEDGER-10. |
| Credential-decode chokepoints *(OQ5 + PP)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` + `ade_codec::conway::governance::decode_stake_credential` | 3 functions | Closed 2-variant mapping. |
| `ConwayCert` *(B3/B4)* | `ade_types::conway::cert` | 19 variants | DC-LEDGER-08. |
| `GovAction` *(PP/ENACTMENT)* | `ade_types::conway::governance` | 7 variants | DC-LEDGER-11. |
| `ProposalProcedure` *(PP)* | `ade_types::conway::governance` | closed 4-field struct | DC-LEDGER-11. |
| `decode_proposal_procedures` / `encode_proposal_procedures` *(PP)* | `ade_codec::conway::governance` | 2 functions | DC-LEDGER-11. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| `CertDisposition` / `DepositEffect` / `CoinSource` *(B3)* | `ade_types::conway::cert` | 3 / 2 / 3 variants | Closed. |
| `ConwayCertAction` *(B4)* | `ade_ledger::delegation` | closed | No `Neutral`. |
| `GovernanceCertEffect` / `OwnerTaggedEffect` / etc. *(B4)* | `ade_ledger::delegation` | closed | B4 plumbing. |
| `GovCertEnv` *(B5)* | `ade_ledger::state` | closed struct | Fail-fast. |
| `apply_conway_gov_cert` dispatch *(B5)* | `ade_ledger::gov_cert` | 1 function | DC-LEDGER-09. |
| `apply_committee_enactment` *(ENACTMENT)* | `ade_ledger::governance` | 1 pure transition | Closed. |
| `IngressSource` *(N-E)* | `ade_ledger::mempool::ingress` | 2 variants | Closed source discriminant. |
| `IngressEvent` *(N-E)* | `ade_ledger::mempool::ingress` | closed struct | Closed flat-data envelope. |
| `mempool_ingress` chokepoint *(N-E)* | `ade_ledger::mempool::ingress` | 1 function | DC-MEM-03. |
| `ProducerTick` *(N-C-S3)* | `ade_ledger::producer::state` | closed 14-field struct | Carried. |
| `forge_block` chokepoint *(N-C-S3)* | `ade_ledger::producer::forge` | 1 function | Carried. |
| `ForgeError` / `ForgeEffects` / `ForgedBlock` *(N-C-S3)* | `ade_ledger::producer::forge` | 7 / 1 / closed struct | Carried. |
| `encode_opcert` / `decode_opcert` chokepoint pair *(N-C-S2)* | `ade_codec::shelley::opcert` | 2 functions | Carried. |
| `OpCertCodecError` *(N-C-S2)* | `ade_codec::shelley::opcert` | 7 variants | Carried. |
| `opcert_validate` chokepoint *(N-C-S2)* | `ade_core::consensus::opcert_validate` | 1 function | Carried. |
| `OpCertError` *(N-C-S2)* | `ade_core::consensus::opcert_validate` | closed | Carried. |
| `block_body_hash_from_buckets` chokepoint *(N-C-S4)* | `ade_ledger::block_body_hash` | 1 function | Carried. |
| `AcceptedBlock` token *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 newtype | Carried. |
| `self_accept` chokepoint *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 function | Carried. |
| `SelfAcceptError` *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 variant | Carried. |
| `SchedulerInput` / `SchedulerEffect` / `SchedulerHaltReason` / `SchedulerState` *(N-C-S6)* | `ade_runtime::producer::scheduler` | closed sums | Carried. |
| `TickInputs` / `TickAssemblyError` / `assemble_tick` *(N-C-S6)* | `ade_runtime::producer::tick_assembler` | closed | Carried. |
| `BroadcastError` *(N-C-S6)* | `ade_runtime::producer::broadcast` | 2 variants | Carried. |
| RED signing primitives + key types *(N-C-S1)* | `ade_runtime::producer::signing::*` | closed | Carried. |
| RED key loader *(N-C-S1)* | `ade_runtime::producer::keys` | closed | Carried. |
| `accepted_block_header_bytes` accessor *(N-G-S1)* | `ade_ledger::block_validity::header_input` | 1 function | Carried. |
| `ServerReply` (chain-sync + block-fetch) *(N-G-S1)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed wrappers | Carried. |
| `HeaderProjection` *(N-G-S3)* | `ade_network::chain_sync::server` | closed struct | Carried. |
| `ServedHeaderLookup` / `ServedRangeLookup` traits *(N-G-S3/S4)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed traits | Carried. |
| `producer_chain_sync_serve` / `producer_chain_sync_advance_tip` *(N-G-S3)* | `ade_network::chain_sync::server` | 2 functions | Carried. |
| `producer_block_fetch_serve` *(N-G-S4)* | `ade_network::block_fetch::server` | 1 function | Carried. |
| `Producer*ServerState` / `ProducerServerError` / `ServerStep` / etc. *(N-G-S3/S4)* | `ade_network::{chain_sync, block_fetch}::server` | closed | Carried. |
| `ServedChainSnapshot` / `served_chain_admit` *(N-G-S2)* | `ade_ledger::producer::served_chain` | closed | Carried. |
| `PerPeerN2nServerState` / `DispatchError` *(N-G-S6)* | `ade_runtime::network::n2n_server` | closed | Carried. |
| `AdmittedBlock` token *(N-H-S1)* | `ade_ledger::receive::admitted` | closed struct | Carried. |
| `AdmittedOutcome` *(N-H-S1)* | `ade_ledger::receive::admitted` | closed struct | Carried. |
| `admit_via_block_validity` chokepoint *(N-H-S1)* | `ade_ledger::receive::admitted` | 1 function | Carried. **N-L note:** now driven by real socket bytes end-to-end via the session reducer + mux pump; `CN-CONS-08.strengthened_in += PHASE4-N-L`. |
| `ReceiveEvent` *(N-H-S1)* | `ade_ledger::receive::events` | 3 variants | Carried. |
| `ReceiveEffect` *(N-H-S1)* | `ade_ledger::receive::events` | 4 variants | Carried. |
| `NoOpReason` *(N-H-S1)* | `ade_ledger::receive::events` | 1 variant | Carried. |
| `ReceiveError` *(N-H-S1)* | `ade_ledger::receive::events` | 4 variants | Carried. |
| `TargetPoint` / `TipPoint` *(N-H-S1 — receive)* | `ade_ledger::receive::events` | 2 closed structs | Carried. |
| `PendingHeaderCache` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | closed struct | Carried. |
| `PendingHeaderCacheError` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | 1 variant | Carried. |
| `ChainDbWrite` trait *(N-H-S1; N-I-S3 extended)* | `ade_ledger::receive::chain_write` | 2 methods | Carried. |
| `ChainWriteError` / `ChainWriteErrorKind` *(N-H-S1)* | `ade_ledger::receive::chain_write` | 2 / 3 variants | Carried. |
| `ReceiveState` *(N-H-S2)* | `ade_ledger::receive::reducer` | closed struct | Carried. |
| `receive_apply` chokepoint *(N-H-S2; N-I-S6 extended)* | `ade_ledger::receive::reducer` | 1 function | Carried. |
| `receive_apply_sequence` driver *(N-H-S2)* | `ade_ledger::receive::reducer` | 1 function | Carried. |
| `PerPeerReceiveState` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | closed RED struct | Carried. **N-L note:** held by `OrchestratorState::per_peer_receive` (BTreeMap keyed by `PeerId`); per-peer wire-layer isolation now also enforced via per-pump `MuxTransportHandle` + `FrameBuffer` + `SessionState`; `DC-NODE-01.strengthened_in += PHASE4-N-L`. |
| `ReceiveDispatchError` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | 3 variants | Carried. |
| `SnapshotReader` trait *(N-I-S1; N-J extended)* | `ade_ledger::rollback::traits` | 1 trait with 1 method | Carried. |
| `BlockSource` trait *(N-I-S1)* | `ade_ledger::rollback::traits` | 1 trait with 1 method | Carried. |
| `MaterializeError` *(N-I-S1)* | `ade_ledger::rollback::error` | 3 variants | Carried. |
| `CommitRollbackError` *(N-I-S1)* | `ade_ledger::rollback::error` | 1 variant | Carried. |
| `TargetPoint` *(N-I-S2 — rollback flavor)* | `ade_ledger::rollback::materialize` | closed struct | Carried. |
| `materialize_rolled_back_state` chokepoint *(N-I-S2 — CN-STORE-07)* | `ade_ledger::rollback::materialize` | 1 function | Carried. |
| `commit_rollback` chokepoint *(N-I-S3)* | `ade_ledger::rollback::commit` | 1 function | Carried. |
| `ChainDbWrite::rollback_to_slot` trait method *(N-I-S3)* | `ade_ledger::receive::chain_write` | 1 method | Carried. |
| `RollbackContext<'a>` *(N-I-S6)* | `ade_ledger::receive::reducer` | closed BLUE struct | Carried. |
| `SnapshotCadence` *(N-I-S4 — DC-STORE-07)* | `ade_runtime::rollback::cadence` | closed BLUE-structural struct (exactly 1 field) | Carried. |
| `SnapshotEncodeError` *(N-J-S1)* | `ade_ledger::snapshot::error` | 1 variant | Carried. |
| `SnapshotDecodeError` *(N-J-S1)* | `ade_ledger::snapshot::error` | 5 variants | Carried. |
| `StructuralReason` *(N-J-S1)* | `ade_ledger::snapshot::error` | 9 variants | Carried. |
| `encode_chain_dep` / `decode_chain_dep` chokepoint pair *(N-J-S1 — CN-STORE-08)* | `ade_ledger::snapshot::chain_dep` | 2 functions | Carried. |
| snapshot sub-state encoder/decoder chokepoint pairs *(N-J-S2..S5)* | `ade_ledger::snapshot::{utxo_state, cert_state, epoch_state, gov_state}` | 12 functions | Carried. |
| `encode_ledger_state` / `decode_ledger_state` chokepoint pair *(N-J-S6 — CN-STORE-08)* | `ade_ledger::snapshot::ledger` | 2 functions | Carried. |
| `encode_snapshot` / `decode_snapshot` chokepoint pair *(N-J-S7 — CN-STORE-08)* | `ade_ledger::snapshot::framing` | 2 functions | Carried. |
| `SCHEMA_VERSION: u32 = 1` *(N-J-S7 — DC-STORE-09)* | `ade_ledger::snapshot::framing` | 1 `pub const` | Carried. |
| `PersistentSnapshotCache<'a, S: SnapshotStore + ?Sized>` *(N-J-S8)* | `ade_runtime::rollback::persistent_cache` | closed GREEN struct | Carried. |
| `PersistentCacheError` *(N-J-S8)* | `ade_runtime::rollback::persistent_cache` | 3 variants | Carried. |
| `PERSISTENT_CACHE_SCHEMA_VERSION: u32` *(N-J-S8)* | `ade_runtime::rollback::persistent_cache` | 1 `pub const` | Carried. |
| `bootstrap_initial_state` chokepoint *(N-K — CN-NODE-01)* | `ade_runtime::bootstrap` | 1 function | Carried. |
| `BootstrapInputs<'a, D, S>` / `BootstrapError` *(N-K)* | `ade_runtime::bootstrap` | closed struct + closed sum | Carried. |
| `Clock` trait + `DeterministicClock` + `SystemClock` *(N-K — DC-NODE-03)* | `ade_runtime::clock` | 1 trait + 2 impls | Carried. **N-L note:** clock-injection seam now covers keep-alive end-to-end via `keep_alive_session` (`DC-NODE-03.strengthened_in += PHASE4-N-L`). |
| `OrchestratorEvent` *(N-K — DC-NODE-01 + DC-NODE-03)* | `ade_runtime::orchestrator::event` | closed sum | Carried. **N-L note:** **NEW variant `OrchestratorEvent::OutboundKeepAlive { peer_id }`** emitted by `keep_alive_session`; closed-sum extension per the N-K rules. |
| `OrchestratorEffect` *(N-K)* | `ade_runtime::orchestrator::event` | closed sum | Carried. |
| `OrchestratorError` / `PeerHaltReason` / `AuthorityFatalKind` *(N-K)* | `ade_runtime::orchestrator::event` | 3 closed sums | Carried. |
| `PeerId` / `PeerRole` *(N-K)* | `ade_runtime::orchestrator::event` | 1 newtype + 1 closed enum | Carried. |
| `OrchestratorState` + `PerPeerReceiveVersions` *(N-K)* | `ade_runtime::orchestrator::state` | closed structs | Carried. |
| `orchestrator::core::step` reducer *(N-K)* | `ade_runtime::orchestrator::core` | 1 function | Carried. |
| `PersistentSnapshotWriter` + `on_admitted` + `force_capture` *(N-K — DC-NODE-02)* | `ade_runtime::rollback::persistent_writer` | closed struct + 2 methods | Carried. |
| `NodeRunError` *(N-K — DC-NODE-04)* | `ade_node::node` | closed sum | Carried. |
| `EXIT_AUTHORITY_FATAL_IO = 10` / `EXIT_AUTHORITY_FATAL_DECODE = 12` / `EXIT_GENERIC_STARTUP = 1` *(N-K — DC-NODE-04)* | `ade_node::node` | 3 `pub const i32` | Carried. |
| **`session::core::step` reducer** *(NEW in N-L — CN-SESS-03)* | `ade_network::session::core` | 1 function — **THE SOLE `pub fn` reducing `(SessionState, ByteChunkIn) -> Result<Vec<SessionEffect>, SessionError>` in the workspace** | Single-authority. Pure; sync; wall-clock-free. CI-defended via `ci_check_session_core_closure.sh` (workspace-wide grep + no-tokio + type-state-gate check). New session-reducer shape = strengthening (CI fail). |
| **`AcceptedMiniProtocol` + `from_id` closed registry** *(NEW in N-L — DC-SESS-02)* | `ade_network::session::event::AcceptedMiniProtocol` | closed enum + closed `from_id` mapping | The mini-protocol id registry. `from_id` closes with `_ => None`; dispatch site has no wildcard accept. New mini-protocol = single variant + single `from_id` arm + single dispatch arm. CI-defended via `ci_check_mini_protocol_id_registry_closed.sh`. |
| **`ByteChunkIn` / `SessionEffect` / `SessionError` / `HandshakeRole`** *(NEW in N-L)* | `ade_network::session::event` | 4 closed sums | Closed event/effect/error vocabulary. No `String`; no `#[non_exhaustive]`. |
| **`SessionState`** *(NEW in N-L — DC-SESS-01)* | `ade_network::session::state` | closed enum `{ Handshaking, Connected }` | Closed type-state. Encodes the handshake-before-traffic gate at the type level; mini-protocol frames rejected in `Handshaking`. CI-defended via `ci_check_session_core_closure.sh` (type-state gate check). |
| **`FrameBuffer`** *(NEW in N-L)* | `ade_network::session::demux` | closed struct | Partial-frame accumulator. Pure; deterministic. |
| **`Transport` sync trait + closed impl set (`Pipe`, `BlockingTransport`)** *(NEW in N-L — CN-SESS-02)* | `ade_network::session::handshake_driver` + `ade_runtime::network::*` (production impl) | 1 trait + 2 impls — **THE handshake transport seam** | Sync trait. Test impl: in-memory `Pipe`. Production impl: `BlockingTransport` over duplex mpsc via `tokio::task::spawn_blocking`. New impls remain deliberate registry-tracked closed additions. |
| **`MuxTransportHandle` + `TransportError` + `DuplexCapacity::DEFAULT { 1024, 256, 16384 }`** *(NEW in N-L — DC-SESS-04)* | `ade_network::mux::transport` | closed struct + closed sum + 1 `pub const` | Bounded full-duplex transport. Overflow → `TransportError::BackpressureExceeded` (fail-fast; no silent drop). `DuplexCapacity::DEFAULT` pinned at compile time. CI-defended via `ci_check_session_no_unbounded.sh`. |
| **`KeepAliveCadence::DEFAULT { interval_ms: 60_000 }`** *(NEW in N-L — DC-SESS-05)* | `ade_runtime::orchestrator::keep_alive_session` | 1 `pub const` — **THE compile-time-pinned keep-alive cadence** | Operator-tunable cadence forbidden in this cluster (mirrors `SnapshotCadence::DEFAULT` discipline). Driven by `ade_runtime::clock::Clock`. CI-defended via `ci_check_clock_seam.sh` (extended). |
| **`OrchestratorEvent::OutboundKeepAlive { peer_id }`** *(NEW variant in N-L on the N-K closed sum)* | `ade_runtime::orchestrator::event::OrchestratorEvent` | 1 new variant | Closed-sum extension per the N-K rules. Emitted by `keep_alive_session`; recorded by the orchestrator core (no immediate effect — session-layer keep-alive frame encoding is a future cluster). |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants | |
| Named ingress chokepoints (block CBOR) | `ade_codec::*` | 10 | |
| Conway cert/withdrawals sub-grammar decoders *(B3 / B4)* | `ade_codec::conway::{cert, withdrawals}` + `ade_codec::shelley::cert::read_pool_registration_cert` | 5 functions | Closed. |
| Named ingress chokepoint (Plutus script CBOR) | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 | |
| `PreservedCbor::new` constructor | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | |
| `CodecError` variants *(B3-extended)* | `ade_codec::error` | + `UnknownCertTag`, `DuplicateMapKey` | |
| Mini-protocol message enums | `ade_network::codec::*` | 11 closed enums | |
| Mini-protocol encode/decode chokepoints | `ade_network::codec::*::{encode_*, decode_*}` | 22 functions | |
| **Mux frame chokepoints** *(now also CN-SESS-01-enforced)* | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | CI-defended by `ci_check_mux_frame_closure.sh` (NEW in N-L). |
| **Handshake transition chokepoints** *(now also CN-SESS-02-enforced)* | `ade_network::handshake::transition::{n2n_transition, n2c_transition}` | 2 free functions | CI-defended by `ci_check_handshake_closure.sh` (NEW in N-L). |
| Mini-protocol transition functions | `ade_network::*::transition` + `n2c::local_*::transition` | 8 modules | |
| Mini-protocol version enums | `ade_network::codec::version::*` | 11 closed enums | |
| `ChainDb` / `SnapshotStore` / `Recoverable` trait surfaces | `ade_runtime::chaindb` + `ade_runtime::recovery` | closed | Carried. |
| Hash domain functions | `ade_crypto::blake2b::*` | 4 named domains | |
| `ChainEvent` / `ChainSelectionReject` *(N-B)* | `ade_core::consensus::events` | 5 / 4 variants | |
| Consensus error families *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | |
| `StreamInput` / `OrchestratorError` (N-B) / `DecodeError` / `GenesisParseError` / `GenesisBlob` / `NetworkMagic` *(N-B)* | various | closed | |
| `LedgerView` trait *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods | |
| `HeaderVrf` *(N-B; B1)* | `ade_core::consensus::header_summary` | 2 variants | |
| `BlockValidityVerdict` / `BlockValidityError` etc. *(B1)* | `ade_ledger::block_validity::verdict` | closed | |
| `block_validity` chokepoint *(B1)* | `ade_ledger::block_validity::transition` | 1 function | Carried. |
| `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` / `SignerSource` / `WitnessClosureError` etc. *(B2)* | `ade_ledger::tx_validity::*` | closed | |
| `AdmitOutcome` / `MempoolState` / `OrderPolicy` *(B2)* | `ade_ledger::mempool::*` | closed | |
| `LeaderScheduleAnswer` / `is_leader_for_vrf_output` *(N-B)* | `ade_core::consensus::leader_schedule` | closed | |
| `PraosNonces` / `NonceScanError` *(B1)* | `ade_ledger::consensus_input_extract` | | |
| `PraosChainDepState` / `ChainEvent` canonical encodings *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| `LedgerFingerprint` fold *(B3/B5)* | `ade_ledger::fingerprint` | | |
| **CI check set** | `ci/ci_check_*.sh` | **66 scripts (61 → 66 in PHASE4-N-L)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-L flipped 8 rules to `enforced`** (`CN-SESS-01..03`, `DC-SESS-01..05`); declared 1 new (`RO-LIVE-03`); strengthened 4 carried (`T-DET-01`, `CN-CONS-08`, `DC-NODE-01`, `DC-NODE-03`). Total: **223 entries** (214 → 223). | Append-only IDs. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | Decoder-driven; constrained by closed `PlutusLanguage`. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Era-versioned. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Shape closed; set open. |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed; instance set open. |
| Tx-body `proposal_procedures` instance set *(PP)* | `ade_types::conway::tx::ConwayTxBody.proposal_procedures` | `Option<Vec<ProposalProcedure>>`. Shape closed; instance set open. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap; inserts strictly increasing per `(pool, kes_period)`. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. |
| Withdrawals map *(B3)* | `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | Never last-wins. |
| Mempool admitted set *(B2)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>`; shape closed; monotonic. |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open. |
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ 2160. |
| `ServedChainSnapshot.blocks` admitted set *(N-G-S2)* | `ade_ledger::producer::served_chain::ServedChainSnapshot` | Shape closed; instance set open. |
| `PerPeerN2nServerState` instance set *(N-G-S6)* | `ade_runtime::network::n2n_server` | One instance per connected peer. |
| `PendingHeaderCache.entries` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache::PendingHeaderCache` | `BTreeMap<(SlotNo, Hash32), Vec<u8>>`. |
| `PerPeerReceiveState` instance set *(N-H-S4)* | `ade_runtime::receive::orchestrator` | One instance per upstream peer. |
| `InMemorySnapshotCache.entries` *(N-I-S4)* | `ade_runtime::rollback::in_memory_cache::InMemorySnapshotCache` | `BTreeMap<SlotNo, (LedgerState, PraosChainDepState)>`. Shape closed; instance set open. |
| Persistent snapshot store contents *(N-J-S8)* | the `SnapshotStore` instance backing `PersistentSnapshotCache` | `BTreeSet<SlotNo>` per `list_snapshot_slots`. |
| Per-peer collections in `OrchestratorState` *(N-K)* | `ade_runtime::orchestrator::state::OrchestratorState::{per_peer_receive, per_peer_server}` | `BTreeMap<PeerId, _>`. **N-L note:** the wire-layer per-pump state (`MuxTransportHandle`, `FrameBuffer`, `SessionState`) is held outside `OrchestratorState` — one set per mux pump task; lifetime managed by `n2n_dialer`/`n2n_server_pump`. Shape closed; instance set open. |
| Orchestrator-event corpus *(N-K — tooling-only)* | `corpus/orchestrator/` | Tooling-only. |
| **Session-replay corpus** *(NEW in N-L — tooling-only)* | `crates/ade_network/tests/session_replay_equivalence.rs` (inline) | Recorded `ByteChunkIn` sequences for replay-equivalence proof (DC-SESS-03). Tooling-only. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus / Consensus corpus / Block-validity corpus / Tx-validity corpus / Mempool ingress corpus / PP canonical corpus / Producer corpus / Server-paths corpus / Receive-paths corpus | various | Tooling-only. |
| Receive-rollback integration test *(N-I-S6)* | `crates/ade_runtime/tests/receive_rollback_integration.rs` | Tooling-only. |
| Persistent-cache inline test set *(N-J-S8)* | `crates/ade_runtime/src/rollback/persistent_cache.rs` (inline) | Tooling-only. |
| Orchestrator integration tests *(N-K — tooling-only)* | `crates/ade_runtime/tests/orchestrator_{peer_isolation,replay_equivalence}.rs` + `crates/ade_node/tests/{shutdown_resume_identity,authority_fatal_decode}.rs` | Tooling-only. |
| **Session integration test** *(NEW in N-L — tooling-only)* | `crates/ade_network/tests/session_replay_equivalence.rs` | Tooling-only. 2 tests. Defends DC-SESS-03. |
| Operator-action probe binaries *(N-B + N-E S6 + N-C S7 + N-G S7 + N-H S6)* | `ade_core_interop::bin::*` | RED operator-action; `#[ignore]`-gated. **N-L added no new binary.** |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. **N-L cargo dep change:** `ade_network/Cargo.toml` gained `sync` + `rt-multi-thread` features on its existing tokio dep (for `mpsc` bounded queues + `tokio::spawn` in `mux::transport`). The session core files MUST NOT import tokio — `ci_check_session_core_closure.sh` enforces. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **N-L SUBSUMED the prior revision's `RO-LIVE-01`/`RO-LIVE-02` "live mux + handshake driver" candidate pair** — the mechanical wire layer is now shipped. The operator-action live-pass half is `RO-LIVE-03`. | | |
| **NEW candidate — `PHASE4-N-L-LIVE` cluster (`RO-LIVE-03`)** *(NEW obligation at N-L close)* | **Operator-driven live pass capturing `RO-LIVE-03_<date>.log`** | One-slice follow-on cluster. `ade_node --peer ADDR` runs against a private cardano-node peer; tip-following pass captured to JSONL. Tracked on `RO-LIVE-03.open_obligation = "blocked_until_operator_peer_available"`. |
| **NEW candidate — TLS / authenticated transport cluster** *(declared `¬P-8` in N-L invariants sketch)* | **A new `Transport` impl + session-level auth gate** | Future cluster; curve25519 auth layer. Out of N-L scope. |
| **NEW candidate — N2C local protocols session driver cluster** *(flagged by N-L close)* | **Local-chain-sync / local-tx-submission / local-state-query family over UDS** | Future cluster; tracked alongside `CE-NODE-N2C-LTX`. |
| **NEW candidate — Peer-sharing + tx-submission live half cluster** *(flagged by N-L close)* | **Live tx submission + peer-sharing over an established N-L session** | Future cluster; tx-submission's live half requires mempool integration (PHASE4-N-E) as precondition. |
| **Snapshot schema migration v1 → v2 tooling cluster** *(carried from N-K — `DC-STORE-09.open_obligation`)* | Carried. | |
| **Metrics + observability cluster** *(carried from N-K)* | Carried. | |
| **Snapshot eviction policy cluster** *(carried from N-J)* | Carried. | |
| **Pre-Conway snapshot encoder cluster** *(carried from N-J)* | Carried. | |
| **Multi-peer fork choice cluster** *(carried; now further enabled by N-L live wire)* | Carried. | |
| **N2C local-chain-sync receive surface cluster** *(carried)* | Carried. | |
| **CE-N-H-6 / CE-N-G-8 / CE-N-C-8 operator-action live evidence** *(carried; CE-N-H-6 subsumed by `RO-LIVE-03` capture scope)* | Carried. | |
| **N-I+ Tier-5 — operator-tunable rollback policy** *(carried)* | Carried. | |
| **N-G+ Tier-5 — operator-tunable server policy** *(carried)* | Carried. | |
| **N-C+ Tier-5 — operator-tunable producer policy** *(carried)* | Carried. | |
| **CE-NODE-N2C-LTX** *(carried from N-E)* | Carried. | |
| **PP OQ-1..OQ-4** *(carried)* | Carried. | |
| N-A (deferred) | Peer address book | Runtime mutable. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

### Closed-grammar audit (PHASE4-N-L full close)

This sweep was performed after PHASE4-N-L full close.

1. **`session::core::step` chokepoint** — **closed by intent and
   CI-defended.** Sole `pub fn` reducing `(SessionState,
   ByteChunkIn)` in the workspace (CN-SESS-03 grep +
   forbidden-patterns check via `ci_check_session_core_closure.sh`).
2. **`AcceptedMiniProtocol` + `from_id` closed registry** — **closed
   by intent and CI-defended.** `from_id` closes with `_ => None`;
   dispatch site has no wildcard accept (DC-SESS-02 grep).
3. **`ByteChunkIn` / `SessionEffect` / `SessionError` /
   `HandshakeRole` closed sums** — **closed by intent.** Trait-less;
   data-only; no `#[non_exhaustive]`; no `String`.
4. **`SessionState` closed type-state** — **closed by intent and
   CI-defended.** `{ Handshaking, Connected }`. Mini-protocol frames
   rejected in `Handshaking` (DC-SESS-01 type-state-gate check).
5. **`Transport` sync trait + closed impl set** — **closed by
   intent.** `Pipe` (test) + `BlockingTransport` (production); new
   impl = deliberate registry-tracked closed addition. CN-SESS-02.
6. **`MuxTransportHandle` + `DuplexCapacity::DEFAULT` + fail-fast
   `TransportError::BackpressureExceeded`** — **closed by intent and
   CI-defended.** No unbounded mpsc anywhere in mux/session path
   (DC-SESS-04 grep via `ci_check_session_no_unbounded.sh`).
7. **`KeepAliveCadence::DEFAULT { interval_ms: 60_000 }`** —
   **closed by intent and CI-defended.** Compile-time-pinned; no
   operator-tunable cadence in this cluster (DC-SESS-05 — mirrors
   `SnapshotCadence::DEFAULT` discipline). Wire layer is
   wall-clock-free (`ci_check_clock_seam.sh` extension covers
   `ade_network::session/`).
8. **Mux frame chokepoint** — **now also CN-SESS-01-enforced.** Sole
   pub `encode_frame`/`decode_frame` pair via
   `ci_check_mux_frame_closure.sh`.
9. **Handshake transition chokepoint** — **now also
   CN-SESS-02-enforced.** Sole pub `n2n_transition`/`n2c_transition`
   pair via `ci_check_handshake_closure.sh`.

**Gap note — `RO-LIVE-03` live operator pass.** The mechanical wire
layer at this HEAD is ready and verified. Running `ade_node --peer
ADDR` against a private cardano-node peer + capturing the JSONL
log is the operator-action follow-on (`PHASE4-N-L-LIVE`). Tracked
on `RO-LIVE-03.open_obligation =
"blocked_until_operator_peer_available"`.

**Gap note — TLS / authenticated transport.** Declared `¬P-8` in
the N-L invariants sketch. Cardano N2N is plain TCP on testnets;
the curve25519 auth layer is a future cluster.

**Gap note — N2C local protocols.** Out of N-L scope. The session
reducer is N2N-scoped; the N2C family is tracked alongside
`CE-NODE-N2C-LTX` and a future N2C session cluster.

### Closed-grammar audit (carried — PHASE4-N-K / N-J / N-I / N-H / N-G / N-C / PROPOSAL-PROCEDURES-DECODE / N-E / B3 / B4 / B5)

All carried unchanged from prior revision.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags 0..=7.
- **`PreservedCbor<T>` invariant**.
- **Hash algorithms**: Blake2b-224 / 256, Ed25519, Byron-bootstrap,
  KES-sum, VRF-draft-03.
- **Era-correct block body hash** *(B1; strengthened in N-C, N-G, N-H)*.
- **Single canonical body-hash authority** *(N-C-S4 — DC-CONS-16)*: carried.
- **Single canonical header/body splitter** *(N-G-S1 — DC-CONS-18)*: carried.
- **Server-agency closure for outgoing mini-protocol messages** *(N-G-S1 — CN-PROTO-06)*: carried.
- **Receive-event closure for incoming peer signals** *(N-H-S1 — CN-PROTO-07)*: carried.
- **Type-level receive admission gate** *(N-H-S1 — CN-CONS-07 strengthening)*: carried.
- **Receive-side admission state-isolation discipline** *(N-H-S2 — CN-CONS-08 / DC-CONS-19)*: carried. **N-L note:** `CN-CONS-08.strengthened_in += PHASE4-N-L` — admit path now driven by real socket bytes end-to-end via the session reducer + mux pump.
- **Single canonical receive-side rollback materialization authority** *(N-I-S2 — CN-STORE-07)*: carried.
- **Replay-forward correctness** *(N-I-S2 — DC-CONS-22)*: carried.
- **Atomic rollback commit discipline** *(N-I-S3)*: carried.
- **Receive-side atomic admit + rollback over ChainDb + LedgerState + PraosChainDepState** *(N-H-S2 + N-I-S6 — DC-CONS-20)*: carried.
- **Receive-reducer rollback-context discipline** *(N-I-S6)*: carried.
- **Snapshot cadence determinism** *(N-I-S4 — DC-STORE-07)*: carried.
- **`ChainDbWrite::rollback_to_slot` trait method semantics** *(N-I-S3)*: carried.
- **Single canonical snapshot encoder authority** *(N-J-S7 — CN-STORE-08)*: carried.
- **Snapshot encoder canonicality** *(N-J — DC-STORE-08)*: carried.
- **Snapshot bytes version-tag + fingerprint discipline** *(N-J — DC-STORE-09)*: carried.
- **Snapshot encoder Conway-only scope** *(N-J)*: carried.
- **Persistent snapshot reader contract** *(N-J-S8)*: carried.
- **Single bootstrap composition root** *(N-K — CN-NODE-01)*: carried.
- **Per-peer task isolation discipline** *(N-K — DC-NODE-01)*: carried. **N-L note:** `DC-NODE-01.strengthened_in += PHASE4-N-L` — per-peer isolation now extends to the wire layer (each pump task owns its own `MuxTransportHandle` + `FrameBuffer` + `SessionState`; no shared mutable state).
- **Single cadence-driven persistent-capture authority** *(N-K — DC-NODE-02)*: carried.
- **Single wall-clock-reading site discipline** *(N-K — DC-NODE-03)*: carried. **N-L note:** `DC-NODE-03.strengthened_in += PHASE4-N-L` — clock-injection seam now covers keep-alive end-to-end; `ci_check_clock_seam.sh` now also covers `ade_network::session/`.
- **Closed authority-fatal exit-code surface** *(N-K — DC-NODE-04)*: carried.
- **Shutdown-then-resume byte-identical state contract** *(N-K — DC-NODE-04)*: carried.
- **Receive-side replay determinism** *(N-H-S3 — DC-PROTO-09)*: carried.
- **Per-peer receive-state independence across peers** *(N-H-S4)*: carried.
- **Key-boundary for receive paths** *(N-H-S4)*: carried.
- **Handshake-negotiated version threading through the receive reducer call site** *(N-H-S4 — DC-PROTO-06 strengthening)*: carried.
- **Served-bytes parity** *(N-G-S4 — DC-CONS-17)*: carried.
- **Header-body wire coherence** *(N-G-S5 — DC-CONS-18)*: carried.
- **Producer-side server-role transcript determinism** *(N-G-S5 — DC-PROTO-07)*: carried.
- **Deterministic-resolution discipline for server-agency waits** *(N-G-S3 — DC-PROTO-08)*: carried.
- **Type-level broadcast and serve gate** *(N-C-S5 — CN-CONS-07)*: carried.
- **Tx id over preserved body bytes** *(B2)*.
- **Conway certificate CDDL grammar** *(B3/B3F/B4)*.
- **Conway `DRep` decode grammar** *(B4)*.
- **Owner-tagged Conway cert-state apply contract** *(B4)*: DC-LEDGER-08.
- **Closed total gov-cert dispatch contract** *(B5)*: DC-LEDGER-09.
- **Fail-fast gov-cert environment** *(B5)*.
- **Checked DRep-expiry arithmetic** *(B5)*.
- **`ConwayGovState` deterministic-fold accumulation** *(B5)*.
- **Conway withdrawals map grammar** *(B3)*: never last-wins.
- **Closed deposit-effect sum types** *(B3)*.
- **Canonical deposit-param authority** *(B3)*: DC-TXV-07.
- **Full Conway value-conservation equation** *(B3)*.
- **`LedgerFingerprint` Conway deposit-param fold** *(B3)*.
- **Closed `proposal_procedures` wire grammar at Conway tx-body key 20** *(PP — DC-LEDGER-11)*.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor`.
- **Plutus language set**: V1, V2, V3.
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`.
- **Ouroboros mux frame layout**: 8-byte big-endian header. **N-L note:** now also CN-SESS-01-enforced (sole pub `encode_frame`/`decode_frame` pair).
- **11 closed mini-protocol message enums** + **8 closed state graphs**.
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*.
- **`EraSchedule` invariants** *(N-B)*.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*.
- **Consensus error taxonomies** *(N-B)*.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**.
- **`block_validity` composition contract** *(B1; N-I strengthened)*: carried.
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*.
- **`mempool_ingress` chokepoint contract** *(N-E)*.
- **`IngressSource` source-invariance contract** *(N-E)*.
- **Verbatim tx-bytes flow through ingress** *(N-E; N-H mirror)*: carried.
- **GREEN single-step replay fold contract** *(N-E — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(N-E; carried)*.
- **Operator-action evidence pattern** *(N-B / N-E / N-C / N-G / N-H)*: carried. **N-L adds one new live-evidence obligation (`RO-LIVE-03`)** but no new probe binary — the live pass runs the `ade_node` production binary directly.
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE / DREP / ENACTMENT / PP)*.
- **Committee-enactment write-back contract** *(ENACTMENT)*.
- **All canonical types**: shapes frozen at the era / version they entered.
- **Handshake-negotiated version threading** *(N-A; strengthened in N-G + N-H)*: carried. **N-L note:** the wire-layer session driver now negotiates the version end-to-end against a real peer via `n2n_dialer` + `BlockingTransport`.
- **TCB color assignments**: per `.idd-config.json` `core_paths`. **N-L additions:** `ade_network::session::{event, state, demux, core, handshake_driver}` are GREEN (no `tokio::*`, no clock-read). `ade_network::mux::transport` (extended), `ade_runtime::network::{mux_pump, n2n_dialer}`, and `ade_runtime::orchestrator::keep_alive_session` are RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D): carried.
- **`AcceptedBlock` type-level broadcast gate** *(N-C-S5)*: carried.
- **`AdmittedBlock` type-level admission gate** *(N-H-S1)*: carried.
- **`RollbackContext` BLUE struct seam** *(N-I-S6)*: carried.
- **`SnapshotCadence` BLUE-structural single-field discipline** *(N-I-S4 — DC-STORE-07)*: carried.
- **`forge_block` pure-transition contract** *(N-C-S3)*: carried.
- **Single source of leader truth** *(N-C-S3)*: carried.
- **Tx-admissibility prefix property** *(N-C-S3)*: carried.
- **Private-key custody RED-confinement** *(N-C-S1)*: carried.
- **Closed-grammar opcert byte authority** *(N-C-S2)*: carried.
- **OpCert serial counter strict monotonicity** *(N-C-S2)*: carried.
- **Single mux frame authority** *(NEW in N-L — CN-SESS-01)*: `ade_network::mux::frame::{encode_frame, decode_frame}` is the SOLE pub pair in the workspace. CI-defended via `ci_check_mux_frame_closure.sh`.
- **Single handshake authority** *(NEW in N-L — CN-SESS-02)*: `ade_network::handshake::transition::{n2n_transition, n2c_transition}` is the SOLE pub pair. CI-defended via `ci_check_handshake_closure.sh`.
- **Single session reducer authority** *(NEW in N-L — CN-SESS-03)*: `ade_network::session::core::step` is the SOLE pub fn reducing `(SessionState, ByteChunkIn)` in the workspace. CI-defended via `ci_check_session_core_closure.sh`.
- **Handshake-before-traffic type-state gate** *(NEW in N-L — DC-SESS-01)*: `SessionState::Handshaking` cannot deliver mini-protocol frames; encoded at the type level. CI-defended via `ci_check_session_core_closure.sh`.
- **Closed mini-protocol id registry** *(NEW in N-L — DC-SESS-02)*: `AcceptedMiniProtocol::from_id` closes with `_ => None`; dispatch site has no wildcard accept. Adding a mini-protocol is a single-variant + single match-arm addition. CI-defended via `ci_check_mini_protocol_id_registry_closed.sh`.
- **Session replay equivalence** *(NEW in N-L — DC-SESS-03)*: two-run byte-identity over recorded `ByteChunkIn` sequences. Proven by `tests/session_replay_equivalence.rs`.
- **Backpressure discipline** *(NEW in N-L — DC-SESS-04)*: bounded `MuxTransportHandle` mpsc + `DuplexCapacity::DEFAULT { 1024, 256, 16384 }` + fail-fast `TransportError::BackpressureExceeded`. CI-defended via `ci_check_session_no_unbounded.sh`.
- **Wire-layer clock injection** *(NEW in N-L — DC-SESS-05)*: session core wall-clock-free; keep-alive routes via `Clock`; `KeepAliveCadence::DEFAULT { interval_ms: 60_000 }` compile-time-pinned. CI-defended via `ci_check_clock_seam.sh` (extended).

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: full coordinated change.
- **New Conway certificate tag** *(B3 / B4 / B5)*.
- **New `CoinSource` deposit-provenance** *(B3)*.
- **Pre-Conway single-tx validity** *(B2 extension point)*.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*.
- **Conway block-body vkey-witness closure** *(B2-carried)*.
- **Conway governance certificate accumulation** *(B5)*.
- **Credential discriminant extension** *(declared non-goal)*.
- **Committee-enactment write-back** *(ENACTMENT)*.
- **Conway tx-body `proposal_procedures` decode** *(PP — wired)*.
- **TPraos full-block validity** *(B1 extension point)*.
- **TPraos producer** *(N-C declared non-goal — OQ-4 lock)*.
- **New `GovAction` / Plutus version variant**.
- **New `SignerSource` / `TxRejectClass` / `BlockRejectClass` / `OrderPolicy` variant**.
- **New protocol parameter field**.
- **New `ProducerTick` field** *(N-C extension point)*.
- **New `ForgeError` / `SchedulerInput` / `SchedulerEffect` variant**.
- **New `SelfAcceptError` variant** *(N-C extension point)*.
- **New `ServerStep` / `BlockFetchServerStep` / `ServerReply` etc.** *(N-G extension points)*: carried.
- **New `ReceiveEvent` variant** *(N-H — CN-PROTO-07 extension point)*: carried.
- **New `ReceiveEffect` variant** *(N-H)*: carried.
- **New `ReceiveError` variant** *(N-H)*: carried.
- **New `ChainDbWrite` impl** *(N-H; N-I extended to 2 methods)*: carried.
- **New `ChainDbWrite` trait method** *(N-H extension point)*: carried.
- **New `ReceiveDispatchError` variant** *(N-H)*: carried.
- **New `SnapshotReader` impl** *(N-I; N-J extended)*: carried.
- **New `BlockSource` impl** *(N-I)*: carried.
- **New `MaterializeError` / `CommitRollbackError` variant** *(N-I)*: carried.
- **New `RollbackContext` field** *(N-I)*: carried.
- **New `SnapshotCadence` field** *(N-I — WITH MANDATORY CLUSTER RATIFICATION)*: carried.
- **New `SnapshotEncodeError` / `SnapshotDecodeError` / `StructuralReason` variant** *(N-J — extension point)*: carried.
- **New snapshot sub-state encoder/decoder pair** *(N-J — extension point)*: carried.
- **`SCHEMA_VERSION` bump (v1 → v2)** *(N-J — extension point; carried `DC-STORE-09` open_obligation)*: carried.
- **New `PersistentSnapshotCache` field** *(N-J — extension point)*: carried.
- **New `PersistentCacheError` variant** *(N-J — extension point)*: carried.
- **New `OrchestratorEvent` variant** *(N-K — extension point)*: closed sum extension. **N-L exercised** by adding `OrchestratorEvent::OutboundKeepAlive { peer_id }` per the discipline (new variant + new reducer arm in `step` + new translation site in the runner).
- **New `OrchestratorEffect` variant** *(N-K — extension point)*: carried.
- **New `PeerHaltReason` discriminant** *(N-K — extension point)*: carried.
- **New `AuthorityFatalKind` discriminant** *(N-K — extension point)*: carried.
- **New `Clock` impl** *(N-K — extension point)*: carried.
- **New `BootstrapError` variant** *(N-K — extension point)*: carried.
- **New CI check**: additive. (N-L added five —
  `ci_check_mux_frame_closure.sh`, `ci_check_handshake_closure.sh`,
  `ci_check_session_core_closure.sh`,
  `ci_check_mini_protocol_id_registry_closed.sh`,
  `ci_check_session_no_unbounded.sh`; extended one —
  `ci_check_clock_seam.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
  **N-L:** `ade_network/Cargo.toml` gained `sync` + `rt-multi-thread`
  features on its existing tokio dep — additive feature flags, not
  a version bump.
- **New mini-protocol** / **Mini-protocol version-table bump**:
  **N-L closed the extension shape** — new mini-protocol = new
  `AcceptedMiniProtocol` variant + new `from_id` arm + new dispatch
  arm in `session::core::step`.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(N-E)*.
- **Live cardano-node N2N block-fetch acceptance / live N2N follow-mode admission** *(N-C / N-G / N-H / **N-L `RO-LIVE-03`**)*: each reopens on operator availability.
- **Phase-4 cluster surface additions** (N-F): each cluster's wire surface gates additions via its own cluster doc.
- **New `AcceptedMiniProtocol` variant** *(NEW in N-L — extension point)*: closed-sum extension. Single variant + single `from_id` arm + single dispatch arm in `session::core::step`. CI-defended.
- **New `ByteChunkIn` / `SessionEffect` / `SessionError` variant** *(NEW in N-L — extension point)*: closed-sum extension. New variant + matching reducer arm. CI-defended.
- **New `Transport` impl** *(NEW in N-L — extension point)*: deliberate registry-tracked closed addition alongside `Pipe` / `BlockingTransport`. Sync trait; new impls remain sync.
- **`KeepAliveCadence` operator-tunability** *(NEW in N-L — extension point; FORBIDDEN in this cluster)*: future cluster may relax to operator-tunable cadence with explicit Tier-5 rationale; currently pinned at compile time per DC-SESS-05.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-L added
five new GREEN files** (`ade_network::session::{event, state, demux,
core, handshake_driver}`) **and three new RED files + one extended**
(`ade_runtime::network::{mux_pump, n2n_dialer}`,
`ade_runtime::orchestrator::keep_alive_session`,
`ade_network::mux::transport` extended). **Five new CI gates + one
extended**. **Eight new registry rules flipped to `enforced`** plus
**one new `RO-LIVE-03` declared**. **Four carried rules
strengthened** (`strengthened_in += PHASE4-N-L`). N-L added **no
new BLUE**, **no new external ingress wire-format frozen contract**
(reuses the closed mux frame + handshake + mini-protocol codec
chokepoints), **no new operator-action probe binary**.

**N-L also strengthened the `ade_runtime → ade_network` cross-color
dependency edge**:

1. `ade_runtime → ade_network` (already established; **further
   strengthened in N-L**) — the RED `mux_pump` / `n2n_dialer` /
   `keep_alive_session` files import GREEN session module surfaces
   (`session::core::step`, `session::event::*`, `session::state::*`,
   `handshake_driver::*`) and the RED `mux::transport` surface.
   Direction unchanged.
2. The GREEN session files in `ade_network` import the existing BLUE
   `mux::frame` + `handshake::transition` + mini-protocol codecs;
   they never reach into RED siblings (`mux::transport`,
   `bin::capture_*`).

**The module-addition rule N-L sets for future wire-layer-side
work:**

1. **A new session-side GREEN primitive attaches inside
   `ade_network::session::*`** as a pure sync function over
   closed-sum events/effects. No `tokio::*`, no clock, no
   `HashMap`/`HashSet`/`rand`/float. New canonical types MUST be
   closed sums or closed structs; no `#[non_exhaustive]`; no
   `String`-bearing variants.
2. **A new mini-protocol attaches as a single-variant addition on
   `AcceptedMiniProtocol`** + a single `from_id` arm + a single
   dispatch arm in `session::core::step`. CI-defended; the
   `_ => None` close MUST be preserved.
3. **A new session event/effect/error variant attaches as a
   closed-sum extension** on `ByteChunkIn` / `SessionEffect` /
   `SessionError` plus matching reducer arm. Code change required;
   no plug-in registry.
4. **A new `Transport` impl attaches in the file declaring its
   composition site** (test impls under
   `ade_network::session::handshake_driver`; production impls under
   `ade_runtime::network::*`). The trait remains sync; new impls
   remain sync; async lives behind `tokio::task::spawn_blocking`.
5. **A new wire-layer RED runner attaches inside
   `ade_runtime::network::*`** (sibling to `mux_pump`, `n2n_dialer`)
   or `ade_runtime::orchestrator::*` (sibling to `keep_alive_session`).
   It translates RED-side asynchrony into closed-sum
   `OrchestratorEvent`s; it never bypasses the GREEN session
   reducer.
6. **`MuxTransportHandle` MUST remain bounded both ways**
   (DC-SESS-04). Any new transport variant MUST honour the bounded
   `mpsc` discipline and the fail-fast
   `TransportError::BackpressureExceeded` route. No
   `mpsc::unbounded_*` anywhere in the mux/session paths.
7. **`KeepAliveCadence::DEFAULT` MUST remain compile-time-pinned in
   this cluster** (DC-SESS-05). Operator-tunable cadence requires a
   future cluster with explicit Tier-5 rationale.
8. **A new wire-layer registry rule attaches as a derived
   `DC-SESS-*` / `CN-SESS-*` family entry** with `code_locus`,
   `ci_script`, `tests`, `cross_ref`. Bidirectional cross-refs to
   consumed rules (`T-DET-01`, `CN-CONS-08`, `DC-NODE-01`,
   `DC-NODE-03`).

### Cross-cluster obligation pattern (carried — `RO-LIVE-03` flagged by N-L close)

**N-L adds one new cross-cluster obligation in the live-evidence
sense** — `RO-LIVE-03` (`blocked_until_operator_peer_available`).
The mechanical wire layer is real and mechanically evidenced; the
operator-action follow-on is the one-slice cluster
`PHASE4-N-L-LIVE`. CE-N-H-6 is subsumed by `RO-LIVE-03` capture
scope (the live pass that captures `RO-LIVE-03` evidence will also
capture CE-N-H-6 evidence).

### Operator-action evidence pattern (carried — no N-L addition to the probe-binary family)

**N-L adds no new operator-action probe binary** — the family
remains at five. The live operator pass runs the `ade_node`
production binary directly with `--peer ADDR --listen ADDR`.

### Cluster scope-edge pattern (carried — strengthened in N-L close)

**N-L applies the scope-edge pattern to the wire-layer / live-pass
split**: the cluster ships the session reducer, handshake driver,
mux pump, n2n dialer, keep-alive session, and exposes them via the
binary; the live operator pass is the deliberate out-of-scope
follow-on tracked by `RO-LIVE-03`. The scope edge is documented in
the cluster doc's "Honest-scope carry-forwards" section and the
closure record.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-L:** no BLUE additions. | Other BLUE crates / submodules only. | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-L:** GREEN session files (`session/{event, state, demux, core, handshake_driver}.rs`) are pure sync: no `tokio::*`, no `SystemTime`/`Instant`, no `HashMap` (CI-defended by `ci_check_session_core_closure.sh` + `ci_check_clock_seam.sh` extension). | BLUE crates + standard library + ecosystem crates. **N-L:** the GREEN session files live inside `ade_network` (BLUE-with-per-file-RED-carve-out crate); per-file color is per the cluster TCB Color Map. | RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. **N-L:** `mux::transport` (extended), `ade_runtime::network::{mux_pump, n2n_dialer}`, and `ade_runtime::orchestrator::keep_alive_session` are RED. | Any BLUE / GREEN crate or submodule (one-way). **N-L:** new edge `ade_runtime::network::* → ade_network::{session, mux::transport, handshake}`. | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for wire-layer-side sub-modules, model the new CI gate
   on `ci_check_session_core_closure.sh` /
   `ci_check_mini_protocol_id_registry_closed.sh` shape
   (workspace-wide single-authority grep + forbidden-patterns
   check + closed-registry close-with-`_ => None` check).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   wire-layer authority rules, append `DC-SESS-0X` / `CN-SESS-0X`
   with bidirectional cross-ref to consumed rules.
7. **New operator-action probe binary:** (not applicable for the
   wire-layer domain — `ade_node` is the production entry; live
   evidence comes from running it against a real peer with
   `--peer ADDR`).
8. **Cross-cluster obligation:** the live operator pass is the named
   follow-on; tracked on `RO-LIVE-03`.
9. **Cluster scope-edge:** if the cluster deliberately scopes down a
   derived constraint, document the carve-out in CODEMAP + the
   cluster doc. N-L's "live pass deferred to `PHASE4-N-L-LIVE`" is
   the canonical example.
10. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-L — FULLY CLOSED at this HEAD** (mechanical close;
  honest-scope wire layer): session reducer + handshake driver +
  mux pump + n2n dialer + keep-alive session + `ade_node`
  `--peer ADDR --listen ADDR` CLI + 8 new registry rules
  (`enforced`) + 5 new CI scripts + 1 extended + 4 carried rules
  strengthened + 1 new `RO-LIVE-03` declared. Live operator pass
  is the one-slice follow-on cluster `PHASE4-N-L-LIVE`.
- **PHASE4-N-K — FULLY CLOSED** (carried).
- **PHASE4-N-J — FULLY CLOSED** (carried).
- **PHASE4-N-I — FULLY CLOSED** (carried).
- **PHASE4-N-H — FULLY CLOSED** (carried).
- **PHASE4-N-G — FULLY CLOSED** (carried).
- **PHASE4-N-C — FULLY CLOSED** (carried).
- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED** (carried).
- **PHASE4-N-E — FULLY CLOSED** (carried).
- **NEW future cluster — `PHASE4-N-L-LIVE` (`RO-LIVE-03`)**
  *(NEW obligation at N-L close)*: one-slice operator-action
  cluster running `ade_node --peer ADDR` against a private
  cardano-node peer; captures `RO-LIVE-03_<date>.log`. Highest
  priority post-N-L cluster for the bounty.
- **NEW future cluster — TLS / authenticated transport** *(declared
  `¬P-8` in N-L invariants sketch)*: curve25519 auth layer; future.
- **NEW future cluster — N2C local protocols session driver**
  *(flagged by N-L close)*: local-chain-sync / local-tx-submission /
  local-state-query family over UDS.
- **NEW future cluster — Peer-sharing + tx-submission live half**
  *(flagged by N-L close)*: requires mempool integration as
  precondition.
- **NEW future cluster — Snapshot schema migration v1 → v2 tooling**
  *(carried from N-K — `DC-STORE-09.open_obligation`)*.
- **NEW future cluster — Metrics + observability** *(carried from
  N-K)*: closed `MetricEffect` arm + Prometheus exporter. Tier-5.
- **NEW future cluster — Snapshot eviction policy** *(carried)*.
- **NEW future cluster — Multi-peer fork choice** *(carried; now
  further enabled by N-L live wire)*.
- **NEW future cluster — N2C local-chain-sync receive surface**
  *(carried)*.
- **NEW low-priority future cluster — Pre-Conway snapshot encoder**
  *(carried)*.
- **Future cluster — `CE-N-H-6` / `CE-N-G-8` / `CE-N-C-8` live
  evidence re-open triggers** (carried; CE-N-H-6 subsumed by
  `RO-LIVE-03` capture scope).
- **Future node-binary cluster (`CE-NODE-N2C-LTX`)** (carried).
- **Tx-validity completeness follow-ups** (carried).
- **PP OQ-1..OQ-4 follow-ups** (carried).
- **N-F (operator API)**: thin RED layer mapping a closed Query
  enum to gRPC/HTTP.

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`.
- No `SystemTime`, `Instant`, `std::time::*` clocks.
- No `rand::thread_rng`, `thread::spawn`.
- No `f32`, `f64`, floating-point arithmetic.
- No `std::fs`, `std::net`, `tokio`, `async fn`.
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist.
- No `#[cfg(feature = ...)]` semantic gating.
- No signing patterns in BLUE.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes only.
- No construction of `PreservedCbor` outside `ade_codec`.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs`.
- No `pallas_*` reference outside `ade_plutus`.
- **(N-A specific)** Carried.
- **(N-B specific)** Carried.
- **(B1 specific)** Carried.
- **(B2 specific)** Carried.
- **(B3 / B4 / B5 specific)** Carried.
- **(OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK)** Carried.
- **(N-E specific — closed BLUE chokepoint `mempool_ingress`)** Carried.
- **(PP specific — closed BLUE sub-grammar `decode_proposal_procedures`)** Carried.
- **(N-C-S1..S7 specific)** All carried.
- **(N-G-S1..S4 specific)** All carried.
- **(N-H-S1..S6 specific)** All carried.
- **(N-I-S1..S6 specific)** All carried.
- **(N-J-S1..S8 specific)** All carried.
- **(N-K specific)** No new BLUE.
- **(N-L specific)** No new BLUE in this cluster.

### GREEN (`ade_testkit` incl. all corpora; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges; `ade_runtime::producer::{tick_assembler, broadcast_to_served, served_chain_lookups}`; `ade_runtime::receive::{events_to_state, in_memory_chain_write}`; `ade_runtime::rollback::{cadence, in_memory_cache, chaindb_block_source, persistent_cache}`; `ade_runtime::bootstrap`; `ade_runtime::clock` [trait + `DeterministicClock`]; `ade_runtime::orchestrator::{mod, event, state, core}`; `ade_runtime::rollback::persistent_writer`; `ade_node::{cli, lib, node}`; **`ade_network::session::{event, state, demux, core, handshake_driver}` — NEW in N-L**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (carried bullets per prior revision)
- (N-K GREEN bullets per prior revision: `bootstrap`, `clock`,
  `orchestrator/*`, `persistent_writer`, `ade_node` — all carried.)
- **(`ade_network::session::core`, NEW in N-L)** Single `pub fn step`
  reducing `(SessionState, ByteChunkIn) -> Result<Vec<SessionEffect>,
  SessionError>` (CN-SESS-03). MUST NOT use `tokio::*`,
  `SystemTime::now()` / `Instant::now()`, `HashMap`, `rand`, float
  (CI-defended by `ci_check_session_core_closure.sh` +
  `ci_check_clock_seam.sh` extension). The handshake-before-traffic
  type-state gate MUST be preserved (DC-SESS-01).
- **(`ade_network::session::event`, NEW in N-L)** Closed event/effect/
  error vocabulary. `AcceptedMiniProtocol::from_id` MUST close with
  `_ => None` (DC-SESS-02). No `String`-bearing variants; no
  `#[non_exhaustive]`.
- **(`ade_network::session::state`, NEW in N-L)** Closed type-state
  `{ Handshaking, Connected }`. MUST NOT add a third state without a
  cluster ratification (DC-SESS-01 extension point).
- **(`ade_network::session::demux`, NEW in N-L)** `FrameBuffer` is a
  pure partial-frame accumulator. MUST NOT call into the BLUE
  `mux::frame::decode_frame` outside its boundary-detection use; MUST
  NOT mutate session state directly.
- **(`ade_network::session::handshake_driver`, NEW in N-L)** `Transport`
  is a **sync** trait (CN-SESS-02). New impls MUST remain sync; async
  lives behind `tokio::task::spawn_blocking` at the call site (the
  production `BlockingTransport` is the canonical example).

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session::*` non-driver siblings as applicable, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. five live-session probe binaries), the RED-behavior `ade_ledger::consensus_input_extract` scan; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` (N-C); `ade_runtime::network::n2n_server` (N-G-S6); `ade_runtime::receive::orchestrator` (N-H-S4); `ade_runtime::rollback::snapshot_writer` (N-I-S5); `ade_runtime::clock::SystemClock`; `ade_runtime::orchestrator::{peer_session, leadership_session, n2n_server_pump}`; `ade_node::main`; **`ade_network::mux::transport` (extended) — NEW in N-L (further RED extension)**; **`ade_runtime::network::{mux_pump, n2n_dialer}` — NEW in N-L**; **`ade_runtime::orchestrator::keep_alive_session` — NEW in N-L**)

- No direct mutation of `ade_ledger` state — all transitions go
  through the established BLUE chokepoints. **(N-L carve-out)** The
  RED wire-layer trio (`mux_pump`, `n2n_dialer`,
  `keep_alive_session`) translates RED-side asynchrony into
  `OrchestratorEvent`s / `ByteChunkIn`s and dispatches
  `SessionEffect`s; the GREEN session reducer is the SOLE mutator
  of `SessionState`. The runner never bypasses the reducer; the
  reducer never bypasses the BLUE chokepoints
  (`mux::frame::decode_frame`, `handshake::transition`, mini-protocol
  decoders).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
- (`ade_runtime` specifically) New edge: `ade_runtime::network::* →
  ade_network::{session, mux::transport, handshake}` (also GREEN →
  BLUE direction at the transitive level). Passes
  `ci_check_dependency_boundary.sh`.
- (carried RED bullets per prior revision: `SystemClock`, orchestrator
  runner trio, `ade_node::main`, `mux::transport`, `session::*`,
  capture binaries, genesis parser, consensus_input_extract,
  N-E live N2N, deferred N2C, `ade_core_interop`, N-C producer,
  N-G n2n_server, N-H receive orchestrator, N-I snapshot_writer —
  all carried.)
- **(`ade_network::mux::transport`, extended in N-L)** Now hosts
  `spawn_duplex` + `MuxTransportHandle` + `TransportError` +
  `DuplexCapacity::DEFAULT { 1024, 256, 16384 }`. MUST keep both
  inbound and outbound mpsc bounded (DC-SESS-04). MUST route
  overflow through `TransportError::BackpressureExceeded`
  (fail-fast; no silent drop). CI-defended by
  `ci_check_session_no_unbounded.sh`. Still no protocol logic — the
  mux frame parsing remains in `ade_network::mux::frame` (BLUE).
- **(`ade_runtime::network::mux_pump`, NEW in N-L)** Per-connection
  tokio task. May use `tokio::*`, `tokio::time::*`, and OS I/O.
  MUST translate inbound `MuxTransportHandle.rx` chunks into
  `ByteChunkIn` for `session::core::step`; MUST dispatch
  `SessionEffect`s via the outbound mpsc; MUST emit per-peer
  `OrchestratorEvent`s upstream. MUST NOT mutate `SessionState`
  directly — only the GREEN `step` reducer mutates state. Per-peer
  failure MUST halt only the failing pump task (DC-NODE-01
  strengthened).
- **(`ade_runtime::network::n2n_dialer`, NEW in N-L)** Outbound TCP +
  `spawn_duplex` + handshake driver via `BlockingTransport`. Emits
  `OrchestratorEvent::PeerConnected` on handshake success.
- **(`ade_runtime::orchestrator::keep_alive_session`, NEW in N-L)**
  Clock-driven (`ade_runtime::clock::Clock`) cadence pump emitting
  `OrchestratorEvent::OutboundKeepAlive { peer_id }` at
  `KeepAliveCadence::DEFAULT { interval_ms: 60_000 }` (DC-SESS-05).
  MUST NOT add operator-tunable cadence in this cluster. MUST NOT
  read the wall clock directly; consumes `Clock` outputs only.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-L:** the node binary's
  CLI accepts `--peer ADDR --listen ADDR` as operator-supplied
  values; no defaults; no hostnames embedded in source.
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-L:** the
  mechanical wire layer is exercised by `Pipe`-pair tests and
  recorded `ByteChunkIn` replay corpora; live evidence comes from
  the operator-action follow-on (`RO-LIVE-03`) running `ade_node`
  against a private cardano-node peer.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. **N-L:** the wire-layer / live-pass split
  (session reducer + handshake driver + mux pump + n2n dialer +
  keep-alive session shipped; live operator pass deferred to
  `PHASE4-N-L-LIVE`) is documented in the cluster doc + here and
  tracked by `RO-LIVE-03` — not a Tier-1 stub.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; the next CODEMAP regen
  picks up the new GREEN files (`session/{event, state, demux, core,
  handshake_driver}.rs`) and the new RED files
  (`mux_pump.rs`, `n2n_dialer.rs`, `keep_alive_session.rs`) and the
  extended `mux/transport.rs`. CI count moves from 61 → 66.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-L flipped to `enforced`:**
  `CN-SESS-01` (`ci_script = ci/ci_check_mux_frame_closure.sh`);
  `CN-SESS-02` (`ci_script = ci/ci_check_handshake_closure.sh`);
  `CN-SESS-03` (`ci_script = ci/ci_check_session_core_closure.sh`);
  `DC-SESS-01` (`ci_script = ci/ci_check_session_core_closure.sh`);
  `DC-SESS-02` (`ci_script =
  ci/ci_check_mini_protocol_id_registry_closed.sh`);
  `DC-SESS-03` (test: `tests/session_replay_equivalence.rs`);
  `DC-SESS-04` (`ci_script = ci/ci_check_session_no_unbounded.sh`);
  `DC-SESS-05` (`ci_script = ci/ci_check_clock_seam.sh` extended).
  **Declared:** `RO-LIVE-03`
  (`open_obligation = "blocked_until_operator_peer_available"`).
  **Strengthened:** `T-DET-01`, `CN-CONS-08`, `DC-NODE-01`,
  `DC-NODE-03` each gain `strengthened_in += PHASE4-N-L`. Total:
  214 → 223 entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D / N-A / N-B / N-H / N-I / N-J / N-K / B1 / B2 / B3 /
  B4 / B5 / OQ5-CREDENTIAL-FIDELITY / COMMITTEE-CRED-FIDELITY /
  DREP-VOTE-FIDELITY / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G: all closed;
  cluster docs carried.
- **Cluster PHASE4-N-L (CLOSED at this HEAD; mechanical wire layer
  closure; live operator pass deferred to `PHASE4-N-L-LIVE`)**: the
  cluster doc + closure record at
  `docs/clusters/completed/PHASE4-N-L/{cluster,CLOSURE}.md`. SHIPS
  the session reducer + handshake driver + mux pump + n2n dialer +
  keep-alive session + `ade_node --peer ADDR --listen ADDR` CLI;
  closes CN-SESS-01..03 + DC-SESS-01..05 (all `enforced`);
  strengthens 4 carried rules; declares `RO-LIVE-03`
  (`blocked_until_operator_peer_available`); adds five CI scripts
  + extends one (count 61 → 66); flips eight derived registry
  rules + declares one new (total 214 → 223). Five operator-action
  probe binaries remain in the family (no N-L addition); the live
  operator pass is the follow-on tracked on `RO-LIVE-03`.
- **Future obligation: `PHASE4-N-L-LIVE` (`RO-LIVE-03`)** —
  highest-priority operator-action follow-on.
- **Future obligation: TLS / authenticated transport** — declared
  `¬P-8` in N-L invariants sketch; future cluster.
- **Future obligation: N2C local protocols session driver** —
  flagged by N-L close.
- **Future obligation: peer-sharing + tx-submission live half** —
  flagged by N-L close.
- **Future obligation: snapshot schema migration v1 → v2 tooling**
  (`DC-STORE-09.open_obligation`) — carried from N-K.
- **Future obligation: metrics + observability cluster** — carried
  from N-K.
- **Future obligation: snapshot eviction policy cluster** — carried.
- **Future obligation: `CE-N-H-6`** — carried (subsumed by
  `RO-LIVE-03` capture scope).
- **Future obligation: `CE-N-G-8`** — carried.
- **Future obligation: `CE-N-C-8`** — carried.
- **Future obligation: `CE-NODE-N2C-LTX`** — carried from N-E.
- **Future seam candidates (flagged by N-L close)**: live operator
  pass `RO-LIVE-03` (highest-priority operator-action follow-on);
  TLS / authenticated transport; N2C local protocols session
  driver; peer-sharing + tx-submission live half; snapshot schema
  migration v1 → v2 tooling (`DC-STORE-09` open_obligation);
  metrics + observability cluster; snapshot eviction policy cluster
  (carried); pre-Conway snapshot encoder (low-priority, carried);
  multi-peer fork choice cluster (now further enabled by N-L live
  wire); N2C local-chain-sync receive surface cluster.
