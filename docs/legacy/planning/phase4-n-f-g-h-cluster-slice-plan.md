# Cluster/Slice Plan — PHASE4-N-F-G-H (Node-spine live serve-to-peer)

> Single follow-on sub-cluster (like G-E/G-D) — **not** a multi-cluster split.
> Source: invariants sketch `docs/planning/phase4-n-f-g-h-invariants.md` (committed
> `c882e861`); declared rule `DC-NODE-07`. Surfaced by the G-D C1 dry-run finding:
> `--mode node` discards the `ServedChainView` read side and runs no serve server, so a
> real Haskell follower cannot fetch Ade's forged block. This cluster wires the owed
> node-spine serve-to-peer leg on the **same** `--mode node` accepted-block path C2/preprod
> needs (NOT `--mode produce`).

## Cluster Index (Dependency Order)

1. **PHASE4-N-F-G-H** — Node-spine live serve-to-peer — primary invariant: `DC-NODE-07` —
   `--mode node` serves real peers **only** the G-B self-accepted `ServedChainView`, through
   a sibling task **outside** `run_relay_loop`, reusing the **single** ChainSync+BlockFetch
   serve-dispatch authority and the **single** CN-WIRE-08 serializer — no second serve
   authority, no `--mode produce` switch, no relay-loop serve mutation.

## Key slice-entry finding (OQ3 — read before S2/S3)

`produce_mode` does **not** currently drive `producer_chain_sync_advance_tip` (zero calls;
no `served_chain_view.changed()` reactor). Its serve is **purely request-driven**:
`dispatch_server_frame_event_to_outbound` reacts only to *incoming* peer frames. The BLUE
`advance_tip` reducer + its unit tests exist in `ade_network`, but the proactive
push-on-view-update behavior is **not wired in any binary**.

Consequence, by Cardano chain-sync semantics:

- **Delivering an already-served forged block** (the follower's `RequestNext` arrives when
  the served view already contains the block): `RollForward` is returned immediately →
  `BlockFetch` gets the body. **Pure reuse — works today; no `advance_tip` needed.**
- **Proactively pushing `RollForward` when the follower is parked at tip** (it sent
  `RequestNext`, got `AwaitReply`, and Ade forges the *next* block afterward): requires a
  `ServedChainView`-update-driven `advance_tip` driver — **new behavior, wired nowhere.**

**G-H is scoped to the first (request-driven) case.** The proactive `advance_tip` driver is
**NOT** pulled into S2. It is **new behavior**, not "reuse," and must not be smuggled in
under that label. If a real C1 Haskell follower proves it is required, that becomes a
**separate new shared-path cluster** (exactly as the G-D dry-run spawned G-H) — never a
G-H slice (no carry-forward).

## PHASE4-N-F-G-H — Node-spine live serve-to-peer

- **Primary invariant:** `DC-NODE-07` (declared → enforced at close). Carries `DC-NODE-06`,
  `CN-NODE-02`, `DC-CONS-17/18`, `CN-PROTO-06`, `DC-PROTO-07/08`, `CN-WIRE-08`, `CN-FORGE-01`;
  cross-refs `RO-LIVE-01` (the live ACCEPT it unblocks, still operator-gated) +
  `CN-REHEARSAL-FIDELITY-01` (path fidelity).
- **TCB partition:**
  - **BLUE** [reused, unchanged] — `ade_network::chain_sync::server` (`producer_chain_sync_serve`),
    `ade_network::block_fetch::server` (`producer_block_fetch_serve`); CN-WIRE-08 tag-24.
    *Any BLUE change is a red flag → reject.*
  - **GREEN** [reused] — `ade_runtime::producer::served_chain_handle::ServedChainView`,
    `served_chain_lookups::ServedChainLookups`; the `ServedBlockEvidence` evidence struct
    (moves with the adapter in S1).
  - **RED** — the extracted shared serve-dispatch adapter's new home in `ade_runtime`
    (`dispatch_server_frame_event_to_outbound` + `ServerPeerStates` + `DispatchError`);
    `ade_runtime::network::n2n_listener::run_n2n_listener` (reused);
    `ade_node::node_lifecycle` (On-arm wiring); `ade_node::produce_mode` (re-pointed at the
    shared adapter).
- **Cluster Exit Criteria:**
  - **CE-G-H-1** (mechanical): a **single** serve-dispatch authority —
    `dispatch_server_frame_event_to_outbound` has exactly one definition, called by both
    `produce_mode` and `node_lifecycle`; CI gate `ci_check_single_serve_dispatch_authority.sh`
    (no parallel/duplicate serve-dispatch in `node_lifecycle`). `produce_mode`'s serve tests
    stay green (behavior byte-unchanged).
  - **CE-G-H-2** (mechanical): `--mode node` (given `--listen`) serves the G-B self-accepted
    `ServedChainView` to a peer via **both** ChainSync (`RollForward` header, `DC-CONS-18`)
    **and** BlockFetch (body, `DC-CONS-17`), through a sibling task **outside** `run_relay_loop`,
    reusing `run_n2n_listener` + the shared adapter; the containment gate
    (`ci_check_node_run_loop_containment.sh`) and the path-fidelity fence
    (`ci_check_node_path_fidelity.sh`) stay byte-/semantically unchanged; a hermetic loopback
    test proves a peer discovers + fetches an **already-served** self-accepted block via the
    request-driven path.
  - **CE-G-H-3** (operator-gated): a C1 dry-run runbook (strict adaptation of the G-C/G-D
    operator-pass runbook, anchored by the S1 path-fidelity fence) + an env-gated operator
    execution harness (`ADE_LIVE_C1_SERVE`) proving a **real Haskell follower** discovers +
    fetches Ade's served forged block; `blocked_until_operator_c1_serve_executed`; **no
    synthetic evidence; no RO-LIVE flip.**
- **Slices:**
  - **S1 Extract shared serve-dispatch authority** — invariant: one serve-dispatch
    definition, no second serve authority (the `DC-NODE-07` "no second serve authority/serializer"
    clause made mechanical). Move `dispatch_server_frame_event_to_outbound` + `ServerPeerStates`
    + `DispatchError` + `ServedBlockEvidence` from private `produce_mode` to a shared
    `ade_runtime` serve module; re-point `produce_mode`; add
    `ci_check_single_serve_dispatch_authority.sh`. Behavior-preserving for `produce_mode`. —
    addresses: CE-G-H-1 — **TCB: RED** (adapter) + GREEN (moved evidence type), no BLUE change.
  - **S2 Node-spine serve wiring + hermetic loopback** — invariant: `--mode node` exposes
    only the G-B `ServedChainView` to a peer via both protocols, outside the relay loop (the
    core `DC-NODE-07` mechanism), **request-driven only** (no proactive `advance_tip`). Stop
    discarding `_serve_view`; spawn a listener + serve-dispatch **sibling** task (gated on
    `--listen`) reading the view, reusing `run_n2n_listener` + the S1 shared adapter; join it
    alongside the existing push sibling; relay-loop body unchanged. Hermetic loopback test
    (`node_spine_serve_loopback_follower_fetches_self_accepted_block`) arranges the follower's
    ChainSync request **after** the served view already contains the self-accepted block, and
    proves request-driven `RollForward` + `BlockFetch`. — addresses: CE-G-H-2 — **TCB: RED**
    (wiring), no BLUE change.
  - **S3 C1 dry-run runbook + operator-gated serve harness** — invariant: the live serve
    mechanism is exercised against a real Haskell follower on the *same* node-spine path, with
    acceptance still proven only via `correlate`. Runbook + env-gated harness
    (`ADE_LIVE_C1_SERVE`); the A7 topology obligation (upstream feed keeps `WirePump`
    Continuing → ForgeTick; a downstream Haskell follower discovers + fetches over `--listen`).
    **Test-topology framing (not a live-run assumption):** the hermetic/C1 serve test *may
    arrange* the follower's ChainSync request to arrive after the served view already contains
    the self-accepted block, proving request-driven discovery/fetch. **If the real follower is
    already parked at tip before Ade forges and requires proactive `advance_tip`, STOP and
    scope that separately** (a new shared-path cluster — not a G-H slice). **Slice-entry proof
    obligation (OQ2):** confirm the C1 topology (one reciprocal Haskell peer vs. two Haskell
    nodes) at slice entry, not by assumption. `blocked_until_operator_c1_serve_executed`; no
    RO-LIVE flip. — addresses: CE-G-H-3 — **TCB: RED** (runbook + env-gated test), no BLUE
    change.
- **Replay obligations:** **none new.** No new authoritative state, no new canonical type, no
  new replay corpus entry. The serve is read-only over the `ServedChainView` (`CN-FORGE-01` /
  `DC-SYNC-01/02` unchanged). Carries `DC-PROTO-07` serve-transcript replay-equivalence
  (reused); S2 adds a hermetic node-spine serve loopback test (not a replay-corpus entry).
- **FC/IS partition:** BLUE = `ade_network::{chain_sync,block_fetch}::server` (reused
  unchanged). GREEN = `ade_runtime::producer::{served_chain_handle::ServedChainView,
  served_chain_lookups}` + the moved `ServedBlockEvidence`. RED = the shared serve-dispatch
  adapter's new `ade_runtime` home, `ade_runtime::network::n2n_listener`,
  `ade_node::node_lifecycle` (On-arm), `ade_node::produce_mode` (re-pointed).

## Hard lines (inherited by every slice)

- No `--mode produce` switch; no private-only path (the S1 path-fidelity fence
  `ci_check_node_path_fidelity.sh` stays green).
- No `push_atomic` / serve / served-chain mutation in `run_relay_loop` (the containment gate
  `ci_check_node_run_loop_containment.sh` stays byte-/semantically unchanged).
- No second `ServedChain` authority; no parallel tag-24 serializer (single CN-WIRE-08 envelope).
- No new `--mode node` argv flag (reuse `--listen`); no from-genesis constructor; no new BLUE
  authority / canonical type / `NodeBlockSource` or `CoordinatorEvent` variant.
- No proactive `advance_tip` / `served_chain_view.changed()` reactor (request-driven serve
  only; the proactive path is a separate future cluster if C1 proves it necessary).
- No peer-acceptance / BA-02 claim without a real peer log through `ba02_evidence::correlate`;
  **no `RO-LIVE-01/06` flip at implementation close** (the operator-witnessed live ACCEPT is
  the separate gating flip — mirrors G-A..G-C). C1 acceptance ≠ bounty completion;
  preview/preprod = completion.
