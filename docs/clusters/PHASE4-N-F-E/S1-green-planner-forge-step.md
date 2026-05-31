# PHASE4-N-F-E — Slice S1: GREEN planner forge step

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S1 row) and
> `../../planning/phase4-n-f-e-cluster-slice-plan.md`. Code-verified against
> HEAD `c875655` at authoring.

> **Slice S1 in one line:** extend the pure GREEN planner with a closed
> `ForgeTick` step + a content-blind `ForgeSlotStatus { Due | NotDue }` input +
> the pure forge-slot monotonic guard, so the RED loop (S2) can attempt a forge
> tick that the planner schedules but never authorizes.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-E (forge-tick on the relay spine, hermetic).
- **Slice:** S1 — GREEN planner forge step.
- **Module (extended):** `crates/ade_node::run_loop_planner` (GREEN-by-content,
  `//! GREEN` banner). Existing from N-F-D S1.
- **Lands tested-but-unwired** — S2 wires the new step into `run_relay_loop`.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-E-1** — `plan_loop_step` returns the closed `LoopStep { SyncOnce,
  ForgeTick, Idle, HaltCleanly }` over closed inputs. Planner step selection may
  receive only a content-blind `ForgeSlotStatus { Due | NotDue }`. The pure
  monotonic guard may consume `SlotNo` values, but `plan_loop_step` itself must
  not carry `SlotNo`, `ChainTip`, block identity, leader status, KES validity,
  or forge eligibility. Precedence shutdown→sync→forge→idle; pure/total.
- **CE-E-2** — the forge-slot monotonic guard is pure and forges at most once
  per `SlotNo`, never ≤ the last forged slot.

(CE-E-3..7 are out of S1 scope — S2/S3a/S3b.)

## 3. Intent (invariant impact)
Introduces the **planner half + monotonic guard of `DC-NODE-05`**: forge-slot
*scheduling* becomes a pure, total, deterministic decision over closed
lifecycle inputs, and forge-slot *monotonicity* (at most once per `SlotNo`,
never a past slot) becomes a pure guard — both unable to encode or observe an
authority decision. Leadership/forge eligibility stays entirely outside the
planner (BLUE `forge_one_from_recovered`, reached only in S2). The closed
`ForgeSlotStatus` input makes "the planner decided to forge because it's a
leader slot" structurally unrepresentable — the planner only sees `Due | NotDue`.

## 4. Pre-conditions
- N-F-D closed (`7de1462`): `run_loop_planner` exists with `plan_loop_step(loop_state,
  sync_status, shutdown_status) -> LoopStep` over `{ SyncOnce, Idle, HaltCleanly }`;
  `ci_check_loop_planner_closed.sh` green.
- `DC-NODE-05` is `declared` in the registry (sketch, `de497c4`).
- No new dependency beyond `ade_types::SlotNo` (for the guard).

## 5. Implementation boundary
- **Closed output (extended):** `enum LoopStep { SyncOnce, ForgeTick, Idle,
  HaltCleanly }` — no `#[non_exhaustive]`. `ForgeTick` is the N-F-E addition.
- **Closed input (new):** `enum ForgeSlotStatus { Due, NotDue }` — content-blind;
  carries no slot/hash/tip/verdict/leader/KES payload.
- **Extended signature:** `pub fn plan_loop_step(loop_state: LoopState,
  sync_status: SyncStatus, forge_slot_status: ForgeSlotStatus, shutdown_status:
  ShutdownStatus) -> LoopStep`. Pure: no `&self`, no I/O, no clock, no `await`,
  no allocation.
- **Decision table (precedence: shutdown → sync → terminal feed-end → forge →
  idle; 16 total):**

  | `shutdown_status` | `sync_status` | `forge_slot_status` | `loop_state` | → `LoopStep` |
  |---|---|---|---|---|
  | `ShutdownRequested` | * | * | * | `HaltCleanly` |
  | `Running` | `WorkAvailable` | * | * | `SyncOnce` |
  | `Running` | `NoWorkReady` | * | `Ending` | `HaltCleanly` |
  | `Running` | `NoWorkReady` | `Due` | `Continuing` | `ForgeTick` |
  | `Running` | `NoWorkReady` | `NotDue` | `Continuing` | `Idle` |

  Rationale: shutdown halts promptly; available relay work drains first (produce
  subordinate to the sync spine); **a terminal feed-end (`Ending`) halts cleanly
  even if a forge slot is due — the loop must not forge after the input feed is
  exhausted**, so the loop's terminal behavior never depends on a producer branch
  N-F-D did not have; only on an open/live feed (`Continuing`) does a due forge
  slot fire, else idle. **Reduction property (load-bearing for CE-E-5):** when
  `forge_slot_status ≡ NotDue` (forge off / no producer material), the table
  collapses **exactly** to the N-F-D 3-input mapping. Total over all 2×2×2×2 = 16
  input combinations by precedence (exhaustive `match`, no wildcard arm).
- **Monotonic guard (new, pure, consumes `SlotNo`):** `pub fn forge_slot_status(
  last_forged_slot: Option<SlotNo>, current_slot: SlotNo) -> ForgeSlotStatus` —
  `Due` iff `last_forged_slot.is_none() || current_slot > last`; else `NotDue`
  (equal slot already forged ⇒ `NotDue`; past slot ⇒ `NotDue`). Pure; the only
  function in the module that references `SlotNo`. (Producer-active gating —
  return `NotDue` when no producer material — is the RED caller's job in S2, not
  the guard's.)

## 6. TCB color
- **GREEN:** `ade_node::run_loop_planner` (extended — the `ForgeTick` variant,
  `ForgeSlotStatus` enum, the extended `plan_loop_step`, the `forge_slot_status`
  guard; `//! GREEN` banner). No BLUE change; no RED behavior; no `await`/I/O/clock.

## 7. Invariants preserved (must not weaken)
- `CN-NODE-02` (planner half) — the planner stays closed, pure, authority-free;
  adding `ForgeTick`/`ForgeSlotStatus` does not let it encode a ledger /
  chain-selection / leadership / forge-eligibility / evidence decision.
- `T-REC-03` — `plan_loop_step` stays total/deterministic (same inputs ⇒ same
  `LoopStep`); the N-F-D 3-input mapping is preserved under the `NotDue` reduction.
- All BLUE invariants — untouched (no BLUE crate referenced).

## 8. Invariants strengthened (one family: DC-NODE-05)
- `DC-NODE-05` (`declared`) — this slice lands its **planner + monotonic-guard
  half**: forge-slot scheduling is a pure closed decision; forge-slot
  monotonicity (at most once per `SlotNo`, never a past slot) is a pure guard.
  (Flips `declared → enforced` only at cluster close, when CE-E-1..7 are all
  green; S1 contributes CE-E-1 + CE-E-2.) Recording `strengthened_in +=
  "PHASE4-N-F-E"` on `CN-NODE-02` is deferred to S2 (the constraint's enforcement
  gate evolves there); S1 only extends the planner the gate guards.

## 9. Replay / determinism obligations
- `plan_loop_step` and `forge_slot_status` are total pure functions: same inputs
  ⇒ byte-identical outputs. No corpus entry; proven by unit tests. This is the
  determinism precondition the forge-tick replay-equivalence (S3a / CE-E-6)
  builds on.

## 10. Mechanical acceptance criteria
- [ ] `LoopStep` gains `ForgeTick`; `ForgeSlotStatus { Due, NotDue }` added; both
      closed (no `#[non_exhaustive]`); `plan_loop_step` extended with the
      `forge_slot_status` param; `match` exhaustive, no wildcard arm.
- [ ] Test `plan_loop_step_forge_precedence_table_is_total` asserts the mapping
      for all 16 input combinations (the §5 table). In particular `Running +
      NoWorkReady + Due + Ending → HaltCleanly` (forge suppressed at feed-end),
      and `ForgeTick` occurs **only** for `Running + NoWorkReady + Due +
      Continuing`.
- [ ] Test `plan_loop_step_reduces_to_relay_table_when_forge_notdue` asserts that
      with `forge_slot_status = NotDue`, all 8 remaining combinations equal the
      N-F-D mapping (CE-E-5 reduction precondition).
- [ ] Test `forge_suppressed_when_feed_ending` asserts `Running + NoWorkReady +
      Due + Ending → HaltCleanly`.
- [ ] Test `forge_slot_guard_at_most_once_per_slot` asserts `current == last ⇒
      NotDue` and `current > last ⇒ Due`.
- [ ] Test `forge_slot_guard_rejects_past_slot` asserts `current < last ⇒ NotDue`;
      `forge_slot_guard_none_is_due` asserts `None ⇒ Due`.
- [ ] Test `plan_loop_step_is_deterministic` (extended) — repeated calls with
      identical 4-tuples return identical `LoopStep`.
- [ ] **Gate evolution** `ci/ci_check_loop_planner_closed.sh` (exit 0): the
      whole-module hard-token ban is retained (`pump_block`, `run_node_sync`,
      `run_real_forge`, `forge_one_from_recovered`, `correlate`, `Ba02Manifest`,
      `ChainDb`, `LedgerState`, `BlockHash`, `ChainTip`, `PumpTip`, `put_block`,
      `AdvanceTip`, `rollback_to_slot`, `std::fs`, `tokio`, `SystemTime`,
      `Instant`, `HashMap`, `await`, no wildcard arm); the `SlotNo` ban is
      **scoped to the `plan_loop_step` function body** (the `forge_slot_status`
      guard may reference `SlotNo`); `LoopStep` + `plan_loop_step` still defined;
      `//! GREEN` banner present.
- [ ] `cargo build -p ade_node` clean; `cargo test -p ade_node run_loop_planner`
      green (count > 0); `rustfmt --edition 2021 crates/ade_node/src/run_loop_planner.rs`;
      the evolved gate passes.

## 11. Forbidden in this slice (inherits the cluster Forbidden list)
- No I/O, no clock, no `await`, no allocation in the planner.
- No authority/forge token in `plan_loop_step` (the gate's whole-module ban,
  minus the `SlotNo` exception scoped to the guard).
- `plan_loop_step` must not take or return `SlotNo`/`ChainTip`/block identity/
  leader status/KES validity/forge eligibility — only the closed `ForgeSlotStatus`.
- No wildcard `match` arm; no `#[non_exhaustive]` on any planner type.
- No wiring into `run_relay_loop` / the binary run path (that is S2).
- No new canonical type / WAL entry / checkpoint format / BLUE change.

## 12. Slice completion checklist
- [ ] `run_loop_planner.rs` extended (`ForgeTick`, `ForgeSlotStatus`,
      `plan_loop_step` param, `forge_slot_status` guard, the tests).
- [ ] `ci/ci_check_loop_planner_closed.sh` evolved (scoped `SlotNo` ban),
      executable, exits 0.
- [ ] `cargo build/test -p ade_node` green; `rustfmt` applied; gate passes.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl
      committed (`feat:`) after green.

## Authority
Registry IDs `DC-NODE-05` (planner + guard half; `declared`), `CN-NODE-02`
(planner half, preserved), `T-REC-03` (determinism precondition, preserved). The
cluster doc `cluster.md` and `docs/ade-invariant-registry.toml` are
authoritative; this slice doc refines, it does not override.
