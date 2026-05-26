# Module Authority Map — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/codemap.md`.

> 11 crates, 424 canonical types, 1703 tests, 66 CI checks at HEAD (`d62c2bc` + PHASE4-N-L worktree).

---

## Conventions

- A **module** in Ade is a Cargo workspace crate (smallest independently-buildable unit). One exception: `ade_network` is split by *submodule color* — its BLUE submodules, its GREEN submodules, and its RED submodules are documented as separate entries below, because `.idd-config.json` `core_paths` resolves BLUE at the submodule path level rather than crate-wide. The `ade_core::consensus` submodule sits *inside* the BLUE `ade_core` crate and is covered by that entry. The `ade_ledger::block_validity` / `ade_ledger::consensus_view` / `ade_ledger::tx_validity` / `ade_ledger::mempool::admit` / `ade_ledger::mempool::ingress` / `ade_ledger::cert_classify` modules sit inside the BLUE `ade_ledger` crate and are covered by that entry; the `ade_ledger::producer` sub-tree (PHASE4-N-C — `forge`, `self_accept`, `state`; PHASE4-N-G — `served_chain`), the `ade_ledger::block_body_hash` top-level module (PHASE4-N-C S4 single body-hash authority), the `ade_ledger::receive` sub-tree (PHASE4-N-H), the `ade_ledger::rollback` sub-tree (PHASE4-N-I — `traits`, `error`, `materialize`, `commit`), and the `ade_ledger::snapshot` sub-tree (PHASE4-N-J — `error`, `chain_dep`, `utxo_state`, `cert_state`, `epoch_state`, `gov_state`, `ledger`, `framing`) are likewise BLUE under the already-BLUE `ade_ledger` crate prefix. The RED `ade_ledger::consensus_input_extract`, the GREEN `ade_ledger::mempool::policy`, and the GREEN `ade_ledger::mempool::canonicalize` sit inside the BLUE `ade_ledger` crate but carry a different color by their own module doc-comment and the cluster TCB Color Maps — they are surfaced as sub-classification notes inside the `ade_ledger` entry. The `ade_testkit::mempool` sub-tree (PHASE4-N-E S2), the `ade_testkit::governance` sub-tree (PROPOSAL-PROCEDURES-DECODE PP-S2), and the `ade_testkit::producer` sub-tree (PHASE4-N-C) sit inside the GREEN `ade_testkit` crate, classified GREEN by their own module doc-comments and the cluster TCB Color Maps. **PHASE4-N-G — `ade_runtime::network` sub-tree** sits inside the RED `ade_runtime` crate and hosts the per-peer N2N **server** session driver (RED). **PHASE4-N-H — `ade_runtime::receive` sub-tree** sits inside the RED `ade_runtime` crate and hosts the GREEN+RED receive-side glue: `events_to_state` (GREEN), `in_memory_chain_write` (GREEN), `orchestrator` (RED). **PHASE4-N-I — `ade_runtime::rollback` sub-tree** sits inside the RED `ade_runtime` crate and hosts the GREEN+RED rollback adapter glue: `cadence` (GREEN), `in_memory_cache` (GREEN), `chaindb_block_source` (GREEN), `snapshot_writer` (RED). **PHASE4-N-J extends `ade_runtime::rollback` with `persistent_cache` (GREEN; closes DC-CONS-21).** **PHASE4-N-K extends `ade_runtime::rollback` with `persistent_writer` (GREEN; cadence-fidelity glue calling `PersistentSnapshotCache::capture` — DC-NODE-02), introduces the new top-level files `ade_runtime::bootstrap` (GREEN; sole `pub fn bootstrap_initial_state` — CN-NODE-01) and `ade_runtime::clock` (GREEN trait + GREEN `DeterministicClock` + the RED `SystemClock` sub-classified inside the same file — DC-NODE-03), and a new sub-tree `ade_runtime::orchestrator` (mixed) hosting the GREEN core reducer `core` + GREEN closed-vocabulary `event` + GREEN `state` + barrel `mod`, alongside the RED tokio runners `peer_session` + `leadership_session` + `n2n_server_pump`. PHASE4-N-K reshapes `ade_node` from a hello-world stub into a lib+bin**: `src/lib.rs` (RED library entry; re-exports `Cli`/`CliError`/`run_node_until_shutdown`/`NodeStartupInputs`/`NodeShutdownEvidence`/`NodeRunError` + exit-code constants), `src/cli.rs` (RED argv parser), `src/node.rs` (RED `run_node_until_shutdown` lifecycle), and the refactored `src/main.rs` (RED bin shim). **NEW — PHASE4-N-L promotes `ade_network::session` from an empty RED placeholder to a populated GREEN sub-tree by content** (6 files: `mod`, `event`, `state`, `demux`, `core`, `handshake_driver` — pure reducer + closed `AcceptedMiniProtocol` registry + `SessionState` type-state + partial-frame buffer + handshake driver over an opaque `Transport` trait). The `.idd-config.json` `_core_paths_doc` still classifies `session/` as RED, but every PHASE4-N-L file carries a GREEN module banner and is gated by `ci/ci_check_session_core_closure.sh` + `ci/ci_check_clock_seam.sh` (extended); the GREEN-by-content sub-classification is documented in `session/mod.rs` and surfaced here as a new GREEN module entry. **NEW — PHASE4-N-L extends `ade_network::mux::transport` (still RED) with `MuxTransportHandle` + closed `TransportError` sum + `DuplexCapacity::DEFAULT` + `spawn_duplex` while preserving the old `MuxTransport` / `open_tcp` API.** **NEW — PHASE4-N-L adds two RED files inside `ade_runtime::network/` (`mux_pump.rs` + `n2n_dialer.rs`) and one RED file inside `ade_runtime::orchestrator/` (`keep_alive_session.rs`).** **NEW — PHASE4-N-L adds one `OrchestratorEvent` variant `OutboundKeepAlive { peer_id }` (additive; closed enum re-closed).**
- Modules are listed by TCB color (BLUE → GREEN → RED), alphabetical within each color.
- TCB color sources, in order of authority:
  1. `.idd-config.json` `core_paths` — substring match against absolute path. BLUE matches: `ade_codec`, `ade_types`, `ade_crypto`, `ade_core`, `ade_ledger` (covers `ade_ledger::{snapshot, rollback, receive, producer, block_validity, tx_validity, mempool, ...}`), `ade_plutus`, and the 9 `ade_network` submodule paths (`mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`).
  2. `.idd-config.json` `_core_paths_doc` — `ade_runtime` is RED; `ade_testkit` is GREEN; `ade_node` is RED; `ade_network::mux::transport` and `ade_network::session` are nominally RED (PHASE4-N-L: `session/` is now GREEN-by-content — see the new GREEN entry below); `ade_network::lib` and `ade_network::mux::mod` are GREEN barrels. `ade_core_interop` is **RED** per its `Cargo.toml` header comment and the PHASE4-N-B TCB Color Map; the `ade_core_interop::tx_submission` (S4) and `ade_core_interop::local_tx_submission` (S5) modules carry a GREEN sub-classification by their own module doc-comments. The RED binaries `ade_core_interop::bin::{live_tx_submission_session, live_block_production_session, live_block_fetch_session, live_block_follow_session}` are the operator-action live evidence pattern.
  3. `docs/clusters/completed/PHASE4-N-B/cluster.md` § "TCB Color Map" — `era_schedule`, `praos_state`, `vrf_cert`, `nonce`, `op_cert`, `leader_schedule`, `header_validate`, `fork_choice`, `rollback` (consensus-side header rollback applier) are BLUE; `chain_selector`, `candidate_fragment` are GREEN; `genesis_parser` is RED; `ade_testkit::consensus` is GREEN; `ade_core_interop` is RED.
  4. PHASE4-B1 — `ade_ledger::consensus_view`, `ade_ledger::block_validity` BLUE; `ade_core::consensus::{header_validate, kes_check}` extensions BLUE; `ade_testkit::validity` GREEN; `ade_ledger::consensus_input_extract` RED.
  5. PHASE4-B2 — `ade_ledger::tx_validity::*`, `ade_ledger::mempool::admit` BLUE; `ade_ledger::mempool::policy` GREEN (Tier-5); `ade_testkit::tx_validity` GREEN.
  6. PHASE4-B3/B5 — all new/changed modules BLUE under already-BLUE crate prefixes.
  7. PHASE4-N-E — `ade_ledger::mempool::ingress` BLUE (S1); `ade_ledger::mempool::canonicalize` GREEN (S3); `ade_testkit::mempool::ingress_replay` GREEN (S2); `ade_core_interop::tx_submission` GREEN (S4); `ade_core_interop::local_tx_submission` GREEN (S5); `ade_core_interop::bin::live_tx_submission_session` RED (S6).
  8. PROPOSAL-PROCEDURES-DECODE — `ade_codec::conway::governance` BLUE; new `ProposalProcedure` in `ade_types::conway::governance` BLUE; `ade_testkit::governance::proposal_procedures_replay` GREEN.
  9. PHASE4-N-C — `ade_ledger::producer::{forge, self_accept, state}` BLUE; `ade_ledger::block_body_hash` BLUE; `ade_codec::shelley::{opcert, tx_components}` BLUE; `ade_core::consensus::opcert_validate` BLUE; `ade_crypto::kes::KesSignature` BLUE; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` RED; `ade_runtime::producer::tick_assembler` GREEN; `ade_testkit::producer::*` GREEN; `ade_core_interop::bin::live_block_production_session` RED.
  10. PHASE4-N-G — `ade_network::chain_sync::server`, `ade_network::block_fetch::server` BLUE; `ade_ledger::producer::served_chain` BLUE; `accepted_block_header_bytes` accessor BLUE; `ade_runtime::producer::{broadcast_to_served, served_chain_lookups}` GREEN; `ade_runtime::network::n2n_server` RED; `ade_core_interop::bin::live_block_fetch_session` RED.
  11. PHASE4-N-H — `ade_ledger::receive::{admitted, chain_write, events, pending_header_cache, reducer}` BLUE; `ade_runtime::receive::{events_to_state, in_memory_chain_write}` GREEN; `ade_runtime::receive::orchestrator` RED; `ade_core_interop::bin::live_block_follow_session` RED.
  12. PHASE4-N-I — `ade_ledger::rollback::{traits, error, materialize, commit}` BLUE; `ChainDbWrite::rollback_to_slot` extension BLUE; the `RollbackContext<'a>` + reducer extensions BLUE; `ade_runtime::rollback::{cadence, in_memory_cache, chaindb_block_source}` GREEN; `ade_runtime::rollback::snapshot_writer` RED.
  13. PHASE4-N-J — `ade_ledger::snapshot::{error, chain_dep, utxo_state, cert_state, epoch_state, gov_state, ledger, framing}` BLUE; `ade_runtime::rollback::persistent_cache` GREEN.
  14. `docs/clusters/completed/PHASE4-N-K/cluster.md` § "TCB color map (FC/IS partition)" — `ade_runtime::bootstrap` GREEN (CN-NODE-01). `ade_runtime::clock` trait + `DeterministicClock` GREEN; `ade_runtime::clock::SystemClock` (same file) RED (DC-NODE-03). `ade_runtime::orchestrator::{mod, event, state, core}` GREEN (`ci/ci_check_orchestrator_core_purity.sh`). `ade_runtime::rollback::persistent_writer` GREEN (DC-NODE-02). `ade_runtime::orchestrator::{peer_session, leadership_session, n2n_server_pump}` RED (DC-NODE-01). `ade_node::{cli, node, lib, main}` RED. No new BLUE in this cluster.
  15. **NEW — `docs/clusters/completed/PHASE4-N-L/cluster.md` § "TCB color map (FC/IS partition)"** — `ade_network::session::{mod, core, event, state, demux, handshake_driver}` are **GREEN** (pure reducer + closed `AcceptedMiniProtocol` registry + `SessionState` type-state + partial-frame buffer + handshake driver over an opaque `Transport` trait; gated by `ci/ci_check_session_core_closure.sh` + `ci/ci_check_mini_protocol_id_registry_closed.sh` + `ci/ci_check_clock_seam.sh` extended). `ade_network::mux::transport` (extended) remains **RED** (now hosts `MuxTransportHandle` + bounded `spawn_duplex` + closed `TransportError` sum + `DuplexCapacity::DEFAULT`; gated by `ci/ci_check_session_no_unbounded.sh`). `ade_runtime::network::{mux_pump, n2n_dialer}` are **RED** (per-connection tokio task + outbound TCP + handshake driver call + `PeerConnected` event emission). `ade_runtime::orchestrator::keep_alive_session` is **RED** (Clock-driven ping pump; DC-SESS-05 consumer). **No new BLUE in this cluster.**

- **Active cluster at HEAD (none).** PHASE4-N-L (mux session driver + handshake) is **closed at HEAD `d62c2bc` + PHASE4-N-L worktree** and archived under `docs/clusters/completed/PHASE4-N-L/` (closure record at `docs/clusters/completed/PHASE4-N-L/CLOSURE.md`). The cluster ships the GREEN session reducer (`session::core::step`), the closed `AcceptedMiniProtocol` mini-protocol id registry, the `Handshaking`/`Connected` `SessionState` type-state, the partial-frame `FrameBuffer`, the `Transport`-trait handshake driver, the RED full-duplex bounded `MuxTransportHandle` + `spawn_duplex`, the RED per-connection `MuxPump`, the RED outbound `N2nDialer`, and the RED Clock-driven `KeepAliveSession`. New registry rules at `status = "enforced"`: `CN-SESS-01` (single mux frame authority — sole pub `encode_frame`/`decode_frame` pair), `CN-SESS-02` (single handshake authority — sole pub `n2n_transition` / `n2c_transition`), `CN-SESS-03` (single session reducer authority — sole pub `step` in `session::core`), `DC-SESS-01` (handshake-before-traffic — `SessionState::Handshaking` cannot deliver mini-protocol frames), `DC-SESS-02` (closed mini-protocol id registry — `AcceptedMiniProtocol::from_id` closes with `_ => None`), `DC-SESS-03` (session replay equivalence — two-run byte-identity), `DC-SESS-04` (backpressure discipline — bounded mpsc + `TransportError::BackpressureExceeded`), `DC-SESS-05` (wire-layer clock injection — session core wall-clock-free; keep-alive routes via `Clock`). `RO-LIVE-03` carries `open_obligation = "blocked_until_operator_peer_available"` (mechanical wire layer ready; live operator pass is the follow-on `PHASE4-N-L-LIVE` cluster). Existing rules strengthened (`strengthened_in += "PHASE4-N-L"`): `T-DET-01` (byte-stream → orchestrator-event determinism now proven end-to-end via the session reducer), `CN-CONS-08` (admit path now driven by real socket bytes end-to-end via mux_pump → orchestrator), `DC-NODE-01` (per-peer isolation extends to the wire layer — each pump task owns its own `MuxTransportHandle` + `SessionState`), `DC-NODE-03` (clock-injection seam covers keep-alive). **Family choice note (carried from CLOSURE.md):** `CN-NET-*` and `DC-NET-01` were already in use (operator-topology rules from the classification table; `DC-NET-01` = three-tier peer management). PHASE4-N-L's session-layer rules therefore use `CN-SESS-*` and `DC-SESS-*` to avoid ID collision while remaining append-only. Existing operator-action `open_obligation`s carried unchanged: `CN-CONS-06` (live producer stake; via N-C), `RO-LIVE-01` (live block-fetch evidence; via N-G), `RO-LIVE-02` (live cross-impl follow evidence; via N-H) — closing these is now the same operator-action follow-on cluster (`PHASE4-N-L-LIVE`) that runs `ade_node --peer ADDR --listen ADDR` against a private cardano-node peer. `docs/clusters/completed/` now contains 21 directories (the 20 prior closures plus PHASE4-N-L).

- **Delta since prior CODEMAP HEAD `d62c2bc` (this regeneration — PHASE4-N-L cluster, 9 slices S1–S9).**

  **Structural deltas summarized:**
  - **NEW GREEN sub-tree `ade_network/src/session/`** (6 files, ~1635 LOC; previously an empty `mod.rs` placeholder).
    - `session/mod.rs` (37 LOC) — barrel; declares `pub mod {core, demux, event, handshake_driver, state}`; re-exports `step`, `FrameBuffer`, `AcceptedMiniProtocol`, `ByteChunkIn`, `SessionEffect`, `SessionError`, `HandshakeRole`, `Transport`, `TransportError`, `NegotiatedN2n`, `run_n2n_handshake_initiator`, `run_n2n_handshake_responder`, `SessionState`, `ConnectedState`, `HandshakeProgress`. The module doc-comment explicitly records the GREEN-by-content sub-classification despite the upstream `.idd-config.json` RED listing.
    - `session/event.rs` (246 LOC, S2) — closed 6-variant `AcceptedMiniProtocol` enum (`Handshake` / `ChainSync` / `BlockFetch` / `TxSubmission` / `KeepAlive` / `PeerSharing`) with `from_id(MiniProtocolId) -> Option<Self>` closing with `_ => None` (DC-SESS-02); closed `ByteChunkIn { peer_id, bytes }` input variant; closed `SessionEffect` sum (per-protocol outbound frame emit + halt); closed `SessionError` sum; closed 2-variant `HandshakeRole` (`Initiator`/`Responder`).
    - `session/state.rs` (85 LOC, S2) — closed `SessionState::{Handshaking(HandshakeProgress)|Connected(ConnectedState)}` type-state (DC-SESS-01 — only the `Connected` arm exposes the per-protocol fanout, so the type system makes "deliver a mini-protocol frame to a still-handshaking session" unrepresentable); `HandshakeProgress { … }`; `ConnectedState { negotiated_version, per_protocol_frame_buffers }`.
    - `session/demux.rs` (217 LOC, S3) — `FrameBuffer { accumulated: Vec<u8> }` partial-frame accumulator; `feed(&mut self, chunk: &[u8]) -> Vec<MuxFrame>` consults `ade_network::mux::frame::decode_frame` to extract complete frames and retains the partial remainder.
    - `session/core.rs` (518 LOC, S2) — pure `pub fn step(state: &mut SessionState, input: ByteChunkIn) -> Result<Vec<SessionEffect>, SessionError>` reducer (CN-SESS-03); composes `mux::frame::decode_frame` + `FrameBuffer::feed` + per-protocol fanout into `AcceptedMiniProtocol`; the dispatch table is a closed `match` over `AcceptedMiniProtocol` (no wildcard accept — DC-SESS-02). 12 inline tests.
    - `session/handshake_driver.rs` (332 LOC, S4) — `Transport` trait (`read_exact` / `write_all` / `close`; abstract over tokio sockets vs in-memory buffers for replay); closed `TransportError` sum (`Eof` / `ProtocolViolation` / `BackpressureExceeded` / `Io`); `NegotiatedN2n { version, params }`; `run_n2n_handshake_initiator<T: Transport>(transport, propose) -> Result<NegotiatedN2n, TransportError>`; `run_n2n_handshake_responder<T: Transport>(transport, accept) -> Result<NegotiatedN2n, TransportError>`. Routes every state advance through `handshake::n2n_transition` (no parallel reducer — CN-SESS-02).
  - **EXTENDED RED file `ade_network/src/mux/transport.rs`** (40 → 231 LOC, S5) — gains `MuxTransportHandle` (bounded inbound + outbound mpsc pair), closed `TransportError` sum (`BackpressureExceeded` / `PeerClosed` / `IoError(io::ErrorKind)` — no `String`), `DuplexCapacity { inbound_chunks: usize, outbound_chunks: usize, read_buffer_bytes: usize }` with `pub const DEFAULT: Self = DuplexCapacity { inbound_chunks: 1024, outbound_chunks: 256, read_buffer_bytes: 16384 }`, and `fn spawn_duplex(stream: TcpStream, capacity: DuplexCapacity) -> MuxTransportHandle` (DC-SESS-04 — bounded queues only; `ci/ci_check_session_no_unbounded.sh` forbids any `mpsc::unbounded_channel` in this file). The pre-existing `MuxTransport` / `open_tcp` / `read_raw` / `write_raw` API is preserved unchanged.
  - **NEW RED files in `ade_runtime/src/network/`.**
    - `mux_pump.rs` (289 LOC, S6) — `MuxPump { handle: MuxTransportHandle, session: SessionState, peer_id: PeerId, events_out: mpsc::Sender<OrchestratorEvent> }` per-connection tokio task; `run(self) -> Result<(), DialError>` async loop pulling inbound chunks from `MuxTransportHandle.inbound.recv()`, feeding `session::core::step`, translating each `SessionEffect` into an outbound `MuxTransportHandle.outbound.send(bytes)` or into an `OrchestratorEvent::PeerChainSyncFrame` / `PeerBlockFetchFrame` / `PeerSessionHalted` forwarded to the orchestrator inbox.
    - `n2n_dialer.rs` (268 LOC, S7) — `N2nDialer { /* propose params */ }`; closed `DialError` sum (`Connect(io::ErrorKind)` / `Handshake(TransportError)` / `OrchestratorClosed`); `dial(peer_addr) -> Result<(NegotiatedN2n, MuxTransportHandle), DialError>` does `TcpStream::connect` + `spawn_duplex` + `run_n2n_handshake_initiator` over an internal `BlockingTransport` adapter, emits `OrchestratorEvent::PeerConnected { peer_id, chain_sync_version, block_fetch_version, role: UpstreamClient }` to the orchestrator inbox, then spawns a `MuxPump` task on the negotiated session.
  - **NEW RED file in `ade_runtime/src/orchestrator/`.**
    - `keep_alive_session.rs` (149 LOC, S8) — `KeepAliveCadence { interval_ms: u64 }`; `KeepAliveSession<C: Clock> { clock: C, cadence: KeepAliveCadence, peer_ids: BTreeSet<PeerId>, events_out: mpsc::Sender<OrchestratorEvent> }`; `run(self)` pulls `clock.next_tick()` and emits `OrchestratorEvent::OutboundKeepAlive { peer_id }` per peer per cadence-aligned tick. Consumes time only via `Clock` (DC-SESS-05 — extended `ci/ci_check_clock_seam.sh` now also scopes `ade_network/src/session/`).
  - **NEW `OrchestratorEvent` variant** (additive, S8) — `OrchestratorEvent::OutboundKeepAlive { peer_id: PeerId }`. The orchestrator `core::step` records it (no immediate `OrchestratorEffect` emitted — encoding the keep-alive into a wire frame at the session reducer is deferred to a future cluster). The N-K-closed 8-variant `OrchestratorEvent` becomes a 9-variant closed enum at HEAD; no `_` arm anywhere.
  - **NEW integration test.** `crates/ade_network/tests/session_replay_equivalence.rs` (140 LOC, S9; 2 tests) — DC-SESS-03 byte-identical effects across two replays of the same byte-chunk corpus driven through `session::core::step`.
  - **NEW closed canonical types (none counted toward the BLUE total).** In `ade_network::session` (GREEN): `AcceptedMiniProtocol`, `ByteChunkIn`, `SessionEffect`, `SessionError`, `HandshakeRole`, `SessionState`, `HandshakeProgress`, `ConnectedState`, `FrameBuffer`, `Transport` (trait), `TransportError` (session-side variant), `NegotiatedN2n`. In `ade_network::mux::transport` (RED): `MuxTransportHandle`, `TransportError` (mux-side variant), `DuplexCapacity`. In `ade_runtime::network` (RED): `MuxPump`, `N2nDialer`, `DialError`, `BlockingTransport` (private helper). In `ade_runtime::orchestrator::keep_alive_session` (RED): `KeepAliveCadence`, `KeepAliveSession<C>`. **All inside `ade_network`'s session/mux paths (RED-classified in `core_paths_doc` but partly GREEN-by-content) or RED `ade_runtime` paths — not counted in the BLUE canonical-type total. BLUE canonical type count unchanged at 424.**
  - **NEW CI gates (+5; +1 extended).** `ci/ci_check_mux_frame_closure.sh` (S1; CN-SESS-01 — single pub `encode_frame`/`decode_frame` pair in the workspace), `ci/ci_check_handshake_closure.sh` (S1; CN-SESS-02 — single pub `n2n_transition` + single pub `n2c_transition`), `ci/ci_check_session_core_closure.sh` (S2; CN-SESS-03 + DC-SESS-01 — `session::core::step` is the only pub reducer in `session/`; `Handshaking`/`Connected` type-state structurally present; session core files contain no tokio imports), `ci/ci_check_mini_protocol_id_registry_closed.sh` (S1; DC-SESS-02 — `AcceptedMiniProtocol` closed; dispatch is a closed `match`), `ci/ci_check_session_no_unbounded.sh` (S5; DC-SESS-04 — no `mpsc::unbounded_channel` / `unbounded`-named constructor in session / mux_pump / n2n_dialer / keep_alive_session files). **EXTENDED:** `ci/ci_check_clock_seam.sh` now also scopes `crates/ade_network/src/session/` for the wire-side wall-clock-free guarantee (DC-SESS-05). **CI script count: 61 → 66 (+5; the clock-seam extension is in-place).**
  - **NEW registry rules / strengthenings.** 9 new entries — `CN-SESS-01` (enforced), `CN-SESS-02` (enforced), `CN-SESS-03` (enforced), `DC-SESS-01` (enforced), `DC-SESS-02` (enforced), `DC-SESS-03` (enforced), `DC-SESS-04` (enforced), `DC-SESS-05` (enforced), `RO-LIVE-03` (declared; `open_obligation = "blocked_until_operator_peer_available"`). 4 existing rules strengthened: `T-DET-01`, `CN-CONS-08`, `DC-NODE-01`, `DC-NODE-03` all gain `PHASE4-N-L` in `strengthened_in`. Registry count: 214 → 223 (+9).
  - **NEW test inventory (+31).** Per-crate at HEAD: `ade_codec` 162 (unchanged), `ade_types` 23 (unchanged), `ade_crypto` 51 (unchanged), `ade_core` 126 (unchanged), `ade_ledger` 574 (unchanged), `ade_plutus` 28 (unchanged), `ade_testkit` 312 (unchanged), `ade_runtime` 164 → **171 (+7)** (mux_pump inline 2 + n2n_dialer inline 2 + keep_alive_session inline 3 = 7), `ade_network` 200 → **224 (+24)** (session/core inline 12 + session/event inline 4 + session/state inline 1 + session/demux inline 3 + session/handshake_driver inline 2 + mux/transport inline +0 (extended, no new inline) + integration `session_replay_equivalence` 2 = 24), `ade_node` 5 (unchanged), `ade_core_interop` 27 (unchanged). **Test inventory 1672 → 1703 (+31)**, reported approximate per the template's fallback rule (counts from `grep -cE "#\[test\]|#\[tokio::test\]"` workspace-wide).
  - **NEW crate-level dependency change.** `ade_network/Cargo.toml` gained tokio features `sync` + `rt-multi-thread` on its existing tokio dep (for `mpsc` bounded queues + `tokio::spawn` in `mux::transport`). The GREEN session core files MUST NOT import `tokio::*` — `ci/ci_check_session_core_closure.sh` enforces structurally. No new outbound workspace edges.

- Counts:
  - Crates: 11, from `Cargo.toml` `[workspace] members`. Unchanged.
  - Canonical types: 424, from `grep -rE "^(pub )?(struct|enum) "` across the full BLUE scope. Breakdown: `ade_codec` 10, `ade_types` 81, `ade_crypto` 13, `ade_core` 44, `ade_ledger` 159, `ade_plutus` 8, plus the 9 BLUE `ade_network` submodule paths 109. Registry `canonical_type_registry: null`, so a structural count is used. **No change since `d62c2bc`** — PHASE4-N-L added no BLUE canonical types; the new ~20 closed sums/structs all live inside `ade_network::session` (GREEN-by-content; not on the BLUE submodule-path list) or RED paths.
  - Tests: 1703 — count of `#[test]` / `#[tokio::test]` attributes across `crates/`. Reported as approximate per the template's fallback rule. **+31 since `d62c2bc`**, breakdown above.
  - CI checks: 66 — file count under `ci/ci_check_*.sh`. **+5 since `d62c2bc`** — `ci_check_mux_frame_closure.sh`, `ci_check_handshake_closure.sh`, `ci_check_session_core_closure.sh`, `ci_check_mini_protocol_id_registry_closed.sh`, `ci_check_session_no_unbounded.sh` (all PHASE4-N-L); `ci_check_clock_seam.sh` is extended in-place to cover `ade_network::session/`. Full inventory at HEAD: `admitted_block_closure`, `block_fetch_server_closure`, `bootstrap_closure`, `broadcast_to_served_purity`, `cbor_round_trip`, `ce_n_a_5_proof`, `chaindb_contract`, `chaindb_crash_safety`, `chain_sync_server_closure`, `clock_seam` *(extended N-L)*, `consensus_closed_enums`, `constitution_coverage`, `conway_cert_classification_closed`, `credential_discriminant_closed`, `crypto_vectors`, `dependency_boundary`, `deposit_param_authority`, `differential_divergence`, `forbidden_patterns`, `forge_purity`, `gov_cert_accumulation_closed`, **`handshake_closure`** *(NEW)*, `hash_uses_wire_bytes`, `hfc_translation`, `ingress_chokepoints`, `ledger_determinism`, `mempool_ingress_closure`, `mempool_ingress_replay`, **`mini_protocol_id_registry_closed`** *(NEW)*, `module_headers`, **`mux_frame_closure`** *(NEW)*, `n2n_server_no_signing_dep`, `no_async_in_blue`, `no_chaindb_in_consensus_blue`, `node_binary_uses_single_bootstrap`, `no_density_in_fork_choice`, `no_float_in_consensus`, `no_parallel_header_splitter`, `no_private_keys_in_corpus`, `no_producer_body_encoder`, `no_secrets`, `no_semantic_cfg`, `no_signing_in_blue`, `opcert_closed`, `orchestrator_core_purity`, `pallas_quarantine`, `peer_session_isolation`, `persistent_writer_no_parallel_cadence`, `private_key_custody`, `producer_corpus_present`, `proposal_procedures_closed`, `receive_orchestrator_no_producer_dep`, `receive_paths_corpus_present`, `receive_reducer_closure`, `receive_replay_purity`, `recovery_contract`, `ref_provenance`, `rollback_materialize_closure`, `scheduler_closure`, `self_accept_gate`, `served_chain_closure`, `server_paths_corpus_present`, **`session_core_closure`** *(NEW)*, **`session_no_unbounded`** *(NEW)*, `snapshot_cadence_purity`, `snapshot_encoder_closure`. The forward-looking `ci/ci_check_no_fail_open_in_validation.sh` (DC-VAL-06 / DC-TXV) is still **not** shipped. No `.github/workflows/` yet.

---

## BLUE Modules — Pure Functional Core

> **Shared header (applies to every BLUE entry below).** Every `.rs` source file begins with the contract banner
> `// Core Contract:` and the following deny attributes are present in each crate's `lib.rs` (or, for `ade_network`,
> at the crate root — the BLUE submodules inherit them):
> `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`,
> `#![deny(clippy::panic)]`, `#![deny(clippy::float_arithmetic)]`.
> CI scripts that enforce the shared rules (all 7 scope the full BLUE set — the 6 BLUE crates plus the 9 BLUE `ade_network` submodule paths declared in `.idd-config.json` `core_paths`):
> - `ci/ci_check_module_headers.sh` — banner first-line check.
> - `ci/ci_check_forbidden_patterns.sh` — `HashMap`, `HashSet`, `IndexMap`/`IndexSet`/`indexmap::`, `SystemTime`, `Instant`, `std::fs`, `std::net`, `tokio`, `async fn`, `f32`/`f64`, `anyhow`, `rand::thread_rng`, `thread::spawn`, plus `unsafe` outside a documented allowlist.
> - `ci/ci_check_dependency_boundary.sh` — no BLUE crate depends on a RED crate.
> - `ci/ci_check_no_signing_in_blue.sh` — `SigningKey`/`SecretKey`/`PrivateKey`/`private_key`/`sign_message`/`sign_block` forbidden in BLUE.
> - `ci/ci_check_no_semantic_cfg.sh` — `#[cfg(feature = ...)]` and `cfg!(feature = ...)` forbidden in BLUE.
> - `ci/ci_check_hash_uses_wire_bytes.sh` — no hashing of `canonical_bytes` / re-encoded bytes in BLUE.
> - `ci/ci_check_ingress_chokepoints.sh` — only named `decode_*` chokepoints construct `PreservedCbor`.
> - `ci/ci_check_pallas_quarantine.sh` — `pallas-*` references confined to `ade_plutus`.
> - `ci/ci_check_no_async_in_blue.sh` *(PHASE4-N-A, S-A1)* — async constructs forbidden anywhere in the BLUE scope. Enforces DC-CORE-01.
>
> Three additional CI scripts narrow the shared header to the `ade_core::consensus` tree (PHASE4-N-B): `ci/ci_check_no_chaindb_in_consensus_blue.sh`, `ci/ci_check_no_float_in_consensus.sh`, `ci/ci_check_consensus_closed_enums.sh` (TARGETS scope: `crates/ade_core/src/consensus/`, `crates/ade_ledger/src/block_validity/`, `crates/ade_ledger/src/tx_validity/`, `crates/ade_ledger/src/mempool/`).
> A fourth narrow check enforces a single fork-choice rule: `ci/ci_check_no_density_in_fork_choice.sh` (DC-CONS-03).
> A fifth narrow check (PHASE4-B3) enforces deposit-parameter authority: `ci/ci_check_deposit_param_authority.sh` (DC-TXV-07).
> A sixth narrow check (PHASE4-B5) enforces gov-cert accumulation closure: `ci/ci_check_gov_cert_accumulation_closed.sh` (DC-LEDGER-09).
> A seventh narrow check (OQ5/COMMITTEE/DREP/ENACTMENT-COMMITTEE-FIDELITY + ENACTMENT-COMMITTEE-WRITEBACK) enforces credential-discriminant closure: `ci/ci_check_credential_discriminant_closed.sh` (DC-LEDGER-10).
> An eighth narrow check (PROPOSAL-PROCEDURES-DECODE, PP-S1) enforces proposal_procedures closure: `ci/ci_check_proposal_procedures_closed.sh` (DC-LEDGER-11).
>
> **Eight narrow checks added by PHASE4-N-C (the producer-authority gate set):** `ci_check_forge_purity.sh` (DC-CONS-13/14/15); `ci_check_no_producer_body_encoder.sh` (DC-CONS-16); `ci_check_no_private_keys_in_corpus.sh` (DC-CRYPTO-03/04/05 corpus discipline); `ci_check_opcert_closed.sh` (DC-CONS-11/12); `ci_check_private_key_custody.sh` (DC-CRYPTO-03/04/05 + OP-OPS-04); `ci_check_producer_corpus_present.sh` (CN-CONS-06 mechanical half); `ci_check_scheduler_closure.sh` (DC-CONS-13 + DC-MEM-03 + OP-OPS-05); `ci_check_self_accept_gate.sh` (CN-CONS-07).
>
> **Seven narrow checks added by PHASE4-N-G (the producer-side server-role gate set):** `ci_check_no_parallel_header_splitter.sh`; `ci_check_served_chain_closure.sh`; `ci_check_chain_sync_server_closure.sh`; `ci_check_block_fetch_server_closure.sh`; `ci_check_broadcast_to_served_purity.sh`; `ci_check_n2n_server_no_signing_dep.sh`; `ci_check_server_paths_corpus_present.sh`.
>
> **Four narrow checks added by PHASE4-N-H (the receive-side authority gate set):** `ci_check_admitted_block_closure.sh`; `ci_check_receive_reducer_closure.sh`; `ci_check_receive_replay_purity.sh`; `ci_check_receive_orchestrator_no_producer_dep.sh`; `ci_check_receive_paths_corpus_present.sh`.
>
> **Two narrow checks added by PHASE4-N-I (the rollback authority gate set):** `ci_check_rollback_materialize_closure.sh` (CN-STORE-07 + DC-CONS-22); `ci_check_snapshot_cadence_purity.sh` (DC-STORE-07).
>
> **One narrow check added by PHASE4-N-J (the snapshot encoder single-authority gate):** `ci_check_snapshot_encoder_closure.sh` (CN-STORE-08 + DC-STORE-08 + DC-STORE-09).
>
> **Six narrow checks added by PHASE4-N-K (the node-orchestrator gate set; all scope `ade_runtime` GREEN files + `ade_node` source — none target BLUE crates):** `ci_check_bootstrap_closure.sh` (CN-NODE-01); `ci_check_clock_seam.sh` (DC-NODE-03 — extended in N-L to also cover `ade_network::session/`); `ci_check_orchestrator_core_purity.sh` (DC-NODE-03 + general purity); `ci_check_persistent_writer_no_parallel_cadence.sh` (DC-NODE-02); `ci_check_peer_session_isolation.sh` (DC-NODE-01); `ci_check_node_binary_uses_single_bootstrap.sh` (CN-NODE-01 + DC-NODE-04).
>
> **NEW — Five narrow checks added by PHASE4-N-L (the wire-session gate set; all scope `ade_network::session/` + `ade_network::mux/` + `ade_runtime::{network, orchestrator}` paths — none target BLUE crates):**
> - `ci/ci_check_mux_frame_closure.sh` (S1; CN-SESS-01) — repo-wide grep gate asserting a single pub `encode_frame` / `decode_frame` pair in the workspace.
> - `ci/ci_check_handshake_closure.sh` (S1; CN-SESS-02) — repo-wide grep gate asserting a single pub `n2n_transition` and a single pub `n2c_transition`.
> - `ci/ci_check_session_core_closure.sh` (S2; CN-SESS-03 + DC-SESS-01 + DC-SESS-05 session-side) — `session::core::step` is the only pub reducer in `session/`; `Handshaking`/`Connected` type-state structurally present; session core files contain no `tokio::*` imports.
> - `ci/ci_check_mini_protocol_id_registry_closed.sh` (S1; DC-SESS-02) — `AcceptedMiniProtocol` enum is closed; the dispatch table is a `match` over it with no wildcard accept.
> - `ci/ci_check_session_no_unbounded.sh` (S5; DC-SESS-04) — no `mpsc::unbounded_channel` / `unbounded`-named queue constructor in `session/` / `mux_pump.rs` / `n2n_dialer.rs` / `keep_alive_session.rs`.
>
> Two checks narrow the shared header to the wire-ingress path of `ade_ledger::mempool` (PHASE4-N-E): `ci_check_mempool_ingress_closure.sh` (DC-MEM-03, S1); `ci_check_mempool_ingress_replay.sh` (DC-MEM-04, S2 + S3).
>
> A BLUE crate or BLUE `ade_network` submodule that adds a feature flag, an async function, a `HashMap`, or a RED dep fails CI on push.
> The 3 RED-scope CI scripts (`ci_check_chaindb_contract.sh`, `ci_check_recovery_contract.sh`, `ci_check_chaindb_crash_safety.sh`) and the 1 evidence script `ci_check_ce_n_a_5_proof.sh` are not part of this shared header — they are documented in the cross-module CI matrix at the bottom.

---

### `ade_codec`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch `ade_codec`. The existing snapshot-encoder consumption sites carry forward unchanged.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns Cardano-canonical CBOR ingress: the only place in the workspace that turns raw bytes into typed semantic values, with wire-byte preservation for every hash-bearing structure. Also owns the standalone opcert byte authority (`shelley::opcert::{encode_opcert, decode_opcert}`) and the canonical Conway-tx preserved-byte splitter (`shelley::tx_components::split_conway_tx_components`). |
| **Creates** | `PreservedCbor<T>`, `RawCbor`, `BlockEnvelope`, `ByronDecodedBlock`, `CodecContext`, `CodecError` (incl. `UnknownCertTag`, `DuplicateMapKey`, `TrailingBytes`, `InvalidCborStructure`), `ContainerEncoding`, `IntWidth`, plus era-tagged block/tx wrappers under `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. Functions: `conway::cert::{decode_conway_certs, decode_drep}`, `conway::withdrawals::{decode_withdrawals, withdrawals_sum}`, `conway::governance::{decode_proposal_procedures, encode_proposal_procedures}`, `shelley::cert::read_pool_registration_cert`, both era `decode_stake_credential`, `shelley::opcert::{encode_opcert, decode_opcert, write_opcert_fields_into}`, `shelley::tx_components::split_conway_tx_components`. Types: `OpCertCodecError`, `TxComponents<'a>`. |
| **Interprets** | All canonical Cardano CBOR — block envelopes, era-specific blocks, tx bodies, tx outs, certificates, addresses. Conway certificate array (closed over CDDL tags 0..18, owner-complete) and Conway withdrawals map (deduplicated). Both era credential decoders preserve the key/script tag. Sole authority for `PreservedCbor::new` (constructor is `pub(crate)`). CIP-1694 `proposal_procedure` array. Standalone cardano-cli `OperationalCertificate` 4-tuple. Conway transaction 4-tuple (preserved-byte slicing only). |
| **MUST NOT** | (1) Construct `PreservedCbor` outside `ade_codec` (`pub(crate)` + `ci_check_ingress_chokepoints.sh`). (2) Re-encode wire bytes when computing hashes (`ci_check_hash_uses_wire_bytes.sh`). (3) Use any forbidden BLUE pattern. (4) Depend on any other workspace crate except `ade_types`. (5) `conway::cert` (DC-LEDGER-08) — no unknown-tag swallow; owner-complete; no catch-all. (6) `conway::withdrawals` — no last-wins on duplicate `RewardAccount`. (7) `decode_stake_credential` (DC-LEDGER-10) — must not erase the credential tag. (8) `conway::governance` (DC-LEDGER-11) — no silent skip on unknown `GovAction`; no opaque pass-through at body codec key 20. (9) `shelley::opcert` (DC-CONS-11/12) — cardano-byte-identical 4-tuple; dedicated `OpCertCodecError` variant per shape failure; not a second header-embedded opcert encoder (enforced by `ci_check_opcert_closed.sh`). (10) `shelley::tx_components` (DC-CONS-13/16) — preserved-byte slices that alias the input buffer (no `to_vec` / clones); reject non-4-tuple shapes; not re-encode the boolean validity flag or the auxiliary-data null. |
| **Inbound deps** | `ade_ledger` (heavy — `ade_ledger::rollback::materialize` uses `cbor::envelope::decode_block_envelope` for replay-forward era detection; `ade_ledger::receive::reducer` reuses `block_validity::decode_block`; `ade_ledger::snapshot::*` uses `ade_codec::cbor::{canonical_width, read_*, write_*, ContainerEncoding, IntWidth, MAJOR_NEGATIVE}` + `ade_codec::CodecError` across all 8 sub-state encoder/decoder files), `ade_plutus`, `ade_testkit`, `ade_network`, `ade_runtime`, `ade_core_interop`, `ade_node`. No new inbound crate edge in PHASE4-N-L. |
| **Outbound deps** | `ade_types`. No external dependencies; std-only. Dev-deps: `serde_json`, `toml`. |
| **Entry points** | `ade_codec::cbor::envelope::decode_block_envelope`, `ade_codec::cbor`, `ade_codec::traits::AdeEncode`, `ade_codec::CodecContext`, per-era `decode_*_block`, `ade_codec::address::decode_address`. B2: `ade_codec::conway::tx::decode_conway_tx_body`. B3: `ade_codec::conway::cert::decode_conway_certs` and `ade_codec::conway::withdrawals::{decode_withdrawals, withdrawals_sum}`. PP-S1: `ade_codec::conway::governance::{decode_proposal_procedures, encode_proposal_procedures}`. N-C: `ade_codec::shelley::opcert::{encode_opcert, decode_opcert, OpCertCodecError}`, `ade_codec::shelley::tx_components::{split_conway_tx_components, TxComponents}`. |
| **Key modules** | `cbor/`, `byron/`, `shelley/` (incl. `cert.rs`, `opcert.rs`, `tx_components.rs`), `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`, `address/`, `preserved.rs`, `traits.rs`, `primitives.rs`, `error.rs`. |

---

### `ade_core`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch `ade_core`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | BLUE authoritative Praos consensus core. Owns canonical types and pure state-transitions that decide which header / chain Ade accepts: HFC era schedule + slot↔era↔time translation, Praos chain-dep state (nonce / op-cert counters), VRF cert verification + leader-eligibility predicate, KES signature + op-cert period verification wired into header admission, header validation pipeline, fork-choice, header-level rollback authority, leader-schedule query, canonical encodings of all chain-dep state and chain events. Owns the producer-side opcert acceptance authority (`opcert_validate`). |
| **Creates** | **Schedule:** `BootstrapAnchorHash`, `EraSchedule`, `EraSummary`, `EraLocation`. **State:** `PraosChainDepState`, `OpCertCounterMap`, `Nonce`. **Events/points:** `Point`, `ChainHash`, `BlockDistance`, `SecurityParam`, `ChainEvent`, `ChainSelectionReject`. **Errors:** `HFCError`, `SlotTimeError`, `OutsideForecastRange`, `HeaderValidationError`, `FieldError`, `FieldKind`, `VrfCertError`, `OpCertCounterError`, `NonceEvolutionError`, `LeaderScheduleError`, `OpCertError`. **Header surface:** `HeaderInput`, `HeaderVrf`, `HeaderKes`, `ValidatedHeaderSummary`, `HeaderApplied`. **Fork-choice:** `TiebreakerView`, `CandidateFragment`, `ChainSelectorState`, `ForkChoiceError`. **Op-cert/nonce:** `OpCertObservation`, `NonceInput`. **VRF:** `VrfRole`, `VerifiedVrf`, `StakeFraction`, `ActiveSlotsCoeff`. **Leader schedule:** `LeaderScheduleQuery`, `LeaderScheduleAnswer`. **Ledger view boundary:** `LedgerView` trait. **Rollback (header-level):** `RollBackRequest`, `RollBackApplied`. **Encoding:** `DecodeError`. 44 public types. |
| **Interprets** | Canonical inputs from the `ade_runtime` shell — `HeaderInput` projections, `EraSchedule` materialized once from genesis JSON, `LedgerView` snapshots from `ade_ledger::consensus_view::PoolDistrView`, ordered `StreamInput` events. KES check verifies hot KES key signature over the header body CBOR bytes. `opcert_validate` consumes an `OperationalCert`, cold key, expected period, and prev counter. |
| **MUST NOT** | (1) Take or accept a `&ChainDb` reference (`ci_check_no_chaindb_in_consensus_blue.sh`). (2) Use any `f32`/`f64` (`ci_check_no_float_in_consensus.sh`). (3) Add `#[non_exhaustive]` to any consensus enum, open-tail `Other`/`Unknown`, owned `String` fields, or `Box<dyn ...>` (`ci_check_consensus_closed_enums.sh`). (4) Reference `density` in `fork_choice.rs`/`candidate.rs` outside a `// no-density:` annotation. (5) Read wall-clock. (6) Use `HashMap`/`HashSet`; `OpCertCounterMap` uses `BTreeMap`. (7) `async fn`, `.await`, `tokio`. (8) Construct a `PreservedCbor`. (9) Re-derive stake snapshots (DC-CONSENSUS-02). (10) Bypass the canonical `validate_and_apply_header` pipeline. (11) B1 fail-closed (DC-VAL-06). (12) B1 KES rule (DC-CRYPTO-01). (13) All shared-header BLUE rules. (14) `consensus::opcert_validate` (DC-CONS-11/12) — `OpCertError` closed; sub-check order pinned; `prev_counter == None` only first-opcert acceptance path; no external counter store. |
| **Inbound deps** | `ade_ledger`, `ade_runtime` (heavy), `ade_testkit`, `ade_core_interop`, `ade_node`. No new inbound crate edge in PHASE4-N-L. |
| **Outbound deps** | `ade_types`, `ade_crypto`. Dev-deps: `ade_testkit`, `serde_json`, `cardano-crypto`. |
| **Entry points** | `use ade_core::consensus::{...}` aggregator, `ade_core::consensus::ledger_view::LedgerView`, `ade_core::consensus::vrf_cert::*`, `ade_core::consensus::kes_check::*`, `ade_core::consensus::praos_state::*`, `ade_core::consensus::header_summary::*`. Top-level transitions: `validate_and_apply_header`, `select_best_chain`, `apply_rollback` (header-level), `apply_nonce_input`, `apply_op_cert`, `query_leader_schedule`, `verify_vrf_cert`, `tiebreaker_prefer`, `encode/decode_chain_dep_state`, `encode/decode_chain_event`. `ade_core::consensus::opcert_validate::{opcert_validate, OpCertError}`. |
| **Key modules** | `consensus/era_schedule.rs`, `consensus/praos_state.rs`, `consensus/events.rs`, `consensus/errors.rs`, `consensus/vrf_cert.rs`, `consensus/kes_check.rs`, `consensus/nonce.rs`, `consensus/op_cert.rs`, `consensus/leader_schedule.rs`, `consensus/header_summary.rs` + `consensus/header_validate.rs`, `consensus/candidate.rs`, `consensus/fork_choice.rs`, `consensus/rollback.rs`, `consensus/ledger_view.rs`, `consensus/encoding.rs`, `consensus/opcert_validate.rs`. |

---

### `ade_crypto`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch `ade_crypto`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns pure cryptographic verification primitives — Blake2b hashing, Ed25519 / Byron-bootstrap signature verification, KES verification with operational certificates, VRF verification, plus the closed signature-artifact types (`KesSignature`, `VrfProof`, `VrfOutput`, `Ed25519Signature`) that BLUE consumes across the RED→BLUE boundary. Verification only — signing lives in `ade_runtime::producer::signing`. |
| **Creates** | `Blake2b224`, `Blake2b256`, `HashAlgorithm` trait, `Ed25519VerificationKey`, `Ed25519Signature`, `ByronExtendedVerificationKey`, `KesVerificationKey`, `KesPeriod`, `OperationalCertData`, `VrfVerificationKey`, `VrfProof`, `VrfOutput`, `CryptoError`, `KesSignature(pub [u8; SUM6_KES_SIG_LEN])`, `SUM6_KES_SIG_LEN: usize = 448`. |
| **Interprets** | Verification key / signature / proof byte structures. Not a CBOR parser — accepts already-decoded byte slices. |
| **MUST NOT** | (1) Implement signing (`ci_check_no_signing_in_blue.sh`). (2) Allocate global state. (3) Use any BLUE forbidden pattern. (4) Use `unsafe` outside the allowlisted FFI in `src/vrf.rs`. (5) `build_opcert_signable` must produce the spec-correct raw concatenation. (6) `KesSignature` (DC-CRYPTO-04) — closed length-pinned wrapper; only `from_bytes` construction; custom redacting `Debug`; no `PartialOrd`/`Ord`/`Hash` derives. (7) No `Drop` impl that zeroizes — `KesSignature` is BLUE and not secret. |
| **Inbound deps** | `ade_core`, `ade_ledger`, `ade_plutus`, `ade_testkit`, `ade_core_interop`, `ade_runtime`. No new inbound crate edge in PHASE4-N-L. |
| **Outbound deps** | `ade_types`, `blake2`, `ed25519-dalek`, `cardano-crypto` (vrf-draft03 + kes-sum + dsign features, `default-features = false`). |
| **Entry points** | `ade_crypto::blake2b::*`, `verify_ed25519`, `verify_byron_bootstrap`, `verify_kes`, `verify_opcert`, `verify_vrf`, `ade_crypto::kes::{KesSignature, SUM6_KES_SIG_LEN}`. |

---

### `ade_ledger`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L added no `ade_ledger` source. All BLUE authorities carry forward; the new wire layer feeds the existing `admit_via_block_validity` chokepoint via the same N-K orchestrator dispatch.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The functional core (ledger half): stateless ledger rules for every Cardano era; the B1 top-level block-validity verdict; the B2 top-level transaction-validity verdict + mempool admission; the B3 full Conway value-conservation accounting; the B4 closed Conway cert-state accumulation; the B5 closed Conway governance-cert accumulation; the live committee-enactment write-back; the single BLUE chokepoint `mempool::ingress::mempool_ingress` from wire ingress into `admit`; the BLUE producer authority; the BLUE producer-side served-chain index and single canonical header-projection authority; the BLUE receive-side header→body bridge authority; the BLUE rollback authority; the BLUE canonical snapshot encoder/decoder authority. |
| **Creates** | All carry-forward (159 BLUE canonical types). No additions in PHASE4-N-L. |
| **Interprets** | Carry-forward. **PHASE4-N-L consumption note (carry-forward only):** the new `MuxPump` translates inbound mini-protocol frames into `OrchestratorEvent::Peer{ChainSync,BlockFetch}Frame`, which then routes through the existing N-K `dispatch_*_inbound` wrappers to `admit_via_block_validity` — no new BLUE entry point. |
| **MUST NOT** | All carry-forward (1)–(50). No PHASE4-N-L additions. |
| **Inbound deps** | `ade_testkit`, `ade_core_interop`, `ade_runtime`, `ade_node`. No new inbound crate edge in PHASE4-N-L. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `ade_plutus`, `ade_core`, `minicbor`, `num-bigint`, `num-integer`, `num-traits`. Dev-dep: `ade_testkit`. No new outbound crate edge in PHASE4-N-L. |
| **Entry points** | Carry-forward (all N-J + earlier surfaces). |
| **Key modules** | All carry-forward. No new module added in PHASE4-N-L. |

> **GREEN sub-classification (`ade_ledger::mempool::policy`, `ade_ledger::mempool::canonicalize`).** Carry-forward.
> **RED sub-classification (`ade_ledger::consensus_input_extract`).** Carry-forward.
> **`ade_ledger::producer::served_chain` BLUE classification (N-G).** Carry-forward.
> **`ade_ledger::receive` BLUE classification (N-H).** Carry-forward.
> **`ade_ledger::rollback` BLUE classification (N-I).** Carry-forward.
> **`ade_ledger::snapshot` BLUE classification (N-J).** Carry-forward.

---

### `ade_network` *(BLUE submodules)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch any BLUE submodule. The BLUE submodule set in `.idd-config.json` `core_paths` (9 paths: `mux/frame.rs` + `codec/` + `handshake/` + `chain_sync/` + `block_fetch/` + `tx_submission/` + `keep_alive/` + `peer_sharing/` + `n2c/`) remains the canonical source of truth — `session/` is **not** on this list (the PHASE4-N-L GREEN-by-content sub-tree is documented as its own GREEN module entry below). The new GREEN session reducer composes the BLUE authorities `mux::frame::{encode_frame, decode_frame}` (CN-SESS-01) + `handshake::n2n_transition` (CN-SESS-02) + the per-mini-protocol state machines without re-implementing any of them.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the Cardano Ouroboros mini-protocol authority — the closed wire grammar (CBOR codecs) and pure state machines for all 11 N2N + N2C mini-protocols, plus the BLUE Ouroboros mux frame primitive. Also owns the producer-side server-role reducer surface (PHASE4-N-G `chain_sync::server` + `block_fetch::server`). |
| **Creates** | **Wire grammar (`codec/`):** 11 closed mini-protocol message enums + closed payload types + structured `CodecError` + `ProtocolKind`. **State machines:** one closed `*State` / `*Agency` / `*Output` / `*Error` quad per protocol. **Mux primitives (`mux/frame.rs`):** `MuxFrame`, `MuxHeader`, `MuxMode`, `MiniProtocolId`, `MuxError`. **PHASE4-N-G (`chain_sync/server.rs` + `block_fetch/server.rs`):** carry-forward. |
| **Interprets** | The on-wire CBOR of every Ouroboros mini-protocol message (closed grammar) and the 8-byte Ouroboros mux frame header. Block/header/tx-body bytes inside RollForward / Block / Tx are opaque pass-through (DC-PROTO-06); LSQ Query/Result payloads also opaque. PHASE4-N-G server reducers interpret already-decoded `ChainSyncMessage` / `BlockFetchMessage` values. |
| **MUST NOT** | (1) `async fn`, `.await`, `tokio`, `async_std`, `futures::`, async-runtime spawn, or async timers in any file under the 9 BLUE paths. (2) Decode block / header / tx-body bytes. (3) Construct a `PreservedCbor` outside `ade_codec`'s named chokepoints. (4) Add `#[non_exhaustive]` to any wire-message enum or introduce a generic `Codec<P>` trait. (5) Define a generic `Agency<P>` wrapper. (6) Read a selected version from ambient session state. (7) Redefine `TxId` / `SlotNo` / `Hash32` / `CardanoEra`. (8) Depend on `pallas-network`. (9) Hold `String` in any error variant. (10) Use any other BLUE forbidden pattern. (11–13) PHASE4-N-G server-role MUST NOTs (carry-forward). **(14) NEW from PHASE4-N-L (CN-SESS-01/02 enforcement):** MUST NOT define a second pub `encode_frame` / `decode_frame` pair anywhere in the workspace (`ci/ci_check_mux_frame_closure.sh`); MUST NOT define a second pub `n2n_transition` or `n2c_transition` anywhere in the workspace (`ci/ci_check_handshake_closure.sh`). |
| **Inbound deps** | `ade_core_interop` (live binaries), `ade_runtime` (heavy — N-G/N-H/N-K/N-L consumers), `ade_node` (carry-forward + N-L consumes `mux::frame::{encode_frame, decode_frame}` transitively via the GREEN session reducer). Dev-deps `ade_network → {ade_ledger, ade_testkit, ade_crypto, ade_core}` carry forward. |
| **Outbound deps** | `ade_types`, `ade_codec`. No external deps in the BLUE submodules. |
| **Entry points** | Codec entry points per protocol; `ade_network::codec::primitives::*`; `ade_network::codec::version::*`; `ade_network::codec::{CodecError, ProtocolKind}`. State-machine `*_transition` per protocol. Mux: `ade_network::mux::frame::*`. PHASE4-N-G: `ade_network::{chain_sync, block_fetch}::server::*`. |
| **Key modules** | `codec/` (11 protocols + `primitives.rs` + `version.rs` + `error.rs`); `handshake/`, `chain_sync/` (incl. PHASE4-N-G `server.rs`), `block_fetch/` (incl. PHASE4-N-G `server.rs`), `tx_submission/`, `keep_alive/`, `peer_sharing/`; `n2c/`; `mux/frame.rs`. |

---

### `ade_plutus`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch `ade_plutus`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Quarantine boundary between the Ade-canonical ledger and the ported UPLC evaluator from `aiken-lang/aiken` (pinned to tag `v1.1.21`). |
| **Creates** | `PlutusScript`, `PlutusLanguage`, `EvalOutput`, `PlutusError`, `CostModels`, `DecoderMode`, `PerScriptResult`, `TxEvalResult`. |
| **Interprets** | UPLC scripts (Plutus V1/V2/V3) and `CostModels` CBOR. Phase-two transaction evaluation. `PlutusScript::from_cbor` is a named ingress chokepoint. |
| **MUST NOT** | (1) Re-export any `pallas_*` or `aiken_uplc::` type. (2) Allow another BLUE crate to bypass the canonical entry. (3) Activate PV11 builtins. (4) Use any BLUE-forbidden pattern. (5) Construct `PreservedCbor` outside `ade_codec`. |
| **Inbound deps** | `ade_ledger`, `ade_testkit`. |
| **Outbound deps** | `ade_types`, `ade_crypto`, `ade_codec`, `aiken_uplc` (git, tag `v1.1.21`), `pallas-primitives` (internal-only). |
| **Entry points** | `ade_plutus::eval_tx_phase_two`, `ade_plutus::tx_eval::*`, `ade_plutus::evaluator::*`, `ade_plutus::cost_model::*`. |
| **Key modules** | `evaluator.rs`, `cost_model.rs`, `script_context.rs`, `script_verdict.rs`, `tx_eval.rs`. |

---

### `ade_types`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch `ade_types`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Owns the canonical Cardano domain vocabulary — primitives, era enum, address forms, era-tagged transaction bodies / outputs / certificates, governance types — used by every other workspace crate as the lingua franca. |
| **Creates** | `CardanoEra`, `SlotNo`, `BlockNo`, `EpochNo`, `Hash28`, `Hash32`, `Coin`, `Lovelace`, `NetworkId`, `Nonce`, `TxIn`, `RewardAccount`, `PoolId`, `Address`, `ByronAddress`, `Credential`, `StakeCredential`, `Certificate`, `PoolRegistrationCert`, `ConwayCert`, `CertDisposition` / `DepositEffect` / `CoinSource`, `MIRCert`, `MIRPot`, `DRep`, `GovAction`, `GovActionState`, `GovActionId`, `Anchor`, `ProposalProcedure`, `OperationalCert`, `NativeScript`, `PlutusV1Script`, `Datum`, `DatumOption`, `MultiAsset`, `AssetName`, `CostModel`, `ExUnits`, plus per-era tx-body / tx-out / witness wrappers. |
| **Interprets** | None — produce-only. |
| **MUST NOT** | (1) Construct or decode `PreservedCbor`. (2) Use any BLUE-forbidden pattern. (3) Depend on any workspace crate. (4) Add open/extensible variants to closed enums without a versioned gate. |
| **Inbound deps** | `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `ade_testkit`, `ade_network`, `ade_core`, `ade_core_interop`, `ade_node`. No new inbound crate edge in PHASE4-N-L. |
| **Outbound deps** | None. |
| **Entry points** | `ade_types::CardanoEra`, `ade_types::tx::{Coin, TxIn, RewardAccount}`, `ade_types::{Hash32, SlotNo, Hash28, BlockNo, EpochNo}`, `ade_types::conway::tx::ConwayTxBody`, `ade_types::conway::cert::*`, `ade_types::conway::governance::{ProposalProcedure, Anchor, GovAction, GovActionId}`, `ade_types::shelley::block::{OperationalCert, ProtocolVersion, ShelleyHeader, ShelleyBlock, VrfData}`. |
| **Key modules** | `primitives.rs`, `era.rs`, `tx.rs`, `address/`, `byron/`, `shelley/`, `allegra/`, `mary/`, `alonzo/`, `babbage/`, `conway/`. |

---

## GREEN Modules — Deterministic Glue

> Deterministic, non-authoritative. May depend on BLUE; must not affect authoritative outputs.

### `ade_testkit`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not add a sub-module to `ade_testkit`; the cluster's evidence surface is the new integration test `crates/ade_network/tests/session_replay_equivalence.rs` (2 tests, DC-SESS-03) + the inline tests in the new GREEN session files and the RED runtime files.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Test infrastructure: differential harness, oracle snapshot loader, regression corpus, genesis loader, era mapping, transcript replay, diff reporting; N-B consensus harness; B1 block-validity harness; B2 transaction-validity harness; N-E S2 mempool-ingress replay harness; PP-S2 proposal_procedures replay harness; PHASE4-N-C producer test harness. |
| **Creates** | Carry-forward (all entries from prior CODEMAP). No new types in PHASE4-N-L. |
| **Interprets** | Carry-forward. |
| **MUST NOT** | Carry-forward (1)–(16). |
| **Inbound deps** | None at compile time; consumption is via integration tests and dev-dep links from `ade_core`, `ade_runtime`, `ade_ledger`, `ade_node`. `ade_core_interop` is a non-dev consumer. PHASE4-N-L does not add a new dev-dep consumer. |
| **Outbound deps** | `ade_types`, `ade_codec`, `ade_crypto`, `ade_core`, `ade_ledger`, `ade_plutus`, `ade_runtime`, `blake2`, `flate2`, `tar`, `serde`, `serde_json`, `toml`, `cardano-crypto` (dev-dep, N-C). |
| **Entry points** | `ade_testkit::harness::*`; N-B `consensus::*`; B1 `validity::*`; B2 `tx_validity::*`. N-E S2: `ade_testkit::mempool::*`. PP-S2: `ade_testkit::governance::*`. PHASE4-N-C: `ade_testkit::producer::*`. |
| **Key modules** | `harness/`, `consensus/`, `validity/`, `tx_validity/`, `mempool/`, `governance/`, `producer/` (N-C). |

> **Classification note carried forward.** `ade_testkit` reads files from disk in test helpers and drives BLUE authorities from corpus inputs.

---

### `ade_network::session` *(GREEN by content — PHASE4-N-L S2/S3/S4, NEW)*

> **Status: NEW at HEAD `d62c2bc` + PHASE4-N-L worktree.** The 6-file sub-tree `ade_network/src/session/` was an empty `mod.rs` placeholder at the prior HEAD and is now a populated GREEN module by content despite the upstream `.idd-config.json` `_core_paths_doc` listing `session` as RED. The GREEN classification rests on: (a) `// Core Contract:` banners on every file and explicit GREEN-by-content declarations in `session/mod.rs`'s doc-comment; (b) `ci/ci_check_session_core_closure.sh` enforces tokio-free / no-second-pub-`step` for the session core files; (c) `ci/ci_check_clock_seam.sh` (extended) now scopes `crates/ade_network/src/session/` and forbids `SystemTime::now()` / `Instant::now()` / `tokio::time::*`; (d) `ci/ci_check_mini_protocol_id_registry_closed.sh` enforces closure of the `AcceptedMiniProtocol` registry. Cross-listed inside the broader `ade_network` discussion as a deliberate per-submodule color split.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The pure session driver. Composes the BLUE authorities `mux::frame::{encode_frame, decode_frame}` (CN-SESS-01), `handshake::n2n_transition` (CN-SESS-02), and the per-mini-protocol state machines from PHASE4-N-A through a single GREEN reducer `session::core::step` (CN-SESS-03) — handshake completes before any frame is delivered (DC-SESS-01); the mini-protocol id space is a closed registry (DC-SESS-02); ordering is preserved per (peer, mini_protocol_id) and replay is byte-identical (DC-SESS-03); no wall-clock or rand reaches the GREEN session core (DC-SESS-05). |
| **Creates** | **`session::event`:** closed 6-variant `AcceptedMiniProtocol` (`Handshake` / `ChainSync` / `BlockFetch` / `TxSubmission` / `KeepAlive` / `PeerSharing`) with `from_id(MiniProtocolId) -> Option<Self>` closing `_ => None`; `ByteChunkIn { peer_id, bytes }`; closed `SessionEffect` sum; closed `SessionError` sum; closed 2-variant `HandshakeRole`. **`session::state`:** closed `SessionState::{Handshaking(HandshakeProgress) | Connected(ConnectedState)}` type-state — only the `Connected` arm exposes the per-protocol fanout, making "deliver a mini-protocol frame to a still-handshaking session" unrepresentable in the type system (DC-SESS-01); `HandshakeProgress`; `ConnectedState { negotiated_version, per_protocol_frame_buffers }`. **`session::demux`:** `FrameBuffer { accumulated: Vec<u8> }` partial-frame accumulator; `feed(&mut self, chunk: &[u8]) -> Vec<MuxFrame>`. **`session::core`:** `pub fn step(state: &mut SessionState, input: ByteChunkIn) -> Result<Vec<SessionEffect>, SessionError>` (CN-SESS-03 — the SOLE pub reducer in `session/`). **`session::handshake_driver`:** `Transport` trait (`read_exact` / `write_all` / `close`); closed `TransportError` sum (`Eof` / `ProtocolViolation` / `BackpressureExceeded` / `Io`); `NegotiatedN2n { version, params }`; `run_n2n_handshake_initiator<T: Transport>`; `run_n2n_handshake_responder<T: Transport>`. |
| **Interprets** | Inbound byte chunks (`ByteChunkIn`) from a RED transport. Routes via `mux::frame::decode_frame` + `FrameBuffer::feed` into per-protocol fanout matched against the closed `AcceptedMiniProtocol` registry. Composes — never re-implements — the BLUE authorities `mux::frame::*` and `handshake::n2n_transition`. |
| **MUST NOT** | (1) Define a second pub `step` anywhere in `session/` — enforced by `ci/ci_check_session_core_closure.sh`. (2) Import `tokio::*` in any file under `session/` (same script). (3) Read `SystemTime::now()` / `Instant::now()` / use `tokio::time::*` in any session file — enforced by `ci/ci_check_clock_seam.sh` (extended in N-L to scope `crates/ade_network/src/session/`). (4) Define a second pub `encode_frame` / `decode_frame` anywhere in the workspace (CN-SESS-01 — `ci/ci_check_mux_frame_closure.sh`). (5) Define a parallel handshake reducer; routes all state advances through `handshake::n2n_transition` (CN-SESS-02 — `ci/ci_check_handshake_closure.sh`). (6) Wildcard-accept unknown `MiniProtocolId` — `AcceptedMiniProtocol::from_id` MUST close with `_ => None` and the dispatch table is a closed `match` (DC-SESS-02 — `ci/ci_check_mini_protocol_id_registry_closed.sh`). (7) Deliver a mini-protocol frame while `SessionState::Handshaking` is the active arm (DC-SESS-01 — structural type-state). (8) Use `mpsc::unbounded_channel` or any `unbounded`-named queue constructor (DC-SESS-04 — `ci/ci_check_session_no_unbounded.sh`). (9) Bypass `mux::frame::{encode,decode}_frame` for outbound or inbound bytes. (10) Construct `PreservedCbor` (the session core never decodes block/header/tx-body bytes; those route through `ade_codec`'s named chokepoints). (11) Hold `String` in any error variant; every closed sum is `String`-free for closed-discriminant discipline. |
| **Inbound deps** | `ade_runtime::network::mux_pump::MuxPump` (per-connection driver — owns its `SessionState`, feeds `step` per inbound chunk); `ade_runtime::network::n2n_dialer::N2nDialer` (consumes `run_n2n_handshake_initiator` + `NegotiatedN2n`); the integration test `session_replay_equivalence.rs`. |
| **Outbound deps** | `ade_types::{SlotNo, …}`, `ade_codec` (transitive — none direct), `ade_network::mux::frame::{decode_frame, encode_frame, MuxFrame, MuxHeader, MiniProtocolId}`, `ade_network::handshake::{n2n_transition, select_n2n_version}`, `ade_network::codec::*` per-protocol re-exports. No tokio. No outbound crate edge beyond BLUE `ade_network` + `ade_types`. |
| **Entry points** | `ade_network::session::{step, FrameBuffer, AcceptedMiniProtocol, ByteChunkIn, SessionEffect, SessionError, HandshakeRole, Transport, TransportError, NegotiatedN2n, run_n2n_handshake_initiator, run_n2n_handshake_responder, SessionState, ConnectedState, HandshakeProgress}`. |
| **Key modules** | `session/mod.rs` (barrel), `session/event.rs`, `session/state.rs`, `session/demux.rs`, `session/core.rs`, `session/handshake_driver.rs`. |

---

### `ade_runtime::bootstrap` *(GREEN — PHASE4-N-K S1)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch `bootstrap.rs`. The function continues to be the SOLE bootstrap authority gated by `ci/ci_check_bootstrap_closure.sh`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The SOLE `pub fn` in the workspace returning the initial `(LedgerState, PraosChainDepState, Option<ChainTip>)` triple at node startup. Closes **CN-NODE-01**. |
| **Creates** | `BootstrapInputs<'a, D, S>`, closed 4-variant `BootstrapError`, `bootstrap_initial_state` function. |
| **MUST NOT** | Carry-forward (1)–(7). |
| **Inbound deps** | `ade_node::node::run_node_until_shutdown`; the 6 inline tests + integration tests. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | `ade_runtime::bootstrap::{bootstrap_initial_state, BootstrapInputs, BootstrapError}`. |

---

### `ade_runtime::clock` *(GREEN trait + GREEN `DeterministicClock` — PHASE4-N-K S1; RED `SystemClock` sub-classified in same file)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not modify the clock seam, but `ci/ci_check_clock_seam.sh` was **extended in N-L** to additionally scope `crates/ade_network/src/session/` (DC-SESS-05). The clock-seam authority itself remains `clock.rs` as the SOLE site of `SystemTime::now()` / `Instant::now()` in `ade_runtime`.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The orchestrator core and the new keep-alive session both consume time exclusively through the `Clock` trait. Production uses `SystemClock` (RED); tests and replay harnesses use `DeterministicClock` (pure). Closes **DC-NODE-03** + strengthened in N-L (DC-SESS-05). |
| **Creates** | `Clock` trait, `DeterministicClock`, `SystemClock` (RED sub-classified), `millis_to_slot` helper. |
| **MUST NOT** | Carry-forward (1)–(5). **NEW from PHASE4-N-L:** the extended `ci/ci_check_clock_seam.sh` additionally forbids `SystemTime::now()` / `Instant::now()` / `tokio::time::*` in any file under `crates/ade_network/src/session/` (DC-SESS-05). |
| **Inbound deps** | Carry-forward (orchestrator runners + `ade_node` + tests). **NEW from PHASE4-N-L:** `ade_runtime::orchestrator::keep_alive_session::KeepAliveSession<C: Clock>` is a new direct consumer (DC-SESS-05). |
| **Outbound deps** | Carry-forward. |
| **Entry points** | `ade_runtime::clock::{Clock, DeterministicClock, SystemClock, millis_to_slot}`. |

---

### `ade_runtime::orchestrator::{mod, event, state, core}` *(GREEN — PHASE4-N-K S2; event extended in N-L S8)*

> **Status change at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L added one `OrchestratorEvent` variant `OutboundKeepAlive { peer_id: PeerId }` (additive, S8) and one `core::step` arm that records it (no immediate `OrchestratorEffect` emitted — encoding the keep-alive into a wire frame at the session reducer is deferred to a future cluster). The 8-variant closed `OrchestratorEvent` is now a 9-variant closed enum; the closure-by-explicit-match discipline is preserved (no `_` arm in any production `match` against `OrchestratorEvent`).

| Attribute | Value |
|-----------|-------|
| **Purpose** | The pure orchestrator reducer that composes all PHASE4-N-A..N-J BLUE authorities + the N-K runner surface + the **N-L wire-layer event surface** into one running node-shaped state machine. Same inputs always produce the same effect vector (T-DET-01 strengthening — extended by N-L to cover byte-chunk → event determinism end-to-end). |
| **Creates** | All N-K types carry-forward. **NEW from PHASE4-N-L:** the closed `OrchestratorEvent` enum gains the `OutboundKeepAlive { peer_id }` variant — emitted by the RED `KeepAliveSession<C>` runner, recorded by `core::step` for future encoding into a wire keep-alive frame. The variant count goes 8 → 9; the enum remains closed and `String`-free. |
| **Interprets** | Carry-forward. **NEW from PHASE4-N-L:** the `OutboundKeepAlive` arm is currently a recording-only path in `core::step` (the variant is observable in the effect log via test introspection but produces no immediate `OrchestratorEffect`); the per-peer keep-alive frame encoding lives in the follow-on cluster. |
| **MUST NOT** | Carry-forward (1)–(10). **(11) NEW from PHASE4-N-L:** MUST NOT widen `OrchestratorEvent` beyond the 9 closed variants without a closed-enum review; the dispatch in `core::step` MUST keep its explicit-arm pattern (no `_` fallback). |
| **Inbound deps** | Carry-forward. **NEW from PHASE4-N-L:** `ade_runtime::orchestrator::keep_alive_session::KeepAliveSession<C>` emits the new event variant via the `events_out: mpsc::Sender<OrchestratorEvent>` channel; `ade_runtime::network::n2n_dialer::N2nDialer` emits `PeerConnected` via the same channel after the handshake completes. |
| **Outbound deps** | Carry-forward. No new outbound crate edge in N-L. |
| **Entry points** | Carry-forward, plus the new `OrchestratorEvent::OutboundKeepAlive` variant. |

---

### `ade_runtime::rollback::persistent_writer` *(GREEN — PHASE4-N-K S3)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch this file.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Cadence-fidelity glue between the orchestrator's `CaptureSnapshot` effect and `PersistentSnapshotCache::capture`. Closes **DC-NODE-02**. |
| **Creates** | `PersistentSnapshotWriter<'a, S>`. |
| **MUST NOT** | Carry-forward (1)–(7). |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | `ade_runtime::rollback::persistent_writer::PersistentSnapshotWriter`. |

---

### `ade_runtime::producer::{broadcast_to_served, served_chain_lookups}` *(GREEN — PHASE4-N-G S5)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch this GREEN sub-tree.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Bridges PHASE4-N-C's `BroadcastQueue` (RED) and PHASE4-N-G's `ServedChainSnapshot` (BLUE) to the `ServedHeaderLookup` / `ServedRangeLookup` trait seams the BLUE server reducers consume. |
| **Creates** | `ServedChainLookups<'a>`; function `drain_and_admit`. |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | `ade_runtime::producer::broadcast_to_served::drain_and_admit`, `ade_runtime::producer::served_chain_lookups::ServedChainLookups`. |

---

### `ade_runtime::receive::{events_to_state, in_memory_chain_write}` *(GREEN — PHASE4-N-H S3, extended PHASE4-N-I S3)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch this GREEN sub-pair.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Bridges PHASE4-N-A's `ForkChoiceSignal` / `BatchDeliveryEvent` to the BLUE `ReceiveEvent` stream, and wires the BLUE `ChainDbWrite` trait to any `ade_runtime::chaindb::ChainDb` impl. |
| **Creates** | Functions `lift_chain_sync_signal` + `lift_block_fetch_event`. Struct `ChainDbWriter<'a, D: ChainDb>`. |
| **MUST NOT** | Carry-forward (1)–(8). |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | `ade_runtime::receive::events_to_state::{lift_chain_sync_signal, lift_block_fetch_event}`, `ade_runtime::receive::in_memory_chain_write::ChainDbWriter`. |

---

### `ade_runtime::rollback::{cadence, in_memory_cache, chaindb_block_source}` *(GREEN — PHASE4-N-I S4)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch the three N-I GREEN files.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Production impls of the BLUE `ade_ledger::rollback` trait seams + the cadence policy. |
| **Creates** | `SnapshotCadence`, `should_snapshot_after_block`, `InMemorySnapshotCache`, `ChainDbBlockSource<'a, D>`. |
| **MUST NOT** | Carry-forward. `should_snapshot_after_block` remains the SOLE cadence-decision function in `ade_runtime` (DC-NODE-02). |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | `ade_runtime::rollback::cadence::{SnapshotCadence, should_snapshot_after_block}`, `ade_runtime::rollback::in_memory_cache::InMemorySnapshotCache`, `ade_runtime::rollback::chaindb_block_source::ChainDbBlockSource`. |

---

### `ade_runtime::rollback::persistent_cache` *(GREEN — PHASE4-N-J S8)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not touch this file.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Bridges the BLUE `ade_ledger::snapshot::framing::{encode_snapshot, decode_snapshot}` pair to any `ade_runtime::chaindb::SnapshotStore` impl. **Closes DC-CONS-21.** |
| **Creates** | `PersistentSnapshotCache<'a, S>`, `PersistentCacheError`, `PERSISTENT_CACHE_SCHEMA_VERSION`. |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | Carry-forward. |
| **Outbound deps** | Carry-forward. |
| **Entry points** | `ade_runtime::rollback::persistent_cache::{PersistentSnapshotCache, PersistentCacheError, PERSISTENT_CACHE_SCHEMA_VERSION}`. |

---

## RED Modules — Imperative Shell

> I/O, network, storage, clocks, retries. May depend on BLUE/GREEN. Must not modify core state directly or construct semantic types unsafely.

### `ade_core_interop` *(PHASE4-N-B S-B10; PHASE4-N-E S4/S5/S6; PHASE4-N-C S7; PHASE4-N-G S7; PHASE4-N-H S6)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not add a new live binary. The wire layer is now mechanically real (via `ade_node` + the new `N2nDialer` + `MuxPump` + `KeepAliveSession`); the live operator pass against a private cardano-node peer is the follow-on `PHASE4-N-L-LIVE` cluster.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Live cardano-node interop driver. Hosts the five `live_*_session` RED binaries plus the N-E S4/S5 GREEN bridges. |
| **Creates** | `fresh_orchestrator` (N-B). N-E S4 `PeerAccumulator`. N-E S5 `ClientAccumulator`. |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | None. |
| **Outbound deps** | `ade_core`, `ade_codec`, `ade_crypto`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_testkit`, `ade_types`, `tokio`. No new edges in N-L. |
| **Entry points** | `ade_core_interop::fresh_orchestrator`, the five `live_*_session` binaries. |
| **Key modules** | `src/lib.rs`, `src/follow.rs`, `src/tx_submission.rs`, `src/local_tx_submission.rs`, `src/bin/live_*_session.rs`. |

> **Gap surfaced (carried).** CE-N-B-6 manual evidence pending. CE-N-E-6 CLOSED. CE-N-E-7 deferred as `CE-NODE-N2C-LTX`. CE-N-C-8 enforced + `CN-CONS-06.open_obligation = blocked_until_operator_stake_available`. CE-N-G-8 (RO-LIVE-01) partial. CE-N-H-6 (RO-LIVE-02) partial. **NEW from PHASE4-N-L** — `RO-LIVE-03` (live mux session pass) carries `open_obligation = blocked_until_operator_peer_available`; the mechanical wire layer is ready, the live binary pass is the `PHASE4-N-L-LIVE` follow-on.

---

### `ade_network::mux::transport` *(RED — extended PHASE4-N-L S5)*

> **Status change at HEAD `d62c2bc` + PHASE4-N-L worktree.** Was a 40-LOC RED stub (`MuxTransport` + `open_tcp` + raw `read_raw`/`write_raw`); now 231 LOC. The pre-existing `MuxTransport` / `open_tcp` / `read_raw` / `write_raw` API is preserved unchanged for `ade_core_interop` continuity. PHASE4-N-L adds the full-duplex bounded-queue replacement.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The only place inside `ade_network` where socket I/O happens. RED tokio shell over the BLUE mux frame primitive and the GREEN session reducer. |
| **Creates** | `MuxTransport` (carry-forward). **NEW from PHASE4-N-L (S5):** `MuxTransportHandle { inbound: mpsc::Receiver<Vec<u8>>, outbound: mpsc::Sender<Vec<u8>> }` (bounded-only); closed `TransportError` sum (`BackpressureExceeded` / `PeerClosed` / `IoError(io::ErrorKind)` — no `String`); `DuplexCapacity { inbound_chunks: usize, outbound_chunks: usize, read_buffer_bytes: usize }` with `pub const DEFAULT: Self = DuplexCapacity { inbound_chunks: 1024, outbound_chunks: 256, read_buffer_bytes: 16384 }`; `fn spawn_duplex(stream: TcpStream, capacity: DuplexCapacity) -> MuxTransportHandle`. |
| **Interprets** | Raw TCP bytes from a `TcpStream`. Translates between bounded mpsc queues and socket reads/writes. |
| **MUST NOT** | (1)–(6) carry-forward. **(7) NEW — PHASE4-N-L (DC-SESS-04):** MUST NOT introduce `mpsc::unbounded_channel` or any `unbounded`-named queue constructor (`ci/ci_check_session_no_unbounded.sh`). (8) **NEW — PHASE4-N-L:** MUST NOT hold `String` in any `TransportError` variant; every error discriminant must close on a known shape (`IoError(io::ErrorKind)` not `IoError(String)`). |
| **Inbound deps** | `ade_runtime::network::mux_pump::MuxPump` (NEW — consumes `MuxTransportHandle` per peer); `ade_runtime::network::n2n_dialer::N2nDialer` (NEW — calls `spawn_duplex`); `ade_core_interop` (carry-forward). |
| **Outbound deps** | `tokio` (gains the existing `sync` + `rt-multi-thread` features via the new `Cargo.toml` declaration), `ade_types`, `ade_codec`, BLUE `ade_network::codec::*`, `ade_network::mux::frame`. |
| **Entry points** | `ade_network::mux::transport::{MuxTransport, MuxTransportHandle, TransportError, DuplexCapacity, spawn_duplex, open_tcp}`. |
| **Key modules** | `mux/transport.rs`. |

> **Gap closed by PHASE4-N-L.** The "`session::mod.rs` is still a placeholder" gap surfaced in the prior CODEMAP is **closed** at HEAD. The placeholder is now populated GREEN-by-content; the new RED files inside `ade_runtime::network/` and `ade_runtime::orchestrator/keep_alive_session.rs` host the async layer. The remaining honest-scope gap is `RO-LIVE-03` — the live operator pass.

---

### `ade_network` *(RED capture binaries — non-session)*

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** The capture binaries under `ade_network/src/bin/` are untouched.

| Attribute | Value |
|-----------|-------|
| **Purpose** | The capture binaries (operator-action evidence harness). |
| **Creates** | None (RED glue). |
| **MUST NOT** | Carry-forward. |
| **Inbound deps** | `ade_core_interop`. |
| **Outbound deps** | `tokio`, `ade_types`, `ade_codec`, BLUE `ade_network::codec::*`, `ade_network::mux::frame`. |
| **Entry points** | The capture binaries. |
| **Key modules** | `bin/capture_*`. |

---

### `ade_node`

> **Status carried from prior CODEMAP — unchanged at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L did not modify `ade_node` source. The binary's existing `--peer ADDR` / `--listen ADDR` CLI flags become operator-meaningful now that the mux session driver is real: the `PHASE4-N-L-LIVE` follow-on runs `ade_node --peer <private-cardano-node-host> --listen 0.0.0.0:3001` against a private peer.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Binary entry point for the node process. Composes `bootstrap_initial_state` → `OrchestratorState` → `LeadershipSession` → orchestrator inbox loop → shutdown drain with `PersistentSnapshotWriter::force_capture`. The binary today, when launched, performs CLI parsing + an honest-scope readiness print; the actual N2N peer dialer + mux session driver above `MuxTransport::read_raw/write_raw` is **now real at the wire layer** (via `N2nDialer` + `MuxPump` + the GREEN session reducer) but the binary still requires the operator-action wiring to compose them against a live peer. |
| **Creates** | Carry-forward — all N-K types (`Cli`, `CliError`, `NodeStartupInputs<'a, D, S, C>`, `NodeShutdownEvidence`, `NodeRunError`, exit-code constants). |
| **MUST NOT** | Carry-forward (1)–(14). |
| **Inbound deps** | None (binary). The lib re-exports are consumed by the two integration tests under `crates/ade_node/tests/`. |
| **Outbound deps** | `ade_types`, `ade_core`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_codec`, `tokio`. Dev-deps: `ade_testkit`, `tempfile`. No new outbound edge in N-L. |
| **Entry points** | `main()`. `ade_node::run_node_until_shutdown` is the library entry the integration tests drive in-process. |

> **Gap (carried + narrowed).** The orchestrator core + bootstrap + persistent writer + leadership session + per-peer session shape + listening-socket pump + **NEW — the mux session driver + handshake driver + bounded duplex transport + outbound dialer + Clock-driven keep-alive** are real and mechanically evidenced. The remaining honest-scope gap is the operator-action follow-on cluster `PHASE4-N-L-LIVE` — running `ade_node` against a private cardano-node peer + capturing the JSONL log for RO-LIVE-01 / RO-LIVE-02 / RO-LIVE-03.

---

### `ade_runtime`

> **Status change at HEAD `d62c2bc` + PHASE4-N-L worktree.** PHASE4-N-L adds three new RED files inside `ade_runtime`: `network/mux_pump.rs` (289 LOC), `network/n2n_dialer.rs` (268 LOC), `orchestrator/keep_alive_session.rs` (149 LOC). `network/mod.rs` extended with `pub mod {mux_pump, n2n_dialer};` + re-exports; `orchestrator/mod.rs` extended with `pub mod keep_alive_session;` + re-exports. One additive `OrchestratorEvent::OutboundKeepAlive { peer_id }` variant in `orchestrator/event.rs`. No new outbound crate edge.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Carry-forward (all N-K + earlier purposes). **NEW from PHASE4-N-L:** (p) the RED per-connection mux pump (`network/mux_pump.rs`, N-L S6) — feeds `MuxTransportHandle` bytes into `session::core::step` and forwards the resulting `OrchestratorEvent`s to the orchestrator inbox; (q) the RED outbound N2N dialer (`network/n2n_dialer.rs`, N-L S7) — `TcpStream::connect` + `spawn_duplex` + `run_n2n_handshake_initiator` → emits `OrchestratorEvent::PeerConnected` + spawns a `MuxPump`; (r) the RED Clock-driven keep-alive pump (`orchestrator/keep_alive_session.rs`, N-L S8) — emits `OrchestratorEvent::OutboundKeepAlive` per cadence-aligned tick. The crate remains RED at the prefix level. |
| **Creates** | All carry-forward. **NEW — PHASE4-N-L (RED):** `MuxPump`; `N2nDialer`, `DialError` (closed sum), `BlockingTransport` (private helper); `KeepAliveCadence`, `KeepAliveSession<C: Clock>`. None counted toward the BLUE total. |
| **Interprets** | Carry-forward. **NEW from PHASE4-N-L:** `MuxPump` interprets inbound `MuxTransportHandle` chunks via `session::core::step`; `N2nDialer` interprets the negotiated handshake outcome; `KeepAliveSession` interprets `Clock::next_tick()` outputs. |
| **MUST NOT** | (1)–(34) carry-forward. **(35) NEW — `network::mux_pump` (N-L S6, RED):** MUST own its own `MuxTransportHandle` per connection (DC-NODE-01 strengthening — per-peer isolation extended to the wire layer); MUST route every inbound chunk through `session::core::step` (CN-SESS-03 strengthening — no parallel session reducer); MUST NOT use `mpsc::unbounded_channel` (DC-SESS-04 — `ci/ci_check_session_no_unbounded.sh`). **(36) NEW — `network::n2n_dialer` (N-L S7, RED):** MUST route every handshake state advance through `handshake::n2n_transition` (CN-SESS-02); MUST NOT define a parallel handshake reducer (`ci/ci_check_handshake_closure.sh`); MUST emit `OrchestratorEvent::PeerConnected` only after the handshake completes (DC-SESS-01 — type-state-enforced via `NegotiatedN2n` being the only path out of `run_n2n_handshake_initiator`). **(37) NEW — `orchestrator::keep_alive_session` (N-L S8, RED):** MUST consume time only via `Clock` (DC-SESS-05 — `ci/ci_check_clock_seam.sh` extended); MUST NOT call `SystemTime::now()` / `Instant::now()` / `tokio::time::*` directly. |
| **Inbound deps** | Carry-forward. The 3 new files are inbound-consumed by `ade_node::node::run_node_until_shutdown` (forward-looking — the binary today still runs idle pending the operator-action wiring). |
| **Outbound deps** | Carry-forward (`tokio` features unchanged at `signal + net + rt + rt-multi-thread + io-util + macros + time + sync`). No new outbound crate edge. |
| **Entry points** | All carry-forward. **NEW — PHASE4-N-L:** `ade_runtime::network::mux_pump::MuxPump`; `ade_runtime::network::n2n_dialer::{N2nDialer, DialError}`; `ade_runtime::orchestrator::keep_alive_session::{KeepAliveCadence, KeepAliveSession}`. |
| **Key modules** | All prior keys. **NEW — `network/`:** `mux_pump.rs`, `n2n_dialer.rs`. **NEW — `orchestrator/`:** `keep_alive_session.rs`. |
| **Mechanical enforcement** | Carry-forward. **NEW — five PHASE4-N-L CI scripts back the new wire-session + transport + dialer + keep-alive surface:** `ci_check_mux_frame_closure.sh` (S1; CN-SESS-01), `ci_check_handshake_closure.sh` (S1; CN-SESS-02), `ci_check_session_core_closure.sh` (S2; CN-SESS-03 + DC-SESS-01 + DC-SESS-05 session-side), `ci_check_mini_protocol_id_registry_closed.sh` (S1; DC-SESS-02), `ci_check_session_no_unbounded.sh` (S5; DC-SESS-04). **One existing script extended:** `ci_check_clock_seam.sh` now also scopes `crates/ade_network/src/session/` (DC-SESS-05). |

> **Gap surfaced (carried + narrowed).** **NEW — PHASE4-N-L narrows the orchestrator-driven path's gap** by closing the mux session driver + handshake driver + bounded duplex transport + outbound dialer + Clock-driven keep-alive halves mechanically. The remaining gap is the same operator-action one: `ade_node` against a private cardano-node peer + JSONL log capture (RO-LIVE-01 / RO-LIVE-02 / RO-LIVE-03). The orchestrator's correctness over the wire layer is mechanically evidenced by `session_replay_equivalence.rs` (DC-SESS-03), the inline tests in `session/core.rs` (12), and the 5 new narrow CI scripts.

---

## Cross-Module Rules (project-wide)

### Dependency direction

`ade_core_interop` → `{ade_core, ade_codec, ade_crypto, ade_ledger, ade_runtime, ade_network, ade_testkit, ade_types, tokio}` is legal (RED leaf binary). **No new crate-level outbound edge in PHASE4-N-L.**
`ade_runtime` → `{ade_core, ade_crypto, ade_codec, ade_types, ade_ledger, ade_network, redb, serde_json, cardano-crypto, ed25519-dalek, tokio}` is legal (RED → BLUE). **PHASE4-N-L unchanged outbound; `tokio` features unchanged.**
`ade_node` → `{ade_types, ade_core, ade_ledger, ade_runtime, ade_network, ade_codec, tokio}` is legal (RED → BLUE/GREEN). Dev-deps: `ade_testkit`, `tempfile`. **No change in N-L.**
`ade_testkit` → `{ade_core, ade_ledger, ade_plutus, ade_runtime, ade_crypto, ade_codec, ade_types, cardano-crypto}` is legal (GREEN). **No change in N-L.**
`ade_network` (BLUE submodules) → `{ade_codec, ade_types}` is legal.
`ade_network` (GREEN-by-content `session/` submodule, NEW in N-L) → `{ade_codec, ade_types, BLUE ade_network::{mux::frame, handshake, codec, chain_sync, block_fetch, keep_alive, peer_sharing, tx_submission, n2c}}` is legal. **No `tokio` in any session/ file.**
`ade_network` (RED submodules + capture bins) → `{tokio, ade_codec, ade_types, ade_network::codec::*, ade_network::mux::frame, ade_network::session::*}` is legal. **PHASE4-N-L added `sync + rt-multi-thread` features to the existing `tokio` dep in `ade_network/Cargo.toml`.**
`ade_ledger` → `{ade_core, ade_plutus, ade_crypto, ade_codec, ade_types, minicbor}` is legal (BLUE among BLUEs).
`ade_core` → `{ade_types, ade_crypto, minicbor}` is legal (BLUE among BLUEs).
`ade_plutus` → `{ade_crypto, ade_codec, ade_types}` is legal.
`ade_crypto` → `{ade_types}` is legal.
`ade_codec` → `{ade_types}` is legal.
`ade_types` → `{}`.

**Forbidden directions.** Any BLUE crate (or BLUE `ade_network` submodule) depending on `ade_runtime`, `ade_node`, `ade_core_interop`, or the RED half of `ade_network` is a CI failure (`ci_check_dependency_boundary.sh` + `ci_check_no_async_in_blue.sh`). Any non-`ade_plutus` crate referring to `pallas_*` is a CI failure. Any reference to `ChainDb` / `chain_db` inside `crates/ade_core/src/consensus/` is a CI failure. All B1/B2/B3/B5/N-E/PP-S1/N-C/N-G/N-H/N-I/N-J/N-K dependency notes carry forward. **NEW — PHASE4-N-L dependency notes (the 5 wire-session CI gates pin specific properties):** any second pub `encode_frame` / `decode_frame` pair anywhere in the workspace is a CI failure (`ci_check_mux_frame_closure.sh`, CN-SESS-01). Any second pub `n2n_transition` / `n2c_transition` anywhere in the workspace is a CI failure (`ci_check_handshake_closure.sh`, CN-SESS-02). Any second pub `step` in `session/` (or any `tokio::*` import in a session core file) is a CI failure (`ci_check_session_core_closure.sh`, CN-SESS-03 + DC-SESS-01). Any wildcard accept in the `AcceptedMiniProtocol` dispatch is a CI failure (`ci_check_mini_protocol_id_registry_closed.sh`, DC-SESS-02). Any `mpsc::unbounded_channel` or `unbounded`-named queue in `session/` / `mux::transport` / `mux_pump.rs` / `n2n_dialer.rs` / `keep_alive_session.rs` is a CI failure (`ci_check_session_no_unbounded.sh`, DC-SESS-04). Any `SystemTime::now()` / `Instant::now()` / `tokio::time::*` in any file under `crates/ade_network/src/session/` is a CI failure (`ci_check_clock_seam.sh` extended, DC-SESS-05). The GREEN `ade_network::session` sub-tree MUST NOT import `tokio::*` even though the broader `ade_network` crate now declares additional tokio features for the RED `mux::transport`.

### Naming convention

All crates are prefixed `ade_`. TCB color is not encoded in the crate name. The authoritative classifier is `.idd-config.json` `core_paths` plus the cluster doc TCB Color Maps for sub-crate scopes; CI scripts hard-code their BLUE list. The seven full-BLUE-scoped scripts use the full BLUE set: 6 BLUE crates + 9 BLUE `ade_network` paths. The B3, B5, OQ5, PP-S1, N-C (8), N-G (7), N-H (5), N-I (2), N-J (1), N-K (6) narrow scripts narrow as previously documented. **NEW — the 5 PHASE4-N-L scripts scope `ade_network/` and `ade_runtime/` paths only — none target BLUE crates:** `ci_check_mux_frame_closure.sh` scopes the workspace via grep for a single `encode_frame`/`decode_frame` pair (sole occurrences expected in `ade_network/src/mux/frame.rs`); `ci_check_handshake_closure.sh` scopes the workspace via grep for a single `n2n_transition` and a single `n2c_transition` (sole occurrences in `ade_network/src/handshake/`); `ci_check_session_core_closure.sh` scopes `crates/ade_network/src/session/`; `ci_check_mini_protocol_id_registry_closed.sh` scopes `crates/ade_network/src/session/event.rs` + the dispatch site in `session/core.rs`; `ci_check_session_no_unbounded.sh` scopes `crates/ade_network/src/session/` + `crates/ade_network/src/mux/transport.rs` + `crates/ade_runtime/src/network/{mux_pump,n2n_dialer}.rs` + `crates/ade_runtime/src/orchestrator/keep_alive_session.rs`. The extended `ci_check_clock_seam.sh` additionally scopes `crates/ade_network/src/session/`. **No CI script scopes the BLUE crates in PHASE4-N-L** — the cluster's invariants are properties of the GREEN session reducer + the RED wire-shell wiring, not of any BLUE module.

### CI enforcement (66 scripts under `ci/`)

| Script | Enforces | Scope |
|---|---|---|
| `ci_check_admitted_block_closure.sh` *(PHASE4-N-H S1)* | CE-N-H-1 + CN-CONS-08 + CN-PROTO-07 | repo-wide grep gate + `crates/ade_ledger/src/receive/events.rs` |
| `ci_check_block_fetch_server_closure.sh` *(PHASE4-N-G S4)* | DC-CONS-17 foundation | `crates/ade_network/src/block_fetch/server.rs` |
| `ci_check_bootstrap_closure.sh` *(PHASE4-N-K S1)* | CN-NODE-01 | `crates/ade_runtime/src/bootstrap.rs` |
| `ci_check_broadcast_to_served_purity.sh` *(PHASE4-N-G S5)* | CE-N-G-5 | `crates/ade_runtime/src/producer/{broadcast_to_served,served_chain_lookups}.rs` |
| `ci_check_cbor_round_trip.sh` | T-ENC-03, DC-CBOR-01, DC-CBOR-02 | golden corpus |
| `ci_check_ce_n_a_5_proof.sh` *(PHASE4-N-A, S-A10)* | CE-N-A-5 5-condition evidence | `ade_network` (RED + real-capture corpus) |
| `ci_check_chaindb_contract.sh` | DC-STORE-02, DC-STORE-03, CN-STORE-04, CN-STORE-05 | `ade_runtime --lib chaindb::` (RED) |
| `ci_check_chaindb_crash_safety.sh` | T-REC-01 (crash variant), DC-STORE-01, CN-STORE-03 | `ade_runtime --test stress_kill_harness` (RED) |
| `ci_check_chain_sync_server_closure.sh` *(PHASE4-N-G S3)* | DC-PROTO-08 | `crates/ade_network/src/chain_sync/server.rs` |
| `ci_check_clock_seam.sh` *(PHASE4-N-K S1, EXTENDED PHASE4-N-L S8)* | **DC-NODE-03 + DC-SESS-05 — `clock.rs` is SOLE site of `SystemTime::now()` / `Instant::now()` in `ade_runtime`; orchestrator core files + `ade_network/src/session/` contain none of `SystemTime`/`Instant`/`tokio::time::*`** | `crates/ade_runtime/src/` + `crates/ade_runtime/src/orchestrator/{core,event,state,mod}.rs` + **NEW: `crates/ade_network/src/session/`** |
| `ci_check_consensus_closed_enums.sh` | DC-CONS-04, DC-CONS-10, T-DET-01, DC-VAL-02/04/05/06, DC-TXV-01/02/04/05, DC-MEM-01/02 | `consensus/` + `block_validity/` + `tx_validity/` + `mempool/` |
| `ci_check_constitution_coverage.sh` | invariant-registry ↔ code/test coverage | repo-wide |
| `ci_check_conway_cert_classification_closed.sh` *(PHASE4-B3F)* | DC-TXV-06 | `ade_types::conway::cert` + `ade_codec::conway::cert` + `ade_ledger::cert_classify` |
| `ci_check_credential_discriminant_closed.sh` *(OQ5)* | DC-LEDGER-10 | `ade_types::shelley::cert` + both era `ade_codec` decoders + `ade_ledger::{state, governance, fingerprint, rules}` + `ade_types::conway::governance` |
| `ci_check_crypto_vectors.sh` | crypto KAT regression | `ade_crypto` |
| `ci_check_dependency_boundary.sh` | T-BOUND-02 — BLUE ⇎ RED separation | full BLUE |
| `ci_check_deposit_param_authority.sh` *(PHASE4-B3)* | DC-TXV-07 | 6 BLUE crates |
| `ci_check_differential_divergence.sh` | DC-DIFF-* | replay outputs |
| `ci_check_forbidden_patterns.sh` | T-CORE-02 + `unsafe` allowlist | full BLUE |
| `ci_check_forge_purity.sh` *(PHASE4-N-C S3)* | DC-CONS-13/14/15 | `crates/ade_ledger/src/producer/forge.rs` |
| `ci_check_gov_cert_accumulation_closed.sh` *(PHASE4-B5)* | DC-LEDGER-09 | `ade_ledger::{gov_cert, rules, error, state}` |
| **`ci_check_handshake_closure.sh`** *(NEW — PHASE4-N-L S1)* | **CN-SESS-02 — single pub `n2n_transition` + single pub `n2c_transition` across the workspace** | repo-wide grep gate (`crates/ade_network/src/handshake/`) |
| `ci_check_hash_uses_wire_bytes.sh` | DC-CBOR-02, T-ENC-01 | full BLUE |
| `ci_check_hfc_translation.sh` | DC-EPOCH-02 | `ade_ledger::hfc` |
| `ci_check_ingress_chokepoints.sh` | DC-INGRESS-01, T-INGRESS-01 | full BLUE |
| `ci_check_ledger_determinism.sh` | DC-LEDGER-01 | `ade_ledger` |
| `ci_check_mempool_ingress_closure.sh` *(PHASE4-N-E, S1)* | DC-MEM-03 | `ade_ledger::mempool/{ingress,admit,mod}.rs` + repo-wide `admit(` scan |
| `ci_check_mempool_ingress_replay.sh` *(PHASE4-N-E, S2 + S3)* | DC-MEM-04 | `ade_testkit::mempool/*` + `ade_testkit::lib.rs` + `ade_ledger::mempool::canonicalize` |
| **`ci_check_mini_protocol_id_registry_closed.sh`** *(NEW — PHASE4-N-L S1)* | **DC-SESS-02 — `AcceptedMiniProtocol::from_id` closes `_ => None`; dispatch is a closed `match`** | `crates/ade_network/src/session/event.rs` + dispatch site in `session/core.rs` |
| `ci_check_module_headers.sh` | CE-04 contract banner | full BLUE |
| **`ci_check_mux_frame_closure.sh`** *(NEW — PHASE4-N-L S1)* | **CN-SESS-01 — single pub `encode_frame` / `decode_frame` pair in the workspace** | repo-wide grep gate (`crates/ade_network/src/mux/frame.rs`) |
| `ci_check_n2n_server_no_signing_dep.sh` *(PHASE4-N-G S6)* | Key-boundary | `crates/ade_runtime/src/network/` |
| `ci_check_no_async_in_blue.sh` *(PHASE4-N-A, S-A1)* | DC-CORE-01 | full BLUE |
| `ci_check_no_chaindb_in_consensus_blue.sh` *(PHASE4-N-B, S-B1)* | DC-CORE-01 + DC-CONS-07 | `crates/ade_core/src/consensus/` |
| `ci_check_node_binary_uses_single_bootstrap.sh` *(PHASE4-N-K S7)* | CN-NODE-01 + DC-NODE-04 | `crates/ade_node/src/` |
| `ci_check_no_density_in_fork_choice.sh` *(PHASE4-N-B, S-B8)* | DC-CONS-03 | `fork_choice.rs` + `candidate.rs` |
| `ci_check_no_float_in_consensus.sh` *(PHASE4-N-B, S-B1)* | T-CORE-02 + DC-CONS-07/08/09 | `crates/ade_core/src/consensus/` |
| `ci_check_no_parallel_header_splitter.sh` *(PHASE4-N-G S1)* | DC-CONS-16 strengthening + DC-CONS-18 | repo-wide grep gate |
| `ci_check_no_private_keys_in_corpus.sh` *(PHASE4-N-C S3)* | DC-CRYPTO-03/04/05 corpus discipline | `crates/ade_testkit/src/producer/fixtures.rs` + `crates/ade_testkit/fixtures/producer/` |
| `ci_check_no_producer_body_encoder.sh` *(PHASE4-N-C S4)* | DC-CONS-16 | repo-wide grep gate |
| `ci_check_no_secrets.sh` | no credentials/IPs/keys | repo-wide |
| `ci_check_no_semantic_cfg.sh` | T-BUILD-01 | full BLUE |
| `ci_check_no_signing_in_blue.sh` | CE-05, T-KEY-01 | full BLUE |
| `ci_check_opcert_closed.sh` *(PHASE4-N-C S2)* | DC-CONS-11/12 | `crates/ade_codec/src/shelley/opcert.rs` + `crates/ade_core/src/consensus/opcert_validate.rs` |
| `ci_check_orchestrator_core_purity.sh` *(PHASE4-N-K S2)* | DC-NODE-03 + general purity | `crates/ade_runtime/src/orchestrator/{core,event,state,mod}.rs` |
| `ci_check_pallas_quarantine.sh` | O-29.2 | non-`ade_plutus` |
| `ci_check_peer_session_isolation.sh` *(PHASE4-N-K S4)* | DC-NODE-01 | `crates/ade_runtime/src/orchestrator/peer_session.rs` |
| `ci_check_persistent_writer_no_parallel_cadence.sh` *(PHASE4-N-K S3)* | DC-NODE-02 | `crates/ade_runtime/src/rollback/` + `crates/ade_runtime/src/orchestrator/` + repo-wide |
| `ci_check_private_key_custody.sh` *(PHASE4-N-C S1)* | DC-CRYPTO-03/04/05 + OP-OPS-04 | full BLUE + `crates/ade_runtime/src/producer/` |
| `ci_check_producer_corpus_present.sh` *(PHASE4-N-C S7)* | CN-CONS-06 mechanical half | `crates/ade_testkit/src/producer/fixtures.rs` |
| `ci_check_proposal_procedures_closed.sh` *(PROPOSAL-PROCEDURES-DECODE PP-S1)* | DC-LEDGER-11 | `conway/{governance,tx}.rs` paths + repo-wide `ProposalProcedure {` scan |
| `ci_check_receive_orchestrator_no_producer_dep.sh` *(PHASE4-N-H S4)* | Key-boundary | `crates/ade_runtime/src/receive/` |
| `ci_check_receive_paths_corpus_present.sh` *(PHASE4-N-H S6)* | CE-N-H-5 + CE-N-H-6 binary presence | N-H integration tests + binary src + procedure doc |
| `ci_check_receive_reducer_closure.sh` *(PHASE4-N-H S2)* | CE-N-H-2 + CN-CONS-08 + DC-CONS-19 | `crates/ade_ledger/src/receive/reducer.rs` |
| `ci_check_receive_replay_purity.sh` *(PHASE4-N-H S3)* | CE-N-H-3 + DC-PROTO-09 | `crates/ade_runtime/src/receive/{events_to_state,in_memory_chain_write}.rs` |
| `ci_check_recovery_contract.sh` | T-REC-01, T-REC-02, DC-STORE-05 | `ade_runtime --lib recovery::` (RED) |
| `ci_check_ref_provenance.sh` | DC-REF-01 | reference corpus |
| `ci_check_rollback_materialize_closure.sh` *(PHASE4-N-I S2)* | CN-STORE-07 + DC-CONS-22 | `crates/ade_ledger/src/rollback/materialize.rs` + `crates/ade_ledger/src/rollback/*.rs` |
| `ci_check_scheduler_closure.sh` *(PHASE4-N-C S6)* | DC-CONS-13 + DC-MEM-03 + OP-OPS-05 | `crates/ade_runtime/src/producer/{scheduler,broadcast,tick_assembler}.rs` |
| `ci_check_self_accept_gate.sh` *(PHASE4-N-C S5)* | CN-CONS-07 | `crates/ade_ledger/src/producer/self_accept.rs` + repo-wide `AcceptedBlock` literal scan |
| `ci_check_served_chain_closure.sh` *(PHASE4-N-G S2)* | CN-CONS-07 strengthening + CE-N-G-2 | `crates/ade_ledger/src/producer/served_chain.rs` |
| `ci_check_server_paths_corpus_present.sh` *(PHASE4-N-G S7)* | CE-N-G-7 + CE-N-G-8 binary presence | N-G integration tests + binary src + procedure doc |
| **`ci_check_session_core_closure.sh`** *(NEW — PHASE4-N-L S2)* | **CN-SESS-03 + DC-SESS-01 + DC-SESS-05 (session side) — `session::core::step` is sole pub reducer in `session/`; `Handshaking`/`Connected` type-state structurally present; no `tokio::*` imports in session core files** | `crates/ade_network/src/session/` |
| **`ci_check_session_no_unbounded.sh`** *(NEW — PHASE4-N-L S5)* | **DC-SESS-04 — no `mpsc::unbounded_channel` / `unbounded`-named queue constructor** | `crates/ade_network/src/session/` + `crates/ade_network/src/mux/transport.rs` + `crates/ade_runtime/src/network/{mux_pump,n2n_dialer}.rs` + `crates/ade_runtime/src/orchestrator/keep_alive_session.rs` |
| `ci_check_snapshot_cadence_purity.sh` *(PHASE4-N-I S4)* | DC-STORE-07 | `crates/ade_runtime/src/rollback/*.rs` |
| `ci_check_snapshot_encoder_closure.sh` *(PHASE4-N-J S7)* | CN-STORE-08 + DC-STORE-08 + DC-STORE-09 | `crates/ade_ledger/src/snapshot/{framing,ledger,chain_dep}.rs` + repo-wide |

> **Post-`d62c2bc` CI delta. +5 new; +1 extended in place.** The new gates are the PHASE4-N-L wire-session gate set. **CI inventory 61 → 66.** The forward-looking `ci/ci_check_no_fail_open_in_validation.sh` (DC-VAL-06 / DC-TXV) is still **not** shipped — carried gap.

> **Carried residual gaps (unchanged at HEAD, plus the N-L narrowings).** Carry-forward gaps from the prior CODEMAP largely still open. **(rr) NARROWED — `ade_node` honest-scope stub** — the wire layer is now real (mux session driver, handshake driver, bounded duplex transport, outbound dialer, Clock-driven keep-alive); the binary still runs idle pending the operator-action wiring against a private cardano-node peer (RO-LIVE-01 / RO-LIVE-02 / RO-LIVE-03). **(ss) UNCHANGED — `clock.rs` hosts both GREEN (`Clock` trait + `DeterministicClock`) and RED (`SystemClock`) in one file** — gated by the extended `ci/ci_check_clock_seam.sh` as the single-sited seam (the extension also gates `session/`). **(tt) NARROWED — no live operator-action binary for the orchestrator-driven path** — `ade_node` is the natural host; the `PHASE4-N-L-LIVE` follow-on cluster runs it against a private peer. **(uu) UNCHANGED — `DC-STORE-09` carries `open_obligation = "snapshot_schema_migration_follow_on_cluster"`**. **(vv) NARROWED — `RO-LIVE-01` / `RO-LIVE-02` / `CN-CONS-06` live-evidence halves + the new `RO-LIVE-03` live-session pass remain `blocked_until_operator_peer_available`** — all four close together when `PHASE4-N-L-LIVE` runs. **(ww) NEW — `OrchestratorEvent::OutboundKeepAlive` is a recording-only path in `core::step`** — the per-peer keep-alive frame encoding at the session reducer is a deferred follow-on; the variant is observable in the effect log via test introspection but produces no immediate `OrchestratorEffect`. **(xx) NEW — the `ade_network::session/` sub-tree is GREEN-by-content despite the upstream `.idd-config.json` `_core_paths_doc` listing `session` as RED** — the classification rests on `// Core Contract:` file banners + explicit GREEN-by-content declarations in `session/mod.rs`'s doc-comment + the 4 narrow CI scripts; a future config update could move `crates/ade_network/src/session/` into the BLUE `core_paths` list, but that would weaken the seam (the session reducer is GREEN-deterministic-glue, not BLUE-authoritative-core — it composes BLUE authorities rather than defining new ones).
