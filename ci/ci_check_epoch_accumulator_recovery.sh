#!/usr/bin/env bash
set -uo pipefail

# DC-EPOCH-20 (LIVE-LEDGER-EPOCH-TRANSITION S2): atomic-or-rematerialized selected-block admission --
# no RESUMED split authority. The EpochAccumulator is the fourth derived store; on warm-start or reorg it
# must rematerialize to the WAL-tail prefix by FOLDING FORWARD over the canonical durable blocks, never by
# ad hoc inverse mutation. Mechanical enforcement (IDD principle 10) of the recovery-fold structure that the
# behavioural unit tests (over_chaindb_*) exercise but do not pin structurally:
#   (A) NO INVERSE MUTATION: the durable store + the advancer expose no subtractive op. The ONLY "undo" is
#       reset_to_bootstrap (restore the sealed seed); recovery is reset + forward replay.
#   (B) FORWARD-FOLD REMATERIALIZE: advance_accumulator_over_chaindb walks the durable ChainDB and folds via
#       advance_accumulator_over_block -> the BLUE apply_selected_block transition (not a bespoke re-derive).
#   (C) OBSERVE-ONLY WRAPPER: advance_accumulator_to_durable_tip (the live/recovery call site) returns unit
#       -- it CANNOT propagate, so a stall/fault never halts the proven follow (S2 PO-6). It detects a reorg
#       (overshoot) and calls reset_to_bootstrap before replaying.
#   (D) FAIL-CLOSED READINESS: the readiness gate is terminal (Lagging / Ahead / Unsealed), so a derived
#       store that is not EXACTLY at the WAL tail is caught, never run on.
#   (E) DC-EPOCH-20 is in the registry.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ADV="$REPO_ROOT/crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs"
STORE="$REPO_ROOT/crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs"
LIFECYCLE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$ADV" "$STORE" "$LIFECYCLE" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done
[[ $FAILED -eq 0 ]] || exit 1

# (A) NO INVERSE MUTATION. Neither the store nor the advancer may define a subtractive / inverse method --
#     the accumulator only ever moves forward; the sole reversal is reset_to_bootstrap. (Match `fn` decls
#     only, so a comment that says "no decrement path" cannot false-FAIL.)
FORBIDDEN_INVERSE='fn[[:space:]]+(decrement|subtract|deduct|undo|unapply|revert|invert|rollback_block|remove_block)'
if grep -Eq "$FORBIDDEN_INVERSE" "$STORE" "$ADV"; then
    print_fail "(A) a subtractive/inverse method exists on the accumulator store/advancer -- recovery must reset+replay, never invert"
fi
grep -Eq 'fn[[:space:]]+reset_to_bootstrap' "$STORE" \
    || print_fail "(A) reset_to_bootstrap (the ONLY sanctioned undo: restore the sealed seed) is missing"

# (B) FORWARD-FOLD REMATERIALIZE. The ChainDB walk exists and folds via the per-block advancer, which in
#     turn applies the BLUE transition -- recovery reuses the proven contract, never a bespoke re-derive.
grep -Eq 'fn[[:space:]]+advance_accumulator_over_chaindb' "$ADV" \
    || print_fail "(B) advance_accumulator_over_chaindb (the durable forward-fold walk) is missing"
grep -q 'advance_accumulator_over_block' "$ADV" \
    || print_fail "(B) the ChainDB walk does not fold via advance_accumulator_over_block"
grep -q 'apply_selected_block' "$ADV" \
    || print_fail "(B) the advancer does not apply the BLUE apply_selected_block transition"

# (C) OBSERVE-ONLY WRAPPER. The live/recovery call site must be unit-returning (cannot propagate) and must
#     drive reorg recovery through reset_to_bootstrap + the forward walk.
# Capture the full signature (fn line through the opening brace) -- the params span several lines, so the
# return-type position is not on the `fn` line; a fixed -A window would miss it.
WRAPPER_SIG="$(awk '/fn advance_accumulator_to_durable_tip/{f=1} f{print} f&&/\{/{exit}' "$LIFECYCLE")"
[[ -n "$WRAPPER_SIG" ]] || print_fail "(C) advance_accumulator_to_durable_tip (the observe-only wrapper) is missing"
if grep -q '\-> Result' <<<"$WRAPPER_SIG"; then
    print_fail "(C) advance_accumulator_to_durable_tip returns Result -- it must be unit-returning (observe-only, cannot halt the follow)"
fi
grep -q 'reset_to_bootstrap' "$LIFECYCLE" \
    || print_fail "(C) the wrapper does not reset_to_bootstrap on reorg (no rematerialize path)"
grep -q 'advance_accumulator_over_chaindb' "$LIFECYCLE" \
    || print_fail "(C) the wrapper does not drive the forward-fold walk"

# (D) FAIL-CLOSED READINESS. The gate is terminal on any non-exact alignment to the WAL tail.
grep -q 'enum AccumulatorReadinessError' "$STORE" \
    || print_fail "(D) AccumulatorReadinessError (the fail-closed readiness gate) is missing"
for variant in Lagging Ahead Unsealed; do
    grep -q "$variant" "$STORE" \
        || print_fail "(D) AccumulatorReadinessError is missing the terminal $variant variant"
done

# (E) DC-EPOCH-20 in the registry.
grep -q 'DC-EPOCH-20' "$REG" \
    || print_fail "(E) DC-EPOCH-20 is not declared in the invariant registry"

if [[ $FAILED -ne 0 ]]; then
    echo "DC-EPOCH-20 epoch-accumulator recovery-fold check FAILED" >&2
    exit 1
fi
echo "OK: epoch-accumulator recovery is reset+forward-fold (no inverse op; observe-only wrapper; fail-closed readiness)"
