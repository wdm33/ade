# Invariant Slice — PHASE4-N-F-G-H S1: Extract shared serve-dispatch authority

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Extract shared serve-dispatch authority (move the coordinator-free serve-dispatch core to a shared `ade_runtime` home so there is exactly **one** serve-dispatch definition both modes call).

### Cluster
**PHASE4-N-F-G-H** — Node-spine live serve-to-peer.

### Status
Merged (impl `8b8b8139`). First slice — the reuse-enabling refactor that S2 builds on.

### Cluster Exit Criteria Addressed
- [ ] **CE-G-H-1 (single serve-dispatch authority — MECHANICAL, closeable)** — a candidate gate `ci_check_single_serve_dispatch_authority.sh` is green: exactly one definition of the serve-dispatch core in `ade_runtime`, imported by both `produce_mode` and `node_lifecycle`; no parallel serve-dispatch defined in `node_lifecycle`. `produce_mode` serve behavior byte-unchanged: `cargo test -p ade_node` green (incl. the existing producer serve tests) + the `ade_network` server tests green.

> CE-G-H-2 (node-spine wiring) and CE-G-H-3 (operator-gated C1 serve) are **out of scope** for this slice.

### Slice Dependencies
- **G-B** (`febee120`, `DC-NODE-06`): the `ServedChainHandle::new() → (handle, view)` pair + sibling `push_atomic`. Merged.
- **Producer serve infra** (N-G/N-R/N-S): `dispatch_server_frame_event_to_outbound` + `handle_listener_event` + `run_n2n_listener`/`new_per_peer_outbound` (already `pub` in `ade_runtime::network`) + the BLUE `producer_{chain_sync,block_fetch}_serve`. Merged.

## 3. Implementation Instruction (AI)
**Slice-entry proof obligation first (verify in code at HEAD `0d55c511`, do not assume):** confirm `dispatch_server_frame_event_to_outbound` (`produce_mode.rs:1332`) is **coordinator-free** — its only inputs are `(&OrchestratorEvent, &mut ServerPeerStates, &ServedChainView, &PerPeerOutbound)`; it decodes the frame, borrows the view, drives the BLUE `producer_chain_sync_serve`/`producer_block_fetch_serve`, sends an `OutboundCommand` via `PerPeerOutbound`, and returns `(usize, Vec<ServedBlockEvidence>)` — **no `CoordinatorState`, no `coordinator_step`, no evidence writer.** Confirm `handle_listener_event` (`produce_mode.rs:1187`) is the entangled wrapper (it holds `CoordinatorState`, calls `coordinator_step` on `PeerConnected`/`PeerDisconnected`, and writes producer evidence via `write_evidence_event`). Then move to a shared `ade_runtime::network` serve module: `dispatch_server_frame_event_to_outbound` + the **per-peer-state lifecycle** (install `PerPeerN2nServerState::new(..)` on `OrchestratorEvent::PeerConnected{role: DownstreamServer}` / remove on `PeerDisconnected`) + `ServerPeerStates` (`:1185`) + `DispatchError` (`:1291`) + `ServedBlockEvidence` (`:1315`). Re-point `produce_mode`'s `handle_listener_event` at the shared core; its `CoordinatorState`/`coordinator_step`/evidence-writer wrapping **stays in `produce_mode`** and is unchanged. Add `ci/ci_check_single_serve_dispatch_authority.sh`. Touch **no** BLUE crate; add **no** `--mode node` flag; do **no** node-spine wiring (that is S2); add **no** proactive `advance_tip`/`changed()` reactor; do **not** relax the containment/path-fidelity/handoff fences. Commit carries the project attribution trailer (CLAUDE.md), no other AI references.

## 4. Intent
Make it **mechanically impossible** for the node spine to grow a *second* serve-dispatch authority: there is exactly one serve-dispatch core (in shared `ade_runtime`), both modes call it, and a CI gate forbids a duplicate. This begins enforcing the **"no second serve authority / serializer"** clause of `DC-NODE-07`, while leaving `produce_mode`'s serve behavior byte-identical (the coordinator/evidence concerns it owns stay with it).

## 5. Scope
- **Modules / crates:**
  - **NEW** `crates/ade_runtime/src/network/serve_dispatch.rs` (or equivalent shared `ade_runtime::network` module, RED) — receives the moved `dispatch_server_frame_event_to_outbound` + the per-peer-state lifecycle helpers + `ServerPeerStates` + `DispatchError` + `ServedBlockEvidence`.
  - `crates/ade_node/src/produce_mode.rs` (RED) — `handle_listener_event` re-pointed at the shared core; its `CoordinatorState`/`coordinator_step`/`write_evidence_event` wrapping retained verbatim. The local definitions of the moved items are deleted (now imported).
  - **NEW** `ci/ci_check_single_serve_dispatch_authority.sh` (RED CI fence).
- **State machines affected:** none. The serve reducers (`producer_*_serve`) and the per-peer `PerPeerN2nServerState` shape are unchanged; only their *home* moves.
- **Persistence impact:** none.
- **Network-visible impact:** none — `produce_mode`'s emitted frames are byte-identical (pure code-motion + re-point). The node spine emits nothing new in S1.
- **Out of scope:** node-spine serve wiring (S2); the runbook + operator harness (S3); any `advance_tip`; any BLUE change; any RO-LIVE/BA-02 claim.

## 6. Execution Boundary
- **BLUE (none — unchanged):** `ade_network::{chain_sync,block_fetch}::server` (`producer_chain_sync_serve`, `producer_block_fetch_serve`) reused VERBATIM (BLUE per their `server.rs` header banners + CODEMAP). A BLUE change → reject.
- **GREEN:** `ServedBlockEvidence` — a pure observation struct (`(slot, hash, bytes_len)` observed present in the served snapshot, collected before any `.await`); **GREEN-by-content**, moved unchanged to `ade_runtime`. `ServedChainView`/`served_chain_lookups` reused unchanged (GREEN).
- **RED:** the moved `dispatch_server_frame_event_to_outbound` (async; sends `OutboundCommand` via channels) + the per-peer-state lifecycle + `ServerPeerStates` + `DispatchError` in the new `ade_runtime::network` home; `produce_mode.rs` (re-pointed); `ci_check_single_serve_dispatch_authority.sh`.
- **Color resolved:** no ambiguity — `ServedBlockEvidence` is GREEN (pure observation), the dispatch adapter is RED (channel I/O), the servers stay BLUE (untouched), the fence is RED tooling.

## 7. Invariants Preserved
- `DC-NODE-06` (G-B serve handoff + sibling `push_atomic`) — unchanged; S1 touches no relay-loop/handoff code.
- `CN-NODE-02` (single live-run lifecycle owner + relay-loop containment) — unchanged; no node-spine wiring in S1.
- `CN-WIRE-08` (single tag-24 serializer) — unchanged; the move adds no serializer.
- `DC-CONS-17`, `DC-CONS-18` (served bytes = self-accepted bytes; header projection) — the serve reducers that enforce these are reused verbatim.
- `DC-PROTO-07`, `DC-PROTO-08` (serve-transcript replay-equivalence; chain-sync server-agency totality), `CN-PROTO-06` (server-agency-only reply surface) — preserved; the reducers and `ServerReply` wrappers are untouched.
- **`produce_mode` serve behavior** — byte-unchanged (`RO-LIVE-01` stays `partial`); proven by the existing `produce_loopback.rs` + `ade_network` server tests staying green.
- `ci_check_node_run_loop_containment.sh`, `ci_check_node_path_fidelity.sh`, `ci_check_served_chain_handoff_fence.sh` — byte-unchanged.

## 8. Invariants Strengthened or Introduced
- **`DC-NODE-07` — "no second serve authority / serializer" clause (strengthened: enforcement begun).** S1 introduces `ci_check_single_serve_dispatch_authority.sh` (green in CI from S1 onward) and records it in the rule's `evidence_notes` now; the **final `ci_script`/`tests` binding + the `declared → enforced` flip happen at G-H close** (after S2/S3), mirroring the G-D per-cluster convention. The single-authority half is mechanically enforced even while the registry status word remains `declared`.

> Single invariant family: "the serve-dispatch authority is unique." S1 covers exactly that; the node-spine serve mechanism (DC-NODE-07's serve-from-the-view clause) is S2.

## 9. Design Summary
- **Code motion, not behavior change:** `dispatch_server_frame_event_to_outbound` and its helper types move verbatim to `ade_runtime::network`; `produce_mode` imports them. The per-peer-state install/remove (currently inline in `handle_listener_event`'s `PeerConnected`/`PeerDisconnected` arms) factors into a small shared helper the core exposes, so the node spine (S2) can install/remove without `coordinator_step`. The coordinator + producer-evidence arms of `handle_listener_event` stay in `produce_mode` and call the shared core for the server-frame arms.
- **The gate `ci_check_single_serve_dispatch_authority.sh` (hermetic grep):**
  - **(a) single definition:** exactly one `fn dispatch_server_frame_event_to_outbound` in the workspace, located under `crates/ade_runtime/`; **zero** under `crates/ade_node/`.
  - **(b) no node-spine parallel dispatch:** `node_lifecycle.rs` defines no serve-dispatch of its own (negative grep for a `fn …dispatch…server…frame…` and for a second direct `producer_{chain_sync,block_fetch}_serve` driver in `node_lifecycle.rs`).
  - **(c) produce_mode re-pointed:** `produce_mode.rs` references the shared `dispatch_server_frame_event_to_outbound` (positive grep) and holds no private copy.
  - The gate does **not** require `node_lifecycle` to call the core (that is S2) — it only forbids a duplicate definition; so it is valid at S1 and stays valid at S2.
- **No proactive serve:** the moved core is request-driven only (it reacts to `PeerN2nServer*Frame` events). No `advance_tip`/`changed()` reactor is added.

## 10. Changes Introduced
### Types
- **Moved (no new canonical type):** `ServerPeerStates`, `DispatchError`, `ServedBlockEvidence` relocate from `produce_mode` to `ade_runtime::network`. No struct/enum field changes. (None of these are BLUE canonical types; the 456-count is unaffected.)
### State Transitions
- None. The serve reducers and per-peer state machine are untouched; only the dispatch adapter's module changes.
### Persistence
- None.
### Removal / Refactors
- `produce_mode`'s private definitions of the moved items are deleted (now imported from `ade_runtime`). `handle_listener_event` is re-pointed at the shared core for its server-frame arms; its coordinator/evidence arms are unchanged.

## 11. Replay, Crash, and Epoch Validation
- **Replay:** no new authoritative state → replay unaffected. `DC-PROTO-07` serve-transcript replay-equivalence is carried (the reducers are unchanged); the existing `ade_network` server unit tests (`producer_block_fetch_serve_*`, `producer_chain_sync_serve_*`) and any transcript replay test pass unchanged.
- **Crash/restart:** unchanged — S1 adds no durable state.
- **Epoch boundary:** not applicable (no forge/epoch logic touched).
- **Behavior-unchanged anchor:** `crates/ade_node/tests/produce_loopback.rs` (the `produce_mode` serve loopback) stays green — the primary proof that the extraction + re-point did not change `produce_mode`'s serve behavior.

## 12. Mechanical Acceptance Criteria
This slice is complete only when **all** of the following exist and pass in CI (hermetic):

- [ ] `ci_check_single_serve_dispatch_authority.sh` — NEW gate, green: (a) exactly one `fn dispatch_server_frame_event_to_outbound`, under `crates/ade_runtime/`, none under `crates/ade_node/`; (b) `node_lifecycle.rs` defines no parallel serve-dispatch; (c) `produce_mode.rs` references the shared core. Smoke-tested fail-closed on an injected second `fn dispatch_server_frame_event_to_outbound` in `node_lifecycle.rs`.
- [ ] `crates/ade_node/tests/produce_loopback.rs` — green (produce_mode serve path byte-unchanged after the move + re-point).
- [ ] `ade_network` server unit tests (`producer_block_fetch_serve_*` in `block_fetch/server.rs`, `producer_chain_sync_serve_*`/`producer_chain_sync_advance_tip_*` in `chain_sync/server.rs`) — green (reducers reused verbatim).
- [ ] `cargo test -p ade_node` + `cargo test -p ade_runtime` + targeted `ade_network` server tests — green (no regression).
- [ ] `ci_check_node_run_loop_containment.sh` + `ci_check_node_path_fidelity.sh` + `ci_check_served_chain_handoff_fence.sh` — **byte-unchanged + green** (verified `git diff` vs `main`).

## 13. Failure Modes
- A second serve-dispatch definition appears (now or in S2) → `ci_check_single_serve_dispatch_authority.sh` **fails closed** (forces the single-authority discipline).
- The extraction changes `produce_mode`'s emitted frames → `produce_loopback.rs` / the `ade_network` server tests fail (behavior-drift caught).
- The move accidentally drags `CoordinatorState`/`coordinator_step`/the evidence writer into the shared core → compile pressure on the node spine in S2 + a review/gate signal; **forbidden** (§14).

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-H "Forbidden During This Cluster" prohibitions apply (no `--mode produce` switch; no private-only path; no relay-loop serve mutation; no second `ServedChain` authority; no parallel tag-24 serializer; no proactive `advance_tip`/`changed()` reactor; no new `--mode node` flag; no from-genesis constructor; no new BLUE authority/canonical type/variant; no acceptance without `correlate`; no RO-LIVE flip).
### Slice-Specific Prohibitions
- **No `coordinator_step` / `CoordinatorState` / producer-evidence-writer in the shared core** — that entanglement stays in `produce_mode`'s `handle_listener_event`; the shared core is coordinator-free.
- **No node-spine serve wiring** — S1 does not touch `node_lifecycle`'s On arm (that is S2). `node_lifecycle` must not call the shared core yet.
- **No behavior change to `produce_mode`'s serve** — pure code-motion + re-point; emitted bytes byte-identical.
- **No proactive `advance_tip` / `served_chain_view.changed()` reactor.**
- **No BLUE change; no new `--mode node` flag; no new canonical type.**

## 15. Explicit Non-Goals
This slice MUST NOT: wire the node-spine listener/serve task (S2); write the C1 serve runbook or operator harness (S3); add a proactive `advance_tip` reactor; modify any BLUE crate or the serve reducers' logic; change `produce_mode`'s serve behavior; add any `--mode node` flag/config; claim peer acceptance / BA-02 / RO-LIVE.

## 16. Completion Checklist
- [ ] Slice-entry proof confirmed: the moved core is coordinator-free; the coordinator/evidence wrapping stayed in `produce_mode`.
- [ ] Exactly one serve-dispatch definition (in `ade_runtime`); gate green + fail-closed smoke verified.
- [ ] `produce_loopback.rs` + `ade_network` server tests + `cargo test -p {ade_node,ade_runtime}` + targeted `ade_network` server tests green.
- [ ] The three existing fences byte-unchanged.
- [ ] No BLUE change; no node-spine wiring; no `advance_tip`.
- [ ] `DC-NODE-07` `evidence_notes` record the S1 gate now; final `ci_script` binding + status flip happen at G-H close.

## 17. Review Notes
- **The load-bearing call:** the clean reuse boundary is the *coordinator-free* core, not `handle_listener_event` wholesale. Dragging `coordinator_step`/`CoordinatorState`/the producer evidence writer into the shared module would force the node spine to instantiate the producer coordinator in S2 — wrong. S1's whole value is isolating exactly the part the node spine needs.
- **Why a gate, not just a refactor:** the refactor de-duplicates *today*; the gate prevents a *future* slice from re-introducing a node-spine serve-dispatch — making "single serve authority" durable, not point-in-time.
- **`ServedBlockEvidence` color:** GREEN (pure observation, no I/O) — it moves with the adapter but is not authoritative; the node spine's use of the returned `Vec` is an S2/S3 concern.
- **Implied follow-up:** S2 wires `node_lifecycle`'s On arm to call this shared core over the retained `ServedChainView`; S3 the operator-gated C1 serve.
