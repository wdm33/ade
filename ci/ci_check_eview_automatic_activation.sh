#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONTINUITY-ACTIVATION ECA-1 (DC-EPOCH-13): no semantic activation gate. No build- or
# runtime-level switch decides WHETHER the epoch-view activation occurs. Activation is AUTOMATIC
# and DETERMINISTIC -- the ONLY gate is the activation predicate over canonical durable state.
# This gate proves the forbidden scaffold (the EVIEW_ACTIVATION_ARMED const / an `armed` bool
# parameter / an `if !armed` short-circuit / any equivalent flag) appears NOWHERE in crates/, and
# that the orchestration gates ONLY on deterministic boundary detection + the canonical-state
# presence + the predicate.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
EW=crates/ade_node/src/epoch_wire.rs
NL=crates/ade_node/src/node_lifecycle.rs

# (1) NEGATIVE: the arming flag is gone from the entire source tree (no const, no equivalent).
if grep -rn 'EVIEW_ACTIVATION_ARMED' crates/ >/dev/null 2>&1; then
    fail "EVIEW_ACTIVATION_ARMED still present in crates/ -- the semantic activation gate must be removed (DC-EPOCH-13)"
fi

# (2) NEGATIVE: no `armed: bool` parameter and no `if !armed` short-circuit in the orchestration.
if grep -nE '^[[:space:]]*armed:[[:space:]]*bool' "$EW" >/dev/null 2>&1; then
    fail "an 'armed: bool' parameter remains in epoch_wire.rs -- activation must not be flag-gated"
fi
if grep -nF 'if !armed' "$EW" >/dev/null 2>&1; then
    fail "an 'if !armed' short-circuit remains in epoch_wire.rs -- activation must not be flag-gated"
fi

# (3) POSITIVE: the orchestration gates on DETERMINISTIC boundary detection (the era schedule, not
#     a flag, not the wall clock) AND the idempotent promoted check.
grep -qF 'era_schedule.locate(durable_tip_slot)' "$EW" || fail "the orchestration does not gate on deterministic boundary detection (era_schedule.locate)"
grep -qF 'if tip_epoch.0 <= seed_epoch.0' "$EW" || fail "the orchestration does not gate on the seed-epoch-completed boundary condition"
grep -qF 'if active_view.promoted().is_some()' "$EW" || fail "the orchestration is not idempotent once a view is promoted"

# (4) POSITIVE: the relay-loop activation is keyed on CANONICAL STATE -- it runs only when EVIEW is
#     configured (the cert-state package + reduced checkpoint present), never a flag.
grep -qF 'let (Some(inputs), Some(live)) = (eview_activation, reduced_checkpoint) else {' "$NL" \
    || fail "the relay-loop activation is not keyed on canonical state (Some inputs + Some checkpoint)"

# (5) POSITIVE: the sole authoritative path is the durable window replay -> the atomic
#     activate_at_boundary; the PREDICATE decides, no flag.
grep -qF 'activate_at_boundary(' "$EW" || fail "the orchestration does not call the atomic activate_at_boundary (the predicate path)"

# (6) the proof: a crossed boundary AUTOMATICALLY drives the activation (fail-closed here on an
#     empty window), not a flag no-op.
grep -qE 'fn maybe_activate_first_boundary_is_automatic_and_fails_closed_not_flag_gated' "$EW" \
    || fail "the ECA-1 automatic-activation proof is missing"

if (( FAILED == 0 )); then
    echo "OK: no semantic activation gate (DC-EPOCH-13; EVIEW_ACTIVATION_ARMED + the armed param/guard removed from crates/; activation gates ONLY on deterministic boundary detection + canonical-state presence + the predicate)"
fi
exit $FAILED
