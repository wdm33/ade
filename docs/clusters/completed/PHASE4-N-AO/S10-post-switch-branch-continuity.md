# Invariant Slice S10 — Post-switch branch-continuity evidence for CE-AO-6

## 2. Slice Header

- **Cluster:** PHASE4-N-AO (live multi-candidate fork-choice SELECT + adopt).
- **Depends on:** S7 (LCA walk, DC-NODE-38), S9 (closed fork-choice evidence, DC-EVIDENCE-04).
- **Declares:** `DC-EVIDENCE-05` (replayable post-switch branch-continuity verdict).
- **Refines:** `DC-EVIDENCE-04` (still `declared`, never enforced) — the CE-AO-6 live
  terminal moves from exact-tip-only to branch-continuity + (agreed OR validated-prefix).
- **Cluster Exit Criteria addressed:** CE-AO-6 (live multi-producer convergence; flips
  `CN-CONS-03`) — this slice makes the flip gate truthful and timing-robust.

## 4. Intent

Replace the flaky exact-tip terminal of CE-AO-6 with a **replayable branch-continuity
property**. After a `ForkChoiceWin` adoption at tip X, prove — from Ade's **own
validated admitted-block lineage**, not peer claims — that every post-switch admitted
block is a descendant of X, nothing diverged, and every fork-choice win has a terminal.
Then accept as the convergence terminal **either** exact agreement **or** a validated
catching-up prefix (Ade continuous from X, peer observed ahead). "Peer is ahead" is a
RED observed comparison and never an input to the continuity verdict or to consensus.

Motivation (empirical): a clean, healthy follow run produces **only `lagging`** verdicts
(the solo producer never pauses for the follower to touch its exact tip), zero `agreed`,
zero `diverged`. Exact-tip agreement is a test-harness artifact (a production "lull"),
not a node guarantee. Branch continuity is the real correctness claim: Ade stayed on the
selected valid chain and did not diverge while catching up.

## 5. Scope

1. **Evidence fidelity (RED capture).** Add `prev_hash` — the admitted block's *validated*
   `decoded.prev_hash` (the same canonical parent field S7's parent-walk consumes via
   `get_block_by_hash`, never peer-supplied) — to `PumpTip` and to the fork-switch-adopt
   tip, and thread it through `emit_admit_and_verdict` → `emit_block_admitted` → the
   `BlockAdmitted` event → the writer as a new `prev_hash_hex` field.
2. **GREEN replayable reducer.** New `crates/ade_node/src/post_switch_continuity.rs`:
   a pure, total, deterministic `derive_post_switch_continuity(events) ->
   PostSwitchContinuity` over the closed transcript vocabulary.
3. **Live gate refinement.** `ci/ci_check_post_switch_convergence_window.sh` invokes the
   one Rust reducer (a thin `post_switch_continuity` bin) so the live gate and the replay
   test share a single implementation.

LATENT-free: the `prev_hash_hex` field is emitted the moment this slice merges; the
reducer is exercised by both the hermetic replay test and the live gate.

## 6. Execution Boundary (TCB color)

- **BLUE — UNCHANGED.** `select_best_chain`, `walk_to_durable_lca`, `apply_fork_switch`,
  `pump_block`, header/body validation. No authority surface is touched, read, or fed by
  this slice.
- **RED (capture fidelity).** `crates/ade_runtime/src/forward_sync/pump.rs` (`PumpTip` +
  the value populated from the just-applied envelope), `crates/ade_node/src/node_lifecycle.rs`
  emit sites (follow admit @ `emit_participant_admit`, fork-switch-adopt admit), the
  `BlockAdmitted` event + writer serialization. Capture only; never read back.
- **GREEN (new, replayable).** `crates/ade_node/src/post_switch_continuity.rs` — the
  pure reducer + closed `PostSwitchContinuity` verdict. No clock / rand / HashMap / float /
  I/O. The CI gate's thin bin is RED glue around this GREEN core.

## 7. Invariants Preserved (registry IDs)

- `DC-NODE-30` (GREEN block_admitted + agreement_verdict side-output) — extended with one
  bounded field; the verdict reducer is untouched.
- `DC-EVIDENCE-01/02/03/04` — closed vocabulary, observe-only containment, no free-form
  strings, write-failure-cannot-alter-authority. The new field + verdict are observe-only.
- `CN-CONS-03` — fork-choice authority unchanged; this slice only audits its outputs.
- `DC-NODE-34` (peer attribution), `DC-NODE-38` (LCA walk), `CN-CONS-01` (fail-closed).

## 8. Invariants Strengthened / Introduced

- **`DC-EVIDENCE-05` (introduced, declared).** *Given the same post-switch admitted-block
  bytes and the same applied fork-switch point, `derive_post_switch_continuity` yields a
  byte-identical `PostSwitchContinuity`.* `ContinuesSelectedBranch` requires: unbroken
  `prev_hash` lineage from X across every post-X `block_admitted`; no `diverged` after X;
  every `fork_choice_selected{win}` paired to a terminal. The peer's tip is **never** an
  input to this verdict — Ade's own admitted blocks only.
- **`DC-EVIDENCE-04` (refined, declared).** The CE-AO-6 live terminal becomes
  branch-continuity + (agreed at X-or-descendant OR validated-prefix-of-peer). Pre-
  enforcement refinement of an acceptance rule; net gate is stronger on lineage (every
  post-X block must chain — the old terminal checked nothing between X and the agreed
  event) and honest about the normal catching-up state.

Closed verdict (no free-form strings):

```
PostSwitchContinuity::ContinuesSelectedBranch { admitted_after_switch, tip_slot, tip_hash }
                    ::Diverged { slot }
                    ::BrokenLineage { at_slot, expected_prev, found_prev }
                    ::DanglingForkChoiceWin { fork_switch_id }
                    ::InsufficientEvidence { reason }   // no applied / no post-X admit / incomplete switch proof
```

## 9. Design Summary

- `derive_post_switch_continuity` finds the `fork_switch_applied{rollback_reason=
  ForkChoiceWin}` event X; requires X's `fork_switch_id` to carry the full hard proof
  (selected{win} → branch_fetch_completed → branch_prevalidated → applied); then walks
  post-X `block_admitted` in order, asserting `admitted[i].prev_hash == admitted[i-1].hash`
  (first post-switch follow block must chain to X.hash). Any `diverged` after X →
  `Diverged`; a broken parent link → `BrokenLineage`; a `win` with no terminal →
  `DanglingForkChoiceWin`; missing prerequisites → `InsufficientEvidence`; else
  `ContinuesSelectedBranch`.
- The reducer depends ONLY on Ade's own admitted-block fields (slot/hash/prev_hash),
  the fork-switch events, and `diverged` — all locally derived. It does NOT read peer tips.
- **Release decision (the gate):** pass iff `ContinuesSelectedBranch` AND, within the
  bounded window (`max_slots=200`, `max_admitted_blocks=20`, fixed up front), the terminal
  is `agreement_verdict{agreed, our==peer}` at X-or-descendant **OR** a validated prefix
  (continuity holds AND an in-window `lagging` with `peer_slot > our_slot`). The peer-ahead
  comparison is RED observed evidence — never consensus.

## 10. Changes Introduced

- `crates/ade_runtime/src/forward_sync/pump.rs`: `PumpTip.prev_hash` populated from the
  applied envelope's validated header.
- `crates/ade_node/src/node_lifecycle.rs`: thread `prev_hash` at both admit emit sites.
- `crates/ade_node/src/convergence_evidence.rs` + `admission_log/{event,writer}.rs`:
  `prev_hash_hex` on `block_admitted`.
- `crates/ade_node/src/post_switch_continuity.rs`: the GREEN reducer + closed verdict.
- `crates/ade_node/src/bin/post_switch_continuity.rs`: thin transcript-to-verdict bin.
- `ci/ci_check_post_switch_convergence_window.sh`: invoke the bin (one implementation).

## 11. Replay / Crash / Epoch Validation

- `post_switch_continuity_replays_byte_identical` — a committed hermetic fixture transcript
  → byte-identical `PostSwitchContinuity` on repeated derivation (the replay invariant).
- The live CI gate runs the **same** reducer (via the bin) on the live transcript, so the
  replay-tested authority and the live acceptance cannot drift.

## 12. Mechanical Acceptance Criteria

- [ ] `block_admitted_carries_prev_hash` — the `BlockAdmitted` event + writer serialize a
  `prev_hash_hex` sourced from the admitted block's validated header (a field-presence +
  provenance test; `prev_hash` is `decoded.prev_hash`, never peer-supplied).
- [ ] `continuity_ok_yields_continues_selected_branch` — a linked post-X admitted sequence
  → `ContinuesSelectedBranch`.
- [ ] `broken_parent_link_yields_broken_lineage` — an injected non-chaining `prev_hash` →
  `BrokenLineage{at_slot,...}`.
- [ ] `post_switch_diverged_yields_diverged` — a `diverged` after X → `Diverged`.
- [ ] `win_without_terminal_yields_dangling` — a `fork_choice_selected{win}` with no
  terminal → `DanglingForkChoiceWin`.
- [ ] `continuity_verdict_ignores_peer_tip` — peer-tip fields permuted in the transcript do
  not change the verdict (proves the reducer reads only Ade's own admitted lineage).
- [ ] `post_switch_continuity_replays_byte_identical` — replay invariant (DC-EVIDENCE-05).
- [ ] Gate **`ci/ci_check_post_switch_convergence_window.sh`** (refined): hard fork-switch
  proof unchanged → derives `PostSwitchContinuity` via the bin → passes iff
  `ContinuesSelectedBranch` AND in-window terminal is agreed-at-X-or-descendant OR
  validated-prefix; no diverged.
- [ ] `cargo test -p ade_node` green; `cargo test -p ade_runtime` green.
- [ ] **Live (CE-AO-6 flip):** a committed two-producer transcript PASSES the refined gate:
  the hard fork-switch proof (`fork_choice_selected{win}` → `branch_fetch_completed` →
  `branch_prevalidated` → `fork_switch_applied{ForkChoiceWin}` at X → `block_admitted X`,
  both peers delivered) then `PostSwitchContinuity::ContinuesSelectedBranch` within the
  bounded window with the terminal = agreed-at-X-or-descendant OR validated-prefix-of-peer,
  0 diverged. ONLY then does `CN-CONS-03` flip.

## 13. Failure Modes

- A post-switch admitted block whose `prev_hash` does not chain → `BrokenLineage` (the gate
  fails — a real rollback-below-X / branch-jump would surface here).
- A `diverged` anywhere after X → `Diverged` (hard fail).
- A win with no terminal → `DanglingForkChoiceWin` (the S9 supersession discipline must
  hold; the gate fails otherwise).
- Too few post-X admits / no applied → `InsufficientEvidence` (the gate fails closed; a
  no-switch follow run never passes).

## 14. Hard Prohibitions

- Do **not** loosen the hard fork-switch proof — `fork_choice_selected{win}` →
  `branch_fetch_completed` → `branch_prevalidated` → `fork_switch_applied{ForkChoiceWin}`
  → `block_admitted X` stay required.
- Do **not** accept "eventual agreement somewhere" — the terminal must be inside the
  bounded window and predicated on `ContinuesSelectedBranch`.
- Do **not** make "prefix of peer" peer-claimed. Descendancy is derived from local
  validated `prev_hash` lineage; the peer tip enters only as an observed comparison point.
- The peer tip must **never** become BLUE authority or an input to the continuity verdict.
- No new BLUE; no change to selection / apply / validation.

## 15. Explicit Non-Goals

- Not a consensus change. The node does not use `PostSwitchContinuity` to select chains;
  CI and `/cluster-close` use it to enforce the release gate.
- Not a cryptographic ancestor proof of the peer's full chain (the transcript cannot carry
  the peer's intermediate blocks). "Validated prefix" means Ade's own chain is a continuous
  descendant of X with no contradicting evidence and the peer observed ahead.

## 16. Completion Checklist

- [ ] `prev_hash_hex` on `block_admitted` (event + writer + both emit sites), sourced from
  the validated header.
- [ ] `PostSwitchContinuity` closed verdict + `derive_post_switch_continuity` GREEN reducer.
- [ ] Unit tests (continuity-OK / broken-lineage / diverged / dangling / ignores-peer-tip)
  + the byte-identical replay test.
- [ ] `post_switch_continuity` bin + `ci_check_post_switch_convergence_window.sh` invoking
  it; one implementation.
- [ ] `cargo test -p ade_node` + `-p ade_runtime` green.
- [ ] `DC-EVIDENCE-05` declared; `DC-EVIDENCE-04` terminal refined; ready to flip at
  `/cluster-close` once a live transcript passes the refined gate.
