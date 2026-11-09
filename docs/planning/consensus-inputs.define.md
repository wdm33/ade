# Define — consensus-inputs

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B06 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers how consensus inputs enter Ade (the imported bundle, the persisted seed-epoch record, its recovery), the VRF input, durable network geometry, and the reduced stake machinery behind Ade's own epoch views (transient storage, stake-reference classification, the reduced UTxO checkpoint, the pool lifecycle). Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 29 legacy IDs (26 assigned, plus three importer entries deferred here by the consensus batch) appears on a line or in a settlement. Several legacy entries describe designs the legacy record itself later superseded; their settlements say so.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants that settlements merge into)

## Drafts

```text
#1  VAL  ClosedConsensusInputImport  [critical]
    statement:      An externally supplied consensus-input bundle enters Ade through one importer, which produces a closed canonical record holding every required field (network magic, genesis hash, era, epoch and its slot window, active-slot coefficient, epoch nonce, the pool distribution with each pool's VRF key hash, the protocol-parameter hash and the bundle's source provenance) and a fingerprint that is blake2b-256 over the record's canonical CBOR encoding, fields in their declared order; a missing or malformed field is a typed import error, never given a default, and the raw bundle never reaches the core.
    counterexample: A bundle with no epoch nonce imports with a zero nonce, or two imports of the same bundle produce different fingerprints.
    secondary:      [DET]
    scope:          consensus-input import (shell), canonical record
    assumptions:    [A2]
    depends_on:     [STR-002, VAL-001]
    locality:       local
    legacy:         [CN-CONS-IN-01, DC-CONS-IN-01, DC-CONS-IN-02]

#2  VAL  ImportedViewEpochGuard  [critical]
    statement:      A ledger view built from imported consensus inputs answers only for its own epoch: a query for any other epoch returns no answer, and a block whose slot lies outside the record's epoch window halts admission instead of being judged with that view.
    counterexample: A header in the next epoch is validated against the imported view of the previous epoch because the view answered every query with its one stake distribution.
    secondary:      [SAF]
    scope:          imported ledger view, admission
    assumptions:    []
    depends_on:     [#1, VAL-033]
    locality:       local
    legacy:         [DC-VIEW-01]

#3  SEC  ConsensusInputsFromVerifiedBootstrapOnly  [critical]
    statement:      The persisted seed-epoch consensus-input record is written only by the verified-bootstrap paths (genesis, Mithril and the admission pre-seed), through one persistence authority; the forge and feed paths never build or write it, never construct a value of its type, and never take a consensus view from an operator-supplied bundle. Leader checks in every epoch read only the sealed leadership object for exactly that epoch (VAL-024); the bootstrap epoch's object is sealed from the certified seed record's pool distribution, the next epoch's from the certified snapshot's imported mark, and every later epoch's from canonical chain state at a boundary the node crossed (VAL-023).
    counterexample: The forge path reads an operator's --consensus-inputs-path bundle, converts it into the seed-record type, and forges with it, so a stake distribution no verified bootstrap produced decides leadership.
    secondary:      [STR]
    scope:          seed-epoch record writers; forge and feed consensus views
    assumptions:    []
    depends_on:     [#1, VAL-023, VAL-024]
    locality:       local
    legacy:         [CN-CINPUT-02, CN-CINPUT-03]

#4  DUR  SeedInputsRecoveryVerified  [critical]
    statement:      Importing the seed-epoch consensus inputs is recorded as a write-ahead-log fact appended after the record is stored, and the append is the commit point; warm start restores the record only if exactly one such fact exists for the store's anchor and the stored bytes hash to the recorded hash, decode, bind to that anchor and epoch, and re-encode byte-identically; anything else halts startup with a typed error, with no fallback to a bundle.
    counterexample: After a crash between storing the record and appending its fact, warm start restores the record anyway; or a record whose hash differs from the recorded one is used because it decodes.
    secondary:      [REC]
    scope:          seed-epoch record persistence and warm start
    assumptions:    [A1]
    depends_on:     [#3, REC-002]
    locality:       transactional
    legacy:         [DC-CINPUT-01]

#5  VAL  VrfInputFromEpochNonce  [critical]
    statement:      The Praos VRF input for a slot is blake2b-256 of the slot as 8 big-endian bytes followed by the epoch nonce of that slot's epoch, as the reference mkInputVRF defines it, for leader checks and for forged headers alike, using the IETF draft-03 VRF with 80-byte proofs.
    counterexample: The producer builds its VRF input with a nonce derived from genesis instead of the slot's epoch nonce, so every header it forges fails VRF verification at its peers.
    secondary:      []
    scope:          leader checks, header VRF
    assumptions:    [A2]
    depends_on:     [VAL-022, SEC-004]
    locality:       local
    legacy:         [DC-CINPUT-03]

#6  DUR  NetworkGeometryAndProfileDurable  [critical]
    statement:      A recovered store replays with the epoch geometry (first slot of the seed epoch and epoch length) and the consensus profile (genesis hash and protocol-parameter hash, exactly as imported) that were persisted when the store was created, never re-derived from configuration, genesis files or command-line values a restart supplies; restart-supplied values are only a consistency check that fails closed on disagreement. A record in an older format without them is refused with a distinct unsupported-format error that names re-import as the remedy, kept separate from corruption errors, and is never upgraded silently.
    counterexample: A preview store restarted with a preprod genesis file replays with preprod's 432,000-slot epochs; or the protocol-parameter hash is recomputed from re-serialized parameters and differs from the imported one.
    secondary:      [REC]
    scope:          seed-epoch record, warm start, era schedule rebuild
    assumptions:    [A2]
    depends_on:     [#4, STR-009]
    locality:       local
    legacy:         [DC-CINPUT-05, DC-CINPUT-06]

#7  SAF  TransientViewStorageNonAuthoritative  [critical]
    statement:      Transient epoch-view storage is never authority: it never survives as a source for following, forging, recovery or activation, nothing in it reaches a core output except through the deterministic derivation it feeds, and a crash at any point leaves the durable chain, write-ahead log and checkpoint unchanged and the next replay identical. Startup purges it before normal operation: it deletes only entries inside the transient store's own directory whose names have the deterministic window-key form, makes the deletion durable by syncing the parent directory, continues only when the directory is empty, and halts on an entry it does not recognize or cannot delete.
    counterexample: After a crash mid-materialization, the next startup finds the half-built transient store and uses it to answer a stake query.
    secondary:      [REC]
    scope:          transient epoch-view storage
    assumptions:    [A1]
    depends_on:     [REC-002]
    locality:       local
    legacy:         [DC-EVIEW-01]

#8  VAL  StakeReferenceClassification  [critical]
    statement:      Each output's stake reference is classified by one function from its address bytes and the era bound to the block being processed, never from configuration, the clock or a caller's flag, into a closed result: a base stake credential, valid only after the address's full 57-byte form is validated; a valid address that contributes no stake (a pointer address from Conway on, decoded exactly as the reference ledger decodes it for that era; an enterprise or Byron address); or a rejection (a malformed address, or a reward address used as an output address). Rejection is never folded into contributing no stake.
    counterexample: A Conway output to a pointer address is credited to the credential the pointer once referred to, or a 56-byte base address is read at fixed offsets and credits a garbage credential.
    secondary:      []
    scope:          stake attribution of outputs
    assumptions:    [A2]
    depends_on:     [VAL-001, STR-005]
    locality:       local
    legacy:         [DC-EVIEW-02, DC-EVIEW-03]

#9  DET  ReducedCheckpointMirrorsLedger  [critical]
    statement:      The reduced UTxO checkpoint is a projection of the ledger's own UTxO application: each block's delta removes exactly the inputs and adds exactly the outputs the ledger's application would, under the same keys, each reduced to its coin and stake reference; its certificate, delegation and governance state advance in block order through the ledger's own certificate processing, starting from the bootstrap certificate state; any step failure aborts the window without producing a result, and a partially applied window leaves the checkpoint marked incomplete, never producing a stake snapshot from partial data; the checkpoint can be rebuilt from the blocks by replay.
    counterexample: The checkpoint's delta is computed by a separate parser that misses collateral returns, so the reduced stake drifts from the ledger's; or a crash mid-window leaves a checkpoint that is read as complete.
    secondary:      [REC]
    scope:          reduced UTxO checkpoint
    assumptions:    [A1]
    depends_on:     [#8, REC-002, VAL-016]
    locality:       transactional
    legacy:         [DC-EVIEW-04, DC-EVIEW-04b, DC-EVIEW-10]

#10 VAL  PoolLifecycleMatchesReference  [critical]
    statement:      The pool lifecycle matches the reference ledger: a first registration takes effect immediately; a re-registration of a registered pool is staged as a future registration, leaving the active parameters and VRF key unchanged until it is adopted at the next boundary, and cancels any pending retirement; at every boundary crossed, including each boundary inside a replayed range, the stake snapshot is taken first (VAL-028), then pool reaping adopts staged registrations (dropping any whose pool is no longer active), reaps exactly the pools retiring at that epoch, and removes delegations to the reaped pools while keeping those credentials' registrations and reward accounts.
    counterexample: A pool's re-registration with a new VRF key takes effect mid-epoch, or a credential delegated to a reaped pool stays delegated and silently reattaches when the pool ID registers again.
    secondary:      []
    scope:          pool registration, retirement and reaping
    assumptions:    [A2]
    depends_on:     [VAL-011, VAL-020, VAL-028]
    locality:       transactional
    legacy:         [DC-EVIEW-13]

#11 STR  SeedRecordClosedSchema  [critical]
    statement:      The persisted seed-epoch consensus-input record is one closed type holding the seed epoch, the active-slot coefficient, the total active stake, each pool's active stake and VRF key hash in canonical key order, the epoch nonce, the epoch geometry and the consensus profile; its decoder accepts only the current format version and only the canonical encoding, so re-encoding a decoded record gives exactly its input bytes, and anything else is rejected.
    counterexample: A record with its pools in non-canonical order decodes and is silently re-sorted, so its stored hash no longer matches its bytes; or a record of an older version is decoded under the new layout.
    secondary:      [VAL]
    scope:          seed-epoch consensus-input record
    assumptions:    []
    depends_on:     [VAL-001, DET-003, DET-007]
    locality:       local
    legacy:         [CN-CINPUT-01]

#12 SAF  CandidateViewHashVerified  [critical]
    statement:      Before an epoch view or sealed leadership object is activated or read, its canonical hash is recomputed over every binding (network, era, epoch, source point, checkpoint commitment, nonce, snapshot phase and profile commitment) and over its whole stake and VRF payload, and must equal its stored hash; its bindings cannot change once bound. The profile commitment it binds is blake2b-256 over a domain tag, the genesis hash, the protocol-parameter hash and the active-slot coefficient, all three taken from the durable consensus profile (#6).
    counterexample: A sealed object corrupted on disk keeps its stored identity fields, is matched by identity alone, and its altered stake map decides leadership.
    secondary:      [DUR]
    scope:          epoch views and sealed leadership objects
    assumptions:    []
    depends_on:     [SAF-004, SAF-005, VAL-024, #6]
    locality:       local
    legacy:         [DC-EVIEW-07]
```

## Transitions in scope

- `import_consensus_inputs(Bundle) -> Result<ConsensusInputsRecord, ImportError>` (shell)
- `recover_seed_inputs(Store, Wal) -> Result<SeedEpochConsensusInputs, BootstrapError>`
- `classify_stake_ref(AddressBytes, Era) -> StakeRefClass`
- `apply_reduced_delta(Checkpoint, Block) -> Result<Checkpoint, CheckpointError>`
- `register_pool(PoolState, Registration) -> PoolState` and `reap_pools(CertState, EpochNo) -> CertState`

## TCB color hypothesis

- BLUE: `ade_ledger` (seed record codec, stake classification, reduced aggregation, pool lifecycle), `ade_codec` (address decoding), `ade_core::consensus` (VRF input)
- GREEN: none
- RED: `ade_runtime` (bundle importer, seed persistence, warm start, transient store, checkpoint storage), `ade_node` (bootstrap composition)

## Open questions

- S1. CN-CINPUT-01 (the seed-epoch record is one closed canonical type with a sole, version-gated, byte-canonical codec) — drafted as #11, which adds the record's fields, current-version-only acceptance and canonical-only decoding to what VAL-001 and DET-007 already require.
- S2. DC-CINPUT-02a, DC-CINPUT-02b, DC-CINPUT-04 (forge and feed views projected from the recovered seed record) — merged into VAL-023 LeadershipSealedAtBoundaryPoint (the bootstrap epochs are sealed from the certified record), VAL-024 LeadershipFromSealedView (leader checks read only sealed objects; missing fails closed) and STR-007 SingleEpochAuthority (one view for validation and leadership). The legacy record itself retired the projection read sites in favour of sealed objects (DC-EPOCH-25); the clause requiring equivalence with the operator-bundle projection is excluded, since that bundle is no longer a source.
- S3. DC-EVIEW-05, DC-EVIEW-12 (per-pool stake aggregation; the leadership-complete view with VRF keys and profile commitment) — merged into VAL-020 EpochBoundaryMatchesReference (snapshot stake matches the reference), VAL-023 (the sealed pool set, stake and VRF keys) and VAL-024 (profile commitment verified before use), with one stated departure: both legacy entries compute the total active stake by summing the retained pool entries, which VAL-024 forbids; their denominator rule is excluded as superseded by VAL-024's credential-side total.
- S4. DC-EVIEW-07 (a bound, immutable epoch view, inert unless every binding matches and its canonical hash verifies) — drafted as #12; SAF-004 and SAF-005 carry the identity and mismatch halves.
- S5. DC-EVIEW-11 (the epoch-rebind seam: promote only a matching view for the immediate next epoch, else fail closed) — merged into ORD-004 ActivationAdvancesOneBoundary and SAF-005.
- S6. DC-EVIEW-06 (a snapshot is usable only once its boundary block is more than k deep) — excluded as superseded: it was recorded as observe-only and never wired in, and sealed leadership objects with rewind coherence (VAL-023, SAF-006) replaced it as the protection against a boundary being rolled back.
- S7. DC-EVIEW-08 (activation through window replay) — excluded as superseded within the legacy record by sealed leadership objects (DC-EPOCH-25, VAL-023, VAL-024).
- S8. DC-EVIEW-09 (manifest-bound bootstrap certificate-state import) and DC-CINPUT-07 (Conway deposit parameters decoded from the certified snapshot, never defaulted) — deferred to B11 mithril-evidence, which covers bootstrap import.
- S10. DC-CONS-IN-02's binding of the import fingerprint into every admission event record — deferred to B10 pipeline-production, with the admission entries.
- S9. DC-EVIEW-01's memory gate (the transient store keeps its bulk off the heap, below a fixed ceiling) — deferred to B12 resources-release with the memory invariants; its authority, crash and purge clauses are drafted as #7.
```
