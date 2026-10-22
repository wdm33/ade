#!/usr/bin/env bash
set -uo pipefail

# LV-11 (DC-LEDGER-SCRIPT-01) -- a script_n_of_k threshold keeps Cardano's signed 64-bit meaning.
#
# Conway CDDL: `script_n_of_k = (3, n : int64, [* native_script])`. cardano-ledger decodes `n` as a
# signed `Int` and evaluates `isValidMOf n ... = n <= 0 || ...`. Ade used to read it with read_uint
# into a u32, which refused preprod block 5,184,855 (threshold -1, the LV-8 live stop at 133,902,219)
# and truncated 2^32 to 0, a pass where Cardano fails.
#
# Structurally refused here: an unsigned threshold read, a u32 threshold type, a narrowing cast in
# the decoder or the evaluator, and an error that cannot carry the real threshold. The LV-11 tests
# are then run and each is required to have run and passed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

TYPES="crates/ade_types/src/allegra/script.rs"
CODEC="crates/ade_codec/src/allegra/script.rs"
EVAL="crates/ade_ledger/src/scripts.rs"
ERR="crates/ade_ledger/src/error.rs"
for f in "$TYPES" "$CODEC" "$EVAL" "$ERR"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

python3 - "$TYPES" "$CODEC" "$EVAL" "$ERR" <<'PYEOF' || FAILED=1
import re, sys
types, codec, ev, err = (open(p).read() for p in sys.argv[1:5])

def prod(src):
    i = src.find("#[cfg(test)]")
    return src if i < 0 else src[:i]

bad = []
types_p, codec_p, ev_p, err_p = prod(types), prod(codec), prod(ev), prod(err)

# LV-12 made the script a flat post-order node list; the threshold lives on NativeScriptNode::MOfN.
if not re.search(r"\bMOfN\s*\{\s*threshold:\s*i64\s*,\s*arity:\s*u64\s*\}", types_p):
    bad.append("NativeScriptNode::MOfN no longer holds an i64 threshold")

# The decoder's tag-3 arm: from `3 =>` to the next arm.
m = re.search(r"\n\s*3\s*=>\s*\{(.*?)\n\s*4\s*=>", codec_p, re.S)
arm = m.group(1) if m else ""
if not arm:
    bad.append("could not isolate the MOfN decode arm")
else:
    if "read_uint" in arm:
        bad.append("the MOfN arm reads its threshold with read_uint (unsigned)")
    if "read_threshold(" not in arm:
        bad.append("the MOfN arm no longer reads its threshold with read_threshold")
m = re.search(r"fn read_threshold\(.*?\n\}", codec_p, re.S)
rt = m.group(0) if m else ""
if not rt:
    bad.append("read_threshold is gone")
else:
    if "read_any_int" not in rt:
        bad.append("read_threshold no longer accepts both CBOR integer major types")
    if "i64::try_from" not in rt:
        bad.append("read_threshold no longer refuses a magnitude outside int64")
if re.search(r"\bas\s+(u32|i32|u16|u8)\b", codec_p):
    bad.append("a narrowing cast appears in the native-script decoder")

m = re.search(r"NativeScriptNode::MOfN\s*\{\s*threshold,\s*\.\.\s*\}\s*=>\s*\{(.*?)\n        NativeScriptNode::", ev_p, re.S)
ea = m.group(1) if m else ""
if not ea:
    bad.append("could not isolate the MOfN evaluation arm")
else:
    if re.search(r"\bas\s+(u32|i32|u16|u8)\b|\bu32\b", ea):
        bad.append("the MOfN evaluation arm narrows to 32 bits")
    if "u64::try_from(*threshold)" not in ea:
        bad.append("the MOfN evaluation arm no longer treats a threshold <= 0 as satisfied (isValidMOf)")

if not re.search(r"ThresholdNotMet\s*\{\s*required:\s*i64,\s*provided:\s*u64\s*\}", err_p):
    bad.append("ThresholdNotMet can no longer carry the untruncated threshold")

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
    printf '%s\n' "$out" | tail -15
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

run_and_require "the LV-11 threshold" \
  cargo test -p ade_ledger --test lv11_native_script_threshold
require_named \
  lv11_the_refused_preprod_block_now_decodes_its_negative_threshold \
  lv11_the_real_negative_threshold_script_passes_with_no_signatures \
  lv11_thresholds_inside_int64_decode_to_exactly_their_value \
  lv11_thresholds_outside_int64_are_refused \
  lv11_a_non_integer_threshold_is_still_refused \
  lv11_a_threshold_of_zero_or_below_is_satisfied_outright \
  lv11_a_threshold_of_one_needs_one_passing_sub_script \
  lv11_a_threshold_above_u32_is_not_truncated_into_a_pass \
  lv11_a_decoded_threshold_of_two_to_the_32_fails_end_to_end

rm -f "$TMP_OUT"

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (LV-11: script_n_of_k threshold is a signed int64 end to end, isValidMOf"
  echo "        semantics, out-of-range refused; $TOTAL_PASSED tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
