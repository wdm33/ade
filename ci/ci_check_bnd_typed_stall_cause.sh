#!/usr/bin/env bash
set -uo pipefail

# BND-1 (DC-EPOCH-39) -- a boundary stall and a within-epoch apply failure are DISTINCT STATES, and
# only a real crossing may enter the boundary machinery.
#
# The regression this prevents is a measured live one. `advance_accumulator_over_block` used to map
# EVERY `apply_selected_block` error onto one `Stalled { reason: String }` variant -- its own doc
# naming both causes for it -- and the relay loop treated every one as a boundary: rewind the reduced
# checkpoint, sum a per-credential mark, attempt a cross. On preprod that ran 84,783 ms of boundary
# machinery plus 23,389 ms undoing its own rewind, every memo scope, for slot 130,350,133, which is
# ordinary epoch-305 traffic and not on a boundary at all
# (docs/evidence/run-stores/preprod-live2c/bnd-census-classified.txt).
#
# This gate asserts STRUCTURE, not merely that tests exist: re-flattening the two states, deriving the
# boundary condition from an error instead of the epochs, rendering the typed error to a String, or
# routing an apply failure back into the boundary arm each fail here even if everything compiles.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail -- grep exits on first
# match, echo takes SIGPIPE, and the gate flakes on large files. Grep files directly, or `<<<`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

ADV="crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs"
LIFE="crates/ade_node/src/node_lifecycle.rs"

for f in "$ADV" "$LIFE"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

# 1. The two states exist and are separate.
grep -q "BoundaryMarkRequired" "$ADV" || fail "AdvanceOutcome::BoundaryMarkRequired is gone"
grep -q "ApplyFailed" "$ADV" || fail "AdvanceOutcome::ApplyFailed is gone"
grep -q "BoundaryRequiredAt" "$ADV" || fail "AccumulatorChaindbOutcome::BoundaryRequiredAt is gone"
grep -q "ApplyFailedAt" "$ADV" || fail "AccumulatorChaindbOutcome::ApplyFailedAt is gone"

# 2. The flattened variants must NOT come back INSIDE the two enums this slice split. Scoped to those
#    enum bodies deliberately: `AccumulatorBoundaryOutcome::Stalled` is a DIFFERENT type (the outcome of
#    an actual cross attempt) and is legitimately untouched here -- a file-wide grep would fail on it.
python3 - <<'PYEOF' || FAILED=1
import re, sys
src = open("crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs").read()
bad = []
def body(name):
    i = src.find("pub enum %s {" % name)
    if i < 0:
        return None
    j = src.find("\n}", i)
    return src[i:j]
for enum, gone in (("AdvanceOutcome", "Stalled"), ("AccumulatorChaindbOutcome", "StalledAt")):
    b = body(enum)
    if b is None:
        bad.append("enum %s not found" % enum); continue
    if re.search(r"^\s*%s\s*\{" % gone, b, re.M):
        bad.append("%s::%s is back -- the two causes are flattened again" % (enum, gone))
for msg in bad:
    print("FAIL: " + msg)
sys.exit(1 if bad else 0)
PYEOF

# 3. The boundary decision is POSITIVE -- taken from the epochs, BEFORE the apply. An error-derived
#    classification would re-couple the two classes through a failure message.
grep -q "ctx.block_epoch.0 > acc_epoch.0" "$ADV" \
  || fail "the boundary predicate is not the strict epoch comparison (block_epoch > acc epoch)"
# It must precede the apply call, or it is not a pre-apply decision.
PRED_LINE=$(grep -n "ctx.block_epoch.0 > acc_epoch.0" "$ADV" | head -1 | cut -d: -f1)
APPLY_LINE=$(grep -n "match apply_selected_block(&acc, block_bytes, &selected_ctx)" "$ADV" | head -1 | cut -d: -f1)
if [ -n "${PRED_LINE:-}" ] && [ -n "${APPLY_LINE:-}" ]; then
  [ "$PRED_LINE" -lt "$APPLY_LINE" ] \
    || fail "the boundary predicate ($PRED_LINE) does not precede the apply ($APPLY_LINE)"
else
  fail "could not locate the predicate and the apply call to compare their order"
fi

# 4. The ledger's typed error survives -- not a rendered string.
grep -q "error: LedgerTransitionError" "$ADV" \
  || fail "ApplyFailed no longer carries LedgerTransitionError by value"
# Scoped to the advancer function: the boundary-cross helper below it legitimately renders its own
# outcome's reason, and this slice does not touch that type.
python3 - <<'PYEOF' || FAILED=1
import sys
src = open("crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs").read()
i = src.find("pub fn advance_accumulator_over_block(")
j = src.find("pub enum AccumulatorChaindbOutcome", i)
fn = src[i:j] if i >= 0 and j > i else ""
if not fn:
    print("FAIL: could not isolate advance_accumulator_over_block"); sys.exit(1)
if "format!(\"{e:?}\")" in fn or "reason:" in fn:
    print("FAIL: the advancer is rendering its typed error to a String again"); sys.exit(1)
print("ok: the advancer returns the typed error unrendered")
PYEOF

# 5. Routing: the boundary arm is entered ONLY from BoundaryRequiredAt, and the apply-failure arm
#    performs none of the boundary work.
grep -q "AccumulatorChaindbOutcome::BoundaryRequiredAt" "$LIFE" \
  || fail "the relay loop no longer matches BoundaryRequiredAt"
grep -q "AccumulatorChaindbOutcome::ApplyFailedAt" "$LIFE" \
  || fail "the relay loop no longer matches ApplyFailedAt"

# The apply-failure arm must not call any boundary primitive. Extract just that arm and check it.
python3 - <<'PY' || FAILED=1
import re, sys
src = open("crates/ade_node/src/node_lifecycle.rs").read()
start = src.find("Ok(AccumulatorChaindbOutcome::ApplyFailedAt")
if start < 0:
    print("FAIL: ApplyFailedAt arm not found"); sys.exit(1)
end = src.find("Ok(AccumulatorChaindbOutcome::BoundaryRequiredAt", start)
if end < 0:
    print("FAIL: BoundaryRequiredAt arm not found after ApplyFailedAt"); sys.exit(1)
arm = src[start:end]
forbidden = [
    "position_reduced_checkpoint_at_boundary",
    "sum_base_credential_stake",
    "cross_accumulator_over_boundary_block",
    "bind_boundary_mark",
]
bad = [f for f in forbidden if f in arm]
if bad:
    print("FAIL: the apply-failure arm runs boundary machinery: " + ", ".join(bad))
    sys.exit(1)
print("ok: the apply-failure arm performs no boundary work")
PY

# 6. The named tests must exist AND actually run (a gate that shells out must assert a nonzero count).
for t in a_within_epoch_apply_failure_is_apply_failed_not_a_boundary \
         a_crossing_is_classified_from_the_epochs_even_when_the_apply_would_fail; do
  grep -q "fn $t" "$ADV" || fail "missing test: $t"
done

OUT=$(cargo test -p ade_runtime --lib chaindb::epoch_accumulator_advance 2>&1)
RC=$?
if [ $RC -ne 0 ]; then
  fail "the accumulator-advance test suite failed"
  printf '%s\n' "$OUT" | tail -25
fi
PASSED=$(printf '%s\n' "$OUT" | grep -oE "^test result: ok\. [0-9]+ passed" | head -1 | awk '{print $4}')
if [ -z "${PASSED:-}" ] || [ "$PASSED" -lt 1 ]; then
  fail "the test suite reported no passing tests (vacuous gate)"
fi

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (BND-1 DC-EPOCH-39: stall causes are typed; only a crossing enters boundary machinery; $PASSED tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
