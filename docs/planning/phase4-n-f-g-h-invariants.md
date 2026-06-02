# Invariant Sketch — PHASE4-N-F-G-H: Node-spine live serve-to-peer

> **Type:** IDD invariant sketch (Part I). Planning artifact — no implementation, no
> cluster/slice breakdown yet. Predecessors: G-A forge fidelity, G-B self-accept→serve
> handoff (`DC-NODE-06`), G-C live feed + BA-02 evidence I/O, G-D bounty dry-run harness
> (`CN-REHEARSAL-FIDELITY-01`) — whose **C1 dry-run surfaced the gap this cluster closes**.
> Code-verified at HEAD `18085231`.

## 0. Framing (read first — and an honesty statement)

**The C1 dry-run found a real, bounty-relevant gap.** `--mode node` forges, self-accepts,
and pushes into a `ServedChainHandle` (G-B) — but **discards the `ServedChainView` read
side** (`node_lifecycle.rs`: `let (serve_handle, _serve_view) = ServedChainHandle::new();`,
comment: *"no live reader"*) and runs **no listener / no block-fetch / no chain-sync
server** on the node spine. So a real Haskell follower physically cannot fetch Ade's forged
block: forge-acceptance is unreachable in `--mode node`.

**G-H wires the owed node-spine serve-to-peer leg; it enables a real follower to discover
and fetch the G-B served chain, while peer acceptance still requires operator-captured
Haskell logs through `correlate`.** It does this on the `--mode node` spine — the same path
C2/preprod needs — **not** by switching to `--mode produce` (that would recreate the
private-only divergence the G-D path-fidelity fence guards against).

**This is reuse, not new authority (honesty statement).** Every serve component already
exists: the BLUE reducers `producer_chain_sync_serve` + `producer_block_fetch_serve` (pure,
`DC-PROTO-07/08`, `DC-CONS-17/18`, server-agency-closed `CN-PROTO-06`) in
`crates/ade_network/src/{chain_sync,block_fetch}/server.rs`; the GREEN `ServedChainView`
(the G-B watch read side) + the pure projections in `ade_runtime`; the RED
`run_n2n_listener` + `dispatch_server_frame_event_to_outbound` + `new_per_peer_outbound` in
`produce_mode`. `produce_mode` already drives **both** serve reducers from the view.
**G-H's only genuinely-new code is RED wiring**: on the node-spine `On` arm, *keep* the
`ServedChainView` (stop discarding it) and spawn a **sibling** listener + serve-dispatch
task reading it — mirroring the G-B sibling push task. The deterministic core (the servers)
is reused unchanged.

**Pure-transformation honesty (IDD demands it):** the authoritative serve transform —
`(served snapshot, peer message) → outgoing frames` — is already a pure, replay-equivalent
BLUE reducer (`DC-PROTO-07`). G-H adds no new authoritative transform; it adds the RED I/O
that *carries* that transform to a real peer. No new BLUE authority, no new canonical type.

**Resolved design choice (verified, not assumed): a follower needs BOTH ChainSync and
BlockFetch.** `produce_mode`'s `dispatch_server_frame_event_to_outbound` drives
`producer_chain_sync_serve` **and** `producer_block_fetch_serve` from the view — the peer
discovers the forged block's header via ChainSync `RollForward` (`DC-CONS-18`) then fetches
the body via BlockFetch (`DC-CONS-17`). BlockFetch alone is insufficient; the C1 follower
flow is the proof.

## 1. What must always be true

- **A0 — Single serve source = the G-B self-accepted chain.** The only blocks `--mode node`
  serves to a peer are the BLUE self-accepted `AcceptedBlock`s reachable through the G-B
  `ServedChainView` (fed solely by the sibling push task via `into_accepted()`,
  `DC-NODE-06`). No raw forged bytes, no failed-forge output, no second source.
- **A1 — Serve is a sibling task; containment byte-unchanged.** The listener +
  serve-dispatch run **outside** `run_relay_loop` (a sibling, like the G-B push task). The
  relay-loop body performs no serve / `push_atomic` / served-chain mutation;
  `ci_check_node_run_loop_containment.sh` stays byte-/semantically unchanged (`CN-NODE-02`).
- **A2 — Reuse the existing N2N serve machinery; single serializer.** The serve reuses
  `run_n2n_listener` + `dispatch_server_frame_event_to_outbound` + the BLUE
  `producer_chain_sync_serve` / `producer_block_fetch_serve` (+ `producer_chain_sync_advance_tip`)
  **verbatim**; the CN-WIRE-08 tag-24 envelope is the **sole** serializer (no parallel one).
  The BLUE servers' invariants (`DC-CONS-17/18`, `CN-PROTO-06`, `DC-PROTO-07/08`) are carried
  unchanged.
- **A3 — Both ChainSync and BlockFetch served.** A peer can discover the forged block's
  header (ChainSync `RollForward`, `DC-CONS-18`) **and** fetch its body (BlockFetch,
  `DC-CONS-17`) from the `ServedChainView`; the advertised header's body-hash binds the
  servable body (`DC-CONS-18` carried).
- **A4 — No new flag; S1 fence intact.** `--mode node` uses the **existing** `--listen`
  flag (already in the closed allow-list); no new `--mode node` argv flag, no from-genesis
  constructor; `ci_check_node_path_fidelity.sh` stays green.
- **A5 — Path fidelity preserved.** The serve-to-peer leg is on the `--mode node` spine (the
  C2 path), never `--mode produce`; the C1 dry-run exercises the same serve path C2 will
  (`CN-REHEARSAL-FIDELITY-01`).
- **A6 — No acceptance without correlate.** Wiring the serve produces **no** peer-acceptance
  / BA-02 claim; acceptance is proven only by a real peer log through
  `ba02_evidence::correlate` (`RO-LIVE-06`). `RO-LIVE-01/06` do **not** flip at G-H's
  implementation close (the operator-witnessed ACCEPT is the separate gating flip — mirrors
  G-A..G-C).
- **A7 — C1 must prove BOTH required traffic directions.** The C1 dry-run must demonstrate a
  topology that makes **both** flows true, because they are distinct traffic directions and
  "serve is wired" alone is insufficient:
  - **(a) upstream feed peer** — Ade receives a Continuing live feed that keeps
    `NodeBlockSource::WirePump` `Continuing` so the forge loop reaches `ForgeTick`;
  - **(b) downstream follower peer** — a Haskell follower connects to Ade's `--listen`
    address, ChainSync-discovers Ade's served header, BlockFetch-fetches Ade's block body,
    and validates/accepts or rejects.

  Whether one reciprocal Haskell peer (`Ade --peer Haskell`; the Haskell node's topology
  includes Ade) or two Haskell nodes realizes this topology is a **slice-entry proof
  obligation** (OQ2) — *do not assume one connection can satisfy both directions.*

## 2. What must never be possible

- **N0 — Serve a non-self-accepted artifact.** No bytes that didn't trace through G-B
  `self_accept`; no raw forged bytes / failed outcome / second source.
- **N1 — Relay-loop serve mutation.** No `push_atomic` / served-chain mutation / serve in
  `run_relay_loop`'s body (`CN-NODE-02` containment held byte-/semantically unchanged).
- **N2 — Second serve authority or serializer.** No second `ServedChain` authority; no
  parallel tag-24 serializer (reuse the G-B `ServedChainHandle`/`View` + CN-WIRE-08).
- **N3 — `--mode produce` switch / private-only path.** The C1 serve is on the node spine;
  the S1 path-fidelity fence stays green.
- **N4 — New `--mode node` flag / from-genesis constructor.** `--listen` is reused.
- **N5 — Acceptance overclaim.** No peer-acceptance / BA-02 claim without a real peer log
  through `correlate`; no RO-LIVE flip at impl close; no synthetic evidence.
- **N6 — New BLUE authority / canonical type / variant.** The BLUE servers are reused
  unchanged; no new `NodeBlockSource` / `CoordinatorEvent` variant.
- **N7 — Advertise-without-serve / stale-view serve.** Never advertise a header whose body
  the node can't serve from the same `AcceptedBlock` (`DC-CONS-18` binding); never serve a
  block the node didn't self-accept; never advertise a client-agency message from the
  server-role pump (`CN-PROTO-06`).

## 3. What must remain identical across executions (deterministic surface)

- **I1 — Serve reducers deterministic (reused).** `producer_chain_sync_serve` +
  `producer_block_fetch_serve` are pure/total/deterministic (`DC-PROTO-07/08`): same
  `(negotiated_version, served-snapshot sequence, peer message sequence, session events)` →
  byte-identical outgoing frames; the chain-sync server-agency reducer returns exactly one
  legal reply (`DC-PROTO-08`, no ambiguous wait).
- **I2 — Served bytes = self-accepted bytes (reused).** BlockFetch payload =
  `AcceptedBlock.as_bytes()` verbatim, never re-encoded (`DC-CONS-17`); ChainSync
  `RollForward` header = the header sub-segment via the single canonical projection
  (`DC-CONS-18`).

## 4. What must be replay-equivalent

- **R1 — Serve transcript replay (reused `DC-PROTO-07`).** Given canonical inputs
  (negotiated version, served-snapshot sequence, peer message sequence, session events),
  the serve dispatch emits a byte-identical outgoing frame sequence across replays.
- **R2 — No new authoritative state.** The serve is read-only over the `ServedChainView`:
  it advances no durable tip, mints no `AcceptedBlock` (`CN-FORGE-01` unchanged), adds no
  WAL/checkpoint/canonical type. The durable tip still advances only via
  `run_node_sync → pump_block` (`DC-SYNC-01/02` unchanged).

## 5. State transitions in scope

| # | Transition | Color | Status |
|---|---|---|---|
| T1 | `(listen_addr, shutdown) → run_n2n_listener spawns; peer connections arrive as server-frame events` | RED (reused) | reuse |
| T2 | `(server_frame_event, ServedChainView, peer_outbound) → dispatch → producer_{chain_sync,block_fetch}_serve → OutboundCommand` | RED adapter + BLUE reducers (reused) | reuse |
| T3 | `--mode node On arm: retain the ServedChainView (stop discarding) + spawn the sibling listener+serve-dispatch task reading it` | RED wiring | **NEW (the only new code)** |

The forge / self-accept / G-B push transitions and the BLUE servers are reused unchanged.

## 6. TCB color hypothesis

- **BLUE (reused, unchanged):** `producer_chain_sync_serve` / `producer_block_fetch_serve`
  (+ `producer_chain_sync_advance_tip`) in `crates/ade_network/src/{chain_sync,block_fetch}/server.rs`;
  the CN-WIRE-08 tag-24 envelope; `self_accept` / `served_chain`.
  **A BLUE change is a red flag → reject.**
- **GREEN (reused):** the `ServedChainView` + the pure served-chain projections
  (`crates/ade_runtime/src/producer/served_chain_handle.rs`, `served_chain_lookups.rs`,
  `broadcast_to_served.rs`).
- **RED (the bulk — wiring):** the node-spine sibling listener + serve-dispatch task
  (`run_n2n_listener` + the dispatch loop) on `node_lifecycle`; key custody stays RED.
- **Open color (OQ1):** `dispatch_server_frame_event_to_outbound` + `new_per_peer_outbound`
  currently live **private in RED `produce_mode`**. Reuse on the node spine requires
  *sharing* them (no second serve authority → N2) — likely extract to a shared serve module;
  the extracted adapter's TCB color (GREEN-by-content deterministic adapter vs. RED) is
  resolved at cluster-plan.

## 7. Open questions (resolve before / at cluster-plan)

- **OQ1 — Home + color of the shared serve adapter.** `dispatch_server_frame_event_to_outbound`
  + `new_per_peer_outbound` + the listener wiring are private in `produce_mode` (RED). Reuse
  them on the node spine by **extracting to a shared serve module** both modes call (never
  duplicate — N2). Resolve the extracted adapter's TCB color and home.
- **OQ2 — The C1 topology / connection direction (load-bearing; A7's resolution).** A7
  states the obligation; OQ2 is *how* it is realized. Two roles are required:
  - **upstream feed peer** → keeps `NodeBlockSource::WirePump` `Continuing` → forge reaches
    `ForgeTick`;
  - **downstream follower peer** → connects to Ade's `--listen` → ChainSync discovers Ade's
    served header → BlockFetch fetches Ade's body → validates/accepts or rejects.

  In the simplest private topology the Haskell node *might* play both roles only if the
  protocol/session wiring supports simultaneous reciprocal connections cleanly (`Ade --peer
  Haskell`; the Haskell topology includes Ade). **Do not assume one connection can satisfy
  both directions.** Whether C1 needs one reciprocal Haskell peer or two Haskell nodes is a
  **slice-entry proof obligation**. The kept `~/.cardano-private-testnet-c1` net is the
  regression harness for proving it.
- **OQ3 — Node-spine ChainSync tip-advance / serve (kept open for the plan).** The
  node-spine ChainSync server must **advertise the served tip when `ServedChainView`
  updates**, so a follower can request the block body by hash through BlockFetch. That
  likely means the shared serve adapter must include the **same ChainSync advance/serve
  behavior `produce_mode` uses (`producer_chain_sync_advance_tip`), not BlockFetch alone** —
  a follower that never sees a `RollForward` never asks for the body. Confirm what drives
  `advance_tip` on the node spine (the `ServedChainView` watch update).
- **OQ4 — Genesis/intersection consistency in C1.** Both Ade (recovered seed epoch) and the
  follower (fresh) run the same `create-testnet-data` genesis; ChainSync intersects at
  genesis. Confirm Ade's pre-seeded store + the extracted consensus-inputs bundle are
  consistent with that genesis so the follower accepts (the G-D pinning obligation, carried).
- **OQ5 — Close boundary.** G-H closes the serve **mechanism** (a hermetic loopback proving
  a peer fetches the served self-accepted block + the C1 dry-run proving a real Haskell
  follower fetches) — but `RO-LIVE-01/06` flip only at the separate operator-witnessed
  ACCEPT through `correlate`. Confirm this mechanical-vs-operator split (mirrors G-C).

## 8. Registry surface (one new rule — declared)

- **`DC-NODE-07`** *(tier `derived`; `introduced_in = PHASE4-N-F-G-H`; `status = declared`)* —
  **Node-spine live serve-to-peer.** `--mode node` serves real peers **only** from the G-B
  self-accepted `ServedChainView`, **outside** `run_relay_loop`, **through the existing
  ChainSync + BlockFetch serve reducers**, with **no second serve authority or serializer**.
  Reuses `run_n2n_listener` + `dispatch_server_frame_event_to_outbound` + the BLUE
  `producer_chain_sync_serve` / `producer_block_fetch_serve` (both the ChainSync header
  advertisement and the BlockFetch body) + the single CN-WIRE-08 tag-24 envelope; no
  relay-loop serve mutation (containment byte-unchanged); no new `--mode node` flag (reuses
  `--listen`); not switched to `--mode produce`. Wiring the serve is not a peer-acceptance
  claim — acceptance is proven only by the peer's validation log through `correlate`, and
  `RO-LIVE-01/06` do not flip at this cluster's implementation close. Carries `DC-NODE-06`,
  `CN-NODE-02`, `DC-CONS-17/18`, `CN-PROTO-06`, `DC-PROTO-07/08`, `CN-WIRE-08`, `CN-FORGE-01`;
  cross-refs `RO-LIVE-01` (the live ACCEPT it unblocks, still operator-gated) and
  `CN-REHEARSAL-FIDELITY-01` (path fidelity). `declared` at sketch → `enforced` at close
  (tests + ci_script populate at slice time).

## 9. Generation notes

- **Carried scope guards:** the serve stays subordinate + self-accept-only (the forge
  advances no durable tip; the serve is read-only over the G-B view); containment
  byte-unchanged; no overclaim (acceptance only via `correlate`); path fidelity (node spine,
  not produce) preserved.
- **The C1 net is the regression harness:** `~/.cardano-private-testnet-c1` (genesis + pool
  keys) is retained to rerun the dry-run once G-H lands — the dry-run is the live proof of
  A7 / OQ2 / OQ3.
- **Next:** `/cluster-plan PHASE4-N-F-G-H` (this one warrants a plan — a shared-adapter
  extraction + the C1 two-direction topology resolve across ≥2 slices) → `/cluster-doc` →
  `/slice-doc` → implement on a fresh branch off `main` (`18085231`).
