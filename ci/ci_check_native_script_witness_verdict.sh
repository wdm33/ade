#!/usr/bin/env bash
set -uo pipefail

# LV-13 (DC-LEDGER-SCRIPT-03) -- native-script witness verdicts decide transaction validity.
#
# One authority (tx_validity::script_witnesses) decides a Conway tx's witness-script closure, with two
# evidence levels: without full UTxO context it rejects exactly when a witness-set native script fails
# and is otherwise Undetermined (never valid); with every spend and reference input resolved it gives
# cardano-ledger's exact UTXOW result (not-validating, extraneous, missing). Structurally refused here:
#   * the evaluator taking a slot instead of the tx validity interval;
#   * a production native-script evaluation outside the authority (a second verdict path);
#   * either path (block, standalone) not calling the shared seam;
#   * an exact verdict built anywhere but from a resolved context, or a public context constructor
#     that skips resolution;
#   * a needed-set derivation missing any of the six Conway purposes;
#   * a native-script hash not taken over the script's own wire bytes.
# Then the LV-13 tests and the evaluator's timelock tests are each required to have run and passed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

SCRIPTS="crates/ade_ledger/src/scripts.rs"
AUTH="crates/ade_ledger/src/tx_validity/script_witnesses.rs"
RULES="crates/ade_ledger/src/rules.rs"
PHASE1="crates/ade_ledger/src/tx_validity/phase1.rs"
WITNESS="crates/ade_ledger/src/witness.rs"
for f in "$SCRIPTS" "$AUTH" "$RULES" "$PHASE1" "$WITNESS"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

python3 - "$SCRIPTS" "$AUTH" "$RULES" "$PHASE1" "$WITNESS" <<'PYEOF' || FAILED=1
import re, sys, glob

def prod(path):
    src = re.sub(r"//[^\n]*", "", open(path).read())
    # drop #[cfg(test)] modules; keep everything else
    out, rest = "", src
    while True:
        m = re.search(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*mod\s+\w+\s*(;|\{)", rest)
        if not m:
            return out + rest
        out += rest[:m.start()]
        if m.group(1) == ";":
            rest = rest[m.end():]
            continue
        depth, j = 1, m.end()
        while j < len(rest) and depth:
            depth += {"{": 1, "}": -1}.get(rest[j], 0)
            j += 1
        rest = rest[j:]

def body(src, name):
    m = re.search(r"\bfn\s+" + name + r"\b[^{]*\{", src)
    if not m:
        return None
    depth, k = 1, m.end()
    while k < len(src) and depth:
        depth += {"{": 1, "}": -1}.get(src[k], 0)
        k += 1
    return src[m.end():k]

scripts, auth, rules, phase1, witness = (prod(p) for p in sys.argv[1:6])
bad = []

sig = re.search(r"pub fn evaluate_native_script\s*\(([^)]*)\)", scripts, re.S)
if not sig or "interval: &ValidityInterval" not in sig.group(1):
    bad.append("evaluate_native_script does not take the tx validity interval")
if sig and re.search(r"slot\s*:\s*u64", sig.group(1)):
    bad.append("evaluate_native_script takes a slot")

for path in glob.glob("crates/ade_ledger/src/**/*.rs", recursive=True):
    if path in (sys.argv[1], sys.argv[2]):
        continue
    if re.search(r"\bevaluate_native_script\s*\(", prod(path)):
        bad.append(f"{path}: evaluates native scripts outside the witness-script authority")

closure = body(rules, "verify_conway_witness_closure")
if closure is None or "check_script_witnesses(" not in closure:
    bad.append("the block path (verify_conway_witness_closure) does not call check_script_witnesses")
p1 = body(phase1, "tx_phase_one")
if p1 is None or "check_script_witnesses(" not in p1:
    bad.append("the standalone path (tx_phase_one) does not call check_script_witnesses")

if len(re.findall(r"NativeScriptVerdict::Exact\(\s*exact_verdict\(", auth)) != 1 or \
   len(re.findall(r"NativeScriptVerdict::Exact\(", auth)) != len(re.findall(r"NativeScriptVerdict::Exact\(", body(auth, "check_script_witnesses") or "")) + 1:
    bad.append("an exact verdict is built somewhere other than from a resolved context")
nv = body(auth, "native_script_verdict") or ""
if not re.search(r"Some\(ctx\)\s*=>\s*NativeScriptVerdict::Exact\(exact_verdict\(", nv):
    bad.append("native_script_verdict does not tie the exact level to a resolved context")
ctx = re.search(r"pub struct ResolvedScriptContext\s*\{(.*?)\}", auth, re.S)
if not ctx or re.search(r"\bpub\b", ctx.group(1)):
    bad.append("ResolvedScriptContext's fields are public: a context can skip resolution")

needed = (body(auth, "scripts_needed") or "") + (body(auth, "tx_derived_scripts_needed") or "")
for purpose in ["Spending", "Rewarding", "Certifying", "Minting", "Voting", "Proposing"]:
    if not re.search(r"need\(\s*ScriptPurpose::" + purpose + r"\b", needed):
        bad.append(f"the needed set does not derive the {purpose} purpose")

if not re.search(r"prefixed_script_hash\(\s*NATIVE_SCRIPT_PREFIX\s*,\s*&data\[range\]\s*\)", witness):
    bad.append("witness native-script hashes are not taken over the script's own wire bytes")

for b in bad:
    print("FAIL: " + b)
sys.exit(1 if bad else 0)
PYEOF

TMP_OUT=$(mktemp); TOTAL_PASSED=0
run_and_require() {
  local desc="$1"; shift
  local out rc passed
  out=$("$@" 2>&1); rc=$?
  if [ $rc -ne 0 ]; then
    fail "$desc suite failed"
    printf '%s\n' "$out" | grep -E "panicked|FAILED|^error" | head -8
    return
  fi
  passed=$(grep -oE "^test result: ok\. [0-9]+ passed" <<<"$out" | head -1 | awk '{print $4}')
  if [ -z "${passed:-}" ] || [ "$passed" -lt 1 ]; then
    fail "$desc ran no tests (vacuous gate)"
    return
  fi
  printf '%s\n' "$out" > "$TMP_OUT"
  TOTAL_PASSED=$((TOTAL_PASSED + passed))
}
require_named() {
  for t in "$@"; do
    grep -qE "^test ([A-Za-z0-9_:]+::)?${t} \.\.\. ok$" "$TMP_OUT" \
      || fail "test did not run and pass: $t"
  done
}

run_and_require "the LV-13 witness-script" \
  cargo test -p ade_ledger --test lv13_native_script_witness_verdict
require_named \
  lv13_a_needed_passing_native_script_is_accepted \
  lv13_the_same_tx_with_a_failing_native_script_is_rejected_at_both_levels \
  lv13_failing_all_any_and_n_of_k_each_reject \
  lv13_a_threshold_of_zero_or_below_passes_exactly_as_cardano \
  lv13_invalid_before_holds_from_the_lock_and_never_without_a_start \
  lv13_invalid_hereafter_holds_up_to_the_lock_and_never_without_an_expiry \
  lv13_the_block_slot_does_not_decide_a_timelock \
  lv13_a_missing_required_native_script_is_missing \
  lv13_a_reference_script_satisfies_the_need_and_a_witness_copy_is_extraneous \
  lv13_a_failing_needed_reference_native_script_does_not_validate \
  lv13_an_extra_passing_script_is_undetermined_without_context_and_extraneous_with_it \
  lv13_an_extra_failing_script_is_a_definite_reject_and_exactly_extraneous \
  lv13_the_needed_set_covers_every_conway_purpose \
  lv13_a_native_script_hash_is_over_its_own_bytes_not_a_re_encoding \
  lv13_a_plutus_witness_counts_toward_the_closure_but_is_not_evaluated_here \
  lv13_a_script_ref_decodes_to_the_hash_of_its_native_bytes \
  lv13_a_real_timelock_mint_is_exactly_valid \
  lv13_a_real_reference_script_multisig_spend_is_exactly_valid \
  lv13_a_real_bootstrap_witness_key_hash_is_its_byron_address_root \
  lv13_the_block_path_accepts_the_real_block_and_rejects_it_with_a_failing_timelock \
  lv13_the_mutated_real_script_is_exactly_missing_and_extraneous \
  lv13_verdicts_are_deterministic

run_and_require "the evaluator's timelock" \
  cargo test -p ade_ledger --lib scripts::tests
require_named \
  invalid_before_holds_when_the_tx_starts_at_or_after_the_lock \
  invalid_before_never_holds_without_a_tx_start \
  invalid_hereafter_holds_when_the_tx_expires_at_or_before_the_lock \
  invalid_hereafter_never_holds_without_a_tx_expiry

run_and_require "SHA3-256" cargo test -p ade_crypto --lib sha3
require_named fips_202_vectors

rm -f "$TMP_OUT"

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (LV-13: one witness-script authority on both paths; reduced level rejects only"
  echo "        failing scripts; exact level over all six purposes; interval timelocks; $TOTAL_PASSED tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
