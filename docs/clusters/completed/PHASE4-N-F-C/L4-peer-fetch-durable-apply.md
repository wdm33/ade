# Slice PHASE4-N-F-C / L4 — Peer BlockFetch → durable validated apply

> Connects the two mature halves the cluster doc records as "never connected":
> `admission::wire_pump` (real N2N `BlockFetch` source — but admission only admits + WALs a
> verdict; it does NOT persist block bytes, advance a recoverable tip, or call `forward_sync`)
> and `forward_sync::pump_block` (durable validated apply → `PersistentChainDb` bytes + WAL
> `AdmitBlock`, durable-before-tip — but with zero production callers / no fetcher). L4 wires
> the first into the second so the `--mode node` lifecycle reaches a **recoverable** selected
> tip, then proves recovery round-trips through the L3 warm-start. Authority doc: `cluster.md`.
> Plan: `../../planning/phase4-n-f-c-cluster-slice-plan.md`. Invariant sketch:
> `../../planning/phase4-n-f-c-invariants.md`. Builds on L1 (owner), L2 (first-run), L3
> (warm-start recovery — `warm_start_recovery` is the L4c recovery target).

## 2. Slice Header
- **Slice Name:** Integrate the peer `BlockFetch` byte source (`admission::wire_pump`) with the
  durable validated-apply engine (`forward_sync::pump_block`) on the `--mode node` lifecycle
  path: ordered peer block bytes → `pump_block` (first production caller) → `PersistentChainDb`
  block bytes + WAL `AdmitBlock` durable-before-tip → a **recoverable** selected tip that a
  kill + L3 warm-start recovers byte-identically.
- **Cluster:** PHASE4-N-F-C — Build the real Ade node lifecycle.
- **Status:** Proposed.
- **Cluster Exit Criteria Addressed:** CE-L-4.
- **Slice Dependencies:** L1 (`--mode node` owner + persistent stores), L3
  (`warm_start_recovery` — the recovery the L4c proof round-trips against; L4c extends its RED
  driver to populate the replay `block_bytes` map for `AdmitBlock` entries from `PersistentChainDb`,
  matching the `recover_node_state` pattern — no BLUE replay/bootstrap change).
  L2 (first-run bootstrap that seeds the anchor lineage + sidecar) is the precondition the
  applied tip is anchored to.

### Split (inside this slice)
- **L4a — Block-source abstraction.** Extract an ordered peer-block-bytes source from the N2N
  `BlockFetch` `RequestRange`→`Block{bytes}` stream (`admission::wire_pump`), behind a closed
  interface, decoupled from admission's verdict loop.
- **L4b — Durable apply.** Drive `forward_sync::pump_block` (first production caller) from the
  L4a source: validated apply → `PersistentChainDb` bytes + WAL `AdmitBlock`, durable-before-tip.
- **L4c — Selected-tip recovery proof.** After L4b advances the tip, a kill + warm-start (L3)
  recovers byte-identically to the same selected tip — the join point between sync and recovery.

## 3. Implementation Instruction (AI)
Implement §10 only — the L4a source seam, the L4b `pump_block` driver on the lifecycle path,
the L4c recovery proof, and the CI gate. Do NOT forge (`run_real_forge` / any forge handler),
do NOT convert or touch `produce_mode`, do NOT read `--consensus-inputs-path` as a forge/runtime
input, do NOT add a genesis fallback, do NOT emit any BA-02 / peer-accept claim, do NOT treat
`ade_core_interop::follow` as validating sync, and do NOT treat admission's verdict-only flow as
recoverable sync. `pump_block`, `forward_sync_step`, `ForwardSyncState`, and
`run_admission_wire_pump` already exist and are consumed verbatim. **L4c extends the RED
`warm_start_recovery` driver to populate the replay `block_bytes` map for `AdmitBlock` entries from
`PersistentChainDb`, matching the `recover_node_state` pattern. This is required once L4b appends
`AdmitBlock` entries and does not alter BLUE replay or bootstrap authorities.** **Add no new BLUE
authority** and do not change the BLUE admit chokepoint, the reducer, or `bootstrap.rs`/`replay.rs`.
Resolve the entry obligations in §9.0 before coding. Commit with the model-attribution trailer.

## 4. Intent
Make the Ade node's selected tip a **durably recoverable** fact established only by validated
durable apply: blocks reach the tip via the real peer `BlockFetch` source feeding the
durable-before-tip `pump_block`, and the advanced tip survives a kill + L3 warm-start
byte-identically. No "sync-to-tip" is representable on the lifecycle path through admission's
verdict-only loop or through a non-validating follower — the only path that advances a
recoverable tip is fetch → `pump_block`.

## 5. Scope
- **Modules / crates:**
  - `ade_runtime::admission::wire_pump` (RED) — **consumed unchanged** as the block source; L4a
    adds only a thin adaptor (see §9.1) that exposes its `AdmissionPeerEvent::Block { block_bytes }`
    stream as an ordered byte source, decoupled from any verdict logic.
  - `ade_runtime::forward_sync::{pump, reducer}` (RED pump / GREEN reducer) — **consumed
    unchanged**; L4b is `pump_block`'s first production caller.
  - `ade_node::node_lifecycle` (RED) — the lifecycle owner gains an L4 sync step that builds a
    `ForwardSyncState` from the bootstrapped/recovered `BootstrapState` and drives `pump_block`
    over the L4a source against the owner's `PersistentChainDb` + `FileWalStore`.
  - `ci/ci_check_node_sync_via_pump.sh` — **new** (see §9.4): proves the node-lifecycle sync path
    *calls* `pump_block` and advances the tip ONLY via it — no manual tip advance, no
    `ade_core_interop::follow` as validating sync, no `derive_verdict`/`run_admission` as sync, no
    `run_real_forge`, no `InMemoryChainDb`, no `--consensus-inputs-path` read at sync time.
- **State machines affected:** none new. L4 composes the existing GREEN `forward_sync` reducer +
  the BLUE admit chokepoint it already wraps.
- **Persistence impact:** L4b is the first production path that writes **block bytes** +
  `WalEntry::AdmitBlock` on the `--mode node` path (via `pump_block`'s existing effects). No new
  persisted format. The seed-epoch sidecar + provenance (L2) are untouched.
- **Network-visible impact:** consumes the existing N2N `BlockFetch` initiator stream
  (`run_admission_wire_pump`); no new wire messages, no serve-side change.
- **Out of scope:** L5 (produce / consume fence), L6 (BA-02); any forge; any `produce_mode`
  change; any new BLUE authority; multi-peer fork choice (single ordered source this slice);
  in-flight RollForward block fetching (the wire pump's documented deferral stays deferred).

## 6. Execution Boundary (TCB color)
- **BLUE (reuse only — no change):** the admit chokepoint inside `forward_sync_step`
  (`ade_ledger::receive` / `admit_via_block_validity`), `decode_block`, the WAL entry codec, and
  the L3 warm-start verify chain the L4c proof round-trips through.
- **GREEN (reuse only):** `forward_sync::reducer::{forward_sync_step, AdmitPlan, SyncEffect}` —
  the closed durable-before-tip plan (DC-SYNC-01); `ForwardSyncState`.
- **RED:** `run_admission_wire_pump` (the BlockFetch source, unchanged); the L4a source adaptor;
  the L4b `pump_block` driver loop in `node_lifecycle`; the L4c kill/restart test harness.
- **CI:** the new `ci_check_node_sync_via_pump.sh`.

## 7. Invariants Preserved
- `DC-SYNC-01` — durable-before-tip: `pump_block` issues `AdvanceTip` only after
  `StoreBlockBytes` + `AppendWal` returned Ok. L4 drives it; it does not weaken the ordering
  (the `AdmitPlan` constructor still fixes it).
- `CN-NODE-01` — initial state still flows only through the single `bootstrap_initial_state`
  (L1/L2/L3); L4 advances the tip from that recovered/bootstrapped base, it does not introduce a
  second bootstrap/storage-init authority.
- `DC-WAL-03` / the WAL fingerprint chain — `pump_block` appends `AdmitBlock` whose `prior_fp`
  links the chain from the anchor; `replay_from_anchor` (L3) still validates it.
- `CN-STORE-02` — persistent `PersistentChainDb` + `FileWalStore` on the lifecycle path (no
  `InMemoryChainDb`).
- `CN-CINPUT-02` — the seed-epoch sidecar populate path stays contained to verified bootstrap;
  L4 writes block bytes + `AdmitBlock`, never the sidecar.
- The BLUE admit chokepoint, the diagnostic `produce_mode` / `admission` (verdict) paths, and the
  L2 FirstRun / L3 WarmStart arms — unchanged.

## 8. Invariants Strengthened or Introduced
- **Strengthens `DC-SYNC-01` (registry id `DC-SYNC-01`): `pump_block` gains its first production
  driver** on the `--mode node` lifecycle path (previously zero production callers). The enforcing
  artifacts are the L4b durable-apply test + the new `ci_check_node_sync_via_pump.sh` gate.
- **Strengthens `T-REC-01` / `T-REC-02` / `DC-WAL-03`**: the L4c kill→warm-start proof extends
  recovery byte-identity from the L3 sidecar-only precondition to a **sync-advanced** tip (the
  precondition L3's W1 noted is created naturally by durable apply — now demonstrated).
- No registry status flip in-slice; `strengthened_in += "PHASE4-N-F-C"` and any `tests`/
  `ci_scripts` appends are recorded at `/cluster-close` (consistent with L1/L2/L3). DC-CINPUT-01
  stays partial (L3's evidence) — L4 does not touch the produce/consume surface.

## 9. Design Summary

### 9.0 Entry obligations (resolve before coding)
- **(E1) Single ordered source, not multi-peer fork choice.** `run_admission_wire_pump` is
  per-peer and admission spawns one pump per `--peer`. L4 drives `pump_block` from **one**
  ordered block-bytes source; multi-peer arrival interleaving + fork choice is NOT in scope (no
  `fork_choice` call on the lifecycle path this slice). The L4a adaptor therefore consumes a
  single pump's `AdmissionPeerEvent::Block` stream (one peer, or a deterministic in-memory feed
  for the hermetic test). **L4 proves durable apply from one ordered peer source. Multi-peer
  interleaving, peer selection, and fork-choice are explicitly out of scope.** L4 should prove the
  durable validated path exists before expanding the network topology.
- **(E2) The L4a source must yield ONLY block bytes, decoupled from verdicts.** The wire pump
  already emits a closed `AdmissionPeerEvent` sum (`Block { block_bytes }`, `TipUpdate`,
  `Disconnected`). L4a's adaptor selects `Block` → ordered `Vec<u8>` and drops `TipUpdate`/
  `Disconnected` from the *apply* path (a clean disconnect ends the feed; it is not a tip
  authority). The adaptor must not invoke `derive_verdict` / admission's verdict loop — the
  lifecycle tip is a durable-apply fact, not an agreement verdict (¬ verdict-as-sync).
- **(E3) `ForwardSyncState` seed = the bootstrapped/recovered base.** `pump_block` needs a
  `ForwardSyncState` seeded from `ReceiveState::new(ledger, chain_dep)` + the anchor
  fingerprint. On the lifecycle path that `(ledger, chain_dep)` is the `BootstrapState` L2
  (first run) or L3 (warm start) produced, and the anchor fingerprint is the same independent
  anchor_fp L3 discovers. L4 threads those through — it does NOT re-bootstrap and does NOT
  fabricate a fresh genesis state (¬ genesis fallback).
- **(E4) Pinned: L4b captures the advanced-tip snapshot via `PersistentSnapshotCache::capture`;
  L4c recovers from that persisted checkpoint.** After a `pump_block` for the selected tip returns
  Ok, the L4b driver captures a checkpoint **at the advanced-tip slot** using
  `PersistentSnapshotCache::new(chaindb).capture(tip.slot, &ledger, &chain_dep)` — the exact path
  L3's `warm_start_recovery` reads back (via `nearest_le` → `decode_snapshot`). This makes L3's
  snapshot-at-tip requirement (W1) satisfied by a genuine durable artifact of the apply path.
  **After successful durable apply, L4 captures the tip snapshot using the same
  `PersistentSnapshotCache` path read by warm-start recovery. L4c must recover from that persisted
  checkpoint. A hand-constructed recovery precondition is not acceptable for L4c.** The L4c test
  runs L4b (which performs the capture), drops the handles, reopens, runs `warm_start_recovery`,
  and asserts the recovered tip equals the L4b-advanced tip — with NO test-side snapshot injection.

### 9.1 L4a — block-source adaptor
A thin RED adaptor that owns (or receives) the `mpsc::Receiver<AdmissionPeerEvent>` a single
`run_admission_wire_pump` feeds, and exposes "next ordered block bytes" by selecting the `Block`
variant. Closed surface: it yields `Option<Vec<u8>>` (None on clean end-of-feed) and never
surfaces a verdict, tip-agreement, or follow decision. For the hermetic test the same surface is
fed from a deterministic in-memory `Vec<Vec<u8>>` (corpus blocks), so L4b is provable without a
live socket — exactly the `pump_block` design note ("a live socket is not required").

### 9.2 L4b — durable apply driver
On the lifecycle path, after bootstrap/recovery yields `BootstrapState { ledger, chain_dep, tip,
.. }`, build `ForwardSyncState::new(ReceiveState::new(ledger, chain_dep), anchor_fp,
SnapshotCadence::DEFAULT)` and, for each block from the L4a source, call:

```
pump_block(&mut state, chaindb, wal, snapshot_sink, &block_bytes, era_schedule, ledger_view)
```

over the owner's `PersistentChainDb` + `FileWalStore`. `pump_block`'s existing `apply_plan`
guarantees `StoreBlockBytes` + `AppendWal` are durable before `AdvanceTip` (DC-SYNC-01). The
driver records the latest `PumpTip` as the selected tip. A `PumpError` halts the lifecycle
fail-closed (typed, non-zero exit) — no skip-and-continue past a rejected block.

**Tip-snapshot capture (E4, pinned).** After the `pump_block` that advances the selected tip
returns Ok, the driver captures a checkpoint at that tip:
`PersistentSnapshotCache::new(chaindb).capture(tip.slot, &state.receive.ledger,
&state.receive.chain_dep)`. This is the SAME `PersistentSnapshotCache` path L3's
`warm_start_recovery` reads back, so the snapshot-at-tip precondition L3-W1 requires is created by
the apply path itself — not by a test fixture. A capture failure halts fail-closed (typed,
non-zero) like any durability failure; an unrecoverable tip is never reported as success.

### 9.3 L4c — selected-tip recovery proof
The join point: `run L4b over an ordered sequence (which captures the tip checkpoint via
PersistentSnapshotCache, E4) → advanced PumpTip T → drop handles → reopen PersistentChainDb +
FileWalStore → warm_start_recovery → recovered BootstrapState`. Assert the recovered tip slot+hash
equals T, and (carrying the L3 byte-identity property) the recovered seed-epoch sidecar still
verifies. **The test injects NO snapshot of its own — recovery must succeed from the checkpoint
L4b's apply path persisted.** This demonstrates the L3-W1 claim mechanically: durable apply is what
naturally creates the warm-startable precondition.

### 9.4 CI gate (new) — usage, not mere existence
`ci_check_node_sync_via_pump.sh` must prove the lifecycle sync path *uses* `pump_block` AND does
not advance the tip through any other path — not merely that `pump_block` exists somewhere. On the
lifecycle owner (comment- + `#[cfg(test)]`-stripped):
- **POSITIVE:** the owner's sync path calls `pump_block(` (the durable apply engine), and the
  advanced tip the lifecycle reports comes from that call's `PumpTip`.
- **NEGATIVE (no alternate tip-advance / no verdict-as-sync):** the sync path calls NO
  `ade_core_interop::` / `follow(` (follow is not validating sync), NO `derive_verdict(` /
  `run_admission(` (admission's verdict derivation is not sync), and performs NO manual tip
  advancement outside `pump_block` — i.e. the owner's sync path does not itself call
  `ChainDb::put_block(` / a direct tip write / `AdvanceTip` construction to move the tip (the only
  tip-advancing site on the lifecycle path is `pump_block`'s `apply_plan`).
- **NEGATIVE (no forge / cold / bundle):** no `run_real_forge`, no `InMemoryChainDb`, no
  `consensus_inputs_path` read on the sync path.
Mirrors the data-flow-resistant, marker-scoped style of `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`.

## 10. Changes Introduced
### Types
- L4a: a closed block-source adaptor (`enum`/`struct` wrapping the `Block`-event selection;
  yields `Option<Vec<u8>>`). Additional fail-closed `NodeLifecycleError` variants for the sync
  path: `SyncPumpError(String)` (maps `PumpError`), `SyncSourceEnded`-style typed completion. A
  candidate `EXIT_NODE_SYNC_FAILED = 43` (distinct from 40/41/42).
- No new BLUE / canonical type.
### State Transitions
- L4b drives the existing `forward_sync_step` → `AdmitPlan` → `pump_block` apply. No new
  authoritative transition.
### Persistence
- First production writes of block bytes + `WalEntry::AdmitBlock` on the `--mode node` path (via
  `pump_block`). No new format.
### Removal / Refactors
- None to `wire_pump.rs`, `forward_sync/*`, `bootstrap.rs`, `replay.rs`, `produce_mode`,
  `admission` (verdict), or the L2/L3 arms. L4 adds the adaptor + driver + gate.

## 11. Replay, Crash, and Epoch Validation
- **Replay (reused, preserved):** `forward_sync_replay_two_runs_byte_identical`
  (`forward_sync/mod.rs`) and `pump_block_stores_bytes_and_wal_then_advances_tip`
  (`forward_sync/pump.rs`) — L4 must keep both green.
- **Replay (new, this slice):** `node_sync_pump_advances_recoverable_tip` (in `node_lifecycle`
  tests): drive L4b over a deterministic in-memory block sequence on a real `PersistentChainDb` +
  `FileWalStore`; assert the tip advanced and the block bytes + WAL `AdmitBlock` are durable.
- **Crash/restart (the L4c proof):** `node_sync_kill_then_warm_start_recovers_same_tip` — run
  L4b, drop handles, reopen, `warm_start_recovery`, assert recovered tip == advanced tip
  byte-identically.
- **Epoch:** single seed epoch (cluster scope); the applied blocks are within the imported epoch
  window. L4 adds no epoch-boundary transition.

## 12. Mechanical Acceptance Criteria
- [ ] `ci/ci_check_node_sync_via_pump.sh` passes: lifecycle sync path calls `pump_block(` and
      advances the tip ONLY via it — no `ade_core_interop::` / `follow(`, no `derive_verdict(` /
      `run_admission(`, no manual tip advance (`ChainDb::put_block(` / direct tip write /
      `AdvanceTip` construction) outside `pump_block`, no `run_real_forge` / `InMemoryChainDb` /
      sync-time `consensus_inputs_path` on that path.
- [ ] `ci/ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` stays green (L4 adds no second
      bootstrap authority; the owner marker stays single).
- [ ] `ci/ci_check_node_mode_closure.sh` + `ci/ci_check_bootstrap_closure.sh` +
      `ci/ci_check_consensus_input_provenance.sh` stay green.
- [ ] L4a test: the adaptor yields the fed block sequence in order and `None` at end; it surfaces
      no verdict/tip-agreement.
- [ ] L4b test `node_sync_pump_advances_recoverable_tip`: blocks durably stored + WAL `AdmitBlock`
      appended durable-before-tip; tip advances to the last applied block.
- [ ] L4c test `node_sync_kill_then_warm_start_recovers_same_tip`: kill→reopen→`warm_start_recovery`
      recovers the same selected tip byte-identically.
- [ ] Fail-closed test: a `pump_block` `PumpError` (e.g. an undecodable/invalid block in the feed)
      halts with a typed `NodeLifecycleError` + non-zero exit; no skip-past, no genesis fallback.
- [ ] `cargo build` + scoped `ade_node`/`ade_runtime` tests + the named gates pass. Full
      `ade_testkit` corpus/oracle lane is NOT an L4 gate (times out ~600s on clean HEAD).

## 13. Failure Modes (all fail-closed, typed)
A `pump_block` reject (`Receive`/`Wal`/`Store`/`Checkpoint`/`TipBeforeDurable`) → typed
`NodeLifecycleError::SyncPumpError` + non-zero exit, before any tip advance for that block. A
source/transport end is a clean feed completion (not a tip authority). No genesis / bundle /
`--consensus-inputs-path` / cold fallback, and no verdict-as-sync, is reachable on the lifecycle
sync path. Any failure that could affect the recoverable tip is fail-fast.

## 14. Hard Prohibitions
**Inherited (cluster):** no forge / `run_real_forge`; no `produce_mode` conversion or change; no
`--consensus-inputs-path` as forge/runtime input; no genesis/bundle/cold fallback; no second
bootstrap/recovery/storage-init authority (CN-NODE-01); no shape-swap; no new BLUE authority/type;
no `HashMap`/clock/float in BLUE.
**Slice-specific (from the L4 brief):** no BA-02 / peer-accept claim; `ade_core_interop::follow`
is NOT validating sync and must not be used as the lifecycle sync path; admission's verdict-only
flow must NOT be treated as recoverable sync; no change to the BLUE admit chokepoint, the GREEN
reducer, `wire_pump.rs`, `bootstrap.rs`, or `replay.rs`; no change to the L2 FirstRun arm. **L4c
MAY extend the RED `warm_start_recovery` driver to populate the replay `block_bytes` map for
`AdmitBlock` entries from `PersistentChainDb` (the `recover_node_state` pattern) — this is the only
authorized L3-arm change and it alters no BLUE replay/bootstrap authority.** No multi-peer fork
choice on the lifecycle path; no registry status flip; no grounding-doc regeneration.

## 15. Explicit Non-Goals
No produce / consume-side fence (L5); no BA-02 evidence (L6); no forge of any kind; no multi-peer
fork choice; no in-flight RollForward block fetching (the wire pump's documented deferral stays);
no live-preprod claim; no registry append; no grounding-doc refresh.

## 16. Completion Checklist
- [ ] §9.0 E1–E4 resolved (single ordered source; verdict-decoupled adaptor; `ForwardSyncState`
      seeded from the bootstrapped/recovered base; L4c proof boundary is a real drop+reopen with a
      genuine tip snapshot).
- [ ] L4a adaptor yields ordered block bytes only; L4b drives `pump_block` (first production
      caller) durable-before-tip over the owner's persistent stores; L4c kill→warm-start recovers
      the same tip byte-identically.
- [ ] Fail-closed on a `pump_block` reject (typed, non-zero, no fallback, no skip-past).
- [ ] New `ci_check_node_sync_via_pump.sh` + the four existing gates green.
- [ ] BLUE chokepoint / reducer / `wire_pump.rs` / `bootstrap.rs` / `replay.rs` / `produce_mode` /
      `admission` (verdict) / L2 / L3 unchanged.
- [ ] `cargo build` + scoped tests + named gates pass (full corpus lane excluded).

## 17. Review Notes
- **Invariant risk considered:** that L4 quietly makes admission's verdict loop or
  `ade_core_interop::follow` the lifecycle sync path. It does not — the only tip-advancing path is
  fetch→`pump_block`, mechanically fenced by `ci_check_node_sync_via_pump.sh`.
- **Assumption challenged (E2/E3):** the wire pump already emits a verdict-free `Block` event and
  `pump_block` already seeds from a `ReceiveState`, so L4 is genuinely a wiring slice — no new BLUE
  authority, the two halves were built to connect.
- **Assumption challenged (E4):** L3's warm-start needs a snapshot at the tip; L4c must ensure
  durable apply (or the harness) leaves a genuine tip checkpoint, else recovery isn't reachable —
  this is exactly the L3-W1 precondition, now created by the durable-apply path it predicted.
- **Follow-up slices implied:** L5 (produce from the recovered tip + recovered consensus inputs —
  DC-CINPUT-02b / CN-CINPUT-03), L6 (BA-02 peer-accept evidence). Multi-peer fork choice and
  in-flight block fetching are post-cluster strengthenings.
