# SLICE: ADE-TESTKIT-EPOCH-BOUNDARY-HANG / S1 — investigate + watchdog the hanging epoch test

## Status: OPEN — release / CI hygiene blocker

A test in `crates/ade_testkit` hangs indefinitely, so `cargo test -p ade_testkit` (and any
`cargo test --workspace`) never completes. **The full ade_testkit suite is therefore not currently a
reliable completion signal** and must not gate CI as-is.

## Established facts (mechanically evidenced 2026-06-23)

- **Hanging test:** `all_epoch_boundaries_fire` in
  `crates/ade_testkit/tests/epoch_boundary_logic.rs`. With `--test-threads=1` the harness prints
  `test all_epoch_boundaries_fire ...` and never returns; `timeout` kills it (exit 124). The
  preceding tests pass (`shelley_epoch_boundary_fires`, `reward_arithmetic_verification`,
  `allegra_epoch_boundary_summary_comparison`).
- **Pre-existing, NOT a regression:** it hangs at the parent commit (HEAD `6cab0d6c`) with the
  LEDGER-VALUE-QUANTITY-CORRECTNESS changes stashed out —
  `timeout 240 cargo test -p ade_testkit --test epoch_boundary_logic -- --test-threads=1` → exit 124.
  So it is independent of `DC-LEDGER-VALUE-01`.
- **Not the byte-identity replay:** the six `differential_*` replay binaries pass in ~4s. A separate
  slow (not hanging) test is `contiguous_plutus_verdict_harness` (~141s).

## Investigation to do

1. **Reproduce at a known baseline** — confirmed hanging at `6cab0d6c`. Bisect earlier to find the
   commit where `all_epoch_boundaries_fire` began hanging (or whether it ever completed).
2. **Root-cause** the hang in `all_epoch_boundaries_fire` — infinite loop, blocking wait, or an
   unbounded computation — by reading the test and the epoch-boundary code it drives.
3. **Add a deterministic timeout / watchdog in CI** so a hung test fails fast with a clear signal
   instead of wedging the suite (per-test timeout wrapper around `cargo test -p ade_testkit`, and/or
   quarantine the test behind an opt-in feature with a tracking reference).
4. Until fixed, **CI gates on the targeted suites** (`-p ade_types -p ade_ledger`, the `differential_*`
   replay binaries, the per-invariant `ci_check_*` gates) — never a full `cargo test -p ade_testkit`
   completing.

## Acceptance

- [ ] root cause of the `all_epoch_boundaries_fire` hang identified and documented;
- [ ] the test fixed (completes) or bounded by a deterministic per-test timeout that fails fast;
- [ ] a CI watchdog guarantees no single test can wedge the suite;
- [ ] `cargo test -p ade_testkit` completes deterministically (or the hanging test is explicitly
      quarantined with a tracking reference).

## Scope fence

Test-harness / CI hygiene only. This slice does NOT change ledger authority.
