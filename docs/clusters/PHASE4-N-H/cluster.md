# Cluster PHASE4-N-H — Receive-side header→body bridge (admit-only)

> **Status:** Planning artifact (non-normative). Strengthens
> `T-DET-01`, `T-ENC-01`, `DC-CONS-13`, `DC-CONS-16`, `CN-CONS-07`,
> `DC-PROTO-06`. Introduces `CN-CONS-08`, `DC-CONS-19`, `DC-CONS-20`,
> `DC-PROTO-09`, `CN-PROTO-07`, `RO-LIVE-02` (already declared at
> sketch close; this cluster flips most to `enforced`). `DC-CONS-20`
> stays `declared` with a Path A rollback-side cross-cluster
> obligation. Produced from
> `docs/planning/receive-side-bridge-invariants.md` and
> `docs/planning/phase4-n-h-cluster-slice-plan.md`. If this doc
> conflicts with the registry / specs, those win.

---

## Primary invariant

> Peer-supplied header+body bytes are admitted to ChainDb +
> LedgerState + PraosChainDepState **only** via `block_validity` (B1)
> producing an `AdmittedBlock` token; the receive bridge reducer is
> pure, total, and deterministic over canonical inputs; `RollForward`
> caches header bytes without mutating any sub-state; body admission
> requires a header cross-check first; `RollBackward` returns
> `Err(ReceiveError::RollbackOutOfScope)` with state unchanged
> (Path A scope edge — full rollback authority is a follow-on
> cluster).

## Normative anchors

- `docs/ade-invariant-registry.toml` — `T-DET-01` (strengthened in
  N-H), `T-ENC-01` (strengthened in N-H), `DC-CONS-13` (strengthened
  in N-H), `DC-CONS-16` (strengthened in N-H), `DC-PROTO-06`
  (strengthened in N-H), `CN-CONS-07` (strengthened in N-H), and the
  6 new entries `CN-CONS-08`, `DC-CONS-19`, `DC-CONS-20`,
  `DC-PROTO-09`, `CN-PROTO-07`, `RO-LIVE-02`.
- Project constitution §2 (`T-DET-01`, `T-ENC-01`, Byte Authority
  Model; FC/IS).
- IDD `~/.claude/methodology/idd.md` Part I §§4, §5, §6, §9.
- Ouroboros mini-protocol specs (chain-sync, block-fetch) — the
  client-role events arrive from a peer we've already negotiated with
  via handshake.
- `docs/planning/receive-side-bridge-invariants.md` §§1–8 (sketch
  with Path A scope decisions).

## OQ resolutions (locked — see invariants sketch §7)

- **OQ-1 (Path A scope)** — RESOLVED. Receive cluster ships
  admit-only. `RollBackward` is a structured
  `RollbackOutOfScope` error; receive state stays consistent. A
  follow-on rollback cluster closes DC-CONS-20's rollback half.
- **OQ-2 (PendingHeaderCache size + eviction)** — Resolved BLUE
  structural. Cache is `BTreeMap<(SlotNo, Hash32), Vec<u8>>` with no
  hard size bound at this layer; eviction policy is an operator
  config injected as canonical input on the orchestrator side
  (S4). The reducer itself never evicts — eviction is a deterministic
  RED concern (slot-window-keyed, replay-stable).
- **OQ-3 (AdmittedBlock construction site)** — Resolved. Lives at
  `ade_ledger::receive::admitted::admit_via_block_validity` — the
  only public entry that returns `AdmittedBlock`. Module-private
  struct constructor; CI grep gate enforces single site.
- **OQ-4 (multi-peer fork choice)** — Out of scope. Single-source
  follow only; Praos longest-chain is a future cluster.
- **OQ-5 (replay corpus shape)** — Synthetic single-peer streams +
  Conway-576 corpus for the mechanical adapter (S5).
- **OQ-6 (live-evidence binary)** — New binary
  `live_block_follow_session` mirrors the N-C / N-G pattern.

## Grounding (verified at HEAD `2adfb45`)

- **N-A signal/event sources (BLUE, unchanged):**
  - `ade_network::chain_sync::signal::ForkChoiceSignal` — 4 closed
    variants: `RollForward { header_bytes, tip }`, `RollBackward`,
    `Intersected`, `NoIntersection`.
  - `ade_network::block_fetch::event::BatchDeliveryEvent` — 4 closed
    variants: `BatchStarted`, `BlockDelivered { block_bytes }`,
    `NoBlocks`, `BatchCompleted`.
- **B1 admission authority (BLUE, unchanged):**
  - `ade_ledger::block_validity::transition::block_validity` returns
    `BlockValidityOutcome { verdict, ledger, chain_dep }`. On
    `Valid`, the outcome carries the evolved sub-states. This is
    Ade's single block-admission gate.
- **N-D ChainDb (Tier 1 trait):**
  - `ade_runtime::chaindb::ChainDb::{put_block, get_block_by_*, tip,
    iter_from_slot, rollback_to_slot}`. `InMemoryChainDb` +
    `PersistentChainDb` impls present. Block-store rollback is
    available even though ledger rollback is not.
- **Existing canonical header projection (N-G S1):**
  - `ade_ledger::block_validity::accepted_block_header_bytes`
    accepts an `AcceptedBlock` — needs a sibling that walks raw
    block bytes for cross-check use (the receive side has bytes from
    the wire before any token exists). Decision: add a small private
    helper in `receive::reducer` that walks the same envelope+header
    recipe via `decode_block_envelope` + `cbor::skip_item`, but does
    not expose a parallel public splitter (so the
    `ci_check_no_parallel_header_splitter.sh` gate stays green).
- **No `ade_ledger::receive::*` module exists at HEAD** — N-H is
  greenfield for the receive bridge.
- **No consumer of `ForkChoiceSignal` or `BatchDeliveryEvent` exists
  outside their defining crate at HEAD** — confirmed by repo grep.
- **Trailer ratio at HEAD `2adfb45`:** 82.86%.

## Entry Conditions

- **PHASE4-N-A closed** — Mini-protocol codecs + signal/event
  taxonomies + per-protocol agency types exist.
- **PHASE4-N-B closed** — Header validator + Praos chain-dep
  evolution exists; rollback authority primitive
  (`apply_rollback`) exists (caller-driven materialization — not
  used in N-H per Path A scope).
- **PHASE4-N-C closed** — `AcceptedBlock` precedent for
  private-constructor admission tokens. N-H's `AdmittedBlock` is a
  separate symmetric token; no shared code path with `AcceptedBlock`.
- **PHASE4-N-D closed** — `ChainDb` trait + impls; `SnapshotStore`
  trait + impls (latter used by a future rollback cluster, not N-H).
- **PHASE4-N-G closed** — Producer-side server pump shipped;
  receive-side is the mirror. `accepted_block_header_bytes` exists
  as the canonical header splitter (we reuse the recipe by
  walking it inline in the receive reducer — same recipe, same
  canonical site).
- **PHASE4-B1..B5 closed** — `block_validity` is the single
  admission authority.
- **Constitution-coverage gate PASSES** at HEAD:
  `bash ci/ci_check_constitution_coverage.sh`.

## Exit Criteria (CI-Verifiable)

Each CE names the test or check that proves it.

- **CE-N-H-1 (token + closed sums + cache)** — Named tests:
  - `admitted_block_constructor_is_module_private` (S1) — runtime
    witness paired with the CI grep gate.
  - `receive_event_is_closed` / `receive_effect_is_closed` /
    `receive_error_is_closed` (S1) — exhaustive `match` over each
    enum variant compiles and no `_` arm is needed.
  - `pending_header_cache_is_btreemap_backed` (S1).
  - `cn_proto_07_no_locally_originated_event_constructor` (S1) — the
    closed `ReceiveEvent` sum has no constructor for client-output
    events; mechanical via the grep gate.
  - CI: `ci/ci_check_admitted_block_closure.sh` (S1) — forbids any
    `pub fn .*-> *AdmittedBlock` outside the canonical site.
  - Registry flip on close: `CN-PROTO-07` → `enforced`.

- **CE-N-H-2 (reducer)** — Named tests:
  - `receive_apply_roll_forward_caches_header_without_state_mutation`
    (S2) — ledger / chain_dep / chaindb-fingerprint unchanged after
    `RollForward`.
  - `receive_apply_block_delivered_with_matching_header_admits` (S2)
    — corpus block; admission emits `Admitted { slot, hash }` and
    all three sub-states advance.
  - `receive_apply_block_delivered_with_no_cached_header_rejects`
    (S2) — `HeaderBodyMismatch`.
  - `receive_apply_block_delivered_with_mismatched_cached_header_rejects`
    (S2) — `HeaderBodyMismatch`.
  - `receive_apply_block_delivered_validity_invalid_rejects` (S2) —
    corrupted body bytes; `Err(Validity(_))`; state unchanged.
  - `receive_apply_rollback_returns_out_of_scope` (S2) — receive
    state unchanged; `Err(RollbackOutOfScope { target_point })`.
  - `receive_apply_replay_byte_identical_over_corpus` (S2).
  - CI: `ci/ci_check_receive_reducer_closure.sh` (S2).
  - Registry flip on close: `CN-CONS-08`, `DC-CONS-19` → `enforced`.

- **CE-N-H-3 (GREEN adapter + replay)** — Named tests:
  - `events_to_state_is_pure_no_io` (S3, CI grep).
  - `events_to_state_lifts_roll_forward_to_receive_event` (S3).
  - `events_to_state_lifts_block_delivered_to_receive_event` (S3).
  - `in_memory_chain_write_admits_via_admitted_block` (S3).
  - `receive_session_transcript_replay_byte_identical` (S3
    integration) — DC-PROTO-09 closure.
  - CI: `ci/ci_check_receive_replay_purity.sh` (S3); extend
    `ci/ci_check_no_private_keys_in_corpus.sh` to the new fixture
    root.
  - Registry flip on close: `DC-PROTO-09` → `enforced`.

- **CE-N-H-4 (RED orchestrator)** — Named tests:
  - `dispatch_chain_sync_inbound_decodes_then_caches` (S4).
  - `dispatch_block_fetch_inbound_decodes_then_admits` (S4).
  - `dispatch_chain_sync_inbound_threads_negotiated_version` (S4).
  - `dispatch_rejects_undecodable_input` (S4).
  - `two_synthetic_peers_preserve_per_session_transcripts_receive`
    (S4 integration) — multi-peer determinism with a shared ChainDb.
  - CI: `ci/ci_check_receive_orchestrator_no_producer_dep.sh` (S4).

- **CE-N-H-5 (mechanical cross-impl)** — Named tests:
  - `receive_pipeline_corpus_drive_admits_every_block` (S5
    integration).
  - `receive_pipeline_corpus_drive_chaindb_tip_matches_expected` (S5).
  - `receive_pipeline_corpus_drive_admitted_bytes_equal_corpus_bytes`
    (S5).
  - `receive_pipeline_corpus_drive_ledger_fingerprint_matches_expected`
    (S5).

- **CE-N-H-6 (live evidence — conditional)** — Either:
  - (a) `docs/clusters/PHASE4-N-H/CE-N-H-LIVE_<date>.log` captures a
    real cardano-node follow over N blocks with ChainDb tip equal
    to peer tip at every step, AND
    `docs/clusters/PHASE4-N-H/CE-N-H-6_PROCEDURE.md` documents the
    procedure; OR
  - (b) Procedure doc records `blocked_until_operator_peer_available`
    with the specific blocker (no private cardano-node peer wired
    at HEAD `<hash>`) and a re-open obligation.
  - Registry flip on close: `RO-LIVE-02` → `enforced` (case a) or
    `partial` + `open_obligation` (case b).

## Slice index

| Slice | One-line scope | TCB |
|----|----|----|
| **N-H-S1** | BLUE `AdmittedBlock` private-constructor token + `ReceiveEvent` / `ReceiveEffect` / `ReceiveError` closed sums + `PendingHeaderCache` + `ChainDbWrite` narrow trait. | BLUE |
| **N-H-S2** | BLUE `receive_apply` reducer composing `block_validity` + header cross-check; `RollBackward` returns `Err(RollbackOutOfScope)`. | BLUE |
| **N-H-S3** | GREEN `events_to_state` adapter + `in_memory_chain_write` adapter + session-transcript replay corpus + replay test. | GREEN + test |
| **N-H-S4** | RED per-peer receive orchestrator; multi-peer determinism. | RED |
| **N-H-S5** | Mechanical cross-impl adapter driving Conway-576 corpus through the full receive pipeline. | RED/test |
| **N-H-S6** | Operator-action `live_block_follow_session` binary + CE-N-H-6 procedure. | RED |

## TCB Color Map (FC/IS Partition)

**BLUE (new):**
- `ade_ledger::receive::admitted` — `AdmittedBlock`,
  `admit_via_block_validity` (S1).
- `ade_ledger::receive::events` — `ReceiveEvent` / `ReceiveEffect`
  / `ReceiveError` closed sums (S1).
- `ade_ledger::receive::pending_header_cache` —
  `PendingHeaderCache` (S1).
- `ade_ledger::receive::chain_write` — `ChainDbWrite` narrow trait
  (S1).
- `ade_ledger::receive::reducer` — `ReceiveState`,
  `receive_apply` (S2).

**GREEN (new):**
- `ade_runtime::receive::events_to_state` — pure adapter (S3).
- `ade_runtime::receive::in_memory_chain_write` — `ChainDbWrite`
  impl backed by `InMemoryChainDb` (S3).
- `ade_testkit::receive_paths::{fixtures, replay}` — corpus +
  replay scaffolding (S3).

**RED (new):**
- `ade_runtime::receive::orchestrator` — per-peer dispatch (S4).
- `ade_core_interop::bin::live_block_follow_session` — operator
  evidence binary (S6).

**Color rules:**
- No RED behavior in BLUE — enforced by per-slice CI gates +
  existing `ci_check_forbidden_patterns.sh`.
- No producer-side dep from the receive orchestrator — enforced by
  `ci/ci_check_receive_orchestrator_no_producer_dep.sh` (S4).
- No HashMap in BLUE — enforced by per-slice closure gates.

## Forbidden during this cluster

- Any new `pub fn` returning `AdmittedBlock` outside
  `ade_ledger::receive::admitted::admit_via_block_validity`. CI:
  `ci_check_admitted_block_closure.sh`.
- Any path in the receive reducer (`receive_apply`) that mutates
  any of (ledger, chain_dep, chaindb-write trait calls) from a
  `RollForward` branch.
- Any `pub fn` in `ade_runtime::receive::orchestrator` that imports
  `producer::signing` / `producer::broadcast` /
  `producer::scheduler`. CI:
  `ci_check_receive_orchestrator_no_producer_dep.sh`.
- HashMap / HashSet / wall-clock / tokio / rand in any BLUE or
  GREEN receive module.
- `RollBackward` returning `Ok` from `receive_apply` — the only
  reducer arm for that variant returns `Err(RollbackOutOfScope)`.
- Replay corpus carrying private-key bytes — existing
  `ci/ci_check_no_private_keys_in_corpus.sh` extended in S3 to
  cover the new fixture root.
- `git commit --no-verify`.

## Replay obligations introduced by this cluster

- **New canonical replay corpus**:
  `crates/ade_testkit/fixtures/receive_paths/` — ordered
  `(initial_state, ReceiveEvent_sequence) -> expected_state`
  triples. Drives `receive_session_transcript_replay_byte_identical`
  and `receive_pipeline_corpus_drive_*`.
- **`T-DET-01` strengthening**: PHASE4-N-H — new
  authoritative-deterministic surface (receive-transcript
  reduction).
- **`T-ENC-01` strengthening**: PHASE4-N-H — peer-supplied wire
  bytes flow into ChainDb verbatim.
- **`DC-CONS-13` strengthening**: PHASE4-N-H — symmetric receive
  closure (admit = `block_validity::Valid` only).
- **`CN-CONS-07` strengthening**: PHASE4-N-H — broadcast gate's
  mirror via `AdmittedBlock`.
- **`DC-CONS-16` strengthening**: PHASE4-N-H — header projection
  recipe reused for receive-side cross-check.
- **`DC-PROTO-06` strengthening**: PHASE4-N-H — version threaded
  through the receive reducer's call site (the orchestrator passes
  the handshake-negotiated version on every reducer call).

## Authority reminder

This document is a planning aid only. All correctness rules live in
the project's normative specifications and the invariant registry.

> **Normative documents + registry + CI enforcement win.**
