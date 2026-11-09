# Slice PHASE4-N-F-C / L1 — Lifecycle owner skeleton + branch (`--mode node`)

> **Supersedes the withdrawn "C1 — evolve `Mode::Admission` (Option A)" slice.** Reading the actual code
> disproved the premise: `admission::run_admission_inner` builds ledger/chain_dep manually via
> `seed_to_snapshot` and **never calls `bootstrap_initial_state`** (its module banner claims a
> `bootstrap_initial_state` warm-start that the body does not perform). Evolving admission would mean
> *adding* the authority call to a mode whose job is peer-verdict comparison. The owner decision is
> therefore resolved the other way: a **dedicated `--mode node` lifecycle owner** built on the paths that
> already route through the single bootstrap authority. `produce_mode` and `admission` are demoted to
> diagnostic; neither is the Ade node lifecycle.

## 2. Slice Header
- **Slice Name:** Stand up the `--mode node` Ade node lifecycle owner over `PersistentChainDb` +
  `FileWalStore`, with a first-run-vs-warm-start branch that is a pure function of on-disk state, routed
  solely through `bootstrap_initial_state`. Repair the two stale-on-`main` CN-NODE-01 gates.
- **Cluster:** PHASE4-N-F-C — Build the real Ade node lifecycle.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-L-1.
- **Slice Dependencies:** none (first slice). Consumes only shipped surfaces.

## 3. Implementation Instruction (AI)
Implement §10 only. Stand up the `--mode node` owner skeleton + the on-disk branch + repair the two gates.
Do NOT wire Mithril first-run composition (L2), warm-start recovery (L3), peer-fetch→apply (L4), produce
handoff (L5), or BA-02 evidence (L6). Do NOT make `produce_mode` or `admission` the lifecycle owner. Leave
both working as diagnostic modes. Commit with the model-attribution trailer.

## 4. Intent
Make it impossible for PHASE4-N-F-C to proceed without exactly one named lifecycle owner that obtains
initial state **solely** through `bootstrap_initial_state` (CN-NODE-01) and whose first-run-vs-warm-start
decision is a pure function of what is persisted on disk. No Mithril/genesis/bundle/cold fallback path
exists in the branch.

## 5. Scope
- **Modules / crates:** `ade_node` (RED) — new `Mode::Node` + its `run_node_lifecycle` owner over
  `PersistentChainDb` + `FileWalStore`; `cli.rs` + `main.rs` closed-mode dispatch; `ci/` — the two repaired
  gates + the candidate owner gate.
- **State machines affected:** none (the branch is a pure on-disk-state classifier; no new authoritative
  transition).
- **Persistence impact:** opens `PersistentChainDb` + `FileWalStore`; persists nothing new in L1 (L2 adds
  the sidecar; L4 adds blocks).
- **Network-visible impact:** none in L1.
- **Out of scope:** L2–L6 entirely.

## 6. Execution Boundary (TCB color)
- **BLUE:** none changed.
- **GREEN:** the first-run-vs-warm-start branch classifier (pure over on-disk state) — `ade_runtime`
  GREEN-by-content, or inline in the RED owner with a banner; resolve at implement time.
- **RED:** the `--mode node` owner/driver (store/WAL open, dispatch).
- **CI:** repaired `ci_check_node_mode_closure.sh` (mode set grows to include `Node`, still closed, no
  wildcard arm); repaired `ci_check_bootstrap_closure.sh` (expects the current `BootstrapState` return,
  not the pre-A3b tuple); candidate `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`.

## 7. Invariants Preserved
- `CN-NODE-01` — `bootstrap_initial_state` stays the sole bootstrap pub fn; the new owner calls it; no
  second authority.
- Mode-closure — the dispatch stays exhaustive, no wildcard arm, after adding `Node`.
- `produce_mode` / `admission` diagnostic paths keep working unchanged.
- All BLUE forbidden-pattern / dependency-boundary / determinism invariants (no BLUE touched).

## 8. Invariants Strengthened or Introduced
- **Strengthens `CN-NODE-01`** — adds a mechanical proof that the *lifecycle owner* (not just
  `bootstrap_initial_state` in isolation) obtains initial state solely via the authority. No registry edit
  in-slice; `strengthened_in += "PHASE4-N-F-C"` is recorded at `/cluster-close`.

## 9. Design Summary
A dedicated `Mode::Node` whose owner `run_node_lifecycle`:
1. opens `PersistentChainDb` + `FileWalStore`;
2. classifies first-run (empty store) vs warm-start (non-empty) — a pure function of on-disk state, no
   wall-clock/env;
3. routes both arms through `bootstrap_initial_state` (first-run arm supplies the Mithril-composed
   genesis_initial in L2; warm-start arm supplies `RequiredFromRecoveredProvenance` in L3 — L1 leaves both
   arms as typed stubs that fail closed rather than fall back).

The two existing gates are repaired because they are already RED on clean `main` (mode-closure predates
`KeyGenKes`/`Produce`; bootstrap-closure predates the N-F-A `BootstrapState` return) — CN-NODE-01 hygiene
is in scope for a cluster whose primary invariant is CN-NODE-01.

## 10. Changes Introduced
### Types
- `Mode::Node` variant (closed); the owner's typed first-run/warm-start branch enum.
### State Transitions
- None authoritative.
### Persistence
- Opens the persistent stores; no new persisted record in L1.
### Removal / Refactors
- None to `produce_mode`/`admission` (demoted by scope, not deleted). Gate repairs only.

## 11. Replay, Crash, and Epoch Validation
- Replay tests: none new in L1 (no authoritative state added). Replay obligations are owed by L3/L4c/L5.
- Crash/restart: L1 adds no persist/recover path; the branch classifier is unit-tested as a pure function.
- Epoch boundary: n/a.

## 12. Mechanical Acceptance Criteria
- [ ] `ci/ci_check_node_mode_closure.sh` passes (repaired; `Node` in the closed set; no wildcard arm).
- [ ] `ci/ci_check_bootstrap_closure.sh` passes (repaired for the current `BootstrapState` return).
- [ ] `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` exists and passes (the `--mode node`
      owner obtains initial state solely via `bootstrap_initial_state`).
- [ ] Branch classifier unit test: empty store ⇒ first-run, non-empty ⇒ warm-start, pure (two runs
      identical).
- [ ] `cargo build` + `cargo clippy` clean.
- [ ] Scoped affected-crate tests + relevant CI gates pass. Full `ade_testkit` corpus/oracle lane is NOT a
      C1 gate (times out ~600s on clean HEAD).

## 13. Failure Modes
L1 adds no runtime transition; failure = mechanical-gate failure. The first-run/warm-start arms are typed
stubs that **fail closed** (no Mithril/genesis/bundle/cold fallback) until L2/L3 fill them — a half-wired
owner does not merge.

## 14. Hard Prohibitions
**Inherited (cluster):** no first-run bootstrap without verified Mithril provenance; no
genesis/`--consensus-inputs-path`/tip-bundle/cold fallback; no second bootstrap/recovery/storage-init
authority; no shape-swap; no new BLUE authority/type; no `HashMap`/clock/float/async in BLUE.
**Slice-specific:** no L2–L6 work; no making `produce_mode`/`admission` the owner; no Mithril query in L1;
no registry promotion; no grounding-doc regeneration.

## 15. Explicit Non-Goals
No Mithril composition (L2), recovery (L3), fetch→apply (L4), produce (L5), BA-02 (L6). No registry append.
No grounding-doc refresh.

## 16. Completion Checklist
- [ ] `Mode::Node` + `run_node_lifecycle` owner stood up over `PersistentChainDb` + `FileWalStore`.
- [ ] First-run/warm-start branch is a pure function of on-disk state, both arms route through
      `bootstrap_initial_state`, both fail closed pending L2/L3.
- [ ] Two stale CN-NODE-01 gates repaired + the candidate owner gate added; all green.
- [ ] `produce_mode`/`admission` still build and run as diagnostic modes.
- [ ] `cargo build`/`clippy` clean; scoped tests + gates green.
