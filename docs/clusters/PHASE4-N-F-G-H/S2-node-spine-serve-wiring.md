# Invariant Slice — PHASE4-N-F-G-H S2: Node-spine serve wiring + hermetic loopback

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Node-spine serve wiring + hermetic loopback (retain the `ServedChainView`; spawn a sibling listener + serve-dispatch task outside `run_relay_loop`; serve ChainSync + BlockFetch from the G-B self-accepted view).

### Cluster
**PHASE4-N-F-G-H** — Node-spine live serve-to-peer.

### Status
Proposed (doc-before-implement; not yet implemented). Depends on S1 (shared serve-dispatch core).

### Cluster Exit Criteria Addressed
- [ ] **CE-G-H-2 (node-spine request-driven serve — MECHANICAL, closeable)** — `--mode node` (given `--listen`) serves the G-B self-accepted `ServedChainView` to a peer via **both** ChainSync (`RollForward` header, `DC-CONS-18`) **and** BlockFetch (body, `DC-CONS-17`), through a sibling task **outside** `run_relay_loop`, reusing `run_n2n_listener` + the shared adapter; the containment gate (`ci_check_node_run_loop_containment.sh`) and the path-fidelity fence (`ci_check_node_path_fidelity.sh`) stay byte-/semantically unchanged; a hermetic loopback test proves a peer discovers + fetches an **already-served** self-accepted block via the request-driven path.

> CE-G-H-1 (S1, merged) and CE-G-H-3 (operator-gated C1 serve, S3) are **out of scope** for this slice.

### Slice Dependencies
- **S1** (`8b8b8139`): the shared coordinator-free `ade_runtime::network::serve_dispatch` core (`dispatch_server_frame_event_to_outbound` + `install_server_peer_state` + `remove_server_peer_state` + `ServerPeerStates`). Merged.
- **G-B** (`febee120`, `DC-NODE-06`): the `ServedChainHandle::new() → (handle, view)` pair + the sibling `push_atomic` task on the node On-arm.

## 3. Implementation Instruction (AI)
On the `--mode node` `On` arm (`node_lifecycle.rs:~475`): stop discarding the read side — bind `let (serve_handle, serve_view) = ServedChainHandle::new();` (was `_serve_view`). When `cli.listen_addr` is `Some` (the **existing** `--listen` flag — no new flag), spawn a **second sibling task** (alongside the existing push sibling, **outside** `run_relay_loop`) that: builds an `mpsc::channel::<OrchestratorEvent>` + `new_per_peer_outbound()`, spawns `run_n2n_listener` (with `our_supported = N2N_SUPPORTED`, a `PeerIdGenerator`, `events_out`, `peer_outbound: Some(..)`), and runs a `select!` loop that routes each inbound event to the **S1 coordinator-free primitives directly** — `PeerConnected{role: DownstreamServer}` → `install_server_peer_state`; `PeerDisconnected` → `remove_server_peer_state`; `PeerN2nServer{ChainSync,BlockFetch}Frame` → `dispatch_server_frame_event_to_outbound(.., &serve_view, &peer_outbound)`. **Do NOT call `handle_listener_event`** (it is coordinator-coupled — `CoordinatorState`/`coordinator_step`/producer-evidence). **Do NOT** drive `producer_chain_sync_advance_tip` or react to `serve_view.changed()` — request-driven only. **Do NOT** touch `run_relay_loop`'s body (containment byte-unchanged). Factor the serve loop into a testable async fn (e.g. `run_node_serve_task(listen_addr, serve_view, shutdown_rx)`) the On-arm spawns and the loopback test drives. A serve-start (bind) failure under `--listen` MUST be surfaced explicitly per the chosen On-arm lifecycle policy — never silently disabled while the run proceeds as if serving. Add the hermetic socket loopback test + the serve-start-failure-surfaced test. No BLUE change; no new flag; no second serializer; no RO-LIVE flip. Commit carries the project attribution trailer (CLAUDE.md), no other AI references.

## 4. Intent
Make `--mode node` actually **serve its self-accepted chain to a real peer** — the missing leg the G-D C1 dry-run surfaced — by exposing **only** the G-B `ServedChainView` through a sibling listener + the single S1 serve-dispatch core, request-driven, outside `run_relay_loop`. (Enforces the serve-from-the-view clause of `DC-NODE-07`; preserves G-B's handoff/containment, the single serializer, and the reused BLUE serve reducers.)

## 5. Scope
- **Modules / crates:**
  - `crates/ade_node/src/node_lifecycle.rs` (RED) — On-arm: retain `serve_view`; spawn the serve sibling (gated on `cli.listen_addr`); join it at shutdown alongside the push sibling. Factor `run_node_serve_task(listen_addr, ServedChainView, shutdown_rx)`.
  - `crates/ade_node/tests/node_spine_serve_loopback.rs` (NEW, RED test) — the hermetic socket loopback + the serve-start-failure-surfaced test.
- **State machines affected:** none new. The serve reducers + per-peer state are the S1-shared ones, reused.
- **Persistence impact:** none — the serve is read-only over the `ServedChainView`; no WAL/checkpoint/durable-tip change.
- **Network-visible impact:** `--mode node` with `--listen` now **accepts inbound peers and answers ChainSync + BlockFetch** from the served chain (it previously bound no listener). No new wire message; reuses the closed N2N grammar + CN-WIRE-08 tag-24.
- **Out of scope:** proactive `advance_tip` / `serve_view.changed()` reactor; the C1 runbook + operator harness (S3); any BLUE change; any RO-LIVE/BA-02 claim.

## 6. Execution Boundary
- **BLUE (none — unchanged):** `ade_network::{chain_sync,block_fetch}::server` reducers reused VERBATIM; CN-WIRE-08 tag-24. A BLUE change → reject.
- **GREEN:** `ade_runtime::producer::served_chain_handle::ServedChainView` (the retained read side) + `served_chain_lookups` — reused, read-only.
- **RED:** `node_lifecycle.rs` On-arm wiring + `run_node_serve_task` (sockets, tokio, channels); the reused `ade_runtime::network::{serve_dispatch, n2n_listener, outbound_command}`; the new loopback test.
- **Color resolved:** the serve task is RED I/O driving the BLUE reducers via the S1 RED adapter over the GREEN view; no ambiguity.

## 7. Invariants Preserved
- `DC-NODE-06` (G-B serve handoff + sibling `push_atomic`) — unchanged; the push sibling and `serve_handle` are untouched; the new serve sibling only **reads** `serve_view`.
- `CN-NODE-02` (single live-run owner + relay-loop containment) — the serve sibling is spawned **outside** `run_relay_loop` (like the push sibling); `run_relay_loop`'s body is untouched; `ci_check_node_run_loop_containment.sh` stays byte-/semantically unchanged.
- `DC-NODE-07` "no second serve authority" (S1) — preserved: the sibling calls the **single** shared `dispatch_server_frame_event_to_outbound`; `ci_check_single_serve_dispatch_authority.sh` stays green.
- `DC-CONS-17` / `DC-CONS-18` — served body == `AcceptedBlock.as_bytes()`, advertised header == `accepted_block_header_bytes`; enforced by the reused servers.
- `CN-PROTO-06` / `DC-PROTO-07` / `DC-PROTO-08` — server-agency-only replies, deterministic/total serve reducers — reused unchanged.
- `CN-WIRE-08` — single tag-24 serializer (no parallel serializer added).
- `CN-FORGE-01` / `DC-SYNC-01` / `DC-SYNC-02` — the durable tip still advances only via `run_node_sync`/the forge; the serve is read-only.
- `ci_check_node_path_fidelity.sh` — green (reuses `--listen`; no new `--mode node` flag).

## 8. Invariants Strengthened or Introduced
- **`DC-NODE-07` — serve-from-the-view clause (strengthened: enforcement extended to the node spine).** S2 makes `--mode node` actually serve the `ServedChainView` to a peer via ChainSync + BlockFetch through a sibling task outside `run_relay_loop`. The binding test is `node_spine_serve_loopback_follower_fetches_self_accepted_block`. Recorded in the rule's `evidence_notes` now; the final `tests` binding + the `declared → enforced` flip happen at the G-H close (after S3).

> Single invariant family: "`--mode node` serves only the G-B self-accepted view, request-driven, outside the relay loop." S2 covers the node-spine serve mechanism; the operator-witnessed over-the-wire C1 proof is S3.

## 9. Design Summary
- **Retain the view:** `node_lifecycle.rs` binds `serve_view` (was discarded as `_serve_view`); the existing push sibling keeps `serve_handle`.
- **Serve sibling (mirrors `produce_mode`'s listener+select wiring, coordinator-free):** `run_node_serve_task` builds the events channel + `peer_outbound`, spawns `run_n2n_listener`, and `select!`s on `events_rx.recv()` + `shutdown_rx.changed()`, routing events to `install_server_peer_state` / `remove_server_peer_state` / `dispatch_server_frame_event_to_outbound(.., &serve_view, &peer_outbound)`. Spawned only when `cli.listen_addr` is `Some`; joined at On-arm shutdown.
- **Request-driven only:** no `advance_tip`, no `serve_view.changed()` reactor. A follower's `RequestNext` is answered with `RollForward` **iff** the block is already in `serve_view` at request time (the test arranges exactly that).
- **Serve-start failure is explicit:** a bind failure under `--listen` is surfaced per the chosen On-arm lifecycle policy (fail-fast startup error, or run-without-serving with a structured serve-start failure) — never a silent disable while the run claims live serve capability.
- **Hermetic socket loopback** (`node_spine_serve_loopback.rs`, modeled on `wire_only_loopback.rs`): pre-load a `ServedChainView` with a real self-accepted `AcceptedBlock` (corpus block via `ServedChainHandle::push_atomic`, as `produce_loopback.rs` does); spawn `run_node_serve_task` on an ephemeral `127.0.0.1:0`; connect an in-process synthetic follower (Ade's mux + N2N handshake initiator + chain_sync/block_fetch client codecs) that does `FindIntersect(Origin) → IntersectFound`, `RequestNext → RollForward`, then `BlockFetch RequestRange → Block`. Assert the rolled header == `accepted_block_header_bytes` and the fetched body == `AcceptedBlock.as_bytes()`.

## 10. Changes Introduced
### Types
- None canonical. Possibly a small private `run_node_serve_task` fn + its arg struct (RED, non-canonical) + a closed serve-start-error variant if the lifecycle policy needs one. No new `Mode`/CLI flag/`CoordinatorEvent`/`NodeBlockSource` variant.
### State Transitions
- None new. The serve reducers + per-peer state machine are the S1-shared ones.
### Persistence
- None.
### Removal / Refactors
- `_serve_view` → `serve_view` (stop discarding). The On-arm spawns one additional sibling task and joins it.

## 11. Replay, Crash, and Epoch Validation
- **Replay:** no new authoritative state → replay unaffected. `DC-PROTO-07` serve-transcript replay-equivalence is carried (reducers unchanged). The loopback test is deterministic: a fixed pre-loaded view + fixed client request sequence → fixed `RollForward`/`Block` bytes.
- **Crash/restart:** unchanged — the serve sibling holds no durable state; the durable tip advances only via `run_node_sync`.
- **Epoch boundary:** not applicable (no forge/epoch logic touched).

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic):

- [ ] `node_spine_serve_loopback_follower_fetches_self_accepted_block` (`crates/ade_node/tests/node_spine_serve_loopback.rs`) — passes: a self-accepted block pre-loaded into a `ServedChainView`, served via `run_node_serve_task` on an ephemeral port; an in-process follower completes the N2N handshake, gets `RollForward` whose header == `accepted_block_header_bytes`, and `BlockFetch`es a body whose bytes == `AcceptedBlock.as_bytes()`.
- [ ] `node_serve_start_failure_is_surfaced_not_silent` (`crates/ade_node/tests/node_spine_serve_loopback.rs`) — passes: when `--listen` is set but `run_node_serve_task` cannot bind (e.g. an already-bound / invalid address), the failure is surfaced as a structured serve-start outcome per the chosen On-arm lifecycle policy — **never** silently dropped while the run proceeds as if serving (the "no silent live-serve claim" invariant).
- [ ] `ci_check_single_serve_dispatch_authority.sh` — green (the node serve sibling calls the shared core; defines no second serve-dispatch).
- [ ] `ci_check_node_run_loop_containment.sh` — **byte-unchanged + green** (verified `git diff` vs `main`); the serve sibling is outside `run_relay_loop`.
- [ ] `ci_check_node_path_fidelity.sh` — **byte-unchanged + green** (no new `--mode node` flag; `--listen` reused).
- [ ] `ci_check_served_chain_handoff_fence.sh` — **byte-unchanged + green**.
- [ ] `cargo test -p ade_node` + `cargo test -p ade_runtime` + targeted `ade_network` server tests — green (no regression; `produce_loopback` still green).

## 13. Failure Modes
- A follower's `RequestNext` arrives when the block is **not** yet in `serve_view` → the server replies `AwaitReply` and the follower parks (no proactive push in S2). **This is the explicit S2 boundary:** if a real C1 follower hits this (parks at tip before Ade forges) and requires proactive `advance_tip`, **stop and scope it as a separate cluster** — do not add a reactor here. (Deterministic, not a crash.)
- Listener bind failure must be surfaced as a structured serve-start failure under the On-arm lifecycle policy chosen during implementation. It must not silently disable serving while allowing the run to claim live serve capability.
- Malformed inbound frame → the BLUE reducer rejects it (`DispatchError::ReducerError`); the dispatch returns the closed error; the peer is dropped; no panic, no authoritative-state effect.

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-H "Forbidden During This Cluster" prohibitions apply (no `--mode produce` switch; no private-only path; no relay-loop serve mutation; no second `ServedChain` authority; no parallel tag-24 serializer; no proactive `advance_tip`/`changed()` reactor; no new `--mode node` flag; no from-genesis constructor; no new BLUE authority/canonical type/variant; no acceptance without `correlate`; no RO-LIVE flip).
### Slice-Specific Prohibitions
- **No `push_atomic` / served-chain mutation / serve inside `run_relay_loop`** — the serve sibling is spawned in the On-arm, outside the loop; `ci_check_node_run_loop_containment.sh` byte-unchanged.
- **No `handle_listener_event`** on the node spine — it is coordinator-coupled; route events to the S1 coordinator-free primitives directly (no `CoordinatorState` / `coordinator_step` / producer evidence writer on the node spine).
- **No proactive `advance_tip` / `serve_view.changed()` reactor** — request-driven serve only.
- **No silent live-serve claim** — a serve-start (bind) failure under `--listen` must be surfaced explicitly + testably; the node must never proceed as if serving while serving is silently disabled.
- **No second `ServedChain` authority** — reuse the G-B `serve_handle`/`serve_view`; the serve sibling only reads `serve_view`.
- **No second serializer / second serve-dispatch** — `ci_check_single_serve_dispatch_authority.sh` stays green.
- **No new `--mode node` flag** (reuse `--listen`); **no BLUE change**.

## 15. Explicit Non-Goals
This slice MUST NOT: add a proactive `advance_tip` reactor; write the C1 runbook / operator harness (S3); modify the serve reducers or any BLUE crate; mutate the served chain or durable tip from `run_relay_loop`; add a `--mode node` flag/config; claim peer acceptance / BA-02 / RO-LIVE.

## 16. Completion Checklist
- [ ] `_serve_view` retained as `serve_view`; serve sibling spawned (gated on `--listen`) outside `run_relay_loop`; joined at shutdown.
- [ ] Serve sibling routes events to the S1 coordinator-free primitives only (no coordinator/evidence on the node spine).
- [ ] `node_spine_serve_loopback_follower_fetches_self_accepted_block` green (RollForward header + BlockFetch body == `as_bytes()`).
- [ ] `node_serve_start_failure_is_surfaced_not_silent` green (no silent live-serve claim).
- [ ] The 3 fences byte-unchanged; the single-serve-dispatch gate green.
- [ ] No proactive `advance_tip`; no BLUE change; no new flag; no RO-LIVE flip.
- [ ] `DC-NODE-07` `evidence_notes` record the S2 tests; final binding + flip at G-H close.

## 17. Review Notes
- **Request-driven boundary is load-bearing (honest):** S2 proves serving an *already-served* block (follower behind tip). The parked-at-tip → proactive-`RollForward` case is **not** wired (it isn't wired in `produce_mode` either) and must not be smuggled in. If S3's real C1 follower needs it, that's a new shared-path cluster.
- **No silent live-serve claim:** whether the node aborts startup or runs-without-serving on a bind failure is an implementation/lifecycle decision, but it must be explicit and testable — the node must never appear to serve when it does not.
- **Why a socket loopback, not dispatch-level:** the bounty path is a real peer fetching over the wire; `wire_only_loopback.rs` proves the in-process socket+handshake harness is feasible. If during implementation the synthetic follower proves disproportionate, **surface it** rather than silently downgrading — the committed cluster doc specifies an ephemeral-address loopback.
- **Coordinator-free reuse:** the node serve loop deliberately bypasses `handle_listener_event` — that function's coordinator/evidence coupling is exactly what S1 separated out. The node calls `install`/`remove`/`dispatch` directly.
