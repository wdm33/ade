# Invariant Slice S11 — Post-ForkChoiceWin forward-follow continuity

## 2. Slice Header

- **Cluster:** PHASE4-N-AO (live multi-candidate fork-choice SELECT + adopt).
- **Type:** diagnostic / fix slice (NOT evidence polishing). Entry-gated on an
  instrumented diagnostic that must ANSWER the run-1 stall before any fix is scoped.
- **Depends on:** S7 (LCA walk, DC-NODE-38), S9/S10 (closed + continuity evidence,
  DC-EVIDENCE-04/05).
- **Declares:** `DC-NODE-39` (post-ForkChoiceWin forward-follow continuity).
- **Cluster Exit Criteria addressed:** CE-AO-6 — closes the robustness gap blocking the
  `CN-CONS-03` flip. Run 2 (`~/.cardano-ceai6/ao-s10-run2-PASS-conv.jsonl`) is preserved
  positive evidence that the mechanism CAN complete; run 1 is the blocking finding.

## 4. Intent

`CN-CONS-03` is not "Ade can complete convergence once" — it is "Ade RELIABLY performs
live multi-candidate fork-choice convergence under the exercised two-producer
conditions." Run 1 (same binary as the passing run 2) revealed a **conditional
follow-forward hole**: after `fork_switch_applied` adopted cn2@298, the winning peer's
later descendants (≈340, 388) arrived, but Ade received only 388 and **missed the
bridge** (340) — so no descendant admitted, no divergence, **but no convergence**. This
is fail-closed (good) but a stall is not convergence (the proof is not robust enough to
mark `CN-CONS-03` enforced).

S11 makes the post-switch forward-follow either **complete** or **fail closed with a
structured reason** — never a silent skip-and-stall.

## 5. Scope

**Phase 1 — instrumented diagnostic (ENTRY GATE; must precede any fix).** Answer ONE
question from a live two-producer run that reproduces the stall:
> *Did the winning peer fail to serve the bridge block, or did Ade receive it and
> drop / misclassify / skip it?*
Instrument (temporary markers, reverted after the answer; promoted to a closed event
only if the fix needs durable observability):
- **wire-pump RollForward from the winning peer** (`crates/ade_runtime/src/admission/wire_pump.rs:508`):
  `peer`, `slot`, `hash`, `prev_hash`, `block_no` of every served header.
- **`classify_receive`** (`crates/ade_node/src/node_sync.rs:937`): `durable_tip`
  {slot,hash,block_no}, incoming `prev_hash`, decision (`LinearExtend` / `Competing` /
  `AlreadyHave`).
- **`walk_to_durable_lca`** (`crates/ade_node/src/lca_walk.rs`): anchor-found / branch-gap
  / missing-intermediate.
- **post-switch follow state**: winning peer, adopted tip X, next received block from the
  winning peer, whether `prev_hash == X.hash`.

**Phase 2 — fix, scoped ONLY to the confirmed cause.** Check these surfaces first
(do not pre-pick — Phase 1 decides):
1. After `apply_fork_switch`, does the ChainSync intersect / follow point update to the
   adopted winner tip X? (Run-1 candidate: the wire pump intersects once at startup and
   is never re-pointed — but run 1 PROVED descendants ARE received, so a pure re-intersect
   gap is already partly refuted; Phase 1 must reconcile.)
2. Does the per-peer S7 branch cache retain the post-switch winner-branch headers needed
   to bridge descendants?
3. Do S4/S6 consume the fetched branch bodies without teaching the follow stream that Ade
   now holds X?
4. Does `classify_receive` treat descendant 340 as `Competing` because the durable tip is
   projected incorrectly (block_no / hash) right after the switch?
5. Does a later 388 arrive before 340 (out-of-order) and poison the candidate cache /
   produce a `BranchGap` the follow never recovers from?

**Phase 3 — the invariant becomes mechanical** (the fix + the fail-closed reason).

## 6. Execution Boundary (TCB color)

- **RED (capture + the wire-follow shell):** `wire_pump.rs` RollForward path — the likely
  fix locus if Phase 1 finds a skip/race/re-intersect gap. Diagnostic markers are RED.
- **GREEN:** `classify_receive` (pure classifier) + any new closed `MissingBridge`
  evidence/reason discriminant + the S7 cache interaction. Diagnostic markers here are
  GREEN.
- **BLUE (fail-closed authority decision):** the post-switch admit path must, on a proven
  missing bridge, **HOLD / fail closed with a structured reason** rather than silently
  stall — this is the authority behavior `DC-NODE-39` strengthens. No change to
  `select_best_chain` / `apply_fork_switch` adoption authority.

## 7. Invariants Preserved (registry IDs)

- `DC-NODE-24/25/26/28/29` (receive routing, rollback discipline, reconcile), `DC-NODE-38`
  (LCA walk), `DC-CONS-03` (single-best-peer follow path), `CN-CONS-01` (fail-closed),
  `DC-EVIDENCE-04/05` (the continuity evidence the fix is verified against). The fix must
  not weaken fail-closed behavior — it REPLACES a silent stall with a STRUCTURED one.

## 8. Invariants Strengthened / Introduced

- **`DC-NODE-39` (introduced, declared).** *After a `ForkChoiceWin` adoption at tip X,
  Ade must continue receiving and admitting the winning peer's descendants in parent-link
  order, OR fail closed with a structured missing-bridge reason; it must NOT silently skip
  a required bridge block and stall behind the winning branch.* Replayable: given the same
  post-switch served sequence, Ade derives the same admit / `MissingBridge` outcome.

## 9. Design Summary (provisional — Phase 1 confirms)

The hypothesis to confirm or refute: a correct sequential follow of the winning peer
should never skip the bridge (340) and still deliver 388, unless there is a
stream / re-intersect / cache race, OR the peer served a non-linear event Ade mis-handled
as a gap. Phase 1's instrumentation distinguishes peer-fault (the bridge was never served)
from Ade-fault (served-then-dropped / misclassified / out-of-order-poisoned). The fix then
either (a) ensures the winner's descendants are served + admitted in parent-link order, or
(b) on a genuinely missing bridge, emits a closed `MissingBridge` reason and fails closed
(holds the fence) — never a silent stall.

## 11. Replay / Crash / Epoch Validation

- Hermetic replay: same post-switch served sequence ⇒ same admit/`MissingBridge` outcome,
  byte-identical.

## 12. Mechanical Acceptance Criteria

- [ ] **Phase 1 diagnostic ANSWERED** (recorded in the gap doc): peer-fault vs Ade-fault,
  with the exact served `prev_hash`/`block_no` sequence and the classify/walk decisions for
  cn2's post-switch blocks. No fix is implemented before this is recorded.
- [ ] **Hermetic — happy path:** `ForkChoiceWin` adopts X; the winning peer later provides
  X+1 (`prev_hash == X.hash`); Ade admits X+1 as `LinearExtend` / `ChainSelected`. Test
  `post_switch_admits_winner_descendant_x_plus_1`.
- [ ] **Hermetic — fail-closed gap:** the winning peer provides X+2 WITHOUT X+1; Ade emits
  a structured `MissingBridge` (closed reason) and holds fence / fails closed — **no silent
  stall**. Test `post_switch_missing_bridge_fails_closed_not_silent`.
- [ ] New gate or extension asserting `DC-NODE-39` (the no-silent-skip predicate).
- [ ] `cargo test -p ade_node` (+ `-p ade_runtime` if the wire pump changes) green; all
  prior AO gates (lca / fairness / evidence-closed / post-switch-window) unregressed.
- [ ] **Live (CE-AO-6 flip):** a fresh two-producer transcript PASSES
  `ci/ci_check_post_switch_convergence_window.sh` — `fork_switch_applied` X,
  `block_admitted` X, ≥1 admitted descendant of X (or agreed-at-descendant), no diverged,
  all wins terminal — AND the run-1 stall no longer reproduces (or is proven venue-caused).
  ONLY then does `CN-CONS-03` flip.

## 14. Hard Prohibitions

- Do **not** weaken fail-closed behavior — a missing bridge must produce a STRUCTURED
  reason + fence hold, never a silent stall and never a forced/guessed admit.
- Do **not** admit a descendant whose parent link Ade has not validated (no peer-claimed
  bridging; the parent must be in Ade's validated store / proven branch).
- Do **not** scope the fix before Phase 1 answers peer-fault vs Ade-fault (the S7/S8
  wrong-layer lesson).
- No change to `select_best_chain` / `apply_fork_switch` adoption authority.

## 15. Explicit Non-Goals

- Not a re-litigation of S10's gate (the gate is correct — run 2 passes it). S11 makes the
  node's behavior robust so a passing transcript is reliably achievable, not lucky.

## 16. Completion Checklist

- [ ] Phase 1 diagnostic run + recorded answer (peer-fault vs Ade-fault).
- [ ] Phase 2 fix scoped to the confirmed cause; structured `MissingBridge` reason if a
  genuine gap is possible.
- [ ] Hermetic happy-path + fail-closed-gap tests; `DC-NODE-39` gate.
- [ ] All AO gates + `cargo test` green.
- [ ] Live re-run passes the post-switch window; run-1 stall explained/fixed.
- [ ] `DC-NODE-39` declared → enforced at `/cluster-close`; `CN-CONS-03` flip unblocked.
