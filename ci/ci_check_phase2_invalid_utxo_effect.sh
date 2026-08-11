#!/usr/bin/env bash
set -uo pipefail

# BND-2a (INV-BND-2a) -- a phase-2-invalid transaction's UTxO effect is COLLATERAL-ONLY, derived in
# exactly ONE place, and its consumers are validity-blind.
#
# The regression this prevents was live and silent. `reduced_block_delta` and `track_utxo` applied
# every transaction's ordinary inputs and outputs regardless of phase-2 validity and never consumed
# collateral, so Ade's reduced UTxO -- the STAKE AUTHORITY that seals frozen leadership at every
# boundary (DC-EPOCH-32/33) -- diverged from cardano-ledger on preprod block 130,350,133 without
# anything announcing it. Reference: Cardano.Ledger.Babbage.Rules.Utxo, Phase2Invalid; extraction in
# docs/evidence/run-stores/preprod-live2c/bnd2-oracle-extraction.md.
#
# This gate asserts STRUCTURE, not merely that tests exist: reintroducing a second rule path, letting
# a consumer branch on validity itself, placing the collateral return at a positional index, or
# reading total_collateral as the source of truth each fail here even if everything compiles.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

RULES="crates/ade_ledger/src/rules.rs"
RED="crates/ade_ledger/src/reduced_advance.rs"
SEM="crates/ade_ledger/src/store_semantics.rs"
for f in "$RULES" "$RED" "$SEM"; do
  [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

# 1. ONE derivation, and it is validity-aware.
grep -q "fn extract_tx_utxo_effect" "$RULES" || fail "extract_tx_utxo_effect is gone"
grep -q "phase2_invalid: bool" "$RULES" || fail "the effect derivation no longer takes the phase-2 flag"

# 2. BOTH consumers route through it.
grep -q "extract_tx_utxo_effect(" "$RED" || fail "reduced_advance does not use the shared effect derivation"
grep -q "extract_tx_utxo_effect(" "$RULES" || fail "track_utxo does not use the shared effect derivation"

# 3. Consumers are VALIDITY-BLIND: they may supply the tx index, but must not implement the rule.
python3 - <<'PYEOF' || FAILED=1
import re, sys
bad = []
def body(path, start_marker, end_marker):
    src = open(path).read()
    i = src.find(start_marker)
    if i < 0: return None
    j = src.find(end_marker, i)
    return src[i:j if j > i else len(src)]

# track_utxo's body
tu = body("crates/ade_ledger/src/rules.rs", "pub(crate) fn track_utxo(", "\n/// Conway vkey-witness")
# process_one_tx's body
po = body("crates/ade_ledger/src/reduced_advance.rs", "fn process_one_tx(", "\n/// Advance the cert")
for name, b in (("track_utxo", tu), ("process_one_tx", po)):
    if b is None:
        bad.append(f"could not isolate {name}"); continue
    # A consumer implementing the rule would branch on the flag and choose different fields.
    for pat in (r"if\s+invalid", r"if\s+phase2_invalid", r"collateral_inputs", r"collateral_return"):
        if re.search(pat, b):
            bad.append(f"{name} implements the phase-2 rule itself (matched /{pat}/)")
for m in bad: print("FAIL: " + m)
sys.exit(1 if bad else 0)
PYEOF

# 4. The collateral return goes at len(ordinary outputs), NOT a positional index.
grep -q "tx.outputs.len() as u16" "$RULES" \
  || fail "the collateral return is not indexed at len(ordinary outputs) (mkCollateralTxIn)"

# 5. total_collateral is NOT the source of truth in the effect derivation.
python3 - <<'PYEOF' || FAILED=1
import sys
src = open("crates/ade_ledger/src/rules.rs").read()
i = src.find("pub(crate) fn extract_tx_utxo_effect(")
j = src.find("pub(crate) fn extract_inputs_outputs_from_tx(", i)
fn = src[i:j] if i >= 0 and j > i else ""
if not fn:
    print("FAIL: could not isolate extract_tx_utxo_effect"); sys.exit(1)
if "total_collateral" in fn:
    print("FAIL: extract_tx_utxo_effect reads total_collateral -- it is a declared assertion, not the source of truth")
    sys.exit(1)
print("ok: the effect derivation does not read total_collateral")
PYEOF

# 6. The store-semantics marker moved (an old artifact must be refused, never reinterpreted).
V=$(grep -oE "pub const STORE_SEMANTICS_VERSION: u32 = [0-9]+" "$SEM" | grep -oE "[0-9]+$")
if [ -z "${V:-}" ] || [ "$V" -lt 4 ]; then
  fail "STORE_SEMANTICS_VERSION must be >= 4 for the phase-2-invalid UTxO semantics (found: ${V:-none})"
fi

# 7. The named tests must exist AND run (nonzero passing count -- no vacuous gate).
T="crates/ade_ledger/tests/bnd2a_phase2_invalid_utxo_effect.rs"
[ -f "$T" ] || fail "missing $T"
for t in real_preprod_block_130350133_produces_the_cardano_collateral_only_effect \
         a_collateral_return_is_produced_at_len_ordinary_outputs_with_verbatim_bytes \
         a_pre_alonzo_invalid_transaction_fails_closed \
         a_block_with_no_invalid_transactions_is_unchanged; do
  grep -q "fn $t" "$T" || fail "missing test: $t"
done

OUT=$(cargo test -p ade_ledger --test bnd2a_phase2_invalid_utxo_effect 2>&1)
RC=$?
if [ $RC -ne 0 ]; then
  fail "the BND-2a differential suite failed"
  printf '%s\n' "$OUT" | tail -25
fi
PASSED=$(printf '%s\n' "$OUT" | grep -oE "^test result: ok\. [0-9]+ passed" | head -1 | awk '{print $4}')
if [ -z "${PASSED:-}" ] || [ "$PASSED" -lt 4 ]; then
  fail "expected >=4 passing differential tests, got ${PASSED:-0} (vacuous gate)"
fi

if [ $FAILED -eq 0 ]; then
  echo "RESULT: PASS (BND-2a: collateral-only invalid effect, one derivation, validity-blind consumers, store v$V; $PASSED tests ran)"
  exit 0
fi
echo "RESULT: FAIL"
exit 1
