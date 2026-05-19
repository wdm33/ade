# Invariant Slice: T-26 — Hard Fork Combinator — Ledger Side

> **Status:** Proposed
>
> This document defines the standard slice for implementing the Hard Fork Combinator's
> ledger-side era translation functions across all 6 Cardano era transitions.
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

T-26: Hard Fork Combinator — Ledger Side

### Cluster

Phase 2C: Ledger Rules Phase 2

### Status

Proposed

### Cluster Exit Criteria Addressed

This slice directly contributes to satisfying the following cluster exit criteria (verbatim from cluster plan):

- [ ] CE-73: Era translations for all 6 transitions (Byron->Shelley, Shelley->Allegra, Allegra->Mary, Mary->Alonzo, Alonzo->Babbage, Babbage->Conway): ledger state translated correctly, producing identical state hashes to oracle after translation.
- [ ] CE-74: `ci_check_ledger_determinism.sh` passes: applies same block sequence twice, asserts identical state hashes. Covers all 7 eras. Tests both single-block and multi-block sequences.
- [ ] CE-75: `ci_check_differential_divergence.sh` passes: runs differential ledger harness on expanded corpus (1,500+ blocks per era), reports zero divergence on all non-Plutus-dependent blocks. Version-scoped to cardano-node 10.6.2.
- [ ] CE-80: All prior exit criteria (CE-01 through CE-57) still pass. No regression. `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- [ ] CE-81: No BLUE->RED dependency introduced. `ade_ledger` is BLUE with BLUE-only dependencies (`ade_types`, `ade_crypto`, `ade_codec`). `ade_core` remains empty (no source files beyond deny attributes and contract header). `ci_check_dependency_boundary.sh` passes.
- [ ] CE-82: All equivalence claims in this cluster are version-scoped to cardano-node 10.6.2. No version-independent equivalence claims appear in code comments, test names, CI scripts, or registry entries. Version scope documented in every registry entry that moves to "partial" or "enforced".

Exit criteria not listed here are explicitly out of scope for this slice.

### Slice Dependencies

This slice assumes the following slices have been completed and merged:

- T-25 — Epoch Boundary Logic

T-25 establishes epoch boundary transitions for Shelley through Conway. T-26 builds on this by implementing the era translation functions that transform ledger state at hard fork boundaries. Epoch boundary logic must be correct before era translation can be validated, because era translations occur at epoch boundaries.

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

**Key constraint**: HFC translation functions are pure functions of `(old_state, new_era_genesis)`. They must NOT depend on slot number, block number, chain tip, or any consensus-level concept. They transform ledger state from era N to era N+1 — nothing more.

**CRITICAL: Temporary co-location with forced extraction trigger.** HFC translation functions are temporarily co-located in `ade_ledger` because they transform ledger state between eras. **Forced extraction trigger**: When Phase 4 introduces consensus chain selection (DC-CONSENSUS-*), the HFC must be extracted to its own crate or to `ade_consensus`, because cross-era chain selection is consensus authority, not ledger authority. T-26 must not introduce coupling that would make this extraction expensive. Specifically:

- `hfc.rs` module has NO dependencies on internal `ade_ledger` types that would prevent extraction to a separate crate.
- Translation functions take explicit type parameters, not `LedgerState` directly.
- Module-level documentation notes the forced extraction trigger.
- No cross-module coupling beyond type imports from `ade_types`.

**Final slice note**: T-26 is the last slice in Phase 2C (and the last slice in Phase 2 overall). After T-26, the cluster's final exit criteria validation runs — all CE-58 through CE-82 must pass. This slice is responsible for the final Phase 2 exit criteria sweep, including CE-78 registry updates and CE-79 four-tier gate verification.

---

## 4. Intent

Make it impossible for Ade to produce different ledger state across era transitions than the Haskell node. Era translation functions transform ledger state from era N to era N+1 at hard fork boundaries. These are pure functions of the old state and the new era's genesis parameters. If any translation produces a different post-translation state hash than the oracle, the chain diverges permanently at that hard fork boundary.

---

## 5. Scope

### Modules / crates

- `ade_ledger/src/hfc.rs` (BLUE) — era translation functions for all 6 transitions
- `ade_ledger/src/rules.rs` (BLUE) — era transition dispatch integration
- GREEN: differential harness for era transition comparison against oracle reference outputs
- CI: final Phase 2 exit criteria validation

### State machines affected

None. Translations are pure functions — no state machines introduced.

### Persistence impact

None. No WAL changes, no checkpoint changes.

### Network-visible impact

None.

### Out of scope

- Consensus chain selection across eras (Phase 4)
- Cross-era chain selection (Phase 4)
- Hard fork scheduling — determining *when* transitions occur (Phase 4)
- Slot-to-era mapping (Phase 4)
- Forecast horizon (Phase 4)
- Plutus script evaluation (Phase 3)
- Block production (Phase 7)
- Storage or persistence (Phase 5)

---

## 6. Execution Boundary

### BLUE (deterministic, authoritative)

- `ade_ledger/src/hfc.rs` — all 6 era translation functions
- `ade_ledger/src/rules.rs` — era transition dispatch updates (if any)

### GREEN (deterministic glue, non-authoritative)

- Differential harness extensions for era transition comparison
- Era transition test vectors extracted from oracle via T-19 pipeline
- Final Phase 2 exit criteria validation scripts

### RED (nondeterministic shell)

None. This slice introduces no RED code.

---

## 7. Invariants Preserved

All invariants established by Phase 0A, Phase 0B, Phase 1, Phase 2A, and prior Phase 2B/2C slices (T-19 through T-25) are preserved:

- T-DET-01 (determinism) — translation functions are pure, deterministic
- T-ENC-01 (canonical encoding) — no encoding changes; translations operate on decoded state
- T-CORE-01 (pure authoritative logic) — translations are BLUE, no I/O
- T-CORE-02 (forbidden nondeterminism) — no HashMap, no floating point, no randomness in translations
- T-ERR-01 (structured errors) — `TranslationError` variant uses `&'static str`, no `String`
- T-BUILD-01 (no semantic build variability) — no `cfg` changes
- T-BOUND-02 (dependency direction) — `ade_ledger` depends only on `ade_types`, `ade_crypto`, `ade_codec` (all BLUE)
- DC-CBOR-02 (wire bytes for hashing) — no hash path modifications
- DC-CRYPTO-01 (crypto matches oracle) — no crypto operations introduced
- DC-LEDGER-01 (apply_block purity) — apply_block unchanged; translations are called at era boundaries
- T-CONSERV-01 (conservation) — UTxO conservation unchanged
- T-NOSPEND-01 (double-spend) — double-spend rejection unchanged
- DC-EPOCH-01 (epoch boundary) — epoch boundary transitions unchanged
- CE-01 through CE-72 — all prior exit criteria maintained

---

## 8. Invariants Strengthened or Introduced

| Invariant | How Strengthened |
|-----------|-----------------|
| DC-EPOCH-02 | Strengthened (partial) — HFC ledger-side era translation functions implemented for all 6 transitions. Ledger state translated correctly at hard fork boundaries with oracle-matched state hashes. Consensus-side HFC (forecast horizons, era scheduling, cross-era chain selection) deferred to Phase 4. |

T-26 does not introduce new invariants. It strengthens DC-EPOCH-02 by implementing the ledger-side component of the Hard Fork Combinator. The consensus-side component remains deferred to Phase 4.

---

## 9. Design Summary

### Era translation function signature

Each translation is a pure function:

```rust
fn translate_era_n_to_n1(
    old_state: &EraState<N>,
    new_genesis: &GenesisParams<N+1>,
) -> Result<EraState<N+1>, LedgerError>
```

Translation input is `(old_state, new_era_genesis)`, NOT `(old_state, slot_number, hard_fork_schedule)`. The translation function has no awareness of the current slot, block number, chain tip, or which era is "current." It receives an old era's state and new era's genesis parameters, and produces the new era's state.

### The 6 era translations

1. **Byron -> Shelley**: Transform Byron UTxO set into Shelley UTxO format. Initial delegation map from Shelley genesis. Protocol parameters from Shelley genesis. No stake distribution (Shelley starts with genesis delegation only). This is the most complex translation — Byron and Shelley have fundamentally different state representations.

2. **Shelley -> Allegra**: Minimal transformation. Allegra adds timelock native script support but the core state structure is unchanged. Type evolution of transaction and script types. No semantic changes to existing state components.

3. **Allegra -> Mary**: Add multi-asset support to `Value`. Existing Shelley/Allegra `Value` (Coin only) becomes `Value` (Coin + MultiAsset). Initially empty `MultiAsset` maps. TxOut format gains multi-asset capability. No change to existing Coin balances.

4. **Mary -> Alonzo**: Add Plutus infrastructure. Script witnesses gain `PlutusScript` variant. TxOut format gains optional datum hash. Transaction body gains `script_data_hash`, `collateral`, `required_signers`. Redeemers and datums added to witness set. All Plutus-related state initially empty — existing transactions have no Plutus components.

5. **Alonzo -> Babbage**: Add inline datum and reference script support to TxOut format. TxOut gains optional inline datum and optional reference script fields. Reference inputs added to transaction body. Collateral return and total collateral added. Existing TxOuts have no inline datums or reference scripts.

6. **Babbage -> Conway**: Add governance state. Initial governance state from Conway genesis: empty proposal list, initial constitutional committee, initial constitution, empty DRep state. TxOut format unchanged. Transaction body gains governance-related certificates (DRep registration, committee certificates, governance actions, treasury withdrawals).

### Extraction preparation

To ensure Phase 4 extraction is inexpensive:

- `hfc.rs` imports types from `ade_types` only, not from internal `ade_ledger` modules (e.g., not from `state.rs`, `utxo.rs`, `epoch.rs` directly)
- Translation functions are parameterized over era state types — they do not take `LedgerState` (which is an `ade_ledger`-internal composite)
- `hfc.rs` module-level documentation includes the forced extraction trigger statement
- No `pub(crate)` visibility on translation functions — all `pub` so they are usable from outside the crate after extraction
- No coupling to `rules.rs` dispatch logic beyond type-compatible function signatures

### Era transition dispatch

`rules.rs` (or the era-specific modules) calls the appropriate translation function when an era transition occurs. The dispatch is simple: match on `(from_era, to_era)` and call the corresponding translation function. The dispatch itself is a pure function.

### Differential validation

Oracle reference data for era transitions (extracted by T-19, stored in corpus by T-20) provides pre-translation and post-translation state hashes for each of the 6 transitions. The GREEN differential harness compares:

```
translate(oracle_pre_state, new_genesis).state_hash == oracle_post_state.state_hash
```

Zero divergence is required across all 6 transitions. All comparisons are version-scoped to cardano-node 10.6.2.

---

## 10. Changes Introduced

### Types

New types in `ade_ledger/src/hfc.rs`:

- No new public types beyond the translation functions themselves. Era state types (`EraState<N>`) and genesis parameter types (`GenesisParams<N+1>`) are defined in `ade_types` or in existing `ade_ledger` era-specific modules (T-21 through T-25). Translation functions operate on these existing types.

### Functions

New functions in `ade_ledger/src/hfc.rs`:

| Function | Signature (conceptual) | Purpose |
|----------|----------------------|---------|
| `translate_byron_to_shelley` | `(&ByronState, &ShelleyGenesis) -> Result<ShelleyState, LedgerError>` | Byron->Shelley era translation |
| `translate_shelley_to_allegra` | `(&ShelleyState, &AllegraGenesis) -> Result<AllegraState, LedgerError>` | Shelley->Allegra era translation |
| `translate_allegra_to_mary` | `(&AllegraState, &MaryGenesis) -> Result<MaryState, LedgerError>` | Allegra->Mary era translation |
| `translate_mary_to_alonzo` | `(&MaryState, &AlonzoGenesis) -> Result<AlonzoState, LedgerError>` | Mary->Alonzo era translation |
| `translate_alonzo_to_babbage` | `(&AlonzoState, &BabbageGenesis) -> Result<BabbageState, LedgerError>` | Alonzo->Babbage era translation |
| `translate_babbage_to_conway` | `(&BabbageState, &ConwayGenesis) -> Result<ConwayState, LedgerError>` | Babbage->Conway era translation |

### State Transitions

None. Translations are pure functions, not state machine transitions.

### Persistence

None.

### Removal / Refactors

None.

---

## 11. Replay, Crash, and Epoch Validation

### Replay tests added or updated

- **Era transition replay determinism**: Apply the same era translation (same old state, same new genesis) twice. Output states must be byte-identical. Covers all 6 transitions.
- **Era transition differential comparison**: Compare translation output state hashes against oracle reference outputs for all 6 transitions in the corpus. Zero divergence required.
- **Cross-transition chain replay**: Apply a sequence of blocks spanning an era transition (from corpus). The ledger state after applying blocks in era N, then translating, then applying blocks in era N+1 must produce oracle-matched state hashes at every step.
- **Full Phase 2 regression**: All prior replay tests (T-21 through T-25) continue to pass.

### Crash/restart behavior

Not applicable. Translation functions are pure — there is no crash/restart concern. Translations are called during epoch boundary processing (T-25), which has its own crash safety guarantees.

### Epoch boundary behavior

Era translations occur at epoch boundaries (hard fork boundaries are a special case of epoch boundaries). T-25 handles epoch boundary logic; T-26 adds era translation as an additional step at hard-fork-coincident epoch boundaries. The epoch boundary code calls the appropriate translation function when the epoch boundary is also a hard fork boundary.

---

## 12. Mechanical Acceptance Criteria

This slice is complete only when **all** of the following exist and pass in CI:

- [ ] All 6 era translation functions exist in `ade_ledger/src/hfc.rs` and are pure/deterministic
- [ ] Byron->Shelley translation produces correct initial Shelley state from Byron UTxO, delegation from Shelley genesis, protocol parameters from Shelley genesis
- [ ] Shelley->Allegra translation handles type evolution correctly, no semantic changes to existing state
- [ ] Allegra->Mary translation adds multi-asset support to Value (initially empty MultiAsset)
- [ ] Mary->Alonzo translation adds Plutus infrastructure (initially empty script witnesses, datums, redeemers)
- [ ] Alonzo->Babbage translation adds inline datum/reference script support to TxOut format
- [ ] Babbage->Conway translation adds governance state from Conway genesis (empty proposals, initial committee, initial constitution, empty DRep state)
- [ ] Translation functions take `(old_state, new_genesis)` — no slot/block/chain-tip awareness
- [ ] Differential harness reports zero divergence across all 6 era transitions in corpus (version-scoped to cardano-node 10.6.2)
- [ ] Cross-transition block replay produces oracle-matched state hashes through era boundaries
- [ ] No coupling that prevents Phase 4 extraction: `hfc.rs` imports from `ade_types` only, not from internal `ade_ledger` modules; translation functions are `pub`, not `pub(crate)`; module-level documentation notes forced extraction trigger
- [ ] `ade_core` remains empty (no source files beyond deny attributes and contract header)
- [ ] `ade_ledger` depends only on `ade_types`, `ade_crypto`, `ade_codec` (all BLUE)
- [ ] All equivalence claims version-scoped to oracle (cardano-node 10.6.2)
- [ ] `ci_check_differential_divergence.sh` passes across era transitions
- [ ] `ci_check_ledger_determinism.sh` passes (including era transition sequences)
- [ ] `ci_check_dependency_boundary.sh` passes
- [ ] Phase 2 four-tier gate verified (CE-79): true (purity/determinism), derived (non-Plutus corpus equivalence), release (non-Plutus certification only), non-goal (no partial Plutus)
- [ ] All registry updates from CE-78 applied: DC-LEDGER-01 enforced; DC-LEDGER-02/03/04/05 partial; DC-EPOCH-01 partial; DC-EPOCH-02 partial; T-CONSERV-01 enforced; T-NOSPEND-01 enforced; all confirmatory CN-* cross-references noted; `ci_check_constitution_coverage.sh` passes
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] All prior exit criteria (CE-01 through CE-72) still pass

---

## 13. Failure Modes

| Failure | Shape | Behavior | Replay Impact |
|---------|-------|----------|---------------|
| Translation produces incorrect state hash | `TranslationError { from_era, to_era, detail }` / differential divergence detected | Fail-fast, deterministic | Fatal — incorrect era translation means permanent chain divergence at that hard fork boundary. Must be fixed before merge. |
| Translation function receives invalid old state | `TranslationError { from_era, to_era, detail: "invalid source state" }` | Fail-fast, deterministic | Should not occur on valid chain state. If triggered, indicates upstream validation failure (T-21 through T-25). |
| Missing or invalid new-era genesis parameters | `TranslationError { from_era, to_era, detail: "missing genesis parameter" }` | Fail-fast, deterministic | Configuration error. Genesis parameters must be complete and valid. |
| Byron->Shelley UTxO conversion failure | `TranslationError { from_era: "byron", to_era: "shelley", detail }` | Fail-fast, deterministic | Byron UTxO format must be correctly mapped to Shelley UTxO. Conversion logic error. |
| Multi-asset Value initialization failure (Allegra->Mary) | `TranslationError { from_era: "allegra", to_era: "mary", detail }` | Fail-fast, deterministic | Value type evolution error. Existing Coin values must be preserved exactly. |
| Governance state initialization failure (Babbage->Conway) | `TranslationError { from_era: "babbage", to_era: "conway", detail }` | Fail-fast, deterministic | Conway genesis parameters must produce correct initial governance state. |

All translation errors are deterministic and structured. The same invalid input produces the same error on every invocation. All translation errors are fail-fast — there is no partial translation or recovery. A failed translation means the chain cannot proceed past that hard fork boundary, which is correct behavior (the implementation is wrong, not the chain).

---

## 14. Hard Prohibitions

### Inherited Cluster-Level Prohibitions

This slice inherits and MUST comply with all forbidden patterns defined in the Phase 2B cluster plan's "Forbidden Patterns (Cluster-Level)" section, including:

- `HashMap` or `HashSet` in BLUE code
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
- `.unwrap()`, `.expect()`, or `panic!()` in BLUE codec/crypto/ledger paths
- `unsafe` code in `ade_ledger`
- Ledger code in `ade_core`
- Partial Plutus evaluation
- Mocking or stubbing ledger logic
- Version-independent equivalence claims

No slice may weaken or override cluster-level prohibitions.

### Slice-Specific Prohibitions

The following are strictly forbidden in this slice:

- **No consensus-level era awareness** — Translation functions must NOT know which era is "current." They translate from era N to era N+1. The decision of *when* to translate is consensus authority (Phase 4), not ledger authority.
- **No slot/block/chain-tip dependency** — Translation functions take `(old_state, new_genesis)` only. No slot number, block number, chain tip, or hard fork schedule as input.
- **No code in `ade_core`** — All translation logic goes in `ade_ledger/src/hfc.rs`. `ade_core` remains empty.
- **No coupling that prevents extraction** — `hfc.rs` must not depend on internal `ade_ledger` modules (`state.rs`, `utxo.rs`, `epoch.rs`, etc.) in ways that would prevent moving `hfc.rs` to a separate crate in Phase 4. Type imports from `ade_types` are the allowed dependency surface.
- **No query-like APIs** — No functions that return translated state in a format assuming a specific query version. Translation produces era state, not query responses.
- **No signing operations** — Translation is pure state transformation. No key generation, no signing.
- **No Plutus evaluation** — Translation functions do not evaluate Plutus scripts. Plutus-related state (script witnesses, datums, redeemers) is structurally transformed but not evaluated.
- **No hard fork scheduling logic** — Translation functions do not determine when transitions occur. They execute transitions when called.

**If any of these appear, the slice is incorrect.**

---

## 15. Explicit Non-Goals

This slice MUST NOT:

- Implement consensus chain selection across eras (Phase 4)
- Implement cross-era chain selection (Phase 4)
- Implement hard fork scheduling or slot-to-era mapping (Phase 4)
- Implement forecast horizon logic (Phase 4)
- Implement Plutus script evaluation (Phase 3)
- Implement block production (Phase 7)
- Implement storage or persistence (Phase 5)
- Implement networking or protocol state machines (Phase 4)
- Introduce new crates (translations co-located in `ade_ledger` until Phase 4 extraction)
- Optimize for performance
- Add feature flags or configuration switches
- Prepare for future consensus-level HFC behavior not required by this slice
- Determine which era is "current" based on slot or chain state

Any work outside the stated scope is scope creep and must be rejected.

---

## 16. Completion Checklist

This slice may be merged only when **all** items are satisfied:

- [ ] All 6 era translation functions exist and are pure/deterministic
- [ ] All translations take `(old_state, new_genesis)` — no slot/chain awareness
- [ ] Differential harness reports zero divergence across all 6 era transitions
- [ ] Cross-transition block replay produces oracle-matched state hashes
- [ ] All new state is replay-derivable
- [ ] All new data is canonically encoded
- [ ] All failure modes are deterministic
- [ ] No TODOs or placeholders in authoritative (BLUE) paths
- [ ] No coupling preventing Phase 4 extraction
- [ ] `hfc.rs` module documentation includes forced extraction trigger statement
- [ ] `ade_core` remains empty
- [ ] `ade_ledger` depends only on `ade_types`, `ade_crypto`, `ade_codec`
- [ ] All equivalence claims version-scoped to oracle (10.6.2)
- [ ] CI enforces the invariant strengthened by this slice (DC-EPOCH-02 partial)
- [ ] Phase 2 four-tier gate verified (CE-79)
- [ ] All CE-78 registry updates applied
- [ ] Replay-equivalence tests pass across runs
- [ ] All prior exit criteria (CE-01 through CE-72) still pass
- [ ] `cargo test --workspace` and `cargo clippy` pass

---

## 17. Review Notes

- **Extraction cost is the primary design risk**: T-26 temporarily co-locates HFC translation in `ade_ledger`. The forced extraction trigger (Phase 4 consensus chain selection) is well-defined. Reviewers must verify that `hfc.rs` has no dependencies on internal `ade_ledger` modules that would make extraction require refactoring beyond a simple `mv` + dependency update. The test: can `hfc.rs` be moved to a new crate that depends only on `ade_types`, `ade_crypto`, and `ade_codec` without modifying any translation function body?
- **Byron->Shelley is the hardest translation**: Byron and Shelley have fundamentally different state representations. Byron uses a different UTxO format, different address encoding, and different delegation model. The translation must correctly convert all of these. All other translations are incremental type evolution. Reviewers should focus review effort on Byron->Shelley.
- **Translation vs. epoch boundary**: Era transitions happen at epoch boundaries, but not all epoch boundaries are era transitions. T-25 handles epoch boundary logic. T-26 adds translation as an additional step when the epoch boundary coincides with a hard fork. Reviewers should verify that the integration point between T-25 and T-26 is clean — epoch boundary logic calls translation when appropriate, and translation has no knowledge of epoch boundary mechanics.
- **Genesis parameter completeness**: Each translation requires genesis parameters for the new era. These parameters must be complete — a translation with missing genesis parameters should fail deterministically, not produce partial state. Reviewers should verify that genesis parameter types carry all required fields for each era.
- **Follow-up**: Phase 4 will extract HFC to its own crate or `ade_consensus` when consensus chain selection is introduced. The extraction must be mechanical, not architectural — T-26 must guarantee this.
- **Final Phase 2 slice**: T-26 is responsible for the final Phase 2 exit criteria sweep. CE-78 (registry updates) and CE-79 (four-tier gate) are verified as part of this slice's acceptance. Reviewers should verify that all 25 exit criteria (CE-58 through CE-82) pass after T-26 merges.

---

## 18. Authority Reminder

This template is a planning and review aid only.

Correctness requirements are defined exclusively by:
- the project constitution (`ade_replay_first_constitutional_node_plan_v1.md` S2-S4b),
- `01_core_determinism_and_contract.md`,
- `classification_table.md`,
- and other normative specifications.

If there is ever a conflict:

> **Normative documents and CI enforcement are authoritative.**
