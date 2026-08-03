# SLICE P5 — the ledger epoch must agree with the venue era schedule, and mainnet constants must stay contained

> Direct hardening from **P4** (`e1de7a2e`), which proved the failure was MIXED-SEMANTICS EXECUTION,
> not crash recovery. This slice builds the two mechanisms that would have prevented the whole chain,
> in the order that matters: the missing invariant first, then containment of the construct that
> caused it.

## Why P4 was possible at all

P4's root cause was that a preview store's ledger sat at epoch **1375** while the venue era schedule
said **1378** — for the store's entire life. Two independent authorities disagreed by three epochs
and **nothing ever compared them**:

```
era_schedule.locate(119075343).epoch = 1378      <- the venue authority
ledger.epoch_state.epoch             = 1375      <- the ledger, silently frozen
epoch-accumulator: CROSSED 1377 -> 1378          <- a THIRD authority, correct all along
```

The bug that froze it (`detect_epoch_transition` computing the epoch from hardcoded MAINNET
constants) was fixed in P3. But P3 fixed the *instance*. Nothing prevents the *class*, and nothing
would have noticed the drift if the constants had been wrong in some other way.

This is the IDD failure mode named in the constitution: *"If an invariant cannot be checked by types,
tests, lints, or CI, it is not protected."* The invariant existed in everyone's head. It was never
mechanical.

## Invariant 1 — epoch agreement (DC-EPOCH-36)

**After the epoch-boundary decision for a block at `slot`, the ledger's epoch MUST equal the venue era
schedule's epoch for that slot.** A disagreement in EITHER direction is a durable-state contradiction
and fails closed.

Why this is the right invariant, not merely a useful assertion:

- It is **strictly stronger than the detection logic it guards.** `detect_epoch_transition` fires only
  on `schedule_epoch > ledger_epoch`. It is structurally blind to `schedule_epoch < ledger_epoch` — a
  ledger AHEAD of the schedule — which is equally a contradiction and equally silent.
- It would have caught P3's bug **on the first block, on both venues**, where the mainnet-corpus test
  suite could not: preview (473 vs 1375 → frozen forever) and preprod (498 vs 304 → phantom boundary).
  Both are the same defect; both are one comparison away from fail-fast.
- It costs one `locate()` per block on a path that already calls `locate()`.

### Placement (the part that needs care)

The check is a **post-condition of the boundary decision**, not a precondition of the apply:

- It must run **after** any boundary application, because the epoch legitimately changes mid-apply.
- `era_schedule.locate(slot)` failing (a slot before the schedule's first era — the mainnet corpus has
  pre-Shelley slots) means the invariant is **unverifiable**, not violated. Skipping the check there
  preserves P3's exact behaviour (`detect_epoch_transition` already returns `None` via `.ok()?`).
  Turning an unlocatable slot into an error would be an unrelated behaviour change and is NOT done here.

### Making it unforgettable

Three call sites (`rules.rs:125`, `306`, `399`) each open-code detect-then-dispatch. Adding a fourth
that forgets the check is the obvious future regression, so the check is not offered as a
call-it-yourself helper. Instead:

- `detect_epoch_transition` is demoted from `pub` to `pub(crate)`.
- A single `cross_epoch_boundary_for_slot` performs detect → dispatch → **verify** and is the only
  caller of `detect_epoch_transition`. All three sites route through it.
- `ci/ci_check_epoch_agreement.sh` enforces the single-caller invariant mechanically.

This mirrors the pattern the repo already proved for `block_validity_trusted_replay` — `pub(crate)`,
single caller, CI-enforced (`ci/ci_check_trusted_replay_boundary.sh`).

## Invariant 2 — venue-constant containment (DC-LEDGER-13)

**Mainnet Shelley constants may enter a computation only through the explicitly-named
`mainnet_shelley_schedule()`.** P3 established this in prose ("retained but can now only enter through
an explicit, named function, never by default"); this makes it mechanical.

Current state, measured rather than assumed:

| construct | status |
|---|---|
| `slot_to_epoch` (mainnet constants) | **no production callers**; 3 call sites in `tests/rvbp_live_path_reduced_dispatch.rs` |
| `SHELLEY_EPOCH_LENGTH` in `apply_epoch_boundary_full` (`rules.rs:707`) | **live production use** |

So the trap is still armed on both counts, and the gate surfaces a **second instance of the same bug
class** that P3 did not remove.

### The `apply_epoch_boundary_full` denominator is NOT fixed here — deliberately

`apply_epoch_boundary_full` passes `SHELLEY_EPOCH_LENGTH / 20` as the monetary-expansion
expected-blocks denominator (mainnet `432_000/20 = 21_600`). Its own comment concedes the asymmetry:
*"The accumulator path (preview / multi-network) sources the REAL per-era epoch length from the era
schedule instead."*

This is the full-ledger (`track_utxo=true`) path that produces the **mainnet reward results** the
CE-71 / CE-3d work is measured against. Changing that denominator is a reward-semantics change, not a
containment cleanup, and doing it inside this slice would entangle a prevention mechanism with a
result-changing edit. It is therefore **allowlisted with an explicit justification** so it cannot grow
silently, and recorded as follow-up. The gate fails if any *other* site appears, or if the allowlist
grows without an accompanying justification comment.

## Mechanical acceptance criteria

| CE | Criterion |
|---|---|
| **CE-P5-1** | `check_epoch_agreement` rejects a ledger epoch that disagrees with the schedule in EITHER direction (stale and ahead), as a typed `LedgerError`, not a panic |
| **CE-P5-2** | An unlocatable slot (before the schedule's first era) is NOT a violation — behaviour identical to pre-slice |
| **CE-P5-3** | `detect_epoch_transition` is `pub(crate)` and has exactly ONE non-test caller (`cross_epoch_boundary_for_slot`); CI-enforced |
| **CE-P5-4** | All three former call sites route through `cross_epoch_boundary_for_slot`; a boundary crossing still produces byte-identical state (mainnet corpus unchanged) |
| **CE-P5-5** | `slot_to_epoch` is not `pub`; the integration test derives its epoch from a schedule instead |
| **CE-P5-6** | CI fails if mainnet constants are referenced outside `mainnet_shelley_schedule()` + the single justified allowlist entry |
| **CE-P5-7** | Negative-tested: both CI gates fail when the violation they exist to catch is introduced |
| **CE-P5-8** | Existing suites green — `ade_ledger`, `ade_node`, `ade_runtime`, `ade_core` — with no fingerprint change on the mainnet corpus |

## What this does NOT do

- It does **not** recover the P4 store. Nothing can; three epochs of boundary effects were never
  applied, and the store must be re-bootstrapped.
- It does **not** add the store-semantics version gate (P4 follow-up #1). That is the separate,
  larger slice — this one prevents the ledger from silently going wrong; that one prevents an
  already-durable store from being silently re-interpreted. Both are needed.
- It does **not** change the `apply_epoch_boundary_full` reward denominator (see above).
