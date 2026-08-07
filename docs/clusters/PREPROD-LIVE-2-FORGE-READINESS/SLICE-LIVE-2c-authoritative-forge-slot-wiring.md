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

## SCOPE — this is calendar geometry, NOT era support

Read this before the wording below, because "complete schedule covering system start" sounds larger
than it is and would otherwise be misread as reopening Ade to historical eras. **It does not.**

> Ade is a **Conway-active** node bootstrapped from a certified recent snapshot. It implements the
> current Conway ledger, consensus, governance, transaction, N2N and N2C semantics required for live
> compatibility. It does **not** replay or execute historical eras. Historical era information is
> retained only where current Cardano semantics depend on it — such as absolute slot-time geometry,
> immutable protocol identifiers, and hard-fork lineage.

Byron appears here for one reason: **Cardano slot numbers are global and did not restart at Conway.**
Preprod's first 86,400 slots lasted 20s; later slots last 1s. Translating "now" into today's absolute
slot must account for the time that historical segment consumed. That is arithmetic about a calendar,
not Byron ledger semantics — the way computing someone's age needs their birth date but not a
reconstruction of their childhood.

**The implementation already matches this scope, verifiably.** `slot_at` reads exactly four things:
`system_start_unix_ms`, the era list, and per segment `start_slot` and `slot_length_ms`. It never reads
`era: CardanoEra`, `start_epoch`, `epoch_length_slots`, `safe_zone_slots` or the RSW. So the authority
this slice wires is **timing-only by construction**, and CE-L2c-3 requires timing facts, not Byron
support:

| the schedule must carry | it must NOT carry |
|---|---|
| system start | Byron transactions or ledger rules |
| segment start slot | Byron block production |
| segment slot length | Byron chain state |
| transition into the next timing segment | any pre-Conway execution semantics |

Ade still enters at an existing non-Origin Conway tip and never grows the chain from genesis (C2
doctrine). Nothing here changes that.

### What the remaining transaction work actually is

Under this scope, transaction compatibility means agreement on the **Conway** ledger's current surface —
UTxO spending, witnesses, fees and validity intervals, Conway-legal delegation/pool certificates,
withdrawals, native scripts, Plutus V1/V2/V3 as supported in Conway, collateral, redeemers, datums,
reference inputs and scripts, minting and multi-assets, governance proposals/votes/certificates and
treasury operations, malformed-Conway rejection, and N2C `LocalTxSubmission` + mempool handling —
including legacy-compatible transaction FORMS that Conway itself still accepts. It does **not** mean a
separate Byron transaction engine.

## RULING on the truncated Mithril schedule — do NOT paper over it

`ScheduleDoesNotCoverSystemStart` must **not** be solved by supplying an arbitrary first-segment start
time from node configuration. That would create two possible slot authorities — full era geometry
accumulated from system start, and a truncated segment plus a separately supplied absolute timestamp.
They may be mathematically equivalent, but unless the second is *derived and bound from the first*, they
are two semantic paths that can disagree. This project has spent three slices removing exactly that
shape.

### Preferred solution

The authoritative node path receives one **bootstrap-bound TIMING schedule sufficient to map UTC to the
current absolute slot** — i.e. covering system start through the active era in *timing segments only*
(see the scope section above) — constructed once through the existing era-schedule authority and **not
rebuilt inside `operator_forge.rs`**, from canonical venue inputs bootstrap already admits:

- Byron genesis start + slot geometry (timing only: start time, 20s slot length, segment length);
- Shelley system start + slot geometry (timing only: 1s slot length);
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

Without those proofs, **reject that design.** Note what is NOT rejected: a compact canonical timing
anchor *derived from and bound to* the genesis timing history is admissible — the objection is to a
second, independently-supplied clock authority, not to compactness.

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
| **CE-L2c-1** | `--mode node` calls the shared `EraSchedule::slot_at` | **MET** (`747b01ae`) — via `BootstrapBoundTimingAuthority::slot_at`, which projects the same accumulated geometry; ONE call site, gate-counted by occurrence |
| **CE-L2c-2** | The naive `system_start + single slot length` conversion is UNREACHABLE from forging | **MET** (`747b01ae`) — `checked_millis_to_slot` + `SlotAlignmentError` DELETED; `millis_to_slot` survives only on a test-reachable orchestrator path, gate-pinned |
| **CE-L2c-3** | Native-Mithril warm start supplies a schedule covering system start | **MET** (`8dd36d1c`) — reconstructed from the committed venue calendar selected by the DURABLE genesis hash, bound to the store's epoch geometry; timing facts only, no Byron ledger semantics |
| **CE-L2c-4** | A truncated snapshot-only schedule returns `ScheduleDoesNotCoverSystemStart` | **MET** — `refusals_are_structured` + `a_truncated_calendar_cannot_establish_a_timing_authority`; the guard is pinned on BOTH conversion directions (`slot_at` and `slot_start_time_ms`) |
| **CE-L2c-5** | The preserved preprod instant produces slot **130,338,561** through the ACTUAL node wiring | **MET** (`747b01ae`) — `ce_l2c_5_and_6_live_instant_derives_the_measured_slot_and_refuses_typed` drives the real loop and reads the slot back out of the typed refusal, so it is proven to REACH the KES gate, not merely be computed |
| **CE-L2c-6** | B11 returns a typed `ForgeRefused`, never a skip | **MET** (`747b01ae`) — `ForgeRefused::KesWindow` + three distinct emitted reasons + `ForgeOutcome::Refused` |
| **CE-L2c-7** | The configured pool reaches `classify_leader_schedule` | **open — blocked on B12**, not on this slice's parts |
| **CE-L2c-8** | The branch marker proves the KNOWN operator pool was evaluated | **open — blocked on B12** |
| **CE-L2c-9** | The live run emits a decided `ForgeOutcome` | **open — blocked on B12** |
| **CE-L2c-10** | Replaying instant + schedule + anchor + authority inputs reproduces the same slot and outcome | **MET** (`8dd36d1c`) — `ce_l2c_a3_reconstruction_is_byte_identical_and_replayable`, `reconstruction_is_byte_identical_across_restarts` |
| **CE-L2c-11** | Negative-tested (seven mutations below) | **MET** — nine mutations run, all caught; the gate itself negative-tested 11 ways (which found two real weaknesses in it) |

### CE-L2c-A1..A5 — the activation criteria

| CE | Criterion | status |
|---|---|---|
| **CE-L2c-A1** | The calendar is reconstructed only from bootstrap-verified inputs and is selected by the DURABLE genesis hash; no CLI value can override it | **MET** — `the_operator_cannot_choose_the_calendar` (a `--network` naming another venue is terminal; an absent one is simply no cross-check) |
| **CE-L2c-A2** | The reconstruction reproduces the durable bootstrap facts or refuses | **MET** — preprod 304 ⇒ 129_686_400; Byron-dropped ⇒ 131_328_000 ⇒ refusal |
| **CE-L2c-A3** | Byte-identical anchor across reconstructions; warm start verifies lineage before forging | **MET** — plus a forge-ON start with no sidecar FAILS CLOSED |
| **CE-L2c-A4** | An altered timing schedule is rejected | **MET** — both a changed slot length (commitment moves, answer moves) and a shifted system start |
| **CE-L2c-A5** | An admitted tick never reports a reason that is not its own | **MET** — `ce_l2c_a5_a_refusal_never_outlives_its_own_tick` |

**Recorded limit** (a test, not an assumption): the durable binding pins segment BOUNDARIES, not slot
DURATIONS. A calendar with correct boundaries but a wrong historical slot length reproduces the
store's epoch geometry exactly and still mis-converts by the full 1_641_600 slots. Durations are held
by the committed genesis-hash-selected table plus a fail-closed cross-check of the ACTIVE segment
against the operator's real `shelley-genesis.json` — the same standing `security_param` /
`active_slots_coeff` / `epoch_length` already have in the profile registry.

### Why CE-L2c-7/8/9 are blocked on something this slice did not cause

Decomposing run 4's own artifacts (see `SLICE-LIVE-2c-ACTIVATION-handoff.md` §M2) found a **seventh**
exit, **B12**: the DC-NODE-15 catch-up gate refused all 354 admitted ticks with
`local_tip_block_no − peer_tip_block_no == +1` in 354/354 samples, because the peer-advertised tip
lags Ade's durable tip by exactly the block just delivered. Leadership is evaluated *downstream* of
that gate, so a correct slot cannot reach it until B12 is resolved. B12 is deliberately not fixed
here — changing a DC-NODE-15 operand or predicate is consensus-adjacent and needs its own census.
| **CE-L2c-12** | **Scope guard**: holding timing fields constant, arbitrary changes to `era`, `start_epoch`, `epoch_length_slots`, `safe_zone_slots` and the RSW must NOT change `slot_at` for any captured instant | **MET** — `non_timing_fields_cannot_influence_slot_derivation`, negative-tested three ways (leak `epoch_length_slots`; branch on era identity; leak `safe_zone_slots`) |
| **CE-L2c-13** | **Compact-anchor equivalence**: `full_timing_history.slot_at(t) == compact_anchor.slot_at(t)` for EVERY instant in the compact anchor's declared domain, including its FIRST instant and every transition edge | **MET** — `ce_l2c_13_compact_anchor_equals_full_history_over_its_domain` (`352dfb95`): nine domain starts spanning the transition, each probed at its first admissible instant, every edge and both sides, the live fixture, and a 200-point 997ms sweep; four mutations caught |

Required mutations: restore the naive conversion; hardcode the preprod boundary; accept a truncated
schedule as complete; node path bypasses `slot_at`; B11 restored to `None`; diagnostic path fixed while
the node path stays old; known-pool evaluation replaced by `UnknownPool`.

### CE-L2c-14 — domain-start determinism (CLOSED, `8cf14529`)

| CE | Criterion | status |
|---|---|---|
| **CE-L2c-14** | The anchor domain comes from a canonical bootstrap FACT — never process start, wall clock, peer tip or operator input — so `same bootstrap anchor + same timing history ⇒ byte-identical anchor` | **MET** — `derive_for_bootstrap_anchor` takes a `SlotNo` and no timestamp; `anchor_is_reproducible_from_the_bootstrap_slot_not_the_clock` also asserts the CONTRAST (two clock-derived domains 1s apart differ) |
| **CE-L2c-15** | `slot_start_time_ms` is the exact inverse of `slot_at` on slot boundaries, across transitions | **MET** — `slot_start_time_is_the_inverse_of_slot_at`; the shared accumulated geometry is what makes the mid-slot origin bug structurally impossible via this constructor |

Mutation note: removing the derive-time round-trip guard is an **observationally equivalent** edit when
both conversions are correct (the inverse-drift mutation covers that condition directly), so its
survival is a correct non-catch, not missing coverage. The guard stays as defence in depth.

### CE-L2c-13 — the equivalence proof the compact anchor must carry

Proof obligation 4 of the six-proof bar ("replay proves it yields the same slots as the complete
schedule over its domain") is stated here as a mechanical property so it cannot be satisfied by
argument:

```
for every instant t in the compact anchor's DECLARED domain:
    slot_at(full_timing_history, t) == slot_at(compact_anchor, t)
```

Must include the domain's **first instant** and **every transition edge** it spans — those are where a
derived anchor's rounding or off-by-one would hide, and they are exactly the instants an
interior-only sample would miss.

**Not written yet, deliberately**: the compact anchor type does not exist. Note the current shape makes
any compact form *inexpressible* — `slot_at` refuses a schedule whose first era does not start at slot 0
(`ScheduleDoesNotCoverSystemStart`). That refusal is not an obstacle to route around; it IS the design
constraint the anchor must satisfy, by being derived from the full history rather than by relaxing the
check.

## ACTIVATION SLICE — the remaining authority handoff, in three connected parts

Everything above is closed. What remains is one handoff, deliberately kept together because splitting
it would leave two reachable slot authorities mid-way.

**1. Bootstrap binding.** Construct the complete timing history ONLY from already-verified venue
inputs, then derive the anchor from the verified bootstrap slot. Required result:
`same bootstrap lineage + same timing inputs ⇒ byte-identical timing anchor`. Warm start must restore
or reconstruct that same authority **and verify its source commitment before forging becomes active**.

**2. Producer wiring.** `--mode node` receives the derived anchor and calls `anchor.slot_at(captured_ms)`.
The naive triple must be **removed or made unreachable** — merely *preferring* the new path is
insufficient, because two reachable slot authorities recreate the original defect class.

**3. B11 closure.** `Result<KesPeriod, KesSlotError>` propagated as `ForgeRefused::KesWindow(..)`.
Completed invariant: **every admitted `ForgeTick` produces either a structured refusal or a
leader-schedule decision. No admitted tick may disappear.**

### Closure criteria for activation

timing history bootstrap-bound · anchor domain from the verified bootstrap slot · warm restart verifies
identical lineage · an altered timing schedule is rejected · `--mode node` uses ONLY the derived anchor ·
the naive conversion is unreachable · B11 emits a typed refusal · a live corrected slot reaches the
configured pool's leadership evaluation · a decided `ForgeOutcome` is emitted · replay of the captured
instant and restored authority reproduces the same slot and outcome.

## Store-semantics decision — reassess DURING wiring

Keeping v3 was correct for `31f7c754`. For this slice:

- **No bump** if the full schedule is reconstructed solely from already bootstrap-bound genesis/era
  inputs and no persisted format or interpretation changes.
- **Bump required** if the snapshot, checkpoint, WAL, store metadata or fingerprint gains a schedule
  commitment, **or** if an existing persisted schedule field changes meaning from "snapshot-local" to
  "absolute slot authority".

Sharpened: **do not pre-commit to a bump.** Reconstructing the anchor from existing durable bootstrap
inputs can remain store-neutral. *Persisting* a new anchor or commitment versions THAT ARTIFACT. A
global `STORE_SEMANTICS_VERSION` bump is required only if existing stored data gains a new
interpretation or recovery behaviour changes — not because forging now uses the correct slot.

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
