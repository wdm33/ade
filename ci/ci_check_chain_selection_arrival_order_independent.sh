#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AI AI-S5 (CE-AI-5, CN-CONS-01) — select_best_chain is
# arrival-order-independent: a fixed candidate set yields the same
# fork-choice-maximal tip regardless of presentation order. The hermetic
# permutation test over ALL orderings is the determinism proof; select_best_chain
# is the sole selection authority (DC-CONS-03), unchanged by this slice.
#
# Greps use here-strings (`<<<`), not `echo "$VAR" | grep -q`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FC="$REPO_ROOT/crates/ade_core/src/consensus/fork_choice.rs"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

[[ -f "$FC" ]] || fail "missing $FC"
SRC=$(cat "$FC")

grep -qE 'pub fn select_best_chain' <<< "$SRC" || fail "select_best_chain missing (the selection authority)"

for t in \
    select_best_chain_arrival_order_independent_distinct_heights \
    select_best_chain_arrival_order_independent_tiebreaker ; do
    grep -qF "$t" <<< "$SRC" || fail "missing permutation test: $t"
done

# The proof must enumerate ALL orderings, not a single fixed pair.
grep -qE 'fn permute' <<< "$SRC" \
    || fail "permutation generator missing (the proof must cover all candidate orderings)"

if (( FAILED == 0 )); then
    echo "OK: chain selection arrival-order-independent (CE-AI-5, CN-CONS-01)"
fi
exit $FAILED
