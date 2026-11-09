# PHASE4-N-S — Invariants sketch (bridges + bounty acceptance leg)

> **Concept.** Close the two named bridges that N-R deferred so
> the **block-production acceptance leg** of the bounty is
> reachable. (1) KES-signs-real-unsigned-header — `run_real_forge`
> step 3 currently signs a placeholder; the real Praos protocol
> requires KES to sign the canonical unsigned-header CBOR
> pre-image. (2) MuxPump outbound-relay — `dispatch_server_frame_event`
> computes response bytes but cannot transmit them back to
> peers. (3) Operator-pass paired evidence (private testnet
> first; docker preprod as a separate bounty-facing
> strengthening).
>
> **Scope discipline:** N-S proves **block production +
> acceptance over N2N block-fetch**. It does **NOT** close
> the TxSubmission2 leg, the mempool-inclusion leg, or the
> private-testnet-two-Haskell-node leg. Those are separate
> bounty surfaces tracked under their own rules.

## §1 What must always be true (I-rules)

- **I1.** The KES signature in a forged block's header is
  over the **canonical unsigned-header CBOR pre-image** —
  the bytes that would be the encoded `ShelleyHeaderBody`
  with the KES signature field omitted (per the Praos
  protocol spec). Producer-side (forge) and validator-side
  (`header_validate`) use the **same** BLUE recipe — single
  source of truth.
- **I2.** The unsigned-header pre-image is a pure
  deterministic function of `(slot, block_number, prev_hash,
  vrf_vk, vrf_proof, vrf_output, opcert, kes_period,
  hot_vkey, body_hash, body_size, protocol_version)`. No
  ambient state, no clock, no nondeterminism.
- **I3.** When `run_real_forge` step 6 (`self_accept`)
  returns `Accepted` against a KES-signed-real-header block,
  the resulting `ForgeSucceeded` artifact's bytes pass
  cardano-node's own header+body validator (cross-impl
  claim — block-production acceptance leg precondition).
- **I4.** `dispatch_server_frame_event` reply bytes traverse
  the existing `MuxPump` session-aware outbound encoder
  before reaching the peer's TCP socket. No direct socket
  write from `produce_mode`. **OutboundCommand is the
  canonical surface** — typed mini-protocol messages, not
  pre-encoded `Vec<u8>`.
- **I5.** Outbound relay is per-peer: an `OutboundCommand`
  destined for `PeerId(p)` is delivered to the `MuxPump`
  task owning `PeerId(p)`'s TCP socket, never another
  peer's. Keyed by `BTreeMap<PeerId, Sender>`.
- **I6.** Bounty-facing paired evidence:
  `Ade evidence.jsonl` carries `BlockForged H` AND the
  cardano-node peer log captured raw via
  `docker logs cardano-node-preprod 2>&1` (or equivalent
  for private testnet) contains a line with the same
  `H` + one of `BlockAccepted` / `AddedToCurrentChain`.
  Committed under
  `docs/clusters/PHASE4-N-S/CE-N-S-LIVE_YYYYMMDD-<short_commit>.{jsonl,log,toml}`
  with the closed manifest schema from N-R DQ-C4, extended
  with `peer_log_capture_command`, `peer_log_filter`, and
  `peer_log_file_sha256` (per correction 6).

## §2 What must never be possible (N-rules)

- **N1.** No KES signature computed over arbitrary bytes.
  `kes_sign_header` accepts only `&UnsignedHeaderPreImage`
  (branded newtype); the type system mechanically rejects
  any other byte sequence. The only constructor for
  `UnsignedHeaderPreImage` is the canonical recipe function.
- **N2.** No two distinct pre-image recipes. The recipe is
  one BLUE function used by both `forge_block` (producer-
  side) and `verify_header_kes` (validator-side). Mechanical
  grep gate enforces single call site for the recipe.
- **N3.** No `ForgeSucceeded` whose KES signature fails to
  verify against `(hot_vkey, unsigned_header_pre_image)` —
  covered structurally by `self_accept` step 6 calling the
  validator's `verify_header_kes`.
- **N4.** No outbound bytes from `produce_mode` reach a
  peer's TCP socket via a path other than `MuxPump`.
  `produce_mode` MUST NOT hold an `mpsc::Sender<Vec<u8>>`
  writing directly into `MuxTransportHandle::outbound`. The
  only outbound API is
  `peer_outbound.get(&peer_id)?.send(OutboundCommand { ... })`.
- **N5.** No cross-peer leakage. The per-peer outbound map
  is `BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>`;
  lookup failure is structured (`UnknownPeer` /
  `PeerOutboundMissing`).
- **N6.** No fabricated bounty evidence. The peer log file
  is the **raw captured output** of the documented capture
  command; the pair manifest carries `peer_log_file_sha256`
  matching the committed file's hash. No hand-edited
  entries. The grep filter is documented in the manifest
  (`peer_log_filter`), not applied destructively to the
  captured file.
- **N7.** No claim of cardano-node-acceptance without a peer
  log line literally containing the Ade-forged block hash +
  an acceptance keyword. The pair manifest's
  `acceptance_keyword_match` field is checked in CI: fails
  if absent from the captured log.
- **N8.** No untyped pre-encoded `Vec<u8>` traverses the
  outbound-relay channel. The channel carries
  `OutboundCommand` exclusively; MuxPump's session-aware
  encoder is the sole producer of the wire-byte stream.

## §3 Determinism (D-rules)

- **D1.** `unsigned_header_pre_image(slot, block_number,
  prev_hash, vrf_data, opcert, kes_period, hot_vkey,
  body_hash, body_size, protocol_version) ->
  UnsignedHeaderPreImage` — pure BLUE function. Same inputs
  → byte-identical pre-image bytes.
- **D2.** Given the same KES signing key + KES period +
  pre-image bytes, `kes_sign_header(...) -> KesSignature` is
  deterministic (carry-forward from Ade's existing Sum6Kes
  implementation; the `KesSecret::kes_sign_at` primitive
  already has this property).
- **D3.** `OutboundCommand → encoded mini-protocol bytes`
  is a deterministic encoding step done inside `MuxPump`'s
  session-aware encoder (the same encoder used today for
  internally-emitted `SessionEffect::SendBytes`).
- **D4.** Per-peer outbound channel preserves FIFO order:
  responses queued for `PeerId(p)` in order O₁, O₂, ..., Oₙ
  arrive at the peer's TCP socket in the same order.

## §4 Replay equivalence (R-rules)

- **R1.** Two `run_real_forge` invocations against the same
  canonical inputs (including KES seed + period + epoch
  nonce) produce byte-identical forged-block bytes —
  including the KES signature. Strengthens DC-FORGE-01
  under real KES-signs-real-header.
- **R2.** Two operator-pass runs against the same controlled
  network state (private testnet with the operator's stake)
  produce byte-identical Ade `ProducerLogEvent` streams when
  filtered to the replayable vocabulary. Strengthens
  DC-PROD-02 under real bounty load.

## §5 State transitions in scope

```rust
// BLUE branded type — only constructor is the canonical recipe.
pub struct UnsignedHeaderPreImage(Vec<u8>);

// BLUE recipe (canonical; used by forge AND verify).
pub fn unsigned_header_pre_image(
    slot: SlotNo,
    block_number: BlockNo,
    prev_hash: Hash32,
    vrf_vk: VrfVerificationKey,
    vrf_proof: VrfProof,
    vrf_output: VrfOutput,
    opcert: &OperationalCert,
    kes_period: KesPeriod,
    hot_vkey: [u8; 32],
    body_hash: Hash32,
    body_size: u32,
    protocol_version: ProtocolVersion,
) -> UnsignedHeaderPreImage;

// RED KES signing accepts the branded type only.
pub fn kes_sign_header(
    sk: &mut KesSecret,
    period: KesPeriod,
    preimage: &UnsignedHeaderPreImage,
) -> Result<KesSignature, ShellSignError>;

// MuxPump extension: typed outbound commands only.
pub enum OutboundCommand {
    ChainSync {
        peer: PeerId,
        msg: ChainSyncServerMsg,
    },
    BlockFetch {
        peer: PeerId,
        msg: BlockFetchServerMsg,
    },
    ClosePeer {
        peer: PeerId,
        reason: CloseReason,
    },
}

pub struct MuxPump {
    // ... existing fields ...
    pub outbound_relay: Option<mpsc::Receiver<OutboundCommand>>,
}

// Shared per-peer outbound map.
pub type PerPeerOutbound = Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>;

// dispatch_server_frame_event extended.
fn dispatch_server_frame_event(
    event: &OrchestratorEvent,
    peers_state: &mut ServerPeerStates,
    served_chain_view: &ServedChainView,
    peer_outbound: &PerPeerOutbound,
) -> Result<usize, DispatchError>;

// Closed dispatch error surface.
pub enum DispatchError {
    UnknownPeer { peer: PeerId },
    PeerOutboundMissing { peer: PeerId },
    ReducerError(/* existing variants */),
    SendFailure { peer: PeerId },
}
```

### Construction sequence (per correction 5)

The body-size chicken-and-egg from OQ2 is solved by the
following **permitted sequence** inside `forge_block` (or
the refactored bridge):

1. Construct unsigned body bytes (tx_bodies, witness_sets,
   metadata, invalid_txs buckets).
2. Compute `body_hash` = `blake2b_256(...)` over the buckets.
3. Compute `body_size` = length of the encoded body byte
   sequence.
4. Construct unsigned `ShelleyHeaderBody` (with placeholder
   `KesSignature` field — or with a slot for it).
5. Compute `unsigned_header_pre_image(...)` from the
   canonical inputs (DOES NOT include the KES signature
   itself; the recipe encodes `ShelleyHeaderBody` minus the
   signature field).
6. `kes_sign_header(sk, period, &preimage) -> KesSignature`.
7. Assemble signed header by inserting the KES signature
   into the prepared slot.
8. Assemble `ShelleyBlock { header, body }`.
9. `self_accept(block_bytes, ledger, chain_dep, schedule,
   view)` runs full validation including
   `verify_header_kes(unsigned_header_pre_image, hot_vkey,
   kes_signature)`.

**Discipline:** if `forge_block`'s current internals cannot
expose this sequence cleanly, N-S-A refactors it. Do NOT
patch by signing a placeholder, signing a partially
assembled block, or signing the encoded full header (which
would include the signature field itself — circular).

## §6 TCB color hypothesis (corrected)

- **BLUE — authoritative core.**
  - `unsigned_header_pre_image` recipe.
  - `UnsignedHeaderPreImage` branded type.
  - `verify_header_kes` (already BLUE; carry-forward).
  - `ChainSyncServerMsg`, `BlockFetchServerMsg` typed
    message enums (already BLUE in ade_network's codec).
- **GREEN — deterministic glue.**
  - `OutboundCommand` enum + the per-command encode step
    inside MuxPump (the encode is pure CBOR; MuxPump's
    session-aware wrapper is RED but the encode itself is
    GREEN).
  - `dispatch_server_frame_event` body — composes BLUE
    reducer output → `OutboundCommand` enqueue.
- **RED — shell.**
  - `MuxPump`'s outbound-relay tokio task (polls receiver,
    encodes via session, writes to transport).
  - `PerPeerOutbound` shared map + `Arc<RwLock<_>>` handle
    (live runtime capability; RED state, not replay value).
  - Operator-pass evidence capture (docker logs, file I/O,
    real network).
- **Color rule:** RED MuxPump may not decide *what* to
  send; that's the BLUE/GREEN reducer's authority. RED's job
  is serializing already-decided OutboundCommands.

## §7 Pre-flight proof obligations (not design choices)

These have one right answer captured before the relevant
slice's implementation begins. Bundled into N-S-A's first
slice with the hard rule "if it bloats, split into
`N-S-PREFLIGHT`."

| ID | What | Captured under | Consumed by |
|---|---|---|---|
| **OQ1** | Audit `ade_ledger::block_validity::header_input` — does it already expose an unsigned-header pre-image helper, or is the recipe inlined in `verify_header_kes`? | A1 slice doc | N-S-A A2 (recipe extraction) |
| **OQ2** | Audit `forge_block`'s internal sequence — does it construct the body + body_hash + body_size before the KES signature, or does it depend on the signature being present in the tick? | A1 slice doc | N-S-A A2 / A3 (refactor scope) |
| **OQ-S-A** | **Reference fixture proof:** capture a real Shelley/Conway block from the docker preprod chain (or extract from the existing block corpus); decode the header; extract or reconstruct the unsigned-header pre-image bytes; verify that Ade's `unsigned_header_pre_image(...)` output **matches the reference bytes byte-for-byte** before signing integration lands. Without this fixture, the "single source of truth" claim is unverified. | `crates/ade_ledger/tests/fixtures/unsigned_header_preimage/` | N-S-A A2 + A3 |
| **OQ-S-B** | Audit existing `MuxPump::run` — what's the exact `tokio::select!` shape after adding the outbound-relay receiver? Confirm no double-mut-borrow of `transport.outbound`. | B1 slice doc | N-S-B B2 |

## §8 Cluster scope (proposal — 3 sub-clusters, 4 phases total)

### N-S-A — KES-signs-real-unsigned-header bridge

**Closes:** I1, I2, I3, N1, N2, N3, D1, D2, R1.

- A1 — planning + 3 candidate registry entries declared
  (`CN-KES-HEADER-01`, `DC-KES-HEADER-01`,
  `CN-PREIMAGE-FIXTURE-01`) + OQ1 + OQ2 + OQ-S-A pre-flight
  capture.
- A2 — BLUE module
  `ade_ledger::block_validity::unsigned_header_pre_image`
  with `UnsignedHeaderPreImage` branded newtype + the canonical
  recipe. Refactor `verify_header_kes` to consume the same
  recipe (single source of truth). Reference fixture
  byte-identity test.
- A3 — Refactor `forge_block` (or thread a new bridge
  function) so the construction sequence in §5 holds:
  body → body_hash → body_size → header pre-image → KES
  sign → signed header. Replace `run_real_forge` step 3's
  placeholder with the real `kes_sign_header(&preimage)`
  call. Self-accept now passes against the synthetic-stake
  corpus (`full_stake_answer_reaches_self_accept_and_rejects`
  inverts to `…_and_accepts`).
- A4 — Integration tests + sub-cluster close: 4-variant
  branch coverage now includes `ForgeSucceeded` reachable
  end-to-end. Flip the 3 N-S-A rules + record
  strengthenings (`CN-FORGE-01`, `DC-CONS-18`).

### N-S-B — MuxPump outbound-relay extension

**Closes:** I4, I5, N4, N5, N8, D3, D4.

- B1 — planning + 3 candidate registry entries declared
  (`CN-OUTBOUND-RELAY-01`, `CN-PEER-OUTBOUND-MAP-01`,
  `DC-OUTBOUND-FIFO-01`) + OQ-S-B audit + `OutboundCommand`
  enum design locked.
- B2 — `OutboundCommand` enum + `MuxPump::outbound_relay`
  field + `tokio::select!` integration (poll receiver
  alongside `transport.inbound`; route OutboundCommands
  through the existing session-aware encoder). `PerPeerOutbound`
  shared map populated by `run_per_peer_session` on
  PeerConnected; cleared on PeerDisconnected.
- B3 — `dispatch_server_frame_event` extended to consume
  `&PerPeerOutbound`; reducer outputs converted to
  `OutboundCommand::{ChainSync,BlockFetch}` and enqueued
  through the per-peer sender. Integration test: synthetic
  dialer peer sends `RequestRange`, server replies via
  outbound-relay, dialer receives byte-identical bytes.
- B4 — Sub-cluster close: flip the 3 N-S-B rules + record
  strengthening on `CN-PROD-01` (per-peer dispatch
  closure clears its remaining `open_obligation`).

### N-S-C — Operator-pass paired evidence

**Closes:** I6, N6, N7, R2; strengthens / narrows (NOT
blindly flips) `CN-CONS-06` and `RO-LIVE-01`.

- C1 — **Hermetic private-testnet pass.** Operator controls
  stake on a private testnet (single-pool topology;
  documented bring-up). Run `ade_node --mode produce`;
  capture Ade evidence + peer log; commit paired manifest.
  This proves the bridge end-to-end without depending on
  external faucet/delegation. **C1's success is what flips
  the bridge-closure registry entries** (e.g.,
  `CN-FORGE-01.open_obligation` cleared, `CN-PROD-01.open_obligation`
  cleared).
- C2 — **Preprod operator pass (bounty-facing
  strengthening).** Once preprod stake is provisioned, run
  the same pass against docker `cardano-node-preprod`.
  Capture paired evidence. **C2's success is the bounty-
  facing strengthening for `CN-CONS-06` and `RO-LIVE-01`**
  — their existing partial/enforced status either narrows
  (open_obligation reduced) or flips fully `enforced` only
  if all remaining sub-items are also covered.
- C-close — Cluster-level close. Strengthen / narrow the
  release rules; document remaining bounty surfaces (txsub
  + N2C + multi-node) as separate-cluster work. No
  blanket "enforced" flip without an artifact for each
  open obligation.

## §9 Honest framing of bounty surface (per correction 1)

The bounty acceptance test has multiple surfaces:

| Surface | Closure status after N-S |
|---|---|
| N2N block-fetch — Ade-forged block accepted by cardano-node | **N-S target** — closes via C1/C2 |
| N2C block-fetch (local chain-sync to a downstream client) | Out of scope; separate cluster |
| TxSubmission2 → mempool → block inclusion | Out of scope; separate cluster |
| Private-testnet two-Haskell-node topology | Out of scope; C1 uses a single private testnet pass, NOT the two-Haskell-node leg |

The runbook explicitly states: N-S proves the
**block-production acceptance leg only**. Empty-block
forging is the scope. TxSubmission/mempool/N2C/multi-node
remain open obligations on their respective rules.

## §10 Out of scope (deferred to future clusters)

- Multi-peer concurrent forge load.
- Mlocked KES memory.
- Hot-key KES rotation across periods.
- TxSubmission2 / mempool / block-inclusion path.
- N2C local-chain-sync / local-tx-submission surfaces.
- Private-testnet two-Haskell-node bounty leg (N-S-C1 is
  Ade + one Haskell peer, not two-Haskell-peer topology).

## §11 References

- N-R cluster close: [[project-phase4-n-r-closed]].
- N-R-A close: [[project-phase4-n-r-a-closed]].
- N-R-B close: [[project-phase4-n-r-b-closed]].
- N-R-C close: this commit's predecessor (HEAD `c02aefc`).
- Bounty: [[project-bounty-requirements]].
- Doctrine:
  - [[feedback-hard-closure-gates]] — flips happen against
    evidence, not against scope claims.
  - [[feedback-proof-discipline]] — OQ-S-A reference
    fixture is a proof obligation, not an assumption.
  - [[feedback-shell-must-not-overstate-semantic-truth]] —
    N-S proves the block-production acceptance leg; does
    NOT close TxSubmission/mempool/N2C/multi-node legs.
  - [[feedback-fail-closed-validation]] — UnsignedHeaderPreImage
    is the branded gate; arbitrary bytes structurally
    rejected.
  - [[reference-local-preprod-docker-cardano-node]] — C2
    target for the preprod evidence leg.
  - [[feedback-bounded-smoke-slices]] — C1 (private
    testnet) and C2 (preprod) are distinct evidence legs,
    not redundant smokes.
