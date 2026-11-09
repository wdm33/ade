# Define — consensus

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B05 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers chain selection, convergence, header binding, the chain-dependent state, rollback and replay equivalence, snapshots, the era schedule and forecast range, operational-certificate counters, and lockstep admission. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 39 legacy IDs appears on a line or in a settlement. Forging, serving and importing entries that arrived in the consensus area are deferred to the batches that own those subjects.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants and assumptions that lines and settlements cite)

## Drafts

```text
#1  VAL  ChainSelectionMatchesReference  [critical]
    statement:      Given the same candidate chains and consensus state, Ade selects the same chain as the reference node's Praos chain selection: the higher block number wins, an equal block number is decided by the reference Praos tiebreak over slot, issuer, operational-certificate issue number and VRF output, density is never used for caught-up selection, and the decision depends on nothing but those canonical observables and the current selection.
    counterexample: Two candidate chains of equal block number from different pools, where Ade prefers the one whose header arrived first while the reference node prefers the one with the lower VRF tiebreak value.
    secondary:      [ORD]
    scope:          chain selection
    assumptions:    [A2]
    depends_on:     [STR-001, STR-002]
    locality:       local
    legacy:         [T-CONS-01, CN-CONS-01, DC-CONSENSUS-01, DC-CONS-03]

#2  LIV  PartitionConvergence  [critical]
    statement:      If, after a network partition heals, the node keeps receiving from at least one peer that offers the chain the network is converging on (N1), and that chain forks from the node's selection at most k blocks deep, then the node's selected chain eventually equals the chain the reference selection rule prefers among the valid chains delivered to it that #5 allows it to adopt, its current selection included, with ties between equally preferred chains settled as the reference rule settles them; it chooses using only protocol-defined observables.
    counterexample: After a partition heals and the honest chain has been delivered, Ade keeps its own shorter fork indefinitely because it treats blocks it forged as preferred.
    secondary:      [FIN]
    scope:          chain selection across peers
    assumptions:    [A2, A4, N1]
    depends_on:     [#1, #5]
    locality:       distributed
    legacy:         [CN-CONS-03]

#3  VAL  HeaderBindsBodyAndContext  [critical]
    statement:      A block is accepted only if its header binds exactly its body and its chain position: the header's body hash equals the hash of the body bytes as received, and its previous-block hash, block number and slot agree with the chain point and consensus state it is validated against.
    counterexample: A peer delivers a valid header with the body of a different block, or a header whose block number skips one, and the block is accepted.
    secondary:      []
    scope:          header validation
    assumptions:    [A2]
    depends_on:     [DET-002, VAL-019]
    locality:       local
    legacy:         [CN-CONS-04]

#4  STR  ChainDepStateOwnedByConsensus  [critical]
    statement:      The Praos chain-dependent state (its nonces, operational-certificate counters and last slot) is owned by the consensus layer, not the ledger. Moving forward along a chain, it changes only through validated headers and epoch ticks; a rollback or a verified snapshot restore sets it to the chain-dependent state of the restored point.
    counterexample: The ledger's block application writes the operational-certificate counter map directly, outside the consensus header transition.
    secondary:      [DET]
    scope:          Praos chain-dependent state
    assumptions:    []
    depends_on:     [VAL-022]
    locality:       local
    legacy:         [DC-CONS-04]

#5  SAF  RollbackBoundedByK  [critical]
    statement:      No rollback deeper than k blocks, counted in blocks and with k from the network's genesis, is ever applied: a deeper request, or one crossing the immutable tip, is rejected with a typed error and changes nothing; forecast and stability windows, though measured in slots, always cover at least k+1 blocks.
    counterexample: A peer's fork that diverges 2,161 blocks back on mainnet (k = 2160) is adopted, or a rejected deep rollback leaves the chain database truncated.
    secondary:      [FIN]
    scope:          rollback and chain selection
    assumptions:    [A2]
    depends_on:     [SAF-001]
    locality:       transactional
    legacy:         [DC-CONS-05]

#6  DET  RollbackAndReplayEquivalence  [critical]
    statement:      Rolling back to a point, or restoring a snapshot and replaying forward from it, yields state byte-identical to applying the same blocks directly from the bootstrap baseline; snapshots and replay are caches of the direct application, never a separate authority, and a replay over an epoch boundary goes through the one epoch-boundary transition.
    counterexample: Restoring a snapshot taken mid-epoch and replaying across the next boundary skips the boundary's reward update, giving a different state fingerprint from direct application.
    secondary:      [REC]
    scope:          rollback, snapshot restore, replay forward
    assumptions:    [A1, A2]
    depends_on:     [DET-001, VAL-020]
    locality:       local
    legacy:         [DC-CONS-06, DC-CONS-22]

#7  DET  SnapshotRoundTrip  [critical]
    statement:      Every persisted ledger and chain-dependent state snapshot begins with a closed version tag and embeds the state's fingerprint, encodes canonically, and decodes to a state with that same fingerprint; an unknown version or a fingerprint mismatch is rejected.
    counterexample: A snapshot written by an older format version is decoded under the new layout and silently yields a state with a different fingerprint.
    secondary:      [VAL]
    scope:          state snapshots
    assumptions:    [A1]
    depends_on:     [DET-007, VAL-001]
    locality:       local
    legacy:         [DC-CONS-21]

#8  STR  EraScheduleAnchored  [critical]
    statement:      The core receives the era schedule only as a typed value bound to the bootstrap anchor hash and recorded in replay evidence; genesis files and configuration are parsed only in the shell, and no core path reads files, JSON or operator configuration.
    counterexample: A consensus function parses the Shelley genesis JSON itself to find the epoch length, or uses an era schedule not bound to the anchor the node bootstrapped from.
    secondary:      [DET]
    scope:          era schedule, shell/core boundary
    assumptions:    [A1, A2]
    depends_on:     [STR-002, VAL-012]
    locality:       local
    legacy:         [DC-CONS-07]

#9  VAL  ForecastRangeEnforced  [critical]
    statement:      A consensus query for a slot beyond the ledger view's safe forecast range returns a typed out-of-range result, never a guessed value; the bound is derived from the era history, the safe zone and the era schedule, never written as a constant by the caller.
    counterexample: Leader-schedule data for a slot two stability windows ahead is computed from the current epoch's stake distribution instead of being refused.
    secondary:      [SAF]
    scope:          consensus forecasting
    assumptions:    [A2]
    depends_on:     [#8]
    locality:       local
    legacy:         [DC-CONS-09]

#10 SEC  OpCertCounterRule  [critical]
    statement:      A header is valid only under the reference OCERT rule: its pool's current issue number is the one recorded in the chain-dependent state, or 0 when none is recorded and the pool is in the epoch's stake distribution, and a header from a pool with neither is invalid; the header's operational-certificate issue number must be at least the current one and at most one more than it, otherwise the header is invalid with a typed reason. Counters are keyed by pool; a forward header transition never lowers a pool's recorded number, and a rollback or verified snapshot restore sets the counters to those of the restored point.
    counterexample: A header using an old operational certificate whose issue number is below the pool's recorded one is accepted, letting the holder of a stolen, superseded hot key forge for the pool; or a header whose issue number jumps by two is accepted.
    secondary:      [VAL]
    scope:          header validation, chain-dependent state
    assumptions:    [A2]
    depends_on:     [#4]
    locality:       local
    legacy:         [DC-CONS-10]

#11 SEC  AnnouncedHeaderMatchesDelivered  [critical]
    statement:      When a peer delivers a block, the header bytes in the delivered block are byte-for-byte equal to the header bytes that peer announced for the same slot and hash; a peer cannot switch headers between announcement and delivery.
    counterexample: A peer announces a header that passes validation, then delivers a block carrying a different header with the same hash claim, and the block is admitted.
    secondary:      [VAL]
    scope:          receive path (chain-sync announcement, block-fetch delivery)
    assumptions:    []
    depends_on:     [#3]
    locality:       local
    legacy:         [DC-CONS-19]

#12 REC  LockstepAdmissionAndRollback  [critical]
    statement:      A successful block admission advances the chain database, the ledger state and the Praos chain-dependent state as one transition, and a rollback returns all three to the same point; no path leaves them at different points, admits partially or rolls back partially.
    counterexample: A rollback truncates the chain database to slot 1000 but leaves the chain-dependent state at slot 1010, so the next header's op-cert counter check uses counters from the abandoned blocks.
    secondary:      [SAF]
    scope:          block admission and rollback
    assumptions:    [A1]
    depends_on:     [SAF-001, REC-002]
    locality:       transactional
    legacy:         [DC-CONS-20]

#13 DET  ConsensusIndependentOfArrival  [critical]
    statement:      Every consensus decision (header validity, chain selection, leadership, rollback) is a function of the consensus state, the current selection, the set of candidates delivered, the protocol version and the recorded explicit context, the current slot included; with all of those equal, the decision is the same whatever order peers' messages arrived in, however tasks were scheduled, and whatever the wall clock or OS reported.
    counterexample: Two runs receive the same two equal-preference candidates from different peers in opposite order and select different chains, or the shell drops a candidate because its peer's message arrived after a timer fired.
    secondary:      [STR]
    scope:          consensus decisions, shell/core boundary
    assumptions:    [A1]
    depends_on:     [STR-001, STR-002, #1]
    locality:       local
    legacy:         [T-CONS-02, CN-CONS-05]

#14 SEC  RolloutSkewCannotSplit  [critical]
    statement:      No input, adversarial or not, can lead two supported Ade releases authorized to run on the network at the same time, starting from the same state and receiving the same inputs, to different authoritative states or different selected chains.
    counterexample: A crafted block that both releases accept is applied with different ledger effects by each, so their nodes persistently disagree on later blocks.
    secondary:      [EVO]
    scope:          Ade release compatibility
    assumptions:    [A2]
    depends_on:     [EVO-003, DET-001]
    locality:       distributed
    legacy:         [CN-CONS-02]
```

## New assumptions

```text
N1  honest-peer-delivery: after a network partition heals, the node stays connected to at least one peer that offers the chain the network is converging on, and messages from that peer are delivered and processed within a bounded time.
```

## Transitions in scope

- `select_chain(ChainState, Candidates, Context) -> Result<(ChainState, Effects), SelectionRejection>`
- `validate_header(ChainDepState, LedgerView, Header) -> Result<ChainDepState, HeaderRejection>`
- `rollback(ChainDb, LedgerState, ChainDepState, Point) -> Result<(ChainDb, LedgerState, ChainDepState), RollbackRejection>`
- `forecast(LedgerView, EraSchedule, SlotNo) -> Result<ViewAtSlot, OutsideForecastRange>`

## TCB color hypothesis

- BLUE: `ade_core::consensus` (chain selection, header validation, chain-dependent state, forecasting), `ade_ledger` (snapshots, replay)
- GREEN: `ade_testkit` consensus stream replay
- RED: `ade_runtime` (chain database, rollback orchestration, genesis parsing, receive bridge)

## Open questions

- S1. T-CONS-02, CN-CONS-05 (consensus decisions never depend on wall-clock time, arrival-order races, scheduler interleavings or OS behaviour) — drafted as #13: STR-001 and STR-002 cover what happens inside the core and on one ordered input stream, not the choice among candidates the shell delivers.
- S2. DC-CONSENSUS-02 (leadership verification is pure) and DC-CONS-08 (slot-to-time is pure; no wall clock) — merged into STR-001.
- S3. CN-CONS-02 (supported rollout skew never lets one adversarial input cause persistent divergence) — drafted as #14: EVO-003 covers verdicts only, not resulting state or chain selection.
- S4. DC-CONS-11 (the forged header's op-cert KES period is computed from the slot under an anchor supplied on the producer tick, never inferred from the clock or filesystem, and a mismatch is rejected at forge time) — deferred to B10 pipeline-production with the other forging entries.
- S11. #10 departs from DC-CONS-10 in two ways, both toward the reference OCERT rule: counters are keyed by pool, not by pool and KES period, and an issue number more than one above the current one is invalid. If Ade's code keys by KES period or lacks the upper bound, #10 is recorded as falsified at the re-evidence pass.
- S12. #2 drops the deep-fork clause, that a fork deeper than k is never adopted automatically, because #5 RollbackBoundedByK already states it.
- S5. A4 (rollback-depth-bound-k) stays as recorded. #5 is the invariant that enforces it, so evidence for #5 discharges A4. Retiring A4 would require redefining DET-009, REC-001 and LIV-001, which cite it, for no change in meaning.
- S6. Forging entries — deferred to B10 pipeline-production: DC-CONS-12 (op-cert serial monotonic per cold key), DC-CONS-13 (forge is pure), DC-CONS-14 (forge byte-equal across replays), DC-CONS-15 (forge only when the leader check passes), DC-CONS-16 (forged body hash from the validator's encoder), CN-CONS-07 (self-acceptance before broadcast; serve provenance), CN-CONS-08 (receive-side single admission authority), DC-CONS-23 (own-forged block admitted only if it extends the durable tip), DC-CONS-24 (forged parent equals the peer-visible tip).
- S7. Serving entries — deferred to B08 network: DC-CONS-17 (served block bytes equal the accepted block's bytes), DC-CONS-18 (announced header and served body cohere).
- S8. CN-CONS-06 (blocks Ade forges are accepted by a real cardano-node) — deferred to B12 resources-release, with the live acceptance criteria.
- S9. Live consensus-input importer entries (CN-CONS-IN-01, DC-CONS-IN-01, DC-CONS-IN-02) — deferred to B06 consensus-inputs.
- S10. JSON UTxO seed importer entries (CN-SEED-01, DC-SEED-01) — deferred to B11 mithril-evidence, which covers bootstrap import.
```
