#!/usr/bin/env bash
set -uo pipefail

# DC-LEDGER-09: Conway governance-certificate accumulation is a closed, total,
# era-versioned transition into ConwayGovState. This gate defends that the
# governance certs B4 owner-tagged are APPLIED (not observed-and-dropped) and
# that the apply surface stays closed — CI-enforced, not only test-covered:
#
#   1. apply_conway_gov_cert exists and its match over ConwayCert has NO `_ =>`
#      wildcard arm, so adding a ConwayCert variant breaks the build instead of
#      silently dropping its governance effect (crates/ade_ledger/src/gov_cert.rs).
#   2. The B4 observe-and-drop is GONE from the accumulation path — the
#      "routed out of B4 mutation scope" comment must not reappear, and
#      accumulate_tx_certs must call apply_conway_gov_cert
#      (crates/ade_ledger/src/rules.rs).
#   3. The DRep-expiry env fail-fast is wired — MissingDRepActivityParam exists
#      and gov_cert_env() is the only constructor of GovCertEnv
#      (crates/ade_ledger/src/{error.rs,state.rs}).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

GOV_CERT="$REPO_ROOT/crates/ade_ledger/src/gov_cert.rs"
RULES="$REPO_ROOT/crates/ade_ledger/src/rules.rs"
ERROR="$REPO_ROOT/crates/ade_ledger/src/error.rs"
STATE="$REPO_ROOT/crates/ade_ledger/src/state.rs"

FAIL=0

for f in "$GOV_CERT" "$RULES" "$ERROR" "$STATE"; do
    if [ ! -f "$f" ]; then
        echo "FAIL: expected gov-accumulation file missing: $f"
        FAIL=1
    fi
done
[ "$FAIL" -eq 0 ] || exit 1

# 1. apply_conway_gov_cert present, no wildcard arm in its dispatch.
if ! grep -q 'pub fn apply_conway_gov_cert' "$GOV_CERT"; then
    echo "FAIL: apply_conway_gov_cert missing from gov_cert.rs"
    FAIL=1
fi
if grep -Eq '^\s*_\s*=>' "$GOV_CERT"; then
    echo "FAIL: gov_cert.rs has a catch-all '_ =>' arm — the gov dispatch must stay exhaustive"
    FAIL=1
fi

# 2. B4 observe-and-drop is gone; accumulate_tx_certs applies the gov half.
if grep -q 'routed out of B4 mutation scope' "$RULES"; then
    echo "FAIL: the B4 observe-and-drop comment reappeared in rules.rs — gov certs must be applied"
    FAIL=1
fi
if ! grep -q 'apply_conway_gov_cert' "$RULES"; then
    echo "FAIL: rules.rs accumulate_tx_certs does not call apply_conway_gov_cert"
    FAIL=1
fi

# 3. env fail-fast wired.
if ! grep -q 'MissingDRepActivityParam' "$ERROR"; then
    echo "FAIL: ValidationEnvironmentError::MissingDRepActivityParam missing from error.rs"
    FAIL=1
fi
if ! grep -q 'fn gov_cert_env' "$STATE"; then
    echo "FAIL: LedgerState::gov_cert_env() missing from state.rs"
    FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: DC-LEDGER-09 gov-cert accumulation surface is closed and applied"
fi
exit "$FAIL"
