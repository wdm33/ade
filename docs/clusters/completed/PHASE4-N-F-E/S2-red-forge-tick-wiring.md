# PHASE4-N-F-E — Slice S2: RED forge-tick wiring (self-accept-only)

> **Status:** slice doc (IDD Part IV). Companion to `cluster.md` (S2 row) and
> `../../planning/phase4-n-f-e-cluster-slice-plan.md`. Code-verified against
> HEAD `2980861` at authoring (S1 = `214b0d3`, hotfix = `ffa76fc`).

> **Slice S2 in one line:** wire the planned `ForgeTick` branch into
> `run_relay_loop` as self-accept-only RED orchestration — clock seam → `SlotNo`
> → `kes_period_for_slot` → exactly one fenced `forge_one_from_recovered` call,
> producing a local self-accept outcome only, advancing no tip and serving
> nothing.

## 1. Slice identity
- **Cluster:** PHASE4-N-F-E (forge-tick on the relay spine, hermetic).
- **Slice:** S2 — RED forge-tick wiring.
- **Module (extended):** `crates/ade_node::node_lifecycle` (RED `run_relay_loop`).
- **Gate (evolved):** `ci/ci_check_node_run_loop_containment.sh`.
- **Reuses unchanged:** `run_loop_planner` (S1), `clock::{Clock, millis_to_slot}`,
  `producer::coordinator::kes_period_for_slot`, `node_sync::forge_one_from_recovered`.

## 2. Cluster Exit Criteria addressed (verbatim)
- **CE-E-3** — the current slot is derived only via the clock seam (`SystemClock`
  → `millis_to_slot` → `SlotNo`); no `SystemTime`/`Instant`/float past the RED
  boundary; KES period via reused `kes_period_for_slot` (pure, fail-closed).
- **CE-E-4** — `ForgeTick` is wired → `forge_one_from_recovered` on
  hermetic/fenced producer-shell material; `ForgeSucceeded` only via BLUE
  `self_accept`; recovered-surface-only leadership (guard d holds); the forge
  advances **no** durable tip and serves/admits/gossips nothing (containment
  gate evolved).
- **CE-E-5** — forge is opt-in: absent producer material preserves N-F-D
  behavior on authoritative/test-visible outputs; **no** operator-key
  file/config ingestion in `--mode node`.

(CE-E-1/CE-E-2 done in S1; CE-E-6 is S3a; CE-E-7 is S3b.)

## 3. Intent (invariant impact)
Lands the **wiring + clock-seam + no-tip half of `DC-NODE-05`**: the relay loop
gains a forge branch that is *subordinate* (sync drains first; feed-end
suppresses forge), observes wall-clock only through the existing clock seam
(only `SlotNo` crosses), derives the KES period only through the existing pure
`kes_period_for_slot`, and reaches the forge authority through exactly **one
fenced** `forge_one_from_recovered` call. The forged artifact is a **local
self-accept outcome only** — never applied, served, admitted, gossiped, or
persisted as authority. The single durable tip-advance path remains
`run_node_sync → pump_block` (CN-NODE-02 / DC-SYNC-02 hold byte-for-byte).

## 4. Pre-conditions
- S1 (`214b0d3`): planner emits the closed `{SyncOnce, ForgeTick, Idle,
  HaltCleanly}`; `forge_slot_status` guard exists; `run_relay_loop` passes
  `ForgeSlotStatus::NotDue` (forge off).
- Hotfix (`ffa76fc`): `ade_node` lib-test target compiles and is green;
  `relay_loop_two_clean_runs_byte_identical` (N-F-D T-REC-03 evidence) passes.
- Verified surfaces: `forge_one_from_recovered(recovered, selected_tip, shell,
  pool_id, pparams, era_schedule, slot, kes_period, protocol_version)`
  (node_sync.rs:378, enforced DC-CINPUT-02b); `clock::millis_to_slot` (pure);
  `clock::{Clock, SystemClock, DeterministicClock}`; `kes_period_for_slot`
  (pure, `None` past max period); `ChainDb::tip` (selected tip).

## 5. Implementation boundary
- **`run_relay_loop` gains a forge-activation parameter** (an `Option`): when
  `None`, forge is OFF — `forge_slot_status` is always `NotDue`, exact N-F-D
  behavior. When `Some`, it carries (constructed hermetically in tests, **never**
  from operator files): an injected `&dyn Clock` or equivalent clock trait
  object — hermetic tests use `DeterministicClock`; **any real wall-clock use
  remains RED and must produce only `now_millis` → `SlotNo`** (DC-NODE-03 seam) —
  plus the recovered `BootstrapState` (forge base), a `ProducerShell` (KES/VRF/
  cold custody), `pool_id: Hash28`, `pparams: ProtocolParameters`,
  `protocol_version: ProtocolVersion`, and the clock anchor (`SystemStart` +
  slot length from the existing `EraSchedule`).
- **Per-iteration forge-slot derivation (RED observe → GREEN convert):**
  `clock.now_millis()` → `millis_to_slot(now, anchor, start_slot, slot_length)`
  → current `SlotNo`. **Only `SlotNo` crosses into the planner / forge call.**
  Then `forge_slot_status(last_forged_slot, current_slot)` (S1 guard) →
  `ForgeSlotStatus` fed to `plan_loop_step`. Forge-off ⇒ `NotDue`.
- **`ForgeTick` arm:** `kes_period_for_slot(slot)` → `None` ⇒ skip (no forge;
  S3b proves fail-closed) → else exactly one `forge_one_from_recovered(...)`
  call → **return/emit the returned `CoordinatorEvent` only inside the hermetic
  loop test surface** (no new log/event vocabulary) → **update
  `last_forged_slot = current_slot` only after the `ForgeTick` arm performs its
  single permitted `forge_one_from_recovered` attempt; do not update it when
  KES-period derivation returns `None` or when forge activation is absent.**
  **No `AdvanceTip`, no `put_block`, no serve/admit/broadcast.** The tip is read
  (for `selected_tip`) but never written by this arm.
- **`run_node_sync` is UNMODIFIED**; the `SyncOnce`/`Idle`/`HaltCleanly` arms are
  unchanged from N-F-D.
- **No production operator-key ingestion** — `--mode node`'s CLI/config surface
  is not extended to load KES/VRF/cold/opcert/pool-id/pparams files (that is a
  separate RED key-ingress cluster). Forge-activation is constructed only by
  hermetic tests in S2.

## 6. TCB color
- **BLUE:** none changed (forge authority reached transitively via the fenced
  `forge_one_from_recovered`).
- **GREEN:** reused only — `millis_to_slot`, `forge_slot_status`/`plan_loop_step`
  (S1), `kes_period_for_slot`. No new GREEN.
- **RED:** `ade_node::node_lifecycle::run_relay_loop` (forge branch + recovered/
  shell threading); the injected-`Clock` wall-clock observation (only `SlotNo`
  crosses).

## 7. Invariants preserved (must not weaken)
- `CN-NODE-02` / `DC-SYNC-02` — the loop advances the durable tip ONLY via
  `run_node_sync → pump_block`; the forge arm advances no tip.
- `T-REC-03` — the N-F-D relay replay evidence
  (`relay_loop_two_clean_runs_byte_identical`) must stay green with the forge
  branch present (re-run in §10; integrity carry from `ffa76fc`).
- `DC-CINPUT-02b` / `CN-CINPUT-03` — leadership projected only from the
  recovered surface inside `forge_one_from_recovered` (guard d); S2 adds no
  bundle/literal/`--consensus-inputs-path` path.
- `CN-FORGE-01/03` — `ForgeSucceeded` only via BLUE `self_accept`; S2 calls the
  fenced fn and never `run_real_forge` directly, adding no second forge path.
- `CN-PROD-02` — KES-period purity; S2 derives `kes_period` only via
  `kes_period_for_slot`.
- All BLUE invariants — untouched (no BLUE crate referenced).

## 8. Invariants strengthened (one family: DC-NODE-05)
- `DC-NODE-05` (`declared`) — S2 lands its **wiring + clock-seam + no-tip half**
  (CE-E-3/4/5). **No registry edit in S2:** `DC-NODE-05` stays `declared`, and
  the `strengthened_in` appends for `CN-NODE-02`/`DC-SYNC-02`/`DC-NODE-03`/
  `CN-PROD-02`/`DC-CINPUT-02b` are deferred to cluster close (when CE-E-1..7 are
  all green). No status flip yet; no S3a replay claim yet.

## 9. Replay / determinism obligations
- The wall-clock observation is the lone RED nondeterminism; it is canonicalized
  to `SlotNo` before crossing any seam. The clock is **injected** (`&dyn Clock`)
  so S3a can drive a `DeterministicClock` tick schedule for replay-equivalence.
- S2 makes **no replay claim** — it proves containment + opt-in reduction only.
  Forge-tick replay-equivalence is S3a (CE-E-6).

## 10. Mechanical acceptance criteria
- [ ] `run_relay_loop` gains the forge-activation parameter; `ForgeTick` arm
      calls exactly one `forge_one_from_recovered(`; no `AdvanceTip`/`put_block`/
      `rollback_to_slot`/serve/admit/broadcast in that arm.
- [ ] Test `relay_loop_forge_tick_attempts_forge_advances_no_tip` (hermetic,
      forge-activation `Some`, injected `DeterministicClock` at a forge slot):
      the `ForgeTick` arm invokes `forge_one_from_recovered` (outcome
      `ForgeSucceeded`, `ForgeNotLeader`, or structured `ForgeFailed`, provided
      the failure is produced by the fenced forge path and the containment
      assertions still hold) and the durable tip is **unchanged** by the forge
      (only sync advances it); nothing is served/admitted/persisted as authority.
- [ ] Test `relay_loop_without_producer_material_matches_nfd_relay`
      (forge-activation `None`): tips, WAL, checkpoints, and halt behavior are
      byte-identical to the N-F-D relay run over the same feed, and **no forged
      artifact** is produced. (CE-E-5; any disabled-forge probe is
      non-authoritative and excluded from replay-visible evidence.)
- [ ] Test `relay_loop_forge_slot_derived_via_clock_seam`: the slot fed to the
      forge path equals `millis_to_slot(clock.now_millis(), …)`; no `SystemTime`/
      `Instant` is observed outside the injected `Clock`.
- [ ] **Gate evolution** `ci/ci_check_node_run_loop_containment.sh` (exit 0):
      `neg2` no longer bans `forge_one_from_recovered` (now the **one permitted**
      fenced call — assert it appears **exactly once** in the loop body); still
      bans `run_real_forge`, `correlate(`, `Ba02Manifest`; **retains** `neg1`
      (`pump_block(` direct / `.put_block(` / `AdvanceTip` / `rollback_to_slot(`),
      `neg3` (verdict/follower-as-sync), `neg4` (second bootstrap); **adds**
      no-serve tokens (`served_chain_admit` / `push_atomic` / `OutboundCommand` /
      `broadcast` / `block_fetch` serve) on the loop path.
- [ ] **Integrity re-run** (`ffa76fc` carry): `cargo test -p ade_node --lib
      relay_loop_two_clean_runs_byte_identical` stays green with the forge branch
      present (the relay spine remains verified while forge is layered on).
- [ ] `cargo build -p ade_node` clean; `cargo test -p ade_node --lib` green
      (full target, count unchanged-or-grown, 0 failed); `rustfmt --edition 2021`
      on `node_lifecycle.rs`; the evolved gate + `ci_check_node_sync_via_pump.sh`
      + `ci_check_loop_planner_closed.sh` all pass.

## 11. Forbidden in this slice (inherits the cluster Forbidden list)
- No BLUE crate changes; no `run_node_sync` changes; no new canonical type / WAL
  entry / checkpoint format / log/event/JSONL vocabulary.
- No **durable apply** of a forged block; no `AdvanceTip`/`put_block`/
  `rollback_to_slot` from the loop.
- No **serve / broadcast / gossip / block-fetch** of a forged block; no
  `correlate`/`Ba02Manifest`.
- No `run_real_forge(` direct call — forge only via the fenced
  `forge_one_from_recovered`.
- No **operator-key file/config ingestion** in `--mode node`.
- No fabricated `SeedEpochConsensusInputs`/`pparams`/`protocol_version`/
  `pool_id`/KES literal; no `--consensus-inputs-path`/bundle token.
- No `SystemTime`/`Instant`/float/wall-clock value crossing past the injected
  `Clock` boundary; only `SlotNo` crosses the seam.
- No live peer, BA-02, RO-LIVE claim. No registry status flip. No S3a replay
  claim, no S3b fail-closed claim (those are their slices).

## 12. Slice completion checklist
- [ ] `node_lifecycle.rs` `run_relay_loop` extended (forge-activation param,
      clock-seam slot derivation, `ForgeTick` arm, the tests).
- [ ] `ci/ci_check_node_run_loop_containment.sh` evolved (permit one fenced
      forge call; retain + add prohibitions), executable, exits 0.
- [ ] `cargo build/test -p ade_node` green; the N-F-D T-REC-03 evidence re-run
      green; `rustfmt` applied; all three loop/sync/planner gates pass.
- [ ] Slice doc committed standalone (`docs:`) before implementation; impl
      committed (`feat:`) after green.

## Authority
Registry IDs `DC-NODE-05` (wiring/clock-seam/no-tip half; stays `declared`),
`CN-NODE-02`/`DC-SYNC-02`/`DC-CINPUT-02b`/`CN-CINPUT-03`/`CN-FORGE-01/03`/
`CN-PROD-02`/`DC-NODE-03`/`T-REC-03` (all preserved). The cluster doc `cluster.md`
and `docs/ade-invariant-registry.toml` are authoritative; this slice doc refines,
it does not override.
