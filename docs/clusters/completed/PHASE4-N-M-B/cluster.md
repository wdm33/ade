# Cluster PHASE4-N-M-B — Admission orchestrator + AgreementVerdict + admission JSONL

> **Status:** Planning artifact (non-normative). Sub-cluster B of
> the PHASE4-N-M family. Introduces `CN-ADMIT-01/02` +
> `DC-ADMIT-01..09` as enforced. Strengthens `T-DET-01`,
> `CN-CONS-08`, `CN-NODE-01`, `CN-WAL-01`, `CN-STORE-03`.
> Does NOT close `RO-LIVE-05` or `RO-LIVE-03` — those flip at
> sub-cluster C (operator pass against local docker preprod).

## Primary invariant

> The `ade_node` binary's admission mode composes the
> Ade-native storage architecture from N-M-A (seed importer +
> BootstrapAnchor + WAL) and the wire stack from N-L
> (handshake + mux + dialer) into a single tokio runner. Every
> live block from the peer is admitted via the
> CN-CONS-08 single admit authority, recorded as one
> `WalEntry::AdmitBlock` (CN-WAL-01), and compared to the peer's
> announced tip via the pure GREEN `verdict::derive` reducer,
> which emits a closed `AgreementVerdict` evidence sum
> (`Agreed | Lagging | Diverged | InputNotFound`). Diverged and
> InputNotFound halt the binary deterministically. The
> admission JSONL vocabulary is closed and physically isolated
> from wire-only mode's vocabulary.
>
> The runtime contract pinned by B's integration tests: same
> anchor + same imported seed + same recorded block sequence +
> DeterministicClock → byte-identical post-admit LedgerState
> fingerprints AND byte-identical AgreementVerdict per admit
> AND byte-identical JSONL output across two replays.

## Scope

- **GREEN (new):**
  - `ade_node::admission::verdict` — closed `AgreementVerdict`,
    closed `AdmitOutcome`, pure `derive` reducer.
  - `ade_node::admission::seed_to_snapshot` — adapter from
    `(UTxOState, ledger_fp, SeedPoint)` to
    `PersistentSnapshotCache::capture`.
  - `ade_node::admission_log::{event, writer}` — closed
    `AdmissionLogEvent` + hand-rolled JSON writer.
- **RED (new):**
  - `ade_node::admission::runner` — tokio admission-mode entry
    point.
  - `ade_node::main` — extended `--mode admission` dispatch.
  - `ade_node::cli` — extended (`--json-seed`, `--wal-dir`,
    `--genesis-hash`).
- **BLUE:** unchanged.

Out-of-scope (declared, with hard prohibitions):
- Reference-script TxOut decode in the seed importer
  (DC-ADMIT-09 — A1.1 work).
- Operator pass + RO-LIVE-05 close (sub-cluster C).
- Genesis → P replay (`RO-GENESIS-REPLAY-01` open).
- Mithril import (`RO-MITHRIL-IMPORT-01` open).
- utxohd-mem binary decode (Tier-5 non-goal).

## Grounding (verified at HEAD `c7e2a23`)

- N-M-A: `ade_runtime::seed_import::import_cardano_cli_json_utxo`
  (CN-SEED-01) + `ade_runtime::bootstrap_anchor::mint`
  (CN-ANCHOR-01) + `ade_ledger::wal::WalStore` trait (CN-WAL-01)
  + `ade_runtime::wal::FileWalStore`.
- N-L: `ade_runtime::network::n2n_dialer::N2nDialer` (CN-SESS-02
  / CN-SESS-03) + session reducer + mux pump.
- N-K: `ade_runtime::bootstrap::bootstrap_initial_state`
  (CN-NODE-01) + orchestrator core + `Clock` seam.
- N-J: `ade_runtime::rollback::PersistentSnapshotCache`
  (CN-STORE-08).
- BLUE: `ade_ledger::receive::admit_via_block_validity`
  (CN-CONS-08), `ade_ledger::block_validity::transition::block_validity`,
  `ade_ledger::fingerprint::fingerprint`.

## Slice index

| Slice | Scope | TCB |
|-------|-------|-----|
| B1 | GREEN `admission::verdict` — closed `AgreementVerdict` + `AdmitOutcome` + pure `derive` reducer. CI `ci_check_lagging_is_evidence_only.sh`. DC-ADMIT-01/06/08. | GREEN + CI |
| B2 | GREEN `admission_log::{event, writer}` — closed `AdmissionLogEvent` + writer. CI `ci_check_admission_log_vocabulary_closed.sh` (both directions). DC-ADMIT-04. | GREEN + CI |
| B3 | GREEN `admission::seed_to_snapshot` adapter. CI `ci_check_admission_no_refscript_skip.sh`. CN-ADMIT-02 + DC-ADMIT-09. | GREEN + CI |
| B4 | RED `admission::runner` — tokio entry + per-AdmittedBlock loop + WAL append + verdict emit + fatal-on-Diverged/InputNotFound. CN-ADMIT-01 + DC-ADMIT-02/03/05. | RED |
| B5 | RED `main.rs` `--mode admission` dispatch + CLI flags (`--json-seed`, `--wal-dir`, `--genesis-hash`). | RED |
| B6 | Hermetic loopback admission test + DC-ADMIT-07 admit-replay-equivalence test (true-tier; strengthens CN-STORE-03). | RED + test |
| B7 | Cluster close — flip 11 rules, 5 strengthenings, commit + push. | — |

## Exit criteria (CI-verifiable)

- [ ] **CE-N-M-B-1 (CN-ADMIT-01)** — `ci/ci_check_admission_runner_closure.sh`
  asserts single `pub fn run_admission` in `ade_node`.
- [ ] **CE-N-M-B-2 (CN-ADMIT-02)** — `ci/ci_check_admission_runner_closure.sh`
  also asserts single `pub fn seed_to_snapshot`.
- [ ] **CE-N-M-B-3 (DC-ADMIT-04)** — `ci/ci_check_admission_log_vocabulary_closed.sh`
  passes (both directions).
- [ ] **CE-N-M-B-4 (DC-ADMIT-08)** — `ci/ci_check_lagging_is_evidence_only.sh`
  passes.
- [ ] **CE-N-M-B-5 (DC-ADMIT-09)** — `ci/ci_check_admission_no_refscript_skip.sh`
  passes.
- [ ] **CE-N-M-B-6 (DC-ADMIT-01/02/03/05/06)** — admission unit
  tests pass (verdict reducer, log writer, runner integration
  smoke).
- [ ] **CE-N-M-B-7 (DC-ADMIT-07; true-tier)** —
  `admit_replay_equivalence_holds` integration test: replay
  from prior checkpoint + WAL → same post-admit
  `LedgerState` fingerprint AND same emitted
  `AgreementVerdict`.

> No human review may substitute for these checks.
> DC-ADMIT-07 is the headline true-tier mechanical proof.

## TCB color + tier classification

Per the sketch §6 classification table. Two key principles
load-bearing for this cluster:

1. **`AgreementVerdict` is GREEN evidence**, not authority.
   `verdict::derive` consumes outputs of authority paths
   (CN-CONS-08 admit, peer chain-sync tip); it does NOT decide
   validity, chain selection, or canonical state.
2. **Per-admit WAL append + admit-replay-equivalence is
   true-tier**, mechanically enforced by DC-ADMIT-07 integration
   test. Strengthens CN-STORE-03.

## Forbidden during this cluster

- No `Lagging` matched as part of a success-result pattern
  outside the verdict reducer + its tests.
- No partial reference-script support / permissive ref-script
  skipping / seed-import fallback in any admission code path.
- No mixing of admission and wire-only JSONL event-name
  literals (CI grep enforces both directions).
- No second `pub fn run_admission` / `seed_to_snapshot` /
  `verdict::derive`.
- No bypass of `bootstrap_initial_state` (CN-NODE-01 stays
  the single bootstrap authority).
- No `HashMap` / `HashSet` / wall-clock / rand / float in
  GREEN admission files.
- No `tokio` import in GREEN admission files
  (`verdict::derive`, `admission_log::*`, `seed_to_snapshot`
  are sync + pure).

## Replay obligations introduced

- DC-ADMIT-07 — admit-replay-equivalence (true-tier).
  Strengthens CN-STORE-03.
- `T-DET-01.strengthened_in += "PHASE4-N-M-B"` — verdict
  reducer + admission JSONL determinism.
- `CN-CONS-08.strengthened_in += "PHASE4-N-M-B"` — admit path
  now driven by real-peer chain-sync via admission mode
  (mechanical half).
- `CN-NODE-01.strengthened_in += "PHASE4-N-M-B"` — admission
  routes through `bootstrap_initial_state`; single bootstrap
  authority preserved.
- `CN-WAL-01.strengthened_in += "PHASE4-N-M-B"` — every admit
  append goes through `WalStore::append`.
- `CN-STORE-03.strengthened_in += "PHASE4-N-M-B"` —
  replay-equivalent recovery extended to per-admit
  verdict re-derivation.

## Open obligations carried after closure

- `RO-LIVE-05`, `RO-LIVE-03` — still open until C.
- `RO-GENESIS-REPLAY-01` — open obligation.
- `RO-MITHRIL-IMPORT-01` — open.
- **A1.1 reference-script TxOut decode** — hard prereq for
  sub-cluster C operator pass. Tracked outside this cluster
  per DC-ADMIT-09.

## Authority reminder

Correctness rules live in `docs/ade-invariant-registry.toml`.
If guidance here conflicts with the registry:

> **Registry + CI enforcement win.**
