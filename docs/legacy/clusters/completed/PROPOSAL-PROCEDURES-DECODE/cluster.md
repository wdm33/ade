# Cluster PROPOSAL-PROCEDURES-DECODE — closed Conway tx-body proposal-procedure grammar

> **Status:** Planning artifact (non-normative). **Introduces `DC-LEDGER-11`**
> (new derived-Cardano rule) on first-slice landing. Produced from
> `docs/planning/proposal-procedures-decode-invariants.md` and
> `docs/planning/proposal-procedures-decode-cluster-slice-plan.md`.
> If this doc conflicts with the registry/specs, those win.

---

## Primary invariant

> `proposal_procedures` MUST NOT remain an opaque byte field in the
> authoritative Conway tx-body shape. `ConwayTxBody.proposal_procedures`
> is `Option<Vec<ProposalProcedure>>`, decoded through a single closed
> entry point `decode_proposal_procedures` that rejects unknown
> `gov_action` tags, structural failures, empty sets, and trailing
> garbage deterministically; the typed form re-encodes byte-identically
> (PreservedCbor) for every well-formed Conway tx body. Strengthens
> `DC-LEDGER-10` (UpdateCommittee discriminant) by closing its
> authoritative entry path.

## Normative anchors

- `docs/ade-invariant-registry.toml` — new `DC-LEDGER-11` (appended on
  PP-S1 landing); existing `DC-LEDGER-10` (consumed; preserved unchanged);
  `T-CORE-01` (closed semantic surfaces).
- CIP-1694 — proposal_procedure wire grammar.
- Project constitution §3 (derived-Cardano determinism doctrine).
- IDD `~/.claude/methodology/idd.md` Part I §1 (invariants precede
  features), §6 (closed semantic surfaces), §9 (FC/IS partition).

## OQ resolutions (locked — see invariants sketch §Out of scope)

- **OQ-1** Defer `voting_procedures` (key 19). Separate authority surface; folding it in turns a sealed decode slice into a broad governance refactor.
- **OQ-2** Keep `GovAction::ParameterChange.update` opaque. Full pparams update sub-grammar is a separate large semantic surface; pulling it in blurs this slice's proof obligation.
- **OQ-3** Keep `GovAction::NewConstitution.raw` opaque. Same reasoning; later strengthening slice.
- **OQ-4** Keep `proposal_procedure.return_addr` as raw reward-account `Vec<u8>`. Typed `RewardAccount` is a separate fidelity decision.
- **OQ-5** Prefer existing corpus; synthesize canonical fixtures if no real Conway tx in current corpus carries proposals. Real-chain corpus is a strengthening slice (or rolled into PP-S2 if corpus is available).
- **OQ-6** New registry rule `DC-LEDGER-11` (not a strengthening of an existing one).
- **OQ-7** New dedicated CI gate `ci/ci_check_proposal_procedures_closed.sh`.
- **OQ-8** Reset irrelevant test fixtures setting `proposal_procedures: Some(vec![0x80])` to `None`; synthesize only where load-bearing.

## Grounding (verified at HEAD `3af9e2b`)

- Current shape: `ConwayTxBody.proposal_procedures: Option<Vec<u8>>`
  (`crates/ade_types/src/conway/tx.rs:46`); populated by the body
  decoder at key 20 in `crates/ade_codec/src/conway/tx.rs:132-135`
  (skip_item pass-through).
- Re-encoder uses the same opaque-bytes pass-through at
  `crates/ade_codec/src/conway/tx.rs:349`.
- `GovAction` enum already closed (7 variants) in
  `crates/ade_types/src/conway/governance.rs:30-51`.
  `UpdateCommittee.{removed, added}` already discriminated
  `StakeCredential` (DC-LEDGER-10).
- `Anchor` struct already defined in
  `crates/ade_types/src/conway/governance.rs:75`.
- Era-gate already in place: pre-Conway era rejects key 20 via
  `ade_ledger::error::ProposalProceduresInPreConway`.
- Approximately 10 test files set `proposal_procedures: None` and 2
  set `Some(vec![0x80])` (see grep evidence in invariants sketch §7
  OQ-8); the `None` callers stay compatible, the two `Some` callers
  need updating per OQ-8.
- Existing `decode_conway_certs` pattern in
  `crates/ade_codec/src/conway/cert.rs` is the template for the new
  closed-grammar decoder.

## Entry Conditions

- PHASE4-B1..B5 + PHASE4-B3F closed (`ConwayTxBody` is the
  authoritative tx-body shape; `tx_validity` consumes it; no-false-accept
  proven for in-scope adversarials).
- ENACTMENT-COMMITTEE-WRITEBACK closed (`GovAction::UpdateCommittee`
  is structured + discriminated via DC-LEDGER-10).
- Constitution-coverage gate PASSES at HEAD
  (`bash ci/ci_check_constitution_coverage.sh`).

## Exit Criteria (CI-Verifiable)

- **CE-PP-1 (closure)** — `ConwayTxBody.proposal_procedures` is
  `Option<Vec<ProposalProcedure>>`, not `Option<Vec<u8>>`;
  `decode_proposal_procedures` is the only sanctioned production
  decoder; `ci/ci_check_proposal_procedures_closed.sh` forbids
  opaque-bytes reconstruction of the field and out-of-decoder
  construction of `ProposalProcedure`. (DC-LEDGER-11.)
- **CE-PP-2 (PreservedCbor round-trip — synthetic)** — for every
  well-formed synthetic `Vec<ProposalProcedure>` covering all 7
  `GovAction` variants, `decode → encode` is byte-identical.
- **CE-PP-3 (closed `GovAction` reuse + DC-LEDGER-10 preserved)** —
  `decode_gov_action` decodes all 7 variants through the existing
  closed enum; `UpdateCommittee.{removed, added}` continue to carry
  the discriminated `StakeCredential` form.
- **CE-PP-4 (era-gate preserved)** — `proposal_procedures` continues
  to reject pre-Conway-era decode (no change to the era-gate path).
- **CE-PP-5 (rejection grammar)** — adversarial inputs reject
  deterministically: unknown `gov_action` tag, empty
  `proposal_procedures` set, trailing garbage, missing required
  fields, structurally-invalid `Anchor`.
- **CE-PP-6 (corpus replay)** — corpus of Conway txs carrying
  `proposal_procedures` decodes and re-encodes byte-identically.
  Real-chain extracted if available; otherwise canonical synthetic
  fixtures per OQ-5.

## Expected Slice Types

- **PP-S1** — BLUE decoder + closed `ProposalProcedure` type + CI gate +
  DC-LEDGER-11 registry append. Modifies `ConwayTxBody.proposal_procedures`
  to `Option<Vec<ProposalProcedure>>`; ships
  `crates/ade_codec/src/conway/governance.rs` (new) with `decode_proposal_procedures` /
  `decode_proposal_procedure` / `decode_gov_action` /
  `encode_proposal_procedures`; ships
  `ci/ci_check_proposal_procedures_closed.sh`; appends `DC-LEDGER-11`.
  Cascading test-file updates per OQ-8. *(CE-PP-1, CE-PP-2, CE-PP-3,
  CE-PP-4, CE-PP-5)*. **TCB: BLUE + CI.**
- **PP-S2** — Corpus replay harness. Adds
  `crates/ade_testkit/src/governance/proposal_procedures_replay.rs`
  (or similar); decodes + re-encodes every Conway tx with
  `proposal_procedures` from the existing corpus (real-chain) or
  from synthetic canonical fixtures; asserts byte-identical
  round-trip. Extends `DC-LEDGER-11.tests` with the new harness test
  names. *(CE-PP-6)*. **TCB: GREEN.**

## TCB Color Map

- **BLUE** — `ade_codec::conway::governance` (new module);
  `ade_types::conway::governance::ProposalProcedure` (new closed
  struct, existing `GovAction` + `Anchor` reused);
  `ade_codec::conway::tx` (modified — body key 20 path now calls the
  typed decoder/encoder).
- **GREEN** — `ade_testkit::governance::proposal_procedures_replay`
  (new harness, PP-S2). PP-S1 may include a minimal inline synthetic
  builder for its unit tests.
- **RED** — none.

## Forbidden During This Cluster

- Reverting `ConwayTxBody.proposal_procedures` back to
  `Option<Vec<u8>>` anywhere on the BLUE authority path (CI-enforced).
- Constructing `ProposalProcedure` from raw bytes outside
  `decode_proposal_procedures` on the production path (CI-enforced).
- Branching admission / validation logic on `ProposalProcedure` shape
  via stringly-keyed map or `dyn` trait object (the closed struct is
  the only sanctioned shape).
- Decoding `voting_procedures` (key 19), `ParameterChange.update`,
  or `NewConstitution.raw` in this cluster — OQ-1/2/3 are scope
  locks. Strengthening slices may close those later.
- Typing `proposal_procedure.return_addr` beyond `Vec<u8>` —
  separate fidelity decision (OQ-4).
- Touching `voting_procedures` codec path in any way (other than
  noting it stays opaque alongside the now-typed
  `proposal_procedures`).
- Touching `GREEN` snapshot-loader's `parse_governance_proposals`
  path. This cluster works only on the in-flight tx-body codec.
- Wall-clock, randomness, `HashMap`/`HashSet` ordering, or floats
  anywhere in the BLUE decoder.

## Declared non-goals

- `voting_procedures` tx-body decode (OQ-1).
- Full pparams update sub-grammar (`ParameterChange.update`) decode
  (OQ-2).
- `NewConstitution.raw` decode (OQ-3).
- Typed `RewardAccount` for `return_addr` and `TreasuryWithdrawals`
  (OQ-4).
- GREEN snapshot-loader changes (different path).
- Plutus phase-2 validation of proposal procedures.
- Mempool / propagation effects of proposals (separate cluster).
- Tier-5 governance surfaces (operator metrics, query API for
  proposals).

## Follow-ups (NOT regressions — separable strengthenings)

- **`voting_procedures` closed decode.** Same shape as this
  cluster; would strengthen `DC-LEDGER-11` (or add `DC-LEDGER-12`,
  the registry's choice when opened) to also cover the voting-side
  governance tx-body field.
- **`ParameterChange.update` closed decode.** Large pparams update
  sub-grammar; warrants its own cluster.
- **`NewConstitution.raw` closed decode.** Small but separable;
  bundleable with the voting-procedures cluster or shipped alone.
- **Typed `RewardAccount`.** Would strengthen both
  `proposal_procedure.return_addr` and
  `TreasuryWithdrawals.withdrawals` in one move.
