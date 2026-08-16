#!/usr/bin/env bash
set -uo pipefail

# BND-2d (INV-BND-2d) -- the UTxO authority RETAINS what it destroys on another reader's behalf.
#
# THE CONTRACT THIS GUARDS. A collateral value is authoritative only in `[create(x), B)`, where `B`
# is the block whose phase-2-invalid transaction spends it. The co-advancer drives the reduced
# checkpoint to the durable TIP at the end of every pass, so at accumulator-walk time the authority
# is routinely PAST `B` and the entry is legitimately gone -- which is exactly how BND-2c's live bar
# failed on preprod 130,350,133 (2026-08-16). The fix makes the answer POSITION-INVARIANT: the
# authority records the binding at the one instant it still holds it, in the SAME write transaction
# that advances its cursor.
#
# Four ways to break that are cheap and silent, so all four are gated structurally:
#   * writing the retention in a SECOND transaction -- the store could then record a slot it did not
#     retain for, and a crash between the two commits would leave a cursor that lies;
#   * defaulting an unheld binding to zero instead of recording nothing -- that converts an explicit
#     "I do not know" into a wrong fee delta, the exact failure the fail-closed guards exist to stop;
#   * leaving the retention behind on `reset_to_bootstrap` -- it would then answer for blocks the
#     checkpoint no longer claims to have applied;
#   * folding the retention into `compute_fingerprint` -- that commitment names the reduced UTxO and
#     is sealed into frozen leadership (S4-L2); entries the UTxO no longer contains are not part of it.
#
# CE-2d-9 (the accumulator gains no UTxO access) is deliberately NOT re-implemented here: it is
# already enforced by ci/ci_check_epoch_accumulator_no_utxo.sh, which this gate RUNS rather than
# duplicates. A second copy of an invariant is a second thing to drift.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

CP="crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs"
BLUE="crates/ade_ledger/src/reduced_advance.rs"
SEM="crates/ade_ledger/src/store_semantics.rs"
for f in "$CP" "$BLUE" "$SEM"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

# 1. BLUE names the bindings; it does not value the ones it cannot see, and it introduces no second
#    reader of body field 13 (the single derivation is extract_tx_utxo_effect -- INV-BND-2a).
grep -q "pub collateral_consumed: Vec<(TxIn, Option<Coin>)>" "$BLUE" \
  || fail "ReducedBlockDelta no longer names the collateral bindings for retention"
python3 - <<'PYEOF' || FAILED=1
import re, sys
src = open("crates/ade_ledger/src/reduced_advance.rs").read()
i = src.find("fn process_one_tx(")
j = src.find("#[cfg(test)]", i)
fn = src[i:j] if i >= 0 and j > i else ""
if not fn:
    print("FAIL: could not isolate process_one_tx"); sys.exit(1)
bad = []
# The naming must come from the SINGLE derivation, not from a second field-13 read, and the
# consumer must stay VALIDITY-BLIND (INV-BND-2a): no branch on the phase-2 flag here.
if "collateral_inputs" in fn or "collateral_return" in fn:
    bad.append("process_one_tx reads the collateral fields directly -- extract_tx_utxo_effect is the single derivation")
if "effect.collateral_consumed" not in fn:
    bad.append("process_one_tx no longer names the retention from the derivation's own collateral_consumed")
if re.search(r"if\s+invalid", fn):
    bad.append("process_one_tx branches on the phase-2 flag -- consumers must stay validity-blind (INV-BND-2a)")
# The naming must PRECEDE the cancel loop, or an intra-block binding is erased before it is read.
name_at = fn.find("collateral_consumed.push")
cancel_at = fn.find("produced.remove")
if name_at < 0:
    bad.append("process_one_tx no longer records any collateral binding")
elif cancel_at >= 0 and name_at > cancel_at:
    bad.append("the retention naming runs AFTER the intra-block cancel -- the binding is gone by then")
for m in bad: print("FAIL: " + m)
sys.exit(1 if bad else 0)
PYEOF

# 2. The retention is a DISTINCT durable table, and it is written inside the SAME transaction that
#    advances the cursor -- one begin_write, one commit, retention before the removals.
grep -q "COLLATERAL_RETAINED_TABLE" "$CP" || fail "the retention table is gone"
python3 - <<'PYEOF' || FAILED=1
import re, sys
src = open("crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs").read()

def body(sig, src=src):
    i = src.find(sig)
    if i < 0: return ""
    # brace-match from the opening brace of the fn body
    b = src.find("{", src.find(")", i))
    d, k = 0, b
    while k < len(src):
        if src[k] == "{": d += 1
        elif src[k] == "}":
            d -= 1
            if d == 0: return src[b:k+1]
        k += 1
    return ""

bad = []
adv = body("pub fn advance_block(")
if not adv:
    bad.append("could not isolate advance_block")
else:
    if adv.count("begin_write()") != 1:
        bad.append(f"advance_block opens {adv.count('begin_write()')} write transactions, expected exactly 1 "
                   "(the retention must not be a SECOND commit)")
    if adv.count("txn.commit()") != 1:
        bad.append(f"advance_block commits {adv.count('txn.commit()')} times, expected exactly 1")
    if "retain_collateral(" not in adv:
        bad.append("advance_block no longer retains the destroyed collateral bindings")
    else:
        r, rm = adv.find("retain_collateral("), adv.find("table.remove(")
        if rm >= 0 and r > rm:
            bad.append("advance_block retains AFTER removing -- the binding no longer exists by then")

ret = body("fn retain_collateral(")
if not ret:
    bad.append("could not isolate retain_collateral")
else:
    for pat in (r"unwrap_or_default\(", r"unwrap_or\(", r"Coin\(0\)", r"\.unwrap_or_else\("):
        if re.search(pat, ret):
            bad.append(f"an unheld collateral binding is being defaulted (matched /{pat}/)")

rst = body("pub fn reset_to_bootstrap(")
if not rst:
    bad.append("could not isolate reset_to_bootstrap")
elif "COLLATERAL_RETAINED_TABLE" not in rst or "pop_first" not in rst:
    bad.append("reset_to_bootstrap does not CLEAR the retention -- its scope must stay (seed, cursor]")

fp = body("fn compute_fingerprint(")
if not fp:
    bad.append("could not isolate compute_fingerprint")
elif "COLLATERAL_RETAINED_TABLE" in fp:
    bad.append("the retention entered compute_fingerprint -- the checkpoint commitment names the "
               "reduced UTxO and is sealed into frozen leadership; it must not move")

res = body("fn collateral_value(&self, txin: &TxIn) -> Option<Coin> {")
if not res:
    bad.append("could not isolate the CollateralValueResolver impl")
else:
    if "retained_collateral_value" not in res:
        bad.append("the resolver does not consult the retention -- it is position-dependent again, "
                   "which is precisely the BND-2c live failure")
    if "self.get(" not in res:
        bad.append("the resolver no longer consults the live table -- the widening must be a SUPERSET")

for m in bad: print("FAIL: " + m)
sys.exit(1 if bad else 0)
PYEOF

# 3. The typed refusal is NOT retired -- it is what caught this, and BND-2d keeps it reachable.
grep -q "UnresolvedCollateralInput" crates/ade_ledger/src/collateral.rs \
  || fail "the unresolved-collateral refusal is gone -- it goes only when made UNREACHABLE"

# 4. A store written before the retention existed must be refused, not reinterpreted.
VER=$(grep -oE 'pub const STORE_SEMANTICS_VERSION: u32 = [0-9]+' "$SEM" | grep -oE '[0-9]+$')
if [ -z "${VER:-}" ] || [ "$VER" -lt 6 ]; then
  fail "STORE_SEMANTICS_VERSION is ${VER:-unset}, expected >= 6 (a v5 store holds no retention)"
fi
if ! ./ci/ci_check_store_semantics_lock.sh >/dev/null 2>&1; then
  fail "ci_check_store_semantics_lock.sh failed -- the surface changed without a reconciled entry"
fi

# 5. The accumulator still owns no UTxO -- RUN the existing gate rather than restate it.
if ! ./ci/ci_check_epoch_accumulator_no_utxo.sh >/dev/null 2>&1; then
  fail "ci_check_epoch_accumulator_no_utxo.sh failed -- the accumulator gained UTxO access"
fi

# 6. The named tests must exist AND RUN AND PASS. Each is asserted by name in the runner output, so
#    a deleted or renamed test cannot be mistaken for a passing one.
run_and_require() {
  local desc="$1"; shift
  local out rc passed
  out=$("$@" 2>&1); rc=$?
  if [ $rc -ne 0 ]; then
    fail "$desc suite failed"
    printf '%s\n' "$out" | tail -15
    return
  fi
  passed=$(printf '%s\n' "$out" | grep -oE "^test result: ok\. [0-9]+ passed" | head -1 | awk '{print $4}')
  if [ -z "${passed:-}" ] || [ "$passed" -lt 1 ]; then
    fail "$desc ran no tests (vacuous gate)"
    return
  fi
  echo "$out" > "$TMP_OUT"
  TOTAL_PASSED=$((TOTAL_PASSED + passed))
}
TMP_OUT=$(mktemp); TOTAL_PASSED=0
require_named() {
  for t in "$@"; do
    # unit tests print `test path::to::name ... ok`; integration tests print `test name ... ok`.
    grep -qE "^test ([A-Za-z0-9_:]+::)?${t} \.\.\. ok$" "$TMP_OUT" \
      || fail "test did not run and pass: $t"
  done
}

run_and_require "the checkpoint retention" \
  cargo test -p ade_runtime --lib chaindb::reduced_utxo_checkpoint::tests
require_named \
  the_resolver_answers_after_the_authority_has_already_spent_the_collateral \
  reset_to_bootstrap_clears_the_retention_and_the_replay_rederives_it \
  the_retention_does_not_move_the_checkpoint_commitment \
  an_unheld_collateral_binding_is_retained_as_nothing_never_as_zero \
  an_intra_block_created_collateral_binding_is_retained_from_the_block

run_and_require "the walk-time" \
  cargo test -p ade_runtime --lib chaindb::epoch_accumulator_advance::tests
require_named \
  the_accumulator_walk_resolves_collateral_the_authority_already_spent \
  without_the_retention_the_same_walk_still_refuses_and_pins

run_and_require "the BLUE naming" \
  cargo test -p ade_ledger --test bnd2d_collateral_retention
require_named \
  the_real_failing_block_names_its_collateral_binding_for_retention \
  an_ordinary_spend_is_never_named_for_retention

run_and_require "the intra-block naming" \
  cargo test -p ade_ledger --lib reduced_advance::tests
require_named an_intra_block_created_collateral_binding_is_valued_by_the_block_itself

rm -f "$TMP_OUT"

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (BND-2d: retention atomic with the cursor, cleared with the live table, invisible"
  echo "        to the commitment, never defaulted; the walk-time resolution is proven both ways;"
  echo "        $TOTAL_PASSED tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
