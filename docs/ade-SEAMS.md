# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **54 CI checks** at HEAD (`75f75da`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml` — **206 entries**) for
> rule IDs; reads the Phase 4 cluster plan
> (`docs/active/phase_4_cluster_plan.md`), the closed N-D / N-A / N-B /
> N-E / N-C / N-G / N-H / B1 / B2 / B3 / B4 / B5 cluster docs, the OQ5
> / COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY /
> ENACTMENT-COMMITTEE-WRITEBACK / PROPOSAL-PROCEDURES-DECODE cluster
> docs, and the **just-closed PHASE4-N-I cluster doc + S1..S6 slice
> docs** (`docs/clusters/completed/PHASE4-N-I/cluster.md` +
> `N-I-S{1..6}.md`).
>
> **This is the PHASE4-N-I FULL CLOSE refresh (HEAD `75f75da`).** The
> previous SEAMS (HEAD `efe1fb9`) pinned the PHASE4-N-H full-close
> state. Six N-I slices have landed between that revision and this one
> and close the **rollback-side header→body bridge** (i.e. the
> `DC-CONS-20` rollback-half open obligation that N-H deliberately
> carried forward as the Path A scope edge). The cluster ships in
> **Path A in-memory scope** — persistent snapshot encoding is carved
> out to a follow-on cluster (`DC-CONS-21.open_obligation =
> persistent_ledger_snapshot_encoding_follow_on_cluster`):
>
> 1. **N-I-S1** ships the BLUE rollback-driver substrate at a new
>    sub-tree `ade_ledger::rollback::{traits, error, mod}`: two narrow
>    read-only traits (`SnapshotReader::nearest_le`,
>    `BlockSource::blocks_in_range`) — single-method each, object-safe,
>    BLUE-only — plus closed `MaterializeError` (3 variants) /
>    `CommitRollbackError` (1 variant) sums. **No registry flip at S1
>    alone** — `CN-STORE-07` + `DC-CONS-22` flip at S2 once the
>    materialize driver lands.
> 2. **N-I-S2** ships the BLUE **`materialize_rolled_back_state`**
>    chokepoint in `ade_ledger::rollback::materialize`: the SOLE
>    `pub fn` returning `(LedgerState, PraosChainDepState)` in the
>    rollback module tree (CN-STORE-07 single-authority). Composes one
>    `SnapshotReader::nearest_le` lookup + per-block `block_validity`
>    fold over `BlockSource::blocks_in_range`. Pure, total,
>    deterministic. Registry rules `CN-STORE-07` + `DC-CONS-22` flip
>    to `enforced`. CI gate `ci/ci_check_rollback_materialize_closure.sh`
>    introduced.
> 3. **N-I-S3** ships the BLUE **`commit_rollback`** chokepoint in
>    `ade_ledger::rollback::commit` and extends the existing
>    `ChainDbWrite` trait with a second method `rollback_to_slot(slot)`.
>    Sequence is staged-then-committed (irreversible step first):
>    `chain_write.rollback_to_slot(target.slot)` → `state.ledger =
>    new_ledger` → `state.chain_dep = new_chain_dep` →
>    `state.pending_headers = PendingHeaderCache::new()`. On
>    `chain_write` failure, the receive state is unchanged. All
>    existing `ChainDbWrite` impls (production `ChainDbWriter` in
>    ade_runtime; N-H mock impls) extended.
> 4. **N-I-S4** ships the GREEN rollback runtime adapter layer at a
>    new sub-tree `ade_runtime::rollback::{cadence,
>    in_memory_cache, chaindb_block_source, snapshot_writer}`. Includes
>    `SnapshotCadence` (BLUE-structural single-field
>    `{ every_n_blocks: u32 }`; default 100; operator-tunable runtime
>    cadence is CI-forbidden — `DC-STORE-07`), the pure decision
>    function `should_snapshot_after_block`, `InMemorySnapshotCache`
>    (BTreeMap-backed impl of `SnapshotReader`), and
>    `ChainDbBlockSource<'a, D: ChainDb>` (impl of `BlockSource` over
>    any `ChainDb`). Registry rule `DC-STORE-07` flips to `enforced`.
>    CI gate `ci/ci_check_snapshot_cadence_purity.sh` introduced.
> 5. **N-I-S5** ships the RED snapshot-write hook
>    `ade_runtime::rollback::snapshot_writer::maybe_capture_snapshot`
>    — pure-function hook the caller invokes after each
>    `dispatch_*_inbound` call; on `ReceiveEffect::Admitted` consults
>    the cadence policy and captures the per-peer `(ledger, chain_dep)`
>    into the in-memory cache when due. Decision logic itself is
>    pure (no I/O, no clock); RED-classified by location only.
> 6. **N-I-S6** ships the BLUE-edit wiring: introduces
>    **`RollbackContext`** in `ade_ledger::receive::reducer` —
>    bundles `&dyn SnapshotReader` + `&dyn BlockSource`. The receive
>    reducer's signature gains `Option<&RollbackContext>` — **None
>    preserves N-H legacy behavior** (`Err(RollbackOutOfScope)`);
>    **Some wires the rollback path** through
>    `materialize_rolled_back_state` + `commit_rollback`, returning
>    `Ok(ReceiveEffect::RolledBack { to_slot })`. End-to-end integration
>    test `crates/ade_runtime/tests/receive_rollback_integration.rs`
>    proves: (a) rollback returns `RolledBack` on in-memory snapshot;
>    (b) rollback returns `RollbackTooDeep`/`RollbackOutOfScope` when
>    no snapshot; (c) `rollback_then_continue_admit_equals_straight_line_admit`
>    (snapshot-is-cache proof — DC-CONS-22 end-to-end). Registry rule
>    `DC-CONS-20` flips to `enforced` (`open_obligation` removed);
>    `DC-CONS-20.strengthened_in += PHASE4-N-I`.
>
> **THE KEY FULL-CLOSE DELTAS.** The prior SEAMS revision flagged
> "Full rollback authority" as **the highest-priority remaining
> candidate seam** (closure of `DC-CONS-20` rollback-half;
> `open_obligation = rollback_side_blocked_until_ledger_snapshot_cluster`).
> PHASE4-N-I closes it end to end in Path A in-memory scope. One §1
> candidate row flips from "next-cluster seam (HIGHEST PRIORITY)" to
> "wired & closed":
>
> - **Rollback authority — `ReceiveEvent::RollBackward` returning
>   `Ok(ReceiveEffect::RolledBack { to_slot })` instead of
>   `Err(RollbackOutOfScope)`** → wired via `RollbackContext`
>   threaded into `receive_apply`, materializing via
>   `materialize_rolled_back_state` + committing via `commit_rollback`,
>   defended by `ci_check_rollback_materialize_closure.sh` +
>   `ci_check_snapshot_cadence_purity.sh`.
>
> Counts at this refresh: **+2 CI scripts** (52 → 54:
> `ci_check_rollback_materialize_closure.sh`,
> `ci_check_snapshot_cadence_purity.sh`); **+4 registry rules**
> introduced (`DC-CONS-21` `declared`, `DC-CONS-22` `enforced`,
> `CN-STORE-07` `enforced`, `DC-STORE-07` `enforced`); **1 carried
> rule strengthened + closed** (`DC-CONS-20` gains
> `strengthened_in += PHASE4-N-I` and `open_obligation` removed —
> status flips from `declared` to `enforced`); **+4 new BLUE
> submodules** (`ade_ledger::rollback::{traits, error, materialize,
> commit}` under a new `ade_ledger::rollback` barrel); **+4 new
> GREEN/RED submodules under a new `ade_runtime::rollback` barrel**
> (`cadence`, `in_memory_cache`, `chaindb_block_source` GREEN;
> `snapshot_writer` RED); **+1 narrow trait extension** (`ChainDbWrite`
> gains `rollback_to_slot` second method); **+1 BLUE struct seam**
> (`RollbackContext` in `ade_ledger::receive::reducer`); **+1
> reducer signature evolution** (`receive_apply` gains
> `Option<&RollbackContext>` parameter; `None` preserves N-H legacy
> behavior — additive at the call site). **0 new operator-action
> probe binaries** at this HEAD — rollback is wholly internal-
> authority and has no Tier-1 wire-format counterpart that needs a
> live cross-impl probe. Total invariant registry: **206 entries**
> (202 → 206). `DC-CONS-21` (persistent snapshot encoding) is the
> **new explicit carried-forward open obligation**, surfaced as the
> next planner's highest-priority candidate seam.

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-I is fully closed at this HEAD.** The receive-side
rollback authority — i.e. the path by which a peer-originated
`RollBackward(target_point)` event causes `(LedgerState,
PraosChainDepState, ChainDb tip, pending headers)` to be atomically
rolled back to `target_point.slot` via snapshot lookup +
replay-forward over `block_validity` — is wired and CI-defended end
to end. Scope is **Path A in-memory only**: `SnapshotReader` has one
production impl (`InMemorySnapshotCache`); persistent on-disk
encoding (`DC-CONS-21`) is explicitly carved out to a follow-on
cluster (`open_obligation =
persistent_ledger_snapshot_encoding_follow_on_cluster`). This is the
same Path A discipline N-H used for rollback-side itself, applied
one level deeper.

**PHASE4-N-H remains fully closed** (carried; `DC-CONS-20`
`open_obligation` now removed). **PHASE4-N-G remains fully closed**
(carried). **PHASE4-N-C remains fully closed** (carried).
**PHASE4-N-E remains fully closed** (carried).
**PROPOSAL-PROCEDURES-DECODE remains fully closed** (carried).
**PHASE4-B3..B5, OQ5 / COMMITTEE / DREP /
ENACTMENT-COMMITTEE-WRITEBACK** all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there remain **eight** fully-wired *external*
> ingress surfaces (block bytes, Plutus script bytes, snapshot bytes,
> Ouroboros mux frames, genesis JSON bundles, chain-selector stream
> inputs, the N-E wire-level mempool ingress, and the N-H receive-side
> N2N peer ingress — receive admission half). PHASE4-N-I adds **no new
> external ingress surface**: rollback authority is wholly internal —
> the `RollBackward` peer-originated signal was already in the closed
> `ReceiveEvent` taxonomy (N-H-S1, CN-PROTO-07); what changes is the
> reducer's response to it. All other internal composition roots are
> unchanged (`block_validity` / `tx_validity` / `mempool_ingress` /
> `forge_block` / `self_accept` / `served_chain_admit` /
> `admit_via_block_validity`).

### Surface: Receive-side N2N peer ingress (carried from N-H; **rollback half newly closed in N-I**)

```
Surface: A peer-originated chain-sync ForkChoiceSignal
         (RollForward { header_bytes, tip }
         | RollBackward { target_point, tip }
         | Intersected | NoIntersection)
         OR a peer-originated block-fetch BatchDeliveryEvent
         (BatchStarted | BlockDelivered { block_bytes }
         | NoBlocks | BatchCompleted)
         delivered by a real cardano-node peer over N2N mux
Reduces to: ReceiveEffect — closed 4-variant sum
            { Admitted { slot, hash } | Cached { slot, hash }
            | RolledBack { to_slot }   (** NEWLY REACHABLE in N-I via
                                          RollbackContext-some path **)
            | NoOp { reason: HeaderAlreadyCached } }
            — OR ReceiveError — closed 4-variant sum
            { HeaderBodyMismatch | Validity(BlockValidityError)
            | RollbackOutOfScope { target_point }
              (** still reachable when materialize returns
                 RollbackTooDeep or when RollbackContext is None **)
            | ChainDb(ChainWriteError) }
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED transport (ade_network::mux::transport) decodes mux frame
  2. BLUE chain-sync / block-fetch codec (N-A) — decode_*_message
  3. RED dispatcher
     (ade_runtime::receive::orchestrator::dispatch_*_inbound)
     wraps PerPeerReceiveState
  4. RED-internal translation: peer-agency message →
     ForkChoiceSignal / BatchDeliveryEvent (N-A signal/event types)
  5. GREEN lift (ade_runtime::receive::events_to_state) — pure
     pass-through translator into ReceiveEvent.
  6. BLUE reducer — receive_apply(state, event, chain_write,
     era_schedule, ledger_view, rollback_ctx: Option<&RollbackContext>):
       - RollForward: pending_header_cache.insert((slot,hash) ->
         header_bytes); mutates only state.pending_headers (I-6).
       - BlockDelivered: header cross-check → admit_via_block_validity
         → chain_write.write_admitted → commit; staged-then-committed.
       - RollBackward:
           * None (rollback_ctx not supplied) →
             Err(ReceiveError::RollbackOutOfScope) (legacy N-H path,
             retained for callers that haven't wired rollback yet).
           * Some(ctx) → roll_backward(state, target_point, chain_write,
             era_schedule, ledger_view, ctx) — calls
             materialize_rolled_back_state(target, ctx.snapshot_reader,
             ctx.block_source, era_schedule, ledger_view) → on success
             commit_rollback(state, target, new_ledger, new_chain_dep,
             chain_write) → returns Ok(ReceiveEffect::RolledBack {
             to_slot: target.slot }). materialize errors map: RollbackTooDeep
             → ReceiveError::RollbackOutOfScope (preserved shape;
             materialize context logged at orchestrator); ReplayFailedAt
             → ReceiveError::Validity; EraNotSupported → RollbackOutOfScope
             (Path A scope: pre-Conway out of scope).
  7. GREEN ChainDb write (ade_runtime::receive::in_memory_chain_write
     — ChainDbWriter<'a, D>): admit-side decodes once and calls
     ChainDb::put_block; rollback-side calls ChainDb::rollback_to_slot(slot)
     (NEW N-I-S3 trait method).
  8. RED snapshot-write hook (ade_runtime::rollback::snapshot_writer
     — maybe_capture_snapshot, NEW N-I-S5): on each ReceiveEffect::Admitted
     consults cadence policy + captures (ledger, chain_dep) into
     InMemorySnapshotCache when due. Pure decision + in-memory write
     — no I/O at this layer.
Cross-surface state sharing: per-peer state is fully independent
  (N-H invariant carried); the rollback addition introduces a per-
  peer InMemorySnapshotCache that grows monotonically during the
  session (no eviction at N-I scope per OQ-5). The ONLY cross-peer
  shared state remains the shared ChainDb — now also rolled back via
  ChainDb::rollback_to_slot when the RolledBack effect fires.
```

**Rule (carried + extended).** `receive_apply` (with
`receive_apply_sequence` as its deterministic driver) is still the
**single receive-side composition root** into `LedgerState` +
`PraosChainDepState` + `ChainDb` + `pending_headers`. The
`ReceiveEvent` taxonomy is unchanged at three variants
(`RollForward`, `RollBackward`, `BlockDelivered`); CN-PROTO-07
unchanged. **N-I introduces one new BLUE call seam**
(`RollbackContext`) and **one new BLUE composition chokepoint**
(`materialize_rolled_back_state`) — composed into the `RollBackward`
arm via `commit_rollback`. The chokepoint **never moves**:
`materialize_rolled_back_state` is the SOLE `pub fn` returning
`(LedgerState, PraosChainDepState)` in the rollback module tree
(CI-defended via single-authority grep; mirrors `block_validity`'s
single-authority discipline for the admit side). **New work** that
adds a rollback feature attaches by extending the closed
`MaterializeError` / `CommitRollbackError` arms, by introducing a
second `SnapshotReader` impl (e.g. the future persistent decoder —
deliberate registry-tracked addition), or by extending the
`BlockSource` impl set — **not** by exposing a parallel materializer,
not by bypassing `block_validity` in the replay-forward fold, not by
moving the chokepoint outside `ade_ledger::rollback::materialize`.

### Surface: Producer-side chain-sync server-role ingress (wired in N-G; carried unchanged)

Carried. No N-I interaction (server-role is send-side; rollback is
receive-side).

### Surface: Producer-side block-fetch server-role ingress (wired in N-G; carried unchanged)

Carried.

### Surface: Forge-block transition (carried unchanged from N-C)

Carried.

### Surface: Self-accept broadcast gate (carried unchanged from N-C)

Carried. **N-I note:** `AcceptedBlock` (producer-side broadcast/serve
gate) and `AdmittedBlock` (receive-side admit gate) remain the
matched pair; rollback authority does not introduce a third
admission/broadcast token. The rollback path acts on already-admitted
state — it rolls a sequence of past `AdmittedBlock` admissions
backward, not bypass the gate.

### Surface: Scheduler input ingress (carried unchanged from N-C)

Carried.

### Surface: Mempool ingress (Tier-1 wire-level — wired in N-E; unchanged)

Carried.

### Surface: Conway tx-body `proposal_procedures` sub-grammar (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Surface: Single-tx validity (composition root — wired in B2; unchanged)

Carried.

### Surface: Mempool admission (Tier-1 gate — wired in B2; unchanged)

Carried.

### Surface: Full block validity (composition root — wired in B1; **N-I usage**: replay-forward fold)

Carried. **N-I usage:** the new BLUE chokepoint
`materialize_rolled_back_state` composes `block_validity` as the
per-block step in the replay-forward fold. `block_validity` is
unchanged; it gains a **fifth public consumer** beyond the
validator's direct callers, `self_accept` (N-C),
`served_chain_admit` (N-G), and `admit_via_block_validity` (N-H).
Single admission authority unchanged.

### Surface: Block bytes, Plutus script bytes, Snapshot bytes, Consensus-input extraction, Ouroboros mux frames, Genesis JSON bundles, Chain-selector stream inputs (carried)

All seven external ingress surfaces are unchanged at this HEAD.

### Candidates — surfaces not yet wired (persistent snapshot encoding, multi-peer fork choice, N2C surfaces, B+ residuals, PP open obligations)

The following surfaces are named in the Phase 4 plan / B+ planning /
the N-I Path A scope edge / the PP open-obligation set but have no
source today. They are listed so future slice docs can attach without
reinventing the reduction step. **Each is a candidate seam pending
confirmation at cluster entry.**

- **N-I-S1..S6 WIRED AND CLOSED the prior revision's "Full rollback
  authority" candidate** — removed (now `materialize_rolled_back_state`
  + `commit_rollback` + `RollbackContext` + `InMemorySnapshotCache` +
  `ChainDbBlockSource` + `should_snapshot_after_block` +
  `maybe_capture_snapshot`).
- **NEW HIGHEST-PRIORITY CANDIDATE (flagged by N-I close —
  `DC-CONS-21` `open_obligation =
  persistent_ledger_snapshot_encoding_follow_on_cluster`): persistent
  ledger snapshot encoding.** N-I ships an in-memory `SnapshotReader`
  only (`InMemorySnapshotCache`); across restarts, snapshots are
  lost. The follow-on cluster closes that gap by shipping
  (a) the canonical `encode_ledger_state` + `decode_ledger_state` pair
  (a ~1500-2000 LoC BLUE encoder mirroring `ade_ledger::fingerprint`'s
  field-walk structure — too large for a single N-I slice, carved
  out as a dedicated cluster), and (b) wiring `SnapshotReader` to
  `SnapshotStore` bytes (a persistent impl alongside the in-memory
  `InMemorySnapshotCache`). The cluster does not change any BLUE
  rollback chokepoint — `materialize_rolled_back_state` already takes
  `&dyn SnapshotReader`, so a persistent impl drops in. **This is
  the highest-priority candidate seam for the next cluster planner**
  — surface it; do not invent invariants for it here.
- **CANDIDATE (carried from N-H — OQ-4 lock): multi-peer fork choice
  (Praos longest-chain selection across competing peers).** With both
  receive-side admission (N-H) and receive-side rollback (N-I) now
  closed, the rollback prerequisite for fork-choice commits is in
  place. Today the receive bridge is still single-source follow: each
  peer is applied independently against a shared ChainDb; whichever
  block arrives first under a `(slot, hash)` key wins by ChainDb's
  byte-identity idempotency. Praos longest-chain across competing
  forks requires a fork-choice rule consumer of
  `(PerPeerReceiveState[])` that materializes a candidate set, runs
  `chain_selector::select_best_chain`, and either commits the chosen
  fork (now possible — re-using N-I's `RollbackContext` to roll back
  losing forks) or rejects. Surface for the next planner.
- **CANDIDATE (carried from N-H): N2C local-chain-sync receive
  surface.** Sibling of `producer_chain_sync_serve` (N-G); operator
  clients (`db-sync`, wallets, explorers) consuming a chain-sync
  stream from Ade. Unchanged at N-I.
- **CANDIDATE (NEW — flagged by N-I close): snapshot eviction
  policy.** OQ-5 declared eviction explicitly out-of-scope for the
  N-I cluster (`InMemorySnapshotCache` grows monotonically until
  process restart). Eviction is a follow-on operational concern —
  a cluster-level operational seam, Tier-5; bounded-ring policy
  modeled on `chain_selector::OrchestratorState::recent_snapshots`
  (≤ 2160) is the obvious starting shape. Surface for the next
  planner.
- **CE-N-H-6 live-evidence — still
  `blocked_until_operator_peer_available`** (carried).
- **CE-N-G-8 / CE-N-C-8 live-evidence — still
  `blocked_until_operator_*_available`** (carried).
- **PROPOSAL-PROCEDURES-DECODE remains closed** (carried). The four
  PP open obligations remain separable candidate seams (carried).
- **PHASE4-N-E remains closed** (carried).

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-I** *(FULLY CLOSED at this HEAD — mechanical close; Path A in-memory scope; persistent encoding deferred to follow-on per DC-CONS-21)* | **Receive-side rollback authority: `ReceiveEvent::RollBackward` → `Ok(ReceiveEffect::RolledBack { to_slot })` via snapshot lookup + replay-forward** | `(target_point, &dyn SnapshotReader, &dyn BlockSource, &EraSchedule, &dyn LedgerView)` → `(LedgerState, PraosChainDepState)` then atomic 4-step commit | **DONE:** `ade_ledger::rollback::{traits::{SnapshotReader, BlockSource}, error::{MaterializeError, CommitRollbackError}, materialize::{materialize_rolled_back_state, TargetPoint}, commit::commit_rollback}` (BLUE); `ade_runtime::rollback::{cadence::{SnapshotCadence, should_snapshot_after_block}, in_memory_cache::InMemorySnapshotCache, chaindb_block_source::ChainDbBlockSource, snapshot_writer::maybe_capture_snapshot}` (GREEN + RED snapshot_writer); `ade_ledger::receive::reducer::RollbackContext` (BLUE struct) + `receive_apply` signature evolution; `ChainDbWrite::rollback_to_slot` (BLUE trait method extension). CI gates `ci_check_rollback_materialize_closure.sh`, `ci_check_snapshot_cadence_purity.sh`. Registry rules `CN-STORE-07`, `DC-CONS-22`, `DC-STORE-07` (`enforced`); `DC-CONS-20` flipped from `declared` to `enforced` with `strengthened_in += PHASE4-N-I` and `open_obligation` removed. Tests: named tests across S1..S6 plus end-to-end integration test `crates/ade_runtime/tests/receive_rollback_integration.rs`. | **wired & closed in PHASE4-N-I (mechanical half — wholly internal authority, no Tier-1 wire-format counterpart that requires live cross-impl probe); persistent encoding deliberately deferred to follow-on cluster per DC-CONS-21** |
| **NEW HIGHEST-PRIORITY CANDIDATE — Persistent ledger snapshot encoding** *(flagged by N-I close — `DC-CONS-21.open_obligation = persistent_ledger_snapshot_encoding_follow_on_cluster`)* | **`InMemorySnapshotCache` sibling that round-trips through bytes on `SnapshotStore`; restart-safe rollback authority** | A persistent `SnapshotReader` impl whose `nearest_le` decodes from bytes; backed by the canonical `encode_ledger_state` + `decode_ledger_state` pair (~1500-2000 LoC BLUE encoder mirroring `ade_ledger::fingerprint`'s field-walk structure) | New BLUE module `ade_ledger::snapshot::{encode, decode}` plus a new GREEN `ade_runtime::rollback::persistent_snapshot_store` impl of `SnapshotReader`. No change to `materialize_rolled_back_state` (it already takes `&dyn SnapshotReader`). The encoder writes [closed version tag][canonical-CBOR LedgerState bytes][fingerprint] so decode-side can verify integrity (DC-CONS-21 statement). | **candidate (next-cluster seam — HIGHEST PRIORITY for next planner; surface; do not invent invariants here)** |
| **CANDIDATE — Multi-peer fork choice (Praos longest-chain across competing peers)** *(carried from N-H — OQ-4 lock; now unblocked by N-I's rollback closure)* | **Per-peer `ReceiveState[]` resolution to a single canonical chain** | A fork-choice consumer of `(PerPeerReceiveState[])` returning the canonical-chain `BlockHash` | `ade_runtime::consensus::chain_selector::select_best_chain` (existing GREEN) consumed by a new RED multi-peer coordinator. Commits to the chosen fork via N-I's `RollbackContext` to roll back losing forks. | **candidate (next-cluster seam; surface; sequenced before or after persistent-encoding cluster per planner discretion — both gating fork-choice from different angles)** |
| **CANDIDATE — N2C local-chain-sync receive surface** *(carried from N-H)* | **Operator-side N2C clients consume Ade's chain via `LocalChainSyncMessage` requests** | per-client `(PerClientLocalChainSyncState, shared ChainDb)`; closed `ServerReply<LocalChainSyncMessage>` wrapper | Sibling of `producer_chain_sync_serve` (N-G) over the local-chain-sync N2C codec. Reuses `ServedHeaderLookup`-style trait, this time over the persisted ChainDb tip. | **candidate (next-cluster seam; surface)** |
| **NEW CANDIDATE — Snapshot eviction policy** *(flagged by N-I close — OQ-5 declared non-goal)* | **Bounded `InMemorySnapshotCache` size (or persistent-store eviction)** | A bounded ring or stability-window policy modeled on `chain_selector::OrchestratorState::recent_snapshots` (≤ 2160) | Extension inside `ade_runtime::rollback::in_memory_cache::InMemorySnapshotCache` — add an `evict_older_than(slot)` method gated by a `SnapshotEvictionPolicy` (Tier-5 operator-tunable; must remain replay-deterministic per OQ-5's deferred constraint). | **candidate (next-cluster seam — operational/Tier-5; surface)** |
| **CE-N-H-6 (cross-cluster obligation carried from N-H; operator-action live evidence)** | **Live N2N follow-mode admission** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-G-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade serving)** | Carried. | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-C-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade forging)** | Carried. | Carried. | **carried (`blocked_until_operator_stake_available`)** |
| **N-C+ (declared non-goal in N-C cluster doc; OQ-4 lock)** | **TPraos producer (Shelley..Alonzo full-block production)** | Carried. | Carried. | candidate (declared non-goal) |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | Carried. | Carried. | **deferred cross-cluster obligation (NOT an open seam in N-E)** |
| **PP OQ-1..OQ-4 (separable seams)** | various | Carried. | Carried. | candidate (carried) |
| B+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO | Carried. | Carried. | candidate |
| B+ (Conway body witness depth) | Conway block-body vkey-witness closure | Carried. | Carried. | candidate (B2-carried) |
| B+ (pre-Conway tx) | Pre-Conway single-tx validity | Carried. | Carried. | candidate |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | Carried. | Carried. | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum | Single dispatch fn over opaque-bytes payloads | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn over opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver | `StreamInput` translated from `ChainSyncMessage` + `BlockFetchMessage` | Composition layer in `ade_core_interop` | candidate |

### Operator-action evidence (live-wire artifacts — not BLUE seams)

The Ade workspace closes Tier-1 wire-level seams in two halves: a
mechanical / GREEN half (code + harness + CI gates that the workspace
itself can certify on every push) and a **live-wire operator-action
half** (a real peer / client at the other end of a real socket
producing bytes Ade has never seen).

**At this HEAD two live-evidence logs remain committed**, three
cross-cluster obligations remain `blocked_until_operator_*_available`,
and one cross-cluster obligation is carried from N-E. **N-I added
no new operator-action obligation** — rollback authority is wholly
internal (no Tier-1 wire-format counterpart that requires a real
peer to certify).

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried from N-B close) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried from N-E close) | Outbound-client probe against a real preprod N2N relay | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` in the future node-binary cluster | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS | RED operator action (deferred) |
| `docs/clusters/completed/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | (pending) `CE-N-C-LIVE_<date>.log` | **`blocked_until_operator_stake_available`** (carried) | Cardano-node accepts an Ade-forged block as the next chain head | RED operator action |
| `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` | (pending) `CE-N-G-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | A real cardano-node peer issuing `RequestRange` accepts Ade-served bytes | RED operator action |
| `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` | (pending) `docs/clusters/completed/PHASE4-N-H/CE-N-H-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | Ade follower fed RollForward + BlockDelivered from a real cardano-node peer produces a matching ChainDb tip | RED operator action |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
At this HEAD there are still **five** such binaries (no N-I addition):

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 (live chain-sync follow-mode tip agreement) | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 (live N2N tx-submission2 outbound-client probe) | captured |
| `live_block_production_session` (PHASE4-N-C S7) | N-C S7 | CE-N-C-8 (live N2N block-fetch acceptance by cardano-node — producer-side forge) | blocked_until_operator_stake_available |
| `live_block_fetch_session` (PHASE4-N-G S7) | N-G S7 | CE-N-G-8 (live N2N server-role block-fetch served by Ade to cardano-node) | blocked_until_operator_peer_available |
| `live_block_follow_session` (PHASE4-N-H S6) | N-H S6 | CE-N-H-6 (live N2N receive-side follow-mode admission) | blocked_until_operator_peer_available |

**Pattern carried.** Hermetic default + `--connect <peer>` live pass.
**N-I has no new entry in this family** — rollback authority is
exercised by the end-to-end integration test
`crates/ade_runtime/tests/receive_rollback_integration.rs` and the
existing receive-side live binary (`live_block_follow_session`)
covers the surrounding admission path; if a future cluster wires
live rollback evidence (e.g. a peer issuing a real `RollBackward`
across a captured window), it would extend the same probe-binary
family.

**These are evidence-log patterns, not BLUE seams.**

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are
**CE-N-C-8** (live cardano-node forge acceptance), **CE-N-G-8** (live
cardano-node block-fetch acceptance — Ade-serving counterpart),
**CE-N-H-6** (live cardano-node follow-mode admission — Ade-receiving
counterpart), **persistent ledger snapshot encoding** (closure of the
`DC-CONS-21` `open_obligation` — restart-safe rollback),
**CE-NODE-N2C-LTX** (the deferred live N2C UDS server + N2N bulk-tx
inbound listener), and the four **PROPOSAL-PROCEDURES-DECODE open
obligations**.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **eighteen** authoritative domains. **PHASE4-N-I added one
new domain — receive-side rollback authority** — a new BLUE
composition root (`materialize_rolled_back_state`) producing
`(LedgerState, PraosChainDepState)` via repeated composition of the
existing B1 chokepoint (`block_validity`), plus a new BLUE atomic
commit helper (`commit_rollback`), driven through the receive
reducer's new `Option<&RollbackContext>` parameter and the
extended `ChainDbWrite::rollback_to_slot` trait method. Prior cluster
narratives are preserved unchanged below.

### Receive-side rollback authority (NEW in PHASE4-N-I)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **BLUE narrow read-only traits (S1)** | `ade_ledger::rollback::traits::{SnapshotReader, BlockSource}` | BLUE | Two single-method traits. `SnapshotReader::nearest_le(target_slot) -> Option<(SlotNo, LedgerState, PraosChainDepState)>` returns the largest snapshot ≤ target. `BlockSource::blocks_in_range(from_exclusive, to_inclusive) -> Vec<(SlotNo, Vec<u8>)>` yields ordered block bytes for replay-forward. Both object-safe. Production impls live in `ade_runtime::rollback` (S4 GREEN). **Closed seams** — one production impl each at this cluster (in-memory only). |
| **BLUE closed error sums (S1)** | `ade_ledger::rollback::error::{MaterializeError, CommitRollbackError}` | BLUE | Closed sums. `MaterializeError` has 3 variants — `RollbackTooDeep { target_slot, oldest_snapshot: Option<SlotNo> }`, `ReplayFailedAt { slot, error: BlockValidityError }`, `EraNotSupported { era, slot }`. `CommitRollbackError` has 1 variant — `ChainDb(ChainWriteError)`. No `#[non_exhaustive]`; no `String`. |
| **BLUE single materialize authority (S2)** | `ade_ledger::rollback::materialize::{materialize_rolled_back_state, TargetPoint}` | BLUE | **The SOLE `pub fn` returning `(LedgerState, PraosChainDepState)` in the rollback module tree** (CN-STORE-07 single-authority discipline; CI-defended by `ci_check_rollback_materialize_closure.sh` via single-authority grep). Pure, total, deterministic. Composes one `SnapshotReader::nearest_le` lookup + per-block `block_validity` fold over `BlockSource::blocks_in_range`. Returns `Ok` on `BlockValidityVerdict::Valid`; structured error on every failure (no panic). Era support is closed to Babbage + Conway at this cluster (pre-Conway → `MaterializeError::EraNotSupported`). |
| **BLUE atomic commit chokepoint (S3)** | `ade_ledger::rollback::commit::commit_rollback` | BLUE | **Closed seam** — atomic 4-step state replacement. Sequence (irreversible-first staged commit): (1) `chain_write.rollback_to_slot(target.slot)` — irreversible; failure → `Err`, state unchanged; (2) `state.ledger = new_ledger` (infallible); (3) `state.chain_dep = new_chain_dep` (infallible); (4) `state.pending_headers = PendingHeaderCache::new()` (post-rollback cached headers are stale). Pure logic + one trait call. |
| **BLUE narrow trait extension (S3)** | `ade_ledger::receive::chain_write::ChainDbWrite::rollback_to_slot` | BLUE | **NEW second method on the existing closed `ChainDbWrite` trait** (N-H-S1). Signature: `fn rollback_to_slot(&mut self, slot: SlotNo) -> Result<(), ChainWriteError>`. Rolls the underlying chain store back to `slot`, discarding all blocks at slots strictly greater than `slot`. After this returns `Ok`, no read operation observes such a block. Rolling back beyond the empty tip is `Ok(())` per the underlying ChainDb contract. All existing impls extended (production `ChainDbWriter` in ade_runtime; mock impls in tests). |
| **BLUE struct seam — receive-reducer rollback context (S6)** | `ade_ledger::receive::reducer::RollbackContext<'a>` | BLUE | Closed struct bundling `&'a dyn SnapshotReader` + `&'a dyn BlockSource`. **The single way the reducer obtains rollback authority.** New work that adds a rollback source (e.g. persistent snapshot store) attaches by implementing `SnapshotReader` and passing the runtime instance via `RollbackContext` — it does not extend the struct's shape. |
| **BLUE receive reducer (S6 BLUE-edit)** | `ade_ledger::receive::reducer::receive_apply` | BLUE | Signature evolution: gains `rollback_ctx: Option<&RollbackContext>` as a sixth parameter. **None preserves N-H legacy behavior** (`Err(ReceiveError::RollbackOutOfScope)`); **Some wires the rollback path** through `roll_backward` → `materialize_rolled_back_state` → `commit_rollback` → `Ok(ReceiveEffect::RolledBack { to_slot })`. The `RollForward` arm and `BlockDelivered` arm are unchanged from N-H — only the `RollBackward` arm gains a new behavior under `Some(ctx)`. Staged-then-committed shape preserved: any materialize or commit failure leaves state AND chain_write unchanged. Error mapping in `map_materialize_err`: `MaterializeError::RollbackTooDeep` → `ReceiveError::RollbackOutOfScope` (preserved shape — materialize context logged at orchestrator); `ReplayFailedAt { error }` → `ReceiveError::Validity(error)`; `EraNotSupported` → `ReceiveError::RollbackOutOfScope` (Path A scope). |
| **GREEN snapshot cadence (S4)** | `ade_runtime::rollback::cadence::{SnapshotCadence, should_snapshot_after_block}` | GREEN | `SnapshotCadence` is BLUE-structural with **a single field** (`every_n_blocks: u32`; default 100; CI-defended to exactly one field — operator-tunable runtime cadence is explicitly out of scope per DC-STORE-07 to avoid weakening replay-equivalence). `should_snapshot_after_block(slot, block_no, cadence, last_snapshot) -> bool` is a pure decision function — same inputs → same decision. Defended by `ci_check_snapshot_cadence_purity.sh`. |
| **GREEN in-memory snapshot cache (S4 — single production `SnapshotReader` impl)** | `ade_runtime::rollback::in_memory_cache::InMemorySnapshotCache` | GREEN | `BTreeMap<SlotNo, (LedgerState, PraosChainDepState)>`-backed impl of `SnapshotReader::nearest_le` (canonical iteration; no HashMap). `admit(slot, ledger, chain_dep)` insert/overwrite; `oldest()`/`most_recent()`/`slots()` read-only inspection; `capture_from(slot, &ReceiveState)` convenience. **Path A in-memory scope:** snapshots are lost on restart; the persistent variant is the follow-on cluster's deliverable (DC-CONS-21). |
| **GREEN block source adapter (S4 — single production `BlockSource` impl)** | `ade_runtime::rollback::chaindb_block_source::ChainDbBlockSource<'a, D: ChainDb>` | GREEN | Wraps any `ChainDb` impl and exposes the BLUE `BlockSource` trait. `blocks_in_range(from_exclusive, to_inclusive)` calls `ChainDb::iter_from_slot(from_exclusive + 1)` (saturating) and collects until the iterator yields a slot > `to_inclusive`. Returns `(slot, bytes)` byte-identical to the underlying store (DC-CONS-22 prerequisite). |
| **RED snapshot-write hook (S5)** | `ade_runtime::rollback::snapshot_writer::maybe_capture_snapshot` | RED | Pure-function hook the caller invokes after each `dispatch_*_inbound` call. Inspects the `ReceiveEffect`; on `Admitted` consults the cadence policy and captures `(state.ledger, state.chain_dep)` into the `InMemorySnapshotCache` when due. **Classified RED by location only** (composes RED dispatch outcomes with the GREEN cadence + cache); the decision logic itself is pure (no I/O, no clock). Returns `true` if a snapshot was captured. |
| **GREEN end-to-end integration test (S6)** | `crates/ade_runtime/tests/receive_rollback_integration.rs` | GREEN | End-to-end test wiring `RollbackContext` + `InMemorySnapshotCache` + `ChainDbBlockSource` through the full receive pipeline. Asserts: (a) rollback returns `RolledBack` on in-memory snapshot; (b) rollback returns `RollbackOutOfScope` when no snapshot (preserves error shape via `MaterializeError::RollbackTooDeep` mapping); (c) `rollback_then_continue_admit_equals_straight_line_admit` (snapshot-is-cache proof — DC-CONS-22 end-to-end); (d) `rollback_branch_state_unchanged_on_materialize_failure`. |
| **CI gates (S2, S4)** | `ci/ci_check_{rollback_materialize_closure, snapshot_cadence_purity}.sh` | CI | 2 mechanical gates defending the rollback authority surface. Total CI count: 52 → 54. |

**Rule.** This domain has **one BLUE materialize chokepoint**
(`materialize_rolled_back_state` — CN-STORE-07 single-authority; the
SOLE `pub fn` in the rollback module tree returning the rolled-back
state tuple), **two BLUE narrow read-only traits** (`SnapshotReader`
+ `BlockSource` — single-method each, object-safe; production impls
live in RED/GREEN crates), **one BLUE atomic commit chokepoint**
(`commit_rollback` — irreversible-first staged commit), **one BLUE
trait extension** (`ChainDbWrite::rollback_to_slot` — second method
on N-H's closed trait), **one BLUE struct seam** (`RollbackContext`
— the receive reducer's only rollback-authority entry point), **one
BLUE receive-reducer signature evolution** (`Option<&RollbackContext>`
— None preserves N-H legacy behavior; Some wires rollback), **two
closed BLUE error sums** (`MaterializeError` + `CommitRollbackError`),
**three GREEN adapters** (`SnapshotCadence` + `should_snapshot_after_block`
+ `InMemorySnapshotCache` + `ChainDbBlockSource`), **one RED hook**
(`maybe_capture_snapshot`), and **one end-to-end integration test**
(`receive_rollback_integration`).

**THE KEY SEAMS:**

1. **`materialize_rolled_back_state` is the SOLE `pub fn` returning
   `(LedgerState, PraosChainDepState)`** in the entire rollback
   module tree (CN-STORE-07). Mirrors `block_validity`'s single-
   authority discipline for the admit side. CI-defended by
   `ci_check_rollback_materialize_closure.sh` (single-authority
   grep). The driver composes the same `block_validity` authority
   the receive admit branch uses — no parallel admission path; no
   parallel rolled-back-state computation path.
2. **`SnapshotReader` + `BlockSource` are CLOSED narrow trait seams.**
   Single method each. Read-only. Object-safe. **One production impl
   each at this cluster** — `InMemorySnapshotCache` and
   `ChainDbBlockSource`. New impls would be a deliberate registry-
   tracked addition (the persistent decoder for `SnapshotReader` is
   the named follow-on cluster's deliverable — DC-CONS-21).
3. **`commit_rollback` is the CLOSED atomic chokepoint.** Single
   trait call (`chain_write.rollback_to_slot`) is the irreversible
   step; all other commits are infallible field replacements. On
   `chain_write` failure, the receive state is unchanged. End-to-end
   atomicity proven by `commit_rollback_chain_write_failure_leaves_state_unchanged`.
4. **`RollbackContext` is the SINGLE receive-side rollback entry
   point.** The reducer's `RollBackward` arm has no other way to
   obtain rollback authority; the struct bundles
   `&dyn SnapshotReader` + `&dyn BlockSource` and is the only
   parameter shape the arm consumes. New rollback sources (persistent
   snapshot store, cross-peer snapshot store) attach by implementing
   `SnapshotReader` and passing through `RollbackContext`.
5. **`ChainDbWrite::rollback_to_slot` is a CLOSED trait extension.**
   Second method on N-H's closed seam. All existing impls extended
   (production `ChainDbWriter` in ade_runtime; mock impls in tests).
   No new impl was added at this cluster — the existing in-memory
   ChainDb path covers Path A scope.
6. **`receive_apply` signature evolution is ADDITIVE at the call
   site.** `Option<&RollbackContext>` — `None` preserves N-H legacy
   behavior (the `RollbackOutOfScope` error is retained as a
   variant); `Some` wires the rollback path. Callers that haven't
   wired rollback yet keep compiling unchanged; callers that wire
   the new path get the new behavior.
7. **`SnapshotCadence` is BLUE-structural with one field.**
   CI-defended (`ci_check_snapshot_cadence_purity.sh` enforces
   exactly one field — `every_n_blocks: u32`). Operator-tunable
   runtime cadence is **explicitly out of scope** (DC-STORE-07) to
   preserve replay-equivalence; if a future cluster ratifies an
   operator-tunable knob, it must be represented as anchored,
   replay-derivable runtime data — not a runtime-mutable parameter.
8. **Snapshot eviction is NOT a concern of this cluster** (OQ-5).
   `InMemorySnapshotCache` grows monotonically; eviction is a
   follow-on operational concern. The cache's API surface includes
   `oldest()`/`most_recent()`/`slots()` for inspection but no
   `evict_*` method.

**New work** that adds a rollback feature attaches by:
- Adding a new `SnapshotReader` impl (e.g. the persistent decoder
  for the follow-on cluster — DC-CONS-21).
- Adding a new `BlockSource` impl (none anticipated — `ChainDbBlockSource`
  covers all foreseeable cases since rollback replay always sources
  from the canonical ChainDb).
- Extending the closed `MaterializeError` arms inside the enum body
  (closed-sum extension, version-gated; e.g. a future `IndexCorrupt`
  variant for the persistent decoder).
- Extending the closed `CommitRollbackError` arms (closed-sum
  extension).
- Adding a new `ChainDbWrite` trait method (closed-trait extension,
  version-gated, requires updating all impls).

— **not** by exposing a parallel materializer, **not** by bypassing
`block_validity` in the replay-forward fold, **not** by moving the
chokepoint outside `ade_ledger::rollback::materialize`, **not** by
introducing a second `(LedgerState, PraosChainDepState)`-returning
`pub fn` in the rollback module tree, **not** by adding operator-
tunable cadence fields to `SnapshotCadence`.

**Declared non-goals carried from the cluster doc:** persistent
on-disk snapshot encoding (OQ-3 Path A scope edge —
`DC-CONS-21.open_obligation =
persistent_ledger_snapshot_encoding_follow_on_cluster`), pre-Conway
era support during replay (OQ-4 — `MaterializeError::EraNotSupported`),
snapshot eviction policy (OQ-5 — out of scope; in-memory cache
grows monotonically until process restart), operator-tunable runtime
cadence (OQ-2 — `SnapshotCadence` is BLUE-structural single-field).

### Receive-side admission authority (carried unchanged from PHASE4-N-H)

Carried. **N-I note:** the admit-side reducer arm (`BlockDelivered`)
is **structurally unchanged** at this HEAD; what changes is that the
reducer-side struct seam (`RollbackContext`) and the
`Option<&RollbackContext>` parameter make the rollback half a peer
of the admit half in the same reducer call. The `AdmittedBlock` /
`AdmittedOutcome` / `admit_via_block_validity` chokepoint set is
unchanged; the new rollback path does not produce an `AdmittedBlock`
— it produces `(LedgerState, PraosChainDepState)` via the materialize
chokepoint then commits via field replacement.

### Producer-side server response authority (carried unchanged from N-G)

Carried.

### Block production authority (carried unchanged from N-C)

Carried.

### Mempool ingress (carried unchanged from N-E)

Carried.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged from the prior revision. **N-I-specific
strengthening:** the full block validity composition contract
(`block_validity`) gains a **fifth public consumer** beyond the
validator's direct callers, `self_accept` (N-C), `served_chain_admit`
(N-G), and `admit_via_block_validity` (N-H): the rollback
materialize driver (`materialize_rolled_back_state`) composes
`block_validity` as the per-block step in its replay-forward fold.
`DC-CONS-20.strengthened_in += PHASE4-N-I` (rollback half closed —
admit + rollback symmetry over the same admission authority).

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. N-I added an `ade_runtime → ade_ledger` strengthening (RED →
  BLUE via the `ade_runtime::rollback::*` adapters importing the
  `ade_ledger::rollback::*` BLUE chokepoints + `RollbackContext` +
  the extended `ChainDbWrite::rollback_to_slot`) — same direction
  as existing N-C / N-G / N-H edges; allowed.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE. The new
  `ade_ledger::rollback::*` modules are BLUE; no async.
- **`ci_check_rollback_materialize_closure.sh`** *(N-I-S2 —
  CN-STORE-07; DC-CONS-22 enforcement)* — forbids any
  `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock in
  `crates/ade_ledger/src/rollback/materialize.rs` production code;
  enforces **single-authority discipline** via grep: no other
  `pub fn` in `crates/ade_ledger/src/rollback/*.rs` may return
  `(LedgerState, PraosChainDepState)`. Positive presence:
  `materialize_rolled_back_state` site MUST exist; the function
  body MUST call `block_validity` (single admission authority
  CN-CONS-08).
- **`ci_check_snapshot_cadence_purity.sh`** *(N-I-S4 — DC-STORE-07)*
  — forbids `HashMap`/`HashSet`/wall-clock/`tokio`/`rand` across
  all `crates/ade_runtime/src/rollback/*.rs` production code.
  Enforces **`SnapshotCadence` has exactly one field**
  (`every_n_blocks` — no operator-tunable runtime input). Positive
  presence: `should_snapshot_after_block`, `impl SnapshotReader for
  InMemorySnapshotCache`, `impl<...> BlockSource for ChainDbBlockSource`
  all MUST exist.
- *N-H carried CI gates:* `ci_check_admitted_block_closure.sh`,
  `ci_check_receive_reducer_closure.sh`,
  `ci_check_receive_replay_purity.sh`,
  `ci_check_receive_orchestrator_no_producer_dep.sh`,
  `ci_check_receive_paths_corpus_present.sh`.
- *N-G carried CI gates:* `ci_check_no_parallel_header_splitter.sh`,
  `ci_check_served_chain_closure.sh`,
  `ci_check_chain_sync_server_closure.sh`,
  `ci_check_block_fetch_server_closure.sh`,
  `ci_check_broadcast_to_served_purity.sh`,
  `ci_check_n2n_server_no_signing_dep.sh`,
  `ci_check_server_paths_corpus_present.sh`.
- *N-C carried CI gates:* `ci_check_private_key_custody.sh`,
  `ci_check_opcert_closed.sh`, `ci_check_forge_purity.sh`,
  `ci_check_no_producer_body_encoder.sh`,
  `ci_check_self_accept_gate.sh`, `ci_check_scheduler_closure.sh`,
  `ci_check_producer_corpus_present.sh`.
- `ci_check_constitution_coverage.sh` — carried.
- `ci_check_proposal_procedures_closed.sh` *(PP — DC-LEDGER-11)* — carried.
- `ci_check_mempool_ingress_closure.sh` /
  `ci_check_mempool_ingress_replay.sh` *(N-E)* — carried.
- `ci_check_credential_discriminant_closed.sh` *(OQ5 / COMMITTEE /
  DREP / ENACTMENT)* — carried.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5)* — carried.
- `ci_check_deposit_param_authority.sh` *(B3)* — carried.
- `ci_check_conway_cert_classification_closed.sh` *(B3F)* — carried.
- `ci_check_no_chaindb_in_consensus_blue.sh` /
  `ci_check_no_float_in_consensus.sh` /
  `ci_check_no_density_in_fork_choice.sh` /
  `ci_check_consensus_closed_enums.sh` — carried.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh` —
  carried.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PHASE4-N-I
added eight closed surfaces** — `SnapshotReader` (closed BLUE narrow
trait seam, 1 method), `BlockSource` (closed BLUE narrow trait seam,
1 method), `MaterializeError` (closed 3-variant sum),
`CommitRollbackError` (closed 1-variant sum), `TargetPoint` (closed
struct in `rollback::materialize` — distinct from receive's
`TargetPoint`), **the canonical materialize chokepoint**
(`materialize_rolled_back_state` — SOLE `pub fn` returning
`(LedgerState, PraosChainDepState)`), **the atomic commit chokepoint**
(`commit_rollback`), and **`RollbackContext`** (closed BLUE struct
in `receive::reducer`). Plus **one closed trait method extension**
(`ChainDbWrite::rollback_to_slot`) and **one closed BLUE-structural
struct** (`SnapshotCadence` — exactly one field, CI-defended). Plus
**two CI gates** (CI count 52 → 54) and **four newly-introduced
registry rules + one strengthening + closure** (`DC-CONS-20` flipped
from `declared` to `enforced`, `open_obligation` removed; registry
total 202 → 206).

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `CardanoEra` | `ade_types::era` | 8 variants | New variant = new hard fork. |
| `Certificate` | `ade_types::shelley::cert` | 7 variants | Shelley-era frozen. |
| `StakeCredential` *(OQ5)* | `ade_types::shelley::cert` | 2 variants | DC-LEDGER-10. |
| Credential-decode chokepoints *(OQ5 + PP)* | `ade_codec::{shelley,conway}::cert::decode_stake_credential` + `ade_codec::conway::governance::decode_stake_credential` | 3 functions | Closed 2-variant mapping. |
| `ConwayCert` *(B3/B4)* | `ade_types::conway::cert` | 19 variants | DC-LEDGER-08. |
| `GovAction` *(PP/ENACTMENT)* | `ade_types::conway::governance` | 7 variants | DC-LEDGER-11. |
| `ProposalProcedure` *(PP)* | `ade_types::conway::governance` | closed 4-field struct | DC-LEDGER-11. |
| `decode_proposal_procedures` / `encode_proposal_procedures` *(PP)* | `ade_codec::conway::governance` | 2 functions | DC-LEDGER-11. |
| `MIRPot` | `ade_types::shelley::cert` | 2 variants | Frozen. |
| `DRep` | `ade_types::conway::cert` | 4 variants | CIP-1694 fixed. |
| `CertDisposition` / `DepositEffect` / `CoinSource` *(B3)* | `ade_types::conway::cert` | 3 / 2 / 3 variants | Closed. |
| `ConwayCertAction` *(B4)* | `ade_ledger::delegation` | closed | No `Neutral`. |
| `GovernanceCertEffect` / `OwnerTaggedEffect` / etc. *(B4)* | `ade_ledger::delegation` | closed | B4 plumbing. |
| `GovCertEnv` *(B5)* | `ade_ledger::state` | closed struct | Fail-fast. |
| `apply_conway_gov_cert` dispatch *(B5)* | `ade_ledger::gov_cert` | 1 function | DC-LEDGER-09. |
| `apply_committee_enactment` *(ENACTMENT)* | `ade_ledger::governance` | 1 pure transition | Closed. |
| `IngressSource` *(N-E)* | `ade_ledger::mempool::ingress` | 2 variants | Closed source discriminant. |
| `IngressEvent` *(N-E)* | `ade_ledger::mempool::ingress` | closed struct | Closed flat-data envelope. |
| `mempool_ingress` chokepoint *(N-E)* | `ade_ledger::mempool::ingress` | 1 function | DC-MEM-03. |
| `ProducerTick` *(N-C-S3 — DC-CONS-13; carried)* | `ade_ledger::producer::state` | closed 14-field struct | Carried. |
| `forge_block` chokepoint *(N-C-S3)* | `ade_ledger::producer::forge` | 1 function | Carried. |
| `ForgeError` / `ForgeEffects` / `ForgedBlock` *(N-C-S3)* | `ade_ledger::producer::forge` | 7 / 1 / closed struct | Carried. |
| `encode_opcert` / `decode_opcert` chokepoint pair *(N-C-S2)* | `ade_codec::shelley::opcert` | 2 functions | Carried. |
| `OpCertCodecError` *(N-C-S2)* | `ade_codec::shelley::opcert` | 7 variants | Carried. |
| `opcert_validate` chokepoint *(N-C-S2)* | `ade_core::consensus::opcert_validate` | 1 function | Carried. |
| `OpCertError` *(N-C-S2)* | `ade_core::consensus::opcert_validate` | closed validation-error sum | Carried. |
| `block_body_hash_from_buckets` chokepoint *(N-C-S4 — DC-CONS-16; carried)* | `ade_ledger::block_body_hash` | 1 function | Carried. |
| `AcceptedBlock` token *(N-C-S5 — CN-CONS-07; carried)* | `ade_ledger::producer::self_accept` | 1 newtype (private field) | Carried. |
| `self_accept` chokepoint *(N-C-S5 — CN-CONS-07)* | `ade_ledger::producer::self_accept` | 1 function | Carried. |
| `SelfAcceptError` *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 variant — `Rejected(BlockValidityError)` | Carried. |
| `SchedulerInput` / `SchedulerEffect` / `SchedulerHaltReason` / `SchedulerState` *(N-C-S6)* | `ade_runtime::producer::scheduler` | closed sums | Carried. |
| `TickInputs` / `TickAssemblyError` / `assemble_tick` *(N-C-S6)* | `ade_runtime::producer::tick_assembler` | closed | Carried. |
| `BroadcastError` *(N-C-S6)* | `ade_runtime::producer::broadcast` | 2 variants | Carried. |
| RED signing primitives + key types *(N-C-S1 — DC-CRYPTO-03/04/05, OP-OPS-04)* | `ade_runtime::producer::signing::*` | closed | Carried. |
| RED key loader *(N-C-S1)* | `ade_runtime::producer::keys` | closed | Carried. |
| `accepted_block_header_bytes` canonical accessor *(N-G-S1 — DC-CONS-16 / DC-CONS-18)* | `ade_ledger::block_validity::header_input` | 1 function | Carried. |
| `ServerReply` (chain-sync + block-fetch) *(N-G-S1 — CN-PROTO-06)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed wrappers over private inner enums | Carried. |
| `HeaderProjection` *(N-G-S3)* | `ade_network::chain_sync::server` | closed struct | Carried. |
| `ServedHeaderLookup` / `ServedRangeLookup` traits *(N-G-S3/S4 — DC-PROTO-08 / DC-CONS-17)* | `ade_network::{chain_sync, block_fetch}::server` | 2 closed traits | Carried. |
| `producer_chain_sync_serve` / `producer_chain_sync_advance_tip` *(N-G-S3)* | `ade_network::chain_sync::server` | 2 functions | Carried. |
| `producer_block_fetch_serve` *(N-G-S4)* | `ade_network::block_fetch::server` | 1 function | Carried. |
| `Producer*ServerState` / `ProducerServerError` / `ProducerBlockFetchServerError` / `ServerStep` / `BlockFetchServerStep` *(N-G-S3/S4)* | `ade_network::{chain_sync, block_fetch}::server` | closed | Carried. |
| `ServedChainSnapshot` / `served_chain_admit` / `ServedChainAdmitError` *(N-G-S2)* | `ade_ledger::producer::served_chain` | closed | Carried. |
| `PerPeerN2nServerState` / `DispatchError` *(N-G-S6)* | `ade_runtime::network::n2n_server` | closed | Carried. |
| `AdmittedBlock` token *(N-H-S1 — CN-PROTO-07 + CN-CONS-07 mirror)* | `ade_ledger::receive::admitted` | 1 struct with private `bytes: Vec<u8>` field | Carried. |
| `AdmittedOutcome` *(N-H-S1)* | `ade_ledger::receive::admitted` | closed struct | Carried. |
| `admit_via_block_validity` chokepoint *(N-H-S1 — CN-PROTO-07)* | `ade_ledger::receive::admitted` | 1 function | Carried. |
| `ReceiveEvent` *(N-H-S1 — CN-PROTO-07)* | `ade_ledger::receive::events` | 3 variants | Carried. **N-I note:** unchanged; `RollBackward` now produces a `RolledBack` effect via `RollbackContext` instead of always erroring. |
| `ReceiveEffect` *(N-H-S1 — CN-CONS-08)* | `ade_ledger::receive::events` | 4 variants | Carried. **N-I note:** `RolledBack { to_slot }` arm is **NEWLY REACHABLE** at this HEAD (was unreachable in N-H Path A). |
| `NoOpReason` *(N-H-S1)* | `ade_ledger::receive::events` | 1 variant | Carried. |
| `ReceiveError` *(N-H-S1 — DC-CONS-19, DC-CONS-20)* | `ade_ledger::receive::events` | 4 variants | Carried. **N-I note:** `RollbackOutOfScope { target_point }` is **still reachable** — produced when `RollbackContext` is `None` (legacy callers) or when `MaterializeError::RollbackTooDeep` / `EraNotSupported` fires under `Some(ctx)`. Future cluster (persistent encoding) MAY add a `RollbackTooDeep` variant for clearer surfacing — closed-sum extension. |
| `TargetPoint` / `TipPoint` *(N-H-S1 — receive)* | `ade_ledger::receive::events` | 2 closed structs | Carried. |
| `PendingHeaderCache` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | closed struct | Carried. **N-I note:** `commit_rollback` resets this to `PendingHeaderCache::new()` after a successful rollback (post-rollback cached headers are stale). |
| `PendingHeaderCacheError` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache` | 1 variant | Carried. |
| `ChainDbWrite` trait *(N-H-S1; **N-I extended**)* | `ade_ledger::receive::chain_write` | **2 methods (was 1)** | **Closed seam — trait gained `rollback_to_slot(slot)` second method in N-I-S3.** Production impl (`ChainDbWriter` in ade_runtime, GREEN) extended; all mock impls (incl. N-H test mocks) extended. New impls would be a deliberate registry-tracked addition. New trait method = strengthening (closed extension, version-gated). |
| `ChainWriteError` *(N-H-S1)* | `ade_ledger::receive::chain_write` | 2 variants | Carried. |
| `ChainWriteErrorKind` *(N-H-S1)* | `ade_ledger::receive::chain_write` | 3 variants | Carried. |
| `ReceiveState` *(N-H-S2)* | `ade_ledger::receive::reducer` | closed struct | Carried. **N-I note:** `commit_rollback` mutates the three fields atomically (ledger replacement + chain_dep replacement + pending_headers reset). |
| `receive_apply` chokepoint *(N-H-S2 — DC-CONS-19; CN-CONS-08; **N-I-S6 signature evolution**)* | `ade_ledger::receive::reducer` | 1 function — **signature gained `Option<&RollbackContext>` parameter** | Carried + extended. The reducer is still the single receive-side composition root; the new parameter is additive (None preserves N-H legacy behavior). |
| `receive_apply_sequence` driver *(N-H-S2)* | `ade_ledger::receive::reducer` | 1 function | Carried. |
| `PerPeerReceiveState` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | closed RED struct | Carried. |
| `ReceiveDispatchError` *(N-H-S4)* | `ade_runtime::receive::orchestrator` | 3 variants | Carried. |
| **`SnapshotReader` trait** *(NEW in N-I-S1 — CN-STORE-07; closed narrow trait seam)* | `ade_ledger::rollback::traits` | 1 trait with 1 method — `nearest_le(target_slot: SlotNo) -> Option<(SlotNo, LedgerState, PraosChainDepState)>` | **Closed seam** — single production impl at this cluster (`InMemorySnapshotCache`, GREEN, BTreeMap-backed). Object-safe. Read-only. Returns owned types because the materialize driver needs mutability for the replay-forward fold. **Persistent impl is the follow-on cluster's deliverable (DC-CONS-21)** — drops in without changing `materialize_rolled_back_state`. |
| **`BlockSource` trait** *(NEW in N-I-S1 — CN-STORE-07; closed narrow trait seam)* | `ade_ledger::rollback::traits` | 1 trait with 1 method — `blocks_in_range(from_exclusive: SlotNo, to_inclusive: SlotNo) -> Vec<(SlotNo, Vec<u8>)>` | **Closed seam** — single production impl (`ChainDbBlockSource<'a, D: ChainDb>`, GREEN, over any `ChainDb`). Object-safe. Read-only. Returns owned `Vec<(SlotNo, Vec<u8>)>` byte-identical to the underlying store. No second impl anticipated — `ChainDbBlockSource` covers all foreseeable cases (rollback replay always sources from the canonical ChainDb). |
| **`MaterializeError`** *(NEW in N-I-S1)* | `ade_ledger::rollback::error` | 3 variants — `RollbackTooDeep { target_slot, oldest_snapshot: Option<SlotNo> }`, `ReplayFailedAt { slot, error: BlockValidityError }`, `EraNotSupported { era, slot }` | Closed sum. No `#[non_exhaustive]`; no `String`. New variant = closed-sum extension (e.g. future `IndexCorrupt` for the persistent decoder). |
| **`CommitRollbackError`** *(NEW in N-I-S1)* | `ade_ledger::rollback::error` | 1 variant — `ChainDb(ChainWriteError)` | Closed sum. The single variant signals the chain_write rollback failure path; the receive state is unchanged on this error (irreversible step is first). |
| **`TargetPoint`** *(NEW in N-I-S2 — rollback flavor)* | `ade_ledger::rollback::materialize` | closed struct `{ slot: SlotNo, hash: Hash32 }` | Closed struct. Distinct from `ade_ledger::receive::events::TargetPoint` (which carries the same shape but lives in the receive event taxonomy). The receive reducer's roll_backward helper constructs the rollback `TargetPoint` from the receive `TargetPoint` (mechanical conversion). |
| **`materialize_rolled_back_state` chokepoint** *(NEW in N-I-S2 — CN-STORE-07)* | `ade_ledger::rollback::materialize` | 1 function — **THE SOLE `pub fn` returning `(LedgerState, PraosChainDepState)` in the rollback module tree** | The **single canonical** rollback materialization chokepoint. Mirrors `block_validity`'s single-authority discipline. Composes one `SnapshotReader::nearest_le` lookup + per-block `block_validity` fold over `BlockSource::blocks_in_range`. Pure, total, deterministic. Defended by `ci_check_rollback_materialize_closure.sh` (single-authority grep across `crates/ade_ledger/src/rollback/*.rs`). New chokepoint at this signature = strengthening (CI would fail). |
| **`commit_rollback` chokepoint** *(NEW in N-I-S3)* | `ade_ledger::rollback::commit` | 1 function — generic over `W: ChainDbWrite` | The **atomic commit** for a materialized rollback. Sequence is irreversible-first staged: chain_write rollback (fallible) → ledger replacement (infallible) → chain_dep replacement (infallible) → pending_headers reset (infallible). On chain_write failure, state is unchanged. |
| **`ChainDbWrite::rollback_to_slot` trait method** *(NEW in N-I-S3)* | `ade_ledger::receive::chain_write` | 1 method added to the existing closed `ChainDbWrite` trait | **Closed trait extension** — second method on N-H's closed seam. Signature: `fn rollback_to_slot(&mut self, slot: SlotNo) -> Result<(), ChainWriteError>`. All existing impls extended (production `ChainDbWriter` in ade_runtime, GREEN; mock impls in N-H tests). |
| **`RollbackContext<'a>`** *(NEW in N-I-S6)* | `ade_ledger::receive::reducer` | closed BLUE struct `{ snapshot_reader: &'a dyn SnapshotReader, block_source: &'a dyn BlockSource }` | The **single receive-side rollback entry point**. New rollback sources (persistent snapshot store, cross-peer snapshot store) attach by implementing `SnapshotReader` and passing through this struct. Field set is closed; new field = strengthening (version-gated). |
| **`SnapshotCadence`** *(NEW in N-I-S4 — DC-STORE-07)* | `ade_runtime::rollback::cadence` | closed BLUE-structural struct **with exactly 1 field** (`every_n_blocks: u32`) | CI-defended to exactly 1 field. Default 100. Operator-tunable runtime cadence is **explicitly out of scope** to preserve replay-equivalence. If a future cluster ratifies an operator-tunable knob, it must be represented as anchored, replay-derivable runtime data — not a runtime-mutable parameter. New field = CI failure (cluster boundary). |
| `PlutusLanguage` | `ade_plutus::evaluator` | 3 variants | |
| Named ingress chokepoints (block CBOR) | `ade_codec::*` | 10 | |
| Conway cert/withdrawals sub-grammar decoders *(B3 / B4)* | `ade_codec::conway::{cert, withdrawals}` + `ade_codec::shelley::cert::read_pool_registration_cert` | 5 functions | Closed. |
| Named ingress chokepoint (Plutus script CBOR) | `ade_plutus::evaluator::PlutusScript::from_cbor` | 1 | |
| `PreservedCbor::new` constructor | `ade_codec::preserved` | 1 chokepoint, `pub(crate)` | |
| `CodecError` variants *(B3-extended)* | `ade_codec::error` | + `UnknownCertTag`, `DuplicateMapKey` | |
| Mini-protocol message enums | `ade_network::codec::*` | 11 closed enums | |
| Mini-protocol encode/decode chokepoints | `ade_network::codec::*::{encode_*, decode_*}` | 22 functions | |
| Mux frame chokepoints | `ade_network::mux::frame::{encode_frame, decode_frame}` | 2 free functions | |
| Mini-protocol transition functions | `ade_network::*::transition` + `n2c::local_*::transition` | 8 modules | |
| Mini-protocol version enums | `ade_network::codec::version::*` | 11 closed enums | |
| `ChainDb` / `SnapshotStore` / `Recoverable` trait surfaces | `ade_runtime::chaindb` + `ade_runtime::recovery` | closed | **N-I note:** the `ChainDb::rollback_to_slot(slot)` method already existed (N-D); N-I consumes it through the new `ChainDbWrite::rollback_to_slot` BLUE trait method which production impl `ChainDbWriter` forwards. |
| Hash domain functions | `ade_crypto::blake2b::*` | 4 named domains | |
| `ChainEvent` / `ChainSelectionReject` *(N-B)* | `ade_core::consensus::events` | 5 / 4 variants | |
| Consensus error families *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | |
| `StreamInput` / `OrchestratorError` / `DecodeError` / `GenesisParseError` / `GenesisBlob` / `NetworkMagic` *(N-B)* | various | closed | |
| `LedgerView` trait *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods | |
| `HeaderVrf` *(N-B; B1)* | `ade_core::consensus::header_summary` | 2 variants | |
| `BlockValidityVerdict` / `BlockValidityError` etc. *(B1)* | `ade_ledger::block_validity::verdict` | closed | |
| `block_validity` chokepoint *(B1; **N-I strengthened**)* | `ade_ledger::block_validity::transition` | 1 function | Single chokepoint. `self_accept` (N-C-S5), `admit_via_block_validity` (N-H-S1), `served_chain_admit` (N-G-S2), and **the rollback materialize driver `materialize_rolled_back_state` (N-I-S2 — per-block in the replay-forward fold)** are its public consumers. `DC-CONS-20.strengthened_in += PHASE4-N-I`. |
| `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` / `SignerSource` / `WitnessClosureError` etc. *(B2)* | `ade_ledger::tx_validity::*` | closed | |
| `AdmitOutcome` / `MempoolState` / `OrderPolicy` *(B2)* | `ade_ledger::mempool::*` | closed | |
| `LeaderScheduleAnswer` / `is_leader_for_vrf_output` *(N-B; consumed unchanged by N-C)* | `ade_core::consensus::leader_schedule` | closed | |
| `PraosNonces` / `NonceScanError` *(B1)* | `ade_ledger::consensus_input_extract` | | |
| `PraosChainDepState` / `ChainEvent` canonical encodings *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| `LedgerFingerprint` fold *(B3/B5)* | `ade_ledger::fingerprint` | | **N-I note:** the materialize driver does not call `fingerprint` directly; `fingerprint` is the proof tool used by N-I's S2 test `materialize_replay_forward_equals_direct_apply` to verify byte-equivalence of the snapshot+replay-forward state with the direct-apply state (DC-CONS-22 closure). |
| **CI check set** | `ci/ci_check_*.sh` | **54 scripts (52 → 54 in PHASE4-N-I)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-I added 4 rules** (`CN-STORE-07`, `DC-CONS-22`, `DC-STORE-07`, `DC-CONS-21` `declared`); strengthened + closed `DC-CONS-20` (`open_obligation` removed). Total: **206 entries** (202 → 206). | Append-only IDs. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| `CostModels` map (Plutus V1/V2/V3 cost tables) | `ade_plutus::cost_model::CostModels` | Decoder-driven; constrained by closed `PlutusLanguage`. |
| `ProtocolParameters` / `ProtocolParameterUpdate` field set | `ade_ledger::pparams` | Era-versioned. |
| Pool / DRep / Stake registrations | `ade_ledger::state::{DelegationState, CertState}` | Shape closed; set open. |
| Governance proposal / committee / DRep registration set | `ade_ledger::state::ConwayGovState` | Shape closed; instance set open. |
| Tx-body `proposal_procedures` instance set *(PP)* | `ade_types::conway::tx::ConwayTxBody.proposal_procedures` | `Option<Vec<ProposalProcedure>>`. Shape closed; instance set open. |
| `OpCertCounterMap` *(N-B)* | `ade_core::consensus::praos_state` | BTreeMap; inserts strictly increasing per `(pool, kes_period)`. |
| `PoolDistrView` pool table *(B1)* | `ade_ledger::consensus_view::PoolDistrView::pools` | `BTreeMap<Hash28, PoolEntry>`. |
| Withdrawals map *(B3)* | `ade_codec::conway::withdrawals::decode_withdrawals` → `BTreeMap<RewardAccount, Coin>` | Never last-wins. |
| Mempool admitted set *(B2; ingress-fed in N-E)* | `ade_ledger::mempool::admit::MempoolState::accepted` | `Vec<Hash32>`; shape closed; set open; monotonic. |
| `SignerSource` provenance set *(B2)* | `ade_ledger::tx_validity::required_signers::RequiredSigners::{keys, provenance}` | Per-tx open; closed enum. |
| `RollbackSnapshot` ring *(N-B; carried)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ 2160. **N-I note:** distinct from the new N-I `InMemorySnapshotCache` — `RollbackSnapshot` belongs to the chain-selector orchestrator state (consensus-tip-tracking); `InMemorySnapshotCache` belongs to the receive-side rollback authority (ledger-state-tracking). They could be unified by a future cluster but are deliberately separate today. |
| `ServedChainSnapshot.blocks` admitted set *(N-G-S2)* | `ade_ledger::producer::served_chain::ServedChainSnapshot` | Shape closed; instance set open. |
| `PerPeerN2nServerState` instance set *(N-G-S6)* | `ade_runtime::network::n2n_server` | One instance per connected peer (producer-side server pump). |
| `PendingHeaderCache.entries` *(N-H-S1)* | `ade_ledger::receive::pending_header_cache::PendingHeaderCache` | `BTreeMap<(SlotNo, Hash32), Vec<u8>>`. Shape closed; instance set open. **N-I note:** the cache is reset to empty after every successful rollback (`commit_rollback` step 4). |
| `PerPeerReceiveState` instance set *(N-H-S4)* | `ade_runtime::receive::orchestrator` | One instance per connected upstream peer. |
| **`InMemorySnapshotCache.entries`** *(NEW in N-I-S4 — runtime-extensible **content**, but extension via the closed `InMemorySnapshotCache::admit` chokepoint only; bounded structurally by the snapshot-write cadence policy)* | `ade_runtime::rollback::in_memory_cache::InMemorySnapshotCache` | `BTreeMap<SlotNo, (LedgerState, PraosChainDepState)>`. Shape closed; instance set open. The set grows during a follow session via `maybe_capture_snapshot` calls; no eviction at this cluster (OQ-5 — out-of-scope; in-memory cache grows monotonically until process restart). Eviction is a **named follow-on candidate seam**. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus / Consensus corpus / Block-validity corpus / Tx-validity corpus / Mempool ingress corpus / PP canonical corpus / Producer corpus / Server-paths corpus / Receive-paths corpus | various | Tooling-only. |
| **Receive-rollback integration test** *(NEW in N-I-S6 — tooling-only)* | `crates/ade_runtime/tests/receive_rollback_integration.rs` | Tooling-only. End-to-end test wiring `RollbackContext` + `InMemorySnapshotCache` + `ChainDbBlockSource` through the full receive pipeline; proves `rollback_then_continue_admit_equals_straight_line_admit` (DC-CONS-22 end-to-end snapshot-is-cache proof). |
| Operator-action probe binaries *(N-B + N-E S6 + N-C S7 + N-G S7 + N-H S6)* | `ade_core_interop::bin::{live_consensus_session, live_tx_submission_session, live_block_production_session, live_block_fetch_session, live_block_follow_session}` | RED operator-action; `#[ignore]`-gated by closure-gate tests. **N-I added no new binary** — rollback authority has no Tier-1 wire-format counterpart that requires a live cross-impl probe. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **Persistent ledger snapshot encoding cluster** *(NEW HIGHEST-PRIORITY candidate flagged by N-I close — `DC-CONS-21.open_obligation = persistent_ledger_snapshot_encoding_follow_on_cluster`)* | **`encode_ledger_state` + `decode_ledger_state` chokepoint pair + a `PersistentSnapshotStore` impl of `SnapshotReader` over `SnapshotStore` bytes** | The natural counterpart to N-I's in-memory scope. Encoder is ~1500-2000 LoC BLUE field-walk mirroring `ade_ledger::fingerprint`; encoded layout `[closed version tag][canonical-CBOR LedgerState bytes][fingerprint]` enables decode-side integrity verification. Drops in to `materialize_rolled_back_state` without changing the chokepoint. Surface; do not invent invariants here. |
| **Snapshot eviction policy cluster** *(NEW candidate flagged by N-I close — OQ-5 declared non-goal)* | **`InMemorySnapshotCache::evict_older_than(slot)` + a `SnapshotEvictionPolicy` Tier-5 operator-tunable** | Tier-5 operational concern. Bounded-ring policy modeled on `chain_selector::OrchestratorState::recent_snapshots` (≤ 2160) is the obvious shape. Must remain replay-deterministic. Surface; do not invent invariants here. |
| **Multi-peer fork choice cluster** *(carried from N-H; now unblocked by N-I's rollback closure)* | **Praos longest-chain selection across competing `PerPeerReceiveState[]`** | Re-uses N-I's `RollbackContext` to roll back losing forks. Consumes existing `ade_runtime::consensus::chain_selector`. Surface; do not invent invariants here. |
| **N2C local-chain-sync receive surface cluster** *(carried from N-H)* | **Operator-side N2C clients consume Ade's chain via `LocalChainSyncMessage`** | Sibling of `producer_chain_sync_serve` (N-G). Surface; do not invent invariants here. |
| **CE-N-H-6 (operator-action live evidence — carried)** | **Live N2N follow-mode admission log (Ade consuming, cardano-node serving)** | Carried. |
| **CE-N-G-8 (operator-action live evidence — carried)** | **Live N2N block-fetch acceptance log (Ade serving)** | Carried. |
| **CE-N-C-8 (operator-action live evidence — carried)** | **Live N2N block-fetch acceptance log (Ade forging)** | Carried. |
| **N-I+ Tier-5** | **Operator-tunable rollback policy** (snapshot cadence parameter as anchored replay-derivable runtime data; per-peer rollback throttling; rollback depth bound) | Tier-5 — operator-tunable. Declared OUT-OF-SCOPE in N-I cluster doc. Sequenced after persistent snapshot encoding (cadence-as-runtime-data needs a persistent representation). |
| **N-G+ Tier-5** | **Operator-tunable server policy** | Carried. |
| **N-C+ Tier-5** | **Operator-tunable producer policy** | Carried. |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | Carried. |
| **PP OQ-1..OQ-4** | various | Carried. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

### Closed-grammar audit (PHASE4-N-I full close)

This sweep was performed after PHASE4-N-I full close (S1..S6).

1. **`SnapshotReader` closed narrow trait seam** — **closed by intent.**
   Single method; object-safe; read-only; single production impl
   (`InMemorySnapshotCache`); persistent impl is the follow-on
   cluster's deliverable.
2. **`BlockSource` closed narrow trait seam** — **closed by intent.**
   Single method; object-safe; read-only; single production impl
   (`ChainDbBlockSource`).
3. **`MaterializeError` / `CommitRollbackError` closed sums** —
   **closed by intent.** 3 / 1 variants; no `#[non_exhaustive]`; no
   `String`. Round-trip-through-pattern-match tests confirm a fourth
   variant addition fails to compile.
4. **`materialize_rolled_back_state` chokepoint** — **closed by intent
   and CI-defended (single-authority grep).** The SOLE `pub fn`
   returning `(LedgerState, PraosChainDepState)` in the rollback
   module tree. Composes `block_validity` (B1) via the per-block
   fold — does not re-implement admission.
5. **`commit_rollback` chokepoint** — **closed by intent.** Atomic
   4-step; irreversible-first staged; on chain_write failure the
   state is unchanged. End-to-end atomicity test
   `commit_rollback_chain_write_failure_leaves_state_unchanged`.
6. **`ChainDbWrite::rollback_to_slot` trait extension** — **closed by
   intent.** Second method on N-H's closed trait seam. All existing
   impls extended.
7. **`RollbackContext` BLUE struct seam** — **closed by intent.**
   The single receive-side rollback entry point. Field set closed.
8. **`receive_apply` signature evolution** — **additive at the call
   site.** `Option<&RollbackContext>` — `None` preserves N-H legacy
   behavior; `Some` wires the rollback path.
9. **`SnapshotCadence` BLUE-structural with exactly one field** —
   **closed by intent and CI-defended.** `ci_check_snapshot_cadence_purity.sh`
   enforces exactly one field. Operator-tunable runtime cadence
   explicitly out of scope (DC-STORE-07).
10. **GREEN `InMemorySnapshotCache` adapter** — **closed by intent.**
    Single production `SnapshotReader` impl. BTreeMap-backed (no
    HashMap). No eviction at this cluster (OQ-5).
11. **GREEN `ChainDbBlockSource` adapter** — **closed by intent.**
    Single production `BlockSource` impl over any `ChainDb`. Returns
    bytes byte-identical to the underlying store.
12. **RED `maybe_capture_snapshot` hook** — **closed by intent.**
    Pure-function hook; classified RED by location (composes RED
    dispatch outcomes with GREEN cadence + cache). Decision logic
    itself has no I/O / no clock.

**Gap note — DC-CONS-21 persistent encoding.** The Path A in-memory
scope is NOT a "we'll match it later" stub — it is a working
end-to-end rollback authority that fully closes `DC-CONS-20`
(rollback-side) within a session. Persistence across restarts is the
follow-on cluster's deliverable; the registry rule `DC-CONS-21` ships
`status = "declared"` with `open_obligation =
persistent_ledger_snapshot_encoding_follow_on_cluster` naming the
follow-on cluster's deliverable. **This is the highest-priority
explicit candidate seam for the next planner.**

**Gap note — OQ-5 snapshot eviction.** Not a "we'll fix it later"
issue — the in-memory cache grows monotonically by design at this
cluster. Eviction is a follow-on operational concern, surfaced as a
named candidate seam.

### Closed-grammar audit (carried — PHASE4-N-H / PHASE4-N-G / PHASE4-N-C / PROPOSAL-PROCEDURES-DECODE / PHASE4-N-E / B3 / B4 / B5)

All carried unchanged from prior revision.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags 0..=7.
- **`PreservedCbor<T>` invariant**.
- **Hash algorithms**: Blake2b-224 / 256, Ed25519, Byron-bootstrap,
  KES-sum, VRF-draft-03.
- **Era-correct block body hash** *(B1; strengthened in N-C, N-G,
  N-H)*: preserved-CBOR-segment bytes.
- **Single canonical body-hash authority** *(N-C-S4 — DC-CONS-16;
  strengthened in N-G, N-H)*: `block_body_hash_from_buckets` is the
  **only** function computing the recipe. Carried.
- **Single canonical header/body splitter** *(N-G-S1 — DC-CONS-18)*:
  `accepted_block_header_bytes` is the **only** public header-bytes
  accessor. Carried.
- **Server-agency closure for outgoing mini-protocol messages**
  *(N-G-S1 — CN-PROTO-06)*: carried.
- **Receive-event closure for incoming peer signals** *(N-H-S1 —
  CN-PROTO-07)*: carried. `ReceiveEvent` taxonomy unchanged at three
  variants.
- **Type-level receive admission gate** *(N-H-S1 — CN-CONS-07
  strengthening)*: carried. `AdmittedBlock` private-constructor;
  raw bytes have no public path into ChainDb.
- **Receive-side admission state-isolation discipline (Invariant
  I-6)** *(N-H-S2 — CN-CONS-08 / DC-CONS-19)*: carried.
- **Single canonical receive-side rollback materialization
  authority** *(NEW in N-I-S2 — CN-STORE-07)*: `materialize_rolled_back_state`
  is the **SOLE `pub fn` returning `(LedgerState, PraosChainDepState)`
  in the entire rollback module tree**. Mirrors `block_validity`'s
  single-authority discipline for the admit side. Composes one
  `SnapshotReader::nearest_le` + `BlockSource::blocks_in_range` +
  per-block `block_validity` — does not re-implement admission. CI
  enforcement via single-authority grep across `crates/ade_ledger/src/rollback/*.rs`.
- **Replay-forward correctness** *(NEW in N-I-S2 — DC-CONS-22)*: for
  any reachable `(LedgerState, PraosChainDepState)`, snapshot+replay-
  forward via `materialize_rolled_back_state` produces a state whose
  `ade_ledger::fingerprint::fingerprint` matches the state produced
  by direct-apply via `apply_block_with_verdicts` over the same
  block sequence. Snapshot is a **pure cache** over canonical history,
  never an authoritative side path. Epoch boundaries are handled
  implicitly by `apply_block_with_verdicts` (rules.rs:244-250 calls
  `detect_epoch_transition` + `apply_epoch_boundary_full`) — no
  duplicate epoch-transition path in materialize. End-to-end proof:
  `materialize_replay_forward_equals_direct_apply` (S2) and
  `rollback_then_continue_admit_equals_straight_line_admit` (S6
  integration).
- **Atomic rollback commit discipline** *(NEW in N-I-S3 — supports
  DC-CONS-20 rollback-side closure)*: `commit_rollback` is staged-
  then-committed with irreversible step first. Sequence:
  (1) `chain_write.rollback_to_slot` (fallible; failure → `Err`,
  state unchanged); (2) `state.ledger = new_ledger` (infallible);
  (3) `state.chain_dep = new_chain_dep` (infallible); (4)
  `state.pending_headers = PendingHeaderCache::new()` (infallible).
  Test `commit_rollback_chain_write_failure_leaves_state_unchanged`
  proves the unchanged-on-failure property.
- **Receive-side atomic admit + rollback over ChainDb +
  LedgerState + PraosChainDepState** *(N-H-S2 admit + N-I-S6
  rollback — DC-CONS-20 fully closed at this HEAD)*: a successful
  receive-side admission updates ChainDb, LedgerState, and
  PraosChainDepState as one structural transition. A successful
  RollBackward rolls back all three (plus `pending_headers`) to the
  same slot. No path leaves them out of sync; no partial admission;
  no partial rollback. `DC-CONS-20.strengthened_in += PHASE4-N-I`;
  `open_obligation` removed.
- **Receive-reducer rollback-context discipline** *(NEW in N-I-S6)*:
  `RollbackContext` is the **only** way `receive_apply`'s
  `RollBackward` arm obtains rollback authority. `Option<&RollbackContext>`
  parameter is additive at the call site: `None` preserves N-H
  legacy behavior (`Err(RollbackOutOfScope)`); `Some` wires the
  rollback path. New rollback sources attach by implementing
  `SnapshotReader` and passing through `RollbackContext` — not by
  exposing a parallel arm, not by widening the struct.
- **Snapshot cadence determinism** *(NEW in N-I-S4 — DC-STORE-07)*:
  the decision to take a snapshot at slot `S` is a pure function of
  `(slot, block_no, cadence_params, last_snapshot)`. Same canonical
  input chain history → same set of snapshot slot keys. Cadence is
  **BLUE-structural** (single-field struct, CI-defended), **not
  operator-tunable in this cluster**. Operator-tunable runtime
  cadence is out of scope until represented as anchored, replay-
  derivable runtime data — a future cluster's deliverable.
- **`ChainDbWrite::rollback_to_slot` trait method semantics** *(NEW
  in N-I-S3)*: rolls the underlying chain store back to `slot`,
  discarding all blocks at slots strictly greater than `slot`. After
  this returns `Ok`, no read operation observes such a block. Rolling
  back beyond the empty tip is `Ok(())` per the underlying ChainDb
  contract.
- **Receive-side replay determinism** *(N-H-S3 — DC-PROTO-09)*: carried.
- **Per-peer receive-state independence across peers** *(N-H-S4)*: carried.
- **Key-boundary for receive paths** *(N-H-S4 — OP-OPS-04 mirror)*: carried.
- **Handshake-negotiated version threading through the receive
  reducer call site** *(N-H-S4 — DC-PROTO-06 strengthening)*: carried.
- **Served-bytes parity** *(N-G-S4 — DC-CONS-17)*: carried.
- **Header-body wire coherence** *(N-G-S5 — DC-CONS-18)*: carried.
- **Producer-side server-role transcript determinism** *(N-G-S5 —
  DC-PROTO-07)*: carried.
- **Deterministic-resolution discipline for server-agency waits**
  *(N-G-S3 — DC-PROTO-08)*: carried.
- **Type-level broadcast and serve gate** *(N-C-S5 — CN-CONS-07;
  N-G + N-H strengthened)*: carried.
- **Tx id over preserved body bytes** *(B2)*.
- **Conway certificate CDDL grammar** *(B3/B3F/B4)*.
- **Conway `DRep` decode grammar** *(B4)*.
- **Owner-tagged Conway cert-state apply contract** *(B4)*: DC-LEDGER-08.
- **Closed total gov-cert dispatch contract** *(B5)*: DC-LEDGER-09.
- **Fail-fast gov-cert environment** *(B5)*.
- **Checked DRep-expiry arithmetic** *(B5)*.
- **`ConwayGovState` deterministic-fold accumulation** *(B5)*.
- **Conway withdrawals map grammar** *(B3)*: never last-wins.
- **Closed deposit-effect sum types** *(B3)*.
- **Canonical deposit-param authority** *(B3)*: DC-TXV-07.
- **Full Conway value-conservation equation** *(B3)*: frozen §9.1
  reject precedence.
- **`LedgerFingerprint` Conway deposit-param fold** *(B3)*.
- **Closed `proposal_procedures` wire grammar at Conway tx-body
  key 20** *(PP — DC-LEDGER-11)*.
- **Plutus script ingress chokepoint**: `PlutusScript::from_cbor`.
- **Plutus language set**: V1, V2, V3.
- **Aiken UPLC quarantine pin**: `aiken_uplc` at tag `v1.1.21`.
- **Ouroboros mux frame layout**: 8-byte big-endian header.
- **11 closed mini-protocol message enums** + **8 closed state graphs**.
- **`BootstrapAnchorHash` v1 preimage** *(N-B)*.
- **`EraSchedule` invariants** *(N-B)*.
- **`PraosChainDepState` / `ChainEvent` CBOR encodings** *(N-B)*.
- **Consensus error taxonomies** *(N-B)*.
- **`StreamInput` 3-variant taxonomy** *(N-B)*. **`HeaderVrf` era model**.
- **`block_validity` composition contract** *(B1; **N-I strengthened**
  — fifth public consumer via `materialize_rolled_back_state`'s
  per-block replay-forward fold)*.
  `DC-CONS-20.strengthened_in += PHASE4-N-I` (rollback-side closure:
  admit + rollback symmetry over the same admission authority).
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*.
- **`mempool_ingress` chokepoint contract** *(N-E)*.
- **`IngressSource` source-invariance contract** *(N-E)*.
- **Verbatim tx-bytes flow through ingress** *(N-E; N-H mirror)*: carried.
- **GREEN single-step replay fold contract** *(N-E — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(N-E; carried)*.
- **Operator-action evidence pattern** *(N-B / N-E / N-C / N-G /
  N-H)*: carried. **N-I adds no new instance** — rollback is wholly
  internal authority.
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE /
  DREP / ENACTMENT / PP)*.
- **Committee-enactment write-back contract** *(ENACTMENT)*.
- **All canonical types**: shapes frozen at the era / version they
  entered.
- **Handshake-negotiated version threading** *(N-A; strengthened in
  N-G and N-H — DC-PROTO-06)*: carried.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  **N-I additions:** `ade_ledger::rollback::{traits, error,
  materialize, commit}` are BLUE (under the already-BLUE `ade_ledger`
  crate prefix); `ade_runtime::rollback::{cadence, in_memory_cache,
  chaindb_block_source}` are GREEN-inside-RED-crate;
  `ade_runtime::rollback::snapshot_writer` is RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D).
  **N-I note:** `ChainDb::rollback_to_slot(slot)` already existed
  in N-D; N-I consumes it via the new closed BLUE
  `ChainDbWrite::rollback_to_slot` trait method that production impl
  `ChainDbWriter` forwards.
- **`AcceptedBlock` type-level broadcast gate** *(N-C-S5; carried)*.
- **`AdmittedBlock` type-level admission gate** *(N-H-S1; carried)*.
- **`RollbackContext` BLUE struct seam** *(NEW in N-I-S6)*: the
  **only** receive-side rollback entry point. Field set closed
  (`{ snapshot_reader: &dyn SnapshotReader, block_source: &dyn BlockSource }`).
  New field = strengthening; version-gated.
- **`SnapshotCadence` BLUE-structural single-field discipline** *(NEW
  in N-I-S4 — DC-STORE-07)*: exactly one field
  (`every_n_blocks: u32`). CI-defended. Operator-tunable runtime
  cadence requires explicit cluster ratification (Tier-5 with
  anchored replay-derivable representation).
- **`forge_block` pure-transition contract** *(N-C-S3 — DC-CONS-13)*: carried.
- **Single source of leader truth** *(N-C-S3 — DC-CONS-15)*: carried.
- **Tx-admissibility prefix property** *(N-C-S3 — DC-LEDGER-12)*: carried.
- **Private-key custody RED-confinement** *(N-C-S1; carried)*.
- **Closed-grammar opcert byte authority** *(N-C-S2 — DC-CONS-11)*: carried.
- **OpCert serial counter strict monotonicity** *(N-C-S2 — DC-CONS-12)*: carried.

### Version-gated (can evolve across major versions)

- **New `CardanoEra` variant**: full coordinated change.
- **New Conway certificate tag** *(B3 / B4 / B5)*.
- **New `CoinSource` deposit-provenance** *(B3)*.
- **Pre-Conway single-tx validity** *(B2 extension point)*.
- **Full-scope `track_utxo=true` tx corpus** *(B2 extension point)*.
- **Conway block-body vkey-witness closure** *(B2-carried)*.
- **Conway governance certificate accumulation** *(B5)*.
- **Credential discriminant extension** *(declared non-goal)*.
- **Committee-enactment write-back** *(ENACTMENT)*.
- **Conway tx-body `proposal_procedures` decode** *(PP — wired)*.
- **TPraos full-block validity** *(B1 extension point)*.
- **TPraos producer** *(N-C declared non-goal — OQ-4 lock)*.
- **New `GovAction` / Plutus version variant**.
- **New `SignerSource` / `TxRejectClass` / `BlockRejectClass` /
  `OrderPolicy` variant**.
- **New protocol parameter field**.
- **New `ProducerTick` field** *(N-C extension point)*.
- **New `ForgeError` / `SchedulerInput` / `SchedulerEffect` variant**.
- **New `SelfAcceptError` variant** *(N-C extension point)*.
- **New `ServerStep` / `BlockFetchServerStep` / `ServerReply` /
  `ServedHeaderLookup` method / `ServedRangeLookup` method /
  `ServedHeaderLookup` impl / `ServedRangeLookup` impl /
  `ServedChainAdmitError` variant / `DispatchError` variant**
  *(N-G extension points)*: carried.
- **New `ReceiveEvent` variant** *(N-H — CN-PROTO-07 extension
  point)*: carried.
- **New `ReceiveEffect` variant** *(N-H — CN-CONS-08 extension
  point)*: carried. **N-I note:** `RolledBack { to_slot }` is now
  reachable (was unreachable in N-H Path A).
- **New `ReceiveError` variant** *(N-H — DC-CONS-19 / DC-CONS-20
  extension point)*: carried. **N-I note:** `RollbackOutOfScope` is
  still produced — by `None` callers (legacy) and by
  `MaterializeError::RollbackTooDeep` / `EraNotSupported` mapping
  under `Some(ctx)`. A future cluster MAY add a distinct
  `RollbackTooDeep` variant for clearer surfacing.
- **New `ChainDbWrite` impl** *(N-H — deliberate registry-tracked
  addition; **N-I extended the trait surface to 2 methods**)*: any
  future impl must extend both `write_admitted` AND `rollback_to_slot`.
- **New `ChainDbWrite` trait method** *(N-H extension point; **N-I
  used it once — added `rollback_to_slot`**)*: closed seam; new
  methods are closed-trait extensions and require an updated
  production impl + extended reducer logic. Version-gated.
- **New `ReceiveDispatchError` variant** *(N-H extension point)*: carried.
- **New `SnapshotReader` impl** *(NEW in N-I — deliberate registry-
  tracked addition)*: the trait is a closed seam, but a future
  cluster MUST register a second impl
  (`PersistentSnapshotStoreReader` once the persistent encoding
  cluster ships — `DC-CONS-21`). Such an addition is **deliberate**
  — a registry-tracked closed extension, not a runtime plug-in.
- **New `BlockSource` impl** *(NEW in N-I — extension point)*:
  closed seam; no second impl anticipated at present (`ChainDbBlockSource`
  covers all foreseeable cases — rollback replay always sources
  from the canonical ChainDb).
- **New `MaterializeError` / `CommitRollbackError` variant** *(NEW
  in N-I — extension point)*: closed sums; today 3 / 1 variants;
  e.g. the future persistent decoder cluster MAY add
  `MaterializeError::IndexCorrupt` for snapshot-bytes integrity
  failures.
- **New `RollbackContext` field** *(NEW in N-I — extension point)*:
  closed struct; today 2 fields; e.g. a future cluster MAY add
  `&dyn EvictionPolicy` once snapshot eviction is wired.
- **New `SnapshotCadence` field** *(NEW in N-I — extension point —
  WITH MANDATORY CLUSTER RATIFICATION)*: closed BLUE-structural
  struct with exactly 1 field at this HEAD; CI-defended. **Adding a
  new field = breaking the CI gate** + requires explicit cluster
  ratification (Tier-5 with anchored replay-derivable representation
  per DC-STORE-07). The cadence-as-runtime-data variant is the
  expected ratified shape.
- **New CI check**: additive. (N-I added two —
  `ci_check_rollback_materialize_closure.sh`,
  `ci_check_snapshot_cadence_purity.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol** / **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(N-E)*.
- **Live cardano-node N2N block-fetch acceptance / live N2N follow-
  mode admission** *(N-C / N-G / N-H)*: each reopens on operator
  availability.
- **Phase-4 cluster surface additions** (N-F): each cluster's wire
  surface gates additions via its own cluster doc.
- **Persistent ledger snapshot encoding — DC-CONS-21 closure**
  *(NEW in N-I — extension point flagged by N-I close)*: today
  `DC-CONS-21` ships `status = "declared"` with `open_obligation =
  persistent_ledger_snapshot_encoding_follow_on_cluster`; the follow-
  on cluster ships the canonical `encode_ledger_state` +
  `decode_ledger_state` pair and a `PersistentSnapshotStoreReader`
  impl of `SnapshotReader`. The `open_obligation` removal is the
  surface signal of that cluster's close. `materialize_rolled_back_state`
  is unchanged — drops the new `SnapshotReader` impl in.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-I added
four new BLUE submodules** (`ade_ledger::rollback::{traits, error,
materialize, commit}` under a new `ade_ledger::rollback` barrel),
**three new GREEN submodules inside `ade_runtime`** (`rollback::cadence`,
`rollback::in_memory_cache`, `rollback::chaindb_block_source`),
**one new RED submodule inside `ade_runtime`** (`rollback::snapshot_writer`,
under a new `ade_runtime::rollback` barrel), **two new CI gates**,
**four new registry rules** (one `declared`, three `enforced`), and
**flipped `DC-CONS-20` from `declared` to `enforced` with
`strengthened_in += PHASE4-N-I` and `open_obligation` removed**.
N-I added **no new crate**, **no new external ingress wire-format
frozen contract** (rollback is wholly internal authority — the
`RollBackward` peer-originated signal was already in the closed
`ReceiveEvent` taxonomy from N-H), **no new operator-action probe
binary** (rollback has no Tier-1 wire-format counterpart that
requires live cross-impl probe).

**N-I also strengthened one cross-color dependency edge**:

1. `ade_runtime → ade_ledger` (already added in N-C; strengthened
   in N-G + N-H; **further strengthened in N-I**) — the GREEN
   `rollback::{cadence, in_memory_cache, chaindb_block_source}` and
   RED `rollback::snapshot_writer` adapters import the new
   `ade_ledger::rollback::*` BLUE chokepoints + the `RollbackContext`
   struct seam + the extended `ChainDbWrite::rollback_to_slot` trait
   method. Same direction (RED/GREEN → BLUE); allowed. Passes
   `ci_check_dependency_boundary.sh`.

**The module-addition rule N-I sets for future rollback-side work:**

1. **A new rollback-side BLUE primitive attaches inside
   `ade_ledger::rollback::*`** (sibling of `traits`, `error`,
   `materialize`, `commit`). The module MUST be BLUE: no clock,
   no rand, no I/O, no `HashMap`, no `tokio`, no `async`. New
   canonical types MUST be closed sums or closed structs; no
   `#[non_exhaustive]`; no `String`-bearing variants.
2. **A new rollback-side authority chokepoint attaches inside
   `ade_ledger::rollback::materialize`.** Pure, total, deterministic.
   Composes existing BLUE chokepoints (`block_validity`,
   `apply_block_with_verdicts`, etc.) rather than re-implementing.
   **MUST NOT** return `(LedgerState, PraosChainDepState)` from a
   `pub fn` — that would break CN-STORE-07's single-authority
   discipline (CI-defended). If a new chokepoint legitimately needs
   the tuple, the cluster must justify breaking the single-authority
   invariant explicitly.
3. **A new `SnapshotReader` impl attaches inside the appropriate
   runtime crate** (e.g. `ade_runtime::rollback::persistent_snapshot_store`
   for the persistent decoder). The module MUST be a pure function
   over its inputs (the decode side may consult disk via the
   `SnapshotStore` trait but the result MUST be deterministic).
   Single production impl per snapshot backend.
4. **A new `BlockSource` impl attaches inside the appropriate
   runtime crate.** None anticipated at present — `ChainDbBlockSource`
   covers all foreseeable cases.
5. **A new closed `MaterializeError` / `CommitRollbackError` /
   `RollbackContext` variant or field attaches inside their respective
   enum / struct bodies.** Closed-sum / closed-struct extension;
   version-gated; no `#[non_exhaustive]`.
6. **A new `ChainDbWrite` trait method attaches as a closed-trait
   extension.** N-I added `rollback_to_slot` as the second method;
   future methods (e.g. `bulk_write_admitted` for batched
   admission) follow the same closure: signature-frozen at the
   cluster that introduces it, all impls extended atomically.
7. **A new snapshot cadence policy MUST NOT add fields to
   `SnapshotCadence`** without explicit cluster ratification —
   CI-defended at exactly one field. A future operator-tunable
   cadence cluster ratifies an anchored replay-derivable
   representation (per DC-STORE-07), not a runtime-mutable field.
8. **A new rollback-paths registry rule attaches as a derived `DC-*`
   / `CN-*` family entry** with `code_locus`, `ci_script`, `tests`,
   `cross_ref`. Bidirectional cross-refs to consumed rules
   (`CN-CONS-08`, `DC-CONS-13`, `DC-CONS-20`, `DC-CONS-22`,
   `CN-STORE-07`).

### Cross-cluster obligation pattern (carried — no N-I addition)

**N-I adds no new cross-cluster obligation** — rollback authority
is wholly internal and has no Tier-1 wire-format counterpart that
requires a live cross-impl probe. The N-H / N-G / N-C /
`blocked_until_operator_*_available` precedents stand unchanged.

### Operator-action evidence pattern (carried — no N-I addition)

**N-I adds no new operator-action probe binary** — the family
remains at five. Rollback is exercised by the end-to-end integration
test `crates/ade_runtime/tests/receive_rollback_integration.rs`
plus the existing receive-side live binary
(`live_block_follow_session`) which covers the surrounding admission
path.

### Cluster scope-edge pattern (carried — strengthened in N-I close)

**N-I carries the scope-edge pattern introduced by N-H** (DC-CONS-20
Path A admit-only edge) and applies it **one level deeper** to the
persistent-encoding boundary:

- The N-I scope edge (in-memory only) is NOT a structured failure
  variant on every event — it is a deliberate cluster carve-out
  recorded as a separable registry rule (`DC-CONS-21`) with
  `status = "declared"` and `open_obligation =
  persistent_ledger_snapshot_encoding_follow_on_cluster`. The
  in-memory implementation works end-to-end within a session;
  persistence across restarts is the follow-on cluster's deliverable.
- The pattern is binding: cluster carve-outs MUST be recorded as
  separable registry rules with explicit `open_obligation` naming
  the follow-on cluster's deliverable. Same discipline as N-H's
  rollback-half carve-out.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-I:** `ade_ledger::rollback::*` modules have no `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock (CI-defended); `materialize_rolled_back_state` is the SOLE `pub fn` returning `(LedgerState, PraosChainDepState)` in the rollback module tree (CI grep); `RollbackContext` field set is closed; `commit_rollback` is irreversible-first staged. | Other BLUE crates / submodules only. **N-I:** rollback materialize driver composes `block_validity` (B1) via the per-block fold — no direct dep on `ade_runtime`. The two narrow read-only traits (`SnapshotReader`, `BlockSource`) are the only consumer-facing seams (impls live in RED/GREEN crates). | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. **N-I:** no `*SigningKey` / `KesSecret` / `ColdSigningKey` types (carried). |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-I:** `cadence` is a pure decision function + a single-field BLUE-structural struct (CI-defended); `in_memory_cache::InMemorySnapshotCache` is the single production impl of `SnapshotReader` (BTreeMap-backed; no HashMap); `chaindb_block_source::ChainDbBlockSource` is the single production impl of `BlockSource` over any `ChainDb`. | BLUE crates + standard library + ecosystem crates. **N-I:** the GREEN rollback adapters live inside `ade_runtime` (RED crate) — color is per-module per the cluster TCB Color Map. | `ade_runtime` for `ade_testkit`; RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. **N-I:** `ade_runtime::rollback::snapshot_writer::maybe_capture_snapshot` is a pure-function hook — composes RED dispatch outcomes with the GREEN cadence + cache. RED-classified by location only. | Any BLUE / GREEN crate or submodule (one-way). **N-I strengthened the `ade_runtime → ade_ledger` edge** (RED/GREEN → BLUE via the rollback chokepoints + traits + `RollbackContext` struct + extended `ChainDbWrite::rollback_to_slot` trait method). | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for rollback-domain sub-modules, model the new CI gate
   on `ci_check_rollback_materialize_closure.sh` /
   `ci_check_snapshot_cadence_purity.sh` shape (single-authority
   grep + closure proof + state-isolation proof + BTreeMap-only proof
   + single-field-struct proof).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   rollback-domain authority rules, append `DC-CONS-2X` / `CN-STORE-0X`
   / `DC-STORE-0X` with bidirectional cross-ref to consumed rules.
7. **New operator-action probe binary:** (not applicable for the
   rollback domain — internal authority).
8. **Cross-cluster obligation:** (not applicable for the rollback
   domain at this cluster).
9. **Cluster scope-edge:** if the cluster deliberately scopes down a
   derived constraint, ship a separable registry rule with explicit
   `open_obligation` naming the follow-on cluster's deliverable. N-I
   sets the precedent at the persistent-encoding boundary
   (`DC-CONS-21`); N-H set the precedent at the rollback-half
   boundary (`DC-CONS-20` — now closed).
10. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-I — FULLY CLOSED at this HEAD** (mechanical close;
  Path A in-memory scope; persistent encoding deferred to follow-on
  per DC-CONS-21): code + CI gates + CN-STORE-07 + DC-CONS-22 +
  DC-STORE-07 (`enforced`) + DC-CONS-21 (`declared` with
  `persistent_ledger_snapshot_encoding_follow_on_cluster`) +
  DC-CONS-20 flipped to `enforced` (rollback-side closed; `open_obligation`
  removed) + 2 new CI scripts. No live-evidence obligation (internal
  authority).
- **PHASE4-N-H — FULLY CLOSED** (carried; `DC-CONS-20`
  `open_obligation` now removed by N-I). CE-N-H-6 live-evidence is
  `blocked_until_operator_peer_available`.
- **PHASE4-N-G — FULLY CLOSED** (carried). CE-N-G-8 live-evidence
  is `blocked_until_operator_peer_available`.
- **PHASE4-N-C — FULLY CLOSED** (carried). CE-N-C-8 live-evidence
  is `blocked_until_operator_stake_available`.
- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED** (carried).
- **PHASE4-N-E — FULLY CLOSED** (carried).
- **NEW HIGHEST-PRIORITY future cluster — Persistent ledger
  snapshot encoding** *(NEW HIGHEST-PRIORITY candidate flagged by
  N-I close — DC-CONS-21 closure)*: BLUE `encode_ledger_state` +
  `decode_ledger_state` chokepoint pair (~1500-2000 LoC field-walk
  mirroring `ade_ledger::fingerprint`) + a `PersistentSnapshotStoreReader`
  impl of `SnapshotReader` over `SnapshotStore` bytes. Encoded layout
  `[closed version tag][canonical-CBOR LedgerState bytes][fingerprint]`
  for decode-side integrity. Drops in to `materialize_rolled_back_state`
  unchanged. Surface for the next planner; do not invent invariants
  here.
- **NEW future cluster — Snapshot eviction policy** *(NEW candidate
  flagged by N-I close — OQ-5 declared non-goal)*: Tier-5
  operational concern; bounded-ring or stability-window policy.
  Must remain replay-deterministic.
- **NEW future cluster — Multi-peer fork choice** *(carried from
  N-H; now unblocked by N-I's rollback closure)*: Praos longest-chain
  across competing `PerPeerReceiveState[]`. Re-uses `RollbackContext`
  to roll back losing forks.
- **NEW future cluster — N2C local-chain-sync receive surface**
  *(carried from N-H)*: operator-side N2C clients consume Ade's
  chain via `LocalChainSyncMessage`.
- **Future cluster — `CE-N-H-6` live evidence re-open trigger**:
  reopens when a private cardano-node peer is provisioned (carried).
- **Future cluster — `CE-N-G-8` live evidence re-open trigger**:
  reopens when a private cardano-node peer is provisioned (carried).
- **Future cluster — `CE-N-C-8` live evidence re-open trigger**:
  reopens when testnet SPO stake is provisioned (carried).
- **Future node-binary cluster (`CE-NODE-N2C-LTX`)**: live N2C UDS
  server + N2N bulk-tx inbound listener (carried).
- **Tx-validity completeness follow-ups**: full `track_utxo=true`
  corpus; pre-Conway eras; the Conway block-body vkey-witness
  closure (carried).
- **PP OQ-1..OQ-4 follow-ups** (carried).
- **N-F (operator API)**: thin RED layer mapping a closed Query
  enum to gRPC/HTTP.

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`.
- No `SystemTime`, `Instant`, `std::time::*` clocks.
- No `rand::thread_rng`, `thread::spawn`.
- No `f32`, `f64`, floating-point arithmetic.
- No `std::fs`, `std::net`, `tokio`, `async fn`.
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist.
- No `#[cfg(feature = ...)]` semantic gating.
- No signing patterns in BLUE.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes only.
- No construction of `PreservedCbor` outside `ade_codec`.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs`.
- No `pallas_*` reference outside `ade_plutus`.
- **(N-A specific)** Carried.
- **(N-B specific)** Carried.
- **(B1 specific)** Carried.
- **(B2 specific)** Carried.
- **(B3 / B4 / B5 specific)** Carried.
- **(OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK)** Carried.
- **(N-E specific — closed BLUE chokepoint `mempool_ingress`)** Carried.
- **(PP specific — closed BLUE sub-grammar `decode_proposal_procedures`)** Carried.
- **(N-C-S1..S7 specific)** All carried.
- **(N-G-S1..S4 specific)** All carried.
- **(N-H-S1..S6 specific)** All carried.
- **(N-I-S1 specific — closed BLUE rollback traits + error sums)**
  `SnapshotReader` / `BlockSource` MUST be single-method, object-safe,
  read-only. `MaterializeError` / `CommitRollbackError` MUST be
  closed sums; no `#[non_exhaustive]`; no `String`. Round-trip-
  through-pattern-match tests confirm fourth-variant additions
  fail to compile.
- **(N-I-S2 specific — single materialize authority CN-STORE-07)**
  `materialize_rolled_back_state` MUST be the SOLE `pub fn` returning
  `(LedgerState, PraosChainDepState)` in `crates/ade_ledger/src/rollback/*.rs`
  (CI-defended by `ci_check_rollback_materialize_closure.sh` via
  single-authority grep). Production code in `rollback/materialize.rs`
  MUST NOT import `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock.
  Positive presence: `block_validity` call site MUST exist (single
  admission authority).
- **(N-I-S3 specific — atomic commit + trait extension)**
  `commit_rollback` MUST be irreversible-first staged
  (`chain_write.rollback_to_slot` first; subsequent field
  replacements infallible). On chain_write failure the state MUST
  be unchanged (proven by `commit_rollback_chain_write_failure_leaves_state_unchanged`).
  `ChainDbWrite::rollback_to_slot` MUST exist in the trait; all impls
  MUST be extended.
- **(N-I-S6 specific — receive reducer rollback wiring)**
  `RollbackContext` field set MUST be closed (no `#[non_exhaustive]`).
  `receive_apply` MUST accept `Option<&RollbackContext>` as a
  parameter; `None` MUST preserve the N-H legacy behavior
  (`Err(RollbackOutOfScope)`); `Some` MUST wire the rollback path
  through `materialize_rolled_back_state` + `commit_rollback`. The
  `RollForward` and `BlockDelivered` arms MUST be unchanged from
  N-H — only the `RollBackward` arm gains new behavior under
  `Some(ctx)`.

### GREEN (`ade_testkit` incl. `producer` + `receive_paths` + **`receive_rollback_integration` — NEW in N-I** corpora; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges; `ade_runtime::producer::{tick_assembler, broadcast_to_served, served_chain_lookups}`; `ade_runtime::receive::{events_to_state, in_memory_chain_write}`; **`ade_runtime::rollback::{cadence, in_memory_cache, chaindb_block_source}` — NEW in N-I-S4**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (carried bullets per prior revision)
- **(`ade_runtime::rollback::cadence`, NEW in N-I-S4 — DC-STORE-07)**
  Pure decision function `should_snapshot_after_block`; no
  `HashMap`/`HashSet`/wall-clock/`tokio`/`rand`. `SnapshotCadence`
  MUST have exactly 1 field (`every_n_blocks: u32`) — CI-defended.
  Operator-tunable runtime cadence is explicitly out of scope.
- **(`ade_runtime::rollback::in_memory_cache`, NEW in N-I-S4)**
  Single production impl of `SnapshotReader`. BTreeMap-backed
  canonical iteration (no HashMap). No eviction at this cluster
  (OQ-5).
- **(`ade_runtime::rollback::chaindb_block_source`, NEW in N-I-S4)**
  Single production impl of `BlockSource` over any `ChainDb`.
  `blocks_in_range(from_exclusive, to_inclusive)` translates to
  `ChainDb::iter_from_slot(from_exclusive + 1)` (saturating) +
  collect until slot > to_inclusive. Returns bytes byte-identical
  to the underlying store.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. five live-session probe binaries — N-B / N-E S6 / N-C S7 / N-G S7 / N-H S6), the RED-behavior `ade_ledger::consensus_input_extract` scan; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` (N-C-S1/S6); `ade_runtime::network::n2n_server` (N-G-S6); `ade_runtime::receive::orchestrator` (N-H-S4); **`ade_runtime::rollback::snapshot_writer` — NEW in N-I-S5**)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`, the `block_validity` / `tx_validity`
  composers, `mempool::ingress::mempool_ingress`, the producer
  authority chokepoints `producer::forge::forge_block` +
  `producer::self_accept::self_accept` (N-C), the served-chain
  authority chokepoint `producer::served_chain::served_chain_admit`
  (N-G), the receive authority chokepoint
  `receive::reducer::receive_apply` +
  `receive::admitted::admit_via_block_validity` (N-H), **OR the
  rollback authority chokepoints `rollback::materialize_rolled_back_state`
  + `rollback::commit_rollback` (N-I)**.
- No bypassing `ade_codec` to construct semantic types from raw bytes.
  **(N-C-strengthened)** Constructing `AcceptedBlock` outside
  `self_accept` is CI-forbidden. **(N-G-strengthened)** Constructing
  `ServedChainSnapshot` populated entries outside `served_chain_admit`
  is CI-forbidden; constructing `ServerReply` variants for
  client-agency wire messages is unrepresentable in the public API
  (CN-PROTO-06). **(N-H-strengthened)** Constructing `AdmittedBlock`
  outside `admit_via_block_validity` is CI-forbidden. Constructing
  `ReceiveEvent` for locally-originated chain-sync / block-fetch
  outputs is unrepresentable in the public API (CN-PROTO-07).
  **(N-I-strengthened)** Returning `(LedgerState, PraosChainDepState)`
  from any `pub fn` in `crates/ade_ledger/src/rollback/*.rs` other
  than `materialize_rolled_back_state` is CI-forbidden (CN-STORE-07
  single-authority grep). Constructing `SnapshotCadence` with more
  than one field is CI-forbidden (DC-STORE-07).
- (`ade_runtime` specifically) Existing `ade_runtime → ade_ledger`
  edge (added N-C; strengthened N-G + N-H) is **further strengthened
  in N-I** — the rollback adapters consume the new
  `ade_ledger::rollback::*` BLUE chokepoints + `RollbackContext`
  struct + extended `ChainDbWrite::rollback_to_slot` trait method.
  Pass `ci_check_dependency_boundary.sh`.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- (N-E live N2N operator-action session) Carried.
- (Deferred RED operator-action surfaces — CE-NODE-N2C-LTX) Carried.
- (`ade_core_interop`) Live-interop driver only; library tests
  `#[ignore]`-gated. **N-I added no new binary.**
- **(N-C-S1 / S6 specific — `ade_runtime::producer::{signing, keys,
  scheduler, broadcast}`)** All carried.
- **(N-G-S6 specific — `ade_runtime::network::n2n_server`)** Carried.
- **(N-H-S4 specific — `ade_runtime::receive::orchestrator`)** Carried.
  Key-boundary forbids imports from
  `ade_runtime::producer::{signing, broadcast, scheduler}`.
- **(N-I-S5 specific — `ade_runtime::rollback::snapshot_writer`)**
  Pure-function hook; classified RED by location only. Decision
  logic itself MUST NOT use clocks / async / `tokio` / `rand` /
  `HashMap` — it is a pure read of `(ReceiveEffect, ReceiveState,
  SnapshotCadence, &mut InMemorySnapshotCache)`. The hook MUST only
  capture on `ReceiveEffect::Admitted` (asserted by tests). The
  hook MUST be replay-deterministic (same input sequence → same
  capture set) — proven by `maybe_capture_snapshot_deterministic_over_admission_sequence`.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-I-strengthened:** no
  private-key bytes in rollback fixtures (the
  `receive_rollback_integration` test reuses the Conway-576 corpus —
  same redaction posture as N-H).
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-I:** rollback
  is internal authority (no Tier-1 wire-format counterpart); the
  mechanical end-to-end harness in
  `crates/ade_runtime/tests/receive_rollback_integration.rs` is a
  structural-agreement harness that proves the snapshot-is-cache
  property (DC-CONS-22 end-to-end). The live cross-impl claim of
  receive-side admission is the existing CE-N-H-6 obligation;
  rollback adds no separate live claim.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. **The N-I `DC-CONS-21` declaration is NOT
  a "we'll match it later" stub** — the in-memory rollback authority
  works end-to-end at this HEAD; the registry rule ships
  `status = "declared"` with `open_obligation =
  persistent_ledger_snapshot_encoding_follow_on_cluster` naming the
  follow-on cluster's deliverable. Same discipline as N-H's
  `DC-CONS-20` rollback-half carve-out (which N-I has now closed).
  Likewise the OQ-5 snapshot eviction carve-out is NOT a stub — the
  in-memory cache grows monotonically by design at this cluster;
  eviction is a follow-on operational concern surfaced as a named
  candidate seam.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; pending the regen,
  CODEMAP may pin pre-N-I HEAD `efe1fb9`. The new BLUE submodules
  (`ade_ledger::rollback::{traits, error, materialize, commit}`),
  the new GREEN submodules (`ade_runtime::rollback::{cadence,
  in_memory_cache, chaindb_block_source}`), and the new RED
  submodule (`ade_runtime::rollback::snapshot_writer`) are not yet
  in the prior CODEMAP. The next CODEMAP regen picks these up
  mechanically. CI count moves from 52 → 54.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-I added:**
  `DC-CONS-21` (`declared`, `open_obligation =
  persistent_ledger_snapshot_encoding_follow_on_cluster`); `DC-CONS-22`
  (`enforced`, `ci_script = ci/ci_check_rollback_materialize_closure.sh`);
  `CN-STORE-07` (`enforced`, `ci_script =
  ci/ci_check_rollback_materialize_closure.sh`); `DC-STORE-07`
  (`enforced`, `ci_script = ci/ci_check_snapshot_cadence_purity.sh`);
  flipped `DC-CONS-20` from `declared` to `enforced` with
  `strengthened_in += PHASE4-N-I` and `open_obligation` removed.
  Total: 202 → 206 entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Ledger snapshot + rollback invariants sketch:
  `docs/planning/ledger-snapshot-rollback-invariants.md` (the
  upstream sketch the cluster doc derives from).
- Receive-side bridge invariants sketch (N-H upstream):
  `docs/planning/receive-side-bridge-invariants.md`.
- Cluster N-D / N-A / N-B / N-H / B1 / B2 / B3 / B4 / B5 /
  OQ5-CREDENTIAL-FIDELITY / COMMITTEE-CRED-FIDELITY /
  DREP-VOTE-FIDELITY / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G: all closed;
  cluster docs carried.
- **Cluster PHASE4-N-I (CLOSED + archived at this HEAD; mechanical
  half; Path A in-memory scope)**: the cluster doc + slices
  `cluster.md, N-I-S{1..6}.md` at
  `docs/clusters/completed/PHASE4-N-I/`. WIRES AND CLOSES the
  receive-side rollback authority end-to-end (Path A in-memory
  scope): BLUE `SnapshotReader` + `BlockSource` traits +
  `MaterializeError` + `CommitRollbackError` (S1); BLUE
  `materialize_rolled_back_state` SOLE chokepoint composing
  `block_validity` (S2); BLUE `commit_rollback` atomic-staged
  helper + `ChainDbWrite::rollback_to_slot` trait extension (S3);
  GREEN `SnapshotCadence` + `should_snapshot_after_block` +
  `InMemorySnapshotCache` + `ChainDbBlockSource` (S4); RED
  `maybe_capture_snapshot` hook (S5); BLUE-edit `receive_apply`
  signature evolution + `RollbackContext` + end-to-end integration
  test (S6 — closes DC-CONS-20). Added two CI scripts (count 52 →
  54); added four derived registry rules (total 202 → 206); flipped
  DC-CONS-20 to `enforced` with `strengthened_in += PHASE4-N-I` and
  `open_obligation` removed. **DC-CONS-21 carries new `open_obligation
  = persistent_ledger_snapshot_encoding_follow_on_cluster`** —
  highest-priority candidate seam surfaced for the next planner.
  Five operator-action probe binaries remain in the family (no N-I
  addition).
- **Future obligation: `DC-CONS-21` persistent encoding closure** —
  full persistent snapshot encoder cluster: BLUE canonical
  `encode_ledger_state` + `decode_ledger_state` chokepoint pair +
  a `PersistentSnapshotStoreReader` impl of `SnapshotReader` over
  `SnapshotStore` bytes. **Highest-priority next-cluster candidate
  seam.**
- **Future obligation: snapshot eviction policy cluster** — Tier-5
  operational concern; bounded-ring or stability-window policy;
  named candidate seam.
- **Future obligation: `CE-N-H-6`** — carried.
- **Future obligation: `CE-N-G-8`** — carried.
- **Future obligation: `CE-N-C-8`** — carried.
- **Future obligation: `CE-NODE-N2C-LTX`** — carried from N-E.
- **Future seam candidates (flagged by N-I close)**: persistent
  ledger snapshot encoding cluster (highest priority — DC-CONS-21
  closure); snapshot eviction policy cluster; multi-peer fork
  choice cluster (now unblocked by N-I's rollback closure); N2C
  local-chain-sync receive surface cluster.
