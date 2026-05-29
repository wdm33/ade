# `corpus/sync/regressions`

One committed fixture per discovered snapshot→tip **mismatch**
(PHASE4-N-Y S5, CE-Y-14 / DC-COMPAT-01).

## Convention

Each mismatch found by the differential harness
(`ade_testkit::harness::sync_diff::diff_observable_surfaces`) is
committed here as a single `REG-SYNC-NNN.toml` file matching the
`SyncRegressionFixture` closed schema:

```toml
regression_id = "REG-SYNC-001"
status        = "open"   # or "fixed"

[oracle]
cardano_node_version = "11.0.1"   # MUST be pinned
cardano_cli_version  = "10.1.1.0" # MUST be pinned
network              = "preprod"
fixture_kind         = "synthetic"
start_point          = "..."
end_point            = "..."
# [[oracle.blocks]] / [[oracle.post_window_utxo]] entries follow

[ade_observed]
# [[ade_observed.blocks]] / [[ade_observed.post_window_utxo]] entries follow
```

## Rules

- **Observable surfaces only.** A regression fixture records the
  observable surfaces that diverged (verdict, block hash, selected
  tip hash, `query utxo` set) — never an Ade-internal ledger
  `fingerprint` and never a Haskell serialized-state hash
  (DC-COMPAT-01, CI-blocked by
  `ci/ci_check_no_haskell_fingerprint_equality.sh`).
- **Pinned oracle versions.** Both `cardano_node_version` and
  `cardano_cli_version` must be non-empty.
- **Closed `status`.** Either `open` (still diverges) or `fixed`
  (now agrees). The test
  `ade_testkit::harness::sync_diff::tests::regression_fixture_per_mismatch`
  re-runs each entry deterministically and checks the recorded
  `status` matches the diff outcome.

## Current state

No regression fixture is committed yet — the harness has found no
mismatch. The `regression_fixture_per_mismatch` test is therefore
**vacuously satisfied**. The first discovered mismatch lands here as
`REG-SYNC-001.toml`.
