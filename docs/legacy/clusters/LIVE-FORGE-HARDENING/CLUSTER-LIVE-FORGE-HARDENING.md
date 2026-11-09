# Cluster: LIVE-FORGE-HARDENING

> Close the two gaps that block **sustained** live forging, both surfaced by the 2026-07-31 live
> preview forge attempt. Reuse already-enforced machinery; touch **no BLUE authority**.

## Motivation (what the live attempt proved and what it exposed)

The 2026-07-31 preview forge attempt (epoch 1375 delegation window) proved the forge *machinery*:
Ade did its **first-ever live epoch-boundary crossing** (1374→1375) and computed `eta0(1375)`
**byte-identical to cardano-node's `epochNonce`** — so its leader schedule provably matched
cardano-cli's, and it held the live tip forge-ON. But it could not *sustain* the tip:

- **G1 (the blocker).** After ~14 min at the tip, a routine live-chain fork arrived and the
  `--mode node` forge path failed closed: `relay run-loop sync step failed (UnexpectedRollback)`
  (`crates/ade_node/src/node_sync.rs:640`, the `_ => Err(UnexpectedRollback)` arm). The forge path
  admits the volatile tip durably and refuses every rollback except the recovered-anchor rewind.
  A warm-start restart could not rejoin (durable tip orphaned).
- **G2.** A warm-start-from-a-stale-store that crosses an epoch boundary fails closed on
  `DC-EPOCH-16 epoch-tick eta0 … != bridge eta0 …` — its candidate-nonce reconstruction over-tracks.
  (The **fresh-mithril-bootstrap** forge path is unaffected — it loads the frozen nonce from the
  snapshot and computed `eta0` correctly on 2026-07-31. So G2 is warm-start *resilience*, not the
  primary forge path.)

Neither gap is new consensus code. The rollback-follow machinery is **already ENFORCED**
(DC-NODE-23..29 — venue-blind detector, live durable rollback apply, `WalEntry::RollBack`
replay-equivalence, canonical rollback-target slot/hash binding, no-forge-across-pending-reselection)
— but only in the **participant** path (`run_participant_sync`). The forge path just refuses to use it.

## Invariants (this cluster)

- **INV-FH-1 — the forge path follows legal live rollbacks.** On a peer `RollBackward` to a block
  that is in the durable store, within `k` (=SecurityParam), and on the canonical chain, the forge
  path (`run_node_sync`) applies it through the **same** machinery as the participant path
  (`materialize_rolled_back_state` → `commit_rollback` → append `WalEntry::RollBack`), producing a
  **byte-identical** `WalEntry::RollBack` and preserving replay-equivalence. Reuses DC-NODE-23..29,
  DC-NODE-29 (canonical target = stored slot+hash), DC-NODE-33 (recovered-anchor no-op). No new BLUE.
- **INV-FH-2 — fail-closed only on genuinely-illegal rollbacks.** `Point::Origin`, unknown hash,
  peer-slot ≠ stored-slot, deeper than `k`, or below the bootstrap/seed anchor all keep their exact
  typed halts (`UnexpectedRollback` / `RollbackPointSlotMismatch` / `Pump(ExceededRollback…)`). The
  fail-closed posture is strictly *widened correctly* — only the routine in-store within-`k` fork
  flips from a halt to a durable follow.
- **INV-FH-3 — no forge across a pending rollback (DC-NODE-27/28).** The venue-agnostic `ForgeTick`
  fence (`pending_reselection_forge_refusal`) already gates forging; the forge path sets/clears the
  fence synchronously around the rollback apply, so it never forges on an un-reconciled tip.
- **INV-FH-4 — within-epoch guard (first cut).** The forge path follows only rollbacks whose target
  is at/after the current in-memory promoted authority's epoch-start slot. A rollback that would
  un-cross an epoch boundary against an already-promoted `ActiveEpochAuthority` fails closed;
  cross-boundary authority-rewind is an explicit deferred follow-up. Durable state is protected
  regardless by the BLUE admission/materialize guards.
- **INV-FH-5 — warm-start nonce identity (S2).** Warm-start reconstruction freezes the candidate
  nonce identically to the live fold / fresh-bootstrap, so a warm-start that crosses an epoch
  boundary computes an `eta0` byte-matching cardano-node's `epochNonce`; `DC-EPOCH-16` no longer
  fires on a *legitimate* warm-start. _(Fix shape pending S2 research; candidates: persist RSW/
  securityParam in the sidecar v5→v6; always thread the venue RSW so the freeze slot is never INERT;
  or persist the frozen candidate nonce itself.)_

## Slices

- **S1 — rollback-following in the forge path (priority; the actual forge blocker).**
  Extract the participant `RollBack` arm into a shared `pub(crate) resolve_and_apply_peer_rollback`
  helper (in `node_lifecycle.rs`); collapse `run_participant_sync`'s arm to one call; replace
  `run_node_sync`'s `_ => UnexpectedRollback` (`node_sync.rs:640`) with a call to it; thread
  `security_param: SecurityParam` + `pending_reselection: Option<&mut bool>` into `run_node_sync`
  and its two callers; add the INV-FH-4 within-epoch guard. **Shell-only** (`ade_node` GREEN/RED);
  **no BLUE edits**. See `SLICE-S1-forge-path-rollback-follow.md`.
- **S2 — warm-start candidate-nonce identity (close DC-EPOCH-16 for legitimate warm-starts).**
  Make warm-start reconstruction compute the same frozen candidate nonce as the live fold.
  _Slice doc + exact fix pending S2 research._

## Mechanical acceptance criteria (CE)

- **CE-FH-1** — forge-path rollback tests (mirroring the 4 participant tests in
  `crates/ade_node/tests/live_fork_choice_ai_s4bii.rs`): (a) rollback to a stored within-`k` block
  applies durably — durable tip moves back, `WalEntry::RollBack` written, `pending` cleared;
  (b) unknown-hash target → `UnexpectedRollback`, zero mutation; (c) no snapshot / beyond-`k` →
  `Pump(_)`, `pending` cleared; (d) peer-slot ≠ stored-slot → `RollbackPointSlotMismatch` before any
  mutation.
- **CE-FH-2** — replay-equivalence: the forge path emits the identical `WalEntry::RollBack` via the
  identical `apply_chain_event`; a forge-path replay assertion (mirroring `reselection_replay_s5` /
  `apply_driver_ai_s3`) reproduces the rolled-back authority byte-identically.
- **CE-FH-3** — `cargo test --workspace` green (no regression; the participant path still passes its
  existing rollback-follow tests via the shared helper).
- **CE-FH-4 (S2)** — a warm-start that crosses an epoch boundary computes `eta0` byte-matching the
  fresh-bootstrap / cardano-node value; the `DC-EPOCH-16` guard passes for a legitimate warm-start
  (and still fails closed on a genuinely-inconsistent one).
- **CE-FH-5 (live, operator-gated, deferred)** — a sustained forge-capable live follow (sleep
  disabled) survives ≥1 real preview rollback without an `UnexpectedRollback` halt, holding the tip
  forge-ON across it.

## TCB colors / blast radius

All changes are confined to the **`ade_node` GREEN/RED shell** drivers (`node_sync.rs`,
`node_lifecycle.rs`). The BLUE rollback authority (`materialize_rolled_back_state`, `commit_rollback`,
`admit_rollback` in `ade_ledger`) is **called unchanged**. No BLUE edit; no new `WalEntry` variant;
no new replay logic. Fail-closed posture strictly widened-correct.

## Registry

Reuses (does not redefine) DC-NODE-23..29, DC-NODE-33, DC-CONS-20, T-REC-05. S1 strengthens the
forge-path applicability of DC-NODE-27/28 (the fence is now exercised by the single-producer loop).
New rule candidate on close: **DC-NODE-3x — the `--mode node` forge path follows a legal within-`k`
within-epoch live rollback through the shared `resolve_and_apply_peer_rollback` authority** (append
on CE-FH-1..3 green). S2 candidate: **DC-EPOCH-1x — warm-start candidate-nonce reconstruction is
identity-equal to the live fold** (append on CE-FH-4 green).
