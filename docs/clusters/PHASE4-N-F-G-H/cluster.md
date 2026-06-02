# Cluster PHASE4-N-F-G-H — Node-spine live serve-to-peer

> **Status: PLANNED** (S1–S3 not started; `DC-NODE-07` declared at sketch `c882e861`; from the committed plan `docs/planning/phase4-n-f-g-h-cluster-slice-plan.md` `b22b5a29` + the `/invariants` sketch `docs/planning/phase4-n-f-g-h-invariants.md` `c882e861`). Follow-on sub-cluster of **PHASE4-N-F-G** (RO-LIVE-01). Predecessors: G-A forge fidelity (`62cb8718`), G-B serve handoff (`febee120`, `DC-NODE-06`), G-C live feed + operator-gated evidence (`351d46bc`), G-E live-feed bounded memory (`da205bff`), G-D bounty dry-run (`6bd60c80`, `CN-REHEARSAL-FIDELITY-01`). Code-verified at HEAD `b22b5a29`.
>
> **Cluster character (load-bearing — do not broaden):** G-H wires the **owed shared-path serve-to-peer leg** the G-D C1 dry-run surfaced: `--mode node` forges + self-accepts + pushes into a `ServedChainHandle` (G-B) but **discards the `ServedChainView` read side** (`node_lifecycle.rs:475`, comment *"no live reader"*) and runs **no listener / no serve server**, so a real Haskell follower cannot fetch Ade's forged block. G-H makes `--mode node` serve that block on the **same** accepted-block path C2/preprod needs (NOT `--mode produce`). It is **reuse, not new authority**: the serve reducers + the GREEN `ServedChainView` already exist and `produce_mode` already drives both; the only new code is RED wiring. **Two halves with sharply different IDD status** (mirrors G-C/G-D): (1) a **MECHANICAL half** (closeable hermetically) — extract the single shared serve-dispatch authority (S1) + wire the node spine + a hermetic loopback (S2); (2) an **OPERATOR-GATED half** (stays `blocked_until_operator_c1_serve_executed`) — the actual C1 live execution against a real Haskell follower (S3).
>
> **⚠ LOAD-BEARING FINDING (scope fence — verified at HEAD `b22b5a29`):** `produce_mode` **does not currently drive `producer_chain_sync_advance_tip`** (zero calls; no `served_chain_view.changed()` reactor). Its serve is **purely request-driven** (`dispatch_server_frame_event_to_outbound` reacts only to *incoming* peer frames). So G-H is scoped to the **request-driven case only**: the follower's `RequestNext` arrives *after* the served view already holds the self-accepted block → `RollForward` → `BlockFetch`. The **proactive** `RollForward`-on-view-update (an `advance_tip` driver) is **new behavior wired nowhere** and **MUST NOT** be pulled into S2 under "reuse." If a real C1 Haskell follower proves it necessary, that is a **separate new shared-path cluster** — never a G-H slice (no carry-forward).
>
> **Hard lines (any one breached → stop and re-scope):**
> - **No `--mode produce` switch.** The serve is wired on the `--mode node` spine; `produce_mode` is only *re-pointed* at the shared core (behavior byte-unchanged).
> - **No private-only path.** Every G-H element transfers verbatim to C2/preprod (the S1 path-fidelity fence `ci_check_node_path_fidelity.sh` stays green).
> - **No serve / `push_atomic` / served-chain mutation in `run_relay_loop`** (`ci_check_node_run_loop_containment.sh` byte-/semantically unchanged); the serve is a **sibling** task.
> - **No second `ServedChain` authority; no parallel tag-24 serializer** (single `ServedChainHandle`/`View`; single CN-WIRE-08 envelope).
> - **No proactive `advance_tip` / `served_chain_view.changed()` reactor** (request-driven serve only).
> - **No acceptance without `correlate`; no RO-LIVE flip** at implementation close.

## Primary invariant
**`DC-NODE-07`** (declared; `introduced_in = PHASE4-N-F-G-H`) — `--mode node` serves real peers **only** the G-B self-accepted `ServedChainView`, through a sibling listener + serve-dispatch task **outside** `run_relay_loop`, reusing the **single** ChainSync+BlockFetch serve-dispatch authority and the **single** CN-WIRE-08 serializer — no second serve authority, no `--mode produce` switch, no relay-loop serve mutation, no new `--mode node` flag (reuses `--listen`). Wiring the serve is not a peer-acceptance claim. *(Cited, not restated — see the registry entry.)*

## Invariants strengthened / carried (at close)
- **`DC-NODE-07`** — flips `declared → enforced` (tests + `ci_script` populated by S1/S2/S3).
- **Candidate strengthenings (confirmed at close, not assumed):** `DC-CONS-17`, `DC-CONS-18`, `DC-PROTO-07`, `DC-PROTO-08`, `CN-PROTO-06` may receive `strengthened_in += PHASE4-N-F-G-H` **iff** S2's node-spine loopback becomes a new enforcement site for them. Decided at close.
- **Carried unchanged (not weakened):** `DC-NODE-06` (G-B serve handoff + sibling `push_atomic`), `CN-NODE-02` (single live-run lifecycle owner + relay-loop containment), `CN-FORGE-01` (forge advances no durable tip on the serve path), `DC-SYNC-01`/`DC-SYNC-02` (single durable tip-advance), `CN-WIRE-08` (single tag-24 authority), `CN-REHEARSAL-FIDELITY-01` (path fidelity), `DC-LIVEMEM-01` (live-feed bounded memory).
- **Deliberately NOT strengthened:** `RO-LIVE-01` / `RO-LIVE-06` receive **no `strengthened_in += PHASE4-N-F-G-H` bump** — a bump would wrongly imply G-H advanced the bounty deliverable. G-H `cross_ref`s `RO-LIVE-01` and records the decoupling (the live ACCEPT stays operator-gated).

## Normative anchors
- `docs/planning/phase4-n-f-g-h-cluster-slice-plan.md` — the G-H plan (1 cluster, 3 CEs, 3 slices; the `advance_tip` scope fence).
- `docs/planning/phase4-n-f-g-h-invariants.md` — the `/invariants` sketch (A0–A7; N0–N7; OQ1–OQ5).
- `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` + `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` — the `--mode node` operator-pass runbooks the S3 serve runbook adapts.
- Registry: `DC-NODE-07`, `DC-NODE-06`, `CN-NODE-02`, `DC-CONS-17`, `DC-CONS-18`, `CN-PROTO-06`, `DC-PROTO-07`, `DC-PROTO-08`, `CN-WIRE-08`, `CN-FORGE-01`, `RO-LIVE-01`, `RO-LIVE-06`, `CN-REHEARSAL-FIDELITY-01`.

## Entry conditions (what prior clusters guarantee)
- **G-B (closed, `febee120`):** `DC-NODE-06` enforced — a self-accepted artifact reaches the served chain via the typed `SelfAcceptedHandoff` + the sibling `push_atomic` task fed only by `into_accepted()`; relay-loop body forwards a typed channel send only. The `ServedChainHandle::new() → (handle, view)` pair exists; the `ServedChainView` is the watch read side.
- **G-C/G-D/G-E (closed):** the `--mode node` `On` arm is live-feed-wireable from `--peer` (the A7(a) upstream feed → `WirePump` Continuing → `ForgeTick`); `ba02_evidence::correlate` + `ba02_pass` exist; `ci_check_node_path_fidelity.sh` (G-D S1) pins the closed `--mode node` flag set; live-feed memory bounded (`DC-LIVEMEM-01`).
- **Producer serve infra (N-G/N-R/N-S — enforced, reused VERBATIM):** `producer_chain_sync_serve` + `producer_block_fetch_serve` (`ade_network::{chain_sync,block_fetch}::server`) are pure/total/deterministic (`DC-PROTO-07/08`, `CN-PROTO-06`, `DC-CONS-17/18`); `run_n2n_listener` + `new_per_peer_outbound` are **already public** in `ade_runtime::network`; `produce_mode` drives both serve reducers from the `ServedChainView` via `handle_listener_event` → `dispatch_server_frame_event_to_outbound`. `RO-LIVE-01` is `partial` (the serve-direction operator ACCEPT is unexecuted).

## Verified component inventory (read at HEAD `b22b5a29`, not assumed)
| Component | Real state (verified) | Use |
|---|---|---|
| `dispatch_server_frame_event_to_outbound` (`produce_mode.rs:1332`) | async, **RED**; **coordinator-free**: decodes the frame, borrows `ServedChainView`, drives `producer_chain_sync_serve`/`producer_block_fetch_serve`, sends `OutboundCommand` via `PerPeerOutbound`; returns `(sent, Vec<ServedBlockEvidence>)` | **S1** the core extracted to a shared `ade_runtime` home |
| `handle_listener_event` (`produce_mode.rs:1187`) | async, **RED**; **entangled** with `CoordinatorState`/`coordinator_step` + the producer evidence writer + `connected_peers` | **S1** only the **per-peer-state lifecycle** (install on `PeerConnected` / remove on `PeerDisconnected`) factors into the shared core; the coordinator + producer-evidence wrapping **STAYS in `produce_mode`** — the node spine must NOT drag in `coordinator_step` |
| `ServerPeerStates` (`:1185`), `DispatchError` (`:1291`), `ServedBlockEvidence` (`:1315`) | helper types (RED/GREEN) | **S1** move with the adapter |
| `run_n2n_listener` (`ade_runtime/src/network/n2n_listener.rs:98`), `new_per_peer_outbound` (`ade_runtime/src/network/outbound_command.rs:46`) | **RED; already `pub` in `ade_runtime`** | **S2** reused verbatim (no extraction needed) |
| `producer_chain_sync_serve`, `producer_block_fetch_serve`, `producer_chain_sync_advance_tip` (`ade_network/src/{chain_sync,block_fetch}/server.rs`) | **BLUE** per the repo's own color map (both `server.rs` header banners read "BLUE producer-side … server-role surface (PHASE4-N-G S1)"; CODEMAP lists `chain_sync/` + `block_fetch/` among the 9 BLUE `ade_network` submodules); pure/total/deterministic | reused VERBATIM; **`advance_tip` is NOT wired by any binary — see the finding** |
| `ServedChainView` (`ade_runtime/src/producer/served_chain_handle.rs`) | **GREEN**; watch read side | **S2** retained (stop discarding at `node_lifecycle.rs:475`), read by the serve sibling |
| `node_lifecycle.rs` On arm (`453–514`): `_serve_view` discard (`:475`), push-only `serve_task` (`:477`), `run_relay_loop` (`:500`), `serve_task.await` (`:514`) | **RED**; the node spine | **S2** retain `serve_view` + add the listener+dispatch sibling **alongside** the existing push sibling |
| `--mode node` argv `--listen` | already in the closed flag set (G-D S1 fence) | **S2** reused; **no new flag** |
| `ci_check_node_run_loop_containment.sh`, `ci_check_node_path_fidelity.sh`, `ci_check_served_chain_handoff_fence.sh` | the containment / fidelity / handoff fences | **UNCHANGED** by G-H (hard line) |
| `live_feed_forge_serve_loopback_returns_forged_block` (`forge_succeeds.rs:519`) | the G-B push→served loopback (no socket) | **S2** model; the new socket-serve loopback is a distinct candidate test |

## Slices (safety order)

### S1 — Extract shared serve-dispatch authority *(mechanical; CE-G-H-1)*
Move the **coordinator-free serve-dispatch core** — `dispatch_server_frame_event_to_outbound` + the per-peer-state lifecycle (install on `PeerConnected` / remove on `PeerDisconnected`) + `ServerPeerStates` + `DispatchError` + `ServedBlockEvidence` — from private `produce_mode` to a shared `ade_runtime::network` serve module. Re-point `produce_mode`'s `handle_listener_event` at the shared core (its `CoordinatorState`/`coordinator_step`/producer-evidence wrapping **stays**); `produce_mode` serve behavior is **byte-unchanged**. Add `ci_check_single_serve_dispatch_authority.sh`: exactly one definition of the serve-dispatch core, called by both modes; `node_lifecycle` defines no parallel serve-dispatch. Addresses **CE-G-H-1**. TCB: **RED** (adapter) + **GREEN** (the moved evidence type, if pure). No BLUE change.

### S2 — Node-spine serve wiring + hermetic loopback *(mechanical; CE-G-H-2)*
On the `--mode node` `On` arm: stop discarding `_serve_view`; spawn a listener + serve-dispatch **sibling** task (gated on `--listen` being present) that reads the `ServedChainView`, reusing `run_n2n_listener` + `new_per_peer_outbound` + the S1 shared core; join it **alongside** the existing push sibling; the `run_relay_loop` body is unchanged (containment). **Request-driven only — no `advance_tip`/`changed()` reactor.** A hermetic loopback test (`node_spine_serve_loopback_follower_fetches_self_accepted_block`) brings up the node-spine serve on an ephemeral address, and a test client — with the self-accepted block **already in the served view before the request** — proves ChainSync `RollForward` (header == `accepted_block_header_bytes`, `DC-CONS-18`) **and** BlockFetch (body == `AcceptedBlock.as_bytes()`, `DC-CONS-17`). Addresses **CE-G-H-2**. TCB: **RED** (wiring). No BLUE change.

### S3 — C1 dry-run runbook + operator-gated serve harness *(operator-gated; CE-G-H-3)*
Commit the serve runbook `docs/evidence/phase4-n-f-g-h-node-serve-README.md` as a **strict adaptation** of the G-C/G-D `--mode node` operator-pass runbooks, adding only the downstream follower topology (the A7 two-direction obligation). Wire an env-gated harness `node_c1_serve_live` (`ADE_LIVE_C1_SERVE`), **skipped/blocked** without the C1 net. **Test-topology framing (not a live-run assumption):** the hermetic/C1 serve test *may arrange* the follower's ChainSync request to arrive after the served view already contains the self-accepted block, proving request-driven discovery/fetch; **if the real follower is already parked at tip before Ade forges and requires proactive `advance_tip`, STOP and scope that separately** (a new shared-path cluster). **Slice-entry proof obligation (OQ2):** confirm the C1 topology (one reciprocal Haskell peer vs. two Haskell nodes) at slice entry, not by assumption. No synthetic evidence; the live execution stays `blocked_until_operator_c1_serve_executed`; flips no RO-LIVE rule. Addresses **CE-G-H-3**. TCB: **RED** (runbook / env-gated harness) + **GREEN** (`correlate`, reused for any captured-log evidence).

## Exit criteria (mechanical, CI-verifiable)
New test/check names are **candidate** (created by the owning slice); existing artifacts named as-is.

- **CE-G-H-1 (single serve-dispatch authority — MECHANICAL, closeable)** — a candidate gate `ci_check_single_serve_dispatch_authority.sh` is green: exactly one definition of the serve-dispatch core in `ade_runtime`, imported by both `produce_mode` and `node_lifecycle`; no parallel serve-dispatch defined in `node_lifecycle`. `produce_mode` serve behavior byte-unchanged: `cargo test -p ade_node` green (incl. the existing producer serve tests) + the `ade_network` server tests green.
- **CE-G-H-2 (node-spine request-driven serve — MECHANICAL, closeable)** — a candidate test `node_spine_serve_loopback_follower_fetches_self_accepted_block` passes (hermetic node-spine serve via `run_n2n_listener`; an already-served self-accepted block is discovered via ChainSync `RollForward` and fetched via BlockFetch, header/body bytes asserted equal to the self-accepted artifact); `ci_check_node_run_loop_containment.sh` + `ci_check_node_path_fidelity.sh` + `ci_check_served_chain_handoff_fence.sh` **byte-/semantically unchanged + green**. No RO-LIVE flip; no proactive `advance_tip` introduced.
- **CE-G-H-3 (operator-gated C1 serve — SCAFFOLDS ONLY; live execution BLOCKED)** — the runbook `docs/evidence/phase4-n-f-g-h-node-serve-README.md` is committed (a strict adaptation of the G-C/G-D runbooks); a candidate env-gated `node_c1_serve_live` (`ADE_LIVE_C1_SERVE`) is **skipped/blocked** without the C1 net; **no synthetic evidence committed**; live execution stays `blocked_until_operator_c1_serve_executed`.

> No human review may substitute for these checks. CE-G-H-1 + CE-G-H-2 close the cluster mechanically; CE-G-H-3 closes its **scaffolding** mechanically — the live C1 serve is a separate operator-witnessed leg, never a RO-LIVE flip.

## TCB color map
- **BLUE (none new — reuse only; BLUE per the repo's own color map: the `chain_sync/server.rs` + `block_fetch/server.rs` header banners "BLUE producer-side … server-role surface" + CODEMAP's 9 BLUE `ade_network` submodule paths):** `ade_network::{chain_sync,block_fetch}::server` (`producer_chain_sync_serve`, `producer_block_fetch_serve`, `producer_chain_sync_advance_tip` — reused; advance_tip NOT wired), `ade_codec` tag-24 (`CN-WIRE-08`). **A BLUE change is a red flag → reject.**
- **GREEN:** `ade_runtime::producer::{served_chain_handle::ServedChainView, served_chain_lookups}` (reused); the moved `ServedBlockEvidence` (GREEN if a pure observation type — resolved at S1); `ade_node::ba02_evidence::correlate` (reused, S3).
- **RED:** the extracted shared serve-dispatch core's new `ade_runtime::network` home (`dispatch_server_frame_event_to_outbound` + per-peer-state lifecycle + `ServerPeerStates` + `DispatchError`); `ade_runtime::network::{n2n_listener, outbound_command}` (reused); `ade_node::node_lifecycle` (On-arm wiring); `ade_node::produce_mode` (re-pointed; coordinator/evidence stays); `ci_check_single_serve_dispatch_authority.sh`; the env-gated harness + runbook.

## Forbidden during this cluster *(slice-level prohibitions inherit)*
- **No `--mode produce` switch** — node spine only; `produce_mode` re-pointed (behavior-unchanged), never forked into.
- **No private-only path** — every element transfers verbatim to C2/preprod; the S1 path-fidelity fence stays green.
- **No serve / `push_atomic` / served-chain mutation in `run_relay_loop`** — the serve is a sibling task; `ci_check_node_run_loop_containment.sh` byte-/semantically unchanged.
- **No second `ServedChain` authority; no parallel tag-24 serializer** — single `ServedChainHandle`/`View`; single CN-WIRE-08 envelope via the BLUE servers.
- **No proactive `advance_tip` / `served_chain_view.changed()` reactor** — request-driven serve only; the proactive path is a separate future cluster if C1 proves it necessary.
- **No new `--mode node` argv flag** (reuse `--listen`); **no from-genesis constructor**; **no new BLUE authority / canonical type / `NodeBlockSource` or `CoordinatorEvent` variant**.
- **No peer-acceptance / BA-02 claim without a real peer log through `correlate`**; **no RO-LIVE flip** at implementation close.
- **Hard line:** if the leg needs a containment relaxation, a proactive push reactor, a second serve authority, a parallel serializer, or a `--mode produce` switch — **stop and re-scope** (the gap is a separate shared-path cluster or a scope error, not a thing to special-case).

## Replay obligations (scoped)
- **R1** — serve-transcript replay carried (`DC-PROTO-07`, reused): same `(negotiated_version, served-snapshot sequence, peer message sequence, session events)` → byte-identical outgoing frames.
- **R2** — **no new authoritative state**: the serve is read-only over the `ServedChainView`; it advances no durable tip, mints no `AcceptedBlock` (`CN-FORGE-01` / `DC-SYNC-01/02` unchanged); adds no WAL/checkpoint/canonical type.
- **No new BLUE canonical type, no new replay corpus.** S2 adds a hermetic node-spine serve loopback test (not a corpus entry). Acceptance scoped to touched crates (`ade_node` + consumed `ade_runtime`/`ade_network`/`ade_ledger`/`ade_codec`) + `ci` + `docs` — **not** the full `ade_testkit` corpus lane (pre-existing timeout).

## Registry impact (at close)
- **`DC-NODE-07`** — `declared → enforced`; `tests` + `ci_script` populated (S1: `ci_check_single_serve_dispatch_authority.sh`; S2: `node_spine_serve_loopback_follower_fetches_self_accepted_block`; S3: the env-gated `node_c1_serve_live`).
- **Candidate `strengthened_in += PHASE4-N-F-G-H`** on `DC-CONS-17/18`, `DC-PROTO-07/08`, `CN-PROTO-06` — **iff** S2's loopback is a new enforcement site (decided at close, not assumed).
- **No status flip on any RO-LIVE rule; no `strengthened_in` bump on `RO-LIVE-01` / `RO-LIVE-06`.**
- **Not added here:** any "peer accepted the block" rule; any proactive `advance_tip` rule; any new canonical type; any bounty-completion claim.

## Non-goals
- **The proactive `advance_tip` / `served_chain_view.changed()` reactor** (follower parked at tip → server pushes the next forged block) — a separate future shared-path cluster, only if a real C1 follower proves it required.
- **The bounty deliverable** (preview/preprod acceptance that flips `RO-LIVE-01`) — a separate operator-witnessed C2 leg. A passing C1 serve run is a dry-run signal, never bounty completion.
- **`--mode produce` changes** — only re-pointed at the shared core; its behavior is byte-unchanged.
- **Mempool / tx-submission serve, peer-sharing, additional mini-protocols** beyond ChainSync + BlockFetch.
- **Grounding-doc regeneration** (that's `/cluster-close`).
