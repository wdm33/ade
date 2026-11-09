# PHASE4-N-Q — Cluster slice plan

> **Status:** Planning artifact. Output of `/cluster-plan` from the
> confirmed invariants sketch at
> [`phase4-n-q-invariants.md`](phase4-n-q-invariants.md).
>
> **Predecessors:** PHASE4-N-P close (HEAD `037bad8`); N-C / N-G /
> N-L / N-O / N-P all upstream.
> **Closes (on S6 operator pass):** `CN-CONS-06.open_obligation`,
> `RO-LIVE-01.open_obligation`.

---

## Cluster shape: **6 slices**

Each slice strengthens a specific invariant cluster and is
independently mergeable. The dependency chain is linear: each
slice depends on its predecessor's artifacts but doesn't reach
into later slices' work. S6 is the operator-action closure.

### S1 — Planning artifacts (this commit)

**Strengthens:** none yet (declarative). Lands the invariants
doc + cluster doc + this plan + S1 slice doc + the four new
registry entries (`CN-PROD-01`, `CN-PROD-02`, `DC-PROD-01`,
`DC-PROD-02`) as `status = "declared"`.

**Deliverables:**
- `docs/planning/phase4-n-q-invariants.md` (already drafted)
- `docs/planning/phase4-n-q-cluster-slice-plan.md` (this file)
- `docs/clusters/PHASE4-N-Q/cluster.md` (full cluster doc)
- `docs/clusters/PHASE4-N-Q/S1.md` (this slice's MAC)
- `docs/ade-invariant-registry.toml` (4 new entries appended)

**No code.** S1 is purely planning + registry; mirrors
PHASE4-N-P's S1.

### S2 — GREEN `coordinator` (pure state machine)

**Strengthens:** `CN-PROD-02` (no secret material in coordinator
state), `DC-PROD-02` (slot-tick + forge-result replay
equivalence).

**Deliverables:**
- New `crates/ade_runtime/src/producer/coordinator.rs` (GREEN):
  - `CoordinatorState` (no `KesSecret`/`VrfSigningKey`/`ColdSigningKey`
    field — mechanically enforced by the type signature).
  - `CoordinatorEvent` closed enum: `SlotTick`,
    `PeerConnected`, `PeerDisconnected`, `PeerRequest`,
    `ForgeSucceeded`, `ForgeFailed`, `LedgerSnapshotUpdated`,
    `Shutdown`.
  - `CoordinatorEffect` closed enum: `RequestForge`,
    `BroadcastBlock`, `SendToPeer`, `ClosePeer`, `LogEvidence`.
  - `CoordinatorError` closed enum (per §5 of invariants doc).
  - `pub fn coordinator_init(cfg) -> CoordinatorState`.
  - `pub fn coordinator_step(state, event) -> Result<(state', Vec<Effect>), CoordinatorError>`.
- New `crates/ade_runtime/src/producer/producer_log.rs` (GREEN):
  - `ProducerLogEvent` closed enum (per §1.I11).
  - Closed reason enums for `slot_missed` and `coordinator_shutdown`.
  - No socket addresses inside the type; opaque `PeerId(u64)`
    only.
- Replay test:
  `crates/ade_runtime/tests/producer_coordinator_replay.rs`:
  fixed (slot_tick + forge_result) sequence → same broadcast
  effects + same `ProducerLogEvent` sequence across two runs.

**Mechanical acceptance criteria:**
- `CoordinatorState`'s declared fields do not include any
  signing-key type (mechanically grep-asserted).
- Round-trip determinism: replay the same event stream twice;
  diff broadcast-effect bytes + log-event bytes; must be empty.
- Negative tests: `SlotDrift` (current_slot > target before
  RequestForge issued), `BroadcastFull` (queue at limit),
  `KesPeriodOutOfRange` (slot > anchor + slots_per_period * 64),
  `UnknownPeer`.

### S3 — RED `producer_shell` (key custody + forge effect handler)

**Strengthens:** `CN-PROD-02` (RED owns secret material), I7
(key custody RED-confined).

**Deliverables:**
- New `crates/ade_runtime/src/producer/producer_shell.rs` (RED):
  - Holds `KesSecret`, `VrfSigningKey`, `ColdSigningKey`, `OpCert`.
  - `pub fn shell_init(keys, opcert, ledger_snapshot_ref) -> ProducerShell`.
  - `pub fn handle_request_forge(shell, request) -> CoordinatorEvent` —
    runs `scheduler_step` synchronously, returns
    `ForgeSucceeded` or `ForgeFailed`.
  - Verifies `kes_period` matches `KesSecret.current_period()`;
    mismatch → `ForgeFailed(KesPeriodMismatch)`.
  - Calls `kes_update` if the slot crossed a KES-period
    boundary (RED side, since `KesSecret` lives here).
- Existing `KesSecret::from_seed_at_period` + the loaders
  (N-O / N-P) are reused; no changes to BLUE.

**Mechanical acceptance criteria:**
- `producer_shell.rs` contains the only references to
  `KesSecret`/`VrfSigningKey`/`ColdSigningKey` in N-Q's new
  surface (mechanically grep-asserted; passes
  `ci/ci_check_private_key_custody.sh`).
- Round-trip: shell processes a `RequestForge` →
  `ForgeSucceeded` carrying an `AcceptedBlock`; the
  `AcceptedBlock` bytes equal `forge_block(...)` for the same
  inputs (cross-check via reference test).
- Negative tests: `RequestForge` with stale `kes_period` →
  `ForgeFailed(KesPeriodMismatch)`; expired key → `ForgeFailed(KeyExpired)`.

### S4 — RED `n2n_listener` (tokio server-side wiring)

**Strengthens:** `CN-PROD-01` (handshake closure), I1 / I2 / I6.

**Deliverables:**
- New `crates/ade_runtime/src/network/n2n_listener.rs` (RED):
  - `pub async fn listener_init(addr) -> tokio::TcpListener`.
  - `pub async fn listener_accept_loop(listener, snapshot_handle, coordinator_tx, shutdown_rx)`.
  - Per-peer `pub async fn peer_pump(socket, peer_id, snapshot_handle, coordinator_tx)`:
    - N2N handshake (server-role) via existing `ade_network::handshake`.
    - Mux setup via existing `mux_pump`.
    - Per-peer `n2n_server::run` invocation (existing N-G; GREEN per N-Q's reclassification).
    - Translates inbound mini-protocol messages → `CoordinatorEvent::PeerRequest`.
    - Translates `CoordinatorEffect::SendToPeer` → outbound frames.
  - Mirrors the architecture of `crates/ade_runtime/src/network/n2n_dialer.rs`.

- Integration test (loopback):
  `crates/ade_runtime/tests/n2n_listener_loopback.rs`:
  - Start `n2n_listener` on `127.0.0.1:0`.
  - Use existing `n2n_dialer` from a tokio task as a synthetic peer.
  - Run a small fixed slot sequence; assert the dialer fetches
    the forged block via block-fetch.
  - Assert the fetched bytes equal `scheduler_step`'s output
    byte-for-byte.

**Mechanical acceptance criteria:**
- Listener bytes from a pre-handshake peer never reach
  `n2n_server::run` (mechanically asserted: handshake state
  machine completes before mux init).
- Loopback test: dialer-fetches-block test passes with one
  synthetic peer.
- Negative tests: wrong network magic → handshake fail-closed
  + connection closed; wrong version → same; mid-stream
  malformed envelope → fail-closed + connection closed (peer
  disconnect logged via `ProducerLogEvent`).

### S5 — `ade_node Mode::Produce` (CLI + main loop wiring)

**Strengthens:** all of N-Q's invariants (this is where
everything composes); closes the engineering gap from N-G's
"tokio socket layer — operator-action, not yet wired in-tree".

**Deliverables:**
- `crates/ade_node/src/cli.rs`:
  - New `Mode::Produce`.
  - New CLI flags: `--listen ADDR`, `--cold-skey PATH`,
    `--kes-skey PATH`, `--vrf-skey PATH`, `--opcert PATH`,
    `--genesis-file PATH` (Conway genesis JSON),
    `--evidence-log PATH`.
  - `Cli::extract_produce_cli` validator.
- New `crates/ade_node/src/produce_mode.rs` (RED):
  - `pub async fn run_produce_mode(cli, shutdown_rx) -> ExitCode`.
  - Load keys via existing `producer::keys::load_*`.
  - Parse genesis JSON for `slot_zero_time`, `slot_length`,
    `slots_per_kes_period`, `network_magic`.
  - Init `producer_shell` + `coordinator_init` + spawn
    `listener_accept_loop`.
  - Tokio `select!` over { slot tick, peer accept, peer event,
    forge result, shutdown }.
  - Each branch: `coordinator_step` → apply effects (forward
    `RequestForge` to shell; queue `BroadcastBlock` to served
    snapshot; send `SendToPeer` to per-peer task; append
    `LogEvidence` to JSONL writer).
- Existing `live_block_production_session` binary → either
  retire entirely OR rewrite as a thin wrapper that invokes
  `run_produce_mode` with N-C-procedure-compatible arguments.
  Recommendation: retire; the bounty path now uses
  `ade_node --mode produce`.

- End-to-end smoke test (in-process):
  `crates/ade_node/tests/produce_mode_smoke.rs`:
  - Generate Ade-native KES key (`ade_node --mode key_gen_kes`).
  - Spawn `ade_node --mode produce --listen 127.0.0.1:0 ...`.
  - Connect synthetic peer via `n2n_dialer`.
  - Drive 10 slots; observe at least one forged block fetched.

**Mechanical acceptance criteria:**
- `cargo run -p ade_node -- --mode produce ...` runs without
  panic on a minimal fixture.
- Smoke test passes (block fetched via dialer, bytes match
  forge output).
- Evidence JSONL is produced; every line parses as a
  `ProducerLogEvent`.
- `ci/ci_check_private_key_custody.sh` still green (key
  custody stays in `producer_shell`).

### S6 — Operator runbook + cluster close

**Strengthens:** closes the bounty-facing claim. `CN-CONS-06`
and `RO-LIVE-01` flip to `enforced` (or stay `partial` with
narrowed obligation if the operator pass isn't run).

**Deliverables:**
- `docs/active/cn-cons-06-operator-runbook.md` (new):
  - Conway-era private 2-node testnet setup (cardano-cli
    `conway genesis` flow; topology files; relay + BP nodes).
  - Pool registration: cold-key generation, opcert issuance,
    pool registration cert, delegation cert.
  - Running cardano-node (relay) + Ade (`ade_node --mode produce`).
  - Evidence-capture commands: cardano-node log lines showing
    Ade-forged block accepted; Ade JSONL evidence cross-check.
  - Marked **operational (Tier 5)** per OQ6; not a semantic
    invariant.

- `docs/clusters/PHASE4-N-Q/CE-N-Q-OPERATOR_PROCEDURE.md`:
  - The operator-action procedure (mirrors CE-N-C-8 / CE-N-G-8).
  - Single artifact:
    `docs/evidence/phase4-n-q-operator-pass-transcript.jsonl`
    OR
    `docs/clusters/PHASE4-N-Q/CE-N-Q-LIVE_<date>.log`.

- Helper scripts (operator convenience):
  - `scripts/n-q/setup_private_testnet.sh` — wraps the
    cardano-cli + ade_node steps for the bounty test.
  - `scripts/n-q/verify_block_accepted.sh` — greps cardano-node
    log for "block forged by Ade accepted at slot N".

- Cluster close:
  - Move `docs/clusters/PHASE4-N-Q/` → `docs/clusters/completed/PHASE4-N-Q/`.
  - `CLOSURE.md` with MAC verification matrix.
  - `.idd-config.json` `head_deltas_baseline` bumps to the
    closure commit.
  - Registry strengthenings applied:
    - `CN-CONS-06`, `RO-LIVE-01` — `strengthened_in += "PHASE4-N-Q"`;
      `open_obligation` clears if operator pass captured;
      otherwise narrowed to "blocked_until_operator_runbook_executed".
    - `CN-PROD-01/02`, `DC-PROD-01/02` — `status: "declared"` →
      `"enforced"`.
    - `DC-CONS-17`, `DC-CONS-18`, `CN-SESS-02`, `DC-PROTO-08` —
      `strengthened_in += "PHASE4-N-Q"`.
  - Grounding-doc regen (CODEMAP / SEAMS / HEAD_DELTAS /
    TRACEABILITY).
  - Per-cluster IDD review.

**Mechanical acceptance criteria:**
- All runbook commands reproducible by a reader with
  cardano-cli + docker access.
- Registry strengthenings applied.
- Evidence log committed (if operator pass executed).
- Cluster doc archived; grounding docs refreshed.

---

## Dependency graph

```
S1 (planning) ──► S2 (GREEN coordinator) ──► S3 (RED shell) ──► S5 (Mode::Produce)
                                       └──► S4 (RED listener) ──┘
                                                                ▼
                                                       S6 (runbook + close)
```

S2 and S4 can ship in parallel (coordinator is independent of
listener wiring). S3 depends on S2 (effect/event types). S5
composes S2 + S3 + S4. S6 depends on S5.

Recommended sequence: S1 → S2 → S3 → S4 → S5 → S6 for linear
PR reviewability.

## Out-of-scope (restated from invariants doc)

- Multi-peer concurrent load testing — single-peer bounty path
  is the deliverable.
- Multi-listener / multi-port — single listener.
- TLS / encrypted N2N — `¬P-8` continues to defer.
- Mlocked secret memory — future operational cluster.
- Producer-side N2C (LocalTxSubmission) — separate N-E
  successor work.
- Block-producer relay topology beyond 1 BP + 1 relay — future
  cluster (multi-relay topology).

## Hard prohibitions (restated)

- N9 — no secret-key material in `CoordinatorState`.
- N15 — no socket addresses in replayable event stream.
- N16 — no real-time replay claim.
- All N-O / N-P key-custody invariants carry forward.

## Closure protocol

Per `/cluster-close PHASE4-N-Q` on S6 close:
1. All 6 slices merged with MACs green.
2. Per-cluster IDD review against the full N-Q diff.
3. Operator-action evidence committed (or obligation narrowed).
4. Registry strengthenings applied.
5. Grounding-doc regenerators run.
6. Archive `docs/clusters/PHASE4-N-Q/` to completed.
7. `head_deltas_baseline` bumped to closure commit.
