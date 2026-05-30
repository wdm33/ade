# Slice PHASE4-N-F-C / C1 — Choose & stand up the production lifecycle owner

## 2. Slice Header
- **Slice Name:** Select the production recovered-state lifecycle owner (recommended: evolve `Mode::Admission`), stand up its owner boundary routed solely through `bootstrap_initial_state`, add the lifecycle-owner CI gate, and record the committed A-vs-B rationale — leaving `run_produce_mode` diagnostic/legacy, not bounty-primary.
- **Cluster:** PHASE4-N-F-C — Producer Recovered-State Lifecycle.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-C-1** — production owner stood up & routed through the single authority (mechanical spine: mode-closure green; bootstrap-closure green; lifecycle owner uses `bootstrap_initial_state`).
- **Slice Dependencies:** none (first slice of the cluster). Consumes only shipped N-F-A/N-K/N-T/N-Y surfaces.

## 3. Implementation Instruction (AI)
Implement §10 only. This is the **owner-selection + skeleton + gate + rationale** slice. Choose **Option A (evolve `Mode::Admission`)** unless a hard blocker is discovered (§9) — but the committed decision artifact `C1-DECISION-production-owner.md` must actually **show the A-vs-B call-graph comparison and the rejection criteria**, not merely assert Option A. Add the candidate gate `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`. Do **NOT** wire production bootstrap composition (C2), warm-start recovery (C3), the consume-side fence (C5), produce handoff (C4), or BA-02 evidence (C6). Do **NOT** make `run_produce_mode` consume the recovered surface. Leave the diagnostic `produce_mode` path working unchanged. Commit with the model-attribution trailer.

## 4. Intent
Make it impossible for PHASE4-N-F-C to proceed with an undecided or duplicated production lifecycle owner: exactly one named owner threads verified-bootstrap/recovery→produce, and it obtains initial state **solely** through `bootstrap_initial_state` — no second bootstrap/storage-init authority. This slice fixes *who owns the lifecycle* and proves the single-authority routing mechanically, so C2–C4 attach to a settled owner. *(Strengthens `CN-NODE-01`; sets up candidate `DC-CINPUT-02b`/`CN-CINPUT-03` consumed by C4/C5.)*

## 5. Scope
- **Modules / crates:**
  - `ade_node` (RED) — the chosen owner boundary. **Recommended (Option A):** `ade_node::admission::bootstrap` (`run_admission_inner`) is named/annotated as the production recovered-state lifecycle owner; `main.rs` dispatch unchanged in shape (mode set stays closed). No new `Mode` variant in Option A.
  - `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` (new candidate gate).
  - `docs/clusters/PHASE4-N-F-C/C1-DECISION-production-owner.md` (new committed rationale).
- **State machines affected:** none. (No transition added; this slice names the owner and adds a CI gate.)
- **Persistence impact:** none. (C2 adds the persistent-store composition; C1 does not persist or recover.)
- **Network-visible impact:** none.
- **Out of scope:** production sidecar persist (C2), warm-start recovery wiring (C3), consume-side containment-gate extension (C5), produce-handoff/forge-base-from-recovered (C4), BA-02 evidence (C6), any change making `run_produce_mode` the bounty-primary recovered-state producer, any registry promotion, any grounding-doc regeneration.

## 6. Execution Boundary (TCB color)
- **BLUE:** none changed. (No new authority — A5 §9.)
- **GREEN:** none in this slice. (The first-run-vs-warm-start branch decision is C3.)
- **RED:** the lifecycle-owner boundary in `ade_node` (Option A: `admission::bootstrap`). Owner I/O (store/WAL/peer) is RED; C1 only names the boundary and asserts its routing — it adds no new I/O.
- **CI (enforcement, not a color):** `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`.
- No ambiguous colors leave implementation: owner = RED; no BLUE/GREEN change.

## 7. Invariants Preserved
- `CN-NODE-01` — `bootstrap_initial_state` (`bootstrap.rs:159`) stays the sole bootstrap pub fn; the named owner already calls it (Option A: `admission/bootstrap.rs` warm-start). No parallel path added.
- `ci_check_node_mode_closure.sh` — the `Mode` set stays closed `{WireOnly, Admission, KeyGenKes, Produce}` with no wildcard dispatch arm (Option A adds no variant; if Option B is taken, the new variant must keep the gate green).
- `CN-CINPUT-01`/`CN-CINPUT-02` — sole codec + populate-side/forge-time fence unchanged; C1 populates nothing.
- `CN-PROD-03`/`CN-PROD-04`/`CN-FORGE-01..04` — the diagnostic `produce_mode` forge/serve path is left working and unchanged.
- All BLUE forbidden-pattern, dependency-boundary, and determinism invariants (no BLUE touched).

## 8. Invariants Strengthened or Introduced
- **Strengthens `CN-NODE-01`** — adds a mechanical proof that the *production lifecycle owner* (not just `bootstrap_initial_state` in isolation) obtains initial state solely via that authority, via the new `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`. (No registry edit in-slice; the `strengthened_in += "PHASE4-N-F-C"` is recorded at `/cluster-close`.)
- Does **not** introduce `DC-CINPUT-02b`/`CN-CINPUT-03` — those are enforced by C4/C5. C1 only establishes the owner they attach to.

## 9. Design Summary

**Decision: choose Option A (evolve `Mode::Admission`) unless the slice discovers a hard blocker.** The C1 decision document must **show the A-vs-B call-graph comparison and the rejection criteria** — not merely assert Option A. The recommendation is the default; the evidence must still be laid out so the owner decision is made *with* the call graph, not before it.

**Why Option A is recommended:**
- **Minimal new lifecycle surface** — `run_admission_inner` (`admission/bootstrap.rs:114`) already owns `mint` (`:151`), `FileWalStore::open` (`:181`), and a `bootstrap_initial_state` warm-start path.
- **Best alignment with `CN-NODE-01`** — it already routes through the single bootstrap authority; no second owner to reconcile.
- **Reuses existing admission bootstrap structure** — anchor + WAL + warm-start are in place; C2/C3 extend, not invent.
- **Avoids standing up a second owner** around `PersistentChainDb` + WAL + recovery + produce.
- **Keeps `run_produce_mode` diagnostic/legacy** rather than bounty-primary — the cold `InMemoryChainDb` + `NotRequired` path (`produce_mode.rs:188–215`) stays a diagnostic/fixture surface.

**Hard-blocker test (escalate to Option B only if):** Option A cannot cleanly own `PersistentChainDb` + `FileWalStore` + recovered-state produce handoff without becoming *more invasive* than a dedicated lifecycle mode — e.g. admission's existing consensus-inputs import (`admission/bootstrap.rs:194`) or its peer-admit responsibilities structurally conflict with owning the produce handoff. If so, document the specific conflict in `C1-DECISION-production-owner.md` and take **Option B — a dedicated producer lifecycle mode** (owning `PersistentChainDb` `chaindb/persistent.rs:85` + `FileWalStore` + `BootstrapAnchor` + recovery `recovery/restart.rs:114`/`node.rs:145` + produce), keeping the mode set closed.

**Enforcement shape:** the owner boundary is annotated (module banner / doc-comment naming it the production recovered-state lifecycle owner). The new CI gate is a data-flow-resistant grep (N-Z/N-F-A `ci_check_*` style, strips `#[cfg(test)]` + comments): (a) the named owner references `bootstrap_initial_state`; (b) no `ade_node`/`ade_runtime` production path constructs initial `(LedgerState, PraosChainDepState, tip)` outside `bootstrap_initial_state`; (c) `run_produce_mode` does **not** pass `RequiredFromRecoveredProvenance` (stays diagnostic — narrow assertion; the full no-shape-swap-anywhere fence is C5n).

## 10. Changes Introduced
### Types
- None.
### State Transitions
- None.
### Persistence
- None.
### Removal / Refactors
- None functional. Only: owner-boundary annotation in the chosen module + the committed decision doc + the new CI gate. `run_produce_mode` untouched.

## 11. Replay, Crash, and Epoch Validation
- **Replay tests added/updated:** none (no authoritative state or transition added — A5 §9). Replay obligations are owed by C3/C4/C7, not C1.
- **Crash/restart behavior:** unchanged — C1 adds no persist/recover path.
- **Epoch boundary:** n/a.

## 12. Mechanical Acceptance Criteria
- [ ] `ci/ci_check_node_mode_closure.sh` passes (mode set still closed; no wildcard dispatch arm).
- [ ] `ci/ci_check_bootstrap_closure.sh` passes (`bootstrap_initial_state` still sole bootstrap authority).
- [ ] `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` exists and passes (named owner obtains initial state solely via `bootstrap_initial_state`; `run_produce_mode` does not pass `RequiredFromRecoveredProvenance`).
- [ ] `docs/clusters/PHASE4-N-F-C/C1-DECISION-production-owner.md` exists and **shows the A-vs-B call-graph comparison and the rejection criteria** + the chosen owner (not a bare assertion of Option A).
- [ ] `cargo build --workspace` + `cargo clippy` clean.
- [ ] Scoped affected-crate tests and relevant CI gates pass. The full `ade_testkit` corpus/oracle lane remains opt-in and is **not** a C1 gate unless test hygiene changes first (it can time out ~600s on clean HEAD; do not reintroduce it as a per-slice gate).

## 13. Failure Modes
This slice adds no runtime transition; "failure" is mechanical-gate failure. If the chosen owner cannot be shown to route solely through `bootstrap_initial_state`, the gate fails and the slice is not mergeable — no approximation. If Option A is blocked, the block is documented and Option B is taken (still single-authority, still mode-closed); the slice does not merge a half-evolved owner.

## 14. Hard Prohibitions
**Inherited (cluster):** no patch of cold `produce_mode` into a recovered-state consumer; no shape-swap of `--consensus-inputs-path` into `SeedEpochConsensusInputs`; no second bootstrap/recovery/storage-init authority; no `InMemoryChainDb` as bounty-primary; no bundle fallback; no BA-02 claim without real peer-accept; no new BLUE authority/type; no `HashMap`/clock/float/async in BLUE.
**Slice-specific:**
- No production sidecar persist (C2), warm-start recovery wiring (C3), consume-side fence (C5), produce handoff (C4), or BA-02 evidence (C6).
- No new `Mode` variant under Option A; if Option B, the new variant MUST keep `ci_check_node_mode_closure.sh` green.
- `run_produce_mode` MUST NOT begin consuming the recovered surface in this slice.
- No registry promotion; no grounding-doc regeneration.

## 15. Explicit Non-Goals
No composition (C2), no recovery (C3), no consume-side containment (C5), no produce-from-recovered handoff (C4), no BA-02 harness (C6). No registry append. No grounding-doc refresh. No change to the diagnostic produce path beyond keeping it non-bounty-primary by scope + CI.

## 16. Completion Checklist
- [ ] Owner chosen (Option A unless documented hard blocker) and boundary annotated.
- [ ] `C1-DECISION-production-owner.md` committed — shows the A-vs-B call-graph comparison + rejection criteria + chosen owner.
- [ ] `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` added and green.
- [ ] Mode-closure + bootstrap-closure gates stay green.
- [ ] `run_produce_mode` remains diagnostic/legacy for this slice and is not represented as the bounty-primary recovered-state producer (fenced by scope + CI).
- [ ] `cargo build` + `cargo clippy` clean; scoped affected-crate tests + relevant CI gates green (full `ade_testkit` corpus lane not a C1 gate).
