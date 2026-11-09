# Cluster PHASE4-N-G — Producer-side Server Paths (chain-sync + block-fetch server response)

> **Status:** Planning artifact (non-normative). Strengthens `T-DET-01`,
> `T-ENC-01`, `DC-CONS-16`, `CN-CONS-07`, `DC-PROTO-06` (no new
> constitutional rule). Introduces `DC-CONS-17/18`, `DC-PROTO-07/08`,
> `CN-PROTO-06`, `RO-LIVE-01` (all `status="declared"` at cluster
> entry; flip to `enforced` as the named slices land). Produced from
> `docs/planning/phase4-n-a-successor-invariants.md` and
> `docs/planning/phase4-n-g-cluster-slice-plan.md`. If this doc
> conflicts with the registry / specs, those win.

---

## Primary invariant

> Producer-side chain-sync + block-fetch server-role responses are
> (a) deterministic across replays over canonical
> `(negotiated_version, peer_message_sequence,
> broadcast_arrival_sequence, session_event_sequence)` inputs;
> (b) byte-identical to the `AcceptedBlock` bytes admitted via
> `BroadcastQueue` ↔ `ServedChainSnapshot` — never re-encoded, never
> reconstructed from parsed pieces, never served via a parallel
> header/body splitter; and (c) only ever Server-agency-tagged
> messages, enforced at the type level by a closed `ServerReply<M>`
> wrapper. Private-key custody remains RED-confined; no server
> response path links against `ade_runtime::producer::signing`.

## Normative anchors

- `docs/ade-invariant-registry.toml` — `T-DET-01` (strengthened in
  N-G), `T-ENC-01` (strengthened in N-G), `DC-CONS-16` (strengthened
  in N-G), `DC-CONS-17/18` (new), `DC-PROTO-06` (strengthened in N-G),
  `DC-PROTO-07/08` (new), `CN-CONS-07` (strengthened in N-G),
  `CN-PROTO-06` (new), `RO-LIVE-01` (new).
- Project constitution §2 (`T-DET-01`, `T-ENC-01`, Byte Authority
  Model; Functional Core / Imperative Shell).
- IDD `~/.claude/methodology/idd.md` Part I §§4 (determinism), §5
  (replay), §6 (closed semantic surfaces), §7 (versioned evolution),
  §9 (FC/IS partition).
- Ouroboros mini-protocol specs (chain-sync, block-fetch) — message
  grammars are closed; per-protocol agency is locked decision §7 #7
  from PHASE4-N-A.
- `docs/planning/phase4-n-a-successor-invariants.md` §§1–8 (sketch
  with scope decisions).

## OQ resolutions (locked — see invariants sketch §7)

- **OQ-1 (served-chain scope)** — Narrow. The producer serves only
  blocks admitted from this session's `AcceptedBlock` /
  `BroadcastQueue` path. Broader serving from a co-located N-D
  ChainDB is **out of scope** here and is reserved for a later
  N-D-bridge / served-chain-index cluster. Decided in sketch §"Scope
  decisions" #1.
- **OQ-2 (header projection authority)** — Reuse the validator-shared
  body-hash recipe's existing header/body byte split. Surveyed in S1;
  see Grounding below. No parallel splitter; enforced by
  `ci/ci_check_no_parallel_header_splitter.sh` (introduced by S1).
- **OQ-3 (FindIntersect under narrow scope)** — A `FindIntersect`
  with no point in the served chain returns `IntersectNotFound`. A
  `FindIntersect` whose points include a `(slot, hash)` admitted to
  `ServedChainSnapshot` returns `IntersectFound { point, tip }` over
  that point. No synthetic anchor.
- **OQ-4 (multiple concurrent peers)** — Per-peer independent
  `ProducerChainSyncServerState` + `ProducerBlockFetchServerState`;
  all coordination across peers goes through the single
  `ServedChainSnapshot`. Tested by S6's "two synthetic peers"
  determinism test.
- **OQ-5 (eviction)** — None within this cluster (narrow scope; the
  served chain only ever grows during the session). Memory bound is
  session lifetime; persistence is N-D-bridge scope.
- **OQ-6 (live-evidence binary peer)** — A private cardano-node peer
  using the existing N2N handshake. Conditional via
  `blocked_until_operator_peer_available` if no peer is available at
  cluster close.
- **OQ-7 (server agency progress shape)** — Deterministic resolution
  per `DC-PROTO-08`: legal `RollForward` / `RollBackward` /
  `AwaitReply` / structured close-or-error. The numeric slot-tick SLA
  is **deliberately not** a load-bearing invariant in this cluster.
- **OQ-8 (N2N vs N2C)** — N2N only. N2C local-chain-sync /
  local-tx-submission server response paths are out of scope (sketch
  §"Scope decisions" #3).

## Grounding (verified at HEAD `df56e2d`)

- **BLUE mini-protocol state machines already exist (PHASE4-N-A):**
  - `ade_network::chain_sync::transition::chain_sync_transition`
    (`crates/ade_network/src/chain_sync/transition.rs:59`) — covers
    the full bidirectional state graph including Server-agency
    transitions (`CanAwait + Server + RollForward`, `MustReply +
    Server + RollForward`, etc.). The server reducers compose this;
    they do not re-implement the grammar.
  - `ade_network::block_fetch::transition::block_fetch_transition`
    (`crates/ade_network/src/block_fetch/transition.rs:56`) — same,
    for block-fetch (`Busy + Server + StartBatch`, `Streaming +
    Server + Block { bytes }`, `Streaming + Server + BatchDone`).
  - Per-protocol agency types are non-interchangeable
    (`ade_network::chain_sync::agency::ChainSyncAgency`;
    `ade_network::block_fetch::agency::BlockFetchAgency`).
- **`AcceptedBlock` + `BroadcastQueue` (PHASE4-N-C):**
  - `ade_ledger::producer::AcceptedBlock` — opaque token; private
    constructor; only `self_accept` returns `Ok(AcceptedBlock)`.
    `AcceptedBlock::as_bytes() -> &[u8]` is the canonical accessor for
    the cleared bytes.
  - `ade_runtime::producer::broadcast::BroadcastQueue::dequeue() ->
    Option<AcceptedBlock>` (`crates/ade_runtime/src/producer/broadcast.rs:70`).
- **Validator-shared body-hash recipe (`DC-CONS-16` site):**
  - `ade_ledger::block_validity::header_input` — extracts the header
    bytes the validator hashes; this is the existing split site to
    reuse for `accepted_block_header_bytes`. S1 lifts a public
    accessor there without introducing a parallel splitter.
- **Codec sites the orchestrator (S6) will hand encoded frames to:**
  - `ade_network::codec::chain_sync` (encodes
    `ChainSyncMessage::RollForward { header, tip }` etc.)
  - `ade_network::codec::block_fetch` (encodes
    `BlockFetchMessage::Block { bytes }` etc.)
- **Existing live-evidence binary patterns to mirror:**
  - `crates/ade_core_interop/src/bin/live_consensus_session.rs`
    (CE-N-B-6 precedent)
  - `crates/ade_core_interop/src/bin/live_block_production_session.rs`
    (CE-N-C-8 precedent — the closest sibling to N-G's S7 binary).
- **No `producer::served_chain` module exists** at HEAD — S2 is
  greenfield for the served-chain index.
- **No `chain_sync::server` / `block_fetch::server` submodules
  exist** at HEAD — S1, S3, S4 are greenfield for the server-role
  reducers.
- **Existing trailer ratio**: 254/307 = 82.74% at HEAD `df56e2d`.
  Project hook `ci/git-hooks/commit-msg` enforces the
  `Co-Authored-By: Claude Opus 4.7 (1M context)` trailer; activated
  in this clone via `git config core.hooksPath ci/git-hooks` (already
  active).

## Entry Conditions

- **PHASE4-N-A closed** — N2N mini-protocol codecs + handshake +
  per-protocol agency types + bidirectional BLUE state machines
  exist. Server-role transitions are already type-supported.
- **PHASE4-N-B closed** — Header validation chokepoint
  (`header_validate::validate_and_apply_header`) exists; per-session
  validation is reachable from the orchestrator if ever needed.
- **PHASE4-N-C closed** — `AcceptedBlock` token + `BroadcastQueue` +
  `self_accept` + type-level broadcast gate (`CN-CONS-07`) exist.
  Private-key custody is RED-confined. The producer pipeline emits
  `AcceptedBlock` tokens this cluster consumes.
- **PHASE4-B1..B5 closed** — Body-hash recipe canonical (`DC-CONS-16`
  enforced); `block_body_hash` is the single authority the
  header/body splitter S1 lifts from.
- **Constitution-coverage gate PASSES** at HEAD:
  `bash ci/ci_check_constitution_coverage.sh`.

## Exit Criteria (CI-Verifiable)

Each CE names the test or check that proves it. Every named test /
script is introduced by the slice that owns the CE.

- **CE-N-G-1 (header projection + ServerReply&lt;M&gt; closed wrapper)** —
  Single canonical header/body splitter; type-level enforcement that
  the server pump cannot construct client-agency messages. Named
  tests:
  - `accepted_block_header_bytes_equals_validator_split_on_corpus`
    (S1) — for every block in the PHASE4-N-C corpus, the new
    accessor's output equals the validator's existing header-bytes
    extraction (`block_validity::header_input`).
  - `accepted_block_header_bytes_is_subslice_of_as_bytes` (S1) —
    header bytes are a literal subslice of the full envelope.
  - `server_reply_chain_sync_constructors_compile_only_server_variants`
    (S1, compile-fail test under `trybuild` or a `compile_fail`
    doctest) — attempts to construct
    `ServerReply::<ChainSyncMessage>::from_client_variant(...)` MUST
    fail at type-check.
  - `server_reply_block_fetch_constructors_compile_only_server_variants`
    (S1, compile-fail test) — same for block-fetch.
  - `server_reply_round_trip_through_codec_yields_canonical_bytes`
    (S1) — every constructible `ServerReply<_>` value, when
    encoded via the existing codec, round-trips byte-identically.
  - CI: `ci/ci_check_no_parallel_header_splitter.sh` (S1) — grep gate
    forbidding new `pub fn .*header_bytes`, new `fn .*split_header`,
    or any duplicate header/body byte-splitter outside the canonical
    site.
  - Registry flip on close: `CN-PROTO-06` → `enforced`.

- **CE-N-G-2 (ServedChainSnapshot + served_chain_admit)** — Canonical,
  deterministic, append-only snapshot of admitted blocks. Named
  tests:
  - `served_chain_admit_idempotent_on_byte_identity` (S2) — admitting
    the same `AcceptedBlock` twice yields the same snapshot
    fingerprint.
  - `served_chain_admit_rejects_hash_bytes_mismatch` (S2) — a
    constructed-by-test pseudo-`AcceptedBlock` whose recomputed block
    hash differs from its asserted hash is rejected with
    structured error.
  - `served_chain_snapshot_iteration_is_btreemap_ordered` (S2) —
    iteration over admitted entries is `(SlotNo, Hash32)`-sorted;
    no `HashMap`.
  - `served_chain_admit_replay_byte_identical_fingerprint` (S2) —
    replay of an arbitrary admission sequence produces an identical
    snapshot fingerprint across two runs.
  - `served_chain_block_bytes_accessor_returns_accepted_block_slice`
    (S2) — `ServedChainSnapshot::block_bytes(&self, point) -> &[u8]`
    returns a slice that is byte-identical to the original
    `AcceptedBlock.as_bytes()`.
  - CI: `ci/ci_check_served_chain_closure.sh` (S2) — forbids any
    `HashMap` / `HashSet` in `ade_ledger::producer::served_chain`;
    forbids any construction of `ServedChainSnapshot` outside the
    canonical admit path.
  - Registry strengthening on close: `CN-CONS-07.strengthened_in +=
    PHASE4-N-G` (broadcast gate preserved across the seam — the only
    way bytes enter the served chain is via `AcceptedBlock`).

- **CE-N-G-3 (chain-sync server reducers)** — BLUE `producer_chain_sync_serve`
  + `producer_chain_sync_advance_tip` total and deterministic over
  server-agency states. Named tests:
  - `producer_chain_sync_serve_total_over_server_agency_states` (S3)
    — every `(server_state, in_msg)` pair under server-agency returns
    `Ok` with a `ServerReply` or a structured close-or-error; no
    panic, no `unimplemented!`.
  - `producer_chain_sync_serve_request_next_idle_yields_can_await` (S3)
  - `producer_chain_sync_serve_find_intersect_known_point_yields_intersect_found`
    (S3)
  - `producer_chain_sync_serve_find_intersect_unknown_point_yields_intersect_not_found`
    (S3) — narrow-scope OQ-3.
  - `producer_chain_sync_advance_tip_idle_yields_none` (S3) — no
    `RollForward` emitted until the client has asked.
  - `producer_chain_sync_advance_tip_can_await_yields_roll_forward_with_projected_header`
    (S3) — `RollForward.header` bytes equal
    `accepted_block_header_bytes(&new_accepted)` (S1 dependency).
  - `producer_chain_sync_advance_tip_can_await_yields_roll_forward_with_tip_at_admitted_slot_hash`
    (S3).
  - `producer_chain_sync_serve_rejects_must_reply_silent_wait` (S3) —
    in `MustReply`, the reducer never returns `Ok((MustReply,
    silent-wait))` without an explicit replay-input wait condition.
    Deterministic resolution per DC-PROTO-08.
  - CI: `ci/ci_check_chain_sync_server_closure.sh` (S3) — forbids any
    function in `ade_network::chain_sync::server` returning a raw
    `ChainSyncMessage` instead of `ServerReply<ChainSyncMessage>`;
    forbids construction of `RollForward` whose `header` did not come
    from `accepted_block_header_bytes`.
  - Registry flip on close: `DC-PROTO-08` → `enforced`.

- **CE-N-G-4 (block-fetch server reducer)** — BLUE `producer_block_fetch_serve`
  total and deterministic. Named tests:
  - `producer_block_fetch_serve_total_over_server_agency_states` (S4)
    — every `(server_state, in_msg)` pair returns `Ok` with a
    `Vec<ServerReply<BlockFetchMessage>>` or structured
    close-or-error.
  - `producer_block_fetch_serve_request_range_in_chain_yields_start_batch_blocks_batch_done`
    (S4) — exact sequence shape over a curated range.
  - `producer_block_fetch_serve_request_range_empty_in_chain_yields_no_blocks`
    (S4) — narrow-scope: range whose bounds resolve to no admitted
    blocks → `[NoBlocks]`.
  - `producer_block_fetch_serve_request_range_outside_chain_yields_no_blocks`
    (S4) — OQ-3-adjacent narrow-scope answerability.
  - `producer_block_fetch_serve_block_bytes_equal_accepted_block_as_bytes`
    (S4) — for every `Block { bytes }` in the response, the bytes are
    byte-identical to the corresponding
    `ServedChainSnapshot::block_bytes(point)` (which is itself the
    original `AcceptedBlock.as_bytes()` per S2).
  - `producer_block_fetch_serve_inverted_range_returns_structured_error`
    (S4) — `from > to` is rejected with the same structured shape as
    the underlying BLUE state machine's `MalformedMessage`.
  - CI: `ci/ci_check_block_fetch_server_closure.sh` (S4) — forbids any
    function in `ade_network::block_fetch::server` constructing
    `Block { bytes }` from anything other than a
    `ServedChainSnapshot` slice.
  - Registry contribution on close: enforcement foundation for
    `DC-CONS-17` and `DC-CONS-18` (full flip at CE-N-G-5 once replay
    confirms across the whole pipeline).

- **CE-N-G-5 (GREEN adapter + synthetic transcript replay)** —
  Pipeline determinism + served-bytes parity + header-body coherence
  proved by replay. Named tests:
  - `broadcast_to_served_is_pure_no_io` (S5, grep gate + no
    `std::time` / `tokio` imports in the GREEN module).
  - `broadcast_to_served_deterministic_over_captured_arrivals` (S5)
    — given a captured `Vec<AcceptedBlock>` arrival sequence, two
    runs produce identical (`ServedChainSnapshot`,
    `Vec<Option<RollForward>>`) outputs.
  - `session_transcript_replay_byte_identical` (S5) — the corpus
    `(version, peer_message_sequence, broadcast_arrival_sequence,
    session_event_sequence)` drives the full pipeline twice; outgoing
    encoded frame `Vec<Vec<u8>>` is byte-identical across runs.
  - `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`
    (S5) — for every served `Block { bytes }` in the corpus
    transcript, bytes equal the admitted `AcceptedBlock.as_bytes()`
    (DC-CONS-17 closure).
  - `session_transcript_announced_header_matches_served_body_recipe`
    (S5) — for every `RollForward { header, tip }` in the transcript,
    when the matching block-fetch range later serves the same
    `(slot, hash)`, `block_body_hash(served_body_segment)` equals the
    body-hash field inside `header` (DC-CONS-18 closure).
  - CI: `ci/ci_check_no_private_keys_in_corpus.sh` extended (S5) to
    cover `crates/ade_testkit/fixtures/server_paths/`.
  - Registry flip on close: `DC-CONS-17`, `DC-CONS-18`, `DC-PROTO-07`
    → `enforced`.

- **CE-N-G-6 (RED per-peer N2N server orchestrator)** — RED
  orchestrator wires mux + handshake + reducers; key-boundary
  preserved. Named tests:
  - `n2n_server_threads_negotiated_version_to_reducers` (S6) — every
    reducer call from the orchestrator carries the version returned
    by the handshake; never reads it from a session global.
  - `n2n_server_only_encodes_server_reply_outputs` (S6, grep gate +
    runtime test) — orchestrator never calls
    `encode_chain_sync_message` / `encode_block_fetch_message`
    directly; only via `ServerReply::into_message()` projection.
  - `n2n_server_two_synthetic_peers_preserve_per_session_transcripts`
    (S6) — OQ-4 multi-peer determinism: driving two peers in parallel
    against one orchestrator yields per-session transcripts identical
    to running each peer alone.
  - `n2n_server_does_not_link_against_producer_signing` (S6, build-time
    dep test) — `ade_runtime::network::n2n_server` (or whichever
    module hosts the orchestrator) has no path to
    `ade_runtime::producer::signing` items.
  - CI: `ci/ci_check_dependency_boundary.sh` extended (S6) — adds
    the n2n-server-no-signing-dep rule.
  - No registry flip — confirms `CN-PROTO-06` end-to-end and
    contributes to `DC-PROTO-06.strengthened_in += PHASE4-N-G`.

- **CE-N-G-7 (mechanical cross-impl adapter)** — CI test drives the
  full server-side pipeline against captured N-C corpus and verifies
  Ade's own decoder + body-hash recipe accept the served bytes.
  Named tests:
  - `cross_impl_server_pipeline_request_range_returns_decodable_bytes`
    (S7) — for every `(ProducerTick, AcceptedBlock)` pair from N-C's
    corpus, drive the GREEN adapter + BLUE reducers against a
    synthetic `RequestRange` covering that block; assert
    `decode_block_envelope` succeeds on every `Block { bytes }`
    payload and the recomputed body-hash matches the announced
    header's body-hash field.
  - `cross_impl_server_pipeline_request_range_byte_identical_to_self_accept_input`
    (S7) — the served bytes equal the bytes that `self_accept` saw.
  - CI: `ci/ci_check_server_paths_corpus_present.sh` (S7) — guards
    fixture presence + non-empty `expected_transcript.cbor` outputs.
  - Registry contribution on close: mechanical pre-condition for
    `RO-LIVE-01` — the bytes-the-Haskell-peer-would-see are proven
    Ade-decodable independently of the network.

- **CE-N-G-8 (operator-action live evidence)** — Conditional. Either:
  - (a) `docs/clusters/PHASE4-N-G/CE-N-G-LIVE_<date>.log` captures
    a private cardano-node peer completing a successful `RequestRange`
    against an Ade-forged block and accepting it under its own
    header+body validation, AND `docs/clusters/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`
    documents how the session was run; OR
  - (b) `CE-N-G-8_PROCEDURE.md` documents the
    `blocked_until_operator_peer_available` status, names the
    specific blocker (no private Haskell peer provisioned at HEAD
    `<hash>`), and records the re-open obligation as a registry
    `open_obligation` on `RO-LIVE-01`.
  - Registry flip on close: `RO-LIVE-01` →
    `enforced` (case a) or `partial` with `open_obligation` (case b).
  - This is the explicit conditional closure pattern, mirroring
    PHASE4-N-C's `CN-CONS-06` and `OP-OPS-04`.

## Slice index

Slices land in this order; each slice independently leaves the
system in a fully correct state. See per-slice docs once `/slice-doc`
runs.

| Slice | One-line scope | TCB |
|----|----|----|
| **N-G-S1** | BLUE header-projection authority lifted from validator body-hash recipe + closed `ServerReply<M>` wrapper (compile-time server-agency closure). | BLUE |
| **N-G-S2** | BLUE `ServedChainSnapshot` canonical type + `served_chain_admit` (deterministic, `BTreeMap`-backed, append-only, broadcast-gate-preserving). | BLUE |
| **N-G-S3** | BLUE chain-sync server reducers: `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`; deterministic-resolution discipline (DC-PROTO-08). | BLUE |
| **N-G-S4** | BLUE block-fetch server reducer: `producer_block_fetch_serve`; served `Block { bytes }` payloads are `AcceptedBlock` slices verbatim. | BLUE |
| **N-G-S5** | GREEN `broadcast_to_served` adapter + synthetic session-transcript replay corpus + transcript-determinism / header-body-coherence tests. | GREEN + test |
| **N-G-S6** | RED per-peer N2N server orchestrator wiring mux + handshake + reducers; key-boundary CI gate; multi-peer determinism test. | RED |
| **N-G-S7** | Mechanical cross-impl adapter + operator-action `live_block_fetch_session` binary; CE-N-G-8 conditional. | RED + test |

## TCB Color Map (FC/IS Partition)

For every module touched / created by this cluster:

**BLUE (deterministic, authoritative):**
- `ade_network::chain_sync::server` (new — S1, S3). Holds
  `ServerReply<ChainSyncMessage>` + `ProducerChainSyncServerState` +
  `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`.
- `ade_network::block_fetch::server` (new — S1, S4). Holds
  `ServerReply<BlockFetchMessage>` + `ProducerBlockFetchServerState`
  + `producer_block_fetch_serve`.
- `ade_ledger::producer::served_chain` (new — S2). Holds
  `ServedChainSnapshot` + `served_chain_admit` +
  `ServedChainSnapshot::block_bytes` accessor.
- `ade_ledger::block_validity::header_input` (existing — S1).
  Lifted: a public `accepted_block_header_bytes(&AcceptedBlock)`
  accessor that returns the existing header-byte slice. No new
  splitter logic; the existing recipe is the canonical authority.
- `ade_network::chain_sync::transition` (existing, **unchanged** —
  PHASE4-N-A). S3's server reducer composes
  `chain_sync_transition` to validate grammar; it does not
  re-implement the state graph.
- `ade_network::block_fetch::transition` (existing, **unchanged** —
  PHASE4-N-A). Same for S4.
- `ade_ledger::producer::AcceptedBlock` (existing, **unchanged** —
  PHASE4-N-C). `as_bytes()` is the canonical bytes accessor S2's
  admit and S4's serve both rely on.

**GREEN (deterministic glue, non-authoritative):**
- `ade_runtime::producer::broadcast_to_served` (new — S5). Pure
  adapter consuming `BroadcastQueue::dequeue()` outputs and producing
  `served_chain_admit` calls + `producer_chain_sync_advance_tip`
  triggers. No I/O, no clock; observably deterministic over captured
  arrival sequences.
- `ade_testkit::server_paths::{fixtures, replay}` (new — S5). Replay
  scaffolding + synthetic corpus drivers.
- `ade_testkit::server_paths::cross_impl_adapter` (new — S7).
  Mechanical adapter binding N-C's `(ProducerTick, AcceptedBlock)`
  corpus to N-G's server pipeline + Ade's own decoder.

**RED (nondeterministic shell):**
- `ade_runtime::network::n2n_server` (new — S6, or extension of
  existing `ade_network::session::*` shell). Per-peer session
  orchestrator owning the mux channel, threading the
  handshake-negotiated version, calling BLUE reducers, writing
  encoded frames. Lifetime, back-pressure, shutdown.
- `ade_core_interop::bin::live_block_fetch_session` (new — S7).
  Operator-action evidence binary producing
  `CE-N-G-LIVE_<date>.log`. Conditional on
  `blocked_until_operator_peer_available`.

**Test harness (`ade_testkit`):**
- `ade_testkit::server_paths::fixtures` (new — S5; corpus root at
  `crates/ade_testkit/fixtures/server_paths/`)
- `ade_testkit::server_paths::replay` (new — S5)
- `ade_testkit::server_paths::cross_impl_adapter` (new — S7)

**Color rules:**
- No RED behaviour may appear in BLUE code (enforced by per-slice CI
  gates + the existing `ci_check_forbidden_patterns.sh`).
- GREEN code must not affect authoritative outputs (enforced by S5's
  `broadcast_to_served_deterministic_over_captured_arrivals` —
  identical captures imply no RED leakage).
- The RED orchestrator must not link against `producer::signing`
  (enforced by `ci_check_dependency_boundary.sh` extension in S6).
- Color is resolved before any slice begins (resolved above; no open
  colors).

## Forbidden during this cluster

- Any new `pub fn` that splits a Cardano block envelope into header /
  body byte segments outside the canonical
  `ade_ledger::block_validity::header_input` recipe (CI:
  `ci_check_no_parallel_header_splitter.sh`).
- Construction of `RollForward { header, tip }` whose `header` is
  anything other than `accepted_block_header_bytes(&accepted)`'s
  output (CI: `ci_check_chain_sync_server_closure.sh`).
- Construction of `Block { bytes }` in
  `ade_network::block_fetch::server` whose `bytes` is anything other
  than a `ServedChainSnapshot` slice (CI:
  `ci_check_block_fetch_server_closure.sh`).
- Any `ServerReply<M>` variant constructor that admits a
  client-agency message (enforced at the type level; compile-fail
  tests in S1).
- Any `HashMap` / `HashSet` in `ade_ledger::producer::served_chain`
  (CI: `ci_check_served_chain_closure.sh`).
- Any path in `ade_runtime::network::n2n_server` that imports from
  `ade_runtime::producer::signing` (CI:
  `ci_check_dependency_boundary.sh` extension in S6).
- Server-agency reducer returning `Ok((MustReply, silent-wait))`
  without an explicit replay-input wait condition (enforced by S3's
  `producer_chain_sync_serve_rejects_must_reply_silent_wait`).
- Replay corpus carrying private-key bytes — existing
  `ci/ci_check_no_private_keys_in_corpus.sh` extended in S5 to cover
  the new fixture root.
- `git commit --no-verify` (silently bypasses the project trailer
  hook).
- Slice docs claiming a CE that the slice does not mechanically
  enforce. Every CE flipped to `enforced` must point to a named test
  + green CI script.
- Any code path that broadens the served-chain scope to include
  non-`AcceptedBlock`-derived bytes (OQ-1 lock).

## Replay obligations introduced by this cluster

- **New canonical replay corpus**:
  `crates/ade_testkit/fixtures/server_paths/` contains ordered
  tuples `(negotiated_version, peer_message_sequence,
  broadcast_arrival_sequence, session_event_sequence)` paired with
  expected `Vec<OutgoingFrameBytes>`. Drives
  `session_transcript_replay_byte_identical` and
  `session_transcript_served_block_bytes_equal_admitted_accepted_block_bytes`.
  Uses N-C-provided `AcceptedBlock` artifacts as the broadcast
  arrival sequence's payload; no private keys.
- **T-DET-01 strengthening**: the producer-side server-role
  transcript joins the list of byte-deterministic transformations.
  Registry entry `T-DET-01`'s `strengthened_in` array gains
  `PHASE4-N-G` on cluster close.
- **T-ENC-01 strengthening**: wire-byte transmission of canonical
  block bytes via the server pump joins the hash-critical byte
  paths. Registry entry `T-ENC-01`'s `strengthened_in` array gains
  `PHASE4-N-G` on cluster close.
- **DC-CONS-16 strengthening**: header/body split reused at the
  server-role surface via `accepted_block_header_bytes`. Registry
  entry `DC-CONS-16`'s `strengthened_in` array gains `PHASE4-N-G` on
  cluster close.
- **CN-CONS-07 strengthening**: broadcast gate preserved across the
  network seam — `ServedChainSnapshot` admit is the only path bytes
  enter the served chain, and the admit accepts only
  `AcceptedBlock`. Registry entry `CN-CONS-07`'s `strengthened_in`
  array gains `PHASE4-N-G` on cluster close.
- **DC-PROTO-06 strengthening**: handshake-negotiated version is
  threaded as explicit input through the new server-role surface
  too. Registry entry `DC-PROTO-06`'s `strengthened_in` array gains
  `PHASE4-N-G` on cluster close.
- **No private-key material in corpora.** Same key-boundary
  discipline as PHASE4-N-C; replay drives BLUE + GREEN reducers
  only.

## Authority reminder

This document is a planning aid only. All correctness rules live in
the project's normative specifications and the invariant registry.
If there is ever a disagreement:

> **Normative documents + registry + CI enforcement win.**
