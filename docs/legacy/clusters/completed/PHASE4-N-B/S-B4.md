# S-B4 — Nonce evolution authority

## Slice Header

**Slice Name**: Nonce evolution authority
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**: substrate for **CE-N-B-4** (leader schedule) and **CE-N-B-5** (replay corpus).

**Slice Dependencies**: S-B1 (`EraSchedule`), S-B2 (`Nonce`, `PraosChainDepState`, `NonceEvolutionError`), S-B3 (`VrfRole::NonceContribution` for VRF input shape).

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified. Do not strengthen header
validation here (S-B7) and do not implement leader schedule (S-B6).

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer per CLAUDE.md.

---

## 4. Intent

Make it impossible to construct a next `PraosChainDepState` whose
nonces do not derive deterministically from the prior state plus a
single canonical header-VRF contribution (within an epoch) or a
single canonical candidate-to-epoch promotion (at an epoch boundary).
Replay equivalence of nonce evolution is the foundational property
that lets leader schedule (S-B6) and fork choice (S-B8) be
deterministic.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/nonce.rs` (NEW — `NonceEvolution`)
- `crates/ade_core/src/consensus/mod.rs` (extend)
- `crates/ade_core/tests/nonce_evolution_corpus.rs` (NEW)
- `corpus/consensus/nonce_evolution/within_epoch.json` (NEW)
- `corpus/consensus/nonce_evolution/epoch_boundary.json` (NEW)
- `crates/ade_testkit/src/consensus/corpus.rs` (extend — helper for
  loading nonce-evolution corpus)

**State machines affected**: `NonceEvolution` transition operates on
`PraosChainDepState` produced in S-B2.

**Persistence impact**: none beyond S-B2's canonical encoding.

**Network-visible impact**: none.

**Out-of-scope**:
- Header field extraction (S-B7)
- Op-cert counter update (S-B5)
- Leader schedule (S-B6)

---

## 6. Execution Boundary

**BLUE**: `ade_core::consensus::nonce`.
**GREEN**: `ade_testkit::consensus` corpus harness extension.
**RED**: none.

---

## 7. Invariants Preserved

- All previous S-B1/S-B2/S-B3 tests still pass.
- `PraosChainDepState` shape unchanged (no new fields).
- `Nonce` newtype unchanged.

---

## 8. Invariants Strengthened or Introduced

- **`DC-CONS-04` strengthened** (behaviour now attached): nonce
  evolution is the deterministic function of `(prior_state,
  nonce_input)` declared by the transition signature; status flips
  from `declared` → `enforced` for the nonce subset (op-cert subset
  flips in S-B5; structural shape already enforced in S-B2).

---

## 9. Design Summary

### Inputs

```rust
// nonce.rs

use ade_types::{EpochNo, SlotNo};
use ade_crypto::vrf::VrfOutput;
use crate::consensus::praos_state::{Nonce, PraosChainDepState};
use crate::consensus::errors::NonceEvolutionError;

/// One input to NonceEvolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonceInput {
    /// A validated header within the current epoch contributes its
    /// VRF *nonce-role* output. `slot` must be strictly greater than
    /// `state.last_slot` (if set).
    HeaderContribution {
        slot:        SlotNo,
        vrf_output:  VrfOutput,
    },
    /// At the canonical Cardano-Praos "8k/f slot" within an epoch, the
    /// evolving nonce is frozen into the candidate nonce. The same
    /// header that triggers it MUST already have been applied via
    /// HeaderContribution; this variant is independent of the header
    /// payload.
    CandidateFreeze {
        at_slot: SlotNo,
        epoch:   EpochNo,
    },
    /// At an epoch boundary, the candidate nonce becomes the new
    /// epoch nonce, the prior epoch nonce becomes previous_epoch_nonce,
    /// evolving is re-seeded to candidate.
    EpochBoundary {
        new_epoch: EpochNo,
        /// last block in the just-finishing epoch (for last_epoch_block)
        last_block_of_prev_epoch: Option<EpochNo>, // see note below
    },
}
```

> Note: `last_block_of_prev_epoch` field is **named imprecisely** in
> the spec sketch. In `PraosChainDepState`, the field is
> `last_epoch_block: Option<EpochNo>`, which actually records the
> *epoch number of the just-finished epoch* (so leader-schedule can
> trace "we are now in epoch N; previous epoch was N-1; last_epoch_
> block is N-1"). Keep the field as declared in S-B2; this slice does
> not change the `PraosChainDepState` shape.

### Transition

```rust
/// Deterministic, total nonce evolution.
///
/// On HeaderContribution:
///   evolving' = blake2b256(evolving ‖ vrf_output_first_32_bytes)
///   last_slot' = Some(slot)
///   all other fields unchanged
///
/// On CandidateFreeze:
///   candidate' = evolving
///   all other fields unchanged
///
/// On EpochBoundary:
///   previous_epoch' = epoch_nonce
///   epoch_nonce'    = candidate_nonce
///   evolving'       = candidate_nonce   (seed for next epoch)
///   lab_nonce'      = lab_nonce          (lab is updated by header-
///                                         validate, not by this slice;
///                                         this slice preserves)
///   last_epoch_block' = Some(new_epoch)
///   op_cert_counters' = op_cert_counters (unchanged; S-B5 owns
///                                         counter pruning if any)
pub fn apply_nonce_input(
    state: &PraosChainDepState,
    input: &NonceInput,
) -> Result<PraosChainDepState, NonceEvolutionError>;
```

Failure conditions:
- `HeaderContribution { slot, .. }` with `slot ≤ state.last_slot` →
  `NonceEvolutionError::SlotBeforeLast { last, attempted }`.
- `EpochBoundary { .. }` while `state.epoch_nonce == Nonce::ZERO` and
  `state.candidate_nonce == Nonce::ZERO` → `NonceEvolutionError::
  UninitialisedEpochNonce`.
- `CandidateFreeze` never fails — it is a structural transition.

### Hashing

Use `ade_crypto::blake2b::blake2b_256(input: &[u8]) -> Hash32`. The
existing `ade_crypto::blake2b` module is already BLUE. Take the first
32 bytes of `VrfOutput` (which is 64 bytes); concatenate with the
prior `evolving_nonce` (32 bytes); hash to 32 bytes → new
`evolving_nonce`.

Verify the byte-order convention matches ouroboros-consensus:
`evolving ‖ vrf_output[0..32]`, not `vrf_output[0..32] ‖ evolving`.
A pinned known-vector test confirms this.

---

## 10. Changes Introduced

### Types
- New: `NonceInput`.

### State Transitions
- New: `apply_nonce_input`.

### Persistence
- None.

### Removal / Refactors
- None.

---

## 11. Replay, Crash, and Epoch Validation

### Corpus

`corpus/consensus/nonce_evolution/within_epoch.json`:

```jsonc
{
  "scenario": "within_epoch_three_header_contributions",
  "initial_state": {
    "evolving_nonce":       "0000000000000000000000000000000000000000000000000000000000000000",
    "candidate_nonce":      "0000000000000000000000000000000000000000000000000000000000000000",
    "epoch_nonce":          "1111111111111111111111111111111111111111111111111111111111111111",
    "previous_epoch_nonce": "2222222222222222222222222222222222222222222222222222222222222222",
    "lab_nonce":            "3333333333333333333333333333333333333333333333333333333333333333",
    "last_epoch_block":     208,
    "last_slot":            4500000,
    "last_block_no":        7800000,
    "op_cert_counters":     []
  },
  "inputs": [
    { "kind": "HeaderContribution", "slot": 4500001, "vrf_output_hex": "<128 hex chars>" },
    { "kind": "HeaderContribution", "slot": 4500002, "vrf_output_hex": "<128 hex chars>" },
    { "kind": "HeaderContribution", "slot": 4500003, "vrf_output_hex": "<128 hex chars>" }
  ],
  "expected_final_evolving_nonce": "<computed by the implementation; pinned>"
}
```

`corpus/consensus/nonce_evolution/epoch_boundary.json`:

```jsonc
{
  "scenario": "candidate_freeze_then_epoch_boundary",
  "initial_state": { ... evolving non-zero, candidate zero ... },
  "inputs": [
    { "kind": "CandidateFreeze", "at_slot": 4924880, "epoch": 208 },
    { "kind": "EpochBoundary",   "new_epoch": 209, "last_epoch_block": 208 }
  ],
  "expected_final_epoch_nonce":     "<computed; pinned>",
  "expected_final_previous_epoch":  "<must equal initial epoch_nonce>",
  "expected_final_evolving":        "<must equal initial candidate AFTER freeze and epoch boundary>"
}
```

> The implementer fills the `expected_*` values on first test run
> (the test is "compute and compare against the pinned value"; the
> pinned value is checked into the corpus once it's stable).

### Tests

- `crates/ade_core/tests/nonce_evolution_corpus.rs`:
  - `within_epoch_evolving_nonce_matches_corpus`
  - `epoch_boundary_freezes_and_rotates_correctly`
  - `nonce_evolution_replay_is_deterministic` — apply the same input
    sequence twice; assert byte-identical final states.

- Unit tests in `nonce.rs`:
  - `header_contribution_rejects_non_monotonic_slot`
  - `header_contribution_advances_evolving_nonce_deterministically`
  - `header_contribution_does_not_touch_epoch_nonce`
  - `candidate_freeze_copies_evolving_to_candidate`
  - `candidate_freeze_does_not_advance_slot` — last_slot unchanged
  - `epoch_boundary_promotes_candidate_to_epoch_nonce`
  - `epoch_boundary_rotates_previous_epoch_nonce`
  - `epoch_boundary_rejects_uninitialised_candidate`
  - `epoch_boundary_preserves_op_cert_counters`
  - `epoch_boundary_preserves_lab_nonce`
  - `blake2b_input_order_is_evolving_then_vrf_output_first_32`

### Replay impact
- Pure function; same inputs → same outputs.

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core` PASS
- [ ] `cargo test -p ade_core --lib consensus::nonce` PASS
- [ ] `cargo test -p ade_core --test nonce_evolution_corpus` PASS
- [ ] `cargo clippy -p ade_core --all-targets -- -D warnings` PASS
- [ ] No `HashMap` / `HashSet` (grep)
- [ ] No `String` in `NonceInput` or its serialisation path
- [ ] Replay determinism asserted by the corpus test running the
      sequence twice

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| Non-monotonic header slot | `NonceEvolutionError::SlotBeforeLast { last, attempted }` | yes |
| Epoch boundary with uninitialised candidate | `NonceEvolutionError::UninitialisedEpochNonce` | yes |

---

## 14. Hard Prohibitions

### Inherited (from cluster.md)
- BLUE receiving `&ChainDb`, `&Mux`, parsing genesis text
- Wall-clock reads in BLUE
- `HashMap` / `HashSet`
- Floating-point arithmetic
- TODO/placeholder error variants
- `async fn`, `.await`, `tokio` in BLUE

### Slice-specific
- No `Vec<u8>` allocation in the hot path for the BLAKE2b input —
  use a fixed `[u8; 64]` buffer (32 evolving + 32 vrf output prefix).
- No string-formatted reject reasons.

---

## 15. Explicit Non-Goals

- Do NOT do op-cert counter check (S-B5).
- Do NOT extract VRF output from a header — S-B7 will produce
  `NonceInput::HeaderContribution { slot, vrf_output }` after header
  validation.
- Do NOT touch `lab_nonce` — lab is a header-validate concept and
  this slice preserves it.

---

## 16. Completion Checklist

- [ ] All transitions are pure functions
- [ ] Replay determinism asserted in tests
- [ ] No TODOs in BLUE
- [ ] Corpus files committed with pinned expected values

---

## 17. Review Notes

- `lab_nonce` (look-ahead-block nonce) is preserved by this slice;
  S-B7 (header validation) will populate it at the right slot
  (typically the last header of the previous epoch's stable region,
  per ouroboros-consensus PraosState). If S-B7 finds a cleaner home
  for it, that's a S-B7-time decision.
- The `EpochBoundary.last_block_of_prev_epoch` field in the input is
  not the same shape as `PraosChainDepState::last_epoch_block`.
  See the inline note in §9.

---

## 18. Authority Reminder

Correctness rules live in `docs/ade-invariant-registry.toml`. If
this doc conflicts with the registry, the registry wins.
