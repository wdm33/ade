# Invariant Slice — PHASE4-N-K S6

## Slice Header

**Slice Name:** RED `orchestrator::n2n_server_pump` — listening
socket + per-peer N2N server task spawner.
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** none directly (DC-NODE-01 isolation already
covered by S4; this slice is the producer-side analogue).
**Registry effects on merge:** none.
**Dependencies:** S2, S4 (peer-session pattern).

---

## Intent

Make the N2N listening socket a thin per-connection task spawner.
Each accepted connection becomes a `PeerSession` with the
`ChainSyncServer` / `BlockFetchServer` frame kinds. Connection-
lifecycle errors stay scoped to that connection.

---

## Scope

- `crates/ade_runtime/src/orchestrator/n2n_server_pump.rs` — RED
  bound-socket loop:
  ```rust
  pub struct N2nServerPump {
      pub listener: TcpListener,
      pub events_out: mpsc::Sender<OrchestratorEvent>,
      pub next_peer_id: PeerIdGenerator,
  }
  pub async fn N2nServerPump::run(self);
  ```
- `crates/ade_runtime/src/orchestrator/mod.rs` — re-export.

Because the Ouroboros mux + handshake driver layer above
`MuxTransport` is not in this cluster, the actual frame layer
this slice provides is a **bounded honest stub** matching the
existing live-binary convention: the pump accepts connections,
emits `PeerConnected` to the orchestrator, and stays
"connected-but-quiet" until an operator-action layer adds the
mux driver. The pump's correctness contract is the spawn-per-
connection isolation, not the wire frame parsing.

This honesty matches the project's existing
`live_block_follow_session` discipline — mechanical evidence
covers the pure orchestrator; live wire evidence is operator-
action work.

---

## Execution Boundary

- **BLUE:** none.
- **GREEN:** unchanged.
- **RED:** `orchestrator::n2n_server_pump`.

---

## Invariants Preserved

- DC-NODE-01 — per-peer isolation enforced at the orchestrator
  core; the pump's per-connection spawn is a structural mirror.

## Invariants Strengthened or Introduced

- Structural mirror only; flips at S4.

---

## Design Summary

Loop body:
1. `(stream, addr) = listener.accept().await?`.
2. Allocate `peer_id = next_peer_id.next()`.
3. Spawn `PeerSession::run(...)` over an `mpsc::Receiver` whose
   sender is held by the connection task.
4. Send `OrchestratorEvent::PeerConnected { peer_id, … }` to the
   orchestrator.

`PeerIdGenerator` is a monotonically increasing `AtomicU64` —
this keeps `peer_id` allocation deterministic at the value-level
(not session-startup-time) across runs that accept connections
in the same order.

---

## Replay, Crash, and Epoch Validation

- **Tests:**
  - `n2n_server_pump_spawns_per_connection` — drive two
    concurrent connections; expect two distinct `peer_id`s in
    the orchestrator's `PeerConnected` events.
  - `n2n_server_pump_connection_drop_does_not_affect_peer_b` —
    connection A drops; orchestrator emits `PeerDisconnected`
    only for A; B's session continues.
  - `peer_id_generator_is_monotonic_and_deterministic_per_seed`.

(The "real frame parsing" tests live under existing N-G fixtures.
This slice's tests cover the spawn structure.)

## §12 Mechanical Acceptance Criteria

- [ ] `n2n_server_pump_spawns_per_connection`
- [ ] `n2n_server_pump_connection_drop_does_not_affect_peer_b`
- [ ] `peer_id_generator_is_monotonic_and_deterministic_per_seed`

---

## §14 Hard Prohibitions

- No `unwrap()` / `expect()` / `panic!()` in non-test code.
- No shared mutable state across connection tasks except
  `next_peer_id` (atomic).
- No frame parsing in this file — frames go through the
  orchestrator + the existing N-G server driver.

## §15 Explicit Non-Goals

- No mux session driver. The Ouroboros multiplexer layer above
  `MuxTransport::read_raw / write_raw` is operator-action work.
- No TLS / authentication.
- No incoming-connection rate limiting (Tier 5 future).

---

## §16 Completion Checklist

- [ ] All §12 tests added and passing.
- [ ] No new registry flip.
