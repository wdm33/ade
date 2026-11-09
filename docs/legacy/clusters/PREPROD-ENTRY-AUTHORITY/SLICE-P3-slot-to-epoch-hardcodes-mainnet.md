# SLICE P3 — `slot_to_epoch` hardcodes MAINNET constants; preprod's ledger runs 194 epochs ahead

> **SEALED — ROOT CAUSE PROVEN, NOT FIXED.** Found 2026-08-03 while diagnosing P2. This is
> **consensus-critical**: it makes a preprod node's ledger believe it is in epoch 498 when the chain
> is at 304. The fix touches BLUE epoch geometry and must not be written against a forge-window
> clock.

## The defect

`crates/ade_ledger/src/state.rs:312`:

```rust
pub const SHELLEY_START_SLOT:   u64 = 4_492_800;   // MAINNET
pub const SHELLEY_START_EPOCH:  u64 = 208;         // MAINNET
pub const SHELLEY_EPOCH_LENGTH: u64 = 432_000;
/// These are fixed by the Shelley genesis and do not change.

pub fn slot_to_epoch(slot: SlotNo) -> Option<EpochNo> {
    let offset = slot.0 - SHELLEY_START_SLOT;
    Some(EpochNo(SHELLEY_START_EPOCH + offset / SHELLEY_EPOCH_LENGTH))
}
```

That doc comment is true **for mainnet** and false across networks. `detect_epoch_transition` — the
trigger for **every** ledger epoch-boundary application — calls it, so the ledger's boundary
detection **never consults the era schedule**, which is correct and venue-bound.

## Proven, not inferred

Instrumented at the capture refusal (emit-only, no behaviour change):

```
capture-refused-geometry: schedule_locate_epoch=Some(304)   <- era schedule: CORRECT
capture-refused:  tip_slot=130046891 ledger_epoch=498 track_utxo=false
                  cert=REDUCED gov=REDUCED snapshots=REDUCED
```

The mainnet formula **predicts 498 exactly**:
`208 + (130,046,891 − 4,492,800) / 432,000 = 208 + 290 = 498`.

| venue | ade computes | real | `new_epoch > current_epoch`? |
|---|---|---|---|
| **preprod** | **498** | 304 | **TRUE** → spurious boundary fires |
| preview | 473 | 1378 | FALSE → never fires, bug is **silent** |

**Preview working was luck, not correctness.** Its fake epoch sits *below* its real one, so the
comparison is never true and no boundary is ever detected on the preview follow path. Preprod's fake
epoch sits *above*, so it fires.

It fires **once**: the epoch jumps 304 → 498 in a single application, after which `498 > 498` is
false. That one application routes through `dispatch_epoch_boundary` → `apply_reduced_epoch_boundary`
(the node runs `track_utxo=false`), which sets `cert_state`, `gov_state` and `epoch_state.snapshots`
to `ReducedUnavailable` — exactly the three REDUCED flags observed. P2's capture refusal is the
**messenger**, not the defect: the RVBP guard fail-closed correctly on a genuinely corrupt state.

(An earlier estimate of "~194 spurious boundaries" from dividing catch-up slots by the divergence was
wrong — it is one jump, not many.)

## Why this is worse than a failed capture

A preprod node's ledger believing it is 194 epochs in the future would compute **eta0 and the stake
distribution for the wrong epoch**, so leader checks are meaningless. Had the RVBP guard not failed
closed, a forge attempt would have proceeded against fabricated epoch geometry.

The accumulator is unaffected — it uses the real `EraSchedule` and logged **zero** crossings, which is
why the divergence was invisible in the logs until instrumented.

## The live path — PROVEN by instrumentation, after static tracing got it wrong

A temporary probe inside `detect_epoch_transition` caught it firing **exactly once**, on the first
block after bootstrap:

```
P3-detect-epoch-transition: slot=129813444 current_epoch=304 -> new_epoch=498 (MAINNET formula)
```

`current_epoch=304` is the correctly-seeded value; the mainnet formula returns 498; `498 > 304`
declares a boundary; `apply_reduced_epoch_boundary` runs once and the projections stay
`ReducedUnavailable` forever after, because `498 > 498` is false.

**The full live chain** — note where the venue geometry is lost:

```
receive_apply(era_schedule)            <- correct EraSchedule in scope
  -> block_delivered(era_schedule)
  -> admit_via_block_validity(era_schedule)
  -> block_validity/transition.rs:132  -> apply_block_with_verdicts(ledger, era, inner)
                                          ^^^ era_schedule DROPPED HERE
  -> apply_shelley_era_block_with_verdicts
  -> detect_epoch_transition -> slot_to_epoch   [MAINNET CONSTANTS]
```

The schedule is threaded correctly all the way to `admit_via_block_validity` and then **dropped at
the last hop**, which is exactly why a hardcoded fallback exists there at all.

**Method note, recorded because it recurred:** static tracing concluded the callers were
"test-only" — `apply_block_with_verdicts` had one reference in `runtime`/`node` and it was a comment,
`apply_block_with_accounting` only tests. That was wrong: the call comes from **inside `ade_ledger`
itself** (`block_validity/transition.rs`), which a grep scoped to `ade_runtime`/`ade_node` cannot
see. The probe was right and the code-reading was wrong — the third time in this session that
instrumenting beat inference.

## Fix direction (NOT selected — this is the open work)

`detect_epoch_transition` must derive the epoch from the **venue's `EraSchedule`**, the same authority
`schedule_locate_epoch` used to report 304 correctly, rather than from mainnet constants. Open
questions before implementing:

1. `slot_to_epoch` is a free function with no schedule in scope — threading the schedule through
   `apply_block_with_boundary` is an API change across the ledger apply path.
2. Who else calls `slot_to_epoch` / these constants, and are any of them load-bearing on mainnet
   semantics that must not change?
3. Whether the fix should instead make the constants unrepresentable (venue-bound type) so a
   hardcoded-mainnet path cannot be reintroduced — the `compiler-authority` preference.

Explicitly NOT acceptable: adjusting the constants to preprod's values (breaks mainnet/preview),
or suppressing the boundary when the projections would go reduced (hides a wrong epoch).

## Impact

- **P2 is fully explained** and is not an independent defect.
- **LIVE-2 on preprod is blocked** behind this, not behind P1 or P2.
- **Any non-mainnet venue where the mainnet formula yields an epoch ABOVE the real one is affected.**
  Preview is currently safe by numeric accident; that accident is not a property anyone chose, and it
  will stop holding if preview's real epoch ever falls below its computed one.
- Operator prerequisites remain verified and unaffected (opcert to 2026-09-15, KES vkey proven equal,
  stake fully activated, peer synced, P1 fix working, chunk-6009 snapshot bootstraps).

## Not claimed

No fix, no invariant registered, no CE. The instrumentation that found it (`capture-refused` /
`capture-refused-geometry`) is emit-only and annotates an error already being returned.
