# CE-4A.3-R4 — warm restart after rollback-before-refold handles pending-refold state safely

> **Status: GREEN / COMPLETE (R4a + R4b + R4c land together; e2e byte-identical).** Surfaced by the
> CE-4A.3-R2 (#13) rollback+refold proof after R3 landed; a warm-RESTART concern, separate from #13's
> CONTINUOUS rollback+refold path. The R4 proof (`drive_rollback_then_restart_proof` /
> `ce4a_3_r4_warmstart_crash_window_equivalence`) drives the real warm-restart-after-rollback and closed a
> CHAIN of three sub-gaps (§1a): R4a (block-bytes vs RollBack), R4b (reconcile-before-eview-recovery), and
> **R4c (candidate-nonce over-track — now FIXED)**. The proof is GREEN: the post-crash warm-restart refolds
> **byte-identical** to the uninterrupted run (acc_hash `02c016df…`, checkpoint `576591ce…`, final_tip
> 115948834, leadership 1342=`014f96d3…`/1343=`d1ba2eb2…`, promotion-certified 1341/1342/1343;
> `crash_window_state_proven: true`; `forbidden_paths_clean: true`). Warm-restart crash-window recovery is
> now a claimable failure-recovery property (the live/bounty recovery precondition).
>
> **R4c root cause + fix (DC-EPOCH-16).** `warm_start_recovery` built its materialize-replay era-schedule
> with `RSW=None`, which `header_validate` maps to `CANDIDATE_FREEZE_INERT = u64::MAX` — the Praos candidate
> nonce NEVER freezes during the seed→tip replay. A rollback+warm-restart lands the durable tip mid-epoch,
> PAST that epoch's candidate-freeze slot (`firstSlotNextEpoch − ceil(4k/f)`), so the candidate OVER-TRACKS
> (includes blocks cardano excluded) → wrong `eta0(N+1)` → the NEXT boundary's header VRF fails closed
> (`VrfCert`). The consistent (non-rollback) restart at the v5 tip is BEFORE the freeze slot, so it was
> unaffected — which is why #12/#13/CE-4A/CE-4B stayed green and this only bit the rollback-mid-epoch case.
> **Fix (scoped):** thread the venue `rsw` into `warm_start_recovery` so its replay freezes the candidate at
> the SAME slot the live loop does (the loop's `recovered_node_schedule` already takes the `--network` RSW).
> The production caller passes `rsw_for_cli(cli)`. **Deferred follow-up (documented, NOT this slice):**
> persist the RSW/securityParam in the sidecar (v5→v6) so warm-start is fully self-describing (durable store,
> not `--network`, as the sole replay authority) — a broad fixture/blast-radius change, filed separately.

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

- **R4c (FIXED) — candidate-nonce over-track in the warm-restart materialize replay (DC-EPOCH-16).** With
  R4a+R4b, the refold admitted the mid-1341 blocks but failed header validation
  (`Pump(Receive(Validity(Header(VrfCert(VerificationFailed)))))`) at the **1341→1342 boundary** — NOT at
  P+1. The empirical split (`eta0(1341)` epoch-nonce equal; frozen(1341) view byte-identical to run 1; the
  authority correctly promoted to 1341) ruled out the epoch nonce AND the leader schedule, and isolated the
  fault to the **candidate** nonce: `warm_start_recovery`'s materialize schedule used `RSW=None`
  (`CANDIDATE_FREEZE_INERT`), so the candidate never froze during the seed→tip replay and over-tracked past
  1342's freeze slot → wrong `eta0(1342)` at the crossing. A CONSISTENT warm-start (#12) has its tip BEFORE
  the freeze slot, so its candidate is trivially correct — the discriminator. **FIX:** thread the venue
  `rsw` into `warm_start_recovery` (the SAME `--network` RSW the loop's `recovered_node_schedule` uses), so
  the replay freezes the candidate at the correct slot. VALIDATED: the refold is byte-identical to the
  uninterrupted run (see the Status evidence bundle). This was pre-diagnosed 13 days earlier
  (`b4-warmstart-rsw`); the fix matches that note's mechanism.

**Landed together (this slice):** R4a (`compute_superseded` made `pub` + the warm-start block-bytes skip),
R4b (reconcile-before-eview-recovery), R4c (the `rsw` thread), and the `drive_rollback_then_restart_proof`
harness — all in ONE commit, since a sealed slice lands only when e2e green.

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
