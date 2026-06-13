#!/usr/bin/env bash
# ci_check_plutus_oracle_no_false_accept.sh -- DC-LEDGER-03 (Plutus oracle false-accept guard).
#
# The contiguous Plutus verdict harness cross-checks Ade's per-tx Plutus verdict
# against the chain's own phase-2 classification (each block's invalid_transactions
# field -- the Haskell ledger oracle). The load-bearing protection is the
# ZERO-TOLERANCE assertion `diverge_pass == 0`: Ade must NEVER report PASS for a tx
# the chain rejected as phase-2-invalid (a false-accept). This gate locks that
# assertion and keeps the smoke test running, so the Plutus false-accept gate can't
# be silently removed or weakened.
#
# SCOPE NOTE: this guards the ASSERTION, not its current reach. The contiguous
# replay presently stops early on a cert-state divergence, so the smoke reaches few
# passing Plutus txs; widening that reach is separate work. DC-LEDGER-03 stays
# `partial` -- this does not flip it.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
H="$REPO_ROOT/crates/ade_testkit/tests/contiguous_plutus_verdict_harness.rs"
fail() { echo "FAIL (ci_check_plutus_oracle_no_false_accept): $1" >&2; exit 1; }

# Does a harness source carry the zero-tolerance false-accept assertion?
has_no_false_accept_gate() {  # $1 = file
  local s; s="$(sed -E 's://.*$::' "$1")"
  grep -Eq 'oracle_diverge_pass' <<< "$s" \
    || { echo "  $1: no oracle_diverge_pass counter (our-PASS vs chain-INVALID)"; return 1; }
  # `assert_eq!(diverge_pass, 0, ...)` -- the macro call may wrap across lines, so
  # match the distinctive `diverge_pass, 0` argument rather than the macro head.
  grep -Eq 'diverge_pass,[[:space:]]*0\b' <<< "$s" \
    || { echo "  $1: missing the diverge_pass == 0 zero-tolerance assertion"; return 1; }
  return 0
}

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
  # A harness that counts diverge_pass but FORGOT to assert it is zero.
  printf 'let diverge_pass = t.oracle_diverge_pass;\nif diverge_pass > 0 { eprintln!("fyi"); }\n' > "$tmp"
  if has_no_false_accept_gate "$tmp"; then
    echo "FAIL: scanner missed a harness with no diverge_pass==0 assertion" >&2; exit 1
  fi
  echo "PASS: scanner detects a missing false-accept assertion"; exit 0
fi

[ -f "$H" ] || fail "missing $H"
has_no_false_accept_gate "$H" \
  || fail "Plutus oracle false-accept assertion (diverge_pass == 0) is missing or weakened"

# Smoke test present and actually runs (not #[ignore]'d).
grep -Eq 'fn plutus_era_contiguous_smoke' "$H" || fail "missing plutus_era_contiguous_smoke"
ctx="$(grep -B3 'fn plutus_era_contiguous_smoke' "$H" | sed -E 's://.*$::' || true)"
if grep -Eq '#\[ignore' <<< "$ctx"; then
  fail "plutus_era_contiguous_smoke is #[ignore]'d -- the oracle gate must run in CI"
fi

echo "OK: Plutus verdict oracle keeps its zero-tolerance false-accept assertion (diverge_pass == 0) \
and the smoke runs (guards the assertion, not its reach; DC-LEDGER-03 stays partial)"
