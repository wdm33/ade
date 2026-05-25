# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **52 CI checks** at HEAD (`efe1fb9`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml` — **202 entries**) for
> rule IDs; reads the Phase 4 cluster plan
> (`docs/active/phase_4_cluster_plan.md`), the closed N-D / N-A / N-B /
> N-E / N-C / N-G / B1 / B2 / B3 / B4 / B5 cluster docs, the OQ5 /
> COMMITTEE / DREP / ENACTMENT-COMMITTEE-FIDELITY /
> ENACTMENT-COMMITTEE-WRITEBACK / PROPOSAL-PROCEDURES-DECODE cluster
> docs, and the **just-closed PHASE4-N-H cluster doc + S1..S6 slice
> docs** (`docs/clusters/completed/PHASE4-N-H/cluster.md` +
> `N-H-S{1..6}.md` + `CE-N-H-6_PROCEDURE.md`).
>
> **This is the PHASE4-N-H FULL CLOSE refresh (HEAD `efe1fb9`).** The
> previous SEAMS (HEAD `a280954`) pinned the PHASE4-N-G full-close
> state. Six N-H slices have landed between that revision and this one
> and close the **receive-side header→body bridge** (admit-only,
> Path A scope) — the seam previously flagged as the most load-bearing
> candidate at N-G close ("the natural counterpart to N-G's
> send-direction closure"):
>
> 1. **N-H-S1 (commit `b019ee3`)** ships the BLUE receive-side
>    admission primitives in a new sub-tree
>    `ade_ledger::receive::{admitted, chain_write, events,
>    pending_header_cache}`: the **`AdmittedBlock` private-constructor
>    token** (sole constructor `admit_via_block_validity`; distinct
>    from `AcceptedBlock`), the closed `ReceiveEvent` (3 variants) /
>    `ReceiveEffect` (4 variants) / `ReceiveError` (4 variants) sums,
>    `PendingHeaderCache` (`BTreeMap`-backed), and the narrow
>    `ChainDbWrite` trait (single-method, takes `AdmittedBlock` by
>    value). Registry rule `CN-PROTO-07` introduced (closed receive
>    event taxonomy; no constructor for locally-originated chain-sync
>    / block-fetch outputs). CI gate
>    `ci/ci_check_admitted_block_closure.sh` introduced.
> 2. **N-H-S2 (commit `0ecf22f`)** ships the BLUE
>    **`receive_apply` reducer** + `receive_apply_sequence` driver in
>    `ade_ledger::receive::reducer`. Pure, total, deterministic; one
>    `ReceiveEvent` per call; staged-then-committed shape (on error,
>    state is unchanged). `RollForward` caches only;
>    `BlockDelivered` decodes + cross-checks the cached header + runs
>    `admit_via_block_validity` + persists via `ChainDbWrite`;
>    `RollBackward` returns `Err(ReceiveError::RollbackOutOfScope)`
>    (Path A scope edge). Registry rules `CN-CONS-08` and
>    `DC-CONS-19` introduced. CI gate
>    `ci/ci_check_receive_reducer_closure.sh` introduced.
> 3. **N-H-S3 (commit `c584691`)** ships the GREEN adapters
>    `ade_runtime::receive::{events_to_state, in_memory_chain_write}`:
>    `lift_chain_sync_signal` / `lift_block_fetch_event` (pure
>    pass-through translators from N-A signals/events into
>    `ReceiveEvent`), plus the production `ChainDbWriter<'a, D>`
>    impl (GREEN adapter over any `ChainDb`; the wrapper decodes once
>    via `decode_block` to extract `(slot, hash)` then calls
>    `ChainDb::put_block`). Replay corpus driver lives in
>    `crates/ade_runtime/tests/receive_session_transcript_replay.rs`.
>    Registry rule `DC-PROTO-09` introduced. CI gate
>    `ci/ci_check_receive_replay_purity.sh` introduced;
>    `ci/ci_check_no_private_keys_in_corpus.sh` extended to the new
>    `receive_paths` fixture root.
> 4. **N-H-S4 (commit `1d06089`)** ships the RED per-peer N2N receive
>    orchestrator `ade_runtime::receive::orchestrator::{PerPeerReceiveState,
>    ReceiveDispatchError, dispatch_chain_sync_inbound,
>    dispatch_block_fetch_inbound}`. Pure state-machine driver — no
>    socket I/O; per-peer state fully independent; shared `ChainDb` is
>    the only cross-peer coordination point. Key-boundary preserved
>    (cannot import from `ade_runtime::producer::{signing, broadcast,
>    scheduler}`, defended by
>    `ci/ci_check_receive_orchestrator_no_producer_dep.sh`).
>    Multi-peer determinism test in
>    `crates/ade_runtime/tests/receive_two_peer_independence.rs`.
>    `DC-PROTO-06.strengthened_in += PHASE4-N-H`.
> 5. **N-H-S5 (commit `3973261`)** ships the mechanical cross-impl
>    adapter in `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs`
>    (every Conway-576 corpus block drives RollForward + BlockDelivered
>    through the full receive pipeline; ChainDb tip + admitted bytes +
>    ledger fingerprint must agree with the corpus reference).
> 6. **N-H-S6 (commit `efe1fb9`)** ships the **fifth**
>    operator-action probe binary `ade_core_interop::bin::live_block_follow_session`,
>    plus the procedure doc
>    `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md`.
>    Registry rule `RO-LIVE-02` introduced at `status = "partial"`
>    with `open_obligation = blocked_until_operator_peer_available`.
>    CI gate `ci/ci_check_receive_paths_corpus_present.sh` introduced.
>
> **THE KEY FULL-CLOSE DELTAS.** The prior SEAMS revision flagged the
> receive-side header→body bridge as the most load-bearing remaining
> candidate seam. PHASE4-N-H closes it end to end, in **admit-only
> Path A scope**. Two §1 surface rows flip from "candidate" to
> "wired & closed":
>
> - **Receive-side header→body bridge: peer-originated header +
>   body bytes → `AdmittedBlock` → `ChainDb`** → wired via
>   `receive_apply` consuming `ReceiveEvent` from `events_to_state`,
>   producing `AdmittedBlock` via `admit_via_block_validity`,
>   persisting via `ChainDbWrite`.
> - **Per-peer N2N receive orchestrator** → wired via
>   `PerPeerReceiveState` + `dispatch_*_inbound` consuming N-A wire
>   frames and driving the BLUE reducer through the GREEN adapters.
>
> Counts at this refresh: **+5 CI scripts** (47 → 52:
> `ci_check_admitted_block_closure.sh`,
> `ci_check_receive_reducer_closure.sh`,
> `ci_check_receive_replay_purity.sh`,
> `ci_check_receive_orchestrator_no_producer_dep.sh`,
> `ci_check_receive_paths_corpus_present.sh`); **+6 registry rules**
> introduced (`CN-CONS-08`, `DC-CONS-19`, `DC-CONS-20`, `DC-PROTO-09`,
> `CN-PROTO-07`, `RO-LIVE-02`); **6 carried rules strengthened**
> (`T-DET-01`, `T-ENC-01`, `DC-CONS-13`, `DC-CONS-16`, `CN-CONS-07`,
> `DC-PROTO-06` all gain `strengthened_in += PHASE4-N-H`); **+5 new
> BLUE submodules** (`ade_ledger::receive::{admitted, chain_write,
> events, pending_header_cache, reducer}` under a new
> `ade_ledger::receive` barrel); **+3 new GREEN/RED submodules**
> (`ade_runtime::receive::{events_to_state, in_memory_chain_write,
> orchestrator}` under a new `ade_runtime::receive` barrel —
> `events_to_state` and `in_memory_chain_write` GREEN; `orchestrator`
> RED); **+1 new operator-action probe binary**
> (`live_block_follow_session` — fifth in the family alongside
> `live_consensus_session` (N-B), `live_tx_submission_session` (N-E),
> `live_block_production_session` (N-C), and `live_block_fetch_session`
> (N-G)); **+1 new live-evidence procedure doc**
> (`CE-N-H-6_PROCEDURE.md`); **0 new operator-action live-evidence
> log artifacts at this HEAD** — CE-N-H-6 is recorded
> `blocked_until_operator_peer_available` per `RO-LIVE-02`
> `open_obligation`. Total invariant registry: **202 entries**
> (196 → 202). `DC-CONS-20` (Path A scope edge — rollback authority)
> ships `status = "declared"` with
> `open_obligation = rollback_side_blocked_until_ledger_snapshot_cluster`:
> the closure of its rollback half is **the** explicit candidate seam
> for the next planner.

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-H is fully closed at this HEAD.** The receive-side
header→body bridge — i.e. the path by which an externally-arriving
header + body, delivered through N-A chain-sync RollForward and
block-fetch BlockDelivered events, is admitted into Ade's ChainDb +
LedgerState + PraosChainDepState via `block_validity` — is wired and
CI-defended end to end. Scope is **Path A: admit-only**. `RollBackward`
is a structured `Err(ReceiveError::RollbackOutOfScope)`; the rollback
half of `DC-CONS-20` is explicitly carried as the next planner's
candidate seam. The live cross-impl claim (CE-N-H-6) is
`blocked_until_operator_peer_available` per `RO-LIVE-02`
`open_obligation`.

**PHASE4-N-G remains fully closed** (carried). **PHASE4-N-C remains
fully closed** (carried). **PHASE4-N-E remains fully closed**
(carried). **PROPOSAL-PROCEDURES-DECODE remains fully closed**
(carried). **PHASE4-B3..B5, OQ5 / COMMITTEE / DREP /
ENACTMENT-COMMITTEE-WRITEBACK** all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are **eight** fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, chain-selector stream inputs, the
> N-E wire-level mempool ingress, **and — newly closed at this HEAD —
> the receive-side N2N peer ingress** for chain-sync RollForward +
> block-fetch BlockDelivered), plus the producer-side server-role
> ingress closed at N-G. All internal composition roots are unchanged
> from N-G close (`block_validity` / `tx_validity` / `mempool_ingress`
> / `forge_block` / `self_accept` / `served_chain_admit`).

### Surface: Receive-side N2N peer ingress (NEW in N-H-S1..S6 — peer→BLUE→ChainDb seam)

```
Surface: A peer-originated chain-sync ForkChoiceSignal
         (RollForward { header_bytes, tip }
         | RollBackward { point, tip }
         | Intersected | NoIntersection)
         OR a peer-originated block-fetch BatchDeliveryEvent
         (BatchStarted | BlockDelivered { block_bytes }
         | NoBlocks | BatchCompleted)
         delivered by a real cardano-node peer over N2N mux
Reduces to: ReceiveEffect — closed 4-variant sum
            { Admitted { slot, hash } | Cached { slot, hash }
            | RolledBack { to_slot }  (* unreachable in Path A *)
            | NoOp { reason: HeaderAlreadyCached } }
            — OR ReceiveError — closed 4-variant sum
            { HeaderBodyMismatch | Validity(BlockValidityError)
            | RollbackOutOfScope { target_point }
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
     pass-through translator into ReceiveEvent. Non-state-changing
     variants (BatchStarted / BatchCompleted / NoBlocks / Intersected
     / NoIntersection) return None and are filtered before the BLUE
     call.
  6. BLUE reducer — receive_apply(state, event, chain_write,
     era_schedule, ledger_view):
       - RollForward: pending_header_cache.insert((slot,hash) ->
         header_bytes); MUTATES ONLY state.pending_headers
         (Invariant I-6 — ledger / chain_dep / chain_write untouched).
       - BlockDelivered: decode_block(block_bytes) -> (slot,
         block_hash); pending_headers.get((slot, block_hash))
         cross-check; admit_via_block_validity(...) -> AdmittedBlock
         + new (ledger, chain_dep); chain_write.write_admitted(...);
         on success commit new sub-states + evict consumed header.
         Staged-then-committed shape: any failure leaves state
         unchanged.
       - RollBackward: Err(ReceiveError::RollbackOutOfScope { ... })
         (Path A scope edge).
  7. GREEN ChainDb write (ade_runtime::receive::in_memory_chain_write
     — ChainDbWriter<'a, D>): decode_block once to extract (slot,
     hash) -> ChainDb::put_block(StoredBlock { slot, hash, bytes }).
     Maps ChainDbError into ChainWriteError variants.
Cross-surface state sharing: per-peer state is fully independent
  (one PerPeerReceiveState per session: ReceiveState (ledger,
  chain_dep, pending_headers) + chain_sync_version +
  block_fetch_version). The ONLY cross-peer shared state is the
  single shared ChainDb that the orchestrator writes through; both
  InMemoryChainDb and PersistentChainDb are idempotent on byte-
  identity at the same (slot, hash) key, so two peers receiving the
  same block both succeed. Determinism property: per-session
  ReceiveEvent transcript is invariant under interleaving of other
  peers' events (N-H S4 two-peer independence test).
```

**Rule.** `receive_apply` (with `receive_apply_sequence` as its
deterministic driver) is the **single receive-side composition root**
into `LedgerState` + `PraosChainDepState` + `ChainDb` over peer-
supplied bytes. The `ReceiveEvent` taxonomy is the **closed canonical
input** — three variants only (`RollForward`, `RollBackward`,
`BlockDelivered`); **no constructor exists for locally-originated
chain-sync or block-fetch messages** the orchestrator might send
(client requests like `RequestNext`, `RequestRange`, `FindIntersect`,
`Done`, `ClientDone`). That impossibility is the `CN-PROTO-07`
closure — the receive side admits *peer-originated* signals only;
locally-originated outputs are the orchestrator's concern, not the
reducer's. **New work** that adds a receive-side feature attaches by
extending the `ReceiveEffect` / `ReceiveError` arms inside the
reducer or by adding a closed `ReceiveEvent` variant (closed-sum
extension; version-gated; surface ratified at cluster entry) — not by
exposing a parallel admission path, not by bypassing
`admit_via_block_validity`, not by passing raw bytes through
`ChainDbWrite`. **Path A scope edge (DC-CONS-20):**
`ReceiveEvent::RollBackward` returns
`Err(ReceiveError::RollbackOutOfScope { target_point })` —
deliberately fail-closed. The follow-on rollback cluster (ledger
snapshot encode/decode + replay-forward driver) closes the rollback
half; it is **a candidate seam surfaced for the next planner** (see
below).

### Surface: Producer-side chain-sync server-role ingress (wired in N-G; carried unchanged)

Carried. **N-H note:** the producer-side server-role and the
receive-side admission are deliberately **two separate composition
roots**, joined only by the upstream `block_validity` chokepoint they
both compose. There is no cross-call between `producer_chain_sync_serve`
and `receive_apply`; the producer serves `AcceptedBlock`-derived
bytes, the receive bridge admits peer-originated bytes via
`AdmittedBlock`. The two admission tokens are mechanically distinct
(no constructor on either side accepts the other's bytes — Invariant
¬P-6 from the receive-side sketch).

### Surface: Producer-side block-fetch server-role ingress (wired in N-G; carried unchanged)

Carried.

### Surface: Forge-block transition (carried unchanged from N-C)

Carried.

### Surface: Self-accept broadcast gate (carried unchanged from N-C; CN-CONS-07 strengthened in N-H across the receive seam)

Carried. **N-H strengthening:** `AcceptedBlock` remains the gate for
producer-side broadcast + serve admission, but it now has a **mirror**
on the receive side: `AdmittedBlock` is the gate for ChainDb + ledger
admission of peer-originated bytes. Both tokens have private
constructors that return only when their respective authoritative
chokepoint returns `Valid`. End-to-end (producer + receive):
`AcceptedBlock` gates everything broadcast/served outbound;
`AdmittedBlock` gates everything stored/applied inbound. `CN-CONS-07`
strengthening (the broadcast gate's mirror via `AdmittedBlock`)
recorded in `CN-CONS-07.strengthened_in += PHASE4-N-H`.

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

### Surface: Full block validity (composition root — wired in B1; consumed unchanged by N-H `admit_via_block_validity`)

Carried. **N-H usage:** the receive-side admission token
(`AdmittedBlock`) constructor (`admit_via_block_validity`) is a thin
wrapper around `block_validity` that returns
`Ok(AdmittedOutcome { admitted, ledger, chain_dep })` exactly when
the verdict is `BlockValidityVerdict::Valid`. The single block-
admission gate is unchanged; `AdmittedBlock` is its receive-side
symmetry to `AcceptedBlock`'s producer-side. The B1 composition
contract gains a second public consumer beyond `self_accept` (N-C)
and the validator's direct callers — `DC-CONS-13.strengthened_in +=
PHASE4-N-H` (symmetric receive closure: admit = `Valid` only).

### Surface: Block bytes, Plutus script bytes, Snapshot bytes, Consensus-input extraction, Ouroboros mux frames, Genesis JSON bundles, Chain-selector stream inputs (carried)

All seven external ingress surfaces are unchanged at this HEAD.
**N-H note (mux frames):** with both halves of the N2N mini-protocols
now wired — receive-side (N-H) and send-side (N-G) — the producer +
follower duality is structurally complete for chain-sync + block-fetch.
The only remaining N2N mini-protocol halves that are unwired today
are tx-submission2 inbound (N-E's mechanical half captured CE-N-E-6
on the outbound-client probe; full inbound bulk-tx listener is the
deferred `CE-NODE-N2C-LTX`) and the operator-facing protocols (LSQ,
LocalTxMonitor) which remain N-F candidate seams.

### Candidates — surfaces not yet wired (rollback authority, full fork choice, N2C surfaces, B+ residuals, PP open obligations)

The following surfaces are named in the Phase 4 plan / B+ planning /
the N-H Path A scope edge / the PP open-obligation set but have no
source today. They are listed so future slice docs can attach without
reinventing the reduction step. **Each is a candidate seam pending
confirmation at cluster entry.**

- **N-H-S1..S6 WIRED AND CLOSED the prior revision's "receive-side
  header→body bridge" candidate** — removed (now `receive_apply` +
  `events_to_state` + `dispatch_*_inbound` + `ChainDbWriter`).
- **NEW CANDIDATE (flagged by N-H close — `DC-CONS-20` rollback half
  + N-H OQ-1 Path A scope edge): full rollback authority.** With
  receive-side **admission** closed, the natural next seam is the
  receive-side **rollback** — a ledger-state snapshot system (encode
  + decode + restore) plus a replay-forward driver, both gated by the
  same `block_validity` chokepoint the admit half goes through. The
  cluster would convert `ReceiveEvent::RollBackward` from
  `Err(RollbackOutOfScope)` to a real transition that:
  (a) selects a snapshot at-or-before `target_point`,
  (b) restores `(LedgerState, PraosChainDepState, ChainDb tip)`,
  (c) replays forward across the kept blocks, and
  (d) re-emits a `ReceiveEffect::RolledBack { to_slot }`.
  `RollbackSnapshot` ring already exists in
  `ade_runtime::consensus::chain_selector` (bounded ≤ 2160) and could
  seed the snapshot store. **`DC-CONS-20.open_obligation =
  rollback_side_blocked_until_ledger_snapshot_cluster`.** **This is
  the highest-priority candidate seam for the next cluster planner**
  — surface it; do not invent invariants for it here.
- **NEW CANDIDATE (flagged by N-H close — OQ-4 lock): multi-peer
  fork choice (Praos longest-chain selection across competing peers).**
  Today the receive bridge is single-source follow: each peer is
  applied independently against a shared ChainDb; whichever block
  arrives first under a `(slot, hash)` key wins by ChainDb's
  byte-identity idempotency. Praos longest-chain across competing
  forks is a separate authority — it requires a fork-choice rule
  consumer of `(PerPeerReceiveState[])` that materializes a candidate
  set, runs `chain_selector::select_best_chain`, and either commits
  the chosen fork (re-using the rollback cluster's snapshot/replay)
  or rejects. Surface for the next planner.
- **NEW CANDIDATE (flagged by N-H close): N2C local-chain-sync
  receive surface.** N-H closes N2N receive; the local-N2C
  counterpart — operator clients (`db-sync`, wallets, explorers)
  consuming a chain-sync stream from Ade rather than from cardano-node
  — is a separate seam. The N2C codec already exists
  (`ade_network::n2c::local_chain_sync`); what is missing is the
  server-side reducer (closed `LocalChainSyncServerStep`, an N2C-flavor
  `ServerReply<LocalChainSyncMessage>`, and the lookup trait over
  the persisted ChainDb tip). Surface for the next planner.
- **N-G receive-side counterpart fully closed (N-H)** — removed.
- **CE-N-G-8 / CE-N-C-8 live-evidence — still
  `blocked_until_operator_*_available`** (carried).
- **PROPOSAL-PROCEDURES-DECODE remains closed** (carried). The four
  PP open obligations remain separable candidate seams (carried).
- **PHASE4-N-E remains closed** (carried).

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-H** *(FULLY CLOSED at this HEAD — mechanical close; live half blocked_until_operator_peer_available)* | **Receive-side N2N peer ingress: peer chain-sync + block-fetch events → `AdmittedBlock` → ChainDb + LedgerState + PraosChainDepState** | per-peer `(PerPeerReceiveState, shared ChainDb)`; closed `ReceiveEvent` / `ReceiveEffect` / `ReceiveError` sums; `AdmittedBlock` as the single admission token | **DONE:** `ade_ledger::receive::{admitted::{AdmittedBlock, AdmittedOutcome, admit_via_block_validity}, chain_write::{ChainDbWrite, ChainWriteError, ChainWriteErrorKind}, events::{ReceiveEvent, ReceiveEffect, ReceiveError, NoOpReason, TargetPoint, TipPoint}, pending_header_cache::{PendingHeaderCache, PendingHeaderCacheError}, reducer::{ReceiveState, receive_apply, receive_apply_sequence}}` (BLUE); `ade_runtime::receive::{events_to_state::{lift_chain_sync_signal, lift_block_fetch_event}, in_memory_chain_write::ChainDbWriter, orchestrator::{PerPeerReceiveState, ReceiveDispatchError, dispatch_chain_sync_inbound, dispatch_block_fetch_inbound}}` (GREEN + RED). CI gates `ci_check_admitted_block_closure.sh`, `ci_check_receive_reducer_closure.sh`, `ci_check_receive_replay_purity.sh`, `ci_check_receive_orchestrator_no_producer_dep.sh`, `ci_check_receive_paths_corpus_present.sh`. Registry rules `CN-PROTO-07`, `CN-CONS-08`, `DC-CONS-19`, `DC-PROTO-09` (`enforced`); `DC-CONS-20` (`declared` with `rollback_side_blocked_until_ledger_snapshot_cluster`); `RO-LIVE-02` (`partial` with `blocked_until_operator_peer_available`); strengthens `T-DET-01`, `T-ENC-01`, `DC-CONS-13`, `DC-CONS-16`, `CN-CONS-07`, `DC-PROTO-06`. Tests: named tests across S1..S6 plus replay corpus + multi-peer independence + cross-impl pipeline drive. | **wired & closed in PHASE4-N-H (mechanical half + structural cross-impl); live-peer cross-impl awaiting operator-supplied Haskell peer; rollback half deliberately Path A out-of-scope (DC-CONS-20)** |
| **CE-N-H-6 (cross-cluster obligation introduced in N-H S6; operator-action live evidence)** | **Live N2N follow-mode admission: a real cardano-node peer streams RollForward + BlockDelivered across a follow window; Ade ChainDb tip matches the peer's announced tip at every step** | The live cross-impl claim — same operator-action evidence pattern as CE-N-B-6 / CE-N-E-6 / CE-N-C-8 / CE-N-G-8 | The future evidence-capture pass via `live_block_follow_session --connect` against a private cardano-node peer; procedure at `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md`; output `CE-N-H-LIVE_<date>.log`. | **deferred operator-action obligation — `blocked_until_operator_peer_available` per `RO-LIVE-02.open_obligation`** |
| **NEW CANDIDATE — Full rollback authority** *(flagged by the N-H close — N-H OQ-1 Path A scope edge; `DC-CONS-20.open_obligation = rollback_side_blocked_until_ledger_snapshot_cluster`)* | **`ReceiveEvent::RollBackward` returning `Ok(ReceiveEffect::RolledBack { to_slot })` instead of `Err(RollbackOutOfScope)`** | A ledger-state snapshot store (encode + decode of `LedgerState` + `PraosChainDepState`) plus a replay-forward driver consuming `ChainDb::iter_from_slot` and re-running `block_validity` across the kept blocks | Extend `ade_ledger::receive::reducer` to dispatch `RollBackward` into a new BLUE chokepoint `rollback_apply(state, target_point, snapshots, chain_db) -> Result<(ReceiveState, ReceiveEffect), ReceiveError>`. No new external surface — the candidate seam is purely internal authority. | **candidate (next-cluster seam — HIGHEST PRIORITY for next planner; surface; do not invent invariants here)** |
| **NEW CANDIDATE — Multi-peer fork choice (Praos longest-chain across competing peers)** *(flagged by the N-H close — N-H OQ-4 lock)* | **Per-peer `ReceiveState[]` resolution to a single canonical chain** | A fork-choice consumer of `(PerPeerReceiveState[])` returning the canonical-chain `BlockHash` | `ade_runtime::consensus::chain_selector::select_best_chain` (existing GREEN) consumed by a new RED multi-peer coordinator. Requires the rollback cluster first (commits to the chosen fork mean rolling back from the loser). | **candidate (next-cluster seam; surface; sequenced after rollback cluster)** |
| **NEW CANDIDATE — N2C local-chain-sync receive surface** *(flagged by the N-H close)* | **Operator-side N2C clients consume Ade's chain via `LocalChainSyncMessage` requests** | per-client `(PerClientLocalChainSyncState, shared ChainDb)`; closed `ServerReply<LocalChainSyncMessage>` wrapper | Sibling of `producer_chain_sync_serve` (N-G) over the local-chain-sync N2C codec. Reuses `ServedHeaderLookup`-style trait, this time over the persisted ChainDb tip. | **candidate (next-cluster seam; surface)** |
| **CE-N-G-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade serving, cardano-node consuming)** | The live cross-impl claim — carried from N-G | Carried. | **carried (`blocked_until_operator_peer_available`)** |
| **CE-N-C-8 (cross-cluster obligation carried)** | **Live N2N block-fetch acceptance (Ade forging, cardano-node consuming as next chain head)** | Carried. | Carried. | **carried (`blocked_until_operator_stake_available`)** |
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

**At this HEAD two live-evidence logs are committed**, three
cross-cluster obligations remain `blocked_until_operator_*_available`,
and one cross-cluster obligation is carried from N-E.

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried from N-B close) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried from N-E close) | Outbound-client probe against a real preprod N2N relay | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` in the future node-binary cluster | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS | RED operator action (deferred) |
| `docs/clusters/completed/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | (pending) `CE-N-C-LIVE_<date>.log` | **`blocked_until_operator_stake_available`** (carried) | Cardano-node accepts an Ade-forged block as the next chain head | RED operator action |
| `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` | (pending) `CE-N-G-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (carried) | A real cardano-node peer issuing `RequestRange` accepts Ade-served bytes | RED operator action |
| **`docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` (NEW in N-H-S6)** | **(pending)** `docs/clusters/completed/PHASE4-N-H/CE-N-H-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (per `RO-LIVE-02.open_obligation`) | An Ade follower fed RollForward + BlockDelivered from a real cardano-node peer over a captured follow window produces a ChainDb tip equal to the peer's announced tip at every step. Live cross-impl claim (the bytes-shape claim is mechanically closed by `receive_pipeline_corpus_drive`). | RED operator action |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
At this HEAD there are **five** such binaries:

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 (live chain-sync follow-mode tip agreement) | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 (live N2N tx-submission2 outbound-client probe) | captured |
| `live_block_production_session` (PHASE4-N-C S7) | N-C S7 | CE-N-C-8 (live N2N block-fetch acceptance by cardano-node — producer-side forge) | blocked_until_operator_stake_available |
| `live_block_fetch_session` (PHASE4-N-G S7) | N-G S7 | CE-N-G-8 (live N2N server-role block-fetch served by Ade to cardano-node) | blocked_until_operator_peer_available |
| **`live_block_follow_session` (PHASE4-N-H S6) — NEW** | N-H S6 | CE-N-H-6 (live N2N receive-side follow-mode admission of cardano-node-served blocks into Ade's ChainDb) | **blocked_until_operator_peer_available** |

**Pattern.** Hermetic default mode (readiness probe that runs in CI
without network access — gated `#[ignore]`); plus a `--connect <peer>`
live pass that the operator runs against a real cardano-node peer.
The binary's evidence log is committed alongside the `_PROCEDURE.md`
in the cluster directory. **N-H is the fifth instance of the family
and the second receive-direction binary** (N-B was follow-mode read
but stopped at chain-sync; N-H follows through to block-fetch +
admit into ChainDb). It uses the **same `blocked_until_operator_peer_available`
closure-mode variant** as N-G (peer-blocker, not stake-blocker — no
SPO stake is required; the test is receive-side, Ade consumes bytes
the peer serves).

**These are evidence-log patterns, not BLUE seams.**

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are
**CE-N-C-8** (live cardano-node forge acceptance), **CE-N-G-8** (live
cardano-node block-fetch acceptance — Ade-serving counterpart),
**CE-N-H-6** (live cardano-node follow-mode admission — Ade-receiving
counterpart), **full rollback authority** (closure of the
`DC-CONS-20` rollback half — Path A scope edge resolution),
**CE-NODE-N2C-LTX** (the deferred live N2C UDS server + N2N bulk-tx
inbound listener), and the four **PROPOSAL-PROCEDURES-DECODE open
obligations**.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **seventeen** authoritative domains. **PHASE4-N-H added one
new domain — receive-side admission authority** — a new BLUE
composition root (`receive_apply`) producing a closed BLUE token
(`AdmittedBlock`) via the existing B1 chokepoint
(`admit_via_block_validity` wraps `block_validity`), with a narrow
BLUE write-trait (`ChainDbWrite`) consumed through a GREEN adapter
(`ChainDbWriter`), driven by a RED per-peer session driver
(`orchestrator`) over GREEN signal-lift adapters (`events_to_state`).
Prior cluster narratives are preserved unchanged below.

### Receive-side admission authority (NEW in PHASE4-N-H)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **BLUE admission-token authority (S1)** | `ade_ledger::receive::admitted::{AdmittedBlock, AdmittedOutcome, admit_via_block_validity}` | BLUE | The **single canonical** receive-side admission token. Sole public constructor: `admit_via_block_validity(block_bytes, &ledger, &chain_dep, &era_schedule, &dyn LedgerView) -> Result<AdmittedOutcome, BlockValidityError>` — returns `Ok` only when `block_validity` yields `BlockValidityVerdict::Valid`. The inner `bytes: Vec<u8>` field is private; the tuple-struct constructor is module-private; no public path exists from raw bytes to an `AdmittedBlock` outside this site. Distinct from `AcceptedBlock` (producer-side broadcast token) — both private-constructor tokens but **mechanically incompatible**: producer broadcast takes `AcceptedBlock` not `AdmittedBlock`; receive admission takes `AdmittedBlock` not `AcceptedBlock`. Defended by `ci_check_admitted_block_closure.sh`. |
| **BLUE closed event taxonomy (S1)** | `ade_ledger::receive::events::{ReceiveEvent, ReceiveEffect, ReceiveError, NoOpReason, TargetPoint, TipPoint}` | BLUE | Closed canonical input/output sums. `ReceiveEvent` has exactly **3 variants** (`RollForward { slot, hash, header_bytes, tip }`, `RollBackward { target_point, tip }`, `BlockDelivered { block_bytes }`) — **no constructor for locally-originated chain-sync / block-fetch messages** the orchestrator might send. That impossibility is the `CN-PROTO-07` closure: the receive side admits peer-originated signals only. `ReceiveEffect` has 4 variants (`Admitted`, `Cached`, `RolledBack`, `NoOp`); `ReceiveError` has 4 variants (`HeaderBodyMismatch`, `Validity(BlockValidityError)`, `RollbackOutOfScope { target_point }`, `ChainDb(ChainWriteError)`). All closed; no `#[non_exhaustive]`; no `String`-bearing variant. |
| **BLUE pending-header cache (S1)** | `ade_ledger::receive::pending_header_cache::{PendingHeaderCache, PendingHeaderCacheError}` | BLUE | Closed `BTreeMap<(SlotNo, Hash32), Vec<u8>>`-backed cache (canonical iteration; no `HashMap`). Insert/get/contains/remove/evict_below/iter surface. Insertion is idempotent on byte-identity at the same key; byte-divergence at the same key is `PendingHeaderCacheError::ByteConflict` (cryptographically unreachable under blake2b_256 header hashing, but the invariant is explicit). Eviction is NOT a concern of this BLUE type — the orchestrator (S4) decides policy as canonical input. |
| **BLUE narrow write trait (S1)** | `ade_ledger::receive::chain_write::{ChainDbWrite, ChainWriteError, ChainWriteErrorKind}` | BLUE | **Closed seam** — single method `write_admitted(&mut self, block: AdmittedBlock) -> Result<(), ChainWriteError>`. Takes `AdmittedBlock` **by value** — the trait surface preserves the admission gate: a caller cannot persist raw bytes; the only way to obtain an `AdmittedBlock` is `admit_via_block_validity`. `ChainWriteError` is a closed 2-variant sum (`SlotConflict { slot, hash }`, `Underlying(ChainWriteErrorKind)`); `ChainWriteErrorKind` is a closed 3-variant `Copy` enum (`Io`, `InvalidOperation`, `Other`) — no `String`. Production impl: `ChainDbWriter` (GREEN, ade_runtime). **Closed seam — one production impl, no plug-in extension.** |
| **BLUE pure transition (S2)** | `ade_ledger::receive::reducer::{ReceiveState, receive_apply, receive_apply_sequence}` | BLUE | Pure, total, deterministic. `ReceiveState { ledger, chain_dep, pending_headers }` is the bundled receive sub-state. `receive_apply(&mut state, event, &mut chain_write, &era_schedule, &dyn LedgerView) -> Result<ReceiveEffect, ReceiveError>` — one event per call; staged-then-committed shape (on error, `state` AND `chain_write` are unchanged). `RollForward`: caches header bytes; **mutates only `state.pending_headers`** (Invariant I-6 — ledger / chain_dep / chain_write all untouched). `BlockDelivered`: decodes body, cross-checks cached header at `(slot, block_hash)`, runs `admit_via_block_validity`, persists via `ChainDbWrite::write_admitted`, commits new `(ledger, chain_dep)` atomically, evicts consumed header. `RollBackward`: returns `Err(ReceiveError::RollbackOutOfScope)` (Path A scope edge — DC-CONS-20). `receive_apply_sequence` is the byte-identical deterministic driver over an event slice. Defended by `ci_check_receive_reducer_closure.sh`. |
| **GREEN signal-lift adapter (S3)** | `ade_runtime::receive::events_to_state::{lift_chain_sync_signal, lift_block_fetch_event}` | GREEN | Pure pass-through translators from N-A `ForkChoiceSignal` / `BatchDeliveryEvent` into `ReceiveEvent`. Non-state-changing variants (`BatchStarted`, `NoBlocks`, `BatchCompleted`, `Intersected`, `NoIntersection`) return `None` and are filtered out by the orchestrator before the BLUE call. **Pass-through discipline: `header_bytes` and `block_bytes` are NEVER decoded here** — the BLUE reducer's `BlockDelivered` branch is the canonical decode site. Pure; no I/O; observably deterministic. Defended by `ci_check_receive_replay_purity.sh`. |
| **GREEN chain-db write adapter (S3)** | `ade_runtime::receive::in_memory_chain_write::ChainDbWriter<'a, D>` | GREEN | Single production impl of the BLUE `ChainDbWrite` trait, over any `ChainDb` implementor (orphan rule reason — lives in the only crate that depends on both `ade_ledger` and the `ChainDb` trait). `write_admitted` decodes the `AdmittedBlock` bytes once via `decode_block` to extract `(slot, hash)`, then calls `ChainDb::put_block(StoredBlock { slot, hash, bytes })`. Maps `ChainDbError` variants into `ChainWriteError`. Pure; no I/O of its own (the I/O is the wrapped `ChainDb`). |
| **GREEN session-transcript replay corpus (S3/S5)** | `crates/ade_runtime/tests/receive_session_transcript_replay.rs`, `crates/ade_runtime/tests/receive_two_peer_independence.rs`, `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs` | GREEN | Replay scaffolding lives as integration tests in `ade_runtime/tests/`. Drives `(initial_state, ReceiveEvent_sequence)` through the full pipeline twice; resulting `(ReceiveState, ChainDb fingerprint)` must be byte-identical (DC-PROTO-09). Multi-peer independence test (`receive_two_peer_independence.rs`) drives two synthetic peer streams against a shared ChainDb and asserts per-session transcripts are unaffected by interleaving. Cross-impl pipeline drive (`receive_pipeline_corpus_drive.rs`) replays the Conway-576 corpus through the receive path and verifies ChainDb tip + admitted bytes + ledger fingerprint match expected. |
| **RED per-peer N2N receive orchestrator (S4)** | `ade_runtime::receive::orchestrator::{PerPeerReceiveState, ReceiveDispatchError, dispatch_chain_sync_inbound, dispatch_block_fetch_inbound}` | RED | Pure state-machine driver — **no socket I/O** (sockets live one layer up in `ade_network::session` / the binary). Decodes inbound mini-protocol frames via N-A codecs, translates to N-A signals/events, lifts to `ReceiveEvent` via the GREEN adapter, calls the BLUE reducer. Per-peer state independent (one `PerPeerReceiveState` per session, holding the per-peer `ReceiveState` + handshake-negotiated chain-sync + block-fetch versions); cross-peer coordination only via the shared `ChainDb` (which is idempotent on byte-identity, so two peers receiving the same block both succeed). **Key boundary: MUST NOT import from `ade_runtime::producer::{signing, broadcast, scheduler}`** — defended by `ci_check_receive_orchestrator_no_producer_dep.sh`. Closed `ReceiveDispatchError` (3 variants: `ChainSyncDecode(CodecError)`, `BlockFetchDecode(CodecError)`, `Receive(ReceiveError)`). |
| **RED operator-action probe binary (S6)** | `ade_core_interop::bin::live_block_follow_session` | RED | Fifth instance of the operator-action probe binary pattern (second receive-direction binary). Hermetic default + `--connect` live pass. Drives the full receive pipeline against a private cardano-node peer; captures admitted-block bytes + ChainDb tip per block. Status `blocked_until_operator_peer_available`. |
| **CI gates (S1..S6)** | `ci/ci_check_{admitted_block_closure, receive_reducer_closure, receive_replay_purity, receive_orchestrator_no_producer_dep, receive_paths_corpus_present}.sh` | CI | 5 mechanical gates defending the receive-side admission authority surface. Total CI count: 47 → 52. |

**Rule.** This domain has **one BLUE admission token** (`AdmittedBlock`
— gated by the `admit_via_block_validity` chokepoint, which composes
the existing `block_validity` authority), **one BLUE closed event
taxonomy** (`ReceiveEvent` / `ReceiveEffect` / `ReceiveError` — three
event variants only; **no constructor for locally-originated outputs**
— CN-PROTO-07), **one BLUE pending-header cache**
(`PendingHeaderCache` — BTreeMap-backed; eviction is orchestrator
policy, not cache concern), **one BLUE narrow write trait**
(`ChainDbWrite` — single method; takes `AdmittedBlock` by value;
closed admission gate preserved across the trait surface), **one BLUE
pure transition** (`receive_apply` + `receive_apply_sequence`; one
event per call; staged-then-committed; `RollBackward` returns
`Err(RollbackOutOfScope)` — Path A scope edge), **two GREEN adapters**
(`events_to_state` + `ChainDbWriter`), **one RED per-peer session
driver** (`orchestrator` with two dispatch functions), **a
session-transcript replay corpus + multi-peer independence +
cross-impl pipeline drive harness** (integration tests under
`ade_runtime/tests/`), and **one RED operator-action probe binary**
(`live_block_follow_session`).

**THE KEY SEAMS:**

1. **`AdmittedBlock` is a CLOSED type-level admission token** —
   private inner field; sole public constructor
   `admit_via_block_validity`; mechanically distinct from
   `AcceptedBlock` (producer-side broadcast token). End-to-end:
   peer-originated bytes can reach ChainDb only after passing
   `block_validity` — there is no public path that lets a caller
   persist raw bytes through `ChainDbWrite`. Defended by
   `ci_check_admitted_block_closure.sh`.
2. **`ReceiveEvent` is a CLOSED taxonomy of peer-originated
   signals** — three variants only; **no constructor exists for
   locally-originated chain-sync or block-fetch outputs** (the
   client requests the orchestrator might send). CN-PROTO-07 by
   construction (the public API surface IS the closure proof).
3. **`ChainDbWrite` is a CLOSED narrow seam** — single method; takes
   `AdmittedBlock` by value; one production impl (`ChainDbWriter`)
   and no plug-in extension. Documented in §3 below as Closed
   registry, not Extensible.
4. **`receive_apply` is the single receive-side composition root** —
   the BLUE chokepoint into `LedgerState` + `PraosChainDepState` +
   `ChainDb` over peer-supplied bytes. Pure, total, deterministic;
   staged-then-committed; the only public path that calls
   `admit_via_block_validity`.
5. **Per-peer state is independent** — multi-peer independence is by
   construction (each `PerPeerReceiveState` is independent;
   cross-peer coordination is only via the shared `ChainDb`, which
   is idempotent on byte-identity). The two-peer independence test
   confirms per-session transcripts are invariant under interleaving.
6. **The RED orchestrator never sees private keys and never crosses
   to the producer surface** — `orchestrator` has no path to
   `ade_runtime::producer::{signing, broadcast, scheduler}`
   (defended by `ci_check_receive_orchestrator_no_producer_dep.sh`).
   Receive paths admit peer bytes; they do not sign anything and
   they do not feed the producer.
7. **Path A scope edge is explicit and structural (DC-CONS-20)** —
   `ReceiveEvent::RollBackward` returns
   `Err(ReceiveError::RollbackOutOfScope { target_point })`. The
   error variant exists; the closure is not "we forgot rollback";
   the rollback half of `DC-CONS-20` is a deliberately separable
   follow-on cluster's deliverable
   (`open_obligation = rollback_side_blocked_until_ledger_snapshot_cluster`).

**New work** that adds a receive-side feature attaches by extending
the closed `ReceiveEffect` / `ReceiveError` arms inside the reducer
or by adding a closed `ReceiveEvent` variant (closed-sum extension,
version-gated, surface ratified at cluster entry) — not by exposing
a parallel admission path, not by bypassing
`admit_via_block_validity`, not by passing raw bytes through
`ChainDbWrite`, not by widening `AdmittedBlock`'s constructors, not
by introducing a second per-peer state struct that shares the
admission token.

**Declared non-goals carried from the cluster doc:** full rollback
authority (OQ-1 Path A scope edge; `DC-CONS-20.open_obligation`),
multi-peer fork choice / Praos longest-chain selection (OQ-4 lock —
single-source follow only), N2C local-chain-sync receive surface
(out of scope — separate cluster).

### Producer-side server response authority (carried unchanged from N-G; OP-OPS-04 strengthened in N-H via receive-side key-boundary mirror)

Carried. **N-H strengthening:** the producer-side server pump
(`n2n_server`) and the receive-side orchestrator (`receive::orchestrator`)
are **two independent RED drivers** that share the upstream BLUE
authorities (`block_validity`, ChainDb trait) but cannot reach each
other and cannot reach private keys. Both have a CI-defended
key-boundary; the receive-side adds the symmetric guard against
crossing INTO the producer surface (no `crate::producer::signing /
broadcast / scheduler` imports). `OP-OPS-04` already covered the
producer-side; the principle now applies symmetrically.

### Block production authority (carried unchanged from N-C)

Carried.

### Mempool ingress (carried unchanged from N-E)

Carried.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged from the prior revision. **N-H-specific
strengthening:** the full block validity composition contract
(`block_validity`) now has a **fourth public consumer** beyond the
validator's direct callers, `self_accept` (N-C), and (transitively)
the producer's forge path: `admit_via_block_validity` (N-H) wraps it
for the receive side. `DC-CONS-13.strengthened_in += PHASE4-N-H`
(symmetric receive closure: admit = `Valid` only). The single header
projection authority (`accepted_block_header_bytes` and its
underlying body-hash recipe site) gains a fourth consumer in the
receive reducer via the inline header-walker private helper that
reuses the same `decode_block_envelope` + cbor::skip_item recipe —
not a parallel splitter (the public splitter remains the one in
`block_validity/header_input.rs`); `DC-CONS-16.strengthened_in +=
PHASE4-N-H`.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. N-H added an `ade_runtime → ade_ledger` strengthening (RED →
  BLUE via the `receive::orchestrator` module importing the
  `ade_ledger::receive::*` BLUE chokepoints) — same direction as
  existing N-C / N-G edges; allowed.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE. The new
  `ade_ledger::receive::*` modules are BLUE; no async.
- **`ci_check_admitted_block_closure.sh`** *(N-H-S1 — CN-PROTO-07,
  CN-CONS-07 strengthening)* — forbids any `pub fn .* -> *AdmittedBlock`
  outside the canonical site
  `crates/ade_ledger/src/receive/admitted.rs`. Positive presence:
  the `admit_via_block_validity` site itself MUST exist there.
- **`ci_check_receive_reducer_closure.sh`** *(N-H-S2 — CN-CONS-08,
  DC-CONS-19)* — forbids `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock
  in `crates/ade_ledger/src/receive/reducer.rs` production code;
  forbids any `RollForward` arm path mutating `state.ledger` /
  `state.chain_dep` / calling `chain_write.*` (Invariant I-6);
  forbids any `RollBackward` arm path returning `Ok` (Path A scope
  edge — must be `Err(RollbackOutOfScope)`); positive presence
  checks for `receive_apply`, `receive_apply_sequence`,
  `ReceiveState`.
- **`ci_check_receive_replay_purity.sh`** *(N-H-S3 — DC-PROTO-09)* —
  forbids `tokio`/wall-clock/`rand` in
  `crates/ade_runtime/src/receive/{events_to_state,
  in_memory_chain_write}.rs` production code; the GREEN adapters
  must be observably deterministic. Forbids any decode of
  `header_bytes` or `block_bytes` in `events_to_state` (pass-through
  discipline — the BLUE reducer is the canonical decode site).
- **`ci_check_receive_orchestrator_no_producer_dep.sh`** *(N-H-S4 —
  key-boundary doctrine; OP-OPS-04 mirror)* — forbids any import
  path from `crates/ade_runtime/src/receive/` into
  `crate::producer::signing` / `crate::producer::broadcast` /
  `crate::producer::scheduler`. The receive paths cannot sign
  anything, cannot enqueue broadcasts, and cannot drive the
  producer scheduler; they only admit peer bytes and write through
  `ChainDbWrite`.
- **`ci_check_receive_paths_corpus_present.sh`** *(N-H-S6 —
  RO-LIVE-02)* — guards receive-paths fixture corpus presence,
  the named integration test functions, the binary src + Cargo.toml
  `[[bin]]` entry, and the procedure doc
  `docs/clusters/PHASE4-N-H/CE-N-H-6_PROCEDURE.md`.
- `ci_check_no_private_keys_in_corpus.sh` *(extended in N-H-S3)* —
  scope widened to cover the new
  `crates/ade_testkit/fixtures/receive_paths/` corpus root.
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

Ade's authority surface is **almost entirely closed.** **PHASE4-N-H
added thirteen closed surfaces** — `AdmittedBlock` (closed BLUE
token), `AdmittedOutcome` (closed struct), `ReceiveEvent` (closed
3-variant sum), `ReceiveEffect` (closed 4-variant sum), `ReceiveError`
(closed 4-variant sum), `NoOpReason` (closed 1-variant Copy enum),
`TargetPoint` + `TipPoint` (closed structs), `PendingHeaderCache`
(closed BLUE index type), `PendingHeaderCacheError` (closed 1-variant
sum), `ChainWriteError` (closed 2-variant sum), `ChainWriteErrorKind`
(closed 3-variant Copy enum), `ReceiveState` (closed struct),
**plus one closed trait seam** (`ChainDbWrite`), **the canonical
admission chokepoint** (`admit_via_block_validity`), and **the canonical
reducer chokepoint pair** (`receive_apply` + `receive_apply_sequence`).
Plus **five CI gates** (CI count 47 → 52) and **six newly-introduced
registry rules + six strengthenings** (registry total 196 → 202).

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
| `ProducerTick` *(N-C-S3 — DC-CONS-13; **N-H strengthened**)* | `ade_ledger::producer::state` | closed 14-field struct | Carried; `DC-CONS-13.strengthened_in += PHASE4-N-H` (symmetric receive closure). |
| `forge_block` chokepoint *(N-C-S3)* | `ade_ledger::producer::forge` | 1 function | Carried. |
| `ForgeError` / `ForgeEffects` / `ForgedBlock` *(N-C-S3)* | `ade_ledger::producer::forge` | 7 / 1 / closed struct | Carried. |
| `encode_opcert` / `decode_opcert` chokepoint pair *(N-C-S2)* | `ade_codec::shelley::opcert` | 2 functions | Carried. |
| `OpCertCodecError` *(N-C-S2)* | `ade_codec::shelley::opcert` | 7 variants | Carried. |
| `opcert_validate` chokepoint *(N-C-S2)* | `ade_core::consensus::opcert_validate` | 1 function | Carried. |
| `OpCertError` *(N-C-S2)* | `ade_core::consensus::opcert_validate` | closed validation-error sum | Carried. |
| `block_body_hash_from_buckets` chokepoint *(N-C-S4 — DC-CONS-16; **N-H strengthened**)* | `ade_ledger::block_body_hash` | 1 function | Carried. **N-H strengthening**: the header sub-slice walker recipe gains a fourth consumer via the receive reducer's inline private helper (same recipe; no parallel public splitter). `DC-CONS-16.strengthened_in += PHASE4-N-H`. |
| `AcceptedBlock` token *(N-C-S5 — CN-CONS-07; **N-H strengthened via the receive-side mirror**)* | `ade_ledger::producer::self_accept` | 1 newtype (private field) | Carried. **N-H strengthening**: `AcceptedBlock` remains the producer-side broadcast/serve gate; **`AdmittedBlock` is its receive-side symmetric mirror**. End-to-end (producer + receive): `AcceptedBlock` gates outbound; `AdmittedBlock` gates inbound. The tokens are mechanically incompatible (no constructor accepts the other's bytes). `CN-CONS-07.strengthened_in += PHASE4-N-H`. |
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
| **`AdmittedBlock` token** *(NEW in N-H-S1 — CN-PROTO-07 + CN-CONS-07 mirror; closed-grammar token)* | `ade_ledger::receive::admitted` | 1 struct with private `bytes: Vec<u8>` field | The **only** receive-side admission token. Sole public constructor `admit_via_block_validity`. Mechanically distinct from `AcceptedBlock` — no constructor on either side accepts the other's bytes (Invariant ¬P-6). New constructor = strengthening; the single-site invariant is CI-defended by `ci_check_admitted_block_closure.sh`. |
| **`AdmittedOutcome`** *(NEW in N-H-S1)* | `ade_ledger::receive::admitted` | closed struct `{ admitted: AdmittedBlock, ledger: LedgerState, chain_dep: PraosChainDepState }` | The return shape of `admit_via_block_validity`. Closed. |
| **`admit_via_block_validity` chokepoint** *(NEW in N-H-S1 — CN-PROTO-07)* | `ade_ledger::receive::admitted` | 1 function — `pub fn admit_via_block_validity(block_bytes, &LedgerState, &PraosChainDepState, &EraSchedule, &dyn LedgerView) -> Result<AdmittedOutcome, BlockValidityError>` | The **single canonical** receive-side admission chokepoint. Composes `block_validity` (B1) — does not re-implement it. Returns `Ok` only on `BlockValidityVerdict::Valid`. |
| **`ReceiveEvent`** *(NEW in N-H-S1 — CN-PROTO-07; closed canonical input)* | `ade_ledger::receive::events` | 3 variants — `RollForward { slot, hash, header_bytes, tip }`, `RollBackward { target_point, tip }`, `BlockDelivered { block_bytes }` | **The closed receive-event taxonomy.** **No constructor for locally-originated chain-sync / block-fetch messages** (the client requests the orchestrator might send) — that impossibility is the CN-PROTO-07 closure. No `#[non_exhaustive]`; no `String`-bearing variant. New variant = closed-sum extension; version-gated. |
| **`ReceiveEffect`** *(NEW in N-H-S1 — CN-CONS-08)* | `ade_ledger::receive::events` | 4 variants — `Admitted { slot, hash }`, `Cached { slot, hash }`, `RolledBack { to_slot }`, `NoOp { reason: NoOpReason }` | Closed sum. `RolledBack` is unreachable in Path A (reserved for the follow-on rollback cluster). |
| **`NoOpReason`** *(NEW in N-H-S1)* | `ade_ledger::receive::events` | 1 variant — `HeaderAlreadyCached` (Copy enum) | Closed sum; reason-tag for `ReceiveEffect::NoOp`. No `String`. |
| **`ReceiveError`** *(NEW in N-H-S1 — DC-CONS-19, DC-CONS-20)* | `ade_ledger::receive::events` | 4 variants — `HeaderBodyMismatch { decoded_slot, decoded_hash }`, `Validity(BlockValidityError)`, `RollbackOutOfScope { target_point }`, `ChainDb(ChainWriteError)` | Closed sum. `RollbackOutOfScope` is the explicit Path A scope-edge variant — surfaces the rollback half of DC-CONS-20 as a structured failure rather than silent acceptance or panic. |
| **`TargetPoint` / `TipPoint`** *(NEW in N-H-S1)* | `ade_ledger::receive::events` | 2 closed structs | Closed. |
| **`PendingHeaderCache`** *(NEW in N-H-S1 — Invariant I-6 support; closed BLUE index)* | `ade_ledger::receive::pending_header_cache` | closed struct `{ entries: BTreeMap<(SlotNo, Hash32), Vec<u8>> }` | The **single pending-header cache** for the receive bridge. BTreeMap-backed (canonical iteration; no `HashMap`). Insertion idempotent on byte-identity; byte-divergence at the same key is `PendingHeaderCacheError::ByteConflict` (cryptographically unreachable under blake2b_256 header hashing). Surface: insert / get / contains / remove / evict_below / iter / len / is_empty. Eviction policy is orchestrator concern, NOT cache concern. |
| **`PendingHeaderCacheError`** *(NEW in N-H-S1)* | `ade_ledger::receive::pending_header_cache` | 1 variant — `ByteConflict { slot, hash }` | Closed sum. |
| **`ChainDbWrite` trait** *(NEW in N-H-S1 — Invariant I-12 support; closed seam, not a registry)* | `ade_ledger::receive::chain_write` | 1 trait with 1 method — `write_admitted(&mut self, block: AdmittedBlock) -> Result<(), ChainWriteError>` | **Closed seam** — single production impl (`ade_runtime::receive::in_memory_chain_write::ChainDbWriter`, GREEN). **No plug-in extension at runtime.** Takes `AdmittedBlock` BY VALUE — the trait surface preserves the admission gate (a caller cannot persist raw bytes; the only path to `AdmittedBlock` is `admit_via_block_validity`). New impls would be a deliberate registry-tracked addition (e.g. a future `PersistentChainDbWriter` once a persistent ChainDb is wired) — not a runtime plug-in. New trait method = strengthening (closed extension, version-gated). |
| **`ChainWriteError`** *(NEW in N-H-S1)* | `ade_ledger::receive::chain_write` | 2 variants — `SlotConflict { slot, hash }`, `Underlying(ChainWriteErrorKind)` | Closed sum. No `String`. |
| **`ChainWriteErrorKind`** *(NEW in N-H-S1)* | `ade_ledger::receive::chain_write` | 3 variants — `Io`, `InvalidOperation`, `Other` (Copy enum) | Closed sum. |
| **`ReceiveState`** *(NEW in N-H-S2)* | `ade_ledger::receive::reducer` | closed struct `{ ledger: LedgerState, chain_dep: PraosChainDepState, pending_headers: PendingHeaderCache }` | The bundled receive sub-state. Closed. |
| **`receive_apply` chokepoint** *(NEW in N-H-S2 — DC-CONS-19; CN-CONS-08)* | `ade_ledger::receive::reducer` | 1 function — `pub fn receive_apply<W: ChainDbWrite>(state, event, chain_write, era_schedule, ledger_view) -> Result<ReceiveEffect, ReceiveError>` | The **single receive-side composition root**. Pure, total, deterministic. Staged-then-committed shape. RollForward mutates only `state.pending_headers`. RollBackward returns `Err(RollbackOutOfScope)`. Defended by `ci_check_receive_reducer_closure.sh`. |
| **`receive_apply_sequence` driver** *(NEW in N-H-S2)* | `ade_ledger::receive::reducer` | 1 function | The deterministic event-slice driver. Pure. |
| **`PerPeerReceiveState`** *(NEW in N-H-S4)* | `ade_runtime::receive::orchestrator` | closed RED struct `{ receive_state, chain_sync_version, block_fetch_version }` | Closed. Per-peer state independent; cross-peer coordination only via the shared `ChainDb`. |
| **`ReceiveDispatchError`** *(NEW in N-H-S4)* | `ade_runtime::receive::orchestrator` | 3 variants — `ChainSyncDecode(CodecError)`, `BlockFetchDecode(CodecError)`, `Receive(ReceiveError)` | Closed RED dispatch-error sum. |
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
| `ChainDb` / `SnapshotStore` / `Recoverable` trait surfaces | `ade_runtime::chaindb` + `ade_runtime::recovery` | closed | |
| Hash domain functions | `ade_crypto::blake2b::*` | 4 named domains | |
| `ChainEvent` / `ChainSelectionReject` *(N-B)* | `ade_core::consensus::events` | 5 / 4 variants | |
| Consensus error families *(N-B)* | `ade_core::consensus::errors` | 8 closed error enums | |
| `StreamInput` / `OrchestratorError` / `DecodeError` / `GenesisParseError` / `GenesisBlob` / `NetworkMagic` *(N-B)* | various | closed | |
| `LedgerView` trait *(N-B; B1-refined)* | `ade_core::consensus::ledger_view` | 4 methods | |
| `HeaderVrf` *(N-B; B1)* | `ade_core::consensus::header_summary` | 2 variants | |
| `BlockValidityVerdict` / `BlockValidityError` etc. *(B1)* | `ade_ledger::block_validity::verdict` | closed | |
| `block_validity` chokepoint *(B1; **N-H strengthened**)* | `ade_ledger::block_validity::transition` | 1 function | Single chokepoint. `self_accept` (N-C-S5) and **`admit_via_block_validity` (N-H-S1)** are its two BLUE wrappers. `DC-CONS-13.strengthened_in += PHASE4-N-H`. |
| `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` / `SignerSource` / `WitnessClosureError` etc. *(B2)* | `ade_ledger::tx_validity::*` | closed | |
| `AdmitOutcome` / `MempoolState` / `OrderPolicy` *(B2)* | `ade_ledger::mempool::*` | closed | |
| `LeaderScheduleAnswer` / `is_leader_for_vrf_output` *(N-B; consumed unchanged by N-C)* | `ade_core::consensus::leader_schedule` | closed | |
| `PraosNonces` / `NonceScanError` *(B1)* | `ade_ledger::consensus_input_extract` | | |
| `PraosChainDepState` / `ChainEvent` canonical encodings *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| `LedgerFingerprint` fold *(B3/B5)* | `ade_ledger::fingerprint` | | |
| **CI check set** | `ci/ci_check_*.sh` | **52 scripts (47 → 52 in PHASE4-N-H)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-H added 6 rules** (`CN-CONS-08`, `DC-CONS-19`, `DC-CONS-20`, `DC-PROTO-09`, `CN-PROTO-07`, `RO-LIVE-02`); strengthened `T-DET-01`, `T-ENC-01`, `DC-CONS-13`, `DC-CONS-16`, `CN-CONS-07`, `DC-PROTO-06`. Total: **202 entries** (196 → 202). | Append-only IDs. |

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
| `RollbackSnapshot` ring *(N-B; **load-bearing for the future rollback cluster**)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ 2160. **NEXT-CLUSTER consumer (DC-CONS-20 closure):** seeds the receive-side rollback authority. |
| `ServedChainSnapshot.blocks` admitted set *(N-G-S2 — extension via the closed `served_chain_admit` chokepoint only)* | `ade_ledger::producer::served_chain::ServedChainSnapshot` | Shape closed; instance set open. |
| `PerPeerN2nServerState` instance set *(N-G-S6 — extension per session)* | `ade_runtime::network::n2n_server` | One instance per connected peer (producer-side server pump). |
| **`PendingHeaderCache.entries`** *(NEW in N-H-S1 — runtime-extensible **content**, but extension via the **closed** `PendingHeaderCache::insert` chokepoint only)* | `ade_ledger::receive::pending_header_cache::PendingHeaderCache` | `BTreeMap<(SlotNo, Hash32), Vec<u8>>`. Shape closed; instance set open. The set grows during a follow session and is bounded structurally by `evict_below(slot)` (orchestrator policy as canonical input — not cache concern). |
| **`PerPeerReceiveState` instance set** *(NEW in N-H-S4 — runtime-extensible per session, but each instance is itself a closed struct)* | `ade_runtime::receive::orchestrator` | One instance per connected upstream peer (receive-side counterpart of N-G's `PerPeerN2nServerState`). The orchestrator (one layer up, the binary or `ade_network::session::*`) constructs / drops instances as peers connect / disconnect. Per-peer state independent. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus / Consensus corpus / Block-validity corpus / Tx-validity corpus / Mempool ingress corpus / PP canonical corpus / Producer corpus / Server-paths corpus | various | Tooling-only. |
| **Receive-paths session-transcript replay corpus** *(NEW in N-H-S3/S5 — tooling-only)* | `crates/ade_testkit/fixtures/receive_paths/` + `crates/ade_runtime/tests/receive_session_transcript_replay.rs` + `crates/ade_runtime/tests/receive_two_peer_independence.rs` + `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs` | Tooling-only. GREEN. Drives `(initial_state, ReceiveEvent_sequence)` tuples through the full pipeline; resulting `(ReceiveState, ChainDb fingerprint)` must be byte-identical (DC-PROTO-09). Append-only by convention. Defended by `ci_check_receive_paths_corpus_present.sh`. |
| **Operator-action probe binaries** *(N-B + N-E S6 + N-C S7 + N-G S7 + **N-H S6**)* | `ade_core_interop::bin::{live_consensus_session, live_tx_submission_session, live_block_production_session, live_block_fetch_session, live_block_follow_session}` | RED operator-action; `#[ignore]`-gated by closure-gate tests. **N-H added `live_block_follow_session`** — status `blocked_until_operator_peer_available`. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **CE-N-H-6 (operator-action live evidence — `blocked_until_operator_peer_available`)** | **Live N2N follow-mode admission log (Ade consuming, cardano-node serving)** | The live cross-impl claim for the receive-side. Requires a private cardano-node peer. Re-opens on operator availability. |
| **Full rollback authority cluster** *(NEW HIGHEST-PRIORITY candidate flagged by N-H close — DC-CONS-20 rollback half; OQ-1 Path A scope edge)* | **`rollback_apply` chokepoint + ledger-state snapshot store (encode/decode) + replay-forward driver** | The natural counterpart to N-H's admit-only Path A scope. Closure converts `ReceiveEvent::RollBackward` from `Err(RollbackOutOfScope)` to a real transition. Consumes the existing `RollbackSnapshot` ring + `ChainDb::iter_from_slot` + `block_validity`. Surface; do not invent invariants here. |
| **Multi-peer fork choice cluster** *(NEW candidate flagged by N-H close — OQ-4 lock)* | **Praos longest-chain selection across competing `PerPeerReceiveState[]`** | Sequenced after the rollback cluster (commits to the chosen fork mean rolling back from the loser). Consumes existing `ade_runtime::consensus::chain_selector`. Surface; do not invent invariants here. |
| **N2C local-chain-sync receive surface cluster** *(NEW candidate flagged by N-H close)* | **Operator-side N2C clients consume Ade's chain via `LocalChainSyncMessage`** | Sibling of `producer_chain_sync_serve` (N-G) over the local-chain-sync N2C codec; reuses `ServedHeaderLookup`-style trait, this time over the persisted ChainDb tip. Surface; do not invent invariants here. |
| **CE-N-G-8 (operator-action live evidence — `blocked_until_operator_peer_available`)** | **Live N2N block-fetch acceptance log (Ade serving)** | Carried. |
| **CE-N-C-8 (operator-action live evidence — `blocked_until_operator_stake_available`)** | **Live N2N block-fetch acceptance log (Ade forging)** | Carried. |
| **N-H+ Tier-5** | **Operator-tunable receive policy** (pending-header eviction window, per-peer back-pressure, max in-flight cached headers per peer) | Tier-5 — operator-tunable. Declared OUT-OF-SCOPE in N-H cluster doc. |
| **N-G+ Tier-5** | **Operator-tunable server policy** | Carried. |
| **N-C+ Tier-5** | **Operator-tunable producer policy** | Carried. |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | Carried. |
| **PP OQ-1..OQ-4** | various | Carried. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

### Closed-grammar audit (PHASE4-N-H full close)

This sweep was performed after PHASE4-N-H full close (S1..S6).

1. **`AdmittedBlock` closed admission token** — **closed by intent
   and CI-defended.** Private inner field; sole public constructor
   `admit_via_block_validity`; mechanically distinct from
   `AcceptedBlock` (no constructor accepts the other's bytes).
   Defended by `ci_check_admitted_block_closure.sh`.
2. **`admit_via_block_validity` chokepoint** — **closed by intent.**
   Composes `block_validity` (B1) — does not re-implement it.
   Returns `Ok` only on `BlockValidityVerdict::Valid`.
3. **`ReceiveEvent` closed taxonomy** — **closed by intent and
   compile-time-defended.** Three variants only; **no constructor for
   locally-originated chain-sync / block-fetch outputs** (CN-PROTO-07
   by construction). Exhaustive-match round-trip test confirms a
   fourth variant addition fails to compile.
4. **`ReceiveEffect` / `ReceiveError` closed sums** — **closed by
   intent.** 4 / 4 variants; no `#[non_exhaustive]`; no `String`.
   `ReceiveError::RollbackOutOfScope` is the structured Path A
   scope-edge variant.
5. **`PendingHeaderCache` BLUE index** — **closed by intent.**
   BTreeMap-backed (no HashMap); insert/get/contains/remove/
   evict_below/iter surface; insertion idempotent on byte-identity;
   `PendingHeaderCacheError::ByteConflict` for byte-divergence at
   the same key.
6. **`ChainDbWrite` closed narrow trait seam** — **closed by intent.**
   Single method; takes `AdmittedBlock` by value (gate preserved);
   single production impl (`ChainDbWriter`); new impls = deliberate
   registry-tracked addition.
7. **`receive_apply` + `receive_apply_sequence` chokepoints** —
   **closed by intent and CI-defended.** Pure, total, deterministic;
   staged-then-committed; RollForward mutates only
   `state.pending_headers`; RollBackward returns
   `Err(RollbackOutOfScope)`. Defended by
   `ci_check_receive_reducer_closure.sh`.
8. **GREEN `events_to_state` adapter** — **closed by intent and
   CI-defended.** Pure pass-through; `header_bytes` / `block_bytes`
   NEVER decoded (the BLUE reducer is the canonical decode site).
   Defended by `ci_check_receive_replay_purity.sh`.
9. **GREEN `ChainDbWriter` adapter** — **closed by intent.** Single
   production impl of `ChainDbWrite` over any `ChainDb`; decodes
   once for the `(slot, hash)` key; maps `ChainDbError` → `ChainWriteError`.
10. **RED `orchestrator` driver — key-boundary preserved.**
    **CI-defended.** Cannot import from
    `ade_runtime::producer::{signing, broadcast, scheduler}`.
    Defended by `ci_check_receive_orchestrator_no_producer_dep.sh`.
    Closed `PerPeerReceiveState` + `ReceiveDispatchError`
    (3-variant) sums.
11. **`live_block_follow_session` operator-action probe binary** —
    **closed by intent on the harness pattern.** Fifth instance of
    the family (second receive-direction binary).
    Hermetic-default-plus-`--connect`-live. Status
    `blocked_until_operator_peer_available` — same closure-mode
    variant as N-G (peer-blocker).

**Gap note — N-H (CE-N-H-6).** The live cross-impl claim is the only
N-H obligation that depends on an external resource (a private
cardano-node peer). Per `RO-LIVE-02.open_obligation` it is
`blocked_until_operator_peer_available` — not deferred to a future
cluster, not silently accepted. Reopens when a peer is provisioned;
mechanical half (structural cross-impl pipeline) is already enforced
via `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs` +
`ci_check_receive_paths_corpus_present.sh`.

**Gap note — DC-CONS-20 rollback half.** The Path A scope edge is
NOT a "we'll match it later" stub — it is a structured
`ReceiveError::RollbackOutOfScope { target_point }` variant that
surfaces deterministically on every `RollBackward` event. The
registry rule ships `status = "declared"` with
`open_obligation = rollback_side_blocked_until_ledger_snapshot_cluster`,
naming the follow-on cluster's deliverable. **This is the explicit
candidate seam for the next planner.**

### Closed-grammar audit (carried — PHASE4-N-G / PHASE4-N-C / PROPOSAL-PROCEDURES-DECODE / PHASE4-N-E / B3 / B4 / B5)

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
  **N-H**)*: preserved-CBOR-segment bytes
  (`T-ENC-01.strengthened_in += PHASE4-N-H` — peer-supplied wire
  bytes flow into ChainDb verbatim via `AdmittedBlock.bytes`).
- **Single canonical body-hash authority** *(N-C-S4 — DC-CONS-16;
  strengthened in N-G, **N-H**)*: `block_body_hash_from_buckets` is
  the **only** function computing the recipe. **N-H strengthening**:
  the receive reducer's inline header-walker reuses the same
  `decode_block_envelope` + cbor::skip_item recipe — same canonical
  site; no parallel public splitter.
  `DC-CONS-16.strengthened_in += PHASE4-N-H`.
- **Single canonical header/body splitter** *(N-G-S1 — DC-CONS-18)*:
  `accepted_block_header_bytes` is the **only** public header-bytes
  accessor. Carried.
- **Server-agency closure for outgoing mini-protocol messages**
  *(N-G-S1 — CN-PROTO-06)*: carried.
- **Receive-event closure for incoming peer signals** *(NEW in
  N-H-S1 — CN-PROTO-07)*: the `ReceiveEvent` taxonomy has **no
  constructor for locally-originated chain-sync / block-fetch
  outputs** (`RequestNext`, `RequestRange`, `FindIntersect`, `Done`,
  `ClientDone`). The receive reducer cannot be fed the orchestrator's
  own outbound client requests. Compile-time enforcement; the public
  API surface IS the closure proof.
- **Type-level receive admission gate** *(NEW in N-H-S1 — CN-CONS-07
  strengthening)*: `AdmittedBlock` is the **only** token persisted
  via `ChainDbWrite::write_admitted`. The trait method takes the
  token by value; raw bytes have no public path into ChainDb. End-to-
  end: peer-originated bytes can reach ChainDb only after passing
  `block_validity`. **The producer-side `AcceptedBlock` gate
  (N-C-S5) and the receive-side `AdmittedBlock` gate (N-H-S1) are
  the matched pair** — outbound gate + inbound gate, both private-
  constructor, both mechanically distinct from each other.
  `CN-CONS-07.strengthened_in += PHASE4-N-H`.
- **Receive-side admission state-isolation discipline (Invariant
  I-6)** *(NEW in N-H-S2 — CN-CONS-08 / DC-CONS-19)*: the
  `receive_apply` reducer's `RollForward` arm mutates only
  `state.pending_headers` — never `state.ledger`, never
  `state.chain_dep`, never `chain_write`. The `BlockDelivered` arm
  is staged-then-committed: any failure (header cross-check,
  `block_validity::Invalid`, `ChainDbWrite::write_admitted` failure)
  leaves `state` AND `chain_write` unchanged. Defended by
  `ci_check_receive_reducer_closure.sh`.
- **Path A scope-edge structural failure** *(NEW in N-H-S2 —
  DC-CONS-20 admit half + structured Path A scope edge)*: the
  `receive_apply` reducer's `RollBackward` arm returns
  `Err(ReceiveError::RollbackOutOfScope { target_point })`
  deterministically. Receive state is unchanged. The error variant
  exists; the closure is explicit; the rollback half of `DC-CONS-20`
  is a deliberately separable follow-on cluster's deliverable
  (`open_obligation = rollback_side_blocked_until_ledger_snapshot_cluster`).
- **Receive-side replay determinism** *(NEW in N-H-S3 — DC-PROTO-09)*:
  given canonical inputs `(initial ReceiveState, ChainDb,
  ReceiveEvent_sequence)`, the receive pipeline (events_to_state →
  receive_apply → ChainDbWriter) produces a byte-identical
  `(ReceiveState, ChainDb fingerprint)` across replays. The per-
  session reducer is a pure deterministic transition.
  `T-DET-01.strengthened_in += PHASE4-N-H`.
- **Per-peer receive-state independence across peers** *(NEW in
  N-H-S4)*: the RED orchestrator constructs an independent
  `PerPeerReceiveState` per peer; cross-peer coordination is only
  via the shared `ChainDb` (which is idempotent on byte-identity).
  Multi-peer independence (two-peer test) confirms per-session
  transcripts are unaffected by interleaving.
- **Key-boundary for receive paths** *(NEW in N-H-S4 — OP-OPS-04
  mirror)*: the RED `orchestrator` has no path to
  `ade_runtime::producer::{signing, broadcast, scheduler}`. Receive
  paths admit peer bytes; they do not sign anything, do not enqueue
  broadcasts, and do not drive the producer scheduler. Defended by
  `ci_check_receive_orchestrator_no_producer_dep.sh`.
- **Handshake-negotiated version threading through the receive
  reducer call site** *(NEW in N-H-S4 — DC-PROTO-06 strengthening)*:
  the orchestrator passes the handshake-negotiated chain-sync +
  block-fetch versions on every reducer call; never reads from a
  session global. `DC-PROTO-06.strengthened_in += PHASE4-N-H`.
- **Served-bytes parity** *(N-G-S4 — DC-CONS-17)*: carried.
- **Header-body wire coherence** *(N-G-S5 — DC-CONS-18)*: carried.
- **Producer-side server-role transcript determinism** *(N-G-S5 —
  DC-PROTO-07)*: carried.
- **Deterministic-resolution discipline for server-agency waits**
  *(N-G-S3 — DC-PROTO-08)*: carried.
- **Type-level broadcast and serve gate** *(N-C-S5 — CN-CONS-07;
  N-G + N-H strengthened)*: carried; see also the new receive-side
  mirror above.
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
- **`block_validity` composition contract** *(B1; consumed unchanged
  by N-C `self_accept`, N-G `served_chain_admit`, and **N-H
  `admit_via_block_validity`**)*.
  `DC-CONS-13.strengthened_in += PHASE4-N-H` (symmetric receive
  closure: admit = `Valid` only).
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*.
- **`mempool_ingress` chokepoint contract** *(N-E)*.
- **`IngressSource` source-invariance contract** *(N-E)*.
- **Verbatim tx-bytes flow through ingress** *(N-E; **N-H mirror**:
  verbatim block-bytes flow through receive admission into ChainDb
  — `T-ENC-01.strengthened_in += PHASE4-N-H`)*.
- **GREEN single-step replay fold contract** *(N-E — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(N-E; carried)*.
- **Operator-action evidence pattern** *(N-B / N-E / N-C / N-G /
  **N-H**)*: N-H adds the **fifth instance**
  (`live_block_follow_session`) and is the **second instance using
  the `blocked_until_operator_peer_available` closure-mode variant**
  (carrying N-G's precedent; same blocker shape — a private
  cardano-node peer, no SPO stake required).
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE /
  DREP / ENACTMENT / PP)*.
- **Committee-enactment write-back contract** *(ENACTMENT)*.
- **All canonical types**: shapes frozen at the era / version they
  entered.
- **Handshake-negotiated version threading** *(N-A; strengthened in
  N-G and **N-H** — DC-PROTO-06)*: every reducer call from the
  orchestrator (both producer-side server and receive-side admit)
  carries the version returned by the handshake; never reads it
  from a session global. `DC-PROTO-06.strengthened_in += PHASE4-N-H`.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  **N-H additions:** `ade_ledger::receive::{admitted, chain_write,
  events, pending_header_cache, reducer}` are BLUE (under the
  already-BLUE `ade_ledger` crate prefix);
  `ade_runtime::receive::{events_to_state, in_memory_chain_write}`
  are GREEN-inside-RED-crate; `ade_runtime::receive::orchestrator`
  is RED; `ade_core_interop::bin::live_block_follow_session` is RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D).
- **`AcceptedBlock` type-level broadcast gate** *(N-C-S5; strengthened
  in N-G and **N-H** — see above)*.
- **`AdmittedBlock` type-level admission gate** *(NEW in N-H-S1 —
  CN-PROTO-07; CN-CONS-07 receive-side mirror)*: the producer-side
  outbound counterpart of the inbound gate. End-to-end (producer +
  receive): outbound bytes gated by `AcceptedBlock`; inbound bytes
  gated by `AdmittedBlock`. Both private-constructor; both
  mechanically distinct.
- **`forge_block` pure-transition contract** *(N-C-S3 — DC-CONS-13;
  carried + N-H strengthening)*: `DC-CONS-13.strengthened_in +=
  PHASE4-N-H` (symmetric receive closure).
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
- **New `ReceiveEvent` variant** *(NEW in N-H — CN-PROTO-07
  extension point)*: closed sum; today 3 variants (`RollForward`,
  `RollBackward`, `BlockDelivered`); a new variant (e.g. a future
  `EvictHeader` triggered by a peer-originated cache eviction
  signal) is closed-sum extension; version-gated; must update
  `ci_check_receive_reducer_closure.sh` guards. New variants must
  remain peer-originated (CN-PROTO-07 — no locally-originated
  constructors).
- **New `ReceiveEffect` variant** *(NEW in N-H — CN-CONS-08
  extension point)*: closed sum; today 4 variants. The
  `RolledBack { to_slot }` arm becomes reachable when the rollback
  cluster ships.
- **New `ReceiveError` variant** *(NEW in N-H — DC-CONS-19 /
  DC-CONS-20 extension point)*: closed sum; today 4 variants. The
  `RollbackOutOfScope` variant goes away when the rollback cluster
  ships (replaced by `Ok(ReceiveEffect::RolledBack)`); the variant
  removal is the rollback cluster's surface signal.
- **New `ChainDbWrite` impl** *(NEW in N-H — deliberate registry-
  tracked addition)*: the trait is a closed seam, but a future
  cluster MAY register a second impl (e.g. a `PersistentChainDbWriter`
  once a persistent ChainDb is wired). Such an addition is
  **deliberate** — a registry-tracked closed extension, not a
  runtime plug-in.
- **New `ChainDbWrite` trait method** *(NEW in N-H — extension
  point)*: closed seam; new methods are closed-trait extensions and
  require an updated production impl (`ChainDbWriter`) plus
  extended reducer logic. Version-gated.
- **New `ReceiveDispatchError` variant** *(NEW in N-H — extension
  point)*: closed sum; today 3 variants.
- **New CI check**: additive. (N-H added five —
  `ci_check_admitted_block_closure.sh`,
  `ci_check_receive_reducer_closure.sh`,
  `ci_check_receive_replay_purity.sh`,
  `ci_check_receive_orchestrator_no_producer_dep.sh`,
  `ci_check_receive_paths_corpus_present.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol** / **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(N-E)*.
- **Live cardano-node N2N block-fetch acceptance / live N2N follow-
  mode admission** *(N-C / N-G / **N-H**)*: each reopens on operator
  availability.
- **Phase-4 cluster surface additions** (N-F): each cluster's wire
  surface gates additions via its own cluster doc.
- **Rollback authority — `RollBackward` reduction**
  *(NEW in N-H — DC-CONS-20 rollback-half extension point)*: today
  Path A returns `Err(RollbackOutOfScope)`; the follow-on rollback
  cluster wires `Ok(ReceiveEffect::RolledBack { to_slot })` via a
  new BLUE `rollback_apply` chokepoint + a ledger-state snapshot
  store. Closed-sum (`ReceiveError::RollbackOutOfScope`) removal +
  closed-sum (`ReceiveEffect::RolledBack`) reachability is the
  surface signal of that cluster's close.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-H added
five new BLUE submodules** (`ade_ledger::receive::{admitted,
chain_write, events, pending_header_cache, reducer}` under a new
`ade_ledger::receive` barrel), **two new GREEN submodules inside
`ade_runtime`** (`receive::events_to_state`, `receive::in_memory_chain_write`),
**one new RED submodule inside `ade_runtime`** (`receive::orchestrator`,
under a new `ade_runtime::receive` barrel), **one new operator-action
probe binary** (`ade_core_interop::bin::live_block_follow_session`),
**five new CI gates**, **six new registry rules**, and **strengthened
four carried domain-specific rules** (`DC-CONS-13`, `DC-CONS-16`,
`CN-CONS-07`, `DC-PROTO-06`) plus **two universal rules** (`T-DET-01`,
`T-ENC-01`). N-H added **no new crate**, **no new external ingress
wire-format frozen contract beyond the closed `ReceiveEvent` sum**
(the underlying `ChainSyncMessage` / `BlockFetchMessage` enums + the
N-A `ForkChoiceSignal` / `BatchDeliveryEvent` taxonomies were already
frozen), **no new public composer outside the receive-side admission
authority surface**.

**N-H also strengthened one cross-color dependency edge**:

1. `ade_runtime → ade_ledger` (already added in N-C; strengthened
   in N-G; **further strengthened in N-H**) — the RED
   `receive::orchestrator` and the GREEN `receive::in_memory_chain_write`
   adapters import the new `ade_ledger::receive::*` BLUE
   chokepoints. Same direction (RED/GREEN → BLUE); allowed. Passes
   `ci_check_dependency_boundary.sh`.

**The orphan-rule placement decision for the GREEN `ChainDbWriter`
impl** (recorded in `docs/clusters/completed/PHASE4-N-H/N-H-S3.md`):
the `ChainDbWriter<'a, D>` impl of `ChainDbWrite` lives in
`ade_runtime::receive::in_memory_chain_write` because the orphan rule
prevents it from living in either `ade_ledger` (which doesn't
depend on `ChainDb`) or somewhere else (the `ChainDb` trait is in
`ade_runtime::chaindb`). The pattern mirrors N-G's
`ServedChainLookups` placement.

**The module-addition rule N-H sets for future receive-side work:**

1. **A new receive-side BLUE primitive attaches inside
   `ade_ledger::receive::*`** (sibling of `admitted`, `events`,
   `pending_header_cache`, `chain_write`, `reducer`). The module
   MUST be BLUE: no clock, no rand, no I/O, no `HashMap`, no
   `tokio`, no `async`. New canonical types MUST be closed sums or
   closed structs; no `#[non_exhaustive]`; no `String`-bearing
   variants.
2. **A new receive-side authority chokepoint attaches inside the
   same BLUE module.** Pure, total, deterministic. Composes existing
   BLUE chokepoints (`block_validity`, `mempool_ingress`, etc.)
   rather than re-implementing.
3. **A new closed `ReceiveEvent` variant attaches inside the
   `ReceiveEvent` enum body.** New variant = closed-sum extension;
   no `#[non_exhaustive]`; version-gated; MUST remain peer-
   originated (CN-PROTO-07).
4. **A new closed `ReceiveEffect` / `ReceiveError` variant attaches
   inside their respective enum bodies.** `RollbackOutOfScope`
   removal is the rollback cluster's surface signal.
5. **A new `ChainDbWrite` impl attaches inside `ade_runtime::receive`**
   (sibling of `in_memory_chain_write`). The module MUST be a pure
   function over its inputs; MUST NOT invoke signing primitives;
   MUST produce byte-identical outputs across replays. Single
   production impl per ChainDb backend.
6. **A new GREEN signal-lift adapter attaches inside
   `ade_runtime::receive`** (sibling of `events_to_state`). MUST be
   a pure pass-through; MUST NOT decode `header_bytes` or
   `block_bytes` (the BLUE reducer is the canonical decode site).
7. **A new RED per-peer session driver attaches inside
   `ade_runtime::receive`** (sibling of `orchestrator`). The module
   MAY use clocks / async / `tokio` (in the layer above — the
   dispatch functions themselves are pure). The module MUST NOT
   import from `ade_runtime::producer::{signing, broadcast,
   scheduler}` — defended by
   `ci_check_receive_orchestrator_no_producer_dep.sh`-style gates.
8. **A new receive-paths registry rule attaches as a derived `DC-*`
   / `CN-*` family entry** with `code_locus`, `ci_script`, `tests`,
   `cross_ref`. Bidirectional cross-refs to consumed rules.
9. **A new operator-action probe binary attaches inside
   `crates/ade_core_interop/src/bin/`** following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape. The binary MUST stub its live socket halt when an external
   dependency is unavailable; capture status via
   `blocked_until_operator_peer_available` (peer-blocker variant —
   N-G + N-H precedent) or `blocked_until_operator_stake_available`
   (stake-blocker variant — N-C precedent) as appropriate.

### Cross-cluster obligation pattern (carried — strengthened in N-H close)

**N-H carries the `blocked_until_operator_peer_available` variant
(introduced by N-G) into a second instance** (`RO-LIVE-02`,
CE-N-H-6). The pattern is now established across five Tier-1
wire-level seams (chain-sync follow, tx-submission2 outbound, block
forge, block-fetch serve, follow-mode admission). The mechanical
half MUST be closed on the same HEAD (e.g. N-H's
`receive_pipeline_corpus_drive` integration test closes the bytes-
shape claim before the live half ships). **Re-opens on operator
availability** — the procedure doc names the specific blocker and
the re-open criteria.

### Operator-action evidence pattern (carried — strengthened in N-H close)

N-H adds the **fifth instance** of the operator-action probe binary
family: `live_block_follow_session`. The pattern is now established
across five Tier-1 wire-level seams. N-H is the **second
receive-direction binary** (N-B was follow-mode read but stopped at
chain-sync; N-H follows through to block-fetch + admit into ChainDb).

### Cluster scope-edge pattern (NEW in N-H close — DC-CONS-20 Path A)

**N-H introduces a new scope-edge pattern**: a cluster may
deliberately scope down a derived constraint
(`DC-CONS-20` admit-half + scope-edge) and ship a **structured
failure variant** (`ReceiveError::RollbackOutOfScope { target_point }`)
plus an explicit registry-recorded
`open_obligation = <follow-on-cluster-handle>` rather than silently
accepting or panicking. The pattern is binding:

- The scope edge MUST be a deterministic structured error variant
  reachable on every event that crosses it (not a panic, not a log
  + skip, not silent acceptance).
- The registry rule MUST ship `status = "declared"` (or `"partial"`)
  with an `open_obligation` field naming the follow-on cluster
  deliverable.
- The cluster doc MUST name the candidate seam in its handoff
  section for the next planner.
- The CI gate (here: `ci_check_receive_reducer_closure.sh`) MUST
  defend the structural-failure property (forbids any `Ok` return
  from the scoped-out arm).

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-H:** `ade_ledger::receive::*` modules have no `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock (CI-defended); `ReceiveEvent` taxonomy has no constructor for locally-originated outputs; `receive_apply` is staged-then-committed; `RollBackward` arm returns `Err(RollbackOutOfScope)`; `RollForward` arm mutates only `state.pending_headers`. | Other BLUE crates / submodules only. **N-H:** receive reducer composes `block_validity` (B1) via `admit_via_block_validity` — no direct dep on `ade_runtime`. The `ChainDbWrite` trait surface is the only consumer-facing seam (impls live in RED/GREEN crates). | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. **N-H:** no `*SigningKey` / `KesSecret` / `ColdSigningKey` types (carried). |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-H:** `events_to_state::{lift_chain_sync_signal, lift_block_fetch_event}` are pure pass-through translators — MUST NOT decode `header_bytes` / `block_bytes` (CI-defended); `in_memory_chain_write::ChainDbWriter` is the single production impl of `ChainDbWrite` over any `ChainDb`. | BLUE crates + standard library + ecosystem crates. **N-H:** the GREEN adapters live inside `ade_runtime` (RED crate) — color is per-module per the cluster TCB Color Map. | `ade_runtime` for `ade_testkit`; RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. **N-H:** `ade_runtime::receive::orchestrator` is the per-peer receive session driver — pure state-machine dispatch; socket I/O lives one layer up; **MUST NOT import from `ade_runtime::producer::{signing, broadcast, scheduler}`** (CI-defended). | Any BLUE / GREEN crate or submodule (one-way). **N-H strengthened the `ade_runtime → ade_ledger` edge** (RED → BLUE via the receive chokepoints + AdmittedBlock token + ChainDbWrite trait). | Cannot be depended on by BLUE. Receive paths additionally cannot link against `producer::{signing, broadcast, scheduler}`. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for receive-paths-domain sub-modules, model the new CI
   gate on `ci_check_receive_reducer_closure.sh` /
   `ci_check_admitted_block_closure.sh` /
   `ci_check_receive_orchestrator_no_producer_dep.sh` shape (closure
   proof + admission-token closure + reducer state-isolation proof
   + scope-edge structural-failure proof + key-boundary proof +
   BTreeMap-only proof).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   receive-paths-domain authority rules, append `DC-CONS-1X` /
   `CN-CONS-0X` / `CN-PROTO-0X` / `DC-PROTO-0X` with bidirectional
   cross-ref to consumed rules. `T-DET-01` / `T-ENC-01` may receive a
   `strengthened_in` entry when the new module participates in their
   byte-deterministic / byte-authoritative properties.
7. **New operator-action probe binary:** add to
   `crates/ade_core_interop/src/bin/<name>.rs` following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape; document in `<cluster>/CE-<id>_PROCEDURE.md`; capture
   evidence to `<cluster>/CE-<id>_<date>.log` OR mark
   `blocked_until_operator_peer_available` /
   `blocked_until_operator_stake_available` as appropriate.
8. **Cross-cluster obligation:** follow the binding rules from the
   N-E full-close narrative; N-G strengthened the rules with the
   second `blocked_*` variant; N-H carries the variant into a
   second instance.
9. **Cluster scope-edge:** if the cluster deliberately scopes down a
   derived constraint, ship the structured-failure variant +
   registry `open_obligation` + cluster-doc handoff per the N-H
   precedent.
10. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-H — FULLY CLOSED at this HEAD** (mechanical half +
  structural cross-impl; Path A admit-only scope): code + CI gates
  + CN-PROTO-07 + CN-CONS-08 + DC-CONS-19 + DC-PROTO-09 + DC-CONS-20
  (declared with `rollback_side_blocked_until_ledger_snapshot_cluster`)
  + RO-LIVE-02 (partial) + 5 new CI scripts. CE-N-H-6 live-evidence
  is `blocked_until_operator_peer_available` per
  `RO-LIVE-02.open_obligation` — re-opens on operator availability.
- **PHASE4-N-G — FULLY CLOSED** (carried). CE-N-G-8 live-evidence
  is `blocked_until_operator_peer_available`.
- **PHASE4-N-C — FULLY CLOSED** (carried). CE-N-C-8 live-evidence
  is `blocked_until_operator_stake_available`.
- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED** (carried).
- **PHASE4-N-E — FULLY CLOSED** (carried).
- **NEW PRIORITY-1 future cluster — Full rollback authority** *(NEW
  HIGHEST-PRIORITY candidate flagged by N-H close — DC-CONS-20
  rollback half closure)*: BLUE `rollback_apply` chokepoint + a
  ledger-state snapshot store (encode/decode of `LedgerState` +
  `PraosChainDepState`) + replay-forward driver. Wires
  `ReceiveEvent::RollBackward` from `Err(RollbackOutOfScope)` to
  `Ok(ReceiveEffect::RolledBack { to_slot })`. Surface for the next
  planner; do not invent invariants here.
- **NEW future cluster — Multi-peer fork choice** *(NEW candidate
  flagged by N-H close — OQ-4 lock; sequenced after rollback)*:
  Praos longest-chain across competing `PerPeerReceiveState[]`.
- **NEW future cluster — N2C local-chain-sync receive surface**
  *(NEW candidate flagged by N-H close)*: operator-side N2C clients
  consume Ade's chain via `LocalChainSyncMessage`.
- **Future cluster — `CE-N-H-6` live evidence re-open trigger**:
  reopens when a private cardano-node peer is provisioned; the
  procedure is documented at
  `docs/clusters/completed/PHASE4-N-H/CE-N-H-6_PROCEDURE.md`.
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
- **(N-C-S1 / S2 / S3 / S4 / S5 / S6 / S7 specific)** All carried.
- **(N-G-S1 / S2 / S3 / S4 specific)** All carried.
- **(N-H-S1 specific — `AdmittedBlock` private-constructor token +
  closed `ReceiveEvent`/`ReceiveEffect`/`ReceiveError` sums +
  `PendingHeaderCache` + `ChainDbWrite` narrow trait)** No `pub fn`
  with return type `AdmittedBlock` outside
  `crates/ade_ledger/src/receive/admitted.rs`'s
  `admit_via_block_validity` (CI-defended by
  `ci_check_admitted_block_closure.sh`). `AdmittedBlock.bytes` MUST
  be private. `ReceiveEvent` MUST have no constructor for
  locally-originated chain-sync / block-fetch outputs (CN-PROTO-07
  by construction — exhaustive match round-trip test fails to
  compile if a non-peer-originated variant is added). No
  `#[non_exhaustive]` on any of the closed sums. No `String`-bearing
  variant. No `HashMap`/`HashSet` in
  `crates/ade_ledger/src/receive/pending_header_cache.rs`
  (BTreeMap-only).
- **(N-H-S2 specific — closed BLUE receive reducer)** No
  `HashMap`/`HashSet`/`tokio`/`rand`/`std::time` in
  `crates/ade_ledger/src/receive/reducer.rs` production code. No
  `RollForward` arm path may mutate `state.ledger`,
  `state.chain_dep`, or call `chain_write.*` (Invariant I-6,
  CI-defended). No `RollBackward` arm path may return `Ok` (Path A
  scope edge — CI-defended). Positive presence: `receive_apply`,
  `receive_apply_sequence`, `ReceiveState` MUST exist. Defended by
  `ci_check_receive_reducer_closure.sh`.

### GREEN (`ade_testkit` incl. `producer` + `receive_paths` corpora; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges; `ade_runtime::producer::{tick_assembler, broadcast_to_served, served_chain_lookups}`; **`ade_runtime::receive::{events_to_state, in_memory_chain_write}` — NEW in N-H-S3**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (carried bullets per prior revision)
- **(`ade_runtime::producer::{broadcast_to_served, served_chain_lookups}`,
  N-G-S5)** Carried.
- **(`ade_runtime::receive::events_to_state`, NEW in N-H-S3 —
  DC-PROTO-09)** Pure pass-through; no I/O; no clocks; no
  nondeterminism. MUST NOT decode `header_bytes` or `block_bytes`
  (the BLUE reducer is the canonical decode site). Non-state-changing
  N-A variants return `None`. Defended by
  `ci_check_receive_replay_purity.sh`.
- **(`ade_runtime::receive::in_memory_chain_write`, NEW in N-H-S3)**
  Single production impl of `ChainDbWrite` over any `ChainDb`.
  Pure (the I/O is the wrapped `ChainDb`). The `decode_block` call
  is reachable safely under the `AdmittedBlock` invariant
  (`block_validity` already validated the bytes).

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. N-C S7 probe binary `live_block_production_session`, N-G S7 probe binary `live_block_fetch_session`, and **N-H S6 probe binary `live_block_follow_session` — NEW**), and the RED-behavior `ade_ledger::consensus_input_extract` scan; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` (N-C-S1/S6); `ade_runtime::network::n2n_server` (N-G-S6); **`ade_runtime::receive::orchestrator` — NEW in N-H-S4**)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`, the `block_validity` / `tx_validity`
  composers, `mempool::ingress::mempool_ingress`, **the producer
  authority chokepoints `producer::forge::forge_block` +
  `producer::self_accept::self_accept`** (N-C), **the served-chain
  authority chokepoint `producer::served_chain::served_chain_admit`**
  (N-G), or **the receive authority chokepoint
  `receive::reducer::receive_apply` + `receive::admitted::admit_via_block_validity`**
  (N-H).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
  **(N-C-strengthened)** Constructing `AcceptedBlock` outside
  `self_accept` is CI-forbidden. **(N-G-strengthened)** Constructing
  `ServedChainSnapshot` populated entries outside `served_chain_admit`
  is CI-forbidden; constructing `ServerReply` variants for
  client-agency wire messages is unrepresentable in the public API
  (CN-PROTO-06). **(N-H-strengthened)** Constructing `AdmittedBlock`
  outside `admit_via_block_validity` is CI-forbidden (the inner
  `bytes` field is private; the tuple-struct constructor is
  module-private; defended by `ci_check_admitted_block_closure.sh`).
  Constructing `ReceiveEvent` for locally-originated chain-sync /
  block-fetch outputs (the orchestrator's own client requests) is
  unrepresentable in the public API (CN-PROTO-07).
- (`ade_runtime` specifically) Existing `ade_runtime → ade_ledger`
  edge (added N-C; strengthened N-G) is **further strengthened in
  N-H** — the receive orchestrator + GREEN adapters consume the new
  `ade_ledger::receive::*` BLUE chokepoints. Pass
  `ci_check_dependency_boundary.sh`.
- (`ade_network::mux::transport`) No protocol logic.
- (`ade_network::session`) Composition glue only.
- (`ade_network::bin::capture_*`) Live-interop tools only.
- (`ade_runtime::consensus::genesis_parser`) No re-derivation of the
  bootstrap anchor outside `compute_anchor_hash`.
- (`ade_ledger::consensus_input_extract`) Pure-over-bytes.
- (N-E live N2N operator-action session) Carried.
- (Deferred RED operator-action surfaces — CE-NODE-N2C-LTX) Carried.
- (`ade_core_interop`) Live-interop driver only; library tests
  `#[ignore]`-gated. **N-H added `live_block_follow_session`** —
  fifth operator-action probe binary. The binary's default mode
  prints readiness and exits; `--connect` performs the live pass
  against a real cardano-node peer.
- **(N-C-S1 / S6 specific — `ade_runtime::producer::{signing, keys,
  scheduler, broadcast}`)** All carried.
- **(N-G-S6 specific — `ade_runtime::network::n2n_server`)** Carried.
  Key-boundary forbids imports from
  `ade_runtime::producer::signing`.
- **(N-H-S4 specific — `ade_runtime::receive::orchestrator`)** Pure
  state-machine driver — NO socket I/O at this layer (sockets live
  one layer up). MUST NOT import from
  `ade_runtime::producer::{signing, broadcast, scheduler}` —
  defended by `ci_check_receive_orchestrator_no_producer_dep.sh`.
  Per-peer state independent; cross-peer coordination only via the
  shared `ChainDb`. Decoded inbound frames MUST go through the BLUE
  reducer (no inline grammar). Lifted events MUST go through the
  GREEN `events_to_state` adapter (no inline N-A-signal → ReceiveEvent
  translation).
- **(N-H-S6 specific — `live_block_follow_session`)** The live
  socket loop MUST drive the RED receive orchestrator →
  `dispatch_chain_sync_inbound` / `dispatch_block_fetch_inbound`
  pipeline through the canonical chokepoints — no parallel
  admission path, no direct construction of `AdmittedBlock` outside
  the BLUE reducer's call to `admit_via_block_validity`, no bypass
  of `ChainDbWrite::write_admitted`. The live evidence log committed
  alongside the procedure doc redacts hostnames per
  `feedback_no_credential_leaks`.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-H-strengthened:** no
  private-key bytes in receive-paths fixture corpora (defended by
  `ci_check_no_private_keys_in_corpus.sh` extended to cover the new
  `receive_paths` fixture root).
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-H:** the
  mechanical cross-impl harness in
  `crates/ade_runtime/tests/receive_pipeline_corpus_drive.rs` is a
  structural-agreement harness (every Conway-576 corpus block
  admits; ChainDb tip + admitted bytes + ledger fingerprint agree);
  the live cross-impl claim requires operator-action live evidence
  per CE-N-H-6.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The receive-side admission authority
  surface (admit-only Path A; admit chokepoint + reducer +
  pending-header cache + chain-write trait + GREEN adapters + RED
  orchestrator) is Tier 1; the five new CI gates enforce mechanical
  closure. **The N-H `blocked_until_operator_peer_available` status
  is NOT a "we'll match it later" stub** — the mechanical half is
  fully enforced at this HEAD; the live half is recorded as an
  `open_obligation` on `RO-LIVE-02`, tied to a specific
  operator-action procedure (`CE-N-H-6_PROCEDURE.md`), and reopens
  on a named external dependency (private cardano-node peer
  provisioned by the operator). Same closure-mode variant as N-G.
  **Likewise the `DC-CONS-20` Path A scope-edge declaration is NOT
  a stub** — it is a structured `ReceiveError::RollbackOutOfScope`
  variant reachable on every `RollBackward` event, plus a registry
  `open_obligation = rollback_side_blocked_until_ledger_snapshot_cluster`
  naming the follow-on cluster.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; pending the regen,
  CODEMAP may pin pre-N-H HEAD `a280954`. The new BLUE submodules
  (`ade_ledger::receive::{admitted, chain_write, events,
  pending_header_cache, reducer}`), the new GREEN submodules
  (`ade_runtime::receive::{events_to_state, in_memory_chain_write}`),
  the new RED submodule (`ade_runtime::receive::orchestrator`), and
  the new operator-action probe binary
  (`ade_core_interop::bin::live_block_follow_session`) are not yet
  in the prior CODEMAP. The next CODEMAP regen picks these up
  mechanically. CI count moves from 47 → 52.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-H added:**
  `CN-PROTO-07` (`enforced`, `ci_script =
  ci/ci_check_admitted_block_closure.sh`,
  `introduced_in = PHASE4-N-H`); `CN-CONS-08` (`enforced`,
  `ci_script = ci/ci_check_receive_reducer_closure.sh`);
  `DC-CONS-19` (`enforced`, `ci_script =
  ci/ci_check_receive_reducer_closure.sh`); `DC-CONS-20`
  (`declared` + `open_obligation =
  rollback_side_blocked_until_ledger_snapshot_cluster`, `ci_script =
  ci/ci_check_receive_reducer_closure.sh` (admit half only));
  `DC-PROTO-09` (`enforced`, `ci_script =
  ci/ci_check_receive_replay_purity.sh`); `RO-LIVE-02`
  (`partial` + `open_obligation =
  blocked_until_operator_peer_available`, `ci_script =
  ci/ci_check_receive_paths_corpus_present.sh`); appended
  `PHASE4-N-H` to `T-DET-01.strengthened_in`,
  `T-ENC-01.strengthened_in`, `DC-CONS-13.strengthened_in`,
  `DC-CONS-16.strengthened_in`, `CN-CONS-07.strengthened_in`,
  `DC-PROTO-06.strengthened_in`. Total: 196 → 202 entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Receive-side bridge invariants sketch:
  `docs/planning/receive-side-bridge-invariants.md` (the upstream
  sketch the cluster doc derives from; Invariants I-1..I-12 +
  Anti-invariants ¬P-1..¬P-7).
- Cluster N-D / N-A / N-B / B1 / B2 / B3 / B4 / B5 /
  OQ5-CREDENTIAL-FIDELITY / COMMITTEE-CRED-FIDELITY /
  DREP-VOTE-FIDELITY / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK / PHASE4-N-E /
  PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C / PHASE4-N-G: all closed;
  cluster docs carried.
- **Cluster PHASE4-N-H (CLOSED + archived at this HEAD; mechanical
  half + structural cross-impl; Path A admit-only scope)**: the
  cluster doc + slices `cluster.md, N-H-S{1..6}.md` +
  `CE-N-H-6_PROCEDURE.md` at
  `docs/clusters/completed/PHASE4-N-H/`. WIRES AND CLOSES the
  receive-side header→body bridge end-to-end (admit-only Path A
  scope): BLUE `AdmittedBlock` token + receive closed sums +
  `PendingHeaderCache` + `ChainDbWrite` trait (S1), BLUE
  `receive_apply` reducer composing `block_validity` (S2), GREEN
  `events_to_state` + `in_memory_chain_write` + session-transcript
  replay (S3), RED per-peer receive orchestrator + multi-peer
  independence (S4), mechanical cross-impl pipeline drive (S5),
  operator-action `live_block_follow_session` probe binary +
  CE-N-H-6 procedure (S6). Added five CI scripts (count 47 → 52);
  added six derived / release registry rules (total 196 → 202);
  strengthened four carried rules + two universal rules.
  **CE-N-H-6 live-evidence `blocked_until_operator_peer_available`**
  per `RO-LIVE-02.open_obligation`; mechanical bytes-shape claim is
  closed by the cross-impl pipeline drive.
  **`DC-CONS-20` rollback half `declared` with
  `rollback_side_blocked_until_ledger_snapshot_cluster`** —
  candidate seam surfaced for the next planner. Five operator-
  action probe binaries now in the family: `live_consensus_session`
  (N-B), `live_tx_submission_session` (N-E S6),
  `live_block_production_session` (N-C S7), `live_block_fetch_session`
  (N-G S7), `live_block_follow_session` (N-H S6).
- **Future obligation: `CE-N-H-6`** — operator-action live evidence
  for live cross-impl follow-mode admission by Ade of cardano-node-
  served blocks; reopens on private peer availability.
- **Future obligation: `DC-CONS-20` rollback-half closure** — full
  rollback authority cluster: BLUE `rollback_apply` chokepoint +
  ledger-state snapshot store (encode/decode) + replay-forward
  driver. **Highest-priority next-cluster candidate seam.**
- **Future obligation: `CE-N-G-8`** — carried.
- **Future obligation: `CE-N-C-8`** — carried.
- **Future obligation: `CE-NODE-N2C-LTX`** — carried from N-E.
- **Future seam candidates (flagged by N-H close)**: full rollback
  authority cluster (highest priority — DC-CONS-20 rollback half);
  multi-peer fork choice cluster (sequenced after rollback);
  N2C local-chain-sync receive surface cluster.
