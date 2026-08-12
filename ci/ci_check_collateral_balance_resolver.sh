#!/usr/bin/env bash
set -uo pipefail

# BND-2b (INV-BND-2b) -- the UTxO authority RESOLVES, the accumulator CONSUMES one scalar.
#
# `collAdaBalance` is defined over the RESOLVED collateral UTxO entries
# (Cardano.Ledger.Babbage.Collateral): the consumed amount is a property of the entries, not of the
# transaction. Two ways to get this wrong are cheap and silent, so both are gated here:
#   * substituting `total_collateral` (body field 17) for the resolution -- it is a DECLARED
#     ASSERTION the UTXO rule checks when present and which constrains NOTHING when absent, so a
#     transaction that omits it (like preprod 130,350,133) would silently take a wrong path;
#   * treating an unresolved input as zero or skipping it -- that converts an explicit "I do not
#     know" into a wrong fee delta, which is the exact failure the accumulator's fail-closed guards
#     have been protecting against.
#
# CE-2b-5 (the accumulator gains no UTxO access) is deliberately NOT re-implemented here: it is
# already enforced by ci/ci_check_epoch_accumulator_no_utxo.sh, which this gate RUNS rather than
# duplicates. A second copy of an invariant is a second thing to drift.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

SRC="crates/ade_ledger/src/collateral.rs"
[ -f "$SRC" ] || { fail "missing $SRC"; echo "RESULT: FAIL"; exit 1; }

# 1. The seam exists and points inward: the rule declares what it needs.
grep -q "pub trait CollateralValueResolver" "$SRC" || fail "CollateralValueResolver is gone"
grep -q "fn collateral_value(&self, txin: &TxIn) -> Option<Coin>" "$SRC" \
  || fail "the resolver's answer is no longer Option<Coin> (None must remain expressible)"
grep -q "pub fn collateral_balance(" "$SRC" || fail "collateral_balance is gone"

# 2. total_collateral is NEVER the resolution authority. Checked against CODE, not prose: the module
#    documents that it does not consult field 17, and that sentence must not be mistaken for a use.
python3 - <<'PYEOF' || FAILED=1
import sys
code = []
for line in open("crates/ade_ledger/src/collateral.rs"):
    s = line.strip()
    if s.startswith("//") or s.startswith("///") or s.startswith("//!"):
        continue
    code.append(line)
body = "".join(code)
if "total_collateral" in body:
    print("FAIL: collateral.rs READS total_collateral -- it is a declared assertion, never the value source")
    sys.exit(1)
print("ok: total_collateral appears only in prose, never in code")
PYEOF

# 3. An unresolved input must become a typed refusal -- never 0, never skipped.
python3 - <<'PYEOF' || FAILED=1
import re, sys
src = open("crates/ade_ledger/src/collateral.rs").read()
i = src.find("pub fn collateral_balance(")
j = src.find("#[cfg(test)]", i)
fn = src[i:j] if i >= 0 and j > i else ""
if not fn:
    print("FAIL: could not isolate collateral_balance"); sys.exit(1)
bad = []
if "UnresolvedCollateralInput" not in fn:
    bad.append("the unresolved case no longer raises UnresolvedCollateralInput")
for pat in (r"unwrap_or_default\(", r"unwrap_or\(", r"\.unwrap_or_else\(\s*\|\|\s*Coin\(0\)"):
    if re.search(pat, fn):
        bad.append(f"an unresolved value is being defaulted (matched /{pat}/)")
# The return must be subtracted exactly once.
if fn.count("checked_sub") != 1:
    bad.append(f"expected exactly one checked_sub for the collateral return, found {fn.count('checked_sub')}")
for m in bad: print("FAIL: " + m)
sys.exit(1 if bad else 0)
PYEOF

# 4. The accumulator still owns no UTxO -- RUN the existing gate rather than restate it.
if ! ./ci/ci_check_epoch_accumulator_no_utxo.sh >/dev/null 2>&1; then
  fail "ci_check_epoch_accumulator_no_utxo.sh failed -- the accumulator gained UTxO access"
fi

# 5. The named tests must exist AND run (nonzero count -- no vacuous gate).
for t in multiple_collateral_inputs_sum_deterministically \
         the_collateral_return_is_subtracted_exactly_once \
         an_unresolved_collateral_input_is_a_typed_refusal_not_zero \
         one_unresolved_input_among_several_still_refuses \
         the_refusal_is_replay_identical; do
  grep -q "fn $t" "$SRC" || fail "missing test: $t"
done

OUT=$(cargo test -p ade_ledger --lib collateral::tests 2>&1)
RC=$?
if [ $RC -ne 0 ]; then
  fail "the collateral-balance suite failed"
  printf '%s\n' "$OUT" | tail -20
fi
PASSED=$(printf '%s\n' "$OUT" | grep -oE "^test result: ok\. [0-9]+ passed" | head -1 | awk '{print $4}')
if [ -z "${PASSED:-}" ] || [ "$PASSED" -lt 5 ]; then
  fail "expected >=5 passing collateral tests, got ${PASSED:-0} (vacuous gate)"
fi

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (BND-2b: resolver seam intact, total_collateral never the source, unresolved => refusal; $PASSED tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
