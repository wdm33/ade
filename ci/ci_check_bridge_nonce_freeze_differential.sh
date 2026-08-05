#!/usr/bin/env bash
set -uo pipefail

# PREPROD-NONCE-2 (DC-EPOCH-38) -- the Praos nonce / candidate-freeze differential must cover
# SEED-POSITION x VENUE, not one case per venue.
#
# WHY THIS SHAPE, AND WHY A COUNT-PER-VENUE GATE IS NOT ENOUGH
# ------------------------------------------------------------
# The candidate freeze sits at `firstSlotNextEpoch - RSW`, i.e. at `1 - RSW/epoch_length` into the
# epoch. That ratio is 0.4 on EVERY venue in the closed registry (preprod 172800/432000, preview
# 34560/86400), so the freeze lands at 60% into the epoch identically everywhere. A differential that
# exercises each venue ONCE therefore cannot distinguish the two sides of the freeze at all -- which is
# exactly how preprod shipped with a defect preview never showed. Preview's flows happened to seed
# AFTER the freeze; preprod snapshot 6009 seeded 29% in, before it, and bound eta0(305) = e3402a2b..
# where cardano-node says 74f10bea.. .
#
# So this gate checks the MATRIX is present, not merely that a nonce test exists:
#   (A) the finality decision + the binder that consumes it are still wired to each other;
#   (B) the differential is driven from the CLOSED venue registry (not a hand-copied venue list that
#       can silently fall behind `resolve_network_profile`);
#   (C) BOTH sides of the freeze are exercised -- Pending AND Final -- inside the differential;
#   (D) the 0.4 ratio is ASSERTED in the test, so the reason both sides are required stays proven
#       rather than remembered;
#   (E) the tests actually run and pass.
#
# (D) is the load-bearing one. If a future venue arrives with a different ratio the assertion fails
# loudly, and whoever fixes it is forced to read why the matrix exists.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WIRE="$REPO_ROOT/crates/ade_node/src/epoch_wire.rs"
BRIDGE="$REPO_ROOT/crates/ade_ledger/src/bootstrap_bridge.rs"
DIFF_TEST="dc_epoch_38_bridge_nonce_freeze_differential_covers_both_sides_per_venue"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$WIRE" "$BRIDGE"; do
    [[ -e "$f" ]] || print_fail "missing expected path $f"
done
[[ $FAILED -eq 0 ]] || { echo "ci_check_bridge_nonce_freeze_differential: FAILED"; exit 1; }

# Read the differential test body ONCE: from its #[test] attribute line to the next line that starts a
# new test attribute, so later tests cannot satisfy checks on its behalf. Here-string, not a pipe, so
# the per-check greps do not re-read the file (the repo convention for greping large files).
BODY=$(awk -v name="fn ${DIFF_TEST}(" '
    index($0, name) { inside = 1 }
    inside && /^    #\[test\]/ && !first { first = 1 }
    inside { print }
    inside && /^    }$/ { exit }
' "$WIRE")

if [[ -z "$BODY" ]]; then
    print_fail "(0) the DC-EPOCH-38 differential test ${DIFF_TEST} is GONE from epoch_wire.rs"
    echo "ci_check_bridge_nonce_freeze_differential: FAILED"
    exit 1
fi

# --- (A) the decision and the binder stay wired together -------------------------------------------
# The binder must CONSUME a finality decision; a binder that stopped taking one would silently return
# to trusting the stored seed-time value.
if ! grep -q 'finality: BridgeEta0Finality' <<<"$(cat "$WIRE")"; then
    print_fail "(A) bind_bridge_view no longer takes a BridgeEta0Finality -- the stored seed-time nonce could become authority again"
fi
if ! grep -q 'BridgeEta0Finality::Final' <<<"$(cat "$WIRE")"; then
    print_fail "(A) the Final arm is not consumed in epoch_wire -- the cross-check has no teeth"
fi
if ! grep -q 'fn bridge_eta0_finality_at_seed' <<<"$(cat "$WIRE")"; then
    print_fail "(A) the shared seed-position derivation bridge_eta0_finality_at_seed is gone"
fi
# The view's nonce must come from the caller-supplied final eta0, never the durable field.
if ! grep -q 'final_eta0.clone()' <<<"$(cat "$WIRE")"; then
    print_fail "(A) the bound view no longer takes its nonce from the caller-supplied final eta0"
fi

# --- (B) driven from the CLOSED venue registry -----------------------------------------------------
if ! grep -q 'resolve_network_profile' <<<"$(cat "$WIRE")"; then
    print_fail "(B) the differential is not driven from resolve_network_profile -- a hand-copied venue list can fall behind the closed registry"
fi
if ! grep -q 'praos_rsw_slots' <<<"$(cat "$WIRE")"; then
    print_fail "(B) the differential does not derive RSW from praos_rsw_slots -- it must use the SAME source of truth the freeze rule uses"
fi

# --- (C) BOTH sides of the freeze, inside the differential -----------------------------------------
if ! grep -q 'PendingUntilFreeze' <<<"$BODY"; then
    print_fail "(C) the differential does not exercise the PRE-freeze (Pending) side"
fi
if ! grep -q 'BridgeEta0Finality::Final' <<<"$BODY"; then
    print_fail "(C) the differential does not exercise the AT/POST-freeze (Final) side"
fi
if ! grep -q 'BridgeNonceCrossCheck' <<<"$BODY"; then
    print_fail "(C) the differential does not prove a divergent stored nonce is TERMINAL on the Final side"
fi
# Both sides must be per-venue: the checks above must sit inside the venue loop.
if ! grep -qE 'for v in &venues' <<<"$BODY"; then
    print_fail "(C) the differential does not iterate the venue registry -- both sides must be exercised PER VENUE"
fi

# --- (D) the 0.4 ratio is asserted, not remembered -------------------------------------------------
if ! grep -qE 'rsw\) \* 5' <<<"$BODY"; then
    print_fail "(D) the differential no longer ASSERTS rsw/epoch_length = 0.4 -- the reason both sides are required becomes folklore"
fi

# --- (E) the tests run and pass --------------------------------------------------------------------
#
# `cargo test` EXITS 0 WHEN ITS FILTER MATCHES NOTHING, so a name check alone is vacuous -- the first
# version of this gate used `--exact` with a bare fn name (which needs the full module path), matched
# zero tests, reported OK, and passed two deliberate regressions: rebinding the stored seed-time value,
# and deleting the Final cross-check. Both were caught only by negative-testing the gate itself. So
# every run below asserts a NONZERO passed-count, not just an exit status.
run_tests() {
    local label="$1" crate="$2" filter="$3" out passed
    out=$(cargo test -p "$crate" --lib "$filter" 2>&1)
    if [[ $? -ne 0 ]]; then
        print_fail "(E) $label did not pass"
        return
    fi
    passed=$(grep -oE 'test result: ok\. [0-9]+ passed' <<<"$out" | grep -oE '[0-9]+' | head -1)
    if [[ -z "$passed" || "$passed" -eq 0 ]]; then
        print_fail "(E) $label matched ZERO tests -- the filter is stale, so this check proves nothing"
    fi
}
run_tests "the DC-EPOCH-38 differential" ade_node "epoch_wire::tests::${DIFF_TEST}"
run_tests "the CE-N2-4 binder tests" ade_node "epoch_wire::tests::n2_"
run_tests "the bridge_eta0_finality decision tests" ade_ledger "nonce2_finality_tests"

if [[ $FAILED -ne 0 ]]; then
    echo "ci_check_bridge_nonce_freeze_differential: FAILED"
    exit 1
fi
echo "ci_check_bridge_nonce_freeze_differential: OK (seed-position x venue matrix; both sides of the freeze per venue)"
