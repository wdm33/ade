# Define — resources-release

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved, and every `#N` reference in an approved line's text is replaced by the ID line N receives. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B12, the last batch of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers the mempool, resource bounds and memory, the UTxO storage backend and its fingerprint, peer topology, testing and release discipline, operations, and the live acceptance evidence. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 57 legacy IDs appears on a line or in a settlement. It also carries the release acceptance criteria earlier batches deferred here (codec-wire-crypto S6, mithril-evidence S11), and fourteen reinterpretations that replace batch-local `#N` references in approved registry text with the IDs they name.

Many legacy entries in this batch were declared and never enforced. They come in duplicate pairs (a constraint `CN-` entry and a release or operational `RO-`/`OP-` restatement); each pair maps to one line or one settlement. The batch claims no enforcement for any of them; the re-evidence pass sets each claim status (unresolved, partial, supported or falsified) from evidence.

B10 pipeline-production and B11 mithril-evidence lines are cited by name with their batch, because their IDs are assigned only at their approval; this batch is applied after both.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml, docs/planning/pipeline-production.define.md, docs/planning/mithril-evidence.define.md, crates/ade_ledger/src/mempool/, crates/ade_ledger/src/state/ (UTxO and fingerprint), crates/ade_node/src/ba02_evidence.rs

## Drafts

```text
#1  VAL  MempoolAgreesWithLedger  [critical]
    statement:      A transaction that passes resource admission (#4) is admitted to the mempool if and only if standalone transaction validity (VAL-009: phase 1 accepts it and its scripts evaluate successfully), computed by the same phase-1 and script-evaluation functions block validity uses, accepts it against the mempool's running state (the base ledger state with every earlier admitted transaction applied); no mempool-only validity rule exists, and a resource refusal is reported as a distinct reason, never as invalidity.
    counterexample: The mempool skips the script-integrity hash check and admits a transaction that the block containing it then fails on.
    secondary:      []
    scope:          mempool admission
    assumptions:    [A2]
    depends_on:     [VAL-008, VAL-009]
    locality:       local
    legacy:         [DC-MEM-01, CN-MEM-04]

#2  STR  ClosedTxIngress  [standard]
    statement:      Every transaction source (node-to-node submission, node-to-client submission, tests) reduces to one closed ingress event before mempool admission; the source variant is metadata for evidence, policy and replay only and never changes the validity verdict.
    counterexample: Transactions from the local client socket skip the size check that peer-submitted transactions get.
    secondary:      []
    scope:          transaction ingress
    assumptions:    []
    depends_on:     [STR-004, #1]
    locality:       local
    legacy:         [DC-MEM-03]

#3  DET  MempoolReplay  [critical]
    statement:      Replaying the same ordered ingress trace against the same base ledger state produces a byte-identical sequence of mempool states and admission outcomes.
    counterexample: The mempool orders its accepted set by hash-map iteration, so two replays of one trace produce different accepted orders.
    secondary:      []
    scope:          mempool
    assumptions:    [A1]
    depends_on:     [DET-001, #2]
    locality:       local
    legacy:         [DC-MEM-04]

#4  SEC  BoundedDeterministicAdmission  [critical]
    statement:      Untrusted inbound work (transactions, peer requests) is admitted through fixed, bounded policies before it consumes authoritative resources (validation time, mempool capacity, storage), and when demand exceeds a bound the work shed is chosen by a deterministic rule over the recorded arrival order, never by timing, randomness or resource exhaustion; shedding never changes the verdict of any work admitted.
    counterexample: Under a transaction flood the mempool grows until allocation fails and the node crashes, or which transactions survive depends on thread timing.
    secondary:      [DET]
    scope:          transaction and request admission
    assumptions:    [A1]
    depends_on:     [SEC-002, SEC-018]
    locality:       local
    legacy:         [CN-MEM-01, DC-MEM-02, CN-MEM-03]

#5  LIV  AuthoritativeWorkNotStarved  [standard]
    statement:      Block validation, chain selection and persistence never wait on mempool or peer-connection work: they run on execution resources that transaction admission and connection handling cannot occupy or block, so their progress does not depend on ingress or churn load, however high it is within the configured bounds.
    counterexample: Transaction admission and block validation share one worker pool, and a burst of submissions keeps a received block queued behind them.
    secondary:      [STR]
    scope:          node scheduling
    assumptions:    []
    depends_on:     [#4]
    locality:       local
    legacy:         [CN-MEM-02, OP-MEM-01]

#6  DET  FingerprintIndependentOfStorage  [critical]
    statement:      The UTxO and ledger-state fingerprints, and every post-state, are independent of how the UTxO is stored and represented: in-memory and on-disk backends produce byte-identical replay from the same log and checkpoint; the fingerprint is computed by the canonical encoder over canonically encoded keys, never from a backend's iteration order or allocation addresses; a compact representation reads the same values and exposes the same bytes as the fully parsed one; and UTxO lookup returns owned values, so changing the backend changes no verdict, fingerprint or failure.
    counterexample: The on-disk backend iterates keys in its own page order and the fingerprint over it differs from the in-memory run.
    secondary:      []
    scope:          UTxO storage and fingerprint
    assumptions:    []
    depends_on:     [DET-001, DET-004, DET-007]
    locality:       local
    legacy:         [DC-MEM-05, DC-MEM-06, DC-MEM-08, DC-MEM-09]

#7  SAF  BoundedUtxoMemory  [critical]
    statement:      The in-memory part of the UTxO (the read cache and the last-k change log) is bounded by fixed constants no option can change, and reaching a bound never changes an authoritative output.
    counterexample: The read cache grows with every lookup during a long catch-up until the process runs out of memory.
    secondary:      []
    scope:          UTxO backend
    assumptions:    []
    depends_on:     [#6]
    locality:       local
    legacy:         [DC-MEM-07]

#8  DET  UtxoCommitmentVersionedIncremental  [critical]
    statement:      The UTxO component of the ledger fingerprint is a named commutative set commitment (Ristretto255 multiset hash over the canonical encodings of each input and output, domain-separated and version-tagged), versions never mixed in one fold; maintaining it incrementally per block, or reusing a cached component while the UTxO is unchanged (invalidated by any UTxO mutation), always equals the full recomputation byte for byte. Forward-sync and forge admissions take the post fingerprint from the cached component rather than a full recomputation, and convergence evidence reuses the post fingerprint the admission just computed.
    counterexample: The cached component survives a rollback that restored spent outputs, so the logged post fingerprint differs from a recompute.
    secondary:      []
    scope:          UTxO fingerprint
    assumptions:    []
    depends_on:     [DUR-009, #6]
    locality:       local
    legacy:         [DC-MEM-10, DC-MEM-11]

#9  PRC  OwnedMemoryBelowReference  [standard]
    statement:      Measured on the same venue and chain, Ade's owned resident memory (private dirty plus anonymous resident memory) stays below the reference node's, by a measurement method recorded with the evidence, and no change made to reduce memory alters ledger semantics, chain selection, persisted bytes or replay.
    counterexample: A memory reduction drops the full UTxO from the fingerprint, saving memory while silently changing the post fingerprints.
    secondary:      []
    scope:          resource evidence
    assumptions:    [A2]
    depends_on:     [#6, DET-001]
    locality:       distributed
    legacy:         [OP-MEM-02]

#10 SEC  ServeRangeWorkBounded  [critical]
    statement:      A peer's block-fetch range request makes the serve path read at most a fixed number of blocks, without materializing an unbounded range or scanning the whole index per block; a larger range fails closed before storage or CPU work grows, and the bound is fixed, not a runtime option.
    counterexample: A peer requests the range from the anchor to the tip and the serve task reads the whole chain into memory.
    secondary:      []
    scope:          serving
    assumptions:    []
    depends_on:     [SEC-002, VAL-046]
    locality:       local
    legacy:         [DC-SERVEMEM-01]

#11 SEC  NoPeerDiscoveryWhenForging  [critical]
    statement:      A node with forging enabled dials only the peers the operator configured; it never dials a peer learned from the network (peer sharing or ledger relay registrations).
    counterexample: A producer accepts peer-sharing replies and starts dialling the addresses they name, exposing it to arbitrary public peers.
    secondary:      []
    scope:          outbound connections
    assumptions:    []
    depends_on:     [SEC-025]
    locality:       local
    legacy:         [CN-NET-01, OP-NET-01]

#12 PRC  DifferentialFuzzingReleaseBlocking  [critical]
    statement:      Consensus-relevant inputs (blocks, headers, transactions and wire messages) are fuzzed differentially against the reference node across every supported protocol version and every decode and validation path, and accept/reject agreement with the reference on the pinned authoritative corpora (blocks, transactions and ledger-state hashes) is checked before every release; any verdict mismatch from either blocks a release until resolved.
    counterexample: A release ships with a decoder that the fuzz corpus never exercised under the newest protocol version.
    secondary:      []
    scope:          release
    assumptions:    [A2]
    depends_on:     [VAL-007, VAL-008, PRC-002]
    locality:       local
    legacy:         [CN-TEST-01, RO-TEST-01, CN-REL-03, RO-REL-02]

#13 PRC  PermanentRegressionCorpus  [standard]
    statement:      Every input that ever caused a fork, a venue mismatch or a parser disagreement with the reference becomes a named, permanent entry in the regression corpus, checked on every run of the test suite.
    counterexample: The block that once crashed the decoder is fixed but not kept, and a later refactor reintroduces the crash.
    secondary:      []
    scope:          test corpus
    assumptions:    [A2]
    depends_on:     [PRC-001]
    locality:       local
    legacy:         [CN-TEST-02, RO-TEST-02]

#14 PRC  MixedVersionEvidenceBeforeRelease  [standard]
    statement:      A release is eligible for a network only with evidence that it agrees with every supported predecessor running alongside it (EVO-003, SEC-013) on malformed and boundary-case inputs.
    counterexample: A release changes a decoder's handling of an indefinite-length map and ships without being run beside the previous release.
    secondary:      []
    scope:          release
    assumptions:    [A2]
    depends_on:     [EVO-003, SEC-013]
    locality:       distributed
    legacy:         [CN-REL-01, RO-REL-01]

#15 PRC  UnmaskedCloseGate  [standard]
    statement:      A cluster closes only on the real exit status of an unmasked full test run; piped or filtered output is display only, never the pass or fail authority; and a slice that changes canonical bytes, encoded forms, decoder inputs or golden fixtures closes only after every consumer of the changed output across all crates has been found and checked.
    counterexample: The close gate runs `cargo test | tail`, whose exit status is tail's, and a failing test closes the cluster.
    secondary:      []
    scope:          cluster closure
    assumptions:    []
    depends_on:     [PRC-001]
    locality:       local
    legacy:         [RO-CLOSE-01]

#16 LIV  ForgeWithinSlotDeadline  [standard]
    statement:      On a leader slot, forging, self-acceptance and handing the block to serving complete before the slot's deadline on the venue's slot length; missing the deadline loses the slot (pipeline-production's ForgeKesWindowFromBoundGenesis discards the late result) but violates no other invariant.
    counterexample: Self-acceptance re-folds the whole UTxO fingerprint, takes 20 seconds, and every leader slot is lost.
    secondary:      []
    scope:          forging
    assumptions:    [A2]
    depends_on:     [DET-019]
    locality:       local
    legacy:         [OP-OPS-05]

#17 PRC  PeerAcceptanceEvidenceCorrelated  [critical]
    statement:      Peer-acceptance evidence for forged blocks is a closed, versioned manifest built only by one pure correlator: the forged block's hash is the required correlation key, a peer signal's slot must agree when present, conflicting peer signals yield no evidence, the peer-log parser accepts only the allow-listed acceptance lines (a self-acceptance, receipt or verdict line never counts), the forged block's hash and slot are read from the forge artifact, never recomputed or parsed from block bytes; and the manifest names the Ade commit, reference node and cli versions, network, block hash, slot, operational-certificate and genesis fingerprints, the Ade evidence file, the raw peer log with its SHA-256 (which must match the committed log), the command that captured it, the filter used to display it (documentation only, never authority) and the acceptance keyword it matched.
    counterexample: The correlator counts Ade's own forge-succeeded line as acceptance, or a manifest names a filtered excerpt instead of the raw peer log.
    secondary:      []
    scope:          acceptance evidence
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       local
    legacy:         [RO-LIVE-06, CN-OPERATOR-EVIDENCE-01]

#18 PRC  RehearsalNonPromotable  [standard]
    statement:      A private-testnet rehearsal of block acceptance runs the same node-mode path as the public venue, with no rehearsal-only flag, branch, bootstrap authority or constructor, differing only in operator inputs and the evidence label; its evidence is marked rehearsal, stored only in the rehearsal location, built by the same correlator, and never counted or referenced as public-venue acceptance evidence.
    counterexample: A private-only helper makes the rehearsal forge succeed, and its manifest is cited as bounty acceptance.
    secondary:      []
    scope:          rehearsal evidence
    assumptions:    [A2]
    depends_on:     [#17]
    locality:       local
    legacy:         [CN-REHEARSAL-FIDELITY-01]

#19 PRC  SyncLiveLegWitnessed  [standard]
    statement:      A claim that Ade syncs from a certified snapshot to the reference tip is supported only with an operator-witnessed live run on a private Conway testnet of two reference nodes, its transcript and peer logs committed with their hashes under mithril-evidence's SyncEvidenceManifest.
    counterexample: The sync claim is marked supported from a fixture comparison alone.
    secondary:      []
    scope:          sync evidence
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       distributed
    legacy:         []

#20 SEC  LiveFeedBuffersClosedConstants  [critical]
    statement:      Peer-driven memory on the live feed is bounded before authoritative decoding by closed constants that no command-line option, environment variable or configuration can change or disable: a per-mini-protocol reassembly buffer that exceeds its bound fails closed with a typed error and drops that peer, and the pump's lookahead stops draining at its bound so the bounded channel back-pressures it; nothing is truncated or partly decoded.
    counterexample: A configuration key lets an operator set the reassembly bound to unlimited, and a peer streams a 2 GiB frame.
    secondary:      []
    scope:          live feed
    assumptions:    []
    depends_on:     [SEC-002, SEC-018]
    locality:       local
    legacy:         [DC-LIVEMEM-01]

#21 REC  ReconciliationOnlyThroughCanonicalChain  [critical]
    statement:      After a partition or any incident, authoritative state is reconciled only by the node's own paths over the canonical chain (following and selecting chains through admission and fork choice, and recovery from the durable chain and log) or by a verified re-bootstrap through the single bootstrap authority (STR-013, under SEC-015); no operator command, tool or import writes ledger, chain or consensus state outside those paths.
    counterexample: An operator tool patches the ledger snapshot to match a peer after a partition and the node resumes from it.
    secondary:      []
    scope:          incident handling
    assumptions:    [A4]
    depends_on:     [REC-004, DUR-012, STR-014, STR-013, SEC-015]
    locality:       local
    legacy:         [CN-OPS-01, OP-OPS-01]

#22 PRC  LiveFollowWindow  [standard]
    statement:      A live follow or admission-agreement pass against the reference peer counts only if its transcript covers at least 30 continuous minutes, records the peer's tip, Ade's tip and the agreement verdict for every admitted block, and meets mithril-evidence's OperatorPassTranscript conditions.
    counterexample: A two-minute run with one agreed verdict is recorded as the live follow pass.
    secondary:      []
    scope:          live evidence
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       distributed
    legacy:         [RO-LIVE-02, RO-LIVE-03, RO-LIVE-05]

#23 PRC  WireSmokePass  [standard]
    statement:      A wire-smoke pass against the reference peer counts only if the wire-only run completes the node-to-node handshake, sends one FindIntersect offering Origin, records the peer's announced tip, emits only the closed wire-only event vocabulary (never admission, verdict or ledger events) and exits cleanly.
    counterexample: The smoke run exits 0 after a failed handshake, and the pass is recorded.
    secondary:      []
    scope:          live evidence
    assumptions:    [A2]
    depends_on:     [VAL-004]
    locality:       distributed
    legacy:         [RO-LIVE-04]

#24 PRC  LiveAcceptanceReleaseGate  [critical]
    statement:      Block production is claimed, and a release presented as block-producing, only with committed evidence built under #17 from a sustained live session on a public venue in which a reference cardano-node peer adopted Ade-forged blocks delivered over chain-sync and block-fetch. Without stake able to win slots on the venue the gate is unmet and the claim stays unresolved; it is never waived, and rehearsal evidence (#18) never satisfies it.
    counterexample: The release notes claim block production on the strength of a private-testnet rehearsal while no public-venue manifest exists.
    secondary:      []
    scope:          release, block-production claim
    assumptions:    [A2]
    depends_on:     [#17, #18]
    locality:       distributed
    legacy:         [CN-CONS-06]
```

## Redefinitions

```text
R1  DET-005  StructuredDeterministicFailures  reinterpret
    invariant:
      current:  Every consensus-relevant failure, whether a rejection a transition returns or an internal inconsistency that triggers the halt of #8, is reported as a value of a closed, structured type that carries no free-form text and no host-dependent content; given equivalent prior state, canonical input, protocol version and explicit context, two executions report byte-identical failures.
      proposed: Every consensus-relevant failure, whether a rejection a transition returns or an internal inconsistency that triggers the halt of SAF-002, is reported as a value of a closed, structured type that carries no free-form text and no host-dependent content; given equivalent prior state, canonical input, protocol version and explicit context, two executions report byte-identical failures.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/foundations.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R2  VAL-006  SigningMatchesReference  reinterpret
    invariant:
      current:  Ade's Sum6 KES and Praos VRF signing produce output byte-identical to the Haskell cardano-base reference for every input: verification-key derivation, key generation from a seed, evolution across all 64 periods, KES signatures and VRF proofs; every KES signature and VRF proof Ade emits verifies under Ade's own verifier (#8) with the matching verification key; the 608-byte expanded KES signing key serializes and deserializes byte-identically to the reference for every period from 0 to 63, with the current period inferred uniquely from the key's tree shape. The algorithm is Ade's own code: no production path builds a key of an upstream library through unsafe layout assumptions, transmute, vendored private access or fork-only constructors.
      proposed: Ade's Sum6 KES and Praos VRF signing produce output byte-identical to the Haskell cardano-base reference for every input: verification-key derivation, key generation from a seed, evolution across all 64 periods, KES signatures and VRF proofs; every KES signature and VRF proof Ade emits verifies under Ade's own verifier (SEC-003) with the matching verification key; the 608-byte expanded KES signing key serializes and deserializes byte-identically to the reference for every period from 0 to 63, with the current period inferred uniquely from the key's tree shape. The algorithm is Ade's own code: no production path builds a key of an upstream library through unsafe layout assumptions, transmute, vendored private access or fork-only constructors.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/codec-wire-crypto.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R3  SEC-008  WitnessesBindTransaction  reinterpret
    invariant:
      current:  A Conway transaction is accepted only if every key authority it requires has a key or bootstrap witness that verifies over the transaction's preserved body hash, where the required key authorities are the closed set of the key-hash payment credentials and bootstrap addresses of its resolved spending and collateral inputs, its explicit required signers, and the key-hash credentials its certificates, withdrawals, votes and proposals require; every key and bootstrap witness it supplies, required or not, verifies under its own key over that hash. Script-hash credentials are authorized by script witnesses, not keys (#19).
      proposed: A Conway transaction is accepted only if every key authority it requires has a key or bootstrap witness that verifies over the transaction's preserved body hash, where the required key authorities are the closed set of the key-hash payment credentials and bootstrap addresses of its resolved spending and collateral inputs, its explicit required signers, and the key-hash credentials its certificates, withdrawals, votes and proposals require; every key and bootstrap witness it supplies, required or not, verifies under its own key over that hash. Script-hash credentials are authorized by script witnesses, not keys (SEC-010).
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/ledger.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R4  SAF-004  AtMostOneActivationPerEpoch  reinterpret
    invariant:
      current:  Within one canonical chain history, at most one epoch view activates for each target epoch, and its durable activation record holds the view's complete identity: target epoch, network, era, transition point, source checkpoint commitment, snapshot phase, nonce commitment and the view's canonical hashes. A second record for the same target epoch on the same history is an idempotent replay if it is byte-identical to the first and a terminal conflict otherwise. A rollback that abandons an activation's transition point invalidates that activation record with the rollback, before any replacement activation for the epoch, and the record is never reused (#16).
      proposed: Within one canonical chain history, at most one epoch view activates for each target epoch, and its durable activation record holds the view's complete identity: target epoch, network, era, transition point, source checkpoint commitment, snapshot phase, nonce commitment and the view's canonical hashes. A second record for the same target epoch on the same history is an idempotent replay if it is byte-identical to the first and a terminal conflict otherwise. A rollback that abandons an activation's transition point invalidates that activation record with the rollback, before any replacement activation for the epoch, and the record is never reused (DET-009).
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/epoch.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R5  LIV-001  BoundedRefold  reinterpret
    invariant:
      current:  When a settled rewind point exists and every proof about it holds (canonical at its slot, at least k blocks below the tip, at or before the rollback target, fingerprint verified), re-deriving the epoch accumulator after a rollback replays at most 2k blocks, independent of how long the node has run: once the chain has advanced 2k blocks past bootstrap, the settled point is between k and 2k blocks below the tip, and it survives the recovery pass that follows a durable rollback, being re-proved against the chain as it stands after the rollback commits rather than discarded. Before a settled point exists, or when a proof fails, the refold starts from the certified bootstrap baseline (#15, #19).
      proposed: When a settled rewind point exists and every proof about it holds (canonical at its slot, at least k blocks below the tip, at or before the rollback target, fingerprint verified), re-deriving the epoch accumulator after a rollback replays at most 2k blocks, independent of how long the node has run: once the chain has advanced 2k blocks past bootstrap, the settled point is between k and 2k blocks below the tip, and it survives the recovery pass that follows a durable rollback, being re-proved against the chain as it stands after the rollback commits rather than discarded. Before a settled point exists, or when a proof fails, the refold starts from the certified bootstrap baseline (REC-001, DUR-002).
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/epoch.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R6  LIV-003  PartitionConvergence  reinterpret
    invariant:
      current:  If, after a network partition heals, the node keeps receiving from at least one peer that offers the chain the network is converging on (N1), and that chain forks from the node's selection at most k blocks deep, then the node's selected chain eventually equals the chain the reference selection rule prefers among the valid chains delivered to it that #5 allows it to adopt, its current selection included, with ties between equally preferred chains settled as the reference rule settles them; it chooses using only protocol-defined observables.
      proposed: If, after a network partition heals, the node keeps receiving from at least one peer that offers the chain the network is converging on (N1), and that chain forks from the node's selection at most k blocks deep, then the node's selected chain eventually equals the chain the reference selection rule prefers among the valid chains delivered to it that SAF-007 allows it to adopt, its current selection included, with ties between equally preferred chains settled as the reference rule settles them; it chooses using only protocol-defined observables.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/consensus.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R7  SAF-009  CandidateViewHashVerified  reinterpret
    invariant:
      current:  Before an epoch view or sealed leadership object is activated or read, its canonical hash is recomputed over every binding (network, era, epoch, source point, checkpoint commitment, nonce, snapshot phase and profile commitment) and over its whole stake and VRF payload, and must equal its stored hash; its bindings cannot change once bound. The profile commitment it binds is blake2b-256 over a domain tag, the genesis hash, the protocol-parameter hash and the active-slot coefficient, all three taken from the durable consensus profile (#6).
      proposed: Before an epoch view or sealed leadership object is activated or read, its canonical hash is recomputed over every binding (network, era, epoch, source point, checkpoint commitment, nonce, snapshot phase and profile commitment) and over its whole stake and VRF payload, and must equal its stored hash; its bindings cannot change once bound. The profile commitment it binds is blake2b-256 over a domain tag, the genesis hash, the protocol-parameter hash and the active-slot coefficient, all three taken from the durable consensus profile (DUR-005).
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/consensus-inputs.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R8  REC-004  CrashRecoveryReplayEquivalent  reinterpret
    invariant:
      current:  All authoritative state is derivable by replay from the bootstrap anchor and the accepted canonical inputs. Two continuous runs from the same recovered or bootstrapped state, with the same ordered block feed, the same clock-tick schedule, the same leadership and key inputs and the same shutdown schedule, produce byte-identical durable outputs (selected tip, ledger fingerprint, write-ahead-log image, checkpoints and halt state), self-forged admissions included. After any process termination or power loss, recovery reaches exactly the last durably committed prefix, and its durable outputs are byte-identical to those of a run that stopped cleanly at that prefix; the ledger fingerprint replay reaches after log entry N equals the fingerprint the log's fold expects after entry N (#10). Recovery starts from the most recent valid snapshot at or below the durable tip and replays forward only the stored blocks after it.
      proposed: All authoritative state is derivable by replay from the bootstrap anchor and the accepted canonical inputs. Two continuous runs from the same recovered or bootstrapped state, with the same ordered block feed, the same clock-tick schedule, the same leadership and key inputs and the same shutdown schedule, produce byte-identical durable outputs (selected tip, ledger fingerprint, write-ahead-log image, checkpoints and halt state), self-forged admissions included. After any process termination or power loss, recovery reaches exactly the last durably committed prefix, and its durable outputs are byte-identical to those of a run that stopped cleanly at that prefix; the ledger fingerprint replay reaches after log entry N equals the fingerprint the log's fold expects after entry N (DUR-009). Recovery starts from the most recent valid snapshot at or below the durable tip and replays forward only the stored blocks after it.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/durability-recovery.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R9  VAL-044  FullMiniProtocolSurface  reinterpret
    invariant:
      current:  Ade implements the full node-to-node set (Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing) and the full node-to-client set (Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor), each in the roles the reference node plays and in agreement with #3.
      proposed: Ade implements the full node-to-node set (Handshake, ChainSync, BlockFetch, TxSubmission2, KeepAlive, PeerSharing) and the full node-to-client set (Handshake, LocalChainSync, LocalTxSubmission, LocalStateQuery, LocalTxMonitor), each in the roles the reference node plays and in agreement with VAL-042.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/network.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R10  VAL-047  FindIntersectSetsCursor  reinterpret
    invariant:
      current:  After the chain-sync server answers IntersectFound at a point, its read position is exactly that point: the next RequestNext is answered with the block immediately after it when that block is on the unchanged served chain, and otherwise with the waiting or rollback reply #10 allows; an intersection at Origin sets the position before the chain's first block.
      proposed: After the chain-sync server answers IntersectFound at a point, its read position is exactly that point: the next RequestNext is answered with the block immediately after it when that block is on the unchanged served chain, and otherwise with the waiting or rollback reply DET-017 allows; an intersection at Origin sets the position before the chain's first block.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/network.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R11  VAL-053  SingleProducerForgeBase  reinterpret
    invariant:
      current:  In a declared single-producer venue the forge base becomes the node's own selected durable tip only through an established extension: entered by catching up and then self-admitting an own block through the admission path, or, at warm start, by a successful recovery whose durable tip lies above the bootstrap anchor and proves own-forged continuation, and held only while no competing block has been observed on the peer receive stream since catch-up or self-admission, the relay does not produce blocks, the durable chain is contiguous and servable, and no fork-choice decision is required. A failed warm-start entry falls back to #11's catch-up gate; an established extension that loses a fence (a competing block is observed, or the relay produces) refuses to forge, never falling back to the catch-up gate. In the established extension, a clean structural end of the follow feed does not stop forging, while a shutdown request or a fatal feed failure does. A file-based adoption certificate is evidence only, never forge-base authority, and appears in no multi-producer, preprod or production forge path.
      proposed: In a declared single-producer venue the forge base becomes the node's own selected durable tip only through an established extension: entered by catching up and then self-admitting an own block through the admission path, or, at warm start, by a successful recovery whose durable tip lies above the bootstrap anchor and proves own-forged continuation, and held only while no competing block has been observed on the peer receive stream since catch-up or self-admission, the relay does not produce blocks, the durable chain is contiguous and servable, and no fork-choice decision is required. A failed warm-start entry falls back to SAF-013's catch-up gate; an established extension that loses a fence (a competing block is observed, or the relay produces) refuses to forge, never falling back to the catch-up gate. In the established extension, a clean structural end of the follow feed does not stop forging, while a shutdown request or a fatal feed failure does. A file-based adoption certificate is evidence only, never forge-base authority, and appears in no multi-producer, preprod or production forge path.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/node.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R12  SEC-020  RollbackTargetFromStoredPoint  reinterpret
    invariant:
      current:  For a peer's RollBackward on the participant path, the rollback target is the stored chain point for the named hash, and the peer's slot must equal the stored slot; apart from the exact persisted-anchor case of #23, which is evaluated first, an unknown hash, a slot mismatch or Origin fails closed before any rollback, log entry or state change, and no target is ever built from a peer slot combined with a local hash.
      proposed: For a peer's RollBackward on the participant path, the rollback target is the stored chain point for the named hash, and the peer's slot must equal the stored slot; apart from the exact persisted-anchor case of SAF-016, which is evaluated first, an unknown hash, a slot mismatch or Origin fails closed before any rollback, log entry or state change, and no target is ever built from a peer slot combined with a local hash.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/node.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R13  LIV-007  ReconnectResumesFromDelivered  reinterpret
    invariant:
      current:  A reconnected peer session builds its FindIntersect candidates by #33's rule from the durable chain at dial time, putting first the last block delivered downstream since the latest rollback when that block is newer than the durable tip, as a resume hint that #21 still resolves; a rollback resets the hint; and blocks buffered from the old session are delivered before the new session's, the per-peer delivery order being preserved across the reconnect.
      proposed: A reconnected peer session builds its FindIntersect candidates by VAL-055's rule from the durable chain at dial time, putting first the last block delivered downstream since the latest rollback when that block is newer than the durable tip, as a resume hint that SEC-020 still resolves; a rollback resets the hint; and blocks buffered from the old session are delivered before the new session's, the per-peer delivery order being preserved across the reconnect.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/node.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).

R14  VAL-055  IntersectionCandidatesMatchReference  reinterpret
    invariant:
      current:  The points a node offers in chain-sync FindIntersect come only from its durable chain, read back from the tip through stored blocks whose recomputed hashes, heights decreasing by exactly one at each parent link and strictly decreasing slots are checked, any break being a typed error that stops following that peer; the offered depths are the reference client's (0, the Fibonacci numbers below l, and l itself, where l is k or the number of retained blocks if fewer, newest first); the bootstrap anchor is offered only when it is the tip; and one FindIntersect carrying the whole offer is sent per connection. The one exception to durable-chain points is #31's resume hint, offered first after a reconnect; per-peer FIFO delivery makes that block durable before any following rollback is resolved. An IntersectFound naming a point that was not offered fails closed without reconnecting; IntersectNotFound stops following that peer with a typed reason naming the bound that was reached, with no RequestNext, no reconnect and no durable change; and the intersection is never rollback authority, the RollBackward that follows being resolved by #21.
      proposed: The points a node offers in chain-sync FindIntersect come only from its durable chain, read back from the tip through stored blocks whose recomputed hashes, heights decreasing by exactly one at each parent link and strictly decreasing slots are checked, any break being a typed error that stops following that peer; the offered depths are the reference client's (0, the Fibonacci numbers below l, and l itself, where l is k or the number of retained blocks if fewer, newest first); the bootstrap anchor is offered only when it is the tip; and one FindIntersect carrying the whole offer is sent per connection. The one exception to durable-chain points is LIV-007's resume hint, offered first after a reconnect; per-peer FIFO delivery makes that block durable before any following rollback is resolved. An IntersectFound naming a point that was not offered fails closed without reconnecting; IntersectNotFound stops following that peer with a typed reason naming the bound that was reached, with no RequestNext, no reconnect and no durable change; and the intersection is never rollback authority, the RollBackward that follows being resolved by SEC-020.
    why:        replaces the batch-local reference(s) with the ID(s) those lines received in docs/planning/node.define.md; the meaning is unchanged.
    cost:       claim stays unresolved (it is unresolved already).
```

## Transitions in scope

- `admit(MempoolState, TxBytes) -> (MempoolState, AdmitOutcome)`
- `ingress(Source, Bytes) -> IngressEvent`
- `fingerprint(LedgerState) -> Hash32`, `fingerprint_utxo_v2(UTxO) -> Hash32`
- `correlate(ForgedArtifacts, PeerLog) -> Result<Ba02Manifest, NoEvidence>`

## TCB color hypothesis

- BLUE: `ade_ledger::mempool` (admission), the UTxO fingerprint
- GREEN: acceptance correlator, admission and ordering policies
- RED: UTxO storage backend, serve path, dialer, evidence gates under `ci/`

## Open questions

- S1. DC-LIVEMEM-01 — drafted as #20: SEC-002 and SEC-018 do not require the bounds to be closed constants no runtime option can change.
- S2. CN-NET-01 and OP-NET-01's inbound half ("must not accept arbitrary public peer connectivity") — departure: restricting who may connect to a producer's listener is a deployment decision (firewall and relay topology), as for the reference node; #11 carries the outbound half, which is a property of the node.
- S3. CN-NET-02 and OP-NET-02 (relay paths geographically and topologically diverse) — excluded: a requirement on the operator's deployment, not a property of Ade.
- S4. CN-NET-03 and OP-NET-03 (no single peer, ASN, region or operator dominates the authoritative view) — merged into DET-012 ConsensusIndependentOfArrival and LIV-003 PartitionConvergence (selection from protocol-defined observables only) and pipeline-production's MultiPeerFairMerge; the ASN, region and operator diversity part is deployment, as S3.
- S5. CN-NET-04 (peer selection must not let an adversary set starve honest views) — merged into pipeline-production's MultiPeerFairMerge: Ade has no peer selection beyond the operator's list, and the merge is where starvation could occur.
- S6. DC-NET-01 (three-tier cold, warm and hot peer management) — excluded: declared and never designed; Ade has no peer governor, and adding one is a Define of its own.
- S7. CN-TEST-03 and RO-TEST-03 (failed, duplicate and boundary inputs verdict-stable under resubmission and replay) — merged into DET-001 ReplayByteIdentity (the same state and input give a byte-identical verdict).
- S8. CN-REL-03 and RO-REL-02 (cross-implementation accept and reject agreement on authoritative corpora is release-blocking) — drafted into #12.
- S9. CN-REL-02 and RO-REL-03 (no single implementation bug exceeds the protocol's fault threshold at ecosystem level) — excluded: an ecosystem property that no check within Ade can decide.
- S10. CN-OPS-01 and OP-OPS-01 (post-incident reconciliation only from the recovered canonical chain) — drafted as #21.
- S11. CN-OPS-02 and OP-OPS-02 (emergency recovery procedures with admissibility criteria and authority thresholds) — excluded: Ade's only recovery procedure is re-bootstrap from a verified source (EVO-005, SEC-015); an emergency procedure would itself need a Define.
- S12. CN-OPS-03 and OP-OPS-03 (incident evidence reconstructs the decision path without nondeterministic logs) — merged into DET-001 and REC-004 under A1: the recorded inputs replay every decision.
- S13. CN-CONS-06 — drafted as #24. RO-LIVE-01 (a reference peer's block-fetch of an Ade-forged block passes its full validation) — merged into pipeline-production's ForgedBlocksAcceptedByReference; its evidence form is #17 and its release gate #24.
- S14. OP-OPS-04 (both KES key flows; private keys stay in the shell; the shell evolves the key to the current period inside the certificate's window) — merged into SEC-006 ClosedKeyEnvelopes, SEC-001, SEC-005 and pipeline-production's ForgeKesWindowFromBoundGenesis.
- S15. RO-LIVE-02, RO-LIVE-03 and RO-LIVE-05 — drafted as #22, with a stated departure from RO-LIVE-02's "Ade's tip equals the peer's at every step": a healthy follower lags the live producer, so per-step equality is a harness artifact; #22 records both tips per block and relies on agreement where slots meet and no divergence.
- S16. RO-LIVE-04 — drafted as #23.
- S17. Release acceptance criteria deferred here (codec-wire-crypto S6: live acceptance of served blocks; mithril-evidence S11: the sync live leg) — #24 with #17, and #19.
- S18. #12, #13 and #14 carry legacy entries recorded as declared and never enforced; #9's OP-MEM-02 was recorded as enforced. The re-evidence pass sets each claim status (unresolved, partial, supported or falsified) from evidence; this batch claims none.
- S19. R1–R14 replace the batch-local `#N` references in fourteen approved entries with the IDs those lines received in their batches. Each keeps its meaning exactly; SAF-013's reference is replaced by pipeline-production's R1.
