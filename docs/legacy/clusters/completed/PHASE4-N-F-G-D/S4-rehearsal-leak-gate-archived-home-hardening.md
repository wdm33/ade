# Invariant Slice — PHASE4-N-F-G-D S4: Rehearsal leak gate archived-home hardening

> **Status:** Planning Artifact (Non-Normative). Normative authority is the registry + CI.

## 2. Slice Header

### Slice Name
Rehearsal leak gate archived-home hardening (make the non-promotability leak cross-check scan **all** real bounty-evidence homes — active **and** archived — and fail closed; no silent skip of the whole check).

### Cluster
**PHASE4-N-F-G-D** — Private-testnet accepted-block bounty dry-run.

### Status
Merged (PHASE4-N-F-G-D close — impl `6bd60c80`; CE-G-D-2 barrier (b) green). Close-surfaced slice — the per-cluster security review (HIGH, verified) found barrier (b) of CN-REHEARSAL-FIDELITY-01 dead; S4 fixed it and the close resumed.

### Cluster Exit Criteria Addressed
- [ ] **CE-G-D-2 (rehearsal-evidence non-promotability — completes barrier (b))** — the leak cross-check (one of the three non-promotability barriers) must actually run against the **real** G-C home(s). Today it targets only the non-existent active home `docs/clusters/PHASE4-N-F-G-C/` behind an `[[ -d ]]` guard, so it silently skips (G-C is archived to `completed/`). S4 makes it scan both the active and archived homes, fail closed on any rehearsal marker (and on a scan error of an existing home), with no whole-check skip — and adds the boundary negative test that proves it.

### Slice Dependencies
- PHASE4-N-F-G-D S2 (`459cf78d`) — created `ci/ci_check_rehearsal_manifest_schema.sh` (the gate S4 fixes).
- PHASE4-N-F-C / G-C (`351d46bc`) — archived G-C to `docs/clusters/completed/PHASE4-N-F-G-C/` (the real home the leak check must cover).

## 3. Implementation Instruction (AI)
In `ci/ci_check_rehearsal_manifest_schema.sh`, replace the single `BOUNTY_HOME` + `[[ -d ]]`-guarded leak scan with a scan over **all** real G-C bounty-evidence homes — `docs/clusters/PHASE4-N-F-G-C/` (active; created at an operator pass) **and** `docs/clusters/completed/PHASE4-N-F-G-C/` (archived). Build the list of **existing** homes first; scan those; distinguish *home absent* (empty contribution — deliberate) from *scan error on an existing home* (fail closed — never swallowed). A rehearsal marker (`^is_rehearsal =` / `^not_bounty_evidence =`) in any existing home fails closed. Add `crates/ade_node/tests/rehearsal_gate_archived_home.rs`: a durable regression test that (a) asserts the gate is green on the clean tree, and (b) plants a rehearsal-marked `.toml` under the **archived** home, runs the gate, asserts it **fails**, and removes the fixture via a Drop guard (clean even on panic). **Do not** touch `rehearsal_evidence.rs` / `rehearsal_pass.rs` (no `toml_escape` change — deferred), the BA-02 bounty gate (separate follow-up), the S1 fence, or any BLUE/containment surface. Commit carries the project attribution trailer (CLAUDE.md).

## 4. Intent
Make the rehearsal-evidence **non-promotability** barrier (b) — "no rehearsal marker may live in a bounty-evidence home" — actually enforced, durably, across G-C's archival, so `CN-REHEARSAL-FIDELITY-01` can legitimately flip to `enforced`. A silently-skipped guard is false confidence on the cluster's core invariant. (Completes CE-G-D-2; preserves S1/S2/S3, the bounty BA-02 gate, and all containment/memory fences unchanged.)

## 5. Scope
- **Modules / crates / gates:**
  - `ci/ci_check_rehearsal_manifest_schema.sh` (FIX, RED) — scan all existing G-C homes; no whole-check skip; fail closed on a marker or a scan error of an existing home.
  - `crates/ade_node/tests/rehearsal_gate_archived_home.rs` (NEW test, RED) — clean-tree green + archived-home-smuggle fails, Drop-guarded cleanup.
- **State machines / persistence / network:** none.
- **Out of scope (explicit, your scoping):** the `toml_escape` control-char hardening (deferred follow-up — it touches the serializer); the BA-02 bounty gate's same stale active-home glob (separate G-C-gate follow-up); the `pub`-field sole-constructor hardening (inherited from G-C, project-wide). No RO-LIVE flip.

## 6. Execution Boundary
- **BLUE (none — unchanged):** no BLUE crate touched.
- **GREEN (unchanged):** `rehearsal_evidence` / `ba02_evidence` byte-unchanged.
- **RED:** `ci/ci_check_rehearsal_manifest_schema.sh` (the gate) + the new regression test (shells the gate via `bash`, Drop-guarded fixture).
- **Color resolved:** RED gate + RED test only.

## 7. Invariants Preserved
- `CN-REHEARSAL-FIDELITY-01` clause (1) — S1 fence (`ci_check_node_path_fidelity.sh`) byte-unchanged + green.
- `CN-OPERATOR-EVIDENCE-01` / bounty BA-02 gate (`ci_check_ba02_evidence_manifest_schema.sh`) — **byte-unchanged** (S4 does not touch it; its stale-glob is a separate follow-up).
- `RO-LIVE-01` / `RO-LIVE-06` — not flipped.
- `DC-NODE-06` / `CN-NODE-02` / `DC-LIVEMEM-01` — containment / handoff / memory fences byte-unchanged.
- `rehearsal_evidence` / `rehearsal_pass` — byte-unchanged (no serializer change).

## 8. Invariants Strengthened or Introduced
- **`CN-REHEARSAL-FIDELITY-01` — clause (2) barrier (b) now actually enforced.** S4 makes the leak cross-check functional (scans active + archived G-C homes, fail-closed on a marker or a scan error of an existing home, no whole-check skip) and adds the durable boundary negative test. This closes the gap that blocked the `declared → enforced` flip; recorded in `evidence_notes`. Status stays `declared` until `/cluster-close`.

> Single invariant family: the non-promotability leak barrier. S4 makes a promised-but-dead guard real.

## 9. Design Summary
- **Gate fix.** Enumerate candidate homes `docs/clusters/PHASE4-N-F-G-C` + `docs/clusters/completed/PHASE4-N-F-G-C`. **Build the list of EXISTING homes first** — the `[[ -d ]]` test filters the candidate list; it does **not** gate the whole check. Then:
  - If **no** home exists → the leak set is empty (deliberate: "home absent" = "no files there", never "skip").
  - For the existing homes, run `grep -rlE '^(is_rehearsal|not_bounty_evidence)[[:space:]]*=' "${EXISTING[@]}" --include='*.toml'` and **branch on grep's exit code** (captured without `set -e` aborting, and without `2>/dev/null || true` masking): **0 = match → leak → FAIL**; **1 = no match → clean**; **≥2 = scan error on an existing home → FAIL CLOSED**. The implementation must distinguish "home absent" from "grep failed on an existing home": absence yields an empty leak set, but a scan error on an existing directory must fail closed (not be silently swallowed). Any marker found → FAIL with the offending path(s).
- **Regression test** (`rehearsal_gate_archived_home.rs`): resolve repo root via `CARGO_MANIFEST_DIR/../..`; run `bash <gate>` on the clean tree → assert exit 0; create `docs/clusters/completed/PHASE4-N-F-G-C/CE-G-C-LIVE_s4negtest_<pid>.toml` with `is_rehearsal = true` (Drop-guarded) → run the gate → assert non-zero exit → the guard removes the fixture (clean even on panic).

## 10. Changes Introduced
### Types / State Transitions / Persistence
- None.
### Gates / Tests
- `ci_check_rehearsal_manifest_schema.sh` leak-scan repointed (active + archived) + de-skipped + error-distinguishing. New regression test file. Registry `evidence_notes` S4 record.

## 11. Replay, Crash, and Epoch Validation
- n/a (CI gate + test; no authoritative state).

## 12. Mechanical Acceptance Criteria
- [ ] `ci_check_rehearsal_manifest_schema.sh` — **green on the clean tree** (both homes scanned where they exist; no marker present).
- [ ] `ci_check_rehearsal_manifest_schema.sh` — **fails** when a rehearsal marker is placed under `docs/clusters/completed/PHASE4-N-F-G-C/` (re-run the exact smuggle that passed before S4 → now exit 1).
- [ ] `rehearsal_gate_archived_home.rs` — green (clean-tree-pass + archived-home-smuggle-fails, fixture cleaned).
- [ ] `ci_check_node_path_fidelity.sh` (S1) — **byte-unchanged + green**.
- [ ] `ci_check_ba02_evidence_manifest_schema.sh` (bounty) — **byte-unchanged** (not touched by S4).
- [ ] `ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh` + `ci_check_live_feed_memory_bounds.sh` — byte-unchanged + green.
- [ ] `cargo test -p ade_node` green.

## 13. Failure Modes
- A rehearsal marker committed under **either** G-C home (active or archived) → gate **fails closed** (no whole-check skip).
- A **scan error on an existing home** (grep exit ≥2 — e.g. unreadable) → gate **fails closed** (not swallowed).
- A **missing home** → contributes no files (deliberate); never disables the scan of the other home(s).

## 14. Hard Prohibitions
### Inherited Cluster-Level Prohibitions
All PHASE4-N-F-G-D prohibitions apply (no private-only shortcut; no containment/memory relaxation; no synthetic manifest; no RO-LIVE flip; no new BLUE authority/canonical type/`--mode node` flag/from-genesis constructor).
### Slice-Specific Prohibitions
- **No whole-check skip** — "home absent" means "no files there," never "skip the leak check"; a scan error on an existing home fails closed.
- **No change to the BA-02 bounty gate** (separate follow-up) — it stays byte-unchanged.
- **No change to `rehearsal_evidence.rs` / `rehearsal_pass.rs`** (no `toml_escape` fold-in — deferred).
- **No RO-LIVE flip; no BLUE change.**

## 15. Explicit Non-Goals
This slice MUST NOT: fix the BA-02 gate's stale glob; harden `toml_escape`; add the `pub(crate)` sole-constructor enforcement; modify any BLUE crate; flip RO-LIVE; commit any real rehearsal manifest.

## 16. Completion Checklist
- [ ] Gate scans active + archived G-C homes (existing ones); no whole-check skip; fail-closed on marker or existing-home scan error.
- [ ] The pre-S4 archived-home smuggle now makes the gate fail (verified).
- [ ] Regression test green (clean-tree-pass + smuggle-fails, fixture cleaned).
- [ ] S1 fence + BA-02 gate + 3 containment/memory fences byte-unchanged.
- [ ] `cargo test -p ade_node` green.
- [ ] `CN-REHEARSAL-FIDELITY-01` `evidence_notes` record S4 (stays `declared`; flip at re-run `/cluster-close`).

## 17. Review Notes
- **Why this blocks the flip:** the cluster's sole security property is non-promotability, asserted as "three independent barriers." One was silently dead after G-C's archival. A rule cannot flip to `enforced` while a promised barrier doesn't run — hence the close halts until S4.
- **Absent vs. errored (the §9 sharpening):** for a security gate, "directory absent" may legitimately mean "no files there," but a *scan failure on an existing directory* must fail closed — `2>/dev/null || true` would mask both, so S4 builds the existing-home list first and branches on grep's exit code.
- **Durable, not one-shot:** the committed regression test is the regression guard whose absence let the stale path ship green; it re-asserts the barrier on every CI run.
- **Scoped tight:** S4 fixes exactly the rehearsal gate's false barrier. The BA-02 gate's identical stale glob is real and should be fixed soon, but it's G-C's gate and a separate follow-up; S4's closure obligation is the rehearsal barrier.
