# Consensus record — docs/planning/ledger.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/ledger.define.md @ blob 7eda6840a9fc327eb02cbab14de029dd54151e9b · received 7eda6840a9fc327eb02cbab14de029dd54151e9b

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #3 | "Requiring phase 2 to accept every transaction contradicts #15 … “When it carries scripts” also conflates native scripts with Plutus." | revised: block transactions need phase 1 (native scripts included) and Plutus evaluation agreeing with the block's declared phase-2 status; standalone admission needs phase 1 and successful Plutus evaluation; the collateral path is explicitly allowed | open |
| 2 | #1 counterexample | "A script exceeding its budget does not by itself make its containing block invalid" | revised: the transaction is marked phase-2-valid, so the reference rejects the block for the mismatch | open |
| 3 | #6 | "The required-signer set includes all payment credentials, including script credentials, although a key witness cannot authorize a script credential." | revised: required key authorities are key-hash credentials and bootstrap addresses only; script-hash credentials are authorized by script witnesses (#19), now a dependency | open |
| 4 | #8 | "“Exactly one” transition … excludes composite certificates whose effects belong to both owners." | revised: one closed disposition listing all effects, each applied exactly once by its owner; composite certificates have each half applied once | open |
| 5 | #9 | "The legacy credential invariant also requires fingerprints to serialize the discriminant." | revised: every authoritative encoding and fingerprint of a credential includes its key-or-script tag | open |
| 6 | #10 | "The draft removes the legacy’s explicit, justified mainnet monetary-expansion allowlist exception without acknowledging the departure." | revised: S17 records that #10 holds without exceptions, and that the existing site is checked at re-evidence and recorded as falsified if it misbehaves off mainnet | open |
| 7 | #12 | "The legacy entry’s manifest-derived network identity, binding onto all authority-bearing fields and the commitment, and prohibition on using pool reward-account nibbles as network authority disappear." | revised: S18 defers these import-provenance clauses to B11 mithril-evidence | open |
| 8 | #12 | "The legacy requires a distinct minimum-UTxO rule representation and a terminal refusal until per-byte validation exists … includes historical-era execution despite A2’s Conway-only scope." | revised: #12 keeps the distinct representation and states only the Conway check; S19 excludes the pre-Conway refusal and discloses that no Conway minimum-UTxO check was found in the code, so #12 and #2 are expected to be recorded as falsified at re-evidence | open |
| 9 | #15–#16 | "The drafts preserve the resulting arithmetic but drop these authority boundaries." | revised: #15 adds that consumers receive only derived effects, never the phase-2 status; #16 adds the resolver boundary (the accumulator neither owns nor rebuilds a UTxO) | open |
| 10 | S9 | "DET-004 excludes particular host dependencies but does not prohibit clock, environment, randomness or I/O inputs generally." | revised: S9 merges CN-PLUTUS-04 into both STR-001 and DET-004 | open |
| 11 | S10 | "The legacy explicitly forbids that opaque authoritative representation." | revised: DC-LEDGER-11 drafted as new line #23 TypedProposalProcedures | open |
| 12 | #4 | "This authoritative rejection-order invariant is marked standard without explaining why" | revised: #4 is critical | open |
| 13 | #22 | "expands the legacy’s supported-production-version scope to arbitrary obsolete or unsupported releases … The comparison also omits equal prior state and explicit context." | revised: supported releases authorized to coexist, compared on the same prior state, inputs, protocol version and context | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/ledger.define.md @ blob 4aaa917fa6675c5f0aefca1a51a4f7eef3edc3cb · received 4aaa917fa6675c5f0aefca1a51a4f7eef3edc3cb

Round 1 items 1–13: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 14 | #12 | "The minimum-UTxO formula omits the 160-byte overhead: the reference rule is coefficient × (160 + serialized output size)." | revised: the coefficient times (160 plus the byte length of the output's CBOR serialization as it appears in the transaction) | open |
| 15 | #13 and evaluate_script | "Equal script, arguments, context and cost model do not guarantee equal results when initial execution budgets differ." | revised: #13 holds the Plutus language version and initial execution budget fixed; the transition takes both as explicit inputs | open |
| 16 | #19 primary family | "Script-witness sufficiency enforces authorization against unauthorized spending … VAL as primary misses Doctrine §6’s security-first rule." | revised: SEC primary, VAL secondary, still critical | open |
| 17 | S4 | "SAF-001 … does not require deterministic failure residue." | revised: CN-LEDGER-06 merges into SAF-001 and DET-005, with the residue accounted for: no authoritative residue under SAF-001, a deterministic failure report under DET-005, and anything else non-authoritative | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/ledger.define.md @ blob 4c54d2274a952801d75debf5a0155f6ac2399d72 · received 4c54d2274a952801d75debf5a0155f6ac2399d72

Round 2 items 14–17: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 18 | #3 assumptions | "CompleteValidation relies on A2’s reference-node semantics for the phase-2 status agreement and collateral-path rules, but declares no assumptions." | revised: #3 cites A2 | open |

Outcome: continue

## Round 4 — 2026-11-08 — docs/planning/ledger.define.md @ blob 210bca472ad6b6cd7dd2b73a1adf55e3552bfeb9 · received 210bca472ad6b6cd7dd2b73a1adf55e3552bfeb9

Round 3 item 18: Codex marked it `accepted` (and restated items 1–17 as accepted). Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-08 — after round 4 @ blob 210bca472ad6b6cd7dd2b73a1adf55e3552bfeb9

```text
Define batch — ledger (B03) — consensus: round 4 (docs/planning/ledger.define.consensus.md)

 1. VAL  BlockValidityMatchesReference        [critical]  Every Conway block's verdict (and reason class) equals the reference node's for the same state and version.
 2. VAL  TxValidityMatchesReference           [critical]  Every Conway transaction's verdict (and reason class) equals the reference node's for the same state and version.
 3. VAL  CompleteValidation                   [critical]  Valid needs both header and body authorities; block txs need phase 1 and Plutus results matching the declared phase-2 status; nothing skipped.
 4. ORD  HeaderBeforeBody                     [critical]  The header is validated first, the body never for an invalid header, and the first failing authority gives the reason.
 5. VAL  ChecksNeverSkip                      [critical]  Every size, shape and crypto-input check rejects on bad input and never skips; no unwired checks, no self-comparing guards.
 6. SEC  WitnessesBindTransaction             [critical]  Every required key authority has a verifying witness over the body hash, every supplied witness verifies, and scripts authorize script credentials.
 7. CON  DepositAccountingClosed              [critical]  Each certificate's deposit effect is one of new/refund/none/reject, amounts only from the certificate or current parameters.
 8. VAL  CertificateTransitionsClosed         [critical]  Each Conway certificate decodes fully and has all its effects applied exactly once by their owners, or is rejected.
 9. STR  CredentialDiscriminantPreserved      [critical]  Credentials are key-hash or script-hash everywhere, including state keys, encodings and fingerprints.
10. VAL  NetworkConstantsFromSchedule         [critical]  Network constants enter only through the network's era schedule and genesis; mainnet constants only through a named mainnet schedule.
11. VAL  OutputQuantityFullDomain             [critical]  Output asset quantities span the full u64 range, can't be negative, and arithmetic is checked (no wrap, no saturation).
12. VAL  ProtocolParametersEraFaithful        [critical]  Parameters keep their era meaning; Conway min-UTxO = coinsPerUTxOByte × (160 + serialized output size).
13. DET  PlutusEvaluationDeterministic        [critical]  Same script, language, arguments, context, cost model and starting budget → same result and budget use, everywhere.
14. VAL  ScriptContextCanonical               [critical]  The script context comes only from the transaction, resolved ledger state, system start and era history.
15. VAL  SinglePhaseTwoEffect                 [critical]  One place derives each transaction's effect; phase-2-invalid txs only swap collateral for collateral return and the fee pot.
16. CON  CollateralFromResolvedInputs         [critical]  Consumed collateral = resolved collateral inputs − collateral return; unresolved input is a typed refusal; never total-collateral.
17. VAL  NativeScriptThresholdSigned          [critical]  Native-script thresholds are signed 64-bit; ≤ 0 is satisfied; positive n needs n passing sub-scripts.
18. SEC  BoundedStackDepth                    [critical]  Decoding, skipping or evaluating wire-supplied scripts and CBOR never uses stack depth that grows with nesting; no artificial depth limit.
19. SEC  ScriptWitnessClosureSingleAuthority  [critical]  One authority decides script-witness sufficiency for block and standalone paths; reduced mode is never "valid", only reject or undetermined.
20. VAL  IntervalNetworkAndBootstrapChecks    [critical]  Txs must be inside their validity interval at the block slot, carry the node's network id, and keep bootstrap attributes ≤ 64 bytes.
21. VAL  BlockBodySizeMatchesHeader           [critical]  The header's body-size field equals the length of the four body segments as received.
22. EVO  ReleasesAgreeOnVerdicts              [critical]  Supported Ade releases authorized to coexist give the same verdicts for the same state, inputs, version and context.
23. STR  TypedProposalProcedures              [standard]  Proposal procedures are held as typed governance actions, never opaque bytes, and re-encode to the original bytes.

Settled in consensus:
 S1–S3. Purity and replay entries (DC-LEDGER-01, CN-LEDGER-01, DC-LEDGER-02, CN-LEDGER-02, DC-VAL-01, DC-TXV-01) — merged into DET-001, STR-001 (with STR-002, DET-004).
 S4. DC-VAL-05, DC-TXV-04 → merged into SAF-001; CN-LEDGER-06 (failure residue) → SAF-001 + DET-005.
 S5. CN-LEDGER-05 → merged into EVO-001.   S6. CN-LEDGER-07 → CON-001.   S7. CN-LEDGER-08 → SAF-003.
 S8. DC-PLUTUS-02, CN-PLUTUS-02 → merged into DET-005.   S9. CN-PLUTUS-04 → STR-001 + DET-004.
 S10. DC-LEDGER-11 → drafted as 23.
 S11–S12. DC-LEDGER-04 (epoch computations) and DC-GOV-01 (governance deposit refunds) → deferred to B04 epoch.
 S13. DC-LEDGER-12 (forged-block admissibility) → deferred to B10.   S14. DC-LEDGER-PHASE2-04 (collateral retention) → deferred to B07.
 S15. Pre-Conway witness rules in DC-LEDGER-05 → excluded (Conway only, A2).
 S16. Environment-blocked oracle obligations → become evidence limits at re-evidence.
 S17. DC-LEDGER-13's mainnet allowlist exception → excluded; 10 holds with no exception, the site is checked at re-evidence.
 S18. DC-LEDGER-PARAMS-01 import provenance (network identity from the manifest) → deferred to B11.
 S19. Pre-Conway min-UTxO refusal → excluded. No Conway min-UTxO check was found in the code, so 12 (and 2) are expected
      to be recorded FALSIFIED at re-evidence — a known gap for a later cluster.

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"
