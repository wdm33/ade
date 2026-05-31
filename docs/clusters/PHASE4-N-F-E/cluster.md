# Cluster PHASE4-N-F-E — Forge-tick on the relay spine (hermetic, self-accept-only)

> **Status: PLANNED (derived from committed sketch `de497c4` + committed plan `77901d9`).**
> Successor to **PHASE4-N-F-D** (CLOSED + PUSHED, `origin/main 7de1462`), which wired the
> relay half into a continuous loop (`run_relay_loop`, GREEN `run_loop_planner`,
> `run_node_sync → pump_block` containment) and **explicitly fenced `DC-NODE-05` (forge-slot
> discipline) out to this sub-cluster.** N-F-E wires the **forge half** the N-F-D doc deferred.
>
> Companion docs: `../../planning/phase4-n-f-e-invariants.md` (sketch — OQ1–OQ7 settled),
> `../../planning/phase4-n-f-e-cluster-slice-plan.md` (the S1→S2→S3a→S3b ordered plan).
>
> **Cluster character (load-bearing — do not broaden):** hermetic, single-epoch,
> **self-accept-only** forge-tick wiring. *Not* an operator-key-ingress cluster, *not* a
> live/peer-evidence cluster. A forge tick may produce a self-accepted forged-block artifact
> and a local self-accept outcome **within the hermetic loop test surface** — nothing more.
>
> **Hard line:** if wiring the forge requires modifying `run_node_sync`, modifying a BLUE
> crate, ingesting real operator keys, or adding a tip/serve/admit path — **stop and
> re-scope** rather than smuggling authority into the loop.

## Primary invariant
On the `--mode node` relay loop, a forge is attempted **at most once per `SlotNo`** and never
for a past slot; the slot is derived **only** through the clock seam (RED observes wall-clock,
GREEN converts via `millis_to_slot` over `SystemStart` + `EraSchedule` — only `SlotNo` crosses
any seam); the forge tick advances **no** durable tip and admits/serves/gossips nothing (the
sole durable tip-advance path stays `run_node_sync → pump_block`); leadership eligibility stays
BLUE inside `forge_one_from_recovered`; and the forge-attempt sequence + forged bytes are
byte-identical across runs under fixed inputs (`DC-NODE-05`).

**Invariants strengthened:** `CN-NODE-02` (loop vocabulary gains `ForgeTick`, no new tip
authority), `DC-SYNC-02`, `T-REC-03`, `DC-NODE-03` (clock seam now drives the forge slot),
`CN-PROD-02` (KES-period purity on the loop), `DC-CINPUT-02b` (recovered-surface forge
reached on the live loop path).

## Normative anchors
- `docs/planning/phase4-n-f-e-invariants.md` (the `/invariants` sketch; OQ1–OQ7 resolved).
- `docs/planning/phase4-n-f-e-cluster-slice-plan.md` (the committed four-slice plan).
- Registry: `DC-NODE-05` (declared, this cluster), and the six strengthened rules above.
- Carried: `docs/active/CE-79_gate_statement.md` (tier doctrine), `CLAUDE.md`.

## The one loop (extended with the forge branch)

```
ENTRY (unchanged from N-F-D — both arms converge):
  run_node_lifecycle → bootstrap (FirstRun) | warm_start_recovery (WarmStart)
    → BootstrapState  →  run_relay_loop(state, source, …, recovered, forge_activation?)

LOOP (RED driver; GREEN plans; BLUE authority behind the seam):
  loop:
    shutdown        = shutdown_rx.borrow()
    sync_status     = NodeBlockSource readiness (content-blind, RED scheduling only)
    forge_slot_stat = clock seam: SystemClock.now_millis → millis_to_slot(SystemStart,
                      EraSchedule) → SlotNo → monotonic guard → Due | NotDue   (forge OFF
                      when no producer material → always NotDue)
    plan_loop_step(loop_state, sync_status, shutdown, forge_slot_status) → LoopStep
      SyncOnce    → run_node_sync(...) once         (UNMODIFIED; sole durable tip-advance)
      ForgeTick   → kes_period_for_slot(slot) + forge_one_from_recovered(recovered,
                    selected_tip, shell, …, slot, kes_period, …)  → local self-accept
                    outcome ONLY; advances NO tip, serves/admits/gossips NOTHING
      Idle        → select!(source-ready, shutdown.changed())   (cancellation-safe)
      HaltCleanly → break
```

**Precedence: shutdown → sync → forge → idle.** Drain the subordinate spine before forging;
shutdown is a clean boundary and must never interrupt `run_node_sync` mid-batch (the only
`.await` between steps stays `next_block()` / the `Idle` select).

## Locked rules (from the OQ1–OQ7 ratification)

- **Self-accept-only (OQ1).** A forge tick may produce a self-accepted artifact + a local
  self-accept outcome **within the hermetic loop test surface**. It must not durable-apply,
  admit, serve, gossip, count peer acceptance, or advance the tip. This is local self-accept
  evidence — **not** BA-02, **not** RO-LIVE, **not** peer acceptance. No new log/event/JSONL
  vocabulary is introduced.
- **No operator-key ingestion (OQ2/N8).** Forge is activated only through hermetic/test
  signing material or an already-existing fenced producer-shell surface. Real KES/VRF/cold/
  opcert/pool-id/pparams CLI/config loading into `--mode node` is a **separate RED key-ingress
  cluster**. Absent producer material ⇒ exact N-F-D relay behavior preserved.
- **Closed `ForgeTick` step; planner stays authority-free (OQ3).** `LoopStep` becomes
  `{ SyncOnce, ForgeTick, Idle, HaltCleanly }`. The planner's forge input is a content-blind
  `Due | NotDue` — never block/hash/tip/verdict/leader-status/KES-validity/forge-eligibility.
  Leadership stays inside `forge_one_from_recovered`.
- **Sync-before-forge (OQ4).** Precedence above.
- **Single-epoch containment (OQ5).** Cross-epoch consensus-view / KES-period rollover is out
  of scope. An unsupported slot fails closed / skips with a structured **local** outcome.
  **Cluster-scope containment, not permanent Cardano behavior.**
- **Narrowest `pparams`/`protocol_version`/`pool_id` source (OQ6).** They come from existing
  recovered/bootstrap/producer-shell inputs already accepted by `forge_one_from_recovered`.
  N-F-E may *thread* them; it may not introduce a new semantic source, parser, config file,
  or fabricated literal.
- **RO-LIVE-01 untouched (OQ7).** A self-accept-only forge tick that never serves does not
  close RO-LIVE-01's code half or BA-02 (RO-LIVE-06). Both stay partial.
- **Clock seam, not a new adapter (I4).** N-F-E **reuses** `millis_to_slot` (GREEN) +
  `kes_period_for_slot` (GREEN, fail-closed) — no new wall-clock adapter, no new KES-period
  helper. The slot-derivation clause is enforced as part of DC-NODE-05.

## Verified component inventory (read at `7de1462`, not assumed)

| Component | Real state | Use |
|---|---|---|
| `node_lifecycle::run_relay_loop(state: &mut ForwardSyncState, source, chaindb, wal, era_schedule, ledger_view, shutdown)` | the N-F-D relay loop; advances tip only via `run_node_sync` | **S2** adds a `ForgeTick` arm + threads in the recovered `BootstrapState` + an optional producer-shell forge-activation bundle |
| `run_loop_planner::plan_loop_step(loop_state, sync_status, shutdown_status) -> LoopStep` + closed `LoopStep { SyncOnce, Idle, HaltCleanly }` | pure GREEN, total, no authority | **S1** adds `ForgeTick` + a content-blind `forge_slot_status { Due \| NotDue }` input + the monotonic guard; precedence shutdown→sync→forge→idle |
| `node_sync::forge_one_from_recovered(recovered, selected_tip, shell, pool_id, pparams, era_schedule, slot, kes_period, protocol_version) -> Result<CoordinatorEvent, NodeForgeError>` | **exists, enforced** (`DC-CINPUT-02b`); recovered-surface-only leadership (guard d); single-shot; self-accept inside `run_real_forge` | **S2** calls it **unmodified** on the `ForgeTick` arm |
| `clock::millis_to_slot(now_millis, anchor_millis, start_slot, slot_length_ms) -> SlotNo` | pure GREEN (N-K) | **S2** reuses for the slot derivation; `SystemClock` (RED) supplies `now_millis`; `SystemStart` anchor + slot length from `EraSchedule`/recovered state |
| `clock::SystemClock` (`now_millis` reads `SystemTime`) | RED observation surface (N-K) | **S2** the sole wall-clock observation; only `SlotNo` crosses the seam |
| `producer::coordinator::kes_period_for_slot(slot) -> Option<u32>` | pure, fail-closed `None` past the hot-key max period (`CN-PROD-02`) | **S2** reuses to derive `kes_period`; **S3b** off-range ⇒ skip |
| `ci_check_node_run_loop_containment.sh` (N-F-D) | currently **forbids** `forge_one_from_recovered`/`run_real_forge` on the loop path | **S2** evolves it: permit the single fenced `forge_one_from_recovered(` call; **retain** every tip/serve/admit/correlate/second-bootstrap prohibition + add forge-specific ones (see CE-E-4) |

## Slices (safety order)

### S1 — GREEN planner forge step *(hermetic; tested-but-unwired)*
Extend `run_loop_planner`: add the closed `ForgeTick` variant to `LoopStep`, a content-blind
`forge_slot_status { Due | NotDue }` input, the pure forge-slot monotonic guard
(`(last_forged_slot, current_slot) → Due|NotDue`, due only if `current > last`), and the
precedence shutdown → sync → forge → idle. Pure, total, no `#[non_exhaustive]`, no wildcard,
no authority token. Lands tested-but-unwired. Addresses **CE-E-1, CE-E-2**. TCB: **GREEN**.

### S2 — RED forge-tick wiring (self-accept-only) *(hermetic)*
`run_relay_loop` gains a `ForgeTick` arm and threads in the recovered `BootstrapState` + an
optional producer-shell forge-activation bundle. The arm observes `SystemClock` →
`millis_to_slot` (over `SystemStart` + `EraSchedule`) → `SlotNo` (only `SlotNo` crosses),
feeds `forge_slot_status`, reuses `kes_period_for_slot`, and calls the unmodified
`forge_one_from_recovered`. It **returns/emits a local self-accept forge outcome only within
the hermetic loop test surface** — no new log/event vocabulary, no durable apply, no tip
advance, no serve/admit/gossip. Forge is opt-in: absent producer material ⇒ `forge_slot_status`
is always `NotDue` and the loop is byte-identical to N-F-D relay behavior on its authoritative
+ test-visible surface. Evolves the loop-containment gate. Addresses **CE-E-3, CE-E-4, CE-E-5**.
TCB: **RED** (observe) + GREEN (convert/KES-period, reused).

### S3a — Forge-tick replay-equivalence *(hermetic)*
Two clean runs over identical inputs (same recovered state + same feed + same injected clock
tick schedule + same shutdown schedule) produce byte-identical tips, WAL, checkpoints **and**
byte-identical forge-attempt sequence + forged bytes. Flips **CE-E-6**. TCB: **test**.

### S3b — Single-epoch / KES fail-closed containment *(hermetic)*
An unsupported slot (outside the recovered seed-epoch view) or a KES period rotated past the
hot key produces a structured **local** skip / fail-closed outcome and does **not** fabricate
consensus inputs, sign retroactively, advance the tip, or serve/admit anything. A different
proof surface from S3a (off-epoch/KES containment vs. replay determinism). Addresses **CE-E-7**.
TCB: **test** + the RED fail-closed arm.

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the owning slice); existing artifacts named as-is.

- **CE-E-1** — `plan_loop_step` returns the closed `LoopStep { SyncOnce, ForgeTick, Idle,
  HaltCleanly }` over closed inputs. Planner step selection may receive only a content-blind
  `ForgeSlotStatus { Due | NotDue }`. The pure monotonic guard may consume `SlotNo` values, but
  `plan_loop_step` itself must not carry `SlotNo`, `ChainTip`, block identity, leader status,
  KES validity, or forge eligibility. Precedence shutdown→sync→forge→idle; pure/total.
  Candidate gate `ci_check_loop_planner_closed.sh` **extended** (`ForgeTick` + `ForgeSlotStatus`
  added to the closed set; still forbids `pump_block`/`run_node_sync`/`run_real_forge`/
  `forge_one_from_recovered`/`correlate`/`ChainDb`/`LedgerState`/`BlockHash`/`ChainTip` tokens in
  the `plan_loop_step` surface; the monotonic guard may reference `SlotNo`) + candidate test
  `plan_loop_step_forge_precedence_table_is_total`. *(`DC-NODE-05` planner half; `CN-NODE-02`)*
- **CE-E-2** — the forge-slot monotonic guard is pure and forges at most once per `SlotNo`,
  never ≤ the last forged slot. Candidate tests `forge_slot_guard_at_most_once_per_slot`,
  `forge_slot_guard_rejects_past_slot`. *(`DC-NODE-05`)*
- **CE-E-3** — the current slot is derived only via the clock seam; no `SystemTime`/`Instant`/
  float crosses into GREEN/BLUE; KES period via reused `kes_period_for_slot`. Existing
  `ci_check_clock_seam.sh` + `ci_check_forbidden_patterns.sh` hold; candidate test
  `relay_loop_forge_slot_derived_via_millis_to_slot`. *(`DC-NODE-05`; strengthens `DC-NODE-03`/`CN-PROD-02`)*
- **CE-E-4** — `ForgeTick` is wired → the single fenced `forge_one_from_recovered(`; the forge
  advances no durable tip and serves/admits/gossips nothing. Candidate gate
  `ci_check_node_run_loop_containment.sh` **evolved**: permits the one `forge_one_from_recovered(`
  call, **retains** every existing prohibition (manual tip advance `put_block`/`AdvanceTip`/
  `rollback_to_slot`, follower/verdict-as-sync, `correlate`/`Ba02Manifest`, second bootstrap)
  and **adds** new ones (no `run_real_forge(` direct call bypassing the fenced fn, no
  serve/broadcast/gossip of the artifact, no durable apply of it). `ci_check_consensus_input_provenance.sh`
  guard (d) continues to hold (recovered-surface-only leadership). Candidate test
  `relay_loop_forge_tick_self_accepts_advances_no_tip`. *(`DC-NODE-05`; `CN-NODE-02`; `DC-CINPUT-02b`/`CN-CINPUT-03`)*
- **CE-E-5** — forge is opt-in: absent producer material preserves N-F-D behavior on
  authoritative/test-visible outputs — tips, WAL, checkpoints, halt behavior, and no forged
  artifacts. Any additional disabled-forge probes must be non-authoritative and excluded from
  replay-visible evidence. No operator-key file/config ingestion exists in `--mode node`
  (`ci_check_node_mode_closure.sh` holds; no new key/config parse). Candidate test
  `relay_loop_without_producer_material_matches_nfd_relay`. *(`DC-NODE-05`; `CN-NODE-02`)*
- **CE-E-6** — forge-tick replay-equivalence: candidate test
  `relay_loop_forge_two_runs_byte_identical` asserts byte-identical tips + WAL + checkpoints +
  forge-attempt sequence + forged bytes across two clean runs over identical inputs. Flips
  `DC-NODE-05`'s replay clause. *(`DC-NODE-05`; strengthens `T-REC-03`)*
- **CE-E-7** — single-epoch / KES fail-closed: candidate tests
  `forge_tick_off_epoch_slot_fails_closed_local`, `forge_tick_rotated_kes_period_skips_no_retroactive_sign`
  assert a structured local skip/fail-closed with no fabricated inputs, no retroactive sign,
  no tip advance, no serve/admit. *(`DC-NODE-05`)*

> DC-NODE-05 flips `declared → enforced` only when CE-E-1..7 are all green.

## TCB color map
- **BLUE (none — reuse only):** no BLUE crate is touched. The forge's authority (leader
  schedule, `self_accept`, `encode_block_envelope`) is reached transitively through the
  existing closed `forge_one_from_recovered` seam. A BLUE change is a red flag the loop is
  absorbing authority → reject.
- **GREEN:** `ade_node::run_loop_planner` (the `ForgeTick` step + content-blind
  `forge_slot_status` + monotonic guard); reused `ade_runtime::clock::millis_to_slot`; reused
  `ade_runtime::producer::coordinator::kes_period_for_slot`.
- **RED:** `ade_node::node_lifecycle::run_relay_loop` (the `ForgeTick` arm + recovered-state /
  producer-shell threading); the RED wall-clock observation used to produce `now_millis` — only
  the resulting `SlotNo` may cross into forge planning/wiring (the hard invariant is the seam,
  not the exact call site); the hermetic/fenced producer-shell wiring.
- **CI:** evolved `ci_check_loop_planner_closed.sh` (S1) + evolved
  `ci_check_node_run_loop_containment.sh` (S2); existing `ci_check_node_sync_via_pump.sh`,
  `ci_check_consensus_input_provenance.sh` (guard d), `ci_check_clock_seam.sh`,
  `ci_check_node_mode_closure.sh`, `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`
  continue to hold.

## Forbidden during this cluster *(slice-level prohibitions inherit from this list)*
- No BLUE crate changes; no new canonical type; no new WAL/checkpoint/JSONL/event vocabulary.
- No **durable apply** of a forged block; no `AdvanceTip`/`put_block`/`rollback_to_slot` from
  the loop — the tip advances ONLY through `run_node_sync → pump_block`.
- No **serve / broadcast / gossip / block-fetch** of a forged block; no `correlate`/`Ba02Manifest`.
- No **operator-key file/config ingestion** in `--mode node` (hermetic/fenced producer-shell
  material only).
- No `run_real_forge(` direct call on the loop path — forge only via the fenced
  `forge_one_from_recovered`.
- No fabricated `SeedEpochConsensusInputs` / `pparams` / `protocol_version` / `pool_id` /
  KES-period literal; no `--consensus-inputs-path` / bundle token on the forge path (guard d).
- No `SystemTime` / `Instant` / float / wall-clock value crossing past the RED observation
  boundary; only `SlotNo` crosses the seam.
- No cross-epoch production; no retroactive sign past the hot-key KES period.
- `run_node_sync` stays **unmodified**; the planner stays pure/authority-free.
- **Hard line:** if the forge needs a BLUE change, `run_node_sync` modification, real operator
  keys, or a tip/serve/admit path — **stop and re-scope.**

## Replay obligations (scoped)
**No new canonical type, no new authoritative transition, no new WAL/checkpoint format, no new
`ade_testkit` corpus entry** — the forge tick advances no tip and persists nothing durable
(self-accept-only). `DC-NODE-05`'s replay clause + the `T-REC-03` strengthening are discharged
by **tests** (S3a `relay_loop_forge_two_runs_byte_identical`), not corpus. Determinism guard:
the wall-clock observation is the lone RED nondeterminism and is canonicalized to `SlotNo`
before crossing; replay uses an injected deterministic clock-tick schedule. Acceptance scoped
to touched crates (`ade_node`, `ade_runtime`) — **not** the full `ade_testkit` corpus/oracle
lane (times out ~600s on clean HEAD).

## Registry impact (at close)
`DC-NODE-05` already `declared` at sketch (registry 309 → 310). Promotion / strengthening:
- `DC-NODE-05` (derived) — `declared` → **enforced** across S1–S3b (all CE-E-1..7 green).
- `CN-NODE-02`, `DC-SYNC-02` — `strengthened_in += "PHASE4-N-F-E"` (vocab gains `ForgeTick`;
  no new tip authority) in S2.
- `T-REC-03` — `strengthened_in += "PHASE4-N-F-E"` in S3a (replay now covers the forge tick).
- `DC-NODE-03`, `CN-PROD-02` — `strengthened_in += "PHASE4-N-F-E"` in S2 (clock seam drives the
  forge slot; KES-period purity on the loop).
- `DC-CINPUT-02b` — `strengthened_in += "PHASE4-N-F-E"` in S2 (recovered-surface forge reached
  on the live loop path; still no BLUE change, guard d holds).
- **Not added here:** real operator-key ingress (separate RED cluster), `RO-LIVE-01`/BA-02
  live evidence (operator-gated), cross-epoch rollover (successor cluster).

## Non-goals
No serve / broadcast / BA-02 / peer-acceptance claim (RO-LIVE-01 + RO-LIVE-06 stay partial).
No operator-key/config ingestion (separate cluster). No live peer / operator pass. No durable
apply or tip mutation from the forge. No cross-epoch production (single-epoch recovered view is
the boundary; unsupported slot fails closed — cluster-scope containment, not a permanent Cardano
rule). No new BLUE authority/type, no new durability subsystem, no new event vocabulary. No
grounding-doc regeneration (that's `/cluster-close` — SEAMS + TRACEABILITY catch-up through
N-F-D and N-F-E is recorded in the plan's close checklist).
