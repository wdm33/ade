# Cluster/Slice Plan — PHASE4-N-I In-memory snapshot + replay-forward rollback

**Status**: cluster-plan phase complete; awaiting `/cluster-doc`
**HEAD pin**: `f143984`
**Date**: 2026-05-26
**Source**: `docs/planning/ledger-snapshot-rollback-invariants.md`
**Scope (in-memory variant)**: ships a `SnapshotReader` trait + an
in-memory impl. Persistent on-disk encoding is carved out to a
follow-on cluster — DC-CONS-21 stays declared with
`open_obligation = "persistent_ledger_snapshot_encoding_follow_on_cluster"`.

## Cluster Index (Dependency Order)

1. **PHASE4-N-I — In-memory snapshot + replay-forward rollback** —
   primary invariant: when the receive bridge receives
   `RollBackward(target_point)`, materialize the rolled-back
   `(LedgerState, PraosChainDepState)` via the single canonical
   driver `materialize_rolled_back_state` (snapshot + replay-
   forward), then commit it atomically with
   `ChainDb::rollback_to_slot` and per-peer state replacement.
   Snapshot materialization is a pure cache over canonical history,
   not a second ledger evolution path.

Independently mergeable atop PHASE4-N-H (HEAD `f143984`). Closes
DC-CONS-20 rollback-side (the open_obligation N-H deferred).

## PHASE4-N-I — In-memory rollback

- **Primary invariant**: `RollBackward(target_point)` returns
  `Ok(RolledBack { to_slot })` when a snapshot ≤ target_point.slot
  exists and replay-forward from it succeeds; returns
  `Err(RollbackTooDeep)` or `Err(ReplayFailedAt)` otherwise, with
  receive state unchanged. The materialized state goes through
  ChainDb.rollback_to_slot + ledger replacement + chain_dep
  replacement + pending-header reset as one structural transition.

- **Tier**: 1 (rollback authority) for S1-S6; release-evidence
  carry-forward via RO-LIVE-02 (unchanged).

- **TCB partition**:
  - **BLUE (new)**:
    - `ade_ledger::rollback::traits` — `SnapshotReader` +
      `BlockSource` narrow read-only traits.
    - `ade_ledger::rollback::error` — `MaterializeError` +
      `CommitRollbackError` closed sums (variants:
      `RollbackTooDeep`, `ReplayFailedAt`, `EraNotSupported`).
    - `ade_ledger::rollback::materialize` —
      `materialize_rolled_back_state(target, &reader, &source,
      era_schedule, ledger_view) -> Result<(LedgerState,
      PraosChainDepState), MaterializeError>`. Pure replay-forward
      fold over `apply_block_with_verdicts`. Epoch boundaries
      handled internally by `apply_block_with_verdicts` (resolved
      pre-S1: rules.rs:244-250 calls
      `detect_epoch_transition + apply_epoch_boundary_full` before
      block application).
    - `ade_ledger::rollback::commit` —
      `commit_rollback(state, target, new_ledger, new_chain_dep,
      &mut chain_write) -> Result<(), CommitRollbackError>`.
      Atomic state replacement helper; staged-then-committed shape
      mirroring N-H S2 admit branch.
    - `ade_ledger::receive::events::ReceiveError::RollbackOutOfScope`
      → marked deprecated (variant retained for migration; the
      RollBackward branch no longer constructs it after S6).
  - **GREEN (new)**:
    - `ade_runtime::rollback::cadence` —
      `should_snapshot_after_block(slot, block_no, cadence,
      last_snapshot) -> bool`. Pure decision function.
    - `ade_runtime::rollback::in_memory_cache` —
      `InMemorySnapshotCache` implementing `SnapshotReader`. Owns
      a `BTreeMap<SlotNo, (LedgerState, PraosChainDepState)>`.
    - `ade_runtime::rollback::chaindb_block_source` —
      `ChainDbBlockSource<'a, D: ChainDb>` implementing
      `BlockSource`.
  - **RED (extended)**:
    - `ade_runtime::receive::orchestrator` — extended after each
      successful admission: consult the GREEN cadence; on `true`,
      capture `(ledger, chain_dep)` into the in-memory snapshot
      cache.
    - `ade_ledger::receive::reducer` — S6 final-slice update:
      `RollBackward` branch calls
      `materialize_rolled_back_state` + `commit_rollback` instead
      of returning `RollbackOutOfScope`.
  - **Unchanged anchors**:
    - `ade_runtime::chaindb::ChainDb::rollback_to_slot` (N-D).
    - `ade_core::consensus::rollback::apply_rollback` (N-B; used
      by S6 for ChainSelectorState rollback alongside the
      ledger/chain_dep commit).
    - `ade_ledger::rules::apply_block_with_verdicts` (B1+; the
      single forward-application authority replay-forward folds
      over).

- **Cluster Exit Criteria**:

  - **CE-N-I-1** — `SnapshotReader` / `BlockSource` traits +
    closed error sums (BLUE): narrow read-only traits a materialize
    driver can compose against. `MaterializeError` /
    `CommitRollbackError` closed; no `String`, no
    `#[non_exhaustive]`. `RollbackTooDeep`, `ReplayFailedAt`,
    `EraNotSupported` variants present.
    *(Foundation for CN-STORE-07.)*

  - **CE-N-I-2** — `materialize_rolled_back_state` driver (BLUE):
    pure, total, deterministic fold over
    `apply_block_with_verdicts`. Given `(target, &reader,
    &source)`: lookup nearest snapshot ≤ target.slot; if none,
    `RollbackTooDeep`. If snapshot found, iterate blocks
    `(snapshot_slot+1..=target.slot)` from BlockSource; apply each
    via `apply_block_with_verdicts`; return final
    `(LedgerState, PraosChainDepState)`. Any
    `apply_block_with_verdicts` failure → `ReplayFailedAt
    { slot, error }`. Pre-Conway era encountered →
    `EraNotSupported`.
    *(Flips CN-STORE-07 + DC-CONS-22 to `enforced`.)*

  - **CE-N-I-3** — `commit_rollback` helper + atomicity tests
    (BLUE): atomic state replacement. Takes
    `&mut ReceiveState` + materialized `(ledger, chain_dep)` +
    `&mut chain_write`. Sequence: (1) call
    `chain_write.rollback_to_slot(target.slot)` (irreversible
    step first); (2) replace `state.ledger`,
    `state.chain_dep`; (3) reset `state.pending_headers`
    (post-rollback the cached headers are stale). Atomicity: if
    chain_write fails, state unchanged. If chain_write succeeds,
    in-memory updates are infallible.
    *(Closes the atomicity half of DC-CONS-20; final flip in S6.)*

  - **CE-N-I-4** — GREEN cadence policy + `InMemorySnapshotCache`
    (GREEN): `should_snapshot_after_block` is pure and total.
    `SnapshotCadence` carries `every_n_blocks: u32` (default
    N=100, BLUE-structural). `InMemorySnapshotCache::admit(slot,
    ledger, chain_dep)` stores into the BTreeMap. `nearest_le(slot)`
    returns the snapshot at the largest key ≤ slot.
    *(Flips DC-STORE-07 to `enforced`.)*

  - **CE-N-I-5** — Snapshot-write orchestration in receive
    orchestrator (RED): after every successful admission, the
    orchestrator consults `should_snapshot_after_block` and, on
    `true`, captures `(state.receive_state.ledger,
    state.receive_state.chain_dep)` into the per-peer (or shared)
    `InMemorySnapshotCache`. Multi-peer determinism preserved:
    each peer's snapshot decisions are independent functions of
    its admitted-block sequence.
    *(No direct flip; foundational for S6 integration test.)*

  - **CE-N-I-6** — Receive reducer `RollBackward` branch update +
    integration test (BLUE-edit + RED test): the reducer's
    RollBackward arm now calls `materialize_rolled_back_state` +
    `commit_rollback`. End-to-end test: admit blocks A..F; snapshot
    captured at C per cadence; receive RollBackward(C); assert
    state.ledger fingerprint equals what direct apply of A..C
    would have produced (snapshot-then-replay-equivalence). Then
    continue admitting C..H atop the rolled-back state; assert
    final state equals direct apply of A,B,C,D,E,F,G,H without
    the intervening rollback (snapshotting is a cache, not
    authority).
    *(Final flip: DC-CONS-20 → `enforced`; removes
    open_obligation.)*

- **Slices**:

  - **N-I-S1** — Traits + closed error sums
    Invariant: `SnapshotReader` + `BlockSource` are narrow
    read-only traits the driver composes against. Error sums are
    closed.
    Addresses: **CE-N-I-1**.
    TCB: **BLUE** (`ade_ledger::rollback::{traits, error}` new
    modules).
    CI: none added at S1 (S2's gate covers the use-site).

  - **N-I-S2** — `materialize_rolled_back_state` driver
    Invariant: pure replay-forward fold; epoch boundaries handled
    implicitly by `apply_block_with_verdicts`; fail-closed on
    missing snapshot or invalid block in range.
    Addresses: **CE-N-I-2**.
    TCB: **BLUE** (`ade_ledger::rollback::materialize`).
    CI: `ci/ci_check_rollback_materialize_closure.sh` (new) —
    forbids HashMap/wall-clock/tokio in the materialize module;
    forbids pub fn returning `(LedgerState, PraosChainDepState)`
    outside `materialize_rolled_back_state` (CN-STORE-07
    single-authority).

  - **N-I-S3** — `commit_rollback` helper + atomicity tests
    Invariant: chain_write step first (irreversible); in-memory
    replacement is infallible. Per-peer state unchanged on
    chain_write failure.
    Addresses: **CE-N-I-3**.
    TCB: **BLUE** (`ade_ledger::rollback::commit`).

  - **N-I-S4** — GREEN cadence + `InMemorySnapshotCache`
    Invariant: cadence is a pure decision function;
    `InMemorySnapshotCache` is BTreeMap-backed, deterministic.
    Addresses: **CE-N-I-4**.
    TCB: **GREEN** (`ade_runtime::rollback::{cadence,
    in_memory_cache, chaindb_block_source}`).
    CI: `ci/ci_check_snapshot_cadence_purity.sh` (new) — no
    wall-clock / tokio / HashMap in cadence module; no operator-
    tunable runtime input parameter in `SnapshotCadence`
    (compile-time + grep).

  - **N-I-S5** — Snapshot-write orchestration in receive
    orchestrator
    Invariant: orchestrator captures snapshot after each admission
    according to cadence; capture is deterministic given admitted
    block sequence.
    Addresses: **CE-N-I-5**.
    TCB: **RED** (`ade_runtime::receive::orchestrator` extension);
    test infrastructure verifies multi-peer cache independence.

  - **N-I-S6** — Wire RollBackward branch → close DC-CONS-20
    Invariant: receive reducer's RollBackward arm calls
    `materialize_rolled_back_state` + `commit_rollback`; on
    success returns `Ok(ReceiveEffect::RolledBack { to_slot })`;
    on failure returns the relevant `MaterializeError` /
    `CommitRollbackError` wrapped in `ReceiveError`. End-to-end
    integration test: admit-then-rollback-then-admit produces the
    same final state as straight-line admit (snapshot-is-cache
    proof).
    Addresses: **CE-N-I-6**.
    TCB: **BLUE-edit** (`ade_ledger::receive::reducer`) + **RED
    test** (`crates/ade_runtime/tests/receive_rollback_*.rs`).
    *(Final flip on close: DC-CONS-20 → `enforced`.)*

- **Replay obligations**:
  - New canonical replay corpus: synthetic event sequences with
    embedded RollBackward events at various depths. Lives at
    `crates/ade_testkit/fixtures/rollback_corpus/` (or inline in
    the integration test for the in-memory scope).
  - `T-DET-01` strengthened by PHASE4-N-I (materialized rolled-
    back state is a new authoritative-deterministic surface).
  - `DC-PROTO-09` strengthened by PHASE4-N-I (receive transcript
    determinism now includes the rollback transition).
  - `CN-CONS-08` strengthened by PHASE4-N-I (admit + rollback
    symmetry: same single-authority discipline on both transitions).
  - No private-key material in the new corpus.

- **Forbidden states across the cluster**:
  - A pub fn returning `(LedgerState, PraosChainDepState)`
    outside `materialize_rolled_back_state` → CI gate.
  - `RollbackOutOfScope` constructed in the reducer's RollBackward
    arm after S6 → compile-time impossible (the arm calls
    materialize/commit instead).
  - Rollback that updates ChainDb but not LedgerState (or vice
    versa) → S3 atomicity tests cover.
  - HashMap iteration / wall-clock / tokio in BLUE rollback
    modules → CI gates.
  - Operator-tunable runtime cadence parameter (S4) — explicitly
    out of scope per scope decision #1.

- **Conditional / carry-forward**:
  - **DC-CONS-21 stays declared** with new open_obligation
    naming the persistent-encoder follow-on cluster. The
    in-memory variant satisfies all CEs N-I claims; persistent
    snapshot bytes are deferred. The follow-on cluster MUST close
    DC-CONS-21 fully (round-trip + version tag + fingerprint
    cross-check).

## CE coverage matrix

| CE | Slice | Registry IDs flipped to `enforced` on close |
|----|----|----|
| CE-N-I-1 | S1 | *(foundation for CN-STORE-07)* |
| CE-N-I-2 | S2 | CN-STORE-07, DC-CONS-22 |
| CE-N-I-3 | S3 | *(atomicity half of DC-CONS-20)* |
| CE-N-I-4 | S4 | DC-STORE-07 |
| CE-N-I-5 | S5 | *(integration foundation for S6)* |
| CE-N-I-6 | S6 | DC-CONS-20 (final flip; removes open_obligation) |

DC-CONS-21 stays declared with new open_obligation. All other 4 new
N-I entries reachable. 3 existing entries strengthened (T-DET-01,
DC-PROTO-09, CN-CONS-08).
