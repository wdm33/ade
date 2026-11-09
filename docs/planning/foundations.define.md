# Define — foundations

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B01 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It defines the cross-cutting foundations: determinism, encoding, core purity, transitions, errors, build, boundary, process, keys, conservation, spending, causality and resource bounds. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 31 legacy IDs appears either on a line or in a settlement below.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line), docs/completed/phase_0a/phase_0a_implementation_plan.md, docs/completed/phase_0b/phase_0b_implementation_plan.md, corpus/reference/ledger_state_hashes/hash_surface.md

## Drafts

```text
#1  DET  ReplayByteIdentity  [critical]
    statement:      Replaying the same canonical starting state with the same ordered canonical inputs, protocol version and recorded context produces byte-identical authoritative outputs, including the ledger-state fingerprint, persisted state encodings, the consensus event stream, declared effects and every validity verdict.
    counterexample: Two replays of one block stream from one snapshot end with ledger-state fingerprints that differ, or with verdict streams or persisted state encodings that differ in any byte.
    secondary:      [REC]
    scope:          authoritative core (ledger, consensus, block and transaction validity, persisted state)
    assumptions:    [N1]
    depends_on:     []
    locality:       local
    legacy:         [T-DET-01]

#2  DET  DesignatedByteAuthority  [critical]
    statement:      Each authoritative hash, signature check and persisted or transmitted byte string uses its designated byte authority: protocol hashes and signatures (block header, block body, transaction ID, script and datum hashes) are computed over the bytes as they were received or produced on the wire, never over a re-encoding of decoded data; Ade's own commitments (state fingerprints and persisted records) are computed over the output of the single canonical encoder for that type.
    counterexample: A block-body hash computed over a re-encoding of the decoded body, for a body whose producer used an indefinite-length array, so the hash differs from the one in the signed header and a valid block is rejected; or a state fingerprint computed over two different encodings of the same state.
    secondary:      [VAL]
    scope:          codec, ledger, consensus, storage
    assumptions:    [N2]
    depends_on:     []
    locality:       local
    legacy:         [T-ENC-01]

#3  DET  RoundTripIdentity  [critical]
    statement:      For every byte string that decodes successfully, encoding the decoded value yields exactly the original bytes.
    counterexample: A transaction whose witness set uses an indefinite-length map decodes, and re-encodes to a definite-length map with different bytes.
    secondary:      [VAL]
    scope:          codec (ledger and mini-protocol encodings)
    assumptions:    [N2]
    depends_on:     [#2]
    locality:       local
    legacy:         [T-ENC-03]

#4  STR  AuthoritativeCorePurity  [critical]
    statement:      Code in the BLUE core paths performs no I/O, reads no clock, environment or randomness, spawns no tasks and contains no async code; a core function's only inputs are its arguments.
    counterexample: A ledger rule that reads the system time to decide a validity interval, or a BLUE function declared async that awaits a channel.
    secondary:      [DET]
    scope:          BLUE core paths (.idse-config.json core_paths)
    assumptions:    []
    depends_on:     []
    locality:       local
    legacy:         [T-CORE-01, T-CORE-02, DC-CORE-01]

#5  DET  HostIndependentResults  [critical]
    statement:      No BLUE computation's result depends on floating point, on a collection's iteration order unless that order is canonical, or on the host's locale, timezone, pointer width or endianness.
    counterexample: A reward computation that rounds through f64 and gives a different lovelace amount on another architecture, or a state fingerprint built by iterating a HashMap.
    secondary:      []
    scope:          BLUE core paths
    assumptions:    []
    depends_on:     [#4]
    locality:       local
    legacy:         [T-CORE-02, T-COLL-01, T-PLATFORM-01]

#6  SAF  AtomicTransitions  [critical]
    statement:      Every authoritative state change is a transition that takes the prior state, the input and explicit context, and returns either the complete new state with its declared effects or a structured rejection; a rejection leaves the prior state exactly as it was.
    counterexample: Applying a block fails on its third transaction and returns an error, but the first two transactions' UTxO changes remain in the ledger state.
    secondary:      [STR]
    scope:          ledger, consensus and chain-state transitions
    assumptions:    []
    depends_on:     [#4]
    locality:       transactional
    legacy:         [T-CORE-03]

#7  DET  StructuredDeterministicFailures  [standard]
    statement:      Every consensus-relevant failure, whether a rejection a transition returns or an internal inconsistency that triggers the halt of #8, is reported as a value of a closed, structured type that carries no free-form text and no host-dependent content; given equivalent prior state, canonical input, protocol version and explicit context, two executions report byte-identical failures.
    counterexample: A ledger rejection that carries a formatted string containing a file path, or two runs from the same state, transaction, protocol version and context that reject with different error variants.
    secondary:      [OBS]
    scope:          ledger, consensus and codec failures, and the halt reports of #8
    assumptions:    [N1]
    depends_on:     [#4]
    locality:       local
    legacy:         [T-ERR-01, CN-META-02]

#8  SAF  FailFastOnInvariantRisk  [critical]
    statement:      When the node detects an internal inconsistency that could corrupt authoritative state, replay or chain selection, as opposed to an invalid external input, it halts deterministically instead of continuing.
    counterexample: After applying a block, the ledger's recomputed UTxO total disagrees with its running total; the node logs a warning and keeps following the chain.
    secondary:      []
    scope:          node-wide
    assumptions:    []
    depends_on:     [#6]
    locality:       local
    legacy:         [T-ERR-02]

#9  DET  BuildInvariantSemantics  [critical]
    statement:      No cargo feature, cfg flag, build profile or optimization level changes an authoritative result or a persisted byte.
    counterexample: An arithmetic overflow in a ledger rule that panics in a debug build and wraps in a release build, or a feature-gated code path that rounds a reward differently.
    secondary:      [EVO]
    scope:          BLUE core paths and persisted formats
    assumptions:    []
    depends_on:     [#4]
    locality:       local
    legacy:         [T-BUILD-01, CN-BUILD-01]

#10 EVO  SingleSemanticInterpretation  [critical]
    statement:      Given identical canonical inputs and identical explicit governing data (protocol version, on-chain protocol parameters, genesis and bootstrap-anchor data), exactly one semantic interpretation exists: no build option, operator configuration value, command-line flag or alternative code path changes ledger, consensus or persistence semantics. Operator configuration may change only transport, logging and telemetry.
    counterexample: Two implementations of Conway reward calculation, selected by a configuration flag, or a configuration option that disables a ledger validation rule.
    secondary:      [DET]
    scope:          node-wide
    assumptions:    [N2]
    depends_on:     [#9]
    locality:       local
    legacy:         [T-BUILD-02, CN-BUILD-02, CN-BUILD-03, CN-BUILD-04]

#11 STR  ShellCanonicalizesInputs  [critical]
    statement:      Transport artifacts (socket fragmentation, mux segment boundaries, interleaving across mini-protocols, read timing and timeouts) never reach the core: two transport histories that deliver the same ordered sequence of whole messages to each mini-protocol, from the same starting state, under the same protocol version, with the same recorded context and the same authoritative input order, produce identical core outcomes. Nondeterministic facts the core legitimately needs, such as the current slot, enter only as recorded explicit context.
    counterexample: A chain-sync validity decision that differs depending on how a header was split across mux segments, or a core function that reads a timeout flag the shell set.
    secondary:      [DET]
    scope:          shell/core boundary, mux demultiplexing
    assumptions:    [N1]
    depends_on:     [#4]
    locality:       local
    legacy:         [T-BOUND-01, T-TRANSPORT-01]

#12 STR  CoreDependsOnlyInward  [critical]
    statement:      No BLUE path depends on RED code: a crate whose modules are all BLUE depends, directly or transitively, on no RED crate and on no library providing networking I/O, filesystem access, an async runtime, a clock, randomness or a general-purpose serialization framework; a BLUE module inside a mixed crate imports none of that crate's RED modules and none of those libraries.
    counterexample: ade_ledger's Cargo.toml gains a dependency on ade_runtime, tokio or serde_json, or the BLUE module ade_network::mux::frame imports ade_network::mux::transport.
    secondary:      []
    scope:          workspace crate and module graph
    assumptions:    []
    depends_on:     [#4]
    locality:       local
    legacy:         [T-BOUND-02]

#13 PRC  EveryInvariantMechanicallyEnforced  [standard]
    statement:      Every active invariant in the registry has at least one substantive mechanical check (a test, a CI script, or a type the build compiles) that the registry names, that exists at the current revision, and that the project's check suite (test_cmd and the ci/ scripts) runs; no entry is counted as enforced by waiver.
    counterexample: An active registry entry whose only listed check is a test that does not exist, or a CI script that no part of the check suite runs.
    secondary:      []
    scope:          invariant registry and check suite
    assumptions:    []
    depends_on:     []
    locality:       local
    legacy:         [T-CI-01, CN-META-01]

#14 PRC  ReproducibleEquivalenceClaims  [standard]
    statement:      Every claim that Ade agrees with the reference node names the exact reference build, the comparison surface, the fixtures and the replay inputs it was checked against, and rerunning the named check on those reproduces the claimed result; artifacts extracted from one reference build never establish agreement with a different build.
    counterexample: A "block validity agrees with the reference node" claim backed only by a log from a live run, or ledger-state hashes extracted from cardano-node 10.6.2 cited as evidence of agreement with 11.0.1.
    secondary:      [OBS]
    scope:          compatibility evidence
    assumptions:    [N2]
    depends_on:     [#1]
    locality:       local
    legacy:         [CN-META-03]

#15 SEC  SigningInShellVerificationInCore  [critical]
    statement:      Private key material (KES, VRF and cold keys) is loaded, held and passed only by RED shell code, and every signing or proving call is made from RED code; no BLUE transition takes a private key as input or produces a signature or VRF proof. Every signature, VRF and KES verification that decides whether a block, header or transaction is accepted is computed in BLUE code, and no RED code passes a verification verdict into a BLUE transition.
    counterexample: A BLUE block-construction function that takes a KES signing key and signs the header, or a RED function that checks a header's KES signature and hands the BLUE header validator a boolean it trusts.
    secondary:      [STR]
    scope:          keys, forging, verification
    assumptions:    []
    depends_on:     [#4]
    locality:       local
    legacy:         [T-KEY-01]

#16 CON  ValueConservation  [critical]
    statement:      Every accepted ledger transition preserves value as the Conway ledger defines it. For a transaction applied with its scripts valid, consumed value (inputs, withdrawals, deposit refunds, minted value) equals produced value (outputs, fee, new deposits, treasury donation, burned value); for a transaction applied with its scripts invalid, only its collateral inputs are consumed, and they equal the collateral return plus the collateral collected as fee. Reserves, treasury, reward accounts, deposits and the fee pot change only through the protocol rules that authorize those movements.
    counterexample: A script-valid transaction whose produced value (outputs, fee, new deposits, treasury donation, burned value) exceeds its consumed value (inputs, withdrawals, deposit refunds, minted value) by one lovelace is accepted, a script-invalid transaction whose collateral return exceeds its collateral inputs is accepted, or an epoch transition moves funds into the treasury without a rule authorizing it.
    secondary:      [VAL]
    scope:          ledger (transactions and epoch transitions)
    assumptions:    [N2]
    depends_on:     [#6]
    locality:       transactional
    legacy:         [T-CONSERV-01]

#17 SAF  SingleSpendPerHistory  [critical]
    statement:      Along any one adopted chain history, no transaction output is consumed more than once, whether as a regular input or as collateral.
    counterexample: Two blocks on the adopted chain each contain a transaction spending the same TxIn, one as a regular input and one as collateral, and both are accepted.
    secondary:      [VAL]
    scope:          ledger UTxO
    assumptions:    [N2]
    depends_on:     [#6]
    locality:       transactional
    legacy:         [T-NOSPEND-01]

#18 ORD  NoRetroactiveValidation  [critical]
    statement:      The verdict on a block or transaction depends only on the chain prefix before it and on context fixed by that prefix or recorded before its validation, such as the slot it claims; no data first derived after that point (a later block, a later epoch's stake distribution or nonce, a later checkpoint) changes its verdict, and an adopted checkpoint is never reinterpreted under a different rule version.
    counterexample: A header's VRF leader check uses the epoch nonce of the epoch after the header's own, a value that exists only once that later epoch's stability window has passed; or a stored checkpoint is reloaded under a newer rule version and its blocks are judged again with a different verdict.
    secondary:      [VAL]
    scope:          ledger and consensus validation
    assumptions:    [N1, N2]
    depends_on:     [#1]
    locality:       local
    legacy:         [T-CAUSAL-01]

#19 SEC  BoundedPreValidationAllocation  [critical]
    statement:      Before deterministic validation accepts it, an untrusted input can make the node allocate memory or do work only up to a fixed, stated bound that does not depend on length fields the input itself declares.
    counterexample: A CBOR array header from a peer that declares 2^32 elements makes the decoder preallocate a 2^32-element vector.
    secondary:      [SAF]
    scope:          ingress: codec, mini-protocols, block and transaction admission
    assumptions:    []
    depends_on:     []
    locality:       local
    legacy:         [T-RESOURCE-01]
```

## New assumptions

```text
N1  ordered-recorded-inputs: for each authoritative state machine (ledger, chain selection, and each mini-protocol's state), the shell delivers inputs one at a time in one total order, and records the starting state, that order and each input's explicit context, so the history can be replayed completely.
N2  reference-node-semantics: the Haskell cardano-node defines the reference semantics for every observable protocol behaviour Ade claims to match; each comparison names the exact reference build it uses (the live venue runs 11.0.1; the pinned ledger-state hash corpus was extracted from 10.6.2). Ade executes only Conway semantics: it enters at a certified non-Origin Conway tip and never replays or executes historical eras, keeping historical era facts only as identifiers and slot-time geometry.
```

## Transitions in scope

- `apply_block(LedgerState, Block, Context) -> Result<(LedgerState, Effects), BlockRejection>`
- `apply_tx(LedgerState, Tx, Context) -> Result<(LedgerState, Effects), TxRejection>`
- `select_chain(ChainState, Candidate, Context) -> Result<(ChainState, Effects), SelectionRejection>`
- `decode(Bytes) -> Result<Value, DecodeError>` and `encode(Value) -> Bytes` for every authoritative wire and ledger type

## TCB color hypothesis

- BLUE: `ade_codec`, `ade_types`, `ade_crypto`, `ade_plutus`, `ade_ledger`, `ade_core`, and in `ade_network` the modules listed in `core_paths`: `mux/frame.rs`, `codec/`, `handshake/`, `chain_sync/`, `block_fetch/`, `tx_submission/`, `keep_alive/`, `peer_sharing/`, `n2c/`
- GREEN: `ade_testkit` replay and differential harnesses
- RED: `ade_runtime`, `ade_node`, `ade_core_interop`, and the rest of `ade_network` (mux transport and session I/O)

## Open questions

- S1. T-ENC-02 ("non-canonical bytes rejected deterministically") — deferred to B02 codec-wire-crypto. As written it contradicts Cardano: valid CBOR need not be canonical, and Ade must accept such bytes and preserve them (#2, #3). Its intended property, that bytes outside a type's closed grammar are rejected deterministically, belongs with the B02 decoder invariants.
- S2. T-CORE-04 ("illegal states unrepresentable via types where practical") — excluded. "Where practical" cannot be falsified, and the rule is already Constitution rule 4, applied through each invariant's type-level witness rather than as a project invariant.
- S3. T-CORE-02 is split — drafted as #4 and #5. Its clock and randomness clauses go to #4, its float and collection clauses to #5.
- S4. Whether the halt of #8 may cost a block producer its slot — deferred to B09 node, whose liveness lines state the node's progress assumptions, including restart after a halt.
- S5. Phase 0A's ade_core extraction rule (split a crate when it gains a second invariant family) — excluded. The split it required has happened: ledger rules live in ade_ledger, Plutus in ade_plutus, protocol state machines in ade_network, and ade_core holds consensus. Crate layout is not an authoritative-state property, and moving BLUE boundaries is a human decision (Doctrine §20).
- S6. Phase 0B's rule that BLUE crates carry no GREEN-only dependencies (serde, serde_json, toml) — drafted as #12, which forbids general-purpose serialization frameworks in BLUE.
- S7. Phase 0B's deterministic GREEN harness requirements (deterministic DiffReport ordering, manifest provenance) — deferred to B11 mithril-evidence, which carries the differential, reference and evidence entries.
- S8. The hash-surface rule that reference artifacts are valid only for the build they were extracted from — drafted into #14 and N2.
- S9. #13 holds only once every migrated invariant has a check. At the migration's re-evidence pass, entries carried from legacy "declared" records will have none, so #13's claim will be recorded as falsified until later clusters add checks. That is its honest status, not a reason to weaken it.
```
