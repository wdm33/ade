# Invariant Slice: T-24 — Alonzo/Babbage/Conway Transaction Validation (Phase 1)

> **Status:** Proposed
>
> This document defines the standard slice for Alonzo, Babbage, and Conway structural transaction validation with explicit Plutus deferral.
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

T-24: Alonzo/Babbage/Conway Transaction Validation (Phase 1)

### Cluster

Phase 2C: Ledger Rules Phase 2

### Status

Proposed

### Cluster Exit Criteria Addressed

This slice directly contributes to satisfying the following cluster exit criteria (verbatim from cluster plan):

- [ ] CE-68: Alonzo structural validation: all Alonzo corpus blocks structurally validated. Inputs resolved, fees checked, witnesses verified, validity intervals enforced. Plutus scripts produce `ScriptVerdict::NotYetEvaluated`. Collateral inputs validated structurally. Datum hashes checked for presence.
- [ ] CE-69: Babbage structural validation: all Babbage corpus blocks structurally validated with same Plutus deferral as CE-68. Babbage adds: reference inputs, inline datums, reference scripts. Structural presence validated; Plutus evaluation deferred.
- [ ] CE-70: Conway structural validation: all Conway corpus blocks structurally validated with same Plutus deferral. Conway adds: governance actions, DRep certificates, committee certificates, treasury withdrawals. Structural validity checked; Plutus evaluation deferred.
- [ ] CE-74: `ci_check_ledger_determinism.sh` passes: applies same block sequence twice, asserts identical state hashes. Covers all 7 eras. Tests both single-block and multi-block sequences.
- [ ] CE-75: `ci_check_differential_divergence.sh` passes: runs differential ledger harness on expanded corpus (1,500+ blocks per era), reports zero divergence on all non-Plutus-dependent blocks. Version-scoped to cardano-node 10.6.2.
- [ ] CE-77: `ScriptVerdict` enum exists with three variants: `NativeScriptPassed`, `NativeScriptFailed(NativeScriptError)`, `NotYetEvaluated`. Native scripts fully evaluated on all corpus blocks containing native scripts. Plutus scripts produce `NotYetEvaluated`.
- [ ] CE-80: All prior exit criteria (CE-01 through CE-57) still pass. No regression. `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.

Exit criteria not listed here are explicitly out of scope for this slice.

### Slice Dependencies

This slice assumes the following slices have been completed and merged:

- T-23 — Shelley/Allegra/Mary Transaction Validation

T-23 establishes Shelley-era key witness verification, delegation certificate validation, native script evaluation, multi-asset value arithmetic, fee validation, and validity interval checking. T-24 extends these rules incrementally for Alonzo, Babbage, and Conway eras.

---

## 3. Implementation Instruction (AI)

> **READ THIS SECTION FIRST BEFORE WRITING ANY CODE.**

Implement exactly what is specified in this slice.
Do not invent new behavior.
Do not add "helpful" refactors, abstractions, or conveniences beyond what is required.
If a requirement is ambiguous, stop and ask.
If an invariant cannot be enforced mechanically, do not approximate it.
Refer to the **Hard Prohibitions** (§14) and **Explicit Non-Goals** (§15) before writing any code.
The **Mechanical Acceptance Criteria** (§12) define the only way to prove this slice is complete.

**CRITICAL: Plutus Deferral Gate (four-tier)**

| Tier | Statement |
|------|-----------|
| **true** | `apply_block` is pure, deterministic, and replayable for ALL blocks including those containing Plutus scripts. The function itself is deterministic — script evaluation is the deferred component, not purity. |
| **derived** | Matches oracle for non-Plutus-dependent transactions (partial DC-LEDGER-03). Plutus-dependent transactions receive structural validation only. |
| **release** | Phase 2C certifies non-Plutus equivalence only. Plutus equivalence is a Phase 3 release gate. |
| **non-goal** | No partial Plutus evaluation. No approximate budget. No Plutus shortcut. The boundary is clean: native scripts fully evaluated, Plutus scripts produce `NotYetEvaluated`. |

**THIS SLICE ADDS THREE ERAS BUT NO PLUTUS EXECUTION.** Every Plutus script encountered must produce `ScriptVerdict::NotYetEvaluated`. This is not an error, not an approximation, and not a shortcut — it is the explicit deferral boundary between Phase 2C (structural validation) and Phase 3 (Plutus evaluation). Transactions requiring Plutus are categorized "script-execution-deferred" in the differential harness, NOT as "passing" and NOT as "divergent."

**Deny attribute note**: All BLUE crates enforce `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`, `#![deny(clippy::float_arithmetic)]`. Every function must return `Result<T, LedgerError>`.

---

## 4. Intent

Make it impossible for Ade to accept or reject a non-Plutus transaction differently from the Haskell node across Alonzo, Babbage, and Conway eras. Plutus script execution is explicitly deferred — transactions requiring Plutus return `ScriptVerdict::NotYetEvaluated` and are categorized as "script-execution-deferred" in the differential harness, NOT as "passing" and NOT as "divergent."

---

## 5. Scope

### Modules / crates

- `ade_ledger/src/alonzo.rs` (BLUE) — Alonzo structural validation: datum hashes, collateral, script witness presence, redeemer presence, Plutus deferral
- `ade_ledger/src/babbage.rs` (BLUE) — Babbage structural validation: inline datums, reference scripts, reference inputs, collateral return
- `ade_ledger/src/conway.rs` (BLUE) — Conway structural validation: governance actions, DRep/committee certificates, treasury donation, combined delegation certificates
- `ade_ledger/src/governance.rs` (BLUE) — Conway governance types: `ConwayGovState`, `Proposals`, `ProposalProcedure`, `RatifyState`
- `ade_ledger/src/rules.rs` (BLUE) — era dispatch updated with Alonzo/Babbage/Conway branches
- GREEN: differential harness extended with "script-execution-deferred" categorization for Plutus-dependent blocks

### State machines affected

None.

### Persistence impact

None.

### Network-visible impact

None.

### Out of scope

- Plutus script execution (Phase 3)
- Plutus cost model application (Phase 3)
- Script context construction (Phase 3)
- UPLC evaluation (Phase 3)
- Epoch boundary transitions (T-25)
- HFC era translation functions (T-26)
- Consensus logic (Phase 4)
- Full Conway governance enactment (T-25)
- Native script evaluation changes (already complete in T-23)

---

## 6. Execution Boundary

### BLUE (deterministic, authoritative)

- `crates/ade_ledger/src/alonzo.rs` — Alonzo structural transaction validation
- `crates/ade_ledger/src/babbage.rs` — Babbage structural transaction validation
- `crates/ade_ledger/src/conway.rs` — Conway structural transaction validation
- `crates/ade_ledger/src/governance.rs` — Conway governance types
- `crates/ade_ledger/src/rules.rs` — era dispatch (updated)

### GREEN (deterministic glue, non-authoritative)

- Differential harness extension with "script-execution-deferred" categorization
- Negative test vectors for Alonzo/Babbage/Conway structural validation
- Property tests for datum hash presence, collateral presence, governance action structure

### RED (nondeterministic shell)

None.

---

## 7. Invariants Preserved

All invariants established by prior slices are preserved:

- All Phase 0A/0B/1/2A invariants
- T-21 (UTxO Model) — UTxO operations, conservation, double-spend detection unchanged
- T-22 (Byron) — Byron validation unchanged
- T-23 (Shelley/Allegra/Mary) — Shelley/Allegra/Mary validation unchanged
- T-CONSERV-01 — conservation property: `consumed == produced` with explicit exceptions
- T-NOSPEND-01 — double-spend rejection unchanged
- DC-LEDGER-01 — `apply_block` is pure and deterministic
- DC-CBOR-01 / DC-CBOR-02 — codec round-trip and wire-byte preservation for all eras
- DC-CRYPTO-01 — crypto verification matches oracle
- All prior exit criteria (CE-01 through CE-67, plus CE-74/CE-75/CE-77/CE-80 from prior slices)

---

## 8. Invariants Strengthened or Introduced

| Invariant | How Strengthened |
|-----------|-----------------|
| DC-LEDGER-03 | Partially enforced — validity decisions match oracle on non-Plutus corpus blocks across Alonzo, Babbage, and Conway eras. Version-scoped to cardano-node 10.6.2. Plutus equivalence deferred to Phase 3. |
| DC-LEDGER-05 | Partially enforced — witness binding extended to Alonzo+ structural witnesses (script witness presence, redeemer presence). Plutus witness binding (actual execution) deferred to Phase 3. |
| DC-EPOCH-01 | Partially enforced — Conway governance types (`ConwayGovState`, `Proposals`, `ProposalProcedure`, `RatifyState`) exist and are structurally validated. Governance enactment at epoch boundary is T-25. |

---

## 9. Design Summary

### Alonzo structural validation (Phase 1 — no script execution)

Alonzo extends Shelley/Mary rules with Plutus-aware structural validation:

1. **All Shelley/Mary rules inherited** — UTxO operations, conservation, fee validation, validity intervals, key witness verification, native script evaluation.
2. **Datum hash presence**: outputs to script addresses must carry a datum hash. Enforced structurally — `MissingDatumHash` if absent. No datum evaluation.
3. **Collateral presence**: transactions containing Plutus script witnesses must provide collateral inputs. `MissingCollateral` if absent. Collateral inputs must exist in UTxO (standard input resolution).
4. **Script witness presence**: each script-locked input must have a corresponding script in the transaction witnesses. `MissingScriptWitness` if absent.
5. **Redeemer presence**: each Plutus script must have a corresponding redeemer indexed by purpose. `RedeemerMismatch` if absent or mismatched.
6. **Plutus script execution**: NOT performed. Every Plutus script produces `ScriptVerdict::NotYetEvaluated`. Structural checks pass; execution is Phase 3.
7. **Non-script inputs**: validated normally (UTxO lookup, key witnesses, fees, conservation).

### Babbage structural additions (incremental on Alonzo)

1. **Inline datums**: outputs may carry an inline datum (embedded in the TxOut) instead of a datum hash. Both forms are structurally valid.
2. **Reference scripts**: outputs may carry reference scripts, making them available to other transactions without including them in witnesses.
3. **Reference inputs**: transaction body may include reference inputs — UTxOs referenced for reading without being consumed. Must exist in UTxO but are not removed.
4. **Collateral return**: excess collateral returned to a specified output address. Structural presence validated.
5. All Alonzo structural Plutus checks still apply; execution still `NotYetEvaluated`.

### Conway structural additions (incremental on Babbage)

1. **Governance actions**: `ProposalProcedure` with 7 action types: parameter change, hard fork initiation, treasury withdrawal, info action, new committee, new constitution, no confidence. Structural validity checked (well-formed action, valid deposit).
2. **Voting**: DRep votes, committee votes, SPO votes on governance actions. Vote structure validated (valid voter, valid governance action ID).
3. **DRep certificates**: `DRepRegistration`, `DRepDeregistration`, `DRepUpdate`. Certificate structure validated.
4. **Committee certificates**: `CommitteeHotKeyAuth`, `CommitteeResignation`. Certificate structure validated.
5. **Treasury donation**: transaction body may include a `treasury_donation` field (non-negative `Coin`).
6. **Combined delegation certificates**: `StakeVoteDeleg`, `StakeRegDeleg`, `VoteDeleg`, `VoteRegDeleg`, `StakeVoteRegDeleg`. New combined certificates structurally validated.
7. All Babbage structural Plutus checks carry forward; execution `NotYetEvaluated`.

### Conway governance state types

Governance types are defined in this slice; enactment logic (applying ratified proposals at epoch boundary) is T-25.

```rust
pub struct ConwayGovState {
    pub proposals: Proposals,
    pub constitution: Constitution,
    pub committee: Committee,
    pub drep_state: BTreeMap<DRepCredential, DRepState>,
}

pub struct Proposals(pub BTreeMap<GovActionId, ProposalProcedure>);

pub struct RatifyState {
    pub enacted: Vec<GovActionId>,
    pub expired: Vec<GovActionId>,
}
```

### Differential harness categorization

The differential harness (GREEN) is extended to categorize blocks into three bins:

- **Non-Plutus blocks**: zero divergence required (accept/reject must match oracle). Counted as "passing" or "divergent."
- **Plutus-dependent blocks**: categorized "script-execution-deferred."
  - NOT counted as "passing" — verdict is incomplete.
  - NOT counted as "divergent" — Plutus deferral is explicit and intentional.
  - Tracked separately in harness output with count and block identifiers.
  - Phase 3 resolves these to full verdicts.

### Era dispatch

`rules.rs` is updated to dispatch Alonzo/Babbage/Conway blocks to their respective validation modules. The dispatch is era-aware via the block's era tag (already decoded by `ade_codec`/`ade_types`).

---

## 10. Changes Introduced

### Types

| Type | Location | Purpose |
|------|----------|---------|
| `ConwayGovState` | `ade_ledger/src/governance.rs` | Top-level Conway governance state container |
| `Proposals` | `ade_ledger/src/governance.rs` | Ordered map of governance action IDs to proposal procedures |
| `ProposalProcedure` | `ade_ledger/src/governance.rs` | Governance proposal: action, deposit, return address, anchor |
| `GovAction` | `ade_ledger/src/governance.rs` | Enum: ParameterChange, HardForkInitiation, TreasuryWithdrawal, InfoAction, NewCommittee, NewConstitution, NoConfidence |
| `GovActionId` | `ade_ledger/src/governance.rs` | Governance action identifier (TxId + action index) |
| `Committee` | `ade_ledger/src/governance.rs` | Constitutional committee: members + quorum threshold |
| `Constitution` | `ade_ledger/src/governance.rs` | Constitution reference: anchor + optional script hash |
| `DRepState` | `ade_ledger/src/governance.rs` | DRep registration state: deposit, anchor, expiry |
| `RatifyState` | `ade_ledger/src/governance.rs` | Ratification output: enacted and expired action IDs |
| `MissingDatumHash` | `ade_ledger/src/error.rs` | New `LedgerError` variant |
| `MissingCollateral` | `ade_ledger/src/error.rs` | New `LedgerError` variant |
| `MissingScriptWitness` | `ade_ledger/src/error.rs` | New `LedgerError` variant |
| `RedeemerMismatch` | `ade_ledger/src/error.rs` | New `LedgerError` variant |
| `GovernanceActionInvalid` | `ade_ledger/src/error.rs` | New `LedgerError` variant |
| `InvalidCertificate` | `ade_ledger/src/error.rs` | New `LedgerError` variant |

### Functions

| Function | Location | Signature | Purpose |
|----------|----------|-----------|---------|
| `validate_alonzo_tx` | `ade_ledger/src/alonzo.rs` | `(state: &LedgerState, tx: &AlonzoTx, slot: SlotNo) -> Result<LedgerTransition, LedgerError>` | Alonzo structural transaction validation |
| `validate_babbage_tx` | `ade_ledger/src/babbage.rs` | `(state: &LedgerState, tx: &BabbageTx, slot: SlotNo) -> Result<LedgerTransition, LedgerError>` | Babbage structural transaction validation |
| `validate_conway_tx` | `ade_ledger/src/conway.rs` | `(state: &LedgerState, tx: &ConwayTx, slot: SlotNo) -> Result<LedgerTransition, LedgerError>` | Conway structural transaction validation |
| `check_datum_hashes` | `ade_ledger/src/alonzo.rs` | `(outputs: &[TxOut]) -> Result<(), LedgerError>` | Datum hash presence check for script outputs |
| `check_collateral` | `ade_ledger/src/alonzo.rs` | `(tx: &AlonzoTx) -> Result<(), LedgerError>` | Collateral presence check for Plutus transactions |
| `check_script_witnesses` | `ade_ledger/src/alonzo.rs` | `(tx: &AlonzoTx) -> Result<(), LedgerError>` | Script witness presence check |
| `check_redeemers` | `ade_ledger/src/alonzo.rs` | `(tx: &AlonzoTx) -> Result<(), LedgerError>` | Redeemer presence and indexing check |
| `validate_governance_actions` | `ade_ledger/src/conway.rs` | `(actions: &[ProposalProcedure], state: &ConwayGovState) -> Result<(), LedgerError>` | Conway governance action structural validation |
| `validate_conway_certs` | `ade_ledger/src/conway.rs` | `(certs: &[Certificate]) -> Result<(), LedgerError>` | Conway certificate structural validation |

### State Transitions

None. Governance state types are defined here but state transitions (enactment) are T-25.

### Persistence

None.

### Removal / Refactors

None. Era dispatch in `rules.rs` is extended, not replaced.

---

## 11. Replay, Crash, and Epoch Validation

- **Alonzo corpus blocks**: all non-Plutus-dependent Alonzo blocks produce identical accept/reject decisions and state hashes to oracle. Plutus-dependent blocks categorized "script-execution-deferred."
- **Babbage corpus blocks**: all non-Plutus-dependent Babbage blocks produce identical accept/reject decisions and state hashes to oracle. Plutus-dependent blocks categorized "script-execution-deferred."
- **Conway corpus blocks**: all non-Plutus-dependent Conway blocks produce identical accept/reject decisions and state hashes to oracle. Plutus-dependent blocks categorized "script-execution-deferred."
- **Negative test vectors — Alonzo**: at least 5 vectors covering missing datum hash, missing collateral, missing script witness, redeemer mismatch, and combined failures. All rejected deterministically with correct `LedgerError` variant.
- **Negative test vectors — Babbage**: at least 5 vectors covering invalid reference input (not in UTxO), malformed inline datum, missing reference script, invalid collateral return, and combined failures.
- **Negative test vectors — Conway**: at least 5 vectors covering malformed governance action, invalid DRep certificate, invalid committee certificate, invalid treasury donation, and malformed combined delegation certificate.
- **Differential harness categorization**: Plutus-dependent blocks appear in "script-execution-deferred" bin with count and block identifiers. Not counted as passing. Not counted as divergent.
- **Determinism**: applying the same Alonzo/Babbage/Conway block to the same state produces the same output state on every invocation (`ci_check_ledger_determinism.sh`).
- **Conservation**: UTxO conservation property holds for all accepted Alonzo/Babbage/Conway transactions (inherited from T-21/T-23).

No crash recovery or epoch boundary behavior in this slice. Epoch boundary transitions are T-25.

---

## 12. Mechanical Acceptance Criteria

This slice is complete only when **all** of the following exist and pass in CI:

- [ ] Alonzo structural validation implemented: datum hash presence, collateral presence, script witness presence, redeemer presence all checked
- [ ] Babbage structural validation implemented: inline datums, reference scripts, reference inputs, collateral return all handled
- [ ] Conway structural validation implemented: governance actions, DRep/committee certificates, treasury donation, combined delegation certificates all handled
- [ ] Plutus scripts produce `ScriptVerdict::NotYetEvaluated` — no execution, no approximation, no partial evaluation
- [ ] Differential harness reports zero divergence on ALL non-Plutus Alonzo corpus blocks
- [ ] Differential harness reports zero divergence on ALL non-Plutus Babbage corpus blocks
- [ ] Differential harness reports zero divergence on ALL non-Plutus Conway corpus blocks
- [ ] Plutus-dependent blocks categorized "script-execution-deferred" in harness output (not pass, not diverge)
- [ ] `ConwayGovState`, `Proposals`, `ProposalProcedure`, `RatifyState` types exist in `governance.rs`
- [ ] At least 5 negative test vectors for Alonzo structural validation — all rejected deterministically with correct `LedgerError` variants
- [ ] At least 5 negative test vectors for Babbage structural validation — all rejected deterministically with correct `LedgerError` variants
- [ ] At least 5 negative test vectors for Conway structural validation — all rejected deterministically with correct `LedgerError` variants
- [ ] `ci_check_differential_divergence.sh` passes — non-Plutus zero divergence across Alonzo/Babbage/Conway, Plutus tracked separately
- [ ] `ci_check_ledger_determinism.sh` passes — includes Alonzo/Babbage/Conway blocks
- [ ] All equivalence claims version-scoped to oracle (cardano-node 10.6.2)
- [ ] Conway pending reference data documented where applicable
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] All prior exit criteria still pass (CE-80)
- [ ] All new BLUE source files have contract headers and deny attributes

---

## 13. Failure Modes

| Failure | Error Shape | Behavior | Replay Impact |
|---------|-------------|----------|---------------|
| Script output without datum hash (Alonzo+) | `LedgerError::MissingDatumHash { output_index }` | Fail-fast, deterministic | Transaction rejected — output to script address without datum is invalid |
| Plutus transaction without collateral (Alonzo+) | `LedgerError::MissingCollateral` | Fail-fast, deterministic | Transaction rejected — Plutus transactions require collateral |
| Script-locked input without script witness | `LedgerError::MissingScriptWitness { script_hash }` | Fail-fast, deterministic | Transaction rejected — input locked by script with no witness |
| Missing or mismatched redeemer | `LedgerError::RedeemerMismatch { purpose }` | Fail-fast, deterministic | Transaction rejected — Plutus script without matching redeemer |
| Plutus script encountered (execution deferred) | `ScriptVerdict::NotYetEvaluated` | Non-failing sentinel | Block categorized "script-execution-deferred" — Phase 3 resolves |
| Malformed governance action (Conway) | `LedgerError::GovernanceActionInvalid { detail }` | Fail-fast, deterministic | Transaction rejected — governance action structurally invalid |
| Invalid DRep/committee/combined certificate (Conway) | `LedgerError::InvalidCertificate { detail }` | Fail-fast, deterministic | Transaction rejected — certificate structurally invalid |
| Reference input not in UTxO (Babbage+) | `LedgerError::InputNotFound { tx_in }` | Fail-fast, deterministic | Transaction rejected — referenced UTxO does not exist |
| Invalid treasury donation (Conway) | `LedgerError::GovernanceActionInvalid { detail: "invalid_treasury_donation" }` | Fail-fast, deterministic | Transaction rejected — negative or malformed treasury donation |

All failure modes are:
- **Deterministic**: same input always produces the same error variant
- **Fail-fast**: no retry, no fallback, no alternative code path
- **Structured**: `LedgerError` enum with `&'static str` detail fields — no `String` formatting
- **Consensus-neutral**: failed validation produces no state mutation

`ScriptNotYetEvaluated` is explicitly NOT a failure mode — it is a deferral sentinel. It produces no error, causes no rejection, and does not trigger fail-fast behavior. Phase 3 resolves deferred verdicts to full pass/fail.

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
- Private key or signing operations
- TODOs, placeholders, or deferred validation in authoritative code
- Re-encoding for hash computation — hash paths MUST use wire bytes
- `.unwrap()`, `.expect()`, or `panic!()` in BLUE paths
- `unsafe` code in `ade_ledger`
- Ledger code in `ade_core`
- Partial Plutus evaluation
- Unversioned query-like APIs
- Consensus-level era awareness in HFC translation
- Mutable state in `apply_block`
- Implicit conservation exceptions
- Version-independent equivalence claims
- Mocking or stubbing ledger logic

### Slice-Specific Prohibitions

The following are strictly forbidden in this slice:

- **ABSOLUTELY NO Plutus script execution** — not even partial, not approximate, not budget-only. Every Plutus script produces `ScriptVerdict::NotYetEvaluated`.
- **No Plutus cost model application** — cost models are Phase 3. No cost model types, no cost model lookups, no budget estimation.
- **No script context construction** — `ScriptContext` (Plutus V1/V2/V3) is Phase 3. No data marshaling for Plutus evaluation.
- **No UPLC evaluation** — no UPLC AST, no CEK machine, no Plutus interpreter of any kind.
- **No "probably valid" verdicts** — transactions are structurally valid OR rejected OR script-execution-deferred. No intermediate, approximate, or probabilistic verdicts.
- **No signing operations** — verification only. Signing belongs in RED shell (Phase 7).
- **No query-like APIs** — no functions returning ledger state in query-response format. Query formatting is Phase 4.
- **No pre-baked version/query assumptions** — no hardcoded protocol versions, no version-specific branching beyond era dispatch.
- **No governance enactment** — governance types are defined; enactment (applying ratified proposals at epoch boundary) is T-25.
- **No epoch boundary logic** — epoch transitions, stake snapshots, reward calculations are T-25.

**If any of these appear, the slice is incorrect.**

---

## 15. Explicit Non-Goals

This slice MUST NOT:

- Implement Plutus script execution (Phase 3)
- Implement Plutus cost model application (Phase 3)
- Construct Plutus script contexts (Phase 3)
- Evaluate UPLC programs (Phase 3)
- Implement epoch boundary transitions (T-25)
- Implement HFC era translation functions (T-26)
- Implement consensus logic (Phase 4)
- Implement full Conway governance enactment (T-25)
- Modify Byron/Shelley/Allegra/Mary validation (T-22/T-23)
- Introduce new protocol versions
- Optimize for performance
- Add feature flags or configuration switches
- Prepare for future behavior not required by this slice
- Introduce networking, storage, or protocol state machines

Any work outside the stated scope and invariants is scope creep and must be rejected.

---

## 16. Completion Checklist

A slice may be merged only when **all** items are satisfied:

- [ ] `alonzo.rs` implements structural validation: datum hashes, collateral, script witnesses, redeemers
- [ ] `babbage.rs` implements structural validation: inline datums, reference scripts, reference inputs, collateral return
- [ ] `conway.rs` implements structural validation: governance actions, DRep/committee certificates, treasury donation, combined delegation certificates
- [ ] `governance.rs` defines `ConwayGovState`, `Proposals`, `ProposalProcedure`, `GovAction`, `GovActionId`, `Committee`, `Constitution`, `DRepState`, `RatifyState`
- [ ] `rules.rs` dispatches Alonzo/Babbage/Conway blocks to era-specific validation
- [ ] Plutus scripts produce `ScriptVerdict::NotYetEvaluated` — no execution
- [ ] Differential harness categorizes Plutus-dependent blocks as "script-execution-deferred"
- [ ] Zero divergence on all non-Plutus Alonzo/Babbage/Conway corpus blocks
- [ ] At least 5 negative test vectors per era (Alonzo, Babbage, Conway)
- [ ] All new state is replay-derivable
- [ ] All new data is canonically encoded
- [ ] All failure modes are deterministic
- [ ] No TODOs or placeholders in authoritative (BLUE) paths
- [ ] CI enforces the invariants strengthened by this slice
- [ ] All equivalence claims version-scoped to oracle (cardano-node 10.6.2)
- [ ] All new BLUE source files have contract headers and deny attributes
- [ ] All prior CI scripts and exit criteria still pass
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass

---

## 17. Review Notes

- **This slice is the widest in Phase 2C**, covering three eras. The risk is managed by the Plutus deferral gate — all three eras share the same "no Plutus execution" boundary, reducing the validation surface to structural checks only.
- **Alonzo structural validation is the foundation.** Babbage and Conway are incremental on Alonzo. The layered design means Babbage inherits all Alonzo checks and adds its own; Conway inherits all Babbage checks and adds its own. This mirrors the Haskell ledger's era composition.
- **The differential harness "script-execution-deferred" bin is critical.** Without it, Plutus-dependent blocks would be silently miscategorized — either as passing (incorrect, verdict is incomplete) or as divergent (incorrect, deferral is intentional). The three-bin model (pass / diverge / deferred) makes the Plutus boundary visible and auditable.
- **Conway governance types are defined here but enactment is T-25.** This is intentional: governance actions, certificates, and votes must be structurally validated during transaction processing (T-24), but their effects (ratification, enactment, expiry) happen at epoch boundaries (T-25). The type definitions in `governance.rs` are shared between both slices.
- **Conway pending reference data**: some Conway governance features may require reference data from the Haskell node that is not yet available in the corpus. Where applicable, document which Conway features have pending reference data and what additional corpus blocks are needed.
- **Follow-up slices implied**: T-25 (epoch boundary transitions) uses `ConwayGovState` and `RatifyState` for governance enactment. T-26 (HFC era translation) translates governance state across the Babbage-to-Conway boundary. Phase 3 resolves all "script-execution-deferred" blocks to full verdicts.
- **Reference input semantics**: Babbage reference inputs must exist in UTxO but are NOT consumed. The UTxO set after a transaction with reference inputs must still contain those inputs. This is a subtle divergence from regular inputs — verify carefully against the Haskell implementation.
- **Combined delegation certificates (Conway)** are new certificate types that combine stake delegation with vote delegation in a single certificate. These must be structurally validated but their delegation effects are part of epoch boundary processing (T-25).

---

## 18. Authority Reminder

This template is a planning and review aid only.

Correctness requirements are defined exclusively by:
- the project constitution (`ade_replay_first_constitutional_node_plan_v1.md` §2-§4b),
- `01_core_determinism_and_contract.md`,
- `classification_table.md`,
- and other normative specifications.

If there is ever a conflict:

> **Normative documents and CI enforcement are authoritative.**
