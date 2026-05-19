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

## 7. Closed decisions

The seven questions originally raised here are closed. They split into
three categories — semantic authority, evidence / release
infrastructure, and slice-shaping — only two of which materially
shape slice design (#1 and #6).

| # | Decision | Tier | Boundary | Verdict |
|---|---|---|---|---|
| 1 | Thread selected protocol version explicitly into BLUE transitions | true + derived | BLUE | Accept B |
| 2 | Peer-book lives outside `ade_network`; peer-sharing transports peer blobs only | operational | RED | Accept B |
| 3 | LocalStateQuery codec owns envelope and closed discriminants, not ledger semantics | derived | BLUE/GREEN boundary | Accept B with caveat |
| 4 | Reuse `ade_types::TxId` | true | BLUE | Accept B |
| 5 | Put frame corpus under `corpus/network/` | release | evidence layout | Accept A |
| 6 | Hybrid live interop; Docker default, operator peer optional | release + operational | RED test infra | Accept C |
| 7 | Per-protocol agency types | true + derived | BLUE | Accept A |

### #1 — Protocol-version state location

**Load-bearing statement**: a BLUE transition must be a pure function
of canonical prior state, canonical input message, selected protocol
version, and deterministic configuration. **No session-glue state may
alter authoritative behavior invisibly.**

Signatures take version as an explicit input:

```rust
chain_sync_transition(state, version, msg) -> Result<(state', output), err>
```

not `chain_sync_transition(state, msg)` with version hidden in RED
context.

Version markers are **typed per protocol**, not loose integers:

```rust
struct N2NVersion(...)
struct ChainSyncVersion(...)
struct LocalStateQueryVersion(...)
```

(or a protocol-indexed wrapper). Otherwise the design is explicit but
under-typed.

**Mechanical enforcement** (S-A1 deliverable): the replay-trace record
includes `protocol_id`, `selected_version`, `input_message_canonical_bytes`,
`pre_state_hash`, `post_state_hash`, `output_or_error`. CI rejects BLUE
transition APIs that read ambient session state.

Captured in registry as **DC-PROTO-06** (added with this commit, cross-
referenced bidirectionally with T-CORE-02, DC-PROTO-01, DC-PROTO-05,
DC-CORE-01).

### #2 — Peer-sharing authority

`ade_network::peer_sharing` (BLUE) owns:
message taxonomy, agency, version gates, canonical decode/encode,
state machine, structured errors.

It does **not** own:
peer reputation, operator preferences, retention policy, dial
scheduling, topology strategy, geo preferences, liveness scoring.

The interface is an **output event**, not a callback into N-F:

```rust
PeerSharingOutput::ReceivedPeers(CanonicalPeerList)
PeerSharingOutput::SendPeers(CanonicalPeerList)
PeerSharingOutput::ProtocolError(...)
```

RED/GREEN glue takes the peer list into the operator surface (N-F).

### #3 — LocalStateQuery taxonomy (B with caveat)

The codec is **not** "opaque bytes everywhere." It owns the closed
wire grammar:

- mini-protocol framing
- agency
- query envelope
- query discriminant bytes
- era/version gating
- canonical preservation of payload bytes
- structured decode errors

It does **not** own:

- what the query means
- whether the requested state exists
- how results are computed
- era-specific ledger interpretation
- typed result construction

**Correct framing**: *codec models the closed wire grammar, not the
ledger meaning.* If the codec treated too much as opaque, it would
fail to enforce malformed-input rejection at the right boundary.

### #4 — `TxId` reuse

One canonical type authority for persisted, compared, hashed, or
protocol-visible identities. `ade_network` depends on `ade_types`.

**Byte-authority caveat**: For Cardano transaction IDs, the type
records *which bytes were hashed*. If the protocol-defined hash uses
preserved-original transaction bytes, do not silently recompute from
project-canonical bytes unless equivalence has been proven. Cluster
doc captures this requirement explicitly so S-A6 (tx-submission2)
doesn't trip the trap.

### #5 — Frame corpus location

`corpus/network/`, structured as:

```
corpus/network/
  n2n/
    handshake/  chain_sync/  block_fetch/  tx_submission2/
    keep_alive/  peer_sharing/
  n2c/
    handshake/  local_chain_sync/  local_tx_submission/
    local_state_query/  local_tx_monitor/
```

Each captured frame carries metadata: cardano-node version, network
magic, protocol, mini-protocol version, direction, agency, raw bytes,
expected decode result, expected re-encode bytes.

**Acceptance criterion**: preserved input bytes decode to a typed
value AND re-encode to byte-identical wire bytes where the protocol
demands preservation.

### #6 — Live interop infrastructure

Two gates defined separately:

- **CI smoke interop gate** — pinned Docker, reproducible, runs every CI
- **Manual closure interop gate (CE-N-A-5)** — Docker default, operator-provided peer as supported override; full closure-gate evidence captured

Full closure must not depend only on an operator-provided peer
(evidence wouldn't reproduce). Full live interop suite must not run in
every CI (would be flaky and expensive).

Pinned cardano-node 10.6.2 belongs in evidence metadata. Repo records
the compatibility target explicitly:

```
interop target:
  cardano-node: 10.6.2
  network: preview / preprod / mainnet
  supported protocol versions: <enumerated list>
```

**CE-N-A-5 proof obligation** (lowest acceptable):
1. Handshake version negotiation succeeds/fails deterministically
2. All 11 mini-protocols reject unsupported versions deterministically
3. Captured frames decode and re-encode byte-identically where required
4. Live peer interaction produces expected agency transitions
5. Malformed frames produce canonical structured errors

### #7 — Per-protocol agency types

Even if variants look similar, their types are non-interchangeable.

```rust
enum ChainSyncAgency      { ClientHasAgency, ServerHasAgency, NobodyHasAgency }
enum BlockFetchAgency     { ClientHasAgency, ServerHasAgency, NobodyHasAgency }
enum TxSubmission2Agency  { InitiatorHasAgency, ResponderHasAgency, NobodyHasAgency }
```

Verbose, but compile-time prevents passing a `ChainSyncAgency` where
a `BlockFetchAgency` is expected. No generic transition fn for BLUE
paths:

```rust
// NO
fn transition(agency: GenericAgency, msg: AnyMessage) -> ...

// YES
fn chain_sync_transition(
    state: ChainSyncState,
    version: ChainSyncVersion,
    agency: ChainSyncAgency,
    msg: ChainSyncMessage,
) -> ChainSyncTransitionResult
```

IDD doctrine favors illegal-state-unrepresentability over code
compression.

### Slice-shaping implications

**#1 affects S-A2 through S-A8**: every BLUE mini-protocol state
machine acquires an explicit `version` parameter. Decided before
writing any signature, test vector, or replay-trace schema. **Hard
S-A2 entry requirement**: no BLUE mini-protocol transition may read
selected protocol version from RED session state.

**#6 affects CE-N-A-5**: interop infra must not block early
codec/state-machine slices, but the evidence schema must be designed
now so captured corpus (S-A9) and live interop (S-A10) produce the
same artifact shape — selected versions, frame bytes, decoded
messages, agency transitions, errors, final transcript hash.

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
