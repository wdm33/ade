#!/usr/bin/env bash
set -uo pipefail

# PREPROD-ENTRY-AUTHORITY P6-S4 -- a recovery divergence must stay SELF-DESCRIBING.
#
# The historical fault was `FingerprintMismatch { expected, recovered }`: two hashes and nothing else.
# Diagnosing P4 from that took hours and four wrong hypotheses. What actually cracked it was the
# per-COMPONENT fingerprints (the `snapshots` component moving across one mid-epoch block revealed an
# epoch boundary live admission never applied) and the ledger-vs-schedule epoch pair (1375 vs 1378).
#
# Nothing stops a future edit from quietly dropping the report and regressing to the opaque form --
# it would compile, and every test would still pass, because no test asserts on a fault it never
# constructs. Hence a structural gate:
#   (A) the fault variant still CARRIES a report (no regression to the bare two-hash form).
#   (B) the single construction site derives the epoch pair from real sources -- the recovered ledger
#       and the venue era schedule -- not from constants or placeholders.
#   (C) the report type retains the fields the P4 diagnosis actually needed.
#   (D) the report is rendered, not silently swallowed.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIFECYCLE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
REPORT="$REPO_ROOT/crates/ade_ledger/src/replay_divergence.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$LIFECYCLE" "$REPORT"; do
    [[ -e "$f" ]] || print_fail "missing expected path $f"
done

# (A) the variant carries a report.
VARIANT=$(awk '/FingerprintMismatch \{/{f=1} f{print} f&&/^    \},?$/{exit}' "$LIFECYCLE")
if [[ -z "$VARIANT" ]]; then
    print_fail "(A) RecoveryAdmissionFault::FingerprintMismatch not found"
elif ! grep -q 'report' <<<"$VARIANT"; then
    print_fail "(A) FingerprintMismatch no longer carries a report -- it has regressed to the opaque two-hash form that cost P4 hours"
fi

# (B) the construction site derives the epoch pair from real sources.
BUILD=$(awk '/ReplayDivergenceReport \{/{f=1} f{print} f&&/^            \};$/{exit}' "$LIFECYCLE")
if [[ -z "$BUILD" ]]; then
    print_fail "(B) the ReplayDivergenceReport construction site was not found"
else
    grep -q 'ledger_epoch: recovered.ledger.epoch_state.epoch' <<<"$BUILD" \
        || print_fail "(B) ledger_epoch is not read from the RECOVERED ledger"
    grep -q 'schedule_epoch: era_schedule.locate' <<<"$BUILD" \
        || print_fail "(B) schedule_epoch is not read from the venue era schedule"
    grep -q 'anchor:' <<<"$BUILD" \
        || print_fail "(B) the anchor fingerprint is not carried -- moved_components() would always be empty, and 'nothing moved' would be indistinguishable from 'not measured'"
fi

# (C) the report keeps the fields the P4 diagnosis needed.
for field in slot ledger_epoch schedule_epoch expected_combined actual anchor anchor_slot span_blocks store_semantics_version artifact; do
    grep -qE "^\s+pub $field:" "$REPORT" \
        || print_fail "(C) ReplayDivergenceReport lost the '$field' field"
done
# Match the METHOD DEFINITION, not any identifier containing the name. The first version of this
# check grepped 'fn moved_components', which also matched the TEST
# `fn moved_components_names_only_what_changed()` -- so renaming the real method away still passed.
# Its own negative test caught that. Pin the receiver.
grep -qE '^\s*pub fn moved_components\(&self\)' "$REPORT" \
    || print_fail "(C) moved_components(&self) is gone -- naming WHICH component diverged is the signal that identified P4's root cause"
grep -qE '^\s*pub fn epoch_disagreement\(&self\)' "$REPORT" \
    || print_fail "(C) epoch_disagreement(&self) is gone -- the P3/P4 geometry signature would go unflagged"

# (D) the report is actually emitted, not built and dropped.
grep -q 'warmstart-replay-divergence' "$LIFECYCLE" \
    || print_fail "(D) the report is never rendered -- it would exist only inside a returned error nobody prints"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_recovery_fault_self_describing: FAILED"
    exit 1
fi
echo "ci_check_recovery_fault_self_describing: OK (recovery divergence carries its own diagnosis)"
