# Consensus record — docs/planning/network.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/network.define.md @ blob ad57d388243847b76aecd894f8b72cc904ae98a3 · received ad57d388243847b76aecd894f8b72cc904ae98a3

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #4 | "Accepting definite arrays but always encoding indefinite arrays contradicts DET-003 RoundTripIdentity" | revised: a received message re-encodes to its received bytes in whichever form it arrived; only messages Ade originates use the indefinite form | open |
| 2 | #1 | "Identical peer transcripts alone cannot guarantee identical outbound transcripts: different served-chain updates legitimately change RequestNext replies." | revised: equivalence holds the starting state, ordered messages, served-chain updates, session events and recorded context equal | open |
| 3 | #10 | "excludes legal IntersectFound/IntersectNotFound replies and forbids the legitimate MustReply wait after AwaitReply" | revised: legal replies listed per state; after AwaitReply the server waits only until a recorded served-chain or session event allows a reply; cites A2 | open |
| 4 | #8 | "CN-SNAPSHOT-01 requires fail-closed shutdown on PushError … DC-SNAPSHOT-01 also requires a byte-identical served view" | revised: a failed publication shuts down fail-closed; the same sequence gives a byte-identical served view by fingerprint | open |
| 5 | #2 | "“Accepts only peer-originated events” permits constructing those events and rejecting them at runtime, losing the structural guarantee." | revised: the receive reducer's public API offers no way to construct or admit such events, distinct from the runtime agency check | open |
| 6 | #12 | "additionally requires disconnect removal, deterministic map iteration and structured lookup failures" | revised: all three carried | open |
| 7 | #16 | "conflates replaying identical chunk sequences with equivalence across different chunkings" | revised: separate replay (same state, version, chunks and timestamps) and fragmentation (different chunkings, same delivered messages) guarantees | open |
| 8 | #4 criticality | "governs byte preservation and serialization identity but is marked standard" | revised: critical | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/network.define.md @ blob d510ecdcd435a1bbac0878c0e80f3db433967946 · received d510ecdcd435a1bbac0878c0e80f3db433967946

Round 1 items 1–8: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 9 | #2 family | "CN-PROTO-06 explicitly defends against peer impersonation and protocol confusion; its agency boundary is security-related" | revised: SEC primary, VAL and SAF secondary | open |
| 10 | #11 | "Unconditionally requiring the next RequestNext to return the successor contradicts #10 when the intersection is the current tip" | revised: the position equals the intersection; the successor is served when on the unchanged served chain, otherwise #10's waiting or rollback reply | open |
| 11 | #14 | "DC-SESS-01 requires pre-handshake delivery to be structurally unrepresentable." | revised: a handshaking session has no way to produce a delivery effect | open |
| 12 | #16 | "CN-SESS-04 requires closed-sum-indexed per-protocol reassembly storage with deterministic membership, lookup and iteration." | revised: the reassembly buffer is indexed by the closed set of accepted mini-protocols, with deterministic membership, lookup and iteration | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/network.define.md @ blob bb95dac43f342cab524cf86e00ea210368cce5bf · received bb95dac43f342cab524cf86e00ea210368cce5bf

Round 2 items 9–12: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 13 | #19 | "DC-SYNC-01 additionally requires AdvanceTip to be structurally constructible only after StoreBlockBytes and AppendWal, and the RED pump to fail closed on out-of-order application." | revised: #19 adds both, the construction-order rule for the tip-advance effect and the pump failing closed on out-of-order application | open |

Outcome: continue

## Round 4 — 2026-11-08 — docs/planning/network.define.md @ blob c3fe90cf02d423d2852549e2071ee98cfcb44650 · received c3fe90cf02d423d2852549e2071ee98cfcb44650

Round 3 item 13: Codex marked it `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-08 — after round 4 @ blob c3fe90cf02d423d2852549e2071ee98cfcb44650

```text
Define batch — network (B08) — consensus: round 4 (docs/planning/network.define.consensus.md)

 1. DET  MiniProtocolStateMachinesDeterministic  [critical]  Mini-protocols are typed state machines; same inputs and recorded events → same state and outbound transcript.
 2. SEC  AgencyEnforced                          [critical]  Out-of-agency messages end the session; the node cannot even construct a message for a role it doesn't hold.
 3. VAL  MiniProtocolTranscriptMatchesReference  [critical]  Messages and state progression match captured reference-node transcripts per negotiated version.
 4. VAL  TxSubmissionWireForm                    [critical]  Era-tagged tx ids; definite or indefinite arrays accepted; received messages re-encode to their exact bytes.
 5. VAL  FullMiniProtocolSurface                 [standard]  The full N2N set and the full N2C set, in the reference node's roles.
 6. EVO  ClosedVersionNegotiation                [critical]  Only enumerated versions; the reference selection rule; deterministic refusal; version-gated N2C queries.
 7. VAL  ServedBytesAreAcceptedBytes             [critical]  Served block bytes are the accepted block's bytes; announced headers belong to the same block.
 8. SAF  ServedChainPublishedAtomically          [critical]  Blocks become visible only via one atomic publication; failure shuts down; same sequence → same view.
 9. VAL  BlockFetchRangeAllOrNothing             [critical]  A range is served whole or answered NoBlocks; never partial.
10. DET  ServerSessionDeterministic              [critical]  The serving reducer is pure; in each agency state it returns a reply legal for that state, or waits only on a recorded event.
11. VAL  FindIntersectSetsCursor                 [critical]  After IntersectFound the read position is that point; the next reply is its successor or #10's wait/rollback.
12. SEC  PerPeerOutboundIsolation                [critical]  Typed per-peer outbound channels; no cross-peer bytes; FIFO; removed on disconnect; deterministic map.
13. STR  SingleSessionAuthorities                [critical]  One mux frame codec, one handshake reducer per side, one session step reducer.
14. SAF  HandshakeBeforeTraffic                  [critical]  No frame is delivered before the handshake is accepted; a handshaking session cannot even produce a delivery.
15. VAL  ClosedMiniProtocolRegistry              [standard]  An unknown mini-protocol id is peer-fatal, never skipped.
16. DET  SessionReplayEquivalence                [critical]  Same inputs → same frames and deliveries; any chunking → same deliveries; whole CBOR items; no clock.
17. SEC  BoundedBackpressure                     [critical]  Every wire channel is bounded; overflow is a fail-fast error, never unbounded or a silent drop.
18. VAL  OutboundSegmentation                    [critical]  Oversized payloads are split into ordered maximal frames that concatenate exactly; above the ceiling fails closed.
19. DUR  TipAdvancesOnlyAfterDurableAdmit        [critical]  The tip moves only via the one admission path after bytes and log entry are durable; enforced by construction order.
20. PRC  CompatibilityOnObservableSurfaces       [standard]  Agreement is evidenced on observable surfaces only; never Ade fingerprint vs Haskell state hash.

Settled in consensus:
 S1. CN-PROTO-04 — merged into STR-002.   S2. CN-PROTO-05 — merged into SEC-002.   S3. DC-PROTO-09 — merged into DET-001.
 S4. Topology and peer-selection entries (DC-NET-01, CN-NET-01–04, OP-NET-01–03) — deferred to B12 with the operational entries.
 S5. CN-FOLLOW-01, DC-FOLLOW-FORGE-01 — deferred to B10.   S6. RO-SYNC-EVIDENCE-01 — deferred to B11.

Legacy dispositions (52): 1←DC-PROTO-01, CN-PROTO-01, DC-PROTO-06, CN-PROTO-02 · 2←CN-PROTO-03, 06, 07 · 3←DC-PROTO-02 · 4←DC-PROTO-11 · 5←DC-PROTO-03, 04 · 6←DC-PROTO-05, DC-QUERY-01 · 7←DC-CONS-17, 18 · 8←CN-SNAPSHOT-01, DC-SNAPSHOT-01 · 9←CN-SNAPSHOT-02 · 10←DC-PROTO-07, 08 · 11←DC-PROTO-10 · 12←CN-OUTBOUND-RELAY-01, CN-PEER-OUTBOUND-MAP-01, DC-OUTBOUND-FIFO-01 · 13←CN-SESS-01, 02, 03 · 14←DC-SESS-01 · 15←DC-SESS-02 · 16←DC-SESS-03, CN-SESS-04, DC-SESS-06, DC-SESS-05 · 17←DC-SESS-04 · 18←CN-SESS-05 · 19←DC-SYNC-01, 02 · 20←DC-COMPAT-01 · merged 3 · deferred 11

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"
