# Invariant Slice — PHASE4-N-K S7

## Slice Header

**Slice Name:** RED `ade_node::{cli, main}` — `tokio::main`,
signal handlers, bootstrap wiring, shutdown drain.
**Cluster:** PHASE4-N-K
**Status:** In Progress
**CEs addressed:** CE-N-K-5 (DC-NODE-04).
**Registry effects on merge:** DC-NODE-04 → `enforced` with
`code_locus`, `tests`, `ci_script`, `strengthened_in =
["PHASE4-N-K"]`.
**Dependencies:** S1 (bootstrap), S2 (orchestrator core), S3
(persistent writer), S4–S6 (RED runner pieces).

---

## Intent

The `ade_node` binary is the single composition root. On startup
it calls `bootstrap_initial_state`. On `SIGTERM` / `SIGINT` it
drains the admit/write/snapshot pipeline, force-writes a final
persistent snapshot, then exits 0. On authority-fatal errors
(snapshot decode unknown version, fingerprint mismatch,
chain_write IO during commit) it halts deterministically with a
non-zero exit code.

---

## Scope

- `crates/ade_node/src/cli.rs` — CLI flag parsing.
  Mandatory: `--genesis-path PATH`. Optional: `--network NAME`
  (metadata), `--chain-db PATH`, `--snapshot-store PATH`,
  `--listen ADDR`, `--peer ADDR` (repeatable).
- `crates/ade_node/src/main.rs` — `#[tokio::main]` entry. Wires
  bootstrap → orchestrator → signal handler → shutdown drain.
- `crates/ade_node/Cargo.toml` — add `ade_runtime`, `ade_ledger`,
  `ade_core`, `tokio` deps.

---

## Execution Boundary

- **BLUE:** none.
- **GREEN:** unchanged.
- **RED:** `ade_node::cli`, `ade_node::main`.

---

## Invariants Preserved

- All BLUE authorities (CN-CONS-08, CN-STORE-07, CN-STORE-08).
- CN-NODE-01 — calls the single `bootstrap_initial_state`.

## Invariants Strengthened or Introduced

- DC-NODE-04 (this slice introduces).

---

## Design Summary

`main` body:

1. Parse args via `Cli::parse_from(env::args())`.
2. Open `PersistentChainDb`, build `SnapshotStore` adapter.
3. If genesis-only cold-start: parse genesis bundle, build
   `EraSchedule` + initial `(LedgerState, PraosChainDepState)`.
4. Call `bootstrap_initial_state(...)`. Authority-fatal:
   `BootstrapError::SnapshotDecode(UnknownVersion |
   FingerprintMismatch)` → log structured error, exit 12.
5. Build `OrchestratorState` from bootstrap outputs.
6. Wire `(orchestrator_inbox_rx, orchestrator_inbox_tx) =
   mpsc::channel(1024)`.
7. Spawn `LeadershipSession`, `N2nServerPump`, and N
   `PeerSession`s (initially empty; pump installs them on
   accept).
8. Install signal handler: on `SIGTERM` / `SIGINT`, send
   `OrchestratorEvent::Shutdown` to the inbox.
9. Drive the orchestrator core loop:
   ```rust
   while let Some(event) = orchestrator_inbox_rx.recv().await {
       let effects = orchestrator::core::step(&mut state, event, …)
           .map_err(authority_fatal_to_exit_code)?;
       route(effects).await; // SendToPeer → mpsc, CaptureSnapshot →
                              // writer.on_admitted, ShutdownAcknowledged → break
   }
   ```
10. After loop exits: `writer.force_capture(state.last_observed_slot,
    &state.receive_state.ledger, &state.receive_state.chain_dep)?`.
11. Drop tokio runtime via `Drop`. Exit 0.

Authority-fatal exit codes (mechanical-evidence target):
- 10 — orchestrator `AuthorityFatalKind::ChainWriteIo`.
- 12 — `BootstrapError::SnapshotDecode(UnknownVersion)` or
  `(FingerprintMismatch)` or `Materialize(..)` that propagates
  `SnapshotDecode`.
- 1 — generic startup error (bad CLI, missing genesis, etc.).

---

## Replay, Crash, and Epoch Validation

- **Tests** (integration; in `crates/ade_node/tests/`):
  - `shutdown_then_resume_produces_byte_identical_state` —
    end-to-end: spin up the binary in-process (via
    `tokio::main`-equivalent helper), drive a small recorded
    event vector, send `Shutdown`. Reopen in a fresh process
    against the same chaindb + snapshot store. Assert the
    second bootstrap returns byte-identical `(ledger_fp,
    chain_dep, chain_tip)`.
  - `binary_halts_on_authority_fatal_decode_error` — corrupt
    the snapshot bytes (flip a byte in the embedded
    fingerprint). Spawn the binary process; assert exit code
    12 and a structured stderr line containing
    `FingerprintMismatch`.
  - `binary_halts_on_authority_fatal_unknown_version` — write a
    snapshot with version=99; spawn; assert exit code 12 and
    `UnknownVersion`.
  - `binary_exits_zero_on_clean_shutdown` — spawn, send SIGTERM,
    assert exit 0 + a final snapshot present at the most recent
    observed slot.
  - `cli_requires_genesis_path` — invoke without
    `--genesis-path`; assert exit 1 + a structured error to
    stderr.

## §12 Mechanical Acceptance Criteria

- [ ] `shutdown_then_resume_produces_byte_identical_state`
- [ ] `binary_halts_on_authority_fatal_decode_error`
- [ ] `binary_halts_on_authority_fatal_unknown_version`
- [ ] `binary_exits_zero_on_clean_shutdown`
- [ ] `cli_requires_genesis_path`
- [ ] `ci_check_node_binary_uses_single_bootstrap.sh` —
  `crates/ade_node/src/main.rs` calls
  `ade_runtime::bootstrap::bootstrap_initial_state` exactly
  once and never constructs `(LedgerState, PraosChainDepState,
  …)` triples otherwise.

---

## §14 Hard Prohibitions

- No `unwrap()` / `expect()` / `panic!()` in `main.rs` — all
  errors map to deterministic exit codes.
- No alternative bootstrap path in the binary.
- No silent retry of snapshot decode.
- No indefinite blocking shutdown — shutdown is bounded to
  draining the admit/write/snapshot pipeline; peer sockets may
  be closed mid-stream.

## §15 Explicit Non-Goals

- No live cardano-node peer integration in CI (operator-action,
  RO-LIVE-01 / RO-LIVE-02 / CN-CONS-06 live halves).
- No daemon/service-manager integration.
- No log shipping or metrics export.

---

## §16 Completion Checklist

- [ ] All §12 tests added and passing.
- [ ] CI grep gate added.
- [ ] Registry DC-NODE-04 flipped to `enforced`.
