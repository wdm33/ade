# SLICE LIVE-2b — ForgeTick reachability and suppression accounting

> ## ⚠ CORRECTED 2026-08-06 by LIVE-2c, from this slice's OWN run-4 artifacts
>
> Two conclusions below were reached by reading code, and decomposing `wire_smoke.jsonl` +
> `live2-run4.log` overturned both. **No new run was needed** — the evidence was already on disk. The
> original text is left intact underneath, because the way it went wrong is the useful part.
>
> **B11 did NOT fire, and the wrong slot was never caught by the KES window.** This slice concluded
> that `kes_period_for_slot` returned `None` because *"a slot that far out is comfortably outside any
> KES validity window"*, and that *"the only thing preventing [a 19-day-ahead forge] today is an
> accident of the KES range check."* That accident does not exist on this venue. The operator op-cert
> starts at KES period **970**; with `slotsPerKESPeriod = 129_600` and `maxKESEvolutions = 62`, the
> naive slot 131_976_696 is absolute period 1018 → **evolution 48**, inside `[0, 62]`. So
> `kes_period_for_slot` returned `Some(48)` and the wrong slot sailed through. Corroborated
> end-to-end: all 354 `forge_result` records carry `skip_reason = "tip_mismatch"`, only written
> *inside* the `Some(kes_period)` branch — and the **first** record already carries it, while
> `last_forge_refused` starts `None`, so it cannot be a stale sticky value. Pinned as
> `the_naive_19_day_ahead_slot_was_never_refused_by_the_kes_window`.
>
> The consequence is the opposite of what this slice recorded: nothing downstream refused the wrong
> slot, so the correct slot derivation was the load-bearing fix and B11's typed refusal is defence in
> depth. Both shipped in LIVE-2c (`DC-NODE-45`, `DC-NODE-46`).
>
> **The measured suppressor is a SEVENTH exit — B12 — not one of B1–B11.** All 354 admitted ticks
> were refused by the DC-NODE-15 catch-up gate, with `local_tip_block_no − peer_tip_block_no == +1`
> in **354/354** samples, on blocks Ade had fetched *from that same peer*. The followed-peer-tip
> signal is written only from a chain-sync message's `tip` field, and the message delivering block
> `N+1` advertises tip `N`; at the chain tip no further message arrives until the next block, so the
> signal stays one block behind for the whole inter-block interval.
> `durable_servable_tip == followed_peer_tip` is therefore **structurally unsatisfiable while
> following a real cardano-node at the tip**. See
> `SLICE-LIVE-2c-ACTIVATION-handoff.md` §M2 — B12 is deliberately unfixed there (changing a
> DC-NODE-15 operand is consensus-adjacent and needs its own census).
>
> **Method note, since this slice exists to enforce it.** The branch table was built by reading, and
> said so honestly. The failure was not the reading — it was treating a *plausible* mechanism
> (`None` from an out-of-range KES check) as the measured one, when the run's own JSONL named a
> different branch. The discriminator that settled it cost four minutes and no node time.

> **OPEN — DISCRIMINATOR RUN COMPLETE 2026-08-06.** The branch table below was read from code; the
> emit-only probe then ran unchanged and **named the branch**. Two findings, neither of which was the
> leading hypothesis. No fix is written. Blocks E4 and therefore LIVE-2.
>
> **The ForgeTick fires — 354 of 363 probes.** Reachability was never the defect. The loop is
> suppressed at a **SIXTH silent exit the code-read table missed**, and it is gated by a **wall-clock
> slot ~19 days ahead of the chain**.
>
> **ROOT CAUSE SELECTED**: the wall-clock→slot conversion treats Byron's 20s slots as 1s. The gap is
> exactly `86,400 × (20 − 1) = 1,641,600` slots. The peer is FRESH (2 slots off an independent
> derivation), so this is Ade's anchor, not a stale venue. Fix not yet written.

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

## DISCRIMINATOR RESULT — run 4, 363 probes

```
  354  forge_slot_status=Due  sync_status=NoWorkReady   loop_step=ForgeTick     <- ADMITTED
    8  forge_slot_status=Due  sync_status=WorkAvailable loop_step=SyncOnce      <- B6, transient
    1  forge_slot_status=Due  sync_status=NoWorkReady   loop_step=HaltCleanly
```

Against the predicted table: `forge_active=true` throughout, a logical slot always present, status
always `Due`. So **B1, B5 and B8 are eliminated by measurement**, and **B6 is confirmed but transient**
(8 of 363 — real, and still a liveness surface worth the bound, but not the blocker).

**`ForgeTick` was admitted 354 times and produced ZERO `forge-decision` emits and ZERO typed
cannot-answer refusals.** So the suppression is *inside* the ForgeTick arm, below the planner and above
the leader evaluation — a branch the code-read table did not reach.

### B11 — the sixth silent exit, `node_lifecycle.rs:3713`

```rust
} else if let Some(kes_period) = act.coordinator_state.kes_period_for_slot(slot.0) {
    // "Out of range => skip: no forge, no `last_forged_slot` update"
```

`kes_period_for_slot(slot) == None` skips the entire forge attempt: `forged` stays false, no
`ForgeRefused` is recorded, nothing is emitted. It is the only path between an admitted tick and the
typed surface that produces no evidence at all — precisely the class this slice exists to close, found
one level deeper than the table looked.

### And the reason it returns None — a SECOND, more serious finding

| quantity | value |
|---|---|
| Ade `logical_slot` (wall-clock derived) | **131,976,696** |
| cardano-node tip (`query tip`) | **130,335,017** (epoch 305) |
| gap | **~1,641,607 slots ≈ 19 days** |

Ade is deriving a logical slot ~19 days AHEAD of the chain it is following, and asking the KES schedule
to forge there. A slot that far out is comfortably outside any KES validity window, so `None` is the
*correct* answer to a *wrong question*.

**This is the more important of the two findings.** B11 is a missing typed reason; the slot derivation
is a correctness defect on the input to leadership itself — forging on a slot 19 days ahead would be
categorically wrong, and the only thing preventing it today is an accident of the KES range check.

### ROOT CAUSE SELECTED 2026-08-06 — both discriminators run BEFORE any code change

| quantity | value |
|---|---|
| INDEPENDENT derived slot (from genesis, **not** via `checked_millis_to_slot`) | **130,338,561** |
| peer tip slot | **130,338,559** — 2 slots behind ⇒ **the peer is FRESH** |
| naive derivation with Byron's 20s slots IGNORED | **131,980,161** |
| naive − independent | **1,641,600** |
| Ade `logical_slot` (probe) | 131,976,696 — matches the NAIVE line, offset by the elapsed time between captures |

Decision-table verdict: *independent ≈ peer tip*, *Ade ≈ naive* ⇒ **wrong Ade slot anchor /
conversion.** The peer is exonerated — `syncProgress: "100.00"` is corroborated here rather than
trusted, because the peer sits within two slots of an independently derived venue slot.

**The constant is exact, and it names the defect:**

```
86,400 byron slots × (20s − 1s) = 1,641,600 s = 1,641,600 shelley slots
                                = the measured naive-vs-independent gap, to the second
```

The conversion treats the **Byron era as 1-second slots** — it uses the system start with the *Shelley*
slot length and never applies the 20s Byron segment. Preprod's Byron era is 4 epochs (86,400 slots),
which is exactly 19 days of error. Full tuple preserved as the regression fixture:
`docs/evidence/run-stores/preprod-nonce-1/live2b-slot-authority-discriminators.txt`.

### Superseded first reading

**Not diagnosed, and deliberately not attributed.** It is at least two distinct possibilities —
`checked_millis_to_slot`'s anchor (`anchor_millis` / `start_slot` / `slot_length_ms`) being wrong for
this venue, or the docker preprod peer being ~19 days stale while reporting `syncProgress: "100.00"`.
Those have opposite fixes. The discriminator is cheap: compare Ade's derived slot against the venue's
genesis `systemStart` + slot length computed independently, and separately check the peer's real
chain-tip age. Do that before touching either.

## Required design if silence exists (it does — SIX exits, B3/B4/B5/B6/B8 and now B11)

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
