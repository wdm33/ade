#!/usr/bin/env bash
set -uo pipefail

# DC-NODE-06 — node-spine serve-ingress provenance fence.
#
# PHASE4-N-F-G-B/C ORIGIN (historical): the --mode node spine served chain was fed
# by a BLUE self-accepted artifact carried through a typed SelfAcceptedHandoff into
# the single ServedChainHandle::push_atomic accumulator. This gate fenced that
# handoff (only a self-accepted artifact could enter the serve task).
#
# PHASE4-N-U S3 REPOINT (DC-NODE-13): that handoff mechanism is RETIRED. The
# --mode node spine no longer feeds an in-memory accumulator; it SERVES A READ-ONLY
# PROJECTION OF THE DURABLE ChainDb (ServedChainSource::DurableChainDb), whose bytes
# entered ONLY through the validated durable admit (pump_block, DC-NODE-12) + the
# trusted seed (bootstrap_initial_state). DC-NODE-06's DEEPER invariant is PRESERVED
# and STRENGTHENED: only validated/admitted bytes may be served on the node spine —
# now via durable-provenance (CN-CONS-07 restatement), and it now survives restart.
# This gate is repointed to fence that evolved invariant: the node-spine serve
# sources bytes ONLY from the durable ChainDb projection, with NO retired
# non-durable serve ingress (no push_atomic accumulator / handoff / direct
# served_chain_admit). Complementary to ci_check_served_chain_projection.sh (the
# DC-NODE-13 projection-shape gate); this gate is the serve-PROVENANCE angle.
#
# Scope: the --mode node lifecycle owner SET {node_lifecycle.rs, node_sync.rs}.
# Production code only (line comments + each file's `#[cfg(test)]` module stripped).
# The --mode produce path (produce_mode.rs, CN-PROD-04) is a SEPARATE serve
# authority — it legitimately retains the snapshot accumulator (ServedChainSource::
# Snapshot) and is deliberately NOT in scope here.
#
# Guards (scoped to the stripped production bodies of the owner set):
#   (1) the node-spine serve sources the durable ChainDb projection
#       (ServedChainSource::DurableChainDb is present);
#   (2) the serve task takes the durable ChainDb as its read source
#       (run_node_serve_task over Arc<dyn ChainDb>);
#   (3) NO retired non-durable serve ingress on the node spine: no `push_atomic(`,
#       no `served_chain_admit(`, no `ServedChainHandle`, and no unbounded
#       SelfAcceptedHandoff handoff channel.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OWNERS=(
    "$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
    "$REPO_ROOT/crates/ade_node/src/node_sync.rs"
)

FAIL=0
print_fail() { echo "FAIL (served-chain serve-provenance fence): $1"; FAIL=1; }

# Strip the `#[cfg(test)]` module (attribute to EOF) + line comments, so the
# greps see ONLY production code.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

PROD=""
for OWNER in "${OWNERS[@]}"; do
    if [[ ! -f "$OWNER" ]]; then
        echo "FAIL (served-chain serve-provenance fence): node-spine serve owner not found at $OWNER"
        echo "FAIL: ci_check_served_chain_handoff_fence"
        exit 1
    fi
    PROD+="$(strip_for_grep "$OWNER")"$'\n'
done

# All greps feed `$PROD` via a here-string (`<<<`), not a pipe, to avoid spurious
# SIGPIPE under `set -o pipefail`.

# --- guard (1): the node-spine serve sources the durable ChainDb projection -----
if ! grep -qE 'ServedChainSource::DurableChainDb' <<< "$PROD"; then
    print_fail "the node-spine serve does not source the durable ChainDb projection (ServedChainSource::DurableChainDb missing) — DC-NODE-06 durable-provenance serve / DC-NODE-13"
fi

# --- guard (2): the serve task takes the durable ChainDb as its read source -----
if ! grep -qE 'Arc<dyn ChainDb>' <<< "$PROD"; then
    print_fail "run_node_serve_task does not take the durable ChainDb read source (Arc<dyn ChainDb>) — the serve provenance must be the durable store"
fi

# --- guard (3): NO retired non-durable serve ingress on the node spine ----------
if grep -qE '\.push_atomic\(' <<< "$PROD"; then
    print_fail "node-spine code calls .push_atomic( — the retired in-memory accumulator serve ingress; node serve must project the durable ChainDb"
fi
if grep -qE 'served_chain_admit\(' <<< "$PROD"; then
    print_fail "node-spine code calls served_chain_admit( directly — served bytes must come from the durable ChainDb projection, not a direct accumulator mutation"
fi
if grep -qE 'ServedChainHandle' <<< "$PROD"; then
    print_fail "node-spine code references ServedChainHandle — the retired in-memory served-chain accumulator (replaced by the durable ChainDb projection)"
fi
if grep -qE 'Unbounded(Sender|Receiver)<SelfAcceptedHandoff>|unbounded_channel::<SelfAcceptedHandoff>' <<< "$PROD"; then
    print_fail "node-spine code still wires the retired SelfAcceptedHandoff serve-handoff channel — the durable block IS what the serve task projects (no serve handoff)"
fi

if (( FAIL == 0 )); then
    echo "OK (served-chain serve-provenance fence): node-spine serve sources ONLY the durable ChainDb projection (ServedChainSource::DurableChainDb over Arc<dyn ChainDb>); no retired non-durable serve ingress (push_atomic / served_chain_admit / ServedChainHandle / SelfAcceptedHandoff channel) — DC-NODE-06 evolved durable-provenance serve (superseding the G-B handoff via DC-NODE-13)."
fi
exit $FAIL
