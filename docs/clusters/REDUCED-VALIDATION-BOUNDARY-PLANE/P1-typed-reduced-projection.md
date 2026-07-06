# P1 — typed reduced boundary projection

Introduce the type split and remove the reward-only mark, so no fabricated snapshot exists or is persisted
anywhere. Independently safe: after P1, a `track_utxo=false` Conway boundary emits no mark/set/go, and nothing
reward-only reaches persistence or fingerprints. (No authoritative-mark-correction here — that is a later slice.)

## New types (ade_ledger)

```rust
/// The structural block-production-window rollover recorded by a reduced boundary. Distinct from the full
/// nesBcur/reward-calculation input — it CANNOT be converted into one without an explicit FullBoundaryStateRequired.
pub struct ReducedEpochProgress { pub epoch: EpochNo, pub slot: SlotNo, pub reduced_block_window: BlockWindowRolled }

/// A reduced-plane boundary result: epoch/slot + ReducedEpochProgress ONLY. No rewards/pots/snapshots/gov. A
/// DISTINCT type — not a LedgerState, not widenable, not serializable as an accumulator snapshot.
pub struct ReducedBoundaryProjection { pub progress: ReducedEpochProgress /* + the reduced cert/index projection if a later structural check needs one, named, never a full lifecycle claim */ }

/// The authoritative boundary result (= the S1 full path). Base-required; carries the full LedgerState with
/// post-RUPD mark/set/go, pots, gov. (Wraps the existing (LedgerState, EpochBoundaryAccounting).)
pub struct FullEpochBoundaryResult { pub ledger: LedgerState, pub accounting: EpochBoundaryAccounting }

/// Verdict capability. Structural can never be promoted to Full.
pub enum LedgerBoundaryVerdict { Full(FullEpochBoundaryResult), Reduced(ReducedBoundaryProjection) }

/// Terminal: a caller needs a full verdict/state after a reduced boundary.
// GovernanceTerminal::FullBoundaryStateRequired { boundary_point } (or a dedicated boundary-terminal enum).
```

## Changes

1. **Reduced boundary transition.** `reduced_boundary_projection(epoch_before, new_epoch, slot) -> ReducedBoundaryProjection`
   — advances epoch/slot + records `ReducedEpochProgress` (block-window rolled). Touches no rewards/pots/
   snapshots/gov/POOLREAP. Pure.
2. **Fail-closed capability at the full boundary.** `apply_epoch_boundary_with_registrations` requires
   `EpochStakeSnapshots::Authoritative` and returns `FullBoundaryStateRequired` if ever handed a reduced
   projection — it can never silently read a fabricated snapshot.
2b. **Reduced routing (the seal).** `apply_reduced_epoch_boundary` is the reduced-plane boundary transition
   (`ReducedUnavailable` snapshots + cert/gov reset + window rollover), and `apply_block_with_accounting` routes
   EVERY `track_utxo=false` Conway boundary to it. So a reduced boundary emits NO mark on the reduced path —
   there is no reachable reward-only Conway mark in this commit. (The `Authoritative`/full path's pre-RUPD mark
   is real authority, corrected to post-RUPD in the next commit; it was never a *reduced* fake snapshot.) This
   routing was originally sequenced into P3; it is pulled into this first safety-bearing commit so the commit is
   sealed — no fake snapshot reachable — rather than green-but-unsafe.
3. **Persistence/fingerprint distinction.** The recovery-checkpoint encoder and the WAL fingerprint gain an
   explicit reduced-vs-full discriminant, so a reduced projection is never encoded in the authoritative
   accumulator-snapshot format and its fingerprint is distinguishable from full authority state.

## P1 acceptance (sealed activation)

- A `track_utxo=false` Conway boundary emits NO mark/set/go and no advanced cert/gov lifecycle
  (`reduced_epoch_boundary_produces_no_mark_or_cert_lifecycle`); the block-apply path routes it to the reduced
  transition, never the full boundary fn.
- A reduced projection reaching the full boundary fn fails closed with `FullBoundaryStateRequired` (not an empty
  mark).
- `ReducedEpochProgress` cannot be converted to the full reward input (no `From`/`Into`; the type carries no
  reward fields).
- Persistence + fingerprint encode a reduced projection distinguishably from full authority (a round-trip test).
- ade_ledger + ade_node compile; the reduced-plane no longer terminals at boundaries via a fabricated mark (the
  dispatch that routes reduced→projection is P2, so P1 keeps the boundary reachable via the reduced transition
  where the reduced plane calls it, without emitting a fake mark).

## Not in P1

The dispatch/capability-gating of consumers (P2); the authoritative post-RUPD mark correction (separate cluster
slice); recovery/fork-switch/CE-3d proofs (P3 + rerun).
