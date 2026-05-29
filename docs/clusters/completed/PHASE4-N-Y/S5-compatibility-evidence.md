# Invariant Slice — S5: Compatibility evidence bundle

## §2 Slice Header

- **Slice Name:** Observable-surface compatibility evidence — snapshot→tip differential vs Haskell, no fingerprint-equality, named fixtures, regression-per-mismatch; two-Haskell-node testnet (operator-witnessed).
- **Cluster:** PHASE4-N-Y.
- **Status:** Merged.
- **Cluster Exit Criteria Addressed** (verbatim):
  - [ ] **CE-Y-12.** Differential harness `sync_differential_snapshot_to_tip` passes vs the Haskell oracle on selected tip hash + per-block verdict + `query utxo`, fixture pinning oracle versions + reproducible inputs.
  - [ ] **CE-Y-13.** Gate `ci_check_no_haskell_fingerprint_equality.sh` — no test asserts Ade-ledger-fingerprint == a Haskell/cardano-node serialized state hash.
  - [ ] **CE-Y-14.** Each discovered mismatch is a named regression fixture under `corpus/sync/regressions/`.
  - [ ] **CE-Y-16** *(operator-witnessed).* Two-Haskell-node private Conway testnet + snapshot→tip live evidence per the `CN-OPERATOR-EVIDENCE-01` manifest pattern; `blocked_until_operator_pass_executed` until committed.
  - [ ] **CE-Y-15** *(partial):* `DC-COMPAT-01` + `RO-SYNC-EVIDENCE-01` introduced; `CN-OPERATOR-EVIDENCE-01` referenced.
- **Slice Dependencies:** S2 (forward-sync produces the synced chain to diff), S4 (Conway genesis for the two-node testnet).

## §3 Implementation Instruction (AI)

Build the differential harness on the **existing** `ade_testkit::harness::{block_diff, ledger_diff, diff_report}` adapters. Compare **observable** surfaces only — never Ade's internal `fingerprint` against a Haskell serialized-state hash. The two-node live capture is operator-witnessed; ship the manifest schema + a vacuously-satisfied CI gate (like `CN-OPERATOR-EVIDENCE-01`), not a live run. §12 is the contract.

## §4 Intent

Make it impossible to *claim* Cardano compatibility except via observable-surface agreement (verdicts, selected tip hash, block hashes, `query utxo`, transcripts) with pinned oracle versions and reproducible inputs — and impossible to substitute an Ade-internal-fingerprint == Haskell-serialization equality as proof.

## §5 Scope

- **GREEN:** `ade_testkit` differential harness `sync_differential_snapshot_to_tip` — drives Ade forward-sync over a captured snapshot→tip window and diffs per-block verdict + selected tip hash + `query utxo` result against committed oracle fixtures.
- **RED:** live evidence driver in `ade_core_interop` (operator-action; produces the manifest for CE-Y-16) + cardano-cli `query utxo` capture.
- **CI:** `ci_check_no_haskell_fingerprint_equality.sh` (negative grep), `ci_check_operator_evidence_manifest_schema.sh`-style schema gate for the sync-evidence manifest (vacuous until a manifest is committed).
- **Persistence/corpus:** `corpus/sync/regressions/*` (one entry per mismatch); fixtures pin `cardano_node_version` + `cardano_cli_version` + reproducible inputs.
- **Out of scope:** the sync/recovery/genesis machinery itself (S2–S4); changing any BLUE authority.

## §6 Execution Boundary (TCB color)

- **GREEN:** the differential harness (deterministic; reads committed fixtures; `ade_testkit`).
- **RED:** live evidence drivers, cardano-cli invocation, manifest writer (`ade_core_interop`).
- **BLUE:** none new (compares already-authoritative outputs; never decides authority — evidence reducers are GREEN per [[feedback-evidence-reducers-are-green-not-authority]]).

Color resolved.

## §7 Invariants Preserved

[[CN-OPERATOR-EVIDENCE-01]] (manifest schema + sha256 cross-check pattern), [[T-DET-01]] (the harness is deterministic over fixtures), [[DC-CONS-20]] (admission unchanged), and the cluster's observable-surface rule. Evidence is GREEN — it never alters authoritative outputs ([[feedback-evidence-reducers-are-green-not-authority]], [[feedback-shell-must-not-overstate-semantic-truth]]).

## §8 Invariants Strengthened or Introduced

**One family — compatibility evidence:**
- **Introduces `DC-COMPAT-01`** — Cardano compatibility is proven only on observable surfaces (verdicts, selected tip hash, block hashes, `query utxo`, transcripts); asserting Ade-ledger-fingerprint == a Haskell serialized-state hash is forbidden and CI-blocked.
- **Introduces `RO-SYNC-EVIDENCE-01`** (release) — a committed snapshot→tip evidence manifest carries the closed schema (oracle versions, chain point, fixture refs, sha256, acceptance/diff result); operator-witnessed for the two-node live leg.
- References [[CN-OPERATOR-EVIDENCE-01]] (reuses its manifest+sha256 enforcement shape).

## §9 Design Summary

`sync_differential_snapshot_to_tip` replays Ade forward-sync over `corpus/sync/preprod_snapshot_to_tip_*` and, for each block, compares Ade's verdict + the selected tip hash + block hash to the committed oracle (captured from cardano-node) and the post-window `query utxo` to a committed cardano-cli dump. A divergence emits a `diff_report` and the case is committed under `corpus/sync/regressions/`. `ci_check_no_haskell_fingerprint_equality.sh` greps the test tree for any `fingerprint(...) == <haskell/serialized-state-hash>` assertion and fails. The two-node live leg (CE-Y-16) is captured by an operator and committed as a manifest; the schema gate is vacuous until then.

## §10 Changes Introduced

- **Types:** closed sync-evidence manifest schema (TOML, mirrors `CN-OPERATOR-EVIDENCE-01`); `diff_report` reuse.
- **State transitions:** none (harness is read-only over fixtures).
- **Persistence:** `corpus/sync/regressions/*`; the evidence manifest.
- **Removal/refactors:** none.

## §11 Replay / Crash / Epoch Validation

- **Replay:** `sync_differential_snapshot_to_tip` is deterministic over committed fixtures (re-running yields the same diff verdict).
- **Crash/restart:** n/a (evidence harness).
- **Epoch boundary:** the differential window is single-epoch (matches S2's sync window).

## §12 Mechanical Acceptance Criteria

- [ ] `sync_differential_snapshot_to_tip` — per-block verdict + selected tip hash + block hash + `query utxo` agree with the committed oracle fixtures (pinning `cardano_node_version`/`cardano_cli_version`).
- [ ] `ci/ci_check_no_haskell_fingerprint_equality.sh` — no test asserts Ade-fingerprint == a Haskell/serialized-state hash.
- [ ] `sync_evidence_manifest_schema_or_vacuous` — committed sync-evidence manifest conforms to the closed schema + sha256 matches; vacuously passes when none committed (mirrors `ci_check_operator_evidence_manifest_schema.sh`).
- [ ] `regression_fixture_per_mismatch` — any committed `corpus/sync/regressions/*` entry is schema-valid and re-runs deterministically.
- [ ] `cargo test --workspace` clean.

## §13 Failure Modes

A differential mismatch → the harness fails with a `diff_report` (deterministic) and the case must be committed as a regression fixture before close. A malformed evidence manifest → schema gate fails. No authoritative-state impact (evidence is GREEN/RED, never BLUE).

## §14 Hard Prohibitions

**Inherited (cluster §7).** **Slice-specific:** no Ade-fingerprint == Haskell-serialization equality assertion (CE-Y-13); no overstating wire/fetch success as acceptance ([[feedback-shell-must-not-overstate-semantic-truth]]); no evidence reducer deciding authority; no closing CE-Y-16 in CI (operator-witnessed); no unpinned oracle versions in a committed fixture.

## §15 Explicit Non-Goals

No change to sync/recovery/genesis machinery (S2–S4), no new BLUE authority, no live run executed in CI, no performance claims, no full N2N/N2C coverage (sibling cluster).

## §16 Completion Checklist

- [ ] Differential harness deterministic over pinned fixtures; no-fingerprint-equality gate enforced; manifest schema gate present (vacuous until operator commit); regression-per-mismatch machinery in place.

## §17 Review Notes

Risk: a fingerprint-equality shortcut creeping into a test → CE-Y-13 grep gate. Risk: presenting fetch/connect as acceptance → observable-surface + operator-log discipline. CE-Y-16 stays `blocked_until_operator_pass_executed` at cluster close (honest — the schema is enforced, the live capture is operator action).

## §18 Authority Reminder

Planning aid only; registry + CI authoritative.
