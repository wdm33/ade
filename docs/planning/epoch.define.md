# Define — epoch

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B04 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers the epoch boundary: agreement with the reference node's boundary transition, governance timing, the Praos nonce, the epoch view Ade derives and activates for each next epoch, rewinding and refolding the epoch accumulator after a rollback, and the reward and snapshot rules. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 54 legacy IDs (52 assigned, plus DC-LEDGER-04 and DC-GOV-01 deferred here by the ledger batch) appears on a line or in a settlement.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants that settlements merge into)

## Drafts

```text
#1  VAL  EpochBoundaryMatchesReference  [critical]
    statement:      For the same prior ledger state, Ade's epoch-boundary transition produces the same new state as the reference node's NEWEPOCH transition, computed solely from canonical chain state and applied in the reference order: the reward update is applied before the new stake snapshot is taken; pool reaping adopts staged re-registrations, reaps exactly the pools retiring at this epoch and returns each reaped pool's deposit to its reward account if registered, otherwise to the treasury; and the rewards, snapshots, treasury, reserves, deposit pots and governance state all match.
    counterexample: Ade reaps a pool whose retirement epoch is earlier than the current one a second time, or takes the stake snapshot before applying the reward update, so its next stake distribution differs from the reference node's.
    secondary:      [CON]
    scope:          ledger epoch-boundary transition
    assumptions:    [A2]
    depends_on:     [CON-001, DET-001]
    locality:       transactional
    legacy:         [DC-LEDGER-04, CN-EPOCH-03, DC-EPOCH-21, DC-EPOCH-18, DC-EPOCH-43]

#2  STR  SingleEpochAuthority  [critical]
    statement:      At every slot exactly one authoritative epoch view exists, holding that slot's epoch's leadership stake distribution, epoch nonce, constitutional committee, governance state and protocol parameters; one owner holds it, header validation and leadership both read it afresh for each decision, and crossing an epoch boundary replaces it in one atomic transition.
    counterexample: Header validation reads a view built at startup while leadership reads the promoted view, so after a boundary the node validates headers against one epoch's stake and checks its own leadership against another's.
    secondary:      [SAF]
    scope:          consensus epoch authority (header validation and leadership)
    assumptions:    [A2]
    depends_on:     [SAF-001, EVO-001]
    locality:       transactional
    legacy:         [T-EPOCH-01, CN-EPOCH-02, DC-EPOCH-14]

#3  ORD  EffectsActivateAtBoundaries  [critical]
    statement:      Stake distribution changes, rewards, protocol-parameter changes and governance effects take effect only at the epoch boundaries where the protocol defines them to, never in the middle of an epoch.
    counterexample: A ratified parameter change is applied to the ledger as soon as its last vote arrives, mid-epoch, instead of at the next boundary.
    secondary:      []
    scope:          ledger and governance effects
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [CN-EPOCH-01]

#4  VAL  RatificationFrozenAtPriorBoundary  [critical]
    statement:      The ratification result applied at the boundary from epoch E to E+1 is decided once from the state the boundary from E-1 to E produced (that boundary's stake distributions, with proposal deposits counted, and epoch E's proposals, votes, DRep registrations, committee, thresholds and parameters), recorded durably and commitment-bound in the governance state, and applied unchanged: the applying step reads no votes, stake, committee, thresholds or expiry, and a record frozen at another epoch, a missing record or an unenactable record halts with no state change. Enactment applies every effect of each ratified action together at that boundary, never part of them.
    counterexample: Ratification at the E to E+1 boundary counts votes cast after the E-1 to E snapshot against a stake distribution recomputed at E+1, or an enacted committee update changes the committee members but not the quorum.
    secondary:      [ORD]
    scope:          Conway governance ratification and enactment
    assumptions:    [A2]
    depends_on:     [#1, #3]
    locality:       transactional
    legacy:         [DC-EPOCH-01, DC-EPOCH-47]

#5  CON  ProposalDepositReturn  [critical]
    statement:      At each epoch boundary, every governance proposal that leaves the proposal set (enacted, expired, or removed because a sibling or ancestor was enacted or expired) has its deposit returned exactly once: to its return reward account if that account is registered, otherwise to the treasury.
    counterexample: An expired proposal's deposit stays in the deposit pot, or is credited to an unregistered return account where no one can withdraw it, or is returned at two successive boundaries.
    secondary:      [VAL]
    scope:          Conway governance deposits
    assumptions:    [A2]
    depends_on:     [#4, CON-001]
    locality:       transactional
    legacy:         [DC-GOV-01]

#6  VAL  PraosNonceEvolution  [critical]
    statement:      Each validated header drives one indivisible nonce transition as the reference Praos protocol defines it: the evolving nonce becomes the evolving nonce combined with the header's VRF nonce output; the candidate nonce becomes the new evolving nonce while the header's slot is below firstSlotNextEpoch minus ceil(4k/f), and otherwise stays frozen; the last-block nonce becomes the hash of the header's previous block. At each epoch tick the epoch nonce becomes the candidate nonce combined with the last-epoch-block nonce, the previous epoch nonce becomes the old epoch nonce, the last-epoch-block nonce becomes the last-block nonce, and the evolving and candidate nonces carry over unchanged. Combining is blake2b-256 of the concatenation, with the neutral nonce as identity and no extra-entropy operand; an absent last-epoch-block nonce fails closed unless a valid bootstrap bridge supplies it, and no nonce is ever fabricated. The first epoch nonce after bootstrap comes from that same tick over the live chain state; a seed-time nonce stored at bootstrap must equal it when the seed sat at or after the freeze slot and is not used otherwise, and a seed position that cannot be located relative to the freeze, or a network whose stability window is unknown, is terminal.
    counterexample: The candidate nonce keeps evolving after the stability-window slot, or the epoch nonce after bootstrap is copied from a value projected at snapshot time while the candidate was still moving, so the node's leader checks use a nonce the network does not.
    secondary:      [DET]
    scope:          Praos chain-dependent nonce state
    assumptions:    [A2]
    depends_on:     [SEC-004]
    locality:       local
    legacy:         [DC-EPOCH-16, DC-EPOCH-38]

#7  SAF  AtMostOneActivationPerEpoch  [critical]
    statement:      Within one canonical chain history, at most one epoch view activates for each target epoch, and its durable activation record holds the view's complete identity: target epoch, network, era, transition point, source checkpoint commitment, snapshot phase, nonce commitment and the view's canonical hashes. A second record for the same target epoch on the same history is an idempotent replay if it is byte-identical to the first and a terminal conflict otherwise. A rollback that abandons an activation's transition point invalidates that activation record with the rollback, before any replacement activation for the epoch, and the record is never reused (#16).
    counterexample: After a restart the node activates a second, different view for an epoch whose view was already activated, or the activation record holds only a hash and a point, so a different view with a colliding point would be accepted on recovery.
    secondary:      [FIN]
    scope:          epoch-view activation
    assumptions:    []
    depends_on:     [#2]
    locality:       transactional
    legacy:         [DC-EPOCH-04]

#8  ORD  ActivationAdvancesOneBoundary  [critical]
    statement:      The active epoch view moves forward one boundary at a time, from the view for epoch P to the view for epoch P+1 only; decisions for epoch N+1 never read epoch N's view, and a candidate slot two or more epochs past the active view is a terminal failure, never a skip.
    counterexample: After a long outage the node promotes the view for epoch P+2 directly, skipping P+1, or validates an epoch N+1 header with epoch N's stake distribution because the N+1 view was not ready.
    secondary:      []
    scope:          epoch-view activation and promotion
    assumptions:    []
    depends_on:     [#2, #7]
    locality:       local
    legacy:         [DC-EPOCH-05, DC-EPOCH-17]

#9  SAF  ActivationFailsClosed  [critical]
    statement:      Only a genuine epoch transition, decided positively from the block's epoch against the accumulator's epoch before applying the block, enters the boundary machinery; a missing, stale, conflicting or mismatched candidate view is a terminal halt, never a fallback to another view, and an apply failure within an epoch never triggers boundary actions.
    counterexample: A within-epoch transaction failure is read as "boundary due" and rewinds the reduced checkpoint, or a candidate whose bindings do not match the selected chain is activated anyway because no other view is available.
    secondary:      [VAL]
    scope:          epoch-view activation, accumulator advance
    assumptions:    []
    depends_on:     [#8, SAF-002]
    locality:       local
    legacy:         [DC-EPOCH-07, DC-EPOCH-39]

#10 DUR  ActivationDurableBeforeVisible  [critical]
    statement:      An epoch view becomes the active view only after its activation record is durable, through one ordered path: take the candidate (for an epoch past the second bootstrap epoch, the sealed leadership object for exactly that epoch with its stored provenance; for the two bootstrap epochs, the certified bootstrap seeding), check the activation predicate (the transition is due and the candidate's bindings match the selected chain), write the activation record durably, then publish; recovery restores exactly the durably recorded activation, rebuilding the view and requiring it to reproduce the record, and with no record the previous view stays active.
    counterexample: The node publishes the promoted view, then crashes before the activation record reaches disk, and after restart validates headers against the old epoch's view for slots it had already judged under the new one.
    secondary:      [REC]
    scope:          epoch-view activation, recovery
    assumptions:    [A1]
    depends_on:     [#7]
    locality:       transactional
    legacy:         [DC-EPOCH-06, DC-EPOCH-10]

#11 VAL  LeadershipSealedAtBoundaryPoint  [critical]
    statement:      At each epoch boundary from E to E+1 that the node crosses itself, the leadership stake distribution for epoch E+2 is sealed as a frozen leadership object in the same durable commit as the accumulator advance, and no boundary advance commits without it: its pool set is the registered pools with at least one delegator in the delegation map before pool reaping, its stake comes from the stake mark captured from the reduced checkpoint positioned exactly at the closing epoch's last durable block (bound to that point and its lineage before use), and its VRF keys come from the pool parameters frozen before reaping. The two epochs no crossing produces, the bootstrap epoch and the one after it, are sealed instead from the certified snapshot's pool distribution and its imported mark snapshot, bound to the certified bootstrap point.
    counterexample: The stake mark is read from the checkpoint after it has advanced past the boundary into the next epoch; or a retired pool that still has delegators is left out of the sealed set because its VRF key was looked up in the active pool parameters at sealing time.
    secondary:      [ORD]
    scope:          epoch-view derivation, boundary mark capture
    assumptions:    [A2]
    depends_on:     [#1, #2]
    locality:       transactional
    legacy:         [DC-EPOCH-08, DC-EPOCH-09, DC-EPOCH-22, DC-EPOCH-32]

#12 VAL  LeadershipFromSealedView  [critical]
    statement:      The stake distribution used for a leader check in epoch E comes only from the sealed frozen leadership object whose target epoch is exactly E, read after its protocol-parameter commitment is verified; it is never re-derived at use time, never taken from a latest or nearest object, and never replaced by window replay or the bootstrap view, and a missing, unsealed or malformed object fails closed. Its sigma denominator is the snapshot's total active stake recorded when the object was sealed, unchanged by which pools the object lists, never re-summed from its entries, and a missing total fails closed.
    counterexample: Leadership computes the denominator by summing the listed pools, so a positive-stake pool filtered out of the list shrinks the denominator and raises every other pool's leader probability; or a missing object for epoch E is answered with the object for E-1.
    secondary:      []
    scope:          leadership stake distribution
    assumptions:    [A2]
    depends_on:     [#2, #11]
    locality:       local
    legacy:         [DC-EPOCH-12, DC-EPOCH-25, DC-EPOCH-40]

#13 ORD  ForecastHorizonFollowsPromotion  [critical]
    statement:      The era schedule's forecast horizon extends past an epoch boundary if and only if the next epoch's view has been durably promoted, and the extension is part of the same transition as the promotion; a header at a slot beyond the horizon is rejected as out of forecast range before any leader view is read. The extended schedule is derived state: it is computed from the durable activation record, the recovered promoted view and the committed network geometry and profile, never persisted as a separate authority, and recovery reconstructs a schedule identical to the one before the restart, whether or not the promotion had happened; no flag, clock or peer input affects it.
    counterexample: The horizon is extended into epoch N+1 before its view is promoted, so an N+1 header is validated with epoch N's stake distribution.
    secondary:      [SAF]
    scope:          era-schedule forecast, header validation
    assumptions:    [A2]
    depends_on:     [#10]
    locality:       transactional
    legacy:         [DC-EPOCH-15]

#14 VAL  LedgerEpochAgreesWithSchedule  [critical]
    statement:      After the epoch-boundary decision for a block, the ledger's epoch equals the network's era-schedule epoch for that block's slot; a disagreement in either direction is a typed failure that halts the apply with no state change.
    counterexample: The ledger's epoch runs ahead of the schedule's after a mis-detected boundary, and since boundary detection only fires when the schedule is ahead, the node keeps applying blocks in the wrong epoch.
    secondary:      [SAF]
    scope:          ledger epoch, era schedule
    assumptions:    [A2]
    depends_on:     [VAL-012, SAF-001]
    locality:       local
    legacy:         [DC-EPOCH-36]

#15 REC  RewindTargetSettledAndCanonical  [critical]
    statement:      The epoch accumulator is rewound only to a settled point that is at least k blocks below the durable tip, counted in blocks, at or before the rollback target, and whose header hash is still canonical at its slot; any other target is refused and the accumulator resets to its certified bootstrap baseline. A rewound store is uncertified until a canonical refold rewrites its anchor, and a staged point taken on the abandoned chain is discarded.
    counterexample: After a rollback the accumulator is restored to a staged point that lay on the abandoned fork, or to a point five blocks below the tip, which a further admissible rollback could still remove.
    secondary:      [SAF]
    scope:          epoch accumulator rewind
    assumptions:    [N1]
    depends_on:     [#16]
    locality:       local
    legacy:         [DC-EPOCH-26, DC-EPOCH-27, DC-EPOCH-29]

#16 DET  RewindReplayEquivalence  [critical]
    statement:      Refolding the epoch accumulator from a settled rewind point produces state byte-identical to folding from the bootstrap baseline over the same canonical chain; re-deriving an epoch boundary over the same canonical source prefix re-seals a frozen leadership object byte-identical to the one first sealed; and no leadership object or activation record derived from a prefix that a rollback abandoned is ever reused.
    counterexample: A refold from the settled point re-seals a leadership object whose pool map differs from the original, so the durable activation record can no longer be reproduced on recovery.
    secondary:      [REC]
    scope:          epoch accumulator refold, leadership sealing
    assumptions:    [A1, N1]
    depends_on:     [DET-001]
    locality:       local
    legacy:         [DC-EPOCH-31, DC-EPOCH-33]

#17 SAF  RewindRestoresCoherentLeadership  [critical]
    statement:      A rewind restores the sealed leadership table to exactly the epochs valid at the rewind point, together with the accumulator in one atomic restore, so no sealed leadership object is ahead of the refolded accumulator.
    counterexample: The accumulator is rewound to epoch 300 while the leadership table keeps an object sealed for epoch 302, which the node then uses for leader checks.
    secondary:      [DET]
    scope:          epoch accumulator rewind, leadership table
    assumptions:    []
    depends_on:     [#15]
    locality:       transactional
    legacy:         [DC-EPOCH-28]

#18 LIV  BoundedRefold  [critical]
    statement:      When a settled rewind point exists and every proof about it holds (canonical at its slot, at least k blocks below the tip, at or before the rollback target, fingerprint verified), re-deriving the epoch accumulator after a rollback replays at most 2k blocks, independent of how long the node has run: once the chain has advanced 2k blocks past bootstrap, the settled point is between k and 2k blocks below the tip, and it survives the recovery pass that follows a durable rollback, being re-proved against the chain as it stands after the rollback commits rather than discarded. Before a settled point exists, or when a proof fails, the refold starts from the certified bootstrap baseline (#15, #19).
    counterexample: Every rollback rewinds to the bootstrap anchor, so after weeks of uptime a one-block reorg costs a ten-hour refold and the node misses its leadership slots; or the recovery pass after a rollback deletes a valid settled point and refolds from bootstrap.
    secondary:      [REC]
    scope:          epoch accumulator rewind and recovery
    assumptions:    [A1, A2, N1]
    depends_on:     [#15, #19]
    locality:       local
    legacy:         [DC-EPOCH-30, DC-EPOCH-35]

#19 DUR  SettledTripleIntegrity  [critical]
    statement:      The settled rewind state (accumulator, settled point and settled leadership table) is bound by a domain-separated, length-prefixed fingerprint written in the same durable commit that promotes it, and the fingerprint is recomputed and compared before the state is restored; an absent fingerprint, unreadable context or mismatch refuses the restore and resets to the certified bootstrap baseline.
    counterexample: A settled accumulator blob truncated by a crash is restored without its fingerprint being checked, and the node folds forward from corrupt state.
    secondary:      [REC]
    scope:          settled rewind persistence
    assumptions:    []
    depends_on:     [#15]
    locality:       transactional
    legacy:         [DC-EPOCH-34]

#20 DUR  SelfSufficientEpochState  [critical]
    statement:      After every durable selected-chain block, the node holds enough durable, replayable state to derive every future epoch transition (rewards, stake snapshots, pool and certificate lifecycle, leadership authority) without another snapshot import, an external oracle or manually supplied authority. That state is kept in bounded memory: the UTxO stays in the disk-backed reduced checkpoint, never as a full in-memory map; the accumulator is persisted incrementally, with a boundary checkpoint and at most one epoch of within-epoch replay; and no block requires replaying the whole chain or copying the whole accumulator.
    counterexample: Deriving the leadership authority two epochs after bootstrap requires re-importing a Mithril snapshot because the node never persisted the pool-retirement state the boundary needs.
    secondary:      [LIV]
    scope:          epoch accumulator, durable state
    assumptions:    [A1, A2]
    depends_on:     [#1, DET-001]
    locality:       local
    legacy:         [DC-EPOCH-19]

#21 REC  SingleAdmissionPrefix  [critical]
    statement:      The write-ahead log's tail defines the durable selected-chain prefix: the chain database, the Praos chain-dependent state, the epoch accumulator and the reduced UTxO checkpoint are each derived from that one prefix, in chain order, each advancing only after the block is durably admitted, and recovery drops anything above the tail and re-derives any of them that lags or leads it, never resuming with them on different prefixes. The reduced checkpoint is a deterministic projection of the ledger UTxO, so the same blocks give a byte-identical checkpoint, and a missing, corrupt or lagging checkpoint blocks epoch-view production and fails closed.
    counterexample: After a crash the accumulator has applied block 1001 while the WAL tail is block 1000, and the node resumes with the accumulator one block ahead of the chain-dependent state.
    secondary:      [DUR]
    scope:          block admission, recovery
    assumptions:    [A1]
    depends_on:     [SAF-001, DET-001]
    locality:       transactional
    legacy:         [DC-EPOCH-20, DC-EPOCH-11]

#22 VAL  SnapshotNonZeroActiveStake  [critical]
    statement:      A stake snapshot includes a registered, delegated stake credential if and only if its combined active stake (base-address UTxO coin plus reward balance) is non-zero, so a pool whose delegators all hold zero stake is absent from the snapshot.
    counterexample: A pool whose only delegator has a zero balance appears in Ade's snapshot with zero stake, while the reference node's snapshot omits it.
    secondary:      []
    scope:          stake snapshots (mark, set, go)
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [DC-EPOCH-24]

#23 VAL  OwnerStakeSelfDelegated  [critical]
    statement:      In the reward calculation a pool's owner stake is the go-snapshot stake of only those owners delegated to that pool in the go snapshot, defined in one place and used for both the pledge check and the leader reward; an owner delegated elsewhere, or not at all, contributes nothing.
    counterexample: An owner delegated to a different pool is counted toward this pool's pledge, so a pool that fails its pledge receives rewards.
    secondary:      []
    scope:          epoch-boundary rewards
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [DC-EPOCH-42]

#24 VAL  FrozenPoolParameters  [critical]
    statement:      Every stake snapshot carries, for each pool registered when it was taken, that pool's pledge, cost, margin, owners and reward account as they were at that moment, taken at the snapshot step before future-pool adoption and pool reaping. A seed import decodes them from the certified snapshot, checks each entry's derivable values (stake, self-delegated owner stake) against the snapshot, and refuses a staked pool with no entry. The reward calculation reads pool parameters only from the go snapshot, never from the registrations current at the boundary, and a pool with blocks and go-snapshot stake but no frozen parameters is a typed terminal, never skipped.
    counterexample: The reward calculation for epoch N uses a pool's margin as re-registered in epoch N+1 instead of the margin frozen in the snapshot.
    secondary:      []
    scope:          stake snapshots, rewards, seed import
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [DC-EPOCH-44]

#25 CON  DonationPot  [critical]
    statement:      Ade holds exactly one treasury-donation pot with exactly three transitions: a phase-2-valid Conway transaction adds its donation, a phase-2-invalid one adds nothing, and the epoch boundary moves the whole pot into the treasury exactly once, as its last treasury step, and empties it. Every bootstrap takes the pot from the certified snapshot's state and refuses a snapshot without it, never defaulting it to zero; the pot is never an input to ratification, enactment or withdrawal affordability; and an overflow is a typed terminal, never a saturation.
    counterexample: A donation is credited to the treasury immediately on the transaction and again at the boundary, or a phase-2-invalid transaction's donation is added to the pot.
    secondary:      []
    scope:          treasury donations
    assumptions:    [A2]
    depends_on:     [CON-001, VAL-016]
    locality:       transactional
    legacy:         [DC-EPOCH-45]

#26 VAL  SeedBoundaryRewardAuthority  [critical]
    statement:      The reward update applied at the first boundary after bootstrap has exactly one source, chosen by the certified snapshot's reward-update state and recorded as a closed sum inside the durable record's commitment, so an absent update is never represented as a zero-valued one: a completed update is applied once, reducing the fee pot by its fee value; when no update was started, the ordinary reward calculation runs from the snapshot's inputs, the same function every later boundary uses, and the imported fee pot is reduced by the snapshot's fee value before rotation; a snapshot taken while the update was still being computed is refused. A native calculation that yields a zero pool pot while the previous epoch's block counts are non-empty is terminal, and no imported reward delta is applied except from a completed imported update.
    counterexample: The bootstrap boundary applies the snapshot's completed reward update but not its fee-pot reduction, so the fee pot frozen at the next snapshot is too large; or a snapshot taken mid-calculation is accepted and its partial update applied.
    secondary:      [CON]
    scope:          first post-bootstrap epoch boundary
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       transactional
    legacy:         [DC-EPOCH-41, DC-EPOCH-23, DC-EPOCH-18]

#27 PRC  PerNetworkEpochEvidence  [standard]
    statement:      Epoch derivation, boundary firing (exactly at the first slot of a new epoch), ledger-schedule agreement and the nonce and candidate-freeze surface are checked by a differential suite on every network Ade supports, over that network's real geometry and across bootstrap positions before and after the stability window, never inferred from a mainnet-shaped corpus alone.
    counterexample: Epoch arithmetic tested only against mainnet blocks passes while preprod's different Shelley start yields a fictitious epoch number.
    secondary:      []
    scope:          epoch and nonce evidence
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       local
    legacy:         [DC-EPOCH-37, DC-EPOCH-38]

#28 LIV  CoAdvanceRetryOnce  [critical]
    statement:      A co-advance pass never returns with leadership authority stale only because the reduced checkpoint had not yet reached data the accumulator needs: exactly one failure is retryable, an unresolved collateral input at a block the checkpoint has not reached within the accumulator's own epoch, and the pass then advances the checkpoint to that block and retries once in the same pass. Every other apply failure, a retry that fails again and a walk fault halt the pass with a typed cause; the authoritative walk has no observe-only exit.
    counterexample: The accumulator fails on an unresolved collateral input the checkpoint was about to provide, the pass returns, and leadership stays stale until the next pass, missing a leadership slot.
    secondary:      []
    scope:          accumulator and checkpoint co-advance
    assumptions:    []
    depends_on:     [#21]
    locality:       local
    legacy:         [DC-EPOCH-46]

#29 EVO  ProtocolVersionChangesAtBoundaries  [critical]
    statement:      A protocol-version change takes effect only at the epoch boundary where the ratified hard-fork action that sets it is enacted, and the era schedule never forecasts past a boundary at which a protocol-version change is not yet fixed.
    counterexample: A ratified hard-fork action raises the protocol version mid-epoch, or the schedule forecasts slots of the next epoch under the old version while a hard fork enacted at that boundary is still possible.
    secondary:      [ORD]
    scope:          protocol versions, era schedule forecast
    assumptions:    [A2]
    depends_on:     [#3, #4, EVO-001]
    locality:       local
    legacy:         [DC-EPOCH-02]

#30 VAL  VoteDelegationLifecycle  [critical]
    statement:      Vote delegations follow the protocol-version-10 lifecycle: unregistering a stake account removes its vote delegation, unregistering a DRep removes every delegation to it, a delegation to an unregistered credential DRep is an invalid certificate, and a dangling vote delegation found in authoritative state is a typed terminal, never filtered out.
    counterexample: After a DRep unregisters, delegations to it remain and its former delegators' stake still counts toward its votes, or a dangling delegation is silently skipped when the DRep distribution is computed.
    secondary:      []
    scope:          Conway vote delegation
    assumptions:    [A2]
    depends_on:     [VAL-011, #4]
    locality:       transactional
    legacy:         [DC-EPOCH-47]
```

## New assumptions

```text
N1  rollback-depth-bound-k: the node never adopts a chain that requires rolling back more than k blocks, where k is the consensus security parameter the network's genesis defines under the reference semantics of A2; blocks more than k below the selected tip are immutable.
```

## Transitions in scope

- `epoch_boundary(LedgerState, EpochNo, Context) -> Result<(LedgerState, Effects), BoundaryRejection>`
- `evolve_nonce(ChainDepState, HeaderView) -> ChainDepState` and `tick_nonce(ChainDepState, EpochNo) -> ChainDepState`
- `activate_view(ActiveEpochAuthority, Candidate, SelectedPoint) -> Result<(ActiveEpochAuthority, Effects), ActivationError>`
- `advance_accumulator(EpochAccumulator, Block, Context) -> Advanced | AlreadyApplied | BoundaryMarkRequired | ApplyFailed`
- `rewind_accumulator(Store, RollbackTarget) -> Result<Store, RewindRefusal>`

## TCB color hypothesis

- BLUE: `ade_ledger` (epoch boundary, rewards, snapshots, governance, accumulator), `ade_core::consensus` (nonce, epoch view, leadership)
- GREEN: `ade_testkit` per-network differential suites
- RED: `ade_runtime` (activation write and publication, accumulator persistence, rewind orchestration, recovery)

## Open questions

- S1. CN-EPOCH-04 (future decisions may not leak into present validation; no retroactive reinterpretation) — merged into ORD-001 NoRetroactiveValidation, whose text carries it in full.
- S2. DC-EPOCH-13 (no build or runtime switch decides whether epoch-view activation occurs) — merged into EVO-001 SingleSemanticInterpretation (no flag, option or alternative code path changes consensus semantics).
- S3. DC-EPOCH-43 (stake snapshots keyed by the whole credential; reward deltas credited to exactly that credential if registered, otherwise the treasury) — merged into STR-005 CredentialDiscriminantPreserved for the keying, and drafted into #1 for the crediting (rewards must match the reference).
- S4. DC-EPOCH-03 (forging is contained to the recovered seed epoch; never sign past a boundary with a stale epoch nonce) — deferred to B10 pipeline-production, which covers forging.
- S5. DC-GOV-01's approach, that Ade never decides a proposal ratifies and only refunds when it can prove a proposal could not ratify, is superseded by native ratification (#4, from DC-EPOCH-47). Its refund rule is drafted as #5.
- S7. DC-EPOCH-02's era-translation clause (translation functions between eras are mandatory) — excluded: Ade executes only Conway (A2), so no era translation runs; protocol-version changes within Conway are drafted as #29.
- S8. The window-replay source for leadership past the second bootstrap epoch (DC-EPOCH-08, DC-EPOCH-09, DC-EPOCH-17's replay over the epoch C-2 window) is superseded within the legacy record by DC-EPOCH-25: past the bootstrap epochs, promotion consumes only the sealed leadership object. The parts that still hold, the boundary-point mark capture and lineage binding, are drafted into #11; DC-EPOCH-17's one-boundary-at-a-time rule is drafted into #8.
- S9. DC-EPOCH-47's bootstrap admissibility rules (import the snapshot epoch's finished ratification, cross-checked against its post-enactment parameters and its governance root; refuse a snapshot below protocol major 10 or with a dangling delegation) — deferred to B11 mithril-evidence, which covers snapshot import. Its vote-delegation lifecycle is drafted as #30.
- S6. The k-block rollback bound enters as assumption N1 because the consensus batch (B05) defines the invariant that enforces it; that batch's line will cite or supersede N1.
```
