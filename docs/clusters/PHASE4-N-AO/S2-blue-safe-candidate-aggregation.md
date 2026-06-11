# Invariant Slice S2 — BLUE-safe candidate construction (pure GREEN core)

> Slice of cluster PHASE4-N-AO (`docs/clusters/PHASE4-N-AO/cluster.md`). The cluster's **load-bearing** slice — it carries the §1 BLUE-safety proof obligation. **Pure GREEN, no BLUE, no store access.** Depends on S1 (`DC-NODE-34`, `31efec44`). Latent until S3 (`DC-NODE-36`), mirroring AI-S2's pure-GREEN-classifier-latent-until-AI-S4 shape.

## 2. Slice Header
- **Slice Name:** BLUE-safe candidate construction — `build_candidate_fragment` + deterministic candidate-set assembly (validated-only, pure).
- **Cluster:** PHASE4-N-AO — live multi-candidate fork-choice SELECT + adopt (rung-2).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-2** (`DC-NODE-35` BLUE-safe construction) — candidate fragments are built from `validate_and_apply_header` output only; a `follow.rs`-style minted summary reaching `select_best_chain` is mechanically absent; ordering is deterministic; malformed/missing fails closed. New gate `ci/ci_check_candidate_construction_validated.sh` green.
- **Slice Dependencies:** S1 (peer-identity restoration, `DC-NODE-34`) — the driver groups by `NodeSyncItem.peer` before feeding this core.

## 4. Intent
Make it **impossible for an unvalidated or peer-minted candidate to reach the fork-choice authority**: every `CandidateFragment` element is a deterministic projection of an Ade-validated header transition (`validate_and_apply_header` output), or it is rejected. Strengthens `DC-NODE-35`. This core never selects, never reads a store, never mutates — it is consumed live in S3.

## 5. Scope
- **Modules / crates:** NEW GREEN module `ade_node::candidate_aggregator` (`//! GREEN`). Reused unchanged: `ade_core::consensus::header_validate::validate_and_apply_header`, `ade_core::consensus::candidate::{CandidateFragment, TiebreakerView}`, `ade_core::consensus::header_summary::{HeaderInput, ValidatedHeaderSummary}`, `ade_core::consensus::praos_state::PraosChainDepState`, `ade_core::consensus::ledger_view::LedgerView`, `ade_core::consensus::era_schedule::EraSchedule`.
- **State machines affected:** none (the per-peer set assembly is a deterministic projection, `BTreeMap`-ordered).
- **Persistence impact:** **none** — pure functions; nothing persisted / hashed / serialized; no canonical-type or replay obligation.
- **Network-visible impact:** none.
- **Out of scope (→ S3's RED driver):** durable fork-anchor lookup; the **read-only `materialize_rolled_back_state`** that produces `anchor_chain_dep`; block decode; the live peer-tagged feed; the `select_best_chain` dispatch. Out of scope (→ S4): block-fetch, fork-switch apply.

## 6. Execution Boundary (TCB color)
- **BLUE:** none new. Reused unchanged: `validate_and_apply_header` + the candidate / header-summary types.
- **GREEN:** `ade_node::candidate_aggregator` — `build_candidate_fragment` + `assemble_candidate_set`. Pure deterministic projections over **supplied, Ade-validated inputs**; mints nothing.
- **RED:** none.

**Hard boundary (the color resolution, OQ-AO-6 → GREEN):** *S2 proves a pure GREEN construction core: `CandidateFragment`s are deterministic projections of supplied validated header transitions. S2 performs **no store reads, no materialization, no selection, no block-fetch, no WAL, and no durable mutation**.* The aggregator is GREEN because it only groups and constructs fragments from already-supplied, Ade-validated inputs — the color is resolved **by this proof**, not pre-picked.

## 7. Invariants Preserved (registry IDs)
`DC-NODE-34` (peer identity — the per-peer grouping consumes it, supplied by S3; not weakened), `DC-CONS-03` (`select_best_chain` — **not reached**; this core never selects), `CN-CONS-01` (the candidate-set determinism the selector relies on — the assembly ordering is deterministic), `CN-STORE-07` / `DC-CONS-20` / `T-REC-06` (materialize / lockstep / eta0 — **untouched**; no store access here), `DC-NODE-25`…`29` (apply/rollback — untouched), the BLUE header authority `validate_and_apply_header` (reused unchanged).

## 8. Invariants Strengthened / Introduced
- **Strengthens toward enforced** `DC-NODE-35` (BLUE-safe candidate construction): every fragment element is derived from `validate_and_apply_header` output or rejected; the construction is deterministic and fail-closed; no `follow.rs`-minted / peer-trusted / unvalidated summary can enter a fragment. `DC-NODE-35` flips `declared → enforced` at `/cluster-close`. **One invariant family:** candidate-construction BLUE-safety.

## 9. Design Summary
`build_candidate_fragment(anchor, anchor_block_no, current_tip_block_no, anchor_chain_dep, headers, ledger_view, era_schedule) -> Result<CandidateFragment, CandidateBuildError>`: seed a working chain_dep from the **supplied** `anchor_chain_dep`; for each `HeaderInput` in order, call BLUE `validate_and_apply_header` (evolving the chain_dep), collect its `ValidatedHeaderSummary` (or fail closed on any `HeaderValidationError`); assemble `CandidateFragment { anchor, anchor_block_no, headers: [validated summaries], select_view: tip summary's TiebreakerView, rollback_depth: current_tip_block_no − anchor_block_no }`. `assemble_candidate_set(Vec<CandidateFragment>)` returns a deterministically-ordered set for `select_best_chain` (canonical sort; arrival / peer order immaterial). **Byte authority:** hash-critical fields (block / body hash) ride through as the preserved-wire bytes carried in the `HeaderInput` / summary; the comparison surface (`block_no`, `TiebreakerView`) is the project-canonical value `validate_and_apply_header` derives — never a peer claim. `current_tip_block_no` is a supplied selector value (no store read); `rollback_depth` is the saturating difference.

## 10. Changes Introduced
- **Types (GREEN, none canonical / persisted):** `CandidateBuildError` (closed: `HeaderInvalid(HeaderValidationError)`, `EmptyHeaders`); optionally a small input record for set-assembly. No new BLUE type.
- **State transitions:** none (pure projection; the working chain_dep is a local accumulator threaded through `validate_and_apply_header`).
- **Persistence:** none.
- **Removal / refactors:** none.

## 11. Replay / Crash / Epoch Validation
- **Determinism tests** (`ade_node::candidate_aggregator`): `build_candidate_fragment_two_runs_byte_identical` (same inputs → byte-identical fragment); `assemble_candidate_set_ordering_is_arrival_independent` (permuted input order → identical set). Pure, no persisted state → no crash/replay obligation beyond determinism; nothing is materialized, so it is replay-neutral by construction.
- **Crash/restart:** not applicable (no persisted state).
- **Epoch boundary:** not applicable (the supplied `anchor_chain_dep` already encodes the epoch nonce basis).

## 12. Mechanical Acceptance Criteria
- [ ] `build_candidate_fragment_assembles_from_validated_headers` — valid `HeaderInput`s over a fixture `anchor_chain_dep` (the `chain_selector` `header_at(...)` pattern) → a fragment whose `headers` equal the `validate_and_apply_header` summaries; `rollback_depth == current_tip_block_no − anchor_block_no`.
- [ ] `build_candidate_fragment_rejects_invalid_header_fails_closed` — a header that fails `validate_and_apply_header` (bad VRF) → `CandidateBuildError::HeaderInvalid`; **no fragment built, nothing minted.**
- [ ] `build_candidate_fragment_empty_headers_fails_closed` — zero headers above the anchor → `CandidateBuildError::EmptyHeaders`.
- [ ] `build_candidate_fragment_two_runs_byte_identical` + `assemble_candidate_set_ordering_is_arrival_independent`.
- [ ] New gate **`ci/ci_check_candidate_construction_validated.sh`** green: (A) fragment headers come **only** from `validate_and_apply_header` output (no `ValidatedHeaderSummary` literal / mint; no `ade_core_interop::follow` import); (B) **pure** — the module references no `ChainDb` / `SnapshotReader` / `materialize_rolled_back_state` / `WalStore` / `select_best_chain` / socket / clock; (C) deterministic — `BTreeMap` / sort, no `HashMap`; (D) fail-closed — a `HeaderValidationError` yields `CandidateBuildError`, never a fragment.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes
- Header validation failure → `CandidateBuildError::HeaderInvalid` (fail-closed; candidate dropped, never minted).
- No headers above the anchor → `CandidateBuildError::EmptyHeaders` (fail-closed).
- All deterministic; none affects consensus / replay (pure, read-nothing, latent).

## 14. Hard Prohibitions
**Inherits all nine cluster hard lines** (cluster doc §8). **Slice-specific:**
- **S2 may not synthesize `ValidatedHeaderSummary` from peer claims. Every fragment element must be derived from `validate_and_apply_header` output, or rejected.**
- No store reads, no `materialize`, no `select_best_chain`, no block-fetch, no WAL, no durable mutation — the module is a pure projection.
- No `HashMap` (deterministic `BTreeMap` / sorted ordering).
- No peer-supplied `block_no` / `slot` / VRF taken as authority (the validated transition derives them).
- No new BLUE; `validate_and_apply_header` reused unchanged.

## 15. Explicit Non-Goals
No live dispatch (S3); no fork-point `materialize` / store access (S3); no peer block-fetch (S4); no fork-switch apply (S4); no `select_best_chain`; no new BLUE; no persistence.

**Forward obligation to S3:** the live RED driver **must obtain `anchor_chain_dep` by read-only `materialize_rolled_back_state` from Ade's durable stored fork anchor** before invoking this S2 core — so peer-supplied fork state can never be passed into `build_candidate_fragment`. (Recorded here so it cannot be lost when S3 wires the driver.)

## 16. Completion Checklist
- [ ] `ade_node::candidate_aggregator` GREEN module added (`build_candidate_fragment` + `assemble_candidate_set` + `CandidateBuildError`).
- [ ] Fragment headers derive only from `validate_and_apply_header`; invalid / empty fail closed.
- [ ] Pure: no store / materialize / select / WAL / IO references; `BTreeMap` / sorted determinism.
- [ ] Determinism + validated-only + fail-closed tests pass; new gate green; `cargo test -p ade_node` green.
- [ ] No persisted / canonical type; `DC-NODE-35` ready to flip at `/cluster-close`; the S3 forward obligation recorded.
