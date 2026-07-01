# S3 — DRep/SPO voting-stake derivation (the InstantStake-equivalent distribution authority)

## Goal

Make the DRep and SPO voting-stake **distributions** that feed `evaluate_ratification` a first-class,
pure, tested, oracle-anchored unit — the "distribution authority." Today the derivation is an untested
inline block inside the epoch-boundary transition, and a parallel dead function drifts beside it.

S3 derives and VERIFIES the distributions; it does NOT thread them into the live ratification gate (that is
S4's deliberate, oracle-verified activation). Import-not-activate holds. See
[[feedback_imported_data_is_not_activation]].

## What exists (surveyed 2026-07-01)

- `rules.rs:1265-1299` (epoch boundary, Conway): the DRep stake is derived inline as `vote_delegations ×
  mark.delegations` (per-credential mark stake summed per `DRep`, positive-stake only); the SPO stake is
  `go.0.pool_stakes` (the GO snapshot). Both are passed to `evaluate_ratification`.
- `governance.rs:47 active_drep_stake_filtered` — filters the DRep distribution by `AlwaysAbstain` +
  expiry (`drep_expiry >= current_epoch`; absent ⇒ active). The live denominator.
- `governance.rs:155 compute_active_drep_stake` — **DEAD** (not called; `evaluate_ratification` uses
  `active_drep_stake_filtered`). Carries a stale epoch-576 comment. The pre-existing owed cleanup.
- In the LIVE path the inputs are EMPTY (S1 import-not-activate: `vote_delegations`/thresholds seeded empty
  in `mithril_native_assembly.rs`), so the derivation runs over nothing and no ratification fires — starved
  by design until S4.

## Gaps S3 closes

1. **The DRep derivation is inline + untested.** Extract it to the pure `governance` function
   `derive_drep_voting_stake(vote_delegations, mark) -> DRepStakeDistribution` so it is shared between the
   live boundary path and the verification path, and directly testable. (The SPO voting stake needs NO
   extraction — it is already the aggregated `go.0.pool_stakes` snapshot field, passed straight to
   `evaluate_ratification`; there is no per-credential loop to lift. Whether `mark`/`go` are the snapshots
   cardano uses for each role is an S6 question.)
2. **No verification of the distribution.** Verify against the real S1-imported inputs: the 58,525
   `vote_delegations` (proven in S1) × a real mark snapshot must yield a deterministic, non-trivial
   per-DRep distribution with the correct structure (per-DRep sums, `AlwaysAbstain` excluded from the
   *active* denominator, positive-stake only, replay-identical).
3. **Dead `compute_active_drep_stake`.** Remove it (the single active-stake filter is
   `active_drep_stake_filtered`).

## Deferred (explicitly NOT S3)

- **The byte-exact InstantStake oracle match → S6.** Whether `vote_delegations × mark` equals cardano's
  DRepPulser `psDRepDistr` (the InstantStake-equivalent, post-`applyRUpd`) is the byte-exact differential,
  and it belongs to S6's oracle gate. S3 records the two known open questions for S6: (a) the
  mark-vs-InstantStake basis (does the mark stake already fold rewards, matching InstantStake, or is there a
  residual?); (b) the DRep-uses-`mark` / SPO-uses-`go` asymmetry at `rules.rs:1268` vs `:1291` — confirm
  each matches the snapshot cardano uses for that role.
- **`num_dormant_epochs` offset + live-gate activation + SPO non-monotonicity sequencing → S4.**
  `active_drep_stake_filtered` still lacks the `drepExpiry + numDormant >= currentEpoch` offset (S1 captured
  `num_dormant`); applying it, and threading any imported authority into the live gate, is S4.

## Invariants (registry candidates)

- The DRep voting-stake derivation is a **pure, deterministic** function of `(vote_delegations, mark
  snapshot)` — no I/O, no wall-clock, ordered containers, replay-identical. (BLUE)
- Only credentials with **positive** mark stake contribute; a DRep's voting stake is the exact sum of its
  delegators' mark stake. Absent delegator ⇒ 0 (never a default/guess).
- The derivation is **not** threaded into the live ratification gate in S3 (import-not-activate).
- `active_drep_stake_filtered` remains the single active-DRep denominator (no second, drifting filter).

## Acceptance (CE)

- `derive_drep_voting_stake` is a pure `governance` fn; `rules.rs` calls it (no behavior change on the live,
  empty-input path — byte-identical boundary output). The SPO voting stake stays the raw `go.0.pool_stakes`
  snapshot (no extraction — nothing to lift).
- Unit + real-data tests: constructed vote-map cases + the 58,525 real `vote_delegations` × a real mark
  snapshot → a deterministic distribution; `AlwaysAbstain`/expiry/positive-stake handling asserted; replay
  byte-identical.
- `compute_active_drep_stake` removed; `cargo test -p ade_ledger` green; no live-gate activation.
