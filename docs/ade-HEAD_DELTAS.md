# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `1946573` (PHASE4-N-K invariants sketch + DC-NODE/CN-NODE registry seed, 2026-05-26 10:41 +0700)
> HEAD: working tree on top of `1946573` (PHASE4-N-K cluster-close staged, 2026-05-26)
> 1 cluster-close commit (pending), 40 files changed (33 new + 7 modified), +6,268 / −32 lines

> **Baseline shift note.** This regen narrows the baseline from the
> prior `e509886` (PHASE4-N-I close) used by the N-J narrative to
> `1946573` (PHASE4-N-K handoff — invariants sketch landed,
> registry seeded with `CN-NODE-01` + `DC-NODE-01..04` as `declared`).
> HEAD_DELTAS now narrates **only** the PHASE4-N-K cluster:
> orchestrator + `ade_node` binary closure over 8 slices (S1 → S8).
> The prior cluster-by-cluster narratives (Phase 4 N-A through N-J)
> are preserved in the archived cluster docs under
> `docs/clusters/completed/` and in the SEAMS / CODEMAP / TRACEABILITY
> companions. `.idd-config.json` `head_deltas_baseline` was bumped
> from `d509f02` to `1946573` as part of this regen.

> **Cluster summary.** PHASE4-N-K ships the production orchestrator
> (GREEN reducer + RED tokio runners) plus the `ade_node` lib+bin
> over 8 slices, closing all 5 PHASE4-N-K registry rules
> (`CN-NODE-01`, `DC-NODE-01..04`) from `declared` → `enforced` and
> strengthening 7 carried-forward rules (`T-DET-01`, `CN-CONS-08`,
> `CN-STORE-07`, `CN-STORE-08`, `DC-CONS-21`, `DC-STORE-08`,
> `DC-STORE-09`). 6 new CI scripts gate the cluster
> (`ci_check_*` script count moves from 56 → 62). One open
> obligation is **carried** (`DC-STORE-09`
> `snapshot_schema_migration_follow_on_cluster`) and three are
> **unchanged from prior clusters**
> (`RO-LIVE-01`, `RO-LIVE-02`, `CN-CONS-06` — all
> `blocked_until_operator_peer_available`). The cluster is purely
> additive to the existing module graph; no removals, no API
> breakage.

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 1946573..HEAD` (HEAD is
the staged cluster-close commit — slice-by-slice commits were
collapsed into a single cluster-close commit per the project's
cluster-close discipline).

| Hash | Type | Summary |
|------|------|---------|
| (pending) | feat | feat(orchestrator+node): PHASE4-N-K close — orchestrator core + RED runners + `ade_node` lib+bin (S1..S8), flip CN-NODE-01 + DC-NODE-01..04 to enforced |

All cluster work (S1 chain-db wiring + bootstrap cold/warm, S2
orchestrator core reducer, S3 persistent writer, S4 per-peer
isolation + dispatch wrappers, S5 leadership session + clock seam,
S6 N2N server pump, S7 `ade_node` lib+bin + shutdown drain, S8
replay-equivalence proof harness) is contained in a single
cluster-close commit; per-slice context lives under
`docs/clusters/completed/PHASE4-N-K/N-K-S{1..8}.md`. No fix / docs
/ chore / refactor commits in this window — the cluster is a single
linear feature stream.

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_runtime::bootstrap` | **GREEN** | SOLE `pub fn bootstrap_initial_state` returning `(LedgerState, PraosChainDepState, Option<ChainTip>)`; cold-start (genesis) vs warm-start (persistent snapshot via `materialize_rolled_back_state`) is a single in-function branch. Pure projection over operator-supplied chain_db + snapshot_store; no async, no wall-clock, no rand. (CN-NODE-01.) | `bootstrap.rs` (`bootstrap_initial_state`, `BootstrapError`, 5 unit tests) | PHASE4-N-K / S1 |
| `ade_runtime::clock` | **GREEN** (with RED `SystemClock` sub-classification) | `Clock` trait + two impls: `DeterministicClock` (pure, drives replay harness) and `SystemClock` (RED — SOLE wall-clock reader in `ade_runtime`). DC-NODE-03 seam: orchestrator core consumes `Clock`; never reads `SystemTime::now` / `Instant::now` directly. | `clock.rs` (`Clock`, `DeterministicClock`, `SystemClock`, `SlotTimestamp`) | PHASE4-N-K / S1 + S5 |
| `ade_runtime::orchestrator` | **GREEN** (core) + **RED** (runner sub-modules) | Authoritative dispatch reducer + per-peer state container + closed event/effect/error sums for the production orchestrator. Core (`core`, `event`, `state`, `mod`) is pure `step(state, event, clock) -> (new_state, Vec<effect>)` with NO tokio import (enforced). RED sub-modules host the tokio bridges: per-peer task, leadership-slot pump, N2N listening-socket spawner. | `mod.rs` (barrel + re-exports), `event.rs` (`OrchestratorEvent`, `OrchestratorEffect`, `OrchestratorError`, `PeerHaltReason`, `AuthorityFatalKind`, `PeerId`, `PeerRole` — all closed sums), `state.rs` (`OrchestratorState`, `PerPeerReceiveVersions`), `core.rs` (pure `step` reducer + dispatch_*_inbound wrappers), `peer_session.rs` **RED** (per-peer tokio task), `leadership_session.rs` **RED** (slot-tick pump driven by `Clock`), `n2n_server_pump.rs` **RED** (listening-socket per-connection spawner) | PHASE4-N-K / S2 + S4 + S5 + S6 |
| `ade_runtime::rollback::persistent_writer` | **GREEN** | `PersistentSnapshotWriter`: snapshot-after-admission cadence driver that delegates **every** capture decision to `should_snapshot_after_block` (the SOLE cadence policy — DC-NODE-02). `on_admitted` invokes the policy; `force_capture` skips cadence for shutdown drain but still routes through `framing::encode_snapshot`. Pure projection — no async, no clock, no rand. | `persistent_writer.rs` (`PersistentSnapshotWriter`, 4 unit tests) | PHASE4-N-K / S3 |
| `ade_node::cli` | **GREEN** | Closed CLI parser for the binary: `--genesis-path`, `--network`, `--chain-db-path`, `--snapshot-store-path`, `--listen-addr`, `--peer-addr`. Closed `CliError` sum. No external clap dependency — hand-rolled for surface closure. | `cli.rs` (`Cli`, `CliError`, `parse_from`) | PHASE4-N-K / S7 |
| `ade_node` (lib half) | **GREEN** | New `lib.rs` exposing `run_node_until_shutdown`, `Cli`, `CliError`, plus the deterministic exit-code constants (`EXIT_AUTHORITY_FATAL_IO=10`, `EXIT_AUTHORITY_FATAL_DECODE=12`, `EXIT_GENERIC_STARTUP=1`). The crate became lib+bin; the binary is now a thin wrapper over the library. | `lib.rs` (re-exports), `node.rs` (`run_node_until_shutdown`, `NodeRunError`, authority-fatal exit mapping, shutdown drain via `PersistentSnapshotWriter::force_capture`) | PHASE4-N-K / S7 |

No new workspace crates. Workspace member count unchanged.
`ade_node` transitioned from bin-only to lib+bin via the new
`[[bin]] name = "ade_node" path = "src/main.rs"` stanza in
`crates/ade_node/Cargo.toml`.

Cross-reference: the new modules must be reflected in CODEMAP §GREEN
(`ade_runtime::{bootstrap, clock, orchestrator::{core, event,
state, mod}}`, `ade_runtime::rollback::persistent_writer`,
`ade_node::{cli, lib, node}`) and §RED
(`ade_runtime::orchestrator::{peer_session, leadership_session,
n2n_server_pump}`, plus the `ade_node` `main.rs` shell). If absent
at the next read, CODEMAP is stale — regenerate via `/codemap`.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_runtime` (lib.rs + Cargo.toml + rollback/mod.rs) | +13 lines | `lib.rs`: `pub mod bootstrap; pub mod clock; pub mod orchestrator;` declarations wiring the new §2 GREEN/RED sub-trees. `Cargo.toml`: explicit `tokio = { version = "1", features = ["net", "rt", "rt-multi-thread", "io-util", "macros", "time", "sync", "signal"] }` dep added, **confined by CI to the three RED runner files** — the GREEN orchestrator core MUST NOT import `tokio::*` (enforced by `ci_check_orchestrator_core_purity.sh` + `ci_check_clock_seam.sh`). `rollback/mod.rs`: `pub mod persistent_writer;` + `pub use persistent_writer::PersistentSnapshotWriter;`. |
| `ade_node` (Cargo.toml + main.rs) | +70 / −2 lines | `Cargo.toml`: new `[[bin]]` stanza; new `[dependencies]` block adding `ade_types`, `ade_core`, `ade_ledger`, `ade_runtime`, `ade_network`, `ade_codec`, and explicit `tokio` (same feature set as `ade_runtime`); new `[dev-dependencies]` adding `ade_testkit` + `tempfile` for the shutdown-resume integration corpus. `main.rs`: rewritten as thin wrapper over `ade_node::lib` — parses CLI, prints honest-scope readiness line, maps `CliError` to `EXIT_GENERIC_STARTUP`. The full bootstrap+orchestrator drive lives in the new `node.rs`. |
| `docs/ade-invariant-registry.toml` | −30 / +72 lines | 5 PHASE4-N-K rules flipped `declared` → `enforced` (`CN-NODE-01`, `DC-NODE-01`, `DC-NODE-02`, `DC-NODE-03`, `DC-NODE-04`); `code_locus` + `tests` + `ci_script` + `evidence_notes` populated for each; `strengthened_in = ["PHASE4-N-K"]` added on the flip. 7 carried-forward rules gained `strengthened_in += "PHASE4-N-K"` plus an extended evidence/notes paragraph (`T-DET-01`, `CN-CONS-08`, `CN-STORE-07`, `CN-STORE-08`, `DC-CONS-21`, `DC-STORE-08`, `DC-STORE-09`). `DC-STORE-09` additionally gained `open_obligation = "snapshot_schema_migration_follow_on_cluster"` — captures the missing v1→v2 operator-facing migration tooling, deliberately **not** carried by `DC-NODE-04` (the node binary handles unknown-version + fingerprint-mismatch at bootstrap via the authority-fatal-decode exit path; migration tooling is a separate operator concern). |

No other source modules were touched. The cluster is **purely
additive** to the existing module graph — new GREEN sub-trees +
new RED runner files + new CI scripts + registry edits. No
refactors, no API breakage, no removals from any existing module.

---

## 4. Feature Flags

No Cargo `[features]` table is declared in `ade_runtime`,
`ade_node`, or any other workspace crate at baseline or at HEAD.
No new feature flags introduced; no existing feature flags
modified or removed.

The cluster adds three new closed constants that are **not** Cargo
features but are referenced here for completeness — they gate
deterministic behavior at the binary surface:

| Constant | Module | Purpose | Status |
|----------|--------|---------|--------|
| `EXIT_AUTHORITY_FATAL_IO: i32 = 10` | `ade_node::node` | Closed exit code returned when chain_db / snapshot_store I/O fails in a way that signals authority-fatal storage divergence. (DC-NODE-04.) | **New** since baseline |
| `EXIT_AUTHORITY_FATAL_DECODE: i32 = 12` | `ade_node::node` | Closed exit code returned when snapshot decode hits `UnknownVersion` or `FingerprintMismatch` at bootstrap, or when block decode in receive hits a closed structural-decode failure. Halts deterministically — no silent retry. (DC-NODE-04 fail-fast + DC-STORE-09 fail-closed.) | **New** since baseline |
| `EXIT_GENERIC_STARTUP: i32 = 1` | `ade_node::node` | Closed exit code returned for CLI-parse failures and other pre-bootstrap startup errors. | **New** since baseline |

No coupling between the three: each variant of `NodeRunError` (or
`CliError`) maps to exactly one exit code via a `match` in `main.rs`
+ `run_node_until_shutdown`. The mapping is total and verified by
the `binary_halts_on_authority_fatal_decode_error` +
`cold_start_without_genesis_fails_with_generic_startup_code`
integration tests.

The new `tokio` dependency on `ade_runtime` and `ade_node` is **not
gated** by a Cargo feature — it is structurally confined by CI:
`ci/ci_check_orchestrator_core_purity.sh` greps the orchestrator
core files (`core.rs`, `event.rs`, `state.rs`, `mod.rs`) for any
`tokio::` import and fails the build if one appears.
`ci/ci_check_clock_seam.sh` greps `ade_runtime` for
`SystemTime::now` / `Instant::now` outside `clock.rs` and fails
otherwise. The structural confinement is mechanical (CI-enforced),
not Cargo-feature-enforced.

---

## 5. CI Checks

### PHASE4-N-K orchestrator + binary closure — 6 new scripts (`ci_check_*.sh` 57th – 62nd)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_bootstrap_closure.sh` | **New** (S1) — script 57 | Enforces `CN-NODE-01`: `ade_runtime::bootstrap` is the SOLE `pub fn` returning `(LedgerState, PraosChainDepState, Option<ChainTip>)` across the workspace; positive grep that the function body calls `materialize_rolled_back_state` (warm-start authority chain) and references the cold-start branch; negative grep that no parallel bootstrap function exists in `ade_runtime` or `ade_node`. |
| `ci/ci_check_clock_seam.sh` | **New** (S1 + S5) — script 58 | Enforces `DC-NODE-03`: `crates/ade_runtime/src/clock.rs` is the SOLE site of `SystemTime::now` / `Instant::now` in `ade_runtime`; orchestrator core files (`core.rs`, `event.rs`, `state.rs`, `mod.rs`) contain none of `tokio::time`, `rand`, `HashMap`/`HashSet`, or `f32`/`f64`; positive grep that `Clock` trait is the orchestrator core's wall-clock surface. |
| `ci/ci_check_orchestrator_core_purity.sh` | **New** (S2) — script 59 | Enforces `DC-NODE-03` (general purity half): orchestrator core files (`core.rs`, `event.rs`, `state.rs`, `mod.rs`) must NOT import `tokio::*`, must NOT use `std::collections::Hash*`, must NOT introduce `f32`/`f64`, must NOT call `SystemTime::now`/`Instant::now`. Separates the GREEN core from the RED runner sub-modules at grep level. |
| `ci/ci_check_persistent_writer_no_parallel_cadence.sh` | **New** (S3) — script 60 | Enforces `DC-NODE-02`: `PersistentSnapshotWriter::on_admitted` consults `should_snapshot_after_block` (the SOLE cadence policy) — negative grep for parallel cadence-modulo definitions (`% N == 0`) outside `cadence.rs`; positive grep that orchestrator core also routes through `should_snapshot_after_block`. |
| `ci/ci_check_peer_session_isolation.sh` | **New** (S4) — script 61 | Enforces `DC-NODE-01`: per-peer state lives in `OrchestratorState::per_peer_{receive,server}` `BTreeMap`s keyed by `PeerId`; closed `PeerHaltReason` discriminant for halt emission; no shared mutable state across `peer_session` tokio tasks; decode/validity errors emit `PeerSessionHalted` and remove only that peer's entry. |
| `ci/ci_check_node_binary_uses_single_bootstrap.sh` | **New** (S7) — script 62 | Enforces `CN-NODE-01` (binary side) + `DC-NODE-04`: `ade_node` calls `bootstrap_initial_state` exactly once; `run_node_until_shutdown` is the SOLE driver fn; positive grep that the exit-code constants are referenced from the error mapping site. |

Total CI script count: **56 → 62** (`ci/ci_check_*.sh`). 6 new
scripts; no removals; no modifications to existing scripts in the
`1946573..HEAD` window — the cluster strictly appends.

TRACEABILITY cross-reference: each of the 6 new scripts appears as
a `ci_script` on at least one rule in
`docs/ade-invariant-registry.toml` (12 new `ci_script ↔ rule`
edges across the 5 PHASE4-N-K rules + 7 strengthened rules).
Re-traced via `ci/ci_check_constitution_coverage.sh` — expected to
pass at HEAD.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null.
Canonical-type rules live inline in the invariant registry under
family `T`.

**PHASE4-N-K introduced new closed sum types** in support of the
orchestrator (GREEN + RED combined) and the binary:

- `ade_runtime::orchestrator::event::OrchestratorEvent` — closed
  inbound event sum (`PeerInbound`, `ServerInbound`, `SlotTick`,
  `Shutdown`, ...).
- `ade_runtime::orchestrator::event::OrchestratorEffect` — closed
  outbound effect sum (`CaptureSnapshot`, `EmitToReceiveAdapter`,
  `EmitToProducer`, `PeerSessionHalted`, ...).
- `ade_runtime::orchestrator::event::OrchestratorError` — closed
  reducer-failure sum.
- `ade_runtime::orchestrator::event::PeerHaltReason` — closed
  per-peer halt-reason discriminant (DC-NODE-01 evidence).
- `ade_runtime::orchestrator::event::AuthorityFatalKind` — closed
  authority-fatal-error categorization driving the
  `EXIT_AUTHORITY_FATAL_{IO,DECODE}` mapping (DC-NODE-04).
- `ade_runtime::orchestrator::event::PeerId`,
  `ade_runtime::orchestrator::event::PeerRole` — closed peer
  identity + role types (BTreeMap key).
- `ade_runtime::bootstrap::BootstrapError` — closed cold/warm-start
  failure sum.
- `ade_node::cli::CliError` — closed CLI-parse failure sum.
- `ade_node::node::NodeRunError` — closed runtime-failure sum.

Plus the canonical authority sites that are now SOLE-authority
(CN-NODE-01 / DC-NODE-02 / DC-NODE-03 / DC-NODE-04):

- `bootstrap_initial_state` (bootstrap.rs) — SOLE bootstrap fn.
- `PersistentSnapshotWriter::on_admitted` + `force_capture`
  (persistent_writer.rs) — SOLE persistent-snapshot cadence driver.
- `Clock` trait + `SystemClock` (clock.rs) — SOLE wall-clock site
  in `ade_runtime`.
- `run_node_until_shutdown` (node.rs) — SOLE node driver fn.

**Removals: 0** (expected under append-only discipline).

Exact whole-project type recount belongs to the TRACEABILITY regen
that follows this HEAD_DELTAS.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML),
not prose normative-doc rules; this section reports on it.

- Rules at baseline (`1946573:docs/ade-invariant-registry.toml`): **214**
- Rules at HEAD (`HEAD:docs/ade-invariant-registry.toml`): **214**
- Net additions: **0** (the 5 PHASE4-N-K rules were already seeded
  at baseline `1946573` as `declared` placeholders per the cluster
  handoff; PHASE4-N-K populates and flips them in place rather
  than inserting new entries).
- Removals: **0** (expected under append-only discipline; clean).

- **Status flips (5):**
  - **`CN-NODE-01` `declared` → `enforced`** — single
    `pub fn bootstrap_initial_state` in `ade_runtime::bootstrap`;
    cold-start + warm-start are two branches of one function.
    `ci/ci_check_bootstrap_closure.sh`.
  - **`DC-NODE-01` `declared` → `enforced`** — per-peer state in
    `OrchestratorState::per_peer_{receive,server}` BTreeMaps;
    decode/validity errors emit `PeerSessionHalted` (closed reason
    discriminant) and remove only that peer; sibling peers + producer
    continue. `ci/ci_check_peer_session_isolation.sh`.
  - **`DC-NODE-02` `declared` → `enforced`** —
    `PersistentSnapshotWriter::on_admitted` consults
    `should_snapshot_after_block` exclusively; orchestrator core
    routes through the same policy on `Admitted`.
    `ci/ci_check_persistent_writer_no_parallel_cadence.sh`. No
    `open_obligation` (eviction is a storage concern, not cadence
    fidelity).
  - **`DC-NODE-03` `declared` → `enforced`** — `Clock` trait in
    `ade_runtime::clock`; `SystemClock` (RED sub-classified) is the
    SOLE wall-clock-reading site; `DeterministicClock` drives the
    replay harness. `ci/ci_check_clock_seam.sh`.
  - **`DC-NODE-04` `declared` → `enforced`** —
    `ade_node::node::run_node_until_shutdown` maps authority-fatal
    kinds to `EXIT_AUTHORITY_FATAL_IO=10`,
    `EXIT_AUTHORITY_FATAL_DECODE=12`, `EXIT_GENERIC_STARTUP=1`;
    shutdown drain force-captures a final snapshot via
    `PersistentSnapshotWriter::force_capture`.
    `ci/ci_check_node_binary_uses_single_bootstrap.sh`. No
    `open_obligation` (schema-migration tooling is `DC-STORE-09`'s
    home — see below).

- **Strengthenings recorded by PHASE4-N-K (7):**
  - **`T-DET-01.strengthened_in += "PHASE4-N-K"`** — replay
    equivalence now extends across the orchestrator core under
    clock injection (`orchestrator_replay_equivalence.rs`).
  - **`CN-CONS-08.strengthened_in += "PHASE4-N-K"`** — admit path
    driven end-to-end by the production orchestrator
    (`orchestrator::core::step` via GREEN `dispatch_*_inbound`
    wrappers). The orchestrator never reconstructs `AdmittedBlock`
    and never bypasses `receive_apply`; per-peer dispatch errors
    halt only that peer (DC-NODE-01 dovetail).
  - **`CN-STORE-07.strengthened_in += "PHASE4-N-K"`** —
    `materialize_rolled_back_state`'s caller is now the production
    bootstrap warm-start branch;
    `bootstrap_warm_start_equals_direct_materialize` proves
    bootstrap-warm-start = direct-materialize equivalence.
  - **`CN-STORE-08.strengthened_in += "PHASE4-N-K"`** —
    `encode_snapshot` / `decode_snapshot` now driven end-to-end by
    the production orchestrator (bootstrap, `PersistentSnapshotWriter`,
    shutdown drain). All callers route through the single framing
    module — no node-binary-side reimplementation.
  - **`DC-CONS-21.strengthened_in += "PHASE4-N-K"`** — round-trip
    equivalence exercised end-to-end at bootstrap warm-start
    (`bootstrap_warm_start_materializes_from_persistent_snapshot`,
    `bootstrap_warm_start_equals_direct_materialize`) and at
    shutdown-resume (`shutdown_then_resume_produces_byte_identical_state`).
  - **`DC-STORE-08.strengthened_in += "PHASE4-N-K"`** — encoder
    canonicality exercised by `PersistentSnapshotWriter::on_admitted`
    / `force_capture` and by the shutdown drain; end-to-end
    determinism asserted in `shutdown_then_resume_produces_byte_identical_state`.
  - **`DC-STORE-09.strengthened_in += "PHASE4-N-K"`** — the
    authority-fatal-decode exit path (DC-NODE-04) handles
    `UnknownVersion` + `FingerprintMismatch` at bootstrap by
    halting deterministically (`binary_halts_on_authority_fatal_decode_error`).

- **Open obligations status at HEAD:**
  - **`DC-STORE-09.open_obligation = "snapshot_schema_migration_follow_on_cluster"`**
    — **NEW** since baseline. Captures the missing operator-facing
    v1→v2 snapshot-schema migration tooling. The current rule
    already pins the fail-closed posture on unknown versions; the
    open obligation is the migration-tool home, deliberately
    **not** carried by `DC-NODE-04` (the node binary handles
    unknown-version at bootstrap via the deterministic exit code).
  - **`RO-LIVE-02.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-H. Unchanged.
  - **`RO-LIVE-01.open_obligation = "blocked_until_operator_peer_available"`**
    — carried forward from PHASE4-N-G. Unchanged.
  - **`CN-CONS-06.open_obligation = "blocked_until_operator_stake_available"`**
    — carried forward from PHASE4-N-C. Unchanged.
  - **`OP-OPS-04.open_obligation`** (Sum6KES skey loader) — carried
    forward; unchanged.
  - **`DC-CONS-21.open_obligation`** REMAINS removed (closed at N-J
    S8); PHASE4-N-K only strengthens, does not reopen.

---

## Anomalies and Cross-Reference Warnings

- **No canonical-type or invariant-rule removals.** Append-only
  discipline preserved across the cluster.
- **No conventional-commits violations.** The pending cluster-close
  commit follows the `feat(orchestrator+node): PHASE4-N-K close —
  ...` scope+suffix pattern.
- **CODEMAP cross-reference**: the six new modules (§2) must
  appear in CODEMAP. If absent at the next read, CODEMAP is stale
  — regen via `/codemap`. Specifically: GREEN entries for
  `ade_runtime::{bootstrap, clock, orchestrator::{core, event,
  state, mod}}`, `ade_runtime::rollback::persistent_writer`,
  `ade_node::{cli, lib, node}`; RED entries for
  `ade_runtime::orchestrator::{peer_session, leadership_session,
  n2n_server_pump}` and the `ade_node` `main.rs` binary shell.
- **SEAMS cross-reference**: the new orchestrator-event surface
  (`OrchestratorEvent` / `OrchestratorEffect` / `OrchestratorError`
  / `PeerHaltReason` / `AuthorityFatalKind`) is a closed sum
  attachment surface; SEAMS should classify it under closed
  registries. Regen via `/seams` if absent.
- **TRACEABILITY cross-reference**: the 6 new CI scripts (§5) and
  the 5 status flips + 7 strengthenings (§7) must appear in
  TRACEABILITY. If absent at the next read, regen via
  `/traceability`.
- **Honest-scope note (RED runner)**: the three RED tokio runner
  files (`peer_session.rs`, `leadership_session.rs`,
  `n2n_server_pump.rs`) and the `ade_node` `main.rs` shell follow
  the project's established `live_block_follow_session` honest-stub
  pattern. The orchestrator core, bootstrap, and persistent writer
  are real and mechanically evidenced; the actual Ouroboros mux +
  handshake driver above `ade_network::mux::MuxTransport` is
  operator-action work tracked by `RO-LIVE-01` / `RO-LIVE-02`. The
  binary today, when launched, performs bootstrap and prints a
  readiness line; the chain-sync / block-fetch socket pump is the
  follow-on cluster's deliverable. Mechanical evidence for
  `DC-NODE-01` is provided by orchestrator-core dispatch tests +
  integration tests proving per-peer isolation against the pure
  reducer; mechanical evidence for `DC-NODE-04` is provided by the
  in-process `shutdown_resume_identity.rs` integration test which
  exercises the full bootstrap → orchestrator → drain → bootstrap
  cycle on a Conway 576 corpus block.

---

## Generation Notes

This regen was produced by `/head-deltas 1946573` against the
staged PHASE4-N-K cluster-close working tree. The baseline was
shifted from the prior N-J baseline (`e509886`) to the PHASE4-N-K
handoff (`1946573`) per the cluster-close cadence — each grounding
regen baselines at the previous cluster's handoff/close so the
narrative stays narrow and reviewable per-cluster.
`.idd-config.json` `head_deltas_baseline` was bumped from `d509f02`
to `1946573` as part of this regen. Future regens should continue
to baseline at the **previous** cluster's close, not the original
Phase 3 handoff, so the document remains per-cluster.

Mechanical inputs:
- `git log --oneline --no-merges 1946573..HEAD` (cluster-close
  commit pending) → §1.
- `git diff --name-status 1946573 -- crates/ ci/ docs/` → §2 + §3
  (working tree included via `git add -N` for accurate stats).
- `git diff --stat 1946573 -- crates/<crate>/` → §3 scope column.
- `crates/ade_runtime/Cargo.toml` + `crates/ade_node/Cargo.toml`
  diff → §4 (no Cargo features changed; tokio dep + structural
  CI confinement noted).
- `ls ci/` vs `git ls-tree -r --name-only 1946573 ci/` → §5
  (56 → 62).
- `git diff 1946573 -- docs/ade-invariant-registry.toml` + entry
  count (`grep -c '^\[\[rules\]\]'`) → §7 (214 → 214; 5 flips,
  7 strengthenings, 1 new open_obligation).
- `docs/clusters/completed/PHASE4-N-K/CLOSURE.md` →
  cluster-summary header + Modules Modified narrative.
