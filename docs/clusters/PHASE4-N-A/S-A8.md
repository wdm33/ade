# Slice S-A8 — N2C transition authority (4 state machines)

> **Status**: Merged
> **Cluster**: [PHASE4-N-A](cluster.md)

## 2. Slice Header

**Slice Name**: LocalChainSync + LocalTxSubmission + LocalStateQuery + LocalTxMonitor state machines (BLUE)

**Cluster Exit Criteria Addressed**: none. This slice closes no CE on its own — it ships the four remaining N2C state machines for structural completion of the 5-protocol N2C enforcement set (combined with S-A3's N2C handshake portion).

After this slice, every protocol in the cluster TCB color map has a pure-transition state machine + per-protocol event taxonomy.

Out of scope: all 5 cluster CEs.

**Slice Dependencies**: S-A1, S-A2 (LocalChainSync/LocalTxSubmission/LocalStateQuery/LocalTxMonitor messages and types), S-A3 (handshake portion of N2C; supplies pattern), S-A4..S-A7 (DC-PROTO-06 pattern established).

## 4. Intent

`DC-PROTO-01` (mini-protocol state machines have deterministic transitions) gains four N2C protocol enforcements.

`DC-PROTO-04` (full N2C mini-protocol surface: Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor) closes for the four non-handshake protocols. Combined with S-A3's N2C handshake state machine, DC-PROTO-04 reaches code-locus + tests coverage across all 5 protocols.

`DC-PROTO-06` (no ambient session state in BLUE transitions) gains four N2C coverages.

## 5. Scope

**Modules** (under `crates/ade_network/src/n2c/`):
- `mod.rs` — barrel
- `local_chain_sync/{mod,state,agency,event,transition}.rs`
- `local_tx_submission/{mod,state,agency,event,transition}.rs`
- `local_state_query/{mod,state,agency,event,transition}.rs`
- `local_tx_monitor/{mod,state,agency,event,transition}.rs`

**Integration tests** (under `crates/ade_network/tests/`):
- `local_chain_sync_event_trace.rs`
- `local_tx_submission_event_trace.rs`
- `local_state_query_event_trace.rs`
- `local_tx_monitor_event_trace.rs`

**Persistence / network-visible**: none.

**Out of scope**: N2C handshake (already in S-A3); ledger-semantic interpretation of LocalStateQuery `QueryPayload`/`ResultPayload` (opaque pass-through at this layer); mempool-semantic interpretation of LocalTxMonitor `Query`/`Reply` (opaque pass-through); tx-body validation (mempool/ledger); session composition (S-A9); real-capture corpus (S-A9); live cardano-node interop (S-A10).

## 6. Execution Boundary

| Module | Color |
|---|---|
| `ade_network::n2c::*` | **BLUE** |
| All 4 integration tests | **BLUE** |

## 7. Invariants Preserved

`T-DET-01`, `T-CORE-01..03`, `T-INGRESS-01`, `T-CI-01`, `T-BUILD-01`, `T-KEY-01`, `T-BOUND-02`, `T-ERR-01`, `T-ENC-03`, `DC-CORE-01`, `CN-WIRE-07`, `DC-PROTO-02/03/05`.

All 16+1 CI scripts continue to PASS.

## 8. Invariants Strengthened

Three invariant families (DC-PROTO-04 is the primary closure):

- **`DC-PROTO-01`** — protocol state machines have deterministic transitions (four N2C per-protocol enforcements).
- **`DC-PROTO-04`** — full N2C mini-protocol surface coverage. Combined with S-A3 (handshake), DC-PROTO-04 reaches code-locus + tests for all 5 N2C protocols. Status flips per slice §12.
- **`DC-PROTO-06`** — no ambient session state.

Registry strengthenings on this commit:
- `DC-PROTO-01`, `DC-PROTO-06`: `code_locus` += `crates/ade_network/src/n2c/**/*.rs`; `tests` += new tests. No status flip.
- `DC-PROTO-04`: `code_locus` += same paths; `tests` += new tests; `strengthened_in` already contains `PHASE4-N-A` (from S-A2 for codec coverage). Slice §12 specifies whether status flips from "declared" to "enforced" — leave declared until S-A9 real-capture verification lands.

## 9. Design Summary

Four state machines, each in its own `n2c/<protocol>/` submodule. All four use the established pattern (state.rs, agency.rs, event.rs, transition.rs). Output type per protocol: `Event(<ProtoEvent>) | Done`. Errors are structured per-protocol (IllegalTransition, InvalidForVersion, MalformedMessage).

### LocalChainSync

States: `Idle`, `CanAwait`, `MustReply`, `Intersect`, `Done` — mirrors N2N ChainSync structurally, with full `block: Vec<u8>` carried by RollForward instead of header bytes.

Events: `RollForward { block_bytes, tip }`, `RollBackward { point, tip }`, `Intersected { point, tip }`, `NoIntersection { tip }`.

State graph identical to N2N ChainSync — see S-A4 §9. Sole difference: event carries `block_bytes` (full block) rather than header bytes; block-decoding remains opaque at this layer.

### LocalTxSubmission

States: `Idle`, `Busy`, `Done`.

Transitions:
- (Idle, Client, SubmitTx { tx_bytes }) → (Busy, Event(TxSubmitted { tx_bytes }))
- (Busy, Server, AcceptTx(_)) → (Idle, Event(TxAccepted))
- (Busy, Server, RejectTx(rejection)) → (Idle, Event(TxRejected { rejection }))
- (Idle, Client, Done) → (Done, Done)

Events: `TxSubmitted { tx_bytes }`, `TxAccepted`, `TxRejected { rejection: TxRejection }`.

`TxRejection` body is opaque at this layer (ledger-defined reason bytes).

### LocalStateQuery

States: `Idle`, `Acquiring`, `Acquired`, `Querying`, `Done`.

Transitions:
- (Idle, Client, Acquire { point }) → (Acquiring, Event(AcquireRequested { point }))
- (Acquiring, Server, Acquired) → (Acquired, Event(SnapshotAcquired))
- (Acquiring, Server, Failure(reason)) → (Idle, Event(AcquireFailed { reason }))
- (Acquired, Client, Query(payload)) → (Querying, Event(QueryRequested { payload }))
- (Querying, Server, Result(payload)) → (Acquired, Event(QueryReplied { payload }))
- (Acquired, Client, Release) → (Idle, Event(SnapshotReleased))
- (Acquired, Client, ReAcquire { point }) → (Acquiring, Event(ReAcquireRequested { point }))
- (Idle, Client, Done) → (Done, Done)
- (Acquired, Client, Done) → (Done, Done)

Events: `AcquireRequested { point }`, `SnapshotAcquired`, `AcquireFailed { reason }`, `QueryRequested { payload }`, `QueryReplied { payload }`, `SnapshotReleased`, `ReAcquireRequested { point }`.

`QueryPayload` and `ResultPayload` are opaque at this layer (ledger-defined semantic). The state machine owns the closed wire grammar of LSQ, NOT the ledger meaning — per cluster TCB rule on the n2c module.

### LocalTxMonitor

States: `Idle`, `Acquiring`, `Acquired`, `Querying`, `Done`.

Transitions:
- (Idle, Client, Acquire) → (Acquiring, Event(AcquireRequested))
- (Acquiring, Server, AwaitAcquire) → (Acquiring, Event(AwaitingAcquisition))
- (Acquiring, Server, Acquired { slot }) → (Acquired, Event(MempoolAcquired { slot }))
- (Acquired, Client, Query(payload)) → (Querying, Event(QueryRequested { payload }))
- (Querying, Server, Reply(payload)) → (Acquired, Event(QueryReplied { payload }))
- (Acquired, Client, Release) → (Idle, Event(MempoolReleased))
- (Idle, Client, Done) → (Done, Done)
- (Acquired, Client, Done) → (Done, Done)

Events: `AcquireRequested`, `AwaitingAcquisition`, `MempoolAcquired { slot }`, `QueryRequested { payload }`, `QueryReplied { payload }`, `MempoolReleased`.

`LocalTxMonitorQuery` and `LocalTxMonitorReply` bodies opaque at this layer.

### Version threading

Each transition takes its respective version newtype (`LocalChainSyncVersion`, `LocalTxSubmissionVersion`, `LocalStateQueryVersion`, `LocalTxMonitorVersion`). `MAX_LOCAL_*_VERSION = 100` mirrors cluster convention.

### Per-protocol agency

Four distinct agency types: `LocalChainSyncAgency`, `LocalTxSubmissionAgency`, `LocalStateQueryAgency`, `LocalTxMonitorAgency`. None interchangeable; no From/Into to each other or to N2N agency types.

## 10. Changes Introduced

### Types (new)
- 4 × `<Proto>State` enum
- 4 × `<Proto>Agency` enum
- 4 × `<Proto>Output` enum
- 4 × `<Proto>Event` enum
- 4 × `<Proto>Error` enum

### State Transitions
- 4 N2C state machines

## 11. Replay, Crash, and Epoch Validation

Per-protocol unit tests (`n2c::<proto>::transition::tests`):

LocalChainSync (7):
- `local_chain_sync_request_next_then_roll_forward`
- `local_chain_sync_roll_backward_signal`
- `local_chain_sync_find_intersect_known_point`
- `local_chain_sync_find_intersect_unknown`
- `local_chain_sync_client_done_terminates`
- `local_chain_sync_wrong_agency_returns_error`
- `local_chain_sync_version_gating`

LocalTxSubmission (5):
- `submit_tx_then_accept_round_trips`
- `submit_tx_then_reject_carries_reason_bytes`
- `client_done_terminates`
- `wrong_agency_returns_error`
- `version_gating`

LocalStateQuery (8):
- `acquire_then_acquired_transitions_to_acquired`
- `acquire_then_failure_returns_to_idle`
- `query_then_result_round_trips`
- `release_returns_to_idle`
- `re_acquire_from_acquired_transitions_to_acquiring`
- `client_done_from_idle_terminates`
- `client_done_from_acquired_terminates`
- `wrong_agency_returns_error`

LocalTxMonitor (7):
- `acquire_then_acquired_with_slot`
- `acquire_then_await_then_acquired`
- `query_then_reply_round_trips`
- `release_returns_to_idle`
- `client_done_from_idle_terminates`
- `wrong_agency_returns_error`
- `version_gating`

Integration tests (1 per protocol, 4 total):
- `tests/local_chain_sync_event_trace.rs::local_chain_sync_event_trace` — ≥6 scenarios + 1000-run determinism
- `tests/local_tx_submission_event_trace.rs::local_tx_submission_event_trace` — ≥6 scenarios + 1000-run determinism
- `tests/local_state_query_event_trace.rs::local_state_query_event_trace` — ≥6 scenarios + 1000-run determinism
- `tests/local_tx_monitor_event_trace.rs::local_tx_monitor_event_trace` — ≥6 scenarios + 1000-run determinism

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_network --all-targets` — clean
- [ ] `cargo test -p ade_network --lib n2c::` — 27 unit tests PASS
- [ ] `cargo test -p ade_network --test local_chain_sync_event_trace` — PASS
- [ ] `cargo test -p ade_network --test local_tx_submission_event_trace` — PASS
- [ ] `cargo test -p ade_network --test local_state_query_event_trace` — PASS
- [ ] `cargo test -p ade_network --test local_tx_monitor_event_trace` — PASS
- [ ] `cargo clippy -p ade_network --all-targets -- -D warnings` — clean
- [ ] All 8 named CI scripts PASS
- [ ] Registry DC-PROTO-01, DC-PROTO-04, DC-PROTO-06 strengthened: `code_locus` += n2c paths, `tests` += new tests. No status flips.

## 13. Failure Modes

Per-protocol errors (4 × `<Proto>Error`):

| Variant | Cause | Recovery |
|---|---|---|
| `IllegalTransition { state, message_tag, agency }` | Wrong (state, msg, agency) triple | Fail-fast |
| `InvalidForVersion { version, message_tag }` | Out-of-version variant | Fail-fast |
| `MalformedMessage { reason }` | Grammar violation | Fail-fast |

All fail-fast. Replay-safe.

## 14. Hard Prohibitions

Inherited cluster + slice-specific:
- All cluster prohibitions
- Mutating any chain state / ledger state / mempool state (state machines emit events)
- Decoding `block_bytes`, `tx_bytes`, `QueryPayload`, `ResultPayload`, `LocalTxMonitorQuery`, `LocalTxMonitorReply` — all opaque
- Cross-protocol agency type reuse (between any pair of protocols)
- `String` errors
- `HashMap`
- Global / static reads
- `#[non_exhaustive]` on any new enum
- `dyn` dispatch
- `unwrap`/`expect`/`panic` outside `#[cfg(test)]`

## 15. Explicit Non-Goals

- N2N protocols (covered in earlier slices)
- N2C handshake (covered in S-A3)
- Ledger-semantic interpretation of LSQ queries / results
- Mempool-semantic interpretation of LocalTxMonitor queries / replies
- Tx-body validation
- Block-body decoding
- Persistence (N-D)
- Session composition (S-A9)
- Real-capture corpus (S-A9)
- Live cardano-node interop (S-A10)
- Performance optimization

## 16. Completion Checklist

- [ ] Four state machines pure (no ambient state, no globals, no I/O)
- [ ] Four per-protocol agency types, all non-interchangeable
- [ ] Closed enums everywhere (no `#[non_exhaustive]`)
- [ ] Structured errors per protocol
- [ ] Version threaded as explicit input
- [ ] 27 unit + 4 integration tests PASS
- [ ] 1000-run determinism check on each integration test
- [ ] All 16+1 CI scripts PASS
- [ ] Registry DC-PROTO-01/04/06 strengthened (no status flips)
- [ ] No TODOs / placeholders / `unimplemented!()`

## 17. Review Notes

**Invariant risk**:
- **N2C/LSQ closed-grammar / open-semantics boundary**: this slice owns the closed wire grammar of LocalStateQuery; ledger-semantic interpretation of `QueryPayload`/`ResultPayload` lives in a future cluster (N-F). Same for LocalTxMonitor against mempool. The boundary is type-enforced by opaque `Vec<u8>` payloads — no `match` on inner content here.
- **DC-PROTO-04 closure shape**: combined with S-A3, this slice brings all 5 N2C protocols into `code_locus` + `tests` coverage. Status flip to "enforced" is gated on S-A9 real-capture verification per the two-stage closure convention.

**Assumptions challenged**:
- Considered folding 4 protocols into one big module. Rejected — per-protocol modules + agency types matches cluster pattern and keeps grep noise low.
- Considered emitting decoded query results in events. Rejected — keeps the boundary clean and prevents ledger semantics from leaking into the network state machine.

**Follow-up implied**: S-A9 wires all N2C protocols into the session composition; S-A10 verifies live transcript equivalence.

## 18. Authority Reminder

Authority for invariants in `docs/ade-invariant-registry.toml`; mechanical acceptance in §12.
