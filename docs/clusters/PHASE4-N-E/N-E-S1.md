# Invariant Slice — PHASE4-N-E S1

## Slice Header
**Slice Name:** `IngressEvent` + `mempool_ingress` + closure CI gate
**Cluster:** PHASE4-N-E
**Status:** Proposed
**CEs addressed:** CE-N-E-1 (closure), CE-N-E-3 (type-level half)
**Dependencies:** none (BLUE `admit` already exists from PHASE4-B2 / CE-B2-5)

---

## Intent

Make the closed `IngressEvent` the only typed entry into `mempool::admit` from
non-test code, and prove mechanically (CI + type system) that:

- mempool accumulating state cannot be mutated outside `admit`, and
- `admit` cannot be called from production (non-test) code outside `mempool_ingress`.

The `IngressEvent.source` variant is metadata only — it MUST NOT change the
admission verdict.

---

## The change (atomic; compile green as one unit)

### 1. New module `crates/ade_ledger/src/mempool/ingress.rs` (BLUE)

```rust
// closed source discriminant — N2N peer or N2C local IPC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IngressSource {
    N2N,
    N2C,
}

// canonical typed entry into the mempool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressEvent {
    pub source: IngressSource,
    pub tx_bytes: Vec<u8>,  // PreservedCbor end-to-end: bytes flow verbatim
}

impl IngressEvent {
    pub fn new(source: IngressSource, tx_bytes: Vec<u8>) -> Self { ... }
    pub fn source(&self) -> IngressSource { ... }
    pub fn tx_bytes(&self) -> &[u8] { ... }
}

/// The single BLUE chokepoint from wire ingress into the mempool. A pure
/// function of `(mempool, event)`; the `event.source` is recorded for
/// evidence/policy/replay but MUST NOT affect the verdict.
pub fn mempool_ingress(
    mempool: &MempoolState,
    event: &IngressEvent,
) -> (MempoolState, AdmitOutcome) {
    admit(mempool, event.tx_bytes())
}
```

### 2. Re-exports in `crates/ade_ledger/src/mempool/mod.rs`

```rust
pub mod ingress;
pub use ingress::{mempool_ingress, IngressEvent, IngressSource};
```

### 3. Unit tests (co-located in `ingress.rs` `#[cfg(test)] mod tests`)

- `ingress_admits_valid_tx_via_n2n` — N2N event with valid bytes → `Admitted`,
  accumulating evolves.
- `ingress_admits_valid_tx_via_n2c` — same shape over N2C.
- `ingress_rejects_invalid_tx_no_false_accept` — adversarial mutations (forged
  witness / value imbalance) → `Rejected`, mempool unchanged.
- `ingress_source_does_not_change_verdict` — for the same tx_bytes, N2N and
  N2C produce byte-identical `(MempoolState, AdmitOutcome)`. Cover both a
  valid case and at least two adversarial cases.
- `ingress_preserves_tx_bytes_verbatim` — `event.tx_bytes()` is byte-equal to
  what was passed at construction time; no normalization.

### 4. New CI gate `ci/ci_check_mempool_ingress_closure.sh` (closure proof)

Mechanical guards:

1. **`ingress.rs` exists and exports the three names.** `IngressEvent`,
   `IngressSource`, `mempool_ingress` must be defined and re-exported from
   `mempool/mod.rs`.
2. **`IngressSource` is a closed enum** (`pub enum IngressSource` with exactly
   the two variants `N2N` and `N2C`). No `#[non_exhaustive]`.
3. **`MempoolState.accumulating` is mutated only in `admit.rs`.** Grep across
   `crates/ade_ledger/src/` for `accumulating` write patterns; the only
   sanctioned production site is the field assignment inside `admit`.
4. **`admit(` is called only from sanctioned sites.** Grep all `crates/*/src/`
   for `admit(`; flag any call outside
   `crates/ade_ledger/src/mempool/ingress.rs` and outside test files (test
   files live at `crates/*/tests/`). The slice does not yet have any other
   production callers; future RED drivers (S4, S5) MUST go through
   `mempool_ingress`, not `admit` directly.
5. **`mempool_ingress` does not branch on `event.source`.** Grep
   `crates/ade_ledger/src/mempool/ingress.rs` for `match.*source` and
   `event.source` — the body of `mempool_ingress` may *record* source for
   evidence (future) but MUST NOT use it in the verdict path. For this slice
   the body is a single-line pass-through; the gate enforces "no `match`
   on `IngressSource` inside `mempool_ingress`".

### 5. Registry append (same commit)

Append to `docs/ade-invariant-registry.toml`:

```toml
[[rules]]
id = "DC-MEM-03"
tier = "derived"
statement = "Tx ingress reduces to a closed IngressEvent before BLUE mempool admission; the source variant is evidence/policy/replay metadata only and MUST NOT change the validity verdict."
source = "Project constitution §3, T-CORE-01 (closed semantic surfaces), DC-MEM-01"
cross_ref = ["DC-MEM-01", "CN-MEM-04", "T-CORE-01"]
code_locus = "crates/ade_ledger/src/mempool/ingress.rs (IngressEvent, IngressSource, mempool_ingress)"
tests = ["ingress_admits_valid_tx_via_n2n", "ingress_admits_valid_tx_via_n2c", "ingress_rejects_invalid_tx_no_false_accept", "ingress_source_does_not_change_verdict", "ingress_preserves_tx_bytes_verbatim"]
ci_script = "ci/ci_check_mempool_ingress_closure.sh"
status = "enforced"
introduced_in = "PHASE4-N-E"
strengthened_in = []
```

`DC-MEM-01` is NOT modified in this slice; its `strengthened_in += "PHASE4-N-E"`
lands in S2 once the ingress-replay tests exist as concrete test names.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_ledger mempool::ingress` green (all 5 unit tests pass).
- **AC-3** — `cargo test -p ade_ledger` green (no pre-existing mempool tests regress).
- **AC-4** — `bash ci/ci_check_mempool_ingress_closure.sh` returns `PASS` (all 5 guards).
- **AC-5** — `bash ci/ci_check_constitution_coverage.sh` returns `PASS` (DC-MEM-03 picked up).
- **AC-6** — registry rule count goes from 173 → 174; `grep -c '^\[\[rules\]\]' docs/ade-invariant-registry.toml = 174`.

---

## Hard Prohibitions

- No `#[non_exhaustive]` on `IngressSource` (it is a closed sum).
- No branching on `event.source` inside `mempool_ingress` (verdict is a function of `(state, tx_bytes)` alone).
- No mutation of `MempoolState.accumulating` outside `admit.rs`.
- No production caller of `admit(` outside `mempool_ingress` (test callers OK).
- No normalization, decoding, or re-encoding of `tx_bytes` in `ingress.rs`.
- No HashMap / HashSet / wall-clock / RNG / float in `ingress.rs`.
- No async fn in `ingress.rs` (BLUE crate, DC-CORE-01).
- No new dependency edge from `ade_ledger` to `ade_runtime` or any RED crate.

---

## Explicit Non-Goals

- Adversarial corpus reuse + ingress-replay harness — that's S2 (CE-N-E-2, CE-N-E-5).
- Multi-peer canonicalizer — that's S3 (CE-N-E-4 multi-peer half).
- N2N session driver — that's S4 (CE-N-E-6).
- N2C session driver — that's S5 (CE-N-E-7).
- Pinning `DC-MEM-01.strengthened_in += "PHASE4-N-E"` (lands in S2).
- Touching `mempool/policy.rs` or `DC-MEM-02`.
- Touching any other DC-MEM-* or CN-MEM-* rule.
