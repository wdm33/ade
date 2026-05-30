#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-C L4 — node-lifecycle sync path advances the tip ONLY via
# pump_block (DC-SYNC-01 driver containment).
#
# The `--mode node` lifecycle reaches a recoverable selected tip exactly
# one way: the L4 sync driver (`node_sync::run_node_sync`) feeds one
# ordered block-bytes source into `forward_sync::pump_block`, whose
# `apply_plan` makes StoreBlockBytes + AppendWal durable BEFORE AdvanceTip.
# This gate proves the sync path USES pump_block and does NOT advance the
# tip through any other path — not merely that pump_block exists somewhere.
#
# Scope: the sync-driver module `crates/ade_node/src/node_sync.rs`,
# production code only (the `#[cfg(test)]` module + line comments are
# stripped before the greps, so doc-comment prose and test fixtures cannot
# false-trip).
#
# Guards:
#   (pos)  the sync driver calls `pump_block(` (the durable apply engine);
#   (neg1) no follower-as-sync: no `ade_core_interop` / `follow(`;
#   (neg2) no verdict-as-sync: no `derive_verdict` / `run_admission(`;
#   (neg3) no manual tip advance outside pump_block: no `.put_block(`,
#          no `AdvanceTip` construction, no direct chaindb tip write;
#   (neg4) no forge / cold / bundle: no `run_real_forge` / `run_forge`,
#          no `InMemoryChainDb`, no `produce_mode` / `run_produce`,
#          no `consensus_inputs_path`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYNC="$REPO_ROOT/crates/ade_node/src/node_sync.rs"

FAILED=0
print_fail() { echo "FAIL (node sync via pump): $1"; FAILED=1; }

if [[ ! -f "$SYNC" ]]; then
    echo "FAIL (node sync via pump): sync driver not found at $SYNC"
    echo "FAIL: ci_check_node_sync_via_pump"
    exit 1
fi

# Strip the `#[cfg(test)]` module (attribute to EOF) + line comments.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

PROD="$(strip_for_grep "$SYNC")"

# --- guard (pos): the sync driver calls pump_block --------------------------
if ! echo "$PROD" | grep -qE 'pump_block\('; then
    print_fail "node_sync.rs production must call pump_block( — the sync path advances the tip only via the durable apply engine (DC-SYNC-01 / L4b)"
fi

# --- guard (neg1): no follower-as-validating-sync ---------------------------
for tok in 'ade_core_interop' 'follow\('; do
    if echo "$PROD" | grep -qE "$tok"; then
        print_fail "node_sync.rs references a follower-as-sync token: $tok — ade_core_interop::follow is NOT validating sync (L4 fence)"
    fi
done

# --- guard (neg2): no verdict-as-sync ---------------------------------------
for tok in 'derive_verdict' 'run_admission\('; do
    if echo "$PROD" | grep -qE "$tok"; then
        print_fail "node_sync.rs references a verdict-as-sync token: $tok — admission verdict derivation is NOT recoverable sync (L4 fence)"
    fi
done

# --- guard (neg3): no manual tip advance outside pump_block -----------------
# The ONLY tip-advancing site on the lifecycle path is pump_block's
# apply_plan (in forward_sync/pump.rs). The sync driver must not store a
# block, construct an AdvanceTip, or write a tip itself.
for tok in '\.put_block\(' 'AdvanceTip' 'rollback_to_slot\('; do
    if echo "$PROD" | grep -qE "$tok"; then
        print_fail "node_sync.rs performs a manual tip-advance / chain-mutation token outside pump_block: $tok — the tip advances ONLY via pump_block (L4 fence)"
    fi
done

# --- guard (neg4): no forge / cold / bundle on the sync path ----------------
for tok in 'run_real_forge' 'run_forge' 'InMemoryChainDb' 'produce_mode' 'run_produce' 'consensus_inputs_path'; do
    if echo "$PROD" | grep -qE "$tok"; then
        print_fail "node_sync.rs references a forbidden forge/cold/bundle token: $tok (L4 fence)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (node sync via pump): node_sync.rs sync driver calls pump_block and advances the tip only via it — no follower-as-sync, no verdict-as-sync, no manual tip advance, no forge/cold/bundle"
fi
exit $FAILED
