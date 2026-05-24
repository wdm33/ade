# Invariant Slice — PHASE4-N-E S3

## Slice Header
**Slice Name:** Per-peer canonicalizer (deterministic multi-peer ordering)
**Cluster:** PHASE4-N-E
**Status:** Proposed
**CEs addressed:** CE-N-E-4 (multi-peer half)
**Dependencies:** S1 (IngressEvent / IngressSource); S2 (replay_ingress_trace harness — for the equivalence test)

---

## Intent

Provide a GREEN, pure, deterministic ordering function that takes per-peer
FIFO submission queues and produces a single canonical `IngressEvent` stream.
Two distinct concurrent peer interleavings of the same per-peer queues MUST
produce the same `IngressEvent` sequence — which guarantees that the
downstream BLUE replay through `mempool_ingress` is byte-identical regardless
of network-level race ordering.

This closes the multi-peer half of CE-N-E-4 (replay equivalence). The
single-peer half is already proven by S2's `ingress_trace_replay_byte_identical`.

---

## The change

### 1. New GREEN module `crates/ade_ledger/src/mempool/canonicalize.rs`

```rust
/// Opaque peer identifier (e.g., a Blake2b-224 of the peer's public
/// key + a port-disambiguator). Ordering is byte-lexicographic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub Vec<u8>);

/// One peer's submission queue: an ordered list of tx CBOR byte strings
/// in arrival order, paired with the source variant the peer carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerSubmissionQueue {
    pub peer: PeerId,
    pub source: IngressSource,
    pub txs: Vec<Vec<u8>>,
}

/// Deterministic round-robin canonicalization: peers are visited in
/// `PeerId` byte-lex order; each round emits one tx from every peer that
/// still has one, in `PeerId` order. Repeats until every queue is drained.
///
/// Pure: `canonicalize_peer_streams(qs) == canonicalize_peer_streams(qs)`
/// for any input. Independent of `qs.iter()` element order — the function
/// sorts internally on `peer`.
pub fn canonicalize_peer_streams(
    queues: &[PeerSubmissionQueue],
) -> Vec<IngressEvent>;
```

The ordering policy is **round-robin by sorted PeerId**:

- Round 0: take queue[0].txs[0] (sorted by peer), queue[1].txs[0], …
- Round 1: take queue[0].txs[1], queue[1].txs[1], …
- Continue until every queue is drained.

This is fair across peers AND deterministic — no single peer can starve
others by spamming. The exact policy is the cluster's load-bearing
guarantee; any future change to it is a SEAMS-level decision.

### 2. Re-exports in `crates/ade_ledger/src/mempool/mod.rs`

```rust
pub mod canonicalize;
pub use canonicalize::{canonicalize_peer_streams, PeerId, PeerSubmissionQueue};
```

### 3. Unit tests (inline `#[cfg(test)] mod tests`)

- `single_peer_canonicalizes_to_submission_order` — one peer's queue passes
  through verbatim in arrival order.
- `multi_peer_round_robin_by_sorted_peer_id` — three peers with three txs
  each → 9 events in round-robin order, peers sorted lex.
- `unsorted_input_canonicalizes_identically_to_sorted_input` — passing
  the same queues in shuffled order produces an identical output sequence.
- `empty_queue_for_a_peer_skipped` — a peer with zero txs is skipped (no
  event for that peer).
- `peer_with_longest_queue_finishes_alone` — when other peers drain, the
  remaining peer's txs are emitted in submission order with no
  reordering.
- `tx_bytes_preserved_verbatim` — `IngressEvent.tx_bytes()` matches the
  original byte string exactly.
- `source_propagated_from_queue` — each event's `source` matches its
  queue's `source`.

### 4. Cross-check integration test (uses S2's `replay_ingress_trace`)

`crates/ade_testkit/tests/mempool_ingress_canonicalize.rs`:

- `two_interleavings_replay_byte_identical` — take the same multi-peer
  submission set, canonicalize twice from shuffled input orders; replay
  both canonical sequences against the same base; assert byte-identical
  `(MempoolState, Vec<AdmitOutcome>)`. This is the load-bearing CE-N-E-4
  multi-peer evidence.

### 5. Registry update (same commit)

Extend `DC-MEM-04`:

- `code_locus += "; crates/ade_ledger/src/mempool/canonicalize.rs"`
- `tests += ["multi_peer_round_robin_by_sorted_peer_id", "unsorted_input_canonicalizes_identically_to_sorted_input", "two_interleavings_replay_byte_identical"]`

No new registry rule — `canonicalize_peer_streams` is the GREEN ordering
function whose determinism IS what DC-MEM-04 asserts at the multi-peer
boundary. No new CI script — the existing
`ci_check_mempool_ingress_replay.sh` is extended to verify the new
function exists and is sync/pure.

### 6. Extend `ci/ci_check_mempool_ingress_replay.sh`

Add a structural guard:

5. `canonicalize.rs` exists and defines `canonicalize_peer_streams`,
   `PeerId`, `PeerSubmissionQueue`.
6. `canonicalize_peer_streams` body contains no `HashMap`, `HashSet`,
   `tokio`, `std::sync::Mutex`, `RwLock`, `async`, RNG, or wall-clock
   references — strictly sync + deterministic.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_ledger --lib mempool::canonicalize` green
  (all 7 unit tests pass).
- **AC-3** — `cargo test -p ade_testkit --test mempool_ingress_canonicalize` green.
- **AC-4** — `bash ci/ci_check_mempool_ingress_closure.sh` returns `PASS`
  (S1 gate unaffected).
- **AC-5** — `bash ci/ci_check_mempool_ingress_replay.sh` returns `PASS`
  (extended gate covers canonicalize.rs).
- **AC-6** — `bash ci/ci_check_constitution_coverage.sh` returns `PASS`
  (175 entries — no new rule).
- **AC-7** — registry rule count stays at 175 (DC-MEM-04 strengthened
  in-place, no new entry).

---

## Hard Prohibitions

- No `HashMap` / `HashSet` / `tokio` / async / RNG / wall-clock in
  `canonicalize.rs`.
- No `BTreeMap`-of-`Cell` or interior-mutability tricks — the function
  is a pure transform from `&[PeerSubmissionQueue]` to `Vec<IngressEvent>`.
- No policy that depends on peer arrival time, queue length, or anything
  outside the typed inputs.
- No `unsafe`.
- No registry edits beyond DC-MEM-04 strengthening (no new rule).
- The canonicalizer MUST NOT decode tx bytes — it shuffles them only.

---

## Explicit Non-Goals

- Per-peer fairness/priority policy beyond simple round-robin (Tier 5
  cluster, separate).
- Peer reputation, scoring, eviction (Tier 5).
- Concurrent multi-thread peer reading (the canonicalizer is a pure
  function over already-collected queues; the concurrent collection
  layer is RED, lives in S4/S5's session loops).
- N2N / N2C session drivers (S4, S5).
