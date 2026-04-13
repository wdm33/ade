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

## Exit Criteria Matrix (CE-83 through CE-91)

| CE | Claim | State | Blocker |
|---|---|---|---|
| CE-83 | Collateral rules match oracle across Alonzo/Babbage/Conway corpus | **Closed** | — |
| CE-84 | Input & reference-input resolution + network-ID + datum-hash binding | **Closed** | — |
| CE-85 | IOG UPLC conformance suite passes (`.uplc.expected` match) | **Closed** | — |
| CE-86 | Budget accounting matches oracle; mainnet-pparams cost models | **Partial** | Cost-model plumbing from snapshot pparams not wired to eval call; eval currently uses aiken defaults |
| CE-87 | ScriptContext byte-identical to oracle for every mainnet Plutus tx | **Delegated** | Delegated to aiken's impl (passes IOG conformance). Direct Haskell diff requires ShadowBox-style pipeline, explicitly out of scope per plan. |
| CE-88 | 9,436 mainnet Plutus tx verdict agreement with oracle | **Not closed** | Infrastructure blocker: 20-block boundary windows can't resolve historical Plutus predecessors. Requires contiguous-corpus replay harness or paired snapshot+block fixtures. |
| CE-89 | `ScriptVerdict::NotYetEvaluated` eliminated from verdict paths | **Partial** | Enum variant still exists; `plutus_deferred_count` path still used. Will converge once CE-88 harness runs to completion. |
| CE-90 | DC-LEDGER-03 / DC-LEDGER-05 / DC-EPOCH-01 move `partial → enforced` | **Not closed** | Pure admin: TOML updates + `ci_check_constitution_coverage.sh`. Trigger: do at Tier 3 ship. |
| CE-91 | All Phase 3 equivalence claims version-scoped; aiken commit pinned | **Closed** | Cargo.toml pins v1.1.21. |

### Closure tallies

- **Fully closed**: CE-83, CE-84, CE-85, CE-91 (4/9)
- **Partial / Delegated**: CE-86, CE-87, CE-89 (3/9)
- **Not closed**: CE-88, CE-90 (2/9)

---

## Slice Delivery

| Slice | Cluster | State | Evidence |
|---|---|---|---|
| S-27 | P-A | Closed | Collateral + input resolution composer (commits pre-session) |
| S-28 | P-A | Closed | Ref-input + datum-hash + required-signers composer |
| S-29 | P-B | Closed | `ade_plutus` crate, aiken UPLC port, 6,899/6,899 mainnet script decode+round-trip |
| S-30 | P-B | Closed | 514 IOG conformance passes, 0 budget mismatches |
| S-31 | P-C | Partial | Aiken delegation accepted (CE-87 delegated); `eval_tx_phase_two` wrapper complete. Haskell byte-diff not performed. |
| S-32 | P-D | Partial | 6/7 discharge items delivered. Missing: `BlockApplyResult` per-tx verdict surface (S-32 discharge item 7). |

---

## What Actually Got Built This Phase

### Engineering (~90% complete)

- `ade_plutus` BLUE quarantine: UPLC evaluator, cost-model parser,
  ScriptContext delegated to aiken. Pallas/aiken types do not cross
  the crate boundary (CI-enforced).
- `ade_ledger::late_era_validation` composer surface: 8 check
  functions covering all non-Plutus Alonzo+ rules.
- Per-era composers wired into `apply_shelley_era_block_classified`
  when `track_utxo=true`. Runs on Alonzo/Babbage/Conway blocks with
  zero spurious rejections across 2,400 real mainnet txs (6 boundary
  sets).
- `TxOut::AlonzoPlus { raw, address, coin }` preserves byte-identical
  output wire form so Plutus eval sees datum/script_ref/multi_asset.
- `run_phase_one_composers` calls `ade_plutus::eval_tx_phase_two` for
  Plutus-bearing txs; outcome counters on `BlockVerdict`
  (`plutus_eval_passed/failed/ineligible`).
- Tx-body encoders for Alonzo/Babbage/Conway (canonical map form).
- End-to-end proof: aiken's `test_eval_0` fixture runs through our
  wire-in to Ok with `mem=747528` (matches aiken oracle byte-for-byte).

### Formal-claim closure (~60%)

4 of 9 exit criteria formally closed. The remaining 5 are either
infrastructure-blocked (CE-88), admin-blocked (CE-90), partial-
cleanup (CE-89), wiring gaps (CE-86), or explicitly delegated
(CE-87).

---

## Revisit Triggers

Reopen Phase 3 work when:

1. **Tier 3 release gate** — CE-88 is the go/no-go for lifting the
   "non-Plutus only" release tier. Full closure required.
2. **Contiguous-corpus replay landed** — if/when the
   `corpus/contiguous/{alonzo,babbage,conway}/` replay harness
   materializes, CE-88/CE-89 drop from weeks to days.
3. **Oracle verdict dataset published** — if IOG/aiken publishes a
   verdict-reference dataset for mainnet Plutus txs, the diff
   harness (#5 / #7 in the priority chart) becomes a weekend task.
4. **ShadowBox-style pipeline** — if a Haskell-diff infrastructure
   arrives, CE-87 flips from delegated to proven. Explicitly out of
   scope per the plan until then.

---

## Known Gaps To Address Before Full Closure

1. **Cost-model wiring** (CE-86 remainder): plumb
   `pparams.cost_models` → `eval_tx_phase_two(... Some(cbor) ...)`.
   Affects budget accuracy, not pass/fail verdicts on simple
   validators. Cost-model parser already exists; just needs the plumb.
2. **Contiguous-corpus harness** (CE-88): replay
   `corpus/contiguous/*` starting from the matching pre-era
   snapshots, count Eval-ok / Eval-failed / Ineligible per era.
3. **`NotYetEvaluated` drop** (CE-89): once the harness produces real
   pass/fail verdicts, the enum variant can be retired and the
   `plutus_deferred_count` code path replaced.
4. **`BlockApplyResult` surface** (S-32 item 7): `apply_block`
   currently returns `(LedgerState, BlockVerdict)`. The diff harness
   needs per-tx verdicts as values. Promised by the S-32 discharge
   doc, not built.

---

## Session Commits (chronological)

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

---

## Test Totals As Of This Snapshot

- `ade_ledger` lib: **315** (unit) + 4 (`plutus_eval`)
- `ade_codec` lib: **86** (+9 body round-trip)
- `ade_plutus` lib: **20** + 7 integration (6 compat + 1 end-to-end)
- `ade_testkit` integrations: **133** including 9 composer-integration
  tests (120 blocks, 2,400 txs, 0 composer rejections; 72 Plutus
  eval attempts on 20-block boundary windows, all Ineligible by
  design — predecessors predate the window)

---

## Authority Reminder

This document is descriptive, not prescriptive. The exit-criteria
text in `docs/active/phase_3_cluster_plan.md` is authoritative for
what counts as closure; `constitution_registry.toml` is
authoritative for which CEs are marked closed. If either of those
conflicts with this snapshot, they win.
