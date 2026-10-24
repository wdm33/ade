#!/usr/bin/env bash
set -uo pipefail

# LV-15a (DC-TXV-09, DC-VAL-07) -- admission obligations that need neither the UTxO nor protocol
# parameters.
#
# cardano-ledger babbageUtxoValidation (both isValid shapes) rejects a transaction outside its validity
# interval (half-open: invalid_before <= slot < invalid_hereafter), with any output or collateral return,
# withdrawal or body network id on the wrong network (a bootstrap address's network is its magic
# attribute), or with bootstrap attributes over 64 bytes; BBODY rejects a block whose header body size is
# not the length of its four body segments. Structurally refused here:
#   * the interval going unchecked, or losing either bound or its half-open upper bound;
#   * the output network check skipping the collateral return, or passing bootstrap addresses unread;
#   * the withdrawal, body-field or bootstrap-size check going missing;
#   * the full path's check_address_network keeping its own definition of an address's network;
#   * the body size not summing all four segments;
#   * the block path (both apply functions) or the standalone path not calling them.
# Then the LV-15a tests are each required to have run and passed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

FREE="crates/ade_ledger/src/tx_validity/utxo_free.rs"
RULES="crates/ade_ledger/src/rules.rs"
PHASE1="crates/ade_ledger/src/tx_validity/phase1.rs"
LATE="crates/ade_ledger/src/late_era_validation.rs"
for f in "$FREE" "$RULES" "$PHASE1" "$LATE"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

python3 - "$FREE" "$RULES" "$PHASE1" "$LATE" <<'PYEOF' || FAILED=1
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

free, rules, phase1, late = (prod(p) for p in sys.argv[1:5])
bad = []
def need(src, fn, pattern, why):
    b = body(src, fn)
    if b is None:
        bad.append(f"{fn} is gone")
    elif not re.search(pattern, b):
        bad.append(f"{fn}: {why}")

need(free, "check_validity_interval", r"invalid_before\.is_none_or\(\|b\|\s*b\s*<=\s*slot\.0\)", "the lower bound is not invalid_before <= slot")
need(free, "check_validity_interval", r"invalid_hereafter\.is_none_or\(\|h\|\s*slot\.0\s*<\s*h\)", "the upper bound is not the half-open slot < invalid_hereafter")
need(free, "check_utxo_free_tx", r"check_validity_interval\(body,\s*slot\)\?", "the per-tx obligations skip the validity interval")
need(free, "check_utxo_free_tx", r"check_tx_networks\(body,\s*network_id\)", "the per-tx obligations skip the network checks")
need(free, "check_tx_networks", r"\.chain\(body\.collateral_return\.iter\(\)", "the output network check skips the collateral return")
need(free, "check_tx_networks", r"walk_withdrawal_accounts\(", "the withdrawal network check is gone")
need(free, "check_tx_networks", r"if\s+declared\s*!=\s*network_id", "the body network field check is gone")
need(free, "check_tx_networks", r"size\s*>\s*MAX_BOOTSTRAP_ATTRIBUTES_SIZE", "the bootstrap attribute size check is gone")
for fn in ["output_network", "address_network"]:
    need(free, fn, r"read_bootstrap_attributes\(", "a bootstrap address is not read for its network")
    need(free, fn, r"has_network_magic", "a bootstrap address's network is not its magic attribute")
need(free, "check_declared_body_size", r"tx_bodies\.len\(\)[\s\S]*witness_sets\.len\(\)[\s\S]*metadata\.len\(\)[\s\S]*invalid_txs", "the body size does not sum all four segments")
need(late, "check_address_network", r"utxo_free::address_network\(", "the full path keeps its own definition of an address's network")
need(rules, "verify_conway_witness_closure", r"check_utxo_free_tx\(&body,\s*slot,\s*network_id\)", "the block path does not check the per-tx obligations")
for fn in ["apply_shelley_era_block_with_verdicts", "apply_shelley_era_block_classified"]:
    need(rules, fn, r"check_declared_body_size\(block\)", "the block path does not check the declared body size")
need(phase1, "tx_phase_one", r"check_tx_networks\(", "the standalone path does not check the network obligations")
for b in bad:
    print("FAIL: " + b)
sys.exit(1 if bad else 0)
PYEOF

OUT=$(cargo test -p ade_ledger --test lv15a_utxo_free_admission_obligations 2>&1); RC=$?
if [ $RC -ne 0 ]; then
  fail "the LV-15a suite failed"
  printf '%s\n' "$OUT" | grep -E "panicked|FAILED|^error" | head -8
fi
PASSED=$(grep -oE "^test result: ok\. [0-9]+ passed" <<<"$OUT" | head -1 | awk '{print $4}')
[ -n "${PASSED:-}" ] && [ "$PASSED" -ge 1 ] || fail "the LV-15a suite ran no tests (vacuous gate)"
for t in \
  lv15a_the_validity_interval_is_half_open \
  lv15a_each_network_check_rejects_the_wrong_network \
  lv15a_a_bootstrap_address_network_is_its_magic_attribute \
  lv15a_bootstrap_attributes_may_total_64_bytes \
  lv15a_the_declared_body_size_must_equal_the_body \
  lv15a_the_block_path_rejects_a_block_outside_a_tx_validity_interval \
  lv15a_the_block_path_checks_the_network_in_both_modes \
  lv15a_the_standalone_path_checks_the_network \
  lv15a_the_obligations_compose; do
  grep -qE "^test ${t} \.\.\. ok$" <<<"$OUT" || fail "test did not run and pass: $t"
done

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (LV-15a: validity interval, network ids, bootstrap attributes and declared body size"
  echo "        are enforced at admission in both modes; ${PASSED} tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
