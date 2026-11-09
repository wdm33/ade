# Invariant Slice S1 — Peer-identity restoration

> Slice of cluster PHASE4-N-AO (`docs/clusters/PHASE4-N-AO/cluster.md`, committed `a87a4eb5`). The cluster's **first** slice — the SELECT foundation. RED/GREEN, **no BLUE**. Latent-until-wired (the consumer is S2 / `DC-NODE-35`), mirroring AI-S1's proven-but-latent shape.

## 2. Slice Header
- **Slice Name:** Peer-identity restoration (`NodeSyncItem` carries its origin `peer`; FOLLOW byte-unchanged).
- **Cluster:** PHASE4-N-AO — live multi-candidate fork-choice SELECT + adopt (rung-2).
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:**
  - [ ] **CE-AO-1** (`DC-NODE-34` peer identity) — a `NodeSyncItem` carries its origin `peer`; a single-peer FOLLOW run admits + replays **byte-identically** to the pre-S1 baseline; new gate `ci/ci_check_peer_identity_preserved.sh` green; `cargo test -p ade_node` green.
- **Slice Dependencies:** none (first slice; S2…S5 depend on it).

## 4. Intent
Make the live receive feed **provenance-faithful**: the origin peer of every received item is preserved end-to-end (`AdmissionPeerEvent.peer` → `NodeSyncItem` → the participant/single-producer loop) instead of being discarded at the `NodeBlockSource → NodeSyncItem` boundary — the precondition that makes per-peer candidate tracking (`DC-NODE-35`) possible. Restoration is **provenance-only**: it strengthens `DC-NODE-34` and **must not** alter any selection, admission, rollback, or evidence-verdict outcome.

## 5. Scope
- **Modules / crates:** RED/GREEN `ade_node::node_sync` — `NodeSyncItem` (add `peer` to both variants), `pump_lookahead` (`:203`/`:217` — capture `peer` instead of `..`), the `next_item` blocking-recv conversion path, the `in_memory*` constructors. RED `ade_node::node_lifecycle` — `run_node_sync` (`:478`) + `run_participant_sync` (`:2510`) item consumers (destructure-and-ignore the new field). Reused unchanged: `ade_runtime::admission::wire_pump` (`AdmissionPeerEvent.peer` — the existing carrier).
- **State machines affected:** none.
- **Persistence impact:** **none** — `NodeSyncItem` is a transient feed type (never persisted / hashed / serialized); no WAL/checkpoint change; **no canonical-type or replay-corpus obligation**.
- **Network-visible impact:** none.
- **Out of scope:** per-peer candidate aggregation (S2 / `DC-NODE-35`); any consumption of the `peer` field for selection/dispatch/fetch (S2–S4); per-peer convergence-evidence attribution (the `peer` field stays **latent** in S1, exactly as AI-S1's `WalEntry::RollBack` was latent until AI-S3).

## 6. Execution Boundary (TCB color)
- **BLUE:** none.
- **GREEN:** none new (the detector/resolver `classify_receive`/`resolve_disposition` are untouched).
- **RED:** `ade_node::node_sync` (`NodeSyncItem` + the two conversion sites + constructors — RED scheduling/feed shell) and `ade_node::node_lifecycle` (`run_node_sync` / `run_participant_sync` consumers).

*No ambiguous colors: `NodeSyncItem` is a transient RED feed item, not a canonical/persisted type. The `peer` label is not semantic authority in S1.*

## 7. Invariants Preserved (registry IDs)
`DC-NODE-23`/`DC-NODE-24` (detector/resolver — peer is **not** observed by classification in S1), `DC-NODE-25`/`26`/`27`/`28`/`29` (apply/reconcile/replay/forge-fence/rollback-binding — unchanged), `DC-NODE-30` (convergence evidence — untouched), `DC-NODE-05`/`12` (pump_block sole admit), `DC-NODE-15`/`16`/`20`/`33` (forge admissibility / idempotency / forge base / participant anchor no-op), `DC-CONS-03` (fork-choice — **not reached**), `DC-CONS-20` (lockstep), `T-REC-05` / `DC-WAL-02` (recover→follow replay-equivalence — the byte-unchanged proof), `DC-PUMP-01`/`02`/`03` (wire pump + `AdmissionPeerEvent.peer` carrier — read, not changed).

## 8. Invariants Strengthened / Introduced
- **Strengthens toward enforced** `DC-NODE-34` (peer-identity restoration) — the mechanism: peer identity survives the `NodeBlockSource → NodeSyncItem` boundary, and a single-peer FOLLOW run is byte-unchanged. `DC-NODE-34` flips `declared → enforced` at S1 close (CE-AO-1). **One invariant family:** receive-feed peer provenance. *(The consumer — per-peer aggregation — is S2 / `DC-NODE-35`, latent until S3.)*

## 9. Design Summary
Add `peer: String` (mirroring `AdmissionPeerEvent.peer` — no new type, minimal per OQ-AO-1's lean) to **both** `NodeSyncItem` variants: `Block { peer, bytes }` | `RollBack { peer, point }`. The **two** RED conversion sites that today drop it — `pump_lookahead` (non-blocking `try_recv`, `:203`/`:217`) and the `next_item` blocking-`recv` path — bind `peer` from the `AdmissionPeerEvent` instead of `..`. The `in_memory*` hermetic constructors tag items with a fixed sentinel peer (hermetic feeds have no real peer; the sentinel is never consumed). The two consumers (`run_node_sync`, `run_participant_sync`) destructure the field and **ignore it** (`_peer`) — no branch, no decision keyed on `peer`. The exhaustive `match` on `NodeSyncItem` makes every site a compile error until updated. *Latency, not dormancy:* the field is proven preserved hermetically now; it is consumed in S2.

## 10. Changes Introduced
- **Types:** `NodeSyncItem::Block { peer: String, bytes: Vec<u8> }` + `NodeSyncItem::RollBack { peer: String, point: Point }` (was tuple variants `Block(Vec<u8>)` / `RollBack(Point)`). *No canonical/persisted type — transient feed item.*
- **State transitions:** none.
- **Persistence:** none.
- **Removal / refactors:** the `{ block_bytes, .. }` / `{ point, .. }` peer-discards at `node_sync.rs:203/:217` (and the `next_item` blocking path) replaced with explicit `peer` capture.

## 11. Replay / Crash / Epoch Validation
- **Preservation tests** (`ade_node::node_sync`): `node_sync_item_carries_peer_from_wire_pump` (a WirePump feed of `Block{peer:"P1",..}` + `RollBackward{peer:"P1",..}` yields `NodeSyncItem::Block{peer:"P1",..}` + `RollBack{peer:"P1",..}` via `pump_lookahead`); `node_sync_item_carries_peer_blocking_recv_path` (the `next_item` blocking-recv path preserves `peer`).
- **FOLLOW byte-unchanged:** `follow_single_peer_durable_output_unchanged_after_peer_threading` — a single-peer `run_participant_sync` over a tagged `in_memory_items` feed yields the **same** durable tip + ledger fingerprint + WAL bytes as the pre-S1 fixture; **the existing participant/single-producer suite** (`crates/ade_node/tests/live_fork_choice_ai_s4bii.rs` participant tests + the `node_sync.rs` feed tests at `:1390-1421`) passes with **no expected-value change** (only `NodeSyncItem` construction gains `peer`).
- **Crash/restart:** unchanged — no persisted state touched (replay-equivalence is the FOLLOW byte-unchanged assertion).
- **Epoch boundary:** not applicable.

## 12. Mechanical Acceptance Criteria
- [ ] `node_sync_item_carries_peer_from_wire_pump` — peer preserved through `pump_lookahead` for both `Block` and `RollBack`.
- [ ] `node_sync_item_carries_peer_blocking_recv_path` — peer preserved through the `next_item` blocking-recv conversion.
- [ ] `follow_single_peer_durable_output_unchanged_after_peer_threading` — single-peer FOLLOW durable tip + ledger fp + WAL byte-identical to the pre-S1 fixture.
- [ ] The existing participant/single-producer test suite passes unchanged (`cargo test -p ade_node` green).
- [ ] New gate **`ci/ci_check_peer_identity_preserved.sh`** green, asserting: (A) `NodeSyncItem::Block` + `::RollBack` each carry a `peer` field; (B) both conversion sites capture `peer` from `AdmissionPeerEvent` (no `peer`-dropping `..`); (C) **no consumer branches on `peer`** — `run_node_sync` / `run_participant_sync` key no selection/admission/rollback/verdict on it (the non-goal fence, grep-enforced); (D) `NodeSyncItem` gains no `encode`/`decode`/serialization (stays transient / non-persisted).

## 13. Failure Modes
**None introduced.** `peer` is an always-present provenance label (`AdmissionPeerEvent.peer` is non-optional); the conversion cannot fail on it. S1 adds no fail-closed path, no new error variant, and no consensus/replay-affecting failure.

## 14. Hard Prohibitions
**Inherits all nine cluster hard lines** (cluster doc §8) — esp. #1 (no live `select_best_chain` dispatch), #9 (no new BLUE canonical type). **Slice-specific:**
- **No consumer may branch on `peer`** — no selection/admission/rollback/verdict/evidence outcome may depend on it in S1 (the non-goal fence).
- `peer` is **provenance-only** — carried, not yet consumed (no aggregation, no per-peer state).
- `NodeSyncItem` stays a **transient feed type** — no `encode`/`decode`/persistence/hashing added (no canonical-type creep).
- No new BLUE; `classify_receive`/`resolve_disposition`/`select_best_chain`/`pump_block` untouched.
- No `String`-keyed semantic logic; `peer` is an opaque label only.

## 15. Explicit Non-Goals
No candidate aggregation (S2); no `select_best_chain` dispatch (S3); no fork-switch/range-fetch (S4); no per-peer convergence-evidence attribution; no multi-peer behavior change; no new fork-choice/consensus; no performance work; no config/feature flags; no newtype `PeerId` (mirror `AdmissionPeerEvent.peer: String` for minimality — a newtype is a possible later tightening, not this slice).

## 16. Completion Checklist
- [ ] `peer` added to both `NodeSyncItem` variants; all exhaustive `match` sites updated (compile-forced).
- [ ] Both conversion sites (`pump_lookahead` + `next_item` blocking path) capture `peer`; constructors tag a sentinel for in-memory feeds.
- [ ] Consumers destructure-and-ignore `peer` (no branch).
- [ ] Preservation + FOLLOW-byte-unchanged tests pass; existing suite unchanged.
- [ ] `ci/ci_check_peer_identity_preserved.sh` green; `cargo test -p ade_node` green.
- [ ] No persisted/canonical type; `DC-NODE-34` ready to flip `declared → enforced` at close.
