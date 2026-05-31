# PHASE4-N-F-F — Slice S5: single-bootstrap gate precision (ReceiveState owner allow-list)

> **Status:** slice doc (IDD Part IV). Close-surfaced remediation (per-cluster
> security review Finding 1). Companion to the cluster-slice-plan. Code-verified
> against HEAD `58acca1` (S4 merged) at authoring.

> **Slice S5 in one line:** fix the stale `ReceiveState::new` guard in
> `ci_check_node_binary_uses_single_bootstrap.sh` so it expresses the real
> CN-NODE-01 invariant — `ReceiveState::new` is confined to the lifecycle-owner
> files (`node.rs` + `node_lifecycle.rs`), with zero tolerance elsewhere —
> instead of the impossible "≤1 per crate", restoring the gate to green.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-F (operator-key ingress → forge-on flip).
- **Slice:** S5 — single-bootstrap gate precision. Close-surfaced remediation
  (the per-cluster security review's Finding 1).
- **File:** `ci/ci_check_node_binary_uses_single_bootstrap.sh` (CI gate only).

## 2. Why this slice exists (provenance)
The per-cluster security review found `ci_check_node_binary_uses_single_bootstrap.sh`
failing at HEAD (`found 3` `ReceiveState::new`). Investigation:
- The gate's `ReceiveState::new` guard asserts `≤1 per crate`. Its comment ("the
  single legit caller is `node.rs`'s run loop") predates PHASE4-N-F-C/N-F-D, which
  added the `--mode node` lifecycle owner `node_lifecycle.rs` — itself a legitimate
  `ReceiveState` constructor.
- **It was already red at baseline `e606ed6`** (`found 2`: `node.rs` 1 +
  `node_lifecycle.rs` 1) and has been red since N-F-C/N-F-D. With **two** legitimate
  owner files the `≤1 per crate` rule is **impossible to satisfy** — a code-only fix
  cannot make it green.
- N-F-F S3's `--mode node` `On` arm legitimately added a 3rd occurrence (the
  mutually-exclusive `ForgeIntent::On` branch of the single `run_node_lifecycle_inner`
  dispatcher), making it `found 3`.
- The real CN-NODE-01 invariant (single bootstrap authority; no second bootstrap /
  rogue state) **holds** — independently verified by `ci_check_node_run_loop_containment.sh`
  (green) and by both close reviews. The count heuristic is the stale part.

## 3. Intent (invariant impact)
Restore the named CN-NODE-01 gate to a green state that **correctly** expresses the
real invariant. `ReceiveState::new` is the recovered/bootstrapped-state entry into
the relay spine; it is legitimate ONLY in the lifecycle-owner files (`node.rs`'s run
loop + `node_lifecycle.rs`'s mutually-exclusive `--mode node` arms). Any occurrence
in any other `ade_node` production file would be a synthetic/rogue bypass of the
recovered state. The new check is a **net tightening** for non-owner files (zero
tolerance, an explicit allow-list) while correctly tolerating the owner's
per-arm construction.

## 4. Pre-conditions
- S1–S4 merged (`58acca1`).
- `ci_check_node_binary_uses_single_bootstrap.sh`'s `bootstrap_initial_state`
  checks (`≤1 per file`, `≥1 per crate`) pass at HEAD and are unchanged by S5.
- `ci_check_node_run_loop_containment.sh` (green) independently asserts the
  loop body adds no second bootstrap / no manual tip path.

## 5. Implementation boundary
- Replace the `ReceiveState::new` `≤1 per crate` count with an **owner allow-list**:
  `ReceiveState::new` may appear only in `node.rs` and `node_lifecycle.rs`
  (the lifecycle owners); any occurrence in any other `ade_node/src/**.rs`
  production body fails closed. No per-count cap on the owner files (the owner may
  construct one per mutually-exclusive `--mode node` arm).
- The `bootstrap_initial_state` per-file `≤1` + `≥1` per-crate checks are
  **unchanged** (double-bootstrap-within-a-path protection retained).
- `#[cfg(test)]` bodies remain stripped before counting (unchanged).
- **No production code change.** S5 touches only the CI gate.

## 6. TCB color
- CI gate only (neither BLUE/GREEN/RED source). No crate modified.

## 7. Invariants preserved (must not weaken) — by registry ID
- **CN-NODE-01** — the single-bootstrap authority invariant is preserved and the
  gate now expresses it correctly; `bootstrap_initial_state` checks unchanged.
- **CN-NODE-02 / CE-F-6** — `ci_check_node_run_loop_containment.sh` /
  `ci_check_loop_planner_closed.sh` unchanged (still green); the run-loop body is
  untouched.
- All other gates + all `ade_node` tests — unchanged (S5 is CI-only).

## 8. Invariants strengthened (one family: CN-NODE-01 enforcement precision)
- The `ReceiveState::new` anti-bypass guard becomes an explicit owner allow-list:
  **zero tolerance** for `ReceiveState::new` outside `{node.rs, node_lifecycle.rs}`
  (previously a single rogue occurrence elsewhere could pass under the `≤1 total`
  count). A net tightening for non-owner files; the gate is restored to green.

## 9. Replay / determinism obligations
- None — CI gate only; no authoritative state, no test surface, no replay impact.

## 10. Mechanical acceptance criteria
- [ ] `ci/ci_check_node_binary_uses_single_bootstrap.sh` exits 0 at HEAD.
- [ ] The gate still FAILS if `ReceiveState::new` appears in a non-owner `ade_node`
      production file (verified by a transient injected occurrence in, e.g.,
      `produce_mode.rs`, then reverted) — the anti-bypass is not vacuous.
- [ ] The `bootstrap_initial_state` `≤1 per file` / `≥1 per crate` checks are
      byte-unchanged (diff shows only the `ReceiveState::new` guard block changed).
- [ ] `ci_check_node_run_loop_containment.sh`, `ci_check_loop_planner_closed.sh`,
      `ci_check_forge_intent_closed.sh`, `ci_check_operator_forge_no_secret_leak.sh`,
      `ci_check_private_key_custody.sh` all still pass.
- [ ] `cargo test -p ade_node` still green (no production change).

## 11. Forbidden in this slice
- No production (`crates/**`) change — S5 is CI-only.
- No weakening of the `bootstrap_initial_state` checks or any other gate.
- No relaxation of the N-F-E forge-containment gate (CE-F-6).

## 12. Slice completion checklist
- [ ] Gate edited; exits 0 at HEAD; anti-bypass re-verified (inject/revert);
      `bootstrap_initial_state` checks unchanged.
- [ ] All N-F-F gates + `cargo test -p ade_node` green.
- [ ] Slice doc committed standalone (`docs:`) before impl; impl (`fix(ci):`) after green.

## Authority
Registry IDs `CN-NODE-01` (gate precision; enforcement strengthened), `CN-NODE-02`
/ CE-F-6 (preserved). The cluster-slice-plan + invariant registry are authoritative.
