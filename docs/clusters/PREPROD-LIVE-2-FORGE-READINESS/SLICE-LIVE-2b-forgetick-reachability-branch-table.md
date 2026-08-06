# SLICE LIVE-2b — ForgeTick reachability and suppression accounting

> **OPEN — FINDINGS FIRST. This is the branch table, read out of the code. No behaviour change is
> written, and no cause is selected.** Blocks E4 and therefore LIVE-2.

## Intent

> Prove that every captured forging opportunity on the warm-started `--mode node` path results in
> exactly one of: an admitted `ForgeTick`, or a closed, typed suppression reason. **Silence must not be
> a valid outcome.**

## Authority colours of the gap

| surface | colour | responsibility |
|---|---|---|
| observe wall-clock / slot progression | **RED** | capture a slot event; never decide leadership |
| decide whether a captured slot may enter forging | **GREEN**, authority-adjacent | deterministic admissibility from explicit node state |
| evaluate operator leadership | **BLUE** | pure schedule result + typed classification |

The risk is **not** "leadership authority missing" — LIVE-2 E1/E2/E3 proved authority is established.
The risk is that the **RED→GREEN handoff or the GREEN admissibility gate suppresses evaluation without
producing evidence explaining why**. A legitimate slot silently suppressed is a *derived reachability
defect*; the live proof closing it is a *release/evidence obligation*. They are not the same tier.

## THE BRANCH TABLE — every exit between the clock and a decided outcome

Traced `node_lifecycle.rs:3236–3272` → `run_loop_planner.rs:153–185` → `node_sync.rs:2061` (the
authoritative `--mode node` chain).

| # | site | condition | result | typed? | emitted? |
|---|---|---|---|---|---|
| B1 | `node_lifecycle.rs:3237` | `forge.as_deref_mut() == None` (forge OFF) | `NotDue` | by construction | not per-iteration |
| B2 | `:3239` | `act.pending_slot = None` reset each iteration | — | n/a | no |
| B3 | `:3265` | `act.clock.next_tick() == None` — "clock exhausted" | `NotDue` | **NO** | **SILENT** |
| B4 | `:3258` | `checked_millis_to_slot` → `Err` (wall-clock before genesis anchor) | `NotDue` | `SlotAlignmentError` stored in `act.last_slot_alignment_fail` | stored, **not emitted here** |
| B5 | `:3250` | `forge_slot_status(last_forged_slot, slot)` → `NotDue` (slot ≤ last forged) | `NotDue` | closed enum, **no operands** | **SILENT** |
| B6 | `run_loop_planner.rs:164` | `SyncStatus::WorkAvailable` → `SyncOnce` **regardless of a due slot** | forge deferred | n/a | **SILENT** |
| B7 | `:171` | `LoopState::Ending` + `HaltOnFeedEnd` | `HaltCleanly` | n/a | `forge_tick_skipped` exists on this arm |
| B8 | `:180` | `ForgeSlotStatus::NotDue` | `Idle` | closed | **SILENT** |
| B9 | `node_sync.rs` ForgeTick arm | DC-NODE-15 / fence / reselection refusals | `ForgeRefused` | **yes**, 7 `ForgeSkipReason` | yes |
| B10 | `node_sync.rs:2061` | leader-schedule classification | 3 branches | **yes** (LIVE-2, `4c03592a`) | yes — `forge-decision` |

**Five silent exits — B3, B5, B6, B8, and B4's non-emission — sit between a captured slot and the first
typed surface (B9).** Every one of them can consume a legitimate forging opportunity and leave no
evidence. That is the defect class this slice exists to close, independent of which one fired.

### RED vs durable-state inputs

| input | source |
|---|---|
| `now_ms` (`clock.next_tick()`) | **RED** — the sole wall-clock observation (DC-NODE-03) |
| `anchor_millis`, `start_slot`, `slot_length_ms` | durable node state |
| `last_forged_slot` | node loop state |
| `SyncStatus` (`has_work_ready`) | **RED** — wire-pump lookahead |
| `LoopState` (feed ending) | **RED** — feed liveness |

### Shared vs duplicated logic

| | |
|---|---|
| B1–B8 | `--mode node` **only** — `--mode produce` has its own loop (`produce_mode.rs:395`) |
| B10 | **SHARED** since `4c03592a` — one `classify_leader_schedule` for both paths |
| B9 | `--mode node` only |

Recorded because this slice already tripped on it once: fixing B10 in produce_mode alone left the
authoritative path defective. **A branch verified on `--mode produce` proves nothing about `--mode node`.**

## What the failing run measured, and what it did NOT

From `live2-e1-e2-warmstart-anchor.log` (~17 min at tip, forge-capable):

- `forge CAPABLE — operator keys loaded` at exit ⇒ **B1 is not the cause** (forge material loaded).
- `AT PEER TIP` sustained ⇒ the node is caught up.
- **zero** forge lines of any kind ⇒ suppression happens at or before B8; B9/B10 were never reached.

Two candidates are **eliminated by reading**, not by guessing:

- **B3 is impossible with `SystemClock`** — `next_tick()` sleeps to the boundary and returns `Some`
  unconditionally (`clock.rs:174`). It cannot exhaust.
- **B6 is unlikely at tip** — `has_work_ready()` for `WirePump` pumps the lookahead and returns
  `!lookahead.is_empty()`; at tip that is normally empty, so `NoWorkReady` should reach the planner.

That leaves **B1's actual runtime value** (is `forge` `Some` *inside the loop*, or only the capability
line at exit?) and **B5** as the live candidates. **Neither is selected.** The cheapest discriminator is
below; guessing between them is what this slice exists to avoid.

## First step when picked up — one measurement, not a fix

Add an emit-only per-iteration line at `node_lifecycle.rs:3236` naming: `forge_active` (is `forge`
`Some`), `pending_slot`, `last_forged_slot`, the resulting `ForgeSlotStatus`, the `SyncStatus`, and the
`LoopStep` the planner returned. One warm-start run then names the exact branch, because every
candidate is a distinct combination of those six values. Only then choose the fix.

## Required design if silence exists (it does — five exits)

```rust
enum ForgeTickAdmissibility { Admit, Suppress(ForgeTickSuppression) }
```

`ForgeTickSuppression` a **closed structured enum** carrying operands — logical slot, selected tip,
caught-up state, anchor status, key readiness, forecast boundary where relevant. **No strings as
authority.** Extracted pure and total, exactly as `classify_leader_schedule` was, so it is
mutation-testable and cannot be duplicated per-path.

Every observed slot-cycle must then yield one canonical chain:

```
ObservedSlot → TickAdmissibility → ForgeTick | TypedSuppression → LeaderScheduleBranch → ForgeOutcome
```

Wall-clock observation stays RED; once converted to an explicit logical slot, everything downstream is
deterministic.

## Mechanical acceptance criteria

| CE | Criterion | status |
|---|---|---|
| **CE-L2b-1** | The authoritative `--mode node` call graph is explicitly tested, not inferred from `--mode produce` | open |
| **CE-L2b-2** | Every suppression branch returns a typed reason; no silent `None`, `continue`, or swallowed error remains | open |
| **CE-L2b-3** | At least one live logical slot observed entering the node-loop gate | open |
| **CE-L2b-4** | At least one slot admitted as `ForgeTick` | open |
| **CE-L2b-5** | The admitted tick reaches `classify_leader_schedule` | open |
| **CE-L2b-6** | The branch marker proves the configured operator pool was actually evaluated | open |
| **CE-L2b-7** | A decided outcome: known pool not-leader, or known pool leader + candidate path entered | open |
| **CE-L2b-8** | Captured slot inputs replay through gate + classifier with identical verdicts | open |
| **CE-L2b-9** | Warm restart preserves the same admissibility inputs and authority binding | open |
| **CE-L2b-10** | Negative-tested: node-path enqueue removed; admissible tick forced into silent suppression; typed suppression replaced by `None`; only the diagnostic path wired; known-pool evaluation replaced with `UnknownPool`; branch marker removed from the returned evidence path | open |

## Hard prohibitions

- No manually injected tick may count as the live proof.
- No bypass of caught-up, anchor, forecast, or key-readiness gates.
- No artificial slot/epoch parameter changes.
- No diagnostic-path output used as evidence for `--mode node`.
- No "node remained healthy" substitution for an actual decision.
- No forge-success or peer-acceptance claim in this slice.

## Closure

> A warm-started, authority-established `--mode node` instance captures a real slot, deterministically
> admits it to `ForgeTick`, evaluates the configured operator pool, and emits a trustworthy decided
> `ForgeOutcome`.

Only that closes E4, and therefore LIVE-2.

## Not claimed

No behaviour change, no root cause, no invariant, no CE. Two candidates eliminated by reading; the
remaining two are named as candidates and deliberately not chosen.
