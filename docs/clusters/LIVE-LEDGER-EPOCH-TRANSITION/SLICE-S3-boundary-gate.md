# S3 — Boundary transition: the byte-exact NEWEPOCH crossing wired onto the live follow

**Rule:** strengthens DC-EPOCH-19; **declares DC-EPOCH-21** (the boundary transition is byte-exact vs the
canonical cardano NEWEPOCH order — a single POOLREAP, discriminant-correct reward crediting). **Cluster:**
LIVE-LEDGER-EPOCH-TRANSITION.
**Depends on:** S1 (`cross_epoch_boundary` + the two-buffer reward model + the codec), S2 (the live
within-epoch fold + the durable accumulator store + the observe-only stall the boundary currently hits),
DC-EPOCH-18 (the seed+2 byte-exact stake — the bootstrap-transient reward seed the native RUPD takes over
from), the reduced-checkpoint stake aggregate (`aggregate_pool_stake`).
**Status:** In progress — item #1 (POOLREAP reconciliation + discriminant) + item #2a (the BLUE
per-credential mark, `build_boundary_mark_snapshot`; oracle 27/27 byte-preserved, +1 test) DONE (CE-3a/CE-3b/CI
green; DC-EPOCH-21 declared). Item #2b reframed as **BOUNDARY-ALIGNED-MARK-CAPTURE** (declares DC-EPOCH-22):
the live mark is captured at the EXACT boundary point from the lineage-matched reduced checkpoint, via a
co-advancer that segments the checkpoint+accumulator advance at each boundary — NOT a naive read at the
post-pass tip (byte-wrong in catch-up AND steady-state). #2b DONE (three hermetic sub-commits `8d047dee` /
`ee33cc4c` / `8232fe73` + **CE-3c PROVEN LIVE 2026-06-29**: two preview crossings 1338→1339 seam + 1339→1340
native, mark at the boundary point 70,655 slots behind the catch-up tip). #3 (CE-3d byte-exact differential
gate) pending. DC-EPOCH-22 stays `declared` (live-proven; the formal flip is a cluster-close event with the
accumulator-as-authority S4 + CE-3d), in step with siblings DC-EPOCH-19/20/21.

> S2 made the accumulator track every within-epoch block and **stall, observe-only, at the boundary**
> (`MissingBoundaryStake`, because the live driver supplies `ctx.boundary_mark = None`). S3 supplies the
> mark and makes the boundary **fire** — RUPD over the held `nesBprev` → SNAP rotation → POOLREAP →
> enactment — and proves the self-computed reward update + rotated go-snapshot **byte-exact against a live
> cardano-node** at ≥2 self-derived boundaries (CE-3). It is the decisive slice for the cluster's thesis:
> after S3 the accumulator no longer runs out of self-derived future authority at a boundary.

---

## 1. The gap S3 closes

`cross_epoch_boundary` exists (S1, `epoch_accumulator.rs:386`) but has **no live caller that can reach
its body**: the S2 live advancer forces `ctx.boundary_mark = None` (`epoch_accumulator_advance.rs:111`),
so every boundary block fail-closes `MissingBoundaryStake` and the accumulator freezes at its last
within-epoch slot (live-proven 2026-06-28: stall at slot 115689630 / epoch 1339). The boundary transition
therefore never runs on the follow, and two byte-exact reconciliation items recorded at S1 are still open
(below). S3 supplies the mark, reconciles the boundary transition to the canonical NEWEPOCH order, and
gates it byte-exact against cardano-node.

---

## 2. Proof obligations — RESOLVED (cardano-ledger source + the Ade surfaces, 2026-06-28)

Read off `~/Documents/ade-planning/reference/cardano-ledger` (Shelley `Rules/PoolReap.hs`, Conway
`Rules/{Epoch,NewEpoch,Ratify}.hs`; Conway reuses the Shelley POOLREAP) and the actual Ade surfaces
(file:line below). These are load-bearing; the reconciliation is built to them.

### PO-S3-1 — POOLREAP is ONE rule in a fixed order; Ade splits it across two fns that DON'T compose.

**Canonical (`PoolReap.hs:132-241`), exact order:**
1. **future-pool adoption** — merge `psFutureStakePoolParams` into `psStakePools` (rebuild matched pools
   with future params, keep current deposit + delegators); `psFutureStakePoolParams := ∅`.
2. **retired = { k : psRetiring[k] == e }** — EXACT epoch `== e`.
3. **refunds** — for each retiring pool, its `spsDeposit` keyed by the pool's OWN reward-account
   credential (`unAccountId spsAccountId`, a full `Credential Staking`); **partition by
   `isAccountRegistered`**: registered → refund to that reward account, unregistered → unclaimed.
4. **accounts update, REFUND BEFORE CLEAR** (the composition
   `removeStakePoolDelegations (delegsToClear cs retired) . addToBalanceAccounts refunds` is g-then-f):
   (a) credit `refunds` to the pools' own reward accounts, then (b) clear the DELEGATORS' delegation
   pointers (`delegsToClear` = the stake creds delegating TO a retiring pool — a DIFFERENT credential set
   from the refund targets).
5. **PState removals** — `psStakePools \ retired`, `psRetiring \ retired`, VRF-hash occurrences.
6. **pots** — `deposited -= (refunded + unclaimed)`; `treasury += unclaimed`.

**Ade today (the agent-mapped split):**
- `apply_epoch_boundary_with_registrations` (`rules.rs:628`, the shared boundary fn) does inline
  retirement (`rules.rs:1135-1172`): refund deposit to the operator reward account (or treasury if
  unregistered) + remove the pool from `pools` AND from `retiring`. It does **NOT** clear delegations,
  and it reaps `≤ epoch` (not `== e`).
- `delegation::apply_pool_reap` (`delegation.rs:315`) does future-pool adoption + delegation-clear +
  pool removal, gated `== epoch`, but **NO** deposits.
- `cross_epoch_boundary` (`epoch_accumulator.rs:417-424`) calls the boundary fn FIRST, then
  `apply_pool_reap`. **BUG (agent-confirmed):** the boundary fn already emptied `retiring`, so
  `apply_pool_reap`'s `retire_epoch == entered_epoch` match finds nothing → **delegations are never
  cleared** (the future-pool adoption half still runs; the clear half is a silent no-op). The full-ledger
  path (`apply_epoch_boundary_full → apply_epoch_boundary_with_registrations`, no `apply_pool_reap`) never
  clears delegations either.

**Resolution (S3):** consolidate POOLREAP into the canonical single order INSIDE the shared boundary fn
(so the full-ledger path is fixed too): adoption → reap `== e` → refund(discriminant-correct) + treasury
split + pot-shrink → **then** delegation-clear → remove pool/retiring/vrf. Retire the now-redundant
trailing `apply_pool_reap` call in `cross_epoch_boundary` (its adoption + clear move inside the fn). This
is a BLUE change to authority code shared with the full ledger — every existing full-ledger boundary test
MUST stay green (the byte-exact corpus is the arbiter; a pool-retirement-boundary corpus case gains
correct delegation-clearing, a *correctness* change, not a regression).

### PO-S3-2 — reward-account credential discriminant (KeyHash vs ScriptHash).

The boundary projects the reward account to `KeyHash` ALWAYS (`rules.rs:964-965, 1152-1154`,
`reward_account[1..29]`, byte 0 ignored). The within-epoch path decodes the real discriminant
(`epoch_accumulator.rs:777 reward_account_credential`: byte 0 bit 4 `0x10` set ⇒ ScriptHash). cardano
uses the full `Credential Staking` for the refund target, `isAccountRegistered`, and reward crediting.
**Resolution:** the boundary's pool-deposit refund target + the RUPD reward crediting use
`reward_account_credential` (the proven 0xE0/0xF0 decoder, `reward_account_credential_decodes_discriminant`
test), not the KeyHash projection. Script-hash reward accounts then route byte-exactly. (S1's order test
used key-hash accounts — the matching projection — so it passed without exercising this.)

> **Split (item #1 finding, tracked).** PO-S3-2 has TWO halves. The POOLREAP deposit-refund target is
> DONE (item #1: `rules.rs` decodes via `reward_account_credential`). The RUPD REWARD-crediting half is
> SEPARATE and still pending: the operator/member reward distribution (`rules.rs` `op_cred` ~934-940 + the
> `delta_t2` partition ~1087-1102) is keyed by a bare `Hash28` (the discriminant already dropped upstream),
> so a script-hash operator/member reward would mis-route. Routing those by the real discriminant touches
> byte-exact reward outputs + their oracle tests, so it is its OWN item BEFORE CE-3d (the live byte-exact
> RUPD gate) — recorded here, not silently dropped.

### PO-S3-3 — the live `boundary_mark` source (SNAP's new mark).

SNAP sets `ssStakeMark := <current instant stake>` (`Snap.hs:95-103`). On Ade's reduced-checkpoint
architecture the instant stake is `aggregate_pool_stake` (`reduced_aggregate.rs:61`) over the reduced UTxO
checkpoint → `StakeByPool { pool_stakes: BTreeMap<PoolId, Coin>, total_active_stake }`, converted by
`form_mark_snapshot` (`reduced_snapshot.rs:55`). `apply_epoch_boundary_with_registrations` ALREADY accepts
`precomputed_mark: Option<&StakeByPool>` and `rotate_snapshots` (`epoch.rs:94`) rotates mark←new_mark,
set←mark, go←set. **Resolution:** at a live boundary the advancer computes the mark from the reduced
checkpoint at the prior durable tip and threads it as `ctx.boundary_mark` (replacing the forced `None`).
This is the only NEW live input the boundary needs; everything else is held in the accumulator (the
two-buffer `prev_block_production`/`prev_epoch_fees` reward inputs, the pots, the snapshots).

> **Granularity verdict — RESOLVED (2026-06-28, investigation): a per-pool mark is BYTE-INSUFFICIENT.**
> The reward computation reads PER-CREDENTIAL stake from the `go` snapshot's `delegations` map, in TWO
> places: the operator leader reward (`rules.rs:1002` `op_stake = go.0.delegations.get(op_cred)` → `0` if
> absent → wrong leader reward) and the member reward loop (`rules.rs:784` builds `delegator_stakes` from
> `go.0.delegations`; `rules.rs:1046` distributes over it → EMPTY map → ZERO member rewards). But
> `form_mark_snapshot` (`reduced_snapshot.rs:55`) sets `delegations: BTreeMap::new()` — so the existing
> per-pool `StakeByPool` mark (built for EVIEW *leadership*, where per-pool suffices for the VRF threshold)
> rotates to a `go` with no per-credential stake → zero member rewards two boundaries later. **The live
> reward-bearing mark MUST be per-credential** (`cred → (pool, stake)`), built from the reduced checkpoint's
> `sum_base_credential_stake()` (`reduced_utxo_checkpoint.rs:524`, per-credential UTxO) + `cert_state.delegation`
> — the same inputs `aggregate_pool_stake` consumes BEFORE it aggregates to per-pool. This RESHAPES item #2:
> the mark plumbing (`SelectedBlockCtx.boundary_mark: Option<StakeByPool>`, `precomputed_mark: Option<&StakeByPool>`,
> `form_mark_snapshot`) is per-pool and must carry per-credential delegations instead.
> **MEM-OPT tension (the §6 risk made concrete):** byte-exact rewards consume the `go` snapshot, which is
> 2–3 epochs STALE — it cannot be recomputed from the current checkpoint, so the per-credential mark/set/go
> MUST be stored in the accumulator (3× the delegation set in RAM — bounded by the delegation set, NOT the
> UTxO set, but a real cost vs the closed MEM-OPT / BA-08 RssAnon budget). cardano stores exactly this;
> the delegation set is far smaller than the UTxO set, but the delta must be measured. This is a design
> decision (it touches a closed cluster's hard invariant), surfaced to the user before the item-#2 redesign.

### PO-S3-4 — the bootstrap→native seam.

`cross_epoch_boundary` applies the DC-EPOCH-18 bootstrap `pending_reward_update` once at its
`target_epoch+1` boundary, then clears it (`epoch_accumulator.rs:399-405`). The native RUPD takes over at
the first boundary whose ENTIRE input epoch was followed (`pending_reward_update == None`). The exact
native-vs-seed layering is what CE-3's live byte-exact gate verifies (S1 PO #2 deferred it here).

---

## 3. The central invariant (DC-EPOCH-21, this slice declares it)

**DC-EPOCH-21.** *The accumulator's epoch-boundary transition reproduces the canonical cardano NEWEPOCH
result byte-for-byte: a SINGLE POOLREAP in the order future-adoption → reap(`== e`) → deposit-refund
(registered→reward-account by its real credential discriminant, unregistered→treasury, deposit pot shrinks
by both) → delegation-clear → pool/retiring/vrf removal; the reward update computed over the held
`nesBprev` and the `go` snapshot AFTER within-epoch withdrawals; SNAP rotating mark→set→go with the new
mark = the reduced-checkpoint stake aggregate. No split POOLREAP whose halves silently fail to compose; no
KeyHash projection that misroutes a script-hash reward account.* Enforced by: byte-exact unit tests vs the
cardano semantics (registered/unclaimed split, refund-before-clear, `== e`, script-hash routing); the
preserved full-ledger boundary corpus; and the CE-3 live differential gate vs cardano-node at ≥2 self-
derived boundaries.

**DC-EPOCH-22 (BOUNDARY-ALIGNED-MARK-CAPTURE — item #2b declares it).** *The live epoch-boundary stake mark
is captured ONLY from the durable reduced checkpoint materialized at the EXACT selected-chain boundary point
(the last durable block of the closing epoch) — never at a later catch-up tip, never via a per-block stake
scan. The capture is durably bound as a BoundaryMark keyed by the canonical boundary chain point `(slot,
hash)` and the checkpoint lineage at that point, persisted BEFORE the accumulator boundary transition
consumes it. The transition consumes a mark only when the binding is present and its point+lineage match the
canonical chain; a reorg that removes or replaces the boundary point INVALIDATES the mark and forces
deterministic rematerialization (reset-to-seed + replay) — a mark is NEVER reused on an epoch-number match
alone.* This protects the boundary transition's INPUT (DC-EPOCH-21 governs the transition's output given a
correct mark): a mark read at the catch-up tip is byte-wrong (the checkpoint is past the boundary), and even
at the steady-state tip it wrongly includes the first block of the new epoch (SNAP captures the end-of-epoch
stake, before that block). Enforced by: the co-advancer that segments the reduced-checkpoint advance at each
boundary (idempotent-resume, byte-identical to advance-to-tip) and captures `sum_base_credential_stake()` at
the boundary point before the cross; the durable BoundaryMark witness written before the cross + the
point+lineage validation on consume; hermetic tests (steady-state crossing; multi-boundary catch-up crossing;
mark-captured-at-boundary-not-tip; reorg invalidation; observe-only stall on fault); the CI guard; and the
CE-3c live venue crossing.

---

## 4. Scope — what S3 wires, what it defers

**Wires (authoritative-track, byte-exact, persisted):**
- The single canonical POOLREAP inside the shared boundary fn (PO-S3-1) + the discriminant-correct
  refund/reward crediting (PO-S3-2).
- The live `boundary_mark` from the reduced checkpoint at the boundary (PO-S3-3): the accumulator CROSSES
  instead of stalling; a full accumulator checkpoint is written at the boundary (the natural barrier, S1
  cadence) and the post-boundary epoch resumes within-epoch folding.
- The CE-3 byte-exact differential gate: capture cardano-node's reward update + stake snapshot at a
  self-derived boundary, assert Ade's self-computed RUPD + rotated go-snapshot == it, ≥2 boundaries.

**Still observe-only (S4 flips it):** the accumulator is NOT yet the leadership/consensus authority — S3
makes it CROSS correctly and proves byte-exactness, but consensus/leadership still read the seed-anchored
re-derive until S4 retires it. A boundary fail-close in S3 STALLS the accumulator (the S2/PO-6 disposition,
durably recorded + readiness-gated) rather than halting the follow.

**Out of scope:** retiring the seed-anchored leadership re-derive (S4); the multi-epoch N→N+3 self-derived
proof with restarts (S6); operational reconnect/forge gates (alongside S6).

---

## 5. Implementation order (each a commit, mirroring S2's cadence)

1. **BLUE POOLREAP reconciliation + discriminant — DONE** (hermetic, no live). Consolidate the single canonical
   POOLREAP order into `apply_epoch_boundary_with_registrations`; thread `reward_account_credential` for
   the refund + reward target; remove the redundant `apply_pool_reap` call from `cross_epoch_boundary`
   (fold adoption+clear inside). Keep ALL full-ledger boundary tests green; add cardano-canonical unit
   tests (registered/unclaimed→treasury split; refund-before-clear at the overlap; `== e` not `≤ e`;
   script-hash reward-account routing; delegation-clear actually clears). Strengthens DC-EPOCH-19;
   declares DC-EPOCH-21 + the CI guard.
2. **Live boundary-mark wiring — BOUNDARY-ALIGNED-MARK-CAPTURE (declares DC-EPOCH-22).**
   - **#2a — the BLUE per-credential mark BUILD — DONE** (`c1a0ec85`). `build_boundary_mark_snapshot` =
     per delegated credential `base UTxO + reward` → a per-credential `StakeSnapshot`; the boundary fn
     consumes it directly. (Was per-pool `form_mark_snapshot`, byte-insufficient per PO-S3-3: the reward
     reads `go.delegations` per-credential.) Oracle 27/27 byte-preserved (the `None` full-ledger path) + a
     non-zero-member-reward test. The per-credential mark/set/go is held in the accumulator (the MEM-OPT
     tension — the `go` snapshot is 2–3 epochs stale, cannot be recomputed; the accumulator already stores it).
   - **#2b — the LIVE capture + crossing (BOUNDARY-ALIGNED).** The mark is a NEW live input sourced from the
     reduced checkpoint, and it MUST be captured at the EXACT boundary point — `s_prev`, the last durable
     block of the closing epoch — NOT at a later catch-up tip and NOT via a per-block scan. A naive read at
     the post-pass tip is byte-WRONG (catch-up: the checkpoint is already past the boundary; steady-state:
     the tip is the FIRST block of the new epoch, whose UTxO delta must NOT be in the mark). So #2b replaces
     the two independent advance-to-tip calls (`node_lifecycle.rs:2304` reduced checkpoint + `:2309`
     accumulator) with ONE **co-advancer** that SEGMENTS at every boundary:
       - within-epoch: fold the accumulator forward — it already STALLS at the boundary block `s_bb`,
         leaving its cursor at `s_prev` (so the boundary point is free, no predecessor search);
       - at the stall: advance the reduced checkpoint EXACTLY to `s_prev` (`advance_reduced_checkpoint_over_chaindb`
         takes an arbitrary `to_slot`; idempotent-resume → byte-identical to advance-to-tip, only adds a
         read-only `sum_base_credential_stake()`), build the per-credential mark, **durably bind** the
         BoundaryMark (point `(s_prev, hash@s_prev)` from `chaindb.get_block_by_slot` + the mark value) BEFORE
         the cross, then cross the accumulator over `s_bb` with `boundary_mark = Some(mark)`;
       - finally advance the checkpoint the rest of the way to the durable tip (EVIEW currency preserved —
         the checkpoint ends at tip exactly as today, only passing through boundary points en route).
     Identical for steady-state and multi-boundary catch-up (the co-advancer re-derives the segmentation from
     the durable ChainDB + era schedule; it does NOT depend on the EVIEW `BoundaryPromoted` yield, which is
     EVIEW-gated and fires one block too late). REORG: the BoundaryMark's `(slot, hash)` lineage key is
     re-validated against the canonical ChainDB on consume; a removed/replaced boundary point invalidates it →
     the existing reset-to-seed + replay rematerializes — never reused on epoch-number match. OBSERVE-ONLY
     preserved (S2/PO-6): a capture/cross fault STALLS (the accumulator stays at `s_prev`), it never halts the
     proven follow; the EVIEW checkpoint still reaches tip fail-closed.
     **Sub-commits:** **#2b-i** the accumulator boundary-cross entry point (`cross_accumulator_over_boundary_block`,
     `epoch_accumulator_advance.rs`; the S2 mark-exclusion lifted ONLY here via a distinct typed ctx — the
     within-epoch `WithinEpochCtx` stays mark-free) + tests; **#2b-ii** the durable BoundaryMark witness
     (`EpochAccumulatorStore`: `bind_boundary_mark` / `take_boundary_mark_for` + the point+lineage validation
     + schema) + tests; **#2b-iii** the co-advancer in `node_lifecycle` (segment → checkpoint-to-`s_prev` →
     capture → bind → cross → checkpoint-to-tip; the call-site swap) + tests. Then the CE-3c venue proof: the
     accumulator CROSSES a real preview boundary (no `MissingBoundaryStake` stall) and keeps folding with
     non-zero member rewards.
3. **CE-3 byte-exact differential gate** (the decisive proof). Capture cardano-node's reward update +
   stake snapshot at a self-derived boundary (the existing reward_provenance / LedgerDB dump tooling),
   assert Ade's self-computed == it at ≥2 boundaries. Surface any go-snapshot stake drift (the B3c risk
   below) as a real finding, not a tolerance.

---

## 6. Risks

- **Shared BLUE authority.** `apply_epoch_boundary_with_registrations` is used by the full ledger too;
  the POOLREAP consolidation changes its behavior (delegation-clear now happens). The byte-exact corpus +
  the full-ledger boundary tests are the arbiters; any corpus delta must be a provable correctness fix
  toward cardano, captured before/after.
- **The B3c stake residual.** The known borderline-pool stake undercount ([[project_b3c_stake_residual]])
  is about the BOOTSTRAP seed+2 borrowed snapshot; S3's mark is self-derived from the LIVE reduced
  checkpoint, so it may NOT inherit B3c — but CE-3's byte-exact reward gate is the same accuracy class and
  WILL surface any go-snapshot drift. If CE-3 fails on stake accuracy, that is the real next invariant,
  not an S3 regression (live venue is the arbiter; correctness-first).

---

## 7. Acceptance (CE — S3 closes cluster CE-3; contributes to CE-4/CE-5)

- [x] **CE-3a (canonical POOLREAP, hermetic) — DONE:** the single-order POOLREAP is consolidated inside
  the shared boundary fn (`rules.rs` `apply_epoch_boundary_with_registrations`) — future-pool adoption →
  reap `== e` (a STALE `< e` is KEPT, proving `==` not `<=`) → discriminant-correct deposit refund
  (registered → reward account / unregistered → treasury) → delegator clear → pool/retiring removal.
  (Ade's accumulator excludes UTxOState, so there is no deposit pot to shrink; `rewards` and `delegations`
  are SEPARATE maps, so cardano's refund-before-clear order is moot — they never touch the same entry.)
  5 tests in `rules::cert_state_dispatch::poolreap_ce3a` (`poolreap_reaps_exact_epoch_only`,
  `poolreap_refund_registered_else_treasury`, `poolreap_clears_reaped_pool_delegations` [the dead-clear
  regression], `poolreap_script_hash_reward_account_refunds_to_script_cred`,
  `poolreap_adopts_future_pool_params`).
- [x] **CE-3b (full-ledger preserved) — DONE:** every existing ade_ledger test stayed green (719→724 lib,
  +5 new only) + the real-snapshot full-ledger boundary corpus passed unchanged (`epoch_boundary_logic`
  4/4 — real Shelley→Conway retiring-pool boundaries; oracle `conway_/alonzo_epoch_boundary_end_to_end`
  2/2). NO existing expectation changed (existing tests use `0xE0` key-hash accounts — for which the
  discriminant decode yields the identical credential — and set up no boundary-level retiring pools).
- [x] **CE-3c (live crossing, BOUNDARY-ALIGNED) — DONE (preview, 2026-06-29):** on real preview boundaries
  the accumulator CROSSES (no `MissingBoundaryStake` stall) — the mark captured at `s_prev` from the reduced
  checkpoint via the co-advancer (DC-EPOCH-22), durably bound, then consumed by the cross — and resumes
  within-epoch folding. **Two distinct live crossings, seed=1338** (native Mithril FirstRun then warm-start
  resume; venue `~/.cardano-ce3c-firstrun`, peer `127.0.0.1:3002`; proof `~/.cardano-ce3c-proof/`):
  - `CROSSED boundary 1338 -> 1339 at slot 115689630 (mark from s_prev 115689595)` — the seed→seed+1 seam
    (FirstRun).
  - `CROSSED boundary 1339 -> 1340 at slot 115776011 (mark from s_prev 115775977)` — the first FULLY NATIVE
    crossing (warm-start resume; the mark derived from a checkpoint advanced PAST bootstrap, not the seam
    bridge). The node then caught up to slot **115846632 — 70,655 slots past `s_prev`** — yet the mark was
    captured at `s_prev` (the boundary point), DECISIVELY demonstrating the co-advancer segments at the
    boundary, NOT the catch-up tip (a tip read would be byte-wrong by a whole epoch of UTxO deltas).
  Non-zero member rewards are proven hermetically by item #2a's `build_boundary_mark_snapshot` per-credential
  reward test (no accumulator-store dump CLI exists; the live CROSSED logs + the hermetic per-credential
  reward proof together establish CE-3c). Hermetic prerequisites (#2b-i/ii/iii) green + the DC-EPOCH-22 CI
  guard.

  > **Continuous-op ceiling found — SEPARATE cluster, NOT this slice:** after crossing seed+2 (1340) the node
  > fail-closes `rc=43` at `eview prepare: WindowReplayPrepare("window-replay beyond seed+2 not yet wired
  > (candidate 1341, seed 1338)")` — the **EVIEW** (EpochConsensusView) seed-anchored window-replay's known
  > limit (`epoch_wire.rs:627`, commit `23829091`, ECA-B3). This live-confirms the cluster's MOTIVATING
  > problem (the seed-anchored re-derive runs out of future authority at seed+3) — exactly what the
  > accumulator removes at S4. The observe-only accumulator crossed every boundary that reached chain.db; the
  > EVIEW ceiling halted the follow before chain.db got the 1341 boundary, so it bounds how far CE-3c reaches
  > live, NOT the accumulator's correctness.
- [ ] **CE-3d (byte-exact RUPD + go-snapshot, the cluster CE-3 gate):** Ade's self-computed reward update
  + rotated go-snapshot == the live cardano-node's at ≥2 self-derived boundaries (the live differential).
- [x] **CI — DONE:** `ci/ci_check_poolreap_single_canonical.sh` asserts the single-POOLREAP structure —
  (A) strict `== e` (no `<= new_epoch.0`); (B) discriminant-correct refund (`reward_account_credential`);
  (C) the live clear (`!retired.contains(pool_id)`); (D) no trailing `apply_pool_reap(` call in
  `cross_epoch_boundary`; (E) DC-EPOCH-21 in the registry. Registry `tests`/`ci_scripts` populated;
  DC-EPOCH-21 declared (the RUPD-crediting + live-mark + differential-gate enforce as later S3 items land).

## 8. What S3 does NOT do

No leadership-authority flip (S4). No multi-epoch self-derived N→N+3 with restarts (S6). No operational
reconnect/forge gates (alongside S6). The accumulator remains observe-only — S3 proves it CROSSES
byte-exactly; S4 makes consensus read it.
