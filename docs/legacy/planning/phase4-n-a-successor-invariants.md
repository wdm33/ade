# PHASE4-N-A-successor — Invariants sketch

**Concept.** Producer-side block-fetch and chain-sync **server response paths**.
PHASE4-N-C (HEAD `df56e2d`) closed the producer pipeline up to "valid
`AcceptedBlock` bytes sitting in a `BroadcastQueue`." Nothing currently
reads that queue and answers peer `MsgRequestRange` or extends the
announced chain with `MsgRollForward`. This cluster closes the last
engineering gap so a Haskell cardano-node peer can fetch a block we
forged — bounty acceptance test #3 ("produce a valid block accepted on
preview/preprod").

**Status.** Invariants sketch only. Cluster plan + slice docs follow per
the IDD workflow. Not yet implementation-bound.

## Scope decisions (resolved before this sketch)

1. **Served-chain scope — narrow.** The producer serves only blocks
   admitted from this session's `AcceptedBlock` / `BroadcastQueue`
   path. Broader serving from a co-located N-D ChainDB (historical
   range, rollback, intersection over arbitrary points, eviction,
   recovery) is **out of scope** here and is reserved for a later
   N-D-bridge / served-chain-index cluster. The narrow scope is
   sufficient for the bounty's "Haskell peer fetches our forged block"
   claim and avoids silently expanding the authority surface from
   "serve the freshly-forged block" to "be a ChainDB replacement."
2. **Internal served-chain representation — permitted divergence.**
   As long as wire-visible semantics + replay evidence hold, the
   in-memory served-chain shape is allowed to diverge from cardano-node's
   ChainDB. The Haskell node is the compatibility oracle, not the
   architecture law. (See [[feedback-diverge-on-internal-surfaces]].)
3. **N2N only.** N2C block-fetch / local-chain-sync is out of scope;
   that surface stays with CE-NODE-N2C-LTX-adjacent work.

## 1. What must always be true

- **I-1 — Served-bytes parity.** Any block bytes delivered via
  block-fetch `Block { bytes }` are byte-identical to the bytes inside
  the `AcceptedBlock` that cleared self-accept. The producer never
  re-encodes a forged block for transmission. *(Strengthens T-ENC-01,
  DC-CONS-16.)*
- **I-2 — Header-body wire coherence.** The header bytes announced via
  chain-sync `RollForward { header, tip }` are exactly the header
  sub-segment of the `AcceptedBlock` whose body bytes are subsequently
  servable via block-fetch. A peer running `block_body_hash(body_segment)`
  on what we serve MUST equal the body-hash field of the announced
  header. *(Strengthens DC-CONS-16.)*
- **I-3 — Broadcast-gate preservation across the network seam.** Wire
  bytes for chain-sync `RollForward` and block-fetch `Block` are sourced
  exclusively from `AcceptedBlock` tokens dequeued from
  `BroadcastQueue` (or a derived served-chain index whose admit-path
  is the broadcast queue only). The type-level gate from CN-CONS-07 is
  preserved at the wire boundary.
- **I-4 — Closed server agency.** The producer-side session orchestrator
  can only construct outgoing mini-protocol messages tagged with
  Server agency. Client-originated messages from the server-role pump
  are unrepresentable in the public API; misuse is a compile error.
- **I-5 — Version-gated server replies.** The producer-side handler
  refuses to construct any reply whose `ChainSyncMessage` /
  `BlockFetchMessage` variant is gated above the negotiated
  mini-protocol version for the session. *(Strengthens DC-PROTO-06.)*
- **I-6 — Key-boundary preservation.** The server-role pump only
  touches `AcceptedBlock` bytes and per-session ephemeral state. No
  code path on the server-response side has access to VRF / KES
  private keys, opcert seeds, or any `producer::signing::*` material.
- **I-7 — FIFO-respect across the seam.** The order in which
  `AcceptedBlock` tokens are dequeued from `BroadcastQueue` is the
  order in which they become discoverable via chain-sync `RollForward`.
  Block N+1 cannot be announced before block N.
- **I-8 — Deterministic progress in server agency.** Once chain-sync
  enters a state where the server holds agency (`CanAwait` /
  `MustReply` / `Intersect`), the pure reducer must return exactly one
  of: a legal server reply, or a structured deterministic
  session-close / error. It must never return an ambiguous wait state
  unless the wait condition is an explicit replay input. *(No silent
  unbounded wait. The numeric SLA — "within N slot ticks" — is
  deliberately not a load-bearing invariant at this stage; slot ticks
  are RED/GREEN scheduling input unless captured as canonical replay
  inputs, and freezing an arbitrary bound now would over-specify.)*

## 2. What must never be possible

- **¬P-1.** Serving a block via block-fetch whose bytes do not appear
  in any `AcceptedBlock` token. *(Includes: re-encoded bytes, mutated
  bytes, bytes from an `AcceptedBlock` that was rolled back.)*
- **¬P-2.** Announcing a `RollForward { header, tip }` whose `tip`
  references a `(slot, hash)` the producer cannot subsequently serve
  via block-fetch.
- **¬P-3.** Sending any chain-sync server-originated message while the
  per-session state machine is in `Idle` or `Done`; sending any
  client-originated message from the server pump. *(Both already
  barred by BLUE; the orchestrator must not construct outputs that
  would fall into these holes.)*
- **¬P-4.** Returning from the per-session reducer in a server-agency
  state with neither a legal server message nor a structured close /
  error — i.e. an unbounded silent wait. *(The shape of I-8; the
  numeric bound is deferred.)*
- **¬P-5.** A single `AcceptedBlock` token producing serving-side
  state in more than one persistent index. *(One canonical
  served-chain admit path.)*
- **¬P-6.** A peer's `FindIntersect { points }` causing the server
  pump to disclose state about points the producer did not itself
  create. *(Bounds the FindIntersect answer surface under the narrow
  served-chain scope.)*
- **¬P-7.** A peer's `RequestRange` triggering allocation, file I/O,
  or work scaled to the chain length rather than the requested range.
  *(Resource-bound invariant; matters under adversarial test.)*

## 3. What must remain identical across executions

The **session transcript.** Given canonical inputs
`(negotiated_version, peer_message_sequence, broadcast_arrival_sequence,
session_event_sequence)`, the orchestrator MUST produce a byte-identical
sequence of outgoing `ChainSyncMessage` / `BlockFetchMessage` byte
frames. No wall-clock, no `HashMap` iteration over peer state, no
thread-scheduling-dependent ordering.

The orchestrator itself is RED, but its **per-session reducer** — the
function that, given a peer message + current orchestrator state +
current served-chain index, computes the next outgoing message — MUST
be a pure, deterministic transition, separable for replay.

## 4. What must be replay-equivalent

For a synthetic fixture corpus of `(peer_message_sequence,
broadcast_arrival_sequence, session_event_sequence)` triples:

- Replaying the corpus twice MUST produce byte-identical outgoing
  mini-protocol frame sequences.
- For block-fetch specifically: the served `Block { bytes }` payload
  sequence MUST be byte-identical to the `AcceptedBlock.as_bytes()`
  sequence dequeued from `BroadcastQueue` over the same fixture, in
  the same order.
- The corpus carries signed artifacts (the `AcceptedBlock` bytes),
  never private keys or seeds — same key-boundary discipline as N-C's
  fixtures.

## 5. State transitions in scope

```text
fn producer_chain_sync_serve(
    server_state: ProducerChainSyncServerState,
    in_msg: ChainSyncMessage,            // client-originated
    served_chain: &ServedChainSnapshot,
    version: ChainSyncVersion,
) -> Result<(ProducerChainSyncServerState, ServerReply<ChainSyncMessage>), Error>
```

```text
fn producer_chain_sync_advance_tip(
    server_state: ProducerChainSyncServerState,
    new_accepted: AcceptedBlockRef,      // dequeued from BroadcastQueue
) -> Result<(ProducerChainSyncServerState, Option<ChainSyncMessage /* RollForward */>), Error>
```

```text
fn producer_block_fetch_serve(
    server_state: ProducerBlockFetchServerState,
    in_msg: BlockFetchMessage,           // client-originated
    served_chain: &ServedChainSnapshot,
    version: BlockFetchVersion,
) -> Result<(ProducerBlockFetchServerState, Vec<ServerReply<BlockFetchMessage>>), Error>
```

```text
fn served_chain_admit(
    served: ServedChainSnapshot,
    block: AcceptedBlock,                // moved in; broadcast-gate preserved
) -> Result<ServedChainSnapshot, Error>
```

All four are pure, total (errors are structured, not panics), and
BLUE-eligible. `ServerReply<M>` is a closed wrapper that statically
limits constructible variants to server-agency-legal subsets of
`ChainSyncMessage` / `BlockFetchMessage` (the type-level enforcement
of I-4). The RED orchestrator wraps the four with the actual I/O, mux,
and peer-bookkeeping.

## 6. TCB color hypothesis

- **BLUE (new):**
  - `producer_chain_sync_serve`, `producer_chain_sync_advance_tip` —
    pure reducers over chain-sync server state.
  - `producer_block_fetch_serve` — pure reducer over block-fetch
    server state.
  - `served_chain_admit` — deterministic insertion of an
    `AcceptedBlock` into a canonical served-chain index.
  - A header-projection function
    `accepted_block_header_bytes(&AcceptedBlock) -> &[u8]` (or
    equivalent), reusing the validator-shared body-hash recipe's
    header/body split (DC-CONS-16). MUST NOT introduce a parallel
    splitter.
  - The `ServerReply<M>` closed wrapper (I-4 enforcement at the type
    level).
- **GREEN:**
  - A small adapter translating `BroadcastQueue::dequeue()` into
    `served_chain_admit` calls plus `producer_chain_sync_advance_tip`
    triggers. Pure, no I/O.
- **RED:**
  - The per-peer session orchestrator that owns the mux channel, runs
    the handshake-negotiated version, calls the BLUE reducers, and
    writes the encoded byte frames out.
  - Connection acceptance, peer lifecycle, back-pressure, timeouts.
- **Resolved color question.** `ServedChainSnapshot` is BLUE under the
  narrow scope decision — it is an in-memory snapshot derived
  deterministically from the broadcast-queue arrival sequence, with
  no cross-restart persistence in this cluster.

## 7. Open questions (must resolve before cluster-plan)

1. **Header projection authority.** Is there already a canonical
   "split header vs body bytes" function inside the validator-shared
   self-accept recipe, or does this cluster need to expose one?
   Reusing the existing DC-CONS-16 recipe is mandatory; we must not
   introduce a parallel header/body splitter. *(Survey before
   cluster-plan; likely a small extraction from the body-hash code
   path.)*
2. **`FindIntersect` answerability under narrow scope.** With the
   served-chain limited to this-session forged blocks, what does the
   producer answer to `FindIntersect { points }` for a point not in
   that set? `IntersectNotFound`? `IntersectFound` on a synthetic
   anchor (e.g. the producer's "session-start" tip)? Bounds ¬P-6 and
   determines what a Haskell peer sees on first contact.
3. **Multiple concurrent peers.** Per-peer independent server-state
   with all coordination through `ServedChainSnapshot`, or shared
   scheduler? Determines whether per-session replay is sufficient or
   if a multi-peer replay corpus is needed.
4. **Streaming-batch eviction interaction.** Under the narrow scope,
   eviction is bounded by session lifetime, so probably nil for this
   cluster. Confirm before cluster-plan.

## 8. Acceptance evidence shape

Mechanical CEs prove the BLUE reducers and the synthetic-corpus
session-transcript replay (closes I-1 through I-8 and ¬P-1 through
¬P-7).

The bounty's "cardano-node accepts our block" is the **live** half. It
is treated as **bounty / live-evidence**, not as a semantic invariant
(per RO-LIVE-01 below). Captured as an operator-action log against a
private Haskell peer, in the same shape as PHASE4-N-C's
`CN-CONS-06.open_obligation` for live block acceptance — `enforced +
open_obligation` until the log is committed.

---

## Proposed registry entries (6 entries — revised after review)

Matching the existing registry schema (tier / statement / source /
cross_ref / code_locus / tests / ci_script / status), status =
`"declared"` pre-implementation. `cluster` field will be filled by
`/cluster-doc` when the cluster ID is assigned.

```toml
[[rules]]
id = "DC-CONS-17"
tier = "derived"
statement = "Block bytes delivered via producer-side block-fetch Block{bytes} are byte-identical to AcceptedBlock.as_bytes() for the AcceptedBlock that cleared self_accept. The producer-side server never re-encodes."
source = null
cross_ref = ["T-ENC-01", "DC-CONS-16", "CN-CONS-07"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "DC-CONS-18"
tier = "derived"
statement = "Header bytes announced via chain-sync RollForward{header,tip} are the header sub-segment of the AcceptedBlock whose body bytes are subsequently servable via block-fetch. block_body_hash applied to the served body MUST equal the body-hash field of the announced header."
source = null
cross_ref = ["DC-CONS-16", "DC-CONS-17"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "DC-PROTO-07"
tier = "derived"
statement = "Given canonical inputs (negotiated_version, peer_message_sequence, broadcast_arrival_sequence, session_event_sequence), the producer-side chain-sync / block-fetch session orchestrator emits a byte-identical sequence of outgoing mini-protocol frames across replays. The per-session reducer is a pure deterministic transition."
source = null
cross_ref = ["T-DET-01", "DC-PROTO-06"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "DC-PROTO-08"
tier = "derived"
statement = "Once chain-sync enters a state where the server holds agency, the pure per-session reducer must return exactly one of: a legal RollForward, a legal RollBackward, a legal AwaitReply, or a structured deterministic session-close/error. It must not return an ambiguous wait state unless the wait condition is an explicit replay input."
source = null
cross_ref = ["DC-PROTO-06", "DC-PROTO-07"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "CN-PROTO-06"
tier = "derived"
statement = "The producer-side session orchestrator can only construct outgoing mini-protocol messages tagged with Server agency. Client-originated messages from the server-role pump are unrepresentable in the public API; misuse is a compile error (closed ServerReply<M> wrapper)."
source = null
cross_ref = ["DC-PROTO-06", "CN-CONS-07"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
attack_rationale = "A producer that can construct client-originated server-pump messages can confuse a peer's state machine or impersonate a peer to itself."
evidence_notes = "Enforcement is type-level: the ServerReply<M> wrapper's variants are restricted at the type system, not at runtime."

[[rules]]
id = "RO-LIVE-01"
tier = "release"
statement = "A Haskell cardano-node peer issuing RequestRange covering an Ade-forged block receives, via the producer-side block-fetch server, bytes that pass that peer's full header+body validation. Captured as an operator-action log against a private Haskell peer; the underlying semantic invariants are DC-CONS-17, DC-CONS-18, CN-PROTO-06, DC-PROTO-07, DC-PROTO-08, and the existing self-accept + body-hash recipe (DC-CONS-16)."
source = null
cross_ref = ["DC-CONS-16", "DC-CONS-17", "DC-CONS-18", "CN-PROTO-06", "DC-PROTO-07", "DC-PROTO-08", "CN-CONS-06"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
```

## Existing rules this cluster will eventually strengthen

The following rules will receive `strengthened_in += "<cluster-id>"`
at `/cluster-doc` time, once the cluster ID is assigned. They are NOT
modified by this sketch.

- `T-ENC-01` — canonical encoding (the wire-byte transmission corollary).
- `T-DET-01` — same canonical inputs → same authoritative bytes (the
  session-transcript surface).
- `DC-CONS-16` — body-hash parity via single canonical recipe
  (the wire/served-bytes corollary).
- `CN-CONS-07` — self-acceptance / type-level broadcast gate (preserved
  across the network seam).
- `DC-PROTO-06` — selected version threaded as explicit input (now
  threaded through the server-role surface too).

## Related

- [[project-phase4-n-c-handoff]] — the producer half this cluster
  completes the wire of.
- [[project-bounty-requirements]] — why this is the bounty-critical
  path.
- [[feedback-diverge-on-internal-surfaces]] — internal served-chain
  representation is permitted divergence.
- [[feedback-bounded-smoke-slices]] — live-evidence binaries are
  bounded operator-action evidence, never the deliverable.
- [[feedback-fail-closed-validation]] — applies to the server pump's
  rejection paths (¬P-3, ¬P-6).
