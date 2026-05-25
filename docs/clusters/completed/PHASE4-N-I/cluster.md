# Cluster PHASE4-N-I — In-memory snapshot + replay-forward rollback (closes DC-CONS-20 rollback-side)

> **Status:** Planning artifact (non-normative). Strengthens
> `T-DET-01`, `DC-PROTO-09`, `CN-CONS-08`. Closes `DC-CONS-20`
> rollback-side (`open_obligation` removed on final-slice merge).
> Introduces `CN-STORE-07`, `DC-CONS-22`, `DC-STORE-07` as enforced;
> leaves `DC-CONS-21` declared with a new `open_obligation` naming
> a persistent-encoder follow-on cluster. Produced from
> `docs/planning/ledger-snapshot-rollback-invariants.md` and
> `docs/planning/phase4-n-i-cluster-slice-plan.md`. If this doc
> conflicts with the registry / specs, those win.

---

## Primary invariant

> When the receive bridge receives `RollBackward(target_point)`,
> materialize the rolled-back `(LedgerState, PraosChainDepState)`
> via the single canonical driver `materialize_rolled_back_state`
> (snapshot + replay-forward fold over
> `apply_block_with_verdicts`), then commit it atomically with
> `ChainDb::rollback_to_slot` + ledger replacement + chain_dep
> replacement + pending-header reset. Snapshot materialization is
> a pure cache over canonical history, not a second ledger
> evolution path.

## Normative anchors

- `docs/ade-invariant-registry.toml` — `T-DET-01` (strengthened
  in N-I), `DC-PROTO-09` (strengthened in N-I), `CN-CONS-08`
  (strengthened in N-I), `DC-CONS-20` (closed in N-I), the 4 new
  entries `CN-STORE-07`, `DC-CONS-22`, `DC-STORE-07`, `DC-CONS-21`
  (latter stays declared with open_obligation).
- Project constitution §2 (`T-DET-01`, Functional Core /
  Imperative Shell).
- IDD `~/.claude/methodology/idd.md` Part I §§4, §5, §6, §9.
- `docs/planning/ledger-snapshot-rollback-invariants.md` §§1–8.

## OQ resolutions (locked — see invariants sketch §7)

- **OQ-1 (BLOCKING pre-S1 — epoch boundary handling)** —
  **RESOLVED 2026-05-26.** Survey confirms
  `apply_block_with_verdicts` (rules.rs:244-250) calls
  `detect_epoch_transition` + `apply_epoch_boundary_full` before
  block application. Replay-forward is therefore a simple fold;
  no separate epoch-boundary invocation needed in the materialize
  driver.
- **OQ-2 (Cadence default)** — N=100 blocks; BLUE-structural in
  `SnapshotCadence { every_n_blocks: u32 }`. Operator-tunable
  cadence explicitly out of scope.
- **OQ-3 (Persistent encoding scope)** — RESOLVED via Path A
  recursive scoping (see "Scope decisions" below): N-I ships
  in-memory only; persistent encoding is a follow-on cluster.
  `DC-CONS-21` carries new `open_obligation`.
- **OQ-4 (Pre-Conway era support)** — Out of scope; pre-Conway
  blocks encountered during replay return
  `MaterializeError::EraNotSupported { era }` structurally.
- **OQ-5 (Snapshot eviction)** — Out of scope for this cluster;
  in-memory cache grows monotonically until process restart.
  Eviction is a follow-on operational concern.
- **OQ-6 (LedgerView determinism)** — Survey: the `LedgerView`
  trait is satisfied by `ade_ledger::consensus_view::PoolDistrView`
  which is constructed deterministically from canonical inputs.
  No peer queries / no non-deterministic data sources.
- **OQ-7 (RollBackward sequencing)** — Trivial: each rollback is
  a fresh transition over the reducer's state. No special
  handling. Confirmed in S6 integration test.

## Scope decisions (Path A recursive — in-memory only)

The full canonical `LedgerState` encoder is ~1500-2000 LoC of
field-walk code mirroring `ade_ledger::fingerprint`. Shipping it
in one slice without risking partial-implementation bugs is too
big. This cluster therefore narrows to **in-memory snapshot
storage only**:

- `SnapshotReader` is a BLUE trait with one production impl:
  `InMemorySnapshotCache` (a `BTreeMap<SlotNo, (LedgerState,
  PraosChainDepState)>`).
- Rollback works fully **within a session**. ChainDb rollback +
  ledger replacement + chain_dep replacement + pending-header
  reset + ChainSelectorState rollback all happen atomically.
- Across restarts, snapshots are lost. The follow-on persistent-
  encoder cluster closes that gap.

CEs enforced this cluster: CN-STORE-07, DC-CONS-22, DC-STORE-07,
DC-CONS-20 (rollback-side).
CEs carried forward (open_obligation): DC-CONS-21.

This is the same Path A discipline N-H used for rollback-side
itself, applied one level deeper.

## Grounding (verified at HEAD `f143984`)

- **Forward apply authority (B1+ + N-C):**
  - `ade_ledger::rules::apply_block_with_verdicts` (line 197) —
    composed by the materialize driver. Internally handles epoch
    boundaries.
  - `ade_ledger::block_validity::block_validity` — single
    admission authority (CN-CONS-08).
- **ChainDb (N-D, Tier 1):**
  - `ChainDb::rollback_to_slot(slot)` — block-store rollback. S3
    commit_rollback's first step.
  - `ChainDb::iter_from_slot(from)` — block iterator for
    replay-forward.
- **Praos consensus (N-B, BLUE):**
  - `ade_core::consensus::rollback::apply_rollback` — used in
    S6's commit path for `ChainSelectorState` rollback.
- **Receive bridge (N-H, BLUE+RED):**
  - `ade_ledger::receive::reducer::receive_apply` — S6 updates
    the RollBackward branch.
  - `ade_ledger::receive::events::ReceiveError::RollbackOutOfScope`
    — the variant the new branch replaces (variant retained for
    migration; unused after S6).
- **No `ade_ledger::rollback::*` module exists at HEAD** — N-I
  is greenfield for the rollback driver tree.

## Entry Conditions

- **PHASE4-N-A..F closed** — codecs, consensus, ChainDb, mempool,
  block validity.
- **PHASE4-N-G closed** — send-side server pump.
- **PHASE4-N-H closed** — receive bridge admit-only with
  `RollbackOutOfScope` placeholder.
- **Constitution-coverage gate PASSES** at HEAD.

## Exit Criteria (CI-Verifiable)

- **CE-N-I-1 (traits + closed sums)** — Named tests:
  - `snapshot_reader_trait_is_object_safe` (S1).
  - `block_source_trait_is_object_safe` (S1).
  - `materialize_error_round_trips_through_pattern_match` (S1).
  - `commit_rollback_error_round_trips_through_pattern_match` (S1).
  - No registry flip at S1 alone; CN-STORE-07 + DC-CONS-22 flip at
    S2.

- **CE-N-I-2 (materialize driver)** — Named tests:
  - `materialize_returns_rollback_too_deep_when_no_snapshot` (S2).
  - `materialize_with_snapshot_at_target_returns_snapshot_state`
    (S2) — degenerate case: snapshot exactly at target.
  - `materialize_with_snapshot_below_target_replays_forward` (S2).
  - `materialize_fails_closed_on_invalid_block` (S2) — synthetic
    bad block → `Err(ReplayFailedAt)`.
  - `materialize_replay_forward_equals_direct_apply` (S2) — proves
    snapshot is a cache (DC-CONS-22 closure).
  - CI: `ci/ci_check_rollback_materialize_closure.sh` (S2) —
    single-authority + no HashMap/wall-clock/tokio.
  - Registry flip on close: `CN-STORE-07`, `DC-CONS-22` →
    `enforced`.

- **CE-N-I-3 (commit_rollback atomicity)** — Named tests:
  - `commit_rollback_advances_chaindb_and_ledger_atomically` (S3).
  - `commit_rollback_chain_write_failure_leaves_state_unchanged`
    (S3).
  - `commit_rollback_resets_pending_headers` (S3).
  - No registry flip at S3 alone; atomicity is structural support
    for DC-CONS-20's final flip at S6.

- **CE-N-I-4 (cadence + InMemorySnapshotCache)** — Named tests:
  - `should_snapshot_after_block_every_n_returns_true_at_cadence`
    (S4).
  - `should_snapshot_after_block_returns_false_off_cadence` (S4).
  - `should_snapshot_after_block_is_pure` (S4, replay-equivalence
    over a sequence).
  - `in_memory_snapshot_cache_nearest_le_returns_largest_key` (S4).
  - `in_memory_snapshot_cache_iteration_is_btreemap_ordered` (S4).
  - CI: `ci/ci_check_snapshot_cadence_purity.sh` (S4) — cadence
    module is HashMap/wall-clock/tokio-free; SnapshotCadence has
    no operator-tunable runtime input.
  - Registry flip on close: `DC-STORE-07` → `enforced`.

- **CE-N-I-5 (snapshot-write orchestration)** — Named tests:
  - `orchestrator_captures_snapshot_at_cadence_after_admission`
    (S5).
  - `orchestrator_per_peer_snapshot_decisions_are_independent`
    (S5).
  - No registry flip at S5 alone; integration foundation for S6.

- **CE-N-I-6 (close DC-CONS-20)** — Named tests:
  - `rollback_branch_returns_rolled_back_on_in_memory_snapshot`
    (S6).
  - `rollback_branch_returns_rollback_too_deep_when_no_snapshot`
    (S6).
  - `rollback_then_continue_admit_equals_straight_line_admit` (S6
    integration; snapshot-is-cache proof).
  - `rollback_branch_state_unchanged_on_materialize_failure` (S6).
  - Registry flip on close: `DC-CONS-20` → `enforced`
    (removes `open_obligation`).

## Slice index

| Slice | One-line scope | TCB |
|----|----|----|
| **N-I-S1** | BLUE `SnapshotReader` + `BlockSource` traits + `MaterializeError` + `CommitRollbackError` closed sums. | BLUE |
| **N-I-S2** | BLUE `materialize_rolled_back_state` driver — pure replay-forward fold. | BLUE |
| **N-I-S3** | BLUE `commit_rollback` helper — atomic state replacement. | BLUE |
| **N-I-S4** | GREEN cadence + `InMemorySnapshotCache` + `ChainDbBlockSource`. | GREEN |
| **N-I-S5** | RED snapshot-write orchestration in receive orchestrator. | RED |
| **N-I-S6** | BLUE-edit receive reducer `RollBackward` branch + end-to-end test; closes DC-CONS-20. | BLUE-edit + RED test |

## TCB Color Map (FC/IS Partition)

**BLUE (new):**
- `ade_ledger::rollback::traits` (S1) — `SnapshotReader`,
  `BlockSource`.
- `ade_ledger::rollback::error` (S1) — closed error sums.
- `ade_ledger::rollback::materialize` (S2) —
  `materialize_rolled_back_state`.
- `ade_ledger::rollback::commit` (S3) — `commit_rollback`.

**GREEN (new):**
- `ade_runtime::rollback::cadence` (S4) —
  `should_snapshot_after_block` + `SnapshotCadence`.
- `ade_runtime::rollback::in_memory_cache` (S4) —
  `InMemorySnapshotCache`.
- `ade_runtime::rollback::chaindb_block_source` (S4) —
  `ChainDbBlockSource<'a, D: ChainDb>`.

**RED (extended):**
- `ade_runtime::receive::orchestrator` (S5) — snapshot-write hook
  after each successful admission.

**BLUE-edit (final slice):**
- `ade_ledger::receive::reducer::receive_apply::block_delivered`
  → unchanged (still admits).
- `ade_ledger::receive::reducer::receive_apply::RollBackward arm`
  → S6 replaces `RollbackOutOfScope` with materialize + commit.

## Forbidden during this cluster

- Any new `pub fn` returning `(LedgerState, PraosChainDepState)`
  outside `materialize_rolled_back_state`. CI:
  `ci_check_rollback_materialize_closure.sh`.
- Construction of `ReceiveError::RollbackOutOfScope` from the
  reducer's RollBackward arm after S6.
- HashMap / HashSet / wall-clock / tokio / rand in any BLUE
  rollback module.
- Operator-tunable runtime cadence parameter in `SnapshotCadence`.
- `git commit --no-verify`.

## Replay obligations introduced by this cluster

- New canonical replay surface: rollback-then-replay-forward.
  S2's `materialize_replay_forward_equals_direct_apply` is the
  reducer-level proof; S6's
  `rollback_then_continue_admit_equals_straight_line_admit` is
  the end-to-end proof.
- `T-DET-01` strengthening: PHASE4-N-I (materialized rolled-back
  state is a new authoritative-deterministic surface).
- `DC-PROTO-09` strengthening: PHASE4-N-I (receive transcript
  determinism now includes the rollback transition).
- `CN-CONS-08` strengthening: PHASE4-N-I (admit + rollback
  symmetry).

## Authority reminder

> **Normative documents + registry + CI enforcement win.**
