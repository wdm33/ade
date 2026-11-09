# CE3D-REWARD-ACCOUNT-EVOLUTION-CORRECTION — invariants sketch

The go-stake residual is a boundary snapshot-ordering defect (see `S0-DIAGNOSIS.md`): the mark snapshot is built
from PRE-reward-update balances. S0 is GREEN diagnosis. The BLUE corrective slice reorders the boundary so the
mark reflects post-RUPD rewards, and is governed by these invariants.

## What must ALWAYS be true

- **I-RAE-1 (post-RUPD mark).** The epoch-boundary mark snapshot is built from the reward accounts AFTER the
  boundary reward-update is applied — the same order cardano's `SNAP`/`applyRUpd` uses. The mark's per-credential
  stake is `base_utxo(cred) + post_RUPD_reward(cred)`.
- **I-RAE-2 (one atomic boundary).** The reorder happens inside the single boundary authority
  (`cross_epoch_boundary` / `apply_epoch_boundary_with_registrations`) — RUPD, mark, rotation, POOLREAP, and
  governance stay one atomic, deterministic transition. No split effects, no second pass.
- **I-RAE-3 (byte-exact reward accounts).** After the fix, all affected delegated reward-account values match the
  oracle; the mark, set, and go per-credential maps match cardano at the target boundaries; the aggregate go
  residual is EXACTLY zero.
- **I-RAE-4 (preserved authority).** Reward pots (treasury/reserves), RUPD consumption (including the one-shot
  bootstrap RUPD), POOLREAP, and the five CPDE governance refunds remain byte-exact — the reorder touches only
  WHEN the mark reads rewards, not the reward/pot arithmetic.

## What must NEVER be possible

- **N-RAE-1 (no offset / no compensating adjustment).** No constant, per-credential, or aggregate offset added
  anywhere. `build_boundary_mark_snapshot`'s arithmetic is correct and unchanged; only its input timing moves.
- **N-RAE-2 (no aggregate-only match).** The fix must match at the CREDENTIAL level (all 55,820), never merely the
  total.
- **N-RAE-3 (no external stake/reward dump as live authority).** The cardano reference stays the differential
  target only.
- **N-RAE-4 (no CPDE entanglement).** The −500B undelegated governance-refund gap stays a disjoint regression
  assertion; this slice does not touch governance-refund routing.
- **N-RAE-5 (no scope bleed).** No `MissingDRepActivityParam` work; no S6; no base-UTxO change (exonerated).

## What must remain identical across executions (replay / restart)

- **R-RAE-1.** Replay, warm-restart, and the accumulator-vs-direct-boundary paths remain byte-identical after the
  fix (the boundary is a pure deterministic transition). The full CE-3d differential is re-run and must reach
  byte-exact reward updates + mark/set/go snapshots.

## Acceptance (the corrective slice is not done until)

All 55,820 affected reward-account values match the oracle; mark/set/go per-credential maps match at the target
boundaries; the aggregate residual is EXACTLY zero (not "close"); reward pots, RUPD consumption, and the five
governance refunds remain correct; replay, restart, and accumulator/direct-boundary paths are byte-identical;
then the full CE-3d differential is re-run green. Critical path: CE-3d must have byte-exact reward updates +
mark/set/go before Ade can replace seed-anchored authority and prove continuous self-derived epoch operation.

## Tiering

- **True**: deterministic replay; one atomic boundary transition; one authoritative snapshot result.
- **Derived**: cardano reward/snapshot parity (the differential target).
- **Release**: the S0 elimination-trail fixtures + the post-fix CE-3d byte-exact differential.
- **Operational**: local ChainDB corpus + the re-bootstrapped seed copies (isolated, single-process).
