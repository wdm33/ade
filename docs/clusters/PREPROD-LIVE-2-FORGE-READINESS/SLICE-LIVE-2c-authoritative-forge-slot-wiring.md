# SLICE LIVE-2c — authoritative forge-slot wiring

> **OPEN — DOC BEFORE IMPL.** The canonical arithmetic is closed (`31f7c754`,
> `EraSchedule::slot_at`). This slice makes the authoritative `--mode node` producer path USE it,
> removes the naive anchor triple, and turns B11's silent `Option` into a typed refusal.

## Intent

> Wire the complete era-schedule authority into the `--mode node` producer path, remove the naive
> anchor triple, and turn B11 into a typed refusal.

Explicitly **does not** absorb B6's sync-deferral liveness work.

## Colour law — `slot_at` is BLUE, not GREEN

Corrected here because the arithmetic is small enough to be mistaken for transport. The `SlotNo` it
returns selects the KES period, drives VRF leadership evaluation, is written into the block header, and
is signed — a wrong result produces an **invalid block**. So the conversion owns authoritative meaning:

```
RED   capture UnixMillis
BLUE  EraSchedule::slot_at(captured_ms) -> SlotNo | SlotDerivationError
BLUE  forecast / KES / leadership decisions
RED   signing and transmission
```

GREEN may transport the captured instant in and the verdict out. It must never own the conversion, or
GREEN would be affecting authoritative output. Structurally this already holds — `slot_at` lives in
`ade_core` — but the classification is now named, not merely implied by file location.

## The defect being removed

`operator_forge.rs:182–192`:

```rust
let anchor_millis  = genesis.slot_zero_time_unix_ms;   // the BYRON start
let start_slot     = SlotNo(0);
let slot_length_ms = /* shelley */ 1000;
```

Those three together **are** the naive calculation. Preprod error: exactly
`86_400 × (20 − 1) = 1_641_600` slots ≈ 19 days.

## RULING on the truncated Mithril schedule — do NOT paper over it

`ScheduleDoesNotCoverSystemStart` must **not** be solved by supplying an arbitrary first-segment start
time from node configuration. That would create two possible slot authorities — full era geometry
accumulated from system start, and a truncated segment plus a separately supplied absolute timestamp.
They may be mathematically equivalent, but unless the second is *derived and bound from the first*, they
are two semantic paths that can disagree. This project has spent three slices removing exactly that
shape.

### Preferred solution

The authoritative node path receives a **complete, bootstrap-bound slotting schedule covering system
start through the active era**, constructed once through the existing era-schedule authority — **not
rebuilt inside `operator_forge.rs`** — from the canonical venue inputs bootstrap already admits:

- Byron genesis start + slot geometry;
- Shelley system start + slot geometry;
- canonical era-transition boundaries;
- the network/genesis commitments bootstrap already verified.

A truncated snapshot schedule stays useful for epoch/ledger calculations whose domain begins at the
snapshot point. It is **insufficient as wall-clock→absolute-slot authority**.

### Permitted fallback shape — admissible ONLY with all six proofs

A partial schedule plus an explicit segment start time is admissible **only if**:

1. the segment start time is deterministically derived from the complete canonical schedule;
2. its provenance is bootstrap-bound;
3. the partial form has ONE canonical encoding;
4. replay proves it yields the same slots as the complete schedule over its domain;
5. no operator configuration can supply or override it;
6. there is no second independent conversion implementation.

Without those proofs, **reject that design.**

## Required transitions

```rust
let captured_ms = clock.capture_millis();                       // RED
let slot = era_schedule.slot_at(captured_ms)
    .map_err(ForgeRefused::SlotDerivation)?;                    // BLUE
let kes_period = coordinator_state.kes_period_for_slot(slot)
    .map_err(ForgeRefused::KesWindow)?;                         // BLUE
let leadership = evaluate_operator_leadership(slot, kes_period, ...);
```

APIs may differ; **there must be no `Option`-based disappearance.**

### B11 error shape

`kes_period_for_slot` must expose WHY, e.g. — only variants the implementation actually supports:

```rust
enum KesSlotError {
    BeforeOperationalCertificateStart { slot: SlotNo, first_supported_slot: SlotNo },
    AfterOperationalCertificateEnd    { slot: SlotNo, last_supported_slot: SlotNo },
    PeriodArithmeticOverflow          { slot: SlotNo },
}
```

And the refusals stay **distinguishable authority failures**, never collapsed:

```rust
enum ForgeRefused {
    SlotDerivation(SlotDerivationError),
    Forecast(ForecastError),
    KesWindow(KesSlotError),
    LeadershipAuthority(LeaderScheduleError),
}
```

## Mechanical acceptance criteria

| CE | Criterion | status |
|---|---|---|
| **CE-L2c-1** | `--mode node` calls the shared `EraSchedule::slot_at` | open |
| **CE-L2c-2** | The naive `system_start + single slot length` conversion is UNREACHABLE from forging | open |
| **CE-L2c-3** | Native-Mithril warm start supplies a schedule covering system start | open |
| **CE-L2c-4** | A truncated snapshot-only schedule returns `ScheduleDoesNotCoverSystemStart` | open |
| **CE-L2c-5** | The preserved preprod instant produces slot **130,338,561** through the ACTUAL node wiring | open |
| **CE-L2c-6** | B11 returns a typed `ForgeRefused`, never a skip | open |
| **CE-L2c-7** | The configured pool reaches `classify_leader_schedule` | open |
| **CE-L2c-8** | The branch marker proves the KNOWN operator pool was evaluated | open |
| **CE-L2c-9** | The live run emits a decided `ForgeOutcome` | open |
| **CE-L2c-10** | Replaying instant + schedule + anchor + authority inputs reproduces the same slot and outcome | open |
| **CE-L2c-11** | Negative-tested (seven mutations below) | open |

Required mutations: restore the naive conversion; hardcode the preprod boundary; accept a truncated
schedule as complete; node path bypasses `slot_at`; B11 restored to `None`; diagnostic path fixed while
the node path stays old; known-pool evaluation replaced by `UnknownPool`.

## Store-semantics decision — reassess DURING wiring

Keeping v3 was correct for `31f7c754`. For this slice:

- **No bump** if the full schedule is reconstructed solely from already bootstrap-bound genesis/era
  inputs and no persisted format or interpretation changes.
- **Bump required** if the snapshot, checkpoint, WAL, store metadata or fingerprint gains a schedule
  commitment, **or** if an existing persisted schedule field changes meaning from "snapshot-local" to
  "absolute slot authority".

The decision turns on **durable interpretation**, not on how much code moved.

## Status this slice inherits

| | |
|---|---|
| canonical slot arithmetic | **closed** (`31f7c754`) |
| root cause | **closed** (`7e2c66ee`) |
| authoritative node wiring | **open — this slice** |
| B11 typed refusal | **open — this slice** |
| known-pool leader evaluation | unreached |
| LIVE-2 | open on E4 |
| B6 bounded deferral | separate open liveness slice |
| LIVE-3 | untouched |

## Not claimed

No code. This records the wiring contract, the truncated-schedule ruling and its six-proof fallback
bar, the B11 error shape, and the store-semantics reassessment trigger — before implementation, because
the schedule-construction decision is consensus-critical and is exactly where a convenient shortcut
would reintroduce a second slot authority.
