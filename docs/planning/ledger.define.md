# Define — ledger

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B03 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers block and transaction validity, witnesses, certificates and deposits, values and parameters, Plutus and native scripts, and the phase-2 collateral path. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 55 legacy IDs appears on a line or in a settlement.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants that settlements merge into)

## Drafts

```text
#1  VAL  BlockValidityMatchesReference  [critical]
    statement:      For every Conway block, Ade's valid or invalid verdict equals the reference node's verdict for the same prior ledger and consensus state and protocol version, and wherever the reference exposes a rejection reason, Ade's reason falls in the same class.
    counterexample: A block containing a transaction the block marks phase-2-valid whose Plutus script exceeds its execution budget, which the reference node rejects for the mismatch and Ade accepts.
    secondary:      []
    scope:          block validation
    assumptions:    [A2]
    depends_on:     [#3, #22]
    locality:       local
    legacy:         [DC-LEDGER-03, CN-LEDGER-03, DC-VAL-04]

#2  VAL  TxValidityMatchesReference  [critical]
    statement:      For every Conway transaction, Ade's valid or invalid verdict equals the reference node's verdict for the same ledger state and protocol version, and wherever the reference exposes a rejection reason, Ade's reason falls in the same class.
    counterexample: A transaction that the reference node rejects for a missing required signer, which Ade accepts.
    secondary:      []
    scope:          transaction validation (block path and standalone)
    assumptions:    [A2]
    depends_on:     [#3]
    locality:       local
    legacy:         [DC-LEDGER-03, CN-LEDGER-03, DC-TXV-03]

#3  VAL  CompleteValidation  [critical]
    statement:      A block is valid only if both the consensus header authority and the ledger body authority accept it. A transaction in a block is accepted only if phase 1 accepts it (native scripts included) and the outcome of evaluating its Plutus scripts agrees with the phase-2 status the block declares for it: evaluation succeeds for a transaction marked valid and fails for one in the block's invalid-transactions set, which then takes the collateral path. A standalone transaction is admitted only if phase 1 accepts it and its Plutus scripts evaluate successfully. No path produces a valid verdict while skipping any of these checks.
    counterexample: A block-validation path used during recovery that applies the body without running header validation and reports the block valid; or a block that marks a transaction phase-2-invalid although its scripts succeed, which is accepted.
    secondary:      []
    scope:          block and transaction validation
    assumptions:    [A2]
    depends_on:     []
    locality:       local
    legacy:         [DC-VAL-02, DC-TXV-02]

#4  ORD  HeaderBeforeBody  [critical]
    statement:      A block's header is validated before its body, its body is never validated when its header is invalid, and the first authority to fail determines the block's rejection reason.
    counterexample: The body of a block with an invalid KES signature is applied to a scratch ledger state, and the block is reported as rejected for a body error.
    secondary:      [VAL]
    scope:          block validation
    assumptions:    []
    depends_on:     [#3]
    locality:       local
    legacy:         [DC-VAL-03]

#5  VAL  ChecksNeverSkip  [critical]
    statement:      Every size, shape and cryptographic-input check on the validation path rejects when its input has the wrong size or shape and never skips silently; no check is defined but left unwired, and no guard compares a value with itself.
    counterexample: A signature check written as "if the signature is 64 bytes, verify it, otherwise skip", so a 63-byte signature passes.
    secondary:      []
    scope:          BLUE block and transaction validation
    assumptions:    []
    depends_on:     [#3]
    locality:       local
    legacy:         [DC-VAL-06]

#6  SEC  WitnessesBindTransaction  [critical]
    statement:      A Conway transaction is accepted only if every key authority it requires has a key or bootstrap witness that verifies over the transaction's preserved body hash, where the required key authorities are the closed set of the key-hash payment credentials and bootstrap addresses of its resolved spending and collateral inputs, its explicit required signers, and the key-hash credentials its certificates, withdrawals, votes and proposals require; every key and bootstrap witness it supplies, required or not, verifies under its own key over that hash. Script-hash credentials are authorized by script witnesses, not keys (#19).
    counterexample: A transaction that withdraws rewards from a key-hash stake credential with no witness for that key, which is accepted; or a supplied but unneeded witness with an invalid signature, which is accepted.
    secondary:      [VAL]
    scope:          transaction witnesses (block path and standalone)
    assumptions:    [A2]
    depends_on:     [DET-002, SEC-003, #19]
    locality:       local
    legacy:         [DC-LEDGER-05, CN-LEDGER-09, DC-TXV-05, DC-TXV-08]

#7  CON  DepositAccountingClosed  [critical]
    statement:      Every certificate's deposit effect resolves through a closed, total function to exactly one of a new deposit, a refund, no deposit effect, or a rejection; each amount comes only from the certificate's explicit deposit field or the ledger's current protocol parameters, never from a default, configuration or fallback constant; an unrecognized or malformed certificate, or a state-dependent case Ade cannot account for, is rejected.
    counterexample: A stake-registration certificate whose deposit is taken from a hard-coded 2 ADA default instead of the current keyDeposit parameter, or a certificate with an unknown tag treated as having no deposit effect.
    secondary:      [VAL]
    scope:          transaction certificates, value accounting
    assumptions:    [A2]
    depends_on:     [CON-001]
    locality:       local
    legacy:         [DC-TXV-06, DC-TXV-07]

#8  VAL  CertificateTransitionsClosed  [critical]
    statement:      Every Conway certificate decodes through the Conway certificate grammar with all its payload fields and resolves to one closed disposition: either a structured rejection, or the complete list of its effects, each applied exactly once by the owner of the state it changes, delegation and pool state or governance state (vote delegation, committee hot keys, DRep expiry). A composite certificate has each half applied once by its own owner; no effect is dropped, flattened or applied twice, and DRep expiry is computed only from explicit governance context.
    counterexample: A combined stake-and-vote delegation certificate whose stake half is applied by both the delegation owner and the governance owner, or a committee hot-key certificate that is decoded and then ignored.
    secondary:      []
    scope:          ledger certificate state and Conway governance state
    assumptions:    [A2]
    depends_on:     [VAL-001, SAF-001]
    locality:       local
    legacy:         [CN-LEDGER-10, DC-LEDGER-08, DC-LEDGER-09]

#9  STR  CredentialDiscriminantPreserved  [critical]
    statement:      A stake, committee or DRep credential is a closed sum of a key hash or a script hash over 28 bytes everywhere: in decoding, as a ledger-state key and in comparison; a key-hash credential and a script-hash credential with the same 28 bytes are distinct, no conversion from a bare 28-byte hash to a credential exists on the authoritative path, and every authoritative encoding and fingerprint of a credential includes its key-or-script tag.
    counterexample: A script-hash stake credential and a key-hash stake credential with the same 28 bytes share one reward account in Ade's ledger state.
    secondary:      [VAL]
    scope:          ledger credentials, certificate and governance state
    assumptions:    [A2]
    depends_on:     [VAL-001]
    locality:       local
    legacy:         [DC-LEDGER-10]

#10 VAL  NetworkConstantsFromSchedule  [critical]
    statement:      No network-specific constant enters an authoritative computation except through the network's declared era schedule and genesis data; every epoch number on an authoritative path is derived from the caller's era schedule, and any mainnet-only constant is reachable only through an explicitly named mainnet schedule.
    counterexample: A slot-to-epoch conversion that applies mainnet's Shelley start slot and epoch length to a preprod slot, producing a fictitious epoch number.
    secondary:      [DET]
    scope:          ledger and consensus epoch and slot arithmetic
    assumptions:    [A2]
    depends_on:     [EVO-001]
    locality:       local
    legacy:         [DC-LEDGER-13]

#11 VAL  OutputQuantityFullDomain  [critical]
    statement:      Every output asset quantity holds Cardano's full unsigned 64-bit domain, a negative output quantity cannot be represented, and output-value arithmetic is checked: overflow and underflow are structured errors, never a wrap or a saturation.
    counterexample: An output carrying 2^63 units of a token is rejected because the quantity is stored as a signed 64-bit integer, or adding two outputs' quantities wraps to a small number.
    secondary:      [CON]
    scope:          ledger values and UTxO outputs
    assumptions:    [A2]
    depends_on:     []
    locality:       local
    legacy:         [DC-LEDGER-VALUE-01]

#12 VAL  ProtocolParametersEraFaithful  [critical]
    statement:      Imported protocol parameters keep the meaning their era gives them and are never remapped: Conway's coins-per-UTxO-byte is held as a per-byte coefficient, in a representation distinct from an absolute minimum, and the Conway minimum-UTxO check requires each output's coin to be at least the coefficient times (160 plus the byte length of the output's CBOR serialization as it appears in the transaction), as the reference node computes it.
    counterexample: Conway's coinsPerUTxOByte of 4310 imported as an absolute 4310-lovelace minimum, so an output far below the real minimum is accepted.
    secondary:      []
    scope:          protocol parameters, snapshot import, Conway minimum-UTxO validation
    assumptions:    [A2]
    depends_on:     [VAL-001]
    locality:       local
    legacy:         [DC-LEDGER-PARAMS-01]

#13 DET  PlutusEvaluationDeterministic  [critical]
    statement:      Evaluating a Plutus script with the same script, Plutus language version, arguments, script context, cost model and initial execution budget always gives the same result and the same execution-budget consumption, on every run and every host.
    counterexample: Two evaluations of the same script against the same context that report different consumed CPU units, or one that succeeds on one host and runs out of budget on another.
    secondary:      []
    scope:          ade_plutus evaluation
    assumptions:    []
    depends_on:     [DET-004]
    locality:       local
    legacy:         [DC-PLUTUS-01, CN-PLUTUS-01]

#14 VAL  ScriptContextCanonical  [critical]
    statement:      The script context a Plutus script receives is derived canonically and completely from the transaction, the resolved ledger state and the network-wide constants (system start and era history); nothing else enters it.
    counterexample: The validity range in the script context is converted to POSIX time with the host clock, or with mainnet's system start on a testnet.
    secondary:      [DET]
    scope:          script-context construction
    assumptions:    [A2]
    depends_on:     [#10, STR-001]
    locality:       local
    legacy:         [DC-LEDGER-06, CN-PLUTUS-03]

#15 VAL  SinglePhaseTwoEffect  [critical]
    statement:      A transaction's UTxO and accounting effect is derived in exactly one place, from the block and its invalid-transactions set: a phase-2-valid transaction spends its ordinary inputs and produces its ordinary outputs at indices 0 to n-1; a phase-2-invalid transaction spends only its collateral inputs, produces only its collateral return at index n, where n is its number of ordinary outputs, adds its consumed collateral to the fee pot, and has no other effect, its certificates, withdrawals, votes and proposals being discarded. Consumers receive only the derived spends, outputs and fee contribution, never the phase-2 status, so they cannot branch on it.
    counterexample: A phase-2-invalid transaction whose ordinary inputs are removed from the UTxO, or whose stake registration certificate is applied.
    secondary:      [CON]
    scope:          UTxO application and accumulators
    assumptions:    [A2]
    depends_on:     [CON-001, #16]
    locality:       transactional
    legacy:         [DC-LEDGER-PHASE2-01, DC-LEDGER-PHASE2-03]

#16 CON  CollateralFromResolvedInputs  [critical]
    statement:      The ADA a phase-2-invalid transaction consumes equals the sum of its resolved collateral inputs' values minus its collateral return; an unresolved collateral input is a typed refusal, never zero or a skipped contribution, and the declared total-collateral field is never used as the value. The accumulator receives these values from the UTxO authority through a resolver; it neither owns nor rebuilds a UTxO.
    counterexample: A collateral input missing from the reduced UTxO is counted as zero, so the fee pot is credited too little; or the total-collateral field is used in place of the resolved inputs.
    secondary:      [VAL]
    scope:          collateral accounting
    assumptions:    [A2]
    depends_on:     [CON-001]
    locality:       local
    legacy:         [DC-LEDGER-PHASE2-02]

#17 VAL  NativeScriptThresholdSigned  [critical]
    statement:      A native script's at-least-n-of-k threshold is decoded and held as a signed 64-bit integer: every integer in that range is accepted and every other refused, a threshold of zero or less is satisfied, and a positive threshold is satisfied exactly when at least that many sub-scripts pass.
    counterexample: A threshold of -1 is rejected at decode because it is read as an unsigned integer, though the reference node accepts the script and treats it as satisfied.
    secondary:      []
    scope:          native scripts
    assumptions:    [A2]
    depends_on:     [VAL-001]
    locality:       local
    legacy:         [DC-LEDGER-SCRIPT-01]

#18 SEC  BoundedStackDepth  [critical]
    statement:      No code path that decodes, skips, evaluates, clones, compares or drops a wire-supplied native script, or skips any wire-supplied CBOR item, uses call-stack depth that grows with the input's nesting depth; pending work is kept on the heap, and no nesting limit below what transaction and block size allow is imposed.
    counterexample: A recursive native-script decoder that overflows the stack on a transaction nesting 100,000 all-of scripts, crashing the node; or a depth limit of 64 that rejects a valid block.
    secondary:      [SAF]
    scope:          CBOR skipping, native-script decoding and evaluation
    assumptions:    [A2]
    depends_on:     [SEC-002]
    locality:       local
    legacy:         [DC-LEDGER-SCRIPT-02]

#19 SEC  ScriptWitnessClosureSingleAuthority  [critical]
    statement:      Whether a Conway transaction's script witnesses are sufficient and not extraneous is decided by one authority, which both the block path and the standalone transaction path call: with every spending and reference input resolved it gives the reference node's verdict; without full UTxO context it rejects exactly when a supplied native script fails evaluation and otherwise returns undetermined, never valid.
    counterexample: The standalone path accepts a transaction with an extraneous script witness that the block path rejects, or reduced mode reports valid for a transaction whose required script was never checked.
    secondary:      [VAL]
    scope:          script witness closure (block path, standalone, reduced mode)
    assumptions:    [A2]
    depends_on:     [#3, #17]
    locality:       local
    legacy:         [DC-LEDGER-SCRIPT-03]

#20 VAL  IntervalNetworkAndBootstrapChecks  [critical]
    statement:      Every Conway transaction in a block lies inside its validity interval at the block's slot (invalid-before at most the slot, slot below invalid-hereafter); every output, collateral return and withdrawal account, and the optional body network field, carry the node's network id; and every bootstrap output's attributes total at most 64 bytes; a violation rejects the transaction and the block, with or without full UTxO context.
    counterexample: A transaction whose invalid-hereafter equals the block's slot is accepted, or an output paying a mainnet address is accepted on preprod.
    secondary:      []
    scope:          transaction validation in the block path and standalone
    assumptions:    [A2]
    depends_on:     [#3]
    locality:       local
    legacy:         [DC-TXV-09]

#21 VAL  BlockBodySizeMatchesHeader  [critical]
    statement:      A Conway block's header body-size field equals the byte length of its four body segments (transaction bodies, witness sets, auxiliary data and invalid transactions) concatenated as received; a mismatch rejects the block.
    counterexample: A block whose header claims a body one byte longer than its actual body segments is accepted.
    secondary:      []
    scope:          block validation
    assumptions:    [A2]
    depends_on:     [DET-002]
    locality:       local
    legacy:         [DC-VAL-07]

#22 EVO  ReleasesAgreeOnVerdicts  [critical]
    statement:      Any two supported Ade releases that are authorized to run on the network at the same time return the same verdict for every consensus-relevant block and transaction, given the same prior state, inputs, protocol version and explicit context.
    counterexample: An Ade release that fixed a decoder bug accepts a transaction the previous release rejects, while operators run both on the same network.
    secondary:      [VAL]
    scope:          Ade release compatibility
    assumptions:    [A2]
    depends_on:     [EVO-001]
    locality:       distributed
    legacy:         [DC-LEDGER-07, CN-LEDGER-04]

#23 STR  TypedProposalProcedures  [standard]
    statement:      In the authoritative Conway transaction-body type, proposal procedures are held as typed governance actions with their payloads, decoded by one closed decoder, never as opaque bytes, and the typed form re-encodes to the original bytes.
    counterexample: The transaction body keeps proposal procedures as an uninterpreted byte string after checking only that they are well-formed CBOR.
    secondary:      [VAL]
    scope:          Conway transaction body
    assumptions:    [A2]
    depends_on:     [VAL-001, DET-003]
    locality:       local
    legacy:         [DC-LEDGER-11]
```

## Transitions in scope

- `validate_block(LedgerState, ChainDepState, EraSchedule, LedgerView, BlockBytes) -> Result<(LedgerState, ChainDepState, Verdicts), BlockRejection>`
- `validate_tx(LedgerState, TxBytes, Context) -> Result<(LedgerState, Effects), TxRejection>` (phase 1, then phase 2 when scripts are present)
- `apply_certificate(CertState, GovState, Certificate, Context) -> Result<(CertState, GovState), CertRejection>`
- `evaluate_script(Script, PlutusLanguage, Arguments, ScriptContext, CostModel, InitialBudget) -> Result<(Value, Budget), ScriptFailure>`

## TCB color hypothesis

- BLUE: `ade_ledger` (block and transaction validity, certificates, values, parameters, phase-2 effects), `ade_plutus`, `ade_types`, `ade_codec`
- GREEN: `ade_testkit` validity and differential corpora
- RED: none in scope

## Open questions

- S1. DC-LEDGER-01, CN-LEDGER-01 (apply_block is a pure, deterministic function of prior state and block) — merged into DET-001 ReplayByteIdentity; the purity half is carried by STR-001 AuthoritativeCorePurity.
- S2. DC-LEDGER-02, CN-LEDGER-02 (same genesis or bootstrap and same blocks give byte-identical state) — merged into DET-001.
- S3. DC-VAL-01, DC-TXV-01 (a verdict is a function only of named state and the block or transaction bytes; no clock, arrival order, hash iteration, float or ambient state) — merged into STR-001 (a core function's only inputs are its arguments). STR-002 carries arrival order and DET-004 floats and iteration.
- S4. DC-VAL-05, DC-TXV-04 (an invalid block or transaction leaves the input state unchanged) — merged into SAF-001 AtomicTransitions. CN-LEDGER-06 (failure residue is deterministic and consensus-neutral) — merged into SAF-001 together with DET-005 StructuredDeterministicFailures: SAF-001 leaves no authoritative residue, since the prior state is exactly as it was, and DET-005 makes the failure report itself deterministic; anything else a failure leaves, such as a log line, is not authoritative state and has no consensus effect by definition.
- S5. CN-LEDGER-05 (each feature has one semantic processing result; no alternate path disagrees) — merged into EVO-001 SingleSemanticInterpretation.
- S6. CN-LEDGER-07 (conservation) — merged into CON-001 ValueConservation.
- S7. CN-LEDGER-08 (no double spend) — merged into SAF-003 SingleSpendPerHistory.
- S8. DC-PLUTUS-02, CN-PLUTUS-02 (budget exhaustion and script failure have one deterministic failure shape) — merged into DET-005 StructuredDeterministicFailures.
- S9. CN-PLUTUS-04 (no host-environment property influences script results) — merged into STR-001 AuthoritativeCorePurity (no clock, environment, randomness or I/O) together with DET-004 HostIndependentResults (no floats, locale, timezone, pointer width or endianness).
- S10. DC-LEDGER-11 (proposal procedures decoded through one closed decoder, never opaque bytes) — drafted as #23.
- S11. DC-LEDGER-04 (epoch-boundary computations match the reference) — deferred to B04 epoch.
- S12. DC-GOV-01 (governance deposit refunds only for provably unratifiable expired proposals) — deferred to B04 epoch, with the boundary transitions it belongs to.
- S13. DC-LEDGER-12 (every transaction in a forged block is admissible, in canonical order) — deferred to B10 pipeline-production.
- S14. DC-LEDGER-PHASE2-04 (the reduced UTxO checkpoint durably retains the collateral values it destroys) — deferred to B07 durability-recovery.
- S15. DC-LEDGER-05's era-specific witness rules for Byron and earlier eras — excluded beyond Conway: Ade executes only Conway semantics (A2). The Conway part is drafted as #6.
- S17. DC-LEDGER-13's single allowlisted exception (a mainnet epoch-length constant in the epoch-boundary monetary-expansion denominator) — excluded from #10, which holds without exceptions. The site is checked at the re-evidence pass; if it uses a mainnet constant on another network, #10 is recorded as falsified and a later cluster fixes it.
- S18. DC-LEDGER-PARAMS-01's import-provenance clauses (network identity taken from the snapshot manifest and bound onto every authority-bearing field and the commitment; pool reward-account bytes never used as network authority) — deferred to B11 mithril-evidence, which covers snapshot import.
- S19. DC-LEDGER-PARAMS-01's terminal refusal of the per-byte rule on pre-Conway validation paths — excluded: Ade executes only Conway semantics (A2), so #12 states the Conway check itself. No Conway minimum-UTxO check was found in the code (no match for min_utxo, coins_per_utxo or OutputTooSmall in the Conway validation paths), so #12, and #2 with it, are expected to be recorded as falsified at the re-evidence pass. That is a known gap for a later cluster, not a reason to weaken them.
- S16. The legacy entries' environment-blocked oracle obligations (DC-LEDGER-08, DC-LEDGER-09, DC-LEDGER-10) — not definitions; they become limits on the evidence recorded at the migration's re-evidence pass.
```
