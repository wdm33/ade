# Consensus record — docs/planning/mithril-evidence.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-09 — docs/planning/mithril-evidence.define.md @ blob e68305a6f65a57acd58ecfc46d1158129918d723 · received e68305a6f65a57acd58ecfc46d1158129918d723

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | N1; #5–#6; acquisition description | "N1 is needed because A2 does not establish Mithril or ancillary-signer trust, but its conditional guarantee does not establish that bootstrap consumes the bytes actually verified. mithril_fetch.rs treats any existing db/ledger directory as previously verified, even without a digest; bootstrap accepts local files without …" | revised: new #19 SnapshotBytesBoundToVerification binds consumed state and tables to verification under the network's committed keys, covering reuse and post-download substitution; S9 records it as expected falsified; N1 kept to authenticated contents | accepted |
| 2 | #6; S3 | "S3’s point-check diagnosis is correct, but #6 also requires an unsupported independent network check. decode_native_nonutxo_state derives network_id from manifest_network_magic, and assembly compares it with the same derivation. ledgerdb_state.rs explicitly states that the state file has no network-authoritative identity …" | revised: #6 states network identity as the manifest magic bound to the selected profile and genesis; S3 records the tautological network check too | accepted |
| 3 | #4 | "“The check always compares two origins and never a value with itself” overstates independence across all four fields. The anchor’s Mithril certificate hash is copied from imported provenance; comparing it with the report checks internal consistency, not independent certification." | revised: #4 claims independent origins only for seed point, magic and genesis hash; certificate-hash equality is provenance consistency resting on N1 and #19 | accepted |
| 4 | S1 | "PRC-002’s approved text requires the exact reference build, comparison surface, fixtures, replay inputs and reproducible result, but does not require the extraction method or a path for reproducing extraction. The proposed merge loses explicit DC-REF-01 meaning." | revised: DC-REF-01 drafted as #20 ReferenceExtractionReproducible; S1 rewritten | accepted |
| 5 | S8 | "Neither N1 nor STR-003 prohibits every Mithril/STM dependency under BLUE paths. STR-003 permits pure libraries without the listed effects, and N1’s statement that verification happens outside the core does not prohibit importing an unused pure Mithril crate." | revised: the BLUE dependency prohibition is carried on #19; S8 rewritten | accepted |
| 6 | #16–#17; DC-EVIDENCE-04 disposition | "The event vocabulary and continuity reducer omit DC-EVIDENCE-04’s promotion condition: committed two-producer evidence must prove both-peer receipt, the SELECT middle, applied adoption at X and admitted X, followed by continuity within a bounded window and either matching agreement or a validated prefix with the peer ahead. …" | revised: new #21 ForkSwitchConvergencePromotion carries DC-EVIDENCE-04's promotion condition (S10) | accepted |
| 7 | #14 | "DC-EVIDENCE-01 requires the BlockAdmitted event itself to carry the consensus-input fingerprint. #14 requires that fingerprint only on admission-start and bootstrap-complete, and lacks an OBS-005 dependency carrying the per-event binding. It also silently broadens the specified local docker venue." | revised: #14 requires the fingerprint on block-admitted events, depends on OBS-005, and names the canonical local reference-node peer (S12) | accepted |
| 8 | #18 | "RO-SYNC-EVIDENCE-01’s operator-witnessed two-Haskell-node private-Conway-testnet live leg is absent, with no settlement disposing of it. Manifest hashes and comparison results do not establish that leg." | revised: the operator-witnessed two-reference-node live leg is deferred to B12 (S11) | accepted |
| 9 | S6; #10 | "Leaving the both-files-absent branch unresolved leaves DC-EVIEW-09 incompletely settled. #10 currently permits that branch. “Seed-only producers” do not provide an exception to DUR-003’s requirement after every durable selected-chain block." | revised: adopted the recommendation; #10 requires both package files for node mode, the package-free path confined to admission mode and diagnostics (S6) | accepted |
| 10 | #5–#6; DC-MITHRIL-07 disposition | "The legacy live-route contract explicitly rejects incomplete native component sets before decoding. #5 addresses forbidden CLI flags, and #6 addresses coherence disagreements, but neither requires rejection of partial native inputs before route selection or prevents such inputs from selecting the CLI route." | revised: #5 requires all native components together, a partial set terminal before decode or route selection | accepted |
| 11 | #2 | "CN-SEED-01’s “no per-call fallback” and “no silent partial seed” clauses are missing. One deterministic importer can still consistently omit an unsupported entry or substitute a fallback result while satisfying #2." | revised: #2 requires complete import or a typed rejection, no skipped entry, partial seed or per-call fallback; counterexample added | accepted |
| 12 | #2; #11 assumptions | "#2 interprets the reference node’s cardano-cli dump, and #11 claims sufficient state for deriving the next epoch’s authority, but neither cites A2. Their reference-semantic obligations remain even though dependencies cite related invariants." | revised: A2 added to #2 and #11 | accepted |
| 13 | #7 locality | "The statement governs an atomic commit across bootstrap artifacts and recovery visibility, but declares locality local. This is the transactional guarantee distinguished by Doctrine §8." | revised: #7 locality transactional; the provenance append is its commit boundary | accepted |
| 14 | #2; #16 criticality | "#2 defines a canonical commitment fold, and #16 requires exactly one terminal event per win; both are standard despite being surfaces Doctrine §9 identifies as usually critical. No rationale explains the exceptions." | revised: #2 and #16 marked critical | accepted |

## Round 2 — 2026-11-09 — docs/planning/mithril-evidence.define.md @ blob 4215d57b344c9bd9f907389c96d29f959a8c95b4 · received 4215d57b344c9bd9f907389c96d29f959a8c95b4

Items 1–14: accepted.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 15 | #4; #19 | "#19 protects only the state and tables consumed by native bootstrap. It never requires the CLI route’s manifest or its certificate hash to match verified acquisition evidence." | revised: #19 covers both routes; the consumed manifest's certificate identity, network and point must match the recorded acquisition evidence | accepted |
| 16 | #5–#6 | "The native route’s supplied Shelley genesis bytes are not explicitly bound to the committed genesis hash." | revised: #6 requires the parsed genesis file to hash to the profile's committed genesis hash before assembly; S13 records it as expected falsified | accepted |
| 17 | #11; DC-MITHRIL-08 settlement | "#11 preserves construction timing and failure semantics but drops that structural requirement without a settlement" | revised: S14 excludes DC-MITHRIL-08 (b) as a code-organization rule; the property it protects is STR-013 | accepted |

## Round 3 — 2026-11-09 — docs/planning/mithril-evidence.define.md @ blob 38b75a9ffd8269ac887c7165e83af9a76d17bd2e · received 38b75a9ffd8269ac887c7165e83af9a76d17bd2e

Round 2 items 15–17: Codex marked all three `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-09 — after round 3 @ blob 38b75a9ffd8269ac887c7165e83af9a76d17bd2e

```text
Define batch — mithril-evidence (B11) — consensus: round 3 (docs/planning/mithril-evidence.define.consensus.md)

 1. PRC  DifferentialLocalizesFirstDivergence [standard]  Differential harnesses report the first point of divergence, never only 'differs'.
 2. DET  CliSeedImportDeterministic           [critical]  One cardano-cli seed importer; canonical fingerprint; complete import or typed rejection.
 3. STR  BootstrapAnchorSingleMint            [critical]  One anchor mint, every field required; versioned canonical round trip.
 4. SEC  MithrilBindingIndependentOrigins     [critical]  CLI seed route: manifest point/magic/genesis checked against the separately extracted anchor before storage.
 5. SEC  NativeSnapshotSoleSource             [critical]  Native route: seed only from the verified snapshot + genesis; all components required; no CLI fallback.
 6. SEC  NativeSnapshotPointCoherence         [critical]  Native route: certified point = ledger state's own tip; Conway; epoch; genesis file hash checked. (expected FALSIFIED)
 7. DUR  NativeBootstrapCommitsAtomically     [critical]  Native bootstrap commits atomically at the log provenance append; nothing bootable before it.
 8. VAL  LedgerStateDecodeFaithful            [critical]  Ledger-state decode faithful: Conway era by count, real VRF keys, epoch cross-check, round trip.
 9. VAL  UtxoTablesMaterializeFaithfully      [critical]  UTxO tables: full u64 quantities, datum/script bytes verbatim, ascending keys, commitment bound to the point.
10. SEC  CertStatePackageHashBound            [critical]  Cert-state package hash-bound by manifest; node mode needs both files. (expected FALSIFIED if empty-state branch remains)
11. VAL  NativeBootstrapBoundaryReady         [critical]  Native first run builds and seals the reduced checkpoint so the first boundary works.
12. VAL  ConwayDepositParamsFromSnapshot      [critical]  Conway deposit params decoded from the snapshot, never defaulted; old stores fail closed.
13. SEC  AdversarialBlockMutationsRejected    [critical]  Four block mutations (body flip, body hash, KES/sig, VRF) are always rejected.
14. PRC  OperatorPassTranscript               [standard]  Operator-pass transcript: fingerprinted events, ≥1 agreed, 0 diverged, against the local reference peer.
15. PRC  ConvergenceThroughReorgTranscript    [standard]  Reorg-convergence transcript must show a followed rollback, an agreed re-converged tip, no divergence.
16. OBS  ForkChoiceEvidenceClosed             [critical]  Closed observe-only fork-choice evidence; every win has exactly one terminal event.
17. DET  PostSwitchContinuityVerdict          [standard]  Post-switch continuity verdict: pure, from Ade's own validated lineage, never the peer's tip.
18. PRC  SyncEvidenceManifest                 [standard]  Sync-evidence manifest: closed schema, hashes match fixtures, mismatches become regression fixtures.
19. SEC  SnapshotBytesBoundToVerification     [critical]  Bootstrap consumes exactly the Mithril-verified bytes; reuse without evidence refused. (expected FALSIFIED)
20. PRC  ReferenceExtractionReproducible      [standard]  Reference artifacts name source, extraction method and a reproducible extraction path.
21. PRC  ForkSwitchConvergencePromotion       [standard]  Partition convergence promoted only on a committed two-producer switch-and-continue transcript.

New assumption:
 N1 mithril-snapshot-trust — a snapshot whose immutable files the Mithril client verifies against a certificate chain rooted
    in the network's Mithril genesis key, and whose ancillary files (ledger state, UTxO tables) verify under the network's
    ancillary key, holds the reference node's ledger state at the point that state records. Ade does not verify the
    multisignature itself. Relied on by: 4, 5, 6, 19.

Settled in consensus:
 S1. DC-REF-01 — drafted as 20.   S2. N1 states what A2's "certified" trusts.   S3. 6 expected FALSIFIED (tautological checks).
 S4. The i64 multi-asset blocker — retired by VAL-013.   S5. Slice-scope clauses not carried.
 S6. Empty cert-state branch retired for node mode (10).   S7. Clause map for 5, 6, 7.   S8. BLUE Mithril-crate ban — 19.
 S9. 19 expected FALSIFIED (unverified reuse).   S10. DC-EVIDENCE-04 promotion — 21.   S11. Sync live leg → B12.
 S12. 14 names the local reference peer.   S13. 6's genesis-file binding expected FALSIFIED.
 S14. DC-MITHRIL-08 (b) excluded as code organization (STR-013 carries the property).

Legacy dispositions (24): 1←DC-DIFF-01 · 2←CN-SEED-01, DC-SEED-01 · 3←CN-ANCHOR-01, DC-ANCHOR-01 · 4←CN-MITHRIL-01, DC-MITHRIL-01, 02 · 5←DC-MITHRIL-03, 07, RO-MITHRIL-IMPORT-01 · 8←DC-MITHRIL-04 · 9←DC-MITHRIL-05, 06 · 10←DC-EVIEW-09 · 11←DC-MITHRIL-08 · 12←DC-CINPUT-07 · 13←DC-EVIDENCE-02 · 14←DC-EVIDENCE-01 · 15←DC-EVIDENCE-03 · 16←DC-EVIDENCE-04 · 17←DC-EVIDENCE-05 · 18←RO-SYNC-EVIDENCE-01 · 20←DC-REF-01

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok"; "N1 ok|reject|amend: <text>".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all" (one reply, given after B10, B11 and B12 were presented together; it covers all three batches)
