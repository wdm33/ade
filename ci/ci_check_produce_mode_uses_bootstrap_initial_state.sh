#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-T S5 — produce mode derives its initial forge state from the
# single bootstrap authority, never a synthetic shortcut.
#
# Mechanically strengthens BOTH CN-NODE-01 (bootstrap_initial_state is
# the sole initial-state authority — and is actually USED by --mode
# produce, not bypassed) and CN-PROD-02 (no parallel synthetic forge
# codepath). The pre-existing ci_check_node_binary_uses_single_bootstrap.sh
# only polices "called more than once"; it cannot catch a "called zero
# times via a synthetic bypass". This gate closes that hole for produce
# mode.
#
# Guards:
#   1. produce_mode.rs calls bootstrap_initial_state(.
#   2. produce_mode.rs seeds the GREEN ChainEvolution typestate
#      (ChainEvolution::seed) — the forge state flows through the
#      bootstrap-derived typestate.
#   3. produce_mode.rs contains NO SyntheticForgeInputs /
#      build_synthetic_forge_context (the synthetic never-leader
#      shortcut is deleted, not merely unused).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PRODUCE_RS="$REPO_ROOT/crates/ade_node/src/produce_mode.rs"

if [ ! -f "$PRODUCE_RS" ]; then
    echo "[ci_check_produce_mode_uses_bootstrap_initial_state] FAIL: missing $PRODUCE_RS"
    exit 1
fi

FAILED=0
print_fail() {
    echo "[ci_check_produce_mode_uses_bootstrap_initial_state] FAIL: $1"
    FAILED=1
}

# Guard 1 — positive: bootstrap_initial_state is called.
if ! grep -qE 'bootstrap_initial_state\(' "$PRODUCE_RS"; then
    print_fail "Guard 1 — produce_mode.rs does not call bootstrap_initial_state( (initial forge state must come from the single bootstrap authority)"
fi

# Guard 2 — positive: forge state flows through the ChainEvolution typestate.
if ! grep -qE 'ChainEvolution::seed' "$PRODUCE_RS"; then
    print_fail "Guard 2 — produce_mode.rs does not seed ChainEvolution (forge state must thread through the bootstrap-derived typestate)"
fi

# Guard 3 — negative: the synthetic never-leader shortcut is gone.
SYNTH=$(grep -nE 'SyntheticForgeInputs|build_synthetic_forge_context' "$PRODUCE_RS" || true)
if [ -n "$SYNTH" ]; then
    print_fail "Guard 3 — produce_mode.rs still references the synthetic forge shortcut (must be deleted):"
    echo "$SYNTH"
fi

if [ "$FAILED" -eq 0 ]; then
    echo "[ci_check_produce_mode_uses_bootstrap_initial_state] PASS (3/3 guards green)"
    exit 0
else
    exit 1
fi
