# CE-4A.3 — restart + rollback replay-equivalence INSIDE the production-loop harness

> **Status: OPEN (scoped, doc-before-impl).** The restart/rollback layer of CE-4A, proven through the
> REAL production composition (`run_relay_loop_with_sched`), not the `co_advance` differential harness
> S5 used. Builds on CE-4A.1 (`9c6fc3c4`, continuous self-sufficiency) and CE-4A.2 (`af3dc9c7`, boundary
> outputs byte-match cardano). **This slice is where the `EpochViewPostPromotionMismatch` surfaced by
> CE-4A.2 gets resolved as a first-class finding (§4) — either proven a harness-only re-entry artifact,
> or fixed as a sealed production slice. It may NOT be worked around.**

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Parent:** `SLICE-CE-4A-CONTINUOUS-SELF-SUFFICIENCY.md` §4.
**Depends on:** CE-4A.1 (`9c6fc3c4`), CE-4A.2 (`af3dc9c7`), S5 (`687fea98`, restart/rollback replay-equivalence
via `co_advance`), the across-boundary warm-start recovery fixes (`d7653561` bridge-recovery twin,
`dabb4210` across-boundary recovery), the k-bounded rollback guard (`48fc423a`).

---

## 1. The claim (exact — non-overclaiming)

> Inside the CE-4A production-loop harness, Ade remains replay-equivalent across warm restart and
> controlled rollback/refold while preserving self-derived boundary authority.

The value: S5 proved restart/rollback replay-equivalence behind `co_advance` (a differential fold, not
the production loop). CE-4A.3 proves it through the SAME production composition CE-4A.1/4A.2 drive — so a
real warm restart mid-run + one within-k rollback/refold produce byte-identical final authority to the
uninterrupted run.

**CE-4A.3 MAY say:**
- restart + rollback replay-equivalence proven through the production composition
- self-derived boundary authority preserved across a genuine warm restart

**CE-4A.3 MAY NOT say:**
- restart equivalence "proven" if the harness avoided `EpochViewPostPromotionMismatch` only by using a
  NON-production setup (see §4 — this is a hard gate)
- literal three-boundary N→N+3 proof complete (CE-4B)
- live preview/preprod operation proven
- bounty-ready continuous operation certified

---

## 2. Scope (the sequence)

One continuous exercise driven through the production loop:

1. Start from the CE-4A fixture (v5 seed, POST-1340 durable tip; the CE-4A.1/4A.2 prep-refold seals the
   native promotion-certified band).
2. Run through the production loop; cross **at least 1340→1341**.
3. **Warm restart from durable state** — a GENUINE restart: drop every store handle + the in-memory
   `ForwardSyncState`, reopen the durable stores from disk, run the production `warm_start_recovery`, and
   reassemble the production authority inputs (the SAME sequence `run_node_lifecycle_inner` performs on a
   real process restart). NOT a reuse of in-memory state (§4).
4. Continue through **1341→1342**.
5. Perform **one controlled rollback/refold inside k** (the `48fc423a` k-bounded, lineage-checked
   rollback guard; a real within-k canonical rollback target, then forward refold via the production path).
6. Re-derive authority.
7. Compare the final evidence bundle against an **uninterrupted** run (CE-4A.1's `drive` to 1342).

---

## 3. Hard asserts (byte-identical: interrupted run == uninterrupted run)

1. **same final selected tip** (durable `ChainDb::tip`).
2. **same accumulator canonical hash** (`blake2b_256(encode_epoch_accumulator)`).
3. **same reduced-checkpoint commitment** (the reduced base-credential stake commitment / checkpoint
   state hash).
4. **same frozen-leadership hashes** (`frozen_leadership::canonical_hash` for each sealed target epoch,
   epoch-indexed).
5. **same rewards/go/pots evidence surfaces from CE-4A.2** where retained (treasury/reserves/go/rewards
   at the boundaries the run still holds; leadership nesPd is epoch-indexed and always retained).
6. **same promotion-certified authority availability** (`promotion_leadership_authority_for_epoch` resolves
   for the same epochs).
7. **same `forbidden_paths = false`** (no reimport, cli_oracle, seed_window_replay, materialize_bootstrap_into).

**FAIL-LOUD** on any divergence; machine-readable `ce4a-3-evidence.json` (interrupted vs uninterrupted,
per-assert). Local `#[ignore]` evidence run (like CE-4A.1/4A.2), NOT a CI gate.

---

## 4. `EpochViewPostPromotionMismatch` — FIRST-CLASS finding (the load-bearing gate)

> **RESULT (2026-07-24): landed on (b) — a REAL production restart-authority gap.** The restart-only
> proof ran the genuine warm restart and tripped `EpochViewPostPromotionMismatch` at POST-1341 (seed+3).
> Root cause: `maybe_recover_promoted_authority` (`epoch_wire.rs:912–915`) terminates at seed+3 — S4-L2
> extended the LIVE promotion to the frozen regime but not the recovery seam. Sealed fix opened as
> **`SLICE-CE-4A-3-R1-WARMSTART-FROZEN-RECOVERY.md`**; CE-4A.3 restart-only (#12) is BLOCKED on it. The
> restart harness is written (local, uncommitted) as the reproducer/validator.


CE-4A.2 surfaced it: splitting one continuous fold into TWO `run_relay_loop_with_sched` calls over the
SAME live stores + SAME in-memory `ForwardSyncState`, **without a `warm_start_recovery` between them**,
re-enters the eview warm-start-across-boundary recovery at POST-1341 and fails closed
`Activate(EpochViewPostPromotionMismatch)`. CE-4A.2 avoided it correctly (two independent single-call
runs) because 4A.2's claim was byte-exact OUTPUTS, not restart. **CE-4A.3 is where it must be resolved.**

**The discriminating test.** CE-4A.3's restart is a GENUINE warm restart (§2 step 3: drop → reopen from
durable → `warm_start_recovery` → input reassembly) — the production path the across-boundary recovery
fixes (`d7653561` / `dabb4210`) were built for. Two outcomes, and CE-4A.3 MUST land on one explicitly:

- **(a) Genuine warm restart is replay-equivalent.** Then the CE-4A.2 mismatch is proven a HARNESS-ONLY
  re-entry artifact (calling the loop twice while skipping recovery — an invalid setup), and the
  production restart authority is sound. CE-4A.3 claims restart equivalence, and records that the invalid
  two-calls-without-recovery setup is forbidden in the harness.
- **(b) Genuine warm restart ALSO trips `EpochViewPostPromotionMismatch`.** Then it is a REAL production
  restart/re-entry authority gap. CE-4A.3 does NOT claim restart equivalence; it opens a **sealed fix
  slice** (the eview post-promotion cross-check must admit a legitimate warm-start re-entry after a
  crossed boundary) and lands only once the production path is fixed and the restart proof is green.

**Hard gate (verbatim intent):** CE-4A.3 may NOT claim restart equivalence if it only avoids the mismatch
by using a non-production setup. The restart MUST exercise the real `warm_start_recovery` + production
input reassembly across the 1341 boundary. If a shortcut is needed to make it pass, that shortcut is the
finding — pursue (b), not a green-by-avoidance.

*Hypothesis (to be proven, not assumed): outcome (a).* `d7653561` fixed the EVIEW warm-start-across-first-
boundary halt (recovery dispatches by the durable record's `target_epoch`; seed+1 re-binds from the
bridge), and CE-4A.1's warm-start survival proof shows a real kill+recover is deterministic. The CE-4A.2
mismatch skipped that recovery. But CE-4A.3 proves it through a genuine restart — the proof decides.

---

## 5. Design (extend the CE-4A harness; production path only)

- Extend `#[cfg(test)] mod ce4a_continuous_self_sufficiency` in `crates/ade_node/src/node_lifecycle.rs`.
- **Restart-only proof (build FIRST):** a `drive`-style run that crosses 1340→1341, then performs the
  genuine warm restart (drop handles + fwd → reopen → `warm_start_recovery` → reassemble inputs →
  continue the loop to 1342), and compares the §3 surfaces to the uninterrupted CE-4A.1 run. The restart
  must NOT reuse the pre-restart `ForwardSyncState` or live store handles (that is the §4 invalid setup).
- **Rollback/refold (add SECOND) — `SLICE-CE-4A-3-R2-ROLLBACK-REFOLD.md` (#13):** within-k canonical
  rollback (the `48fc423a` guard) + forward refold through the production path (the `WalEntry::RollBack`
  marker, `RecoveryAdmissionPolicy::cardano()`), then re-compare. Reuses the S5 rollback machinery but
  drives it via the production loop, not `co_advance`. Restart-only (#12) is R1-sealed (`7266f90c`).
- THE HARD RULE: no change to the production composition to make the test pass. If restart re-entry needs
  a production change, that is §4 outcome (b) — a sealed slice, reviewed and committed on its own.

---

## 6. Sequence (commit discipline)

1. ✅ Push `af3dc9c7` (CE-4A.2).
2. Commit this CE-4A.3 authority doc (doc-before-impl).
3. Build the **restart-only** proof first; resolve §4 (a) or (b) before proceeding.
4. Add **rollback/refold** second.
5. Commit CE-4A.3 only when BOTH are green (restart-equivalence + rollback-equivalence through the
   production loop). If §4 lands on (b), the production fix slice commits first, then CE-4A.3.
6. Then decide **CE-4B** / 1343 extraction.

No CE-4-final claim. No live-operation claim. No bounty claim.

---

## 7. Invariants (evidence — no new IDs unless §4(b) requires one)

- **T-REC-03 / T-REC-05 / DC-WAL-02** (restart/recovery continuity) — exercised through the production
  loop, not just `co_advance`.
- **DC-NODE-22** (single-producer warm-start re-entry from the recovered own-spine tip) — extended to a
  restart ACROSS a crossed boundary.
- **DC-EPOCH-19/25** (self-sufficiency / frozen leadership authority) — preserved across restart+rollback.
- The S5 replay-equivalence contract (`687fea98`) — re-proven on the production path.
- If §4 lands on (b), a new/strengthened invariant for the eview warm-start-re-entry post-promotion
  cross-check is added with the sealed fix slice.
