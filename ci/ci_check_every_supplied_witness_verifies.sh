#!/usr/bin/env bash
set -uo pipefail

# LV-14 (DC-TXV-08) -- every supplied witness signature verifies.
#
# cardano-ledger validateVerifiedWits rejects a transaction if ANY supplied vkey or bootstrap witness has
# a signature that does not verify over the body hash, required or not; validateNeededWitnesses is a
# separate rule. Structurally refused here:
#   * verify_supplied_witnesses not walking every vkey witness and every bootstrap witness;
#   * its verdict not taken from verify_ed25519's result;
#   * either path (block, standalone) not calling it, or no longer calling the distinct
#     required-signer check.
# Then the LV-14 tests are each required to have run and passed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

WIT="crates/ade_ledger/src/tx_validity/witness.rs"
RULES="crates/ade_ledger/src/rules.rs"
PHASE1="crates/ade_ledger/src/tx_validity/phase1.rs"
for f in "$WIT" "$RULES" "$PHASE1"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

python3 - "$WIT" "$RULES" "$PHASE1" <<'PYEOF' || FAILED=1
import re, sys

def prod(path):
    src = re.sub(r"//[^\n]*", "", open(path).read())
    i = src.find("#[cfg(test)]\nmod ")
    return src if i < 0 else src[:i]

def body(src, name):
    m = re.search(r"\bfn\s+" + name + r"\b[^{]*\{", src)
    if not m:
        return None
    depth, k = 1, m.end()
    while k < len(src) and depth:
        depth += {"{": 1, "}": -1}.get(src[k], 0)
        k += 1
    return src[m.end():k]

wit, rules, phase1 = (prod(p) for p in sys.argv[1:4])
bad = []
f = body(wit, "verify_supplied_witnesses")
if f is None:
    bad.append("verify_supplied_witnesses is gone")
else:
    if not re.search(r"for\s+w\s+in\s+vkey_witnesses\s*\{", f):
        bad.append("verify_supplied_witnesses does not walk every vkey witness")
    if not re.search(r"for\s+w\s+in\s+bootstrap_witnesses\s*\{", f):
        bad.append("verify_supplied_witnesses does not walk every bootstrap witness")
    if not re.search(r"Ok\(matches!\(verify_ed25519\(", f):
        bad.append("verify_supplied_witnesses does not take its verdict from verify_ed25519")
for name, src, fn in [("block path", rules, "verify_conway_witness_closure"), ("standalone path", phase1, "tx_phase_one")]:
    b = body(src, fn) or ""
    if "verify_supplied_witnesses(" not in b:
        bad.append(f"the {name} ({fn}) does not verify every supplied witness")
    if "verify_required_witnesses(" not in b:
        bad.append(f"the {name} ({fn}) no longer runs the distinct required-signer check")
for b in bad:
    print("FAIL: " + b)
sys.exit(1 if bad else 0)
PYEOF

OUT=$(cargo test -p ade_ledger --test lv14_every_supplied_witness_verifies 2>&1); RC=$?
if [ $RC -ne 0 ]; then
  fail "the LV-14 suite failed"
  printf '%s\n' "$OUT" | grep -E "panicked|FAILED|^error" | head -8
fi
PASSED=$(grep -oE "^test result: ok\. [0-9]+ passed" <<<"$OUT" | head -1 | awk '{print $4}')
[ -n "${PASSED:-}" ] && [ "$PASSED" -ge 1 ] || fail "the LV-14 suite ran no tests (vacuous gate)"
for t in \
  lv14_a_bad_signature_on_a_non_required_witness_rejects_in_both_modes \
  lv14_required_signer_failures_keep_their_own_errors \
  lv14_wrong_size_keys_and_signatures_are_malformed \
  lv14_a_native_script_on_a_badly_signed_key_does_not_carry_the_tx \
  lv14_a_real_bootstrap_witness_verifies_and_a_flipped_one_rejects_in_both_paths \
  lv14_a_real_vkey_signature_flipped_gives_the_same_error_on_both_paths; do
  grep -qE "^test ${t} \.\.\. ok$" <<<"$OUT" || fail "test did not run and pass: $t"
done

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (LV-14: every supplied vkey and bootstrap witness verifies on both paths, distinct"
  echo "        from required-signer coverage; ${PASSED} tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
