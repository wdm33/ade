# Invariant Slice: T-25 — Epoch Boundary Logic

> **Status:** Proposed
>
> This document defines the standard slice for epoch boundary transitions
> across Shelley through Conway, including stake snapshots, reward computation,
> protocol parameter updates, and Conway governance ratification/enactment.
> It introduces no new requirements and does not override any normative specification.
> If any content here conflicts with the project constitution or other normative documents,
> the normative documents are authoritative.

---

## 1. What an Invariant Slice Is

An **Invariant Slice** is the smallest unit of work that may be merged into the codebase.

A slice:
- strengthens or enforces a specific invariant,
- preserves *all existing* invariants,
- leaves the system in a fully correct state,
- and is replay-verifiable end-to-end.

A slice is **not** a feature, experiment, or partial implementation.
If it cannot be merged safely, it is not a slice.

---

## 1b. Invariant-Driven Development

This project is not developed by incrementally adding features. It is developed by
**extending a continuously valid correctness proof**.

Work is organized around **invariant-driven slices**:

- A *slice* is a small, closed unit of change that:
  - introduces no invariant violations,
  - is replay-verifiable end-to-end,
  - and leaves the system in a fully correct state at all times.
- A slice may add only minimal behavior, types, or state transitions.
- A slice is complete only when replay, crash recovery, and epoch boundaries behave correctly.

Slices are grouped by **invariant clusters** (e.g., determinism, authority, replay, atomicity),
not by user-visible features.

---

## 2. Slice Header

### Slice Name

T-25: Epoch Boundary Logic

### Cluster

Phase 2C: Ledger Rules Phase 2

### Status

Proposed

### Cluster Exit Criteria Addressed

This slice directly contributes to satisfying the following cluster exit criteria (verbatim from cluster plan):

- [ ] CE-71: Epoch boundary transitions (Shelley through Babbage) zero divergence: stake snapshot rotation, reward distribution, and protocol parameter updates match oracle at every corpus epoch boundary. At least one epoch boundary tested per era.
- [ ] CE-72: Conway epoch boundary with atomic pulser proof: DRep stake computation, governance proposal ratification, and governance enactment match oracle at all Conway corpus epoch boundaries. Required proofs documented: same results, same enactment effects, no causal leakage, no replay divergence.
- [ ] CE-74: `ci_check_ledger_determinism.sh` passes: applies same block sequence twice, asserts identical state hashes. Covers all 7 eras. Tests both single-block and multi-block sequences.
- [ ] CE-75: `ci_check_differential_divergence.sh` passes: runs differential ledger harness on expanded corpus (1,500+ blocks per era), reports zero divergence on all non-Plutus-dependent blocks. Version-scoped to cardano-node 10.6.2. (Epoch boundary portion.)
- [ ] CE-78: Registry status transitions completed: DC-LEDGER-01 -> `status = "enforced"`; DC-LEDGER-02 -> `status = "partial"`; DC-LEDGER-03/04/05 -> `status = "partial"`; DC-EPOCH-01 -> `status = "partial"` (Conway governance, Plutus deferred); DC-EPOCH-02 -> `status = "partial"` (HFC ledger-side, consensus deferred); T-CONSERV-01 -> `status = "enforced"`; T-NOSPEND-01 -> `status = "enforced"`. DC-EPOCH-01 wording revised per pulser reconciliation.
- [ ] CE-79: Four-tier gate statement documented: true (purity/determinism), derived (non-Plutus corpus equivalence), release (non-Plutus certification only), non-goal (no partial Plutus). Gate statement is part of cluster completion evidence.
- [ ] CE-80: All prior exit criteria (CE-01 through CE-57) still pass. No regression. `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.

Exit criteria not listed here are explicitly out of scope for this slice.

### Slice Dependencies

This slice assumes the following slices have been completed and merged:

- T-24 — Alonzo/Babbage/Conway Transaction Validation Phase 1

T-24 provides era-specific transaction validation, governance action types, DRep certificate types, committee certificate types, `ScriptVerdict` enum, and the structural validation framework for Conway blocks. Epoch boundary logic builds on these types to implement ratification and enactment at epoch transitions.

---

## 3. Implementation Instruction (AI)

> **READ THIS SECTION FIRST BEFORE WRITING ANY CODE.**

Implement exactly what is specified in this slice.
Do not invent new behavior.
Do not add "helpful" refactors, abstractions, or conveniences beyond what is required.
If a requirement is ambiguous, stop and ask.
If an invariant cannot be enforced mechanically, do not approximate it.
Refer to the **Hard Prohibitions** (S14) and **Explicit Non-Goals** (S15) before writing any code.
The **Mechanical Acceptance Criteria** (S12) define the only way to prove this slice is complete.

**Deny attribute note**: All BLUE crates enforce `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`, `#![deny(clippy::float_arithmetic)]`. Every epoch boundary path must return `Result<T, LedgerError>`.

**Conway pulser equivalence is a hard gate**: This slice proposes computing DRep stake distribution atomically at the epoch boundary rather than spreading it across the epoch via the Haskell `DRepPulser`. This is an improving reinterpretation that preserves the same observable semantics more simply. However, if differential comparison reveals ANY divergence at a Conway epoch boundary that traces to pulser behavior, the atomic approach MUST be abandoned in favor of matching the pulsed approach exactly. Do not approximate. Do not paper over differences. Either prove equivalence or match the reference.

**Epoch boundary computations are pure functions**: They take the current `EpochState` and produce the next `EpochState`. No I/O, no wall-clock time, no randomness, no mutation. The epoch number is an input parameter, not derived from a clock.

---

## 4. Intent

Make it impossible for Ade to compute different epoch boundary state than the Haskell node. Epoch boundaries trigger stake snapshots, reward computation, protocol parameter updates, and (in Conway) governance ratification and enactment. These are the most complex pure computations in the ledger. This slice ensures that identical block sequences produce identical post-epoch-boundary state hashes, regardless of computation scheduling strategy.

---

## 5. Scope

### Modules / crates

**BLUE** -- `crates/ade_ledger/`:
- `src/epoch.rs` -- epoch boundary transition logic: stake snapshot rotation (mark/set/go), reward computation, protocol parameter update application, pool retirement
- `src/governance.rs` -- Conway governance ratification and enactment (extends T-24 governance types)
- `src/delegation.rs` -- stake snapshot types: `MarkSnapshot`, `SetSnapshot`, `GoSnapshot`, stake distribution computation
- `src/state.rs` -- `EpochState` expanded with epoch boundary state, snapshot fields, reward accounts, governance state

**GREEN** -- `crates/ade_testkit/`:
- Epoch boundary differential comparison harness: applies blocks across epoch boundaries via `LedgerApplicator`, compares state hashes at every epoch boundary against oracle reference

**CI**:
- `ci/ci_check_ledger_determinism.sh` -- extended to cover epoch boundary transitions
- `ci/ci_check_differential_divergence.sh` -- extended to verify zero divergence across epoch boundaries

### State machines affected

Epoch boundary is a pure state transition: `EpochState(N) -> EpochState(N+1)` at the epoch boundary. No persistent state machines. No protocol state machines.

### Persistence impact

None. Epoch boundary is a pure computation. Persistence of epoch state is a Phase 5 concern.

### Network-visible impact

None. Epoch boundary computation is internal ledger logic with no network messages.

### Out of scope

- HFC era translation functions (T-26)
- Plutus script evaluation (even for governance proposals containing scripts)
- Consensus chain selection, leadership verification (Phase 4)
- Slot-to-epoch mapping (epoch boundary takes epoch number as input)
- Leader selection using stake distribution (Phase 4)
- Block production (Phase 7)
- Persistence of epoch state (Phase 5)

---

## 6. Execution Boundary

### BLUE (deterministic, authoritative)

- `crates/ade_ledger/src/epoch.rs` -- epoch boundary transition: snapshot rotation, reward computation, parameter updates, pool retirement, Conway governance ratification/enactment
- `crates/ade_ledger/src/governance.rs` -- Conway governance ratification logic (DRep stake computation, committee votes, SPO votes, threshold evaluation), enactment ordering and application
- `crates/ade_ledger/src/delegation.rs` -- stake snapshot types and stake distribution computation
- `crates/ade_ledger/src/state.rs` -- `EpochState` expanded with epoch boundary fields

### GREEN (deterministic glue, non-authoritative)

- Epoch boundary differential comparison harness in `ade_testkit`: drives `apply_block` across epoch boundaries and compares state hashes against oracle at each boundary

### RED (nondeterministic shell)

None.

---

## 7. Invariants Preserved

All invariants established by prior slices are preserved:

- All Phase 0A/0B/1/2A invariants
- T-19 through T-24 invariants (reference pipeline, corpus, UTxO model, era-specific validation)
- DC-LEDGER-01 -- `apply_block` pure and deterministic
- T-CONSERV-01 -- conservation law: `consumed == produced` with explicit exceptions
- T-NOSPEND-01 -- double-spend rejection
- DC-CRYPTO-01/02 -- cryptographic verification matches oracle; no signing in BLUE
- T-ENC-03 / DC-CBOR-01 / DC-CBOR-02 -- encoding round-trip and wire-byte preservation
- All prior exit criteria (CE-01 through CE-70)

---

## 8. Invariants Strengthened or Introduced

| Invariant | How Strengthened |
|-----------|-----------------|
| DC-LEDGER-01 | Enforced -- `apply_block` remains pure and deterministic across epoch boundaries. Epoch boundary transitions are deterministic pure functions with no hidden state. |
| DC-LEDGER-02 | Partial -- error taxonomy extended with `EpochTransitionError` variants. |
| DC-LEDGER-04 | Partial -- epoch boundary state hashes match oracle across all corpus epoch boundaries (version-scoped to 10.6.2). |
| DC-EPOCH-01 | Partial -- Conway governance timing enforced: proposals accumulate during epoch; ratification and enactment are atomic at epoch boundary; DRep stake distribution used for ratification is derived solely from canonical chain state and is identical regardless of computation scheduling. Wording revised per pulser reconciliation. |
| DC-EPOCH-02 | Partial -- epoch boundary transitions implemented for Shelley through Conway. HFC consensus-side deferred to Phase 4. |
| T-CONSERV-01 | Enforced -- conservation law verified across epoch boundaries (rewards, treasury, pool retirement do not violate conservation when protocol-authorized exceptions are accounted for). |
| T-NOSPEND-01 | Enforced -- double-spend rejection continues to hold across epoch boundaries. |

---

## 9. Design Summary

### Epoch boundary as pure state transition

The epoch boundary is modeled as a pure function:

```
apply_epoch_boundary(state: EpochState, epoch: EpochNo) -> Result<EpochState, LedgerError>
```

This function is called when the first block of a new epoch is applied. It transforms the ledger state according to the rules of the current era. The function is deterministic: same inputs produce identical outputs on every invocation.

### Stake snapshot rotation (Shelley through Conway)

Three snapshots rotate at each epoch boundary:

- **Mark snapshot** (epoch N): current stake distribution captured at epoch N boundary
- **Set snapshot** (epoch N-1): previous mark becomes set
- **Go snapshot** (epoch N-2): previous set becomes go (used for leader selection in the current epoch)

At each epoch boundary: `go <- set`, `set <- mark`, `mark <- current_delegation_state`. This is a pure data rotation with no I/O.

### Reward computation (Shelley through Conway)

Rewards are computed per epoch boundary based on the go snapshot:

1. **Pool rewards**: calculated from stake, relative pool performance, pledge influence, and protocol parameters (a0, rho, tau)
2. **Member rewards**: distributed proportionally to delegators based on their stake in the go snapshot
3. **Treasury**: remainder (after pool and member rewards) directed to treasury
4. **Reserve depletion**: rewards drawn from reserves according to monetary expansion rate

All arithmetic uses integer operations. No floating point. Reward computation is a pure function of the go snapshot, pool performance data, and protocol parameters.

### Protocol parameter updates (all eras)

Parameter update proposals submitted in epoch N-2 are applied at the boundary of epoch N. The update mechanism:

1. Collect all `ProposedPPUpdates` from the previous epoch
2. If sufficient governance authority exists (quorum met), apply the update
3. New parameters take effect for the new epoch

### Pool retirement (Shelley through Conway)

Pools with `retireEpoch == currentEpoch` are retired at the epoch boundary. Their stake is returned to delegators. Retirement is deterministic and based solely on the registered retirement epoch.

### Conway governance ratification

At the Conway epoch boundary, governance proposals are evaluated for ratification:

1. **DRep stake distribution**: computed atomically from the current delegation state. This replaces the Haskell `DRepPulser` approach -- see pulser proof obligation below.
2. **Committee votes**: counted from committee member votes on each proposal
3. **SPO votes**: counted from stake pool operator votes, weighted by stake
4. **Threshold evaluation**: each proposal type has a specific ratification threshold. Proposals meeting their threshold are ratified.

Ratification order is deterministic: proposals are evaluated in the order they appear in the governance state (by `GovActionId`).

### Conway governance enactment

Ratified proposals are enacted in deterministic order at the epoch boundary:

1. Parameter changes applied
2. Hard fork initiation recorded
3. Treasury withdrawals executed
4. Committee/constitution changes applied
5. No-confidence results processed

Enactment ordering follows the Haskell reference: proposals are enacted in `GovActionId` order within each enactment priority class. The priority ordering is defined by the Conway specification.

### Proposal and DRep expiry

- **Proposal expiry**: proposals past their `expiryEpoch` are removed from governance state
- **DRep expiry**: DReps whose last activity is more than `drepActivity` epochs ago are marked inactive and excluded from ratification quorum calculations

### Conway pulser equivalence -- explicit proof obligation

The Haskell implementation uses `DRepPulser` to spread DRep stake distribution computation across the epoch to avoid a computation spike at the epoch boundary. Ade computes this atomically at the epoch boundary.

**Classification**: Improving reinterpretation that preserves the same observable semantics more simply.

**Required proofs** (must be delivered in this slice):

1. **Same results**: Atomic computation produces identical DRep stake distribution to pulsed computation. Proven by differential comparison on Conway epoch boundaries in corpus.
2. **Same epoch-boundary enactment effects**: Ratification decisions and enactment ordering identical to oracle at every Conway epoch boundary.
3. **No causal leakage**: No intermediate pulsing state affects any validation decision within the epoch. Proven by demonstrating that no validation path reads pulsing intermediate state.
4. **No replay divergence**: Same blocks produce same state hash at epoch boundary. Proven by `ci_check_ledger_determinism.sh` and `ci_check_differential_divergence.sh` across Conway epoch boundaries.

**Hard gate**: If differential comparison reveals ANY divergence at a Conway epoch boundary that traces to pulser behavior, the atomic approach MUST be abandoned in favor of matching the pulsed approach exactly.

### DC-EPOCH-01 wording revision

This slice proposes revised wording for DC-EPOCH-01 in the classification table:

> "Conway governance timing: proposals accumulate during epoch; ratification and enactment are atomic at epoch boundary; DRep stake distribution used for ratification is derived solely from canonical chain state and is identical regardless of computation scheduling."

This replaces any reference-structural wording with observable-outcome wording. The registry update occurs when this slice merges.

---

## 10. Changes Introduced

### Types

| Type | Location | Purpose |
|------|----------|---------|
| `MarkSnapshot` | `ade_ledger/src/delegation.rs` | Stake distribution snapshot taken at current epoch boundary |
| `SetSnapshot` | `ade_ledger/src/delegation.rs` | Previous mark snapshot, used for reward calculation |
| `GoSnapshot` | `ade_ledger/src/delegation.rs` | Previous set snapshot, used for leader selection |
| `StakeDistribution` | `ade_ledger/src/delegation.rs` | Mapping of stake credentials to stake amounts (BTreeMap) |
| `RewardUpdate` | `ade_ledger/src/epoch.rs` | Computed rewards for an epoch: pool rewards, member rewards, treasury delta, reserve delta |
| `PoolReward` | `ade_ledger/src/epoch.rs` | Per-pool reward computation result |
| `EpochTransitionResult` | `ade_ledger/src/epoch.rs` | Result of applying epoch boundary: new state + diagnostic info |
| `RatificationResult` | `ade_ledger/src/governance.rs` | Result of evaluating governance proposals: ratified proposals list, expired proposals list |
| `EnactmentResult` | `ade_ledger/src/governance.rs` | Result of enacting ratified proposals: parameter changes, committee changes, treasury withdrawals |
| `DRepStakeDistribution` | `ade_ledger/src/governance.rs` | DRep to voting stake mapping, computed atomically |

### Functions

| Function | Location | Signature |
|----------|----------|-----------|
| `apply_epoch_boundary` | `ade_ledger/src/epoch.rs` | `(state: &EpochState, epoch: EpochNo) -> Result<EpochState, LedgerError>` |
| `rotate_snapshots` | `ade_ledger/src/epoch.rs` | `(state: &EpochState) -> SnapshotState` |
| `compute_rewards` | `ade_ledger/src/epoch.rs` | `(go_snapshot: &GoSnapshot, params: &ProtocolParameters, pool_perf: &PoolPerformance) -> Result<RewardUpdate, LedgerError>` |
| `apply_parameter_updates` | `ade_ledger/src/epoch.rs` | `(params: &ProtocolParameters, updates: &ProposedPPUpdates) -> Result<ProtocolParameters, LedgerError>` |
| `retire_pools` | `ade_ledger/src/epoch.rs` | `(state: &EpochState, epoch: EpochNo) -> Result<EpochState, LedgerError>` |
| `compute_drep_stake_distribution` | `ade_ledger/src/governance.rs` | `(delegation_state: &DelegationState) -> DRepStakeDistribution` |
| `ratify_proposals` | `ade_ledger/src/governance.rs` | `(proposals: &GovState, drep_stake: &DRepStakeDistribution, committee: &Committee, spo_stake: &StakeDistribution, params: &ProtocolParameters) -> RatificationResult` |
| `enact_proposals` | `ade_ledger/src/governance.rs` | `(state: &EpochState, ratified: &[RatifiedProposal]) -> Result<EpochState, LedgerError>` |
| `expire_proposals` | `ade_ledger/src/governance.rs` | `(gov_state: &GovState, epoch: EpochNo) -> GovState` |
| `expire_dreps` | `ade_ledger/src/governance.rs` | `(delegation_state: &DelegationState, epoch: EpochNo, activity_period: EpochInterval) -> DelegationState` |

### State Transitions

| Transition | Description |
|------------|-------------|
| `EpochState(N) -> EpochState(N+1)` | Full epoch boundary transition: snapshot rotation, reward computation, parameter updates, pool retirement, Conway governance (if Conway era) |

### Persistence

None. Epoch boundary is a pure computation. Persistence is a Phase 5 concern.

### Removal / Refactors

None.

### Registry Update

- DC-LEDGER-01 -> `status = "enforced"` (apply_block remains pure/deterministic across epoch boundaries)
- DC-LEDGER-02 -> `status = "partial"` (error taxonomy extended)
- DC-LEDGER-03 -> `status = "partial"` (version-scoped, Plutus deferred)
- DC-LEDGER-04 -> `status = "partial"` (epoch boundary matches oracle)
- DC-LEDGER-05 -> `status = "partial"` (witness binding, Plutus deferred)
- DC-EPOCH-01 -> `status = "partial"` (Conway governance timing -- wording revised per pulser reconciliation)
- DC-EPOCH-02 -> `status = "partial"` (HFC ledger-side, consensus deferred)
- T-CONSERV-01 -> `status = "enforced"` (conservation verified across epoch boundaries)
- T-NOSPEND-01 -> `status = "enforced"` (double-spend rejection across epoch boundaries)

---

## 11. Replay, Crash, and Epoch Validation

- **Epoch boundary determinism tests**: `apply_epoch_boundary` applied twice to the same `EpochState` produces identical output. Tested for each era (Shelley, Allegra, Mary, Alonzo, Babbage, Conway).
- **Stake snapshot rotation tests**: Mark/set/go rotation produces correct snapshot state at each boundary. Tested with known stake distributions and verified against oracle.
- **Reward computation tests**: Reward distribution for known pool configurations matches oracle output. Covers: single pool, multiple pools, pools with pledge, pools with varying performance, empty epoch (no blocks produced).
- **Protocol parameter update tests**: Parameter updates proposed in epoch N-2 applied correctly at epoch N boundary. Tested with known update proposals and verified against oracle.
- **Pool retirement tests**: Pools scheduled for retirement at current epoch are correctly retired. Stake returned to delegators verified.
- **Conway governance ratification tests**: DRep stake distribution, committee votes, SPO votes computed correctly. Proposals meeting thresholds ratified. Proposals below thresholds not ratified. Tested against oracle at Conway corpus epoch boundaries.
- **Conway governance enactment tests**: Ratified proposals enacted in correct order. Parameter changes, treasury withdrawals, committee changes applied correctly. Verified against oracle.
- **Conway pulser equivalence proof**: Atomic DRep stake computation produces identical results to Haskell pulsed computation at every Conway epoch boundary in corpus. If any divergence traces to pulser behavior, test fails and atomic approach must be abandoned.
- **Proposal and DRep expiry tests**: Expired proposals removed. Inactive DReps marked correctly. Verified against oracle.
- **Differential comparison across epoch boundaries**: `ci_check_differential_divergence.sh` runs on all corpus epoch boundaries and reports zero divergence.
- **Cross-epoch conservation tests**: Conservation law holds across epoch boundaries when protocol-authorized exceptions (rewards, treasury) are accounted for.

No crash recovery behavior -- epoch boundary is a pure stateless function (stateless in the sense that it takes state as input and returns new state, with no side effects or persistence).

---

## 12. Mechanical Acceptance Criteria

This slice is complete only when **all** of the following exist and pass in CI:

- [ ] `apply_epoch_boundary` is pure and deterministic: same `EpochState` + same `EpochNo` produces identical output on every invocation, for all eras
- [ ] Stake snapshot rotation (mark/set/go) produces correct snapshots at every corpus epoch boundary, verified against oracle
- [ ] Reward computation matches oracle at every Shelley through Babbage corpus epoch boundary: pool rewards, member rewards, treasury delta, reserve delta identical
- [ ] Protocol parameter updates applied at correct epoch boundaries, verified against oracle
- [ ] Pool retirement executed at correct epoch boundaries, verified against oracle
- [ ] Conway DRep stake distribution computed atomically produces identical results to Haskell pulsed computation (differential comparison on all Conway corpus epoch boundaries)
- [ ] Conway governance ratification decisions identical to oracle at every Conway corpus epoch boundary
- [ ] Conway governance enactment ordering and effects identical to oracle at every Conway corpus epoch boundary
- [ ] Conway proposal expiry correct at every Conway corpus epoch boundary
- [ ] Conway DRep expiry correct at every Conway corpus epoch boundary
- [ ] Pulser equivalence proofs documented: same results, same enactment effects, no causal leakage, no replay divergence
- [ ] DC-EPOCH-01 wording revised in classification table to observable-outcome wording
- [ ] Registry status transitions completed per CE-78
- [ ] Four-tier gate statement documented per CE-79: true -- `apply_block`/`apply_epoch_boundary` pure/deterministic; derived -- matches oracle on non-Plutus corpus; release -- non-Plutus equivalence only; non-goal -- Plutus deferred
- [ ] `ci_check_differential_divergence.sh` passes across all epoch boundaries in corpus (zero divergence)
- [ ] `ci_check_ledger_determinism.sh` passes across epoch boundaries (all eras)
- [ ] Conway epoch boundaries: zero divergence where reference data available, structural validation where pending
- [ ] Conservation law holds across epoch boundaries (property tests with protocol-authorized exceptions)
- [ ] No `.unwrap()`, `.expect()`, or `panic!()` in BLUE epoch/governance/delegation paths
- [ ] No `unsafe` code in epoch.rs, governance.rs, delegation.rs, state.rs
- [ ] All new BLUE source files have contract headers and deny attributes
- [ ] All prior exit criteria (CE-01 through CE-70) still pass (CE-80)
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass

---

## 13. Failure Modes

| Failure | Error Shape | Behavior | Replay Impact |
|---------|------------|----------|---------------|
| Epoch boundary computation failure on valid chain | `Err(LedgerError::EpochTransition { epoch, era, detail })` | Fail-fast, deterministic | Fatal -- same failure on replay (deterministic) |
| Reward computation overflow | `Err(LedgerError::EpochTransition { epoch, era, detail: "reward computation overflow" })` | Fail-fast, deterministic | Fatal -- same failure on replay |
| Protocol parameter update with invalid values | `Err(LedgerError::EpochTransition { epoch, era, detail: "invalid parameter update" })` | Fail-fast, deterministic | Fatal -- same failure on replay |
| Conway ratification threshold evaluation failure | `Err(LedgerError::EpochTransition { epoch, era, detail: "ratification evaluation failure" })` | Fail-fast, deterministic | Fatal -- same failure on replay |
| Conway enactment ordering violation | `Err(LedgerError::EpochTransition { epoch, era, detail: "enactment ordering violation" })` | Fail-fast, deterministic | Fatal -- same failure on replay |

All failure modes are deterministic. Epoch boundary computations on a valid canonical chain should not produce errors -- these error paths exist to ensure deterministic failure if the impossible occurs. Every error carries the epoch number, era, and structured detail for first-divergence localization.

All epoch boundary computations are pure -- no I/O, no wall-clock, no randomness. The same inputs always produce the same result (whether success or failure).

---

## 14. Hard Prohibitions

### Inherited Cluster-Level Prohibitions

This slice inherits and MUST comply with all forbidden patterns defined in the Phase 2B cluster plan's "Forbidden Patterns (Cluster-Level)" section, including:

- `HashMap` or `HashSet` in BLUE code (use `BTreeMap`, `BTreeSet`)
- `std::time::Instant` or `std::time::SystemTime` (wall-clock)
- `f32` or `f64` (floating point)
- `std::fs` or `std::net` (I/O)
- `tokio` or any async runtime
- `async fn` in BLUE paths
- `rand::thread_rng` or unseeded randomness
- `thread::spawn` or threading primitives
- `anyhow` or unstructured error types
- `String` errors in authoritative paths
- `cfg(feature = ...)` or other semantic conditional compilation
- Private key or signing operations (signing belongs in RED shell only; verification is allowed)
- TODOs, placeholders, or deferred validation in authoritative code
- Re-encoding for hash computation -- hash paths MUST use wire bytes
- `.unwrap()`, `.expect()`, or `panic!()` in BLUE codec/crypto/ledger paths
- `unsafe` code in `ade_ledger` (no FFI, no raw pointer operations)
- Ledger code in `ade_core` (all ledger logic in `ade_ledger`)
- Partial Plutus evaluation (Plutus scripts produce `ScriptVerdict::NotYetEvaluated`)
- Unversioned query-like APIs
- Consensus-level era awareness in HFC translation
- Mutable state in `apply_block`
- Implicit conservation exceptions
- Version-independent equivalence claims
- Mocking or stubbing ledger logic

### Slice-Specific Prohibitions

The following are strictly forbidden in this slice:

- **No wall-clock time in epoch boundary logic** -- epoch number is an input parameter, never derived from a system clock. No `Instant`, `SystemTime`, `Duration`, or any temporal type.
- **No Plutus execution** -- even for governance proposals that contain Plutus scripts. Plutus evaluation is Phase 3. Governance proposals with Plutus components are structurally validated only.
- **No signing operations** -- epoch boundary logic is verification and computation only.
- **No scheduling or timing-dependent computation** -- DRep stake distribution is computed atomically. No pulsing, no incremental spreading, no background computation. The computation is a single pure function call at epoch boundary.
- **No floating-point arithmetic in reward computation** -- all reward arithmetic uses integer operations with explicit rounding rules matching the Haskell reference.
- **No query-like APIs** -- epoch boundary logic is not exposed as a queryable service. It is an internal state transition.
- **No pre-baked version assumptions** -- no hardcoded protocol parameter values or era-specific constants that assume a specific chain history. All values come from the ledger state.

**If any of these appear, the slice is incorrect.**

---

## 15. Explicit Non-Goals

This slice MUST NOT:

- Implement HFC era translation functions (T-26)
- Implement Plutus script evaluation, even for governance proposals
- Implement consensus chain selection or leadership verification (Phase 4)
- Implement slot-to-epoch mapping (epoch boundary takes epoch number as input)
- Implement leader selection using stake distribution (Phase 4)
- Implement block production (Phase 7)
- Implement persistence of epoch state (Phase 5)
- Add feature flags or configuration switches
- Optimize reward computation for performance (correctness first)
- Prepare for future behavior not required by this slice
- Modify any existing types, modules, or crate structure outside the stated scope
- Make version-independent equivalence claims

Any work outside the stated scope is scope creep and must be rejected.

---

## 16. Completion Checklist

This slice may be merged only when **all** items are satisfied:

- [ ] `apply_epoch_boundary` exists as a pure function in `ade_ledger/src/epoch.rs`
- [ ] Stake snapshot rotation (mark/set/go) implemented and verified against oracle
- [ ] Reward computation implemented with integer arithmetic and verified against oracle
- [ ] Protocol parameter updates applied at correct epoch boundaries
- [ ] Pool retirement implemented and verified against oracle
- [ ] Conway governance ratification implemented (DRep stake, committee votes, SPO votes, thresholds)
- [ ] Conway governance enactment implemented in correct order
- [ ] Conway proposal and DRep expiry implemented
- [ ] Atomic DRep stake computation proven equivalent to Haskell pulsed computation (or abandoned in favor of matching pulsed approach)
- [ ] DC-EPOCH-01 wording revised to observable-outcome wording in classification table
- [ ] All epoch boundary state is replay-derivable
- [ ] All epoch boundary data is canonically encoded for state hashing
- [ ] All failure modes are deterministic with structured `LedgerError` variants
- [ ] No TODOs or placeholders in authoritative (BLUE) paths
- [ ] CI enforces epoch boundary invariants (`ci_check_ledger_determinism.sh`, `ci_check_differential_divergence.sh`)
- [ ] Registry status transitions completed per CE-78
- [ ] Four-tier gate statement documented per CE-79
- [ ] Replay-equivalence tests pass across runs
- [ ] All prior exit criteria still pass (CE-80)

---

## 17. Review Notes

- **Reward computation arithmetic**: The Haskell node uses a specific fixed-point arithmetic library for reward computation. Ade must match the rounding behavior exactly. This is a known source of subtle divergence -- careful attention to integer division rounding direction (truncation vs. floor) is critical. Test with edge cases: very small pools, pools with zero performance, pools with maximum pledge, single-lovelace rewards.
- **Conway pulser equivalence risk**: The atomic computation approach is simpler but carries risk. The Haskell `DRepPulser` was introduced for performance reasons, not correctness reasons, so the observable result should be identical. However, any subtle ordering dependency in how pulsing accumulates stake could produce different results. The differential comparison on Conway epoch boundaries is the definitive test. If equivalence cannot be demonstrated, matching the pulsed approach exactly is the fallback -- this is acceptable because the goal is correctness, not simplicity.
- **Snapshot timing edge cases**: The mark/set/go rotation must happen at exactly the right boundary. Off-by-one errors in epoch numbering are a classic source of divergence. Test with the first epoch boundary in each era and with epoch boundaries immediately following hard forks.
- **Conway governance enactment ordering**: The Conway specification defines a priority ordering for enactment. Multiple ratified proposals of the same type must be enacted in `GovActionId` order. Verify this ordering matches the Haskell reference exactly.
- **DRep activity tracking**: DRep activity period tracking must match the Haskell reference exactly. The definition of "activity" (which actions reset the activity counter) must be verified against the Conway specification and oracle.
- **Follow-up slices implied**: T-26 (HFC era translation) depends on this slice. Phase 3 (Plutus) will need to handle governance proposals containing Plutus scripts. Phase 4 (consensus) will consume the go snapshot for leader selection.
- **Conservation across epoch boundaries**: Rewards, treasury deposits, and reserve depletion are protocol-authorized exceptions to the conservation law. The total system value (UTxO + reserves + treasury + reward accounts) must remain constant across epoch boundaries. This is a property-testable invariant.

---

## 18. Authority Reminder

This template is a planning and review aid only.

Correctness requirements are defined exclusively by:
- the project constitution (`ade_replay_first_constitutional_node_plan_v1.md` S2--S4b),
- `01_core_determinism_and_contract.md`,
- `classification_table.md`,
- and other normative specifications.

If there is ever a conflict:

> **Normative documents and CI enforcement are authoritative.**
