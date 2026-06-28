# S3 — Boundary transition: the byte-exact NEWEPOCH crossing wired onto the live follow

**Rule:** strengthens DC-EPOCH-19; **declares DC-EPOCH-21** (the boundary transition is byte-exact vs the
canonical cardano NEWEPOCH order — a single POOLREAP, discriminant-correct reward crediting). **Cluster:**
LIVE-LEDGER-EPOCH-TRANSITION.
**Depends on:** S1 (`cross_epoch_boundary` + the two-buffer reward model + the codec), S2 (the live
within-epoch fold + the durable accumulator store + the observe-only stall the boundary currently hits),
DC-EPOCH-18 (the seed+2 byte-exact stake — the bootstrap-transient reward seed the native RUPD takes over
from), the reduced-checkpoint stake aggregate (`aggregate_pool_stake`).
**Status:** Proposed.

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

> **Granularity note (verify at impl):** `form_mark_snapshot` aggregates delegations away
> (`delegations: BTreeMap::new()`, only `pool_stakes`). Confirm the RUPD + leadership only need the
> per-pool aggregate at the mark (cardano's mark is per-credential, but the reward/leadership consumers
> read the pool distribution). If a per-credential mark is required for a downstream consumer, that is an
> S3 sub-item to surface, not paper over.

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

1. **BLUE POOLREAP reconciliation + discriminant** (hermetic, no live). Consolidate the single canonical
   POOLREAP order into `apply_epoch_boundary_with_registrations`; thread `reward_account_credential` for
   the refund + reward target; remove the redundant `apply_pool_reap` call from `cross_epoch_boundary`
   (fold adoption+clear inside). Keep ALL full-ledger boundary tests green; add cardano-canonical unit
   tests (registered/unclaimed→treasury split; refund-before-clear at the overlap; `== e` not `≤ e`;
   script-hash reward-account routing; delegation-clear actually clears). Strengthens DC-EPOCH-19;
   declares DC-EPOCH-21 + the CI guard.
2. **Live `boundary_mark` wiring** (the proven-path touch). At the durable-admit boundary, compute
   `aggregate_pool_stake` over the reduced checkpoint at the prior tip → `form_mark_snapshot` →
   `ctx.boundary_mark`; the advancer crosses (no longer forces `None`), writes the boundary checkpoint,
   resumes within-epoch folding. Non-native/test callers unchanged (byte-identical proven path). Venue
   FirstRun proof: the accumulator CROSSES a real preview boundary (no stall) and keeps folding.
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

- [ ] **CE-3a (canonical POOLREAP, hermetic):** the single-order POOLREAP matches cardano —
  registered→refund / unregistered→treasury split, deposit pot shrinks by both, refund-before-clear,
  `== e`, delegations of retired pools ARE cleared, script-hash reward accounts route correctly (tests).
- [ ] **CE-3b (full-ledger preserved):** every existing full-ledger boundary test stays green; any corpus
  delta is a documented correctness move toward cardano (before/after).
- [ ] **CE-3c (live crossing):** on a real preview boundary the accumulator CROSSES (no
  `MissingBoundaryStake` stall), writes the boundary checkpoint, and resumes within-epoch folding (venue).
- [ ] **CE-3d (byte-exact RUPD + go-snapshot, the cluster CE-3 gate):** Ade's self-computed reward update
  + rotated go-snapshot == the live cardano-node's at ≥2 self-derived boundaries (the live differential).
- [ ] **CI:** a DC-EPOCH-21 guard asserts the single-POOLREAP structure (no split whose clear half is
  dead) + the discriminant-correct refund/reward target; registry `tests`/`ci_scripts` populated.

## 8. What S3 does NOT do

No leadership-authority flip (S4). No multi-epoch self-derived N→N+3 with restarts (S6). No operational
reconnect/forge gates (alongside S6). The accumulator remains observe-only — S3 proves it CROSSES
byte-exactly; S4 makes consensus read it.
