# PHASE4-N-A — Invariant Sketch

> **Status**: planning artifact, not normative.
> **Concept**: Ouroboros mini-protocols — 11 protocols (6 N2N + 5 N2C)
> that cardano-node 10.6.2 speaks.
> **Seeded by**: `docs/active/PHASE4-N-A_scope_decisions.md` (locked
> design decisions).
> **Authority**: registry entries in
> `docs/ade-invariant-registry.toml` are normative; this sketch
> frames them.

## 1. What must always be true

| # | Invariant | Registry rule |
|---|---|---|
| 1.1 | Every protocol-visible message decodes into exactly one closed, versioned message type | CN-WIRE-07 |
| 1.2 | Every valid message round-trips byte-identically: `encode(decode(b)) == b` and `decode(encode(m)) == m` | T-ENC-03 |
| 1.3 | Handshake selects exactly one protocol version per session (or rejects) | DC-PROTO-05 |
| 1.4 | Once a version is selected, all subsequent session messages use that version's encoding | DC-PROTO-05 (implicit) |
| 1.5 | Each mini-protocol state machine is a total function: every `(state, input)` maps to `Result<(state', output), error>` | DC-PROTO-01, T-CORE-03 |
| 1.6 | Mux framing bytes for a given logical message sequence are byte-identical to cardano-node's mux output | DC-PROTO-02 |
| 1.7 | Chain-sync emits fork-choice signals byte-identical to what cardano-node would emit for the same `(state, message)` input | DC-PROTO-02 |
| 1.8 | Full N2N surface (6 protocols): Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing | DC-PROTO-03 |
| 1.9 | Full N2C surface (5 protocols): Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor | DC-PROTO-04 |
| 1.10 | Decode failures are structured (typed errors), comparable across runs | T-ERR-01 |

## 2. What must never be possible

| # | Forbidden | Registry rule |
|---|---|---|
| 2.1 | A protocol message decoding into more than one type (ambiguous taxonomy) | CN-WIRE-07 |
| 2.2 | An unknown/unrecognized message tag silently accepted; must fail deterministically | DC-INGRESS-01 |
| 2.3 | Mixed protocol versions within a single session | DC-PROTO-05 |
| 2.4 | Runtime registration of new message types (no plugin pattern) | CN-WIRE-07, DC-PROTO-03/04 |
| 2.5 | Wall-clock, randomness, raw `HashMap`/`HashSet`, floating point, or OS-dependent iteration in BLUE codec or state-machine modules | T-CORE-02 |
| 2.6 | `async fn`, `.await`, `tokio::`, `async_std::`, `Future`, `futures::`, task spawning, async channels, or timers in BLUE modules | **DC-CORE-01** (new) |
| 2.7 | A handshake "Accept" naming a version not in the intersection of proposed × supported | DC-PROTO-05 |
| 2.8 | Any flow-controlled protocol accepting a transition that violates its flow-control semantics | DC-PROTO-01 |
| 2.9 | Network bytes reaching a BLUE state machine without traversing the codec chokepoint | DC-INGRESS-01 |
| 2.10 | Mux frames leaking past the codec layer in a non-canonicalized form | DC-INGRESS-01 |

## 3. What must remain identical across executions

- **Encoding**: same `Message` value → same bytes, every time, every host, every order
- **Decoding**: same bytes → same `Message` value (or same typed error), every time
- **State transitions**: same `(state, input)` → same `(state', output)` or same typed error
- **Handshake version selection**: same `(proposed_table, supported_table)` → same selected version (or same rejection)
- **Chain-sync rollforward/rollback signals**: same input sequence → same emitted signal sequence
- **Block-fetch frame ordering**: same request → same frame sequence

Determinism surfaces of N-A. Anchored by T-DET-01, DC-PROTO-01, DC-PROTO-02.

## 4. What must be replay-equivalent

- A captured cardano-node↔cardano-node session, decoded by Ade's codec, produces the same logical message sequence as captured
- Given the corresponding state-machine inputs, Ade's encoder produces byte-identical frames to the captured bytes
- Two parallel Ade instances given the same canonical message sequence produce byte-identical outputs at the wire

Canonical inputs source: `corpus/network/` (created in S-A9). Mechanically enforced by replay tests on the captured frames. Anchored by T-DET-01 and DC-PROTO-02.

## 5. State transitions in scope

All transitions are pure: `fn transition(state, input) -> Result<(state', output), error>`. No I/O, no async, no time.

```
Handshake          (HandshakeState,    HsMsg)   → Result<(HandshakeState',    HsOut | Done),     HsErr>
ChainSync          (ChainSyncState,    CsMsg)   → Result<(ChainSyncState',    CsOut | Signal),   CsErr>
BlockFetch         (BlockFetchState,   BfMsg)   → Result<(BlockFetchState',   BfOut | Frames),   BfErr>
TxSubmission2      (TxSubState,        TxSMsg)  → Result<(TxSubState',        TxSOut | TxIds),   TxSErr>
KeepAlive          (KeepAliveState,    KaMsg)   → Result<(KeepAliveState',    Cookie | Reply),   KaErr>
PeerSharing        (PeerShareState,    PsMsg)   → Result<(PeerShareState',    Peers | Reply),    PsErr>
N2C-Handshake      same shape as N2N handshake, different version table
LocalChainSync     same shape as ChainSync
LocalTxSubmission  (LocalTxState,      LtsMsg)  → Result<(LocalTxState',      Accept | Reject),  LtsErr>
LocalStateQuery    (LsqState,          LsqMsg)  → Result<(LsqState',          QueryReply),       LsqErr>
LocalTxMonitor     (LtmState,          LtmMsg)  → Result<(LtmState',          MempoolEvent),     LtmErr>
```

Each state machine encodes explicit agency in the state type (illegal agency combinations unrepresentable).

## 6. TCB color hypothesis

| Module | Color | Rationale |
|---|---|---|
| `ade_network::codec` | **BLUE** | Pure CBOR transformation, sync, no I/O |
| `ade_network::handshake` | **BLUE** | Pure version-negotiation state machine |
| `ade_network::chain_sync` | **BLUE** | Pure transition function; signals are values |
| `ade_network::block_fetch` | **BLUE** | Pure transition function; outputs are frame values |
| `ade_network::tx_submission` | **BLUE** | Pure transition + inventory state machine |
| `ade_network::keep_alive` | **BLUE** | Pure cookie protocol |
| `ade_network::peer_sharing` | **BLUE** | Pure message exchange (peer-book authority lives in N-F) |
| `ade_network::n2c` | **BLUE** | All 4 N2C state machines, same shape |
| `ade_network::mux` | **RED** | TCP/Unix sockets, tokio, framing, flow control |
| `ade_network::session` | **RED** | Composition glue: socket ↔ mux ↔ codec ↔ state machine |
| Frame corpus replay (in `ade_testkit`) | **GREEN** | Deterministic transcript harness, non-authoritative |
| Live interop binary | **RED** | Real socket against real cardano-node |

8 BLUE modules, 2 RED modules in `ade_network`. Plus GREEN test infra and one RED interop driver. Cleanest FC/IS partition in the project so far: every authoritative concern is a pure transition.

## 7. Open questions (to resolve before /cluster-doc)

1. **Protocol-version state location**: where does the selected version live during a session — in RED session-glue (BLUE state machines stay pure-state) or threaded through state-machine input? Lean: thread it as input.
2. **Peer-sharing authority**: peer-sharing message exchange is BLUE, but the peer-book it describes — does that live in `ade_network` (RED) or in cluster N-F (operator surface)? Lean: peer-book is RED operational state owned by N-F; peer-sharing is just transport.
3. **LocalStateQuery taxonomy**: query types overlap with ledger-state types. Where does the typed-query taxonomy live — `ade_network::codec` (BLUE, frames only) or in an `ade_ledger` query surface? Lean: codec models the wire shape; semantic interpretation of query results is N-B/N-F concern.
4. **Tx-submission2 ↔ mempool boundary**: tx-submission2 inventory negotiation references tx ids. `ade_types` has the canonical tx-id type; `ade_network::codec` should reference but not redefine.
5. **Frame corpus location**: `corpus/network/` alongside existing `corpus/snapshots/`, or a separate top-level location? Lean: `corpus/network/`.
6. **Live interop infrastructure**: docker-compose with cardano-node committed to the repo, or operator-provided peer? The repo's `corpus/boundary_blocks/` etc. originate from external data, so external-peer-provided model has precedent.
7. **Agency type encoding**: per-protocol agency states (`ClientHasAgency` / `ServerHasAgency`) as separate types per protocol, or one generic `Agency<P>` wrapper? Generic is more code-reusable; per-protocol is more type-safe.

## Can N-A be expressed as `canonical input → canonical output`?

**Yes**, for every BLUE slice:
- Codec: bytes ↔ Message (pure transformation, both directions)
- State machines: `(State, Message) → (State', Output)` (pure transition)

The RED slices (mux, session) wrap I/O around these pure cores; they don't add authority, they just move bytes. Every authoritative concern is a pure transition function. This is the cleanest FC/IS partition in the project so far.

---

## Registry status

The constitution already declares the rules cluster N-A will enforce.
Cluster N-A **strengthens** existing entries (populating `code_locus`,
`tests`, `ci_script` as slices ship and CI lands), rather than
introducing new IDs. Verified by inspection on 2026-05-19.

| Rule | Pre-existing | Strengthened by |
|---|---|---|
| `CN-WIRE-07` | yes (line 905) | S-A2 codec slice |
| `DC-PROTO-01` | yes (line 525) | S-A3..S-A8 state-machine slices |
| `DC-PROTO-02` | yes (line 535) | S-A9 frame corpus + S-A10 live interop |
| `DC-PROTO-03` | yes (line 616) | S-A2 codec (N2N coverage) + S-A4..S-A7 state machines |
| `DC-PROTO-04` | yes (line 627) | S-A2 codec (N2C coverage) + S-A8 state machines |
| `DC-PROTO-05` | yes (line 637) | S-A3 handshake slice |

**One genuinely new rule added** by this invariants step:

| Rule | Added at | Statement |
|---|---|---|
| `DC-CORE-01` | this commit (status: declared) | BLUE authoritative crates are sync-only: no `async fn`, `.await`, `tokio::`, `async_std::`, `Future`, `futures::`, task spawning, async channels, or timers. Async runtime concerns are confined to RED transport/runtime code. |

`DC-CORE-01` will be enforced by `ci/ci_check_no_async_in_blue.sh`
landing with S-A1, completing the BLUE/RED partition's mechanical
gate before S-A2 begins.

Bidirectional cross-refs updated:
- T-CORE-02 ← DC-CORE-01 (new)
- DC-PROTO-01 ← DC-CORE-01 (new)
- DC-CORE-01 → T-CORE-02, DC-PROTO-01

## Note on the scope-decisions doc

`docs/active/PHASE4-N-A_scope_decisions.md` lists CN-WIRE-07,
DC-PROTO-03, DC-PROTO-04 under "registry entries to add." That was
written before the existing registry was inspected for these IDs.
The doc should be updated (in a follow-up edit) to read
"strengthen existing CN-WIRE-07, DC-PROTO-01..05" — and to add
DC-CORE-01 to its operational-consequences list.
