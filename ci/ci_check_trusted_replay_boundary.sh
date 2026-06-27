#!/usr/bin/env bash
set -uo pipefail

# Warm-start multi-boundary recovery -- the TrustDurable trust boundary. The leader-eligibility-SKIPPING
# block-validity variant must be reachable ONLY from the durable replay (materialize_rolled_back_state),
# NEVER from a live / untrusted (peer / admission) entry point. Mechanical enforcement (IDD principle 10)
# of the boundary the security review verified by hand:
#   (A) block_validity_trusted_replay is pub(crate), NOT bare pub (no cross-crate reach).
#   (B) it has EXACTLY ONE call site, and that site is the durable replay (rollback/materialize.rs).
#   (C) LeaderEligibility::TrustDurable is CONSTRUCTED in exactly ONE place (the trusted-replay wrapper).
#   (D) neither the trust-skipping fn nor TrustDurable appears in the live receive / admission paths.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="$REPO_ROOT/crates"
TRANS="$CRATES/ade_ledger/src/block_validity/transition.rs"
MAT="$CRATES/ade_ledger/src/rollback/materialize.rs"
RECV="$CRATES/ade_ledger/src/receive"
ADMIT="$CRATES/ade_node/src/admission"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$TRANS" "$MAT"; do
    [[ -e "$f" ]] || print_fail "missing expected path $f"
done

# (A) pub(crate), not bare pub.
grep -Eq 'pub\(crate\) fn block_validity_trusted_replay' "$TRANS" \
    || print_fail "(A) block_validity_trusted_replay is not pub(crate) -- the trust boundary is convention-only"
if grep -Eq '^[[:space:]]*pub fn block_validity_trusted_replay' "$TRANS"; then
    print_fail "(A) block_validity_trusted_replay is bare pub -- cross-crate reachable"
fi

# (B) EXACTLY ONE call site, and it is the durable replay.
CALLS=$(grep -rn 'block_validity_trusted_replay(' "$CRATES" --include=*.rs \
    | grep -v 'fn block_validity_trusted_replay(' || true)
N=$(printf '%s' "$CALLS" | grep -c . || true)
if [[ "$N" -ne 1 ]]; then
    print_fail "(B) block_validity_trusted_replay must have EXACTLY ONE caller, found $N: ${CALLS:-<none>}"
fi
printf '%s' "$CALLS" | grep -q 'rollback/materialize.rs' \
    || print_fail "(B) the single block_validity_trusted_replay caller is not rollback/materialize.rs: ${CALLS:-<none>}"

# (C) LeaderEligibility::TrustDurable constructed in exactly one place (the wrapper).
TD=$(grep -rn 'LeaderEligibility::TrustDurable' "$CRATES" --include=*.rs || true)
TDN=$(printf '%s' "$TD" | grep -c . || true)
if [[ "$TDN" -ne 1 ]]; then
    print_fail "(C) LeaderEligibility::TrustDurable must be constructed in EXACTLY ONE place, found $TDN: ${TD:-<none>}"
fi
printf '%s' "$TD" | grep -q 'block_validity/transition.rs' \
    || print_fail "(C) the single TrustDurable construction is not in block_validity/transition.rs: ${TD:-<none>}"

# (D) the trust-skipping variant never appears in the live receive / admission paths.
if grep -rnE 'block_validity_trusted_replay|TrustDurable' "$RECV" "$ADMIT" --include=*.rs 2>/dev/null; then
    print_fail "(D) the trust-skipping replay variant appears in a live receive/admission path"
fi

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_trusted_replay_boundary: FAILED"
    exit 1
fi
echo "ci_check_trusted_replay_boundary: OK (TrustDurable reachable only from durable replay)"
