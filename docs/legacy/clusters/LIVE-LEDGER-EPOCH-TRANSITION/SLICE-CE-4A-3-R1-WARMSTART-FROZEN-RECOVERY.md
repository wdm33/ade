# CE-4A.3-R1 — warm-start recovery reconstructs the frozen-promoted epoch authority

> **Status: OPEN (scoped, doc-before-impl).** The sealed production fix surfaced by CE-4A.3 (§4 outcome
> **b**): a genuine warm restart in the post-S4 frozen-promotion regime (durable promotion target ≥
> seed+3) fails closed `EpochViewPostPromotionMismatch` because the warm-start recovery seam still
> terminates at seed+3 instead of reconstructing the same frozen-promoted authority the live path
> produces. This slice makes **live promotion and recovery promotion the same authority contract.**

**Cluster:** LIVE-LEDGER-EPOCH-TRANSITION. **Blocks:** CE-4A.3 (restart-only #12 cannot pass until this
lands). **Depends on / mirrors:** S4-L2 (`db702a54`, the live frozen-authority flip), the warm-start
recovery dispatch (`d7653561`, seed+1 bridge; `ad41b274`/`c4e0413b`, seed+2 window replay).

---

## 1. The finding (localized, evidence-backed)

`maybe_recover_promoted_authority` (`crates/ade_node/src/epoch_wire.rs:800`) dispatches by the durable
activation record's `target_epoch`:
- `seed+1` → bridge recovery (`bind_bridge_view`) — `d7653561`.
- `seed+2` → window replay (`try_recover_at_boundary`) — `c4e0413b`/`ad41b274`.
- **`seed+3+` → line 912–915, terminal `EpochViewPostPromotionMismatch`.**

S4-L2 (`db702a54`) flipped the **live** promotion (`prepare_authority_for_candidate_slot`, the
`candidate_epoch.0 >= seed_epoch.0 + 2` branch, `epoch_wire.rs:633–693`) to read the promotion-certified
epoch-indexed frozen authority for EVERY candidate ≥ seed+2 (the seed+2 ceiling deleted). **`git log -L`
confirms S4-L2 did NOT touch `maybe_recover_promoted_authority`** — the recovery seam was left behind. So
a node that crosses into seed+3 and warm-restarts cannot recover its promoted authority. **Evidence:**
`ce4a-3-STOP-evidence.json` — genuine warm restart at `restart_tip=115862416` (POST-1341, seed+3) →
`RelaySync("eview recovery: Activate(EpochViewPostPromotionMismatch)")`.

This violates recovery replay-equivalence: **crash recovery must produce the same authoritative state as
clean uninterrupted replay, and recovered state must be derivable from persisted canonical inputs.**

---

## 2. Intent

Make warm-start recovery reconstruct the SAME `EpochConsensusView` that uninterrupted live promotion
produces, from the SAME persisted canonical inputs:

```
uninterrupted live promotion (epoch_wire.rs:633–693):
  prepare_authority_for_candidate_slot
  → store.promotion_leadership_authority_for_epoch(C)           (the frozen object)
  → FrozenLeadershipViewMetadata { source_point = (frozen.source_slot, frozen.source_hash),
                                   checkpoint_commitment = frozen.source_checkpoint_commitment,
                                   nonce = eta0(C), snapshot_phase = Set, protocol_params_commitment }
  → EpochConsensusView::from_frozen_leadership(&frozen, &metadata)
  → to_pool_distr_view(...)

warm restart (this slice):
  maybe_recover_promoted_authority   [target_epoch = C, C >= seed+2, post-S4 store]
  → SAME frozen object, SAME metadata, SAME from_frozen_leadership, SAME projection
  → recover the active view against the durable record (no NEW WAL write)
```

---

## 3. Tier classification

- **true:** recovery must be replay-equivalent to clean uninterrupted execution (the recovered view is
  byte-identical to the live-promoted view).
- **derived:** Cardano Praos validation after restart must use the same epoch leadership, nonce, source
  point, and checkpoint commitment as the uninterrupted node.
- **release:** the CE-4A.3 restart-only proof (#12) cannot pass until this is mechanically shown.
- **operational:** none — no operator workaround (a restart in the frozen regime is a normal event).

---

## 4. Required design

Extend `maybe_recover_promoted_authority` into the frozen regime; keep the bridge only where still valid.

**Required shape:**
- **`target_epoch == seed+1`** → keep the bridge recovery (bootstrap seam, DC-EPOCH-15; still valid).
- **`target_epoch >= seed+2` (post-S4 frozen regime)** → reconstruct from the epoch-indexed frozen
  authority, mirroring the live branch byte-for-byte:
  - the frozen object = `store.promotion_leadership_authority_for_epoch(target_epoch)` — REQUIRED;
  - source point = `frozen.source_slot` / `frozen.source_hash` (persisted freeze-time lineage, NEVER the
    durable tip);
  - checkpoint commitment = `frozen.source_checkpoint_commitment`;
  - view = `EpochConsensusView::from_frozen_leadership(&frozen, &metadata)`, projected via
    `to_pool_distr_view`;
  - **no window replay, no `materialize_bootstrap_into`, no seed fallback, no latest/current/nearest
    read, no terminal at seed+3.**
  - Recover the active view against the durable record (mirror the seed+1 branch's
    `recover_active_view(Some(record), Some(&source))` + `promote`); **NO new WAL write** (the record is
    already authoritative).
- **Missing / non-promotion-certified / mismatched / malformed frozen authority** → fail closed with a
  **structured recovery error** (a `PromotionLeadershipUnavailable`-class / `NotPromotionCertified`
  variant), NEVER a fallback to the old window-replay path for a post-S4 store.

**KEY DESIGN OBLIGATION (proof-discipline, resolve at implement-entry): eta0(C).** The live branch
computes `nonce = eta0(C)` by ticking `chain_dep` (`apply_nonce_input(EpochBoundary{C})`,
`epoch_wire.rs:652`). **`maybe_recover_promoted_authority` currently has NO `chain_dep` parameter.** On
recovery the recovered `chain_dep.epoch_nonce` at POST-C already IS eta0(C) (the boundary tick was
applied durably when the boundary was crossed). The fix must obtain eta0(C) canonically — thread the
recovered `chain_dep` (or its `epoch_nonce`) into the recovery, and PROVE the recovered nonce equals the
live-path `ticked.epoch_nonce` (§5 assert on nonce metadata). This is a signature change to the recovery
+ its caller; get it right or the recovered view will not be byte-identical.

**BLUE-adjacent, consensus-critical** (the eview recovery authority). Implement INLINE; per-slice
security review at commit.

---

## 5. Mechanical acceptance (before committing the implementation)

1. **Direct regression** on `maybe_recover_promoted_authority`: a seed+3+ durable record recovers the
   promoted authority from the frozen object and SUCCEEDS (no terminal).
2. **Recovered view == uninterrupted promoted view** (byte-identical):
   - same canonical hash (the `EpochConsensusView` / projected `PoolDistrView` commitment),
   - same source point (`frozen.source_slot` / `source_hash`),
   - same checkpoint commitment (`frozen.source_checkpoint_commitment`),
   - same pool leadership view,
   - same nonce metadata (eta0(C)).
3. **Negative cases (all structured failures, no fallback):** missing store → structured failure; missing
   epoch → structured failure; bootstrap-only epoch → `NotPromotionCertified` (or equivalent); malformed
   frozen object → structured failure; the old seed-window path is NOT reached for a post-S4 recovery.
4. **Re-run CE-4A.3 restart-only #12** (the production-loop genuine warm restart) → GREEN: recovered
   authority fingerprint byte-identical to the uninterrupted run.
5. **Only if restart-only #12 is green**, continue to CE-4A.3 rollback/refold #13.

---

## 6. Hard prohibitions

- no recovery fallback to seed-window replay for post-S4 stores;
- no latest / current / nearest leadership read;
- no synthetic re-entry setup (the restart proof uses the genuine `warm_start_recovery` path);
- no special-case of epoch 1341 / 1342;
- no weakening of `EpochViewPostPromotionMismatch` (it stays terminal for the cases it legitimately
  guards — malformed/missing/non-certified — it is only REPLACED by real reconstruction for the frozen
  regime);
- no patching the test harness around production recovery;
- no rollback/refold proof (#13) until restart-only (#12) is green.

---

## 7. Commit shape

1. **CE-4A.3-R1 doc-before-implement** (this doc).
2. **CE-4A.3-R1 implementation + restart-only proof green** (the recovery extension + the §5 regressions +
   #12 green). Per-slice security review (BLUE-adjacent) before the commit.

Then resume CE-4A.3 proper (#13 rollback/refold, only on #12 green).

---

## 8. Invariants

- Strengthens the S4-L2 authority contract to the recovery seam: the **recovery** promotion path reads
  the promotion-certified epoch-indexed frozen authority for candidate ≥ seed+2, identically to the live
  path (candidate registry family DC-EPOCH-17/25, and the T-REC recovery-equivalence family).
- The S4-L2 resurrection guard `ci/ci_check_frozen_promotion_no_seed_window.sh` must extend to (or a twin
  must cover) the recovery seam — a post-S4 recovery must not reach the seed-window path.
- A structured recovery-error variant may be added for the frozen-regime failure modes (§5.3); no
  existing invariant is weakened.
