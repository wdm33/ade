# S-B5 — Op-cert counter monotonicity

## Slice Header

**Slice Name**: Op-cert counter monotonicity
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**: substrate for header validation (S-B7); no CE closes here.

**Slice Dependencies**: S-B2 (`OpCertCounterMap`, `OpCertCounterError`, `PraosChainDepState`).

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified. The state already has
`OpCertCounterMap::upsert_strict` from S-B2; this slice **wraps** it
in a typed `OpCertCounterCheck` transition that returns a new
`PraosChainDepState`, plus tests that prove monotonicity holds across
`(pool, kes_period)` windows.

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer per CLAUDE.md.

---

## 4. Intent

Make it impossible for any header to be applied if its op-cert
issue counter regresses relative to the highest observed counter for
the same `(pool_id, kes_period)`. Regression is the operator's
mechanism to revoke a hot key; the chain must enforce strict
monotonicity per kes-period window so a revoked key can never resign
a block that any other node would accept.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/op_cert.rs` (NEW)
- `crates/ade_core/src/consensus/mod.rs` (extend — re-exports)
- `crates/ade_core/tests/op_cert_counter_corpus.rs` (NEW)
- `corpus/consensus/op_cert/regression_case.json` (NEW)
- `corpus/consensus/op_cert/normal_progression.json` (NEW)
- `crates/ade_testkit/src/consensus/corpus.rs` (extend with op_cert
  path helper)

**State machines affected**: `OpCertCounterCheck` transition on
`PraosChainDepState`.

**Persistence impact**: none (canonical encoding from S-B2).

**Network-visible impact**: none.

**Out-of-scope**:
- Header signature verification under the op-cert's KES key — that
  is a separate concern (KES verification lives in `ade_crypto::kes`
  and is consumed by S-B7 header-validate).
- VRF (S-B3) and nonce (S-B4).
- Op-cert binding to pool stake (this is a *counter* check; the
  pool-identity binding is part of header-validate in S-B7).

---

## 6. Execution Boundary

**BLUE**: `ade_core::consensus::op_cert`.
**GREEN**: testkit corpus helper.
**RED**: none.

---

## 7. Invariants Preserved

- All previous slice tests pass.
- `PraosChainDepState` shape unchanged.
- `OpCertCounterMap::upsert_strict` semantics unchanged.

---

## 8. Invariants Strengthened or Introduced

- **`DC-CONS-10` (NEW)** — Op-cert counter is monotonic per
  `(pool, kes_period)`; introduced into the registry by this slice.
- **`DC-CONS-04` strengthened — status flip eligible**: with this
  slice the op-cert subset of `DC-CONS-04` (state-behavior) attaches.
  S-B7 will perform the final `declared` → `enforced` flip once
  header-validate composes nonce + op-cert + VRF in a single
  transition. This slice leaves `DC-CONS-04` at `declared`.

---

## 9. Design Summary

```rust
// op_cert.rs

use ade_types::Hash28;
use crate::consensus::praos_state::{PraosChainDepState, OpCertCounterMap};
use crate::consensus::errors::OpCertCounterError;

/// One op-cert observation from a header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpCertObservation {
    pub pool:       Hash28,   // pool VRF cold key hash
    pub kes_period: u64,
    pub counter:    u64,
}

/// Apply an op-cert observation to the chain-dep state.
///
/// Returns a new state with the counter recorded if and only if
/// `counter > existing` (or no existing entry for the (pool,
/// kes_period) key). Regression — including `counter == existing`
/// — returns OpCertCounterError::Regression.
///
/// Pure function: same (state, observation) → same result.
pub fn apply_op_cert(
    state:        &PraosChainDepState,
    observation:  &OpCertObservation,
) -> Result<PraosChainDepState, OpCertCounterError>;
```

Implementation: clone the state's `op_cert_counters`, call
`upsert_strict`, return a new state with the updated map. Other
fields are passed through unchanged.

Note: `kes_period` is `u64`, matching ouroboros-consensus
`OCertIssueNumber` and `KESPeriod` conventions. The (pool,
kes_period) tuple keys the map so the same pool can have independent
counter histories across KES rotation windows.

---

## 10. Changes Introduced

### Types
- New: `OpCertObservation`.

### State Transitions
- New: `apply_op_cert`.

### Persistence
- None.

---

## 11. Replay, Crash, and Epoch Validation

### Corpus

`corpus/consensus/op_cert/normal_progression.json`:

```jsonc
{
  "scenario": "two_pools_three_observations_each_strictly_increasing",
  "observations": [
    { "pool_hex": "<28 hex bytes>", "kes_period": 100, "counter": 0 },
    { "pool_hex": "<28 hex bytes>", "kes_period": 100, "counter": 1 },
    { "pool_hex": "<28 hex bytes>", "kes_period": 100, "counter": 5 },
    { "pool_hex": "<28 hex bytes other pool>", "kes_period": 100, "counter": 0 },
    { "pool_hex": "<28 hex bytes other pool>", "kes_period": 100, "counter": 2 },
    { "pool_hex": "<28 hex bytes other pool>", "kes_period": 101, "counter": 0 }
  ],
  "expected_final_map_size": 3,
  "expected_final_counters": [
    { "pool_hex": "<bytes>", "kes_period": 100, "counter": 5 },
    { "pool_hex": "<bytes other>", "kes_period": 100, "counter": 2 },
    { "pool_hex": "<bytes other>", "kes_period": 101, "counter": 0 }
  ]
}
```

`corpus/consensus/op_cert/regression_case.json`:

```jsonc
{
  "scenario": "regression_after_two_progressions",
  "observations": [
    { "pool_hex": "<bytes>", "kes_period": 200, "counter": 0 },
    { "pool_hex": "<bytes>", "kes_period": 200, "counter": 5 }
  ],
  "regression_observation": {
    "pool_hex": "<same bytes>", "kes_period": 200, "counter": 4
  },
  "expected_error": { "kind": "Regression", "existing": 5, "attempted": 4 }
}
```

### Tests

- `crates/ade_core/tests/op_cert_counter_corpus.rs`:
  - `normal_progression_records_highest_counter_per_window`
  - `regression_after_progression_rejected_with_typed_error`
  - `op_cert_replay_is_deterministic`

- Unit tests in `op_cert.rs`:
  - `apply_op_cert_inserts_first_observation`
  - `apply_op_cert_advances_existing_strictly`
  - `apply_op_cert_rejects_equal_counter` — equal counter is
    regression (strictly increasing)
  - `apply_op_cert_rejects_lower_counter`
  - `apply_op_cert_independent_kes_periods_dont_collide`
  - `apply_op_cert_independent_pools_dont_collide`
  - `apply_op_cert_does_not_touch_nonces`
  - `apply_op_cert_does_not_touch_last_slot_or_block_no`

### Replay impact
- Pure function.

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core` PASS
- [ ] `cargo test -p ade_core --lib consensus::op_cert` PASS
- [ ] `cargo test -p ade_core --test op_cert_counter_corpus` PASS
- [ ] `cargo clippy -p ade_core --all-targets -- -D warnings` PASS
- [ ] No `HashMap` / `HashSet`
- [ ] Equality regression rejected (counter == existing → error)

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| Counter equal-or-lower | `OpCertCounterError::Regression { existing, attempted }` | yes |

---

## 14. Hard Prohibitions

### Inherited (from cluster.md)
- BLUE receiving `&ChainDb`, `&Mux`, parsing genesis text
- Wall-clock reads
- `HashMap` / `HashSet`
- Floating-point
- TODO/placeholder error variants
- `async fn` / `tokio` in BLUE

### Slice-specific
- No `unwrap` / `expect` / `panic`.
- No exposing the inner `BTreeMap` of `OpCertCounterMap` as a
  mutable reference — every write goes through `upsert_strict` or
  `insert_unchecked` (the latter is `pub(crate)` and used only for
  canonical decode).

---

## 15. Explicit Non-Goals

- Do NOT verify KES signatures.
- Do NOT bind op-certs to pool stake.
- Do NOT change `OpCertCounterMap`'s public API beyond an
  `apply_op_cert` wrapper.

---

## 16. Completion Checklist

- [ ] Transition is pure
- [ ] Reject is deterministic
- [ ] Corpus pinned

---

## 17. Review Notes

- `apply_op_cert` allocates a clone of the state. For replay-heavy
  workloads this might be optimised later; do not optimise here.

---

## 18. Authority Reminder

Registry > slice doc.
