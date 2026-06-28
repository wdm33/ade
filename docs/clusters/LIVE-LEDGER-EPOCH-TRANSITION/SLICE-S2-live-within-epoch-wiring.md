# S2 — Per-block non-UTxO evolution on selected-chain admit (the live within-epoch wiring)

**Rule:** strengthens DC-EPOCH-19; **declares DC-EPOCH-20** (atomic-or-rematerialized selected-block
admission — no *resumed* split authority across the four derived stores). **Cluster:**
LIVE-LEDGER-EPOCH-TRANSITION.
**Depends on:** S1 (`apply_selected_block` + `EpochAccumulator` + the canonical codec), DC-SYNC-01
(tip-after-durable in `apply_plan`), DC-EPOCH-11 (the reduced-checkpoint `LAST_SLOT` lockstep + readiness
gates), the WAL-is-admission-authority recovery (`recovery/restart.rs`), and
`materialize_rolled_back_state` (the proven replay-equivalence-is-recovery pattern).
**Status:** Proposed.

> S2 connects the S1 state machine to EVERY selected-chain admission — **the within-epoch half only.**
> The boundary crossing (the reward over `nesBprev`, SNAP rotation, POOLREAP, the KeyHash withdrawal
> projection) is STRUCTURALLY EXCLUDED from the live path until S3's byte-exact gate: the live driver
> supplies no `ctx.boundary_mark`, so `cross_epoch_boundary` fail-closes `MissingBoundaryStake`. The
> exclusion is enforced by the S1 type, not by a comment.
>
> The decisive property is **not** memory. It is that the four derived authorities — ChainDB/WAL,
> the Praos chain-dep, the `EpochAccumulator`, and the reduced UTxO checkpoint — can never be **resumed**
> at a *mixed* selected-chain prefix (ChainDB at N, accumulator at N−1, checkpoint at N−2). A mixed
> prefix is either impossible or a fail-closed state that deterministically rematerializes before
> validation resumes.

---

## 1. The gap S2 closes

After S1 the transition exists but has zero live callers: `pump_block` (`forward_sync/pump.rs:83`, the
sole tip-advancing call) still does header-validate + `put_block` + WAL + chain-dep and **nothing else** —
the live node never applies a ledger transition, so `block_production` is a stub, `epoch_fees` never
accumulates on the follow, and the cert state is only ever *reconstructed* by the seed-anchored
window-replay. S2 makes the within-epoch half of `apply_selected_block` run on every admitted block,
durably, recoverably, exactly once.

---

## 2. Scope — what S2 wires, what it structurally excludes

**Wires (live, authoritative-track, persisted, recovered):**

- `SelectedBlockCtx` construction at the tip-advancing site from **decoded canonical block data + durable
  selected-chain geometry only** (no peer handle, CLI, or wall-clock).
- The within-epoch fold: certificates + governance (`process_block_certificates`), issuer
  `block_production[issuer] += 1`, `epoch_fees += Σ fee` (with the phase-2 correction — §5 PO-1), and
  within-epoch withdrawals (zero the named reward account).
- Compact-delta persistence of the accumulator beside the reduced checkpoint, with a durable `LAST_SLOT`.
- Restart + reorg recovery by **rematerialization** from the durable canonical prefix (never inverse
  mutation), gated by a fail-closed readiness check against the WAL tail.
- The seed-epoch `nesBcur` seed (partial epoch-N counts) decoded from the bootstrap snapshot, **bound to
  the manifest and epoch-checked**.

**Structurally excludes (until S3):** any epoch-boundary crossing. `ctx.boundary_mark` is `None` on the
live S2 path, so `cross_epoch_boundary` — and with it the reward-over-`nesBprev`, SNAP, POOLREAP, and the
KeyHash withdrawal projection, i.e. *both* known byte-uncertain reconciliation items from S1 — fail-closes
`MissingBoundaryStake` and is never executed live. This is the *full structural exclusion* the cluster
plan and your hard rule require: the live accumulator cannot silently process pool lifecycle or the
boundary reward crediting under byte-compatibility uncertainty, because the path is unreachable without a
mark the driver does not provide. S3 supplies the mark + the byte-exact gate that opens the boundary.

**Out of scope (later slices):** the boundary transition + its byte-exact gate (S3); making the
accumulator the *leadership* authority — S2 persists + recovers it but consensus/leadership still read the
existing seed-anchored re-derive (S4); the live N→N+3 self-derived proof (S6).

---

## 3. The central invariant — atomic-or-rematerialized admission (DC-EPOCH-20)

**The model (grounded in the existing machinery, not a new transaction protocol):**

The **WAL is the single admission authority** — recovery drops every block above the WAL-tail slot and
reconciles the ChainDB tip *to* the WAL tail (`recovery/restart.rs`; `node_lifecycle.rs:2856`,
`rollback_to_slot(wal_tail_slot)`). So the authoritative selected-chain prefix is *defined* as the WAL
tail. The chain-dep, the reduced checkpoint, and now the accumulator are **derived stores**: each is a
pure function of that prefix, and each is already (chain-dep, checkpoint) or newly (accumulator)
**rematerialized to the WAL tail on recovery** via `materialize_rolled_back_state`'s replay-forward fold.

This makes a *resumed* mixed prefix impossible by construction:

1. **Live admit** advances the derived stores after the block is durably admitted (WAL append returned
   `Ok`, gated by DC-SYNC-01). A derived store may momentarily lag the WAL tail (a torn write, a lazy
   cadence) — this is allowed *in-flight*.
2. **Recovery** rematerializes every lagging derived store **up to the WAL tail** by folding its
   transition over the canonical durable blocks `(last_durable_checkpoint, wal_tail]` — the accumulator
   folds `apply_selected_block`, exactly as the reduced checkpoint folds `reduced_block_delta` and the
   ledger/chain-dep fold `block_validity_trusted_replay`.
3. **A fail-closed readiness gate** (`verify_advanced_through`-style: `Lagging`/`Ahead`/`SeedMismatch`/
   `Unsealed`) refuses to resume validation/forge until the accumulator's `LAST_SLOT == wal_tail`. So the
   only states ever *resumed* are "all four at the WAL tail." A mixed prefix is caught and closed, never
   run on.
4. **A reorg** never inverts: the accumulator resets to its nearest durable checkpoint ≤ the rollback
   point and replays the rolled-back canonical blocks (the `reset_to_bootstrap` + replay pattern), the
   same fold as restart.

**DC-EPOCH-20 (this slice declares it):** *For every selected block admitted to the durable chain, the
ChainDB/WAL record, the Praos chain-dep, the `EpochAccumulator` transition, and the reduced-UTxO-checkpoint
advancement are advanced from the same selected-chain prefix; on any restart or rollback each derived store
is rematerialized from the WAL-tail canonical prefix before validation or forging resumes. No derived
authority is ever resumed at a selected-chain prefix different from the WAL tail.* Enforced by: the
recovery rematerialize fold; the fail-closed readiness gate; a CI guard that the live accumulator advance
sits behind the DC-SYNC-01 durable-admit boundary and that recovery folds `apply_selected_block` over the
canonical prefix.

---

## 4. Your hard rules → concrete mechanism (each binding, each at a named seam)

| # | Hard rule | Mechanism in S2 |
|---|-----------|-----------------|
| A | Not durably admitted unless ChainDB/WAL **and** chain-dep **and** accumulator **and** reduced-checkpoint obligation are durable | WAL tail = the authority; accumulator advance sits *after* the DC-SYNC-01 durable-admit boundary; recovery rematerializes all derived stores to the WAL tail; readiness gate fails closed on any lag (§3). |
| B | Recovery: all four reflect one prefix, or rematerialize from it; never a mixed prefix | The accumulator joins `materialize_rolled_back_state`'s replay fold; readiness gate forbids resuming at a mixed prefix (§3, §6). |
| 1 | `SelectedBlockCtx` derives only from decoded canonical block data + durable selected-chain context | `era`/`block_slot`/`issuer_pool` from the *same* `decode_block` the reducer consumed; `block_epoch` from `era_schedule.locate`; `boundary_mark = None` (S2). No peer/CLI/wall-clock (the `SelectedBlockCtx` doc already forbids them). |
| 2 | Issuer accounting uses the **verified** issuer identity | `issuer_pool = blake2b_224(header.issuer_vkey)` (`block_validity/header_input.rs`) — the identity Praos VRF/KES validation checks, not a peer convenience field. |
| 3 | Fee accounting uses the same canonical block semantics full ledger application would | **PO-1 — DONE (`epoch_accumulator.rs`).** The `invalid_transactions` set (decoded no-UTxO via `decode_is_valid_indices`) gates ALL within-epoch effects: valid tx → declared fee (key 2) + withdrawals; phase-2-invalid tx → `total_collateral` (key 17), body effects discarded. **Fail-closed:** `InvalidTxCollateralNeedsUtxo` (invalid tx, no declared collateral) and `InvalidTxCarriesAuthorityEffect` (invalid tx carrying certs/withdrawals — the discarded-effect skip is S3's gate, never silently applied; the cert guard means `process_block_certificates` never applies an invalid tx's certs). 4 tests. |
| 4 | Withdrawal / certificate processing exactly once per selected block | The accumulator advance runs *after* `pump_block`'s idempotent re-announce no-op (`pump.rs:109`), inside the single admit step; one `process_block_certificates` + one `scan_block_tx_effects` per block. A re-announced block applies nothing. |
| 5 | Within-epoch persistence is canonical + ordered; no map iteration leaks into encoded bytes | The codec is `BTreeMap`-ordered + definite CBOR + the re-encode backstop (S1); the compact delta is encoded by the same canonical writers. `ci_check_epoch_accumulator_no_utxo.sh` already forbids `HashMap`-shaped fields; extend the guard to the delta. |
| 6 | The snapshot-provided `nesBcur` seed is manifest-bound + its epoch identity checked | **PO-3:** the seed reads `block_production` from the bootstrap snapshot and asserts the snapshot's epoch == the accumulator's start epoch (fail-closed `SeedEpochMismatch`); the seed is bound to the same manifest the reduced checkpoint's `seed_slot` is sealed against. |
| 7 | A reorg cannot undo by ad hoc inverse mutation; rematerialize from the last durable checkpoint + canonical blocks | The accumulator has **no** decrement/inverse path; reorg = reset-to-checkpoint + replay (§3.4). The codec exposes no subtractive op. |
| 8 | Deferred POOLREAP / withdrawal-discriminant: fully handled **or** structurally excluded until S3 | **Structurally excluded:** both live only inside `cross_epoch_boundary`, unreachable on the S2 live path (`boundary_mark = None → MissingBoundaryStake`). §2. |

---

## 5. Proof obligations (resolve before coding — IDD: not footnotes)

- **PO-1 (fee semantics) — RESOLVED.** Finding: `phase.rs:199` (`apply_phase_2_failure`) is the *only*
  `epoch_fees` writer in Ade's full ledger, and it implements the **invalid-tx collateral half only**
  (`total_collateral` if declared, else `Σ collateral_inputs − collateral_return`, which needs the UTxO).
  There is no valid-tx fee accumulation in Ade's full-ledger code — the live follower never needed it. So
  the authority is **cardano-ledger's UTXOS rule directly**, not a byte-match against Ade's partial path:
  for tx `i`, `epoch_fees += (i ∈ invalid_transactions ? collateral_consumed : body.fee)`. S1's
  `scan_block_tx_effects` adds `body.fee` (key 2) for *all* txs → wrong for any block with a phase-2-invalid
  tx. **S2 refinement:** decode the block's `invalid_transactions` index set
  (`plutus_eval::decode_is_valid_indices`, no UTxO needed); valid tx → `+= body.fee`; invalid tx →
  `+= total_collateral` (key 17); **fail-closed** (`InvalidTxCollateralNeedsUtxo`) if an invalid tx lacks
  `total_collateral`. The fail-closed case is rare (Conway txs generally declare `total_collateral`) and is
  the correct refusal of a genuinely byte-uncertain value, never a silent wrong sum. (Within-epoch
  accumulation only; the boundary that *consumes* `epoch_fees` is gated to S3.)
  **Deeper finding (implemented):** the `invalid_transactions` set gates ALL within-epoch effects, not just
  fees — cardano discards an invalid tx's certs/withdrawals/mint/outputs too (only collateral is consumed).
  Neither the live reduced-window cert path (`reduced_advance::advance_cert_state`) nor the reused
  `process_block_certificates` skips invalid txs today — a *shared, pre-existing* behavior, so the
  accumulator not skipping is consistent with the existing live path, not a new divergence. Rather than
  silently apply a discarded effect, S2 **fail-closes** on the rare invalid-tx-carrying-certs/withdrawals
  block (`InvalidTxCarriesAuthorityEffect`), deferring the exact skip semantics to S3's byte-exact gate
  (which fixes both paths together against cardano-node). **Landed** in `epoch_accumulator.rs`
  (`scan_block_tx_effects` is validity-aware: `TxScan` per-tx collector + `apply_tx_scan`; the scan runs
  *before* the cert pass so the guard fires first). **Fail-closed decode of the invalid set itself** (IDD
  reviewer HIGH): the authority path uses a new `decode_invalid_tx_indices_canonical` (DEFINITE array of
  canonical-minimal, strictly-ascending uints, each `< tx_count`, no trailing) — NOT the lenient diagnostic
  `plutus_eval::decode_invalid_tx_indices`, which returns an empty/truncated set on malformed CBOR and would
  silently under-report the set (applying a discarded tx's effects to authoritative state). 7 tests; 715 lib
  tests green, clippy + the DC-EPOCH-19 guard clean.
- **PO-2 (persistence cadence + compact delta).** Pin the durable representation: a separate redb table
  beside the reduced checkpoint vs. extending the snapshot frame; a per-block compact delta + periodic
  full checkpoint vs. boundary-only checkpoints; the durable `LAST_SLOT` cursor. Constraint: **no
  full-accumulator clone per block** (cluster §4) and recovery replay bounded to ≤ one epoch of blocks.
- **PO-3 (`nesBcur` seed binding).** Confirm the bootstrap snapshot carries the seed epoch's partial
  `block_production`, how its epoch identity is read, and the manifest it binds to; define the fail-closed
  `SeedEpochMismatch`.
- **PO-4 (recovery fold hook).** Confirm where the accumulator's rematerialize fold attaches:
  inside/alongside `materialize_rolled_back_state`'s replay loop (`rollback/materialize.rs:92`) so it
  lands at the *same* target as the ledger + chain-dep, and the readiness gate's placement on the
  resume/forge path.
- **PO-5 (readiness-gate placement).** Where the fail-closed `accumulator.LAST_SLOT == wal_tail` gate sits
  on startup and before any consumer reads the accumulator (S4 will add the leadership consumer; S2
  installs the gate).
- **PO-6 (observe-only stall vs. follow halt) — from the IDD review of the fee unit.** The fee unit makes
  `apply_within_epoch` HALT on a rare-but-legal Conway block: a phase-2-invalid tx that omits
  `total_collateral`, or one carrying certs/withdrawals (both deferred to S3). Per IDD §8 a halt beats a
  silent mis-apply — but in S2 the accumulator is NOT yet the consensus/leadership authority (S4 flips it),
  so the live wiring must make such a halt **stall the accumulator** (durably record "stalled at slot X,
  reason"; the readiness gate then fail-closes any future authoritative read until S3 resolves it) rather
  than halt the whole follow, which continues via the existing bridge/reduced-checkpoint machinery. This is
  the "structurally excluded until S3" disposition the user's hard rule sanctions, made non-silent. The
  live-wiring unit owns this stall-vs-halt seam.

> **Review note (tracked, not S2's to fix):** `rules.rs:294` (the legacy `apply_*_classified` full-ledger
> path) still populates `BlockApplyResult.invalid_tx_indices` from the fail-open
> `plutus_eval::decode_invalid_tx_indices`. No downstream *authoritative* reader of that field exists today
> (the accumulator is the authority path and now decodes fail-closed), so it is not a live defect — but that
> helper must never be promoted onto an authority path; an authority consumer must use
> `decode_invalid_tx_indices_canonical`.

---

## 6. Persistence + recovery design (the proven pattern, extended)

- **Live advance.** In the admit step, after DC-SYNC-01's durable-admit boundary, fold
  `apply_selected_block(prior, block_bytes, ctx)` with `ctx.boundary_mark = None`; persist the compact
  delta + bump `LAST_SLOT` (PO-2). In-place delta, never a per-block clone.
- **Restart.** `recover_node_state` reconstructs to the WAL tail; the accumulator is rematerialized by
  folding `apply_selected_block` over `(last_accumulator_checkpoint, wal_tail]` (PO-4), then the readiness
  gate asserts `LAST_SLOT == wal_tail` (fail-closed otherwise).
- **Reorg.** Reset to the nearest accumulator checkpoint ≤ rollback slot; replay the rolled-back canonical
  blocks through the same fold. No inverse mutation.

---

## 7. Acceptance (CE — S2 contributes to cluster CE-2; CE-3..CE-6 are S3+)

- [ ] **CE-2a (live within-epoch fold):** every admitted within-epoch block advances cert state +
  `block_production[issuer]` + `epoch_fees` via `apply_selected_block`; hermetic + one live preview
  within-epoch follow.
- [ ] **CE-2b (exactly once):** re-announced / idempotent-no-op blocks apply nothing; one cert + one fee
  scan per admitted block (test).
- [ ] **CE-2c (fee byte-semantics, PO-1):** `epoch_fees` matches full-ledger accumulation incl.
  phase-2-invalid collateral; fail-closed on undeclarable collateral (test).
- [ ] **CE-2d (DC-EPOCH-20 durability/recovery):** restart rematerializes the accumulator to the WAL tail
  byte-identically; a forced lag trips the fail-closed readiness gate; a reorg rematerializes via replay,
  not inverse mutation (tests + the live restart).
- [ ] **CE-2e (boundary structurally excluded):** a boundary-crossing block on the live S2 path
  fail-closes `MissingBoundaryStake` (test) — POOLREAP + the KeyHash projection are never executed live.
- [ ] **CE-2f (`nesBcur` seed, PO-3):** the seed binds epoch-identity; `SeedEpochMismatch` is fail-closed
  (test).
- [ ] **CI:** extend `ci_check_epoch_accumulator_no_utxo.sh` (or a sibling) to assert the live advance is
  behind the durable-admit boundary, recovery folds `apply_selected_block` over the canonical prefix, and
  the delta codec stays canonical/UTxO-free; **DC-EPOCH-20** present in the registry.

## 8. What S2 does NOT do

No boundary crossing (S3). No leadership-authority flip — the accumulator is persisted + recovered but
consensus/leadership still read the seed-anchored re-derive (S4). No live byte-exact-vs-cardano-node gate
(S3). No multi-epoch self-derived proof (S6). The bootstrap bridge + Option-B seeds remain the seed/seed+2
authority until S4 retires them.
