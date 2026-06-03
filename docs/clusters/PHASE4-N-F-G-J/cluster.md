# Cluster — PHASE4-N-F-G-J — Node forge when feed is empty/at tip

> Single follow-on sub-cluster (like G-E/G-H/G-I). Sources: invariants sketch
> `docs/planning/phase4-n-f-g-j-invariants.md` (`9eb6f39b`) + plan
> `docs/planning/phase4-n-f-g-j-cluster-slice-plan.md` (`6461160e`). Declares `CN-NODE-04`
> (operational) + `DC-NODE-08` (derived). Surfaced by the C1 forge dry-run, **after** G-I
> fixed the anchor-lineage WarmStart gap and the whole-second-`systemStart` regen fixed
> `SystemStartParseFailure`: on a fresh **sole-producer** net Ade fully WarmStart-recovers and
> wires the live feed, but the genesis peer has **zero blocks**, the WirePump ends, and
> `plan_loop_step` halts before any `ForgeTick` — Ade can never produce the **first** block.
> A **shared `--mode node`** producer-scheduling change (also serves C2/preprod, unexercised
> there because preprod's feed is already Continuing) — **not** a C1-only path.

## §0 Two halves with sharply different IDD status
- **Mechanical, hermetically closeable (S1 + S2):** the closed feed-state taxonomy + diagnostic
  events (S1) and the planner scheduling allowance (S2) close on hermetic tests + CI gates.
- **Operator-gated (S3):** the live C1 forge rerun stays `blocked_until_operator_c1_forge_rerun`
  — the mechanical scaffold closes; the live execution + any acceptance stay gated (G-H precedent).

## §1 Primary invariant
**`DC-NODE-08`** (declared → enforced at S2 close) — `--mode node` may run the producer tick from
the **recovered authoritative base** when the feed is `Empty|AtTip|NoBlockAvailable` (and only
then), gated on a recovered base + `ForgeIntent::On` + a **due forge slot**, with
**epoch/KES/leader eligibility enforced by the existing forge path**, the forged block still
flowing `self_accept → SelfAcceptedHandoff → ServedChainView`; an ineligible peer-lost/error feed
stays fail-closed/halt. Introduces **`CN-NODE-04`** (declared → enforced at S1 close) — the closed
diagnostic feed/forge event vocabulary.

## §2 Planner-is-scheduler boundary (read first — load-bearing)
The pure planner `run_loop_planner::plan_loop_step` decides **WHEN to attempt a forge tick**
(scheduling), **never** whether Ade is the leader. Leadership/epoch/KES authority stays in the
**BLUE** `forge_one_from_recovered` leader check + KES/opcert + `DC-EPOCH-03`. S2 adds only a
**scheduling** allowance (eligible empty/at-tip feed + recovered base + producer intent + a due
forge slot → emit `ForgeTick`); the forge path still enforces leader eligibility and may return
`ForgeNotLeader` (no block). The planner **never** becomes a leadership authority, and **never**
consumes `CN-NODE-04` events (emit-only, one-directional planner → log).

## §3 Load-bearing finding (the S1/S2 spine)
`NodeBlockSource` (`crates/ade_node/src/node_sync.rs`) today collapses every feed-end into a
single `is_ended: bool` — **no distinction** between an **eligible** empty/at-tip end (the
sole-producer/genesis peer had nothing) and an **ineligible** peer-loss/error end. The **closed
feed-state taxonomy is the spine**:
- **eligible:** `NoBlockAvailable` / `AtTip` / `CleanEmpty`
- **ineligible:** `PeerLost` / `DecodeError` / `ProtocolError` / `SourceInvalid`

S1 introduces the taxonomy (emit-only, **no behavior change**); S2 consumes it (the one semantic
gate). The observed C1 case (WirePump channel disconnected because the genesis peer had nothing to
serve) **MUST** classify as an eligible `CleanEmpty`/`NoBlockAvailable` case, **never** `PeerLost`
— the exact `is_ended → taxonomy` mapping is the load-bearing **OQ1** for S1.

## §4 Normative anchors
- `docs/planning/phase4-n-f-g-j-invariants.md` (`9eb6f39b`); `docs/planning/phase4-n-f-g-j-cluster-slice-plan.md` (`6461160e`).
- Registry: `DC-NODE-08`, `CN-NODE-04` (declared); preserves `DC-NODE-05/06/07`, `DC-EPOCH-03`, `CN-CINPUT-03`, `DC-CINPUT-02b`, `DC-FORGE-01`, `CN-FORGE-01..04`; cross-refs `RO-LIVE-01`.

## §5 Entry conditions (what prior clusters guarantee)
- G-A..G-E/G-F: real forge composition + `forge_one_from_recovered` + the relay-spine forge tick (`DC-NODE-05`, `CN-FORGE-*`, `DC-FORGE-01`).
- G-B/G-H: `self_accept → SelfAcceptedHandoff (DC-NODE-06) → ServedChainView (DC-NODE-07)` — the single accepted/serve path.
- G-I: the admission/pre-seed bootstrap persists the seed-epoch anchor lineage → `--mode node` WarmStart recovers a forge-capable base (`CN-CINPUT-02`). **This cluster builds directly on that recovered base.**
- The C1 net (`~/.cardano-private-testnet-c1-conway`, Conway, whole-second `systemStart`) reproduces the empty-feed halt — the cluster's regression harness.

## §6 Verified component inventory + TCB color map
| Module | Role | Color |
|---|---|---|
| `ade_node::run_loop_planner` (`plan_loop_step` / `forge_slot_status` / `LoopState` / `LoopStep` / `ForgeSlotStatus`) | pure planner; total deterministic decision table (`…precedence_table_is_total`, `…is_deterministic`). **S2 extends the table.** | **GREEN** |
| feed-state **taxonomy enum** (new; classifies the `NodeBlockSource` end/empty condition) | closed eligible/ineligible split | **GREEN** |
| `ade_node::live_log::event` (`LiveLogEvent` + allow-list) | **CN-NODE-04** vocabulary home | **GREEN** |
| `ade_node::node_sync` (`NodeBlockSource` `WirePump`/`InMemory`; `is_ended`/`has_work_ready`/`next_block`) | source shell; **the `is_ended → taxonomy` classification lands here (S1)** | **RED** |
| `ade_node::node_lifecycle` + `run_node_sync` / `run_relay_loop` | executes the planner, emits events | **RED** |
| `ade_node::live_log::writer` | event emission | **RED** |
| `forge_one_from_recovered` (`node_sync.rs`) + the `ade_core`/`ade_ledger` forge composition + leader check + `self_accept` | the forge authority — **reused VERBATIM** | **BLUE** (unchanged) |
| `ade_node::forge_intent` (`ForgeIntent`) | the producer-intent signal | **RED** (read by the planner inputs) |

## §7 Cluster Exit Criteria (CI-verifiable)
- **CE-G-J-1** (mechanical, S1): `--mode node` emits the closed, allow-listed `CN-NODE-04`
  vocabulary — `feed_unavailable{reason}` (closed reason enum, eligible `NoBlockAvailable|AtTip|
  CleanEmpty` vs ineligible `PeerLost|DecodeError|ProtocolError|SourceInvalid`) +
  `forge_tick_considered` / `forge_tick_skipped{reason}` / `forge_attempted` /
  `forge_result{outcome}` — through `live_log`. Verified by: a positive emit test
  (`node_sched_events_emit_closed_vocabulary`); an **allow-list negative test that rejects
  non-vocabulary/stringly event variants** (`node_sched_event_allowlist_rejects_unknown_variants`)
  — **fail-closed** on an unknown/added variant, **never silently dropped**; an **emit-only** CI
  gate (`ci/ci_check_node_sched_events_emit_only.sh` — `run_loop_planner`/`plan_loop_step` reads no
  `LiveLogEvent`); and a **no-behavior-change** proof (the existing
  `plan_loop_step_forge_precedence_table_is_total` + `plan_loop_step_is_deterministic` stay green,
  byte-unchanged). `CN-NODE-04` declared → enforced.
- **CE-G-J-2** (mechanical, S2): `plan_loop_step`, given an **eligible** feed + recovered base +
  `ForgeIntent::On` + a **due forge slot**, returns `ForgeTick` (was `Halt`) — leadership/epoch/KES
  still enforced by `forge_one_from_recovered` + `DC-EPOCH-03`; an **ineligible** feed still
  returns `Halt`/fail-closed; the extended table stays total + deterministic. Verified by:
  `plan_loop_step_eligible_empty_feed_forges_from_recovered_base`,
  `plan_loop_step_ineligible_feed_fails_closed`, the extended
  `plan_loop_step_forge_precedence_table_is_total`/`…is_deterministic`, and a hermetic
  `empty_feed_recovered_base_forge_tick_self_accepts` (the tick fires + the block `self_accept`s →
  `SelfAcceptedHandoff` → `ServedChainView`). BLUE forge bytes/base unchanged (`DC-FORGE-01`
  reused; `ci/ci_check_consensus_input_provenance.sh` + the `DC-NODE-06/07` handoff/serve gates
  stay byte-/semantically green). `DC-NODE-08` declared → enforced.
- **CE-G-J-3** (operator-gated, S3): a C1 forge-rerun runbook
  (`docs/evidence/phase4-n-f-g-j-c1-forge-rerun-README.md`, strict adaptation of the G-H/G-D
  operator pattern) + an env-gated harness (`ADE_LIVE_C1_FORGE_RERUN`, `node_c1_forge_rerun_live`)
  proving, on the real C1 net, that `--mode node` reaches a forge tick + self-accepts from the
  empty feed (the prior halt gone), the S1 events showing
  `forge_tick_skipped → forge_attempted → forge_result`. `blocked_until_operator_c1_forge_rerun`;
  **no synthetic evidence; no RO-LIVE flip** (peer ACCEPT operator-gated via `correlate`).

## §8 Slices
- **S1 — Closed feed-state taxonomy + diagnostic events** (CE-G-J-1) — invariant: a closed,
  allow-listed `CN-NODE-04` vocabulary + the closed feed-state taxonomy, **emit-only**, **no
  behavior change**. — **TCB: GREEN** (vocab + taxonomy) **+ RED** (classification + emission); no
  BLUE change, no scheduling change.
- **S2 — Empty/at-tip recovered-base forge scheduling** (CE-G-J-2) — invariant: the `DC-NODE-08`
  gate — extend `plan_loop_step` so an eligible empty/at-tip feed + recovered base + producer
  intent + a due forge slot yields `ForgeTick` (ineligible stays fail-closed); leadership/epoch/KES
  enforced by the existing forge path; reuse `forge_one_from_recovered` verbatim. — **TCB: GREEN**
  (planner table) **+ RED** (loop wiring); no BLUE change.
- **S3 — C1 forge rerun harness + runbook** (CE-G-J-3) — invariant: the empty-feed forge mechanism
  is exercised on the real C1 net (sole producer; the Haskell node a follower-**not**-co-producer),
  the prior halt gone, acceptance still proven only via `correlate`. — **TCB: RED** (harness +
  runbook); no BLUE change.

## §9 Replay obligations
**None new.** No new authoritative state, no new canonical type on the forge path; the forged block
+ `self_accept/handoff/served` effects stay byte-identical (`DC-FORGE-01` / `DC-NODE-06/07`).
`plan_loop_step` stays a total, deterministic table (existing tests **extended, not weakened**).
The feed-state taxonomy classifies a **RED shell input**, not authoritative state. `CN-NODE-04`
events are **operational tier**, **outside** the replay-equivalence weight class (never gate
acceptance, never read back by authority).

## §10 FC/IS partition
- **BLUE** = `ade_core`/`ade_ledger` forge composition (reused, unchanged).
- **GREEN** = `ade_node::run_loop_planner` + the feed-state taxonomy enum + `ade_node::live_log::event`.
- **RED** = `ade_node::node_sync` (`NodeBlockSource`), `ade_node::node_lifecycle` (relay loop), `ade_node::live_log::writer`.

## §11 Forbidden during this cluster (inherited by every slice)
- No co-producer workaround; no private-only/C1-only flag or branch.
- No bypass of `import_live_consensus_inputs`; forge base is the recovered surface only
  (`CN-CINPUT-03`/`DC-CINPUT-02b` byte-/semantically unchanged); no forge from unanchored genesis;
  no stale-base forge.
- No durable tip advance from forge scheduling alone; no serve of non-self-accepted bytes
  (`DC-NODE-06/07` unweakened); no BLUE change to the forge composition.
- The planner stays a **scheduler, never a leadership authority**; the planner **never consumes**
  `CN-NODE-04` events (emit-only).
- No peer-acceptance/BA-02 claim without a real peer log through `ba02_evidence::correlate`; **no
  `RO-LIVE-01/06` flip** at implementation close.

## §12 Non-goals
- A proactive serve `advance_tip` driver (separate cluster, if a parked follower needs it).
- Multi-producer / co-producer topology; any change to the leader-election authority.
- Any C2/preprod behavior change — the Continuing-feed path is byte-unchanged for them.

## §13 Open questions (carried to slice docs)
- **OQ1 (load-bearing, S1):** the exact `is_ended → closed feed_state` mapping in `NodeBlockSource`
  — which concrete WirePump end conditions map to eligible (`NoBlockAvailable|AtTip|CleanEmpty`) vs
  ineligible (`PeerLost|…`). The C1 disconnect **MUST** be eligible.
- **OQ-color (S2):** confirm `plan_loop_step` stays a separable pure GREEN fn — **extend the table,
  do not embed leadership**.
- **OQ-liveness (S2):** an eligible empty feed at a **non-forge-slot** must `Idle` (bounded), never
  busy-loop; confirm the existing `NoWorkReady` idle/backpressure bounds it.
