# Consensus record — docs/planning/foundations.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/foundations.define.md @ blob 322384b69f4b9aaf2ca085fa09a5c94ed9eb1bd6 · received 322384b69f4b9aaf2ca085fa09a5c94ed9eb1bd6

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #1 | "The output list narrows T-DET-01 from all authoritative bytes to fingerprints, events and verdicts; differing authoritative state encodings or effects could escape it." | revised: "byte-identical authoritative outputs, including …", now also naming persisted state encodings and declared effects | open |
| 2 | #2 | "“Original bytes or canonical encoder” leaves byte authority selectable … conversely, the final prohibition also forbids legitimate hashing of canonical internal state reconstructed by decoding." | revised: renamed DesignatedByteAuthority; protocol hashes and signatures use wire bytes, Ade's own commitments use the single canonical encoder; counterexample extended | open |
| 3 | #7 | "The merge loses CN-META-02’s coverage of every consensus-relevant failure mode by covering only returned BLUE rejections; internal failures and shell-detected consensus failures are omitted." | revised: renamed StructuredDeterministicFailures; it covers every consensus-relevant failure, rejections and the halt reports of #8 | open |
| 4 | #7 | "“Same input” alone does not imply identical rejections: prior state, protocol version and context can legitimately change the error." | revised: equivalent prior state, canonical input, protocol version and explicit context, in the statement and counterexample; cites N1 | open |
| 5 | #10 | "excludes other explicit governing data, including bootstrap configuration, although the legacy requirements permit explicit runtime protocol data." | revised: quantified over identical canonical inputs and explicit governing data (protocol version, parameters, genesis and bootstrap anchor); operator configuration limited to transport, logging, telemetry | open |
| 6 | #11 | "Recording fragmentation, mux interleaving or timeout observations permits authoritative decisions to depend on them … T-TRANSPORT-01’s independence requirement is lost." | revised: transport artifacts never reach the core; equal ordered whole-message sequences per mini-protocol give identical outcomes; only legitimately needed facts such as the current slot enter as recorded context | open |
| 7 | #12; TCB | "“BLUE crate” misses BLUE modules inside the mixed RED/BLUE ade_network crate. The hypothesis also omits mux/frame.rs" | revised: crate-level and module-level clauses (mixed-crate BLUE modules import no RED modules); counterexample adds mux::frame importing mux::transport; TCB lists every ade_network core path including mux/frame.rs | open |
| 8 | #13 | "Existence of an enforcement entry does not establish substantive mechanical enforcement or execution in CI." | revised: a substantive mechanical check that exists and that the check suite runs; no waivers; renamed EveryInvariantMechanicallyEnforced; S9 discloses its expected falsified status at cutover | open |
| 9 | #14; N2; Sources | "The cited hash surface pins oracle artifacts to 10.6.2 … while N2 selects 11.0.1." | revised: #14 requires the exact reference build and comparison surface and forbids carrying artifacts across builds; N2 names both builds; S8 records it | open |
| 10 | #16 | "The universal transaction equation omits Conway donations and does not distinguish ordinary transaction application from collateral-only application." | revised: separate equations for script-valid and script-invalid application, treasury donation included; extends to all accepted ledger transitions | open |
| 11 | #17 | "The migration drops “or equivalent spend authority” … Its FIN classification also implies finality" | revised: family SAF, renamed SingleSpendPerHistory, scoped to one adopted history; the concrete spend instances in Conway are regular inputs and collateral, both named; reference inputs are not spends | open |
| 12 | #18 | "Unrestricted “explicit context” can carry future decisions … also omits the legacy prohibition on retroactively reinterpreting checkpoints" | revised: context restricted to what the prefix fixes or what is recorded before validation; checkpoints never reinterpreted under another rule version; counterexample imports later-derived data explicitly | open |
| 13 | N1; N2; citations | "N1 does not identify the ordering domain or require replay-complete starting state and context. N2 omits the project’s explicit prohibition on historical-era execution" | revised: N1 is per state machine and replay-complete; N2 adds Conway-only execution; N2 cited by #2, #3, #10, #17, #18 (and #14, #16); N1 by #1, #7, #11, #18 | open |
| 14 | Open questions; Sources | "Phase 0A’s crate extraction rule, Phase 0B’s BLUE dependency restrictions and deterministic GREEN harness requirements, and hash-surface evidence restrictions … name the destination batch" | revised: S5 (excluded), S6 (drafted as #12), S7 (deferred to B11), S8 (drafted into #14 and N2); S4 now deferred to B09 node | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/foundations.define.md @ blob a0b801ada8851e0087827dec34ac945e806b4201 · received a0b801ada8851e0087827dec34ac945e806b4201

Round 1 items 1–14: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 15 | #11 | "Equal per-mini-protocol message sequences alone do not guarantee identical core outcomes: the explicitly permitted current-slot context can differ, and N1 permits different total orders at a shared authoritative state machine." | revised: the comparison now holds the starting state, protocol version, recorded context and authoritative input order equal, and keeps independence from fragmentation, segmentation and transport observations | open |
| 16 | #15; T-KEY-01 | "The migration preserves signing confinement but drops “verification in core.” A RED function could decide signature validity and pass a trusted boolean into BLUE while satisfying #15." | revised: renamed SigningInShellVerificationInCore; acceptance-deciding signature, VRF and KES verification is computed in BLUE, and no RED code passes a verification verdict into a BLUE transition; counterexample added | open |
| 17 | #16 counterexample | "Outputs exceeding consumed inputs is not necessarily a conservation violation … The first counterexample does not exclude these balancing sources." | revised: the counterexample now compares total produced value against total consumed value, each with every listed component | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/foundations.define.md @ blob 7426bb1386ba7f12384665ae01e9dba66a4785a5 · received 7426bb1386ba7f12384665ae01e9dba66a4785a5

Round 2 items 15–17: Codex marked each `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-08 — after round 3 @ blob 7426bb1386ba7f12384665ae01e9dba66a4785a5

```text
Define batch — foundations (B01) — consensus: round 3 (docs/planning/foundations.define.consensus.md)

 1. DET  ReplayByteIdentity                  [critical]  Replaying the same canonical state, ordered inputs, protocol version and recorded context gives byte-identical authoritative outputs.
 2. DET  DesignatedByteAuthority             [critical]  Protocol hashes and signatures use the wire bytes, never a re-encoding; Ade's own commitments use the single canonical encoder.
 3. DET  RoundTripIdentity                   [critical]  Every successfully decoded byte string re-encodes to exactly the original bytes.
 4. STR  AuthoritativeCorePurity             [critical]  BLUE code does no I/O, reads no clock, environment or randomness, and has no async code or tasks.
 5. DET  HostIndependentResults              [critical]  No BLUE result depends on floats, non-canonical iteration order, locale, timezone, pointer width or endianness.
 6. SAF  AtomicTransitions                   [critical]  Every state change returns the complete new state or a structured rejection; a rejection leaves the prior state unchanged.
 7. DET  StructuredDeterministicFailures     [standard]  Every consensus-relevant failure is a closed structured value; equivalent state, input, version and context give byte-identical failures.
 8. SAF  FailFastOnInvariantRisk             [critical]  An internal inconsistency that could corrupt state, replay or chain selection halts the node deterministically.
 9. DET  BuildInvariantSemantics             [critical]  No feature, cfg, profile or optimization level changes an authoritative result or persisted byte.
10. EVO  SingleSemanticInterpretation        [critical]  Identical inputs and governing data have one interpretation; operator config touches only transport, logging and telemetry.
11. STR  ShellCanonicalizesInputs            [critical]  Transport artifacts never reach the core; the slot and other needed facts enter only as recorded context.
12. STR  CoreDependsOnlyInward               [critical]  No BLUE crate or module depends on RED code or on networking, filesystem, async, clock, randomness or serde-style libraries.
13. PRC  EveryInvariantMechanicallyEnforced  [standard]  Every active invariant has a named, existing check that the check suite runs; no waivers.
14. PRC  ReproducibleEquivalenceClaims       [standard]  Every agreement-with-reference claim names its exact reference build, surface, fixtures and inputs, and reruns to the same result.
15. SEC  SigningInShellVerificationInCore    [critical]  Private keys and signing live only in RED; acceptance-deciding verification is computed in BLUE.
16. CON  ValueConservation                   [critical]  Every accepted ledger transition preserves value as Conway defines it, including donations and the script-invalid collateral path.
17. SAF  SingleSpendPerHistory               [critical]  Along one adopted history no output is consumed twice, as an input or as collateral.
18. ORD  NoRetroactiveValidation             [critical]  A verdict depends only on the prior chain prefix and context it fixes; adopted checkpoints are never reinterpreted.
19. SEC  BoundedPreValidationAllocation      [critical]  Before validation, untrusted input can only cause allocation or work up to a fixed bound independent of its own length fields.

New assumptions (approved with the lines that cite them):
 N1  ordered-recorded-inputs: each authoritative state machine gets inputs one at a time in one total order, with starting state, order and context recorded for complete replay — cited by 1, 7, 11, 18
 N2  reference-node-semantics: the Haskell cardano-node defines reference semantics, each comparison names its exact build (live 11.0.1, hash corpus 10.6.2); Ade executes only Conway semantics from a certified non-Origin Conway tip — cited by 2, 3, 10, 14, 16, 17, 18

Settled in consensus:
 S1. T-ENC-02 — deferred to B02 codec-wire-crypto: as written it contradicts Cardano's valid non-canonical CBOR; its closed-grammar intent belongs with the decoder lines.
 S2. T-CORE-04 — excluded: "where practical" is not falsifiable, and it is Constitution rule 4.
 S3. T-CORE-02 — drafted as 4 and 5: the clock and randomness clauses go to 4, floats and collections to 5.
 S4. Whether the halt of 8 may cost a producer its slot — deferred to B09 node liveness.
 S5. Phase 0A ade_core extraction rule — excluded: the split happened; crate layout isn't authoritative state, and boundaries are the human's (§20).
 S6. Phase 0B no GREEN-only dependencies in BLUE — drafted as 12.
 S7. Phase 0B deterministic GREEN harness requirements — deferred to B11 mithril-evidence.
 S8. Reference artifacts valid only for their extraction build — drafted into 14 and N2.
 S9. 13 will be recorded as falsified at cutover until every migrated invariant has a check — its honest status, not a reason to weaken it.

Legacy dispositions (31): 1←T-DET-01 · 2←T-ENC-01 · 3←T-ENC-03 · 4←T-CORE-01, T-CORE-02, DC-CORE-01 · 5←T-CORE-02, T-COLL-01, T-PLATFORM-01 · 6←T-CORE-03 · 7←T-ERR-01, CN-META-02 · 8←T-ERR-02 · 9←T-BUILD-01, CN-BUILD-01 · 10←T-BUILD-02, CN-BUILD-02, CN-BUILD-03, CN-BUILD-04 · 11←T-BOUND-01, T-TRANSPORT-01 · 12←T-BOUND-02 · 13←T-CI-01, CN-META-01 · 14←CN-META-03 · 15←T-KEY-01 · 16←T-CONSERV-01 · 17←T-NOSPEND-01 · 18←T-CAUSAL-01 · 19←T-RESOURCE-01 · T-ENC-02 deferred (S1) · T-CORE-04 excluded (S2)

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok".
Answer each new assumption the same way: "N1 ok".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"
