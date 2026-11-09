# PHASE4-N-Q — Invariants sketch

> **Status:** Planning artifact (non-normative). Frames the live
> producer-mode concept in IDD terms before any cluster-doc
> expansion or slice work.
>
> **Predecessors:** PHASE4-N-C (producer pipeline), PHASE4-N-G
> (`n2n_server` pure state-machine driver + server reducers),
> PHASE4-N-L (`n2n_dialer` precedent for the server-side mirror),
> PHASE4-N-O / N-P (KES infra).
>
> **Closes (on operator pass):** `CN-CONS-06.open_obligation` +
> `RO-LIVE-01.open_obligation`.
>
> **User decisions on file (2026-05-27 session):**
> - OQ1(a) — **GREEN pure coordinator + RED tokio/key-custody driver.** Key
>   custody (KES / VRF / cold) MUST NOT enter coordinator state.
>   Signing is modeled as an effect; RED handles it and returns
>   `ForgeSucceeded` / `ForgeFailed` events.
> - OQ3 — `BroadcastFull` → **fail-closed shutdown** (no
>   backpressure, no silent drop).
> - OQ8 — Slot drift → **skip + closed `slot_missed` evidence**.
>   No retroactive forge.
> - New family prefixes `CN-PROD` / `DC-PROD`. Not folded into
>   `CN-NODE` / `DC-NODE`.
> - Registry entries `CN-PROD-01`, `CN-PROD-02`, `DC-PROD-01`,
>   `DC-PROD-02` approved with refined wording (below).
> - **Reclassification:** `ade_runtime::network::n2n_server` is
>   reclassified from RED (its N-G handoff label) to **GREEN**
>   for N-Q's purposes — it is a pure state-machine driver with
>   no socket I/O; the wire grammar decode/encode lives in
>   `ade_network::codec::*` (BLUE). The N-Q listener (the actual
>   tokio + socket wrapper) is RED. This reclassification is a
>   color-model clarification, not a registry rewrite.

---

## 1. What must always be true

- **I1. Listener is RED.** Every TCP `accept`/`read`/`write`/`flush`
  is in RED only. BLUE and GREEN paths never touch sockets.
- **I2. Per-peer state is BLUE-reducer-driven.** Each accepted
  connection runs the existing `n2n_server` pure driver against
  the BLUE reducers (`producer_chain_sync_serve`,
  `producer_block_fetch_serve`) via the GREEN `ServedChainLookups`
  adapter. The RED listener owns the socket; the GREEN driver owns
  the protocol; BLUE owns the wire grammar (`ade_network::codec`).
- **I3. Slot arithmetic is BLUE/GREEN; wall-clock is RED.**
  Wall-clock reads come from a RED clock seam. Slot computation
  (`current_slot = (now - slot_zero_time) / slot_length`) and the
  consequent ingest into `scheduler_step` are deterministic given
  canonical inputs.
- **I4. Producer pipeline is unchanged.** N-Q drives
  `scheduler_step` per slot tick; it does not modify forge /
  self_accept / opcert validation.
- **I5. `ServedChainSnapshot` updates are atomic relative to peer
  reads.** A peer reading the snapshot sees either the pre- or
  post-update state — never a torn intermediate (no
  half-installed block).
- **I6. Inbound connection lifecycle is closed.**
  `accept → handshake → mux + per-peer driver → graceful close OR
  fail-closed disconnect`. No leaked tasks, no half-open peers.
- **I7. Key custody RED-confined.** `KesSecret` / `VrfSigningKey`
  / `ColdSigningKey` live in the RED **producer shell**, NOT in
  the coordinator state. The coordinator emits a closed
  `RequestForge` effect; the RED shell performs the signing and
  returns a closed `ForgeSucceeded(AcceptedBlock)` /
  `ForgeFailed(structured_error)` event. The GREEN coordinator and
  the per-peer driver never see secret material.
- **I8. Slot-tick / forge-result stream replay-equivalence.**
  For a fixed canonical
  `(initial_coordinator_state, slot_tick_sequence, ledger_snapshot, opcert_metadata, forge_result_event_stream)`,
  the coordinator emits byte-identical broadcast effects and
  byte-identical `ProducerLogEvent` sequence. Replay is over
  canonical slot-tick streams, **not** real wall-clock time.
- **I9. Synthetic-peer integration test proves the loop closes
  Ade-to-Ade.** Before the operator step, an in-process synthetic
  peer (driven by `n2n_dialer`) fetches an Ade-forged block via
  the listener and asserts byte-identity vs `scheduler_step`'s
  output. This proves the wire loop closes structurally; it does
  **NOT** prove cardano-node acceptance. The operator runbook
  remains required for the bounty-facing claim.
- **I10. Operator runbook is reproducible.** A reader with
  cardano-cli + docker access can stand up a private testnet,
  register Ade's pool, run cardano-node + Ade, and capture
  cardano-node-accepts-block evidence by following the documented
  steps. The runbook is **operational** (Tier 5), not a semantic
  invariant.
- **I11. Evidence-capture vocabulary is closed.**
  `ProducerLogEvent` enum:
  `handshake_ok`, `slot_tick{slot, period}`,
  `leader_elected{slot, vrf_output_fingerprint}`,
  `block_forged{slot, hash}`, `block_served{peer_id, hash, bytes_len}`,
  `peer_chain_tip_observed{hash, slot}`,
  `slot_missed{from, to, reason: closed enum}`,
  `coordinator_shutdown{reason: closed enum}`.
  No free-form strings; no key material; no path strings; no
  socket addresses inside the replayable event stream (socket
  addresses are RED operational metadata, surfaced separately from
  the deterministic event log).

## 2. What must never be possible

- **N1. Bare TCP I/O without N2N handshake.** Every accepted
  connection MUST complete `CN-SESS-02` before any mini-protocol
  traffic.
- **N2. Cross-peer state leakage.** Each peer has its own
  `PerPeerN2nServerState`; no shared mutable peer state.
- **N3. Torn `ServedChainSnapshot` writes.** No peer ever reads
  an intermediate state.
- **N4. Forge for a stale slot.** Skip + log per OQ8 below.
- **N5. Forge under wrong KES period.** Slot → KES period is a
  pure function. The RED forge shell rejects a `RequestForge`
  effect whose `kes_period` does not match the live `KesSecret`
  state via `OpCertError::PeriodMismatch` (existing).
- **N6. Silent broadcast-queue overflow.** Fail-closed
  shutdown per OQ3.
- **N7. Server pump under-serves.** Fail-closed (carried from
  DC-CONS-17 / DC-CONS-18 in N-G).
- **N8. Concurrent forge for the same slot.** Scheduler state
  prevents this (existing). Coordinator MUST NOT re-emit
  `RequestForge` for an in-flight slot.
- **N9. Secret-key bytes anywhere in `CoordinatorState`.** Mechanical
  enforcement: the type signature of `CoordinatorState` carries
  only opcert public metadata, ledger snapshot ref, chain tip,
  queue limits, peer limits. **No `KesSecret`/`VrfSigningKey`/
  `ColdSigningKey` field.** Adding one is a compilation error
  for any test exercising the GREEN replay path.
- **N10. Cold key disk-leak.** Cold key loaded once at startup by
  the RED shell, redacted `Debug`, zeroize on `Drop` (existing).
  Same for KES + VRF.
- **N11. Hardcoded network magic / handshake version.** Operator
  supplies via CLI; mismatched peer fails closed (existing).
- **N12. Synthetic-peer test passing only by luck.** The test
  exercises the real wire-format cardano-node would use; negative
  tests (wrong magic, wrong version, malformed envelope) fail
  closed.
- **N13. Operator runbook silent gap.** Every step documented;
  no `(and then somehow)` hand-waves.
- **N14. TLS smuggled in.** N-L deferred TLS via `¬P-8`; N-Q
  stays plaintext.
- **N15. Socket addresses in the replayable event stream.**
  `PeerId` is an opaque `u64` per-process counter; socket
  addresses are RED operational metadata, surfaced separately
  (and excluded from replay-equivalence comparison).
- **N16. Real-time replay claim.** Replay is over canonical
  slot-tick + forge-result streams, NOT real wall-clock.
  Anything that claims "produces same output at real-time T" is
  out of scope.

## 3. What must remain identical across executions

- **D1.** `forge_block(slot, seed, kes_secret_at_period, vrf_secret, cold_key, opcert, ledger_state) → AcceptedBlock` is byte-deterministic (T-DET-01; PHASE4-N-C).
- **D2.** `producer_chain_sync_serve(...)` and
  `producer_block_fetch_serve(...)` are byte-deterministic
  (DC-CONS-17 / 18, N-G).
- **D3.** N2N handshake reply byte-deterministic (DC-SESS-*, N-L).
- **D4.** Slot arithmetic `slot_of(now, slot_zero_time, slot_length)`
  is a pure function. Wall-clock is the only nondeterminism;
  it enters as a canonical input.
- **D5.** Evidence-log events for a fixed canonical input
  sequence are byte-deterministic (modulo wall-clock-timestamp
  metadata in the *line wrapping*, which is non-load-bearing).

## 4. What must be replay-equivalent

- **R1. Producer pipeline.** Same `(slot, seed, ledger_state,
  keys, opcert)` → same `AcceptedBlock` bytes (existing).
- **R2. Server reducers.** Same `(state, msg, snapshot)` → same
  reply bytes (existing).
- **R3. Coordinator slot-tick + forge-result stream.** Same
  `(initial_state, slot_tick_sequence, ledger_snapshot, opcert_metadata, forge_result_event_stream)`
  → same broadcast-effect sequence + same `ProducerLogEvent`
  sequence. **The forge-result event stream is the canonical
  surface across the RED-key-custody boundary; the GREEN
  coordinator is replayable against it without ever seeing
  secrets.**
- **R4. NON-load-bearing.** Real wall-clock timestamps, the order
  of inbound peer connections, TCP buffer arrival timing, and
  socket addresses are NOT replay-load-bearing.

## 5. State transitions in scope

### `ProducerCoordinator` (GREEN pure state machine — corrected per OQ1 amendment)

```text
coordinator_init(cfg) → CoordinatorState

  cfg = (
    opcert_public_metadata,    -- public; NOT secret material
    genesis_anchor,            -- (slot_zero_time, slot_length, slots_per_kes_period)
    initial_ledger_snapshot,   -- frozen reference, ledger-owned
    initial_chain_tip,
    queue_limits,              -- broadcast queue capacity, peer limit
    peer_limits,
  )

  CoordinatorState type-level forbids `KesSecret`/`VrfSigningKey`/
  `ColdSigningKey` fields. Mechanical: no such field declared, no
  `kes-sum` / `vrf-draft03` types referenced inside the type.

coordinator_step(state, event) → Result<(state', Vec<Effect>), CoordinatorError>

  Event = SlotTick { slot }
        | PeerConnected { peer_id, version }
        | PeerDisconnected { peer_id }
        | PeerRequest { peer_id, mini_protocol_msg }
        | ForgeSucceeded { slot, accepted_block }    -- from RED
        | ForgeFailed { slot, structured_error }     -- from RED
        | LedgerSnapshotUpdated { ref }
        | Shutdown

  Effect = RequestForge {                            -- replaces in-line signing
             slot,
             kes_period,
             ledger_snapshot_ref,
             chain_tip,
           }
         | BroadcastBlock { accepted_block }
         | SendToPeer { peer_id, reply_msg }
         | ClosePeer { peer_id }
         | LogEvidence { producer_log_event }

  CoordinatorError = closed enum:
    | SlotDrift { from, to }
    | BroadcastFull
    | UnknownPeer { peer_id }
    | KesPeriodOutOfRange { slot, computed_period, max_period }
    | ScheduleEnded
    | UnexpectedForgeResult { slot, in_flight }
```

### `RED producer shell` — handles RequestForge → emits ForgeSucceeded/ForgeFailed

```text
producer_shell::handle_request_forge(
    cfg_shell,                   -- holds the KesSecret/VrfSigningKey/ColdSigningKey
    request: RequestForge,
    ledger_snapshot,
) → CoordinatorEvent (ForgeSucceeded | ForgeFailed)

  - Verifies kes_period matches live KesSecret's current_period;
    if not, returns ForgeFailed(KesPeriodMismatch).
  - Runs scheduler_step / forge_block / self_accept synchronously
    with current secret material.
  - Returns the resulting Event back into the coordinator's input
    stream.
```

**Key property:** The RED shell is the ONLY surface that ever
touches `KesSecret`/`VrfSigningKey`/`ColdSigningKey`. The
coordinator processes only the `ForgeSucceeded`/`ForgeFailed`
events that the shell emits.

### `n2n_listener` (RED I/O surface — mirror of `n2n_dialer`)

```text
listener_init(addr) → tokio::TcpListener
listener_accept() → (PeerSocket, peer_id := opaque_counter.next())
peer_pump(socket, peer_id, &ServedChainSnapshot, coordinator_tx) → loop
  ├─ drives N2N handshake (N-L code, server-role)
  ├─ drives mux_pump (existing)
  ├─ drives per-peer n2n_server (GREEN per the reclassification)
  ├─ translates inbound mini-protocol msgs → CoordinatorEvent
  └─ translates CoordinatorEffect (SendToPeer) → outbound frames
```

### `ade_node Mode::Produce` (RED main loop)

```text
init: parse CLI → load keys + opcert + genesis → producer_shell::init
                → coordinator_init → spawn listener
main loop (tokio select!):
  ├─ slot_ticker.next()               → coordinator_step(SlotTick)
  │     → if Effect == RequestForge:
  │           producer_shell::handle_request_forge → ForgeSucceeded/ForgeFailed
  │             → coordinator_step(ForgeSucceeded/ForgeFailed)
  ├─ listener.accept()                 → peer_pump task spawn
  ├─ peer_events.next()                → coordinator_step(PeerConnected | PeerRequest | PeerDisconnected)
  └─ shutdown_signal.recv()            → coordinator_step(Shutdown) → drain → exit
```

## 6. TCB color hypothesis

| Component | Color | Notes |
|---|---|---|
| `ade_ledger::producer::forge::forge_block` | **BLUE** (existing) | Unchanged. |
| `ade_core::consensus::self_accept` | **BLUE** (existing) | Unchanged. |
| `ade_network::block_fetch::server::producer_block_fetch_serve` | **BLUE** (existing) | Unchanged. |
| `ade_network::chain_sync::server::producer_chain_sync_serve` | **BLUE** (existing) | Unchanged. |
| `ade_network::codec::*` | **BLUE** (existing) | Wire grammar; deterministic. |
| `ade_runtime::producer::scheduler::scheduler_step` | **BLUE** (existing) | Unchanged. |
| `ade_runtime::producer::broadcast_to_served` | **GREEN** (existing) | Unchanged. |
| `ade_runtime::producer::served_chain_lookups` | **GREEN** (existing) | Unchanged. |
| `ade_runtime::network::n2n_server` | **GREEN** (reclassified) | Pure state-machine driver; no I/O. Decodes via BLUE codec → calls BLUE reducers → encodes via BLUE codec. N-G's handoff classified this as RED; the IDD-color-model-correct classification is GREEN (deterministic glue around BLUE). N-Q uses this clarification without rewriting N-G's registry. |
| `ade_runtime::producer::coordinator` (new) | **GREEN** | Pure state machine `(state, event) → (state', effects)`. No I/O. **No secret-key material in state.** |
| `ade_runtime::producer::producer_log` (new) | **GREEN** | Closed `ProducerLogEvent` enum (closed reason enums; no free-form strings). |
| `ade_runtime::producer::producer_shell` (new) | **RED** | Holds `KesSecret`/`VrfSigningKey`/`ColdSigningKey`. Handles `RequestForge` effect by running the producer pipeline + emitting `ForgeSucceeded`/`ForgeFailed` events. |
| `ade_runtime::network::n2n_listener` (new) | **RED** | Tokio TCP listener + per-peer pump task. Mirror of `n2n_dialer`. |
| `ade_node::produce_mode` (new) | **RED** | Main-loop wiring: CLI args, tokio runtime, slot ticker, coordinator init, shell init, listener init, shutdown signal, evidence-log writer. |
| Synthetic-peer integration test | `#[cfg(test)]` | Uses existing `n2n_dialer` to drive against the new `n2n_listener` end-to-end. Proves Ade-to-Ade loop closure only. |

## 7. Open questions — resolved

| OQ | Decision |
|---|---|
| OQ1 | **GREEN coordinator + RED producer shell + RED tokio driver.** Key custody never enters coordinator state. Signing is a closed effect. |
| OQ2 | `PeerId` = opaque `u64` (coordinator-internal counter). Socket addresses are RED operational metadata, surfaced separately. |
| OQ3 | `BroadcastFull` → fail-closed shutdown. |
| OQ4 | N peers concurrent (per-peer state already independent per N-G); integration test exercises 1; multi-peer load is out of scope. |
| OQ5 | Synthetic-peer test uses `n2n_dialer` → loopback against `n2n_listener`. **Caveat:** proves Ade-to-Ade only; operator runbook still required for cardano-node-acceptance evidence. |
| OQ6 | Operator runbook targets **Conway-era genesis directly** via `cardano-cli conway genesis` (avoid hardfork orchestration). **Classified as operational, not a semantic invariant** — the implementation does NOT assume Conway-only; the full N2N/N2C surface contracts remain era-aware per existing version-gating. |
| OQ7 | Closed-vocabulary JSONL evidence; `ProducerLogEvent` distinct from `LiveLogEvent` / `AdmissionLogEvent`. |
| OQ8 | Slot drift → skip + log `slot_missed`. Policy: `if current_slot > target_slot before RED forge begins OR before RED forge returns: discard, emit slot_missed{from, to, reason}`. The log is evidence; not semantic authority. |
| OQ9 | Genesis-anchor (`slot_zero_time`, `slots_per_kes_period`) extracted from genesis JSON; explicit CLI flag for `--genesis-anchor-slot` default to avoid hidden invariants. |
| OQ10 | Single listener for N-Q; multi-listener future cluster. |

## 8. Cannot-yet-be-expressed concerns

The concept **can** be expressed as a pure transformation:

```
(genesis_anchor, opcert_public_metadata, ledger_snapshot,
 slot_event_stream, peer_event_stream, forge_result_event_stream)
  → (broadcast_effect_stream, send_to_peer_effect_stream, producer_log_event_stream)
```

Nondeterminism enters at:
1. Wall-clock reads — RED input. Bytes are slot-determined.
2. Inbound peer arrival order — RED input. Per-peer state independent.
3. TCP buffer arrival timing — RED input. Handshake/mux tolerates fragmentation.
4. The RED producer shell's signing operations — folded into the
   `forge_result_event_stream`, which is the canonical surface for
   coordinator replay. **The coordinator is replayable against
   that stream without ever seeing secrets.**

None are authoritative for chain validity.

---

## Registry entries (approved with edits applied)

Written in the project's actual schema. Family prefixes
`CN-PROD` / `DC-PROD` are new and reserved for live-producer-mode
operational invariants.

```toml
[[rules]]
id = "CN-PROD-01"
tier = "derived"
statement = "Producer-mode listener completes the N2N handshake (CN-SESS-02) on every accepted inbound connection before any mini-protocol traffic is exchanged. Pre-handshake socket bytes never reach the n2n_server reducers; handshake failure fail-closes the connection. Bytes from a peer that has not completed handshake are dropped at the boundary."
source = "docs/planning/phase4-n-q-invariants.md §1 (I1, I6); §2 (N1, N15)"
cross_ref = ["CN-SESS-02", "DC-SESS-01", "DC-SESS-02", "RO-LIVE-01", "CN-CONS-06"]
code_locus = "TBD — populated by N-Q listener slice (ade_runtime/src/network/n2n_listener.rs)"
tests = []
ci_script = ""
status = "declared"
introduced_in = "PHASE4-N-Q"
strengthened_in = []

[[rules]]
id = "CN-PROD-02"
tier = "derived"
statement = "Producer slot loop never signs a block whose KES period has rotated past current_period. Slot → KES period is a pure function of (slot, genesis_kes_anchor, slots_per_kes_period); the coordinator fail-closes (slot_missed event + log) when wall-clock has advanced past the target slot before forge completes. No retroactive forge. **The GREEN coordinator never owns or stores private signing material** — it emits a closed `RequestForge { slot, kes_period, ledger_snapshot_ref, chain_tip }` effect; the RED producer shell either returns an `AcceptedBlock` via a `ForgeSucceeded` event or a structured `ForgeFailed { slot, structured_error }` event. KesSecret / VrfSigningKey / ColdSigningKey never enter CoordinatorState. T-tier key-custody boundary."
source = "docs/planning/phase4-n-q-invariants.md §1 (I3, I4, I7, I8); §2 (N4, N5, N8, N9); §5 (corrected state-transition signature)"
cross_ref = ["T-KEY-01", "DC-CRYPTO-04", "DC-CRYPTO-05", "DC-CONS-11", "OP-OPS-04", "OP-OPS-05"]
code_locus = "TBD — populated by N-Q coordinator slice (ade_runtime/src/producer/coordinator.rs) + RED shell slice (ade_runtime/src/producer/producer_shell.rs)"
tests = []
ci_script = ""
status = "declared"
introduced_in = "PHASE4-N-Q"
strengthened_in = []

[[rules]]
id = "DC-PROD-01"
tier = "derived"
statement = "Producer-mode evidence log emits a closed `ProducerLogEvent` vocabulary: handshake_ok, slot_tick, leader_elected, block_forged, block_served, peer_chain_tip_observed, slot_missed{reason: closed_enum}, coordinator_shutdown{reason: closed_enum}. No free-form reason strings; no key material; no path strings. **Socket addresses MUST NOT appear inside the replayable event stream** — `PeerId` is an opaque `u64` (coordinator-internal counter); socket addresses are RED operational metadata, surfaced separately and excluded from replay-equivalence comparison. Mirrors the LiveLogEvent / AdmissionLogEvent precedent established in N-L / N-M while remaining a distinct vocabulary."
source = "docs/planning/phase4-n-q-invariants.md §1 (I11); §2 (N9, N12, N13, N15)"
cross_ref = ["DC-NODE-01", "DC-NODE-02", "DC-NODE-03", "DC-NODE-04"]
code_locus = "TBD — populated by N-Q producer_log slice (ade_runtime/src/producer/producer_log.rs)"
tests = []
ci_script = ""
status = "declared"
introduced_in = "PHASE4-N-Q"
strengthened_in = []

[[rules]]
id = "DC-PROD-02"
tier = "derived"
statement = "Coordinator slot-tick + forge-result stream replay-equivalence. For a fixed initial CoordinatorState, fixed canonical slot-tick sequence, fixed ledger state, fixed opcert public metadata, and fixed RED forge-result event stream (ForgeSucceeded | ForgeFailed sequence), the coordinator emits byte-identical broadcast effects and byte-identical ProducerLogEvent sequence. Wall-clock real-time timestamps and socket arrival order are non-load-bearing RED metadata. **Replay is over canonical event streams — NOT real wall-clock time.** The forge-result event stream is the canonical surface across the RED-key-custody boundary; the GREEN coordinator is replayable against it without ever seeing secret material."
source = "docs/planning/phase4-n-q-invariants.md §1 (I7, I8); §3 (D5); §4 (R3, R4); §8"
cross_ref = ["T-DET-01", "DC-CONS-13", "DC-CONS-14", "CN-PROD-02"]
code_locus = "TBD — populated by N-Q coordinator slice + N-Q replay-test slice (synthetic slot-tick + forge-result corpus)"
tests = []
ci_script = ""
status = "declared"
introduced_in = "PHASE4-N-Q"
strengthened_in = []
```

## Strengthenings to record on N-Q close (not appended now)

- `CN-CONS-06.strengthened_in += "PHASE4-N-Q"`; `open_obligation` clears IF the operator pass captures live evidence; otherwise the entry stays `partial` with the obligation narrowed to "blocked_until_operator_runbook_executed".
- `RO-LIVE-01.strengthened_in += "PHASE4-N-Q"`; same closure semantics.
- `DC-CONS-17`, `DC-CONS-18` — strengthened (now exercised end-to-end via a real socket through the integration test).
- `CN-SESS-02` — strengthened (handshake now exercised in server-role direction).
- `DC-PROTO-08` (server-agency deterministic resolution, N-G) — strengthened.

---

## Related

- [[project-phase4-n-c-handoff]] — the producer pipeline N-Q drives.
- [[project-phase4-n-g-handoff]] — `n2n_server` reducers + server pump.
- [[project-phase4-n-l-closed]] — `n2n_dialer` precedent for the mirror.
- [[project-phase4-n-o-closed]] / [[project-phase4-n-p-closed]] —
  KES infra the RED shell consumes.
- [[project-bounty-requirements]] — bounty's "cardano-node accepts an
  Ade-forged block" gate.
- [[feedback-shell-must-not-overstate-semantic-truth]] — operator log
  is evidence, not authority.
- [[feedback-bounded-smoke-slices]] — synthetic-peer test is bounded
  proof; operator runbook is the bounty-facing artifact.
- [[feedback-fail-closed-validation]] — BroadcastFull and SlotDrift
  both fail-closed per OQ3 / OQ8.
