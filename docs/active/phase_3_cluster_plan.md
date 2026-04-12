# Cluster Plan: Phase 3 — UPLC Evaluator and State-Backed Late-Era Validation

> **Status:** Proposed
>
> **Authority Level:** Cluster Plan (Non-Normative Planning Artifact)
>
> This document defines the scope, exit criteria, and slice sequencing for
> Phase 3. It introduces no new requirements and does not override any
> normative specification. Normative authority resides in
> `01_core_determinism_and_contract.md`, the project constitution
> (`ade_replay_first_constitutional_node_plan_v1.md` §2–§4b), and
> `classification_table.md`.

---

## Cluster Name

**Phase 3: UPLC Evaluator and State-Backed Late-Era Validation**

## Purpose

Close the two correctness gaps that Phase 2 deferred by design:

1. **State-backed validation** for Alonzo, Babbage, and Conway
   transactions — the ledger checks that require resolved UTxO state
   (collateral existence/total/percent, input resolution, reference
   input resolution, datum-hash-to-datum binding, required-signer
   enforcement, network-id checks).

2. **Plutus script execution** — UPLC (Untyped Plutus Core) evaluation
   for V1/V2/V3 including cost accounting, ScriptContext derivation,
   and result integration with ledger rules.

After Phase 3, a block the Haskell cardano-node accepts or rejects
for *any* reason — structural, state-backed, or script-related —
receives the same verdict from Ade. The "non-Plutus only" release
tier (CE-79 Tier 3) is lifted to "full Cardano mainnet validation
at cardano-node 10.6.2."

## Relationship to Phase 2

Phase 3 inherits the entire Phase 2C authority surface, compatibility
boundary, byte authority contract, forbidden-patterns list, and
mixed-version proof obligations. Those sections remain authoritative
in the Phase 2C cluster plan and apply here without modification.

Phase 3 removes — deliberately — two exclusions from Phase 2:

- **Plutus deferral (CE-79 Tier 4)**: Phase 2 treated all Plutus-using
  blocks as `ScriptVerdict::NotYetEvaluated`. Phase 3 evaluates them.
- **Structural-only late-era validation (CE-68/69/70)**: Phase 2
  accepted Alonzo+ blocks based on wire shape alone. Phase 3 adds the
  state-backed checks the cluster plan originally bundled with Plutus.

## Scope Decisions (Confirmed)

These decisions are ground truth for the cluster. Changing any of
them invalidates downstream slice scopes and proof obligations.

| # | Decision | Ground Truth |
|---|----------|--------------|
| 1 | Evaluator strategy | **Hybrid**: port `aiken-lang/aiken/crates/uplc` (Apache 2.0), wrap in a new `ade_plutus` crate that exposes Ade-canonical types only. Pallas-originated types stay isolated inside `ade_plutus`; they do not leak into `ade_ledger`. |
| 2 | Phase split | **3A → 3B.** 3A delivers state-backed late-era validation without UPLC execution. 3B delivers UPLC execution and integrates with 3A. Each is independently shippable with its own exit criteria. |
| 3 | Plutus versions in scope | **V1, V2, V3 together.** UPLC semantics are the same across versions; version differences reduce to the set of available built-ins and cost-model coefficients. The BLS12-381 primitives introduced in V3 (CIP-0381) are in scope. |
| 4 | Conformance oracle | **Dual**: IOG `plutus-conformance/test-cases/uplc/` (primary, covers semantics + built-ins + BLS) + existing 9,436 mainnet Plutus transactions (secondary, covers ScriptContext + full pipeline). |
| 5 | Cost models | **Parse-only** from protocol parameters. No calibration of cost-model coefficients. Treat the coefficients as protocol input; verify budget accounting matches oracle. |

---

## Inputs

| Input | Source | Purpose |
|-------|--------|---------|
| Phase 2 outputs | Code repo at `/home/ts/Code/rust/ade` | `ade_ledger` with non-Plutus verdict agreement, `LedgerState` fingerprint, snapshot loader, structural late-era validation |
| Phase 2A outputs | Code repo | `ade_crypto`: Blake2b, Ed25519, VRF, KES; secp256k1 available via aiken uplc dependency |
| Phase 2C cluster plan | `docs/active/cluster_plan.md` | Authority surface, forbidden patterns, byte authority, Plutus deferral gate (inherited) |
| CE-79 gate statement | `docs/active/CE-79_gate_statement.md` | Tier vocabulary; Phase 3 lifts Tier 4 "partial Plutus" non-goal |
| aiken UPLC crate | `github.com/aiken-lang/aiken/crates/uplc` (Apache 2.0) | Parser, flat codec, CEK machine, built-ins, cost accounting, BLS12-381 |
| IOG plutus-conformance | `github.com/IntersectMBO/plutus/plutus-conformance/test-cases/uplc/` (Apache 2.0) | Authoritative UPLC test vectors: `.uplc` + `.uplc.expected` + `.uplc.budget.expected` |
| Mainnet Plutus corpus | `corpus/contiguous/{alonzo,babbage,conway}/` (already extracted) | 9,436 transactions flagged `NotYetEvaluated` in Phase 2; now required to verdict-match oracle |
| Cardano ledger specs | Alonzo/Babbage/Conway formal specs | State-backed rules per era |
| CIP-0381 | `cips.cardano.org/cip/CIP-381` | BLS12-381 built-ins specification (V3) |

## Outputs

| Output | Location | Description |
|--------|----------|-------------|
| `ade_plutus` crate | `crates/ade_plutus/` | BLUE crate wrapping aiken UPLC; Ade-canonical types at the surface |
| State-backed late-era rules | `crates/ade_ledger/src/{alonzo,babbage,conway}.rs` | Collateral, input resolution, datum-hash, required-signer, network-id checks |
| ScriptContext derivation | `crates/ade_plutus/src/script_context.rs` | Per-version ScriptContext construction from tx + resolved state |
| Cost model handling | `crates/ade_plutus/src/cost_model.rs` | Parse from pparams, no calibration |
| Full verdict integration | `crates/ade_ledger/src/scripts.rs` | `ScriptVerdict::Passed` / `Failed` replace `NotYetEvaluated` |
| Conformance corpus | `corpus/plutus_conformance/` | Vendored IOG `.uplc` test vectors with provenance manifest |
| CI scripts | `ci/ci_check_plutus_conformance.sh`, `ci/ci_check_plutus_mainnet_verdict.sh` | Conformance-pass + mainnet-verdict gates |
| Registry updates | `constitution_registry.toml` | DC-LEDGER-03/05 and DC-EPOCH-01 move `partial → enforced` |

---

## Authority Boundary and Non-Goals

### In scope (Phase 3)

- UPLC V1/V2/V3 evaluation for all built-ins in the plutus version
  pinned to cardano-node 10.6.2
- BLS12-381 primitives (CIP-0381)
- ScriptContext derivation for each Plutus version
- Cost-model parsing and budget accounting
- State-backed Alonzo/Babbage/Conway ledger rules (collateral, input
  resolution, datum-hash binding, required signers, network ID)
- Full verdict integration: a Plutus-using tx is accepted iff ledger
  rules pass AND all scripts evaluate successfully within budget

### Not in scope (deferred to later phases or declared non-goals)

- Cost-model calibration (parse-only per decision #5)
- Plutus Tx (Plinth) source-level tooling — this is a compiler concern
- Aiken-language compilation — this is a compiler concern
- Consensus-side Plutus (Phase 4) — chain selection, leader schedule
- Block production signing (Phase 5)
- Live differential against a running Haskell node (ShadowBox, external)
- Byte-parity with Haskell on-disk `ExtLedgerState` CBOR (CE-79 Tier 4
  remains a non-goal)
- Protocol versions beyond cardano-node 10.6.2 (version-scoped)

---

## Invariant Authority Clusters

Phase 3 is organized around four invariant authority clusters. Every
slice advances one or more clusters. Clusters are the unit of
correctness claim; slices are the unit of delivery. This matches the
invariant-slice planning convention (plans organize around authority,
not feature accumulation).

### Cluster P-A: State-Backed Late-Era Validation Authority

**Authority surface**: the ledger rules for Alonzo/Babbage/Conway
that resolve transaction components against `LedgerState`, producing
a verdict that does not depend on script execution.

**Invariant**: given `(tx, ledger_state, protocol_version)`, the
state-backed verdict matches oracle for every non-Plutus check,
independent of whether the tx uses scripts.

**Components**:
- Collateral UTxO existence + total + percent (Alonzo+)
- Input UTxO resolution (every input exists in the resolved UTxO)
- Reference input resolution (Babbage+)
- Datum hash in output matches witness-provided datum
- Required signers are present in tx witnesses
- Network ID matches network (Alonzo+)
- Fee vs. min-fee enforcement with reference-script size (Babbage+)

**Slices**: S-27, S-28 (see sequencing).

### Cluster P-B: UPLC Evaluation Authority

**Authority surface**: the CEK machine that evaluates a UPLC term
against arguments, producing a result and budget consumption.

**Invariant**: given `(uplc_term, args, cost_model, builtin_set)`,
the evaluation result (return value or typed error) and budget
consumption are identical to the IOG reference implementation at
the plutus version pinned to cardano-node 10.6.2.

**Components**:
- Parser (flat-encoded and textual UPLC)
- CEK machine semantics (substitution, reduction, environment)
- Built-in function semantics per version (including BLS12-381 in V3)
- Cost accounting (CPU + memory units)
- Error semantics (typed errors for every failure mode)

**Slices**: S-29, S-30.

### Cluster P-C: ScriptContext Derivation Authority

**Authority surface**: the bridge between Ade's `LedgerState` + tx
and the UPLC evaluator's input. Deterministically constructs the
per-script-purpose ScriptContext that the Plutus script receives.

**Invariant**: given `(tx, resolved_utxo, script_purpose,
protocol_version)`, the constructed ScriptContext serializes
identically to oracle at every PV where that script purpose is
available. Includes Conway's governance-action script purposes.

**Components**:
- ScriptInfo / purpose variants per version
- TxInfo construction (inputs, outputs, fees, withdrawals, certs,
  validity interval, mint, required signers, redeemers, datums,
  reference inputs, votes, proposals)
- Resolved-output encoding (datum vs. datum-hash representation)
- Serialization to Plutus Data

**Slices**: S-31.

### Cluster P-D: Script Verdict Integration Authority

**Authority surface**: the gate that combines state-backed verdicts
(from P-A) with script verdicts (from P-B using contexts from P-C)
into a single transaction verdict and state delta.

**Invariant**: a transaction is accepted iff all state-backed checks
pass AND every script executes successfully within its budget. On
failure, the correct state delta applies per era (phase-1 failure =
tx rejected, no state change; phase-2 failure = collateral consumed,
outputs not produced).

**Components**:
- Per-script budget enforcement
- Aggregate tx-level budget cap
- Phase-1 / phase-2 failure distinction
- Collateral consumption on phase-2 failure
- `ScriptVerdict::Passed` / `Failed` replaces `NotYetEvaluated`

**Slices**: S-32.

---

## Slice Sequencing

```
Phase 2 complete (T-19 through T-26)
        │
   ┌────┴──── Phase 3A (State-Backed Late-Era Validation) ────┐
   │                                                          │
   S-27  Collateral + input resolution (Alonzo+)              │
        │                                                     │
   S-28  Reference inputs + datum-hash + required signers     │
        │                                                     │
        │ ────────── Phase 3A ships here ──────────           │
   ┌────┴──── Phase 3B (UPLC Evaluation + Integration) ───────┤
   │                                                          │
   S-29  ade_plutus scaffold + aiken UPLC port                │
        │                                                     │
   S-30  Cost models + budget accounting + conformance suite  │
        │                                                     │
   S-31  ScriptContext derivation (V1/V2/V3)                  │
        │                                                     │
   S-32  Verdict integration + mainnet verdict agreement      │
        │                                                     │
        │ ────────── Phase 3B ships here ──────────           │
   └──────────────── Phase 3 complete ────────────────────────┘
```

Strict sequential chain within each sub-phase. Phase 3A is
independently shippable — at its completion, Ade correctly rejects
late-era blocks with fabricated inputs or bad collateral (closing
the biggest Phase 2 silent acceptance).

### Slice Summaries

| Slice | Name | Cluster | Entry Obligations (must be discharged before code) |
|-------|------|---------|----------------------------------------------------|
| S-27 | Collateral + input resolution | P-A | O-27.1, O-27.2, O-27.3 |
| S-28 | Reference inputs + datum-hash + required signers | P-A | O-28.1, O-28.2, O-28.3, O-28.4 |
| S-29 | `ade_plutus` scaffold + UPLC port | P-B | O-29.1, O-29.2, O-29.3 |
| S-30 | Cost models + budget + conformance | P-B | O-30.1, O-30.2, O-30.3 |
| S-31 | ScriptContext derivation | P-C | O-31.1, O-31.2, O-31.3, O-31.4 |
| S-32 | Verdict integration + mainnet | P-D | O-32.1, O-32.2, O-32.3 |

---

## Slice-Entry Proof Obligations

Every unknown compatibility fact is listed here as a slice-entry
obligation. None of these is a footnote; each must be discharged
(answered with evidence) before its slice begins implementation.
Discharge may be by reading oracle source, by writing a one-off
probe test, or by citing an existing mechanical test. What is
forbidden is waving the obligation away or leaving it implicit.

### S-27 (Collateral + input resolution)

- **O-27.1** — What is the exact collateral percent per era (Alonzo
  5%, Babbage/Conway: verify against 10.6.2)? Is it rounded via
  `ceil`, `floor`, or `round`? Cite cardano-ledger module.
- **O-27.2** — Does collateral require the collateral-return output
  (Babbage+) or is it optional? What's the total-collateral field's
  role? Cite Babbage ledger spec.
- **O-27.3** — At PV<=6 vs PV>6, does input-UTxO-missing produce the
  same error classification (phase-1 failure)? Probe with a golden tx.

### S-28 (Reference inputs + datum-hash + required signers)

- **O-28.1** — Can a reference input be spent in the same tx (via
  regular inputs)? Cite Babbage spec.
- **O-28.2** — Datum hash in output vs. inline datum: does the
  witness-set datum need to match the hash bit-exactly, or is
  canonical-form equality sufficient? Probe with a golden tx.
- **O-28.3** — Required signers: must every Hash28 in the
  required-signers set appear in `vkey_witnesses`, or is a
  redeemer-provided signer acceptable? Cite Alonzo spec.
- **O-28.4** — Network ID check: which field carries it (tx body
  field or implicit from address)? Cite Alonzo spec.

### S-29 (`ade_plutus` scaffold + UPLC port)

- **O-29.1** — Which aiken commit's `uplc` crate do we pin to?
  Target: the commit whose conformance-test output matches the plutus
  version used by cardano-node 10.6.2. Discharge: run aiken's
  conformance-test suite against the IOG vectors at the pinned
  commit; verify zero divergence.
- **O-29.2** — What pallas-* transitive dependencies does aiken
  `uplc` pull in, and which versions? Does any of them conflict with
  Ade's existing deps? Discharge: `cargo tree` output reviewed.
- **O-29.3** — Does aiken `uplc` support the exact Flat byte encoding
  used by cardano-node 10.6.2, including edge cases (empty programs,
  large integers, BLS12-381 element parsing)? Probe with mainnet txs.

### S-30 (Cost models + budget + conformance)

- **O-30.1** — Cost-model CBOR format in pparams: is it a flat
  integer array (V1), keyed map (V2), or versioned map (V3)?
  Discharge: decode from a 10.6.2 snapshot and compare to cardano-cli
  `query protocol-parameters` output.
- **O-30.2** — Does aiken's cost accounting produce byte-identical
  budget consumption to the Haskell reference on every IOG
  conformance test? Discharge: `.uplc.budget.expected` match on the
  full suite.
- **O-30.3** — Budget reporting granularity: is the tx-level budget
  cap enforced per-script or aggregated? Cite Alonzo+ spec.

### S-31 (ScriptContext derivation)

- **O-31.1** — ScriptContext encoding differs between V1, V2, V3.
  What are the exact structural differences? Discharge: reference
  Plutus.V1.Ledger.Api vs V2 vs V3 type definitions; document deltas.
- **O-31.2** — How are reference inputs represented in ScriptContext
  V2 vs V3? Are they indistinguishable from regular inputs to the
  script, or a separate field?
- **O-31.3** — Conway governance scripts: ScriptInfo variants include
  `Voting`, `Proposing`, `Certifying`. What's in each context?
- **O-31.4** — Datum resolution: when an input refers to a datum by
  hash, does the ScriptContext include the datum body or only its
  hash? Cite Plutus V1/V2/V3 Ledger API.

### S-32 (Verdict integration + mainnet)

- **O-32.1** — Phase-1 vs. phase-2 failure: on what exact failure
  classes does collateral get consumed (phase-2) vs. tx-rejected-
  outright (phase-1)? Cite Alonzo spec.
- **O-32.2** — When multiple scripts execute, does the tx-level
  budget cap accumulate across them, or is each script independent?
  Cite ledger spec.
- **O-32.3** — Conway governance-action scripts: if a voting script
  fails, is the vote ignored or does the whole tx fail? Cite Conway
  spec.

---

## Exit Criteria

Phase 3 introduces a new block of exit criteria (CE-83 through
CE-91). Numbering continues from Phase 2C's CE-82. These are
proposed here; the project constitution is amended to adopt them
when this cluster plan is approved.

| CE | Description | Slice |
|----|-------------|-------|
| CE-83 | Collateral rules match oracle across Alonzo/Babbage/Conway corpus | S-27 |
| CE-84 | Input and reference-input resolution matches oracle; network-ID check enforced; datum-hash binding verified | S-28 |
| CE-85 | `ade_plutus` crate passes IOG conformance suite (`.uplc.expected` match, full suite) at pinned aiken commit | S-29 |
| CE-86 | Budget accounting: `.uplc.budget.expected` match across full conformance suite; cost-model parsing matches oracle pparams | S-30 |
| CE-87 | ScriptContext derivation produces oracle-identical CBOR for every mainnet Plutus tx in the corpus, at every PV | S-31 |
| CE-88 | All 9,436 mainnet Plutus transactions receive the same verdict as oracle (accept/reject), with identical phase-1/phase-2 classification | S-32 |
| CE-89 | `ScriptVerdict::NotYetEvaluated` no longer appears in any verdict path in the corpus | S-32 |
| CE-90 | Registry transitions: DC-LEDGER-03 / DC-LEDGER-05 / DC-EPOCH-01 move `partial → enforced` with Phase 3 evidence | S-32 |
| CE-91 | All Phase 3 equivalence claims version-scoped to cardano-node 10.6.2; aiken UPLC pinned commit recorded in `ade_plutus/Cargo.toml` | S-32 |

**Phase 3 final gate**: when S-32 merges, CE-58 through CE-91 all
pass. Release tier (CE-79 Tier 3) is amended from "non-Plutus block
validation only" to "full block validation at cardano-node 10.6.2."

---

## Forbidden Patterns (Cluster-Level)

All Phase 2 forbidden patterns apply. Additionally:

- **No cost-model calibration.** Cost-model coefficients are
  parse-only inputs; fitting new coefficients is out of scope and
  forbidden in Phase 3 slices.
- **No uplc type leakage.** Pallas-originated types (from aiken UPLC's
  transitive deps) must not appear in `ade_ledger` public API.
  `ade_plutus` is the quarantine; the Ade-canonical surface it
  exposes uses Ade types only.
- **No script evaluation in BLUE beyond `ade_plutus`.** Other BLUE
  crates must not import evaluator entry points directly; all script
  execution flows through `ade_plutus` interfaces.
- **No speculative ScriptContext.** A ScriptContext is constructed
  once per `(tx, script_purpose, redeemer_index)` and consumed
  exactly once. No caching, no mutable accumulation across scripts.

---

## Failure Modes

Phase 3 inherits Phase 2's failure-mode philosophy: deterministic,
fail-fast, inspectable. New Phase 3-specific failure modes:

- **UPLC evaluation error** → `ScriptVerdict::Failed(Phase2Error)`;
  collateral consumed; outputs not produced
- **Budget exhaustion** → `ScriptVerdict::Failed(BudgetExhausted)`;
  phase-2 failure
- **ScriptContext derivation error** → `LedgerError` (not a script
  failure — this is a ledger-side invariant violation)
- **Missing cost model for PV** → `LedgerError` (invalid pparams)
- **BLS12-381 primitive failure (invalid point, etc.)** →
  `ScriptVerdict::Failed` inside evaluation, not a ledger error

---

## Proof Obligations Closed by Phase 3

Phase 2C closed obligations #9–#12. Phase 3 closes #13–#16 and
introduces #17–#20.

| # | Obligation | Slice | Verification |
|---|-----------|-------|-------------|
| 13 | State-backed Alonzo+ validation matches oracle for every non-Plutus check | S-27, S-28 | Verdict agreement on full mainnet corpus at all late-era boundaries |
| 14 | UPLC evaluation matches IOG reference at pinned plutus version | S-29 | IOG conformance suite zero divergence |
| 15 | Budget accounting matches oracle on conformance suite | S-30 | `.uplc.budget.expected` match |
| 16 | ScriptContext derivation matches oracle byte-for-byte | S-31 | CBOR match on mainnet Plutus txs |
| 17 | Phase-1 vs phase-2 failure classification matches oracle | S-32 | Verdict agreement on corpus failures |
| 18 | Collateral consumption matches oracle on phase-2 failures | S-32 | State-delta diff vs oracle |
| 19 | Multi-script budget aggregation matches oracle | S-32 | Budget-cap probe on multi-script txs |
| 20 | Conway governance-action script handling matches oracle | S-32 | Conway corpus verdict agreement |

---

## Phase 3 Completion Evidence

After S-32 merges, the following evidence package constitutes Phase 3 completion:

1. **CE-58 through CE-91**: all 34 exit criteria pass
2. **Registry updates**: DC-LEDGER-03 / DC-LEDGER-05 / DC-EPOCH-01
   `partial → enforced`; `ci_check_constitution_coverage.sh` passes
3. **Four-tier gate update** (CE-79 amendment): Tier 3 release scope
   lifted from "non-Plutus only" to "full mainnet validation at
   cardano-node 10.6.2"; Tier 4 non-goal removes "partial Plutus
   execution" (because execution is now full, not partial)
4. **Zero divergence**: mainnet Plutus corpus verdict agreement at
   9,436 / 9,436
5. **IOG conformance**: full suite passes byte-identically including
   `.uplc.budget.expected`
6. **ScriptContext**: mainnet txs produce byte-identical
   ScriptContext CBOR at every PV
7. **Dependency boundary**: `ade_plutus` is BLUE; pallas types do not
   appear in `ade_ledger` public API; `ci_check_dependency_boundary.sh`
   passes
8. **Version scoping**: all equivalence claims version-scoped to
   cardano-node 10.6.2; aiken commit pinned in `Cargo.toml`

---

## Inherited Specifications (from Phase 2)

The following specifications from Phase 2 apply to Phase 3 without
modification. They are NOT duplicated here — refer to the Phase 2C
cluster plan and `01_core_determinism_and_contract.md`:

- Authority surface (BLUE / GREEN / RED crate boundary)
- Compatibility boundary (`apply_block` contract)
- Byte authority contract (wire bytes for hash paths)
- Mixed-version proof obligations (version-scoped equivalence claims)
- Deny-attribute constraints (`#![deny(unsafe_code)]`, etc.)
- Forbidden patterns (no HashMap/HashSet, no SystemTime, no floats,
  no fs/net/async in BLUE)
- No-Mocks contract (deterministic substitutes only)

---

## Open Risks

- **Aiken UPLC divergence from IOG reference**: if aiken has patched
  or deviates from IOG semantics at the pinned commit, obligation
  O-29.1 surfaces this. Mitigation: the conformance-test discharge
  is gate for S-29 — if it fails, we pick a different aiken commit
  or fork.
- **Cost-model format drift**: IOG has changed cost-model encoding
  across eras. Mitigation: parse per-PV; obligation O-30.1 discharges
  the exact format at the pinned version.
- **Mainnet corpus doesn't cover all built-ins**: the 9,436 txs may
  not exercise every V3 BLS12-381 primitive. Mitigation: IOG
  conformance suite covers built-ins directly.
- **ScriptContext version drift**: V1/V2/V3 differ, and minor
  plutus-ledger-api version bumps can shift fields. Mitigation:
  version-scope everything to 10.6.2; obligation O-31.1 discharges
  deltas.

---

## Authority Reminder

This cluster plan is a planning and review aid only.

Correctness requirements are defined exclusively by:

- the project constitution (`ade_replay_first_constitutional_node_plan_v1.md` §2–§4b),
- `01_core_determinism_and_contract.md`,
- `classification_table.md`,
- and other normative specifications.

If there is ever a conflict:

> **Normative documents and CI enforcement are authoritative.**
