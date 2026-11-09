# Consensus record — docs/planning/consensus.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/consensus.define.md @ blob d37cb9313fbc52e4393c5633870ab19f3f0e44d2 · received d37cb9313fbc52e4393c5633870ab19f3f0e44d2

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | S1 | "neither excludes consensus dependence on arrival races or wall-clock values supplied by RED. #1 covers selection only" | revised: T-CONS-02 and CN-CONS-05 drafted as #13 ConsensusIndependentOfArrival | open |
| 2 | S3 | "EVO-003 guarantees equal validity verdicts … it does not guarantee equal resulting state or fork choice." | revised: CN-CONS-02 drafted as #14 RolloutSkewCannotSplit (SEC primary) | open |
| 3 | S4 | "SEC-005 … does not require the operator-supplied anchor, ProducerTick provenance, slot-to-period calculation or forge-time mismatch rejection" | revised: DC-CONS-11 deferred to B10 with the other forging entries | open |
| 4 | #10 | "changes DC-CONS-10 from counters keyed by (pool, KES period) to counters keyed by pool and adds an upper increment bound, without acknowledging the departure. It also leaves the first-observation case undefined" | revised: #10 states the reference rule in full, including the initial lookup (recorded, else 0 if the pool is in the stake distribution, else invalid); S11 records both departures and the expected falsified status if the code differs | open |
| 5 | #2 | "The statement supplies no eventuality condition and chiefly repeats local selection plus rollback safety." | revised: #2 is conditional on new assumption N1 honest-peer-delivery and states an eventual-equality obligation; the deep-fork clause moves out (S12, already #5) | open |
| 6 | #6, #7, #8, #12 | "Assumption citations are incomplete" | revised: #6 cites A1 and A2, #7 A1, #8 A1 and A2, #12 A1 | open |
| 7 | #11 | "permits semantic equality after decoding, whereas DC-CONS-19 requires equality of the preserved header bytes." | revised: byte-for-byte equality of the delivered header bytes and the announced header bytes | open |
| 8 | #4 counterexample | "advancing provisional chain-dependent state after header validation is not itself prohibited when the body later fails." | revised: the counterexample is the ledger writing the counter map outside the consensus header transition | open |
| 9 | #7 family | "Snapshot encoding and decode round-trip integrity does not assert preservation of committed state across failures, so DUR does not directly classify its stated property." | revised: DET primary, VAL secondary | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/consensus.define.md @ blob 59de6e1cf28d4d9cedd56d8a68b3ab02a8d040cc · received 59de6e1cf28d4d9cedd56d8a68b3ab02a8d040cc

Round 1 items 1–9: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 10 | #13 | "The equality obligation fixes state, selection and candidates but omits recorded context, despite explicitly allowing the current slot to affect decisions." | revised: the decision is a function of state, selection, candidates, protocol version and recorded context (the current slot included), and is identical when all are equal | open |
| 11 | #2 | "“Among all the chains delivered” includes invalid chains and chains requiring prohibited rollback." | revised: valid delivered chains that #5 allows it to adopt, the current selection included, ties settled as the reference rule settles them | open |
| 12 | #4, #10 | "rolling back a header that increased a pool’s counter must restore the earlier, lower counter" | revised: forward transitions only through headers and ticks (never lowering a counter); rollback and verified snapshot restore set the state of the restored point | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/consensus.define.md @ blob 19acef07e217d4500e4744bfb82ed212d622fc32 · received 19acef07e217d4500e4744bfb82ed212d622fc32

Round 2 items 10–12: Codex marked each `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-08 — after round 3 @ blob 19acef07e217d4500e4744bfb82ed212d622fc32

```text
Define batch — consensus (B05) — consensus: round 3 (docs/planning/consensus.define.consensus.md)

 1. VAL  ChainSelectionMatchesReference   [critical]  Same candidates and state → the reference node's Praos choice: block number, then the Praos tiebreak; no density.
 2. LIV  PartitionConvergence             [critical]  With an honest peer delivering (N1) and a fork within k, the node eventually selects what the reference rule prefers.
 3. VAL  HeaderBindsBodyAndContext        [critical]  The header's body hash, previous hash, block number and slot bind exactly the body and chain position.
 4. STR  ChainDepStateOwnedByConsensus    [critical]  Consensus owns the chain-dependent state; forward it changes only via headers and ticks; rollback restores the point's state.
 5. SAF  RollbackBoundedByK               [critical]  No rollback deeper than k blocks is ever applied; a deeper request is rejected and changes nothing.
 6. DET  RollbackAndReplayEquivalence     [critical]  Rollback, or snapshot restore plus replay, gives byte-identical state to direct application.
 7. DET  SnapshotRoundTrip                [critical]  Snapshots carry a closed version tag and the state fingerprint, and decode to that fingerprint or are rejected.
 8. STR  EraScheduleAnchored              [critical]  The core gets the era schedule only as a typed value bound to the bootstrap anchor; genesis parsing stays in the shell.
 9. VAL  ForecastRangeEnforced            [critical]  Queries beyond the safe forecast range return a typed out-of-range result, never a guess.
10. SEC  OpCertCounterRule                [critical]  The reference OCERT rule: issue number ≥ current and ≤ current+1, current from state or 0 if staked, keyed by pool.
11. SEC  AnnouncedHeaderMatchesDelivered  [critical]  The delivered block's header bytes equal the announced header bytes, byte for byte.
12. REC  LockstepAdmissionAndRollback     [critical]  Admission and rollback move the chain database, ledger state and chain-dependent state together.
13. DET  ConsensusIndependentOfArrival    [critical]  Consensus decisions depend only on state, selection, candidates, version and recorded context, never on arrival order or clocks.
14. SEC  RolloutSkewCannotSplit           [critical]  No input can drive two coexisting supported releases to different state or different chains.

New assumptions (approved with the lines that cite them):
 N1  honest-peer-delivery: after a partition heals, the node stays connected to at least one peer offering the chain the network converges on, with bounded delivery and processing — cited by 2

Settled in consensus:
 S1. T-CONS-02, CN-CONS-05 — drafted as 13.   S3. CN-CONS-02 — drafted as 14.
 S2. DC-CONSENSUS-02, DC-CONS-08 (leadership verification and slot-to-time are pure) — merged into STR-001.
 S4. DC-CONS-11 (forge-time KES period) — deferred to B10.
 S5. A4 stays; 5 is the invariant that enforces it, and its evidence discharges A4.
 S6. Forging entries DC-CONS-12, 13, 14, 15, 16, CN-CONS-07, CN-CONS-08, DC-CONS-23, 24 — deferred to B10.
 S7. Serving entries DC-CONS-17, 18 — deferred to B08.   S8. CN-CONS-06 (live acceptance by cardano-node) — deferred to B12.
 S9. CN-CONS-IN-01, DC-CONS-IN-01, DC-CONS-IN-02 — deferred to B06.   S10. CN-SEED-01, DC-SEED-01 — deferred to B11.
 S11. 10 departs from DC-CONS-10 toward the reference (keyed by pool, upper bound +1); expected FALSIFIED at re-evidence if the code differs.
 S12. The deep-fork clause of 2 is already 5.

Legacy dispositions (39): 1←T-CONS-01, CN-CONS-01, DC-CONSENSUS-01, DC-CONS-03 · 2←CN-CONS-03 · 3←CN-CONS-04 · 4←DC-CONS-04 · 5←DC-CONS-05 · 6←DC-CONS-06, 22 · 7←DC-CONS-21 · 8←DC-CONS-07 · 9←DC-CONS-09 · 10←DC-CONS-10 · 11←DC-CONS-19 · 12←DC-CONS-20 · 13←T-CONS-02, CN-CONS-05 · 14←CN-CONS-02 · merged DC-CONSENSUS-02, DC-CONS-08→STR-001 · 17 deferred (S4, S6–S10)

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok". Answer N1 the same way.
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"
