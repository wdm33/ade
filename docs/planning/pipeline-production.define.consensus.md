# Consensus record — docs/planning/pipeline-production.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-09 — docs/planning/pipeline-production.define.md @ blob a4b60e69913aa1661ecf984f7e84c54ac96769dd · received a4b60e69913aa1661ecf984f7e84c54ac96769dd

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | S2 | "DUR-009 checks fingerprint continuity, not exactly one WAL entry per successful admission; a duplicate carrying the current prior fingerprint can remain chained. DUR-012 establishes durability before tip advancement, not append cardinality. The merge loses DC-ADMIT-05’s exactly-once requirement." | revised: DC-ADMIT-05 drafted as #27 AdmissionAppendsExactlyOnce (one entry per admission, fatal append failure); S2 rewritten | accepted |
| 2 | #24 | "Requiring diverged and input-not-found outcomes to halt “without passing the offending block to block validity” is circular: those outcomes can arise from validity evaluation. The runner calls admit_via_block_validity before deriving divergence from a rejection." | revised: #24 stops only the epoch-window and undecodable cases before block validity; diverged and input-not-found halt after admission evaluation | accepted |
| 3 | #22 | "An admission outcome and announced tip cannot establish that the local chain is a prefix of the peer’s chain. verdict::derive returns Lagging for any later peer slot, and also for a peer behind or at Origin, without ancestry evidence." | revised: #22 defines lagging as an inconclusive comparison with both slots in the payload and no ancestry claim; departure recorded in S14 | accepted |
| 4 | #8 | "“The first forge’s base … is the state the one bootstrap authority produced” excludes legitimate admissions between bootstrap and the first forge, contradicting VAL-051’s requirement to use the latest admission’s state." | revised: #8 says bootstrap initializes the base and every later admission updates it, admissions before the first forge included | accepted |
| 5 | #11 | "The replay input list omits peer lifecycle events, although coordinator_step consumes them and emits corresponding effects and log events. Identical listed inputs can therefore produce different output streams." | revised: #11 replays the complete ordered coordinator event stream, peer lifecycle events included | accepted |
| 6 | #16 | "A strictly monotone cookie is incompatible with the wire’s finite u16 cookie domain and the implementation’s wrapping_add(1). The prohibition on any log entry also contradicts the pump’s ping/pong stderr diagnostics." | revised: #16 uses a deterministic wrapping 16-bit counter and forbids evidence events, allowing shell diagnostics excluded from evidence and replay | accepted |
| 7 | #5 | "The unconditional claim that no forge output is rejected by decode_block is contradicted by forge.rs tests that successfully forge illegal block-number/previous-hash combinations and then require decode_block to reject them." | revised: #5 requires the forge to reject an illegal header position (VAL-003) before emitting bytes; S15 records it as expected falsified | accepted |
| 8 | #10, #12 | "#10’s restart/replay mechanics and #12’s cross-run chain replay guarantee rely on recorded, ordered canonical inputs, but neither cites A1. #12 also claims reference-governed production semantics without citing A2." | revised: A1 added to #10 and #12, A2 added to #12 | accepted |
| 9 | #25 | "The statement includes protection of key material and sensitive paths at an evidence boundary, but assigns OBS as primary. Secrets protection falls under SEC’s security-first rule." | revised in part: the key-material clause is SEC-025's (which forbids key material in anything logged, persisted, hashed for evidence or replayed), so #25 drops it and depends on SEC-025 (S16); the remaining path clause is host-independence, kept in OBS | accepted |
| 10 | #23 | "Replay equivalence is marked standard even though Doctrine §9 identifies replay equivalence as a surface warranting critical treatment; this line preserves DC-ADMIT-07’s authoritative post-state replay guarantee." | revised: #23 marked critical | accepted |

## Round 2 — 2026-11-09 — docs/planning/pipeline-production.define.md @ blob 39bd881a3d6bc830e7839c38e27a61fb33356e22 · received 39bd881a3d6bc830e7839c38e27a61fb33356e22

Items 1–10: accepted.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 11 | S4 | "VAL-051 and SAF-013 preserve DC-CONS-24’s followed-tip parent equality only outside the forge extensions. Inside either extension, the selected durable parent may differ from the followed peer tip; this is a departure, not a complete merge of the legacy meaning." | revised: S4 records the departure (followed-tip parent equality holds only outside the two extensions) | accepted |
| 12 | #3 | "CN-FORGE-02 explicitly isolates the evaluator from LedgerView, EraSchedule and ChainDepState and requires the caller to supply LeaderScheduleAnswer through the authority path. #3 omits these structural restrictions" | revised: #3 restores the evaluator's dependency restrictions (no ledger view, era schedule, chain-dependent state, clock, storage or shell crate) and the caller-supplied leader-schedule answer from VAL-024's authority | accepted |

## Round 3 — 2026-11-09 — docs/planning/pipeline-production.define.md @ blob 12e6c4e35878680efa4a939c0fc4ecea991153ab · received 12e6c4e35878680efa4a939c0fc4ecea991153ab

Round 2 items 11 and 12: Codex marked both `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-09 — after round 3 @ blob 12e6c4e35878680efa4a939c0fc4ecea991153ab

```text
Define batch — pipeline-production (B10) — consensus: round 3 (docs/planning/pipeline-production.define.consensus.md)

 1. DET  ForgePureOverProducerTick              [critical]  Forge is pure over one producer tick; replay byte-identical, never signs, no keys in corpora.
 2. VAL  ForgeOnlyWhenLeader                    [critical]  Forge only if the validator's own leader check passes; one Praos VRF construction, no TPraos fallback.
 3. SEC  LeaderVerdictCarriesNoKeys             [critical]  Shell proves VRF; core verifies with no keys or ledger view; ineligible verdict carries only a fingerprint.
 4. SEC  ForgeKesWindowFromBoundGenesis         [critical]  KES evolution = slot period − cert start, from bound genesis; out-of-window refused before signing; late results dropped.
 5. VAL  ForgedBlockSingleEncoding              [critical]  One envelope encoder, inverse of the decoder; validator's body-hash recipe; illegal header position refused.
 6. VAL  ForgedTransactionsAdmissibleInOrder    [critical]  Forged transactions re-admitted in snapshot order against the base; never reordered, added or skipped.
 7. SAF  ForgedBlockSelfAccepted                [critical]  Closed outcomes; success only if full validity accepts the exact bytes; failure hands nothing on, node continues.
 8. VAL  ForgeBaseFromBootstrapAuthority        [critical]  Base initialized by the bootstrap authority, updated by every admission; params from the base; nothing synthetic.
 9. SAF  ForgeEpochBoundToActiveAuthority       [critical]  Forge for epoch E uses only E's active view; seed-only past support no-forges; ahead/missing halts.
10. SAF  ParticipantForgeOnSelectedHead         [critical]  Participant: catch-up gate, then forge on live selected durable tip, fenced; sign-time re-check.
11. DET  ProducerCoordinatorReplay              [critical]  Coordinator replays byte-identically over the full ordered event stream; holds no keys.
12. STR  ProduceChainEvolutionLinear            [critical]  Produce mode: linear chain evolution; advances only when validity and self-acceptance agree.
13. SAF  ProduceServesOnlySelfAccepted          [critical]  Produce mode serves only self-accepted blocks, published atomically before the next tick.
14. VAL  ForgedBlocksAcceptedByReference        [critical]  Forged, served blocks are accepted by a reference cardano-node peer. (live evidence)
15. STR  SingleWirePumpPerPeer                  [critical]  One pump per peer, sole producer on its lane; closed events; distinct roll-backward event.
16. VAL  KeepAliveClientWireOnly                [standard]  Keep-alive client: cadence < peer timeout, one outstanding, wrapping cookie, no evidence events.
17. LIV  KeepAliveSurvivesBackpressure          [critical]  Keep-alive keeps running through any downstream stall.
18. ORD  PumpOrderPreservedUnderBackpressure    [critical]  Backpressure delays, never reorders; parked frames bounded by a fixed constant, overflow fails closed.
19. SEC  MultiPeerFairMerge                     [critical]  Multi-peer round-robin merge in configured order; per-peer backpressure; no starvation.
20. LIV  ReconnectTransportOnly                 [critical]  Reconnect only on transport loss with fixed capped backoff; protocol faults and first-dial failures not retried.
21. STR  AdmissionModeSingleEntry               [standard]  Admission mode: one runner; the seed reaches the ledger only via the bootstrap authority's warm start.
22. OBS  AgreementVerdictEvidenceOnly           [critical]  Agreement verdict is closed, pure evidence; lagging is inconclusive, never success.
23. OBS  AdmissionEvidenceBoundAndReplayable    [critical]  One verdict per admission, fingerprint on events, replay reproduces state and verdict.
24. SAF  AdmissionDivergenceHalts               [critical]  Admission mode halts with distinct codes on off-epoch, undecodable, diverged, input-not-found.
25. OBS  ClosedIsolatedEventVocabularies        [standard]  Each mode's evidence log has its own closed vocabulary; no host-dependent content.
26. VAL  SeedImportFailsClosed                  [standard]  Seed importer fails closed on unsupported features; no permissive arm or partial seed.
27. DUR  AdmissionAppendsExactlyOnce            [critical]  Every successful admission appends exactly one log entry; a failed append halts.

R1. SAF-013 ForgeAdmissibilityGatesOnly — weaken: adds #10's participant extension as the second exception to the
    catch-up gate (and replaces the batch-local "#13" with VAL-053). Claim stays unresolved until re-evidence.

Settled in consensus:
 S1. CN-CONS-08 — merged into DUR-012, VAL-009, SEC-019.   S2. DC-ADMIT-05 — drafted as 27.
 S3. DC-CONS-23 — merged into STR-015.   S4. DC-CONS-24 — merged into VAL-051, SAF-013 (departure inside the extensions).
 S5. CN-PROD-01 — merged into SAF-011, EVO-006.   S6. DC-CONS-12 — merged into SEC-011 (departure: reference counter window).
 S7. DC-CONS-11 — drafted as 4 (departure: reference KES rule).   S8. DC-EPOCH-03 — drafted as 9 in its later form.
 S9. Self-acceptance failure: resolved toward a structured failure, node keeps running (reference behaviour).
 S10. CN-NODE-03 production clauses — 8; default protocol parameters retired.   S11. Live acceptance — drafted as 14.
 S12. Import-fingerprint binding — in 23.   S13. CN-FOLLOW-01 / DC-FOLLOW-FORGE-01 — 10, admitted by R1.
 S14. 22's lagging departs from "local chain is a prefix of the peer's".   S15. 5 expected FALSIFIED (illegal positions).
 S16. Key material in logs — SEC-025.

Legacy dispositions (51): 1←DC-CONS-13, 14 · 2←DC-CONS-15, DC-FORGE-01, CN-FORGE-04 · 3←CN-FORGE-02 · 4←DC-CONS-11, CN-PROD-02 · 5←DC-CONS-16, CN-FORGE-03 · 6←DC-LEDGER-12 · 7←CN-CONS-07, CN-FORGE-01 · 8←CN-PROD-03 · 9←DC-EPOCH-03 · 10←CN-FOLLOW-01, DC-FOLLOW-FORGE-01 · 11←DC-PROD-02 · 12←DC-PROD-03 · 13←CN-PROD-04 · 15←CN-PUMP-01, DC-PUMP-01, 02 · 16←DC-PUMP-03 · 17←DC-PUMP-05 · 18←DC-PUMP-06, 07 · 19←DC-PUMP-04 · 20←DC-PUMP-08, 09, 10 · 21←CN-ADMIT-01, 02 · 22←DC-ADMIT-01, 06, 08 · 23←DC-ADMIT-02, 07, 10 · 24←DC-ADMIT-03, 11, 12 · 25←DC-ADMIT-04, DC-PROD-01 · 26←DC-ADMIT-09 · 27←DC-ADMIT-05 · merged CN-CONS-08, DC-CONS-23, DC-CONS-24, CN-PROD-01, DC-CONS-12

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok"; "R1 ok|reject".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all" (one reply, given after B10, B11 and B12 were presented together; it covers all three batches)
