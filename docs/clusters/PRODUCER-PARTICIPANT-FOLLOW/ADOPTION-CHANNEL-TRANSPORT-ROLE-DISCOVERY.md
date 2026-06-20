# ADOPTION-CHANNEL-TRANSPORT-ROLE-DISCOVERY (pre-code)

> **Status:** DISCOVERY (2026-06-20, pre-code). The responder IMPLEMENTATION is GATED on this discovery's outcome — **do NOT write responder code until the live question below is answered.**
> **Cluster:** PRODUCER-PARTICIPANT-FOLLOW (rung-3 prerequisite).
> Supersedes the earlier `ADOPTION-CHANNEL-duplex-responder-scope.md` framing (which prematurely assumed "the peer pulls over duplex").

## Why this is a discovery, not an implementation
Rung-3 adoption needs a real `cardano-node-preview` peer to FETCH an Ade-forged block and log `AddedToCurrentChain`. A hermetic duplex responder proves only that **Ade can serve**; it does NOT prove the Haskell node will **request** Ade's block over the connection. That live compatibility precondition is load-bearing for the whole slice and is currently **UNVERIFIED**. Spending ~200–300 lines on a responder before verifying it risks "we built it but the peer never pulls from Ade."

**Hard framing (do not drift):** until verified, do NOT phrase this as "the peer pulls from Ade over duplex." The open question is exactly:

> **THE QUESTION:** Determine the cardano-node transport + mini-protocol role arrangement required for a Haskell Preview peer to request an Ade-served block.

## The live observation (NO forge required)
Run Ade following the peer (`--participant-venue --mode node`; no leader slot needed) with a **sharp temporary diagnostic**, and answer with EVIDENCE for each:

1. **Connection direction + peer endpoint** — who dialed whom; the `addr:port` of the connection Ade↔node.
2. **Negotiated mini-protocol roles on that same connection** — the diffusion mode actually negotiated; which mini-protocols run initiator vs responder.
3. **FindIntersect to Ade?** — does the Haskell node send ChainSync `MsgFindIntersect` to Ade (a mode=Responder ChainSync(2) frame = the node acting as a CLIENT pulling from Ade)?
4. **RequestNext / RequestRange to Ade?** — does it send ChainSync `MsgRequestNext` / BlockFetch `MsgRequestRange` to Ade?
5. **Ade's server dispatcher reached?** — would the inbound mode=Responder frames reach a serve dispatch (once wired)?
6. **Spontaneous vs configured** — does any of this happen spontaneously, or only after topology/localRoot configuration on the node?
7. **Separate outbound dial?** — does the node instead require a separate outbound dial to **Ade:3033** (the localRoot serve listener) to pull?

**Instrumentation:** a temporary frame-mode diagnostic in Ade's session/mux receive path (log `mode` + mini-protocol-id + message-tag for every inbound frame) + the node's docker logs (ConnectionManager / InboundGovernor / PeerSelection / Mux tracers). **Temporary only; reverted before any slice code.** Mode semantics: mode=Initiator frames = the node replying to Ade's own follow client; **mode=Responder ChainSync(2)/BlockFetch(3) frames = the node pulling FROM Ade** (the load-bearing signal).

## Decision rule (gates the next step)
| Observation | Next step |
|---|---|
| Haskell sends ChainSync/BlockFetch requests over the **duplex** connection | Implement the responder slice **as scoped** (duplex-over-the-dial) |
| Haskell does NOT pull on duplex but **dials Ade** when Ade is a configured localRoot | Scope the responder around the **listener/dial topology** (`:3033`), NOT duplex |
| Haskell **neither pulls nor dials** Ade | **STOP** — adoption architecture is wrong/incomplete; investigate peer-selection/topology before any code |
| **Only Ade** initiates protocol client roles | **STOP** — do not build the responder yet |

---

## Discovery RESULT — 2026-06-20: node does NOT pull (InitiatorOnly); do NOT build the responder
Live no-forge observation (preview epoch 1334; Ade `--participant-venue --mode node`; temporary WPDIAG frame-mode diagnostic at `session/core.rs` drain, since reverted). Findings:
- **Ade's side:** 28 inbound frames, **100% mode=Responder** (ChainSync/BlockFetch/KeepAlive server replies — the node SERVING Ade's follow client), **0 mode=Initiator** (the node sent Ade zero client requests). Tags seen were all server messages (RollForward 0x02, RollBackward 0x03, IntersectFound 0x05, AwaitReply 0x01, StartBatch 0x02, Block 0x04).
- **Node's side:** Ade's connection negotiated **`InitiatorOnlyDiffusionMode` / `InboundIdleSt Unidirectional`**; the node `PromotedToHotRemote` (serves Ade) but ran **no initiator mini-protocol to Ade** and **never dialed `:3033`**.
- Evidence preserved: `$C2/discovery-transport-role/{wpdiag-frames.txt,node-view-of-ade.txt}`.

**Decision-rule outcome: "only Ade initiates / neither pulls nor dials" → do NOT build the responder.** The node cannot pull because the connection is unidirectional — and it is unidirectional because **Ade's dial handshake advertises InitiatorOnly** (contradicting the version-table's claimed duplex). The first gate is the diffusion-mode / topology, NOT the responder. (Note: the initial WPDIAG `peer_as_client` flag keyed on mode=Responder was BACKWARDS — mode=Responder is the node-as-server; the true pull signal is mode=Initiator client tags, of which there were zero.)

**Next (read-only investigation — the rule's "investigate peer-selection/topology before coding"):** (a) why Ade advertises InitiatorOnly + the smallest change to `InitiatorAndResponderDiffusionMode`; (b) whether a duplex advertisement would make the node promote Ade to a hot OUTBOUND/initiator peer; (c) the localRoot/`:3033` alternative (configure Ade's serve listener as a node localRoot). The conditional responder design below applies ONLY after the node demonstrably pulls (mode=Initiator `FindIntersect`/`RequestRange` to Ade).

---

## Discovery RESULT 2 — 2026-06-20: node DOES pull via the localRoot dial (PROVEN — NO responder code needed)
Root of RESULT 1: the node container started **2026-06-17**, but the Ade:3033 localRoot was added to `topology.json` **2026-06-18** — so the running node had never loaded it. After **restarting the node**:
- It loaded the localRoot: `TraceLocalRootPeersChanged ... 172.17.0.1:3033 ... localProvenance = Outbound, IsTrustable`, then `PromoteColdLocalPeers [172.17.0.1:3033]` and dialed it (`ConnectError ... refused` while Ade's `:3033` was still down).
- Once Ade was serving (`--mode node --listen 0.0.0.0:3033`), the localRoot dial **succeeded**: `HandshakeSuccess NodeToNodeV_15` to `172.17.0.1:3033`, `PromoteWarmDone`, and **Ade's serve listener received the node's chain-sync CLIENT requests** — WPDIAG mode=Initiator: `proto=2 tag=0x04` (**MsgFindIntersect**) + `proto=2 tag=0x00` (**MsgRequestNext**) + KeepAlive. The node is pulling Ade's chain. (Ade's separate follow connection still shows mode=Responder frames = Ade following the node.)
- Evidence: `$C2/discovery-transport-role/{localroot-pull-node-initiator-frames.txt,node-localroot-dial.txt}` + `proof1d-*`.

**FINAL DECISION: the adoption channel works via the EXISTING `:3033` serve listener + the node's localRoot — NO new responder code.** The duplex-over-the-dial responder (the conditional design below) is **not needed and is shelved** — cardano-node never pulls over a unidirectional *inbound* connection; the localRoot **dial** is the real Cardano adoption model (producer serves on a listener; relay dials it as a localRoot client). The only operational requirement was the node loading the Ade:3033 localRoot (a restart). **Rung 3 (adoption) now reduces to rung 2:** once Ade forges a block at an ADE1 leader slot, the node — already pulling Ade's chain via the localRoot — fetches it and can log `AddedToCurrentChain`. The remaining blocker is rung 2 (a leader slot), not the channel.

---

## Conditional responder design — SHELVED (duplex-pull outcome did not occur; localRoot dial works instead)
*Carried from the prior scope pass; valid only if the discovery returns "duplex-pull". For the "dials-:3033" outcome the serve core below is reused but driven from the listener, not the dial.*

### The gap
- Ade's dial (`run_admission_wire_pump`, `crates/ade_runtime/src/admission/wire_pump.rs:175-419`) advertises duplex (`version_table.rs:93`) but runs only the **client** mini-protocols; it drains Responder-mode frames from the session but never **dispatches** them (defer marker `wire_pump.rs:659`).
- The serve responder exists only on the separate `:3033` listener (`run_node_serve_task`, `node_lifecycle.rs:1003-1086`).

### Tractability (the serve core is connection-agnostic + already proven)
- Mux/session already carry bidirectional frames (`mux/frame.rs:27-110` MuxMode flag; `session/core.rs:211-269` direction-agnostic demux).
- `dispatch_server_frame_event_to_outbound` (`serve_dispatch.rs:137+`) + `ChainDbServedSource`/`serve_range` (`served_chain_projection.rs:96-152`) + the BLUE serve reducers have NO socket binding — the `:3033` listener already serves with this exact core. ~200–300 lines, ~80% reuse: add to the dial (1) per-connection server FSM state, (2) an outbound-command channel (mirror `mux_pump.rs:133-161`), (3) responder-frame dispatch into the serve core with `ServedChainSource::DurableChainDb`, (4) a select arm draining outbound commands.

### Entry obligations (answer before responder code)
1. **Inbound frame-mode detection** — `SessionEffect::DeliverPeerFrame` lacks the `MuxMode`; decide between extending the effect with `mode`, re-parsing the mux header in the dial, or a parallel direction-tracking buffer. (This discovery's diagnostic is the throwaway version of this.)
2. **Dual-FSM safety** — one connection carries both a client FSM (Ade follows the peer) and a server FSM (Ade serves the peer); a direction violation must fail closed (drop the peer), never cross-feed.
3. **Serve invariants over the dial = identical to the listener** — DC-SERVEMEM-01 (`MAX_SERVE_RANGE_BLOCKS=256`, fail-closed), DC-NODE-13 (durable-ChainDb-only, read-only), DC-CONS-07/CN-CONS-07 (byte-provenance), DC-CONS-18 (single decode authority).

### Candidate MAC
- **Hermetic (independently testable, NO leader slot):** a loopback/mock peer sends ChainSync `FindIntersect` + BlockFetch `RequestRange` over the dial; Ade serves an already-stored block back (bounded ≤256); a direction-violating frame fails closed. This is the "serve an already-known stored block through the duplex responder" test that keeps the slice independently exercisable.
- **Serve-invariant regression:** the existing DC-SERVEMEM-01 / DC-NODE-13 serve tests pass with the serve driven over the dial.
- **Live (rung 3, only AFTER rung 2):** Ade forges an ADE1 block, the node pulls it, `AddedToCurrentChain` for Ade's exact hash. The ONLY thing that earns adoption / BA02 evidence.

### Hard prohibitions / non-goals
- No BA02 claim from building the channel (necessary, not sufficient — only `AddedToCurrentChain` counts).
- Do not conflate with rung 2 (Ade forging at a leader slot).
- Serve stays READ-ONLY; no weakening of DC-SERVEMEM-01 / DC-NODE-13; reuse `dispatch_server_frame_event_to_outbound` + `ChainDbServedSource` (no second serve authority).
- Do not resurrect the quarantined FindIntersect point-list (DC-NODE-42).

---

## Deferred operational gap (recorded — NOT a BA02 blocker)
**Cross-epoch forge halt.** The `--mode node` forge binds single-epoch consensus inputs (eta0 + the `set` stake snapshot) and cannot cross an epoch boundary: off-epoch slots fail closed (`ForgeEpochAdmission::OffEpoch` → `ForgeNotLeader`, node_sync.rs) and epoch-N+1 blocks fail header validation (`PoolDistrView` returns `None` off-epoch), so the node **silently stalls** at the boundary — no clean halt, unlike `--mode admission`'s `CrossEpochUse`/exit-32 guard (admission/runner.rs:431-443). **Deferred** as operational hardening (a multi-epoch unattended leader-hunt enabler), explicitly NOT on the bounty-critical acceptance path. **Mitigation now:** use fresh, targeted, single-epoch runs around known ADE1 leader slots (re-seed per run). A future clean cross-epoch fail-closed halt (mirroring admission's guard) + a re-seed harness would enable unattended runs — but only build it if/when a multi-epoch hunt is chosen.
