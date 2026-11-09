# Consensus record — docs/planning/epoch.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/epoch.define.md @ blob a406ef9ca6ac8233231853cc9497a493907b5a21 · received a406ef9ca6ac8233231853cc9497a493907b5a21

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #11, #12 | "#11 requires every candidate to come from source-window replay, but DC-EPOCH-25 explicitly supersedes that promotion source with frozen leadership objects beyond the bootstrap bridge and forbids window-replay fallback." | revised: #11 renamed LeadershipSealedAtBoundaryPoint and restated as bootstrap seeding (the bootstrap epoch and the next from the certified snapshot) plus native sealing at each crossed boundary, for E+2 at the boundary from E to E+1, atomic with the accumulator advance; #12 requires promotion and reads by exact target epoch from the sealed object, with no window-replay, nearest or bootstrap fallback; S8 records the superseded window-replay design | open |
| 2 | #12 counterexample | "Removing a zero-stake pool cannot change a denominator computed by summing pool stakes" | revised: a positive-stake pool filtered out of the list | open |
| 3 | #15 | "The rewind guard omits DC-EPOCH-27’s requirement that the settled point must not be ahead of the rollback target" | revised: the target must be at or before the rollback target | open |
| 4 | #18, #15, #19 | "The unconditional “always between k and 2k” and bounded replay promises contradict the permitted bootstrap fallback … “About 2k” also lacks a testable bound." | revised: at most 2k blocks, under the stated preconditions (a settled point exists and every proof holds; the chain has advanced 2k past bootstrap); the bootstrap fallback is stated as the exception | open |
| 5 | #16 | "Re-sealing every previously crossed boundary identically is too strong after an admissible rollback changes that boundary’s canonical prefix." | revised: identity holds over the same canonical source prefix, and no object or activation record from an abandoned prefix is reused | open |
| 6 | #6 | "leaves the two nonce registers and their tick ordering ambiguous … absent operands fail closed." | revised: every register update at the header and the tick is spelled out, with the combine operator, no extra entropy, and fail-closed operand presence | open |
| 7 | #6, #27 | "DC-EPOCH-38’s mandatory post-freeze bootstrap cross-check and terminal treatment of unlocatable seeds or unavailable freeze geometry disappear" | revised: #6 requires the stored seed-time nonce to match the tick when the seed sat at or after the freeze, and makes an unlocatable seed or unknown stability window terminal | open |
| 8 | #7, #10 | "DC-EPOCH-04’s replay rule is lost: identical complete activation records are idempotent, while any differing same-epoch record is a terminal conflict." | revised: #7 states byte-identical replay is idempotent and any other same-epoch record is a terminal conflict | open |
| 9 | #13 | "DC-EPOCH-15 requires the forecast schedule to be derived from durable authority and committed geometry, reconstructed identically at recovery, and never persisted as a second authority." | revised: #13 states the schedule is derived, never persisted, reconstructed identically at recovery in both states, and free of flags, clocks and peer input | open |
| 10 | #20, #21 | "The bounded, disk-backed state requirements in DC-EPOCH-11 and DC-EPOCH-19 disappear" | revised: #20 adds bounded memory (disk-backed checkpoint, incremental accumulator, no whole-chain replay or whole-accumulator copy per block); #21 adds chain order and fail-closed on a missing, corrupt or lagging checkpoint | open |
| 11 | #24 | "DC-EPOCH-44’s import cross-checks and runtime refusal are missing." | revised: import cross-checks, go-snapshot-only reads, typed terminal for missing parameters | open |
| 12 | #25 | "DC-EPOCH-45’s canonical bootstrap donation-pot import and prohibition on using donations for ratification, enactment or withdrawal affordability are absent." | revised: canonical import with no zero default, excluded from affordability decisions, overflow terminal | open |
| 13 | #26 | "DC-EPOCH-41’s Native branch loses the explicit ssFee reduction, committed distinction between absent and zero-valued updates, and terminal guard for a zero pool pot with nonempty production counts." | revised: all three restored, plus no imported delta except from a completed imported update | open |
| 14 | #4 | "DC-EPOCH-47’s vote-delegation lifecycle and bootstrap admissibility rules are unaccounted for" | revised: #4 adds the apply step's read restrictions and its halting cases; new line #30 VoteDelegationLifecycle; S9 defers bootstrap admissibility to B11 | open |
| 15 | #28 | "requires every nonretryable failure, failed retry and walk fault to halt the authoritative pass with a typed cause" | revised: those halt with a typed cause; no observe-only exit | open |
| 16 | #13, #18, N1 | "#13 relies on reference forecast semantics without citing A2; #18 uses protocol k and canonical recovery ordering while citing only N1" | revised: #13 cites A2; #18 cites A1, A2 and N1; N1 states that k comes from genesis under A2 | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/epoch.define.md @ blob 3d1c0f02eadbdcea63c2be88dffab8881c8f8156 · received 3d1c0f02eadbdcea63c2be88dffab8881c8f8156

Round 1 items 1–16: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 17 | #10, S8 | "#10 still mandates validating a source window and deriving the activation candidate from it on every activation." | revised: the candidate is the sealed object for exactly that epoch with its stored provenance, or the certified bootstrap seeding for the two bootstrap epochs; no universal source-window step | open |
| 18 | #7, #16 | "After rollback abandons an activation’s transition point, a replacement activation can have a different identity for the same epoch; the draft does not specify whether discarding the first record permits it." | revised: uniqueness holds within one canonical history; a rollback that abandons an activation's transition point invalidates its record before any replacement, and terminal conflict still applies within a history | open |
| 19 | #28 criticality | "Classifying this as standard leaves a consensus-authority failure surface without the critical mutation-witness obligation" | revised: #28 is critical | open |
| 20 | S9 | "omits cross-checking imported finished ratification against the snapshot’s governance root" | revised: S9's deferred obligations include the governance-root cross-check | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/epoch.define.md @ blob b20c91323cb9735dbe7d25aa920bac98d107e726 · received b20c91323cb9735dbe7d25aa920bac98d107e726

Round 2 items 17–20: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 21 | #11 locality | "The statement requires leadership sealing and accumulator advancement in the same durable commit, but declares locality `local`" | revised: #11's locality is transactional | open |

Outcome: continue

## Round 4 — 2026-11-08 — docs/planning/epoch.define.md @ blob 6628b38582d280e301e068cdc99320db3f8be523 · received 6628b38582d280e301e068cdc99320db3f8be523

Round 3 item 21: Codex marked it `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 22 | #20 assumptions | "The durable replayable-state and within-epoch replay guarantees rely on A1's recorded starting state, input order and explicit context, but #20 cites only A2." | revised: #20 cites A1 and A2 | open |

Outcome: continue

## Round 5 — 2026-11-08 — docs/planning/epoch.define.md @ blob b6fef64504747140189f715a602ae6567a8fff9f · received b6fef64504747140189f715a602ae6567a8fff9f

Round 4 item 22: Codex marked it `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 23 | #11, #12 | "DC-EPOCH-40’s denominator construction and commitment requirements are missing: preserving a recorded total at read time does not prohibit computing it from filtered pool entries at sealing time or omitting it from the canonical commitment." Proposed fix: "Require the total to be computed from the snapshot’s credential-side stake before membership filtering, through one definition shared by native sealing and bootstrap import, and included in the frozen object’s canonical encoding and hash; prohibit pool-entry summation at sealing and reading." | Claude agrees with the fix. The round limit (5) is reached, and the draft is not edited after its final round, so the item goes to the human as no consensus, with the amendment text in the batch | open |

Outcome: no-consensus [23]

## Batch 1 — 2026-11-08 — after round 5 @ blob b6fef64504747140189f715a602ae6567a8fff9f

```text
Define batch — epoch (B04) — consensus: round 5, no consensus on item 23 (docs/planning/epoch.define.consensus.md)

 1. VAL  EpochBoundaryMatchesReference       [critical]  Ade's epoch-boundary transition gives the reference NEWEPOCH result, in the reference order, from canonical chain state.
 2. STR  SingleEpochAuthority                [critical]  At every slot exactly one epoch view (stake, nonce, committee, governance, parameters), read afresh by validation and leadership, swapped atomically.
 3. ORD  EffectsActivateAtBoundaries         [critical]  Stake, rewards, parameter and governance changes take effect only at protocol-defined boundaries.
 4. VAL  RatificationFrozenAtPriorBoundary   [critical]  Ratification for E→E+1 is decided from the state the E−1→E boundary built, recorded durably, applied unchanged; enactment is all-or-nothing.
 5. CON  ProposalDepositReturn               [critical]  Every proposal leaving the set returns its deposit exactly once: to its registered return account, otherwise the treasury.
 6. VAL  PraosNonceEvolution                 [critical]  Every nonce register update per header and per epoch tick as Praos defines it; no fabricated nonces; bootstrap nonce from the live tick.
 7. SAF  AtMostOneActivationPerEpoch         [critical]  Per canonical history, at most one view activates per epoch; identical replays are idempotent, differing records terminal.
 8. ORD  ActivationAdvancesOneBoundary       [critical]  The active view moves one boundary at a time; N+1 never reads N's view; skipping a boundary is terminal.
 9. SAF  ActivationFailsClosed               [critical]  Only a real epoch transition enters boundary machinery; a bad candidate halts, never falls back.
10. DUR  ActivationDurableBeforeVisible      [critical]  A view goes active only after its activation record is durable; recovery restores exactly the recorded activation.
11. VAL  LeadershipSealedAtBoundaryPoint     [critical]  Crossing E→E+1 seals E+2's leadership object atomically with the accumulator, from the mark at the exact boundary point.
12. [NO CONSENSUS] VAL  LeadershipFromSealedView  [critical]  Leader checks for epoch E read only the sealed object for exactly E; its sigma denominator is the snapshot total.
      Codex: the line must also say how the denominator is computed and committed: from the snapshot's credential-side stake before
             membership filtering, by one definition shared by sealing and bootstrap, inside the object's canonical hash; never by
             summing pool entries at sealing or reading.
      Claude: agrees. Raised at the round limit, so it couldn't be revised. Resolve it with:
      "12 amend: The stake distribution used for a leader check in epoch E comes only from the sealed frozen leadership object whose target epoch is exactly E, read after its protocol-parameter commitment is verified; it is never re-derived at use time, never taken from a latest or nearest object, and never replaced by window replay or the bootstrap view, and a missing, unsealed or malformed object fails closed. Its sigma denominator is the snapshot's total active stake, computed once from the snapshot's credential-side stake before any pool-membership filtering, by one definition shared by native sealing and bootstrap seeding, and included in the object's canonical encoding and hash; it is never computed by summing the object's pool entries, at sealing or at reading, and a missing total fails closed."
13. ORD  ForecastHorizonFollowsPromotion     [critical]  The forecast horizon passes a boundary iff the next view is durably promoted; the schedule is derived, never persisted.
14. VAL  LedgerEpochAgreesWithSchedule       [critical]  After the boundary decision, the ledger epoch equals the schedule's epoch for that slot, else halt with no change.
15. REC  RewindTargetSettledAndCanonical     [critical]  Rewind only to a settled, canonical point ≥ k blocks below the tip and at or before the rollback target; else bootstrap.
16. DET  RewindReplayEquivalence             [critical]  Refold from the settled point = fold from bootstrap, byte for byte; abandoned-prefix objects never reused.
17. SAF  RewindRestoresCoherentLeadership    [critical]  A rewind restores the leadership table and accumulator together; no sealed object outruns the refold.
18. LIV  BoundedRefold                       [critical]  With a proven settled point, refold after rollback replays at most 2k blocks, and survives the post-rollback recovery pass.
19. DUR  SettledTripleIntegrity              [critical]  The settled rewind state is fingerprinted in its promoting commit and verified before any restore; else bootstrap.
20. DUR  SelfSufficientEpochState            [critical]  Durable state suffices for every future epoch transition without re-import or oracles, in bounded memory.
21. REC  SingleAdmissionPrefix               [critical]  The WAL tail defines the prefix; chain DB, chain-dep state, accumulator and checkpoint all derive from it.
22. VAL  SnapshotNonZeroActiveStake          [critical]  A delegated credential is in a stake snapshot iff its active stake is non-zero.
23. VAL  OwnerStakeSelfDelegated             [critical]  Owner stake = go-snapshot stake of owners delegated to that pool, one definition for pledge and leader reward.
24. VAL  FrozenPoolParameters                [critical]  Snapshots freeze each pool's parameters before adoption and reaping; rewards read only those; missing params terminal.
25. CON  DonationPot                         [critical]  One donation pot: valid txs add, the boundary moves it to the treasury once; imported canonically; never an affordability input.
26. VAL  SeedBoundaryRewardAuthority         [critical]  The first post-bootstrap reward update has exactly one source, chosen by the snapshot's state; absent never means zero.
27. PRC  PerNetworkEpochEvidence             [standard]  Epoch, boundary, agreement and nonce behaviour are checked per supported network, both sides of the freeze.
28. LIV  CoAdvanceRetryOnce                  [critical]  One named retryable failure, retried once in the same pass; every other failure halts with a typed cause.
29. EVO  ProtocolVersionChangesAtBoundaries  [critical]  A protocol-version change takes effect only at the boundary where its hard-fork action is enacted.
30. VAL  VoteDelegationLifecycle             [critical]  PV10 vote-delegation lifecycle: unregistrations remove delegations; dangling delegations are terminal.

New assumptions (approved with the lines that cite them):
 N1  rollback-depth-bound-k: the node never adopts a chain needing a rollback deeper than k blocks (k from genesis, per A2); blocks more than k below the tip are immutable — cited by 15, 16, 18

Settled in consensus:
 S1. CN-EPOCH-04 — merged into ORD-001 NoRetroactiveValidation.
 S2. DC-EPOCH-13 (no activation switch) — merged into EVO-001.
 S3. DC-EPOCH-43 — merged into STR-005 for credential keying; crediting drafted into 1.
 S4. DC-EPOCH-03 (single-epoch forge containment) — deferred to B10.
 S5. DC-GOV-01's "never decide ratification" approach — superseded by native ratification (4); its refund rule drafted as 5.
 S6. The k-block bound enters as N1 until B05 defines the invariant.
 S7. DC-EPOCH-02's era translation — excluded (Conway only); version changes drafted as 29.
 S8. Window-replay leadership past the bootstrap epochs (DC-EPOCH-08, 09, 17) — superseded by sealed objects (DC-EPOCH-25); surviving parts drafted into 11 and 8.
 S9. DC-EPOCH-47 bootstrap admissibility (finished-ratification import cross-checked against parameters and governance root; PV ≥ 10; no dangling delegations) — deferred to B11.

Legacy dispositions (54): 1←DC-LEDGER-04, CN-EPOCH-03, DC-EPOCH-21, 18, 43 · 2←T-EPOCH-01, CN-EPOCH-02, DC-EPOCH-14 · 3←CN-EPOCH-01 · 4←DC-EPOCH-01, 47 · 5←DC-GOV-01 · 6←DC-EPOCH-16, 38 · 7←04 · 8←05, 17 · 9←07, 39 · 10←06, 10 · 11←08, 09, 22, 32 · 12←12, 25, 40 · 13←15 · 14←36 · 15←26, 27, 29 · 16←31, 33 · 17←28 · 18←30, 35 · 19←34 · 20←19 · 21←20, 11 · 22←24 · 23←42 · 24←44 · 25←45 · 26←41, 23, 18 · 27←37, 38 · 28←46 · 29←02 · 30←47 · merged CN-EPOCH-04→ORD-001, DC-EPOCH-13→EVO-001 · deferred DC-EPOCH-03→B10

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-11 ok". Line 12 needs its own answer.
Answer the new assumption the same way: "N1 ok". A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all; 12 amend: Claude's text"

Line 12 is appended with the amendment text given under it in this batch.
