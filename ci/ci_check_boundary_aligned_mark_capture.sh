#!/usr/bin/env bash
set -uo pipefail

# DC-EPOCH-22 (LIVE-LEDGER-EPOCH-TRANSITION S3, BOUNDARY-ALIGNED-MARK-CAPTURE): the live epoch-boundary
# stake mark is captured ONLY at the EXACT selected-chain boundary point (s_prev, the last durable block of
# the closing epoch) -- never at a later catch-up tip, never via a per-block scan -- durably bound as a
# BoundaryMark witness BEFORE the accumulator crosses. This is realized by a co-advancer in node_lifecycle
# that SEGMENTS the reduced-checkpoint + accumulator advance at each boundary: at a boundary stall it brings
# the checkpoint exactly to s_prev, captures sum_base_credential_stake() there, binds the witness, then
# crosses the accumulator with that mark. Mechanical enforcement (IDD principle 10) of the wiring the unit
# tests (co_advance_ledger_state::*) exercise but do not pin structurally:
#   (A) the #2b-i accumulator boundary-cross entry point exists.
#   (B) the #2b-ii durable BoundaryMark witness exists AND reset_to_bootstrap drops it (reorg invalidation).
#   (C) the #2b-iii co-advancer is wired -- it binds the witness, captures the mark, and crosses.
#   (D) the call site uses the SINGLE co-advancer; the pre-S3 two-call split (advance_accumulator_to_durable_tip
#       beside advance_reduced_checkpoint_to_durable_tip) is GONE (subsumed -- no resumed split prefix).
#   (E) DC-EPOCH-22 is in the registry.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ADV="$REPO_ROOT/crates/ade_runtime/src/chaindb/epoch_accumulator_advance.rs"
STORE="$REPO_ROOT/crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs"
NODE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$ADV" "$STORE" "$NODE" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done
[[ $FAILED -eq 0 ]] || exit 1

# (A) #2b-i: the accumulator boundary-cross entry point.
grep -q 'fn cross_accumulator_over_boundary_block' "$ADV" \
    || print_fail "(A) cross_accumulator_over_boundary_block is not defined in epoch_accumulator_advance.rs"

# (B) #2b-ii: the durable BoundaryMark witness + reorg invalidation.
grep -q 'fn bind_boundary_mark' "$STORE" \
    || print_fail "(B) bind_boundary_mark is not defined in epoch_accumulator_store.rs"
grep -q 'fn boundary_mark_binding' "$STORE" \
    || print_fail "(B) boundary_mark_binding is not defined in epoch_accumulator_store.rs"
grep -q 'fn reset_to_bootstrap' "$STORE" \
    || print_fail "(B) reset_to_bootstrap is not defined in epoch_accumulator_store.rs"
grep -Eq 'remove\(PENDING_BOUNDARY_MARK_KEY\)' "$STORE" \
    || print_fail "(B) reset_to_bootstrap does not drop the pending boundary-mark binding (remove(PENDING_BOUNDARY_MARK_KEY))"

# (C) #2b-iii: the co-advancer in node_lifecycle binds the witness, captures the mark, and crosses.
grep -q 'fn advance_ledger_state_to_durable_tip' "$NODE" \
    || print_fail "(C) the co-advancer advance_ledger_state_to_durable_tip is not defined in node_lifecycle.rs"
grep -q 'bind_boundary_mark' "$NODE" \
    || print_fail "(C) the co-advancer does not bind the BoundaryMark witness (bind_boundary_mark)"
grep -q 'sum_base_credential_stake' "$NODE" \
    || print_fail "(C) the co-advancer does not capture the boundary mark (sum_base_credential_stake)"
grep -q 'cross_accumulator_over_boundary_block' "$NODE" \
    || print_fail "(C) the co-advancer does not cross the accumulator (cross_accumulator_over_boundary_block)"

# (D) SINGLE co-advancer at the call site; the pre-S3 two-call split is subsumed. The old observe-only
#     accumulator wrapper advance_accumulator_to_durable_tip must no longer be defined OR called (a call has
#     a paren; assert the name is fully absent so neither remains).
grep -q 'advance_ledger_state_to_durable_tip(' "$NODE" \
    || print_fail "(D) the single co-advancer advance_ledger_state_to_durable_tip( is not called in node_lifecycle.rs"
if grep -q 'advance_accumulator_to_durable_tip' "$NODE"; then
    print_fail "(D) advance_accumulator_to_durable_tip still present -- the pre-S3 two-call split must be subsumed by the single co-advancer"
fi

# (E) DC-EPOCH-22 in the registry.
grep -q 'DC-EPOCH-22' "$REG" \
    || print_fail "(E) DC-EPOCH-22 is not declared in the invariant registry"

if [[ $FAILED -ne 0 ]]; then
    echo "DC-EPOCH-22 boundary-aligned-mark-capture check FAILED" >&2
    exit 1
fi
echo "OK: the boundary-aligned mark capture is wired (entry point; durable witness + reorg drop; co-advancer bind/capture/cross; single co-advancer call site)"
