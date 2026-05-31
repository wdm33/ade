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
# Scope: the SYNC DRIVER `run_node_sync` inside
# `crates/ade_node/src/node_sync.rs`, production code only. The
# `#[cfg(test)]` module + line comments are stripped first, then the
# `run_node_sync` function body is isolated (from its `pub async fn`
# signature to the next top-level `^}`), so the greps see ONLY the sync
# path — NOT L5's `forge_one_from_recovered` handoff, which legitimately
# calls `run_real_forge` in the SAME file (PHASE4-N-F-C L5). The forge
# handoff's provenance is fenced separately by
# `ci_check_consensus_input_provenance.sh` (CN-CINPUT-03).
#
# Guards (scoped to the run_node_sync body):
#   (pos)  the sync driver calls `pump_block(` (the durable apply engine);
#   (neg1) no follower-as-sync: no `ade_core_interop` / `follow(`;
#   (neg2) no verdict-as-sync: no `derive_verdict` / `run_admission(`;
#   (neg3) no manual tip advance outside pump_block: no `.put_block(`,
#          no `AdvanceTip` construction, no `rollback_to_slot(`;
#   (neg4) no forge / cold / bundle on the SYNC path: no `run_real_forge` /
#          `run_forge`, no `InMemoryChainDb`, no `consensus_inputs_path`.

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

# Isolate the `run_node_sync` function body: from the line containing its
# `pub async fn run_node_sync` signature up to (and including) the next
# top-level closing brace (`^}` at column 0). The sync driver is the only
# tip-advancing path; L5's `forge_one_from_recovered` is a separate fn and
# is intentionally excluded here.
isolate_sync_fn() {
    strip_for_grep "$1" | awk '
        /pub async fn run_node_sync/ { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    '
}

SYNC_FN="$(isolate_sync_fn "$SYNC")"

if [[ -z "$SYNC_FN" ]]; then
    print_fail "could not isolate run_node_sync body in node_sync.rs (signature moved/renamed?)"
    echo "FAIL: ci_check_node_sync_via_pump"
    exit 1
fi

# --- guard (pos): the sync driver calls pump_block --------------------------
if ! echo "$SYNC_FN" | grep -qE 'pump_block\('; then
    print_fail "run_node_sync must call pump_block( — the sync path advances the tip only via the durable apply engine (DC-SYNC-01 / L4b)"
fi

# --- guard (neg1): no follower-as-validating-sync ---------------------------
for tok in 'ade_core_interop' 'follow\('; do
    if echo "$SYNC_FN" | grep -qE "$tok"; then
        print_fail "run_node_sync references a follower-as-sync token: $tok — ade_core_interop::follow is NOT validating sync (L4 fence)"
    fi
done

# --- guard (neg2): no verdict-as-sync ---------------------------------------
for tok in 'derive_verdict' 'run_admission\('; do
    if echo "$SYNC_FN" | grep -qE "$tok"; then
        print_fail "run_node_sync references a verdict-as-sync token: $tok — admission verdict derivation is NOT recoverable sync (L4 fence)"
    fi
done

# --- guard (neg3): no manual tip advance outside pump_block -----------------
# The ONLY tip-advancing site on the sync path is pump_block's apply_plan
# (in forward_sync/pump.rs). The sync driver must not store a block,
# construct an AdvanceTip, or write a tip itself.
for tok in '\.put_block\(' 'AdvanceTip' 'rollback_to_slot\('; do
    if echo "$SYNC_FN" | grep -qE "$tok"; then
        print_fail "run_node_sync performs a manual tip-advance / chain-mutation token outside pump_block: $tok — the tip advances ONLY via pump_block (L4 fence)"
    fi
done

# --- guard (neg4): no forge / cold / bundle on the SYNC path ----------------
# Scoped to run_node_sync: L5's forge_one_from_recovered legitimately calls
# run_real_forge, but that is the FORGE path (a separate fn), not sync.
for tok in 'run_real_forge' 'run_forge' 'InMemoryChainDb' 'consensus_inputs_path'; do
    if echo "$SYNC_FN" | grep -qE "$tok"; then
        print_fail "run_node_sync references a forbidden forge/cold/bundle token: $tok — the SYNC path does not forge (L4 fence)"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (node sync via pump): run_node_sync calls pump_block and advances the tip only via it — no follower-as-sync, no verdict-as-sync, no manual tip advance, no forge/cold/bundle (L5 forge handoff fenced separately by ci_check_consensus_input_provenance.sh)"
fi
exit $FAILED
