# Cluster Plan: Phase 2C — Ledger Rules Phase 2 (Alonzo–Conway Structural + Epoch + HFC)

> **Status:** Proposed
>
> **Authority Level:** Cluster Plan (Non-Normative Planning Artifact)
>
> This document defines the scope, exit criteria, and slice sequencing for Phase 2C.
> It introduces no new requirements and does not override any normative specification.
> Normative authority resides in `01_core_determinism_and_contract.md`, the project constitution
> (`ade_replay_first_constitutional_node_plan_v1.md` §2–§4b), and `classification_table.md`.

---

## Cluster Name

**Phase 2C: Ledger Rules Phase 2 (Alonzo–Conway Structural + Epoch + HFC)**

## Purpose

Complete the remaining Phase 2 ledger rules scope that Phase 2B did not close. Phase 2B delivered Byron through Mary transaction validation (19 slices, 462 tests, 6,000-block replay). Phase 2C completes the original Phase 2 scope by implementing Alonzo/Babbage/Conway structural validation, epoch boundary transitions, and HFC ledger-side era translations.

This cluster:

1. Extends `ade_ledger` with Alonzo, Babbage, and Conway structural transaction validation — Plutus scripts produce `ScriptVerdict::NotYetEvaluated` (no UPLC execution).
2. Implements epoch boundary transitions for Shelley through Conway: stake snapshot rotation (mark/set/go), reward computation, protocol parameter updates, pool retirement, Conway governance ratification and enactment.
3. Implements Hard Fork Combinator ledger-side era translation functions for all 6 transitions (Byron→Shelley through Babbage→Conway).
4. Closes all 25 Phase 2B exit criteria (CE-58 through CE-82) that Phase 2B left open — specifically CE-68 through CE-73, CE-78, CE-79, and CE-82.
5. Completes the final Phase 2 exit sweep: all invariant registry updates, four-tier gate statement, and version-scoping audit.

Phase 2C is NOT Phase 3. Phase 3 is the UPLC Evaluator (Plutus script execution, cost models, script context, conformance). Phase 2C contains zero Plutus execution. The naming preserves the existing phase numbering:

- Phase 2A: Crypto Verification (complete)
- Phase 2B: Ledger Rules Phase 1 — Byron through Mary (complete)
- Phase 2C: Ledger Rules Phase 2 — Alonzo through Conway structural + epoch + HFC (this cluster)
- Phase 3: UPLC Evaluator (unchanged)

## Relationship to Phase 2B

Phase 2C inherits the full authority surface, forbidden patterns, byte authority contract, compatibility boundary, scope of equivalence claims, Plutus deferral gate, Conway pulser specification, and mixed-version proof obligations from the Phase 2B cluster plan. Those sections are not duplicated here — they remain authoritative in the Phase 2B cluster plan and apply to Phase 2C slices without modification.

Phase 2C slices were originally designated T-24, T-25, T-26 in the Phase 2B cluster plan's slice sequencing. Their exit criteria (CE-68 through CE-73, plus contributions to CE-74, CE-75, CE-78, CE-79, CE-80, CE-81, CE-82) were defined in Phase 2B's exit criteria section and remain authoritative as originally written.

The only change is the cluster designation in the slice headers: "Phase 2B: Ledger Rules Phase 1" becomes "Phase 2C: Ledger Rules Phase 2."

---

## Inputs

| Input | Source | Purpose |
|-------|--------|---------|
| Phase 2B outputs | Code repo at `/home/ts/Code/rust/ade` | `ade_ledger` with Byron–Mary validation, UTxO model, conservation/double-spend enforcement, `LedgerError`, `ScriptVerdict`, GREEN adapter, CI scripts |
| Phase 2A outputs | Code repo at `/home/ts/Code/rust/ade` | `ade_crypto` crate: Blake2b, Ed25519, VRF, KES verification |
| Phase 1 outputs | Code repo at `/home/ts/Code/rust/ade` | `ade_codec`/`ade_types`: `PreservedCbor<T>`, era-specific types, wire-byte preservation |
| Phase 2B cluster plan | `clusters/phase_2b_ledger_rules_phase1/cluster_plan.md` | Authority surface, forbidden patterns, byte authority, compatibility boundary (inherited) |
| Project plan | `ade_replay_first_constitutional_node_plan_v1.md` | Phase scope, CI scripts, invariant lists |
| Supreme determinism law | `01_core_determinism_and_contract.md` | Module header, forbidden constructs, BLUE/RED boundary |
| Classification table | `classification_table.md` | CN-LEDGER-*, CN-EPOCH-*, CN-WIRE-09 invariants |
| Cardano ledger specs | Shelley/STS formal spec, Conway CDDL, era-specific rules | Transition rules per era |
| Oracle reference data | Haskell cardano-node 10.6.2 | Ledger state hashes, epoch boundary snapshots, era transition reference outputs |
| Audit worksheet | `audit_worksheet.md` | Code-grounded audit findings |
| **Alonzo/Babbage/Conway corpus** | **AWS node extraction (blocker)** | **1,500+ contiguous blocks per era with oracle state hashes and ExtLedgerState dumps** |

## Outputs

| Output | Location | Description |
|--------|----------|-------------|
| Alonzo/Babbage/Conway validation | `crates/ade_ledger/src/{alonzo,babbage,conway,governance,rules}.rs` | Structural tx validation for three eras, Plutus → `NotYetEvaluated` |
| Epoch boundary logic | `crates/ade_ledger/src/{epoch,governance,delegation,state}.rs` | Shelley–Conway epoch transitions, Conway governance ratification/enactment |
| HFC era translations | `crates/ade_ledger/src/hfc.rs` | All 6 ledger-side era translation functions |
| Expanded corpus | `corpus/golden/` | 1,500+ contiguous blocks for Alonzo, Babbage, Conway with oracle state hashes |
| Registry updates | `docs/ade-invariant-registry.toml` | Final CE-78 updates, CE-79 gate statement |
| Phase 2 completion evidence | CI artifacts | All CE-58 through CE-82 passing |

---

## Corpus Data Gap (Blocker)

Phase 2C cannot run differential testing for Alonzo, Babbage, or Conway without contiguous corpus data. Current state:

| Era | Contiguous blocks | State hashes | ExtLedgerState dumps | Golden blocks |
|-----|-------------------|--------------|----------------------|---------------|
| Byron | 1,500 | yes | yes | 3 |
| Shelley | 1,500 | yes | yes | 3 |
| Allegra | 1,500 | yes | yes | 3 |
| Mary | 1,500 | yes | yes | 3 |
| **Alonzo** | **0** | **no** | **no** | **3** |
| **Babbage** | **0** | **no** | **no** | **3** |
| **Conway** | **0** | **no** | **no** | **3** |

**Prerequisite**: Extract and commit contiguous corpus for Alonzo, Babbage, and Conway from the AWS node. Same pipeline as Byron–Mary extraction (T-19/T-20), targeting 1,500 blocks per era with oracle state hashes and ExtLedgerState dumps.

This extraction must complete before T-24 differential testing can run. Golden blocks (3 per era) exist and can support unit-level development, but the CE-75 zero-divergence gate requires contiguous corpus.

---

## Inherited Specifications (from Phase 2B)

The following Phase 2B specifications apply to Phase 2C without modification. They are NOT duplicated here — refer to the Phase 2B cluster plan for authoritative text:

- **Authority Surface** — Phase 2C operates within the same authority boundary
- **Compatibility Boundary** — `apply_block` state hash equivalence contract
- **Scope of Equivalence Claims** — harness-local claims against oracle corpus
- **Plutus Deferral Gate** — four-tier classification, `ScriptVerdict` enum
- **Conway Pulser** — atomic DRep stake computation with proof obligations and fallback
- **Hard Prohibition: No Pre-Baked Query/Version/Protocol Semantics**
- **Mixed-Version Proof Obligations** — all claims version-scoped to 10.6.2
- **Byte Authority Contract** — wire bytes for hash paths, project-canonical for replay
- **Forbidden Patterns (Cluster-Level)** — all inherited prohibitions
- **Decision Tests** — all 5 tests passed in Phase 2B, results unchanged
- **Proof Obligations** — Phase 2B's 16 obligations carry forward; Phase 2C closes #9–#12
- **Failure Modes** — deterministic, fail-fast, inspectable
- **Open Risks** — all Phase 2B risks carry forward
- **Deny attribute constraints** — `#![deny(unsafe_code)]`, etc.

---

## Invariants Touched by Phase 2C

Phase 2C touches the same invariants as Phase 2B (it completes the same cluster's scope). The specific invariant movements are defined in the Phase 2B cluster plan's "Invariants Touched" section and remain authoritative.

Phase 2C is responsible for the final status transitions:

### Enforced (status confirmed as "enforced" after Phase 2C)

| Invariant | Enforcement mechanism |
|-----------|----------------------|
| DC-LEDGER-01 | `apply_block` pure and deterministic — now across ALL 7 eras including epoch boundaries and era translations |
| T-CONSERV-01 | Conservation property-tested across all 7 eras and epoch boundaries |
| T-NOSPEND-01 | Double-spend rejection across all 7 eras |

### Partially enforced (moved to "partial" after Phase 2C)

| Invariant | What is enforced | What remains |
|-----------|-----------------|-------------|
| DC-LEDGER-02 | Byte-identical state on corpus across all 7 eras | Full mainnet sync (Phase 6) |
| DC-LEDGER-03 | Validity matches oracle on non-Plutus corpus across all 7 eras | Plutus equivalence (Phase 3), full mainnet (Phase 6) |
| DC-LEDGER-04 | Epoch boundary matches oracle across all eras | Full mainnet epoch coverage (Phase 6) |
| DC-LEDGER-05 | Witness binding for all non-Plutus witness types across all eras | Plutus witness binding (Phase 3) |
| DC-EPOCH-01 | Conway governance timing: atomic ratification/enactment at epoch boundary | Plutus-dependent governance effects (Phase 3) |
| DC-EPOCH-02 | HFC ledger-side era translation for all 6 transitions | Consensus-side HFC (Phase 4) |

---

## Exit Criteria

Phase 2C closes the following exit criteria that Phase 2B left open. All exit criteria definitions are verbatim from the Phase 2B cluster plan — they are not redefined here, only listed for tracking.

### Phase 2C is directly responsible for:

| CE | Description | Slice |
|----|-------------|-------|
| CE-68 | Alonzo structural validation | T-24 |
| CE-69 | Babbage structural validation | T-24 |
| CE-70 | Conway structural validation | T-24 |
| CE-71 | Epoch boundary transitions (Shelley–Babbage) zero divergence | T-25 |
| CE-72 | Conway epoch boundary with atomic pulser proof | T-25 |
| CE-73 | Era translations for all 6 transitions | T-26 |
| CE-78 | Registry status transitions completed | T-25, T-26 |
| CE-79 | Four-tier gate statement documented | T-25 |
| CE-82 | All equivalence claims version-scoped | T-26 |

### Phase 2C contributes to (already partially satisfied by Phase 2B):

| CE | Description | Phase 2C contribution |
|----|-------------|----------------------|
| CE-74 | `ci_check_ledger_determinism.sh` passes — all 7 eras | Extended to cover Alonzo/Babbage/Conway blocks and epoch boundaries |
| CE-75 | `ci_check_differential_divergence.sh` passes — 1,500+ blocks per era | Extended to Alonzo/Babbage/Conway corpus |
| CE-80 | All prior exit criteria still pass | Regression gate |
| CE-81 | No BLUE→RED dependency | Maintained |

### Phase 2C final gate:

When T-26 merges, ALL 25 Phase 2B/2C exit criteria (CE-58 through CE-82) must pass. This is the Phase 2 completion gate.

---

## Slice Sequencing

```
Phase 2B complete (T-19 through T-23)
        │
   [Corpus extraction: Alonzo/Babbage/Conway — prerequisite]
        │
   T-24 (Alonzo/Babbage/Conway Structural Validation)
        │
   T-25 (Epoch Boundary Logic)
        │
   T-26 (HFC Ledger-Side Era Translation + Phase 2 Exit Sweep)
```

Strict sequential dependency chain: T-24 → T-25 → T-26. No parallelism possible.

### Slice summary

| Slice | Name | Dependencies | Exit Criteria Addressed |
|-------|------|-------------|------------------------|
| T-24 | Alonzo/Babbage/Conway Structural Validation | T-23, corpus extraction | CE-68, CE-69, CE-70, CE-74, CE-75, CE-77, CE-80 |
| T-25 | Epoch Boundary Logic | T-24 | CE-71, CE-72, CE-74, CE-75, CE-78, CE-79, CE-80 |
| T-26 | HFC Ledger-Side Era Translation | T-25 | CE-73, CE-74, CE-75, CE-78, CE-80, CE-81, CE-82 |

---

## Proof Obligations Closed by Phase 2C

Phase 2B defined 16 proof obligations. Phase 2C closes obligations #9 through #12:

| # | Obligation | Slice | Verification |
|---|-----------|-------|-------------|
| 9 | Alonzo/Babbage/Conway structural validation correct | T-24 | Structural fields validated; Plutus deferred; state hashes compared where non-Plutus |
| 10 | Epoch boundary transitions match oracle | T-25 | Stake snapshots, rewards, parameter updates identical at corpus epoch boundaries |
| 11 | Conway atomic pulser produces identical results to Haskell pulsed computation | T-25 | Same DRep stake distribution, same enactment effects, no causal leakage, no replay divergence |
| 12 | Era translations produce identical state for all 6 transitions | T-26 | Post-translation state hash matches oracle |

---

## Phase 2 Completion Evidence

After T-26 merges, the following evidence package constitutes Phase 2 completion:

1. **CE-58 through CE-82**: all 25 exit criteria pass
2. **Registry updates**: all CE-78 transitions applied, `ci_check_constitution_coverage.sh` passes
3. **Four-tier gate statement** (CE-79): true (purity/determinism), derived (non-Plutus corpus equivalence), release (non-Plutus certification only), non-goal (no partial Plutus)
4. **Zero divergence**: `ci_check_differential_divergence.sh` reports zero divergence on all non-Plutus blocks across all 7 eras (1,500+ blocks per era)
5. **Determinism**: `ci_check_ledger_determinism.sh` passes across all 7 eras including epoch boundaries and era translations
6. **Plutus deferral**: all Plutus-dependent blocks categorized "script-execution-deferred" (not pass, not diverge)
7. **Dependency boundary**: `ade_ledger` is BLUE, `ade_core` is empty, `ci_check_dependency_boundary.sh` passes
8. **Version scoping**: all equivalence claims version-scoped to cardano-node 10.6.2

---

## Authority Reminder

This cluster plan is a planning and review aid only.

Correctness requirements are defined exclusively by:
- the project constitution (`ade_replay_first_constitutional_node_plan_v1.md` §2–§4b),
- `01_core_determinism_and_contract.md`,
- `classification_table.md`,
- and other normative specifications.

If there is ever a conflict:

> **Normative documents and CI enforcement are authoritative.**
