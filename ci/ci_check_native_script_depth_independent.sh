#!/usr/bin/env bash
set -uo pipefail

# LV-12 (DC-LEDGER-SCRIPT-02) -- native scripts and CBOR skipping do not depend on thread stack.
#
# Preprod block 5,183,974 (slot 133,883,340) carries a native script ~5,385 levels deep. Recursive
# decoders needed ~2.5 MiB of stack for it in release and ~28 MiB in debug, so it crashed a 2 MiB
# tokio worker. There is deliberately no depth limit: Cardano bounds nesting only by size, and a limit
# below that rejects valid blocks. So recursion itself is refused here, structurally:
#   * any call cycle among the production functions of cbor/mod.rs (skip_item), the native-script
#     decoder, or the evaluator -- direct or mutual;
#   * a NativeScript type with a recursive field (derived Drop/Clone/Eq/Debug would recurse again).
# The LV-12 tests then run on 256 KiB threads and are each required to have run and passed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

CBOR="crates/ade_codec/src/cbor/mod.rs"
CODEC="crates/ade_codec/src/allegra/script.rs"
EVAL="crates/ade_ledger/src/scripts.rs"
TYPES="crates/ade_types/src/allegra/script.rs"
for f in "$CBOR" "$CODEC" "$EVAL" "$TYPES"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

python3 - "$CBOR" "$CODEC" "$EVAL" "$TYPES" <<'PYEOF' || FAILED=1
import re, sys

def strip(src):
    # drop line comments and test modules: only production code counts
    src = re.sub(r"//[^\n]*", "", src)
    i = src.find("#[cfg(test)]")
    out = src if i < 0 else src[:i]
    # cbor/mod.rs has production items after its test module; keep them, skip test module bodies
    rest = src[i:] if i >= 0 else ""
    while rest:
        m = re.search(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*mod\s+\w+\s*(;|\{)", rest)
        if not m:
            out += rest
            break
        out += rest[:m.start()]
        if m.group(1) == ";":
            rest = rest[m.end():]
            continue
        depth, j = 1, m.end()
        while j < len(rest) and depth:
            depth += {"{": 1, "}": -1}.get(rest[j], 0)
            j += 1
        rest = rest[j:]
    return out

def functions(src):
    fns = {}
    for m in re.finditer(r"\bfn\s+([a-z_][a-z0-9_]*)\s*(?:<[^>{]*>)?\s*\(", src):
        name = m.group(1)
        j = src.find("{", m.end())
        if j < 0:
            continue
        depth, k = 1, j + 1
        while k < len(src) and depth:
            depth += {"{": 1, "}": -1}.get(src[k], 0)
            k += 1
        fns[name] = src[j:k]
    return fns

bad = []
for path in sys.argv[1:4]:
    fns = functions(strip(open(path).read()))
    graph = {n: {c for c in fns if c != "" and re.search(r"(?<![A-Za-z0-9_.:])" + c + r"\s*\(", body.replace("fn " + n, ""))} for n, body in fns.items()}
    # cycle detection (direct self-calls included)
    state = {}
    def visit(n, stack):
        state[n] = 1
        stack.append(n)
        for c in sorted(graph[n]):
            if state.get(c) == 1:
                bad.append(f"{path}: recursion {' -> '.join(stack[stack.index(c):] + [c])}")
            elif c not in state:
                visit(c, stack)
        stack.pop()
        state[n] = 2
    for n in sorted(graph):
        if n not in state:
            visit(n, [])
    need = {"crates/ade_codec/src/cbor/mod.rs": ["skip_item"],
            "crates/ade_codec/src/allegra/script.rs": ["decode_native_script", "decode_native_scripts"],
            "crates/ade_ledger/src/scripts.rs": ["evaluate_native_script"]}[path]
    for n in need:
        if n not in fns:
            bad.append(f"{path}: {n} not found")

types = strip(open(sys.argv[4]).read())
m = re.search(r"pub struct NativeScript\s*\{(.*?)\}", types, re.S)
if not m:
    bad.append("NativeScript is no longer a struct")
elif re.search(r"\bNativeScript\b", m.group(1)):
    bad.append("NativeScript holds a NativeScript: the type is recursive again")
m = re.search(r"pub enum NativeScriptNode\s*\{(.*?)\n\}", types, re.S)
if not m:
    bad.append("NativeScriptNode is gone")
elif re.search(r"\bNativeScript\b", m.group(1)):
    bad.append("a NativeScriptNode holds a NativeScript: the type is recursive again")

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
    printf '%s\n' "$out" | grep -E "overflowed its stack|panicked|FAILED" | head -5
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

run_and_require "the LV-12 native-script" \
  cargo test -p ade_ledger --test lv12_native_script_depth
require_named \
  lv12_the_deep_preprod_block_decodes_and_evaluates_on_a_256_kib_thread \
  lv12_the_deep_preprod_script_is_what_the_recursive_reference_decodes \
  lv12_a_script_as_deep_as_a_transaction_allows_round_trips_on_a_256_kib_thread \
  lv12_a_deep_script_inside_a_witness_set_decodes_on_a_256_kib_thread \
  lv12_every_fixture_script_decodes_and_evaluates_as_the_reference_does \
  lv12_generated_scripts_decode_and_evaluate_as_the_reference_does \
  lv12_malformed_scripts_fail_exactly_as_the_reference_does \
  lv12_a_forged_sub_script_count_is_an_error_not_an_allocation \
  lv12_the_flat_type_has_no_recursive_field

run_and_require "the skip_item differential" \
  cargo test -p ade_codec --lib cbor::skip_differential_tests
require_named \
  lv12_skip_item_matches_the_recursive_reference_on_every_fixture_and_inner_offset \
  lv12_skip_item_matches_the_recursive_reference_on_every_truncated_prefix \
  lv12_skip_item_matches_the_recursive_reference_on_constructed_forms \
  lv12_skip_item_walks_ninety_thousand_levels_on_a_256_kib_thread

run_and_require "the flat type" \
  cargo test -p ade_types --lib allegra::script::tests
require_named \
  a_deep_script_drops_clones_and_compares_on_a_small_stack \
  from_post_order_accepts_exactly_one_tree

rm -f "$TMP_OUT"

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (LV-12: no recursion in skip_item, the native-script decoder or evaluator; flat"
  echo "        script type; deep real and constructed scripts on 256 KiB threads; $TOTAL_PASSED tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
