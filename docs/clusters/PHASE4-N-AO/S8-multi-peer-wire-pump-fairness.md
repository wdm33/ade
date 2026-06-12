# Invariant Slice S8 — Multi-peer wire-pump fairness for live SELECT

> **No connected peer may be starved from the participant receive path by another continuously-producing peer; live SELECT must receive bounded, peer-attributed input from each active peer so competing candidates can reach fork-choice.**
>
> Slice of cluster PHASE4-N-AO. The S7 live retry (CE-AO-6, `cabe61ff`) proved S7 fixed the last SELECT-geometry bug (no more `UnexpectedRollback`) but surfaced a gap **below** SELECT: per-peer pump tasks share **one** bounded `mpsc`, so a continuously-growing peer monopolises it and the other peer's branch never reaches the dispatch. RED feed/scheduling discipline only — **no** selector change, **no** S7 change, **no** BLUE change.

## 2. Slice Header
- **Slice Name:** Per-peer bounded queues + deterministic fair merge for the live WirePump source.
- **Cluster:** PHASE4-N-AO.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-9** (`DC-PUMP-04` multi-peer fairness) — with N peers connected, each active peer's blocks reach the participant loop (no starvation); a hot peer backpressures its **own** queue, not the shared path; peer identity preserved; deterministic configured-`--peer`-order merge; a disconnected lane is retired without reordering the remaining peers; single-peer behavior unchanged. New gate `ci/ci_check_wire_pump_fairness.sh` + hermetic tests + `cargo test -p ade_node` green. **Unblocks the CE-AO-6 live retry** (both branches reach S7's dispatch → the LCA walk fires).
- **Slice Dependencies:** S1–S7 (`DC-NODE-34..38`); the live feed wiring (N-F-G-C, `DC-PUMP-01..03`).

## 4. Intent
Give each connected peer its own bounded delivery lane so competing candidates from **every** active peer reach fork-choice. Introduces `DC-PUMP-04`. The fairness/merge layer is **RED scheduling discipline** — it may affect delivery *opportunity* but MUST NOT decide fork-choice; `select_best_chain` stays arrival-order independent (`CN-CONS-01`).

## 5. Scope
- **RED `ade_node::node_lifecycle::spawn_live_wire_pump_source`:** replace the single shared `mpsc` with **per-peer bounded channels** (one per `--peer`), each fed by that peer's `run_admission_wire_pump` task, drained by a **fair-merge task** that forwards into the single merged `rx` the closed `NodeBlockSource::WirePump` arm already consumes.
- **RED new `ade_node` fair-merge helper:** round-robin over the per-peer receivers in a **deterministic order derived from the configured `--peer` list** (one delivery opportunity per peer per round; await readiness when all empty — no busy-loop; a disconnected peer's lane is retired in place, leaving the remaining peers' order stable). Backpressure is **per-peer** (a hot peer fills its own bounded queue → its pump blocks), never global.
- **GREEN `ade_node`:** a deterministic **per-peer delivered-count** evidence counter (fairness assertion / transcript), evidence-only — never alters scheduling.
- **Reused, UNCHANGED:** `run_admission_wire_pump` (per-peer, already peer-labelled); `NodeBlockSource::WirePump`/`next_item`/`pump_lookahead` (still one merged peer-attributed stream); the participant loop; S2/S3/S7; `select_best_chain`.
- **Out of scope:** any selector / S7 / BLUE change (unless a hermetic test exposes a real integration miss); the `CN-CONS-03` flip (gated on the live CE-AO-6 retry).

## 6. Execution Boundary (TCB color)
- **RED:** `spawn_live_wire_pump_source` (per-peer channels + merge task) + the fair-merge helper (scheduling/backpressure/disconnect handling).
- **GREEN:** per-peer delivered-count evidence counters / fairness assertions.
- **BLUE:** unchanged — `select_best_chain`, `walk_to_durable_lca`, `build_candidate_fragment`, validate/apply.

## 7. Invariants Preserved (registry IDs)
`CN-CONS-01` (chain selection deterministic / arrival-order independent — the merge order changes opportunity, never the selected winner); `DC-NODE-34` (peer identity preserved through the flatten — now through the per-peer lanes + merge); `DC-PUMP-01..03` (per-peer pump contract, keep-alive, fragment reassembly); `DC-NODE-38` (S7 LCA walk — unchanged; now reachable for the second peer).

## 8. Invariants Strengthened / Introduced
- **Introduces `DC-PUMP-04`** (declared): *Multi-peer wire-pump fairness.* When multiple peers are connected to the participant receive path, no connected peer may be starved by another continuously-producing peer. Each peer's pump feeds its **own** bounded queue; a fair merge over a **deterministic order derived from the configured `--peer` order** gives each active peer bounded delivery opportunity; backpressure is **per-peer**, never global; a disconnected lane is retired without reordering the remaining peers. The merge order is RED scheduling discipline ONLY — it may affect delivery opportunity but MUST NOT decide fork-choice (`select_best_chain` stays arrival-order independent). Flips `declared → enforced` at `/cluster-close`.

## 9. Design Summary
```
peer_i pump ─▶ per_peer_tx_i ─▶ per_peer_rx_i ─┐
                                                ├─ fair_merge (round-robin, deterministic --peer order) ─▶ merged_tx ─▶ merged_rx ─▶ WirePump{rx} ─▶ next_item
peer_j pump ─▶ per_peer_tx_j ─▶ per_peer_rx_j ─┘
```
(replaces: *all* pumps → one shared bounded `mpsc(64)` → `WirePump{rx}`.) Each peer gets its own bounded lane, so a hot peer's flood fills **its own** queue (self-backpressure) while the merge keeps draining the quiet peer's lane. The merged stream the consumer reads is unchanged in shape (one peer-attributed `NodeSyncItem::Block` sequence) — so the participant loop, S2/S3/S7, and the selector are untouched. The merge order is a **RED deterministic scheduling order** (an explicit `Vec` derived from `cli.peer_addrs`), NOT a canonical/semantic authority — it affects which lane is polled first per round, never the fork-choice winner.

## 10. Changes Introduced
- **Types:** a small RED `FairMerge`/per-peer-lane structure (a `Vec` of receivers in configured-`--peer` order + a round-robin cursor); GREEN per-peer counter. No BLUE/canonical/persisted type. The peer order is an explicit `Vec` from `cli.peer_addrs` — **never** `HashMap`/`HashSet` iteration.
- **State transitions:** `spawn_live_wire_pump_source`: shared-channel fan-in → per-peer lanes + fair merge.

## 11. Replay / Crash / Epoch Validation
No new durable state (RED in-memory scheduling). Replay equivalence is unaffected: the merge changes delivery *order opportunity*, and the BLUE post-state is arrival-order independent (`CN-CONS-01`) — a per-peer counter is the only added (GREEN, non-authoritative) observable.

## 12. Mechanical Acceptance Criteria
- [ ] `hot_peer_cannot_starve_quiet_peer` — a peer flooding its lane to capacity does NOT prevent a quiet peer's blocks from reaching the merged consumer (both peers' items observed).
- [ ] `per_peer_backpressure_not_global` — a full per-peer queue backpressures only that peer's pump; the other lane keeps draining.
- [ ] `peer_identity_preserved_through_merge` — merged items keep the correct `peer` (extends `DC-NODE-34` to the merge).
- [ ] `deterministic_peer_order_from_config` — the merge order derives from the explicit `--peer` list `Vec`, not `HashMap`/`HashSet`/scheduler timing; same lanes → same round-robin sequence.
- [ ] `closed_lane_removed_without_reordering_remaining_peers` — a disconnected peer's lane is retired deterministically; the remaining peers' relative order + round-robin fairness stay stable (no churn-induced reorder).
- [ ] `single_peer_behaviour_unchanged` — one `--peer` degenerates to the prior single-stream behavior (regression).
- [ ] New gate **`ci/ci_check_wire_pump_fairness.sh`**: per-peer channels (no single shared fan-in), deterministic `Vec`-ordered merge (no `HashMap`/`HashSet` peer iteration, no `rand`/wall-clock in the merge), no block dropped because another peer is hot.
- [ ] `cargo test -p ade_node` green.
- [ ] **Live (CE-AO-6 retry, gated — NOT this slice):** `blocks_received_from_peer_:6001 > 0` AND `blocks_received_from_peer_:6002 > 0` · `NeedsForkChoice` observed · `last_common_ancestor_discovered` · `multi_header_candidate_built` · `select_best_chain` winner · `RequestRange(LCA → winner_tip)` · `prevalidate_branch` over N bodies · `RollBack{ForkChoiceWin}` · `ChainSelected × N` · `agreement_verdict{agreed}` · 0 diverged.

## 13. Failure Modes (all → no BLUE effect)
A peer's lane full → that peer's pump backpressures (bounded), others unaffected. A peer disconnects → its lane retired in place, merge continues over the remaining peers (no reorder). All deterministic; no wall-clock/`rand` in the merge order.

## 14. Hard Prohibitions
- **No peer input may gain semantic priority from scheduler timing alone.**
- **No `HashMap`/`HashSet` iteration may define authoritative peer order** — the merge order is an explicit **deterministic** `Vec` derived from the configured `--peer` order (RED scheduling, not canonical/semantic authority).
- **No wall-clock timing may affect BLUE selection results.**
- **No dropping a peer's fork-choice-relevant block merely because another peer is hot** — per-peer backpressure, not drop.
- **No selector / S7 / BLUE change** (unless a hermetic test exposes a real integration miss).
- **No `CN-CONS-03` flip** until both peer branches are observed AND SELECT actually fires (CE-AO-6 retry).

## 15. Explicit Non-Goals
No `CN-CONS-03` flip; no fork-choice / S7 / BLUE change; no new durable state; the merge order is delivery-opportunity only, never fork-choice authority.

## 16. Completion Checklist
- [ ] Per-peer bounded lanes + deterministic configured-order fair merge in `spawn_live_wire_pump_source`; consumer shape unchanged.
- [ ] GREEN per-peer delivered counter (evidence-only).
- [ ] 6 hermetic tests + `ci_check_wire_pump_fairness.sh` green; `cargo test -p ade_node` green.
- [ ] `DC-PUMP-04` declared; ready to flip at `/cluster-close`; CE-AO-6 live retry unblocked.
