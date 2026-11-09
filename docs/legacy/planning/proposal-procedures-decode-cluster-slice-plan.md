# Cluster/Slice Plan — Ade — PROPOSAL-PROCEDURES-DECODE

> **Status:** Planning artifact (non-normative). Produced via `/cluster-plan`
> on 2026-05-25 against HEAD `3af9e2b`. Built on the invariants sketch
> at `docs/planning/proposal-procedures-decode-invariants.md`.
> Authority lives in `docs/ade-invariant-registry.toml`; if this plan
> conflicts with the registry the registry wins.

## Cluster Index (Dependency Order)

1. **PROPOSAL-PROCEDURES-DECODE** — *close `proposal_procedures`
   opacity at the Conway tx-body boundary* — single cluster; depends
   on PHASE4-B1..B5 (Conway tx-body decoder + `GovAction` enum
   already in place) and ENACTMENT-COMMITTEE-WRITEBACK
   (`UpdateCommittee` structured form via DC-LEDGER-10).

`voting_procedures`, `GovAction::ParameterChange.update`, and
`GovAction::NewConstitution.raw` remain opaque per the user-locked
OQ-1 / OQ-2 / OQ-3 scope decisions in the invariants sketch.

---

## Cluster PROPOSAL-PROCEDURES-DECODE — closed Conway tx-body proposal-procedure grammar

### Primary invariant

`proposal_procedures` MUST NOT remain an opaque byte field in the
authoritative Conway tx-body shape. `ConwayTxBody.proposal_procedures`
is `Option<Vec<ProposalProcedure>>`, decoded through a single closed
entry point `decode_proposal_procedures` that rejects unknown
`gov_action` tags, structural failures, empty sets, and trailing
garbage deterministically; the typed form re-encodes byte-identically
(PreservedCbor) for every well-formed Conway tx body. Strengthens
`DC-LEDGER-10` (UpdateCommittee discriminant) by closing its
authoritative entry path.

### TCB partition

| Color | Modules |
|---|---|
| BLUE | `ade_codec::conway::governance` (new module — `decode_proposal_procedures`, `decode_proposal_procedure`, `decode_gov_action`, `encode_proposal_procedures`); `ade_types::conway::governance::ProposalProcedure` (new closed struct); `ade_codec::conway::tx` (modified — body key 20 path now calls the typed decoder/encoder). |
| GREEN | `ade_testkit::governance::proposal_procedures_replay` (new harness, S2). Synthetic-fixture builders live here (S1 may add a minimal inline helper if needed for unit tests). |
| RED | none. |

### Cluster Exit Criteria

- **CE-PP-1 (closure)** — `ConwayTxBody.proposal_procedures` is
  `Option<Vec<ProposalProcedure>>`, not `Option<Vec<u8>>`;
  `decode_proposal_procedures` is the only sanctioned production
  decoder; CI gate `ci/ci_check_proposal_procedures_closed.sh`
  forbids opaque-bytes reconstruction or out-of-decoder construction
  of `ProposalProcedure`. (DC-LEDGER-11.)
- **CE-PP-2 (PreservedCbor round-trip — synthetic)** — for every
  well-formed synthetic `Vec<ProposalProcedure>` covering all 7
  `GovAction` variants, `decode → encode` is byte-identical.
  Unit-test surface.
- **CE-PP-3 (closed `GovAction` reuse + DC-LEDGER-10 preserved)** —
  `decode_gov_action` decodes all 7 variants through the existing
  closed `GovAction` enum; `UpdateCommittee.{removed, added}`
  continue to carry the discriminated `StakeCredential` form
  (DC-LEDGER-10 gate stays green; no regression).
- **CE-PP-4 (era-gate preserved)** — `proposal_procedures` continues
  to reject pre-Conway-era decode with the existing
  `ProposalProceduresInPreConway` error; the new typed decoder does
  not alter the era-gate path.
- **CE-PP-5 (rejection grammar)** — adversarial inputs reject
  deterministically: unknown `gov_action` tag, empty
  `proposal_procedures` set, trailing garbage after the procedures,
  missing `deposit` / `return_addr` / `anchor` / `gov_action`,
  structurally-invalid `Anchor`. Unit-test surface.
- **CE-PP-6 (corpus replay)** — corpus of Conway txs carrying
  `proposal_procedures` (real-chain extracted if available per OQ-5;
  otherwise canonical synthetic fixtures) decodes and re-encodes
  byte-identically. GREEN harness.

### Slices

| ID | Name | Invariant | Addresses | TCB |
|---|---|---|---|---|
| **PP-S1** | BLUE decoder + closed `ProposalProcedure` type + CI gate + DC-LEDGER-11 | `proposal_procedures` decodes through the single closed entry; opaque-bytes form is no longer the wire-side `ConwayTxBody` shape; PreservedCbor round-trips on synthetic inputs; DC-LEDGER-10 stays enforced. | CE-PP-1, CE-PP-2, CE-PP-3, CE-PP-4, CE-PP-5 | BLUE + CI |
| **PP-S2** | Corpus replay harness | An ordered corpus of Conway txs carrying `proposal_procedures` decodes + re-encodes byte-identically (real-chain bytes if any in existing corpus; synthetic canonical fixtures otherwise). | CE-PP-6 | GREEN harness over BLUE |

### Replay obligations

- **New canonical type.** `ProposalProcedure` (closed struct:
  `deposit: Coin`, `return_addr: Vec<u8>`, `anchor: Anchor`,
  `gov_action: GovAction`). Must appear in the project's
  closed-struct CI grep targets (`ci_check_proposal_procedures_closed.sh`).
- **Type-shape change.** `ConwayTxBody.proposal_procedures:
  Option<Vec<u8>>` → `Option<Vec<ProposalProcedure>>`. Cascading
  test-file updates per OQ-8: drop meaningless `Some(vec![0x80])`
  fixtures to `None`; convert any load-bearing tests to synthesize
  via a minimal helper.
- **PreservedCbor round-trip.** Encode/decode equivalence is the
  load-bearing replay claim. Unit tests in S1 cover synthetic
  inputs; corpus harness in S2 covers real-or-synthetic-canonical
  bytes.
- **No new adversarial-corpus tx bytes outside the closed grammar's
  test scope.** Reuses existing tx-body corpus for round-trip;
  adversarial inputs (PP-N1, PP-N5, PP-N6, PP-N8) are synthesized
  inline in S1's unit tests.
- **No GREEN snapshot-loader change** —
  `ConwayGovState.proposals` population from the loader is
  independent of this cluster (different path).

### Independent-mergeability check (per IDD discipline)

- **PP-S1** ships the type change + decoder + encoder + CI gate +
  DC-LEDGER-11. After S1, the system is fully correct:
  `proposal_procedures` is closed at the boundary; round-trip works
  on synthetic inputs; era-gate unchanged; DC-LEDGER-10 preserved.
  No invariant weakened.
- **PP-S2** ships the GREEN corpus harness; no production code
  changes, no registry shape change (may extend
  `DC-LEDGER-11.tests` array with corpus test names). Depends on
  S1 (the harness consumes `ProposalProcedure`).

Each slice independently leaves the system in a fully correct state.

### Registry moves at cluster-close

| Slice | Registry change |
|---|---|
| PP-S1 | **Append `DC-LEDGER-11`** with real `code_locus`, real `ci_script`, real `tests` (S1 round-trip + rejection-grammar test names). `status = "enforced"`, `introduced_in = "PROPOSAL-PROCEDURES-DECODE"`. |
| PP-S2 | Extend `DC-LEDGER-11.tests` with the corpus-harness test names. No new rule. |

`DC-LEDGER-10` is **not** strengthened (UpdateCommittee structured
form is consumed unchanged; no new claim on its surface). Other
`DC-*` / `T-*` / `CN-*` / `RO-*` / `OP-*` rules unchanged.

### Sequencing rationale

PP-S1 before PP-S2 — the harness consumes `ProposalProcedure`.
Single serial dependency; no branching. The cluster can close in
2 commits plus a cluster-close grounding-refresh commit
(3 total), similar shape to the smaller PHASE4-N-E slices.

## Stop conditions

- The cluster fully closes in this work. No operator-action
  half (unlike CE-N-E-6/7); CE-PP-6's corpus comes from the
  existing in-tree Conway tx corpus or from synthetic canonical
  fixtures, both within reach of CI.
- If the existing in-tree corpus contains zero Conway txs with
  `proposal_procedures`, fall back to synthetic canonical
  fixtures per OQ-5 — that's a legitimate close, not a deferral.
