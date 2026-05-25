# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **47 CI checks** at HEAD (`a280954`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml`) for rule IDs; reads the
> Phase 4 cluster plan (`docs/active/phase_4_cluster_plan.md`), the
> closed N-D / N-A / N-B / N-E / N-C / B1 / B2 / B3 / B4 / B5 cluster
> docs, the OQ5-CREDENTIAL-FIDELITY, COMMITTEE-CRED-FIDELITY,
> DREP-VOTE-FIDELITY, ENACTMENT-COMMITTEE-FIDELITY,
> ENACTMENT-COMMITTEE-WRITEBACK and PROPOSAL-PROCEDURES-DECODE cluster
> docs, and the **just-closed and archived PHASE4-N-G cluster doc +
> S1..S7 slice docs**
> (`docs/clusters/completed/PHASE4-N-G/cluster.md` + `N-G-S{1..7}.md` +
> `CE-N-G-8_PROCEDURE.md`).
>
> **This is the PHASE4-N-G FULL CLOSE refresh (HEAD `a280954`).** The
> previous SEAMS (HEAD `694dd74`) pinned the PHASE4-N-C full-close
> state. Seven N-G slices have landed between that revision and this
> one and close the producer-side **server response paths** for
> chain-sync and block-fetch:
>
> 1. **N-G-S1 (commit `8cd17c9`)** ships the BLUE header projection
>    authority `ade_ledger::block_validity::accepted_block_header_bytes`
>    (single canonical header/body splitter, lifted from the existing
>    body-hash recipe — no parallel splitter), plus the **closed
>    `ServerReply<M>` type wrappers** in
>    `ade_network::chain_sync::server` and
>    `ade_network::block_fetch::server`: inner enum private; only
>    Server-agency variants have public constructors; exit via
>    `into_message()`. Compile-time enforcement that the server pump
>    cannot emit client-agency messages. Registry rules `CN-PROTO-06`
>    + `DC-CONS-18` introduced; CI gate
>    `ci/ci_check_no_parallel_header_splitter.sh` introduced.
> 2. **N-G-S2 (commit `dc069cf`)** ships the BLUE canonical served-chain
>    index `ade_ledger::producer::served_chain::{ServedChainSnapshot,
>    served_chain_admit, ServedChainAdmitError}`. `ServedChainSnapshot`
>    is `BTreeMap`-backed (no `HashMap` — DC-PROTO-07 foundation);
>    `served_chain_admit` derives `(slot, hash)` from the bytes via
>    `decode_block` (no caller-asserted hash); the only entry path is
>    via an `AcceptedBlock` token (CN-CONS-07 preserved across the
>    network seam). CI gate `ci/ci_check_served_chain_closure.sh`
>    introduced; `CN-CONS-07.strengthened_in += PHASE4-N-G`.
> 3. **N-G-S3 (commit `cc49b1d`)** ships the BLUE chain-sync server
>    reducers `ade_network::chain_sync::server::{producer_chain_sync_serve,
>    producer_chain_sync_advance_tip}` plus the closed
>    **`ServedHeaderLookup` trait** (read-side seam over the producer's
>    served chain; `next_after(cursor)` / `intersect(points)` /
>    `tip()`), the closed `ProducerChainSyncServerState`,
>    `ProducerServerError`, and `ServerStep` sums. Deterministic
>    resolution per DC-PROTO-08 — no ambiguous wait. Registry rule
>    `DC-PROTO-08` introduced; CI gate
>    `ci/ci_check_chain_sync_server_closure.sh` introduced.
> 4. **N-G-S4 (commit `03d120f`)** ships the BLUE block-fetch server
>    reducer `ade_network::block_fetch::server::producer_block_fetch_serve`
>    plus the closed **`ServedRangeLookup` trait**
>    (`range_bytes(from, to)` over the served chain), the closed
>    `ProducerBlockFetchServerState`, `ProducerBlockFetchServerError`,
>    and `BlockFetchServerStep` sums. Served `Block { bytes }` payloads
>    are `AcceptedBlock` slices verbatim (DC-CONS-17 enforcement
>    foundation). Registry rule `DC-CONS-17` introduced; CI gate
>    `ci/ci_check_block_fetch_server_closure.sh` introduced.
> 5. **N-G-S5 (commit `1a1b8e0`)** ships the GREEN broadcast→served
>    adapter `ade_runtime::producer::broadcast_to_served::drain_and_admit`
>    plus the GREEN trait-impl bridge
>    `ade_runtime::producer::served_chain_lookups::ServedChainLookups`
>    (single production impl of `ServedHeaderLookup` + `ServedRangeLookup`).
>    Replay corpus driver lives in
>    `crates/ade_runtime/tests/server_paths_transcript_replay.rs`.
>    Registry rules `DC-CONS-17`, `DC-CONS-18`, `DC-PROTO-07` flip to
>    `enforced` at this slice; CI gate
>    `ci/ci_check_broadcast_to_served_purity.sh` introduced.
> 6. **N-G-S6 (commit `f773b1c`)** ships the RED per-peer N2N server
>    session driver `ade_runtime::network::n2n_server::{PerPeerN2nServerState,
>    DispatchError, dispatch_chain_sync_frame, dispatch_block_fetch_frame,
>    poll_chain_sync_advance}`. Pure state-machine driver — no socket
>    I/O; key-boundary preserved (cannot import from
>    `ade_runtime::producer::signing`, defended by
>    `ci/ci_check_n2n_server_no_signing_dep.sh`). Multi-peer
>    determinism test in `crates/ade_runtime/tests/n2n_server_two_peer_determinism.rs`.
>    `DC-PROTO-06.strengthened_in += PHASE4-N-G`.
> 7. **N-G-S7 (commit `a280954`)** ships the mechanical cross-impl
>    adapter in `crates/ade_runtime/tests/cross_impl_server_pipeline.rs`
>    (every served `Block { bytes }` decodes via Ade's own
>    envelope+block decoder and the recomputed body-hash matches the
>    announced header's body-hash field) plus the **fourth**
>    operator-action probe binary `ade_core_interop::bin::live_block_fetch_session`.
>    Registry rule `RO-LIVE-01` introduced at `status = "partial"` with
>    `open_obligation = blocked_until_operator_peer_available`; CI gate
>    `ci/ci_check_server_paths_corpus_present.sh` introduced. CE-N-G-8
>    procedure documented at
>    `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`.
>
> **THE KEY FULL-CLOSE DELTAS.** The prior SEAMS revision flagged the
> producer-side block-fetch server response path as the **N-A successor
> gap** (option A in the N-C handoff). PHASE4-N-G closes that gap end
> to end. Two §1 surface rows flip from "candidate" to "wired & closed":
>
> - **N2N producer-side block-fetch server role** → wired via
>   `producer_block_fetch_serve` consuming `ServedRangeLookup`,
>   producing closed `ServerReply<BlockFetchMessage>` wrappers.
> - **N2N producer-side chain-sync extension** → wired via
>   `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`
>   consuming `ServedHeaderLookup`, producing closed
>   `ServerReply<ChainSyncMessage>` wrappers.
>
> Counts at this refresh: **+7 CI scripts** (40 → 47:
> `ci_check_no_parallel_header_splitter.sh`,
> `ci_check_served_chain_closure.sh`,
> `ci_check_chain_sync_server_closure.sh`,
> `ci_check_block_fetch_server_closure.sh`,
> `ci_check_broadcast_to_served_purity.sh`,
> `ci_check_n2n_server_no_signing_dep.sh`,
> `ci_check_server_paths_corpus_present.sh`); **+6 registry rules**
> introduced (`DC-CONS-17`, `DC-CONS-18`, `DC-PROTO-07`, `DC-PROTO-08`,
> `CN-PROTO-06`, `RO-LIVE-01`); **3 carried rules strengthened**
> (`CN-CONS-07.strengthened_in += PHASE4-N-G`,
> `DC-PROTO-06.strengthened_in += PHASE4-N-G`,
> `OP-OPS-04.strengthened_in += PHASE4-N-G`); **+1 carried universal
> rule strengthened** by transitive byte-determinism
> (`T-DET-01.strengthened_in += PHASE4-N-G`,
> `T-ENC-01.strengthened_in += PHASE4-N-G`,
> `DC-CONS-16.strengthened_in += PHASE4-N-G`); **+2 new BLUE submodules**
> (`ade_network::chain_sync::server`, `ade_network::block_fetch::server`);
> **+1 new BLUE submodule** (`ade_ledger::producer::served_chain`);
> **+1 new BLUE public accessor** (`accepted_block_header_bytes` in
> `ade_ledger::block_validity`); **+2 new GREEN submodules**
> (`ade_runtime::producer::broadcast_to_served`,
> `ade_runtime::producer::served_chain_lookups`); **+1 new RED
> submodule** (`ade_runtime::network::n2n_server`); **+1 new
> operator-action probe binary** (`live_block_fetch_session` — fourth
> in the family alongside `live_consensus_session` (N-B),
> `live_tx_submission_session` (N-E), and `live_block_production_session`
> (N-C)); **+1 new live-evidence procedure doc**
> (`CE-N-G-8_PROCEDURE.md`); **0 new operator-action live-evidence
> log artifacts at this HEAD** — CE-N-G-8 is recorded
> `blocked_until_operator_peer_available` per `RO-LIVE-01`
> `open_obligation`. Total invariant registry: **196 entries** (190 →
> 196).

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-G is fully closed at this HEAD.** The producer-side
chain-sync + block-fetch server response paths — i.e. the only path
by which an externally-issued `RequestNext` / `FindIntersect` /
`RequestRange` is answered with bytes the producer has forged — are
all wired and CI-defended. The crypto-level live-peer claim (CE-N-G-8)
is `blocked_until_operator_peer_available` per `RO-LIVE-01`
`open_obligation`.

**PHASE4-N-C remains fully closed** (carried). **PHASE4-N-E
(Tier 1 wire-level mempool ingress) remains fully closed** (carried).
**PROPOSAL-PROCEDURES-DECODE remains fully closed** (carried).
**PHASE4-B3..B5, OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK**
all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there are **seven** fully-wired *external* ingress
> surfaces (block bytes, Plutus script bytes, snapshot bytes, Ouroboros
> mux frames, genesis JSON bundles, chain-selector stream inputs, and
> the N-E wire-level mempool ingress), plus — **newly closed at this
> HEAD** — the **producer-side server-role ingress** (peer-originated
> chain-sync + block-fetch frames reduced through closed
> `ServerReply<M>` wrappers back onto the wire). All internal
> composition roots are unchanged from N-C close (`block_validity` /
> `tx_validity` / `mempool_ingress` / `forge_block` / `self_accept`).

### Surface: Producer-side chain-sync server-role ingress (NEW in N-G-S1/S3/S5/S6 — peer→BLUE→wire seam)

```
Surface: A peer-originated ChainSyncMessage frame
         (RequestNext | FindIntersect{points} | Done)
         delivered by a real cardano-node peer over N2N mux,
         against the producer's ServedChainSnapshot
Reduces to: ServerReply (closed wrapper; inner enum private) projected
            via into_message() onto exactly one of five
            server-agency ChainSyncMessage variants
            { RollForward{header,tip} | RollBackward{point,tip}
            | AwaitReply | IntersectFound{point,tip}
            | IntersectNotFound{tip} }
            — OR ServerStep::Done on Client `Done`
            — OR ProducerServerError::Grammar(_) on illegal pairing
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED transport (ade_network::mux::transport) decodes mux frame
  2. BLUE chain-sync codec (N-A) — decode_chain_sync_message
  3. RED dispatcher (ade_runtime::network::n2n_server) wraps state
  4. BLUE grammar gate — chain_sync_transition(state, Client, version, msg)
       (N-A; total state graph; rejects client-from-server agency)
  5. BLUE producer logic — producer_chain_sync_serve dispatches on msg:
       - RequestNext: check served.next_after(cursor) -> RollForward
                       or park MustReply -> AwaitReply
       - FindIntersect: served.intersect(&points) -> IntersectFound
                        or IntersectNotFound
       - Done: ServerStep::Done
  6. BLUE projection — ServerReply::into_message() yields wire-grammar
       ChainSyncMessage; CN-PROTO-06 holds by construction
       (no public constructor for client variants exists)
  7. BLUE chain-sync codec (N-A) — encode_chain_sync_message
  8. RED transport — mux frame back to peer
Deferred RollForward: producer_chain_sync_advance_tip is polled by the
  orchestrator after each drain_and_admit; in CanAwait/MustReply with
  a new served block past the cursor, returns a fresh RollForward;
  otherwise None.
Cross-surface state sharing: per-peer state is fully independent
  (one PerPeerN2nServerState per session). The ONLY cross-peer shared
  state is the read-only &ServedChainSnapshot (a single index of
  AcceptedBlocks); the orchestrator hands every reducer call this
  shared reference. Determinism property: per-session transcript is
  invariant under interleaving of other peers' frames (N-G S6
  two-peer determinism test).
```

**Rule.** `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`
are the **single producer-side chain-sync server composition root**.
The `ServedHeaderLookup` trait is the **closed read-side seam** —
fixed at this cluster close; **no plug-in extension at runtime**. New
impls would be a deliberate registry-tracked addition. Today there is
**one production impl** (`ade_runtime::producer::served_chain_lookups::ServedChainLookups`)
and one test impl (in `chain_sync/server.rs`'s test module). The
`ServerReply` wrapper is a **closed type-level closure** —
client-agency message constructors are unrepresentable in the public
API (CN-PROTO-06 by construction; CI-defended by
`ci_check_chain_sync_server_closure.sh` + the compile-time match
exhaustiveness proof in tests). **New work** that adds a server-role
chain-sync feature attaches by extending the `ServerStep` arms inside
the reducer — not by exposing raw `ChainSyncMessage` returns from
`chain_sync::server`, not by adding a parallel header splitter.
**Deterministic-resolution discipline (DC-PROTO-08)**: in `MustReply`
the reducer never returns `Ok((MustReply, silent-wait))` without an
explicit wait condition. New trigger conditions for deferred
RollForwards must come via `advance_tip` polled by the orchestrator,
not via side-channel.

### Surface: Producer-side block-fetch server-role ingress (NEW in N-G-S1/S4/S5/S6 — peer→BLUE→wire seam)

```
Surface: A peer-originated BlockFetchMessage frame
         (RequestRange{from,to} | ClientDone)
         delivered by a real cardano-node peer over N2N mux,
         against the producer's ServedChainSnapshot
Reduces to: BlockFetchServerStep::Replies(Vec<ServerReply>) projected
            via into_message() onto a sequence of four possible
            server-agency BlockFetchMessage variants
            { StartBatch | NoBlocks | Block{bytes} | BatchDone }
            — OR BlockFetchServerStep::Done on ClientDone
            — OR ProducerBlockFetchServerError::Grammar(_) on
              illegal pairing
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED transport (ade_network::mux::transport) decodes mux frame
  2. BLUE block-fetch codec (N-A) — decode_block_fetch_message
  3. RED dispatcher (ade_runtime::network::n2n_server) wraps state
  4. BLUE grammar gate — block_fetch_transition(state, Client,
       version, msg) (N-A; total state graph)
  5. BLUE producer logic — producer_block_fetch_serve dispatches:
       - RequestRange with Origin endpoints: [NoBlocks]
       - RequestRange (from, to) -> served.range_bytes(from, to)
         (BTreeMap inclusive range over admitted blocks)
         -> empty: [NoBlocks]
         -> non-empty: [StartBatch, Block{bytes}*, BatchDone]
       - ClientDone: BlockFetchServerStep::Done
  6. BLUE projection — ServerReply::into_message() per reply; bytes
       are sourced verbatim from the ServedRangeLookup output, which
       is itself sourced from AcceptedBlock.as_bytes() (DC-CONS-17 by
       construction)
  7. BLUE block-fetch codec (N-A) — encode_block_fetch_message
  8. RED transport — mux frames back to peer
Cross-surface state sharing: same as chain-sync — per-peer state is
  independent; only the read-only &ServedChainSnapshot is shared.
```

**Rule.** `producer_block_fetch_serve` is the **single producer-side
block-fetch server composition root**. The `ServedRangeLookup` trait
is the **closed read-side seam** — fixed at this cluster close; **no
plug-in extension at runtime**. New impls would be a deliberate
registry-tracked addition. Today there is **one production impl**
(`ServedChainLookups`) and one test impl. The `ServerReply` wrapper
is again the closed type-level closure (CN-PROTO-06); the reducer
itself never re-encodes block bytes — every `Block { bytes }` payload
it constructs is a verbatim `AcceptedBlock` slice from
`ServedRangeLookup::range_bytes()` (DC-CONS-17, defended by
`ci_check_block_fetch_server_closure.sh`). **New work** that adds a
server-role block-fetch feature attaches by extending the served-chain
index or the lookup trait impls — not by introducing a parallel block
source, not by re-encoding wire bytes from parsed pieces.

### Surface: Forge-block transition (carried unchanged from N-C)

Carried. The producer's forge transition is upstream of N-G's server
paths: forged bytes → `AcceptedBlock` (via `self_accept`) →
`BroadcastQueue::enqueue` → `BroadcastQueue::dequeue` → GREEN
`drain_and_admit` (N-G S5) → `ServedChainSnapshot` →
`producer_*_serve` consumes. N-G adds no new step to the forge path
itself; the seam is the GREEN drain.

### Surface: Self-accept broadcast gate (carried unchanged from N-C; strengthened across the network seam)

Carried. **N-G strengthening:** `AcceptedBlock` is now also the
**single gate keeping non-self-accepted bytes out of the served
chain**. The only public path into `ServedChainSnapshot` is
`served_chain_admit(snap, AcceptedBlock)` — and the only
`AcceptedBlock` constructor remains the `Ok(...)` arm of
`self_accept`. `CN-CONS-07` now reads end-to-end: a forged block
whose body-hash, KES signature, leader claim, or body validity
disagrees with Ade's own validator can neither broadcast (N-C-S5) nor
**be served to any peer** (N-G-S2). Registry rule
`CN-CONS-07.strengthened_in += PHASE4-N-G`.

### Surface: Scheduler input ingress (carried unchanged from N-C)

Carried. **N-G note:** the scheduler is the producer-side trigger
into `BroadcastQueue`; N-G's `drain_and_admit` is the GREEN bridge
from there to the served chain. Scheduler state and `SchedulerInput`
are unchanged.

### Surface: Mempool ingress (Tier-1 wire-level — wired in N-E; unchanged)

Carried. N-G does not touch this surface.

### Surface: Conway tx-body `proposal_procedures` sub-grammar (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Surface: Single-tx validity (composition root — wired in B2; unchanged)

Carried.

### Surface: Mempool admission (Tier-1 gate — wired in B2; unchanged)

Carried.

### Surface: Full block validity (composition root — wired in B1; strengthened across the network seam at N-G)

Carried. **N-G strengthening:** the validator-shared body-hash recipe
in `ade_ledger::block_validity::header_input` now has a **second
public consumer** beyond the validator and producer: the GREEN
`ServedChainLookups` adapter calls `accepted_block_header_bytes` to
project a header for `RollForward { header, tip }`. The validator,
the producer (via `forge_block`), and the served-chain adapter all
hash through the same recipe — there is **one** header/body splitter
in the entire workspace (defended by
`ci_check_no_parallel_header_splitter.sh`). `DC-CONS-16` strengthened
in N-G.

### Surface: Block bytes, Plutus script bytes, Snapshot bytes, Consensus-input extraction, Ouroboros mux frames, Genesis JSON bundles, Chain-selector stream inputs (carried)

All seven external ingress surfaces are unchanged at this HEAD.
**N-G note (mux frames):** N-G closes the producer-side
**send-direction** for two mini-protocols (chain-sync server-role
replies, block-fetch server-role replies). The receive-side (an
externally-arriving block's header triggering a full-block decision)
is **still** a candidate surface — see below.

### Candidates — surfaces not yet wired (Phase 4 N-F, B+ residuals; receive-side header→body bridge; PP open obligations)

The following surfaces are named in the Phase 4 plan / B+ planning /
the PP open-obligation set but have no source today. They are listed
so future slice docs can attach without reinventing the reduction
step. **Each is a candidate seam pending confirmation at cluster
entry.**

- **N-G-S3/S4 WIRED AND CLOSED the prior revision's "N-A successor
  block-fetch server role + chain-sync extension" candidate** —
  removed (now `producer_block_fetch_serve` +
  `producer_chain_sync_serve` + `producer_chain_sync_advance_tip`).
- **N-G-S6 WIRED AND CLOSED the per-peer N2N server orchestrator
  gap** — removed (now `dispatch_chain_sync_frame` /
  `dispatch_block_fetch_frame` / `poll_chain_sync_advance` in
  `ade_runtime::network::n2n_server`). The actual socket → driver
  binding remains an out-of-band operator step (one layer up — the
  driver is a pure state machine).
- **NEW CANDIDATE (flagged by N-G close — original sketch
  "§Open questions" and the N-C handoff option B): the receive-side
  header→body bridge.** With send-direction server response paths
  closed by N-G, the natural next seam is the receive-side: an
  externally-arriving header (delivered through `process_stream_input`)
  triggering a `block_validity` decision on the subsequently-fetched
  body. Today the validator can decide a full block end-to-end
  (`block_validity`) and the chain-selector can ingest a candidate
  header (`process_stream_input`); what is missing is the composition
  layer in `ade_node` that joins them — receive header → request body
  via the existing N-A block-fetch client → run `block_validity` →
  fork-choice. **This is a candidate seam for the next cluster
  planner** — surface it; do not invent invariants for it here.
- **PROPOSAL-PROCEDURES-DECODE remains closed** (carried). The four
  PP open obligations remain separable candidate seams (carried).
- **PHASE4-N-E remains closed** (carried).
- **PHASE4-N-C remains closed** (carried). CE-N-C-8 still
  `blocked_until_operator_stake_available`.

| Cluster | Surface | Expected reduction target | Expected chokepoint | Confidence |
|---------|---------|---------------------------|---------------------|------------|
| **PHASE4-N-G** *(FULLY CLOSED at this HEAD — mechanical close; live half blocked_until_operator_peer_available)* | **Producer-side server-role ingress: peer chain-sync + block-fetch frames → ServerReply<M> → wire frames** | per-peer `(PerPeerN2nServerState, Vec<Vec<u8>>)`; closed `ServerReply<_>` wrappers with private inner enums; `ServedChainSnapshot` as the single bytes source | **DONE:** `ade_network::chain_sync::server::{ServerReply, ProducerChainSyncServerState, ProducerServerError, ServerStep, ServedHeaderLookup, HeaderProjection, producer_chain_sync_serve, producer_chain_sync_advance_tip}` (BLUE); `ade_network::block_fetch::server::{ServerReply, ProducerBlockFetchServerState, ProducerBlockFetchServerError, BlockFetchServerStep, ServedRangeLookup, producer_block_fetch_serve}` (BLUE); `ade_ledger::producer::served_chain::{ServedChainSnapshot, served_chain_admit, ServedChainAdmitError}` (BLUE); `ade_ledger::block_validity::accepted_block_header_bytes` (BLUE public accessor); `ade_runtime::producer::{broadcast_to_served::drain_and_admit, served_chain_lookups::ServedChainLookups}` (GREEN); `ade_runtime::network::n2n_server::{PerPeerN2nServerState, DispatchError, dispatch_chain_sync_frame, dispatch_block_fetch_frame, poll_chain_sync_advance}` (RED). CI gates `ci_check_no_parallel_header_splitter.sh`, `ci_check_served_chain_closure.sh`, `ci_check_chain_sync_server_closure.sh`, `ci_check_block_fetch_server_closure.sh`, `ci_check_broadcast_to_served_purity.sh`, `ci_check_n2n_server_no_signing_dep.sh`, `ci_check_server_paths_corpus_present.sh`. Registry rules `DC-CONS-17`, `DC-CONS-18`, `DC-PROTO-07`, `DC-PROTO-08`, `CN-PROTO-06` (`enforced`); `RO-LIVE-01` (`partial` with `open_obligation`); strengthens `CN-CONS-07`, `DC-PROTO-06`, `OP-OPS-04`, `DC-CONS-16`, `T-DET-01`, `T-ENC-01`. Tests: named tests across S1..S7 plus replay corpus + multi-peer determinism + cross-impl pipeline harness. | **wired & closed in PHASE4-N-G (mechanical half + structural cross-impl); live-peer cross-impl awaiting operator-supplied Haskell peer** |
| **CE-N-G-8 (cross-cluster obligation introduced in N-G S7; operator-action live evidence)** | **Live N2N block-fetch acceptance: a real cardano-node peer issues `RequestRange` covering an Ade-served block and accepts the served bytes under its own header+body validation** | The live cross-impl claim — same operator-action evidence pattern as CE-N-B-6 / CE-N-E-6 / CE-N-C-8 | The future evidence-capture pass via `live_block_fetch_session --connect` against a private cardano-node peer; procedure at `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`; output `CE-N-G-LIVE_<date>.log`. | **deferred operator-action obligation — `blocked_until_operator_peer_available` per `RO-LIVE-01.open_obligation`** |
| **NEW CANDIDATE — Receive-side header→body bridge** *(flagged by the N-G close — original sketch §"Open questions" + N-C handoff option B)* | **Externally-arriving header triggering a full-block decision on the fetched body** | `block_validity(...)` over the fetched body after `process_stream_input` ingests a candidate header | `ade_node` composition layer joining `process_stream_input` (N-B; existing) + N-A block-fetch **client** (existing) + `block_validity` (B1; existing). No new BLUE chokepoint is implied — it's a composition layer. | **candidate (next-cluster seam; surface; do not invent invariants here)** |
| **N-C+ (declared non-goal in N-C cluster doc; OQ-4 lock — separable future seam)** | **TPraos producer (Shelley..Alonzo full-block production)** | A TPraos-flavored `ProducerTick` arm + per-era body buckets | Extend `forge_block` to a closed `era` dispatch; today Conway/Praos only. | candidate (declared non-goal — explicit OQ-4 lock) |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | The deferred halves of CE-N-E-7 + CE-N-E-6 | The future node-binary cluster ships the live socket loops. | **deferred cross-cluster obligation (NOT an open seam in N-E)** |
| **PP OQ-1..OQ-4 (NEW separable seams — declared open obligations on DC-LEDGER-11)** | voting_procedures decode / ParameterChange.update nested / NewConstitution.raw nested / typed RewardAccount | per OQ | per OQ | candidate (carried) |
| B+ (full tx UTxO scope) | Full-scope single-tx validity over real resolved UTxO | `TxValidityVerdict` at `track_utxo=true` | `tx_validity` (existing) | candidate |
| B+ (Conway body witness depth) | **Conway block-body vkey-witness closure** — `project_conway_body_witness_gap` | `BlockValidityVerdict` whose body authority runs the same closure as `tx_phase_one` | wire `tx_phase_one` / `verify_required_witnesses` into the Conway block-body path in `rules.rs` | candidate (B2-carried) |
| B+ (pre-Conway tx) | Pre-Conway single-tx validity | `TxValidityVerdict` via per-era body decode + per-era `SignerSource` | extend `decode_tx` + add the era arm to `required_signers` | candidate |
| B1+ (pre-Babbage block) | TPraos full-block validity (Shelley..Alonzo) | `BlockValidityVerdict` via a TPraos `HeaderInput` projection | extend `block_validity::decode_block` to build `HeaderVrf::Tpraos` headers | candidate |
| N-F | LSQ semantic dispatch (LocalStateQuery payloads) | Internal Query enum (closed, not yet defined) | Single dispatch fn that consumes `LocalStateQueryMessage` opaque-bytes payloads | candidate |
| N-F | LocalTxMonitor semantic dispatch | Mempool-snapshot Query/Reply enums | Single dispatch fn that consumes `LocalTxMonitorMessage` opaque-bytes payloads | candidate |
| N-B+ | Live cardano-node session driver (for `ade_core_interop::live_consensus_session`) | `StreamInput` translated from `ChainSyncMessage` and `BlockFetchMessage` events | Composition layer in `ade_core_interop` | candidate |

### Operator-action evidence (live-wire artifacts — not BLUE seams)

The Ade workspace closes Tier-1 wire-level seams in two halves: a
mechanical / GREEN half (code + harness + CI gates that the workspace
itself can certify on every push) and a **live-wire operator-action
half** (a real peer / client at the other end of a real socket
producing bytes Ade has never seen).

**At this HEAD two live-evidence logs are committed**, two
cross-cluster obligations remain `blocked_until_operator_*_available`,
and one cross-cluster obligation is carried from N-E.

| Procedure | Evidence-log artifact | Status at HEAD | What it asserts | TCB |
|-----------|----------------------|----------------|------------------|-----|
| `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-B/CE-N-B-6_<date>.log` | **CAPTURED** (carried from N-B close) | Real cardano-node N-B follow-mode tip agreement | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` | `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_2026-05-25.log` | **CAPTURED** (carried from N-E close) | Outbound-client probe against a real preprod N2N relay | RED operator action |
| `docs/clusters/completed/PHASE4-N-E/CE-N-E-7_PROCEDURE.md` | (deferred) `CE-NODE-N2C-LTX_<date>.log` in the future node-binary cluster | **DEFERRED to CE-NODE-N2C-LTX** | Real `cardano-cli transaction submit` to Ade over the N2C UDS | RED operator action (deferred) |
| `docs/clusters/completed/PHASE4-N-C/CE-N-C-8_PROCEDURE.md` | (pending) `CE-N-C-LIVE_<date>.log` | **`blocked_until_operator_stake_available`** (carried from N-C close) | Cardano-node accepts an Ade-forged block as the next chain head | RED operator action |
| **`docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md` (NEW in N-G-S7)** | **(pending)** `docs/clusters/completed/PHASE4-N-G/CE-N-G-LIVE_<date>.log` | **`blocked_until_operator_peer_available`** (per `RO-LIVE-01.open_obligation`) | A real cardano-node peer issuing `RequestRange` over an Ade-served block via the Ade producer's N2N block-fetch server accepts the served bytes under its own header+body validation. Live cross-impl claim (the bytes-shape claim is mechanically closed by `cross_impl_server_pipeline`). | RED operator action |

**Operator-action probe binaries (RED — `ade_core_interop::bin::*`).**
At this HEAD there are **four** such binaries:

| Binary | Slice | Live-evidence target | Status |
|--------|-------|----------------------|--------|
| `live_consensus_session` (PHASE4-N-B) | N-B | CE-N-B-6 (live chain-sync follow-mode tip agreement) | captured |
| `live_tx_submission_session` (PHASE4-N-E S6) | N-E S6 | CE-N-E-6 (live N2N tx-submission2 outbound-client probe) | captured |
| `live_block_production_session` (PHASE4-N-C S7) | N-C S7 | CE-N-C-8 (live N2N block-fetch acceptance by cardano-node — **producer-side forge**) | blocked_until_operator_stake_available |
| **`live_block_fetch_session` (PHASE4-N-G S7) — NEW** | N-G S7 | CE-N-G-8 (live N2N **server-role** block-fetch served by Ade to cardano-node) | **blocked_until_operator_peer_available** |

**Pattern.** Hermetic default mode (readiness probe that runs in CI
without network access — gated `#[ignore]`); plus a `--connect <peer>`
live pass that the operator runs against a real cardano-node peer.
The binary's evidence log is committed alongside the `_PROCEDURE.md`
in the cluster directory. **N-G adds a second closure-mode variant**
(`blocked_until_operator_peer_available`) to the family. The two
modes are conceptually distinct:

- `blocked_until_operator_stake_available` (N-C / CE-N-C-8) — the
  blocker is **testnet SPO stake registration** the operator must
  provision before Ade can ever forge a block under a real opcert.
- `blocked_until_operator_peer_available` (N-G / CE-N-G-8) — the
  blocker is a **private Haskell cardano-node peer** the operator
  must spin up; no stake registration is required because the test is
  receive-side (the peer fetches an Ade-served block under its own
  validation; Ade does not need to be on-chain).

Both follow the OP-OPS-04 precedent (`enforced` core + structured
`open_obligation` naming the external dependency and the re-open
criteria).

**These are evidence-log patterns, not BLUE seams.**

User confirmation needed for each candidate at cluster entry. **The
most load-bearing remaining candidates for the bounty** are
**CE-N-C-8** (the live cardano-node forge acceptance), **CE-N-G-8**
(the live cardano-node block-fetch acceptance — the receive-side
counterpart of N-C), the **receive-side header→body bridge**,
**CE-NODE-N2C-LTX** (the deferred live N2C UDS server + N2N bulk-tx
inbound listener), and the four **PROPOSAL-PROCEDURES-DECODE open
obligations**.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **sixteen** authoritative domains. **PHASE4-N-G added one new
domain — producer-side server response authority** — a new BLUE
composition root pair (`producer_chain_sync_serve` +
`producer_block_fetch_serve`) consuming a closed BLUE index
(`ServedChainSnapshot`) via closed read-side trait seams
(`ServedHeaderLookup` + `ServedRangeLookup`), gated upstream by the
`AcceptedBlock` broadcast token (carried from N-C) and bridged by a
GREEN adapter (`drain_and_admit` + `ServedChainLookups`) plus a RED
per-peer session driver (`n2n_server`). Prior cluster narratives are
preserved unchanged below.

### Producer-side server response authority (NEW in PHASE4-N-G)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **BLUE header projection authority (S1)** | `ade_ledger::block_validity::accepted_block_header_bytes` | BLUE | The **single canonical** Cardano block-envelope header/body byte splitter. Lifted as a public accessor over the existing validator body-hash recipe (`header_cbor_slice` was already private here). The validator (B1), the producer (N-C `forge_block` via `block_body_hash_from_buckets`), and the served-chain adapter (N-G `ServedChainLookups`) all hash / project through this single site. Defended by `ci_check_no_parallel_header_splitter.sh`. |
| **BLUE closed type wrapper — chain-sync (S1)** | `ade_network::chain_sync::server::ServerReply` | BLUE | Closed wrapper with private inner enum; only Server-agency-legal `ChainSyncMessage` variants have public constructors (`roll_forward`, `roll_backward`, `await_reply`, `intersect_found`, `intersect_not_found`). The single exit is `into_message()`. CN-PROTO-06 by construction. |
| **BLUE closed type wrapper — block-fetch (S1)** | `ade_network::block_fetch::server::ServerReply` | BLUE | Same pattern; constructors `start_batch`, `no_blocks`, `block(bytes)`, `batch_done`. |
| **BLUE served-chain index (S2)** | `ade_ledger::producer::served_chain::{ServedChainSnapshot, served_chain_admit, ServedChainAdmitError}` | BLUE | Closed canonical index. `BTreeMap<(SlotNo, Hash32), AcceptedBlock>`-backed (no `HashMap`). The only entry path is `served_chain_admit(snap, AcceptedBlock)` — and the only `AcceptedBlock` constructor remains `self_accept`'s `Ok(...)` arm. Closed `ServedChainAdmitError` sum (`Decode`, `KeyByteConflict`). Defended by `ci_check_served_chain_closure.sh`. |
| **BLUE closed read-side trait — chain-sync (S3)** | `ade_network::chain_sync::server::ServedHeaderLookup` | BLUE | **Closed seam** — fixed at this cluster close; no plug-in extension. Methods: `next_after(cursor: Option<(SlotNo, Hash32)>) -> Option<HeaderProjection>`, `intersect(points: &[Point]) -> Option<(SlotNo, Hash32)>`, `tip() -> Option<(SlotNo, Hash32, u64)>`. Production impl: `ServedChainLookups` (GREEN, ade_runtime). New impls = deliberate registry-tracked addition. |
| **BLUE closed read-side trait — block-fetch (S4)** | `ade_network::block_fetch::server::ServedRangeLookup` | BLUE | **Closed seam.** Method: `range_bytes(from: (SlotNo, Hash32), to: (SlotNo, Hash32)) -> Vec<(SlotNo, Hash32, Vec<u8>)>`. Production impl: `ServedChainLookups`. |
| **BLUE chain-sync server reducers (S3)** | `ade_network::chain_sync::server::{producer_chain_sync_serve, producer_chain_sync_advance_tip, ProducerChainSyncServerState, ProducerServerError, ServerStep}` | BLUE | Pure, total, deterministic. Closed `ProducerChainSyncServerState`, `ProducerServerError` (1 variant: `Grammar(ChainSyncError)`), `ServerStep` (2 variants: `Done`, `Reply(ServerReply)`). Composes `chain_sync_transition` (N-A grammar gate) — does not re-implement the state graph. Defended by `ci_check_chain_sync_server_closure.sh`. |
| **BLUE block-fetch server reducer (S4)** | `ade_network::block_fetch::server::{producer_block_fetch_serve, ProducerBlockFetchServerState, ProducerBlockFetchServerError, BlockFetchServerStep}` | BLUE | Pure, total, deterministic. Closed `ProducerBlockFetchServerState`, `ProducerBlockFetchServerError` (1 variant: `Grammar(BlockFetchError)`), `BlockFetchServerStep` (2 variants: `Done`, `Replies(Vec<ServerReply>)`). Composes `block_fetch_transition` (N-A grammar gate). Defended by `ci_check_block_fetch_server_closure.sh`. |
| **GREEN broadcast→served adapter (S5)** | `ade_runtime::producer::broadcast_to_served::drain_and_admit` | GREEN | Pure function: drains `BroadcastQueue` and admits every `AcceptedBlock` into `ServedChainSnapshot` via the BLUE `served_chain_admit` chokepoint. Returns the updated snapshot, the drained queue, and the dequeue-ordered admitted blocks. Observably deterministic over captured arrival sequences. Defended by `ci_check_broadcast_to_served_purity.sh`. |
| **GREEN trait-impl bridge (S5)** | `ade_runtime::producer::served_chain_lookups::ServedChainLookups<'a>` | GREEN | Single production impl of both `ServedHeaderLookup` and `ServedRangeLookup` over `&'a ServedChainSnapshot`. Header projection delegates to `accepted_block_header_bytes` — no parallel splitter. Pure; no I/O. Lives in `ade_runtime` because the orphan rule prevents it from living in either `ade_ledger` or `ade_network` alone (the impl spans both crates' types). |
| **RED per-peer N2N server session driver (S6)** | `ade_runtime::network::n2n_server::{PerPeerN2nServerState, DispatchError, dispatch_chain_sync_frame, dispatch_block_fetch_frame, poll_chain_sync_advance}` | RED | Pure state-machine driver — **no socket I/O** (sockets live one layer up in `ade_network::session`). Decodes inbound mini-protocol frames, calls the BLUE reducers via the GREEN `ServedChainLookups`, encodes outgoing frames. Per-peer state independent; cross-peer coordination only via `&ServedChainSnapshot`. **Key boundary: MUST NOT import from `ade_runtime::producer::signing`** — defended by `ci_check_n2n_server_no_signing_dep.sh`. |
| **GREEN session-transcript replay corpus (S5/S7)** | `crates/ade_runtime/tests/server_paths_transcript_replay.rs`, `crates/ade_runtime/tests/n2n_server_two_peer_determinism.rs`, `crates/ade_runtime/tests/cross_impl_server_pipeline.rs` | GREEN | Replay scaffolding lives as integration tests in `ade_runtime/tests/`. Drives `(version, peer_message_sequence, broadcast_arrival_sequence, session_event_sequence)` through the full pipeline twice; outgoing frame sequences must be byte-identical (DC-PROTO-07). Cross-impl pipeline verifies every served `Block { bytes }` decodes via Ade's own decoder and the recomputed body-hash matches the announced header (DC-CONS-17 + DC-CONS-18). |
| **RED operator-action probe binary (S7)** | `ade_core_interop::bin::live_block_fetch_session` | RED | Fourth instance of the operator-action probe binary pattern. Hermetic default + `--connect` live pass. Drives the full producer-side server pipeline against a private cardano-node peer; logs JSON-Lines per RequestRange. Status `blocked_until_operator_peer_available`. |
| **CI gates (S1..S7)** | `ci/ci_check_{no_parallel_header_splitter, served_chain_closure, chain_sync_server_closure, block_fetch_server_closure, broadcast_to_served_purity, n2n_server_no_signing_dep, server_paths_corpus_present}.sh` | CI | 7 mechanical gates defending the producer-side server response authority surface. Total CI count: 40 → 47. |

**Rule.** This domain has **one BLUE header projection authority**
(`accepted_block_header_bytes`), **two BLUE closed type wrappers**
(`ServerReply` for chain-sync + block-fetch; private inner enums),
**one BLUE served-chain index** (`ServedChainSnapshot`), **two BLUE
closed read-side trait seams** (`ServedHeaderLookup`,
`ServedRangeLookup`), **three BLUE server reducers**
(`producer_chain_sync_serve`, `producer_chain_sync_advance_tip`,
`producer_block_fetch_serve`), **one GREEN broadcast→served adapter**
(`drain_and_admit`), **one GREEN trait-impl bridge**
(`ServedChainLookups`), **one RED per-peer session driver**
(`n2n_server` with three dispatch functions), **a session-transcript
replay corpus + multi-peer determinism + cross-impl pipeline harness**
(integration tests under `ade_runtime/tests/`), and **one RED
operator-action probe binary** (`live_block_fetch_session`).

**THE KEY SEAMS:**

1. **The `ServedHeaderLookup` + `ServedRangeLookup` traits are CLOSED
   seams** — fixed at this cluster close. **No plug-in extension at
   runtime.** Today there is exactly one production impl
   (`ServedChainLookups`) and one test impl. **New impls would be a
   deliberate registry-tracked addition**, not a runtime extension
   point. Documented in §3 below as Closed registries, not Extensible
   ones.
2. **`ServerReply<M>` is a CLOSED type-level closure** — inner enum
   private; no public constructor exists for client-agency variants;
   the only exit is `into_message()`. CN-PROTO-06 by construction.
   Defended by `ci_check_chain_sync_server_closure.sh` +
   `ci_check_block_fetch_server_closure.sh` (no `pub fn` returning
   raw `ChainSyncMessage` / `BlockFetchMessage` outside
   `into_message`).
3. **`ServedChainSnapshot` is the single source of served bytes** —
   only `served_chain_admit(snap, AcceptedBlock)` enters bytes;
   `AcceptedBlock` only via `self_accept`. End-to-end: a block whose
   validator-side decision would reject can never be served to a
   peer. CN-CONS-07 strengthened.
4. **`accepted_block_header_bytes` is the single header/body
   splitter** — validator, producer, and server all hash / project
   through the same site. No parallel splitter anywhere in the
   workspace. DC-CONS-16 strengthened.
5. **Per-peer state is independent** — multi-peer determinism is by
   construction (each `PerPeerN2nServerState` is independent;
   cross-peer coordination is only via the read-only
   `&ServedChainSnapshot`). The two-peer determinism test confirms
   per-session transcripts are invariant under interleaving.
6. **The RED orchestrator never sees private keys** — `n2n_server`
   has no path to `ade_runtime::producer::signing` (defended by
   `ci_check_n2n_server_no_signing_dep.sh`). Server response paths
   read served bytes; they do not sign anything. (Producer-side
   signing happens upstream, in N-C's `forge_block` chain.)
7. **Deterministic-resolution discipline (DC-PROTO-08)** — chain-sync
   server-agency reducers never return ambiguous `Ok((MustReply,
   silent-wait))` without an explicit replay-input wait condition.
   New trigger conditions for deferred RollForwards come via
   `advance_tip`, polled by the orchestrator after each
   `drain_and_admit`.

**New work** that adds a producer-side server-role feature attaches by
extending the closed `ServerStep` / `BlockFetchServerStep` arms inside
the existing reducers, by extending the closed `ServerReply`
constructor set, or by adding a new lookup trait method (closed-sum
extension, version-gated) — not by adding a parallel server path, not
by exposing raw `ChainSyncMessage` / `BlockFetchMessage` from
server-side `pub fn`s, not by introducing a second served-chain
index.

**Declared non-goals carried from the cluster doc:** broader serving
from a co-located N-D ChainDB (OQ-1 lock — reserved for a future
N-D-bridge / served-chain-index cluster), eviction of served entries
within this cluster (OQ-5 lock — narrow scope; memory bound is
session lifetime), N2C local-chain-sync / local-tx-submission server
response paths (OQ-8 lock — N2N only), the actual `tokio` socket loop
(one layer up — the driver is a pure state machine the operator binds
to a real socket in `ade_network::session::*`).

### Block production authority (carried unchanged from N-C; CN-CONS-07 strengthened in N-G)

Carried. **N-G strengthening:** the `AcceptedBlock` token now gates a
second authoritative output — admission into the served chain — in
addition to admission into the broadcast queue. The
`CN-CONS-07.strengthened_in` array gains `PHASE4-N-G`.

### Mempool ingress (carried unchanged from N-E)

Carried.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried unchanged from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged from the prior revision. **N-G-specific
strengthening:** the body-hash recipe authority (`block_body_hash`)
now has its header/body splitter (`accepted_block_header_bytes`)
consumed by a third client (the GREEN `ServedChainLookups` adapter,
beyond the validator and producer) — `DC-CONS-16.strengthened_in +=
PHASE4-N-G`. The handshake-negotiated version threading now extends
through the new server-role surface (`DC-PROTO-06.strengthened_in +=
PHASE4-N-G`).

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. N-G added an `ade_runtime → ade_network` edge (RED → BLUE
  via the `n2n_server` module importing `chain_sync::server` +
  `block_fetch::server` reducers) — allowed.
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE.
- **`ci_check_no_parallel_header_splitter.sh`** *(N-G-S1 — DC-CONS-16
  strengthening, DC-CONS-18)* — forbids any new `pub fn .*header_bytes`,
  `pub fn .*split_header`, or `pub fn .*split_block_envelope` outside
  the canonical site `crates/ade_ledger/src/block_validity/header_input.rs`.
  Positive presence check: `accepted_block_header_bytes` must exist
  at the canonical site.
- **`ci_check_served_chain_closure.sh`** *(N-G-S2 — CN-CONS-07
  strengthening)* — forbids `HashMap`/`HashSet` in
  `ade_ledger::producer::served_chain`; positive presence checks for
  the canonical `ServedChainSnapshot` + `served_chain_admit` +
  `ServedChainAdmitError` types.
- **`ci_check_chain_sync_server_closure.sh`** *(N-G-S3 — DC-PROTO-08,
  CN-PROTO-06)* — forbids `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock
  in `chain_sync/server.rs` production code; forbids `pub fn`
  returning raw `ChainSyncMessage` (exit must be via
  `ServerReply::into_message`); positive presence checks for
  `producer_chain_sync_serve`, `producer_chain_sync_advance_tip`,
  `ServedHeaderLookup`.
- **`ci_check_block_fetch_server_closure.sh`** *(N-G-S4 — DC-CONS-17,
  CN-PROTO-06)* — same shape for block-fetch; positive presence checks
  for `producer_block_fetch_serve` + `ServedRangeLookup`.
- **`ci_check_broadcast_to_served_purity.sh`** *(N-G-S5 — DC-PROTO-07)*
  — forbids `tokio`/wall-clock/`rand` in `broadcast_to_served.rs`
  production code; the GREEN adapter must be observably deterministic.
- **`ci_check_n2n_server_no_signing_dep.sh`** *(N-G-S6 — CN-PROTO-06,
  OP-OPS-04 strengthening)* — forbids any import path from
  `ade_runtime::network::n2n_server` into
  `ade_runtime::producer::signing`. The server response paths cannot
  sign anything; they only read served bytes.
- **`ci_check_server_paths_corpus_present.sh`** *(N-G-S7 — RO-LIVE-01)*
  — guards server-paths fixture corpus presence + expected transcript
  artifacts.
- *N-C carried CI gates:* `ci_check_private_key_custody.sh`,
  `ci_check_opcert_closed.sh`, `ci_check_forge_purity.sh`,
  `ci_check_no_private_keys_in_corpus.sh`,
  `ci_check_no_producer_body_encoder.sh`, `ci_check_self_accept_gate.sh`,
  `ci_check_scheduler_closure.sh`, `ci_check_producer_corpus_present.sh`.
- `ci_check_constitution_coverage.sh` — carried (allows enforcement
  evidence on release/operational entries when status is `enforced`).
- `ci_check_proposal_procedures_closed.sh` *(PP — DC-LEDGER-11)* — carried.
- `ci_check_mempool_ingress_closure.sh` / `ci_check_mempool_ingress_replay.sh`
  *(N-E — DC-MEM-03/04)* — carried.
- `ci_check_credential_discriminant_closed.sh` *(OQ5 / COMMITTEE /
  DREP / ENACTMENT — DC-LEDGER-10)* — carried.
- `ci_check_gov_cert_accumulation_closed.sh` *(B5 — DC-LEDGER-09)* —
  carried.
- `ci_check_deposit_param_authority.sh` *(B3 — DC-TXV-07)* — carried.
- `ci_check_conway_cert_classification_closed.sh` *(B3F — DC-TXV-06)*
  — carried.
- `ci_check_no_chaindb_in_consensus_blue.sh` / `ci_check_no_float_in_consensus.sh`
  / `ci_check_no_density_in_fork_choice.sh` / `ci_check_consensus_closed_enums.sh`
  — carried.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh` —
  carried.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PHASE4-N-G
added thirteen closed surfaces** — `ServerReply` (chain-sync, closed
wrapper over a private 5-variant inner enum), `ServerReply`
(block-fetch, closed wrapper over a private 4-variant inner enum),
`HeaderProjection` (closed struct), `ProducerChainSyncServerState`
(closed struct), `ProducerServerError` (closed 1-variant sum),
`ServerStep` (closed 2-variant sum), `ProducerBlockFetchServerState`
(closed struct), `ProducerBlockFetchServerError` (closed 1-variant
sum), `BlockFetchServerStep` (closed 2-variant sum),
`ServedChainSnapshot` (closed BLUE index type), `ServedChainAdmitError`
(closed 2-variant sum), `PerPeerN2nServerState` (closed RED struct),
`DispatchError` (closed 4-variant RED sum), **plus two closed trait
seams** (`ServedHeaderLookup`, `ServedRangeLookup`) and **the
canonical accessor** `accepted_block_header_bytes`. Plus **seven CI
gates** (CI count 40 → 47) and **six newly-introduced registry
rules + four strengthenings** (registry total 190 → 196).

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
| `ProducerTick` *(N-C-S3 — DC-CONS-13)* | `ade_ledger::producer::state` | closed 14-field struct | Carried; canonical input value to `forge_block`. |
| `forge_block` chokepoint *(N-C-S3)* | `ade_ledger::producer::forge` | 1 function | Carried. |
| `ForgeError` / `ForgeEffects` / `ForgedBlock` *(N-C-S3)* | `ade_ledger::producer::forge` | 7 / 1 / closed struct | Carried. |
| `encode_opcert` / `decode_opcert` chokepoint pair *(N-C-S2)* | `ade_codec::shelley::opcert` | 2 functions | Carried. |
| `OpCertCodecError` *(N-C-S2)* | `ade_codec::shelley::opcert` | 7 variants | Carried. |
| `opcert_validate` chokepoint *(N-C-S2)* | `ade_core::consensus::opcert_validate` | 1 function | Carried. |
| `OpCertError` *(N-C-S2)* | `ade_core::consensus::opcert_validate` | closed validation-error sum | Carried. |
| `block_body_hash_from_buckets` chokepoint *(N-C-S4 — DC-CONS-16; **N-G strengthened**)* | `ade_ledger::block_body_hash` | 1 function | The **only** function in the workspace that computes the Cardano block body-hash recipe. **N-G strengthening**: the header sub-slice consumed by this recipe is now also lifted as the public `accepted_block_header_bytes` accessor — three consumers (validator, producer, served-chain adapter) all hash / project through the same single site. `DC-CONS-16.strengthened_in += PHASE4-N-G`. |
| `AcceptedBlock` token *(N-C-S5 — CN-CONS-07; **N-G strengthened across the network seam**)* | `ade_ledger::producer::self_accept` | 1 newtype `{ bytes: Vec<u8> }` (private field) | Carried. **N-G strengthening**: gates **both** broadcast queue admission (carried) **and** served-chain admission. `served_chain_admit` accepts only `AcceptedBlock`. End-to-end: a block whose validator-side decision would reject can neither broadcast nor be served. `CN-CONS-07.strengthened_in += PHASE4-N-G`. |
| `self_accept` chokepoint *(N-C-S5 — CN-CONS-07)* | `ade_ledger::producer::self_accept` | 1 function | Carried. |
| `SelfAcceptError` *(N-C-S5)* | `ade_ledger::producer::self_accept` | 1 variant — `Rejected(BlockValidityError)` | Carried. |
| `SchedulerInput` *(N-C-S6 — OP-OPS-05)* | `ade_runtime::producer::scheduler` | 2 variants | Carried. |
| `SchedulerEffect` *(N-C-S6)* | `ade_runtime::producer::scheduler` | 4 variants | Carried. |
| `SchedulerHaltReason` / `SchedulerState` *(N-C-S6)* | `ade_runtime::producer::scheduler` | 2 variants / closed struct | Carried. |
| `TickInputs` / `TickAssemblyError` *(N-C-S6)* | `ade_runtime::producer::tick_assembler` | closed 13-field struct / 2 variants | Carried. |
| `assemble_tick` chokepoint *(N-C-S6)* | `ade_runtime::producer::tick_assembler` | 1 function | Carried. |
| `BroadcastError` *(N-C-S6)* | `ade_runtime::producer::broadcast` | 2 variants | Carried. |
| RED signing primitives + key types *(N-C-S1 — DC-CRYPTO-03/04/05, OP-OPS-04; **N-G strengthened**)* | `ade_runtime::producer::signing::{vrf_prove, kes_sign, kes_update, VrfSigningKey, KesSecret, ColdSigningKey, SigningError}` | 3 functions + 3 closed key types + 1 closed error sum | Carried. **N-G strengthening**: `OP-OPS-04.strengthened_in += PHASE4-N-G` — the RED server response driver (`n2n_server`) MUST NOT link against `producer::signing` (defended by `ci_check_n2n_server_no_signing_dep.sh`). Private-key custody now constrains the server-role surface in addition to the producer surface. |
| RED key loader *(N-C-S1)* | `ade_runtime::producer::keys` | 3 loader functions + 1 closed error sum | Carried. |
| **`accepted_block_header_bytes` canonical accessor** *(NEW in N-G-S1 — DC-CONS-16 strengthening, DC-CONS-18; closed-grammar accessor, not a registry)* | `ade_ledger::block_validity::header_input` | 1 function | The **single canonical header/body byte splitter** in the workspace. Lifted public accessor over the existing validator body-hash recipe's `header_cbor_slice`. Defended by `ci_check_no_parallel_header_splitter.sh`. |
| **`ServerReply` (chain-sync)** *(NEW in N-G-S1 — CN-PROTO-06; closed-grammar token, not a registry)* | `ade_network::chain_sync::server` | 1 struct with private 5-variant inner enum (`RollForward`, `RollBackward`, `AwaitReply`, `IntersectFound`, `IntersectNotFound`) | The **only** server-agency chain-sync reply wrapper. No public constructor for client-agency variants (`RequestNext`, `FindIntersect`, `Done`). Exit via `into_message()` only. CN-PROTO-06 by construction. New constructor = strengthening (closed-sum extension; closed-grammar discipline). |
| **`ServerReply` (block-fetch)** *(NEW in N-G-S1 — CN-PROTO-06)* | `ade_network::block_fetch::server` | 1 struct with private 4-variant inner enum (`StartBatch`, `NoBlocks`, `Block{bytes}`, `BatchDone`) | The **only** server-agency block-fetch reply wrapper. No public constructor for client-agency variants (`RequestRange`, `ClientDone`). Exit via `into_message()` only. |
| **`HeaderProjection`** *(NEW in N-G-S3)* | `ade_network::chain_sync::server` | closed struct `{ slot, hash, block_no, header_bytes }` | The closed projection value `ServedHeaderLookup::next_after` returns. `header_bytes` is the canonical header sub-slice per `accepted_block_header_bytes`. |
| **`ServedHeaderLookup` trait** *(NEW in N-G-S3 — DC-PROTO-08; closed read-side seam, not a registry)* | `ade_network::chain_sync::server` | 1 trait with 3 methods — `next_after(cursor) -> Option<HeaderProjection>`, `intersect(points: &[Point]) -> Option<(SlotNo, Hash32)>`, `tip() -> Option<(SlotNo, Hash32, u64)>` | **Closed seam** — fixed at this cluster close. **No plug-in extension at runtime.** Today there is exactly one production impl (`ade_runtime::producer::served_chain_lookups::ServedChainLookups`). **New impls would be a deliberate registry-tracked addition**, not a runtime extension point. New trait method = strengthening (closed extension, version-gated). |
| **`ServedRangeLookup` trait** *(NEW in N-G-S4 — DC-CONS-17; closed read-side seam, not a registry)* | `ade_network::block_fetch::server` | 1 trait with 1 method — `range_bytes(from, to) -> Vec<(SlotNo, Hash32, Vec<u8>)>` | **Closed seam.** Production impl: `ServedChainLookups`. |
| **`producer_chain_sync_serve` chokepoint** *(NEW in N-G-S3 — DC-PROTO-08)* | `ade_network::chain_sync::server` | 1 function — `pub fn producer_chain_sync_serve(state, in_msg, served: &dyn ServedHeaderLookup, version) -> Result<(ProducerChainSyncServerState, ServerStep), ProducerServerError>` | The **single producer-side chain-sync server composition root**. Pure, total, deterministic. Defended by `ci_check_chain_sync_server_closure.sh`. |
| **`producer_chain_sync_advance_tip` chokepoint** *(NEW in N-G-S3)* | `ade_network::chain_sync::server` | 1 function | The **single deferred-reply emitter** for server-agency waits. |
| **`ProducerChainSyncServerState`** *(NEW in N-G-S3)* | `ade_network::chain_sync::server` | closed struct `{ state: ChainSyncState, last_announced: Option<(SlotNo, Hash32)> }` | Closed. |
| **`ProducerServerError`** *(NEW in N-G-S3)* | `ade_network::chain_sync::server` | 1 variant — `Grammar(ChainSyncError)` | Closed sum. No `#[non_exhaustive]`. No `String`-bearing variant. |
| **`ServerStep`** *(NEW in N-G-S3)* | `ade_network::chain_sync::server` | 2 variants — `Done`, `Reply(ServerReply)` | Closed sum. |
| **`producer_block_fetch_serve` chokepoint** *(NEW in N-G-S4 — DC-CONS-17)* | `ade_network::block_fetch::server` | 1 function | The **single producer-side block-fetch server composition root**. Pure, total, deterministic. Defended by `ci_check_block_fetch_server_closure.sh`. |
| **`ProducerBlockFetchServerState`** *(NEW in N-G-S4)* | `ade_network::block_fetch::server` | closed struct | Closed. |
| **`ProducerBlockFetchServerError`** *(NEW in N-G-S4)* | `ade_network::block_fetch::server` | 1 variant — `Grammar(BlockFetchError)` | Closed sum. |
| **`BlockFetchServerStep`** *(NEW in N-G-S4)* | `ade_network::block_fetch::server` | 2 variants — `Done`, `Replies(Vec<ServerReply>)` | Closed sum. |
| **`ServedChainSnapshot`** *(NEW in N-G-S2 — CN-CONS-07 strengthening)* | `ade_ledger::producer::served_chain` | closed struct `{ blocks: BTreeMap<(SlotNo, Hash32), AcceptedBlock> }` | The **single served-chain index**. BTreeMap-backed (no HashMap — DC-PROTO-07 foundation). Defended by `ci_check_served_chain_closure.sh`. |
| **`served_chain_admit` chokepoint** *(NEW in N-G-S2)* | `ade_ledger::producer::served_chain` | 1 function — `pub fn served_chain_admit(snap, AcceptedBlock) -> Result<ServedChainSnapshot, ServedChainAdmitError>` | The **only** entry path into `ServedChainSnapshot`. Derives `(slot, hash)` from the bytes via `decode_block` — no caller-asserted hash. |
| **`ServedChainAdmitError`** *(NEW in N-G-S2)* | `ade_ledger::producer::served_chain` | 2 variants — `Decode(BlockValidityError)`, `KeyByteConflict { slot, hash }` | Closed sum. No `#[non_exhaustive]`. No `String`-bearing variant. |
| **`PerPeerN2nServerState`** *(NEW in N-G-S6)* | `ade_runtime::network::n2n_server` | closed RED struct `{ chain_sync, block_fetch, chain_sync_version, block_fetch_version }` | Closed. Per-peer state independent; cross-peer coordination only via shared `&ServedChainSnapshot`. |
| **`DispatchError`** *(NEW in N-G-S6)* | `ade_runtime::network::n2n_server` | 4 variants — `ChainSyncDecode(CodecError)`, `BlockFetchDecode(CodecError)`, `ChainSync(ProducerServerError)`, `BlockFetch(ProducerBlockFetchServerError)` | Closed RED dispatch-error sum. |
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
| `block_validity` chokepoint *(B1)* | `ade_ledger::block_validity::transition` | 1 function | Single chokepoint `self_accept` (N-C-S5) wraps. |
| `TxValidityVerdict` / `TxRejectClass` / `TxValidityError` / `SignerSource` / `WitnessClosureError` etc. *(B2)* | `ade_ledger::tx_validity::*` | closed | |
| `AdmitOutcome` / `MempoolState` / `OrderPolicy` *(B2)* | `ade_ledger::mempool::*` | closed | |
| `LeaderScheduleAnswer` *(N-B; consumed unchanged by N-C)* | `ade_core::consensus::leader_schedule` | closed struct | Shared between validator and producer (NC-VRF-3 — single source of leader truth). |
| `is_leader_for_vrf_output` *(N-B; consumed unchanged by N-C)* | `ade_core::consensus::leader_schedule` | 1 function | |
| `PraosNonces` / `NonceScanError` *(B1)* | `ade_ledger::consensus_input_extract` | | |
| `PraosChainDepState` / `ChainEvent` canonical encodings *(N-B)* | `ade_core::consensus::encoding` | 4 chokepoints | |
| `LedgerFingerprint` fold *(B3/B5)* | `ade_ledger::fingerprint` | | |
| **CI check set** | `ci/ci_check_*.sh` | **47 scripts (40 → 47 in PHASE4-N-G)** | Existing checks may be tightened, never relaxed. |
| **Invariant registry families** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-G added 6 rules** (`DC-CONS-17`, `DC-CONS-18`, `DC-PROTO-07`, `DC-PROTO-08`, `CN-PROTO-06`, `RO-LIVE-01`); strengthened `CN-CONS-07`, `DC-PROTO-06`, `OP-OPS-04`, `DC-CONS-16`, `T-DET-01`, `T-ENC-01`. Total: **196 entries** (190 → 196). | Append-only IDs. |

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
| `RollbackSnapshot` ring *(N-B)* | `ade_runtime::consensus::chain_selector::OrchestratorState::recent_snapshots` | Bounded ≤ 2160. |
| **`ServedChainSnapshot.blocks` admitted set** *(NEW in N-G-S2 — runtime-extensible **content**, but extension via the **closed** `served_chain_admit` chokepoint only)* | `ade_ledger::producer::served_chain::ServedChainSnapshot` | `BTreeMap<(SlotNo, Hash32), AcceptedBlock>`. Shape closed; instance set open. The set grows monotonically during a session; OQ-5 lock — no eviction within this cluster. |
| **`PerPeerN2nServerState` instance set** *(NEW in N-G-S6 — runtime-extensible per session, but each instance is itself a closed struct)* | `ade_runtime::network::n2n_server` | One instance per connected peer. The orchestrator (one layer up, in `ade_network::session::*`) constructs / drops instances as peers connect / disconnect. Per-peer state independent. |
| Oracle reference snapshots / regression corpus | `ade_testkit::harness::*` | Tooling-only. |
| Network corpus / Consensus corpus / Block-validity corpus / Tx-validity corpus / Mempool ingress corpus / PP canonical corpus | various | Tooling-only. |
| Producer replay corpus *(N-C-S3/S4)* | `crates/ade_testkit/fixtures/producer/` + `ade_testkit::producer::{fixtures, replay, reference_vectors}` | Tooling-only. GREEN. Signed-artifact-only by DC-CONS-14. |
| Producer mechanical cross-impl corpus *(N-C-S7)* | `ade_testkit::producer::cross_impl_adapter` | Tooling-only. GREEN. |
| **Server-paths session-transcript replay corpus** *(NEW in N-G-S5/S7 — tooling-only)* | `crates/ade_runtime/tests/server_paths_transcript_replay.rs`, `crates/ade_runtime/tests/n2n_server_two_peer_determinism.rs`, `crates/ade_runtime/tests/cross_impl_server_pipeline.rs` | Tooling-only. GREEN. Drives `(version, peer_message_sequence, broadcast_arrival_sequence, session_event_sequence)` tuples through the full pipeline; outgoing frame sequences must be byte-identical (DC-PROTO-07). Append-only by convention. Defended by `ci_check_server_paths_corpus_present.sh`. |
| **Operator-action probe binaries** *(N-B + N-E S6 + N-C S7 + **N-G S7**)* | `ade_core_interop::bin::{live_consensus_session, live_tx_submission_session, live_block_production_session, live_block_fetch_session}` | RED operator-action; `#[ignore]`-gated by closure-gate tests. **N-G added `live_block_fetch_session`** — status `blocked_until_operator_peer_available`. |
| `KillStrategy<D>` trait impls | `ade_runtime::chaindb::crash_safety` | RED-only test infrastructure. |
| Recovery state types | callers of `Recoverable` | Open: any state with canonical encode + apply-block step. |
| Pinned external crates | `crates/*/Cargo.toml` | Tier-5 rationale doc required. |

### Candidates — extensible surfaces not yet wired

| Cluster | Candidate registry | Rationale |
|---------|-------------------|-----------|
| **CE-N-G-8 (operator-action live evidence — `blocked_until_operator_peer_available`)** | **Live N2N block-fetch acceptance log (Ade serving, cardano-node consuming)** | The live cross-impl claim. Requires a private cardano-node peer. Re-opens on operator availability. |
| **CE-N-C-8 (operator-action live evidence — `blocked_until_operator_stake_available`)** | **Live N2N block-fetch acceptance log (Ade forging, cardano-node consuming as next chain head)** | Carried. |
| **Receive-side header→body bridge** *(NEW candidate flagged by N-G close)* | **`ade_node` composition layer joining `process_stream_input` + N-A block-fetch client + `block_validity`** | The natural counterpart to N-G's send-direction closure. Surface; do not invent invariants here. |
| **N-D-bridge / served-chain-index cluster (carried from N-G OQ-1 lock)** | **Broader served-chain population from a co-located N-D ChainDB** | N-G's narrow scope serves only blocks admitted from this session's `AcceptedBlock` path. Reading from a persisted ChainDB is reserved. |
| **N-G+ Tier-5** | **Operator-tunable server policy** (per-peer back-pressure, peer connection limits, served-chain memory bound) | Tier-5 — operator-tunable. Declared OUT-OF-SCOPE in N-G cluster doc. |
| **N-C+ Tier-5** | **Operator-tunable producer policy** | Carried. |
| **CE-NODE-N2C-LTX (cross-cluster obligation carried from N-E)** | **Live N2C UDS server + N2N bulk-tx inbound listener** | Carried. |
| **PP OQ-1..OQ-4** | various | Carried. |
| N-A (deferred) | Peer address book | Operator-supplied; runtime mutable. |
| N-F | Query API method set | Tier 5 wire / Tier 1 semantics. |
| N-F | Prometheus metric names | Tier 5; append-only registry expected. |

### Closed-grammar audit (PHASE4-N-G full close)

This sweep was performed after PHASE4-N-G full close (S1..S7).

1. **`accepted_block_header_bytes` canonical accessor** — **closed by
   intent and CI-defended.** Single canonical header/body byte
   splitter in the workspace; three consumers (validator, producer,
   served-chain adapter). Defended by
   `ci_check_no_parallel_header_splitter.sh`.
2. **`ServerReply` (chain-sync) closed wrapper** — **closed by intent
   and CI-defended.** Private 5-variant inner enum; no public
   constructor for client variants; exit via `into_message()` only.
   Defended by `ci_check_chain_sync_server_closure.sh` + compile-time
   match exhaustiveness in tests.
3. **`ServerReply` (block-fetch) closed wrapper** — **closed by intent
   and CI-defended.** Private 4-variant inner enum. Defended by
   `ci_check_block_fetch_server_closure.sh`.
4. **`ServedHeaderLookup` + `ServedRangeLookup` closed trait seams** —
   **closed by intent.** Fixed at this cluster close. No plug-in
   extension at runtime. One production impl
   (`ServedChainLookups`); new impls = deliberate registry-tracked
   addition.
5. **`ServedChainSnapshot` + `served_chain_admit` chokepoint** —
   **closed by intent and CI-defended.** BTreeMap-backed (no
   HashMap); only entry path is via `AcceptedBlock`; closed
   `ServedChainAdmitError` sum. Defended by
   `ci_check_served_chain_closure.sh`.
6. **`producer_chain_sync_serve` + `producer_chain_sync_advance_tip`
   chokepoints** — **closed by intent and CI-defended.** Pure, total,
   deterministic. Closed `ProducerServerError` (1-variant) + `ServerStep`
   (2-variant) sums. Defended by `ci_check_chain_sync_server_closure.sh`.
7. **`producer_block_fetch_serve` chokepoint** — **closed by intent
   and CI-defended.** Closed `ProducerBlockFetchServerError`
   (1-variant) + `BlockFetchServerStep` (2-variant) sums. Defended
   by `ci_check_block_fetch_server_closure.sh`.
8. **GREEN `drain_and_admit` adapter** — **closed by intent and
   CI-defended.** Pure; no I/O; observably deterministic. Defended
   by `ci_check_broadcast_to_served_purity.sh`.
9. **GREEN `ServedChainLookups` trait-impl bridge** — **closed by
   intent.** Single production impl of both lookup traits over
   `&ServedChainSnapshot`. Header projection via canonical
   `accepted_block_header_bytes` — no parallel splitter.
10. **RED `n2n_server` driver — key-boundary preserved.** **CI-defended.**
    Cannot import from `ade_runtime::producer::signing`. Defended by
    `ci_check_n2n_server_no_signing_dep.sh`. Closed `PerPeerN2nServerState`
    + `DispatchError` (4-variant) sums.
11. **`live_block_fetch_session` operator-action probe binary** —
    **closed by intent on the harness pattern.** Fourth instance of
    the family (after `live_consensus_session` /
    `live_tx_submission_session` / `live_block_production_session`).
    Hermetic-default-plus-`--connect`-live. Status
    `blocked_until_operator_peer_available` — second variant of the
    `blocked_*` closure mode (peer vs. stake).

**Gap note — N-G (CE-N-G-8).** The live cross-impl claim is the only
N-G obligation that depends on an external resource (a private
cardano-node peer). Per `RO-LIVE-01.open_obligation` it is
`blocked_until_operator_peer_available` — not deferred to a future
cluster, not silently accepted. Reopens when a peer is provisioned;
mechanical half (structural cross-impl pipeline) is already enforced
via `crates/ade_runtime/tests/cross_impl_server_pipeline.rs` +
`ci_check_server_paths_corpus_present.sh`.

### Closed-grammar audit (carried — PHASE4-N-C / PROPOSAL-PROCEDURES-DECODE / PHASE4-N-E / B3 / B4 / B5)

All carried unchanged from prior revision.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **Cardano-canonical CBOR wire format**.
- **Block envelope shape**: `[era_tag:u8, era_block:CBOR]`; era tags 0..=7.
- **`PreservedCbor<T>` invariant**.
- **Hash algorithms**: Blake2b-224 / 256, Ed25519, Byron-bootstrap,
  KES-sum, VRF-draft-03.
- **Era-correct block body hash** *(B1; strengthened in N-C, N-G)*:
  preserved-CBOR-segment bytes (T-ENC-01,
  `strengthened_in += PHASE4-N-G`).
- **Single canonical body-hash authority** *(N-C-S4 — DC-CONS-16;
  strengthened in N-G)*:
  `ade_ledger::block_body_hash::block_body_hash_from_buckets` is the
  **only** function computing the recipe. **N-G strengthening**: the
  header sub-slice consumed by this recipe is now also lifted as
  `accepted_block_header_bytes` — three consumers (validator,
  producer, served-chain adapter) all hash / project through the
  same single site. `DC-CONS-16.strengthened_in += PHASE4-N-G`.
- **Single canonical header/body splitter** *(NEW in N-G-S1 —
  DC-CONS-18)*: `accepted_block_header_bytes` is the **only** public
  header-bytes accessor in the workspace. Defended by
  `ci_check_no_parallel_header_splitter.sh`.
- **Server-agency closure for outgoing mini-protocol messages**
  *(NEW in N-G-S1 — CN-PROTO-06)*: the `ServerReply<M>` wrappers in
  `ade_network::chain_sync::server` and `ade_network::block_fetch::server`
  have private inner enums; no public constructor exists for
  client-agency variants. The producer-side server pump cannot
  construct or emit a client-originated message. Compile-time
  enforcement; the public API surface IS the closure proof.
- **Served-bytes parity** *(NEW in N-G-S4 — DC-CONS-17)*: every
  `Block { bytes }` payload the producer-side block-fetch server
  emits is byte-identical to the `AcceptedBlock.as_bytes()` of the
  AcceptedBlock admitted at that `(slot, hash)` key. The server never
  re-encodes. Bytes flow `AcceptedBlock` →
  `ServedChainSnapshot.blocks` → `ServedRangeLookup::range_bytes` →
  `ServerReply::block(bytes)` → `into_message()` → codec.
- **Header-body wire coherence** *(NEW in N-G-S5 — DC-CONS-18
  enforcement)*: for every `RollForward { header, tip }` the server
  emits, `block_body_hash(served_body_segment)` equals the body-hash
  field inside `header` — verified by replay over the captured
  session transcripts.
- **Producer-side server-role transcript determinism** *(NEW in
  N-G-S5 — DC-PROTO-07)*: given canonical inputs
  `(negotiated_version, peer_message_sequence,
  broadcast_arrival_sequence, session_event_sequence)`, the
  producer-side chain-sync / block-fetch session orchestrator emits a
  byte-identical sequence of outgoing mini-protocol frames across
  replays. The per-session reducer is a pure deterministic
  transition. `T-DET-01.strengthened_in += PHASE4-N-G`.
- **Deterministic-resolution discipline for server-agency waits**
  *(NEW in N-G-S3 — DC-PROTO-08)*: once chain-sync enters a state
  where the server holds agency, the reducer must return exactly one
  of a legal `RollForward`, `RollBackward`, `AwaitReply`, or a
  structured deterministic session-close/error. No ambiguous wait
  state unless the wait condition is an explicit replay input.
- **Type-level broadcast and serve gate** *(N-C-S5 — CN-CONS-07;
  strengthened in N-G across the network seam)*: `AcceptedBlock` is
  the **only** token admitted into both `BroadcastQueue` (N-C) and
  `ServedChainSnapshot` (N-G). End-to-end: a block whose
  validator-side decision would reject can neither broadcast nor be
  served. `CN-CONS-07.strengthened_in += PHASE4-N-G`.
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
  by N-C `self_accept` and N-G `served_chain_admit`)*.
- **`VerdictSurface` CBOR encoding** *(B1)*.
- **`LedgerView` trait shape** *(N-B; B1-refined)*.
- **`tx_validity` composition contract** *(B2)*.
- **`SignerSource` enumeration** *(B2)*.
- **Witness-closure contract** *(B2)*.
- **`TxVerdictSurface` CBOR encoding** *(B2)*.
- **Mempool admission contract** *(B2)*.
- **`mempool_ingress` chokepoint contract** *(N-E)*.
- **`IngressSource` source-invariance contract** *(N-E)*.
- **Verbatim tx-bytes flow through ingress** *(N-E)*.
- **GREEN single-step replay fold contract** *(N-E — DC-MEM-04)*.
- **Cross-cluster obligation pattern** *(N-E)*.
- **Operator-action evidence pattern** *(N-B / N-E / N-C / **N-G**)*:
  N-G adds the **fourth instance** (`live_block_fetch_session`) and
  the **second `blocked_*` closure-mode variant**
  (`blocked_until_operator_peer_available`) alongside N-C's
  `blocked_until_operator_stake_available`.
- **Closed credential discriminant contract** *(OQ5 / COMMITTEE /
  DREP / ENACTMENT / PP)*.
- **Committee-enactment write-back contract** *(ENACTMENT)*.
- **All canonical types**: shapes frozen at the era / version they
  entered.
- **Handshake-negotiated version threading** *(N-A; strengthened in
  N-G — DC-PROTO-06)*: every server reducer call from the
  orchestrator carries the version returned by the handshake; never
  reads it from a session global. `DC-PROTO-06.strengthened_in +=
  PHASE4-N-G`.
- **Per-session-state independence across peers** *(NEW in N-G-S6)*:
  the RED orchestrator constructs an independent `PerPeerN2nServerState`
  per peer; cross-peer coordination is only via the read-only
  `&ServedChainSnapshot`. Multi-peer determinism (two-peer test)
  confirms per-session transcripts are invariant under interleaving.
- **Key-boundary for server response paths** *(NEW in N-G-S6 —
  CN-PROTO-06 / OP-OPS-04 strengthening)*: the RED `n2n_server` module
  has no path to `ade_runtime::producer::signing`. Server response
  paths read served bytes; they do not sign anything. Defended by
  `ci_check_n2n_server_no_signing_dep.sh`.
- **TCB color assignments**: per `.idd-config.json` `core_paths`.
  **N-G additions:** `ade_network::chain_sync::server` and
  `ade_network::block_fetch::server` are BLUE (under the already-BLUE
  `ade_network` submodule prefix); `ade_ledger::producer::served_chain`
  is BLUE; `ade_runtime::producer::broadcast_to_served` and
  `ade_runtime::producer::served_chain_lookups` are GREEN-inside-RED-crate;
  `ade_runtime::network::n2n_server` is RED;
  `ade_core_interop::bin::live_block_fetch_session` is RED.
- **`ChainDb` / `SnapshotStore` / `Recoverable` trait shapes** (N-D).
- **`AcceptedBlock` type-level broadcast gate** *(N-C-S5; strengthened
  in N-G — see above)*.
- **`forge_block` pure-transition contract** *(N-C-S3 — DC-CONS-13)*: carried.
- **Single source of leader truth** *(N-C-S3 — DC-CONS-15)*: carried.
- **Tx-admissibility prefix property** *(N-C-S3 — DC-LEDGER-12)*: carried.
- **Private-key custody RED-confinement** *(N-C-S1; strengthened in
  N-G to constrain `n2n_server` too — OP-OPS-04
  `strengthened_in += PHASE4-N-G`)*.
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
- **New `ServerStep` variant** *(N-G extension point — DC-PROTO-08
  strengthening)*: closed sum; today 2 variants (`Done`, `Reply`);
  broadening (e.g. a third reply-shape) is version-gated and must
  update `ci_check_chain_sync_server_closure.sh` guards.
- **New `BlockFetchServerStep` variant** *(N-G extension point —
  DC-CONS-17 strengthening)*: closed sum; today 2 variants (`Done`,
  `Replies`); same rules.
- **New `ServerReply` constructor** *(N-G extension point —
  CN-PROTO-06 strengthening)*: closed inner enum; new server-agency
  variant requires both a new constructor + a new arm in
  `into_message()` + a new match arm in the wire `ChainSyncMessage` /
  `BlockFetchMessage` enum. Version-gated.
- **New `ServedHeaderLookup` / `ServedRangeLookup` trait method**
  *(N-G extension point)*: closed read-side seam; new methods are
  closed-trait extensions and require an updated production impl
  (`ServedChainLookups`) plus extended reducer logic. Version-gated.
- **New `ServedHeaderLookup` / `ServedRangeLookup` impl** *(N-G
  extension point — deliberate registry-tracked addition)*: the
  traits are closed seams, but a future cluster MAY register a second
  impl (e.g. an N-D-bridge impl reading from a persisted ChainDB).
  Such an addition is **deliberate** — a registry-tracked closed
  extension, not a runtime plug-in.
- **New `ServedChainAdmitError` variant** *(N-G extension point —
  CN-CONS-07 strengthening)*: closed sum; today 2 variants
  (`Decode`, `KeyByteConflict`); broadening requires updated
  `ci_check_served_chain_closure.sh`.
- **New `DispatchError` variant** *(N-G extension point)*: closed sum;
  today 4 variants.
- **New CI check**: additive. (N-G added seven —
  `ci_check_no_parallel_header_splitter.sh`,
  `ci_check_served_chain_closure.sh`,
  `ci_check_chain_sync_server_closure.sh`,
  `ci_check_block_fetch_server_closure.sh`,
  `ci_check_broadcast_to_served_purity.sh`,
  `ci_check_n2n_server_no_signing_dep.sh`,
  `ci_check_server_paths_corpus_present.sh`.)
- **Pinned external crate bump**: Tier-5 rationale doc required.
- **New mini-protocol** / **Mini-protocol version-table bump**.
- **New `ChainEvent` / `ChainSelectionReject` / `StreamInput` variant**.
- **New `NetworkMagic`** *(N-B)*.
- **New `LedgerView` impl / LedgerState-backed `PoolDistrView` constructor**.
- **`BootstrapAnchorHash` preimage v2** *(N-B)*: hard version-gated.
- **N2N/N2C tx-submission → `mempool_ingress` ingress** *(N-E)*.
- **Live cardano-node N2N block-fetch acceptance** *(N-C —
  `blocked_until_operator_stake_available`; **N-G** —
  `blocked_until_operator_peer_available`)*: both reopen on operator
  availability.
- **Phase-4 cluster surface additions** (N-F): each cluster's wire
  surface gates additions via its own cluster doc.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-G added
three new BLUE submodules** (`ade_network::chain_sync::server`,
`ade_network::block_fetch::server`, `ade_ledger::producer::served_chain`),
**one new BLUE public accessor** (`accepted_block_header_bytes` in
`ade_ledger::block_validity::header_input`), **two new GREEN submodules
inside `ade_runtime`** (`producer::broadcast_to_served`,
`producer::served_chain_lookups`), **one new RED submodule inside
`ade_runtime`** (`network::n2n_server`), **one new operator-action
probe binary** (`ade_core_interop::bin::live_block_fetch_session`),
**seven new CI gates**, **six new registry rules**, and **strengthened
four carried rules** (`CN-CONS-07`, `DC-PROTO-06`, `OP-OPS-04`,
`DC-CONS-16`) plus **two universal rules** (`T-DET-01`, `T-ENC-01`).
N-G added **no new crate**, **no new external ingress wire-format
frozen contract beyond the closed `ServerReply` wrappers** (the
underlying `ChainSyncMessage` / `BlockFetchMessage` enums were
already frozen in PHASE4-N-A), **no new public composer outside the
producer-side server authority surface**.

**N-G also added two new cross-color dependency edges**:

1. `ade_runtime → ade_network` (RED → BLUE) — the `n2n_server` driver
   imports the BLUE server reducers from `chain_sync::server` and
   `block_fetch::server`. Passes `ci_check_dependency_boundary.sh`.
2. `ade_runtime → ade_ledger` (already added in N-C; **strengthened
   in N-G**) — the GREEN `broadcast_to_served` adapter imports
   `served_chain_admit` and `ServedChainSnapshot`; the GREEN
   `served_chain_lookups` adapter imports `accepted_block_header_bytes`.
   Same direction (RED/GREEN → BLUE); allowed.

**The orphan-rule placement decision for the GREEN trait-impl
bridge** (recorded in `docs/clusters/completed/PHASE4-N-G/N-G-S5.md`):
`ServedChainLookups` impls live in `ade_runtime` because the orphan
rule prevents them from living in either `ade_ledger` or
`ade_network` alone (the impl spans both crates' types).

**The module-addition rule N-G sets for future producer-side
server-domain work:**

1. **A new producer-side BLUE server reducer attaches inside
   `ade_network::<protocol>::server`** (sibling of `chain_sync::server`
   and `block_fetch::server`). The module MUST be BLUE: no clock, no
   rand, no I/O, no `HashMap`, no `tokio`, no `async`. The reducer
   MUST consume a closed read-side trait seam (no direct dependency
   on `ade_ledger::producer` types — orphan-rule preserved). The
   reducer MUST return a closed `ServerReply<M>` wrapper, never a
   raw wire message type.
2. **A new closed read-side trait seam attaches inside the same BLUE
   server module.** Methods are closed; new methods = strengthening.
   New impls = deliberate registry-tracked addition (not runtime
   plug-in).
3. **A new closed `ServerReply<M>` constructor attaches inside the
   `ServerReply` impl block** for that protocol's wrapper. New
   constructor = closed-sum extension; no `#[non_exhaustive]`;
   version-gated.
4. **A new served-chain index attaches inside
   `ade_ledger::producer`** (sibling of `served_chain`). The module
   MUST be BLUE: BTreeMap-backed (no HashMap — DC-PROTO-07);
   entry-path single chokepoint; closed admit-error sum.
5. **A new GREEN bridge / lookup-trait-impl attaches inside
   `ade_runtime::producer`** (sibling of `broadcast_to_served`,
   `served_chain_lookups`). The module MUST be a pure function over
   its inputs; MUST NOT invoke signing primitives; MUST NOT read I/O;
   MUST produce byte-identical outputs across replays.
6. **A new RED per-peer session driver attaches inside
   `ade_runtime::network`** (sibling of `n2n_server`). The module
   MAY use clocks / async / `tokio` (in the layer above —
   the dispatch functions themselves are pure). The module MUST NOT
   import from `ade_runtime::producer::signing` — defended by
   `ci_check_n2n_server_no_signing_dep.sh`-style gates.
7. **A new server-paths registry rule attaches as a derived `DC-*` /
   `CN-*` family entry** with `code_locus`, `ci_script`, `tests`,
   `cross_ref`. Bidirectional cross-refs to consumed rules.
8. **A new operator-action probe binary attaches inside
   `crates/ade_core_interop/src/bin/`** following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape. The binary MUST stub its live socket halt when an external
   dependency is unavailable; capture status via
   `blocked_until_operator_peer_available` (peer-blocker variant —
   N-G precedent) or `blocked_until_operator_stake_available`
   (stake-blocker variant — N-C precedent) as appropriate.

### Cross-cluster obligation pattern (carried — strengthened in N-G close)

**N-G strengthens the cross-cluster obligation pattern with a
**second** variant of the `blocked_*` closure mode**:
`blocked_until_operator_peer_available`. The mode applies when an
obligation's blocker is a peer / external service the operator must
provision — distinct from N-C's `blocked_until_operator_stake_available`
(stake-registration blocker). Both variants follow OP-OPS-04's
precedent (`enforced` core + structured `open_obligation` naming the
blocker and the re-open criteria). The mechanical half MUST be closed
on the same HEAD (e.g. N-G's `cross_impl_server_pipeline` integration
test closes the bytes-shape claim before the live half ships).
**Re-opens on operator availability** — the procedure doc names the
specific blocker and the re-open criteria.

### Operator-action evidence pattern (carried — strengthened in N-G close)

N-G adds the **fourth instance** of the operator-action probe binary
family: `live_block_fetch_session`. The pattern is now established
across four Tier-1 wire-level seams (chain-sync follow,
tx-submission2 outbound, block forge, block-fetch serve) — each with
a hermetic default that runs in CI without network access, a
`--connect <peer>` live pass that the operator runs against a real
cardano-node peer, and a captured evidence log committed alongside
the procedure doc. **N-G introduces the second `blocked_*` variant**
(`blocked_until_operator_peer_available`) into the pattern's frozen
rules.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-G:** `chain_sync/server.rs` + `block_fetch/server.rs` production code has no `HashMap`/`HashSet`/`tokio`/`rand`/wall-clock (CI-defended); `ServerReply` inner enums are private; `pub fn` cannot return raw wire message types. `served_chain.rs` has no `HashMap`/`HashSet` (BTreeMap-only). Single canonical header/body splitter (`accepted_block_header_bytes`). | Other BLUE crates / submodules only. **N-G:** server reducers consume BLUE state via closed trait seams (`ServedHeaderLookup`, `ServedRangeLookup`) — no direct dep on `ade_ledger::producer` from `ade_network`. | Any RED submodule or crate; GREEN in non-dev deps; `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. **N-G:** no `*SigningKey` / `KesSecret` / `ColdSigningKey` types (carried). |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-G:** `broadcast_to_served::drain_and_admit` is a pure function over its inputs — no I/O, no clocks. `served_chain_lookups::ServedChainLookups` is the single production impl of `ServedHeaderLookup` + `ServedRangeLookup`; header projection delegates to `accepted_block_header_bytes` (no parallel splitter). | BLUE crates + standard library + ecosystem crates. **N-G:** the GREEN adapters live inside `ade_runtime` (RED crate) — color is per-module per the cluster TCB Color Map. | `ade_runtime` for `ade_testkit`; RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. **N-G:** `ade_runtime::network::n2n_server` is the per-peer session driver — pure state-machine dispatch; socket I/O lives one layer up; **MUST NOT import from `ade_runtime::producer::signing`** (CI-defended). | Any BLUE / GREEN crate or submodule (one-way). **N-G added the `ade_runtime → ade_network` edge** (RED → BLUE via the server reducers). | Cannot be depended on by BLUE. Server response paths additionally cannot link against `producer::signing`. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for server-paths-domain sub-modules, model the new CI
   gate on `ci_check_chain_sync_server_closure.sh` /
   `ci_check_block_fetch_server_closure.sh` /
   `ci_check_served_chain_closure.sh` shape (closure proof + no-raw-wire-return
   proof + closed-sum proof + BTreeMap-only proof).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   server-paths-domain authority rules, append `DC-PROTO-0X` /
   `DC-CONS-1X` / `CN-PROTO-0X` with bidirectional cross-ref to
   consumed rules. T-DET-01 / T-ENC-01 may receive a `strengthened_in`
   entry when the new module participates in their byte-deterministic
   / byte-authoritative properties.
7. **New operator-action probe binary:** add to
   `crates/ade_core_interop/src/bin/<name>.rs` following the
   `live_<surface>_session` naming + hermetic-default-plus-`--connect`-live
   shape; document in `<cluster>/CE-<id>_PROCEDURE.md`; capture
   evidence to `<cluster>/CE-<id>_<date>.log` OR mark
   `blocked_until_operator_peer_available` / `blocked_until_operator_stake_available`
   as appropriate.
8. **Cross-cluster obligation:** follow the binding rules from the
   N-E full-close narrative; N-G strengthens the rules with the
   second `blocked_*` variant.
9. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-G — FULLY CLOSED at this HEAD** (mechanical half +
  structural cross-impl): code + CI gates + DC-CONS-17/18 +
  DC-PROTO-07/08 + CN-PROTO-06 + RO-LIVE-01 (partial) + 7 new CI
  scripts. CE-N-G-8 live-evidence is
  `blocked_until_operator_peer_available` per
  `RO-LIVE-01.open_obligation` — re-opens on operator availability.
- **PHASE4-N-C — FULLY CLOSED** (carried). CE-N-C-8 live-evidence is
  `blocked_until_operator_stake_available`.
- **PROPOSAL-PROCEDURES-DECODE — FULLY CLOSED** (carried).
- **PHASE4-N-E — FULLY CLOSED** (carried).
- **Future cluster — receive-side header→body bridge** *(NEW
  candidate flagged by N-G close — original sketch §"Open questions"
  and the N-C handoff option B)*: `ade_node` composition layer
  joining `process_stream_input` (N-B) + N-A block-fetch **client**
  (existing) + `block_validity` (B1). The natural counterpart to
  N-G's send-direction closure. Surface for the next cluster
  planner; do not invent invariants here.
- **Future cluster — `CE-N-G-8` live evidence re-open trigger**:
  reopens when a private cardano-node peer is provisioned; the
  procedure is documented at
  `docs/clusters/completed/PHASE4-N-G/CE-N-G-8_PROCEDURE.md`.
- **Future cluster — `CE-N-C-8` live evidence re-open trigger**:
  reopens when testnet SPO stake is provisioned (carried from N-C).
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
- **(N-G-S1 specific — single canonical header/body splitter +
  closed `ServerReply<M>` wrappers)** No new `pub fn .*header_bytes`,
  `pub fn .*split_header`, or `pub fn .*split_block_envelope` outside
  `crates/ade_ledger/src/block_validity/header_input.rs`. The
  canonical accessor `accepted_block_header_bytes` MUST exist at that
  site. `ServerReply` inner enum MUST be private; no public
  constructor for client-agency variants (`RequestNext`,
  `FindIntersect`, `Done` for chain-sync; `RequestRange`, `ClientDone`
  for block-fetch). The only exit is `into_message()`. Defended by
  `ci_check_no_parallel_header_splitter.sh`.
- **(N-G-S2 specific — closed BLUE served-chain index `ServedChainSnapshot`)**
  No `HashMap` / `HashSet` in `ade_ledger::producer::served_chain`.
  No second entry path into `ServedChainSnapshot` — `served_chain_admit`
  is the only constructor reachable from outside the module. No
  caller-asserted `(slot, hash)` key — the admit chokepoint derives
  the key from the bytes via `decode_block`. No `String`-bearing
  variant on `ServedChainAdmitError`. No `#[non_exhaustive]` on the
  closed sums or the canonical index struct. Defended by
  `ci_check_served_chain_closure.sh`.
- **(N-G-S3 specific — closed BLUE chain-sync server reducers)** No
  `HashMap`/`HashSet`/`tokio`/`rand`/`std::time` in
  `chain_sync/server.rs` production code. No `pub fn` returning raw
  `ChainSyncMessage` — outgoing replies MUST go through
  `ServerReply::into_message()`. No `pub fn` returning
  `Result<.., ChainSyncMessage>`. Positive presence:
  `producer_chain_sync_serve`, `producer_chain_sync_advance_tip`,
  `ServedHeaderLookup` MUST exist. No ambiguous `Ok((MustReply,
  silent-wait))` without an explicit replay-input wait condition
  (DC-PROTO-08). Defended by `ci_check_chain_sync_server_closure.sh`.
- **(N-G-S4 specific — closed BLUE block-fetch server reducer)** Same
  shape for `block_fetch/server.rs`. No `pub fn` returning raw
  `BlockFetchMessage`. No construction of `ServerReply::block(bytes)`
  whose `bytes` is anything other than a `ServedRangeLookup` output
  (the reducer is the enforcement point — DC-CONS-17). Defended by
  `ci_check_block_fetch_server_closure.sh`.

### GREEN (`ade_testkit` incl. `producer` corpus; `ade_runtime::consensus::{candidate_fragment, chain_selector}`; `ade_ledger::mempool::{policy, canonicalize}`; the two `ade_core_interop` N-E bridges; `ade_runtime::producer::tick_assembler` (N-C-S6); **`ade_runtime::producer::{broadcast_to_served, served_chain_lookups}` — NEW in N-G-S5**)

- No nondeterminism that leaks into stored fixtures — fixtures must
  be byte-reproducible.
- No participation in authoritative outputs.
- No `HashMap` even in test helpers — `BTreeMap` only.
- No import of `ade_runtime` from `ade_testkit`.
- (carried bullets per prior revision)
- **(`ade_runtime::producer::broadcast_to_served`, NEW in N-G-S5 —
  DC-PROTO-07)** No I/O; no clocks; no nondeterminism. The function
  MUST be observably deterministic: identical
  `(ServedChainSnapshot, BroadcastQueue)` MUST produce byte-identical
  outputs across replays. Defended by
  `ci_check_broadcast_to_served_purity.sh`.
- **(`ade_runtime::producer::served_chain_lookups`, NEW in N-G-S5 —
  DC-CONS-16 strengthening)** The single production impl of
  `ServedHeaderLookup` + `ServedRangeLookup`. Header projection MUST
  delegate to `accepted_block_header_bytes` — no parallel splitter.
  Pure; no I/O.

### RED (`ade_runtime`, `ade_node`, `ade_network::mux::transport`, `ade_network::session`, `ade_network::bin::capture_*`, `ade_runtime::consensus::genesis_parser`, `ade_core_interop` (incl. N-C S7 probe binary `live_block_production_session` and **N-G S7 probe binary `live_block_fetch_session` — NEW**), and the RED-behavior `ade_ledger::consensus_input_extract` scan; `ade_runtime::producer::{signing, keys, scheduler, broadcast}` (N-C-S1/S6); **`ade_runtime::network::n2n_server` — NEW in N-G-S6**)

- No direct mutation of `ade_ledger` state — all transitions go
  through `ade_ledger::rules::*`, the `block_validity` / `tx_validity`
  composers, `mempool::ingress::mempool_ingress`, **the producer
  authority chokepoints `producer::forge::forge_block` +
  `producer::self_accept::self_accept`** (N-C), or **the served-chain
  authority chokepoint `producer::served_chain::served_chain_admit`**
  (N-G).
- No bypassing `ade_codec` to construct semantic types from raw bytes.
  **(N-C-strengthened)** Constructing `AcceptedBlock` outside
  `self_accept` is CI-forbidden. **(N-G-strengthened)** Constructing
  `ServedChainSnapshot` populated entries outside `served_chain_admit`
  is CI-forbidden; constructing `ServerReply` variants for
  client-agency wire messages is unrepresentable in the public API
  (CN-PROTO-06).
- (`ade_runtime` specifically) Existing `ade_runtime → ade_ledger`
  edge (added N-C) is now also consumed by N-G's
  `broadcast_to_served` (via `served_chain_admit`) and
  `served_chain_lookups` (via `accepted_block_header_bytes`).
  **NEW N-G edge: `ade_runtime → ade_network`** (RED/GREEN → BLUE via
  the server reducers + ServerReply wrappers). Both pass
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
  `#[ignore]`-gated. **N-G added `live_block_fetch_session`** —
  fourth operator-action probe binary. The binary's default mode
  prints readiness and exits; `--connect` performs the live pass
  against a real cardano-node peer.
- **(N-C-S1 / S6 specific — `ade_runtime::producer::{signing, keys,
  scheduler, broadcast}`)** All carried.
- **(N-G-S6 specific — `ade_runtime::network::n2n_server`)** Pure
  state-machine driver — NO socket I/O at this layer (sockets live
  one layer up in `ade_network::session::*`). MUST NOT import from
  `ade_runtime::producer::signing` — defended by
  `ci_check_n2n_server_no_signing_dep.sh`. Per-peer state independent;
  cross-peer coordination only via shared `&ServedChainSnapshot`.
  Decoded inbound frames MUST go through the BLUE reducers (no
  inline grammar). Encoded outbound frames MUST come from
  `ServerReply::into_message()` outputs only (no direct
  `encode_chain_sync_message` / `encode_block_fetch_message` calls
  on raw wire messages).
- **(N-G-S7 specific — `live_block_fetch_session`)** The live socket
  loop MUST drive the RED N2N server driver →
  `dispatch_chain_sync_frame` / `dispatch_block_fetch_frame` /
  `poll_chain_sync_advance` pipeline through the canonical
  chokepoints — no parallel server path, no direct construction of
  `ServerReply` outside the BLUE reducers, no bypass of
  `accepted_block_header_bytes`. The live evidence log committed
  alongside the procedure doc redacts hostnames per
  `feedback_no_credential_leaks`.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-G-strengthened:** no
  private-key bytes in server-paths fixture corpora (defended by
  `ci_check_no_private_keys_in_corpus.sh` extended to cover the new
  fixture root).
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces must
  be exercised against real cardano-node peers. **N-G:** the
  mechanical cross-impl harness in
  `crates/ade_runtime/tests/cross_impl_server_pipeline.rs` is a
  structural-agreement harness (every served `Block { bytes }`
  decodes via Ade's own decoder; recomputed body-hash matches the
  announced header); the live cross-impl claim requires
  operator-action live evidence per CE-N-G-8.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. The producer-side server response authority
  surface (chain-sync serve + block-fetch serve + served-chain index +
  header projection) is now Tier 1; the seven new CI gates enforce
  mechanical closure. **The N-G
  `blocked_until_operator_peer_available` status is NOT a "we'll
  match it later" stub** — the mechanical half is fully enforced at
  this HEAD; the live half is recorded as an `open_obligation` on
  `RO-LIVE-01`, tied to a specific operator-action procedure
  (`CE-N-G-8_PROCEDURE.md`), and reopens on a named external
  dependency (private cardano-node peer provisioned by the operator).
  Follows OP-OPS-04's precedent and is the second `blocked_*` variant
  in the family alongside N-C's
  `blocked_until_operator_stake_available`.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP is being regenerated in parallel; pending the regen,
  CODEMAP may pin pre-N-G HEAD. The new BLUE submodules
  (`ade_network::chain_sync::server`, `ade_network::block_fetch::server`,
  `ade_ledger::producer::served_chain`), the new BLUE public
  accessor (`accepted_block_header_bytes`), the new GREEN submodules
  (`ade_runtime::producer::{broadcast_to_served, served_chain_lookups}`),
  the new RED submodule (`ade_runtime::network::n2n_server`), and the
  new operator-action probe binary
  (`ade_core_interop::bin::live_block_fetch_session`) are not yet in
  the prior CODEMAP. The next CODEMAP regen picks these up
  mechanically. CI count moves from 40 → 47.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-G added:**
  `DC-CONS-17` (`enforced`, `ci_script =
  ci/ci_check_block_fetch_server_closure.sh`,
  `introduced_in = PHASE4-N-G`); `DC-CONS-18` (`enforced`,
  `ci_script = ci/ci_check_no_parallel_header_splitter.sh`);
  `DC-PROTO-07` (`enforced`, `ci_script =
  ci/ci_check_broadcast_to_served_purity.sh`); `DC-PROTO-08`
  (`enforced`, `ci_script = ci/ci_check_chain_sync_server_closure.sh`);
  `CN-PROTO-06` (`enforced`, `ci_script =
  ci/ci_check_chain_sync_server_closure.sh +
  ci/ci_check_block_fetch_server_closure.sh`); `RO-LIVE-01`
  (`partial` + `open_obligation =
  blocked_until_operator_peer_available`, `ci_script =
  ci/ci_check_server_paths_corpus_present.sh`); appended `PHASE4-N-G`
  to `CN-CONS-07.strengthened_in`, `DC-PROTO-06.strengthened_in`,
  `OP-OPS-04.strengthened_in`, `DC-CONS-16.strengthened_in`,
  `T-DET-01.strengthened_in`, `T-ENC-01.strengthened_in`. Total:
  190 → 196 entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-D / N-A / N-B / B1 / B2 / B3 / B4 / B5 / OQ5-CREDENTIAL-FIDELITY
  / COMMITTEE-CRED-FIDELITY / DREP-VOTE-FIDELITY /
  ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK /
  PHASE4-N-E / PROPOSAL-PROCEDURES-DECODE / PHASE4-N-C: all closed;
  cluster docs carried.
- **Cluster PHASE4-N-G (CLOSED + archived at this HEAD; mechanical
  half + structural cross-impl)**: the cluster doc + slices
  `cluster.md, N-G-S{1..7}.md` + `CE-N-G-8_PROCEDURE.md` at
  `docs/clusters/completed/PHASE4-N-G/`. WIRES AND CLOSES the
  producer-side server response paths end-to-end: BLUE header
  projection authority + closed `ServerReply<M>` wrappers (S1),
  BLUE `ServedChainSnapshot` + `served_chain_admit` (S2), BLUE
  chain-sync server reducers (S3), BLUE block-fetch server reducer
  (S4), GREEN broadcast→served adapter + transcript replay (S5),
  RED per-peer N2N server session driver + multi-peer determinism
  (S6), mechanical cross-impl pipeline + `live_block_fetch_session`
  operator-action probe binary (S7). Added seven CI scripts (count
  40 → 47); added six derived / release registry rules (total 190 →
  196); strengthened four carried rules + two universal rules.
  **CE-N-G-8 live-evidence `blocked_until_operator_peer_available`**
  per `RO-LIVE-01.open_obligation`; mechanical bytes-shape claim is
  closed by the cross-impl pipeline. Four operator-action probe
  binaries now in the family: `live_consensus_session` (N-B),
  `live_tx_submission_session` (N-E S6), `live_block_production_session`
  (N-C S7), `live_block_fetch_session` (N-G S7).
- **Future obligation: `CE-N-G-8`** — operator-action live evidence
  for live cross-impl block-fetch acceptance by a real cardano-node
  peer; reopens on private peer availability.
- **Future obligation: `CE-N-C-8`** — operator-action live evidence
  for crypto-level cross-impl block forging; reopens on testnet SPO
  stake registration availability.
- **Future obligation: `CE-NODE-N2C-LTX`** — the node-binary
  cluster's live N2C UDS server + N2N bulk-tx inbound listener;
  carried from N-E.
- **Future seam candidate (flagged by N-G close): receive-side
  header→body bridge** — `ade_node` composition layer joining
  `process_stream_input` (N-B) + N-A block-fetch **client** + B1
  `block_validity`. Surface for the next cluster planner.
