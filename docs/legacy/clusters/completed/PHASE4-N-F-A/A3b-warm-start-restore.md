# Slice PHASE4-N-F-A / A3b — bootstrap-authority warm-start restore capability (WAL-proven sidecar)

> **Scope honesty (revised 2026-05-30, pre-implementation).** A3b is an
> **authority-surface capability**, NOT a production wiring. The original
> draft described a "production warm-start restore"; the code shows no
> production-wired warm-start path exists yet:
> - `node.rs` `run_node_until_shutdown` has only `#[cfg(test)]` callers and
>   owns no `WalStore` / `anchor_fp` (its `leadership_anchor` is a
>   `SlotEraAnchor`, not a `BootstrapAnchor` fingerprint);
> - the two production `produce_mode.rs` callers **cold-start** (an
>   `InMemoryChainDb` + `genesis_initial: Some(..)`), so they have no
>   persisted sidecar and legitimately no provenance — A3b does **not**
>   convert them;
> - the only production `WalStore` owner today is `Mode::Admission`
>   (`admission/bootstrap.rs`), which does **not** call
>   `bootstrap_initial_state` with provenance and still sources inputs from
>   `--consensus-inputs-path`.
>
> So, exactly as A3a enforced the provenance invariant on the (test-exercised)
> composer surface without production-wiring it, A3b implements + tests the
> `bootstrap_initial_state` warm-start **verification chain** so a future
> production-wiring cluster can consume it safely. **Production wiring is a
> later cluster/slice, out of A3b scope.** Fail-closed verification semantics
> hold for any warm-start path that supplies provenance.

## 2. Slice Header
- **Slice Name:** `bootstrap_initial_state` warm-start gains a fail-closed verification chain that restores + verifies the seed-epoch consensus-input sidecar against the WAL provenance view (authority-surface capability; not production-wired).
- **Cluster:** PHASE4-N-F-A. **Status:** Merged (`104982d`); CE-A-3 closed (authority-surface).
- **Cluster Exit Criteria Addressed:** **completes CE-A-3** as an **authority-surface** proof — `bootstrap_initial_state` warm-start, given a persisted sidecar + its WAL provenance, restores the seed-epoch consensus inputs **byte-identically** and fail-closed. The test exercises the warm-start branch directly (persist + A3a WAL append → `bootstrap_initial_state` warm-start → recovered sidecar byte-identical), not a production `main.rs` flow. The remaining *production-wiring* of that warm-start (a real caller threading a `WalStore`/anchor) is a later cluster — CE-A-3's byte-identity claim is mechanically proven on the authority surface here; the production-path claim is explicitly deferred.
- **Slice Dependencies:** A1 (codec), A2 (keyed sidecar + population), A3a (WAL provenance entry + `RecoveredBootstrapProvenance`).

## 3. Implementation Instruction (AI)
Implement §9/§10 only — the `bootstrap_initial_state` warm-start **verification-chain capability**, NOT production wiring. Cold-start unchanged; all 12 call sites updated to the new signature in the not-required mode. No projection (A4), no produce wiring, no `Mode::Admission` wiring, no `produce_mode` conversion, no claim of production restart safety. Commit with the trailer.

## 4. Intent
Make the `bootstrap_initial_state` **warm-start branch** (the sole bootstrap authority) able to restore the seed-epoch consensus inputs **only** from WAL-proven, hash-verified, anchor-bound recovered state — fail-closed on any gap, **never** re-importing a forge-time `--consensus-inputs-path` bundle. This is the authority-surface capability a future production restart path will consume; A3b does **not** itself production-wire that path (see the scope-honesty note above). *(Completes candidate `DC-CINPUT-01` on the authority surface.)*

## 5. Scope
- **Modules:** `ade_runtime::bootstrap` (`BootstrapInputs` gains the `seed_epoch_consensus_source: SeedEpochConsensusSource` field; warm-start verification chain; new `BootstrapState` output struct; new `BootstrapError` variants). Verification helper (hash + binding checks) may be a BLUE fn. **All existing `bootstrap_initial_state` call sites are updated to keep affected-crate builds green** — they pass `SeedEpochConsensusSource::NotRequired` and destructure `BootstrapState`, which is behavior-unaffected.
- **State machines:** the warm-start branch of `bootstrap_initial_state` (cold-start branch unchanged).
- **Persistence:** read-only (consumes A2 sidecar + A3a WAL provenance); no new writes.
- **NOT in scope (production wiring — explicitly deferred):** converting `node.rs` to own/pass a `WalStore`+anchor (it is test-called today and owns neither); converting the `produce_mode.rs` cold-start callers; wiring `Mode::Admission` (the production `WalStore` owner) to this path. These belong to a later production-wiring cluster.
- **Out of scope:** projection to `PoolDistrView`/`ExpectedVrfInput` (A4); produce wiring; `recover_node_state` (stays test-only secondary).

## 6. Execution Boundary (TCB)
- **BLUE:** the verification predicate (hash == `provenance.sidecar_hash`; `sidecar.anchor_fp == provenance.anchor_fp`; `sidecar.epoch_no == provenance.epoch_no`; byte-identity re-encode) — pure; may live as a BLUE fn.
- **GREEN:** the warm-start restore reducer glue in `bootstrap_initial_state` (`ade_runtime` GREEN-by-content), if separable.
- **RED:** the `SnapshotStore::get_seed_epoch_consensus_inputs` read inside `bootstrap_initial_state`. *(No `node.rs` wiring in A3b — deferred, §16. No WAL `read_all` here either: the provenance view arrives pre-replayed via the input enum.)*

## 7. Invariants Preserved
- `CN-NODE-01` — `bootstrap_initial_state` remains the single bootstrap authority; warm-start gains WAL-verification on its existing branch, not a parallel path. The new input is an **optional WAL reader / provenance view**, never arbitrary bundle data; `provenance: None` preserves today's behavior for every current (cold-start / not-yet-wired) caller.
- `CN-CINPUT-02` (A2 containment) + `DC-CINPUT-01`-foundation (A3a) — consumption reads the WAL-proven sidecar only.
- `T-REC-01`/`T-REC-02` — extended: the recovered consensus inputs are part of replay-equivalent recovered state.
- Cold-start branch unchanged (a fresh genesis cold-start that has not yet imported has no warm-start consume).

## 8. Invariants Strengthened or Introduced
- **Completes** candidate `DC-CINPUT-01` **at the authority surface** — *the `bootstrap_initial_state` warm-start, given the WAL-replayed provenance view, restores the seed-epoch consensus inputs byte-identically, verified against the sidecar hash + anchor/epoch binding; fail-closed; no bundle fallback.* The production-path instantiation of this capability is deferred (§16). Strengthens `CN-NODE-01`, `T-REC-01`/`T-REC-02`.

## 9. Design Summary
- **Input — a closed mode enum, not loose `Option`+`bool`** (illegal states unrepresentable). `BootstrapInputs` gains one field `seed_epoch_consensus_source: SeedEpochConsensusSource`, where:
  ```rust
  pub enum SeedEpochConsensusSource {
      /// No seed-epoch provenance demanded. Cold-start, and every
      /// current (not-yet-wired) warm-start caller. Old behavior.
      NotRequired,
      /// Warm-start MUST restore + verify the sidecar against this
      /// already-WAL-replayed (A3a) provenance view, fail-closed.
      RequiredFromRecoveredProvenance(RecoveredBootstrapProvenance),
  }
  ```
  This is preferred over `Option<RecoveredBootstrapProvenance>` + a `require_*: bool` because the enum makes "required-but-absent" and "present-but-not-required" unrepresentable. *(A raw `&dyn WalStore` input is rejected: no current caller owns a `WalStore` to pass, and A3a already returns the typed `RecoveredBootstrapProvenance` view — that view IS the input. A future production-wiring cluster replays the WAL to obtain it.)* The sidecar bytes are read from the existing `snapshot_store: &S` (A2's `get_seed_epoch_consensus_inputs`) — no new store handle.
- **Output — a named struct, not a widened tuple** (auditable):
  ```rust
  pub struct BootstrapState {
      pub ledger: LedgerState,
      pub chain_dep: PraosChainDepState,
      pub tip: Option<ChainTip>,
      pub seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs>,
  }
  ```
  `seed_epoch_consensus_inputs` is `Some` only on a `RequiredFromRecoveredProvenance` warm-start that verified; `None` on cold-start and `NotRequired` warm-start. *(Replaces the bare `(LedgerState, PraosChainDepState, Option<ChainTip>)` triple — all 12 call sites updated to destructure the struct, ignoring the new field where unused.)*
- **Behavior matrix** (deterministic):
  - **cold-start** (any source): unchanged; returns the genesis pair, `seed_epoch_consensus_inputs: None`. Never consumes a forge-time bundle.
  - **warm-start + `NotRequired`**: old materialize behavior; `seed_epoch_consensus_inputs: None`. (Tests / diagnostics / today's callers.)
  - **warm-start + `RequiredFromRecoveredProvenance(P)`**: run the verification chain below; on success return the recovered sidecar in the output struct.
- **Warm-start verification chain** (`RequiredFromRecoveredProvenance(P{A,H,E})`; typed, fail-closed; **no `--consensus-inputs-path` fallback**):
  1. `snapshot_store.get_seed_epoch_consensus_inputs(A)` → sidecar bytes (absent → `SeedConsensusSidecarMissing`);
  2. `blake2b_256(bytes) == H` (else `SeedConsensusHashMismatch`);
  3. `decode_seed_epoch_consensus_inputs(bytes)` (decode/version error → `SeedConsensusSidecarMissing`/decode variant);
  4. `sidecar.anchor_fp == A` && `sidecar.epoch_no == E` (else `SeedConsensusBindingMismatch`);
  5. **byte-identity**: `encode_seed_epoch_consensus_inputs(&sidecar) == bytes` (the A1 codec already enforces byte-canonical round-trip on decode, but re-assert here so CE-A-3's byte-identity claim is explicit at the authority surface);
  6. expose the recovered `SeedEpochConsensusInputs` in `BootstrapState`.
- **Fail-closed:** A3b proves the four binding/hash/decode chain failures via tests (missing sidecar, hash mismatch, anchor mismatch, epoch mismatch, malformed sidecar). The `SeedConsensusProvenanceMissing` variant is **reserved**: with the `SeedEpochConsensusSource` enum, "required-but-no-view" is unrepresentable through the public API, so the variant is **not constructible (and not test-provable) in A3b** — it exists for the future production-wiring layer, which may reach a required warm-start whose WAL replay yielded no view. It is kept (and mapped in `node.rs`'s `exit_code`) so the future slice need not re-open the error enum. Cold-start branch: unaffected.

## 10. Changes Introduced
- **Types:** new closed `SeedEpochConsensusSource` enum (`NotRequired` | `RequiredFromRecoveredProvenance(RecoveredBootstrapProvenance)`); `BootstrapInputs` gains one `seed_epoch_consensus_source` field of that type; new named output struct `BootstrapState { ledger, chain_dep, tip, seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs> }` replacing the bare triple; new typed `BootstrapError` variants (`SeedConsensusProvenanceMissing`, `SeedConsensusSidecarMissing`, `SeedConsensusHashMismatch`, `SeedConsensusBindingMismatch`). All carry only non-secret primitives (no `String`/`anyhow`).
- **Transitions:** `bootstrap_initial_state` warm-start gains the verification chain (fires on `RequiredFromRecoveredProvenance`); returns the recovered `SeedEpochConsensusInputs` in `BootstrapState`. Cold-start and `NotRequired` warm-start return `seed_epoch_consensus_inputs: None`.
- **Callers (compile-green only, NOT production wiring):** all 12 `bootstrap_initial_state` call sites updated to pass `seed_epoch_consensus_source: SeedEpochConsensusSource::NotRequired` and destructure `BootstrapState` (ignoring the new field). No caller is converted to *supply* provenance in A3b — that is the deferred production-wiring slice. `node.rs` is **not** given a `WalStore`/anchor it does not currently own; `produce_mode` is **not** converted from cold-start.

## 11. Replay, Crash, Epoch Validation
- **Tests** (all exercise `bootstrap_initial_state` directly on the authority surface — NOT a `main.rs` flow): `warm_start_restores_seed_epoch_consensus_inputs_byte_identical` (persist sidecar + build the A3a `RecoveredBootstrapProvenance` → `bootstrap_initial_state` warm-start in `RequiredFromRecoveredProvenance` mode → `BootstrapState.seed_epoch_consensus_inputs` is `Some` and byte-identical to the persisted sidecar; all checks pass); fail-closed tests — `warm_start_fails_closed_on_{missing_sidecar,hash_mismatch,anchor_mismatch,epoch_mismatch}` and `warm_start_required_provenance_rejects_malformed_sidecar`; `cold_start_ignores_seed_epoch_source` (cold-start with a `RequiredFromRecoveredProvenance` source returns `None`, never errors); `warm_start_not_required_is_unchanged` (NotRequired warm-start = pre-A3b behavior, `None`); `warm_start_never_falls_back_to_consensus_inputs_path` (the production portion of `bootstrap.rs` references no forge-time bundle token). *(`SeedConsensusProvenanceMissing` has no test — it is unconstructible in A3b by design; see §9.)*
- **Crash:** sidecar-then-WAL ordering (A3a) ⇒ a crash before the WAL append → no provenance entry on replay → the caller has no `RecoveredBootstrapProvenance` to pass → cannot enter `RequiredFromRecoveredProvenance` for that anchor → fail-closed (no half-state, no bundle re-import). *(A3b proves the verification-side fail-closed; the replay-side "no provenance entry" is A3a's, already shipped.)*

## 12. Mechanical Acceptance Criteria
- [ ] `warm_start_restores_seed_epoch_consensus_inputs_byte_identical` passes (**CE-A-3**, authority-surface byte-identity — `bootstrap_initial_state` warm-start, not a `main.rs` flow).
- [ ] the four `warm_start_fails_closed_on_{missing_sidecar,hash_mismatch,anchor_mismatch,epoch_mismatch}` tests + `warm_start_required_provenance_rejects_malformed_sidecar` pass.
- [ ] `cold_start_ignores_seed_epoch_source`, `warm_start_not_required_is_unchanged`, `warm_start_never_falls_back_to_consensus_inputs_path` pass.
- [ ] `cargo build` + `cargo clippy` clean for the changed crates; **affected-crate** tests green (`cargo test -p ade_runtime -p ade_node` + the BLUE crates) — NOT `cargo test --workspace`, since `ade_testkit`'s corpus/oracle suite times out on clean HEAD too (pre-existing/environmental; see the A3a closure + memory `reference_ade_testkit_corpus_suite_times_out`). `ci_check_consensus_input_provenance.sh` still passes.

## 13. Failure Modes
Typed `BootstrapError`, fail-closed, no panic, no bundle fallback, in `RequiredFromRecoveredProvenance` warm-start: sidecar missing (`SeedConsensusSidecarMissing`); hash mismatch (`SeedConsensusHashMismatch`); anchor or epoch mismatch (`SeedConsensusBindingMismatch`); the provenance-view-lost guard (`SeedConsensusProvenanceMissing`); decode/version error on the sidecar. All deterministic; halt the warm-start rather than proceed on unverified consensus inputs. `NotRequired` and cold-start never error on these axes.

## 14. Hard Prohibitions
**Inherited (cluster).** **Slice-specific:** **no fallback to `--consensus-inputs-path`** (any mismatch fails closed); the bootstrap input is the typed A3a `RecoveredBootstrapProvenance` view (via the `SeedEpochConsensusSource` enum), **not** arbitrary bundle data and **not** a raw `WalStore`; no sentinel slot; no parallel bootstrap authority; `recover_node_state` is **not** the production proof (test-only secondary); **no production wiring** — `node.rs` is not given a `WalStore`/anchor, `produce_mode` is not converted from cold-start, `Mode::Admission` is not wired to this path; no claim of production restart safety; no projection (A4); no produce wiring; no `String`/`anyhow` in BLUE verification.

## 15. Explicit Non-Goals
No production wiring of the warm-start path (deferred — see §16). No projection to `PoolDistrView`/`ExpectedVrfInput` (A4). No produce wiring. No `recover_node_state` production wiring. No META pointer.

## 16. Deferred — production wiring (later slice, NOT A3b)
A3b ships only the **capability**. A separate later slice (candidate **PHASE4-N-F-A5 — Production bootstrap provenance wiring**, or folded into N-F-B's production-handoff slice) must connect the real path end-to-end:

> `Mode::Admission` / node startup opens the WAL → replay reads `RecoveredBootstrapProvenance` (A3a) → `bootstrap_initial_state` receives `SeedEpochConsensusSource::RequiredFromRecoveredProvenance(view)` → warm-start returns the verified `SeedEpochConsensusInputs` in `BootstrapState` → produce/leader path consumes the recovered state (with the A4 projection).

That slice owns: giving the real startup caller a `WalStore`+`anchor_fp`, choosing where in `main.rs` dispatch the required-provenance warm-start runs, and the operator-facing fail-closed behavior. It must not be smuggled into A3b.
