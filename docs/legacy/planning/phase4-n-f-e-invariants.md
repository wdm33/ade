# PHASE4-N-F-E — Invariant Sketch

**Concept:** Wire the existing, already-enforced `forge_one_from_recovered`
(DC-CINPUT-02b) into the `--mode node` relay run-loop's slot path, on top of the
N-F-D relay spine. Produce is **subordinate** to the sync spine; the loop gains
no new authority.

**Cluster character (load-bearing — do not broaden):** N-F-E is a **hermetic,
single-epoch, self-accept-only forge-tick wiring cluster**. It is *not* an
operator-key-ingress cluster and *not* a live-production / peer-evidence
cluster. A forge tick may produce a self-accepted forged-block artifact and a
**local** `ForgeSucceeded` coordinator event — and nothing more.

Status: drafted at the `/invariants` gate (HEAD `7de1462`, after the N-F-D
close). Rides the enforced N-F-D relay spine (`run_relay_loop`, the GREEN
`run_loop_planner`, the `run_node_sync -> pump_block` containment gate) and the
already-enforced recovered-surface forge handoff
(`node_sync::forge_one_from_recovered`, DC-CINPUT-02b).

---

## 1. What must always be true

- **I1 — Forge-slot discipline (NEW, DC-NODE-05).** The loop attempts a forge
  **at most once per `SlotNo`**, never for a slot ≤ the last forged slot (no
  past/duplicate forge), and only at a forge-slot boundary derived
  deterministically from the injected clock. *Leadership eligibility is not
  decided here* — that stays in BLUE inside `forge_one_from_recovered`, which
  returns `ForgeNotLeader` deterministically for a non-leader slot.
- **I2 — Produce subordinate; no new tip authority (rides CN-NODE-02 /
  DC-SYNC-02).** A forge tick advances **no** durable tip, admits nothing,
  serves nothing, gossips nothing. The *only* durable tip-advance path remains
  `run_node_sync -> pump_block`. A locally forged block is a local self-accept
  artifact, not a chain mutation.
- **I3 — Recovered-surface-only leadership (rides DC-CINPUT-02b /
  CN-CINPUT-03).** The loop forge derives its leadership view **only** by
  calling the fenced `forge_one_from_recovered` — never a forge-time bundle, a
  fabricated `SeedEpochConsensusInputs` literal, or `--consensus-inputs-path`.
  The loop re-implements no forge path.
- **I4 — Slot enters as a canonical input through the clock seam (rides
  DC-NODE-03).** RED observes wall-clock (`SystemClock::now_millis`); GREEN
  converts via `millis_to_slot(now, SystemStart-anchor, slot_length-from-
  EraSchedule, start_slot) -> SlotNo`. **Only `SlotNo` crosses into the forge
  seam.** No `SystemTime` / `Instant` / float in BLUE or GREEN. N-F-E wires the
  existing `clock.rs` primitives into `--mode node`; the slot-derivation
  clause is enforced as part of DC-NODE-05 (no separate adapter rule).
- **I5 — KES-period safety (rides CN-PROD-02).** `slot -> kes_period` is a pure
  function of `(slot, genesis_kes_anchor, slots_per_kes_period)`; the loop never
  signs for a slot whose KES period has rotated past the hot key — it
  fail-closes / skips with a structured local outcome. No retroactive forge.
- **I6 — Single forge outcome (rides CN-FORGE-01/03).** A forge tick yields
  exactly one `CoordinatorEvent` (`ForgeSucceeded` / `ForgeNotLeader` /
  `ForgeFailed`); `ForgeSucceeded` only if BLUE `self_accept` accepts; the
  artifact round-trips through the single envelope codec. (Already enforced
  inside `run_real_forge`; the loop adds no second forge path.)

## 2. What must never be possible (hard prohibitions)

- **N1** — A forge tick advancing the durable tip, admitting, serving,
  gossiping, or block-fetching a forged block by any path. The sole durable
  tip-advance path stays `run_node_sync -> pump_block` (CN-NODE-02 / DC-SYNC-02
  hold byte-for-byte).
- **N2** — Forging twice for one slot, or forging for a past slot.
- **N3** — The GREEN planner encoding any leadership / forge-eligibility / block
  / tip / verdict / KES-validity decision. The planner's forge input is a
  content-blind `Due | NotDue` only.
- **N4** — `SystemTime` / `Instant` / float / any wall-clock value passing the
  RED observation boundary into GREEN or BLUE. The slot must arrive as a
  canonical `SlotNo`.
- **N5** — The loop forge naming a forge-time bundle token, reading
  `--consensus-inputs-path`, or fabricating a `SeedEpochConsensusInputs`,
  `pparams`, `protocol_version`, `pool_id`, or KES-period literal
  (CN-CINPUT-03 guard d).
- **N6** — Signing with a KES period rotated past the hot key.
- **N7** — Counting a forged-but-unserved/unapplied block as peer acceptance or
  BA-02 / RO-LIVE evidence. Wire/forge success ≠ acceptance. It is **local
  self-accept evidence only**; RO-LIVE-01 and BA-02 (RO-LIVE-06) stay
  partial/untouched.
- **N8** — Real operator key/config file ingestion into `--mode node` in this
  cluster. Forge is activated only through hermetic/test signing material or an
  already-existing fenced producer-shell surface. Real KES/VRF/cold/opcert /
  pool-id / pparams CLI/config loading into `--mode node` is a **separate RED
  key-ingress cluster**.

## 3. What must remain identical across executions (deterministic surface)

- `millis_to_slot` is pure: same `(now_millis, anchor, slot_length, start_slot)`
  → same `SlotNo`.
- The forge-slot decision: same injected clock tick schedule + same loop state →
  same sequence of forge attempts.
- `forge_one_from_recovered` output is deterministic for fixed inputs
  (DC-CINPUT-02b already proves this).
- The wall-clock *observation itself* is the lone RED nondeterminism — it must
  be canonicalized to `SlotNo` before crossing any seam (I4).

## 4. What must be replay-equivalent

- **T-REC-03 strengthening:** same recovered state + same ordered block feed +
  **same injected clock tick schedule** + same shutdown schedule ⇒
  byte-identical tips, WAL, checkpoints **and** byte-identical forge-attempt
  sequence + forged block bytes.
- **DC-NODE-03 strengthening:** the clock-injection seam makes the forge-slot
  derivation replayable under `DeterministicClock`.

## 5. State transitions in scope

- **T-A — plan a tick (GREEN, planner extension).**
  `(loop_state, sync_status, shutdown_status, forge_slot_status) -> LoopStep`,
  where `LoopStep` gains a 4th closed variant `ForgeTick`:
  ```
  enum LoopStep { SyncOnce, ForgeTick, Idle, HaltCleanly }
  ```
  and `forge_slot_status` is a **content-blind** `Due | NotDue` — never a
  block / hash / selected-tip authority / ledger verdict / leader status / KES
  validity / forge eligibility. Pure, total, no authority.
  **Precedence:** shutdown → (source-ready ⇒ `SyncOnce`) → (forge-slot-due ⇒
  `ForgeTick`) → `Idle`. Shutdown is first because it is a clean boundary
  decision; it must not interrupt `run_node_sync` mid-batch (cancellation only
  in the `Idle` branch, exactly as N-F-D).
- **T-B — derive current slot (RED observe → GREEN convert).**
  `SystemClock::now_millis() -> millis_to_slot(anchor = SystemStart,
  slot_length = EraSchedule, start_slot) -> SlotNo`.
- **T-C — forge tick (RED driver over the existing fn).**
  `(recovered BootstrapState, selected_tip, shell, pool_id, pparams,
  era_schedule, slot, kes_period, protocol_version) ->
  Result<CoordinatorEvent, NodeForgeError>` — **already exists**
  (`forge_one_from_recovered`, `node_sync.rs:378`). The loop calls it; on
  `ForgeSucceeded` it records a **local** outcome; it does **not** advance the
  durable tip.
- **T-D — forge-slot monotonic guard (GREEN, DC-NODE-05).**
  `(last_forged_slot: Option<SlotNo>, current_slot: SlotNo) -> ForgeDue |
  NotDue` (due only if `current > last`). Pure.

## 6. TCB color hypothesis

- **BLUE:** unchanged — no BLUE crate change expected (mirrors N-F-D). The forge
  uses existing BLUE authorities transitively via `forge_one_from_recovered`.
- **GREEN:** the planner extension (`ForgeTick` step + content-blind
  `forge_slot_status`), the forge-slot monotonic guard (DC-NODE-05),
  `millis_to_slot` (already GREEN in `clock.rs`), `slot -> kes_period` (pure).
- **RED:** `run_relay_loop` gains the forge branch; the `SystemClock` wall-clock
  observation; the producer-shell wiring (hermetic/fenced material only — **no**
  new operator-key file/config ingestion this cluster, N8).

## 7. Open questions — RESOLVED at this gate

- **OQ1 — Fate of a forged block: self-accept-only.** A forge tick may produce a
  self-accepted forged-block artifact and a **local** `ForgeSucceeded`
  coordinator event. It must not durable-apply, admit, serve, gossip, count peer
  acceptance, or advance the node tip. This is **local self-accept evidence**,
  not BA-02 or RO-LIVE evidence.
- **OQ2 — Operator signing material: NOT ingested this cluster.** Forge is
  activated only through hermetic/test signing material or an already-existing
  fenced producer-shell surface. Real operator key CLI/config ingestion for
  `--mode node` is a separate RED key-ingress cluster. Absent producer material
  ⇒ exact N-F-D relay behavior preserved.
- **OQ3 — Planner shape: 4th closed `ForgeTick` variant** (§5 T-A), gated by a
  content-blind `Due | NotDue` input; leadership stays inside
  `forge_one_from_recovered`.
- **OQ4 — Planner precedence: sync-before-forge** (§5 T-A), shutdown first.
- **OQ5 — Single-epoch only.** Cross-epoch consensus-view / KES-period rollover
  is out of scope. If the recovered seed-epoch view cannot support the slot, the
  forge fail-closes / skips with a structured local outcome. **This is
  cluster-scope containment, not permanent Cardano behavior.**
- **OQ6 — `pparams` / `protocol_version` / `pool_id` source: narrowest rule.**
  They must come from existing recovered / bootstrap / producer-shell inputs
  already accepted by the existing forge handoff. N-F-E may *thread* them; it may
  not introduce a new semantic source, parser, config file, or fabricated
  literal. If the current code cannot provide them without new operator-config
  ingestion, N-F-E uses **hermetic test wiring only** and explicitly defers real
  `--mode node` key/config ingress (per OQ2 / N8).
- **OQ7 — RO-LIVE-01 linkage (correction to the N-F-E kickoff brief).**
  RO-LIVE-01 is the **serve-to-peer** produce-path artifact, not the relay loop.
  A self-accept-only forge tick that never serves does **not** close its code
  half, and does not close BA-02 (RO-LIVE-06). RO-LIVE-01 stays partial,
  untouched. N-F-E strengthens the **local producer path only.**

---

## Registry footprint

One genuinely new rule; the rest are strengthenings recorded via
`strengthened_in` at implementation time.

### NEW

- **DC-NODE-05** (derived; `declared` until enforcement lands) — forge-slot
  discipline on the relay run-loop: at most once per `SlotNo`, never a past
  slot, slot derived only through the clock seam (`SlotNo`-only crosses), the
  forge tick advances no durable tip (subordinate to the sync spine), and the
  forge-attempt sequence + forged bytes are byte-identical across runs under a
  fixed recovered state / feed / injected clock schedule / shutdown schedule.

### STRENGTHENINGS (recorded during implementation)

- **CN-NODE-02** — its `{ SyncOnce, Idle, HaltCleanly }` vocabulary gains
  `ForgeTick`; the constraint that the loop owns no new tip/apply/serve/evidence
  authority must be preserved (the forge tick advances no tip).
- **DC-SYNC-02** — the sole durable tip-advance path stays
  `run_node_sync -> pump_block`, now alongside a forge branch that advances no
  tip.
- **T-REC-03** — loop-as-replay now also covers the forge-attempt sequence +
  forged bytes under a fixed injected clock schedule.
- **DC-NODE-03** — the clock-injection seam now drives the forge-slot
  derivation in `--mode node`.
- **CN-PROD-02** — `slot -> kes_period` purity + no-retroactive-forge now apply
  on the relay-loop forge tick.
- **DC-CINPUT-02b** — the recovered-surface forge consumption is now reached on
  the live relay-loop path (still no BLUE change; still fenced by guard d).
