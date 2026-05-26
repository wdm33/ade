# Invariant Slice — PHASE4-N-K S2

## Slice Header

**Slice Name:** GREEN `orchestrator::core::step` reducer +
`OrchestratorEvent` / `OrchestratorEffect` / `OrchestratorError`
sums.
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** Part of CE-N-K-2 (DC-NODE-01 type-level
isolation), CE-N-K-4 (DC-NODE-03 — pure reducer is replayable).
**Registry effects on merge:** none yet (DC-NODE-03 flips at
S8; DC-NODE-01 flips at S4).
**Dependencies:** S1 (`Clock`), N-H receive driver, N-G server
driver, N-I rollback driver, N-J snapshot framing.

---

## Intent

Make the orchestrator core a single pure reducer over a closed
event sum. All authority dispatch (admit, materialize, encode,
write) flows through this reducer; the RED tokio runner is a thin
adapter that turns sockets + ticks into events and effects into
socket writes.

---

## Scope

- **New modules:**
  - `crates/ade_runtime/src/orchestrator/mod.rs`
  - `crates/ade_runtime/src/orchestrator/event.rs`
  - `crates/ade_runtime/src/orchestrator/core.rs`
  - `crates/ade_runtime/src/orchestrator/state.rs`
- **State machines affected:** introduces `OrchestratorState`
  (composes `ReceiveState` + cadence-tracking).
- **Persistence impact:** none directly (S3 writes; S2 only
  emits the `CaptureSnapshot` effect when appropriate).
- **Network-visible impact:** none directly (effects are routed
  by S4/S5/S6).

---

## Execution Boundary

- **BLUE:** none.
- **GREEN:** all of `orchestrator/{mod,event,core,state}.rs`.
- **RED:** none in this slice.

---

## Invariants Preserved

- CN-CONS-08 — `step` only mutates `ReceiveState` via
  `receive_apply` (called inside `dispatch_*_inbound`).
- CN-STORE-07 — rollback handling routes through
  `materialize_rolled_back_state` + `commit_rollback`.
- DC-CONS-20 — half-committed rollback impossible
  (`commit_rollback` is the only mutating path).

## Invariants Strengthened or Introduced

- DC-NODE-03 (partial: pure reducer is a precondition).
- DC-NODE-01 (partial: type-level per-peer state isolation in
  `OrchestratorState`).

---

## Design Summary

```rust
pub enum OrchestratorEvent {
    SlotTick { slot_millis: u64, slot: SlotNo },
    PeerChainSyncFrame { peer_id: PeerId, bytes: Vec<u8> },
    PeerBlockFetchFrame { peer_id: PeerId, bytes: Vec<u8> },
    PeerN2nServerChainSyncFrame { peer_id: PeerId, bytes: Vec<u8> },
    PeerN2nServerBlockFetchFrame { peer_id: PeerId, bytes: Vec<u8> },
    PeerConnected { peer_id: PeerId, cs_version: ChainSyncVersion, bf_version: BlockFetchVersion },
    PeerDisconnected { peer_id: PeerId },
    Shutdown,
}

pub enum OrchestratorEffect {
    SendToPeer { peer_id: PeerId, bytes: Vec<u8> },
    CaptureSnapshot { slot: SlotNo },
    AdmittedBlock { slot: SlotNo, hash: Hash32 },
    PeerSessionHalted { peer_id: PeerId, reason: PeerHaltReason },
    ShutdownAcknowledged,
}

pub enum OrchestratorError {
    AuthorityFatal(AuthorityFatalKind),
}

pub enum AuthorityFatalKind {
    ChainWriteIo,
    SnapshotDecodeUnknownVersion,
    SnapshotDecodeFingerprintMismatch,
}

pub struct OrchestratorState {
    pub receive_state: ReceiveState,
    pub per_peer_receive: BTreeMap<PeerId, PerPeerReceiveState>,
    pub per_peer_server: BTreeMap<PeerId, PerPeerN2nServerState>,
    pub cadence: SnapshotCadence,
    pub last_persistent_snapshot_slot: Option<SlotNo>,
    pub shutdown_requested: bool,
}

pub fn step<W: ChainDbWrite>(
    state: &mut OrchestratorState,
    event: OrchestratorEvent,
    chain_write: &mut W,
    served_snapshot: &ServedChainSnapshot,
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<Vec<OrchestratorEffect>, OrchestratorError>;
```

Routing rules per event:
- `SlotTick` — for producer leadership-check (S5 wires) — in this
  slice, only updates state.last_observed_slot.
- `PeerChainSyncFrame` / `PeerBlockFetchFrame` — call
  `dispatch_chain_sync_inbound` / `dispatch_block_fetch_inbound`
  on the per-peer state. Decode errors → emit
  `PeerSessionHalted{ DecodeError }`, leave other peers
  unaffected. On `Admitted` effect, check cadence; if due,
  emit `CaptureSnapshot { slot }` (S3 hooks the writer).
- `PeerN2nServerChainSyncFrame` / `PeerN2nServerBlockFetchFrame`
  — call `dispatch_chain_sync_frame` / `dispatch_block_fetch_frame`
  on the per-peer server state. Outgoing frames become
  `SendToPeer` effects.
- `PeerConnected` / `PeerDisconnected` — install / remove per-
  peer state. Disconnect drops the per-peer map entry; never
  touches other peers' state.
- `Shutdown` — set `shutdown_requested = true`; emit
  `ShutdownAcknowledged`.

Authority-fatal errors:
- A `ReceiveError` reading `ChainWriteError::Underlying(Io)` is
  authority-fatal. All other `ReceiveError` variants are
  per-peer-fatal only.

---

## Replay, Crash, and Epoch Validation

- **Replay tests:**
  - `step_two_runs_produce_byte_identical_effects` — replay the
    same event vector twice; expect identical `Vec<Effect>`.
  - `step_per_peer_decode_error_isolates` — a malformed chain-sync
    frame from peer A produces `PeerSessionHalted { peer_id: A }`
    but does not perturb peer B's state nor the global ledger.
  - `step_admit_triggers_capture_snapshot_at_cadence` —
    `should_snapshot_after_block` agreement with the in-slice
    routing.
  - `step_shutdown_drains_then_halts` — after `Shutdown`,
    `step` rejects new ticks (returns no effects beyond the
    drain envelope).

## §12 Mechanical Acceptance Criteria

- [ ] `step_two_runs_produce_byte_identical_effects`
- [ ] `step_per_peer_decode_error_isolates`
- [ ] `step_admit_triggers_capture_snapshot_at_cadence`
- [ ] `step_per_peer_server_routes_frames_to_outgoing_effect`
- [ ] `step_shutdown_drains_then_halts`
- [ ] `step_authority_fatal_chain_write_io_returns_error`
- [ ] `ci_check_orchestrator_core_purity.sh` — orchestrator/core.rs
  contains no `tokio::*`, `SystemTime::*`, `Instant::*`, `rand::*`,
  `HashMap`, `HashSet`, `f32`, `f64`.

---

## §14 Hard Prohibitions

- No `tokio::*` imports in `orchestrator/{mod,event,core,state}.rs`.
- No `SystemTime::*` / `Instant::*` reads in the same files.
- No `HashMap` / `HashSet` — use `BTreeMap` / `BTreeSet`.
- No `unwrap()` / `panic!()` / `expect()` in non-test code.
- No reimplementation of authority paths — `step` calls
  `dispatch_chain_sync_inbound`, `dispatch_block_fetch_inbound`,
  `dispatch_chain_sync_frame`, `dispatch_block_fetch_frame`,
  `commit_rollback`, never inlines their logic.

## §15 Explicit Non-Goals

- No socket I/O (S4/S5/S6).
- No persistent snapshot capture (S3 holds the writer; S2
  only emits the `CaptureSnapshot` effect).
- No producer forging logic (already in `ade_ledger::producer`;
  S5 wires).
- No mempool integration (Phase 4 N-E).

---

## §16 Completion Checklist

- [ ] All §12 tests added and passing.
- [ ] CI purity gate passes.
- [ ] Registry remains untouched (flips happen at S4/S8).
