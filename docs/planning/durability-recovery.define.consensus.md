# Consensus record — docs/planning/durability-recovery.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/durability-recovery.define.md @ blob f5416c31081f15b7586ca529f57768286396ebca · received f5416c31081f15b7586ca529f57768286396ebca

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #10 | "“Previous entry’s post fingerprint” is undefined for provenance and activation entries and incorrect after a rollback" | revised: #10 defines the check as a fold over the log: admissions check and advance, a rollback sets the value to the target admission's post fingerprint, provenance and activation entries leave it unchanged (matches WalEntry's four variants in crates/ade_ledger/src/wal/event.rs) | open |
| 2 | #1 | "assumes every WAL entry records a ledger fingerprint; several current variants do not." | revised: replay's fingerprint after entry N equals the fold's expected value after N (#10) | open |
| 3 | S1 | "DET-010 … does not require recovery to select the most recent valid snapshot and replay forward." | revised: #1 requires recovery to start from the most recent valid snapshot at or below the durable tip | open |
| 4 | S2 | "Decoding stored blocks and trusting cached validity can satisfy STR-004 while violating both legacy entries’ validation requirements." | revised: drafted as #16 RecoveryRevalidatesStoredBlocks; STR-004 keeps only the decode part | open |
| 5 | S3 | "DET-011 does not require a u32 version tag, checking the version before decoding payloads, or specifically a blake2b-256 fingerprint." | revised: DC-STORE-09 drafted as #17 SnapshotHeaderChecks; the u32 width recorded as a departure (an encoding detail) | open |
| 6 | S5 | "none explicitly requires bootstrap to overlay the recovered seed nonce onto chain-dependent state before forging or self-acceptance." | revised: T-REC-04 drafted as #18 WarmStartNonceFromSeedRecord | open |
| 7 | #13 | "A genesis start conflicts with A2’s requirement to enter at a certified non-Origin Conway tip." | revised: #13 confines genesis to test networks, never production entry; S10 records the open A2 question for the human (private testnet from genesis) | open |
| 8 | #6 | "T-REC-06 explicitly limits its nonce guarantee to the recovered seed epoch … this line silently extends it across epoch boundaries." | revised: #6 rebuilds with each replayed epoch's own nonce (seed record for the seed epoch, evolved after), cites A2 and VAL-022; S11 records the scope extension | open |
| 9 | #15 | "Finding an ancestor is promised without a delivery or responsiveness assumption." | revised: phrased as completeness given the peer's responses to intersection requests | open |
| 10 | #15 | "Full durable-state equality conflicts with #2 because append-only provenance retains those abandoned admissions and rollback records." | revised: compares ledger state, chain-dependent state and selected chain at the same canonical points, excluding the log's audit history | open |
| 11 | #1 | "DC-STORE-01 explicitly covers power loss, whereas “process termination” need not cover loss of unsynced writes" | revised: #1 names power loss and the last durably committed prefix; new assumption N1 durable-commit-honoured, cited by #1, #3, #10, #11, #14 | open |
| 12 | #14 | "“the same write that applies the block” does not explicitly include that cursor." | revised: retention, live-table changes and cursor advance in one durable transaction | open |
| 13 | #9 | "allows non-neutral lock entries to repeat a version and does not require every entry to carry a rationale" | revised: every entry carries a rationale; every non-neutral entry strictly increases the version | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/durability-recovery.define.md @ blob f9f8902dff0c6deeb8240f6b35fc8128d0e77bec · received f9f8902dff0c6deeb8240f6b35fc8128d0e77bec

Round 1 items 1–6 and 8–13: Codex marked each `accepted`. Item 7 stayed open: "A2 requires entry at a certified non-Origin Conway tip without a production-only qualification. Restricting genesis starts to test networks does not resolve that conflict. Remove the exception or draft an A2 redefinition with exact current text, proposed text, and all affected active invariants; S10 cannot postpone that decision while #13 grants the exception."

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 7 | #13 | (above) | revised: the genesis-start line is removed (lines 14–18 renumbered 13–17); S10 excludes DC-GENESIS-SRC-01 because it contradicts A2 as approved, and states that allowing a genesis start needs an A2 redefinition decided by the human | open |
| 14 | S4 | "DET-007 requires one encoder … but does not require one decoder authority. STR-004 … does not prohibit parallel public decoders." | rebuttal, with S4 reworded: VAL-001 ClosedGrammarDecoding's approved storage clause reads "each of Ade's own storage record types (write-ahead log, chain database, snapshots) has exactly one decoder per format version", which carries CN-STORE-08's decoder exclusivity; S4 now cites VAL-001 as well as DET-007 | open |
| 15 | #1; T-REC-03; T-REC-05 | "does not explicitly require two continuous executions with identical starting state, feed, clock ticks, leadership/key inputs and shutdown schedule to produce identical durable outputs." | revised: #1 states continuous-run equivalence with those inputs, self-forged admissions included | open |
| 16 | #10 | "the bootstrap anchor has no admission entry. Rolling back the first admitted blocks to that anchor leaves the fold undefined" | revised: a rollback to the anchor resets to its initial fingerprint; any other target must be an admission on the selected prefix; anything else is rejected | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/durability-recovery.define.md @ blob eb2650d9d8428145f25d7a6f13e0ceac4677999a · received eb2650d9d8428145f25d7a6f13e0ceac4677999a

Round 2 items 7, 14, 15 and 16: Codex marked each `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-08 — after round 3 @ blob eb2650d9d8428145f25d7a6f13e0ceac4677999a

```text
Define batch — durability-recovery (B07) — consensus: round 3 (docs/planning/durability-recovery.define.consensus.md)

 1. REC  CrashRecoveryReplayEquivalent     [critical]  Continuous runs and post-crash/power-loss recovery give byte-identical durable outputs over the committed prefix.
 2. DUR  AppendOnlyWal                     [critical]  The write-ahead log is append-only by construction; recorded provenance is never altered.
 3. DUR  AtomicCheckpoints                 [critical]  Every snapshot and checkpoint is fully written and valid, or absent.
 4. SEC  StorageAfterAnchorVerification    [critical]  No authoritative store is initialized before its bootstrap source is verified.
 5. DUR  SingleLineageBinding              [critical]  Every log entry, checkpoint and artifact binds to exactly one anchor; others are refused.
 6. STR  SingleRollbackMaterializer        [critical]  One authority materializes rolled-back state, rebuilding each epoch's nonce as live admission used it.
 7. DET  SnapshotCadenceDeterministic      [standard]  Snapshot points are a pure function of chain position and fixed cadence; not an operator setting.
 8. EVO  StoreSemanticsVersionGate         [critical]  Each artifact carries its semantics version; absent, older or newer fails closed; remedy is re-bootstrap.
 9. PRC  SemanticsSurfaceLocked            [standard]  Semantics-bearing code is hash-locked; changes need a version bump or a neutral rationale.
10. DUR  WalFingerprintChain               [critical]  The log's fingerprint chain is checked by a defined fold (admissions, rollbacks, anchor); breaks halt.
11. DUR  BlockBytesBeforeWal               [critical]  Block bytes are durable before their log entry; a log entry without bytes halts; bytes without an entry are dropped.
12. VAL  ClosedGenesisParser               [critical]  One closed Shelley-genesis parser: required fields, exact types, no defaults, deterministic.
13. DUR  CollateralValueRetained           [critical]  Spent collateral values are retained in the same transaction that advances the checkpoint, for later resolution.
14. REC  RecoveryFindsCommonAncestor       [critical]  A restarted node finds a common ancestor within k when one exists, without changing replay; never resets to genesis.
15. VAL  RecoveryRevalidatesStoredBlocks   [critical]  After an unclean shutdown, every replayed block is validated again; no stored verdict is trusted.
16. DET  SnapshotHeaderChecks              [critical]  A snapshot's version is checked before decoding; its blake2b-256 fingerprint is recomputed and compared.
17. VAL  WarmStartNonceFromSeedRecord      [critical]  Warm start takes the seed epoch's nonce from the recovered seed record, never a placeholder or genesis.

New assumptions (approved with the lines that cite them):
 N1  durable-commit-honoured: a write the storage layer reports durably committed survives process termination and power loss intact — cited by 1, 3, 10, 11, 13

Settled in consensus:
 S1. DC-STORE-05 (snapshot + forward replay recovery) — drafted into 1; equivalence carried by DET-010.
 S2. DC-STORE-06, CN-STORE-06 — drafted as 15; STR-004 keeps the decode chokepoint.
 S3. DC-STORE-08 — merged into DET-011, DET-007; DC-STORE-09 — drafted as 16 (the u32 tag width is an encoding detail).
 S4. CN-STORE-08 — merged into DET-007 (one encoder) and VAL-001 (one decoder per storage format version).
 S5. T-REC-04 — drafted as 17.
 S6. DC-STORE-04 (Haskell ImmutableDB/VolatileDB/LedgerDB layout) — excluded: a layout description, not a property.
 S7. RO-GENESIS-REPLAY-01 (replay from Byron genesis) — excluded: Conway only (A2).
 S8. Served-chain snapshot entries CN-SNAPSHOT-01, 02, DC-SNAPSHOT-01 — deferred to B08.
 S9. DC-WAL-05's one-block-in-memory clause — deferred to B12.
 S10. DC-GENESIS-SRC-01 (genesis start) — excluded: contradicts A2 as approved. A genesis start (e.g. a private testnet) needs an A2 redefinition, yours to decide.
 S11. 6 extends T-REC-06's nonce rule from the seed epoch to every replayed epoch, per VAL-022.

Legacy dispositions (41): 1←T-REC-01, T-REC-02, DC-STORE-01, CN-STORE-03, T-REC-03, T-REC-05, DC-WAL-03 (+DC-STORE-05) · 2←DC-STORE-02, CN-STORE-05, CN-WAL-01, DC-WAL-01 · 3←DC-STORE-03, CN-STORE-04 · 4←CN-STORE-01 · 5←CN-STORE-02 · 6←CN-STORE-07, T-REC-06 · 7←DC-STORE-07 · 8←DC-STORE-10, 11 · 9←DC-STORE-12 · 10←DC-WAL-02, 04 · 11←DC-WAL-05 · 12←CN-GENESIS-01, DC-GENESIS-01 · 13←DC-LEDGER-PHASE2-04 · 14←T-REC-07 · 15←DC-STORE-06, CN-STORE-06 · 16←DC-STORE-09 · 17←T-REC-04 · merged DC-STORE-08, CN-STORE-08 · excluded DC-STORE-04, RO-GENESIS-REPLAY-01, DC-GENESIS-SRC-01 · deferred CN-SNAPSHOT-01, 02, DC-SNAPSHOT-01 → B08

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok". Answer N1 the same way.
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"
