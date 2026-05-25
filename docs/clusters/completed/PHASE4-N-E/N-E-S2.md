# Invariant Slice — PHASE4-N-E S2

## Slice Header
**Slice Name:** Ingress-replay harness + B-track adversarial-corpus reuse
**Cluster:** PHASE4-N-E
**Status:** Proposed
**CEs addressed:** CE-N-E-2 (agreement), CE-N-E-3 (corpus half), CE-N-E-5 (adversarial-reuse no-false-accept), CE-N-E-4 (single-peer half — multi-peer half lands in S3)
**Dependencies:** S1 (IngressEvent + mempool_ingress + DC-MEM-03)

---

## Intent

Prove mechanically that the new `mempool_ingress` chokepoint is a faithful
gate by replaying the existing B-track adversarial corpus through it and
asserting **byte-identical results to direct `admit`** for every case
(valid + every `SyntheticMutation`). Strengthen `DC-MEM-01` with the
new ingress-replay test surface. Append `DC-MEM-04` (replay invariant).

The cluster's no-false-accept obligation reuses the B1–B5 + B3F corpus
verbatim; only the `IngressEvent` envelope is added (per OQ-3).

---

## The change

### 1. New GREEN module `crates/ade_testkit/src/mempool/`

- `mempool/mod.rs` (new) — re-exports the harness items.
- `mempool/ingress_replay.rs` (new) — the harness:

```rust
// Envelope helpers
pub fn wrap_as_ingress(source: IngressSource, tx_cbor: Vec<u8>) -> IngressEvent;
pub fn b_track_corpus_as_ingress(source: IngressSource)
    -> Vec<(IngressEvent, LedgerState, ExpectedOutcome)>;

// Closed expected-outcome variant for B-track cases
pub enum ExpectedOutcome { Admit, Reject(TxRejectClass) }

// Folded replay (single-step BLUE bridge per OQ-6)
pub fn replay_ingress_trace(
    base: LedgerState,
    events: &[IngressEvent],
) -> (MempoolState, Vec<AdmitOutcome>);
```

The harness fold is a literal `events.iter().fold(...)` over the
`mempool_ingress` BLUE bridge — no side state, no batching.

### 2. New integration tests `crates/ade_testkit/tests/mempool_ingress_replay.rs`

- `ingress_admit_equals_direct_admit_on_b_track_corpus` — for every
  `SyntheticMutation` and the valid case, `mempool_ingress(state, event)`
  is byte-identical to `admit(state, event.tx_bytes())`.
- `b_track_adversarial_rejections_preserved_through_ingress` — every
  adversarial mutation's `expected_class()` matches the `Rejected.class`
  observed via `mempool_ingress`.
- `ingress_trace_replay_byte_identical` — replay the same ordered ingress
  trace against the same base state twice; the two
  `(MempoolState, Vec<AdmitOutcome>)` results are byte-identical.
- `dependent_pair_through_ingress_admits_b_after_a` — the existing
  `build_dependent_pair()` admit-A-then-B sequence routed through
  `mempool_ingress` admits B against A's accumulating state (intra-mempool
  dependency invariant N-E-6).

### 3. Registry updates (same commit)

Append:

```toml
[[rules]]
id = "DC-MEM-04"
tier = "derived"
statement = "Replaying the same ordered ingress trace against the same base ledger state produces a byte-identical sequence of (MempoolState, AdmitOutcome) pairs."
source = "Project constitution §3, T-DET-01, DC-MEM-01"
cross_ref = ["DC-MEM-01"]
code_locus = "crates/ade_testkit/src/mempool/ingress_replay.rs; crates/ade_ledger/src/mempool/ingress.rs"
tests = ["ingress_admit_equals_direct_admit_on_b_track_corpus", "b_track_adversarial_rejections_preserved_through_ingress", "ingress_trace_replay_byte_identical", "dependent_pair_through_ingress_admits_b_after_a"]
ci_script = ""
status = "enforced"
introduced_in = "PHASE4-N-E"
strengthened_in = []
```

Modify `DC-MEM-01` (strengthening):

- `strengthened_in += "PHASE4-N-E"`
- `code_locus += "; crates/ade_ledger/src/mempool/ingress.rs; crates/ade_testkit/src/mempool/ingress_replay.rs"`
- `tests += ["ingress_admit_equals_direct_admit_on_b_track_corpus", "b_track_adversarial_rejections_preserved_through_ingress", "dependent_pair_through_ingress_admits_b_after_a"]`
- `cross_ref += "DC-MEM-04"` (and DC-MEM-04 has DC-MEM-01 — bidirectional)

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_testkit --test mempool_ingress_replay` green.
- **AC-3** — `cargo test -p ade_ledger` green (no S1 regression).
- **AC-4** — `bash ci/ci_check_mempool_ingress_closure.sh` returns `PASS`
  (S1 gate still green; harness lives in `ade_testkit` which is exempt as
  a test crate).
- **AC-5** — `bash ci/ci_check_constitution_coverage.sh` returns `PASS`
  (175 entries).
- **AC-6** — registry count: 174 → 175.

---

## Hard Prohibitions

- No new adversarial tx bytes — reuse the existing B-track corpus verbatim.
- No alteration of corpus byte sequences during wrapping (the envelope is
  metadata).
- No batching or out-of-order interleaving in `replay_ingress_trace` —
  it's a literal `fold` over `mempool_ingress` (single-step per OQ-6).
- No `unsafe`, no async, no `HashMap`/`HashSet` in the harness.
- No registry edits beyond DC-MEM-04 append + DC-MEM-01 strengthening.
- The harness must NOT call `admit` directly (it must go through
  `mempool_ingress`), except in the agreement-check test that compares
  both sides.

---

## Explicit Non-Goals

- Multi-peer canonicalizer (S3, CE-N-E-4 multi-peer half).
- N2N session driver (S4, CE-N-E-6).
- N2C session driver (S5, CE-N-E-7).
- Touching policy/eviction / DC-MEM-02 / CN-MEM-01/03.
- New CI script (DC-MEM-04 is enforced by the harness tests alone;
  empty `ci_script` is correct).
