# PHASE4-N-K — Closure record

> Companion to `cluster.md`. Records the mechanical evidence
> shipped, the registry deltas applied, and the carry-forward
> obligations after closure.

## Registry deltas applied

| Rule | Change | Notes |
|------|--------|-------|
| `CN-NODE-01` | `declared → enforced` | single `pub fn bootstrap_initial_state` in `ade_runtime::bootstrap`; cold-start + warm-start are two branches of one function. `ci/ci_check_bootstrap_closure.sh`. |
| `DC-NODE-01` | `declared → enforced` | per-peer state in `OrchestratorState::per_peer_{receive,server}` BTreeMaps; decode/validity errors emit `PeerSessionHalted` (closed reason discriminant) and remove only that peer; sibling peers + producer continue. `ci/ci_check_peer_session_isolation.sh`. |
| `DC-NODE-02` | `declared → enforced` | `PersistentSnapshotWriter::on_admitted` consults `should_snapshot_after_block` exclusively; orchestrator core also routes through it. `ci/ci_check_persistent_writer_no_parallel_cadence.sh`. No `open_obligation` (eviction is a storage concern, not cadence fidelity). |
| `DC-NODE-03` | `declared → enforced` | `Clock` trait in `ade_runtime::clock`; `SystemClock` (RED sub-classified) is the SOLE wall-clock-reading site; `DeterministicClock` drives the replay harness. `ci/ci_check_clock_seam.sh`. |
| `DC-NODE-04` | `declared → enforced` | `ade_node::node::run_node_until_shutdown` maps authority-fatal kinds to `EXIT_AUTHORITY_FATAL_IO=10` / `EXIT_AUTHORITY_FATAL_DECODE=12` / `EXIT_GENERIC_STARTUP=1`; force-captures a final snapshot via `PersistentSnapshotWriter::force_capture` on shutdown drain. `ci/ci_check_node_binary_uses_single_bootstrap.sh`. No `open_obligation` (schema-migration tooling is DC-STORE-09's home). |
| `DC-STORE-09` | strengthened + `open_obligation += "snapshot_schema_migration_follow_on_cluster"` | The open_obligation captures the missing operator-facing v1→v2 migration tooling; DC-NODE-04 deliberately does not carry this. |

## Existing rules strengthened (`strengthened_in += "PHASE4-N-K"`)

- `T-DET-01` — replay equivalence now extends across the
  orchestrator core under clock injection.
- `CN-CONS-08` — admit path driven end-to-end by the production
  orchestrator.
- `CN-STORE-07` — materialize authority's caller is now the
  production bootstrap warm-start branch.
- `CN-STORE-08` — encode/decode driven by the production
  orchestrator (bootstrap, persistent writer, shutdown drain).
- `DC-CONS-21` — round-trip equivalence exercised end-to-end at
  bootstrap warm-start + shutdown-resume.
- `DC-STORE-08` — encoder canonicality exercised by persistent
  writer + shutdown drain.

## Mechanical artifacts shipped

### New GREEN files (8)
- `crates/ade_runtime/src/bootstrap.rs` — `bootstrap_initial_state`
  (CN-NODE-01).
- `crates/ade_runtime/src/clock.rs` — `Clock` trait,
  `DeterministicClock`, `SystemClock` (DC-NODE-03 seam).
- `crates/ade_runtime/src/orchestrator/mod.rs` — module barrel.
- `crates/ade_runtime/src/orchestrator/event.rs` — closed
  `OrchestratorEvent` / `OrchestratorEffect` / `OrchestratorError`
  / `PeerHaltReason` / `AuthorityFatalKind` sums + `PeerId` +
  `PeerRole`.
- `crates/ade_runtime/src/orchestrator/state.rs` —
  `OrchestratorState` + `PerPeerReceiveVersions`.
- `crates/ade_runtime/src/orchestrator/core.rs` — pure
  `step` reducer.
- `crates/ade_runtime/src/rollback/persistent_writer.rs` —
  `PersistentSnapshotWriter` (DC-NODE-02).
- `crates/ade_node/src/cli.rs` — CLI parser.
- `crates/ade_node/src/lib.rs` — library entry.
- `crates/ade_node/src/node.rs` — `run_node_until_shutdown` +
  authority-fatal exit-code mapping.

### New RED files (3)
- `crates/ade_runtime/src/orchestrator/peer_session.rs` —
  per-peer tokio task (DC-NODE-01 structural mirror).
- `crates/ade_runtime/src/orchestrator/leadership_session.rs` —
  slot-tick pump driven by `Clock`.
- `crates/ade_runtime/src/orchestrator/n2n_server_pump.rs` —
  listening-socket per-connection spawner.

### New integration tests (3 files, 7 tests)
- `crates/ade_runtime/tests/orchestrator_peer_isolation.rs`
  (3 tests; DC-NODE-01).
- `crates/ade_runtime/tests/orchestrator_replay_equivalence.rs`
  (2 tests; DC-NODE-03).
- `crates/ade_node/tests/shutdown_resume_identity.rs`
  (3 tests; DC-NODE-04).
- `crates/ade_node/tests/authority_fatal_decode.rs`
  (1 test; DC-NODE-04 fail-fast).

### New CI scripts (6)
- `ci/ci_check_bootstrap_closure.sh` (CN-NODE-01)
- `ci/ci_check_clock_seam.sh` (DC-NODE-03)
- `ci/ci_check_orchestrator_core_purity.sh` (DC-NODE-03 +
  general purity)
- `ci/ci_check_persistent_writer_no_parallel_cadence.sh`
  (DC-NODE-02)
- `ci/ci_check_peer_session_isolation.sh` (DC-NODE-01)
- `ci/ci_check_node_binary_uses_single_bootstrap.sh`
  (CN-NODE-01 + DC-NODE-04)

## Tokio dependency note

`ade_runtime` gained an explicit `tokio` dependency to host the
three RED runner files (`peer_session.rs`,
`leadership_session.rs`, `n2n_server_pump.rs`). The GREEN
orchestrator core (`core.rs`, `event.rs`, `state.rs`, `mod.rs`)
MUST NOT import `tokio::*` —
`ci/ci_check_orchestrator_core_purity.sh` and
`ci/ci_check_clock_seam.sh` enforce this together.

## Open obligations carried after closure

- `DC-STORE-09` carries
  `open_obligation = "snapshot_schema_migration_follow_on_cluster"`
  (v1→v2 schema migration tooling — operator-facing, not a
  node-binary concern).
- `RO-LIVE-01`, `RO-LIVE-02`, `CN-CONS-06` live-evidence halves
  remain `blocked_until_operator_peer_available` — closing them
  is a separate operator-action cluster running `ade_node`
  against a private cardano-node peer.

## Honest-scope note (RED runner)

The RED tokio runner files in this cluster (peer session,
leadership session, server pump, `ade_node::main`) follow the
project's established `live_block_follow_session` discipline:
the orchestrator core, bootstrap, and persistent writer are
real and mechanically evidenced; the actual Ouroboros mux +
handshake driver above `ade_network::mux::MuxTransport` is
operator-action work tracked by RO-LIVE-01 / RO-LIVE-02. The
binary today, when launched, performs bootstrap and prints a
readiness line; the chain-sync / block-fetch socket pump is
the follow-on cluster's deliverable.

Mechanical evidence for DC-NODE-01 is provided by the
orchestrator-core dispatch tests + the integration tests that
prove per-peer isolation against the pure reducer. Mechanical
evidence for DC-NODE-04 is provided by the in-process
shutdown-resume integration test which exercises the full
bootstrap → orchestrator → drain → bootstrap cycle.
