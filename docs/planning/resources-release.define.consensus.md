# Consensus record — docs/planning/resources-release.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-09 — docs/planning/resources-release.define.md @ blob d6343de6c108837ec229dcb7d07e6f52e2100aef · received d6343de6c108837ec229dcb7d07e6f52e2100aef

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #1, #4 | "Unconditional admission iff ledger-valid conflicts with bounded admission: a valid transaction arriving at capacity must be shed under #4 but admitted under #1." | revised: #1 is scoped to transactions that pass resource admission (#4); a resource refusal is a distinct reason, never invalidity | accepted |
| 2 | #1 | "“The same function block validity applies” conflates standalone validity with block transaction validity. VAL-009 permits a block’s phase-2-invalid transaction through the collateral path while requiring successful scripts for standalone admission." | revised: #1 uses VAL-009's standalone validity, computed by the same phase-1 and script-evaluation functions block validity uses | accepted |
| 3 | #8; DC-MEM-11 | "Equality of incremental or cached fingerprints does not require using them. Full recomputation on every forward-sync/forge admission, followed by another evidence recomputation, satisfies #8 but violates DC-MEM-11." | revised: #8 requires forward-sync and forge admissions to use the cached component and convergence evidence to reuse the admission's post fingerprint | accepted |
| 4 | #17 | "The merge drops CN-OPERATOR-EVIDENCE-01’s capture command, documented filter, Ade evidence-file reference and acceptance-keyword match. It also drops RO-LIVE-06’s requirement to read the forged hash from the forge artifact without reparsing or recomputing it." | revised: #17 carries the capture command, the display filter as documentation only, the Ade evidence file, the acceptance keyword, and the forged hash read from the forge artifact without recomputing or parsing | accepted |
| 5 | S1 | "SEC-002 bounds individual untrusted inputs and SEC-018 bounds channels; neither requires non-configurable live-feed reassembly/lookahead constants. Configurable finite caps can satisfy both while violating DC-LIVEMEM-01." | revised: DC-LIVEMEM-01 drafted as #20 LiveFeedBuffersClosedConstants; S1 rewritten | accepted |
| 6 | S8 | "VAL-007/VAL-008 require semantic agreement, while #12 gates differential fuzzing mismatches. None explicitly makes accept/reject comparison on authoritative corpora release-blocking." | revised: #12 makes accept/reject agreement on the pinned authoritative corpora a release gate; CN-REL-03 and RO-REL-02 moved onto #12 (S8) | accepted |
| 7 | S10 | "REC-004 establishes replay and crash recovery equivalence, not that post-partition reconciliation derives solely from the recovered canonical chain. An additional operator-driven reconciliation path can violate the legacy entries without violating REC-004." | revised: CN-OPS-01 and OP-OPS-01 drafted as #21 ReconciliationOnlyThroughCanonicalChain (S10) | accepted |
| 8 | S15 | "OperatorPassTranscript permits a transcript with one admitted block and one agreed verdict. It does not preserve RO-LIVE-03/RO-LIVE-05’s 30-minute live window or per-admitted-block comparison log." | revised: RO-LIVE-02, 03 and 05 drafted as #22 LiveFollowWindow (30-minute window, per-block tips and verdict); the per-step tip-equality departure stays in S15 | accepted |
| 9 | S16 | "Closed event vocabularies and handshake/FindIntersect compatibility do not require a live wire-smoke execution, tip observation or clean completion. VAL-005 concerns the server’s FindIntersect handling, not execution of the smoke client." | revised: RO-LIVE-04 drafted as #23 WireSmokePass (S16) | accepted |
| 10 | #13, #14 | "Both statements rely on reference semantics but omit A2: #13 names disagreements with the reference, and #14 requires consensus equivalence with supported predecessors under the reference rules." | revised: A2 added to #13 and #14 | accepted |
| 11 | #5 | "“Within a bounded delay” supplies no bound or method for fixing one; a minutes-long wait is not necessarily a counterexample. A5 bounds peer-message processing but supplies no execution venue or storage-service assumptions for these scheduling guarantees." | revised: #5 restated structurally: validation, selection and persistence run on resources ingress and connection handling cannot occupy or block; counterexample a shared pool | accepted |
| 12 | #7 | "The invariant is standard despite governing a fail-closed resource bound whose exhaustion can terminate authoritative processing; Doctrine §9 identifies this surface as usually critical." | revised: #7 marked critical | accepted |
| 13 | S18 | "“Declared and never enforced” is inaccurate for #9’s legacy entry: OP-MEM-02 is recorded as enforced. “Unsupported” is also not a Doctrine §10 claim status." | revised: S18 keeps OP-MEM-02's recorded enforcement and leaves claim statuses (unresolved, partial, supported, falsified) to the M2 evidence | accepted |

## Round 2 — 2026-11-09 — docs/planning/resources-release.define.md @ blob 64fec09e1b0ebb0bcfa9dd4f2ad703847fcec7a0 · received 64fec09e1b0ebb0bcfa9dd4f2ad703847fcec7a0

Items 1–12: accepted. Item 13: open — "S18 is corrected, but the introduction still says unenforced entries will be recorded as “unsupported or falsified”; “unsupported” remains outside Doctrine §10’s claim statuses." Revised: the introduction now names the §10 statuses and claims none.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 14 | #21, S11 | "#21 forbids any incident-recovery import outside admission, fork choice and durable replay, whereas S11 permits re-bootstrap from a verified source and EVO-005 requires it for incompatible stores." | revised: #21 permits a verified re-bootstrap through STR-013's single bootstrap authority under SEC-015 | accepted |
| 15 | #23; RO-LIVE-04 | "The legacy smoke pass requires FindIntersect(Origin); #23 permits any FindIntersect request, losing that concrete execution requirement without a stated departure." | revised: #23 requires the one FindIntersect to offer Origin | accepted |
| 16 | #17, S13, S17; CN-CONS-06 | "#17 governs evidence construction but never requires a live acceptance run before release." | revised: CN-CONS-06 drafted as #24 LiveAcceptanceReleaseGate (sustained public-venue session; without winning stake the gate is unmet and the claim unresolved, never waived); S13 and S17 rewritten | accepted |

## Round 3 — 2026-11-09 — docs/planning/resources-release.define.md @ blob dc08dee37b724fe8d0ac9e550006d377134acb0c · received dc08dee37b724fe8d0ac9e550006d377134acb0c

Items 13–16: Codex marked all four `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-09 — after round 3 @ blob dc08dee37b724fe8d0ac9e550006d377134acb0c

```text
Define batch — resources-release (B12) — consensus: round 3 (docs/planning/resources-release.define.consensus.md)

 1. VAL  MempoolAgreesWithLedger                [critical]  After resource admission, mempool admits iff standalone validity (VAL-009) accepts; no mempool-only rules.
 2. STR  ClosedTxIngress                        [standard]  Every tx source reduces to one closed ingress event; source never changes the verdict.
 3. DET  MempoolReplay                          [critical]  Same ingress trace over same base → byte-identical mempool states and outcomes.
 4. SEC  BoundedDeterministicAdmission          [critical]  Untrusted work passes fixed bounds first; overload shed deterministically by arrival order.
 5. LIV  AuthoritativeWorkNotStarved            [standard]  Validation, selection, persistence run on resources tx/connection work cannot occupy.
 6. DET  FingerprintIndependentOfStorage        [critical]  Fingerprints independent of UTxO storage backend, iteration order, allocator, representation.
 7. SAF  BoundedUtxoMemory                      [critical]  In-memory UTxO cache and change log bounded by fixed constants; bound never changes outputs.
 8. DET  UtxoCommitmentVersionedIncremental     [critical]  UTxO set commitment versioned; incremental and cached = full recompute; hot paths use the cache.
 9. PRC  OwnedMemoryBelowReference              [standard]  Owned resident memory below the reference node's on the same venue, with no semantic change.
10. SEC  ServeRangeWorkBounded                  [critical]  Block-fetch range work bounded by a fixed constant; oversized ranges fail closed.
11. SEC  NoPeerDiscoveryWhenForging             [critical]  A forging node dials only configured peers, never discovered ones.
12. PRC  DifferentialFuzzingReleaseBlocking     [critical]  Differential fuzzing and corpus accept/reject agreement gate every release.
13. PRC  PermanentRegressionCorpus              [standard]  Every past fork/mismatch input becomes a permanent regression-corpus entry.
14. PRC  MixedVersionEvidenceBeforeRelease      [standard]  Release needs evidence of agreement with supported predecessors running alongside.
15. PRC  UnmaskedCloseGate                      [standard]  Clusters close on the real exit status of an unmasked full test run; all consumers audited.
16. LIV  ForgeWithinSlotDeadline                [standard]  Forge, self-accept and hand-off finish within the slot deadline.
17. PRC  PeerAcceptanceEvidenceCorrelated       [critical]  Peer-acceptance manifest: one hash-primary correlator, allow-listed log lines, raw log hash-bound.
18. PRC  RehearsalNonPromotable                 [standard]  Private rehearsal uses the same path, is labelled rehearsal, never counts as acceptance.
19. PRC  SyncLiveLegWitnessed                   [standard]  Snapshot-to-tip sync claim needs an operator-witnessed two-reference-node live run.
20. SEC  LiveFeedBuffersClosedConstants         [critical]  Live-feed buffers bounded by closed constants no option can change; overflow drops the peer.
21. REC  ReconciliationOnlyThroughCanonicalChain [critical]  Reconciliation only via the node's own canonical-chain paths or verified re-bootstrap.
22. PRC  LiveFollowWindow                       [standard]  Live follow pass: ≥30 continuous minutes, per-block tips and verdicts.
23. PRC  WireSmokePass                          [standard]  Wire-smoke pass: handshake, FindIntersect(Origin), peer tip, closed vocabulary, clean exit.
24. PRC  LiveAcceptanceReleaseGate              [critical]  Block production claimed only with sustained public-venue peer-acceptance evidence; never waived.

Reinterpretations (replace batch-local "#N" with the IDs named; meaning unchanged):
R1. DET-005 StructuredDeterministicFailures
R2. VAL-006 SigningMatchesReference
R3. SEC-008 WitnessesBindTransaction
R4. SAF-004 AtMostOneActivationPerEpoch
R5. LIV-001 BoundedRefold
R6. LIV-003 PartitionConvergence
R7. SAF-009 CandidateViewHashVerified
R8. REC-004 CrashRecoveryReplayEquivalent
R9. VAL-044 FullMiniProtocolSurface
R10. VAL-047 FindIntersectSetsCursor
R11. VAL-053 SingleProducerForgeBase
R12. SEC-020 RollbackTargetFromStoredPoint
R13. LIV-007 ReconnectResumesFromDelivered
R14. VAL-055 IntersectionCandidatesMatchReference

Settled in consensus:
 S1. DC-LIVEMEM-01 — 20.   S2. Inbound connection restriction is deployment (firewall/relays); 11 carries outbound.
 S3. Relay diversity — excluded (operator deployment).   S4. No peer dominates — DET-012, LIV-003, MultiPeerFairMerge.
 S5. CN-NET-04 — MultiPeerFairMerge.   S6. Three-tier peer governor — excluded (never designed).
 S7. Verdict stability under resubmission — DET-001.   S8. Corpus accept/reject release gate — 12.
 S9. Ecosystem fault threshold — excluded (not decidable within Ade).   S10. Reconciliation — 21.
 S11. Emergency procedures — excluded (re-bootstrap is the only recovery).   S12. Incident reconstruction — DET-001, REC-004.
 S13. CN-CONS-06 — 24; RO-LIVE-01 — ForgedBlocksAcceptedByReference.   S14. OP-OPS-04 — SEC-006, SEC-001, SEC-005, ForgeKesWindowFromBoundGenesis.
 S15. Live follow passes — 22 (departure: no per-step tip equality).   S16. Wire smoke — 23.
 S17. Deferred release criteria — 24, 17, 19.   S18. No enforcement claimed; M2 sets claim statuses.   S19. R1–R14 meaning-preserving.

Legacy dispositions (57): 1←DC-MEM-01, CN-MEM-04 · 2←DC-MEM-03 · 3←DC-MEM-04 · 4←CN-MEM-01, DC-MEM-02, CN-MEM-03 · 5←CN-MEM-02, OP-MEM-01 · 6←DC-MEM-05, 06, 08, 09 · 7←DC-MEM-07 · 8←DC-MEM-10, 11 · 9←OP-MEM-02 · 10←DC-SERVEMEM-01 · 11←CN-NET-01, OP-NET-01 · 12←CN-TEST-01, RO-TEST-01, CN-REL-03, RO-REL-02 · 13←CN-TEST-02, RO-TEST-02 · 14←CN-REL-01, RO-REL-01 · 15←RO-CLOSE-01 · 16←OP-OPS-05 · 17←RO-LIVE-06, CN-OPERATOR-EVIDENCE-01 · 18←CN-REHEARSAL-FIDELITY-01 · 20←DC-LIVEMEM-01 · 21←CN-OPS-01, OP-OPS-01 · 22←RO-LIVE-02, 03, 05 · 23←RO-LIVE-04 · 24←CN-CONS-06 · merged CN-NET-03, OP-NET-03, CN-NET-04, CN-TEST-03, RO-TEST-03, CN-OPS-03, OP-OPS-03, RO-LIVE-01, OP-OPS-04 · excluded DC-NET-01, CN-NET-02, OP-NET-02, CN-REL-02, RO-REL-03, CN-OPS-02, OP-OPS-02

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok"; "R1-R14 ok".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all" (one reply, given after B10, B11 and B12 were presented together; it covers all three batches)
