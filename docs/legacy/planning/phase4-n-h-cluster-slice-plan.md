# Cluster/Slice Plan — PHASE4-N-H Receive-side header→body bridge

**Status**: cluster-plan phase complete; awaiting `/cluster-doc`
**HEAD pin**: `2adfb45`
**Date**: 2026-05-25
**Source**: `docs/planning/receive-side-bridge-invariants.md`
**Scope (Path A)**: admit-only. `RollBackward` is a structured
`RollbackOutOfScope` boundary; full rollback authority is a follow-on
cluster.

## Cluster Index (Dependency Order)

1. **PHASE4-N-H — Receive-side header→body bridge** — primary
   invariant: every block that lands in our ChainDb via the receive
   path passed `block_validity` (B1) with `Valid`, byte-identically;
   the header announced via `RollForward` matches the body
   subsequently delivered via `BlockDelivered`; the receive
   transcript is byte-deterministic across replays; admission is
   atomic across ChainDb + LedgerState + PraosChainDepState;
   `RollBackward` is a structured scope-boundary error.

Independently mergeable atop PHASE4-N-G (HEAD `2adfb45`). Mirrors
N-G's send-side architecture.

## PHASE4-N-H — Receive-side bridge

- **Primary invariant**: peer-supplied header+body bytes are admitted
  to ChainDb + LedgerState + PraosChainDepState **only** via
  `block_validity` Valid producing an `AdmittedBlock` token (private
  constructor; distinct from `AcceptedBlock`); the receive bridge
  reducer is pure, total, and deterministic over canonical inputs;
  `RollBackward` returns `Err(ReceiveError::RollbackOutOfScope)`
  with state unchanged.

- **Tier**: 1 (validator authority + transcript determinism) for
  S1–S5; release-evidence for S6 live half.

- **TCB partition**:
  - **BLUE (new)**:
    - `ade_ledger::receive::admitted` — `AdmittedBlock` token
      (private constructor reachable only from a `block_validity::
      Valid` branch); `admit_via_block_validity` helper.
    - `ade_ledger::receive::reducer` — `ReceiveEvent` / `ReceiveEffect`
      / `ReceiveError` closed sums; `PendingHeaderCache`;
      `ReceiveState` + `receive_apply` pure reducer.
    - `ade_ledger::receive::chain_write` — narrow `ChainDbWrite`
      trait taking `AdmittedBlock` by value.
  - **GREEN (new)**:
    - `ade_runtime::receive::events_to_state` — adapter lifting
      `ForkChoiceSignal` + `BatchDeliveryEvent` into the unified
      `ReceiveEvent` stream. Pure, no I/O.
    - `ade_runtime::receive::in_memory_chain_write` — adapter wiring
      `ChainDbWrite` to `InMemoryChainDb` (kept GREEN because the
      in-memory store is pure).
  - **RED (new)**:
    - `ade_runtime::receive::orchestrator` — per-peer dispatch
      (`dispatch_chain_sync_inbound`, `dispatch_block_fetch_inbound`)
      mirroring `n2n_server` shape; decodes incoming wire frames,
      lifts to `ReceiveEvent`, calls `receive_apply`, persists
      ChainDb writes.
    - `ade_core_interop::bin::live_block_follow_session` — operator
      evidence binary that follows a real cardano-node peer.
  - **Unchanged anchors**:
    - `ade_network::chain_sync::transition` /
      `ade_network::block_fetch::transition` (N-A signal/event
      sources).
    - `ade_ledger::block_validity::transition::block_validity` (B1
      authority — the single admission gate).
    - `ade_runtime::chaindb::{ChainDb, InMemoryChainDb,
      PersistentChainDb}` (N-D storage).

- **Cluster Exit Criteria**:

  - **CE-N-H-1** — `AdmittedBlock` private-constructor token +
    `ReceiveEvent` / `ReceiveEffect` / `ReceiveError` closed sums +
    `PendingHeaderCache` (BLUE): `AdmittedBlock` carries opaque
    bytes; its sole constructor lives in
    `ade_ledger::receive::admitted::admit_via_block_validity`, which
    takes block bytes + the context that `block_validity` needs and
    returns `Ok(AdmittedBlock)` IFF the verdict is `Valid`. The
    receive enums are closed (no `#[non_exhaustive]`, no `String`).
    `PendingHeaderCache` is `BTreeMap`-backed; canonical iteration.
    *(Contributes type-level closure for CN-CONS-08; flips
    CN-PROTO-07.)*

  - **CE-N-H-2** — `receive_apply` reducer (BLUE): pure, total,
    deterministic. `RollForward` caches header bytes + returns
    `Cached { slot, hash }`. `BlockDelivered` decodes the body
    header, cross-checks against the cached header (rejects with
    `HeaderBodyMismatch` if absent or mismatched — DC-CONS-19), runs
    `block_validity`, on `Valid` produces an `AdmittedBlock` +
    advances all three sub-states (ChainDb via the trait, ledger via
    the validity outcome, chain_dep via the validity outcome) +
    returns `Admitted { slot, hash }`. `RollBackward` returns
    `Err(RollbackOutOfScope)`. Header from `RollForward` MUST NOT
    mutate any of the three sub-states (I-6).
    *(Flips CN-CONS-08 and DC-CONS-19 to `enforced`.)*

  - **CE-N-H-3** — GREEN `events_to_state` adapter +
    `in_memory_chain_write` + synthetic session-transcript replay:
    adapter is pure (no I/O, no clock); given the same
    `(initial_state, event_sequence)`, two runs produce identical
    `(ledger', chain_dep', chaindb_fingerprint')`.
    *(Flips DC-PROTO-09 to `enforced`.)*

  - **CE-N-H-4** — RED per-peer receive orchestrator: decodes wire
    frames (chain-sync + block-fetch client-role) via the existing
    N-A codecs; threads the handshake-negotiated version into every
    reducer call; per-peer state independent; two synthetic peers
    against one ChainDb preserve per-session transcripts.
    *(No direct flip; strengthens `DC-PROTO-06` for the receive
    surface. Provides the structural surface RO-LIVE-02's live
    evidence drives through.)*

  - **CE-N-H-5** — Mechanical cross-impl adapter (CI test):
    drive the Conway-576 corpus block-by-block through the full
    receive pipeline; assert the final `ChainDb` tip equals the
    expected `(slot, hash)` for the last admitted block AND the
    admitted bytes equal the corpus bytes byte-identically AND the
    `LedgerState` fingerprint matches the expected post-application
    fingerprint.
    *(Mechanical pre-condition for RO-LIVE-02.)*

  - **CE-N-H-6** — Operator-action live evidence: conditional. Either
    a `CE-N-H-LIVE_<date>.log` captures a real cardano-node follow
    over N blocks with ChainDb-tip-equals-peer-tip at every step, OR
    the cluster ships with `RO-LIVE-02` →
    `partial + blocked_until_operator_peer_available`. Mirrors N-C
    `CN-CONS-06` / N-G `RO-LIVE-01`.
    *(Flips RO-LIVE-02 to `enforced` (case a) or `partial`
    + open_obligation (case b).)*

- **Slices**:

  - **N-H-S1** — `AdmittedBlock` token + receive closed sums +
    `PendingHeaderCache`
    Invariant: `AdmittedBlock` private constructor; `ReceiveEvent` /
    `ReceiveEffect` / `ReceiveError` closed; `PendingHeaderCache`
    BTreeMap-backed; type-level CN-PROTO-07 closure (no
    locally-originated event constructor in the receive reducer's
    public API).
    Addresses: **CE-N-H-1**.
    TCB: **BLUE** (`ade_ledger::receive::{admitted, events,
    pending_header_cache}` new modules); narrow `ChainDbWrite` trait
    in `ade_ledger::receive::chain_write`.
    CI: `ci/ci_check_admitted_block_closure.sh` (new) — forbids any
    `pub fn` constructing `AdmittedBlock` outside the canonical
    `admit_via_block_validity` site.

  - **N-H-S2** — `receive_apply` reducer
    Invariant: pure, total, deterministic transition. Header-body
    cross-check (DC-CONS-19) enforced before `block_validity` runs.
    Admission is one structural transition over the three
    sub-states; partial admission unrepresentable. `RollBackward`
    returns `Err(RollbackOutOfScope)` without mutation. Failure
    modes are closed (HeaderBodyMismatch, Validity(_), ChainDb(_),
    RollbackOutOfScope).
    Addresses: **CE-N-H-2**.
    TCB: **BLUE** (`ade_ledger::receive::reducer`).
    CI: `ci/ci_check_receive_reducer_closure.sh` (new) — forbids
    HashMap/HashSet/wall-clock/tokio in the reducer; positive grep
    for the `block_validity` call site.

  - **N-H-S3** — GREEN `events_to_state` adapter +
    `in_memory_chain_write` + transcript replay corpus
    Invariant: adapter is pure; transcript replay over a synthetic
    corpus produces byte-identical `(ledger', chain_dep',
    chaindb_fingerprint')` across two runs.
    Addresses: **CE-N-H-3**.
    TCB: **GREEN** (`ade_runtime::receive::{events_to_state,
    in_memory_chain_write}`) + corpus in
    `crates/ade_testkit/fixtures/receive_paths/` and replay
    scaffolding.
    CI: `ci/ci_check_receive_replay_purity.sh` (new); extends
    `ci_check_no_private_keys_in_corpus.sh` to the new fixture root.

  - **N-H-S4** — RED per-peer receive orchestrator
    Invariant: orchestrator threads the handshake-negotiated version
    into every reducer call; per-peer state independent. Frame
    decoding goes through the existing N-A codecs only. ChainDb
    writes go through the `ChainDbWrite` trait (BLUE shape, RED
    impl). Multi-peer determinism: two synthetic peers driven in
    parallel against one shared ChainDb produce per-session
    transcripts identical to their solo runs.
    Addresses: **CE-N-H-4**.
    TCB: **RED** (`ade_runtime::receive::orchestrator`).
    CI: `ci/ci_check_receive_orchestrator_no_producer_dep.sh` (new)
    — forbids imports of `producer::signing`/`producer::broadcast`
    /`producer::scheduler` from the receive orchestrator.

  - **N-H-S5** — Mechanical cross-impl adapter
    Invariant: drive the Conway-576 corpus block-by-block through
    the full receive pipeline; final state matches expected. No
    network egress.
    Addresses: **CE-N-H-5**.
    TCB: **GREEN/test** in `crates/ade_runtime/tests/
    receive_pipeline_corpus_drive.rs`.

  - **N-H-S6** — Live evidence binary + procedure
    Invariant: `live_block_follow_session` binary builds and starts
    in hermetic mode; `--connect` mode drives the receive pipeline
    against a real cardano-node peer and captures
    `CE-N-H-LIVE_<date>.log`. Procedure documents how to flip
    `RO-LIVE-02` from `partial` → `enforced`.
    Addresses: **CE-N-H-6**.
    TCB: **RED** (`ade_core_interop::bin::live_block_follow_session`).

- **Replay obligations**:
  - New canonical replay corpus at
    `crates/ade_testkit/fixtures/receive_paths/`: ordered
    `(initial_state, ReceiveEvent_sequence) -> expected_state`
    triples. Uses Conway-576 corpus block bytes as the body source;
    headers projected via the existing `accepted_block_header_bytes`
    recipe (DC-CONS-16) for cross-check fidelity. No private keys.
  - `T-DET-01` strengthened by PHASE4-N-H (new
    authoritative-deterministic surface: receive transcript).
  - `T-ENC-01` strengthened by PHASE4-N-H (peer-supplied wire bytes
    flow into ChainDb verbatim — no re-encoding on the receive
    path).
  - `DC-CONS-13` strengthened by PHASE4-N-H (symmetric receive
    closure: admit = `block_validity::Valid` only).
  - `CN-CONS-07` strengthened by PHASE4-N-H (broadcast gate's mirror:
    receive admission gate via `AdmittedBlock`).
  - `DC-PROTO-06` strengthened by PHASE4-N-H (version threaded
    through the receive-role reducer surface).
  - `DC-CONS-16` strengthened by PHASE4-N-H (header projection
    reused for receive-side cross-check via
    `accepted_block_header_bytes` over the cached header).

- **Forbidden states across the cluster** (cross-slice invariants):
  - A block landing in ChainDb via the receive path whose bytes did
    not pass `block_validity` Valid → compile-time impossible
    (`AdmittedBlock` private constructor + `ChainDbWrite` takes
    `AdmittedBlock` by value).
  - Reducer mutating any of the three sub-states from a `RollForward`
    alone → reducer signature only mutates on the `BlockDelivered` /
    `Valid` branch (S2 enforcement).
  - Body admission without header cross-check →
    `HeaderBodyMismatch` is total over the BlockDelivered branch
    when no matching cache entry exists.
  - `RollBackward` returning `Ok` → reducer signature returns
    `Err(RollbackOutOfScope)` unconditionally for that arm.
  - Receive orchestrator depending on `producer::signing` /
    `producer::broadcast` / `producer::scheduler` → CI gate.
  - Replay corpus carrying private-key bytes → existing
    `ci_check_no_private_keys_in_corpus.sh` extension.

- **Live-evidence conditionality**: CE-N-H-6 follows the
  established pattern (N-C CN-CONS-06, N-G RO-LIVE-01). Cluster
  ships with `RO-LIVE-02` → `partial` +
  `blocked_until_operator_peer_available` if no peer is wired at
  close.

## CE coverage matrix

| CE | Slice | Registry IDs flipped to `enforced` on close |
|----|----|----|
| CE-N-H-1 | S1 | CN-PROTO-07 |
| CE-N-H-2 | S2 | CN-CONS-08, DC-CONS-19 |
| CE-N-H-3 | S3 | DC-PROTO-09 |
| CE-N-H-4 | S4 | *(strengthens DC-PROTO-06)* |
| CE-N-H-5 | S5 | *(mechanical pre-condition for RO-LIVE-02)* |
| CE-N-H-6 | S6 | RO-LIVE-02 (enforced or partial+open_obligation) |

DC-CONS-20 stays `declared` with `open_obligation = "rollback_side_blocked_until_ledger_snapshot_cluster"` — admit-side enforcement is recorded under CN-CONS-08 + DC-CONS-19 per Path A scope decision.

All 5 N-H registry entries reachable as listed (RO-LIVE-02 conditional). 5 existing entries strengthened (T-DET-01, T-ENC-01, DC-CONS-13, CN-CONS-07, DC-PROTO-06, DC-CONS-16). No carry-forward except the explicit DC-CONS-20 rollback-side cross-cluster obligation.
