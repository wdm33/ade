# Slice PHASE4-N-F-A / A3b — production warm-start restore (WAL-proven sidecar)

## 2. Slice Header
- **Slice Name:** `bootstrap_initial_state` warm-start restores + verifies the seed-epoch consensus-input sidecar via the WAL provenance view, fail-closed.
- **Cluster:** PHASE4-N-F-A. **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** **completes CE-A-3** (production warm-start byte-identity — the core safety line).
- **Slice Dependencies:** A1 (codec), A2 (keyed sidecar + population), A3a (WAL provenance entry + `RecoveredBootstrapProvenance`).

## 3. Implementation Instruction (AI)
Implement §9/§10 only. Wire the production warm-start restore + verification. No projection (A4), no produce wiring. Commit with the trailer.

## 4. Intent
Make the **production** restart path (`bootstrap_initial_state` warm-start, `node.rs`) restore the seed-epoch consensus inputs **only** from WAL-proven, hash-verified, anchor-bound recovered state — fail-closed on any gap, **never** re-importing a forge-time `--consensus-inputs-path` bundle. *(Completes candidate `DC-CINPUT-01`.)*

## 5. Scope
- **Modules:** `ade_runtime::bootstrap` (`BootstrapInputs` gains a WAL/provenance reader; warm-start verification); `ade_node::node.rs` (caller). Verification helper (hash + binding checks) may be a BLUE fn.
- **State machines:** the cold-start/warm-start branch of `bootstrap_initial_state`.
- **Persistence:** read-only (consumes A2 sidecar + A3a WAL provenance); no new writes.
- **Out of scope:** projection to `PoolDistrView`/`ExpectedVrfInput` (A4); produce wiring; `recover_node_state` (stays test-only secondary).

## 6. Execution Boundary (TCB)
- **BLUE:** the verification predicate (hash == `WAL.sidecar_hash`; `sidecar.anchor_fp == provenance.anchor_fp`; `sidecar.epoch_no == provenance.epoch_no`) — pure.
- **GREEN:** the warm-start restore reducer glue (`ade_runtime` GREEN-by-content), if separable.
- **RED:** the WAL `read_all` + `SnapshotStore::get_seed_epoch_consensus_inputs` reads inside `bootstrap_initial_state`; the `node.rs` wiring.

## 7. Invariants Preserved
- `CN-NODE-01` — `bootstrap_initial_state` remains the single bootstrap authority; warm-start gains WAL-verification, not a parallel path. The new input is a **WAL reader / provenance view**, never arbitrary bundle data.
- `CN-CINPUT-02` (A2 containment) + `DC-CINPUT-01`-foundation (A3a) — consumption reads the WAL-proven sidecar only.
- `T-REC-01`/`T-REC-02` — extended: the recovered consensus inputs are part of replay-equivalent recovered state.
- Cold-start branch unchanged (a fresh genesis cold-start that has not yet imported has no warm-start consume).

## 8. Invariants Strengthened or Introduced
- **Completes** candidate `DC-CINPUT-01` — *the production warm-start restores the seed-epoch consensus inputs byte-identically, verified against the WAL provenance + sidecar hash + anchor/epoch binding; fail-closed; no bundle fallback.* Strengthens `CN-NODE-01`, `T-REC-01`/`T-REC-02`.

## 9. Design Summary
- `BootstrapInputs` gains a WAL/provenance reader handle (a `&dyn WalStore` or a pre-replayed `RecoveredBootstrapProvenance`); `node.rs` (and any other callers) updated. *(Signature change to the sole bootstrap authority — keep the workspace green.)*
- **Warm-start verification chain** (typed, fail-closed; **no `--consensus-inputs-path` fallback**):
  1. obtain `RecoveredBootstrapProvenance{A, H, E}` from the WAL (A3a);
  2. `get_seed_epoch_consensus_inputs(A)` → sidecar bytes;
  3. `blake2b256(bytes) == H`;
  4. `decode_seed_epoch_consensus_inputs(bytes)`; `sidecar.anchor_fp == A`; `sidecar.epoch_no == E`;
  5. expose the recovered `SeedEpochConsensusInputs`.
- **Fail-closed:** warm-start branch with WAL provenance **absent** → fail; sidecar missing / hash mismatch / anchor mismatch / epoch mismatch → fail (typed `BootstrapError` variants). Cold-start branch: unaffected.

## 10. Changes Introduced
- **Types:** `BootstrapInputs` WAL/provenance field; new typed `BootstrapError` variants (`SeedConsensusProvenanceMissing`, `SeedConsensusSidecarMissing`, `SeedConsensusHashMismatch`, `SeedConsensusBindingMismatch`).
- **Transitions:** `bootstrap_initial_state` warm-start gains the verification chain; returns the recovered `SeedEpochConsensusInputs` (alongside the existing triple).
- **Caller:** `node.rs` passes the WAL/provenance reader.

## 11. Replay, Crash, Epoch Validation
- **Tests:** `warm_start_restores_seed_epoch_consensus_inputs_byte_identical` (production path: bootstrap persist + A3a WAL append → `bootstrap_initial_state` warm-start → recovered sidecar byte-identical, all checks pass); fail-closed tests — `warm_start_fails_closed_on_{absent_provenance,missing_sidecar,hash_mismatch,anchor_mismatch,epoch_mismatch}`; `warm_start_never_falls_back_to_consensus_inputs_path`.
- **Crash:** sidecar-then-WAL ordering (A3a) ⇒ a crash before the WAL append → warm-start sees no provenance → fail-closed (no half-state, no bundle re-import).

## 12. Mechanical Acceptance Criteria
- [ ] `warm_start_restores_seed_epoch_consensus_inputs_byte_identical` passes (**CE-A-3**, production path).
- [ ] the five `warm_start_fails_closed_on_*` tests pass.
- [ ] `cargo build --workspace` + `cargo clippy` clean; `cargo test --workspace` green; `ci_check_consensus_input_provenance.sh` still passes.

## 13. Failure Modes
Typed `BootstrapError`, fail-closed, no panic, no bundle fallback: provenance absent in warm-start; sidecar missing; hash mismatch; anchor mismatch; epoch mismatch. All deterministic; halt the warm-start rather than proceed on unverified consensus inputs.

## 14. Hard Prohibitions
**Inherited (cluster).** **Slice-specific:** **no fallback to `--consensus-inputs-path`** (any mismatch fails closed); the bootstrap input is a WAL reader/provenance view, **not** arbitrary bundle data; no sentinel slot; no parallel bootstrap authority; `recover_node_state` is **not** the production proof (test-only secondary); no projection (A4); no produce wiring; no `String`/`anyhow` in BLUE verification.

## 15. Explicit Non-Goals
No projection to `PoolDistrView`/`ExpectedVrfInput` (A4). No produce wiring. No `recover_node_state` production wiring. No META pointer.
