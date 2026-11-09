# S-B8 — Fork choice + `CandidateFragment` (CE-N-B-1 close)

## Slice Header

**Slice Name**: Fork choice + `CandidateFragment`
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**:
- [x] **CE-N-B-1** — Fork choice produces identical best-chain
  selection on every multi-tip case in a curated divergence corpus;
  rejection-reason byte-identity validated for
  `ForkBeforeImmutableTip`, `ExceededRollback`, `HeaderInvalid`,
  `TiebreakerLossKeepCurrent`.

**Slice Dependencies**: S-B1..S-B7.

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified.

The fork-choice transition operates over a typed `CandidateFragment`
input. **BLUE does NOT receive `&ChainDb`.** A GREEN materializer
(`ade_runtime::consensus::candidate_fragment`) builds the fragment
from N-D and N-A data and hands it to BLUE.

**Slice-entry obligation** (sketch residual `a-residual`): record
the pinned `ouroboros-consensus` revision in a project doc reference
inside the slice's module-level rustdoc. The relevant revision is the
one shipped with cardano-node 10.6.2. If pinning to a specific git
SHA is infeasible from this slice, document the cardano-node version
and the corresponding ouroboros-consensus package version (visible
in `cabal.project.freeze` in cardano-node 10.6.2 tag) — this is the
auditable artifact.

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer.

---

## 4. Intent

Make it impossible for Ade to disagree with cardano-node about which
chain is best, given the same `(candidate_fragments, EraSchedule,
ledger_view, protocol_params)`. The ordering is **block-number
first**, then Praos `TiebreakerView` (slot, issuer, op-cert issue
counter, VRF output). Density-based ordering is reserved for
Genesis / catch-up logic and **forbidden in caught-up Praos
fork-choice**.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/fork_choice.rs` (NEW — BLUE)
- `crates/ade_core/src/consensus/candidate.rs` (NEW —
  `CandidateFragment`, `TiebreakerView`, `ChainSelectorState`,
  pure types BLUE consumes)
- `crates/ade_core/src/consensus/mod.rs` (extend)
- `crates/ade_core/tests/fork_choice_corpus.rs` (NEW — closes
  CE-N-B-1)
- `corpus/consensus/fork_choice/multi_tip.json` (NEW)
- `corpus/consensus/fork_choice/rejects.json` (NEW)
- `crates/ade_runtime/src/consensus/candidate_fragment.rs` (NEW —
  GREEN materializer)
- `crates/ade_runtime/src/consensus/mod.rs` (extend)
- `crates/ade_testkit/src/consensus/corpus.rs` (extend — fork_choice
  path helper)

**State machines affected**: `ForkChoice` transition consumes
`ChainSelectorState` and `CandidateFragment`.

**Persistence impact**: `ChainSelectorState` includes the current
best-tip `Point` and the immutable tip `Point`; persistence is N-D's
concern (out of N-B).

**Network-visible impact**: none.

**Out-of-scope**:
- Rollback transition (S-B9)
- Live N-A wiring (S-B10)
- Density-based Genesis logic — explicitly forbidden in this slice

---

## 6. Execution Boundary

**BLUE**:
- `ade_core::consensus::fork_choice`
- `ade_core::consensus::candidate`

**GREEN**:
- `ade_runtime::consensus::candidate_fragment` — materializes
  `CandidateFragment` from N-D / N-A data

**RED**: none.

---

## 7. Invariants Preserved

- All S-B1..S-B7 tests pass.
- `PraosChainDepState` shape unchanged.
- No new BLUE crate dep.

---

## 8. Invariants Strengthened or Introduced

- **`DC-CONS-03` (NEW)** — Block-number first, then `TiebreakerView`.
  Density forbidden in caught-up path.
- **`DC-CONSENSUS-01` strengthened** — best-chain selection is a pure
  function of canonical inputs.
- **`CN-CONS-01` strengthened** — fork-choice determinism asserted by
  replay corpus.
- **`CN-CONS-02..05` strengthened** — no wall-clock, no `HashMap`, no
  body inspection for tip comparison.

---

## 9. Design Summary

### Canonical types

```rust
// candidate.rs

use ade_types::{BlockNo, Hash28, Hash32, SlotNo};
use ade_crypto::vrf::VrfOutput;
use crate::consensus::events::{BlockDistance, Point, SecurityParam};
use crate::consensus::header_summary::ValidatedHeaderSummary;

/// Tiebreaker view per ouroboros-consensus PraosTiebreaker:
/// (slot, issuer_hash, op_cert_counter, vrf_output).
/// Compared lexicographically; lower is preferred per Cardano-Praos
/// convention (lower slot first because earlier-arriving is more
/// stable; higher op-cert counter is preferred because it indicates
/// active operator). The exact ordering is encoded as:
///
///   primary = slot                     ascending (lower preferred)
///   then    = issuer_hash               ascending (deterministic; not preferential)
///   then    = op_cert_counter           descending (higher preferred)
///   then    = leader_vrf_output_first_8 ascending (lower preferred — lower VRF value beats)
///
/// CRITICAL: the comparison must match ouroboros-consensus exactly.
/// The implementation uses an explicit Ord impl, not derive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TiebreakerView {
    pub slot:           SlotNo,
    pub issuer_hash:    Hash28,
    pub op_cert_counter:u64,
    pub leader_vrf_output_first_8: [u8; 8],
}

/// Total preference order — implemented via an explicit cmp method,
/// not Derive(Ord), to make the semantics auditable.
pub fn tiebreaker_prefer(a: &TiebreakerView, b: &TiebreakerView) -> std::cmp::Ordering;

/// One candidate chain fragment — a sequence of validated headers
/// rooted at a common anchor point with the current chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFragment {
    pub anchor:           Point,
    pub headers:          Vec<ValidatedHeaderSummary>,  // ordered by ascending slot
    pub select_view:      TiebreakerView,                // for the tip
    pub rollback_depth:   BlockDistance,                  // how far back from current tip
}

/// Authoritative selector state — owned by N-B, persisted by N-D.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSelectorState {
    pub current_tip:     Point,
    pub current_tip_block_no:  BlockNo,
    pub current_tiebreaker:    TiebreakerView,
    pub immutable_tip:   Point,
    pub immutable_tip_block_no:BlockNo,
    pub security_param:  SecurityParam,
}
```

### Transition

```rust
// fork_choice.rs

use crate::consensus::candidate::{CandidateFragment, ChainSelectorState, tiebreaker_prefer};
use crate::consensus::events::{ChainEvent, ChainSelectionReject};
use crate::consensus::errors::HeaderValidationError;
use crate::consensus::era_schedule::EraSchedule;
use crate::consensus::praos_state::PraosChainDepState;
use ade_types::ProtocolParameters; // verify existence; if not, declare opaque

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForkChoiceError {
    NoCandidates,
}

pub fn select_best_chain(
    state:          &ChainSelectorState,
    candidates:     &[CandidateFragment],
    _era_schedule:  &EraSchedule,
    _ledger_view:   &dyn crate::consensus::ledger_view::LedgerView,
    _protocol_params: &(),  // placeholder; verify ade_types has a public ProtocolParameters; if not, accept () or omit
) -> Result<(ChainSelectorState, ChainEvent), ForkChoiceError>;
```

> **Implementer note**: if `ade_types::ProtocolParameters` does not
> exist, drop the `protocol_params` parameter entirely — current N-B
> fork-choice does not consult protocol params (it's mostly schedule-
> agnostic). The `era_schedule` and `ledger_view` params are present
> for forward-compatibility but unused this slice; mark them
> `#[allow(unused_variables)]` if necessary (this is the only
> exception to the no-placeholder rule and it's justified by the
> cluster's contract). Better: don't include them — keep the
> signature minimal. The cluster's primary invariant says fork-choice
> is a pure function over `(candidate_fragments, EraSchedule,
> ledger_view, protocol_params)` but in practice the BLUE comparison
> here uses only block-number + tiebreaker view, neither of which
> needs `era_schedule` or `ledger_view` directly. Document this in
> rustdoc.

> **Decision for this slice**: keep the signature minimal —
> ```rust
> pub fn select_best_chain(
>     state:      &ChainSelectorState,
>     candidates: &[CandidateFragment],
> ) -> Result<(ChainSelectorState, ChainEvent), ForkChoiceError>;
> ```
> The wider signature with era_schedule / ledger_view / protocol_params
> belongs to S-B10's chain-selector orchestrator, which threads them
> through where they are needed.

### Algorithm

For each candidate `c`:
1. If `c.anchor.slot < state.immutable_tip.slot`, mark
   `ChainSelectionReject::ForkBeforeImmutableTip { immutable_tip,
   candidate_intersection: c.anchor, rollback_depth: c.rollback_
   depth, security_param: state.security_param }`. This candidate is
   not eligible.
2. If `c.rollback_depth > state.security_param.0`, mark
   `ChainSelectionReject::ExceededRollback { requested:
   c.rollback_depth, max: state.security_param }`. Not eligible.
3. Compute candidate's block-no = `c.anchor.block_no + c.headers.len()`
   — needs the anchor block_no. Add it to `Point`:

```rust
// events.rs already has Point { slot, hash }
// Either extend Point to include block_no, OR pass anchor_block_no
// through CandidateFragment.
```

> Cleaner: add `anchor_block_no: BlockNo` to `CandidateFragment`. Do
> that; it's a one-line addition and doesn't require changing the
> S-B2 `Point` shape.

4. Candidate tip = last header (slot, hash from
   `ValidatedHeaderSummary`).
5. Compare candidates by `(tip_block_no, tiebreaker_prefer(
   candidate_tiebreaker, current_tiebreaker))`:
   - Higher block_no wins outright.
   - Equal block_no → use `tiebreaker_prefer`.
6. Pick the maximally-preferred eligible candidate.
7. If best candidate's `(block_no, tiebreaker)` is "preferred over"
   `(state.current_tip_block_no, state.current_tiebreaker)`, accept
   it → emit `ChainEvent::ChainSelected { new_tip, replaced_tip:
   Some(state.current_tip) }`. Update `state.current_tip`,
   `current_tip_block_no`, `current_tiebreaker`.
8. Otherwise emit `ChainEvent::Rejected { reason:
   ChainSelectionReject::TiebreakerLossKeepCurrent { current_tip,
   candidate_tip: best.tip } }` if there was at least one eligible
   candidate; or `NoCandidates` error if `candidates` is empty.
9. Ineligible candidates contribute their reject reason but the
   final event reflects the chosen winner (or the first reject if
   no eligible candidate). Multiple-reject reporting is one-shot:
   prefer the candidate with the highest block-no among ineligible
   candidates for the reject reason.

### Density forbidden

A `ci/ci_check_no_density_in_fork_choice.sh` script greps
`crates/ade_core/src/consensus/fork_choice.rs` and
`crates/ade_core/src/consensus/candidate.rs` for the substring
`density` (case-insensitive) — if any appear outside a comment line
that begins with `// no-density:`, the script fails. This is a
mechanical guard for `DC-CONS-03`.

### GREEN materializer (sketch — minimal)

```rust
// ade_runtime::consensus::candidate_fragment

use ade_core::consensus::candidate::{CandidateFragment, TiebreakerView};
use ade_core::consensus::header_summary::ValidatedHeaderSummary;
use ade_core::consensus::events::{BlockDistance, Point};
use ade_types::BlockNo;

/// Build a CandidateFragment from a sequence of validated headers
/// and an anchor.
///
/// This is GREEN: deterministic but non-authoritative. It exists so
/// the orchestrator (S-B10) and tests have a uniform construction
/// path.
pub fn build_candidate_fragment(
    anchor:           Point,
    anchor_block_no:  BlockNo,
    headers:          Vec<ValidatedHeaderSummary>,
    rollback_depth:   BlockDistance,
) -> CandidateFragment;
```

---

## 10. Changes Introduced

### Types
- New: `TiebreakerView`, `CandidateFragment`, `ChainSelectorState`,
  `ForkChoiceError`.
- Adjusted: `CandidateFragment` gains `anchor_block_no: BlockNo`.

### State Transitions
- New: `select_best_chain` (BLUE), `build_candidate_fragment`
  (GREEN).

### Persistence
- `ChainSelectorState` shape locked; N-D will persist it.

---

## 11. Replay, Crash, and Epoch Validation

### Corpus

`corpus/consensus/fork_choice/multi_tip.json`:

```jsonc
{
  "scenario": "two_candidates_one_higher_block_no_wins",
  "state": {
    "current_tip": { "slot": 100, "hash_hex": "11..11" },
    "current_tip_block_no": 50,
    "current_tiebreaker": { "slot": 100, "issuer_hex": "aa..aa", "op_cert_counter": 5, "vrf_output_first_8_hex": "0102030405060708" },
    "immutable_tip": { "slot": 50, "hash_hex": "00..00" },
    "immutable_tip_block_no": 25,
    "security_param": 2160
  },
  "candidates": [
    {
      "anchor": { "slot": 95, "hash_hex": "22..22" },
      "anchor_block_no": 48,
      "headers_count": 3,
      "tip": { "slot": 110, "hash_hex": "33..33", "block_no": 51 },
      "tiebreaker": { "slot": 110, "issuer_hex": "bb..bb", "op_cert_counter": 8, "vrf_output_first_8_hex": "0a0a0a0a0a0a0a0a" },
      "rollback_depth": 2
    },
    {
      "anchor": { "slot": 95, "hash_hex": "22..22" },
      "anchor_block_no": 48,
      "headers_count": 2,
      "tip": { "slot": 105, "hash_hex": "44..44", "block_no": 50 },
      "tiebreaker": { "slot": 105, "issuer_hex": "cc..cc", "op_cert_counter": 7, "vrf_output_first_8_hex": "0202020202020202" },
      "rollback_depth": 2
    }
  ],
  "expected": { "event": "ChainSelected", "new_tip": { "slot": 110, "hash_hex": "33..33" } }
}
```

`corpus/consensus/fork_choice/rejects.json`:

```jsonc
{
  "scenarios": [
    {
      "name": "fork_before_immutable_tip",
      "state": { /* immutable at slot 50 */ },
      "candidates": [ { "anchor": { "slot": 40, "hash_hex": "...", "block_no": 20 }, "rollback_depth": 5, "tip_block_no": 30 } ],
      "expected_event": "Rejected",
      "expected_reason": { "kind": "ForkBeforeImmutableTip", "rollback_depth": 30 }
    },
    {
      "name": "exceeded_rollback",
      "state": { "security_param": 100 },
      "candidates": [ { "anchor": { ... }, "rollback_depth": 200, "tip_block_no": 999 } ],
      "expected_event": "Rejected",
      "expected_reason": { "kind": "ExceededRollback", "requested": 200, "max": 100 }
    },
    {
      "name": "tiebreaker_loss",
      "state": { /* current_tip with strong tiebreaker */ },
      "candidates": [ { /* equal block_no, weaker tiebreaker */ } ],
      "expected_event": "Rejected",
      "expected_reason": { "kind": "TiebreakerLossKeepCurrent" }
    }
  ]
}
```

### Tests

- `crates/ade_core/tests/fork_choice_corpus.rs`:
  - `higher_block_no_wins`
  - `equal_block_no_tiebreaker_decides`
  - `fork_before_immutable_tip_rejected`
  - `exceeded_rollback_rejected`
  - `tiebreaker_loss_keeps_current`
  - `replay_is_deterministic`
  - `reject_reason_bytes_are_stable` (encode → hex pinned)

- Unit tests in `fork_choice.rs`:
  - `tiebreaker_prefer_lower_slot_wins`
  - `tiebreaker_prefer_higher_op_cert_wins_on_equal_slot_and_issuer`
  - `tiebreaker_prefer_lower_vrf_value_wins_on_full_tie`
  - `no_candidates_returns_no_candidates_error`
  - `equal_to_current_keeps_current_via_tiebreaker_loss`

- Unit tests in `candidate.rs`:
  - `tiebreaker_view_eq_is_field_wise`
  - `candidate_fragment_carries_anchor_block_no`

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core -p ade_runtime` PASS
- [ ] `cargo test -p ade_core --lib consensus::fork_choice` PASS
- [ ] `cargo test -p ade_core --lib consensus::candidate` PASS
- [ ] `cargo test -p ade_core --test fork_choice_corpus` PASS (this
      is the **CE-N-B-1 close** test)
- [ ] `cargo clippy -p ade_core --lib -- -D warnings` PASS
- [ ] `bash ci/ci_check_no_density_in_fork_choice.sh` PASS
- [ ] Module docstring of `fork_choice.rs` documents the pinned
      cardano-node version (10.6.2) and the corresponding
      ouroboros-consensus revision

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| No candidates | `ForkChoiceError::NoCandidates` | yes |
| Fork before immutable tip | `ChainSelectionReject::ForkBeforeImmutableTip` (carried in `ChainEvent::Rejected`) | yes |
| Exceeded rollback | `ChainSelectionReject::ExceededRollback` | yes |
| Tiebreaker loss | `ChainSelectionReject::TiebreakerLossKeepCurrent` | yes |

---

## 14. Hard Prohibitions

### Inherited (from cluster.md)
- BLUE receiving `&ChainDb`, `&Mux`, parsing genesis text
- Wall-clock reads
- `HashMap` / `HashSet`
- Float
- TODO/placeholder error variants
- `async fn` / `tokio` in BLUE
- Density-based ordering in caught-up Praos fork-choice
- Body inspection for tip comparison

### Slice-specific
- No `Vec::sort_by_key` on `f32`/`f64` — closure must be integer-
  based.
- No "best-effort" tiebreaker — every byte-for-byte tie returns a
  deterministic answer (the explicit `tiebreaker_prefer` function
  is total).
- No silent acceptance of an ineligible candidate (every reject is
  named).

---

## 15. Explicit Non-Goals

- Do NOT implement rollback (S-B9).
- Do NOT consume real N-A chain-sync events — S-B10 wires that.
- Do NOT include `era_schedule` / `ledger_view` / `protocol_params`
  in the BLUE function signature. Keep the surface minimal.

---

## 16. Completion Checklist

- [ ] CE-N-B-1 corpus test passes
- [ ] Density-grep CI passes
- [ ] Rejection reason byte stability test passes
- [ ] Tiebreaker comparison auditable (explicit cmp, not derive)

---

## 17. Review Notes

- The tiebreaker ordering: confirm against ouroboros-consensus.
  Cardano-Praos: lower slot wins (earlier proposal is more stable),
  higher op-cert counter wins (live operator beats stale), lower VRF
  output bytes wins (lottery-style). The exact order documented in
  rustdoc + test vector.
- The pinned ouroboros-consensus revision: cardano-node 10.6.2 ships
  with ouroboros-consensus 0.22.0.0 (verify in 10.6.2 release notes
  or cabal.project.freeze). Pin this in module docstring.

---

## 18. Authority Reminder

Registry > slice doc.
