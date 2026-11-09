# Define — durability-recovery

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B07 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers crash recovery, the write-ahead log, checkpoints, store lineage and semantics versioning, rolled-back state, genesis entry, collateral retention and finding a common ancestor after restart. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 41 legacy IDs (40 assigned, plus DC-LEDGER-PHASE2-04 deferred here by the ledger batch) appears on a line or in a settlement.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants that settlements merge into)

## Drafts

```text
#1  REC  CrashRecoveryReplayEquivalent  [critical]
    statement:      All authoritative state is derivable by replay from the bootstrap anchor and the accepted canonical inputs. Two continuous runs from the same recovered or bootstrapped state, with the same ordered block feed, the same clock-tick schedule, the same leadership and key inputs and the same shutdown schedule, produce byte-identical durable outputs (selected tip, ledger fingerprint, write-ahead-log image, checkpoints and halt state), self-forged admissions included. After any process termination or power loss, recovery reaches exactly the last durably committed prefix, and its durable outputs are byte-identical to those of a run that stopped cleanly at that prefix; the ledger fingerprint replay reaches after log entry N equals the fingerprint the log's fold expects after entry N (#10). Recovery starts from the most recent valid snapshot at or below the durable tip and replays forward only the stored blocks after it.
    counterexample: A node that loses power mid-admission recovers to a ledger fingerprint different from a clean run over its committed blocks, or a forged block admitted before the crash is replayed with different bytes after it.
    secondary:      [DET, DUR]
    scope:          node recovery, write-ahead-log replay
    assumptions:    [A1, N1]
    depends_on:     [DET-001, REC-002]
    locality:       transactional
    legacy:         [T-REC-01, T-REC-02, DC-STORE-01, CN-STORE-03, T-REC-03, T-REC-05, DC-WAL-03]

#2  DUR  AppendOnlyWal  [critical]
    statement:      The write-ahead log is append-only by construction: its store interface offers no operation that truncates, rewrites, replaces or deletes an entry, append is its only mutation, and recorded provenance is never altered; any later correction is a new appended entry.
    counterexample: A recovery routine truncates the log's last entries to match the chain database, erasing the record of blocks that were admitted.
    secondary:      [OBS]
    scope:          write-ahead log
    assumptions:    []
    depends_on:     []
    locality:       transactional
    legacy:         [DC-STORE-02, CN-STORE-05, CN-WAL-01, DC-WAL-01]

#3  DUR  AtomicCheckpoints  [critical]
    statement:      Every snapshot and checkpoint is either fully written and valid or absent: a crash at any point during a write leaves either the previous complete version or the new complete version, never a partial one that recovery could read.
    counterexample: A crash midway through writing a ledger snapshot leaves a truncated file under the snapshot's final name, which recovery loads.
    secondary:      [REC]
    scope:          snapshots and checkpoints
    assumptions:    [N1]
    depends_on:     [DET-011]
    locality:       transactional
    legacy:         [DC-STORE-03, CN-STORE-04]

#4  SEC  StorageAfterAnchorVerification  [critical]
    statement:      No authoritative store is initialized or written before the bootstrap source or anchor it derives from has been verified; a verification failure leaves no authoritative store behind.
    counterexample: The node creates its chain database and writes the snapshot's UTxO into it, then discovers the Mithril certificate does not verify, and a later start resumes from that store.
    secondary:      [SAF]
    scope:          bootstrap, store initialization
    assumptions:    []
    depends_on:     []
    locality:       transactional
    legacy:         [CN-STORE-01]

#5  DUR  SingleLineageBinding  [critical]
    statement:      Every write-ahead-log entry, checkpoint and recovered artifact is bound to exactly one bootstrap anchor, and recovery refuses any artifact bound to a different anchor or to none.
    counterexample: An epoch-accumulator file copied from another node's data directory, bootstrapped from a different snapshot, is opened and folded forward alongside this node's chain database.
    secondary:      [SAF]
    scope:          durable artifacts, recovery
    assumptions:    []
    depends_on:     [#4]
    locality:       local
    legacy:         [CN-STORE-02]

#6  STR  SingleRollbackMaterializer  [critical]
    statement:      Ledger and chain-dependent state at a rollback target are materialized by one authority only, from one stored snapshot and the stored blocks after it, applied through the normal block application and epoch-boundary transitions; the chain-dependent state is rebuilt with the same epoch nonce the live admission path used for each epoch replayed (the recovered seed-record nonce for the seed epoch, the evolved nonce after it, VAL-022); no parallel computation of rolled-back state exists, and VRF verification on the rollback path is never weakened.
    counterexample: The rollback path rebuilds the chain-dependent state with the snapshot's placeholder nonce, so a block that validated during live admission fails VRF verification when replayed after a rollback.
    secondary:      [DET]
    scope:          rollback materialization
    assumptions:    [A1, A2]
    depends_on:     [DET-010, VAL-022, VAL-036]
    locality:       local
    legacy:         [CN-STORE-07, T-REC-06]

#7  DET  SnapshotCadenceDeterministic  [standard]
    statement:      Whether a snapshot is taken at a block is a pure function of its slot, its block number, the fixed cadence parameters and the last snapshot taken, so the same chain history produces the same set of snapshot points; the cadence is not an operator setting.
    counterexample: Snapshots are taken every ten minutes of wall-clock time, so two replays of the same chain store different snapshot sets and different store images.
    secondary:      []
    scope:          snapshot scheduling
    assumptions:    []
    depends_on:     [STR-001]
    locality:       local
    legacy:         [DC-STORE-07]

#8  EVO  StoreSemanticsVersionGate  [critical]
    statement:      Every durable authority artifact (the chain database with its write-ahead log, the epoch accumulator and the reduced checkpoint) carries its own marker of the semantics version of the rules that produced it, checked when it is opened, before any recovery work; a marker that is absent, older or newer than the binary's fails closed with a typed error whose only remedy is re-bootstrapping. There is no stamping tool, override or bypass; a future migration is allowed only as a sealed, evidenced transform from a marked old store to a new one.
    counterexample: A store written by a binary whose reward rules have since changed is opened by the new binary and folded forward, mixing two sets of semantics in one history.
    secondary:      [DUR]
    scope:          durable artifacts, binary upgrades
    assumptions:    []
    depends_on:     [EVO-001]
    locality:       local
    legacy:         [DC-STORE-10, DC-STORE-11]

#9  PRC  SemanticsSurfaceLocked  [standard]
    statement:      The code whose behaviour determines what the durable artifacts contain is listed and content-hashed in a lock file, and any change to it fails CI until the author either raises the semantics version or records the change as semantics-neutral; the lock is append-only, every entry carries a rationale, versions never decrease, every non-neutral entry strictly increases the version, and the last entry agrees with the code's version constant.
    counterexample: A change to the reward calculation merges without a version bump, so stores written before and after it carry the same marker but different meaning.
    secondary:      [EVO]
    scope:          CI, semantics versioning
    assumptions:    []
    depends_on:     [#8]
    locality:       local
    legacy:         [DC-STORE-12]

#10 DUR  WalFingerprintChain  [critical]
    statement:      The write-ahead log's fingerprint chain is checked by folding the log from the bootstrap anchor's initial ledger fingerprint: a block-admission entry, received or self-forged, must record a prior fingerprint equal to the folded value, binds the exact admitted block bytes, and advances the folded value to its post fingerprint; a rollback entry whose target is the bootstrap anchor resets the folded value to the anchor's initial ledger fingerprint, and one whose target is an admission on the currently selected prefix sets it to that admission's post fingerprint, any other target being rejected; provenance and activation entries leave it unchanged. An entry that would break the chain is rejected and halts the node, and verifying the chain at startup is fatal on failure.
    counterexample: A self-forged block is appended with a prior fingerprint taken from a stale tip, so the chain silently forks inside the log.
    secondary:      [REC]
    scope:          write-ahead log
    assumptions:    [N1]
    depends_on:     [#2, DET-002]
    locality:       transactional
    legacy:         [DC-WAL-02, DC-WAL-04]

#11 DUR  BlockBytesBeforeWal  [critical]
    statement:      An admitted block's original bytes are durably stored in the chain database before its write-ahead-log entry is appended, on every admission path; a storage failure halts before the append, and at recovery a log entry whose block bytes are missing halts with a typed error, while stored bytes with no log entry are dropped as an orphan.
    counterexample: The log records a block whose bytes never reached disk, and recovery skips the missing block silently, replaying a shorter chain than the log claims.
    secondary:      [REC]
    scope:          block admission, recovery
    assumptions:    [N1]
    depends_on:     [#10, REC-002]
    locality:       transactional
    legacy:         [DC-WAL-05]

#12 VAL  ClosedGenesisParser  [critical]
    statement:      The Shelley genesis file is read by one closed parser that requires the network magic, system start, slot length, slots per KES period and maximum KES evolutions with exact types, never assumes a default or network, ignores unknown keys only when they change nothing, converts the system start deterministically with no time library, and produces the same genesis anchor from the same file bytes and supplied KES anchor slot every time.
    counterexample: A genesis file missing its network magic is parsed as preprod, or a numeric field given as the string "1" is accepted.
    secondary:      [DET]
    scope:          genesis parsing (shell)
    assumptions:    []
    depends_on:     [VAL-001, STR-009]
    locality:       local
    legacy:         [CN-GENESIS-01, DC-GENESIS-01]

#13 DUR  CollateralValueRetained  [critical]
    statement:      When a block's phase-2-invalid transaction spends a collateral input, the UTxO authority durably records that input's coin value in the same durable transaction that applies the block's changes to the live table and advances the checkpoint cursor, so the accumulator can resolve it at any later cursor position; retention covers exactly the bindings destroyed since bootstrap, is cleared with the live table when the store resets, and lies outside the checkpoint commitment; an input that neither the block nor the authority holds resolves to nothing, and the accumulator refuses, naming the input.
    counterexample: The checkpoint deletes a spent collateral input before the accumulator, one block behind, resolves it, and the accumulator counts its value as zero.
    secondary:      [CON]
    scope:          reduced UTxO checkpoint, collateral resolution
    assumptions:    [N1]
    depends_on:     [CON-003, DET-013]
    locality:       transactional
    legacy:         [DC-LEDGER-PHASE2-04]

#14 REC  RecoveryFindsCommonAncestor  [critical]
    statement:      After any process termination, a node recovering from valid durable state finds a common ancestor with a compatible peer, given that peer's responses to its intersection requests, whenever one exists within its supported intersection history (k blocks below its durable tip, or its oldest stored block if fewer are kept), without discarding authoritative state silently and without changing replay: the ledger state, chain-dependent state and selected chain after rolling back to that ancestor and replaying forward equal those of a run that never adopted the abandoned blocks, compared at the same canonical chain points, the write-ahead log's audit history of the abandoned admissions and the rollback being excluded from the comparison. When no such ancestor exists, it stops following that peer with a structured outcome and leaves its durable state untouched; a peer's reply never instructs it to rewrite the chain back to genesis.
    counterexample: A restarted node offers the peer only its tip, gets no intersection, and resets its chain to genesis.
    secondary:      [LIV]
    scope:          recovery, chain-sync intersection
    assumptions:    [A1, A4]
    depends_on:     [#1, SAF-007]
    locality:       distributed
    legacy:         [T-REC-07]

#15 VAL  RecoveryRevalidatesStoredBlocks  [critical]
    statement:      After an unclean shutdown, every stored block that recovery applies passes through the same header and body validation authorities as live admission; no stored verdict or validity flag stands in for validation.
    counterexample: Recovery replays stored blocks by trusting the verdict tag recorded in each log entry, so a block corrupted on disk after admission is applied unvalidated.
    secondary:      [REC]
    scope:          recovery after unclean shutdown
    assumptions:    []
    depends_on:     [VAL-009, STR-004]
    locality:       local
    legacy:         [DC-STORE-06, CN-STORE-06]

#16 DET  SnapshotHeaderChecks  [critical]
    statement:      A snapshot's version tag is read and checked before any of its payload is decoded, and its fingerprint is blake2b-256 of the decoded state's canonical encoding, recomputed after decoding and compared with the stored one.
    counterexample: The decoder parses the ledger payload under the current layout first and only then looks at the version tag, so an older snapshot's bytes are misread before the version mismatch is noticed.
    secondary:      [VAL]
    scope:          state snapshots
    assumptions:    []
    depends_on:     [DET-011]
    locality:       local
    legacy:         [DC-STORE-09]

#17 VAL  WarmStartNonceFromSeedRecord  [critical]
    statement:      At warm start, the epoch nonce in the recovered chain-dependent state for the seed epoch is the nonce held in the recovered seed record, applied by the one bootstrap authority before any forging or self-acceptance; a snapshot placeholder nonce or a genesis-derived nonce never reaches VRF verification or forging.
    counterexample: A warm-started node forges with the zero nonce its ledger snapshot carried, so peers reject every block it produces.
    secondary:      []
    scope:          warm start, chain-dependent state
    assumptions:    [A2]
    depends_on:     [DUR-004, STR-010, VAL-022]
    locality:       local
    legacy:         [T-REC-04]
```

## New assumptions

```text
N1  durable-commit-honoured: a write that the storage layer reports as durably committed (a completed fsync or committed database transaction) survives process termination and power loss intact; writes not yet reported committed may be lost.
```

## Transitions in scope

- `recover(Anchor, Wal, ChainDb, Snapshots) -> Result<RecoveredState, RecoveryError>`
- `append(Wal, WalEntry) -> Result<Wal, WalError>` (append only)
- `materialize_rolled_back_state(Snapshots, ChainDb, Point) -> Result<(LedgerState, ChainDepState), MaterializeError>`
- `open_store(Path, SemanticsVersion) -> Result<Store, StoreOpenError>`
- `parse_genesis(Bytes, KesAnchorSlot) -> Result<GenesisAnchor, GenesisError>`

## TCB color hypothesis

- BLUE: `ade_ledger` (snapshot codec, replay), `ade_core::consensus` (rollback state)
- GREEN: none
- RED: `ade_runtime` (write-ahead log, chain database, snapshot storage, recovery, semantics marker check, genesis parser, reduced checkpoint storage), `ade_node` (bootstrap and recovery orchestration)

## Open questions

- S1. DC-STORE-05 (recovery is a snapshot plus forward replay, not a full replay from genesis) — drafted into #1, which requires recovery to start from the most recent valid snapshot; DET-010 carries the equivalence.
- S2. DC-STORE-06 (validate all volatile blocks after an unclean shutdown) and CN-STORE-06 (on-disk bytes re-enter through the network's canonical decode and validation chokepoints) — drafted as #15 for validation; STR-004 carries the decode chokepoint. DC-STORE-06's Haskell VolatileDB modes have no Ade counterpart.
- S3. DC-STORE-08 (canonical snapshot encoder) — merged into DET-011 SnapshotRoundTrip and DET-007 SingleProjectEncoding. DC-STORE-09 (version tag checked first; blake2b-256 fingerprint) — drafted as #16, with one departure: the tag's width (u32) is an encoding detail fixed by the codec, not part of the invariant.
- S4. CN-STORE-08 (one encoder and decoder pair per persisted state type) — merged into DET-007 SingleProjectEncoding (one encoder) and VAL-001 ClosedGrammarDecoding, whose storage clause requires exactly one decoder per Ade storage record type and format version.
- S5. T-REC-04 (the warm-start epoch nonce comes from the recovered seed record, never a placeholder or genesis; a record without it is refused as a version mismatch) — drafted as #17; STR-010 SeedRecordClosedSchema carries the version refusal.
- S6. DC-STORE-04 (a ChainDB structure of ImmutableDB, VolatileDB and LedgerDB) — excluded: it describes the Haskell node's storage layout, not a property; the immutability it implies for blocks more than k deep is A4 and SAF-007.
- S7. RO-GENESIS-REPLAY-01 (replay the chain from Byron genesis to the bootstrap point) — excluded: Ade executes only Conway and never replays historical eras (A2).
- S8. CN-SNAPSHOT-01, CN-SNAPSHOT-02, DC-SNAPSHOT-01 (the served-chain snapshot: atomic publication of forged blocks, all-or-nothing range replies, deterministic insertion) — deferred to B08 network, which covers serving.
- S10. DC-GENESIS-SRC-01 (a controlled genesis enters initial state through one bootstrap authority; a non-Conway genesis fails closed) — excluded: A2, as approved, has Ade enter only at a certified non-Origin Conway tip, so a genesis start contradicts it. Allowing a genesis start, for example for a private testnet, needs a redefinition of A2 decided by the human; this batch does not propose one.
- S11. #6 extends T-REC-06's nonce guarantee from the seed epoch to every epoch replayed, consistent with VAL-022, which defines the nonce of each epoch.
- S9. DC-WAL-05's memory clause (the admission runner holds at most one block's bytes at a time) — deferred to B12 resources-release with the memory invariants.
```
