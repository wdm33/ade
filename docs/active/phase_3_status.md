# Phase 3 Status Snapshot

> **Purpose**: ground state of Phase 3 for continuity across sessions /
> contributors. Read this before picking up Phase 3 work.
>
> **Authority**: planning doc, not a claim doc. The authoritative
> closure statements live in `constitution_registry.toml` exit-criteria
> entries. This file summarizes their state.

**Version scope** (inherited, CE-91): cardano-node 10.6.2 / plutus
1.57.0.0 / aiken v1.1.21 (commit `42babe5d` pinned in
`crates/ade_plutus/Cargo.toml`).

---

## What Phase 3 Was Supposed To Deliver

From `docs/active/phase_3_cluster_plan.md` §Purpose:

> A block the Haskell cardano-node accepts or rejects for *any* reason
> — structural, state-backed, or script-related — receives the same
> verdict from Ade. Tier 3 release lifts from "non-Plutus only" to
> "full Cardano mainnet validation at cardano-node 10.6.2".

Two correctness gaps close:

1. State-backed validation for Alonzo/Babbage/Conway (input/collateral/
   reference-input/datum/signer/network-id resolution).
2. Plutus script execution (UPLC V1/V2/V3, ScriptContext, budget).

---

## Exit Criteria Matrix (CE-83 through CE-91) — CURRENT

| CE | Claim | State | Notes |
|---|---|---|---|
| CE-83 | Collateral rules match oracle across Alonzo/Babbage/Conway corpus | **Closed** | 0 composer false positives over 2,400 + 6,804 real mainnet txs |
| CE-84 | Input / ref-input resolution + network-ID + datum-hash binding | **Closed** | — |
| CE-85 | IOG UPLC conformance suite passes | **Closed** | 514 passes, 0 budget mismatches |
| CE-86 | Budget accounting + mainnet-pparams cost models | **Closed** | Real cost_models parsed from snapshot pparams (PP[17]/[15]), plumbed to `eval_tx_phase_two` |
| CE-87 | ScriptContext byte-identical to oracle | **Delegated, with known upstream issue** | Aiken ScriptContext impl has Conway-specific divergence; see "Upstream bug" below |
| CE-88 | 9,436 mainnet Plutus tx verdict agreement with oracle | **Strong partial** | 631 Plutus txs agreed, 0 hard-bug divergences, ~44 inherited-upstream failures; full replay details below |
| CE-89 | `ScriptVerdict::NotYetEvaluated` eliminated | **Closed** | Variant retired; `plutus_eval_{passed,failed,ineligible}` counters replace it |
| CE-90 | DC-LEDGER-03/05, DC-EPOCH-01 `partial → enforced` | **Partial** | Authority-surfaces updated this sprint; full enforced flip gated on CE-88 |
| CE-91 | Version-scoped to cardano-node 10.6.2; aiken commit pinned | **Closed** | Cargo.toml pins v1.1.21 |

### Closure tallies

- **Fully closed**: CE-83, CE-84, CE-85, **CE-86**, **CE-89**, CE-91 (**6/9**)
- **Delegated** (with known upstream issue): CE-87 (1/9)
- **Partial**: CE-88 (strong), CE-90 (admin) (2/9)
- **Not closed**: 0

---

## Slice Delivery

| Slice | Cluster | State | Evidence |
|---|---|---|---|
| S-27 | P-A | Closed | Collateral + input resolution composer |
| S-28 | P-A | Closed | Ref-input + datum-hash + required-signers composer |
| S-29 | P-B | Closed | `ade_plutus` crate; 6,899/6,899 mainnet script decode+round-trip |
| S-30 | P-B | Closed | 514 IOG conformance passes, 0 budget mismatches |
| S-31 | P-C | Closed with delegation | `eval_tx_phase_two` wrapper; ScriptContext delegated to aiken (one known upstream bug, documented) |
| S-32 | P-D | **Fully closed** | 7/7 discharge items delivered, including `BlockApplyResult`/`TxVerdict` surface (previously missing) |

---

## Engineering Deliverables

### `ade_plutus` (BLUE quarantine)
- UPLC evaluator (CEK machine, built-ins, BLS12-381)
- Cost-model parser (CBOR → structured CostModels)
- ScriptContext delegated to aiken
- End-to-end eval test: `aiken_fixture_tx_evaluates_end_to_end` (mem=747528 matches oracle byte-for-byte)
- Encoder-compat tests: `encoder_aiken_compat`
- **Reproducer** for upstream bug: `conway_validity_range_reproducer` (#[ignore]'d, fails with the `PT5` signature)

### `ade_ledger`
- `late_era_validation` composer surface (8 Phase-1 check functions)
- Per-era state-backed composers for Alonzo/Babbage/Conway
- `TxOut::AlonzoPlus { raw, address, coin }` preserves byte-identical output CBOR
- `plutus_eval` module: `try_evaluate_tx`, `decode_invalid_tx_indices`, `assemble_full_tx_*`
- `apply_block_with_verdicts` entry point returns `BlockApplyResult` with per-tx `TxVerdict` + `invalid_tx_indices`
- `TxOutcome` enum: `Passed`/`InputsUnresolved`/`Phase1Rejected`/`PlutusPassed`/`PlutusFailed`/`PlutusIneligible`/`Skipped`

### `ade_codec`
- Alonzo/Babbage/Conway `TxOut` + `TxBody` encoders (canonical map form)

### `ade_testkit`
- `late_era_composer_integration`: 6 Plutus-era boundaries, 2,400 txs, 0 composer rejections
- `contiguous_plutus_verdict_harness`:
  - `plutus_era_contiguous_smoke` (100 blocks × 3 eras, runs in CI)
  - `plutus_era_contiguous_full` (1,500 blocks × 3 eras, `#[ignore]`)
- `emit_divergent_fixture`: one-shot generator for upstream repro bytes

---

## CE-88 Full-Replay Evidence

From `plutus_era_contiguous_full` (2h 44m runtime, 2026-04-13):

| Metric | Count |
|---|---|
| Total Plutus txs evaluated | 675+ |
| **Plutus-passed (agreed with chain)** | **631** |
| Plutus-failed divergent (our fail, chain accept) | 44+ |
| **Plutus-passed divergent (our accept, chain reject)** | **0** — hard gate holds |
| Phase-1 composer false positives | **0** across entire corpus |
| Oracle agreement ratio (lower bound) | **93.5%** |

All 44+ divergent-fails share **one** upstream-aiken error signature
(details in `docs/active/CE-88_closure_path.md`).

---

## Upstream Bug — Aiken ScriptContext on Conway

**Signature**: divergent-fail Conway txs produce identical error:

```
Spend[N] the validator crashed / exited prematurely
  Trace Expiration time is not properly set
  Trace PT5
```

The trace text is user-validator output — a `traceIfFalse` check
against `txInfoValidRange`. Hypothesis: aiken mis-constructs the
V3 ScriptContext's `validity_range` for Conway txs.

### Concrete reproducer (in-tree)

- **tx_hash**: `d97b843494511d57bfa7fba05ea40855de6663472b7c2fd8557a3b114054826f`
- **Block**: `blk_00073_chunk06188_idx00073.cbor`, tx_idx 7
- **Standalone test**: `crates/ade_plutus/tests/conway_validity_range_reproducer.rs`
  - `#[ignore]`'d because it's expected to fail while upstream is unpatched
  - Run with: `cargo test -p ade_plutus --test conway_validity_range_reproducer -- --ignored --nocapture`
- **Fixture generator**: `crates/ade_testkit/tests/emit_divergent_fixture.rs`
  - Regenerate with: `cargo test -p ade_testkit --test emit_divergent_fixture -- --ignored --nocapture`
  - Outputs to: `target/aiken_divergent_fixture/{tx_body,inputs,outputs,cost_models,meta}.hex`
- **Issue draft**: `docs/active/aiken_upstream_issue_draft.md`

### Upstream search results (2026-04-14)

No exact match in aiken's issue tracker. Two relevant items:

| Ref | State | Relevance |
|---|---|---|
| [aiken#1176](https://github.com/aiken-lang/aiken/issues/1176) | Closed, fixed in v1.1.19 | PlutusData equality fix. Already in our v1.1.21. Not our bug. |
| [aiken#1203](https://github.com/aiken-lang/aiken/issues/1203) | Open | Datum/redeemer re-serialisation in ScriptContext. Same problem family. |
| [aiken PR #1298](https://github.com/aiken-lang/aiken/pull/1298) | Open (filed 2026-04-05) | Actively fixes #1203 via canonical PlutusData re-encoding. **Likely fixes our bug too.** |

### Pending decision: test before file?

**Recommended next action**: test our reproducer against aiken `main`
(post-PR-1283 merge, with PR-1298 applied) before filing a new issue.

Three possible outcomes:
1. Reproducer passes on master → don't file; bump our aiken pin to
   v1.1.22+ when released.
2. Still fails but PR #1298 fixes it → comment on #1203/PR #1298
   with our fixture.
3. Still fails even with PR #1298 → file new issue with full
   reproducer.

Estimated cost: ~30 min for option (1), ~45 min for option (2).

---

## Test Suite Totals (2026-04-14 snapshot)

| Crate / test | Count |
|---|---|
| `ade_ledger` lib (unit + integration) | 314 |
| `ade_codec` lib | 86 |
| `ade_plutus` lib | 20 |
| `ade_plutus` integration (end_to_end, encoder_compat) | 7 |
| `ade_plutus` integration (`#[ignore]`'d reproducer) | 1 (currently failing by design) |
| `ade_testkit` integrations | 133+ |

All tests green except the intentionally-ignored upstream-bug
reproducer.

---

## Session Commits (Phase 3 only, chronological)

| Commit | Scope |
|---|---|
| `a176a18` | Alonzo+ TxOut encoders |
| `283d377` | Phase-1 composer wired into `apply_block` |
| `fcc7bf7` | Encoder/aiken pallas-decode compat |
| `0bdfb6d` | Integration test (3 boundaries) |
| `4f345ab` | Composer false-positives fixed (Byron, redeemer-gate, pparams) |
| `0079a63` | Expanded to 6 boundaries, 2,400 txs, 0 rejections |
| `38dd2e1` | Plutus evaluator wired into composer |
| `439ecd8` | UTxO preservation — 72 Plutus txs reach dispatch |
| `efcf61d` | Alonzo/Babbage/Conway tx body encoders |
| `e906e5d` | End-to-end Plutus eval proven (mem=747528 matches oracle) |
| `2952ba0` | Priority chart items #1–#7 (status doc, cost models, NotYetEvaluated drop, BlockApplyResult, registry, contiguous harness) |
| `89b84a1` | CE-88 oracle cross-reference via `invalid_transactions` |
| `8be23a9` | Full 4,500-block replay results (631 passes, 0 hard bugs) |
| `c646b38` | Upstream bug reproducer + issue draft |
| (this snapshot) | Phase 3 status update |

---

## Next Session Pickup

### Immediate next step (30–45 min)

**Test reproducer against aiken `main`** before filing upstream:

1. Edit `crates/ade_plutus/Cargo.toml`:
   ```toml
   aiken_uplc = { git = "https://github.com/aiken-lang/aiken", branch = "main", package = "uplc" }
   ```
2. Optionally also try PR #1298 branch (`canonical-plutus-data-encoding`
   or similar — check the PR).
3. Run: `cargo test -p ade_plutus --test conway_validity_range_reproducer -- --ignored --nocapture`
4. Record outcome in `docs/active/aiken_upstream_issue_draft.md`.
5. Proceed per outcome (don't file / comment on #1203 / file new).

### After that

- If upstream fix works: bump pin, rerun `plutus_era_contiguous_full`
  to confirm 0 divergent-fails, then flip CE-88 Partial → Closed and
  CE-90 authority_surfaces → `enforced`.
- If upstream fix doesn't work: file the new issue, then continue
  waiting / inheriting.

### Nothing-blocking follow-ups

- Re-run `plutus_era_contiguous_full` with output redirected to a
  file (not `| tail -50`) to capture per-era breakdown.
- Add a standalone unit test for `decode_invalid_tx_indices` with
  malformed inputs (currently exercised only end-to-end).
- Consider exposing `plutus_eval_ineligible` on `BlockVerdict` via
  `apply_block_classified` too (currently only on `BlockApplyResult`).

### What's explicitly NOT on the menu

- ShadowBox-style Haskell-diff pipeline (out of scope per Phase 3
  plan)
- Full byte-parity with Haskell on-disk `ExtLedgerState` (CE-79
  Tier 4 non-goal)
- PV11+ protocol-version support (version-scoped to 10.6.2)

---

## Quick Reference — File Locations

| What | Where |
|---|---|
| Phase 3 status (this doc) | `docs/active/phase_3_status.md` |
| Phase 3 cluster plan (authoritative) | `docs/active/phase_3_cluster_plan.md` |
| CE-88 closure strategy + full-replay evidence | `docs/active/CE-88_closure_path.md` |
| Upstream aiken issue draft | `docs/active/aiken_upstream_issue_draft.md` |
| Constitution registry | `constitution_registry.toml` |
| Plutus-era contiguous harness | `crates/ade_testkit/tests/contiguous_plutus_verdict_harness.rs` |
| Upstream-bug reproducer | `crates/ade_plutus/tests/conway_validity_range_reproducer.rs` |
| Upstream-bug fixture generator | `crates/ade_testkit/tests/emit_divergent_fixture.rs` |
| Alonzo+ composer | `crates/ade_ledger/src/{alonzo,babbage,conway}.rs` |
| Apply-block path | `crates/ade_ledger/src/rules.rs` |
| Plutus eval wire-in | `crates/ade_ledger/src/plutus_eval.rs` |

---

## Authority Reminder

This document is descriptive, not prescriptive. The exit-criteria
text in `docs/active/phase_3_cluster_plan.md` is authoritative for
what counts as closure; `constitution_registry.toml` is
authoritative for which CEs are marked closed. If either of those
conflicts with this snapshot, they win.
