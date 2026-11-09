# SLICE BND-1 — a boundary stall and an apply failure stop being the same state

> **DOC BEFORE IMPL.** Entry evidence: `docs/evidence/run-stores/preprod-live2c/bnd-census-classified.txt`
> (commit `941f98a2`). This slice implements **BND-1 only**. It changes no ledger verdict, and it does
> **not** touch `InvalidTxCarriesAuthorityEffect` or any guard in `apply_tx_scan` — BND-2 is gated on
> reference-semantics extraction and is out of scope here.

## The defect, as measured

```
bnd-census: stall_slot=130350133 cursor_slot=130350114
            stall_block_epoch=305 cursor_epoch=305  detected_transition=false
            stall_reason=InvalidTxCarriesAuthorityEffect { tx_index: 0 }
```

An ordinary within-epoch apply failure is currently indistinguishable from "a boundary whose mark was
withheld", because both collapse into one state:

```
apply_selected_block(..) -> Err(_)   ⇒   AdvanceOutcome::Stalled { slot, reason: String }
                                     ⇒   AccumulatorChaindbOutcome::StalledAt
                                     ⇒   the relay loop's boundary arm
                                     ⇒   rewind the reduced checkpoint, sum a SNAP mark, cross
```

`advance_accumulator_over_block`'s own doc names both causes for the one variant — "a boundary the mark
is withheld for, **or** a byte-uncertain block". That is the flattening, stated in the source and
measured live: 84,783 ms of boundary machinery plus 23,389 ms of forward advance undoing its rewind,
for a block the walk classified in 195 ms and which is **not on a boundary at all**.

## Invariant

**INV-BND-1 — a stall's CAUSE is typed, and only a real epoch transition may enter boundary machinery.**
Advancing the accumulator over one block yields exactly one of: applied, already applied, *a boundary
crossing is required* (the block's epoch is strictly ahead of the accumulator's), or *the apply failed*
(carrying the ledger's own typed error). A within-epoch apply failure MUST NOT cause a checkpoint
rewind, a boundary-mark capture, or a boundary cross to be attempted.

Registry: **DC-EPOCH-39** (derived). Related: DC-EPOCH-20 (accumulator advance), DC-EPOCH-22 (boundary
crossing), DC-EPOCH-32 (positioned checkpoint before sealing).

## The predicate is POSITIVE and structural — not an error-string inference

The boundary decision is taken **before** the apply, from canonical data on both sides:

```
ctx.block_epoch  >  acc.epoch_state.epoch   ⇒  BoundaryMarkRequired   (a crossing is genuinely due)
ctx.block_epoch ==  acc.epoch_state.epoch   ⇒  apply; any Err is ApplyFailed
ctx.block_epoch  <  acc.epoch_state.epoch   ⇒  apply; the ledger already returns the typed
                                               `BoundaryGap`, which is an ApplyFailed like any other
```

Total by construction, and it matches the authority it is predicting: `apply_selected_block_core`
crosses exactly `first_boundary ..= ctx.block_epoch` where `first_boundary = acc.epoch + 1`, so the
loop fires **iff** `block_epoch > acc.epoch`. Classifying by *error variant* was rejected: it would
re-derive the boundary condition from a failure message, and a future boundary-related error would
silently rejoin the two classes. This slice's whole point is that the two classes stop being one.

## What ships

**1. `AdvanceOutcome` gains a discriminated cause.** `Stalled { slot, reason: String }` is replaced by
two variants — `BoundaryMarkRequired { slot, from_epoch, to_epoch }` and `ApplyFailed { slot, error }`,
the latter carrying `LedgerTransitionError` itself rather than a rendered string. Replacement, not
addition: leaving `Stalled` reachable would let a caller keep conflating them.

**2. `AccumulatorChaindbOutcome` mirrors it.** `StalledAt { slot, reason }` becomes
`BoundaryRequiredAt { slot, from_epoch, to_epoch }` and `ApplyFailedAt { slot, error }`.

**3. The relay loop routes on the cause.** Only `BoundaryRequiredAt` reaches the existing boundary arm
(checkpoint positioning, mark capture, cross) — that path is otherwise unchanged, including its B6
memo. `ApplyFailedAt` records a typed, announced-once observe-only accumulator failure and breaks:
no rewind, no mark, no cross.

**4. The accumulator stays observe-only.** An `ApplyFailedAt` does not halt the follow and does not
advance the cursor. Behaviour on the live store is *identical* except that the wrong path stops
running: same block, same error, same pinned cursor.

## Mechanical acceptance criteria

| CE | Criterion | how it is judged |
|---|---|---|
| **CE-BND1-1** | A within-epoch apply failure yields `ApplyFailed`, never a boundary state | unit: an accumulator at epoch E, a block at epoch E whose apply fails ⇒ `ApplyFailed`, and the boundary predicate is false |
| **CE-BND1-2** | A genuine crossing yields `BoundaryMarkRequired` carrying both epochs | unit: accumulator at E, block at E+1 ⇒ `BoundaryMarkRequired { from: E, to: E+1 }`, *without* calling apply |
| **CE-BND1-3** | The boundary arm is UNREACHABLE from a within-epoch failure | the loop matches on the typed cause; a test drives the co-advancer over a within-epoch failing block and asserts the reduced checkpoint's cursor is untouched (a rewind would move it) |
| **CE-BND1-4** | The ledger's typed error survives to the caller unrendered | `ApplyFailed { error: LedgerTransitionError }` compared by value, not by string |
| **CE-BND1-5** | Cursor and verdict are unchanged | same block ⇒ same pinned cursor, same error, before and after the slice |
| **CE-BND1-6** | LIVE: zero boundary-path execution when `detected_transition=false` | the live re-run emits the apply-failed line and **no** `REWOUND onto boundary point` / `boundary cross stalled`; `boundary_arm_ms` collapses |
| **CE-BND1-7** | Negative-tested | mutations below |

### Required mutations

route `ApplyFailed` back into the boundary arm (must fail CE-BND1-3/6) · derive the boundary condition
from the error variant instead of the epoch comparison (must fail CE-BND1-1) · use `>=` instead of `>`
in the boundary predicate (must fail CE-BND1-1: every within-epoch block becomes a boundary) · render
the error to `String` (must fail CE-BND1-4) · make `ApplyFailed` halt the follow (must fail the
observe-only contract).

## Colour law

```
BLUE   apply_selected_block, LedgerTransitionError, the epoch comparison operands   | authority
GREEN  AdvanceOutcome / AccumulatorChaindbOutcome — closed sums over that authority | classification
RED    the relay loop's routing + the observe-only log                              | shell
```

The predicate reads two BLUE values (`ctx.block_epoch`, `acc.epoch_state.epoch`) and produces a GREEN
classification. No ledger rule is consulted, added, or weakened.

## Store semantics — NEUTRAL, no bump

Nothing persisted changes. No accumulator, checkpoint, WAL, or sidecar field gains or changes meaning;
the cursor moves exactly as before (i.e. not at all, on this block). `STORE_SEMANTICS_VERSION` stays
**v3**.

## Explicitly NOT in this slice

- Any change to `InvalidTxCarriesAuthorityEffect`, `InvalidTxCollateralNeedsUtxo`, or the phase-2-invalid
  tx semantics. **BND-2 begins with oracle extraction, not implementation.**
- Unpinning the cursor. BND-1 does not let the accumulator cross 130,350,133 — only BND-2 can, and this
  slice must not create the appearance of progress there.
- Any DC-NODE-15 / B12 change. The `+1` is classified benign (`fcbabb67`) and stays gated until the
  authority behind it is healthy.
- The 84.8 s cost is a *consequence* removed here, not the justification: this is a correctness slice
  about two failure classes being one state, and it would be worth landing at zero time saved.
