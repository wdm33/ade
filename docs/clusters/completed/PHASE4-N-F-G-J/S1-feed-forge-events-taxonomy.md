# Slice PHASE4-N-F-G-J S1 — Closed feed-state taxonomy + diagnostic events

## §1 Slice ID
- **Cluster:** PHASE4-N-F-G-J — Node forge when feed is empty/at tip (`docs/clusters/PHASE4-N-F-G-J/cluster.md`, `ff03e244`)
- **Slice:** S1 — Closed feed-state taxonomy + diagnostic events (observability-first)
- **Status:** Merged (`60303079`)
- **TCB:** GREEN (closed enums) + RED (classification + emission); **no BLUE change**

## §2 Cluster Exit Criteria Addressed
- **CE-G-J-1** (verbatim): `--mode node` emits the closed, allow-listed `CN-NODE-04` vocabulary —
  `feed_unavailable{reason}` (closed reason enum, eligible split `NoBlockAvailable|AtTip|CleanEmpty`
  vs ineligible `PeerLost|DecodeError|ProtocolError|SourceInvalid`) + `forge_tick_considered` /
  `forge_tick_skipped{reason}` / `forge_attempted` / `forge_result{outcome}` — through `live_log`;
  verified by `node_sched_events_emit_closed_vocabulary`, the allow-list negative test
  `node_sched_event_allowlist_rejects_unknown_variants` (fail-closed on an unknown/added variant,
  never dropped), the emit-only gate `ci/ci_check_node_sched_events_emit_only.sh`, and the
  no-behavior-change proof (`plan_loop_step_forge_precedence_table_is_total` +
  `plan_loop_step_is_deterministic` stay green, byte-unchanged). `CN-NODE-04` declared → enforced.

## §4 Intent (invariant impact)
Make the `--mode node` feed/forge **scheduling decision observable** through a **closed,
fail-closed-on-unknown, emit-only** diagnostic surface (`CN-NODE-04`) + a **closed feed-state
taxonomy** — **without changing the decision**. Today the scheduling is a black box (the C1 run
emitted nothing but a final stderr summary). Invariant impact: a new closed operational event
vocabulary becomes mechanically enforced; the planner's scheduling decision stays byte-identical.
This makes S2's semantic change provable (`forge_tick_skipped{reason}` → … → `forge_result`)
rather than guessed.

## §5 Scope
**In:**
- A **closed feed-state taxonomy** enum — the *reason* a feed yields no block: eligible
  `{NoBlockAvailable, AtTip, CleanEmpty}` | ineligible `{PeerLost, DecodeError, ProtocolError,
  SourceInvalid}`.
- A `NodeBlockSource` **classification** (`node_sync.rs`, RED) mapping the concrete `is_ended`/
  end/empty conditions → the taxonomy (the OQ1 mapping, §13).
- A **new closed `CN-NODE-04` event vocabulary** — a sibling closed enum (working name
  `NodeSchedEvent`) in `live_log/`, **separate** from the wire-only `LiveLogEvent`:
  `feed_unavailable{reason}`, `forge_tick_considered`, `forge_tick_skipped{reason}`,
  `forge_attempted`, `forge_result{outcome}` (closed reason/outcome enums).
- **Emission** of the events from the `--mode node` relay loop (`node_lifecycle`/`run_node_sync` +
  `live_log::writer`, RED) at the relevant transitions.
- The **emit-only gate** (`ci/ci_check_node_sched_events_emit_only.sh`), the
  **allow-list-rejects-unknown** negative test, the positive emit test, the no-behavior-change proof.

**Out (S2/S3 — not this slice):**
- Wiring the taxonomy into `plan_loop_step` (S2 — the scheduling change).
- The C1 forge rerun harness/runbook (S3).
- **Any** change to `plan_loop_step` / `LoopState` / the precedence table.

## §6 Execution Boundary (TCB color)
- **GREEN:** the closed feed-state taxonomy enum; the closed `CN-NODE-04` `NodeSchedEvent`
  vocabulary enum (`live_log/`) — pure closed types. The wire-only `LiveLogEvent` is a **sibling,
  untouched**.
- **RED:** `ade_node::node_sync` (`NodeBlockSource` `is_ended → taxonomy` classification — reads
  channel/lookahead state); `ade_node::node_lifecycle` / `run_node_sync` (the relay loop emits the
  events); `ade_node::live_log::writer` (serialization/emission).
- **BLUE:** **none.** `forge_one_from_recovered` + the forge composition unchanged.
- **UNCHANGED (byte-identical — the no-behavior-change proof):** `ade_node::run_loop_planner`
  (`plan_loop_step` / `LoopState` / `LoopStep` / `ForgeSlotStatus` / the precedence table). The
  planner reads `LoopState`/`ForgeSlotStatus`, **never** a `NodeSchedEvent` → emit-only.

## §7 Invariants Preserved (registry IDs)
- `DC-NODE-05`, `DC-NODE-06`, `DC-NODE-07` (forge tick on relay spine / self-accept handoff /
  single shared serve) — untouched.
- `DC-FORGE-01`, `CN-FORGE-01..04` (forge composition + determinism) — untouched (no BLUE change).
- `DC-EPOCH-03` (single-epoch forge containment) — untouched.
- `CN-CINPUT-03`, `DC-CINPUT-02b` (forge base = recovered surface only) — untouched.
- **The forge SCHEDULING** — `plan_loop_step`'s total, deterministic table is **byte-unchanged**
  (no-behavior-change proof via the existing `plan_loop_step_forge_precedence_table_is_total` +
  `plan_loop_step_is_deterministic`).
- The **wire-only event vocabulary** (`LiveLogEvent` + `ci/ci_check_wire_only_event_vocabulary_closed.sh`)
  — untouched; the `CN-NODE-04` vocabulary is a separate closed enum.

## §8 Invariants Strengthened (registry ID)
- **`CN-NODE-04`** (declared → **enforced** at this S1 close). The slice's commit binds:
  - `tests += ["node_sched_events_emit_closed_vocabulary", "node_sched_event_allowlist_rejects_unknown_variants"]`
  - `ci_script = "ci/ci_check_node_sched_events_emit_only.sh"`
  - status flip recorded at close. The closed, fail-closed-on-unknown, **emit-only** diagnostic
    vocabulary becomes mechanically enforced.

## §9 Closed surface / surface reduction
The `CN-NODE-04` vocabulary is a **closed enum** — no catch-all/`Other` variant carrying arbitrary
data; `reason`/`outcome` are closed enums (no stringly-typed errors). Adding a variant requires a
code change **+** an allow-list update, or the gate/negative test **fails closed**. The feed-state
taxonomy is likewise a closed enum.

## §10 Determinism (Part V tripwires)
Events are deterministic given the same `(feed-state transitions, slot sequence, forge outcomes)`
but are **operational tier** — never consensus evidence, never replay-equivalence-weighted. No
wall-clock beyond the existing captured slot/tick; no `HashMap`/float/rand introduced.

## §11 Replay/Crash/Epoch Validation (named tests)
- **No-behavior-change (the load-bearing proof):**
  `run_loop_planner::tests::plan_loop_step_forge_precedence_table_is_total` +
  `…::plan_loop_step_is_deterministic` stay **green, byte-unchanged** (S1 does not touch the planner).
- `node_sched_events_emit_closed_vocabulary` (new): a hermetic `--mode node` driver run emits the
  closed `CN-NODE-04` vocabulary for the relevant feed/forge transitions.
- `node_sched_event_allowlist_rejects_unknown_variants` (new): the allow-list **fails closed** on
  an unknown/added variant (rejects, never silently drops).
- No new replay-corpus entry (operational-tier events; no authoritative state).

## §12 Mechanical Acceptance Criteria
- `cargo test -p ade_node` green, **including** `node_sched_events_emit_closed_vocabulary`,
  `node_sched_event_allowlist_rejects_unknown_variants`, **and** the unchanged
  `plan_loop_step_forge_precedence_table_is_total` + `plan_loop_step_is_deterministic`.
- `ci/ci_check_node_sched_events_emit_only.sh` green (data-flow-resistant scan:
  `run_loop_planner`/`plan_loop_step` names/reads no `NodeSchedEvent`).
- `ci/ci_check_wire_only_event_vocabulary_closed.sh` stays green (wire-only vocabulary untouched).

## §13 OQ resolved / carried — the `is_ended → taxonomy` mapping (fail-closed-on-ambiguity)
**OQ1 (load-bearing) — RESOLVED in this slice (the classification).** Mapping the concrete
`NodeBlockSource` end/empty conditions to the closed taxonomy, under a **fail-closed-on-ambiguity**
rule:
- **S1 does NOT enrich the wire-pump reason** (option (b)). `AdmissionPeerEvent::Disconnected`
  carries no reason and is emitted for **both** clean EOF and protocol error; `NodeBlockSource`
  collapses it to `disconnected: bool`. So the source **cannot prove** a clean drain today.
- S1's **producible** closed taxonomy from the current signals: a WirePump **open but momentarily
  empty** (not disconnected, lookahead empty) → eligible `NoBlockAvailable`; an **InMemory** feed
  drained → eligible `CleanEmpty` (a deterministic, provably-clean exhaustion — the hermetic
  source); a WirePump **disconnect** (reason-less) → **ineligible `UnknownDisconnected`**.
- **`channel disconnected` does NOT mean `CleanEmpty`.** A reason-less / ambiguous WirePump end is
  `UnknownDisconnected` — **ineligible, fail-closed** — until a future wire-pump enrichment captures
  a closed clean/error reason. Eligible-by-default is forbidden.
- The observed C1 disconnect is therefore **`UnknownDisconnected` (ineligible)** — **not** assumed
  `CleanEmpty`. The C1 rerun (with S1 events) reveals the real reason; only a *provably* clean
  no-block end could later be eligible.
- **Hard rule: no ambiguous disconnect may become forge-eligible.**
- The specific error reasons (`PeerLost` / `DecodeError` / `ProtocolError` / `SourceInvalid`) and a
  reason-enriched **live** `CleanEmpty` path are a **future wire-pump prerequisite, NOT S1**.

## §14 Hard Prohibitions
Inherits cluster §11 (no co-producer; no private-only/C1-only flag; no bypass of
`import_live_consensus_inputs`; forge base = recovered surface only; no forge from unanchored
genesis; no stale-base forge; no durable tip advance from forge scheduling alone; no serve of
non-self-accepted bytes; no BLUE change; no RO-LIVE flip), **plus slice-specific:**
- **NO behavior change** — `plan_loop_step`'s decision is byte-identical pre/post S1.
- **EMIT-ONLY** — the planner never reads a `CN-NODE-04` event (one-directional planner → log; the
  gate enforces it).
- The `CN-NODE-04` vocabulary **fails closed** on an unknown variant (allow-list rejects, never drops).
- **Fail closed on ambiguity** — a reason-less/ambiguous feed end classifies **ineligible
  `UnknownDisconnected`**, never eligible `CleanEmpty`. No ambiguous disconnect may become
  forge-eligible. **No cross-crate wire-pump reason enrichment in S1** (that is a future
  prerequisite, not this slice).
- **Operational/diagnostic tier only** — never a consensus/acceptance/BA-02 signal; no
  stringly-typed authoritative errors.
- No wall-clock beyond the existing captured slot/tick.
- **Do NOT wire the taxonomy into `plan_loop_step`** (that is S2 — no scheduling change here).
- No new `--mode node` flag.
