# CE-4A.3-R2 — rollback + refold replay-equivalence INSIDE the production-loop harness

> **Status: OPEN (scoped, doc-before-impl).** The final CE-4A.3 leg (#13). CE-4A.3 restart-only (#12) is
> GREEN and the recovery contract is fixed + sealed (CE-4A.3-R1, `7266f90c`). This proves the second half:
> a k-bounded rollback + refold through the SAME production composition is byte-identical to the
> uninterrupted run. When this is green, CE-4A.3 is complete (both legs).

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Parent:** `SLICE-CE-4A-3-RESTART-ROLLBACK.md`.
**Depends on:** CE-4A.3-R1 (`7266f90c`, the frozen-recovery fix + fixture-lineage refresh — the refold
re-crosses boundaries and re-recovers, so it needs strict frozen recovery over current-lineage records),
CE-4A.3 restart-only #12 (green), S5 (`48fc423a` the k-bounded lineage-checked rollback guard, `aa2bba37`
recovery-reconcile, the `WalEntry::RollBack` marker), CE-4A.1/4A.2.

---

## 1. Intent (the claim)

> A k-bounded rollback and refold inside the CE-4A production-loop harness produces byte-identical
> authority and state to the uninterrupted run.

S5 proved rollback replay-equivalence behind `co_advance` (the differential fold). CE-4A.3-R2 proves it
through the SAME production composition CE-4A.1/4A.2/#12 drive — the rollback + refold re-cross the real
boundaries and re-recover the frozen-promoted authority end to end.

---

## 1a. Ratified mechanism (option a — controlled durable rollback, NOT a natural fork-switch)

**Claim nuance (load-bearing):** CE-4A.3 does NOT claim a natural live fork-switch event. It proves
replay equivalence after a CONTROLLED durable rollback using the SAME production rollback + recovery
machinery a real rollback path uses:

```
controlled durable rollback to a canonical within-k ancestor P
  -> production commit_rollback (real WalEntry::RollBack marker)
  -> production admit_rollback k-guard approves
  -> ResetAndRefold
  -> refold the SAME canonical block bytes
  -> compare against the uninterrupted run
```

This satisfies CE-4A.3's intent (replay-equivalent recovery/refold) better than a fork-switch, which
adopts a DIFFERENT chain. The rollback is CONTROLLED but NOT fake: the authority transition still goes
through the real durable rollback + recovery/refold path.

**Allowed:** `commit_rollback` to move the ChainDB to P; a replay-reconstructed P ledger/chain_dep ONLY
as harness setup input; P MUST be a real canonical point from the same corpus, within k; the production
`admit_rollback` guard MUST approve it; `ResetAndRefold` MUST execute; refold the EXACT same canonical
block bytes; compare final state to the uninterrupted run.

**Forbidden:** no direct accumulator mutation; no manual WAL surgery; no bypassing `commit_rollback`; no
bypassing `admit_rollback`; no fake rollback marker; no synthetic non-canonical point; no fork-switch
claim; no different-chain adoption claim; no CE-4 final claim from this alone.

**Evidence bundle MUST carry:**
```json
{
  "rollback_trigger": "controlled_commit_rollback_to_canonical_within_k_point",
  "natural_fork_switch": false,
  "same_block_refold": true,
  "production_commit_rollback_used": true,
  "production_admit_rollback_used": true,
  "reset_and_refold_used": true
}
```

---

## 2. Path

- **uninterrupted CE-4A run** (the reference — `drive_restart_proof(do_restart=false)`; the #12
  uninterrupted fingerprint).
- **vs. rollback + refold:** run through the production loop; **roll back to an earlier durable point
  within k** (a real canonical corpus point a few hundred blocks below the tip, well inside the security
  window); **refold the SAME canonical corpus blocks** through the production loop; continue.
- Compare the final authority fingerprint to the uninterrupted run.

The rollback is the GENUINE production path: the `48fc423a` k-bounded + lineage-checked rollback-admission
guard + the `WalEntry::RollBack` marker + `RecoveryAdmissionPolicy::cardano()`, re-materialized via the
production reset+forward-refold (never an inverse mutation, never a synthetic edit).

---

## 3. Hard asserts (rolled-back+refolded run == uninterrupted run, byte-identical)

1. **same final selected tip** (durable `ChainDb::tip`).
2. **same accumulator canonical hash** (`blake2b_256(encode_epoch_accumulator)`).
3. **same reduced-checkpoint commitment**.
4. **same frozen-leadership hashes** (each sealed target epoch, epoch-indexed).
5. **same recovered/promoted epoch-authority hashes** (the eview promoted-view identity after the refold's
   re-recovery — the CE-4A.3-R1 frozen reconstruction).
6. **same rewards/go/pots evidence surfaces** where retained.
7. **same `forbidden_paths = false`** (no reimport / cli_oracle / seed_window_replay / materialize_into).

**FAIL-LOUD** on any divergence; machine-readable `ce4a-3-r2-evidence.json`. Local `#[ignore]` evidence
run over the current-lineage fixture (the CE-4A.3-R1 refresh), like #12.

---

## 4. Hard prohibitions

- no seed fallback;
- no window-replay resurrection (recovery stays strict frozen — CE-4A.3-R1);
- no `materialize_bootstrap_into`;
- no synthetic / non-production rollback (the rollback uses the real k-bounded guard + `RollBack` marker +
  the production reset+refold, NOT a hand-edited store);
- no arbitrary old-store migration (the fixture-lineage refresh is harness-local, per CE-4A.3-R1);
- **no CE-4 final claim until #13 is green** (and even then CE-4A.3 == both legs proven, NOT CE-4).

---

## 5. Design (extend the CE-4A.3 harness; production path only)

- A `drive_rollback_proof`-style run in the CE-4A mod (`node_lifecycle.rs`), reusing the CE-4A.3 setup +
  `refresh_prep_eview_records` (current-lineage records) + `capture_authority_fp` (the §3 fingerprint).
- Cross at least one boundary, then perform ONE within-k canonical rollback (the `48fc423a` guard + the
  `WalEntry::RollBack` marker) to a real corpus point, refold the SAME blocks through
  `run_relay_loop_with_sched`, continue, capture. Compare to the uninterrupted fingerprint.
- THE HARD RULE: no production-composition change. If the rollback/refold needs a production change, that
  is its own sealed slice, reviewed + committed on its own (as CE-4A.3-R1 was for the recovery gap).

---

## 6. Commit boundary

1. This authority doc (doc-before-impl).
2. Implement the rollback/refold harness.
3. Run the long proof.
4. **Commit CE-4A.3 only if rollback/refold == uninterrupted** (byte-identical on all §3 surfaces). Then
   CE-4A.3 is complete — restart-only (#12, R1-sealed) + rollback/refold (R2). Per-slice review first.

No CE-4 final claim. No live-operation claim. No bounty claim.

---

## 7. Invariants

- The S5 replay-equivalence contract (`687fea98`) re-proven on the production path (rollback axis).
- **DC-NODE-23..29** (rollback-follow wiring, k-bounded admission, rollback+reselection replay-equivalence
  via `WalEntry::RollBack`) — exercised through the CE-4A production loop, not just the S5 unit scope.
- **DC-EPOCH-19/25** (self-sufficiency / frozen leadership authority) preserved across rollback+refold.
- No new IDs unless the rollback/refold surfaces a genuine production gap (as #12 surfaced the recovery
  gap → CE-4A.3-R1).
