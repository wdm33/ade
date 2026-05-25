# Invariant Slice — PHASE4-N-E S4

## Slice Header
**Slice Name:** N2N tx-submission2 → mempool_ingress bridge + live-evidence procedure
**Cluster:** PHASE4-N-E
**Status:** Proposed
**CEs addressed:** CE-N-E-6 (live N2N evidence) — code half mechanical; live-log half operator-action
**Dependencies:** S1 (`IngressEvent`/`mempool_ingress`), S2 (`replay_ingress_trace`), S3 (`canonicalize_peer_streams`); PHASE4-N-A closed (tx-submission2 codec + state machine + N2N handshake)

---

## Intent

Wire the existing `ade_network::tx_submission` state machine to the new
`ade_ledger::mempool::mempool_ingress` chokepoint:

1. Translate `InventoryEvent::TxsDelivered` outputs into `IngressEvent`s
   under `IngressSource::N2N` (deterministic GREEN adapter).
2. Per-peer accumulator that collects tx bytes from one or more peers
   into `PeerSubmissionQueue`s, then canonicalizes them
   (`canonicalize_peer_streams`) and replays them through
   `replay_ingress_trace`. Pure, testable.
3. Document the operator procedure for the CE-N-E-6 live evidence
   capture against a real cardano-node peer.

The home is `ade_core_interop` — the project's established RED
live-interop crate (already houses the PHASE4-N-B follow-mode bridge
that closes CE-N-B-6 with the same pattern). The cluster doc's prior
"ade_runtime" placement is corrected here.

---

## The change

### 1. New module `crates/ade_core_interop/src/tx_submission.rs`

```rust
// GREEN adapter: map a single InventoryEvent from a known peer source
// to zero or more IngressEvents. Pure; no I/O.
pub fn event_to_ingress(
    event: &InventoryEvent,
    source: IngressSource,
) -> Vec<IngressEvent> {
    match event {
        InventoryEvent::TxsDelivered { tx_bytes } => tx_bytes
            .iter()
            .cloned()
            .map(|b| IngressEvent::new(source, b))
            .collect(),
        // Non-tx events carry no bytes; nothing to ingest.
        InventoryEvent::ServerOpened
        | InventoryEvent::IdsRequested { .. }
        | InventoryEvent::IdsDelivered { .. }
        | InventoryEvent::TxsRequested { .. } => Vec::new(),
    }
}

// GREEN per-peer accumulator: collects N2N tx_bytes deliveries from
// one peer into a PeerSubmissionQueue.
pub struct PeerAccumulator {
    peer: PeerId,
    txs: Vec<Vec<u8>>,
}

impl PeerAccumulator {
    pub fn new(peer: PeerId) -> Self { ... }
    pub fn observe(&mut self, event: &InventoryEvent) { ... }
    pub fn drain(&mut self) -> PeerSubmissionQueue { ... }
}

// GREEN orchestrator: given per-peer InventoryEvent streams, build
// queues, canonicalize, and replay through mempool_ingress.
//
// Pure function of the inputs — the production socket loop driving
// peers is the only RED layer; this function is exercised by tests
// using synthetic InventoryEvents.
pub fn ingest_n2n_events(
    base: LedgerState,
    per_peer: &[(PeerId, Vec<InventoryEvent>)],
) -> (MempoolState, Vec<AdmitOutcome>) {
    let queues: Vec<PeerSubmissionQueue> = per_peer
        .iter()
        .map(|(peer, events)| {
            let txs = events
                .iter()
                .flat_map(|e| match e {
                    InventoryEvent::TxsDelivered { tx_bytes } => tx_bytes.clone(),
                    _ => Vec::new(),
                })
                .collect();
            PeerSubmissionQueue { peer: peer.clone(), source: IngressSource::N2N, txs }
        })
        .collect();
    let canonical = canonicalize_peer_streams(&queues);
    replay_ingress_trace(base, &canonical)
}
```

The RED socket-driver loop (real cardano-node N2N handshake + tx-submission2
chatter) is **out of scope as automated code for this slice** — it follows
the established PHASE4-N-B CE-N-B-6 pattern: a manual operator binary that
captures a log artifact under `docs/clusters/PHASE4-N-E/`. The mechanical
half lands here; the live-log half is operator-action.

### 2. Re-exports `crates/ade_core_interop/src/lib.rs`

```rust
pub mod tx_submission;
```

### 3. Cargo dependency edge (already present)

`ade_core_interop` already depends on `ade_ledger`-adjacent crates via
`ade_runtime`. Direct dependencies needed: `ade_ledger`, `ade_testkit`
(for tests), `ade_network` (already there for codecs + state machines).

### 4. Integration tests `crates/ade_core_interop/tests/tx_submission_ingress.rs`

- `event_to_ingress_maps_txs_delivered` — `InventoryEvent::TxsDelivered`
  with N tx_bytes → N `IngressEvent`s with the matching source.
- `event_to_ingress_other_events_emit_nothing` — `ServerOpened`,
  `IdsRequested`, `IdsDelivered`, `TxsRequested` yield empty Vec.
- `peer_accumulator_round_trip` — observe two `TxsDelivered`s; drain
  produces a `PeerSubmissionQueue` with all tx_bytes in observation
  order.
- `ingest_n2n_events_admits_valid_corpus` — feed the B-track valid +
  adversarial corpus as synthetic `InventoryEvent::TxsDelivered`
  streams; `ingest_n2n_events` produces the same admit/reject sequence
  as `replay_ingress_trace` over the manually-built ingress trace.
- `multi_peer_n2n_events_canonicalize_deterministically` — two
  distinct interleavings of per-peer InventoryEvent streams produce
  byte-identical `(MempoolState, Vec<AdmitOutcome>)`.

### 5. Operator procedure: `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md`

A documented procedure (NOT a binary in this slice) for capturing the
live evidence log against a real cardano-node N2N peer:

```
Pre-conditions:
- A reachable cardano-node N2N relay (preprod or mainnet).
- Ade workspace built; `cargo run -p ade_core_interop` reachable.
- The operator has a fresh `LedgerState` snapshot to use as `base`.

Procedure:
1. Run an N2N handshake + tx-submission2 client against the relay,
   collecting every InventoryEvent::TxsDelivered for a sustained window
   (≥ 10 min, ≥ 50 distinct txs observed).
2. Feed the captured stream through `event_to_ingress` → per-peer queues
   → `ingest_n2n_events(base, ...)`.
3. For each admitted tx, log: tx_id, AdmitOutcome variant, accumulating
   state delta size.
4. For comparison, take the same captured tx bytes and feed them
   directly through `tx_validity` (no ingress wrapping). Assert
   byte-identical verdicts.
5. Commit the log to `docs/clusters/PHASE4-N-E/CE-N-E-6_<YYYY-MM-DD>.log`
   following the CE-N-B-6 log format.

Closure: the log entry counts as CE-N-E-6 evidence when committed.
The agreement assertion in step 4 is the load-bearing no-false-accept
property at the wire boundary.
```

### 6. Cluster doc TCB partition update

Update `docs/clusters/PHASE4-N-E/cluster.md` TCB Color Map row from
`ade_runtime::tx_submission::n2n_session` to:

> `ade_core_interop::tx_submission` (S4 — GREEN adapter + RED operator
> procedure; live-log artifact lives under `docs/clusters/PHASE4-N-E/`).

Same shape applies to S5 (`ade_core_interop::local_tx_submission`).

### 7. Registry — no changes

S4 produces mechanical GREEN evidence (the adapter + accumulator + tests)
plus an operator-procedure document. No new rule. `DC-MEM-01.tests`
already covers the agreement evidence via S2's ingress-replay harness;
S4's new adapter is tested in-place but is not registry-load-bearing.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build -p ade_core_interop` green.
- **AC-2** — `cargo test -p ade_core_interop --test tx_submission_ingress`
  green (5 tests).
- **AC-3** — `cargo test -p ade_ledger` and `cargo test -p ade_testkit`
  unchanged.
- **AC-4** — All four existing N-E CI gates still PASS
  (`ci_check_mempool_ingress_closure.sh`,
  `ci_check_mempool_ingress_replay.sh`,
  `ci_check_constitution_coverage.sh`).
- **AC-5** — `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` exists
  with the operator procedure.
- **AC-6** — `docs/clusters/PHASE4-N-E/cluster.md` TCB Color Map reflects
  `ade_core_interop::tx_submission` (not `ade_runtime`).

---

## Hard Prohibitions

- No new dependency edge from any BLUE crate to `ade_core_interop`.
- No mutation of `MempoolState.accumulating` from anywhere in
  `ade_core_interop` — must go through `mempool_ingress`
  (S1 closure gate continues to enforce this).
- No new registry rule.
- The GREEN adapter must NOT decode tx_bytes — it wraps them.
- The GREEN orchestrator (`ingest_n2n_events`) must call
  `mempool_ingress` (via `replay_ingress_trace`), never `admit`
  directly.
- The actual async socket loop is operator-action; do not attempt to
  ship a half-baked one as automated code (the operator procedure is
  the contract).

---

## Explicit Non-Goals

- Full async N2N session loop binary (operator procedure references
  the `live_consensus_session` pattern; a full N2N+tx-submission2
  client binary is its own follow-up if and when an operator wants to
  capture the evidence in this session).
- N2C / local-tx-submission (S5).
- Mempool bounds / shedding (Tier-5 cluster).
- Outbound propagation (separate cluster).
