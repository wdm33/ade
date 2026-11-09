# Cluster/Slice Plan — PHASE4-N-G Producer-side Server Paths

**Status**: cluster-plan phase complete; awaiting `/cluster-doc`
**HEAD pin**: `df56e2d`
**Date**: 2026-05-25
**Source**: `docs/planning/phase4-n-a-successor-invariants.md`

## Cluster Index (Dependency Order)

1. **PHASE4-N-G — Producer-side server paths** — primary invariant: a
   peer running cardano-node's N2N chain-sync + block-fetch as client
   sees a byte-identical, deterministic transcript whose served block
   bytes are exactly the `AcceptedBlock` bytes the producer's
   self-accept gate cleared — no re-encoding, no parallel header
   splitter, no client-tagged messages from the server pump.

This cluster is independently mergeable atop PHASE4-N-C
(HEAD `df56e2d`). It introduces no dependency on later clusters.

## PHASE4-N-G — Producer-side server paths

- **Primary invariant**: producer-side chain-sync + block-fetch
  server-role responses are (a) deterministic across replays over
  canonical `(negotiated_version, peer_message_sequence,
  broadcast_arrival_sequence, session_event_sequence)` inputs,
  (b) byte-identical to what `AcceptedBlock` admitted via
  `BroadcastQueue` ↔ `ServedChainSnapshot`, and (c) only ever
  Server-agency messages by construction (`ServerReply<M>` closed
  wrapper).

- **Tier**: 1 (mini-protocol wire bytes + agency semantics) for S1–S6,
  release-evidence for S7 live half.

- **TCB partition**:
  - **BLUE (new)**:
    - `ade_network::chain_sync::server`: `producer_chain_sync_serve`,
      `producer_chain_sync_advance_tip`, `ProducerChainSyncServerState`,
      `ServerReply<ChainSyncMessage>`.
    - `ade_network::block_fetch::server`: `producer_block_fetch_serve`,
      `ProducerBlockFetchServerState`, `ServerReply<BlockFetchMessage>`.
    - `ade_ledger::producer::served_chain`: `ServedChainSnapshot`,
      `served_chain_admit`, `accepted_block_header_bytes`.
    - Header/body byte-split extraction inside the existing
      validator-shared body-hash recipe (DC-CONS-16 site;
      `ade_ledger::block_validity::header_input` or its
      `ade_ledger::producer::self_accept` shared module). No parallel
      splitter.
  - **GREEN (new)**:
    - `ade_runtime::producer::broadcast_to_served`: pure adapter
      translating `BroadcastQueue::dequeue()` outputs into
      `served_chain_admit` calls + `producer_chain_sync_advance_tip`
      triggers. No I/O.
  - **RED (new)**:
    - `ade_runtime::network::n2n_server` (new submodule, or extension
      of existing `ade_network::session::*` shell): per-peer session
      orchestrator owning the mux channel, threading the negotiated
      mini-protocol version, calling the BLUE reducers, writing
      encoded byte frames.
    - `ade_core_interop::bin::live_block_fetch_session`: new evidence
      binary that drives the orchestrator against a real Haskell peer.
  - **Unchanged (anchor)**:
    - `ade_network::chain_sync::transition` /
      `ade_network::block_fetch::transition` — BLUE mini-protocol
      state machines from PHASE4-N-A. The server-role reducers
      compose these; they are not re-implemented.
    - `ade_runtime::producer::broadcast::BroadcastQueue` — N-C-S6.
    - `ade_ledger::producer::{self_accept, AcceptedBlock}` — N-C-S5.

- **Cluster Exit Criteria**:

  - **CE-N-G-1** — Header projection + `ServerReply<M>` closed wrapper
    (BLUE): `accepted_block_header_bytes(&AcceptedBlock) -> &[u8]`
    reuses the validator-shared body-hash recipe's header/body split
    (DC-CONS-16) — no parallel splitter; a CI gate forbids
    construction of a second header/body byte-splitter.
    `ServerReply<ChainSyncMessage>` and `ServerReply<BlockFetchMessage>`
    are closed wrappers whose constructors statically restrict to the
    Server-agency-legal subset of variants. Constructing a
    `ServerReply<_>::RequestNext` etc. is a compile error.
    *(Flips CN-PROTO-06 to `enforced`.)*

  - **CE-N-G-2** — `ServedChainSnapshot` canonical type +
    `served_chain_admit` (BLUE): canonical, deterministic, append-only
    snapshot of admitted `AcceptedBlock` tokens keyed by `(slot, hash)`,
    with `BTreeMap`-backed iteration (no `HashMap`). `served_chain_admit`
    is total, pure, and rejects (a) admission of a token whose bytes
    have already been admitted under the same hash with different
    bytes (impossible by construction; structured error), (b) admission
    of a token whose hash does not match the recomputed block hash
    over its bytes. Replay of an arbitrary admission sequence is
    byte-deterministic over fingerprint.
    *(Contributes to CN-CONS-07 strengthening.)*

  - **CE-N-G-3** — Chain-sync server reducers (BLUE):
    `producer_chain_sync_serve(state, in_msg, &served, version)` is
    pure and total, returning `Result<(state',
    ServerReply<ChainSyncMessage>), Error>` for every
    `(state, in_msg)` pair that protocol grammar accepts; rejects
    out-of-grammar pairs as `IllegalServerStep` (closed shape). In
    server-agency states (`CanAwait` / `MustReply` / `Intersect`), the
    reducer's outputs cover the full deterministic-resolution set:
    legal `RollForward`, legal `RollBackward`, legal `AwaitReply`, or
    structured deterministic session-close/error — no ambiguous wait
    state. `producer_chain_sync_advance_tip(state, new_accepted)` is
    likewise pure and total; admitting a new accepted block either
    yields `Some(RollForward { header, tip })` ready to send, or
    `None` when the per-session client has not yet requested next
    (still in `Idle`). Header bytes in `RollForward` come from
    `accepted_block_header_bytes` (CE-N-G-1), not a parallel splitter.
    *(Flips DC-PROTO-08 to `enforced`.)*

  - **CE-N-G-4** — Block-fetch server reducer (BLUE):
    `producer_block_fetch_serve(state, in_msg, &served, version)` is
    pure and total. On `RequestRange { from, to }`, the reducer
    returns the exact sequence
    `[StartBatch, Block{bytes}, Block{bytes}, ..., BatchDone]` (or
    `[NoBlocks]` if the range is empty within the served chain), where
    every `Block { bytes }` payload is `AcceptedBlock.as_bytes()`
    sourced from `ServedChainSnapshot` verbatim — no transformation,
    no re-encoding, no header/body roundtrip. Streaming-batch state
    transitions are byte-deterministic. Range bounds outside the
    served chain are answered with `NoBlocks` (narrow-scope
    answerability per the sketch's open question #2).
    *(Contributes to DC-CONS-17, DC-CONS-18 enforcement.)*

  - **CE-N-G-5** — GREEN broadcast→served adapter + synthetic
    session-transcript replay (GREEN + replay corpus): the GREEN
    adapter `broadcast_to_served` is pure, has no I/O, and
    deterministically composes `BroadcastQueue::dequeue()` with
    `served_chain_admit` + `producer_chain_sync_advance_tip`. A
    synthetic fixture corpus of
    `(peer_message_sequence, broadcast_arrival_sequence,
    session_event_sequence)` drives the BLUE reducers + GREEN adapter
    twice and asserts the outgoing frame sequence is byte-identical
    across runs and that every served `Block { bytes }` equals the
    corresponding admitted `AcceptedBlock.as_bytes()`. Header bytes
    announced in any `RollForward` equal the header sub-segment of
    the body bytes subsequently served in the matching block-fetch
    range. Corpus carries signed artifacts only — no private keys
    (CI gate extended).
    *(Flips DC-CONS-17, DC-CONS-18, DC-PROTO-07 to `enforced`.)*

  - **CE-N-G-6** — RED per-peer N2N server orchestrator (RED): the
    orchestrator wires the existing N2N mux + handshake (PHASE4-N-A)
    to the new BLUE server reducers via the GREEN adapter. The
    handshake-negotiated version is threaded explicitly into every
    reducer call (DC-PROTO-06 strengthened across the server-role
    surface). The orchestrator never re-encodes block bytes; it only
    encodes `ServerReply<M>` outputs via the existing codec. No code
    path on the server orchestrator has access to
    `producer::signing::*` — enforced by `ci_check_dependency_boundary.sh`
    extension. Two synthetic peers can be driven in parallel against
    one orchestrator without altering per-session transcripts
    (multi-peer determinism — sketch open question #3 resolved).
    *(No new flips; confirms CN-PROTO-06 end-to-end and the
    key-boundary doctrine across the new RED.)*

  - **CE-N-G-7** — Mechanical cross-impl adapter (CI gate): a CI test
    captures a `(ProducerTick, AcceptedBlock)` pair from the N-C
    corpus, drives it through the GREEN adapter + BLUE reducers
    against a synthetic `RequestRange` covering exactly the forged
    block, and verifies the bytes round-trip through Ade's own
    `ade_codec::cbor::envelope::decode_block_envelope` +
    `ade_ledger::block_validity::header_input` body-hash recipe.
    Independent of any external network — this is the mechanical
    bridge that establishes "we will serve bytes a Haskell peer can
    fetch" without requiring a Haskell peer in CI.
    *(Contributes to RO-LIVE-01 mechanical half — provides the
    pre-condition that the live half can rest on.)*

  - **CE-N-G-8** — Operator-action live evidence: a sustained-window
    `live_block_fetch_session` against a private cardano-node
    produces `CE-N-G-LIVE_<date>.log` showing the Haskell peer
    completing a successful `RequestRange` against an Ade-forged
    block and accepting it under its own header+body validation.
    Marked `blocked_until_operator_peer_available` if a private
    Haskell peer is not yet provisioned at cluster close
    (same pattern as PHASE4-N-C's `CN-CONS-06` open obligation).
    *(Flips RO-LIVE-01 to `enforced` if log captured;
    `enforced + open_obligation` if not.)*

- **Slices**:

  - **N-G-S1** — Header projection authority + `ServerReply<M>`
    closed wrapper
    Invariant: a single canonical
    `accepted_block_header_bytes(&AcceptedBlock) -> &[u8]` lifted from
    the validator-shared body-hash recipe; no parallel splitter.
    `ServerReply<ChainSyncMessage>` / `ServerReply<BlockFetchMessage>`
    closed wrappers admit only Server-agency-legal variants by type
    construction (compile-time enforcement of I-4).
    Addresses: **CE-N-G-1**.
    TCB: **BLUE** (`ade_ledger::block_validity::header_input` shared
    header/body extractor; `ade_network::{chain_sync,block_fetch}::server`
    new modules holding the wrappers).
    CI: `ci/ci_check_no_parallel_header_splitter.sh` (new).

  - **N-G-S2** — `ServedChainSnapshot` + `served_chain_admit`
    Invariant: canonical, deterministic, append-only `BTreeMap<(SlotNo,
    Hash32), AcceptedBlock>` snapshot. `served_chain_admit` rejects
    hash-bytes mismatch with a structured error; admission is
    idempotent on byte-identity. Replay over an arbitrary admission
    sequence is fingerprint-deterministic across runs.
    Addresses: **CE-N-G-2**.
    TCB: **BLUE** (`ade_ledger::producer::served_chain` new module).
    Forbidden: `HashMap`/`HashSet` for the admitted-blocks index;
    enforced by `ci_check_forbidden_patterns.sh` extension or the
    existing closed-iteration audit.

  - **N-G-S3** — Chain-sync server reducers
    Invariant: `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`
    are pure, total over Server-agency states; client-originated
    requests are answered with deterministic resolution
    (`RollForward` / `RollBackward` / `AwaitReply` / structured
    close-or-error). `FindIntersect` with no point in the served chain
    returns `IntersectNotFound` (narrow scope). Header bytes in any
    `RollForward` reply come from `accepted_block_header_bytes`
    (S1), never from a parallel splitter.
    Addresses: **CE-N-G-3**.
    TCB: **BLUE** (`ade_network::chain_sync::server`).

  - **N-G-S4** — Block-fetch server reducer
    Invariant: `producer_block_fetch_serve` returns the canonical
    `[StartBatch, Block{bytes}*, BatchDone]` (or `[NoBlocks]`) sequence
    over `RequestRange { from, to }` against `ServedChainSnapshot`.
    Every `Block { bytes }` is `AcceptedBlock.as_bytes()` verbatim;
    no re-encoding. Out-of-served-chain ranges → `NoBlocks`.
    Addresses: **CE-N-G-4**.
    TCB: **BLUE** (`ade_network::block_fetch::server`).

  - **N-G-S5** — GREEN broadcast→served adapter + synthetic
    session-transcript replay corpus
    Invariant: the GREEN adapter is pure (no I/O, no clock).
    A synthetic replay corpus drives the full server-side pipeline
    twice and asserts (a) byte-identical outgoing frame sequence,
    (b) `Block{bytes}` equals admitted `AcceptedBlock.as_bytes()`,
    (c) any `RollForward.header` is the header sub-segment of the
    body that's later served for the same `(slot,hash)`. Corpus
    excludes private-key material; `ci_check_no_private_keys_in_corpus.sh`
    extended to cover the new corpus directory.
    Addresses: **CE-N-G-5**.
    TCB: **GREEN** adapter (`ade_runtime::producer::broadcast_to_served`);
    replay scaffolding **GREEN/test** in
    `ade_testkit::server_paths::{fixtures, replay}`.

  - **N-G-S6** — RED per-peer N2N server orchestrator
    Invariant: orchestrator threads handshake-negotiated version into
    every reducer call (DC-PROTO-06 across server role); only encodes
    `ServerReply<M>` outputs; never accesses `producer::signing::*`
    (enforced by `ci_check_dependency_boundary.sh` extension).
    Two synthetic peers driven in parallel preserve per-session
    transcripts. Connection lifecycle, back-pressure, and shutdown
    are RED-only.
    Addresses: **CE-N-G-6**.
    TCB: **RED** (`ade_runtime::network::n2n_server` new submodule,
    or `ade_network::session` shell extension).

  - **N-G-S7** — Mechanical cross-impl adapter + live-evidence binary
    Invariant: a CI test drives the full server-side pipeline against
    a captured `(ProducerTick, AcceptedBlock)` pair from N-C's corpus
    and verifies the bytes round-trip through Ade's own decoder +
    body-hash recipe. The `live_block_fetch_session` binary in
    `ade_core_interop::bin` connects to a private cardano-node peer
    and captures `CE-N-G-LIVE_<date>.log` of a successful RequestRange.
    Live half is conditional via
    `blocked_until_operator_peer_available` (mirrors N-C's
    `blocked_until_operator_stake_available`).
    Addresses: **CE-N-G-7**, **CE-N-G-8**.
    TCB: mechanical adapter **GREEN/test** in
    `ade_testkit::server_paths::cross_impl_adapter`;
    binary **RED** (`ade_core_interop::bin::live_block_fetch_session`).

- **Replay obligations**:
  - New canonical replay corpus: ordered tuples
    `(negotiated_version, peer_message_sequence,
    broadcast_arrival_sequence, session_event_sequence)` paired with
    expected `Vec<OutgoingFrameBytes>`. Lives at
    `crates/ade_testkit/src/server_paths/replay.rs` with fixtures
    under `crates/ade_testkit/fixtures/server_paths/`.
  - Corpus excludes private-key material entirely (drives the
    server-side pipeline with captured `AcceptedBlock` artifacts from
    PHASE4-N-C's corpus).
  - `T-DET-01` strengthened by PHASE4-N-G (session transcript is a
    new authoritative-deterministic surface).
  - `T-ENC-01` strengthened by PHASE4-N-G (wire-byte transmission of
    canonical block bytes is a new hash-critical path).
  - `DC-CONS-16` strengthened by PHASE4-N-G (header/body split reused
    at the server-role surface).
  - `CN-CONS-07` strengthened by PHASE4-N-G (broadcast gate preserved
    across the network seam via `ServedChainSnapshot` admit pinned to
    `AcceptedBlock`).
  - `DC-PROTO-06` strengthened by PHASE4-N-G (version threaded through
    server-role surface).

- **Forbidden states across the cluster** (cross-slice invariants):
  - A `Block { bytes }` payload whose bytes are not in any
    `AcceptedBlock.as_bytes()` → compile-time impossible
    (`ProducerBlockFetchServerState` only constructs `Block { bytes }`
    via `ServedChainSnapshot::block_bytes(&self, point) -> &[u8]`
    accessor that returns `AcceptedBlock`-derived slice references).
  - `ServerReply<M>` carrying a client-agency variant → compile-time
    impossible (wrapper constructors do not exist for Client variants).
  - A parallel header/body splitter outside the validator-shared
    recipe → CI gate
    `ci/ci_check_no_parallel_header_splitter.sh` rejects.
  - Server-agency reducer returning without resolution → exhaustive
    match in the reducer signature; missing arm is a compile error.
  - Private keys in the new replay corpus → existing
    `ci/ci_check_no_private_keys_in_corpus.sh` extended to cover
    `crates/ade_testkit/fixtures/server_paths/`.
  - Server orchestrator depending on `ade_runtime::producer::signing`
    → CI dep-boundary gate.

- **Live-evidence conditionality**: CE-N-G-8's
  `blocked_until_operator_peer_available` mirrors PHASE4-N-C's
  conditional-closure pattern. If a private Haskell peer is not
  available at cluster close, the mechanical half (CE-N-G-7) plus
  all enforced invariants close the cluster; CE-N-G-8 is recorded
  with the conditional status and a re-open obligation tied to
  peer provisioning. This is the third use of the
  `enforced + open_obligation` pattern in the project (after
  PHASE4-N-C's `CN-CONS-06` and `OP-OPS-04`).

## CE coverage matrix

| CE | Slice | Registry IDs flipped to `enforced` on close |
|----|----|----|
| CE-N-G-1 | S1 | CN-PROTO-06 |
| CE-N-G-2 | S2 | *(contributes to CN-CONS-07 strengthening)* |
| CE-N-G-3 | S3 | DC-PROTO-08 |
| CE-N-G-4 | S4 | *(contributes to DC-CONS-17, DC-CONS-18 enforcement)* |
| CE-N-G-5 | S5 | DC-CONS-17, DC-CONS-18, DC-PROTO-07 |
| CE-N-G-6 | S6 | *(confirms CN-PROTO-06 end-to-end; strengthens DC-PROTO-06)* |
| CE-N-G-7 | S7 | *(mechanical pre-condition for RO-LIVE-01)* |
| CE-N-G-8 | S7 | RO-LIVE-01 (enforced or enforced+open_obligation) |

All 6 new N-G registry entries reachable. Five existing entries
strengthened (T-DET-01, T-ENC-01, DC-CONS-16, CN-CONS-07,
DC-PROTO-06). No carry-forward.
