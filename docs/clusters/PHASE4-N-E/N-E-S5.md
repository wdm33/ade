# Invariant Slice — PHASE4-N-E S5

## Slice Header
**Slice Name:** N2C local-tx-submission → mempool_ingress bridge + live-evidence procedure
**Cluster:** PHASE4-N-E
**Status:** Proposed
**CEs addressed:** CE-N-E-7 (live N2C evidence) — code half mechanical; live-log half operator-action
**Dependencies:** S1 (`IngressEvent`/`mempool_ingress`), S2 (`replay_ingress_trace`), S3 (`canonicalize_peer_streams`), S4 (`ade_core_interop::tx_submission` pattern); PHASE4-N-A closed (local-tx-submission codec + state machine + N2C handshake)

---

## Intent

Mirror S4 for the N2C local-tx-submission mini-protocol:

1. Translate `LocalTxSubmissionEvent::TxSubmitted` outputs into
   `IngressEvent`s under `IngressSource::N2C` (deterministic GREEN adapter).
2. Per-client accumulator that collects tx bytes from one or more local
   IPC clients into `PeerSubmissionQueue`s (peer = `ClientId`), then
   canonicalizes + replays through `mempool_ingress`.
3. Cross-check at the wire-event layer: the same tx bytes submitted via
   N2C produce byte-identical `(MempoolState, AdmitOutcome)` to N2N
   submission of the same bytes — the load-bearing N-E-N7 / N-E-8
   property at the new wire surface.
4. Operator procedure for the CE-N-E-7 live evidence capture against
   real cardano-cli.

---

## The change

### 1. New module `crates/ade_core_interop/src/local_tx_submission.rs`

```rust
// GREEN adapter: map one LocalTxSubmissionEvent into zero or one
// IngressEvent. Only TxSubmitted carries bytes; TxAccepted /
// TxRejected are server-to-client responses (no tx to admit).
pub fn local_event_to_ingress(event: &LocalTxSubmissionEvent) -> Vec<IngressEvent> {
    match event {
        LocalTxSubmissionEvent::TxSubmitted { tx_bytes } => {
            vec![IngressEvent::new(IngressSource::N2C, tx_bytes.clone())]
        }
        LocalTxSubmissionEvent::TxAccepted
        | LocalTxSubmissionEvent::TxRejected { .. } => Vec::new(),
    }
}

// GREEN per-client accumulator over a LocalTxSubmissionEvent stream.
pub struct ClientAccumulator {
    client: PeerId,   // ClientId is just a PeerId-shaped opaque tag
    txs: Vec<Vec<u8>>,
}

impl ClientAccumulator {
    pub fn new(client: PeerId) -> Self;
    pub fn observe(&mut self, event: &LocalTxSubmissionEvent);
    pub fn drain(self) -> PeerSubmissionQueue;  // source = IngressSource::N2C
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

// GREEN orchestrator: given per-client LocalTxSubmissionEvent streams,
// build queues, canonicalize, and replay through mempool_ingress.
pub fn ingest_n2c_events(
    base: LedgerState,
    per_client: &[(PeerId, Vec<LocalTxSubmissionEvent>)],
) -> (MempoolState, Vec<AdmitOutcome>);
```

Same shape as S4's `ade_core_interop::tx_submission`, just over the N2C
transport. The actual UDS socket loop is operator-action.

### 2. Re-export `crates/ade_core_interop/src/lib.rs`

```rust
pub mod local_tx_submission;
```

### 3. Integration tests `crates/ade_core_interop/tests/local_tx_submission_ingress.rs`

- `local_event_to_ingress_maps_tx_submitted` — `TxSubmitted` →
  one `IngressEvent` with `IngressSource::N2C`.
- `local_event_to_ingress_other_events_emit_nothing` — `TxAccepted`
  and `TxRejected` yield empty Vec.
- `client_accumulator_round_trip` — observe / drain produces queue with
  N2C source.
- `ingest_n2c_events_admits_b_track_corpus` — feed the B-track corpus
  as synthetic `TxSubmitted` events; result agrees with direct
  `replay_ingress_trace`.
- `n2n_and_n2c_bridges_produce_identical_outcomes` — load-bearing
  source-invariance at the wire-event layer: take the same tx bytes,
  route via `ingest_n2n_events` and `ingest_n2c_events`, assert
  byte-identical `(MempoolState, Vec<AdmitOutcome>)`. This is the
  CE-N-E-7 mechanical evidence.
- `multi_client_n2c_canonicalize_deterministically` — analogous to
  S4's multi-peer N2N determinism test.

### 4. Operator procedure: `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md`

```
Pre-conditions:
- A running Ade node binary exposing a Unix-domain N2C socket.
- cardano-cli installed (any 8.x series; the local-tx-submission
  wire format is era-stable).
- A fresh `LedgerState` snapshot to use as `base`.

Procedure:
1. Bring up Ade's N2C UDS endpoint serving the local-tx-submission
   mini-protocol.
2. Use `cardano-cli transaction submit --tx-file <path>` to submit
   at least 10 distinct txs across at least 2 client invocations.
3. The N2C session loop collects every
   LocalTxSubmissionEvent::TxSubmitted, feeds it through
   `local_event_to_ingress` and `ingest_n2c_events`, and logs each
   tx's AdmitOutcome.
4. Cross-check: for every captured tx bytes, ALSO route the same
   bytes through the S4 N2N bridge (`ingest_n2n_events` with a
   single synthetic peer). Assert byte-identical
   `(MempoolState, AdmitOutcome)` to the N2C result.
5. Commit `docs/clusters/PHASE4-N-E/CE-N-E-7_<YYYY-MM-DD>.log` with
   handshake details, N count, agreement assertion result.

Closure: log committed AND `[agreement] divergences: 0`.
```

### 5. Cluster doc TCB partition update

Already updated in S4 for the broader "RED = operator-action live
sessions" framing. S5 extends the RED row to mention
`CE-N-E-7_PROCEDURE.md` and the N2C operator-action artifact.

### 6. Registry — no changes

S5 produces mechanical GREEN evidence at a new wire surface. The
cross-bridge agreement test
(`n2n_and_n2c_bridges_produce_identical_outcomes`) is the new
load-bearing N-E-N7 / N-E-8 evidence at the wire-event layer, but
the underlying invariants (DC-MEM-01, DC-MEM-03, DC-MEM-04) are
already in the registry. No new rule.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build -p ade_core_interop` green.
- **AC-2** — `cargo test -p ade_core_interop --test local_tx_submission_ingress` green (6 tests).
- **AC-3** — `cargo test -p ade_core_interop --test tx_submission_ingress` green (S4 unchanged).
- **AC-4** — `cargo test -p ade_ledger` and `cargo test -p ade_testkit` unchanged.
- **AC-5** — `bash ci/ci_check_mempool_ingress_closure.sh` PASS;
  `bash ci/ci_check_mempool_ingress_replay.sh` PASS;
  `bash ci/ci_check_constitution_coverage.sh` PASS (175 entries).
- **AC-6** — `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` exists.

---

## Hard Prohibitions

- No new dependency edge from any BLUE crate to `ade_core_interop`.
- The N2C adapter must produce `IngressSource::N2C`; the orchestrator
  must build `PeerSubmissionQueue { source: IngressSource::N2C, ... }`.
- The N2C bridge must call `mempool_ingress` (via `replay_ingress_trace`),
  never `admit` directly.
- No new registry rule.
- Adapter must NOT decode `tx_bytes` — it wraps them.
- The actual async UDS socket loop is operator-action; do not attempt
  to ship a half-baked one as automated code.

---

## Explicit Non-Goals

- Full async N2C UDS server binary.
- Local-state-query / local-tx-monitor wiring (different mini-protocols).
- Multi-tenant cardano-cli auth, ACL, rate-limiting (Tier-5).
- Mempool bounds / shedding (Tier-5 cluster).
- Outbound propagation (separate cluster).
