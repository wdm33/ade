#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-4d-1 (DC-EPOCH-08): the activation SOURCE WINDOW + the explicit
# source->target epoch mapping. The window that produces an activation candidate is NOT
# generically "epoch N": NAMED ROLES (source_epoch / source_window_start/end / snapshot_phase
# / target_epoch) avoid the Mark->Set off-by-one. The durable ChainDB range must be pinned to
# the selected lineage + source epoch, complete (contiguous prev_hash), ordered, and bounded;
# any violation fails closed. The lag lives in ONE named constant (a proof obligation), never
# an inline source+k.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
W=crates/ade_node/src/epoch_source_window.rs

test -f "$W" || fail "the source-window module ($W) is missing"

# (1) the named roles -- no generic "epoch N".
grep -qE 'pub struct ActivationSourceWindow' "$W" || fail "ActivationSourceWindow missing"
for r in source_epoch source_window_start source_window_end snapshot_phase target_epoch source_window_anchor lineage_pin; do
    grep -qE "pub $r" "$W" || fail "ActivationSourceWindow is missing the $r role"
done

# (2) the lag is ONE named constant (a proof obligation), derived via the explicit mapping --
#     never an inline source+k.
grep -qE 'pub const LEADERSHIP_SNAPSHOT_LAG_EPOCHS: u64' "$W" \
    || fail "the leadership-snapshot lag is not a single named constant (off-by-one hazard)"
grep -qE 'pub fn target_epoch_for_source' "$W" || fail "the explicit source->target mapping is missing"
grep -qE 'PROOF OBLIGATION' "$W" || fail "the lag is not marked as a proof obligation"

# (3) the validation is COMPLETE + ORDERED + BOUNDED + PINNED, fail-closed.
grep -qE 'pub fn validate_source_window' "$W" || fail "validate_source_window missing"
for e in Empty OutOfWindow NotOrdered Duplicate AnchorMismatch ChainGap LineageMismatch TargetEpochMismatch; do
    grep -qE "SourceWindowError::$e" "$W" || fail "the $e fail-closed reject is missing"
done
# completeness is via contiguous prev_hash links (no missing block), not just a count.
grep -qF 'b.prev_hash != p.hash' "$W" || fail "completeness is not enforced by contiguous prev_hash links"
grep -qF 'b.prev_hash != window.source_window_anchor' "$W" || fail "the window start is not anchored to the pre-window tip"

# (4) the load-bearing proofs.
for t in target_epoch_is_the_explicit_lag valid_window_passes empty_window_fails_closed \
         out_of_window_block_fails_closed unordered_and_duplicate_fail_closed \
         missing_block_breaks_the_chain anchor_and_lineage_pin_fail_closed wrong_target_epoch_fails_closed; do
    grep -qE "fn $t" "$W" || fail "the $t proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: activation source window (DC-EPOCH-08; named roles, durable-lineage-pinned complete+ordered+bounded window fail-closed, lag a single named proof-obligation constant)"
fi
exit $FAILED
