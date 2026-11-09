# S-B7 — Praos header validation

## Slice Header

**Slice Name**: Praos header validation
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**: substrate for **CE-N-B-1** (fork choice, S-B8); flips status of `DC-CONS-04` and `DC-CONS-10` to `enforced`.

**Slice Dependencies**: S-B1, S-B2, S-B3, S-B4, S-B5, S-B6.

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified. This slice composes
S-B3 (VRF), S-B4 (nonce), S-B5 (op-cert) into a single
`PraosHeaderValidate` transition. It does not own VRF / nonce /
op-cert logic itself — it sequences them.

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer.

---

## 4. Intent

Make it impossible to extend `PraosChainDepState` from a header
without that header passing VRF (twice — nonce and leader), op-cert
counter, and forecast-horizon checks. The single point of header
admission is `validate_and_apply_header`. Every other consensus
component (fork-choice, rollback) consumes the resulting
`ValidatedHeaderSummary`.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/header_validate.rs` (NEW)
- `crates/ade_core/src/consensus/header_summary.rs` (NEW —
  `ValidatedHeaderSummary` type and field-extraction helpers)
- `crates/ade_core/src/consensus/mod.rs` (extend)
- `crates/ade_core/tests/header_validate_compose.rs` (NEW —
  composition test exercising the full pipeline on synthetic data)

**State machines affected**: `PraosHeaderValidate` transition on
`PraosChainDepState`.

**Persistence impact**: none.

**Network-visible impact**: none.

**Out-of-scope**:
- Decoding header bytes from the network — this slice assumes the
  caller provides a parsed `HeaderInput` produced by `ade_network`
  (S-A4 block-fetch / chain-sync) or by a test driver
- Fork choice (S-B8)
- Rollback (S-B9)
- KES signature verification (separate concern; placeholder in
  `HeaderInput` for now, see §15 non-goals)

---

## 6. Execution Boundary

**BLUE**: `ade_core::consensus::header_validate`, `ade_core::consensus::header_summary`.
**GREEN**: none.
**RED**: none.

---

## 7. Invariants Preserved

- S-B1..S-B6 tests all pass.
- `PraosChainDepState` shape unchanged.

---

## 8. Invariants Strengthened or Introduced

- **`CN-CONS-04` strengthened**: header validation binds exactly to
  the accepted body and consensus context via `ValidatedHeaderSummary
  { body_hash, .. }`.
- **`DC-CONS-04`** — **enforced**: header-validate is the single
  point where nonce evolution, op-cert update, and `last_slot`
  /`last_block_no` advance happen. Earlier slices (S-B4/S-B5)
  introduced the underlying transitions; this slice closes the
  composition gap. Status: `declared` → `enforced`.
- **`DC-CONS-10`** — **enforced**: op-cert counter is now checked
  on every header. Status: `declared` → `enforced`.
- **`CN-CONS-05` strengthened**: header validation is pure of wall
  clock and arrival order.

---

## 9. Design Summary

### Input

```rust
// header_summary.rs

use ade_types::{BlockNo, Hash28, Hash32, SlotNo};
use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
use crate::consensus::vrf_cert::ActiveSlotsCoeff;

/// Everything a validator needs to admit a header. Constructed by
/// the network layer (S-A4 chain-sync) or by a test driver.
///
/// This is NOT the raw on-wire shape — it's a structured projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderInput {
    pub slot:              SlotNo,
    pub block_no:          BlockNo,
    pub body_hash:         Hash32,
    pub issuer_pool:       Hash28,
    pub op_cert_kes_period:u64,
    pub op_cert_counter:   u64,
    /// VRF verification key registered for this pool in the snapshot
    /// (lookup belongs to the caller via LedgerView).
    pub vrf_vk:            VrfVerificationKey,
    /// VRF nonce-role proof.
    pub vrf_nonce_proof:   VrfProof,
    /// VRF leader-role proof.
    pub vrf_leader_proof:  VrfProof,
}

/// The output produced when a header validates — consumed by
/// fork-choice (S-B8) and rollback (S-B9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedHeaderSummary {
    pub slot:           SlotNo,
    pub block_no:       BlockNo,
    pub body_hash:      Hash32,
    pub issuer_pool:    Hash28,
    pub op_cert_counter:u64,
    pub vrf_leader_output: VrfOutput,
}
```

### Transition

```rust
// header_validate.rs

use ade_types::EpochNo;
use crate::consensus::era_schedule::EraSchedule;
use crate::consensus::header_summary::{HeaderInput, ValidatedHeaderSummary};
use crate::consensus::ledger_view::LedgerView;
use crate::consensus::praos_state::PraosChainDepState;
use crate::consensus::errors::{HeaderValidationError, OutsideForecastRange};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderApplied {
    pub new_state: PraosChainDepState,
    pub summary:   ValidatedHeaderSummary,
}

/// The single point of header admission. Composes:
///   1. forecast-horizon check (EraSchedule)
///   2. monotone slot check (state.last_slot)
///   3. monotone block-no check (state.last_block_no)
///   4. op-cert counter monotonicity (state.op_cert_counters)
///   5. VRF cert verify — nonce role (vrf_cert::verify_vrf_cert)
///   6. VRF cert verify — leader role (vrf_cert::verify_vrf_cert)
///   7. leader threshold check (vrf_cert::is_leader on leader output)
///   8. apply op-cert observation (op_cert::apply_op_cert)
///   9. apply nonce contribution (nonce::apply_nonce_input)
///  10. advance last_slot / last_block_no
///
/// All checks must succeed; on any failure return a single typed
/// HeaderValidationError. No partial state — failure is fail-fast.
pub fn validate_and_apply_header(
    state:         &PraosChainDepState,
    header:        &HeaderInput,
    ledger_view:   &dyn LedgerView,
    era_schedule:  &EraSchedule,
) -> Result<HeaderApplied, HeaderValidationError>;
```

### Pipeline ordering & error mapping

| Step | Failure | Maps to |
|---|---|---|
| 1 | `OutsideForecastRange` | `HeaderValidationError::HFC(HFCError::...)` (via `LeaderScheduleError::OutsideForecastRange`) — actually we propagate `OutsideForecastRange` directly; add a new variant `HeaderValidationError::OutsideForecastRange(OutsideForecastRange)` to S-B2's enum if not already present (verify; extend `errors.rs` if needed) |
| 2 | `slot <= state.last_slot` | `HeaderValidationError::SlotBeforeLastApplied { last, attempted }` |
| 3 | `block_no <= state.last_block_no` | `HeaderValidationError::BlockNoOutOfOrder { last, attempted }` |
| 4 | counter regression | `HeaderValidationError::OpCertCounter(OpCertCounterError::Regression { .. })` |
| 5 | VRF nonce-role fail | `HeaderValidationError::VrfCert(VrfCertError::...)` |
| 6 | VRF leader-role fail | `HeaderValidationError::VrfCert(VrfCertError::...)` |
| 7 | leader threshold fail | `HeaderValidationError::VrfCert(VrfCertError::LeaderValueAboveThreshold { .. })` |
| 8 | op-cert apply (cannot fail since step 4 already gated; defensive) | `HeaderValidationError::OpCertCounter(...)` |
| 9 | nonce apply | `HeaderValidationError::Nonce(NonceEvolutionError::...)` |

Step 9 mutation uses
`NonceInput::HeaderContribution { slot, vrf_output: leader_proof.derived }`
— actually the **nonce contribution** comes from the *nonce-role* VRF
output (step 5's `VerifiedVrf.output`), not the leader-role output.
Use the correct one.

### Body-hash + era binding (deferred to S-B8 wiring)

The `HeaderInput.body_hash` is recorded in the summary for downstream
consumers; this slice does not fetch the body. CN-CONS-04 strengthening
("header validation binds exactly to the accepted body") is closed in
the wider sense: the summary carries the body hash; the *body* check
happens at body-admission time (out of N-B).

`HeaderValidationError::BodyHashMismatch` and `EraMismatch` exist in the
S-B2 closed enum but are not produced by this slice (they're for
consumers — block-fetch or chain-db). Document this in module-level
rustdoc.

---

## 10. Changes Introduced

### Types
- New: `HeaderInput`, `ValidatedHeaderSummary`, `HeaderApplied`.

### State Transitions
- New: `validate_and_apply_header`.

### Persistence
- None.

### Errors
- Extend `HeaderValidationError` with
  `OutsideForecastRange(OutsideForecastRange)` if not present.

---

## 11. Replay, Crash, and Epoch Validation

### Tests

- `crates/ade_core/tests/header_validate_compose.rs`:
  - `valid_header_accepted_advances_state` — synth VRF proofs via
    `cardano_crypto::vrf::VrfDraft03`, asc 1/1 + σ = 1/1 so the
    leader threshold trivially passes, observe new state's
    last_slot/last_block_no/op_cert_counters/evolving_nonce all
    advanced.
  - `header_with_slot_regression_rejected` — second header with
    smaller slot → `SlotBeforeLastApplied`.
  - `header_with_block_no_regression_rejected` →
    `BlockNoOutOfOrder`.
  - `header_with_op_cert_regression_rejected` →
    `OpCertCounter(Regression {..})`.
  - `header_with_invalid_vrf_proof_rejected` →
    `VrfCert(VerificationFailed)`.
  - `header_beyond_forecast_horizon_rejected` →
    `OutsideForecastRange { .. }`.
  - `validate_replay_is_deterministic` — apply the same header
    twice from the same start state → same `HeaderApplied`.

- Unit tests in `header_validate.rs`:
  - `pipeline_short_circuits_on_first_failure` — feed a header that
    fails step 2; assert no later step's failure is reported and
    state is unchanged.
  - `nonce_contribution_uses_nonce_role_vrf_output_not_leader_role`
    — synthesize a state, apply a header, and verify the resulting
    evolving_nonce matches blake2b256(prior_evolving ‖ nonce_role_
    output[0..32]), NOT leader_role.

### Replay impact
- Pure function. Replay-equivalent.

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core` PASS
- [ ] `cargo test -p ade_core --lib consensus::header_validate` PASS
- [ ] `cargo test -p ade_core --test header_validate_compose` PASS
- [ ] `cargo clippy -p ade_core --lib -- -D warnings` PASS
- [ ] No `HashMap` / `HashSet`
- [ ] No float
- [ ] DC-CONS-04 and DC-CONS-10 status flipped to `enforced` in the
      registry, with `code_locus` covering header_validate.rs and
      with the new tests appended

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| Slot before last applied | `HeaderValidationError::SlotBeforeLastApplied { last, attempted }` | yes |
| Block-no out of order | `HeaderValidationError::BlockNoOutOfOrder` | yes |
| Op-cert regression | `HeaderValidationError::OpCertCounter` | yes |
| VRF malformed/failed | `HeaderValidationError::VrfCert` | yes |
| Leader threshold | `HeaderValidationError::VrfCert(LeaderValueAboveThreshold)` | yes |
| Nonce evolution | `HeaderValidationError::Nonce` | yes |
| Forecast horizon | `HeaderValidationError::OutsideForecastRange` | yes |
| HFC translation | `HeaderValidationError::HFC` | yes |

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
- No mutation of `state` in place — always return a new
  `PraosChainDepState`.
- No silent fallback to a "best effort" header acceptance — every
  failure must produce a structured `HeaderValidationError`.
- The body-hash field is recorded but not verified here; this is
  intentional and documented in rustdoc.

---

## 15. Explicit Non-Goals

- Do NOT decode raw header bytes (responsibility of `ade_network`).
- Do NOT fetch or verify bodies.
- Do NOT implement KES signature verification. Mark a placeholder
  in module docstring: `// KES sig over op-cert is checked at body-
  admission time (out of N-B)`. The `op_cert_kes_period` and
  `op_cert_counter` fields are still extracted and counter-checked.
- Do NOT implement fork choice or rollback.

---

## 16. Completion Checklist

- [ ] `validate_and_apply_header` is the *only* exported function
      that advances `PraosChainDepState` with a header
- [ ] All failure paths return typed errors
- [ ] DC-CONS-04 / DC-CONS-10 enforced flag flipped
- [ ] Test for short-circuit ordering present

---

## 17. Review Notes

- The leader-role VRF output is used for *leader threshold*; the
  nonce-role VRF output is used for *nonce contribution*. They are
  distinct outputs from distinct VRF inputs. Mixing them is a real
  bug and the unit test `nonce_contribution_uses_nonce_role_vrf_
  output_not_leader_role` exists to prevent it.

---

## 18. Authority Reminder

Registry > slice doc.
