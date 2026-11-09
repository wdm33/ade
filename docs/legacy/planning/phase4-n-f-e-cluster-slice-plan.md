# Cluster/Slice Plan — PHASE4-N-F-E (forge-tick on the relay spine)

> Per-cluster slice plan. Derived from `docs/planning/phase4-n-f-e-invariants.md`
> (the `/invariants` sketch, DC-NODE-05 declared at registry HEAD 310). Saved
> here — NOT in `docs/active/phase_4_cluster_plan.md` (the multi-cluster phase
> index `/cluster-doc` seeds from; a single sub-cluster must not clobber it).

## Cluster Index (Dependency Order)

Single sub-cluster. It rides only **already-enforced** invariants — the N-F-D
relay spine (CN-NODE-02, DC-SYNC-02, T-REC-03), the N-F-A/N-F-C recovered-surface
forge handoff (DC-CINPUT-02b, CN-CINPUT-03), the N-K clock seam (DC-NODE-03), and
the producer authorities (CN-PROD-02, CN-FORGE-01/03). No dependency on any later
cluster.

1. **PHASE4-N-F-E** — forge-tick wiring — primary invariant: **the `--mode node`
   relay loop may attempt a self-accept-only forge at a deterministically-derived
   forge-slot, advancing no tip and owning no new authority (DC-NODE-05).**

---

## PHASE4-N-F-E — forge-tick on the relay spine

- **Primary invariant:** DC-NODE-05 — forge-slot discipline: at most once per
  `SlotNo`, never a past slot, slot derived only through the clock seam, the
  forge advances no durable tip (subordinate to the sync spine), replay-equivalent
  forge-attempt sequence; leadership eligibility stays BLUE inside
  `forge_one_from_recovered`.

- **TCB partition:**
  - **BLUE:** none changed (the forge uses existing BLUE `leader_schedule` /
    `self_accept` / `encode_block_envelope` transitively via
    `forge_one_from_recovered`). No BLUE crate is modified.
  - **GREEN:** `ade_node::run_loop_planner` (new closed `ForgeTick` step,
    content-blind `forge_slot_status`, forge-slot monotonic guard); reused
    `ade_runtime::clock::millis_to_slot`; reused
    `ade_runtime::producer::coordinator::kes_period_for_slot`.
  - **RED:** `ade_node::node_lifecycle::run_relay_loop` (forge branch); the
    `SystemClock` wall-clock observation; hermetic/fenced producer-shell wiring
    in `--mode node`.

- **Cluster Exit Criteria:**
  - **CE-E-1:** the GREEN planner emits the closed `{ SyncOnce, ForgeTick, Idle,
    HaltCleanly }` vocabulary; `forge_slot_status` is content-blind
    `Due | NotDue`; no authority token in the planner; precedence
    shutdown → sync → forge → idle; the decision table is total.
  - **CE-E-2:** the forge-slot monotonic guard is pure — forge at most once per
    `SlotNo`, never ≤ the last forged slot.
  - **CE-E-3:** the current slot is derived only via the clock seam (`SystemClock`
    → `millis_to_slot` → `SlotNo`); no `SystemTime` / `Instant` / float past the
    RED observation boundary; the KES period comes from the reused
    `kes_period_for_slot` (pure, fail-closed past the hot-key max period).
  - **CE-E-4:** `ForgeTick` is wired → `forge_one_from_recovered` on
    hermetic/fenced producer-shell material; `ForgeSucceeded` only via BLUE
    `self_accept`; recovered-surface-only leadership (CN-CINPUT-03 guard d holds);
    the forge advances **no** durable tip and serves/admits/gossips nothing
    (the loop containment gate is extended to allow the fenced forge call while
    still forbidding any tip/serve/admit/second-bootstrap path).
  - **CE-E-5:** forge is opt-in — absent producer material, the loop is
    byte-identical to N-F-D relay behavior on its authoritative + test-visible
    surface (tips, WAL, checkpoints, loop steps excluding the disabled-forge
    status probes, and no forged artifacts); **no** operator-key file/config
    ingestion exists in `--mode node`.
  - **CE-E-6:** forge-tick replay-equivalence — two clean runs are byte-identical
    in tips/WAL/checkpoints **and** in the forge-attempt sequence + forged bytes,
    under a fixed recovered state / feed / injected clock schedule / shutdown
    schedule.
  - **CE-E-7:** single-epoch / KES containment — an unsupported slot (outside the
    recovered seed-epoch view, or a KES period rotated past the hot key) produces
    a structured **local** skip / fail-closed outcome and does **not** fabricate
    consensus inputs, sign retroactively, advance the tip, or serve/admit
    anything. No cross-epoch path.

- **Slices:**
  - **S1 — GREEN planner forge step** — invariant: a closed `ForgeTick` step +
    content-blind `forge_slot_status { Due | NotDue }` + the forge-slot monotonic
    guard + precedence (shutdown → sync → forge → idle); pure, total, no
    authority; lands tested-but-unwired. — addresses: CE-E-1, CE-E-2 —
    TCB: **GREEN**.
  - **S2 — RED forge-tick wiring (self-accept-only)** — invariant: `run_relay_loop`
    observes `SystemClock` → `millis_to_slot` → `SlotNo` (only `SlotNo` crosses
    the seam), feeds the planner's `forge_slot_status`; on `ForgeTick` it reuses
    `kes_period_for_slot` + calls `forge_one_from_recovered` with hermetic
    producer-shell material; it **returns/emits a local self-accept forge outcome
    only within the hermetic loop test surface** (no new persisted log / evidence
    / event vocabulary); it advances no tip and serves/admits/gossips nothing; no
    operator-key ingestion; absent producer material preserves exact N-F-D relay
    behavior on the authoritative + test-visible surface (CE-E-5); the loop
    containment gate is extended. — addresses: CE-E-3, CE-E-4, CE-E-5 —
    TCB: **RED** (observe) + GREEN (convert / KES-period, both reused).
  - **S3a — Forge-tick replay-equivalence** — invariant: same recovered state +
    same feed + same injected clock schedule + same shutdown schedule ⇒
    byte-identical tips/WAL/checkpoints **and** byte-identical forge-attempt
    sequence + forged bytes (two-run). — addresses: CE-E-6 —
    TCB: **test** (RED orchestration over the pure GREEN planner + the
    deterministic forge handoff).
  - **S3b — Single-epoch / KES fail-closed containment** — invariant: an
    unsupported slot (outside the recovered seed-epoch view) or a KES period past
    the hot key produces a structured local skip / fail-closed outcome and does
    not fabricate consensus inputs, sign retroactively, advance the tip, or
    serve/admit anything. — addresses: CE-E-7 — TCB: **test** + RED
    fail-closed arm.

  > **Why S3 is split (mirrors N-F-D's replay / crash split):** replay
  > determinism (S3a) and off-epoch/KES containment (S3b) are separate proof
  > surfaces. If one fails, the split localizes the failure immediately.

- **Replay obligations:** **None new on disk.** The forge tick advances no tip
  and persists nothing durable (self-accept-only), so there is **no new
  authoritative durable state, no new canonical/BLUE type, and no new on-disk
  replay corpus** — the N-F-D WAL/checkpoint corpus is unchanged. The sole replay
  obligation is the **in-memory two-run determinism** of the forge-attempt
  sequence + forged bytes (S3a / CE-E-6), strengthening **T-REC-03** (matches
  N-F-D S3a's "no new durability law").

- **Registry footprint:** flips **DC-NODE-05** `declared → enforced` (S1 = guard
  half · S2 = forge-slot + clock-seam + no-tip · S3a = replay clause · S3b =
  single-epoch fail-closed). Records `strengthened_in += "PHASE4-N-F-E"` on
  **CN-NODE-02** (vocab + no-new-authority), **DC-SYNC-02**, **T-REC-03**,
  **DC-NODE-03**, **CN-PROD-02**, **DC-CINPUT-02b** (reached on the live path).

---

## Close checklist (for `/cluster-close`)

- All 7 CEs green in CI; `cargo test --workspace` (scope verification to
  `ade_node` + `ade_runtime`; the `ade_testkit` corpus suite times out
  environmentally — pre-existing, not an N-F-E regression — see
  `reference_ade_testkit_corpus_suite_times_out`).
- **DC-NODE-05** flipped `declared → enforced`; the six strengthenings recorded
  (`strengthened_in += "PHASE4-N-F-E"`).
- **Regenerate the stale grounding docs** — SEAMS + TRACEABILITY are two clusters
  behind (last regenerated at the N-F-C close; must catch up through N-F-D **and**
  N-F-E); refresh CODEMAP + HEAD_DELTAS; bump `.idd-config.json`
  `head_deltas_baseline`. (This is the drift deferred from the `/invariants` and
  `/cluster-plan` gates — folded into the close, no separate refresh pass now.)
- Per-cluster security review against the full N-F-E diff (block on HIGH+),
  between the IDD-reviewer step and the grounding-doc refresh.
- **Confirm N-F-D's restored T-REC-03 evidence remained green while layering
  N-F-E replay tests** — the relay spine's own replay-equivalence test
  (`relay_loop_two_clean_runs_byte_identical`, repaired in `ffa76fc` from a
  non-compiling/non-passing state shipped at the N-F-D close) must still pass
  alongside the N-F-E forge-tick replay test (S3a). Also re-verify that
  `cargo test -p ade_node` (the full lib-test target) actually compiles and
  passes at close — the N-F-D close masked or scoped past this target.
