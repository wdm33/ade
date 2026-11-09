# PHASE4-N-AK — Recovered anchor tip is the live-follow start authority

> Invariant sketch (IDD Part I). NARROW regression-remediation cluster (recovery/follow
> authority — NOT evidence emission). **N-AJ is paused until AK restores the live
> recover→follow path.** The DC-NODE-31 registry append is deferred to `/cluster-doc`
> (matching the AJ discipline).
>
> **REVISED (Option A — persist):** the recovered store does NOT carry the anchor `(slot, hash)`
> point today; the sidecar + WAL provenance carry only `anchor_fp` (a fingerprint), and the FirstRun
> arm gets the point from the CLI. AK therefore **persists the bootstrap anchor point as additive,
> replayable recovery provenance** and resolves the live-follow start from it — store-derived, never
> CLI-re-supplied at restart (replay-first).

## Context — the regression (confirmed by a live A/B)

Same venue / recovered store / frozen relay / flags, only the binary differs:

- **N-AH binary (`c66fa9a9`) `--mode node --single-producer-venue`** recovers @ a block-8 anchor and
  **FOLLOWS** the frozen relay (sched: `caught_up_to_peer_tip`, `forge_base_block_no: 13`, 29 forges).
- **Current binary (HEAD)** halts at `UnsupportedRollbackPoint`, 0 forges, never follows.

Not venue drift — a real post-N-AH regression **exposed (not caused)** by N-AI AI-S4a's stricter
`RollBackward(Origin) => UnsupportedRollbackPoint` refusal.

### Root cause (grounded) + the N-AH mechanism it replaces
`crates/ade_runtime/src/bootstrap.rs::bootstrap_initial_state` returns `BootstrapState.tip = None` for
a **bare-anchor** recovery (snapshot at the anchor slot, no servable post-anchor block) → the wire
pump starts ChainSync from `Origin` → the relay's initial `RollBackward(Origin)` hits AI-S4a
(`wire_pump.rs:447`, correct) → fail-closed halt. **N-AH "worked" only by accepting that
`RollBackward(Origin)` and re-syncing the whole chain from genesis (1→13)** — wasteful and infeasible
at scale; it never used the anchor as the start point and so never needed the anchor hash. The fix is
to make Ade **FindIntersect at the recovered anchor** (efficient, replay-correct), which requires the
anchor `(slot, hash)` to be available at warm-start.

## Pure transformation (the concept is understood)

```
resolve_live_follow_start(servable_chaindb_tip: Option<ChainTip>,
                          persisted_recovered_anchor: Option<ChainTip>) -> Option<ChainTip>
  = (1) servable_chaindb_tip                      if Some
    (2) else persisted_recovered_anchor           if Some(non-Origin, provenance-bound)
    (3) else None                                 (truly Origin / cold-start)
```
Deterministic, no I/O — `canonical recovered store → canonical live-follow start tip`.

## 1. What must always be true

- **AK-INV-1 (core, new — DC-NODE-31):** After recovery from a non-Origin bootstrap anchor, the
  recovered store **persists the bootstrap anchor point `(slot, hash)` as recovery provenance** (bound
  to the recovered anchor fingerprint). On warm-start, `BootstrapState` **resolves the live-follow
  start tip from that persisted anchor point** whenever ChainDb has no servable post-anchor block. A
  non-Origin recovered store whose anchor-point record is missing / malformed / fingerprint-mismatched
  **fails closed** before live follow starts. *(About the live-follow start authority — it does not
  assert any consumer treats the anchor as a servable ChainDb block.)*
- **AK-INV-2:** the wire-pump FindIntersect `start_point` == the exposed live-follow start tip
  (`Origin` **iff** that tip is `None`) — unchanged consumer (`spawn_live_wire_pump_source`).
- **AK-INV-3 (replay-first):** same recovered store + same WAL ⇒ same persisted anchor point ⇒ same
  `BootstrapState.tip` ⇒ same FindIntersect start. Extends T-REC-05 from the recovered *ledger* to the
  recovered *tip* surface. **Restart correctness is store-derived, never CLI-dependent.**
- **AK-INV-4 (preserved):** AI-S4a — `RollBackward(Origin)` on the single-best-peer pump stays
  fail-closed.
- **AK-INV-5 (preserved):** `ChainDb::tip()` returns `Some` only for a servable post-anchor block —
  the storage contract is unchanged.
- **AK-INV-6 (preserved):** `pump_block` sole roll-forward admit; `apply_chain_event` sole rollback
  authority; recovered ledger fp == WAL-tail post_fp (T-REC-05).

## 2. What must never be possible

- A non-Origin recovered anchor surfacing as a `None`/Origin live-follow start tip (the regression).
- A non-Origin recovered store proceeding to live follow **without** a valid, provenance-bound
  anchor-point record (must fail closed — no silent Origin fallback).
- **Restart correctness depending on CLI re-supply** (same store + different restart flags ⇒ different
  live-follow start — the footgun Option B would create).
- Synthesizing a servable block in ChainDb to carry the tip.
- Weakening AI-S4a's `RollBackward(Origin)` fail-close.
- Using WAL `admit_count` (or any guess) as the anchor point — the point comes only from the persisted,
  provenance-bound record.
- A true Origin / cold-start recovery surfacing a non-`None` live-follow start tip.
- Overriding a servable ChainDb tip with the anchor when post-anchor blocks exist.
- Any change to the N-AJ convergence-evidence emission.

## 3. Deterministic surface · 4. Replay-equivalent

`resolve_live_follow_start` and the persisted anchor-point record are pure functions of the on-disk
recovered store. Same store ⇒ byte-identical persisted anchor point ⇒ byte-identical live-follow start
tip ⇒ byte-identical FindIntersect `start_point`.

## 5. State transitions in scope

1. **PERSIST** (at seed/recover, when `BootstrapAnchor.seed_point` is known):
   `seed/recover: (BootstrapAnchor.seed_point (slot, hash), anchor_fp) → write additive anchor-point
   provenance record (bound to anchor_fp)`.
2. **LOAD + RESOLVE** (at warm-start):
   `bootstrap_initial_state: (chaindb_tip: Option, snapshot_slots, loaded persisted anchor point bound
   to the recovered anchor_fp) → Result<BootstrapState{ tip = resolve_live_follow_start(...) }, Err>`;
   a non-Origin recovered store with a missing/malformed/mismatched record ⇒ `Err` (fail closed).
3. `spawn_live_wire_pump_source: (BootstrapState.tip) → start_point` (`Block(tip)` | `Origin`) — **unchanged**.
4. wire-pump `RollBackward(point) → event | UnsupportedRollbackPoint(Origin)` — **AI-S4a UNCHANGED**.

## 6. TCB color hypothesis

- **BLUE** — `resolve_live_follow_start` + the persisted anchor-point provenance record (its content +
  the provenance binding) + the `BootstrapState` live-follow start tip resolution
  (`ade_runtime/bootstrap.rs`): the authoritative, replay-equivalent recovery decision (what point the
  node recovered to). The *write* of the record at recover is RED I/O writing a BLUE-authoritative
  record; the *load + bind + resolve* is BLUE.
- **Canonical input** — the recovered anchor `seed_point` (`BootstrapAnchor`, minted from
  `seed_slot`/`seed_block_hash`).
- **RED (unchanged)** — `spawn_live_wire_pump_source` / the wire pump.

## 7. Open questions

- **OQ-AK-1 (anchor source) — answered (CORRECTED):** the recovered store does NOT persist the
  seed-point today — the sidecar (`SeedEpochConsensusInputs`) + the WAL `RecoveredBootstrapProvenance`
  carry only `anchor_fp` (a fingerprint), not the `(slot, hash)` point; the FirstRun arm gets the point
  from the CLI. So AK **persists the `BootstrapAnchor.seed_point` as an ADDITIVE recovery-provenance
  record** (written at seed/recover, bound to `anchor_fp`); warm-start **loads** it. **CLI seed-point
  is first-run input only, NOT restart authority** — warm-start is store-derived (replay-first). A
  non-Origin recovered store missing the record fails closed.
- **OQ-AK-2 (materialization, out of scope):** AK targets the live-follow start point. It must NOT
  alter ledger materialization (`bootstrap.rs:216` null-hash target) unless a test proves
  materialization depends on the placeholder.
- **OQ-AK-3 (admission is diagnostic, not primary):** `--mode node` is in scope (the CE-AH-6 proven
  live-follow path). `--mode admission` (`our_hash=0000`) is in scope only if it consumes the same
  `resolve_live_follow_start` helper; do not redesign admission orchestration.

## 8. Proposed registry rule (DECLARED at `/cluster-doc`)

- **DC-NODE-31** (family DC, derived) — *Recovered-anchor live-follow start authority.* The recovered
  non-Origin bootstrap anchor point is **persisted as replayable recovery provenance** and is the
  fallback live-follow start authority when ChainDb has no servable post-anchor tip; resolution =
  servable ChainDb tip → persisted recovered anchor point (non-Origin + provenance-bound) → Origin/None
  only if truly Origin/cold-start; missing/malformed/mismatched record on a non-Origin store fails
  closed. **Does not change `ChainDb::tip()` semantics, does not synthesize a servable block, does not
  weaken `RollBackward(Origin)` fail-close.** Replay-equivalent (extends T-REC-05). `introduced_in =
  PHASE4-N-AK`.

## What AK now touches (still a single narrow remediation slice)

1. Add an additive persisted **anchor-point provenance record** (`(slot, hash)` bound to `anchor_fp`).
2. **Write** it during seed/recover when `BootstrapAnchor.seed_point` is known.
3. **Load** it during warm-start.
4. **Resolve** live-follow start from the servable ChainDb tip or the persisted anchor.
5. **Fail closed** on a missing/malformed/mismatched record for non-Origin recovered stores.

Still: not a rollback change, not an evidence change, not a `ChainDb::tip()` semantic change.

## Acceptance bar (carried into the cluster doc)

- `bootstrap_recover_persists_anchor_point_sidecar` — seed/recover writes the anchor-point record.
- `warm_start_loads_persisted_anchor_point` — warm-start loads it and surfaces it as the live-follow tip.
- `warm_start_non_origin_anchor_missing_anchor_point_fails_closed`.
- `warm_start_anchor_point_fingerprint_mismatch_fails_closed`.
- `same_store_same_anchor_point_same_findintersect_start` (replay-equivalence of the tip surface).
- `bootstrap_bare_anchor_recovery_surfaces_anchor_as_live_follow_tip` (bare anchor ⇒ tip == anchor).
- `bootstrap_true_origin_recovery_surfaces_none_tip` (true Origin ⇒ tip == None).
- `bootstrap_servable_chaindb_tip_wins_over_anchor` (post-anchor ChainDb tip wins).
- `resolve_live_follow_start_treats_zero_hash_anchor_as_origin` (pure-fn unit).
- **Live (operator-run at close):** the FIXED binary on the SAME frozen venue ⇒
  `forge_base_block_no == frozen relay tip block_no` ⇒ 0 `UnsupportedRollbackPoint`.
