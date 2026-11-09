# Consensus record — docs/planning/consensus-inputs.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/consensus-inputs.define.md @ blob 9b38859d5cc425e84ef51384421d859c58664c20 · received 9b38859d5cc425e84ef51384421d859c58664c20

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #3 | "“take their consensus views only from what was recovered” excludes native boundary-derived leadership objects required by VAL-023 and VAL-024" | revised: the recovered record serves only the two bootstrap epochs; later views come from sealed objects (VAL-023, VAL-024); the bundle prohibition stays | open |
| 2 | S1 | "VAL-001 and DET-007 … do not define the seed record’s required fields, current-version-only acceptance, or rejection of noncanonical encodings" | revised: CN-CINPUT-01 drafted as #11 SeedRecordClosedSchema | open |
| 3 | S4 | "neither explicitly requires recomputing the candidate’s canonical hash over every binding and payload field before activation" | revised: DC-EVIEW-07 drafted as #12 CandidateViewHashVerified | open |
| 4 | #8 | "permits silently classifying them as valid non-staking addresses, losing DC-EVIEW-02’s explicit Reject-versus-Null distinction" | revised: a closed result (base credential, valid no-stake, rejection); malformed addresses and reward addresses used as outputs are rejections, never folded into no-stake | open |
| 5 | #9 | "omits DC-EVIEW-10’s requirement to thread governance state alongside certificate state and to abort aggregation on any processing error" | revised: certificate, delegation and governance state advance in block order; any step failure aborts the window | open |
| 6 | #10 | "omits DC-EVIEW-13’s replay-window ordering: reap at every crossed boundary and capture the mark before future-pool adoption and reaping" | revised: at every boundary crossed, including inside a replayed range, the snapshot is taken first (VAL-028), then reaping; VAL-028 added as a dependency | open |
| 7 | S3 | "DC-EVIEW-05 and DC-EVIEW-12 compute total active stake by summing retained pool entries, whereas VAL-024 explicitly forbids that" | revised: S3 states the departure, excluding the legacy denominator rule as superseded by VAL-024 | open |
| 8 | #7 / S9 | "#7 also drops DC-EVIEW-01’s owned-subtree restriction, deterministic window-key validation, parent fsync, and terminal handling of invalid names" | revised: #7 states each purge constraint | open |
| 9 | #6 | "requires unsupported-schema errors to be distinguishable from corruption and to require explicit reimport" | revised: a distinct unsupported-format error naming re-import, separate from corruption, never silently upgraded | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/consensus-inputs.define.md @ blob db287a4d010e7aa021fbecbdae1d129b5317d2d1 · received db287a4d010e7aa021fbecbdae1d129b5317d2d1

Round 1 items 2–9: Codex marked each `accepted`. Item 1 stayed open: "#3 still says both bootstrap epochs’ views come from the recovered seed record. VAL-023 instead requires seed+1 leadership from the certified snapshot’s imported mark, and VAL-024 requires exact-epoch sealed objects for leadership checks in every epoch."

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #3 | (above) | revised: leader checks in every epoch read only the exact-epoch sealed object; the bootstrap epoch's object is sealed from the seed record's pool distribution, the next epoch's from the imported mark, later ones at crossed boundaries | open |
| 10 | #1 | "“a fingerprint over the record’s canonical encoding” leaves the algorithm unspecified. Its admission-event fingerprint binding also has no disposition." | revised: blake2b-256 over the canonical CBOR encoding in declared field order; S10 defers the admission-event binding to B10 | open |
| 11 | #7 assumptions | "The crash/replay identity guarantee relies on A1’s recorded starting state, ordered inputs and explicit context, but the line cites no assumptions." | revised: #7 cites A1 | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/consensus-inputs.define.md @ blob 4b1cdd6adb9957a272ba86883d66b71d44452175 · received 4b1cdd6adb9957a272ba86883d66b71d44452175

Round 2 items 1, 10 and 11: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 12 | S3 / #12 | "DC-EVIEW-12 requires the profile commitment to bind genesis hash, protocol-parameter hash and active-slot coefficient. … An ASC-only commitment loses legacy meaning despite passing those checks." | revised: #12 defines the profile commitment as blake2b-256 over a domain tag, genesis hash, protocol-parameter hash and active-slot coefficient from the durable profile (#6), now a dependency | open |

Outcome: continue

## Round 4 — 2026-11-08 — docs/planning/consensus-inputs.define.md @ blob 494e89bc0110f9e2ed5d7c9459fe49ba8361b7af · received 494e89bc0110f9e2ed5d7c9459fe49ba8361b7af

Round 3 item 12: Codex marked it `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-08 — after round 4 @ blob 494e89bc0110f9e2ed5d7c9459fe49ba8361b7af

```text
Define batch — consensus-inputs (B06) — consensus: round 4 (docs/planning/consensus-inputs.define.consensus.md)

 1. VAL  ClosedConsensusInputImport                [critical]  One importer turns a consensus-input bundle into a closed record with every field and a blake2b-256 fingerprint; no defaults.
 2. VAL  ImportedViewEpochGuard                    [critical]  An imported view answers only for its own epoch; a block outside its window halts admission.
 3. SEC  ConsensusInputsFromVerifiedBootstrapOnly  [critical]  Only verified bootstrap writes the seed record; forge and feed never use operator bundles; leadership reads sealed objects.
 4. DUR  SeedInputsRecoveryVerified                [critical]  Warm start restores the seed record only from exactly one WAL fact whose hash, decode and bindings all check.
 5. VAL  VrfInputFromEpochNonce                    [critical]  VRF input = blake2b-256(slot as 8 BE bytes ‖ the slot's epoch nonce), IETF draft-03, 80-byte proofs.
 6. DUR  NetworkGeometryAndProfileDurable          [critical]  Replay uses the persisted epoch geometry and consensus profile, never restart config; old formats need re-import.
 7. SAF  TransientViewStorageNonAuthoritative      [critical]  Transient epoch-view storage is never authority, survives no crash as state, and is purged safely at startup.
 8. VAL  StakeReferenceClassification              [critical]  One function classifies an output's stake reference: base credential, valid no-stake, or rejection.
 9. DET  ReducedCheckpointMirrorsLedger            [critical]  The reduced checkpoint mirrors the ledger's own UTxO and certificate processing; partial windows yield nothing.
10. VAL  PoolLifecycleMatchesReference             [critical]  Registration, staged re-registration, snapshot-then-reap and delegation clearing follow the reference ledger.
11. STR  SeedRecordClosedSchema                    [critical]  The seed record is one closed type; only the current version and only canonical bytes decode.
12. SAF  CandidateViewHashVerified                 [critical]  A view's canonical hash is recomputed over every binding and payload before use; the profile commitment covers genesis, parameters and ASC.

Settled in consensus:
 S1. CN-CINPUT-01 — drafted as 11.   S4. DC-EVIEW-07 — drafted as 12.
 S2. DC-CINPUT-02a, 02b, 04 (views projected from the recovered record) — merged into VAL-023, VAL-024, STR-007; the bundle-equivalence clause is excluded.
 S3. DC-EVIEW-05, 12 (stake aggregation; leadership-complete view) — merged into VAL-020, VAL-023, VAL-024; their pool-sum denominator rule is excluded as superseded by VAL-024.
 S5. DC-EVIEW-11 (epoch-rebind seam) — merged into ORD-004 and SAF-005.
 S6. DC-EVIEW-06 (k-deep stability gate) — excluded as superseded: observe-only, never wired; replaced by sealed objects and rewind coherence.
 S7. DC-EVIEW-08 (window-replay activation) — excluded as superseded by sealed leadership objects.
 S8. DC-EVIEW-09 (bootstrap cert-state import), DC-CINPUT-07 (Conway deposit parameters from the snapshot) — deferred to B11.
 S9. DC-EVIEW-01's memory gate — deferred to B12.
 S10. DC-CONS-IN-02's fingerprint binding into admission events — deferred to B10.

Legacy dispositions (29): 1←CN-CONS-IN-01, DC-CONS-IN-01, DC-CONS-IN-02 · 2←DC-VIEW-01 · 3←CN-CINPUT-02, CN-CINPUT-03 · 4←DC-CINPUT-01 · 5←DC-CINPUT-03 · 6←DC-CINPUT-05, 06 · 7←DC-EVIEW-01 · 8←DC-EVIEW-02, 03 · 9←DC-EVIEW-04, 04b, 10 · 10←DC-EVIEW-13 · 11←CN-CINPUT-01 · 12←DC-EVIEW-07 · merged DC-CINPUT-02a, 02b, 04, DC-EVIEW-05, 11, 12 · excluded DC-EVIEW-06, 08 · deferred DC-EVIEW-09, DC-CINPUT-07 → B11

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"
