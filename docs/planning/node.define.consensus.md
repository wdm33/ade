# Consensus record — docs/planning/node.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/node.define.md @ blob f506d626e161bc687b7307319d959ce9de6a4f82 · received f506d626e161bc687b7307319d959ce9de6a4f82

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | S3 | "SEC-006 and SEC-001 do not cover CN-NODE-03’s all-or-nothing forge activation, rejection of partial key sets, non-fabricated pool identity, or prohibition on private-key snapshots and evidence hashes" | revised: CN-NODE-03 drafted as #36 ForgeActivationAllOrNothing; production-specific clauses deferred to B10 in S3 | open |
| 2 | S2 | "SAF-002 … does not require DC-NODE-04’s non-zero process exit or explicitly prohibit retrying committed-write failures" | revised: drafted as #37 AuthorityFatalExit | open |
| 3 | #11 | "never requires the DC-NODE-15 catch-up gate … DC-NODE-20 retires only the repeated post-self-admission check" | revised: the initial catch-up gate is required; the single-producer extension replaces only the repeated check; the anchor is never a forge base | open |
| 4 | #11 | "DC-NODE-47’s rollback-clearing and consumer separation are missing" | revised: admit-gated served evidence from the admission's own validated tip, cleared by rollback, ties fall to the advertisement, and the possession and testimony readers are named | open |
| 5 | #13 | "Declaring a single-producer venue alone replaces DC-NODE-20/22’s stronger fences" | revised: all fences restored (no competing block observed, non-producing relay, contiguous servable spine, no fork choice required, entry via catch-up plus self-admission, warm-start entry only on proven own-forged continuation); clean structural feed end distinguished from shutdown or fatal failure | open |
| 6 | #16 | "its emit-only boundary and fail-closed feed split disappear" | revised: diagnostics are emit-only and never read by the planner; an ambiguous disconnect is never forge-eligible | open |
| 7 | #17 | "The domain excludes candidates already admitted, yet includes “already held” … Durable tip plus candidate header also cannot establish membership of an older stored block." | revised: every peer announcement is classified, with explicit durable-membership evidence as an input | open |
| 8 | #35 | "conflicts with participant fork choice in #18–#27, where such blocks can form a valid winning branch" | revised: scoped to linear admission against the current tip; competing-branch blocks go through fork choice | open |
| 9 | #28 | "require a missing bridge to hold the forge fence until real admitted progress" | revised: forging stays refused while a missing bridge is held; only real linear admission or a proven fork-switch adoption clears it | open |
| 10 | #28 | "omits DC-NODE-41’s deterministic bounded retry and complete range validation before admission" | revised: winning peer only, deterministic bounded retries, admission only of a complete, parent-linked, body-validated range | open |
| 11 | #33 | "IntersectFound must name an offered point; IntersectNotFound must terminate following with a typed bound reason, without RequestNext or reconnect." | revised: both reply guards added; the intersection is not rollback authority | open |
| 12 | #31; #33 | "#31 requires reconnect to resume from the last delivered block unconditionally, while #33 permits only durable-chain candidates." | revised: #31 now builds candidates by #33's rule at dial time with the delivered-since-rollback resume hint first when newer, reset by rollback, resolved by #21, order preserved | open |
| 13 | #3 | "DC-NODE-45’s bootstrap binding is reduced to selecting a calendar by genesis hash." | revised: calendar self-consistency, recorded geometry, seed-point containment and lineage verification required; reconstructed, not persisted; forging without the seed record fails closed | open |
| 14 | #27 | "omits DC-NODE-38’s independent bound on traversed competing headers" | revised: both the traversed header count and the tip-to-anchor depth are at most k | open |
| 15 | #34 | "DC-NODE-30’s evidence completeness and failure semantics disappear" | revised: every considered block and every admission evented, verdicts fingerprint-bound; a sink failure is non-fatal but marks the transcript incomplete | open |
| 16 | #4; #26; #27; #29 | "These include security properties classified outside SEC" | revised: #4, #26, #27 and #29 are SEC primary | open |
| 17 | #5 | "“Within a bounded time” names neither a bound nor conditions" | revised: a fixed, stated deadline while storage keeps completing writes; the deadline or storage failure gives a distinct non-zero exit with no partial snapshot | open |
| 18 | #5; #15; #24 | "Assumption citations are incomplete" | revised: #5 cites A1 and A6, #15 A2, #24 A1; audit added A6 to #9, #19 and #26 | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/node.define.md @ blob 99acba7cbd32ab46a457b114b15688c1b5014e6d · received 99acba7cbd32ab46a457b114b15688c1b5014e6d

Round 1 items 1, 2, 4, 6–11, 14 and 16–18: Codex marked each `accepted`. Items 3, 5, 12, 13 and 15 stayed open (reasons quoted below).

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 3 | #11 | "requires equality only before the first own forge. DC-NODE-15 requires it for every forge except the fenced single-producer extension" | revised: equality is required for every forge except under #13; S11 defers participant forge-mode rules to B10, where any further exception must be a redefinition | open |
| 5 | #13 | "DC-NODE-20 explicitly prohibits that fallback after an observed competing block" | revised: a failed warm-start entry falls back to the catch-up gate; an established extension that loses a fence refuses to forge | open |
| 12 | #31; #33 | "State the resume-hint exception explicitly in #33 and require FIFO processing to make that point durable before its subsequent rollback is resolved." | revised: #33 names the resume hint as its one exception, with FIFO delivery making it durable first | open |
| 13 | #3 | "Require containment within the recorded seed epoch." | revised | open |
| 15 | #34 | "Bind every emitted event to it." | revised: every event bound to the consensus-input fingerprint | open |
| 19 | #21; #23 | "The required exception and evaluation order remain implicit." | revised: #21 excepts the persisted-anchor case of #23, which is evaluated first; #23 says so too | open |
| 20 | #10 | "allowing self-acceptance against stale startup state" | revised: the forge's ledger and chain-dependent base also come from the latest admission | open |
| 21 | #33 | "“Decreasing heights” permits ancestry heights to skip" | revised: heights decrease by exactly one at each parent link | open |
| 22 | #15 | "permits distinct failures to share a discriminator" | revised: distinct closed discriminators listed, absent-tip only for an actually absent tip | open |
| 23 | #3 | "DC-NODE-45’s refusal of snapshot-local schedules lacking system-start coverage, in both conversion directions, is not retained" | revised: a schedule not covering system start is refused for both conversions | open |
| 24 | Transitions; #17 | "classify_receive lists only durable tip and header inputs" | revised: the signature takes the durable-membership input | open |
| 25 | #12 | "allows a recovered anchor made intersectable as a forge parent, while #11 says the recovered anchor is never a forge base" | revised: #12 drops the anchor alternative; S12 records the legacy contradiction resolved toward DC-NODE-15 | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/node.define.md @ blob 943c7ff54a137b7963e3fbe8bc81f2e369c851a2 · received 943c7ff54a137b7963e3fbe8bc81f2e369c851a2

Round 2 items 3, 5, 12, 13, 15 and 19–25: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 26 | S1 | "DET-014 … does not require DC-NODE-02’s exclusive use of the existing cadence policy or prohibit an orchestrator-side override." | revised: DC-NODE-02 drafted as #38 SingleSnapshotCadencePolicy; S1 updated | open |

Outcome: continue

## Round 4 — 2026-11-08 — docs/planning/node.define.md @ blob cb7977784b08d4a72f8236a7df3e91f0d7f819f0 · received cb7977784b08d4a72f8236a7df3e91f0d7f819f0

Round 3 item 26: Codex marked it `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 27 | #5; #38; DET-014 | "#5 requires a final snapshot on clean shutdown, but #38 permits capture only at cadence-scheduled points … Specify whether shutdown only flushes an already scheduled snapshot" | revised: shutdown finishes only snapshots the cadence policy scheduled, restart identity coming from replaying the log; S13 records the departure from DC-NODE-04's final snapshot; DET-014 unchanged | open |

Outcome: continue

## Round 5 — 2026-11-08 — docs/planning/node.define.md @ blob 80ebaa6b56d6ea32b19627f142ec14d93c701948 · received 80ebaa6b56d6ea32b19627f142ec14d93c701948

Round 4 item 27: Codex marked it `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-09 — after round 5 @ blob 80ebaa6b56d6ea32b19627f142ec14d93c701948

```text
Define batch — node (B09) — consensus: round 5 (docs/planning/node.define.consensus.md)

 1. STR  SingleBootstrapAuthority                [critical]  One bootstrap authority; verified first run and warm start are branches of it.
 2. STR  SingleLiveLifecycleOwner                [critical]  Node mode owns the live run; state changes only via closed authorities; the planner can't decide authority.
 3. DET  SingleSlotAuthority                     [critical]  One wall-clock→slot authority over the full calendar, bound to the durable record; no clock in the core.
 4. SEC  PerPeerSessionIsolation                 [critical]  One peer's failure ends only its own session.
 5. LIV  BoundedCleanShutdown                    [critical]  Shutdown drains within a stated deadline and exits; failure gives a distinct code; restart is identical.
 6. VAL  ServedViewIsDurableProjection           [critical]  The served chain is a projection of the durable chain, through the existing serve reducers, read-only.
 7. LIV  ServeOutlivesFeed                       [standard]  The end of the upstream feed doesn't stop serving.
 8. SAF  ForgeOncePerSlotMonotone                [critical]  At most one forge per slot, never at or before the last forged slot.
 9. STR  OwnForgedAdmitThroughChokepoint         [critical]  Forged blocks become durable only through the normal admission path, bytes unchanged.
10. VAL  ForgeSuccessorPosition                  [critical]  Parent hash, block number and validation base all come from the latest admitted state.
11. SAF  ForgeAdmissibilityGatesOnly             [critical]  Forge needs durable tip = followed tip (except #13); the peer-tip signal only gates, never selects.
12. VAL  ForgeParentIntersectable                [critical]  Forge only on a stored block a peer can intersect; never on the recovered anchor.
13. VAL  SingleProducerForgeBase                 [critical]  Single-producer forge base = own durable tip, under explicit fences; a lost fence refuses, never falls back.
14. SAF  NoForgeDuringPendingSelection           [critical]  No forging while a fork-choice decision is pending.
15. OBS  EveryForgeTickAccounted                 [standard]  Every admitted forge tick ends in a distinct, closed refusal reason or a leadership decision.
16. OBS  ClosedDiagnosticVocabulary              [standard]  Closed, emit-only diagnostics; ambiguous disconnects never make forging eligible.
17. VAL  ReceiveDispositionSingleClassifier      [critical]  One pure classifier for every peer announcement, with explicit durable-membership evidence.
18. SAF  VenueSplitForkChoice                    [critical]  Single-producer refuses, participant selects, unknown refuses; raw peer tips never reach selection.
19. STR  ForkChoiceAppliedThroughAuthorities     [critical]  Selection outcomes change durable state only via existing authorities, after bodies validate.
20. REC  SelectorReconciledWithDurable           [critical]  After every decision, the selector's tip and state equal the durable ones.
21. SEC  RollbackTargetFromStoredPoint           [critical]  Rollback targets are stored chain points; a peer slot must match; mixed authority is impossible.
22. REC  FollowStartFromPersistedAnchor          [critical]  Live follow starts from the servable tip, else the persisted anchor; a bad anchor record fails closed.
23. SAF  AnchorRollbackNoOp                      [critical]  Rollback to exactly the recovered anchor is a no-op; the anchor is never served as a block.
24. OBS  PeerIdentityPreserved                   [standard]  Peer identity is carried as provenance only, changing nothing.
25. SEC  CandidatesFromOwnValidatedHeaders       [critical]  Selection candidates come only from headers Ade validated; one selector.
26. SEC  ForkSwitchNeverAbandons                 [critical]  No rollback until the replacement branch is fetched, linked and validated; failure changes nothing.
27. SEC  ForkAnchorWithinKValidated              [critical]  Branches need a stored anchor within k (count and depth) and fully validated headers.
28. LIV  PostSwitchContinuity                    [critical]  After a switch, descendants are admitted in order, missing bridges re-fetched within bounds, forging held meanwhile.
29. SEC  RetainedBranchEvidenceNonAuthoritative  [critical]  Rolled-back blocks kept as k-bounded, self-bound evidence, never authority.
30. SAF  ForgePathRollbackWithinPromotedEpoch    [critical]  Forge-path rollback into an earlier epoch than the promoted authority fails closed.
31. LIV  ReconnectResumesFromDelivered           [critical]  Reconnects rebuild candidates from the durable chain, the delivered block first as a hint; order kept.
32. OBS  ReplayDivergenceSelfDescribing          [standard]  Replay divergence reports name the component, epoch, anchor and span.
33. VAL  IntersectionCandidatesMatchReference    [critical]  FindIntersect points come from durable ancestry at the reference depths; replies checked.
34. OBS  ConvergenceEvidenceNonAuthoritative     [standard]  Convergence evidence is complete, fingerprint-bound and never authority; write failure marks it incomplete.
35. SAF  ReceiveIdempotentByHash                 [critical]  On linear admission, an already-stored identical block is a no-op; decided by hash, not slot.
36. SEC  ForgeActivationAllOrNothing             [critical]  Forging needs the complete operator key set; partial sets are errors; private keys never logged or hashed.
37. SAF  AuthorityFatalExit                      [critical]  Authority-fatal errors exit non-zero with no retry and no fallback decode.
38. STR  SingleSnapshotCadencePolicy             [standard]  Snapshots are captured only on the one cadence policy's schedule.

Settled in consensus:
 S1. DC-NODE-02 — drafted as 38.   S2. DC-NODE-04 — drafted as 37 and 5.   S3. CN-NODE-03 — drafted as 36; its production-specific clauses → B10.
 S4. DC-NODE-27 (rollback/reselection replay) — merged into REC-004 and DUR-009.
 S5. DC-NODE-06/07's "served chain fed only by self-accepted blocks" — superseded by DC-NODE-13 (durable projection); the rest is in 6.
 S6. DC-NODE-11 (block-0 serve gate) — excluded: superseded by DC-NODE-13.
 S7. DC-NODE-18 (forge after an adoption certificate) — excluded: retired by DC-NODE-20.
 S8. DC-NODE-08 (forge the genesis-successor block) — excluded: contradicts A2.
 S9. DC-NODE-05's clock and once-per-slot clauses — in 3 and 8.
 S10. 5 is expected to be FALSIFIED at re-evidence (the node ignores SIGINT/SIGTERM today).
 S11. 11's gate covers every forge outside 13; participant forge-mode rules → B10, as a redefinition of 11 if needed.
 S12. DC-NODE-14's anchor-as-forge-parent clause contradicts DC-NODE-15; resolved toward DC-NODE-15 in 12.
 S13. 5 drops DC-NODE-04's "always write a final snapshot"; only cadence-scheduled snapshots are finished, restart replays the log.

Legacy dispositions (52): every ID on lines 1–38 except: merged DC-NODE-27 · excluded DC-NODE-08, DC-NODE-11, DC-NODE-18

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"
