# Define — mithril-evidence

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved, and every `#N` reference in an approved line's text is replaced by the ID line N receives. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B11 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers where the node's starting state comes from and how compatibility is evidenced: the cardano-cli seed importer and the bootstrap anchor, the Mithril binding on the cardano-cli seed route, the native Mithril route (decoding the reference node's ledger state and UTxO tables from a Mithril snapshot, assembling the seed, committing it atomically), the certificate-state package, the Conway deposit parameters, and the evidence the project requires before claiming agreement (differential localization, adversarial mutations, operator-pass, convergence and sync transcripts). Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 24 legacy IDs appears on a line or in a settlement.

Ade has two first-run routes in node mode. The native route reads a Mithril cardano-database snapshot directly (its manifest, the reference node's ledger `state` file and its UTxO `tables` file) with the Shelley genesis. The cardano-cli seed route imports a `query utxo` JSON dump and a consensus-input bundle and binds them to a Mithril manifest. Neither is a fallback for the other. The manifest is written by `ade mithril snapshot fetch`, which runs the Mithril client to download and verify the snapshot and takes the certified point from the tip the snapshot's ledger state records.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line and settlement), docs/ade-invariant-registry.toml (the defined invariants and assumptions that lines and settlements cite), crates/ade_ledger/src/bootstrap_anchor/, crates/ade_ledger/src/ledgerdb_state.rs, crates/ade_runtime/src/mithril_native_assembly.rs, crates/ade_node/src/native_firstrun.rs, crates/ade_node/src/mithril_fetch.rs, crates/ade_runtime/src/seed_import/importer.rs

## Drafts

```text
#1  PRC  DifferentialLocalizesFirstDivergence  [standard]
    statement:      Every differential comparison between Ade and the reference node reports the first point at which the two disagree (the first block, transaction, event or field whose outcome differs, with its position), never only that the runs differ.
    counterexample: The ledger differential compares only the final fingerprints of two 10,000-block runs and reports "mismatch", leaving the divergent block unknown.
    secondary:      [OBS]
    scope:          differential harnesses
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       local
    legacy:         [DC-DIFF-01]

#2  DET  CliSeedImportDeterministic  [critical]
    statement:      One importer converts a cardano-cli whole-UTxO JSON dump into Ade's canonical UTxO state and its fingerprint, which is blake2b-256 over the canonical CBOR map of the UTxO in ascending transaction-input order; re-importing the same JSON yields byte-identical state and fingerprint. An import either takes every entry of the dump or fails with a typed error: no entry is skipped, no partial seed is returned, and no per-call fallback exists. No second importer exists, and the importer is never reachable from the native route (#5).
    counterexample: A second JSON reader used by admission tests orders the UTxO by hash-map iteration, so two imports of one dump yield different fingerprints; or the importer silently omits outputs it cannot parse and returns the rest.
    secondary:      [STR]
    scope:          cardano-cli seed import (cardano-cli seed route, admission mode)
    assumptions:    [A2]
    depends_on:     [DET-004, DET-007]
    locality:       local
    legacy:         [CN-SEED-01, DC-SEED-01]

#3  STR  BootstrapAnchorSingleMint  [critical]
    statement:      A bootstrap anchor is produced by one mint function, with every field required and none defaulted: the network magic, the genesis hash, the seed point's slot and block hash, the seed artifact hash, the imported UTxO fingerprint, the initial ledger fingerprint and the seed provenance. Its canonical encoding carries a schema version and round-trips byte-identically, and an unknown version is rejected.
    counterexample: A test helper builds an anchor with a zero initial ledger fingerprint, and recovery accepts a log bound to it.
    secondary:      [DET]
    scope:          bootstrap anchor
    assumptions:    []
    depends_on:     [DUR-008, DET-003]
    locality:       local
    legacy:         [CN-ANCHOR-01, DC-ANCHOR-01]

#4  SEC  MithrilBindingIndependentOrigins  [critical]
    statement:      On the cardano-cli seed route, a seed bootstraps only after a pure predicate cross-checks the Mithril manifest's network magic, genesis hash, certified point and certificate hash against the bootstrap anchor minted from the operator's separately extracted seed, before any authoritative store is initialized; each disagreement is a distinct closed error. The seed point, network magic and genesis hash on the anchor side come from the operator's separate extraction and the selected network's genesis, never from the manifest, so those three checks compare two origins; the certificate-hash check is provenance consistency between the anchor's recorded Mithril provenance and the manifest, its authenticity resting on N1 and #19.
    counterexample: The composition copies the manifest's certified point into the anchor before minting, so a snapshot certified at a different point than the seed dump binds successfully.
    secondary:      []
    scope:          cardano-cli seed route
    assumptions:    [N1]
    depends_on:     [SEC-015, #3, #19]
    locality:       local
    legacy:         [CN-MITHRIL-01, DC-MITHRIL-01, DC-MITHRIL-02]

#5  SEC  NativeSnapshotSoleSource  [critical]
    statement:      The native route assembles the complete seed (ledger state, chain-dependent state and seed-epoch consensus inputs) only from the verified Mithril snapshot (its manifest, ledger state and UTxO tables) and the Shelley genesis constants, each seed field having one declared source among them; it reaches no cardano-cli importer, consensus-input bundle or operator-supplied seed, and has no fallback. The native components (manifest, ledger state, UTxO tables and Shelley genesis) are required together: a partial set is a typed terminal error before any decode and before either route is selected, never a reason to take the cardano-cli route. A cardano-cli seed flag supplied alongside the native inputs is a typed terminal error before any decode, and the two routes are never fallbacks for each other.
    counterexample: When the snapshot's tables file is missing, the native route quietly seeds the UTxO from a --json-seed file the operator also passed.
    secondary:      [STR]
    scope:          native Mithril first run
    assumptions:    [N1, A2]
    depends_on:     [STR-013, SEC-014]
    locality:       local
    legacy:         [DC-MITHRIL-03, DC-MITHRIL-07, RO-MITHRIL-IMPORT-01]

#6  SEC  NativeSnapshotPointCoherence  [critical]
    statement:      Before anything is assembled or persisted, the native route checks that the snapshot is one Conway chain point: the certified point equals the tip the snapshot's own ledger state records (slot and header hash), the ledger state's current era is Conway, and its epoch equals the epoch the era schedule assigns to the certified slot. The ledger state carries no network identity of its own, so the network is the manifest's magic, which must match the selected network's committed profile and genesis hash; its authenticity rests on #19. The Shelley genesis file whose constants the native route uses must hash to the selected profile's committed genesis hash before anything is assembled. The era schedule is built from the genesis epoch length and the network's committed Shelley start boundary, an unknown network being terminal, and the active-slot coefficient is parsed to an exact rational with no floating point. Any disagreement is a typed terminal error with nothing persisted.
    counterexample: The manifest file is edited to name a different slot; the node decodes the ledger state and anchors it at the manifest's slot without comparing it with the tip the state records.
    secondary:      [VAL]
    scope:          native Mithril first run
    assumptions:    [N1, A2]
    depends_on:     [#5, #19, DUR-005]
    locality:       local
    legacy:         []

#7  DUR  NativeBootstrapCommitsAtomically  [critical]
    statement:      The native route persists the bootstrap through the one bootstrap composition (the bootstrap authority, the seed-epoch consensus record, the recovered anchor point and the write-ahead-log provenance entry), and appending that provenance entry is the only point at which the bootstrap becomes discoverable: a failure at any earlier step, including building the reduced checkpoint, leaves nothing a warm start would boot from, and after the append the anchor point is recoverable. No second storage-initialization path exists.
    counterexample: The import writes the ledger snapshot and the seed record, crashes before the log append, and the next start boots from the half-written store.
    secondary:      [REC]
    scope:          native Mithril first run
    assumptions:    [A6]
    depends_on:     [SEC-015, DUR-004, STR-013]
    locality:       transactional
    legacy:         []

#8  VAL  LedgerStateDecodeFaithful  [critical]
    statement:      The reference node's ledger-state file is decoded into Ade's canonical certificate state, pool distribution, Praos nonces and protocol parameters by one deterministic decoder: the same bytes and certified epoch give byte-identical canonical output; the hard-fork telescope is navigated by counting past eras to the one current era, which must be Conway; every active pool's VRF key hash is decoded from its registration parameters, a zero VRF key hash or a disagreement with the pool distribution's VRF key hash being terminal; the decoded epoch is checked against the certified point's epoch; the decoded certificate state survives a canonical round trip; and malformed input halts with a typed error after bounded reads, never a partial decode.
    counterexample: The decoder takes the last element of the telescope as the current era, or fills every pool's VRF key hash with zeros, and leadership is later checked against keys no pool holds.
    secondary:      [DET]
    scope:          ledger-state decode (native route)
    assumptions:    [A2]
    depends_on:     [VAL-001, SEC-002]
    locality:       local
    legacy:         [DC-MITHRIL-04]

#9  VAL  UtxoTablesMaterializeFaithfully  [critical]
    statement:      The snapshot's UTxO tables are decoded and materialized into Ade's UTxO state without loss: every asset quantity keeps its full unsigned 64-bit value; inline datums and scripts keep their original bytes, wrapped by the shared tag-24 authority and never re-encoded; every nesting boundary consumes exactly its bytes, with explicit endianness; an unknown output tag, address form, value tag or script language is a typed terminal error; keys must arrive in strictly ascending transaction-input order; and the era is taken from the same snapshot's ledger state. The materialized UTxO's fingerprint is bound, with the manifest's certified point, the ledger-state commitment and the tables commitment, into one record whose check is terminal on any mismatch, and the UTxO state round-trips through persistence with an identical fingerprint, quantities above the signed 64-bit maximum included.
    counterexample: An output holding 2^63 units of a token is materialized through a signed cast and stored as a negative quantity; or an inline datum is re-encoded, changing its hash.
    secondary:      [DET]
    scope:          UTxO tables decode and materialization (native route)
    assumptions:    [A2]
    depends_on:     [VAL-013, VAL-002, VAL-001]
    locality:       local
    legacy:         [DC-MITHRIL-05, DC-MITHRIL-06]

#10 SEC  CertStatePackageHashBound  [critical]
    statement:      The seed-epoch consensus record and the bootstrap certificate state stay separately typed and are bound only through a canonical bootstrap manifest naming the network, era, source point, seed hash, certificate-state hash and source commitment. The certificate state is the complete state written and read by the canonical certificate-state codec. Node-mode bootstrap on the cardano-cli seed route requires both package files; import fails closed before anything is durable when either is missing, on a malformed manifest, a network or era mismatch, seed or certificate-state bytes that do not hash to the manifest's values, or a certificate state that does not decode; the imported state populates both the captured snapshot and the running ledger. A package-free import, yielding an empty certificate state, exists only in admission mode and diagnostics and never becomes node-mode authority.
    counterexample: The importer accepts a certificate-state file without its manifest and loads delegations from a different epoch's state.
    secondary:      [VAL]
    scope:          cardano-cli seed route bootstrap package, admission mode
    assumptions:    []
    depends_on:     [STR-010, SEC-015, DUR-003]
    locality:       local
    legacy:         [DC-EVIEW-09]

#11 VAL  NativeBootstrapBoundaryReady  [critical]
    statement:      When the decoded certificate state carries delegations, the native first run builds the reduced UTxO checkpoint from the materialized UTxO and seals it at the certified slot before the UTxO is consumed and before the bootstrap commits, so the node can derive its next epoch's authority at the first boundary it crosses; when it carries none, no checkpoint is built and the bootstrap output is byte-identical to a build without this step; a checkpoint failure is terminal before the commit.
    counterexample: A Mithril-started node keeps no reduced checkpoint, reaches the next epoch boundary and halts because no stake distribution can be derived.
    secondary:      [LIV]
    scope:          native Mithril first run
    assumptions:    [A2]
    depends_on:     [DUR-003, DET-013, #7]
    locality:       local
    legacy:         [DC-MITHRIL-08]

#12 VAL  ConwayDepositParamsFromSnapshot  [critical]
    statement:      The Conway deposit parameters (DRep deposit, governance-action deposit and DRep activity) are decoded from the certified snapshot's current protocol parameters by the one protocol-parameter decoder and carried as a required field into the assembled ledger state and the epoch accumulator; they are never defaulted, guessed or read by a second parser; they are covered by the ledger fingerprint and the ledger-state commitment; missing or malformed parameters are terminal; and an accumulator encoded without them, or in the format from before them, fails closed at decode, its remedy being re-bootstrap.
    counterexample: The native assembly sets the DRep activity to a hard-coded 20 epochs, so DRep expiry and ratification denominators are mis-timed after the first governance-active boundary.
    secondary:      []
    scope:          native Mithril first run, epoch accumulator
    assumptions:    [A2]
    depends_on:     [VAL-014, CON-002, EVO-005]
    locality:       local
    legacy:         [DC-CINPUT-07]

#13 SEC  AdversarialBlockMutationsRejected  [critical]
    statement:      Each of four block mutations made to a block the reference node accepts (a body byte flip that keeps the envelope's shape, a header body-hash mismatch, a KES or signature corruption, and a VRF proof or output tamper) is rejected: it is never admitted and never yields an agreed or input-not-found verdict, but a diverged verdict with admission halting, or an undecodable-bytes halt when the corruption breaks decoding.
    counterexample: A flipped byte inside a transaction body leaves the header's body hash unchecked, and the corrupted block is admitted and reported agreed.
    secondary:      [VAL]
    scope:          block admission (admission mode evidence)
    assumptions:    [A2]
    depends_on:     [VAL-032, SEC-003, VAL-009]
    locality:       local
    legacy:         [DC-EVIDENCE-02]

#14 PRC  OperatorPassTranscript  [standard]
    statement:      An operator pass against the reference node peer counts as passing only if its committed transcript against the project's canonical local reference-node peer contains an admission-start event, a bootstrap-complete event and at least one block-admitted event, each carrying the consensus-input fingerprint, at least one agreed verdict, no diverged verdict, and no admitted block whose hash differs from the block the peer announced at the same slot; lagging verdicts do not count either way.
    counterexample: A pass with zero agreed verdicts and 300 lagging verdicts is recorded as green.
    secondary:      []
    scope:          operator-pass evidence
    assumptions:    [A2]
    depends_on:     [PRC-002, OBS-005]
    locality:       distributed
    legacy:         [DC-EVIDENCE-01]

#15 PRC  ConvergenceThroughReorgTranscript  [standard]
    statement:      A participant convergence pass counts as passing only if its one transcript, whose hash the committed manifest binds, shows a strict slot regression in the observed peer-block sequence (a peer roll backward was followed), at least one agreed verdict at the re-converged tip and no diverged verdict; a run in which the tip never regresses does not count.
    counterexample: A run that only ever follows one peer's tip forward is committed as the reorg-convergence evidence.
    secondary:      []
    scope:          participant convergence evidence
    assumptions:    [A2]
    depends_on:     [PRC-002, OBS-005]
    locality:       distributed
    legacy:         [DC-EVIDENCE-03]

#16 OBS  ForkChoiceEvidenceClosed  [critical]
    statement:      The live fork-choice path emits a closed, observe-only sequence of evidence events (needs fork choice, common ancestor found, candidate built, selected, branch fetch started and completed, branch prevalidated, then exactly one of switch applied, switch failed or switch superseded), equal to an emit-only allow-list, with bounded typed fields, closed failure codes and a fork-switch id derived deterministically from the winning peer, fork anchor and winning tip; every win is followed by exactly one terminal event for its id; and no evidence event or type is read by chain selection, the ancestor walk, the switch application or the forge fence, so a write failure cannot change their behaviour.
    counterexample: A fork-choice win is logged with no terminal event because the switch was dropped silently, or the selector reads the evidence log to decide whether a switch already happened.
    secondary:      []
    scope:          fork-choice evidence
    assumptions:    []
    depends_on:     [OBS-005, STR-016]
    locality:       local
    legacy:         [DC-EVIDENCE-04]

#17 DET  PostSwitchContinuityVerdict  [standard]
    statement:      After a fork-choice adoption at tip X, one pure reducer classifies Ade's own admitted lineage into a closed verdict (continues selected branch, diverged, broken lineage, dangling fork-choice win, insufficient evidence); it returns continues-selected-branch only if every block admitted after X chains from X through its validated previous-hash field, no diverged verdict follows X, and every win has a terminal event. Its inputs are Ade's admitted blocks and the applied switch point, never the peer's tip; the same inputs give a byte-identical verdict; the same implementation serves the replay test and the live gate; and nothing on the node's selection, admission or forge path reads it.
    counterexample: The gate accepts a run that switched to X, later rolled back below X onto a different branch, and then reported agreement with the peer.
    secondary:      [OBS]
    scope:          post-switch evidence gate
    assumptions:    [A1]
    depends_on:     [LIV-006, #16]
    locality:       local
    legacy:         [DC-EVIDENCE-05]

#18 PRC  SyncEvidenceManifest  [standard]
    statement:      A committed snapshot-to-tip sync-evidence manifest carries a closed schema (the reference versions, the chain point, the fixture references with their SHA-256 hashes and the comparison result), each named hash matches the committed fixture, and each mismatch found against the reference node becomes a named regression fixture.
    counterexample: A manifest claims agreement at a chain point while its fixture hash names a file that was later replaced.
    secondary:      []
    scope:          sync evidence
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       local
    legacy:         [RO-SYNC-EVIDENCE-01]

#19 SEC  SnapshotBytesBoundToVerification  [critical]
    statement:      What a Mithril bootstrap consumes is exactly what the Mithril client verified under the selected network's committed Mithril keys: the acquisition records verification evidence (the certificate identity, the network, the certified point and the hashes of the verified files), a directory reused without that evidence is re-verified or refused, and on either route bootstrap refuses a manifest whose certificate identity, network or point differs from that evidence, and on the native route ledger-state or UTxO-tables files whose hashes differ from the recorded ones. No BLUE crate depends on a Mithril or multisignature crate; verification runs only in the shell.
    counterexample: The fetch command finds an existing ledger directory, labels it "previously verified" without any digest, and the node bootstraps from tables an operator replaced after download.
    secondary:      [DUR]
    scope:          Mithril acquisition, both bootstrap routes
    assumptions:    [N1]
    depends_on:     [SEC-015, STR-003]
    locality:       local
    legacy:         []

#20 PRC  ReferenceExtractionReproducible  [standard]
    statement:      Every reference artifact used to claim agreement (a fixture, a corpus entry, a captured transcript) names its reference source, the method by which it was extracted from the reference node, and a path for reproducing that extraction, so the artifact can be regenerated from the reference independently of Ade.
    counterexample: A ledger-state hash corpus is committed with the reference node version named but no record of the query or dump command that produced it.
    secondary:      []
    scope:          reference fixtures and corpora
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       local
    legacy:         [DC-REF-01]

#21 PRC  ForkSwitchConvergencePromotion  [standard]
    statement:      The claim of LIV-003 is promoted only on a committed two-producer transcript showing both peers' blocks received, the full fork-choice sequence of #16, the switch applied at X for a fork-choice win and X admitted, followed within a bounded window by #17's continues-selected-branch verdict ending either in an agreed verdict with equal hashes at X or a descendant, or in a validated prefix of the peer's chain with the peer observed ahead, and no diverged verdict; never on diagnostics or a momentary equal tip.
    counterexample: LIV-003 is marked supported from a transcript in which Ade followed a single peer forward and once had the same tip as the other producer.
    secondary:      []
    scope:          partition-convergence evidence
    assumptions:    [A2, A5]
    depends_on:     [LIV-003, #16, #17]
    locality:       distributed
    legacy:         []
```

## New assumptions

N1 mithril-snapshot-trust: a Mithril cardano-database snapshot whose immutable files the Mithril client verifies against a certificate chain rooted in the network's published Mithril genesis verification key, and whose ancillary files (the ledger state and UTxO tables) verify under the network's published ancillary verification key, holds the reference node's ledger state at the chain point that state records. Ade does not verify the Mithril multisignature itself: the Mithril client, run outside Ade's core, does.

## Transitions in scope

- `import_cardano_cli_json_utxo(Json) -> Result<(UTxOState, UtxoFingerprint), JsonSeedError>`
- `mint_anchor(Inputs) -> BootstrapAnchor`
- `verify_mithril_binding(ManifestReport, BootstrapAnchor) -> Result<(), MithrilImportError>`
- `decode_native_nonutxo_state(StateBytes, Point, Epoch, Magic) -> Result<(NativeSnapshotNonUtxoState, Commitment), NativeNonUtxoError>`
- `materialize_tables_to_utxo(TablesBytes, Era) -> Result<UTxOState, MaterializeError>`
- `assemble_native_mithril_seed(...) -> Result<Seed, MithrilNativeAssemblyError>`
- `derive_post_switch_continuity(Events) -> PostSwitchContinuity`

## TCB color hypothesis

- BLUE: `ade_ledger::bootstrap_anchor`, `ade_ledger::ledgerdb_state` (decoders), the tag-24 authority in `ade_codec`
- GREEN: post-switch continuity reducer, fork-choice evidence vocabulary
- RED: `ade_node::mithril_fetch` (runs the Mithril client), `ade_node::native_firstrun`, `ade_runtime::mithril_native_assembly`, `ade_runtime::seed_import`, evidence gates under `ci/`

## Open questions

- S1. DC-REF-01 — drafted as #20: PRC-002 requires the reference build, surface, fixtures and replay inputs but not the extraction method or a path for reproducing extraction.
- S2. N1 is a new trust assumption (Constitution rule 17). A2 already says Ade enters at a certified Conway tip; N1 states what "certified" trusts. The ledger state and UTxO tables are covered by the ancillary signature, not by the Mithril certificate chain, and the certified point Ade uses is the tip the ledger state records.
- S3. #6 is new: the legacy record's point-coherence check (DC-MITHRIL-03 (b)) compares the manifest's point with a value derived from that same manifest point, since the decoder is handed the manifest's point, and its network check compares the manifest magic with a network id derived from that same magic. The manifest's point is derived from the ledger state's tip only when `ade mithril snapshot fetch` writes the manifest, and nothing checks it again at bootstrap. #6 requires the check against the tip the ledger state records at bootstrap and states network identity as the manifest's magic bound to the selected profile. #6 is expected to be recorded as falsified at the re-evidence pass.
- S4. DC-MITHRIL-05's "i64 multi-asset model cannot validate quantities above i64::MAX" release blocker — retired: VAL-013 OutputQuantityFullDomain defines the full unsigned 64-bit output domain.
- S5. DC-MITHRIL-04's "non-emitting (stage 1)" scope clause and DC-MITHRIL-07's "S1a/S1b/S1c are frozen" are slice-scope statements, not properties of the system; they are not carried.
- S6. DC-EVIEW-09's branch "neither the manifest nor the certificate state present → the empty pre-import certificate state" is retired for node mode (#10): an empty certificate state cannot satisfy DUR-003. It survives only in admission mode and diagnostics. #10 is expected to be recorded as falsified at the re-evidence pass if node mode still takes that branch.
- S7. #7 carries DC-MITHRIL-03 (c) and DC-MITHRIL-08 (c); #5 carries DC-MITHRIL-03 (a), DC-MITHRIL-07 (a, c) and RO-MITHRIL-IMPORT-01 (Ade imports a Mithril-authenticated snapshot); #6 carries DC-MITHRIL-03 (b) and DC-MITHRIL-07 (b, d).
- S8. CN-MITHRIL-01's "no Mithril or multisignature crate is imported under a BLUE crate path" — carried by #19.
- S9. #19 is new, from the round-1 review: `ade mithril snapshot fetch` treats an existing ledger directory as previously verified without checking any digest, and bootstrap accepts local files without verification evidence. #19 is expected to be recorded as falsified at the re-evidence pass.
- S10. DC-EVIDENCE-04's promotion condition for the partition-convergence claim — drafted as #21.
- S11. RO-SYNC-EVIDENCE-01's operator-witnessed live leg on a private Conway testnet of two reference nodes — deferred to B12 resources-release with the other live acceptance criteria; #18 carries the manifest.
- S12. #14 names the project's canonical local reference-node peer (the cardano-node-preprod container in CLAUDE.md), as DC-EVIDENCE-01 did.
- S13. #6's genesis-file binding is new from the round-2 review: the native route checks the manifest's genesis hash against the profile but not the genesis file it parses, so a substituted file could change the active-slot coefficient. Expected to be recorded as falsified at the re-evidence pass.
- S14. DC-MITHRIL-08 (b) (build the native checkpoint from the shared BLUE and GREEN primitives, not from the admission bootstrap's private helper) — excluded as a code-organization rule, not a property of the system: the property it protects, one bootstrap authority with no second path, is STR-013, and #11 carries the checkpoint's construction, timing and failure semantics.
