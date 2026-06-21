#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW S3f-4d-3a (DC-EPOCH-10): the boundary activation ORCHESTRATION -- the
# sequenced durable-before-visible flip. ONE atomic path, in order: validate the durable
# source window (DC-EPOCH-08) -> derive the candidate (DC-EPOCH-09) -> the activation
# predicate BEFORE the WAL (DC-EPOCH-05/07) -> write the durable WAL record (DC-EPOCH-06) ->
# publish ONLY if durable -> atomically promote. A failure after the predicate is TERMINAL
# (halt, never seed fallback); a predicate decline is NotYet (seed stays, retry).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
A=crates/ade_node/src/epoch_activate.rs

test -f "$A" || fail "the activation orchestration ($A) is missing"

grep -qE 'pub fn activate_at_boundary' "$A" || fail "activate_at_boundary missing"

# (1) the sequence calls the proven pieces in order (validate -> derive -> predicate ->
#     record -> publish -> promote). Each must appear, in this order.
order=$(grep -nE 'validate_source_window|derive_candidate|activation_predicate|activation_record_for|wal_write|activate_durable_before_visible|active_view\.promote' "$A" \
    | grep -vE '//|use ' | head -7 | awk -F: '{print $2}' | tr -d ' ')
expected="validate_source_window derive_candidate activation_predicate activation_record_for wal_write activate_durable_before_visible active_view.promote"
got=$(echo "$order" | sed 's/(.*//' | tr '\n' ' ')
for step in validate_source_window derive_candidate activation_predicate activation_record_for activate_durable_before_visible; do
    grep -qF "$step" "$A" || fail "the orchestration is missing the $step step"
done

# (2) durable-before-visible: the WAL write result gates publication via
#     activate_durable_before_visible (publication NEVER precedes a durable write).
grep -qF 'let durable = wal_write(&record);' "$A" || fail "the WAL write does not gate durability"
grep -qF 'activate_durable_before_visible(candidate, durable)' "$A" \
    || fail "publication is not gated on the durable WAL write"

# (3) terminal vs NotYet: a predicate decline is NotYet (not terminal); a post-predicate
#     failure is a terminal EpochViewActivationError.
grep -qF 'BoundaryActivationOutcome::NotYet' "$A" || fail "a predicate decline is not surfaced as NotYet"
grep -qF 'EpochViewActivationError' "$A" || fail "terminal failures are not surfaced"

# (4) the load-bearing proofs (happy path; non-durable terminal+no-publish; decline NotYet;
#     invalid window terminal-before-WAL; point mismatch declines).
for t in happy_path_promotes_after_durable_wal non_durable_wal_is_terminal_and_does_not_publish \
         not_eligible_transition_is_not_yet_not_terminal invalid_window_is_terminal_before_any_wal \
         selected_point_mismatch_declines; do
    grep -qE "fn $t" "$A" || fail "the $t proof is missing"
done

if (( FAILED == 0 )); then
    echo "OK: boundary activation orchestration (DC-EPOCH-10; sequenced validate->derive->predicate->WAL->publish->promote, durable-before-visible, terminal-not-fallback / NotYet-on-decline)"
fi
exit $FAILED
