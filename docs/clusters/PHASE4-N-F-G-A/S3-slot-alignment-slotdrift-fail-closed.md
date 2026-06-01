# PHASE4-N-F-G-A — Slice S3: Slot alignment + SlotDrift fail-closed

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S3 row + CE-G-A-3). Code-verified
> against HEAD `11704998`.
>
> **Slice S3 in one line:** on the `--mode node` forge path, derive the forge slot from the
> wall-clock **only** through the single RED clock seam over the **real** S2-parsed genesis anchor,
> and **fail closed** (no forge) when the clock→slot alignment is implausible — instead of letting
> `millis_to_slot` saturate a before-anchor wall-clock to slot 0 and silently forge it.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-G-A (forge fidelity). Gated behind S1/S2a/S2 (green). S4 owns the
  *epoch-boundary* fail-closed (a different axis); S3 owns only the clock→slot alignment axis.
- **Slice:** S3 — slot alignment + SlotDrift fail-closed (the clock→slot plausibility guard).
- **Modules:** **GREEN** `ade_runtime::clock` (a new pure `checked_millis_to_slot` guard alongside
  the existing saturating `millis_to_slot`); **RED** `ade_node::node_lifecycle` (the clock-seam
  integration: the forge tick consumes the checked guard and fails closed on misalignment). **No
  BLUE change.**

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-G-A-3** — slot alignment: candidate tests
  `node_forge_slot_via_millis_to_slot_over_real_genesis_anchor`, `node_forge_slot_drift_fails_closed`;
  existing `ci_check_clock_seam.sh` + `ci_check_forbidden_patterns.sh` hold (no
  `SystemTime`/`Instant`/float past the seam). *(strengthens `DC-NODE-03`.)*

(CE-G-A-1 = S1, CE-G-A-2a = S2a, CE-G-A-2 = S2 — done; CE-G-A-4 = S4 epoch-boundary — out of S3 scope.)

## 3. Intent (invariant impact)
Make it **impossible for the node forge to produce a block for a slot whose wall-clock→slot
alignment is implausible.** Today `millis_to_slot` (`clock.rs:88-93`) `saturating_sub`s the anchor:
a wall-clock **before** the genesis `systemStart` collapses to slot 0 and the forge attempts it — a
silent, drifted forge. S3 closes that: the node forge path derives its slot through a **checked**
clock-seam map over the real S2 genesis anchor, and a wall-clock that cannot be exactly aligned
(before the anchor / unsupported) yields a **structured fail-closed outcome** (no forge, surfaced
through the existing hermetic forge-outcome/test observation surface or an explicitly structured
local node-forge error, without persistence or evidence overclaim), while the `(genesis-anchor,
wall_clock_millis) → SlotNo` map stays pure and deterministic. This is the clock-alignment
plausibility wall; the epoch-boundary wall is S4's.

## 4. Pre-conditions (verified at HEAD `11704998`)
- **The seam + the saturation:** `clock::millis_to_slot(tick, start_millis, start_slot,
  slot_length_ms)` (`clock.rs:82-94`) is pure but **saturates**: `delta = tick.saturating_sub(
  start_millis)`; a before-anchor `tick` → `delta 0` → `start_slot`. `clock::SystemClock` (`:107`)
  is the sole RED wall-clock observation (`ci_check_clock_seam.sh` enforces `clock.rs` as the only
  `SystemTime`/`Instant` site).
- **The forge slot derivation (no guard today):** `node_lifecycle.rs:631-650` — `act.clock
  .next_tick()` → `now_ms` → `millis_to_slot(now_ms, act.anchor_millis, act.start_slot,
  act.slot_length_ms)` → `slot` → `forge_slot_status(act.last_forged_slot, slot)`. Only `SlotNo`
  crosses into the GREEN planner.
- **The anchor is now real (S2):** `act.anchor_millis` / `act.slot_length_ms` come from
  `parse_shelley_genesis` (`systemStart`→ms, `slotLength`→ms) via `build_operator_forge_material` —
  no longer the simple-JSON genesis.
- **The planner is a closed table:** `ForgeSlotStatus` (`run_loop_planner.rs:68`) = `Due | NotDue`;
  `plan_loop_step` is total (`Due→ForgeTick`, `NotDue→Idle`), enforced by
  `ci_check_loop_planner_closed.sh`. `forge_slot_status` (`:162`) is the DC-NODE-05 monotonic guard
  (at most once per slot, never past).
- **The produce path swallows drift (the contrast):** `producer::coordinator::CoordinatorError::
  SlotDrift{from,to}` (`coordinator.rs:254`, raised `:395`) is swallowed on the produce tip path
  (C1 §1d). S3 makes the **node** path fail closed instead — it does **not** reuse or touch the
  produce coordinator.

## 5. The fix (a checked clock-seam guard, fail-closed at the RED seam)
1. **GREEN guard** (`clock.rs`): add `checked_millis_to_slot(tick, start_millis, start_slot,
   slot_length_ms) -> Result<SlotNo, SlotAlignmentError>` alongside `millis_to_slot`. It returns
   `Err(SlotAlignmentError::BeforeGenesisAnchor)` when `tick < start_millis` (the exact case the
   saturating map masks), and otherwise the **exact** `millis_to_slot` result. Pure integer
   arithmetic; the existing saturating `millis_to_slot` is unchanged (the produce path keeps it).
2. **RED clock-seam integration** (`node_lifecycle.rs`): the forge tick consumes
   `checked_millis_to_slot`. On `Ok(slot)` → the current `forge_slot_status` path (unchanged). On
   `Err` → a **structured fail-closed outcome at the clock-seam site**: the forge does not attempt,
   `last_forged_slot` is **not** advanced, and the misalignment is surfaced through the existing
   hermetic forge-outcome / test observation surface (or an explicitly structured local node-forge
   error) — no new durable log/event vocabulary, no persistence, no BA-02/RO-LIVE overclaim. The
   `ForgeSlotStatus` planner input is `NotDue` for that tick, so `plan_loop_step` stays the frozen
   2-variant table; the relay loop continues syncing (the forge stays subordinate to the sync
   spine, DC-NODE-05).

## 6. TCB color (execution boundary)
- **GREEN (extended):** `ade_runtime::clock::checked_millis_to_slot` (pure guard) + the
  `SlotAlignmentError` sum — deterministic, no I/O, no clock/rand/float.
- **RED (integrated):** `ade_node::node_lifecycle` forge-tick slot derivation — the single RED
  `SystemClock` seam; only `SlotNo` (or the fail-closed signal) crosses. No new wall-clock site.
- **BLUE:** none — no authoritative transition, no canonical type. A BLUE change is a red flag →
  reject.

## 7. Invariants preserved (must not weaken) — by registry ID
- `DC-NODE-05` — forge-slot discipline (at most once per slot, never past, subordinate to the sync
  spine, no durable-tip advance): the monotonic guard + containment are untouched; the new
  fail-closed path advances no tip and forges nothing.
- `CN-NODE-02` / `DC-SYNC-02` — the relay run-loop + `run_node_sync` durable-tip authority
  unchanged; the fail-closed forge tick does not affect the sync spine.
- `T-DET-01` — no determinism tripwire introduced (the guard is pure; the lone wall-clock
  observation stays the single `SystemClock` seam).
- `CN-NODE-01` — no second bootstrap / recovery path.
- The loop-planner closure (`ci_check_loop_planner_closed.sh`) and the relay-loop containment gate
  (`ci_check_node_run_loop_containment.sh`) stay green/unchanged.

## 8. Invariants strengthened (one family: node clock→slot alignment fail-closed)
**Family:** *the node forge slot is derived only through the clock seam over the real genesis
anchor, and an implausible clock→slot alignment (before the genesis anchor / unsupported) fails
closed — never a saturation-masked slot-0 forge.*
- `DC-NODE-03` — the clock-seam slot derivation gains the fail-closed plausibility guard over the
  real S2 genesis anchor (was: derive-via-seam only). `strengthened_in += "PHASE4-N-F-G-A"`.
- **No registry edit in this slice** (deferred to G-A close, per the S1/S2/S2a pattern). No status
  flip in S3.

## 9. Slice-entry decisions (settled)
- **D-1 — fail-closed integration shape (DECIDED: option B).** Handle the `Err` at the RED
  clock-seam site (`node_lifecycle`): surface a structured local node-forge fail-closed through the
  existing hermetic forge-outcome / test observation surface (or an explicitly structured local
  error) and skip the forge, keeping `ForgeSlotStatus` the frozen 2-variant `Due | NotDue`.
  Alignment failure is a RED clock-seam validation failure, **not** a lifecycle planning decision —
  the planner gains no third state. No new durable log/event vocabulary, no persistence, no
  BA-02/RO-LIVE evidence overclaim.
- **D-2 — what is "unsupported alignment."** The clear, in-scope case is `tick < anchor_millis`
  (before genesis). `slot_length_ms == 0` cannot occur (build-time `.max(1)`). **No** epoch-range
  check here — a slot in the wrong epoch is S4's fail-closed (CE-G-A-4), not S3's.

## 10. Replay / determinism obligations
`checked_millis_to_slot` is a pure function of `(tick, anchor, start_slot, slot_length_ms)` — same
inputs → same `Ok(SlotNo)` or same `Err`. No wall-clock/rand/float in the guard (the lone
observation is the existing RED `SystemClock`). For a fixed injected clock tick schedule + recovered
state, the forge-attempt-or-fail-closed sequence is byte-identical across runs (extends DC-NODE-05's
replay clause). No new authoritative state, no new canonical type, no WAL/checkpoint change.

## 11. Replay / crash / epoch validation (tests by name)
- **New (the guard, in `clock` tests):**
  - `checked_millis_to_slot_matches_millis_to_slot_when_aligned` — for `tick >= anchor`, the checked
    map equals the saturating map (exact derivation).
  - `checked_millis_to_slot_before_anchor_fails_closed` — `tick < anchor` → `Err(BeforeGenesisAnchor)`
    (where the saturating map would return slot 0).
- **New (the node forge path):**
  - `node_forge_slot_via_millis_to_slot_over_real_genesis_anchor` — the forge slot equals the
    clock-seam `checked_millis_to_slot` over the real (S2 genesis) anchor for an aligned wall-clock
    (pins the derivation source).
  - `node_forge_slot_drift_fails_closed` — a before-anchor injected clock tick yields a **structured
    fail-closed outcome** (no forge, no `last_forged_slot` advance, surfaced via the existing
    hermetic outcome / structured local error — no new durable evidence), not a slot-0 forge.
- **No epoch/crash semantics changed** — epoch-boundary fail-closed is S4; crash recovery is
  N-F-A/N-F-D's domain.

## 12. Mechanical acceptance criteria
- [ ] `cargo test -p ade_runtime --lib clock` (the two `checked_millis_to_slot` tests) green,
      two-run-stable.
- [ ] `cargo test -p ade_node --lib` (`node_forge_slot_via_millis_to_slot_over_real_genesis_anchor`,
      `node_forge_slot_drift_fails_closed`) green.
- [ ] `ci_check_clock_seam.sh` + `ci_check_forbidden_patterns.sh` green (no new
      `SystemTime`/`Instant`/float past the seam).
- [ ] `ci_check_loop_planner_closed.sh` + `ci_check_node_run_loop_containment.sh` green (planner
      totality + relay-loop containment unchanged).
- [ ] `cargo build` + `cargo clippy` clean on touched crates; **`rustfmt` applied to the changed
      files only** (no workspace `cargo fmt -p` — the repo is not fmt-maintained).
- [ ] Acceptance scoped to touched crates (`ade_runtime`, `ade_node`) — not the full `ade_testkit`
      corpus lane.

## 13. Failure modes
All **fail-fast / fail-closed** (structured, visible without overclaim):
- Wall-clock before the genesis anchor → `SlotAlignmentError::BeforeGenesisAnchor` → forge tick
  fails closed (no forge; surfaced via the existing hermetic outcome / structured local error),
  relay loop continues.
- No silent saturation-masked slot-0 forge may occur on the node path.
- The fail-closed forge tick affects no durable tip, serves/admits/gossips nothing (DC-NODE-05).

## 14. Hard prohibitions (inherits the cluster "Forbidden during this cluster" list)
- **No fallback to a local system slot interpretation outside the clock seam** — the wall-clock→slot
  map is the single `clock` seam; no ad-hoc slot math.
- **No `SystemTime`/`Instant` past the RED seam**; the lone observation stays `SystemClock`.
- **No floating point** anywhere in the guard or its integration.
- **No default genesis constants** — the anchor is the real S2-parsed genesis (`anchor_millis`/
  `slot_length_ms`); no hardcoded fallback.
- **No forge if slot alignment cannot be proven** — an unprovable alignment fails closed, never
  forges.
- **No planner variant change** — `ForgeSlotStatus` stays the frozen 2-variant `Due | NotDue`.
- **No relay-containment relaxation** — `ci_check_node_run_loop_containment.sh` stays unchanged.
- No **new clock observation** — the lone RED `SystemClock` seam is the only wall-clock site.
- No **epoch-boundary / nonce** work beyond identifying drift/unsupported slot (S4 owns DC-EPOCH-03).
- No **serve / serve-handoff / live-feed / WirePump / RO-LIVE / BA-02** work (G-B/G-C); no durable
  evidence / persistence.
- No **registry edit** (strengthening deferred to cluster close).
- **Hard line:** if the guard needs a BLUE change, a second clock observation, a planner variant, or
  serve/live wiring — **stop and re-scope.**

## 15. Explicit non-goals
No S4 epoch-boundary fail-closed (DC-EPOCH-03). No nonce / `EpochBoundary` work. No serve/live/
operator-pass. No produce-path change (the produce coordinator keeps its swallowing `SlotDrift`). No
new clock observation site. No change to the saturating `millis_to_slot` (the produce path keeps it).

## 16. Completion checklist
- [ ] `checked_millis_to_slot` + `SlotAlignmentError` added (GREEN, pure); the node forge tick
      consumes it and fails closed (surfaced, no overclaim) on misalignment; the saturating
      `millis_to_slot` is untouched; `ForgeSlotStatus` stays 2-variant.
- [ ] All §12 tests green; clock-seam + forbidden-patterns + loop-planner + containment gates green
      & unchanged; `cargo test` scoped to touched crates green; `clippy` clean; changed files
      rustfmt'd (no workspace fmt).
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl committed (`feat:`/
      `test:`) after green, model-attribution trailer. **No registry edit** (deferred to cluster
      close).

## Authority
Registry IDs `DC-NODE-03` (strengthened — clock-seam slot derivation fail-closed over the real
genesis anchor; registry append **deferred to cluster close**), `DC-NODE-05` / `CN-NODE-02` /
`DC-SYNC-02` / `T-DET-01` / `CN-NODE-01` (preserved). The cluster doc `cluster.md` and
`docs/ade-invariant-registry.toml` are authoritative; this slice doc refines, it does not override.
