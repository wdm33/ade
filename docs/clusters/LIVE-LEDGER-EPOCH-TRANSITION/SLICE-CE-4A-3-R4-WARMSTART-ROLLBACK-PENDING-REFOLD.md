# CE-4A.3-R4 — warm restart after rollback-before-refold handles pending-refold state safely

> **Status: OPEN (scoped, doc-before-impl). HARDENING — NOT a #13 blocker.** Surfaced by the CE-4A.3-R2
> (#13) rollback+refold proof after R3 landed. This is the REAL (narrow) gap the #13 harness models away
> via option (a); it is a warm-RESTART concern, separate from #13's CONTINUOUS rollback+refold path. Decide
> after #13 is green, before broader CE-4 closure.

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

## 4. Design decision (the slice resolves ONE of these)

- **(i) Detect pending-refold, defer eview recovery.** The startup recovery detects the rollback-pending /
  refold-required state (e.g. accumulator anchor cleared / durable tip epoch not sealed while a `RollBack`
  is the latest lineage event) and **defers** the eview recovery until after the `ResetAndRefold` reseals
  the tip-epoch authority — then recovers from the resealed, current-lineage authority.
- **(ii) Fail closed with a clearer structured status.** Keep failing closed, but with a distinct
  `RecoveryPendingRefold { tip_epoch }` (not a generic `RecoveryEpochUnsealed`) so an operator/harness sees
  the state is recoverable-after-refold, and the lifecycle drives the refold first.

Either way: NEVER fabricate authority; NEVER weaken `RecoveryEpochUnsealed` for the genuinely-corrupt case;
the reseal MUST go through the production `ResetAndRefold`, never a manual seal.

---

## 5. Hard prohibitions

- no manual reseal / manual WAL edit;
- no startup recovery pretending the refold already happened (it must actually run the production refold);
- no weakening `RecoveryEpochUnsealed` for a truly-unsealed (non-pending-refold) epoch;
- no claiming crash-window warm-restart safety until this slice is green.

---

## 6. Tests

1. warm restart with `tip = 1341, 1341 not yet resealed, latest lineage event = RollBack` → recovers safely
   (design i) OR a distinct `RecoveryPendingRefold` terminal + a driven refold (design ii); NEVER a generic
   silent/opaque failure.
2. warm restart with a genuinely-unsealed epoch (no pending rollback) → still `RecoveryEpochUnsealed`
   (unchanged fail-closed).
3. after the refold, the resealed tip-epoch authority is current-lineage (canonical hash matches the
   continuous-run authority).

---

## 7. Invariants

- **DC-EPOCH-06** (recovery exactness) strengthened: recovery distinguishes pending-refold from
  genuinely-unsealed and never fabricates authority.
- The CE-4A.3-R1 `RecoveryEpochUnsealed` / `RecoveredEpochNonce` guards remain (correct); this only adds a
  pending-refold path.
- **NOT a CE-4 / CE-4B / live / bounty claim.** Warm-restart crash-window safety is claimed ONLY when R4 is
  green.
