#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-B S3 — served-chain handoff fence.
#
# Closes the DC-NODE-06 serve-ingress clause for the `--mode node` spine: the
# node-spine served chain is fed ONLY by a BLUE self-accepted artifact carried
# through the S1 handoff's `into_accepted()` into the single
# `ServedChainHandle::push_atomic` authority. Raw forge bytes, a failed forge
# outcome (`ForgeNotLeader` / `ForgeFailed`), a self-declared acceptance flag,
# and a peer-verdict substitute are banned from the serve ingress.
#
# Scope: the `--mode node` lifecycle owner `crates/ade_node/src/node_lifecycle.rs`,
# production code only (line comments + the `#[cfg(test)]` module stripped).
#
# Guards (scoped to the stripped production body):
#   (1) every `push_atomic(` on the node spine is fed by `into_accepted()`
#       (no raw-bytes / non-handoff serve ingress);
#   (2) no direct `served_chain_admit(` on the node spine — served-chain
#       mutation happens ONLY through the single `push_atomic` authority;
#   (3) the handoff channel is typed `UnboundedSender<SelfAcceptedHandoff>` and
#       NOT `<Vec<u8>>` / `<ForgedBlockArtifact>` / `<bool>` (the serve-ingress
#       carrier is the S1 self-accepted fence, never raw bytes / a flag).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OWNER="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAIL=0
print_fail() { echo "FAIL (served-chain handoff fence): $1"; FAIL=1; }

if [[ ! -f "$OWNER" ]]; then
    echo "FAIL (served-chain handoff fence): lifecycle owner not found at $OWNER"
    echo "FAIL: ci_check_served_chain_handoff_fence"
    exit 1
fi

# Strip the `#[cfg(test)]` module (attribute to EOF) + line comments, so the
# greps see ONLY production code (a `push_atomic` in a comment/test is ignored).
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

PROD="$(strip_for_grep "$OWNER")"

# --- guard (1): every push_atomic( is fed by into_accepted() ----------------
PUSH_LINES="$(echo "$PROD" | grep -nE 'push_atomic\(' || true)"
if [[ -z "$PUSH_LINES" ]]; then
    print_fail "no push_atomic( on the node spine — the S2 served-chain admit site is missing"
else
    NON_HANDOFF="$(echo "$PUSH_LINES" | grep -vE 'into_accepted\(\)' || true)"
    if [[ -n "$NON_HANDOFF" ]]; then
        print_fail "a node-spine push_atomic( is not fed by into_accepted() (raw-bytes / non-handoff serve ingress): $NON_HANDOFF"
    fi
fi

# --- guard (2): no direct served_chain_admit( on the node spine -------------
if echo "$PROD" | grep -qE 'served_chain_admit\('; then
    print_fail "node-spine code calls served_chain_admit( directly — served-chain mutation must go through the single push_atomic authority (CN-PROD-04)"
fi

# --- guard (3): the handoff channel carries the self-accepted fence ---------
if ! echo "$PROD" | grep -qE 'UnboundedSender<SelfAcceptedHandoff>'; then
    print_fail "the handoff channel is not typed UnboundedSender<SelfAcceptedHandoff> — the serve-ingress carrier must be the S1 self-accepted fence"
fi
for bad in 'UnboundedSender<Vec<u8>>' 'UnboundedSender<ForgedBlockArtifact>' 'UnboundedSender<bool>'; do
    if echo "$PROD" | grep -qE "$bad"; then
        print_fail "the handoff channel carries a non-self-accepted payload: $bad"
    fi
done

if (( FAIL == 0 )); then
    echo "OK (served-chain handoff fence): node-spine served chain fed ONLY by SelfAcceptedHandoff::into_accepted() -> the single push_atomic authority; no direct served_chain_admit; handoff channel typed to the self-accepted carrier (DC-NODE-06 serve-ingress clause)."
fi
exit $FAIL
