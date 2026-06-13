#!/usr/bin/env bash
# ci_check_plutus_budget_cap.sh -- per-script Plutus ex_units cap (false-accept guard).
#
# cardano-ledger phase-2 caps each Plutus script at the ex_units DECLARED in its
# redeemer; a script that overruns its declared budget fails (ValidationTagMismatch,
# collateral consumed). aiken's eval_phase_two_raw caps only at the tx-wide
# `initial_budget` Ade passes (the protocol max) and never enforces the smaller
# per-script declared cap -- so a tx that UNDER-DECLARES ex_units (a script that
# consumes more than it declared, while the tx total stays under the global cap)
# would be ACCEPTED by Ade but REJECTED by cardano-node: a false-accept.
#
# This gate locks the per-script enforcement STRUCTURALLY:
#   1. ade_plutus derives each script's DECLARED cap from the tx
#      (declared_ex_units_by_pointer) and binds PerScriptResult.success to the
#      actual<=declared comparison -- never a hardcoded `true`.
#   2. the ledger consumer REJECTS (PlutusEvalOutcome::Failed) when any script !success.
#   3. the adversarial regression test exists and actually runs (not #[ignore]'d).
# Behavioral half = under_declared_ex_units_must_reject + the tx_eval parser unit tests.
#
# SCOPE NOTE: this closes the per-script budget-cap false-accept. It does NOT flip
# CN-PLUTUS-02 (the closed failure-shape classification -- budget vs context vs exec --
# is not yet wired through to the verdict) nor DC-LEDGER-03 (still partial). It guards
# the fix from silent regression.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TXE="$REPO_ROOT/crates/ade_plutus/src/tx_eval.rs"
PE="$REPO_ROOT/crates/ade_ledger/src/plutus_eval.rs"
TEST="$REPO_ROOT/crates/ade_plutus/tests/end_to_end_plutus_eval.rs"
fail() { echo "FAIL (ci_check_plutus_budget_cap): $1" >&2; exit 1; }

# Does an ade_plutus evaluator source enforce the per-script declared cap?
# Returns 0 (enforced) / 1 (not enforced) for the file passed as $1.
enforces_declared_cap() {
  local s; s="$(sed -E 's://.*$::' "$1")"   # strip line comments
  # the declared cap must be DERIVED from the tx ...
  grep -Eq 'declared_ex_units_by_pointer' <<< "$s" \
    || { echo "  $1: declared cap not derived from the tx"; return 1; }
  # ... COMPARED against the measured ex_units (actual <= declared) on both axes ...
  grep -Eq '\.mem[[:space:]]*<=' <<< "$s" \
    || { echo "  $1: no actual-mem <= declared-mem comparison"; return 1; }
  grep -Eq '\.cpu[[:space:]]*<=' <<< "$s" \
    || { echo "  $1: no actual-cpu <= declared-cpu comparison"; return 1; }
  # ... and per-script success must come from that comparison, never hardcoded.
  grep -Eq 'success:[[:space:]]*within_declared' <<< "$s" \
    || { echo "  $1: success not bound to the declared-cap result"; return 1; }
  return 0
}

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
  # The ORIGINAL bug: appears to consult the cap but hardcodes success = true.
  {
    printf 'let declared = declared_ex_units_by_pointer(tx_cbor);\n'
    printf 'if f.mem <= d.mem && f.cpu <= d.cpu {}\n'
    printf 'scripts.push(PerScriptResult { success: true, cpu: f.cpu, mem: f.mem });\n'
  } > "$tmp"
  if enforces_declared_cap "$tmp"; then
    echo "FAIL: scanner missed a hardcoded-pass (unenforced) evaluator" >&2; exit 1
  fi
  echo "PASS: scanner detects an unenforced per-script budget cap"; exit 0
fi

for f in "$TXE" "$PE" "$TEST"; do [ -f "$f" ] || fail "missing $f"; done

enforces_declared_cap "$TXE" \
  || fail "ade_plutus does not enforce the per-script declared ex_units cap (false-accept risk)"

# Ledger consumer must REJECT when any script overran its declared cap.
LS="$(sed -E 's://.*$::' "$PE")"
grep -Eq '!s\.success' <<< "$LS" \
  || fail "ledger does not reject on a per-script budget overrun (!s.success -> Failed)"
grep -Eq 'PlutusEvalOutcome::Failed' <<< "$LS" \
  || fail "ledger missing the Failed rejection path for an over-budget script"

# Adversarial Plutus reject corpus present and actually running (not #[ignore]'d):
# the under-declared budget false-accept (A1) plus the broader must-reject classes
# (a validator that returns false; an extraneous redeemer with no matching script).
for t in under_declared_ex_units_must_reject failing_validator_must_reject extraneous_redeemer_must_reject; do
  grep -Eq "fn $t" "$TEST" || fail "missing adversarial Plutus reject test: $t"
  ctx="$(grep -B3 "fn $t" "$TEST" || true)"
  if grep -Eq '#\[ignore' <<< "$ctx"; then
    fail "adversarial reject test $t is #[ignore]'d -- it must run in CI"
  fi
done

echo "OK: per-script declared ex_units cap enforced in ade_plutus + rejected by the ledger; \
Plutus adversarial reject corpus present (budget / failing-validator / extraneous-redeemer); \
does not flip CN-PLUTUS-02 / DC-LEDGER-03"
