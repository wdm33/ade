# CE-4A.3-R4 — warm restart after rollback-before-refold handles pending-refold state safely

> **Status: OPEN + PARKED (implementation reverted, NOT committed). HARDENING — NOT a #13/CE-4A.3 blocker.**
> Surfaced by the CE-4A.3-R2 (#13) rollback+refold proof after R3 landed; a warm-RESTART concern, separate
> from #13's CONTINUOUS rollback+refold path. The R4 proof (`drive_rollback_then_restart_proof`) drove the
> real warm-restart-after-rollback and surfaced a CHAIN of sub-gaps (§1a): R4a + R4b were fixed and
> validated past their seams, but **R4c (a deeper VRF/nonce reconstruction gap) is still RED**, so R4 is
> NOT end-to-end green. Per the invariant-slice rule (a sealed slice must be replay-verifiable end-to-end),
> the R4 implementation MUST NOT land partially — fixes (a)+(b) are parked (off-repo patch, §1a), NOT
> committed. **Return to R4c BEFORE any live / bounty failure-recovery certification** (a hard precondition
> there; NOT a precondition for CE-4B's longer continuous-operation proof, which may NOT claim crash-window
> recovery closure).

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Related:** CE-4A.3-R1 (`7266f90c`, the frozen-recovery seam +
`RecoveryEpochUnsealed`), CE-4A.3-R3 (rollback-aware eview resolution), CE-4A.3-R2/#13 (the proof that
surfaced this). **Does NOT block #13.**

---

## 1. The finding

With R3 landed (the eview resolver correctly selects the rolled-back tip epoch 1341, not the stale 1342),
the #13 rerun hit a DEEPER error at run 2's startup eview recovery:

```
eview recovery: RecoveryEpochUnsealed { target_epoch: EpochNo(1341) }
```

**Ordering root cause.** The startup eview recovery runs at `node_lifecycle.rs:2383` — **before** the loop,
once per `run_relay_loop_with_sched` call. The `ResetAndRefold` that reseals epoch authority is at
`:2847` — **inside** the loop, after each admit. After a rollback that un-crossed 1341→1342, epoch 1341's
frozen leadership is not yet resealed (the earlier fold to 1342 moved past it); the startup recovery targets
1341 and fails closed on the not-yet-resealed authority, **before** the advance that would reseal it.

**Why R1's `RecoveryEpochUnsealed` is CORRECT here.** It fails closed rather than fabricating authority — the
right behaviour. The gap is that the startup recovery does not recognise a *pending-refold* state (a rollback
whose reconcile has not yet run) and reports a generic unsealed terminal instead of deferring to the refold.

---

## 1a. Proof-run findings (R4a / R4b / R4c) — the chain of warm-restart-after-rollback gaps

The R4 proof `drive_rollback_then_restart_proof` (in the parked patch) drives the REAL scenario: fold to
1342 → controlled rollback to P (epoch 1341) → CRASH (drop all handles, NO reconcile) → warm-restart
(reopen + `warm_start_recovery` + reassemble + `run_relay_loop_with_sched`) → refold (P, 1342] → compare to
uninterrupted. It surfaced THREE sub-gaps in the never-before-tested warm-restart-after-rollback path (S5
proved rollback replay-equivalence behind `co_advance`, never through `warm_start_recovery`):

- **R4a (FIXED, validated, parked) — `warm_start_recovery` block-bytes vs RollBack.** The pre-load loop
  (`node_lifecycle.rs`, ~L3463) required ChainDb bytes for EVERY WAL `AdmitBlock`, including ones a later
  `WalEntry::RollBack` superseded (their blocks TRIMMED by `commit_rollback`) → `DurableBlockBytesMissing`.
  The downstream `replay_from_anchor` already honors the RollBack (its `compute_superseded` pre-pass abandons
  them). FIX: the pre-load reuses `ade_ledger::wal::compute_superseded` (made `pub`) to skip superseded
  AdmitBlocks; a NON-superseded missing block still fails closed (corruption invariant preserved). VALIDATED:
  the rerun's `warm_start_recovery` succeeded (past `DurableBlockBytesMissing`).

- **R4b (FIXED, validated, parked) — reconcile-before-eview-recovery.** `run_relay_loop_with_sched`'s startup
  eview recovery ran before the accumulator was resealed (the fold to 1342 pruned 1341; the rollback cleared
  the anchor) → `RecoveryEpochUnsealed{1341}`. FIX: the loop invokes the PRODUCTION `ResetAndRefold`
  (`advance_ledger_state_to_durable_tip`, tip-extended schedule, gated by `recovery_policy`) BEFORE the eview
  recovery — no-op ForwardFold for a consistent warm-start, reseal for a pending one. VALIDATED: the rerun
  resealed 1341 and ran the post-restart refold (crash-window state `1341 unsealed at restart = true`).

- **R4c (OPEN, RED — the blocker) — VRF/nonce reconstruction after warm-restart-after-rollback.** With
  R4a+R4b, the refold's FIRST re-fed 1341 block (P+1 = 115942674) fails header validation:
  `Pump(Receive(Validity(Header(VrfCert(VerificationFailed)))))`. The warm-restart-after-rollback
  reconstructs a subtly-WRONG epoch nonce (eta0) / leader schedule for validating 1341 headers (a CONSISTENT
  warm-start — #12 — reconstructs a valid authority; the rollback-pending reconstruction does not). This is
  a DEEPER class than R4a/R4b (consensus-state reconstruction, not marker-honoring): it needs investigation
  of how `warm_start_recovery` rebuilds `chain_dep.epoch_nonce` / the leader schedule from a rolled-back WAL
  + a trimmed ChainDb + a nearest-snapshot forward-replay.

**Parked artifact:** the exact R4a/R4b fixes + the `drive_rollback_then_restart_proof` harness are saved
off-repo at `~/.cardano-ce3d-extract/ce4a-3-r4-parked-fixes-ab-and-harness.patch` (NOT committed — R4 is not
end-to-end green). The working tree is reverted. Resume R4c from that patch.

---

## 2. Why this is SEPARATE from #13 (the discrimination)

- **Continuous production rollback** (participant loop): the eview recovery runs **once** at loop start,
  *before* the rollback — never re-run. The rollback's reconcile (`ResetAndRefold`) reseals on the next
  advance. `RecoveryEpochUnsealed` is **not reachable** in the continuous path.
- **This gap** only bites on a **warm RESTART in the crash window** between `commit_rollback` and the
  `ResetAndRefold` that reseals — i.e. the durable state is `tip = 1341, 1341-authority-not-yet-resealed`,
  and a fresh process start re-runs the startup recovery on it.

#13 (option a) faithfully models the continuous path by invoking the PRODUCTION `ResetAndRefold`
(`advance_ledger_state_to_durable_tip`) between the rollback and run 2 — the same reconcile the continuous
loop's next advance does, never a manual reseal. That is correct for #13's continuous claim and does NOT
close this warm-restart gap.

---

## 3. Intent

Make warm-start recovery **safe when the durable state carries a rollback whose reconcile (ResetAndRefold)
has not yet run** — a pending-refold state — instead of failing closed on a not-yet-resealed tip epoch.

---

## 4. Design decision — RESOLVED: reconcile-before-recovery (proactive "drive the refold first")

The startup eview recovery must operate on an accumulator ALREADY reconciled to the durable tip. So
`run_relay_loop_with_sched` invokes the PRODUCTION reconcile
(`advance_ledger_state_to_durable_tip` → `accumulator_recover_admit` → `ResetAndRefold`, gated by
`recovery_policy`) with a tip-extended schedule **BEFORE** the eview recovery (`node_lifecycle.rs:2389`).

This subsumes options (i)/(ii): it "drives the refold first" unconditionally —
- a **no-op** `ForwardFold` for a consistent warm-start (byte-identical to today; the proven #12/#13/live
  paths start from a consistent accumulator so they are unchanged);
- a **reseal** for a rollback-pending / lagging accumulator (a crash in the rollback→refold window);
- **fail-closed** on an uncertified / inadmissible / lineage-contradicted accumulator (the `recovery_policy`
  integrity exception).

No new status, no defer/retry loop, minimal surface. NEVER fabricates authority; NEVER a manual reseal;
NEVER weakens `RecoveryEpochUnsealed` for a genuinely-corrupt store — the reconcile reseals ONLY from the
durable ChainDb (the SOLE authority), else fails closed. It reuses the SAME production `ResetAndRefold`
seam that the loop already runs after every admit (`:2847`) and that #13 option (a) drove explicitly.

---

## 5. Hard prohibitions

- no manual reseal / manual WAL edit;
- no startup recovery pretending the refold already happened (it must actually run the production refold);
- no weakening `RecoveryEpochUnsealed` for a truly-unsealed (non-pending-refold) epoch;
- no claiming crash-window warm-restart safety until this slice is green.

---

## 6. Tests

1. **Targeted proof (the slice's green gate):** fold to 1342 → controlled rollback to P (epoch 1341, the
   crash window: NO explicit reconcile) → **CRASH** (drop every handle + the ForwardSyncState) → warm-restart
   (reopen from durable + `warm_start_recovery` + input reassembly + `run_relay_loop_with_sched`) → the R4
   reconcile-before-recovery reseals 1341 → recover → refold (P, 1342] → the final authority fingerprint is
   BYTE-IDENTICAL to the uninterrupted run. (Without R4 the warm-restart's eview recovery hits
   `RecoveryEpochUnsealed{1341}` — the exact #13 pre-option-(a) failure.)
2. after the refold, the resealed tip-epoch authority is current-lineage (the fingerprint — acc hash,
   checkpoint, frozen leadership, promotion-certified — matches the uninterrupted run).
3. a consistent warm-start (no rollback) is unchanged (the reconcile is a `ForwardFold` no-op) — evidenced
   by #12 restart-only and the live warm-start paths staying green; a genuinely-unsealed epoch with no
   admissible ChainDb refold still fails closed (the `recovery_policy` integrity exception).

---

## 7. Invariants

- **DC-EPOCH-06** (recovery exactness) strengthened: recovery distinguishes pending-refold from
  genuinely-unsealed and never fabricates authority.
- The CE-4A.3-R1 `RecoveryEpochUnsealed` / `RecoveredEpochNonce` guards remain (correct); this only adds a
  pending-refold path.
- **NOT a CE-4 / CE-4B / live / bounty claim.** Warm-restart crash-window safety is claimed ONLY when R4 is
  green.
