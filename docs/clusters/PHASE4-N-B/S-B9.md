# S-B9 — Rollback authority (CE-N-B-2 close)

## Slice Header

**Slice Name**: Rollback authority + k-bound + immutable-tip refusal
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**:
- [x] **CE-N-B-2** — Rollback to k-deep produces identical post-
  rollback state as oracle for a curated rollback corpus;
  `ExceededRollback` (k = 2160 mainnet) and `ForkBeforeImmutableTip`
  are byte-identical with oracle.

**Slice Dependencies**: S-B1..S-B8.

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified.

The rollback transition is **independent** of fork-choice — it
operates on `ChainSelectorState` and a `RollBackRequest`, returning a
rolled-back state or a structured reject. It does not query a
ChainDb; it does not fetch headers. The caller (chain-selector
orchestrator, S-B10) supplies an ordered list of *checkpoint headers*
that the rollback transition uses to reconstruct the rolled-back
tip.

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer.

---

## 4. Intent

Make it impossible for Ade to accept a rollback that:
1. Would cross the immutable tip (k-deep is final per Cardano
   security parameter), OR
2. Exceeds the security parameter `k` blocks in depth.

And make it deterministic: `rollback(state, depth)` produces a
state byte-identical to truncated replay from the nearest
checkpoint.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/rollback.rs` (NEW)
- `crates/ade_core/src/consensus/mod.rs` (extend — re-exports)
- `crates/ade_core/tests/rollback_corpus.rs` (NEW — CE-N-B-2 close)
- `corpus/consensus/rollback/within_k.json` (NEW)
- `corpus/consensus/rollback/exceeds_k.json` (NEW)
- `corpus/consensus/rollback/before_immutable.json` (NEW)
- `crates/ade_testkit/src/consensus/corpus.rs` (extend — rollback
  path helper)

**State machines affected**: `RollBack` transition on
`ChainSelectorState`.

**Persistence impact**: none (state is owned by N-D).

**Network-visible impact**: none.

**Out-of-scope**:
- Header re-validation after rollback (S-B7 handles header
  validation; this slice composes its output)
- Live chain-sync rollback events from N-A (S-B10)
- ChainDb rollback IO (out of N-B)

---

## 6. Execution Boundary

**BLUE**: `ade_core::consensus::rollback`.
**GREEN**: none.
**RED**: none.

---

## 7. Invariants Preserved

- All S-B1..S-B8 tests pass.
- `PraosChainDepState` and `ChainSelectorState` shapes unchanged.

---

## 8. Invariants Strengthened or Introduced

- **`DC-CONS-05` (NEW)** — Rollback bounded by k blocks.
- **`DC-CONS-06` (NEW)** — Rollback = truncated replay; immutable
  tip is final.
- **`DC-CONSENSUS-01` strengthened** — best-chain authority extends
  to rollback acceptance.

---

## 9. Design Summary

### Inputs

```rust
// rollback.rs

use ade_types::BlockNo;
use crate::consensus::candidate::ChainSelectorState;
use crate::consensus::events::{BlockDistance, ChainEvent, ChainSelectionReject, Point, SecurityParam};
use crate::consensus::header_summary::ValidatedHeaderSummary;
use crate::consensus::praos_state::PraosChainDepState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollBackRequest {
    /// The point to roll back to. Must be an ancestor of
    /// `state.current_tip`.
    pub to_point:     Point,
    pub to_block_no:  BlockNo,
    /// How many blocks deep this rollback is from `state.current_tip`.
    /// Caller computes this from its chain history; the transition
    /// uses it for the k-bound check and surfaces it in the reject
    /// reason.
    pub depth:        BlockDistance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollBackApplied {
    pub new_state:     ChainSelectorState,
    pub new_chain_dep: PraosChainDepState,
    pub event:         ChainEvent,
}
```

### Transition

```rust
/// Pure rollback transition.
///
/// Inputs:
///   - state:         current ChainSelectorState (current_tip, immutable_tip, k)
///   - chain_dep:     current PraosChainDepState (for the chain-dep
///                    state at the rolled-back point)
///   - rolled_back_chain_dep:  ChainDep state at the rolled-back
///                    point, supplied by the caller (N-D snapshot
///                    or replay)
///   - rolled_back_tiebreaker: TiebreakerView at the rolled-back
///                    point, supplied by the caller
///   - request:       RollBackRequest
///
/// Returns RollBackApplied on success, or ChainEvent::Rejected with
/// the appropriate ChainSelectionReject variant on failure (the
/// transition does NOT return Err for rejects — it returns Ok with
/// a ChainEvent::Rejected, so the caller can record both the event
/// and the unchanged state).
pub fn apply_rollback(
    state:                    &ChainSelectorState,
    chain_dep:                &PraosChainDepState,
    rolled_back_chain_dep:    &PraosChainDepState,
    rolled_back_tiebreaker:   &crate::consensus::candidate::TiebreakerView,
    request:                  &RollBackRequest,
) -> RollBackApplied;
```

### Algorithm

1. If `request.to_block_no < state.immutable_tip_block_no` →
   return state unchanged + `ChainEvent::Rejected { reason:
   ChainSelectionReject::ForkBeforeImmutableTip { immutable_tip,
   candidate_intersection: request.to_point, rollback_depth:
   request.depth, security_param: state.security_param } }`.
2. If `request.depth.0 > state.security_param.0` → return state
   unchanged + `ChainEvent::Rejected { reason:
   ChainSelectionReject::ExceededRollback { requested: request.depth,
   max: state.security_param } }`.
3. Otherwise: apply the rollback. Construct
   `new_state = ChainSelectorState { current_tip: request.to_point,
   current_tip_block_no: request.to_block_no, current_tiebreaker:
   *rolled_back_tiebreaker, immutable_tip: state.immutable_tip,
   immutable_tip_block_no: state.immutable_tip_block_no,
   security_param: state.security_param }`. The chain-dep state at
   the rolled-back point is the supplied `rolled_back_chain_dep`.
   Emit `ChainEvent::RolledBack { to_point: request.to_point, depth:
   request.depth }`.

> **Replay equivalence**: this slice asserts that rolling back to
> point P and then replaying the same headers (via S-B7
> `validate_and_apply_header`) reaches a state byte-identical to
> never having had the divergent headers in the first place. The
> test `rollback_equivalent_to_truncated_replay` verifies this:
> apply 5 headers from genesis, snapshot the state at block 3,
> apply 2 more headers, roll back to block 3 (depth = 2), assert
> state == snapshot.

---

## 10. Changes Introduced

### Types
- New: `RollBackRequest`, `RollBackApplied`.

### State Transitions
- New: `apply_rollback`.

### Persistence
- None.

---

## 11. Replay, Crash, and Epoch Validation

### Corpus

`corpus/consensus/rollback/within_k.json`:

```jsonc
{
  "scenario": "rollback_within_k_succeeds",
  "state": {
    "current_tip": { "slot": 1000, "hash_hex": "..." },
    "current_tip_block_no": 500,
    "immutable_tip": { "slot": 100, "hash_hex": "..." },
    "immutable_tip_block_no": 50,
    "security_param": 100
  },
  "request": {
    "to_point": { "slot": 950, "hash_hex": "..." },
    "to_block_no": 495,
    "depth": 5
  },
  "expected_event": { "kind": "RolledBack", "to_block_no": 495, "depth": 5 }
}
```

`corpus/consensus/rollback/exceeds_k.json`:

```jsonc
{
  "scenario": "rollback_exceeds_k_rejected",
  "state": { ..., "security_param": 100 },
  "request": { "depth": 200, "to_block_no": 300 },
  "expected_event": { "kind": "Rejected", "reason_kind": "ExceededRollback", "requested": 200, "max": 100 }
}
```

`corpus/consensus/rollback/before_immutable.json`:

```jsonc
{
  "scenario": "rollback_before_immutable_rejected",
  "state": { ..., "immutable_tip_block_no": 50 },
  "request": { "to_block_no": 30, "depth": 470 },
  "expected_event": { "kind": "Rejected", "reason_kind": "ForkBeforeImmutableTip" }
}
```

### Tests

- `crates/ade_core/tests/rollback_corpus.rs`:
  - `rollback_within_k_succeeds`
  - `rollback_exceeding_k_rejected_with_typed_reason`
  - `rollback_before_immutable_tip_rejected`
  - `rollback_event_bytes_are_stable` (encode → hex pinned)
  - `rollback_equivalent_to_truncated_replay` — apply N headers from
    genesis, snapshot at block K, apply M more, roll back depth=M;
    assert state byte-identical to snapshot.
  - `rollback_is_deterministic` — same inputs twice → same output.

- Unit tests in `rollback.rs`:
  - `rollback_preserves_immutable_tip`
  - `rollback_preserves_security_param`
  - `rollback_with_zero_depth_is_noop`
  - `rollback_to_equal_block_no_as_immutable_succeeds`
  - `rollback_to_one_below_immutable_rejected`

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core` PASS
- [ ] `cargo test -p ade_core --lib consensus::rollback` PASS
- [ ] `cargo test -p ade_core --test rollback_corpus` PASS — **CE-N-B-2
      close**
- [ ] `cargo clippy -p ade_core --lib -- -D warnings` PASS
- [ ] No `HashMap` / `HashSet`
- [ ] No float
- [ ] Truncated-replay equivalence asserted in tests

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| Rollback exceeds k | `ChainSelectionReject::ExceededRollback` (in `ChainEvent::Rejected`) | yes |
| Rollback before immutable tip | `ChainSelectionReject::ForkBeforeImmutableTip` | yes |

---

## 14. Hard Prohibitions

### Inherited (from cluster.md)
- BLUE receiving `&ChainDb`, `&Mux`, parsing genesis text
- Wall-clock reads
- `HashMap` / `HashSet`
- Float
- TODO/placeholder error variants
- `async fn` / `tokio` in BLUE

### Slice-specific
- No "best-effort" rollback that silently caps depth at k — every
  over-k rollback request returns `ExceededRollback`.
- No undocumented mutation of `immutable_tip` — rollback never
  advances or moves the immutable tip.

---

## 15. Explicit Non-Goals

- Do NOT re-validate headers post-rollback (S-B7 does that).
- Do NOT fetch the rolled-back chain-dep state from any IO source.
  The caller supplies it (typically from a N-D snapshot).
- Do NOT implement the immutable-tip advancement rule (that happens
  in the chain-selector orchestrator, S-B10, when a block becomes
  k-deep).

---

## 16. Completion Checklist

- [ ] CE-N-B-2 corpus test passes
- [ ] Truncated-replay equivalence asserted
- [ ] Reject reason byte stability test passes

---

## 17. Review Notes

- This slice carries no advance-immutable-tip rule. S-B10 owns
  that: when a block becomes k-deep in the current chain, the
  orchestrator updates `ChainSelectorState.immutable_tip`. The
  rollback transition reads but never writes that field.

---

## 18. Authority Reminder

Registry > slice doc.
