# SLICE LV-17 — certified accumulator recovery across rollback

**Status:** OPEN (opening record). No design is chosen and no code is changed. LV-16 stays as it is while
its evidence run completes; this slice does not touch intersection candidate selection.

**Entry state:** `352bf783` (LV-16, local), store semantics 13. A true-tier replay and recovery concern
with a large operational cost: on the preprod store a needless reset costs a 10-hour accumulator rebuild
(epochs 304 to 323).

---

## 0. Findings — settled facts, do not re-derive

| fact | evidence |
|---|---|
| The uninterrupted v13 run handled its rollbacks without rebuilding | `lv15b0-step6-bootstrap.log`: three times `settled rewind RE-CERTIFIED at 137894414/5362788 after rollback -- next pass forward-folds, no bootstrap refold (DC-EPOCH-35)` |
| The killed master store holds a settled triple at that same point, with the anchor absent | read on a copy (`lv17_report_anchor_and_settled_point`): `cursor 137894414; anchor absent; settled 137894414/5362788/1620afdc`. The kill landed after `reset_to_settled` (the S5 pre-clear) and before the ChainDb rollback committed. |
| Recovery from that store discarded the settled triple and rebuilt from the bootstrap baseline | recovery A: `recovery_admit action=reset_and_refold reason=anchor_absent`; `recover_admit` maps an absent anchor to `ResetAndRefold`, which calls `reset_to_bootstrap` (`node_lifecycle.rs` `RecoveryAction::ResetAndRefold`), and `reset_to_bootstrap` deletes the settled triple and its fingerprint (`epoch_accumulator_store.rs`, DC-EPOCH-29) |
| That settled point would pass every RF-1 proof against the recovered chain | it is on the durable chain at its slot (the rollback target 138046915 is above it, and blocks and WAL agree from 137894414 with the master), and it is 6,838 blocks behind the durable tip 138046917 (k = 2160) |
| After the rebuild reached the tip, no settled triple existed, so the queued rollback rebuilt again | recovery A: `rollback_admit action=reset_to_bootstrap ... anchor_before=138046917/5369626/f8c86fec ... anchor_after=absent`; copies taken during the second rebuild: `settled absent` (at 130350402 and at 136993436) |
| Why no settled triple existed | `roll_settled_rewind_point` runs only when a pass reaches the tip (`AccumulatorChaindbOutcome::ReachedTip`). It stages the current point as pending and promotes it only once the tip is k blocks past it. A rebuild walks 19 epochs of points that are already far more than k behind the tip and stages none of them. So for the first k blocks after any rebuild, any rollback falls back to the bootstrap baseline. |
| Progress inside a rebuild is durable | the restart after the second kill: `recovery_admit action=forward_fold reason=forward_fold_no_reset anchor_before=136993436/5322209/ec1a18bd`; it resumed at epoch 320 instead of 304 |

DC-EPOCH-35 states the first behaviour as a decision: "a crash in that window still refolds from
canonical". That was the safe choice when RF-1 landed. It is now measured as the dominant recovery cost,
and the second cause compounds it.

---

## 1. Invariant (proposed, true tier)

> A valid durable rollback, or a restart inside a rollback window, must not invalidate an independently
> certified accumulator state that is still provably on the durable chain and still k-settled. Recovery
> must be replay-equivalent whether it starts from such a certified state or from the bootstrap baseline.

Replay-equivalent means: the accumulator, governance state, delegation state, reduced checkpoint and
the leadership tables reached at any canonical cursor are byte-identical on both paths.

---

## 2. Proof obligations (before any design is chosen)

1. **Identity.** Establish mechanically what the settled triple certifies: the blob, the point (slot,
   block number, header hash), the leadership table, the CE-RF-6 fingerprint, and the lineage and schema
   context the fingerprint binds. Recognising the point is not enough; the dependency chain must be
   checked.
2. **Still valid after a restart.** Show which proofs RF-1 runs after a committed rollback (on the chain
   at its slot by header hash, k blocks behind the tip, fingerprint, cursor) and whether the same proofs
   are sound against the durable chain a restart sees, including a restart in the window between the
   pre-clear and the rollback commit. The S5 order must hold: the anchor is never carried across an
   uncommitted rollback.
3. **No new authority path.** Any reuse must go through the existing settled-triple restore and
   re-certification (DC-EPOCH-34, DC-EPOCH-35), not a second route to a certified anchor.
4. **Equivalence.** Show by a reference execution that the state reached from the reused triple equals
   the state reached by the bootstrap rebuild at the same canonical cursors. The preprod captures already
   give a reference: recovery A's rebuild reproduced the uninterrupted run's pass-end fingerprints, and
   the master's accumulator at 137894414 is byte-identical (10,149,025 bytes) to the uninterrupted run's
   e323 capture.
5. **The rebuild gap.** Decide whether a rebuild may stage or promote a settled point from cursors it
   passes that are already k blocks behind the tip, and if so prove the staged state is the one the
   live path would have staged at that point.
6. **Fallback unchanged.** Any failed proof falls back to the bootstrap rebuild, as today.

If obligation 2 cannot be met, reconstruction remains necessary, and this slice reduces to documenting
why.

---

## 3. Not in this slice

- LV-16's intersection candidates, its evidence run, or its closure.
- The shutdown non-response (a separate defect with its own gate).
- Memory use during a rebuild (the earlyoom kills are explained by host memory; they are not this
  defect, though this defect multiplies the exposure).
- Any change to store semantics, unless the persisted interpretation changes.
