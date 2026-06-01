#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-B S3 — served-chain handoff fence.
# PHASE4-N-F-G-C S1 — BROADENED for the live-feed path (scope + guard-3 allow-list).
#
# Closes the DC-NODE-06 serve-ingress clause for the `--mode node` spine: the
# node-spine served chain is fed ONLY by a BLUE self-accepted artifact carried
# through the S1 handoff's `into_accepted()` into the single
# `ServedChainHandle::push_atomic` authority. Raw forge bytes, a failed forge
# outcome (`ForgeNotLeader` / `ForgeFailed`), a self-declared acceptance flag,
# and a peer-verdict substitute are banned from the serve ingress.
#
# Scope: the `--mode node` lifecycle owner SET (G-C broadened beyond the single
# `node_lifecycle.rs` to every node-spine serve owner — currently
# `{node_lifecycle.rs, node_sync.rs}` — so the fence still holds if the serve
# wiring moves between them). Production code only (line comments + each file's
# `#[cfg(test)]` module stripped). The `--mode produce` path (`produce_mode.rs`,
# CN-PROD-04) is a SEPARATE serve authority + gate and is deliberately NOT in
# scope here — its `push_atomic` is fed by the produce-mode broadcast bridge,
# not the node-spine self-accept handoff.
#
# Guards (scoped to the stripped production bodies of the owner set):
#   (1) every `push_atomic(` on the node spine is fed by `into_accepted()`
#       (no raw-bytes / non-handoff serve ingress);
#   (2) no direct `served_chain_admit(` on the node spine — served-chain
#       mutation happens ONLY through the single `push_atomic` authority;
#   (3) ALLOW-LIST (G-C, was a 3-name deny-list): every node-spine unbounded
#       handoff channel (`UnboundedSender<…>` / `UnboundedReceiver<…>` /
#       `unbounded_channel::<…>`) MUST carry `SelfAcceptedHandoff` — ANY other
#       payload fails (not just `<Vec<u8>>` / `<ForgedBlockArtifact>` / `<bool>`);
#       and at least one `UnboundedSender<SelfAcceptedHandoff>` must be present.
#       The bounded live-feed channel (`mpsc::channel::<AdmissionPeerEvent>`) is
#       NOT a handoff channel and is intentionally not matched.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OWNERS=(
    "$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
    "$REPO_ROOT/crates/ade_node/src/node_sync.rs"
)

FAIL=0
print_fail() { echo "FAIL (served-chain handoff fence): $1"; FAIL=1; }

# Strip the `#[cfg(test)]` module (attribute to EOF) + line comments, so the
# greps see ONLY production code (a `push_atomic` in a comment/test is ignored).
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Concatenate the stripped production bodies of every node-spine serve owner.
PROD=""
for OWNER in "${OWNERS[@]}"; do
    if [[ ! -f "$OWNER" ]]; then
        echo "FAIL (served-chain handoff fence): node-spine serve owner not found at $OWNER"
        echo "FAIL: ci_check_served_chain_handoff_fence"
        exit 1
    fi
    PROD+="$(strip_for_grep "$OWNER")"$'\n'
done

# All greps below feed `$PROD` via a here-string (`<<<`), NOT `echo "$PROD" |
# grep`. With `set -o pipefail`, `grep -q` exits early on a match while the
# upstream `echo` is still writing a large input — `echo` then takes SIGPIPE
# (141), which pipefail would surface as a spurious pipe failure (a false
# negative on the positive checks). A here-string avoids the pipe entirely.

# --- guard (1): every push_atomic( is fed by into_accepted() ----------------
PUSH_LINES="$(grep -nE 'push_atomic\(' <<< "$PROD" || true)"
if [[ -z "$PUSH_LINES" ]]; then
    print_fail "no push_atomic( on the node spine — the S2 served-chain admit site is missing"
else
    NON_HANDOFF="$(grep -vE 'into_accepted\(\)' <<< "$PUSH_LINES" || true)"
    if [[ -n "$NON_HANDOFF" ]]; then
        print_fail "a node-spine push_atomic( is not fed by into_accepted() (raw-bytes / non-handoff serve ingress): $NON_HANDOFF"
    fi
fi

# --- guard (2): no direct served_chain_admit( on the node spine -------------
if grep -qE 'served_chain_admit\(' <<< "$PROD"; then
    print_fail "node-spine code calls served_chain_admit( directly — served-chain mutation must go through the single push_atomic authority (CN-PROD-04)"
fi

# --- guard (3): ALLOW-LIST — every node-spine unbounded handoff channel must --
# --- carry SelfAcceptedHandoff (G-C; was a 3-name deny-list) ----------------
# Extract every unbounded-channel payload type, then keep only the ones that are
# NOT SelfAcceptedHandoff — any such token is an allow-list violation.
BAD_UNBOUNDED="$(grep -oE 'Unbounded(Sender|Receiver)<[^>]*>|unbounded_channel::<[^>]*>' <<< "$PROD" | grep -vE '<SelfAcceptedHandoff>' || true)"
if [[ -n "$BAD_UNBOUNDED" ]]; then
    print_fail "a node-spine unbounded handoff channel carries a non-SelfAcceptedHandoff payload (allow-list violation): $BAD_UNBOUNDED"
fi
# And the self-accepted handoff channel must be present at all.
if ! grep -qE 'UnboundedSender<SelfAcceptedHandoff>' <<< "$PROD"; then
    print_fail "the handoff channel is not typed UnboundedSender<SelfAcceptedHandoff> — the serve-ingress carrier must be the S1 self-accepted fence"
fi

if (( FAIL == 0 )); then
    echo "OK (served-chain handoff fence): node-spine served chain fed ONLY by SelfAcceptedHandoff::into_accepted() -> the single push_atomic authority; no direct served_chain_admit; handoff channel typed to the self-accepted carrier (DC-NODE-06 serve-ingress clause)."
fi
exit $FAIL
