# S4-0 — Epoch-Indexed Leadership Authority (the bridge before the flip)

> **Status: OPEN.** The necessary bridge between "the right authority EXISTS" (S4-pre-1/2) and "production
> reads the right authority" (S4 proper). S4 proper (the three-call-site swap) is BLOCKED until S4-0 proves
> epoch-indexed reads. This is the S4 OPENING MOVE — do it before swapping any production read site.

## The problem S4-0 fixes (the reader-timing gap)

S4-pre-2's boundary freeze produces `nesPd_{E+1}` at the cross INTO epoch `E` (target+1, MARK-based — proven:
boundary 1340→1341 → `target_leadership_epoch=1342`). So while the node operates in epoch `E`, a boundary may
already have sealed `nesPd_{E+1}` (or `nesPd_{E+2}`). The production leader schedule for epoch `E` needs
`nesPd_E` — NOT the latest/current sealed object. "Current means usable" is WRONG.

There is also a bootstrap gap: the cross into 1339 seals `nesPd_1340`, so **`nesPd_1339` is never natively
produced**. It must come from a bootstrap-certified initial condition (the seed-window frozen leadership,
imported as an explicit epoch-indexed authority) — NOT a fallback.

## The contract

```
leadership_authority_for_epoch(epoch) -> FrozenLeadershipPoolDistr
```
returns the object whose `target_leadership_epoch == epoch`, and NOTHING else. Fail closed (typed) if the epoch
is missing, wrong-epoch, malformed, or uncertified. No "current/latest means usable" shortcut.

## Store shape — epoch-indexed two-key model

Generalize the two-key model (S4-pre-2) to per-epoch:
- `bootstrap_frozen_leadership_by_epoch` — the immutable bootstrap-certified initial condition (the seed-window
  leadership epochs imported at bootstrap).
- `current_frozen_leadership_by_epoch` — the live authority (bootstrap epochs + native boundary freezes).

Transitions:
- **bootstrap**: seed ALL manifest-bound leadership epochs available from the seed record/window into BOTH
  tables, keyed by `target_leadership_epoch`.
- **boundary freeze**: insert the `FrozenLeadershipPoolDistr` keyed by its `target_leadership_epoch` (into
  current). Atomically with the accumulator advance (S4-pre-2's `advance_with_current_leadership`, now writing
  the epoch-keyed entry).
- **reset_to_bootstrap**: `current_by_epoch := bootstrap_by_epoch` (drop native freezes, restore the
  bootstrap-certified epochs; the refold re-produces the native freezes — replay-equivalent).

## Bootstrap-certified initial condition (NOT a fallback)

If the seed artifact contains only ONE epoch (1338), that is INSUFFICIENT: `nesPd_1339` is a gap (native starts
at 1340). S4-0's first obligation:

> For every production leadership read from bootstrap until the first native freeze becomes usable, does the
> store contain `leadership_authority_for_epoch(read_epoch)`?

If not, import the seed-window frozen leadership objects (whatever the seed record/state can produce for the
bootstrap epochs — e.g. `nesPd_1338` from the set snapshot, `nesPd_1339` from the mark snapshot) as explicit
epoch-indexed frozen leadership authorities. This is a bootstrap-certified initial condition, not a fallback.
The EXACT set of bootstrap epochs is TEST-DISCOVERED, not assumed.

## Acceptance (mapping test-discovered, printed explicitly)

Prove epoch-indexed reads across 1338→1340 (+ 1341/1342 native, already proven by S4-pre-2):
- read epoch 1338 → frozen object `target_epoch == 1338`;
- read epoch 1339 → frozen object `target_epoch == 1339` (bootstrap/import if needed);
- read epoch 1340 → frozen object `target_epoch == 1340` (native-frozen if available, else bootstrap/import);
- read epoch 1341 / 1342 → native-frozen (S4-pre-2);
- every read is EXACT (`requested epoch == frozen target_leadership_epoch`); a missing/wrong/malformed/uncertified
  read fails closed (typed).

## Hard prohibition

Do NOT resolve the timing with: the latest leadership object; the current leadership object; "if missing then
seed"; "if epoch mismatch then use nearest"; a go/set fallback; an active-params fallback. The production read
is EXACT: `requested epoch == frozen target_leadership_epoch`.

## Then S4 proper (after epoch-indexed reads are proven)

1. Replace the 3 production seed-window reads (`from_seed_epoch_consensus_inputs`) with
   `leadership_authority_for_epoch(slot_epoch)`.
2. Delete the seed+2 ceiling (`epoch_wire.rs`).
3. Add the seed-authority-resurrection guard (no production seed-window authority reads).
4. Prove the former ceiling is crossed with accumulator frozen leadership only.

## Bootstrap-certified initial condition (RESOLVED — agent-mapped)

The pre-S4 seed window serves EXACTLY seed..seed+2 (the ceiling at `epoch_wire.rs:624`,
`prepare_authority_for_candidate_slot`), each from a DIFFERENT source — the seed `pool_distribution` is NEVER
reused verbatim:

| epoch | leadership source | object |
|---|---|---|
| 1338 (seed) | imported seed `nesPd` (SET-derived) | `SeedEpochConsensusInputs.pool_distribution` — already an epoch-indexed frozen object via `from_seed_epoch_consensus_inputs` |
| 1339 (seed+1) | imported MARK snapshot `calculatePoolDistr(ssStakeMark)` | `s1a.mark_pool_distr` → currently ONLY the seed+1 bridge (`BridgeSourceKind::ImportedMarkSnapshot`, `native_firstrun.rs:525-577`) — MUST be imported as `nesPd_1339` (no native-freeze source) |
| 1340 (seed+2) | native boundary freeze (cross into 1339 → `nesPd_1340`) OR the seed+2 window-replay — both = the same SET-derived `nesPd_1340` | native provides it in time (the first native/window overlap) |
| 1341+ | native boundary freeze | proven (S4-pre-2: 1342 == POST-1342 nesPd) |

So the bootstrap seeds **`nesPd_1338`** (seed record) + **`nesPd_1339`** (`mark_pool_distr`, available at bootstrap
in `native_firstrun.rs` beside the bridge build); native freezes cover 1340+. There is NO gap: by the time the
node validates a slot in epoch E, `nesPd_E` is either bootstrap-seeded (1338/1339) or native-frozen (1340+, from
the cross into E-1). The read for epoch 1339 does NOT come from the cross into 1339 (that freezes `nesPd_1340`).

**Note (1339 fidelity):** `mark_pool_distr` is what the seed window ALREADY serves for 1339 (the bridge) — S4-0
imports it verbatim as the bootstrap-certified initial condition (matching current behavior; no POST-1339
reference exists to compare, and any lossiness is a pre-existing bridge property, not introduced here). The
native reference-proven path begins at 1340.

## Production read sites (RESOLVED — agent-mapped)

Three sites, all `crates/ade_node/src/node_lifecycle.rs` (658 forge-OFF header-validate view, 840 forge-ON
view, 3397 warm-start recovery). All build a `PoolDistrView` for the SEED epoch (`record.epoch_no`); the
`PoolDistrView.epoch` acts as a fail-closed GUARD in the `LedgerView` impl (`pool_active_stake` returns `None`
if `queried_epoch != self.epoch`). The per-slot epoch is derived by the CONSUMER (`EraSchedule::locate`), and
cross-epoch continuity is a SEPARATE mechanism — `ActiveEpochAuthority::{seed,continuity}` + the promoted N+1
view (`epoch_activation.rs`, `epoch_wire.rs` bridge/window-replay). S4-0 is the store + reader ONLY; wiring
these sites + `ActiveEpochAuthority` to `leadership_authority_for_epoch` is S4 proper.
