# S-B6 — Leader schedule (CE-N-B-4 close)

## Slice Header

**Slice Name**: Leader schedule
**Cluster**: PHASE4-N-B
**Status**: In Progress
**Cluster Exit Criteria Addressed**:
- [x] **CE-N-B-4** — Leader schedule produces identical `is_leader`
  answers and `expected_vrf_proof` values as oracle for a curated
  epoch-replay corpus.

**Slice Dependencies**: S-B1 (`EraSchedule`, `OutsideForecastRange`), S-B2 (`Nonce`, `PraosChainDepState`, `LeaderScheduleError`), S-B3 (`VrfRole::LeaderEligibility`, `is_leader`, `vrf_input`).

---

## 3. Implementation Instruction (AI)

Implement exactly what is specified. The leader-schedule query is a
**pure function** over `(query, ledger_view, era_schedule,
chain_dep_state)`. It does not own stake — it consumes a typed
`LedgerView` trait.

Commit with the `Co-Authored-By: Claude <model+context>
<noreply@anthropic.com>` trailer.

---

## 4. Intent

Make it impossible for any consensus path to compute leader
eligibility from anything other than the canonical inputs:
`(slot, pool_id, epoch_nonce, stake_snapshot_from_ledger_view,
active_slots_coeff)`. Stake snapshots are owned by the ledger and
**consumed by reference** through a typed boundary. Out-of-range
queries return `OutsideForecastRange` as a structured fail-fast
error — never a guessed answer.

---

## 5. Scope

**Modules / crates**:
- `crates/ade_core/src/consensus/leader_schedule.rs` (NEW)
- `crates/ade_core/src/consensus/ledger_view.rs` (NEW — declares
  the `LedgerView` trait surface; this slice introduces it because
  it's the first consumer)
- `crates/ade_core/src/consensus/mod.rs` (extend — re-exports)
- `crates/ade_core/tests/leader_schedule_corpus.rs` (NEW — closes
  CE-N-B-4)
- `corpus/consensus/leader_schedule/scenario_one_epoch.json` (NEW)
- `crates/ade_testkit/src/consensus/corpus.rs` (extend — leader-
  schedule path helper)
- `crates/ade_testkit/src/consensus/ledger_view_stub.rs` (NEW —
  GREEN test stub implementing `LedgerView` from corpus data)

**State machines affected**: none (pure query).

**Persistence impact**: none.

**Network-visible impact**: none.

**Out-of-scope**:
- Header validation (S-B7)
- Fork choice (S-B8)
- Real ledger-side stake-snapshot computation — that's `ade_ledger`'s
  job; this slice declares only the trait surface BLUE consumes
- Block production / KES signing (out of N-B)

---

## 6. Execution Boundary

**BLUE**:
- `ade_core::consensus::leader_schedule`
- `ade_core::consensus::ledger_view` (trait surface only)

**GREEN**:
- `ade_testkit::consensus::ledger_view_stub` (test-only impl of
  `LedgerView`)

**RED**: none.

---

## 7. Invariants Preserved

- All previous S-B1..S-B5 tests pass.
- No new dep on `ade_runtime`.
- No real stake-snapshot computation in this crate (lives in
  `ade_ledger`).

---

## 8. Invariants Strengthened or Introduced

- **`DC-CONSENSUS-02` strengthened**: leader schedule is `is_leader(
  slot, vrf_key, stake_dist_from_ledger_view, asc, epoch_nonce)`
  exactly; the typed `LedgerView` boundary makes any alternative
  unrepresentable.
- **`CN-EPOCH-01` strengthened**: leader schedule for epoch E
  consumes stake snapshot frozen at E−2 *from the ledger view* — N-B
  does not rederive.
- **`DC-CONS-09` enforced (further)**: forecast horizon now applies
  to leader-schedule queries.

---

## 9. Design Summary

### `LedgerView` trait

```rust
// ledger_view.rs

use ade_types::{Hash28, EpochNo};
use ade_crypto::vrf::VrfVerificationKey;

/// Stake snapshot frozen at epoch E-2, surfaced for the active
/// epoch E. Consumed by-reference; never owned by BLUE consensus.
pub trait LedgerView {
    /// Total active stake (lovelace) across all registered pools
    /// for the current operating epoch.
    fn total_active_stake(&self, epoch: EpochNo) -> Option<u64>;

    /// Active stake for one pool. Returns None if the pool is
    /// unknown to this snapshot.
    fn pool_active_stake(
        &self,
        epoch: EpochNo,
        pool: &Hash28,
    ) -> Option<u64>;

    /// Pool's registered VRF verification key for the operating
    /// epoch. None if unknown.
    fn pool_vrf_key(
        &self,
        epoch: EpochNo,
        pool: &Hash28,
    ) -> Option<VrfVerificationKey>;

    /// Active-slots-coefficient for the operating epoch — pulled
    /// from the era's protocol parameters; ledger surfaces it so
    /// BLUE has one canonical source for f.
    fn active_slots_coeff(
        &self,
        epoch: EpochNo,
    ) -> Option<crate::consensus::vrf_cert::ActiveSlotsCoeff>;
}
```

### Query

```rust
// leader_schedule.rs

use ade_types::{EpochNo, Hash28, SlotNo};
use ade_crypto::vrf::{VrfOutput, VrfProof, VrfVerificationKey};
use crate::consensus::ledger_view::LedgerView;
use crate::consensus::praos_state::{Nonce, PraosChainDepState};
use crate::consensus::era_schedule::EraSchedule;
use crate::consensus::errors::{LeaderScheduleError, OutsideForecastRange};
use crate::consensus::vrf_cert::{is_leader, VrfRole};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderScheduleQuery {
    pub slot: SlotNo,
    pub pool: Hash28,
}

/// Distinct answer types: known/unknown leadership + a deterministic
/// VRF-input hash so callers can verify after-the-fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderScheduleAnswer {
    pub slot:                SlotNo,
    pub pool:                Hash28,
    pub epoch:               EpochNo,
    pub leads:               bool,
    /// The 41-byte VRF input (slot ‖ epoch_nonce ‖ tag=LEADER) the
    /// pool's VRF key must produce a proof over to be valid.
    pub expected_vrf_input:  [u8; 41],
    /// Stake fraction used in the threshold check (numer / denom).
    pub stake_fraction:      (u64, u64),
}

pub fn query_leader_schedule(
    query:        &LeaderScheduleQuery,
    ledger_view:  &dyn LedgerView,
    era_schedule: &EraSchedule,
    state:        &PraosChainDepState,
) -> Result<LeaderScheduleAnswer, LeaderScheduleError>;
```

Algorithm:
1. `era_schedule.check_forecast_horizon(query.slot)` →
   `LeaderScheduleError::OutsideForecastRange { .. }` on failure.
2. `era_schedule.locate(query.slot)?` → `EraLocation { epoch, .. }`,
   mapping `HFCError` → `LeaderScheduleError::HFC`.
3. `ledger_view.pool_vrf_key(epoch, &query.pool)`:
   - None → `LeaderScheduleError::UnknownPool`.
4. `ledger_view.pool_active_stake(epoch, &query.pool)`:
   - None → `LeaderScheduleError::UnknownPool`.
5. `ledger_view.total_active_stake(epoch)`:
   - None or 0 → `LeaderScheduleError::UnknownPool` (an epoch with no
     active stake is not queryable).
6. `ledger_view.active_slots_coeff(epoch)?`:
   - None → `LeaderScheduleError::UnknownPool`.
7. Build `vrf_input(query.slot, &state.epoch_nonce,
   VrfRole::LeaderEligibility)` → 41 bytes.
8. **This slice does not require the caller to provide the VRF proof
   or output.** Instead, `LeaderScheduleAnswer::leads` is computed as
   follows: a pool *can* lead this slot iff there *exists* a VRF
   output `o` such that `is_leader(o, sigma, asc) == true`. Because
   `is_leader` depends on the actual output bytes, this is not a
   pure-of-output question — it's effectively a *threshold probe*
   that the caller will resolve against an actual VRF proof
   (header-validate, S-B7). For this slice, `leads` is set to
   `false` by default and the consumer must call
   `is_leader_for_vrf_output(answer, vrf_output)` (a tiny helper that
   recombines `answer.stake_fraction`, `asc` retrieved during the
   query, and the given output) to decide.
   - **Actually** — to avoid leaking asc back out of the answer, the
     simpler shape is: include `asc` directly in `LeaderScheduleAnswer`.
     Updated answer struct:
     ```rust
     pub struct LeaderScheduleAnswer {
         pub slot:                SlotNo,
         pub pool:                Hash28,
         pub epoch:               EpochNo,
         pub expected_vrf_input:  [u8; 41],
         pub stake_fraction:      (u64, u64),
         pub asc:                 ActiveSlotsCoeff,
     }
     pub fn is_leader_for_vrf_output(
         answer: &LeaderScheduleAnswer,
         output: &VrfOutput,
     ) -> bool { /* delegates to vrf_cert::is_leader */ }
     ```
   - This removes the misleading `leads: bool` field. The answer is
     the **threshold context**; the bool is per-VRF-proof.

> **Implementer note**: implement per the second shape (no `leads`
> field, expose `is_leader_for_vrf_output`). The corpus test in §11
> validates by including `(vrf_output_hex, expected_leads_bool)`
> pairs and calling `is_leader_for_vrf_output`.

---

## 10. Changes Introduced

### Types
- New: `LedgerView` trait, `LeaderScheduleQuery`, `LeaderScheduleAnswer`.

### State Transitions
- New (pure queries): `query_leader_schedule`, `is_leader_for_vrf_output`.

### Persistence
- None.

---

## 11. Replay, Crash, and Epoch Validation

### Corpus

`corpus/consensus/leader_schedule/scenario_one_epoch.json`:

```jsonc
{
  "scenario": "leader_schedule_two_pools_one_epoch",
  "epoch": 300,
  "epoch_nonce_hex": "<32 hex bytes>",
  "stake": [
    { "pool_hex": "<28 hex pool A>", "active_stake": 1_000_000_000_000, "vrf_key_hex": "<32 hex>" },
    { "pool_hex": "<28 hex pool B>", "active_stake": 4_000_000_000_000, "vrf_key_hex": "<32 hex>" }
  ],
  "total_active_stake": 5_000_000_000_000,
  "asc": { "numer": 1, "denom": 20 },
  "queries": [
    {
      "slot": 80_000_000,
      "pool_hex": "<pool A>",
      "expected_stake_fraction": [1_000_000_000_000, 5_000_000_000_000],
      "expected_vrf_input_prefix_hex": "<first few bytes; full 41 bytes also compared>"
    },
    {
      "slot": 80_000_001,
      "pool_hex": "<pool B>",
      "expected_stake_fraction": [4_000_000_000_000, 5_000_000_000_000]
    },
    {
      "slot": 80_000_002,
      "pool_hex": "<28 hex unknown>",
      "expected_error": "UnknownPool"
    }
  ],
  "leader_probe": {
    "slot": 80_000_000,
    "pool_hex": "<pool A>",
    "vrf_output_hex": "<128 hex chars — pinned synthetic>",
    "expected_leads": false
  },
  "horizon_probe": {
    "slot": 999_999_999_999,
    "expected_error": "OutsideForecastRange"
  }
}
```

### Tests

- `crates/ade_core/tests/leader_schedule_corpus.rs`:
  - `corpus_returns_canonical_answer_for_known_pools`
  - `corpus_rejects_unknown_pool`
  - `corpus_rejects_out_of_forecast_horizon`
  - `corpus_is_leader_helper_matches_pinned_probe`
  - `corpus_is_deterministic_across_runs`

- Unit tests in `leader_schedule.rs`:
  - `query_uses_state_epoch_nonce_for_vrf_input`
  - `query_returns_unknown_pool_when_no_vrf_key`
  - `query_returns_outside_forecast_range_for_far_future`
  - `query_does_not_mutate_state` (compile-time guaranteed by `&`)
  - `is_leader_for_vrf_output_delegates_to_vrf_cert`

### Replay impact
- Pure function.

---

## 12. Mechanical Acceptance Criteria

- [ ] `cargo build -p ade_core -p ade_testkit` PASS
- [ ] `cargo test -p ade_core --lib consensus::leader_schedule` PASS
- [ ] `cargo test -p ade_core --test leader_schedule_corpus` PASS —
      this is the **CE-N-B-4 close** test
- [ ] `cargo clippy -p ade_core -p ade_testkit --all-targets -- -D warnings` PASS
- [ ] No `HashMap` / `HashSet`
- [ ] No floating-point
- [ ] `LedgerView` is a trait, not a concrete type — multiple impls
      possible (test stub + future `ade_ledger` impl)

---

## 13. Failure Modes

| Failure | Shape | Fail-fast? |
|---|---|---|
| Pool unknown | `LeaderScheduleError::UnknownPool` | yes |
| Slot past forecast | `LeaderScheduleError::OutsideForecastRange(...)` | yes |
| Slot translation failure | `LeaderScheduleError::HFC(HFCError::...)` | yes |

---

## 14. Hard Prohibitions

### Inherited (from cluster.md)
- BLUE receiving `&ChainDb`, `&Mux`, parsing genesis text
- Wall-clock reads
- `HashMap` / `HashSet`
- Floating-point
- TODO/placeholder error variants
- `async fn` / `tokio` in BLUE
- Stake-snapshot rederivation in N-B

### Slice-specific
- No `Box<dyn LedgerView>` in BLUE production code — pass by
  `&dyn LedgerView` reference.
- No `Option<u64>` exposed as a public answer field — failure is
  always a typed `LeaderScheduleError`.
- No `HashMap` in the testkit stub either; use `BTreeMap`.

---

## 15. Explicit Non-Goals

- Do NOT implement `ade_ledger`'s `LedgerView` impl. That's a
  ledger-side concern picked up in N-E or a later integration slice.
- Do NOT compute stake snapshots.
- Do NOT validate VRF proofs (S-B3 does that; this slice consumes
  it indirectly via `is_leader_for_vrf_output`).
- Do NOT introduce a "leader oracle" tester binary; tests are
  embedded.

---

## 16. Completion Checklist

- [ ] `LedgerView` is a trait with at least one impl in testkit
- [ ] CE-N-B-4 test exists and passes
- [ ] All failure modes typed
- [ ] No TODOs in BLUE
- [ ] Corpus pinned

---

## 17. Review Notes

- The "epoch E consumes E-2 snapshot" semantic is enforced *by the
  ledger* — N-B's job is to consume `total_active_stake(epoch)` and
  trust that the ledger has correctly frozen the E-2 snapshot under
  that key. CE-N-B-4 closure proves the threshold math; CE-N-B-5
  (S-B10) proves the ledger-view boundary holds in the live
  pipeline.

---

## 18. Authority Reminder

Registry > slice doc.
