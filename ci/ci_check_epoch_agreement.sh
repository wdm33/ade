#!/usr/bin/env bash
set -uo pipefail

# PREPROD-ENTRY-AUTHORITY P5 (DC-EPOCH-36) -- the epoch-agreement invariant must stay UNFORGETTABLE.
#
# P4 (e1de7a2e) proved what its absence costs: a preview store ran its entire life with
# ledger_epoch=1375 against schedule_epoch=1378, because `detect_epoch_transition` fires only on
# `schedule > ledger` and NOTHING compared the two authorities in the other direction. The fix is not
# "remember to call check_epoch_agreement" -- three call sites open-coded detect-then-dispatch, and a
# fourth that forgot would silently reopen the hole. The fix is that detection is only reachable from
# ONE function, which pairs it with the check. This gate enforces that mechanically (IDD principle 10):
#   (A) detect_epoch_transition is pub(crate), NOT bare pub (no cross-crate reach).
#   (B) it has EXACTLY ONE non-test call site, and that site is cross_epoch_boundary_for_slot.
#   (C) cross_epoch_boundary_for_slot actually calls check_epoch_agreement (the pairing is real).
#   (D) check_epoch_agreement is total on the failure path -- it maps to a typed LedgerError, never
#       a panic/unwrap/expect.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="$REPO_ROOT/crates"
STATE="$CRATES/ade_ledger/src/state.rs"
RULES="$CRATES/ade_ledger/src/rules.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$STATE" "$RULES"; do
    [[ -e "$f" ]] || print_fail "missing expected path $f"
done

# (A) pub(crate), not bare pub.
grep -Eq 'pub\(crate\) fn detect_epoch_transition' "$STATE" \
    || print_fail "(A) detect_epoch_transition is not pub(crate) -- the pairing is convention-only"
if grep -Eq '^[[:space:]]*pub fn detect_epoch_transition' "$STATE"; then
    print_fail "(A) detect_epoch_transition is bare pub -- reachable without the agreement check"
fi

# (B) EXACTLY ONE non-test call site, and it is the single crossing point. `#[cfg(test)]` modules
#     legitimately exercise detection directly, so unit-test files are excluded by path.
CALLS=$(grep -rn 'detect_epoch_transition(' "$CRATES" --include=*.rs \
    | grep -v 'fn detect_epoch_transition(' \
    | grep -v '/tests/' \
    | grep -v 'ade_ledger/src/state.rs' || true)
N=$(printf '%s' "$CALLS" | grep -c . || true)
if [[ "$N" -ne 1 ]]; then
    print_fail "(B) detect_epoch_transition must have EXACTLY ONE non-test caller, found $N: ${CALLS:-<none>}"
fi
printf '%s' "$CALLS" | grep -q 'ade_ledger/src/rules.rs' \
    || print_fail "(B) the single detect_epoch_transition caller is not rules.rs: ${CALLS:-<none>}"

# (C) the single crossing point genuinely pairs detection with the check. Read the function body from
#     its signature to the next top-level `fn` and require check_epoch_agreement inside it. A here-string
#     keeps the grep off a large file (repo convention).
BODY=$(awk '/^fn cross_epoch_boundary_for_slot\(/{f=1} f{print} f&&/^}/{exit}' "$RULES")
if [[ -z "$BODY" ]]; then
    print_fail "(C) cross_epoch_boundary_for_slot not found in rules.rs -- the single crossing point is gone"
else
    grep -q 'check_epoch_agreement' <<<"$BODY" \
        || print_fail "(C) cross_epoch_boundary_for_slot does not call check_epoch_agreement -- detection is unpaired"
    grep -q 'detect_epoch_transition' <<<"$BODY" \
        || print_fail "(C) cross_epoch_boundary_for_slot does not call detect_epoch_transition"
fi

# (D) the check fails CLOSED as a typed error, never a panic.
CHECK_BODY=$(awk '/^pub fn check_epoch_agreement\(/{f=1} f{print} f&&/^}/{exit}' "$STATE")
if [[ -z "$CHECK_BODY" ]]; then
    print_fail "(D) check_epoch_agreement not found in state.rs"
else
    if grep -Eq 'panic!|unwrap\(\)|expect\(' <<<"$CHECK_BODY"; then
        print_fail "(D) check_epoch_agreement contains a panic path -- an invariant guard must fail closed, not abort"
    fi
fi
grep -q 'EpochAgreement(crate::state::EpochAgreementViolation)' "$CRATES/ade_ledger/src/error.rs" \
    || print_fail "(D) LedgerError::EpochAgreement is missing -- the violation has no typed surface"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_epoch_agreement: FAILED"
    exit 1
fi
echo "ci_check_epoch_agreement: OK (epoch boundary crossings are paired with the agreement check)"
