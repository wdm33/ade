# Define — network

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B08 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers the mini-protocols (determinism, agency, agreement with the reference, the N2N and N2C surfaces, version negotiation), serving blocks to peers, the mux session layer, durable-before-tip sync, and how compatibility is evidenced. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 52 legacy IDs (47 assigned, plus serving entries deferred here by the consensus and durability batches) appears on a line or in a settlement.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants that settlements merge into)

## Drafts

```text
#1  DET  MiniProtocolStateMachinesDeterministic  [critical]
    statement:      Each mini-protocol is an explicit state machine with only legal typed transitions, and each transition is a pure function of the prior protocol state, the incoming message, the negotiated protocol version, fixed configuration and recorded explicit inputs (served-chain updates and session events); given the same starting state and the same ordered peer messages, served-chain updates, session events and recorded context, the resulting protocol state and the outbound transcript are identical, and no ambient session state changes the result.
    counterexample: The chain-sync server's reply to RequestNext depends on a session flag set by an earlier, unrelated keep-alive timeout, so two runs over the same transcript reply differently.
    secondary:      [STR]
    scope:          N2N and N2C mini-protocol state machines
    assumptions:    [A1]
    depends_on:     [STR-002]
    locality:       local
    legacy:         [DC-PROTO-01, CN-PROTO-01, DC-PROTO-06, CN-PROTO-02]

#2  SEC  AgencyEnforced  [critical]
    statement:      Every mini-protocol enforces agency strictly: a message from the side that does not hold agency in the current state is rejected deterministically and ends the session. Beyond that runtime check, the node's own code cannot construct a message for a role or agency it does not hold: the serving side's public API can build only server-agency messages, and the receive reducer's public API offers no way to construct or admit a locally originated or client-output event.
    counterexample: A peer sends MsgRollForward while the client holds agency and the node accepts it, or the server-side code emits a client request on a session it serves.
    secondary:      [VAL, SAF]
    scope:          mini-protocol sessions, serve and receive paths
    assumptions:    [A2]
    depends_on:     [#1, EVO-002]
    locality:       local
    legacy:         [CN-PROTO-03, CN-PROTO-06, CN-PROTO-07]

#3  VAL  MiniProtocolTranscriptMatchesReference  [critical]
    statement:      For the same inputs and peer messages, each mini-protocol's messages and state progression match the reference node's for the same negotiated version, checked against transcripts captured from a reference node.
    counterexample: Ade's block-fetch server answers a RequestRange whose start is not on its chain with an empty batch instead of NoBlocks, which a reference client treats as a protocol violation.
    secondary:      []
    scope:          N2N and N2C mini-protocols
    assumptions:    [A2]
    depends_on:     [#1, PRC-003]
    locality:       distributed
    legacy:         [DC-PROTO-02]

#4  VAL  TxSubmissionWireForm  [critical]
    statement:      The tx-submission codec accepts and preserves the reference node's wire form: each transaction id is the era-tagged pair [era index, 32-byte hash], never stripped or guessed, so a requester echoes the exact advertised id; id and transaction sequences are accepted as definite or indefinite-length arrays, a received message re-encodes to exactly its received bytes in whichever form it arrived, and messages Ade originates use the indefinite form the reference node sends.
    counterexample: Ade strips the era tag from an advertised transaction id and requests it back as a bare hash, which the reference node does not recognize.
    secondary:      []
    scope:          TxSubmission2 codec
    assumptions:    [A2]
    depends_on:     [VAL-001, DET-003]
    locality:       local
    legacy:         [DC-PROTO-11]

#5  VAL  FullMiniProtocolSurface  [standard]
    statement:      Ade implements the full node-to-node set (Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing) and the full node-to-client set (Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor), each in the roles the reference node plays and in agreement with #3.
    counterexample: A cardano-cli query over the node-to-client socket fails because LocalStateQuery is not served.
    secondary:      []
    scope:          N2N and N2C mini-protocol coverage
    assumptions:    [A2]
    depends_on:     [#3]
    locality:       local
    legacy:         [DC-PROTO-03, DC-PROTO-04]

#6  EVO  ClosedVersionNegotiation  [critical]
    statement:      The node offers and accepts only an enumerated set of node-to-node and node-to-client protocol versions; negotiation selects the version the reference rule selects and, with no common version, refuses deterministically; node-to-client queries are typed, era-aware and available only in the versions that define them.
    counterexample: A handshake proposing only unknown versions is accepted with a default version, or a LocalStateQuery introduced in a later version is answered on a session that negotiated an earlier one.
    secondary:      [VAL]
    scope:          handshake, node-to-client queries
    assumptions:    [A2]
    depends_on:     [VAL-004, EVO-002]
    locality:       local
    legacy:         [DC-PROTO-05, DC-QUERY-01]

#7  VAL  ServedBytesAreAcceptedBytes  [critical]
    statement:      Block bytes served over block-fetch are byte-identical to the bytes of the accepted block they belong to, never re-encoded, and a header announced over chain-sync is the header of that same accepted block, so the body hash of the served body equals the body-hash field of the announced header.
    counterexample: The server re-encodes a stored block before sending it, so the served body's hash differs from the header a peer received earlier and the peer rejects the block.
    secondary:      [DET]
    scope:          chain-sync and block-fetch servers
    assumptions:    []
    depends_on:     [DET-002, VAL-032]
    locality:       local
    legacy:         [DC-CONS-17, DC-CONS-18]

#8  SAF  ServedChainPublishedAtomically  [critical]
    statement:      A block becomes visible to peers only after it is published to the served chain in one atomic update, so no peer can observe a partially updated served chain; a failed publication shuts the node down fail-closed rather than continuing; and publishing the same sequence of blocks always produces a byte-identical served view, as measured by its fingerprint.
    counterexample: A peer's chain-sync session reads the served chain while a forged block's header is inserted but its body is not, announces the header, and then cannot serve the body.
    secondary:      [DET]
    scope:          served chain
    assumptions:    []
    depends_on:     [#7]
    locality:       transactional
    legacy:         [CN-SNAPSHOT-01, DC-SNAPSHOT-01]

#9  VAL  BlockFetchRangeAllOrNothing  [critical]
    statement:      A block-fetch range request is answered with a batch only if both endpoints and every block between them are on the served chain; otherwise the reply is NoBlocks, never a partial batch, a truncated range or a strict prefix.
    counterexample: A range whose end block was rolled back is answered with the blocks up to the rollback point and BatchDone.
    secondary:      []
    scope:          block-fetch server
    assumptions:    [A2]
    depends_on:     [#8]
    locality:       local
    legacy:         [CN-SNAPSHOT-02]

#10 DET  ServerSessionDeterministic  [critical]
    statement:      The serving side's per-session reducer is a pure function of the negotiated version, the peer's message sequence, the sequence of newly served blocks and the recorded session events, so the same inputs give byte-identical outbound frames. In each state where the server holds agency it returns a reply legal for that state as the reference protocol defines it (IntersectFound or IntersectNotFound after FindIntersect; RollForward, RollBackward or AwaitReply after RequestNext; StartBatch with blocks or NoBlocks after RequestRange) or a structured close; after AwaitReply it waits only until a recorded served-chain or session event makes a RollForward or RollBackward possible, never in an undefined state.
    counterexample: The chain-sync server, holding agency with no new block, neither sends AwaitReply nor closes, and the client times out.
    secondary:      [LIV]
    scope:          chain-sync and block-fetch servers
    assumptions:    [A1, A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [DC-PROTO-07, DC-PROTO-08]

#11 VAL  FindIntersectSetsCursor  [critical]
    statement:      After the chain-sync server answers IntersectFound at a point, its read position is exactly that point: the next RequestNext is answered with the block immediately after it when that block is on the unchanged served chain, and otherwise with the waiting or rollback reply #10 allows; an intersection at Origin sets the position before the chain's first block.
    counterexample: After IntersectFound at the client's tip, the server's next RollForward sends block 0, which the client rejects as an unexpected block number.
    secondary:      [ORD]
    scope:          chain-sync server
    assumptions:    [A2]
    depends_on:     [VAL-005, #10]
    locality:       local
    legacy:         [DC-PROTO-10]

#12 SEC  PerPeerOutboundIsolation  [critical]
    statement:      Outbound messages reach a peer only as typed commands through that peer's own outbound channel and the one session-aware encoder; no raw byte tunnel exists, no code can create a channel for a peer that did not connect, a peer's channel is removed when it disconnects, the channel map iterates in a deterministic order, looking up a missing peer is a structured error, bytes meant for one peer never reach another, and commands for a peer arrive in the order they were sent.
    counterexample: A served block meant for peer A is written to peer B's socket because the outbound map is looked up by a stale index, or two commands for one peer arrive in reverse order.
    secondary:      [ORD]
    scope:          outbound dispatch
    assumptions:    []
    depends_on:     [#10]
    locality:       local
    legacy:         [CN-OUTBOUND-RELAY-01, CN-PEER-OUTBOUND-MAP-01, DC-OUTBOUND-FIFO-01]

#13 STR  SingleSessionAuthorities  [critical]
    statement:      Each layer of the wire session has one authority: one mux frame encoder and decoder pair, one handshake reducer for each of node-to-node and node-to-client, and one session step reducer; no parallel implementation exists.
    counterexample: A second mux frame decoder added for the serve path accepts a frame length the session decoder rejects.
    secondary:      []
    scope:          mux, handshake and session layers
    assumptions:    []
    depends_on:     [VAL-001]
    locality:       local
    legacy:         [CN-SESS-01, CN-SESS-02, CN-SESS-03]

#14 SAF  HandshakeBeforeTraffic  [critical]
    statement:      No mini-protocol frame from a peer is delivered for processing until that session's handshake has been accepted, and this holds by construction: a session still handshaking has no way to produce a delivery effect.
    counterexample: A chain-sync frame arriving in the same read as the handshake proposal is delivered before the version is agreed.
    secondary:      [ORD]
    scope:          session layer
    assumptions:    []
    depends_on:     [#6]
    locality:       local
    legacy:         [DC-SESS-01]

#15 VAL  ClosedMiniProtocolRegistry  [standard]
    statement:      The mini-protocol identifiers the session dispatches on form a closed set; a frame for any other identifier is a peer-fatal error, never ignored or accepted.
    counterexample: A frame for an unknown mini-protocol id is skipped and the session continues.
    secondary:      [EVO]
    scope:          session dispatch
    assumptions:    [A2]
    depends_on:     [#13]
    locality:       local
    legacy:         [DC-SESS-02]

#16 DET  SessionReplayEquivalence  [critical]
    statement:      Two guarantees hold for the session layer. Replay: given the same starting state, negotiated version, inbound byte chunks and recorded timestamps, it produces byte-identical outbound frames and an identical sequence of delivered messages. Fragmentation: any two chunkings of the same inbound byte stream produce the same sequence of delivered messages. Each delivery is exactly one complete CBOR item of one mini-protocol, never one mux frame; incomplete trailing bytes are kept until complete, in a reassembly buffer per mini-protocol indexed by the closed set of accepted mini-protocols, so its membership, lookup and iteration are deterministic; malformed CBOR at an item boundary is a structured error, never dropped or partly accepted; order within each peer and mini-protocol is preserved; and the session layer reads no clock, timestamps entering only as values the caller supplies.
    counterexample: A block-fetch message split across two mux frames is delivered as two partial messages, or keep-alive timing inside the session reducer reads the system clock.
    secondary:      [ORD]
    scope:          session layer
    assumptions:    [A1]
    depends_on:     [STR-002, #13]
    locality:       local
    legacy:         [DC-SESS-03, CN-SESS-04, DC-SESS-06, DC-SESS-05]

#17 SEC  BoundedBackpressure  [critical]
    statement:      Every per-peer and per-mini-protocol channel in the wire session, mux pump, dialer and keep-alive is bounded, and overflow is a fail-fast error, never an unbounded queue or a silent drop.
    counterexample: A peer that sends faster than the node processes fills an unbounded channel until the node runs out of memory, or excess frames are dropped without the session noticing.
    secondary:      [SAF]
    scope:          wire session channels
    assumptions:    []
    depends_on:     [SEC-002]
    locality:       local
    legacy:         [DC-SESS-04]

#18 VAL  OutboundSegmentation  [critical]
    statement:      An outbound mini-protocol payload larger than the mux maximum is split into ordered frames no larger than that maximum, each with the same mini-protocol id and mode, whose payloads concatenate to the original exactly, using the one frame encoder; a payload above the fixed outbound ceiling fails closed.
    counterexample: A 90 KB served block is sent as one oversized mux frame, which the reference node rejects.
    secondary:      []
    scope:          session layer, outbound
    assumptions:    [A2]
    depends_on:     [#13, #16]
    locality:       local
    legacy:         [CN-SESS-05]

#19 DUR  TipAdvancesOnlyAfterDurableAdmit  [critical]
    statement:      The selected tip advances to a block only after that block's bytes and its write-ahead-log entry are durable, and only through the one admission path (decode, header validation, block validity, chain selection); no verdict, follower or diagnostic path, and no direct storage call, moves the live tip. The tip-advance effect can be constructed only after the store-bytes and append-log effects for that block, and the pump that applies effects fails closed on any out-of-order application.
    counterexample: The follower used for a diagnostic stream calls the chain database's advance-tip operation directly, so the live tip moves to a block that was never logged.
    secondary:      [STR]
    scope:          forward sync, continuous relay loop
    assumptions:    [A6]
    depends_on:     [DUR-010, STR-004]
    locality:       transactional
    legacy:         [DC-SYNC-01, DC-SYNC-02]

#20 PRC  CompatibilityOnObservableSurfaces  [standard]
    statement:      Agreement with the reference node is evidenced only on observable surfaces (per-block verdicts, the selected tip hash, block hashes, cardano-cli query results and protocol transcripts) with pinned reference versions and reproducible inputs; comparing Ade's internal ledger fingerprint with a hash of the reference node's serialized state is forbidden, and fingerprint equality is asserted only between Ade's own paths.
    counterexample: A test claims ledger agreement by comparing Ade's state fingerprint with a hash of cardano-node's ExtLedgerState file.
    secondary:      [OBS]
    scope:          compatibility evidence
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       local
    legacy:         [DC-COMPAT-01]
```

## Transitions in scope

- `protocol_step(ProtocolState, Message, Version, Config) -> Result<(ProtocolState, Vec<Message>), ProtocolError>` for each mini-protocol
- `session_step(SessionState, ByteChunk) -> Result<(SessionState, Vec<SessionEffect>), SessionError>`
- `handshake(HandshakeState, Message) -> Result<(HandshakeState, Effects), Refusal>`
- `publish(ServedChain, AcceptedBlock) -> Result<ServedChain, PushError>`

## TCB color hypothesis

- BLUE: `ade_network` codec, handshake, chain-sync, block-fetch, tx-submission, keep-alive, peer-sharing and node-to-client modules, `mux/frame.rs`
- GREEN: `ade_network` session reducer
- RED: `ade_network` mux transport, `ade_runtime` (served chain storage, outbound dispatch, wire pump, forward sync), `ade_node`

## Open questions

- S1. CN-PROTO-04 (transport nondeterminism never reaches authoritative logic) — merged into STR-002 ShellCanonicalizesInputs, which states it.
- S2. CN-PROTO-05 (untrusted network input cannot allocate unbounded authoritative resources before validation) — merged into SEC-002 BoundedPreValidationAllocation.
- S3. DC-PROTO-09 (the receive bridge's output state is byte-identical across replays) — merged into DET-001 ReplayByteIdentity.
- S4. Network topology and peer-selection entries (DC-NET-01, CN-NET-01, CN-NET-02, CN-NET-03, CN-NET-04, OP-NET-01, OP-NET-02, OP-NET-03: a producer reachable only through trusted relays, diverse relay paths, no single peer or cluster dominating, peer selection not starvable) — deferred to B12 resources-release, which holds the operational entries.
- S5. CN-FOLLOW-01, DC-FOLLOW-FORGE-01 (forge only on the selected head; participant forge decisions) — deferred to B10 pipeline-production with the forging entries.
- S6. RO-SYNC-EVIDENCE-01 (the committed sync-evidence manifest schema) — deferred to B11 mithril-evidence with the evidence entries.
```
