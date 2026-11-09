# Define — node

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B09 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers the running node: bootstrap and lifecycle ownership, the slot clock, shutdown, peer isolation, serving, forge decisions, live fork choice, rollback handling, recovery of the follow position, and diagnostics. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 52 legacy IDs appears on a line or in a settlement. Several legacy entries were superseded within the legacy record itself; their settlements say so.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants that lines and settlements cite)

## Drafts

```text
#1  STR  SingleBootstrapAuthority  [critical]
    statement:      The node's initial ledger state, chain-dependent state and chain tip at startup come from one bootstrap authority; first-run bootstrap from a verified source and warm start from durable state are branches of that one function, never parallel paths.
    counterexample: A second startup path used by produce mode builds its initial ledger state from a snapshot without the checks the bootstrap authority applies.
    secondary:      []
    scope:          node startup
    assumptions:    [A2]
    depends_on:     [SEC-015, REC-004]
    locality:       local
    legacy:         [CN-NODE-01]

#2  STR  SingleLiveLifecycleOwner  [critical]
    statement:      The node mode is the single owner of the live run: the run loop changes authoritative state only through the existing closed authorities (the bootstrap authority for initial state, the one admission path for tip advance), no second binary mode drives the live node, and the loop's planner emits only a closed lifecycle vocabulary that cannot express an authority decision.
    counterexample: A diagnostic subcommand opens the live data directory and advances the tip while the node runs, or the loop planner decides to accept a block itself.
    secondary:      []
    scope:          node run loop
    assumptions:    []
    depends_on:     [#1, DUR-012]
    locality:       local
    legacy:         [CN-NODE-02]

#3  DET  SingleSlotAuthority  [critical]
    statement:      The current slot is derived by one authority: the shell observes wall-clock time, and one conversion projects it over the network's complete timing calendar from system start, the calendar selected by the durable record's genesis hash and never by operator flags (operator-supplied network or genesis values are only fail-closed cross-checks). The authority is established only if the calendar is self-consistent (each segment boundary aligned to the previous segment's epoch length), reproduces the store's recorded epoch start and epoch length, places the seed point's slot inside the recorded seed epoch, and yields an anchor whose lineage verifies against the calendar; it is reconstructed from the bootstrap anchor slot, never persisted, and a forging start without the durable seed record fails closed; a schedule that does not cover system start is refused for both conversions, time to slot and slot to time. Only the slot crosses into the orchestrator core, which reads no clock, so replaying the core with a recorded clock and event stream gives byte-identical final state.
    counterexample: The forge converts time to a slot with a single anchor that ignores an earlier era's 20-second slots, so on preprod it computes a slot thousands of slots away from the real one.
    secondary:      [STR]
    scope:          slot clock, orchestrator core
    assumptions:    [A1, A2]
    depends_on:     [STR-002, DUR-005]
    locality:       local
    legacy:         [DC-NODE-03, DC-NODE-45, DC-NODE-05]

#4  SEC  PerPeerSessionIsolation  [critical]
    statement:      A failure in one peer's session (a decode error, a validity rejection, a too-deep rollback, a protocol violation) ends only that peer's session; the node keeps serving other peers and producing blocks, and peer sessions share no working state.
    counterexample: One peer sending a malformed block-fetch reply crashes the node's whole network layer.
    secondary:      [LIV]
    scope:          peer sessions
    assumptions:    []
    depends_on:     [SEC-017]
    locality:       local
    legacy:         [DC-NODE-01]

#5  LIV  BoundedCleanShutdown  [critical]
    statement:      On a shutdown request, while storage keeps completing writes, the node drains its admission, write and snapshot pipeline to a quiescent point within a fixed, stated deadline, without waiting for peer sessions, finishes writing every snapshot the cadence policy has scheduled up to that point, and exits successfully; restarting on the same stores yields a byte-identical initial state, the blocks after the last snapshot being replayed from the log. If the deadline passes or storage fails, the node exits with a distinct non-zero code without writing a partial snapshot, its durable state being that of the last commit.
    counterexample: The node ignores SIGTERM and must be killed, or a clean shutdown waits forever for a peer session to close.
    secondary:      [REC]
    scope:          node shutdown
    assumptions:    [A1, A6]
    depends_on:     [REC-004]
    locality:       local
    legacy:         [DC-NODE-04]

#6  VAL  ServedViewIsDurableProjection  [critical]
    statement:      The chain served to peers (chain-sync headers and block-fetch bodies) is a deterministic projection of the durable adopted chain, received and self-forged blocks alike, so a peer can fetch coherent history; it is served only through the existing chain-sync and block-fetch serve reducers, by a serve task that only reads it and holds no write handle to the chain, log or forge.
    counterexample: The served view holds a forged block B whose parent A was never added to it, so a follower can intersect at B's parent but cannot fetch it.
    secondary:      [STR]
    scope:          serving
    assumptions:    []
    depends_on:     [VAL-045, SAF-010]
    locality:       local
    legacy:         [DC-NODE-13, DC-NODE-07, DC-NODE-06]

#7  LIV  ServeOutlivesFeed  [standard]
    statement:      Once the node has started serving, the end of its upstream feed does not by itself stop the serve task; serving stops only on node shutdown, a fatal serve error or cancellation by the lifecycle owner.
    counterexample: The upstream relay disconnects, the run loop returns, and the node stops serving the blocks it already holds.
    secondary:      []
    scope:          serve task lifetime
    assumptions:    []
    depends_on:     [#6]
    locality:       local
    legacy:         [DC-NODE-09]

#8  SAF  ForgeOncePerSlotMonotone  [critical]
    statement:      A forge is attempted at most once per slot and never for a slot at or before the last forged slot.
    counterexample: After a restart the node forges a second, different block for a slot it already forged in, producing an equivocation.
    secondary:      [ORD]
    scope:          forge scheduling
    assumptions:    []
    depends_on:     [#3]
    locality:       local
    legacy:         [DC-NODE-05]

#9  STR  OwnForgedAdmitThroughChokepoint  [critical]
    statement:      A self-accepted forged block becomes part of the durable chain only by entering the same admission path as received blocks (store bytes, append log, advance tip, behind header and block validation, extending the tip only), with the bytes admitted byte-identical to the bytes self-acceptance validated; the forge has no tip-advance path of its own.
    counterexample: The forge writes its block to the chain database and advances the tip directly, skipping validation, or re-encodes the block between self-acceptance and admission.
    secondary:      [DUR]
    scope:          forge, admission
    assumptions:    [A6]
    depends_on:     [DUR-012, VAL-009]
    locality:       transactional
    legacy:         [DC-NODE-12]

#10 VAL  ForgeSuccessorPosition  [critical]
    statement:      A forged block's previous-block hash is the hash of the forge base and its block number is the forge base's plus one, and the ledger state and chain-dependent state the forge builds on and self-accepts against are the ones the latest admission produced; none of these comes from a stale startup baseline, a guess or a default.
    counterexample: After admitting a feed block, the next forge numbers its block from the warm-start baseline and produces block 101 on top of block 105.
    secondary:      []
    scope:          forge
    assumptions:    []
    depends_on:     [VAL-032]
    locality:       local
    legacy:         [DC-NODE-10]

#11 SAF  ForgeAdmissibilityGatesOnly  [critical]
    statement:      Every forge is admissible only when the durable servable tip equals the followed peer tip in hash and block number, except under #13's fenced extension in a single-producer venue; the recovered anchor is never a forge base. The followed-peer-tip signal holds two pieces of evidence separately: the peer's advertised tip, and the tip it served that Ade durably admitted, written only after a successful admission from that admission's own validated tip and cleared by any rollback. The combined view prefers the served tip only when strictly higher, a tie with differing hashes falling to the advertisement; possession questions (the forge gate and the fork-switch fence) read the combined view, and convergence evidence reads the advertisement. The signal can only prevent a forge (a refusal for not being caught up is a typed refusal with no state change), never selects, replaces or reorders chains, and never changes durable state.
    counterexample: A stale advertised peer tip is fed into chain selection and makes the node prefer a shorter chain, or a refused forge still advances the slot cursor.
    secondary:      []
    scope:          forge admissibility
    assumptions:    []
    depends_on:     [VAL-031]
    locality:       local
    legacy:         [DC-NODE-15, DC-NODE-17, DC-NODE-47]

#12 VAL  ForgeParentIntersectable  [critical]
    statement:      A forge builds only on a durably stored block that a reference peer can find with FindIntersect on Ade's served chain; the recovered snapshot anchor is never a forge parent and is never served as a chain head a peer cannot intersect.
    counterexample: The node forges on its bare recovered anchor, which it does not serve, so peers cannot intersect and reject the forged block as disconnected.
    secondary:      []
    scope:          forge base, serving
    assumptions:    [A2]
    depends_on:     [#6, VAL-047]
    locality:       distributed
    legacy:         [DC-NODE-14]

#13 VAL  SingleProducerForgeBase  [critical]
    statement:      In a declared single-producer venue the forge base becomes the node's own selected durable tip only through an established extension: entered by catching up and then self-admitting an own block through the admission path, or, at warm start, by a successful recovery whose durable tip lies above the bootstrap anchor and proves own-forged continuation, and held only while no competing block has been observed on the peer receive stream since catch-up or self-admission, the relay does not produce blocks, the durable chain is contiguous and servable, and no fork-choice decision is required. A failed warm-start entry falls back to #11's catch-up gate; an established extension that loses a fence (a competing block is observed, or the relay produces) refuses to forge, never falling back to the catch-up gate. In the established extension, a clean structural end of the follow feed does not stop forging, while a shutdown request or a fatal feed failure does. A file-based adoption certificate is evidence only, never forge-base authority, and appears in no multi-producer, preprod or production forge path.
    counterexample: A single-producer node keeps waiting for a relay to echo its own block before forging the next one, or an operator certificate file decides which tip the node forges on.
    secondary:      []
    scope:          single-producer forging
    assumptions:    []
    depends_on:     [#9, #11]
    locality:       local
    legacy:         [DC-NODE-20, DC-NODE-22, DC-NODE-19, DC-NODE-21]

#14 SAF  NoForgeDuringPendingSelection  [critical]
    statement:      While a peer candidate awaits a fork-choice decision, forging is refused with a typed reason until the decision is durably applied and reconciled or rejected with durable state unchanged; a forge never builds on the tip from before the decision.
    counterexample: The node forges on its old tip while a competing branch it is about to adopt is being fetched, producing an orphan.
    secondary:      [ORD]
    scope:          forge, live fork choice
    assumptions:    []
    depends_on:     [#19, #20]
    locality:       local
    legacy:         [DC-NODE-28]

#15 OBS  EveryForgeTickAccounted  [standard]
    statement:      Every forge tick the node admits ends in either a structured refusal carrying its own reason or a leader-schedule decision; none disappears. Refusal reasons are distinct closed discriminators: slot derivation failure; the three KES-window failures (before the operational certificate's start, after its end, period arithmetic overflow), each naming the slot bound and decided by the one checked KES-period function; tip and fence refusals, with the absent-tip reason used only when the tip is actually absent; and the leadership outcome.
    counterexample: A slot before the certificate's start silently skips the forge attempt, so the operator sees no forging and no reason.
    secondary:      []
    scope:          forge scheduling, diagnostics
    assumptions:    [A2]
    depends_on:     [SEC-005]
    locality:       local
    legacy:         [DC-NODE-46]

#16 OBS  ClosedDiagnosticVocabulary  [standard]
    statement:      The node's feed and forge scheduling diagnostics use a closed, allow-listed event vocabulary with closed reason and outcome sets and no catch-all variant, so a new reason cannot be emitted without being added to the vocabulary; the diagnostics are emit-only, never read by the planner and never changing forge scheduling, base or authority; and an ambiguous or reason-less feed disconnect never makes a feed state forge-eligible.
    counterexample: A forge skip is logged with a free-text "other" reason that monitoring cannot classify.
    secondary:      []
    scope:          diagnostics
    assumptions:    []
    depends_on:     []
    locality:       local
    legacy:         [CN-NODE-04]

#17 VAL  ReceiveDispositionSingleClassifier  [critical]
    statement:      Every peer announcement is classified once, by one pure total function of the durable tip, the candidate header and explicit evidence of whether the candidate's hash is already stored on the durable chain, as already held, a linear extension, refused in single-producer mode, or needing fork choice; a re-announcement of a block Ade already holds is "already held", never a competing candidate.
    counterexample: A relay's echo of Ade's own just-adopted block is treated as a competing fork and triggers a rollback.
    secondary:      []
    scope:          receive path
    assumptions:    []
    depends_on:     [DET-012]
    locality:       local
    legacy:         [DC-NODE-23]

#18 SAF  VenueSplitForkChoice  [critical]
    statement:      A candidate that needs fork choice is handled according to the declared venue: a single-producer venue refuses it, a participant venue passes the validated candidate to the chain selector, and an undeclared or unknown venue takes the refusing path; a raw followed-peer tip never reaches chain selection.
    counterexample: A node with no declared venue adopts a peer's fork, or a participant node feeds the peer's advertised tip straight into chain selection.
    secondary:      []
    scope:          live fork choice
    assumptions:    []
    depends_on:     [#17, VAL-031]
    locality:       local
    legacy:         [DC-NODE-24]

#19 STR  ForkChoiceAppliedThroughAuthorities  [critical]
    statement:      A chain-selection outcome changes durable state only through the existing authorities (lockstep admission and rollback, the single rollback materializer, the one admission path), and a fork-choice win becomes durable only when the new chain's bodies validate and apply; there is no header-only tip advance and no second apply or rollback path.
    counterexample: After a fork-choice win the node moves its tip to the winning header before fetching and validating its body.
    secondary:      []
    scope:          live fork choice
    assumptions:    [A6]
    depends_on:     [REC-003, STR-011, DUR-012]
    locality:       transactional
    legacy:         [DC-NODE-25]

#20 REC  SelectorReconciledWithDurable  [critical]
    statement:      After every applied receive decision, the chain selector's current tip equals the durable chain tip and its chain-dependent state equals the durable chain-dependent state; the in-memory decision state never runs ahead of, behind or forked from the durable chain.
    counterexample: A rollback is applied to the chain database but the selector still believes the old tip is current, and its next decision compares against a block that is no longer stored.
    secondary:      []
    scope:          chain selector, durable state
    assumptions:    []
    depends_on:     [#19]
    locality:       transactional
    legacy:         [DC-NODE-26]

#21 SEC  RollbackTargetFromStoredPoint  [critical]
    statement:      For a peer's RollBackward on the participant path, the rollback target is the stored chain point for the named hash, and the peer's slot must equal the stored slot; apart from the exact persisted-anchor case of #23, which is evaluated first, an unknown hash, a slot mismatch or Origin fails closed before any rollback, log entry or state change, and no target is ever built from a peer slot combined with a local hash.
    counterexample: A peer sends RollBackward naming a real block hash with a forged slot, and the node rolls back to a point that is not on its chain.
    secondary:      [VAL]
    scope:          live rollback
    assumptions:    []
    depends_on:     [SAF-007]
    locality:       local
    legacy:         [DC-NODE-29]

#22 REC  FollowStartFromPersistedAnchor  [critical]
    statement:      A store recovered from a non-Origin bootstrap anchor persists that anchor point, bound to its fingerprint, and live follow starts from the servable chain tip if one exists, otherwise from that persisted anchor point; a missing, malformed or mismatched anchor record fails closed before following starts, and Origin is used only for a store that truly starts at Origin.
    counterexample: A warm-started node with no blocks past its anchor starts chain-sync from Origin, asking the peer for the whole chain.
    secondary:      []
    scope:          warm start, live follow
    assumptions:    [A2]
    depends_on:     [DUR-008]
    locality:       local
    legacy:         [DC-NODE-31]

#23 SAF  AnchorRollbackNoOp  [critical]
    statement:      On both the single-producer and the participant follow paths, a peer RollBackward to exactly the persisted recovered anchor point (slot and hash), checked before stored-block resolution, is an idempotent no-op with no log entry and no state change; the anchor is never turned into a servable stored block, and a RollBackward to Origin or to any other non-stored point fails closed.
    counterexample: A peer's rollback to the recovered anchor makes the node synthesize a placeholder block at the anchor and serve it.
    secondary:      []
    scope:          live rollback
    assumptions:    []
    depends_on:     [#21, #22]
    locality:       local
    legacy:         [DC-NODE-32, DC-NODE-33]

#24 OBS  PeerIdentityPreserved  [standard]
    statement:      The receive path carries each block's origin peer identity through to the consumer as provenance only, so per-peer candidates can be tracked; carrying it changes no selection, admission, rollback or verdict, and a single-peer run replays byte-identically with or without it.
    counterexample: The peer label is dropped when blocks enter the node's feed, so candidates from two peers are merged into one.
    secondary:      []
    scope:          receive path
    assumptions:    [A1]
    depends_on:     []
    locality:       local
    legacy:         [DC-NODE-34]

#25 SEC  CandidatesFromOwnValidatedHeaders  [critical]
    statement:      Every candidate given to chain selection is built from headers Ade itself validated, never from a peer-supplied summary or a raw followed-peer tip; candidates keep their peer identity, the candidate set is ordered deterministically, and the one chain-selection authority receives it, with no second selector.
    counterexample: A peer-minted header summary that skipped Ade's VRF check is used as a fork-choice candidate.
    secondary:      [VAL]
    scope:          live fork choice
    assumptions:    []
    depends_on:     [VAL-031, DET-012]
    locality:       local
    legacy:         [DC-NODE-35, DC-NODE-36]

#26 SEC  ForkSwitchNeverAbandons  [critical]
    statement:      When chain selection picks a branch that forks below the durable tip, no rollback of the current chain is committed until the replacement branch's bodies are fetched, linked and validated as a complete branch from a fork anchor bound to Ade's stored chain point; a failed, incomplete or lying branch leaves the chain database, ledger and chain-dependent state unchanged.
    counterexample: The node rolls back three blocks to switch to a peer's branch, the peer then fails to deliver the bodies, and the node is left on a shorter chain than it started with.
    secondary:      [SAF]
    scope:          live fork switch
    assumptions:    [A4, A6]
    depends_on:     [#21, #19]
    locality:       transactional
    legacy:         [DC-NODE-37]

#27 SEC  ForkAnchorWithinKValidated  [critical]
    statement:      A competing branch is eligible for selection only when its parent links lead back to a durably stored fork anchor, with both the number of headers traversed and the distance from the durable tip's block number to the anchor's at most k blocks, and every header from that anchor to the branch tip has been validated; each cached header entry is bound to its own recomputed hash, slot and parent, and the anchor counts only when the chain database confirms its slot and hash.
    counterexample: A branch whose cached headers include one whose stored hash differs from its recomputed hash is selected, or a branch anchored 3,000 blocks back is considered.
    secondary:      [VAL]
    scope:          live fork choice
    assumptions:    [A4]
    depends_on:     [SAF-007, #25]
    locality:       local
    legacy:         [DC-NODE-38]

#28 LIV  PostSwitchContinuity  [critical]
    statement:      After adopting a winning branch at tip X, the node admits the winning peer's descendants in parent-link order (X+1, then X+2), never admitting a descendant whose parent it has not validated. When a bridge block is missing, it re-fetches the missing range from the winning peer only, with a deterministic bounded number of retries, and admits it in order only when the fetched range is complete, parent-linked and body-validated; otherwise it holds with a structured missing-bridge reason and its durable chain unchanged. While a missing bridge is held, forging stays refused; only a real linear admission or a proven fork-switch adoption clears the hold. It never stalls silently.
    counterexample: The peer serves X+2 without X+1, and the node either admits X+2 on the peer's word or stops following without saying why.
    secondary:      [ORD]
    scope:          post-fork-switch follow
    assumptions:    [A5]
    depends_on:     [#26]
    locality:       distributed
    legacy:         [DC-NODE-39, DC-NODE-41]

#29 SEC  RetainedBranchEvidenceNonAuthoritative  [critical]
    statement:      Blocks rolled back during a fork switch may be kept only as evidence for reconstructing future competing branches: bounded to k blocks, keyed by hash in a deterministic map, each entry bound to its recomputed hash; they never become durable authority, a rollback target or a fork anchor, and never bypass header or body validation.
    counterexample: A retained rolled-back block is used as the fork anchor for a later switch, though the chain database no longer holds it.
    secondary:      [SAF]
    scope:          fork-switch evidence cache
    assumptions:    [A4]
    depends_on:     [#27]
    locality:       local
    legacy:         [DC-NODE-40]

#30 SAF  ForgePathRollbackWithinPromotedEpoch  [critical]
    statement:      On the forge and live-follow path, a peer RollBackward whose target lies in an epoch earlier than the promoted leadership authority's epoch fails closed before any rollback, log entry or state change.
    counterexample: A peer's rollback to the previous epoch rewinds the chain while the promoted leadership authority stays at the newer epoch, and the node forges with the wrong epoch's stake.
    secondary:      [ORD]
    scope:          forge-path rollback
    assumptions:    []
    depends_on:     [STR-007, #21]
    locality:       local
    legacy:         [DC-NODE-42]

#31 LIV  ReconnectResumesFromDelivered  [critical]
    statement:      A reconnected peer session builds its FindIntersect candidates by #33's rule from the durable chain at dial time, putting first the last block delivered downstream since the latest rollback when that block is newer than the durable tip, as a resume hint that #21 still resolves; a rollback resets the hint; and blocks buffered from the old session are delivered before the new session's, the per-peer delivery order being preserved across the reconnect.
    counterexample: After a reconnect the node asks the peer to roll back to its startup anchor, now more than k blocks behind, and the rollback fails closed.
    secondary:      [ORD]
    scope:          peer reconnection
    assumptions:    [A4]
    depends_on:     [SAF-007, #33, #21]
    locality:       distributed
    legacy:         [DC-NODE-43]

#32 OBS  ReplayDivergenceSelfDescribing  [standard]
    statement:      A warm-start replay divergence is reported with a structured account naming the log-tail slot, admission count, the ledger's epoch beside the schedule's epoch at that slot, the replay anchor and span, the per-component fingerprints of the anchor and the result, the store's semantics generation and the artifact; an epoch disagreement is named as the fault, and an unreadable anchor is reported distinctly from "nothing changed".
    counterexample: A replay mismatch is reported only as two differing fingerprints, leaving the operator unable to tell which part of the ledger diverged.
    secondary:      []
    scope:          recovery diagnostics
    assumptions:    []
    depends_on:     [REC-004]
    locality:       local
    legacy:         [DC-NODE-44]

#33 VAL  IntersectionCandidatesMatchReference  [critical]
    statement:      The points a node offers in chain-sync FindIntersect come only from its durable chain, read back from the tip through stored blocks whose recomputed hashes, heights decreasing by exactly one at each parent link and strictly decreasing slots are checked, any break being a typed error that stops following that peer; the offered depths are the reference client's (0, the Fibonacci numbers below l, and l itself, where l is k or the number of retained blocks if fewer, newest first); the bootstrap anchor is offered only when it is the tip; and one FindIntersect carrying the whole offer is sent per connection. The one exception to durable-chain points is #31's resume hint, offered first after a reconnect; per-peer FIFO delivery makes that block durable before any following rollback is resolved. An IntersectFound naming a point that was not offered fails closed without reconnecting; IntersectNotFound stops following that peer with a typed reason naming the bound that was reached, with no RequestNext, no reconnect and no durable change; and the intersection is never rollback authority, the RollBackward that follows being resolved by #21.
    counterexample: A restarted node offers only its tip, gets no intersection after a short reorg, and stops following.
    secondary:      []
    scope:          chain-sync client
    assumptions:    [A2, A4]
    depends_on:     [REC-005]
    locality:       local
    legacy:         [DC-NODE-48]

#34 OBS  ConvergenceEvidenceNonAuthoritative  [standard]
    statement:      Convergence evidence emitted on the participant path is a deterministic side output of outcomes already decided: a receipt event for every peer block the receive path considers, an admission event for every admission, and agreement verdicts, every event bound to the consensus-input fingerprint; it never gates admission, triggers or shapes a rollback, influences fork choice or changes the durable chain. A failure to write evidence does not stop the node, but marks the evidence transcript incomplete, so a partial transcript never passes as complete.
    counterexample: An agreement verdict of "diverged" triggers an automatic rollback.
    secondary:      []
    scope:          participant evidence output
    assumptions:    []
    depends_on:     []
    locality:       local
    legacy:         [DC-NODE-30]

#35 SAF  ReceiveIdempotentByHash  [critical]
    statement:      On linear admission against the current tip, a peer-delivered block already durably stored with the same slot and hash is an idempotent no-op (no validation step, no log entry, no tip change), and a different block at or before the last applied slot is not short-circuited but reaches header validation against the current tip, which rejects it; the decision compares hashes, never slots alone. A block belonging to a competing branch is handled by fork choice and validated against its own fork state, not by this rule.
    counterexample: A relay re-announcing Ade's own adopted block makes the node halt with "slot before last applied", or a different block at the same slot is skipped because only the slot was compared.
    secondary:      [DET]
    scope:          admission
    assumptions:    []
    depends_on:     [DUR-012]
    locality:       local
    legacy:         [DC-NODE-16]

#36 SEC  ForgeActivationAllOrNothing  [critical]
    statement:      Forging is enabled only when the complete operator set is supplied (cold signing key, KES signing key, VRF signing key, operational certificate and genesis file): all present enables forging, all absent leaves a non-forging node, and any partial set is a structured error, never a silent non-forging fallback or a missing, zero or fabricated key; the pool id is derived in one named place, never fabricated; and private key material is never copied into coordinator, planner or loop state, or into anything persisted, logged, hashed for evidence or replayed, tests and debug output included.
    counterexample: A node started with a VRF key but no KES key silently runs as a relay, or a test snapshot records the KES signing key's bytes.
    secondary:      [VAL]
    scope:          operator key ingress, forge activation
    assumptions:    []
    depends_on:     [SEC-006, SEC-001]
    locality:       local
    legacy:         [CN-NODE-03]

#37 SAF  AuthorityFatalExit  [critical]
    statement:      An authority-fatal error (a failed chain write on a committed rollback, or an unknown snapshot version or fingerprint mismatch at bootstrap, among others) halts the process deterministically with a non-zero exit code, with no retry of the failed write and no fallback decode.
    counterexample: A chain-database write fails after a rollback is committed, and the node retries it in a loop and carries on with a half-applied rollback.
    secondary:      []
    scope:          node process
    assumptions:    []
    depends_on:     [SAF-002]
    locality:       local
    legacy:         [DC-NODE-04]

#38 STR  SingleSnapshotCadencePolicy  [standard]
    statement:      The persistent snapshot writer captures a snapshot only when the one cadence policy schedules it; no orchestrator-side override and no second cadence policy exists.
    counterexample: The orchestrator adds an extra capture on every epoch boundary on top of the cadence policy's schedule, so the snapshot set depends on which component ran.
    secondary:      [DET]
    scope:          snapshot writer
    assumptions:    []
    depends_on:     [DET-014]
    locality:       local
    legacy:         [DC-NODE-02]
```

## Transitions in scope

- `bootstrap(Source, Stores) -> Result<InitialState, BootstrapError>`
- `run_loop_step(LoopState, Event, Clock) -> (LoopState, LifecycleAction)` (closed vocabulary)
- `classify_receive(DurableTip, HeaderSummary, DurableMembership) -> ReceiveDisposition`
- `forge_decision(ForgeState, Slot, FollowedPeerTip, PendingSelection) -> ForgeAttempt | ForgeRefused`
- `resolve_rollback(ChainDb, PeerPoint) -> Result<StoredPoint, RollbackRefusal>`

## TCB color hypothesis

- BLUE: `ade_core::consensus` (chain selection), `ade_ledger` (validation)
- GREEN: loop planner, receive classifier, forge decision, slot conversion
- RED: `ade_node` (lifecycle, run loop, serve task, peer sessions), `ade_runtime` (admission, rollback application, chain database)

## Open questions

- S1. DC-NODE-02 (the snapshot writer captures only on the cadence schedule; no override) — drafted as #38; DET-014 carries the determinism of the schedule.
- S2. DC-NODE-04 — drafted as #37 (authority-fatal exit) and #5 (clean shutdown and resume identity).
- S3. CN-NODE-03 — drafted as #36; its loader clauses are SEC-006 ClosedKeyEnvelopes and its custody clauses SEC-001. Its production-specific clauses (the forge base is the one recovered bootstrap state; forge containment unchanged; default protocol parameters as an honest-scope limit) are deferred to B10 pipeline-production.
- S4. DC-NODE-27 (live rollback and reselection replay to byte-identical durable state) — merged into REC-004 CrashRecoveryReplayEquivalent and DUR-009 WalFingerprintChain (rollback entries in the log fold).
- S5. DC-NODE-06 and DC-NODE-07's clause that the served chain is fed only by self-accepted forged blocks — superseded within the legacy record by DC-NODE-13 (the served view is a projection of the durable chain, received blocks included); their surviving clauses are drafted into #6.
- S6. DC-NODE-11 (block-0 monotone serve gate) — excluded: DC-NODE-13 states that it supersedes the monotone serve gate.
- S7. DC-NODE-18 (forge the next successor after an operator adoption certificate) — excluded: DC-NODE-20 retires the certificate as forge authority; #13 carries the current rule.
- S8. DC-NODE-08 (forge the genesis-successor block when no tip exists) — excluded: it applies only to a chain starting at genesis, which contradicts A2 as approved (see the durability batch's S10).
- S9. #3 carries DC-NODE-05's clock clause (the slot crosses the seam, no clock in the core); #8 carries its once-per-slot clause.
- S13. #5 departs from DC-NODE-04's "writes a final snapshot": shutdown finishes only the snapshots the cadence policy scheduled, so snapshot points stay a function of chain history (DET-014, #38), and restart identity comes from replaying the log after the last snapshot (REC-004).
- S11. #11 keeps DC-NODE-15's gate for every forge outside #13's single-producer extension. Participant-venue forge-mode rules (DC-FOLLOW-FORGE-01, CN-FOLLOW-01) are deferred to B10; any further exception they need must be made there by redefining #11.
- S12. DC-NODE-14 allows a recovered anchor made intersectable as a forge parent, which contradicts DC-NODE-15 (the recovered anchor is never a forge base). #12 resolves the conflict toward DC-NODE-15 and drops DC-NODE-14's anchor clause.
- S10. #5 restates DC-NODE-04's bounded clean shutdown. A recorded defect (the node ignores SIGINT and SIGTERM and needs SIGKILL) means #5 is expected to be recorded as falsified at the re-evidence pass.
```
