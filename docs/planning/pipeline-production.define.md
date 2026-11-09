# Define — pipeline-production

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved, and every `#N` reference in an approved line's text is replaced by the ID line N receives. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B10 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers block production and the receive pipeline that feeds it: the pure forge, leader eligibility, KES and operational-certificate use at forge time, the forged block's bytes and transactions, self-acceptance, the forge base and its epoch, the participant forge mode, the produce-mode coordinator, the per-peer wire pump (keep-alive, backpressure, fairness, reconnect), and admission mode's evidence. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 51 legacy IDs appears on a line or in a settlement. It also carries meanings earlier batches deferred here: CN-NODE-03's production clauses (node batch S3), the participant forge rules with any exception they need to SAF-013 (node batch S11), the import fingerprint's binding into admission events (consensus-inputs batch S10), and live acceptance of served blocks by a real peer (codec-wire-crypto batch S6).

Ade has three binary modes that run these paths: node mode (the live node), produce mode (a standalone producer with its own listener and coordinator) and admission mode (a follower that imports a seed and emits agreement evidence). The wire pump and the forge engine are shared; lines scoped to one mode say so.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants that lines and settlements cite), crates/ade_ledger/src/producer/forge.rs, crates/ade_ledger/src/producer/self_accept.rs, crates/ade_runtime/src/producer/coordinator.rs, crates/ade_node/src/node_sync.rs, crates/ade_runtime/src/admission/wire_pump.rs

## Drafts

```text
#1  DET  ForgePureOverProducerTick  [critical]
    statement:      Building a block is a pure function of one canonical producer tick (the slot, the forge base's ledger and chain-dependent state, the transaction snapshot, the protocol parameters and version, and the already-produced VRF proof, KES signature and operational certificate): it reads no clock, randomness, file, unordered collection or ambient state. Replaying the same tick stream over the same starting state yields byte-identical forged blocks; replay uses the recorded signed artifacts, never calls a signing function, and no replay corpus holds private key material.
    counterexample: The block builder stamps a field from the system clock, or iterates a hash map to order the transactions, so two replays of the same tick produce different block bytes.
    secondary:      [STR]
    scope:          forge engine (ade_ledger::producer)
    assumptions:    [A1]
    depends_on:     [DET-001, SEC-001]
    locality:       local
    legacy:         [DC-CONS-13, DC-CONS-14]

#2  VAL  ForgeOnlyWhenLeader  [critical]
    statement:      A block is forged for a slot only if the leader check passes for that slot, evaluated by the same leader-check functions header validation uses (VRF input by VAL-036, the leader value from the VRF output, the threshold from the slot's epoch's sealed stake distribution by VAL-024), with no producer-side variant of any of them; for the same canonical inputs the leader verdict is byte-identical across runs, and for the Conway era exactly one Praos VRF construction exists, with no fallback that builds or accepts the TPraos input.
    counterexample: The producer computes its own leader threshold with a rounding the validator does not use and forges a block for a slot where the validator finds it ineligible; or verification accepts a proof over either the Praos or the TPraos input.
    secondary:      [DET]
    scope:          leader check on the forge path
    assumptions:    [A2]
    depends_on:     [VAL-036, VAL-024, SEC-003]
    locality:       local
    legacy:         [DC-CONS-15, DC-FORGE-01, CN-FORGE-04]

#3  SEC  LeaderVerdictCarriesNoKeys  [critical]
    statement:      Leader eligibility crosses the shell/core boundary in one direction: shell code produces the VRF proof with the operator's VRF signing key, and the core verifies that proof and decides eligibility from canonical inputs only (slot, epoch nonce, VRF verification key, proof, and a leader-schedule answer giving the pool's stake share and threshold, which the caller derives through the leader-schedule authority of VAL-024 and passes in); the core evaluator depends on no ledger view, era schedule, chain-dependent state, clock, storage or shell crate and never receives a signing key, and its closed two-variant verdict carries forge-capable material only when eligible, an ineligible verdict carrying no more than a bounded fingerprint of the VRF output.
    counterexample: The core leader evaluator takes the VRF signing key so it can compute the proof itself, or an ineligible verdict carries the full VRF proof and output, which a caller then uses to forge.
    secondary:      [STR]
    scope:          leader check boundary
    assumptions:    []
    depends_on:     [SEC-001, VAL-024, #2]
    locality:       local
    legacy:         [CN-FORGE-02]

#4  SEC  ForgeKesWindowFromBoundGenesis  [critical]
    statement:      The KES evolution a forge signs at is computed by one checked function of the slot, the KES anchor slot and slots per KES period of the bound genesis (never the clock, a file read at forge time or a default) and the operational certificate's start period: the slot's absolute KES period minus the start period. A slot before the certificate's first covered slot, after its last, or whose arithmetic overflows is a typed refusal before any signing, naming the slot bound; and a forge result that arrives after a newer slot tick, or after its slot has passed, is discarded as a missed slot, never signed, served or admitted late.
    counterexample: The forge computes the evolution as the slot's absolute KES period, so on a chain whose certificate starts at period 885 it signs at evolution 888 and every block fails verification; or a result computed for slot s is served after slot s+3 has begun.
    secondary:      [SAF]
    scope:          forge-time KES period, forge result timing
    assumptions:    [A2]
    depends_on:     [SEC-005, DET-019, OBS-001]
    locality:       local
    legacy:         [DC-CONS-11, CN-PROD-02]

#5  VAL  ForgedBlockSingleEncoding  [critical]
    statement:      A forged block is emitted as the era-tagged [era, block] envelope by the one block encoder that is the inverse of the block decoder header validation uses, and its header's body hash is computed by the validator's body-hash recipe over the very body bytes it emits; the forge rejects, with a typed error and before emitting any bytes, a tick whose block number and previous hash form an illegal header position (VAL-003); and decoding a forged block yields exactly the header fields and four body segments that were forged, so no forge output exists that the block decoder rejects.
    counterexample: The forge emits a bare five-element block without the era tag, so its own decoder rejects every forged block at offset 0; or the forge hashes a re-encoded body while the emitted body uses different bytes.
    secondary:      [DET]
    scope:          forged block bytes
    assumptions:    [A2]
    depends_on:     [VAL-032, VAL-003, DET-002, DET-007]
    locality:       local
    legacy:         [DC-CONS-16, CN-FORGE-03]

#6  VAL  ForgedTransactionsAdmissibleInOrder  [critical]
    statement:      Every transaction in a forged block is admissible by the mempool admission function against the forge base's ledger state, applied in the transaction snapshot's accumulating order; the forge re-admits the snapshot in that order before building the body and fails with a typed error at the first transaction that is rejected or differs, and never reorders, adds, drops or skips a transaction.
    counterexample: The forge includes a transaction whose input a preceding transaction in the same block already spent, because it checked each transaction against the base state alone.
    secondary:      [ORD]
    scope:          forged block body
    assumptions:    []
    depends_on:     [VAL-008, SAF-003]
    locality:       local
    legacy:         [DC-LEDGER-12]

#7  SAF  ForgedBlockSelfAccepted  [critical]
    statement:      A forge attempt ends in exactly one of three closed outcomes: forged and self-accepted, not leader (with a bounded VRF fingerprint), or failed with a structured reason. It is forged and self-accepted only if the forged bytes decode through the block decoder and full block validity (VAL-009) accepts those exact bytes against the forge base; a self-acceptance rejection is a structured failure that hands nothing on. No forged byte is served, announced, broadcast or admitted except as the bytes self-acceptance accepted, and the node keeps following after a failure, the failure being recorded as that tick's outcome.
    counterexample: The forge handler reports success for a block whose own validation failed and hands it to the serve task, which offers it to peers.
    secondary:      [VAL]
    scope:          every forge path (node and produce modes)
    assumptions:    []
    depends_on:     [VAL-009, #5, OBS-001]
    locality:       local
    legacy:         [CN-CONS-07, CN-FORGE-01]

#8  VAL  ForgeBaseFromBootstrapAuthority  [critical]
    statement:      In every mode, the forge base (ledger state, chain-dependent state, stake distribution view, epoch nonce and tip) is initialized from the state the one bootstrap authority produced and is updated by every later admission, admissions before the first forge included, so each forge builds on the latest admission's state (VAL-051); the protocol version and parameters a forge uses come from that base's ledger state. No forge path builds a synthetic base: no zero-stake or empty ledger, no default protocol parameters or version, no constant previous hash and no second recovered state.
    counterexample: Produce mode forges against a freshly constructed empty ledger with a zero-stake distribution, so its blocks are never leader-eligible on the real chain; or node mode stamps the header with a default protocol version.
    secondary:      [STR]
    scope:          forge base, every mode
    assumptions:    [A2]
    depends_on:     [STR-013, VAL-051]
    locality:       local
    legacy:         [CN-PROD-03]

#9  SAF  ForgeEpochBoundToActiveAuthority  [critical]
    statement:      A forge for a slot in epoch E uses only the active epoch authority's view for exactly E (its stake distribution and epoch nonce); the authority is consulted before any leader check or signing. If the authority holds an earlier epoch than E and the node is a producer whose support ends at its seed epoch, the slot is a typed no-forge and the node keeps following; if the authority is ahead of E, or E's promotion is missing where continuity is required, the forge halts with a typed terminal error. A block is never signed for epoch E with another epoch's nonce or stake distribution.
    counterexample: After an epoch boundary the forge keeps using the seed epoch's nonce, signs a block for the new epoch, and peers reject it.
    secondary:      [VAL]
    scope:          forge path, epoch boundaries
    assumptions:    []
    depends_on:     [STR-007, VAL-024, SAF-005]
    locality:       local
    legacy:         [DC-EPOCH-03]

#10 SAF  ParticipantForgeOnSelectedHead  [critical]
    statement:      In a declared participant venue, forging first passes SAF-013's catch-up gate; from the first instant the durable servable tip equals the followed peer tip, it enters a participant extension whose forge base at each decision is the live selected durable tip. A decision in the extension refuses with a typed reason when the venue is not declared participant, when any fork-choice decision is pending (a reselection, a fork switch or a missing bridge; SAF-014), or when no durable servable tip exists, and it never gates on the tip of Ade's own last forge. Immediately before signing, the durable tip is read again and the forge is refused if it differs from the decision's base. The decision is a pure total function over closed types that never invokes chain selection and holds no signing material; the extension is not persisted, a restart entering it again only through the catch-up gate.
    counterexample: The participant decision re-checks that the durable tip still equals Ade's own last forged block, so once a peer's block is admitted it refuses every later slot; or a peer block admitted between decision and signing leaves the forge signing on the old tip.
    secondary:      [LIV]
    scope:          node mode, participant venue forge decision
    assumptions:    [A1]
    depends_on:     [SAF-013, SAF-014, VAL-031, STR-015]
    locality:       local
    legacy:         [CN-FOLLOW-01, DC-FOLLOW-FORGE-01]

#11 DET  ProducerCoordinatorReplay  [critical]
    statement:      The producer coordinator is a deterministic reducer over canonical event streams: for the same initial coordinator state, ledger state, operational-certificate public metadata and complete ordered stream of coordinator events (slot ticks, forge results succeeded or failed, peer lifecycle events and every other coordinator input), it emits byte-identical effects and producer log events. Its forge request is a closed effect carrying the slot, KES period, ledger snapshot reference and chain tip; its state holds no signing key; and wall-clock timestamps and socket arrival order are shell metadata outside the compared streams.
    counterexample: The coordinator records socket addresses in its replayable log, or stores the KES key in its state to sign directly, so two replays of the same streams differ or key bytes reach the replay surface.
    secondary:      []
    scope:          producer coordinator (node and produce modes)
    assumptions:    [A1]
    depends_on:     [SEC-001, DET-001]
    locality:       local
    legacy:         [DC-PROD-02]

#12 STR  ProduceChainEvolutionLinear  [critical]
    statement:      In produce mode each forge's base is the post-state of the previous self-accepted forge, carried by a linear value that is consumed when it advances, so forging against a stale base cannot be expressed; it advances only when block validity's post-state and self-acceptance agree on the same inputs, a disagreement being a typed error that does not advance; it never constructs an accepted-block token itself; and for the same bootstrap seed, slot sequence and keys the series of block numbers, previous hashes, post-state fingerprints and forged bytes is byte-identical across runs.
    counterexample: Produce mode keeps the bootstrap ledger as the base for every forge, so its second block re-spends its first block's outputs and repeats its block number.
    secondary:      [DET]
    scope:          produce mode chain evolution
    assumptions:    [A1, A2]
    depends_on:     [#7, #8]
    locality:       local
    legacy:         [DC-PROD-03]

#13 SAF  ProduceServesOnlySelfAccepted  [critical]
    statement:      In produce mode every broadcast reconstructs the accepted block from the forged bytes through self-acceptance against the pre-forge base and publishes it to the served chain through the one atomic publication (SAF-010) before the next slot tick; if self-acceptance rejects, nothing is published and a structured broadcast error is recorded; and a block-served event is emitted only for a block present in the served chain.
    counterexample: A broadcast is dropped silently when publication fails, while the log still records the block as served.
    secondary:      [OBS]
    scope:          produce mode serving
    assumptions:    []
    depends_on:     [#7, SAF-010]
    locality:       local
    legacy:         [CN-PROD-04]

#14 VAL  ForgedBlocksAcceptedByReference  [critical]
    statement:      A block Ade forges, self-accepts and serves is accepted by the reference node: a reference cardano-node peer that follows Ade fetches it over chain-sync and block-fetch, validates it and adopts it when it is that peer's best chain, and never rejects it as invalid.
    counterexample: Ade serves a forged block whose header the reference node rejects for a bad VRF input or a wrong KES evolution, and the peer disconnects.
    secondary:      [PRC]
    scope:          forged and served blocks, live venue
    assumptions:    [A2]
    depends_on:     [VAL-007, VAL-002, #7]
    locality:       distributed
    legacy:         []

#15 STR  SingleWirePumpPerPeer  [critical]
    statement:      Exactly one wire pump drives each connected peer's chain-sync and block-fetch: it owns that peer's transport handle and is the only producer on that peer's event lane. It emits only the closed events block, tip update, roll backward and disconnected, never a verdict or validity claim; and every chain-sync reply carrying a tip emits exactly one closed event: a tip update for an intersect reply or a roll forward, and the distinct roll-backward event, carrying the peer's post-rollback tip, for a roll backward.
    counterexample: A second code path reads blocks from the same peer connection and pushes them into the consumer, or the pump reports a rollback only as a lower tip update.
    secondary:      []
    scope:          per-peer wire pump (ade_runtime::admission::wire_pump)
    assumptions:    []
    depends_on:     [STR-012, SEC-017]
    locality:       local
    legacy:         [CN-PUMP-01, DC-PUMP-01, DC-PUMP-02]

#16 VAL  KeepAliveClientWireOnly  [standard]
    statement:      The wire pump runs the node-to-node keep-alive client through the shared keep-alive state machine and codec: it sends a keep-alive on a fixed cadence shorter than the peer's keep-alive timeout, never while one is outstanding, with a cookie from a deterministic wrapping 16-bit counter, and accepts a response only if its cookie matches; a grammar or cookie violation ends that peer's session with a typed error. Keep-alive traffic produces no pump event, no event in any evidence vocabulary and no canonical input (shell diagnostics, excluded from evidence and replay, may record it), is never dispatched as chain-sync or block-fetch traffic, and does not delay or reorder them.
    counterexample: The pump never sends keep-alive, so the reference peer drops every follow session after about 97 seconds; or a mismatched cookie is ignored.
    secondary:      [LIV]
    scope:          wire pump keep-alive client
    assumptions:    []
    depends_on:     [DET-016, SEC-016]
    locality:       local
    legacy:         [DC-PUMP-03]

#17 LIV  KeepAliveSurvivesBackpressure  [critical]
    statement:      No stall of the downstream consumer, of any length or cause (block application, an epoch-boundary fold, a checkpoint capture), stops the wire pump from sending keep-alive on its cadence and consuming the peer's response: the wait for downstream capacity also waits on the cadence timer and on inbound frames, so the cadence is reachable while the pump is parked.
    counterexample: The pump blocks on a full events channel during a ten-minute epoch-boundary fold, misses keep-alive, and the peer drops the session.
    secondary:      []
    scope:          wire pump under backpressure
    assumptions:    [A5]
    depends_on:     [#16]
    locality:       local
    legacy:         [DC-PUMP-05]

#18 ORD  PumpOrderPreservedUnderBackpressure  [critical]
    statement:      Backpressure may delay a peer's events but never reorder them: the events the pump delivers are the same sequence it would deliver unstalled, frames decoded while parked join the same single FIFO as the rest of their chunk, and one frame is dispatched per loop iteration. Frames held while parked are bounded by a fixed, non-configurable constant, exceeding which fails that peer closed with a typed error; the bound holds because the pump issues no new chain-sync or block-fetch request while parked.
    counterexample: A frame read while parked is dispatched ahead of the remaining frames of the chunk it interrupted, so a roll forward reaches the consumer before the roll backward that preceded it on the wire.
    secondary:      []
    scope:          wire pump under backpressure
    assumptions:    []
    depends_on:     [#15, SEC-018]
    locality:       local
    legacy:         [DC-PUMP-06, DC-PUMP-07]

#19 SEC  MultiPeerFairMerge  [critical]
    statement:      When several peers feed the receive path, no connected peer is starved by another: each peer's pump fills its own bounded lane, the lanes are merged round-robin (one item per peer per round) in the configured peer order, never by hash iteration, randomness, clock or scheduler timing, backpressure is per peer, and a disconnected peer's lane is retired without reordering the others. The merge decides only delivery opportunity: peer identity is preserved and chain selection stays independent of arrival order.
    counterexample: All pumps feed one shared bounded channel, a continuously producing peer fills it, and a competing honest peer's branch never reaches fork choice.
    secondary:      [LIV]
    scope:          multi-peer receive merge
    assumptions:    []
    depends_on:     [OBS-003, DET-012, SEC-018]
    locality:       local
    legacy:         [DC-PUMP-04]

#20 LIV  ReconnectTransportOnly  [critical]
    statement:      One total policy classifies every pump outcome: a transport loss (end of stream, read or write failure) on an established session reconnects it, re-dialling on a fixed, monotone, capped backoff schedule with no randomness or clock-derived jitter; a protocol or grammar violation, a deferred-frame overflow or a keep-alive violation ends that peer without retry; and a consumer that has gone away ends the pump. A peer that cannot be parsed, or whose first dial fails, is dropped without retry. Reconnecting re-establishes only the wire session and never turns a fail-closed authority outcome (admission, the rollback depth bound, boundary promotion, the forge fence) into a retry.
    counterexample: A peer that sends a malformed block is reconnected every second forever, or an unreachable peer at startup is re-dialled in a tight loop.
    secondary:      []
    scope:          wire pump reconnect
    assumptions:    []
    depends_on:     [SEC-019, LIV-007]
    locality:       local
    legacy:         [DC-PUMP-08, DC-PUMP-09, DC-PUMP-10]

#21 STR  AdmissionModeSingleEntry  [standard]
    statement:      Admission mode is entered through one runner, which composes the seed import, the bootstrap anchor, the bootstrap authority's warm start, the dialer, the per-block admission loop, the log append, the verdict reducer and the evidence log; the imported seed reaches the ledger only through one bridge that persists it as a snapshot at the seed point for the bootstrap authority's warm-start branch, never around that authority.
    counterexample: An admission test path builds its initial ledger state directly from the imported seed and skips the warm-start branch's checks.
    secondary:      []
    scope:          admission mode
    assumptions:    []
    depends_on:     [STR-013]
    locality:       local
    legacy:         [CN-ADMIT-01, CN-ADMIT-02]

#22 OBS  AgreementVerdictEvidenceOnly  [critical]
    statement:      The agreement verdict is a closed four-variant value (agreed, lagging, diverged, input not found) computed by a pure reducer from the admission outcome and the peer's announced tip; it compares outcomes already decided and never decides validity, chain selection or canonical state. Lagging means only that no same-slot comparison was possible: the peer's announced tip is at a different slot from the admitted block, ahead or behind, with both slots in the payload so the direction is readable, or the peer announced Origin; it asserts no ancestry and no agreement, and no caller treats it as success, health, readiness or agreement.
    counterexample: The readiness check counts a lagging verdict as live-ready, so an operator pass is reported green while Ade is behind.
    secondary:      []
    scope:          agreement verdict reducer and its callers
    assumptions:    []
    depends_on:     [OBS-005]
    locality:       local
    legacy:         [DC-ADMIT-01, DC-ADMIT-06, DC-ADMIT-08]

#23 OBS  AdmissionEvidenceBoundAndReplayable  [critical]
    statement:      Every successful admission emits exactly one agreement-verdict event, never two for one block hash; the admission start, bootstrap-complete, block-admitted and verdict events each carry the consensus-input fingerprint (VAL-034), added as a field so the event vocabulary stays closed; and replaying an admission from the prior checkpoint and log yields the same post-admission ledger fingerprint and the same verdict.
    counterexample: A retried log write emits a second verdict for the same block, or the transcript's verdicts carry no fingerprint, so they cannot be tied to the consensus inputs they were judged under.
    secondary:      [DET]
    scope:          admission and convergence evidence
    assumptions:    [A1]
    depends_on:     [#22, VAL-034, REC-004]
    locality:       local
    legacy:         [DC-ADMIT-02, DC-ADMIT-07, DC-ADMIT-10]

#24 SAF  AdmissionDivergenceHalts  [critical]
    statement:      In admission mode a block whose slot lies outside the consensus inputs' epoch window, and undecodable peer bytes, are stopped before block validity is called; they, and a diverged or input-not-found verdict derived from admission evaluation, each halt the run with a distinct non-zero exit code and a typed halt event; undecodable bytes are classified as diverged (when the peer has a tip at that slot) or as a typed undecodable-bytes halt, never as input not found and never as a clean exit. The reducer stays pure; the halt is the binary's response.
    counterexample: The runner logs an undecodable block and exits 0, so an adversarial peer makes a failed run look clean.
    secondary:      []
    scope:          admission mode
    assumptions:    []
    depends_on:     [#22, VAL-035, SAF-019]
    locality:       local
    legacy:         [DC-ADMIT-03, DC-ADMIT-11, DC-ADMIT-12]

#25 OBS  ClosedIsolatedEventVocabularies  [standard]
    statement:      Each mode's evidence log has its own closed event vocabulary (wire-only, admission, producer, and the three-event convergence subset), written through its own sink, and no mode emits another mode's events; events carry no free-form reason strings and no host-dependent content such as file paths, and peers appear in replayable events only as opaque counters, socket addresses being shell metadata outside replay comparison. Keeping key material out of every log is SEC-025.
    counterexample: The producer log prints a key file's path in a failure reason, or the convergence transcript contains forge scheduling events.
    secondary:      []
    scope:          evidence logs, every mode
    assumptions:    []
    depends_on:     [OBS-002, OBS-005, SEC-025]
    locality:       local
    legacy:         [DC-ADMIT-04, DC-PROD-01]

#26 VAL  SeedImportFailsClosed  [standard]
    statement:      The UTxO seed importer accepts only the transaction-output features it fully supports; an unsupported feature or a malformed reference script is a typed import error that stops the import, and no admission path matches that error with a permissive arm, skips the output or falls back to a partial seed.
    counterexample: The importer drops outputs carrying an unsupported feature and continues, so the seeded UTxO silently differs from the source.
    secondary:      []
    scope:          UTxO seed import
    assumptions:    []
    depends_on:     [VAL-001]
    locality:       local
    legacy:         [DC-ADMIT-09]

#27 DUR  AdmissionAppendsExactlyOnce  [critical]
    statement:      Every successful admission, received or self-forged, appends exactly one block-admission entry to the write-ahead log, chained from the folded fingerprint (DUR-009), and no admission entry exists without its admission or for a block already admitted on the selected prefix; an append that fails halts the node with the tip unadvanced (SAF-019).
    counterexample: A retried admission appends a second entry for the same block, both carrying a prior fingerprint the fold accepts, so recovery replays the block twice.
    secondary:      [REC]
    scope:          admission path, write-ahead log
    assumptions:    [A6]
    depends_on:     [DUR-009, DUR-012, SAF-019]
    locality:       local
    legacy:         [DC-ADMIT-05]
```

## Redefinitions

```text
R1  SAF-013  ForgeAdmissibilityGatesOnly  weaken
    invariant:
      current:  Every forge is admissible only when the durable servable tip equals the followed peer tip in hash and block number, except under #13's fenced extension in a single-producer venue; the recovered anchor is never a forge base. The followed-peer-tip signal holds two pieces of evidence separately: the peer's advertised tip, and the tip it served that Ade durably admitted, written only after a successful admission from that admission's own validated tip and cleared by any rollback. The combined view prefers the served tip only when strictly higher, a tie with differing hashes falling to the advertisement; possession questions (the forge gate and the fork-switch fence) read the combined view, and convergence evidence reads the advertisement. The signal can only prevent a forge (a refusal for not being caught up is a typed refusal with no state change), never selects, replaces or reorders chains, and never changes durable state.
      proposed: Every forge is admissible only when the durable servable tip equals the followed peer tip in hash and block number, except under VAL-053's fenced extension in a single-producer venue and under #10's participant extension in a participant venue; the recovered anchor is never a forge base. The followed-peer-tip signal holds two pieces of evidence separately: the peer's advertised tip, and the tip it served that Ade durably admitted, written only after a successful admission from that admission's own validated tip and cleared by any rollback. The combined view prefers the served tip only when strictly higher, a tie with differing hashes falling to the advertisement; possession questions (the forge gate and the fork-switch fence) read the combined view, and convergence evidence reads the advertisement. The signal can only prevent a forge (a refusal for not being caught up is a typed refusal with no state change), never selects, replaces or reorders chains, and never changes durable state.
    why:        The node batch's S11 required any participant exception to be made here. A participant producer must forge on the selected durable head while the live frontier races ahead; requiring exact equality with the followed peer tip on every forge holds only about 5% of ticks live, and the 2026-06-19 run forged nothing in about 5 hours. #10 bounds the exception with its own fences (pending fork choice, absent tip, sign-time re-check). The change also replaces the batch-local "#13" with the ID it names, VAL-053.
    cost:       claim becomes unresolved until the migration's re-evidence pass (M2) records evidence for the new meaning; it is unresolved already.
```

## Transitions in scope

- `forge_block(ProducerTick) -> Result<(ForgedBlock, Vec<ForgeEffect>), ForgeError>`
- `self_accept(Bytes, LedgerState, ChainDepState, EraSchedule, LedgerView) -> Result<AcceptedBlock, SelfAcceptError>`
- `verify_and_evaluate_leader(Slot, Nonce, VrfVk, VrfProof, LeaderScheduleAnswer) -> LeaderCheckVerdict`
- `kes_period_for_slot_checked(CoordinatorState, Slot) -> Result<KesEvolution, KesSlotError>`
- `participant_forge_decision(ForgeMode, DurableTip, FollowedTip, Venue, Pending) -> ParticipantForgeDecision`
- `coordinator_step(CoordinatorState, CoordinatorEvent) -> (CoordinatorState, Vec<CoordinatorEffect>)`
- `verdict::derive(AdmitOutcome, PeerTip) -> AgreementVerdict`
- `should_reconnect_after(PumpOutcome, Established) -> ReconnectDecision`

## TCB color hypothesis

- BLUE: `ade_ledger::producer` (forge, self-acceptance), `ade_core::consensus` (leader check, operational-certificate and KES verification), `ade_codec` (block envelope)
- GREEN: producer coordinator, participant forge decision, agreement verdict reducer, reconnect policy, fair merge order
- RED: `ade_runtime::producer` shell (signing), `ade_runtime::admission::wire_pump`, `ade_node` (modes, evidence logs, seed import)

## Open questions

- S1. CN-CONS-08 (every received block in the chain database passed block validity; no bypass, header-only path or trusted prefix; an invalid block leaves state unchanged and ends that peer's pipeline) — merged into DUR-012 TipAdvancesOnlyAfterDurableAdmit (the one admission path, with header validation and block validity), VAL-009 CompleteValidation (no valid verdict that skips a check) and SEC-019 PerPeerSessionIsolation (a validity rejection ends only that peer's session).
- S2. DC-ADMIT-05 — drafted as #27. DUR-009 checks fingerprint continuity and DUR-012 durability before tip advance; neither fixes one entry per admission, so #27 states it.
- S3. DC-CONS-23 (an own-forged block is admitted only if it still extends the durable tip at admission; a block that lost the race to a feed block fails ordinary validation; no own-block override or admission-time fork choice) — merged into STR-015 OwnForgedAdmitThroughChokepoint ("extending the tip only", "the forge has no tip-advance path of its own").
- S4. DC-CONS-24 (the forged parent hash is the followed tip's hash and the block number is one more) — merged into VAL-051 ForgeSuccessorPosition (parent and number from the forge base) and SAF-013 (outside the extensions the base equals the followed tip in hash and block number), with a stated departure: inside VAL-053's single-producer extension and #10's participant extension the forge base is the selected durable tip, which may differ from the followed peer tip, so followed-tip parent equality holds only outside the extensions.
- S5. CN-PROD-01 (the producer listener completes the handshake before any mini-protocol traffic; failure closes the connection) — merged into SAF-011 HandshakeBeforeTraffic, which binds every session including the producer listener's, and EVO-006 ClosedVersionNegotiation (deterministic refusal).
- S6. DC-CONS-12 (operational-certificate counter strictly greater than the previous counter) — merged into SEC-011 OpCertCounterRule, with a stated departure: a strictly increasing counter on every forge would refuse the second block forged under one certificate, which the reference accepts. The reference window (at least the recorded number, at most one more) is what binds, and #7 applies it to every forged block through full block validity. The forge engine's own counter check receives no previous counter in node mode, and its period check compares the certificate's period with itself (VAL-010); the re-evidence pass records both.
- S7. DC-CONS-11 — drafted as #4, with a stated departure: the legacy text says the certificate's KES period field equals the slot's period, but that field is the certificate's start period; #4 states the reference rule (the signing evolution is the slot's absolute period minus the start period).
- S8. DC-EPOCH-03 (single-epoch forge containment) — drafted as #9 in its later form: the legacy record itself replaced the single-epoch wall with forging on a promoted view for the slot's epoch (DC-EVIEW-11, DC-EPOCH-14, approved as SAF-005 and STR-007).
- S9. CN-CONS-07's "self-acceptance failure halts the producer" contradicts CN-FORGE-01 (a self-acceptance rejection is a structured forge failure). #7 resolves toward CN-FORGE-01, as the reference node does with a block it forged that fails validation (it traces the failure and keeps running); the forged bytes go nowhere either way. CN-CONS-07's serve-provenance clause is VAL-050.
- S10. CN-NODE-03's production clauses (node batch S3): the forge base is the one recovered bootstrap state — #8; forge containment unchanged — superseded by the forge path the node batch defined (SAF-013, VAL-053, STR-015); default protocol parameters as an honest-scope limit — retired in the code, which now reads them from the recovered ledger, and #8 forbids defaults.
- S11. Live acceptance of served blocks by a real peer (codec-wire-crypto batch S6) — drafted as #14. Its release acceptance criteria stay with B12.
- S12. The import fingerprint's binding into admission events (consensus-inputs batch S10) — drafted in #23.
- S13. CN-FOLLOW-01's deterministic-selection clause is VAL-031 and DET-012; its forge-on-selected-head clause and DC-FOLLOW-FORGE-01 are #10, which R1 admits as SAF-013's second exception.
```
- S14. #22 departs from DC-ADMIT-08's reading of lagging as "the local chain is a prefix of the peer's": the reducer has no ancestry evidence and returns lagging whenever the slots differ in either direction or the peer is at Origin, so #22 defines it as an inconclusive comparison carrying both slots.
- S15. #5's typed rejection of an illegal header position is expected to be recorded as falsified at the re-evidence pass: the forge engine currently emits such blocks, which the decoder then rejects (#7 turns that into a forge failure, so none leaves the forge).
- S16. #25's key-material clause (DC-PROD-01 "no key material") is SEC-025, which already forbids private key material in anything logged, persisted, hashed for evidence or replayed.
