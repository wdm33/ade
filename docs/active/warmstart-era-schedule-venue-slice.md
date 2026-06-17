# WARMSTART-ERA-SCHEDULE-VENUE (slice)

**Status:** code + hermetic tests enforced (DC-CINPUT-05); live proof = the
C2-PREVIEW-BA02 forge resume (fresh admission → warm-start replay).
**Date:** 2026-06-17
**TCB color:** BLUE-adjacent durable-recovery authority (the sidecar codec +
the era-schedule reconstruction are deterministic; `ChainDb`/genesis I/O is the
RED shell).

## The bug (live finding)

The DURABLE-ADMISSION-BYTES forge resume got **past** the block-bytes stage
(that fix proven) and reached **forward-replay of the followed preview block at
slot 115030409**, failing:

```
Materialize(ReplayFailedAt { slot: 115030409,
  Header(HFC(SlotBeforeSystemStart { first_era_start: 574992000 })) })
```

`574992000 = 1331 × 432000`. The warm-start era-schedule hardcoded the
**preprod** epoch length (432000) — `make_node_schedule` (`epoch_length_slots:
432_000`) + recomputed the era start as `epoch_no * 432_000` — so on **preview**
(epoch length 86400) it placed epoch 1331's start at 574992000, *after* the
block. The admission path was already venue-correct (it built its schedule from
the canonical bundle's `epoch_start_slot`); only the **warm-start** path
re-derived it wrongly. Does not affect preprod (where 432000 is correct).

## The invariant (DC-CINPUT-05)

**Venue epoch geometry is durable replay authority.** A recovered store MUST
replay using the geometry persisted with the seed/import that created it —
never re-derived from whatever genesis/CLI a restart supplies.

- true: same durable store + same WAL + same checkpoint ⇒ same replay behavior.
- derived: the venue epoch length/start must match the venue (preview 86400,
  preprod 432000, …) — no venue-name switch, no hidden default, no 432000
  fallback.
- release: the BA02 preview forge WarmStart must replay preview-followed blocks.
- operational: operators must not "repair" a store by passing a different
  genesis at restart.

## The fix

1. `SeedEpochConsensusInputs` (sidecar) gains `epoch_start_slot` +
   `epoch_length_slots`; schema **v2 → v3**, so old v2 sidecars **fail closed**
   at decode (`UnknownVersion`) — a store must be re-seeded, never silently
   re-geometried.
2. `merge_seed_epoch_consensus_inputs` persists the geometry from
   `canonical.epoch_length_slots()` (= `epoch_end_slot − epoch_start_slot + 1`),
   fail-closed `InvalidEpochWindow` on a degenerate window.
3. `make_node_schedule` takes `epoch_length_slots` **explicitly** — no hardcoded
   432000. `warm_start_recovery`, `recovered_node_schedule` (the live-follow /
   forge arms), and the import-window caller all source the geometry from the
   sidecar / canonical.
4. A restart `--genesis-file` is **only** a consistency check
   (`assert_restart_genesis_matches_sidecar` →
   `RestartGenesisGeometryMismatch` fail-closed on disagreement); never a
   re-derivation source.

HARD: no `if preview then 86400 else 432000`, no implicit default, no hidden
432000 fallback.

## Enforcement

- **Tests:**
  - `warm_start_schedule_locates_block_by_venue_geometry_not_hardcoded_432000` —
    preview 86400 **locates** slot 115030409; the wrong preprod-length geometry
    **rejects** it `SlotBeforeSystemStart@574992000` (the exact live failure);
    preprod 432000 also locates. (The prior warm-start tests all used
    snapshot-at-tip *degenerate* replay and never called `locate()` — which is
    why they missed the bug.)
  - `merge_persists_venue_epoch_geometry_preview_and_preprod` — both venues +
    degenerate-window fail-closed.
  - `restart_genesis_epoch_length_mismatch_fails_closed` — sidecar is authority;
    a different-venue genesis fails closed; absent/field-less genesis ⇒ no check.
  - codec: `seed_epoch_consensus_inputs_round_trips_byte_identical`,
    `seed_cinput_decode_rejects_unknown_version` (v1 **and** v2 → fail closed).
- 68 test binaries green across `ade_ledger` + `ade_runtime` + `ade_node`.

## Open obligation (live proof)

A fresh admission with this binary writes a **v3** sidecar carrying the preview
geometry (86400); the warm-start forge then builds the 86400 schedule and
forward-replay **accepts** the followed block. That is the C2-PREVIEW-BA02 forge
resume — the same run that previously failed `SlotBeforeSystemStart`.
